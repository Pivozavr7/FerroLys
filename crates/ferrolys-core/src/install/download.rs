//! Конкурентный движок загрузок (план §7.1): resume через Range, SHA-1
//! каждого файла, ретраи с бэкоффом и общий прогресс через `watch`.
//!
//! Каждый файл скачивается в `<dest>.part` и переименовывается только после
//! успешной проверки SHA-1 — бракованный контент никогда не попадает в `dest`.

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use futures_util::stream::{self, StreamExt};
use tokio::sync::watch;

use super::cache;
use crate::error::CoreError;

/// Сколько загрузок идёт одновременно.
pub const DOWNLOAD_CONCURRENCY: usize = 6;

/// Сколько попыток на одну задачу (сетевые ошибки, 5xx, битый SHA-1).
pub const DOWNLOAD_ATTEMPTS: u32 = 3;

/// Базовый бэк-офф ретрая: `200ms * 2^(attempt-1)`.
const RETRY_BASE: Duration = Duration::from_millis(200);

/// Как часто обновлять прогресс по байтам.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

/// Задача загрузки одного файла.
#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub url: String,
    pub dest: PathBuf,
    /// Ожидаемый SHA-1 (hex); `None` — принять как есть.
    pub sha1: Option<String>,
    /// Ожидаемый размер в байтах (для прогресса).
    pub size: Option<u64>,
}

/// Общий прогресс установки (байты/скорость/ETA), шлётся в `watch`.
#[derive(Debug, Clone, Default)]
pub struct Progress {
    pub files_total: usize,
    pub files_done: usize,
    pub bytes_total: u64,
    pub bytes_done: u64,
    pub speed_bps: u64,
    pub eta_secs: Option<u64>,
}

/// Ошибка одной попытки: с ретраем или без.
struct AttemptFail {
    error: CoreError,
    retryable: bool,
}

impl AttemptFail {
    fn retryable(error: CoreError) -> Self {
        Self {
            error,
            retryable: true,
        }
    }

    fn fatal(error: CoreError) -> Self {
        Self {
            error,
            retryable: false,
        }
    }
}

/// Общее состояние прогресса всех задач.
///
/// `bytes_done = completed_bytes + Σ inflight`: каждый файл учитывается ровно
/// один раз — при завершении его вклад переносится из inflight в completed.
/// Поэтому фаза установки не сбрасывает счётчики, а суммирует их.
struct ProgressState {
    files_total: usize,
    bytes_total: u64,
    completed_bytes: AtomicU64,
    files_done: AtomicU64,
    /// Сколько байт уже «принято» по каждому незавершённому файлу.
    inflight: Mutex<HashMap<PathBuf, u64>>,
    start: Instant,
    last_publish: Mutex<Instant>,
}

impl ProgressState {
    fn bytes_done(&self) -> u64 {
        let inflight = match self.inflight.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        self.completed_bytes.load(Ordering::Relaxed) + inflight.values().sum::<u64>()
    }

    /// Задать вклад `.part` файла (0 — убрать из inflight).
    fn set_inflight(&self, dest: &Path, bytes: u64) {
        let mut inflight = match self.inflight.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if bytes == 0 {
            inflight.remove(dest);
        } else {
            inflight.insert(dest.to_path_buf(), bytes);
        }
    }

    /// Дозаписать байты в вклад текущего файла.
    fn add_inflight(&self, dest: &Path, bytes: u64) {
        if bytes == 0 {
            return;
        }
        let mut inflight = match self.inflight.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        *inflight.entry(dest.to_path_buf()).or_insert(0) += bytes;
    }

    /// Сверить inflight с реальным размером `.part` на диске (после ошибки).
    fn sync_inflight(&self, dest: &Path) {
        let len = fs::metadata(part_path(dest))
            .map(|meta| meta.len())
            .unwrap_or(0);
        self.set_inflight(dest, len);
    }

    /// Файл завершён: вклад `.part` переносится в completed.
    fn complete(&self, dest: &Path, bytes: u64) {
        self.set_inflight(dest, 0);
        self.completed_bytes.fetch_add(bytes, Ordering::Relaxed);
        self.files_done.fetch_add(1, Ordering::Relaxed);
    }

    fn publish(&self, tx: &watch::Sender<Progress>) {
        let bytes_done = self.bytes_done();
        let elapsed = self.start.elapsed().as_secs_f64().max(0.001);
        let speed_bps = (bytes_done as f64 / elapsed) as u64;
        let eta_secs = self
            .bytes_total
            .checked_sub(bytes_done)
            .and_then(|remaining| remaining.checked_div(speed_bps));
        tx.send_replace(Progress {
            files_total: self.files_total,
            files_done: self.files_done.load(Ordering::Relaxed) as usize,
            bytes_total: self.bytes_total,
            bytes_done,
            speed_bps,
            eta_secs,
        });
    }

    /// Опубликовать, если с прошлого раза прошло ≥ [`PROGRESS_INTERVAL`].
    fn publish_throttled(&self, tx: &watch::Sender<Progress>) {
        let mut last = match self.last_publish.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if last.elapsed() >= PROGRESS_INTERVAL {
            *last = Instant::now();
            self.publish(tx);
        }
    }
}

/// `<dest>.part` — путь временного файла (через OsString, не `with_extension`).
fn part_path(dest: &Path) -> PathBuf {
    let mut name = OsString::from(dest.as_os_str());
    name.push(".part");
    PathBuf::from(name)
}

/// Существующий файл уже валиден? (sha1 == None → считаем валидным).
fn existing_is_valid(dest: &Path, sha1: Option<&str>) -> bool {
    if !dest.exists() {
        return false;
    }
    match sha1 {
        None => true,
        Some(expected) => match cache::sha1_file(dest) {
            Ok(actual) => actual == expected,
            Err(err) => {
                tracing::warn!(%err, path = %dest.display(), "cannot hash existing file, redownloading");
                false
            }
        },
    }
}

/// Одна попытка скачать файл целиком (с учётом `.part`).
async fn attempt_download(
    client: &reqwest::Client,
    task: &DownloadTask,
    state: &ProgressState,
    tx: &watch::Sender<Progress>,
) -> Result<(), AttemptFail> {
    if let Some(parent) = task.dest.parent() {
        fs::create_dir_all(parent).map_err(|err| AttemptFail::fatal(err.into()))?;
    }
    let part = part_path(&task.dest);
    let part_len = fs::metadata(&part).map(|meta| meta.len()).unwrap_or(0);
    // уже лежащий в .part объём входит в прогресс с самого старта попытки
    state.set_inflight(&task.dest, part_len);

    let mut request = client
        .get(&task.url)
        .header(reqwest::header::USER_AGENT, crate::APP_USER_AGENT);
    if part_len > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={part_len}-"));
    }
    let mut response = request
        .send()
        .await
        .map_err(|err| AttemptFail::retryable(err.into()))?;
    let status = response.status();

    if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE && part_len > 0 {
        // .part больше не нужен (например, файл уже целиком скачан)
        if let Err(err) = fs::remove_file(&part) {
            tracing::warn!(%err, "failed to drop stale .part");
        }
        return Err(AttemptFail::retryable(CoreError::DownloadFailed {
            url: task.url.clone(),
            attempts: 1,
        }));
    }
    if status.is_client_error() {
        return Err(AttemptFail::fatal(CoreError::DownloadFailed {
            url: task.url.clone(),
            attempts: 1,
        }));
    }
    if !status.is_success() {
        return Err(AttemptFail::retryable(CoreError::DownloadFailed {
            url: task.url.clone(),
            attempts: 1,
        }));
    }

    // 206 → дозаписываем; 200 → сервер проигнорировал Range, начинаем заново
    let append = part_len > 0 && status.as_u16() == 206;
    if part_len > 0 && !append {
        tracing::warn!(url = %task.url, "server ignored Range header, restarting");
        // файл обнуляется — старый вклад .part больше не существует
        state.set_inflight(&task.dest, 0);
    }
    let mut file = fs::File::options()
        .create(true)
        .append(append)
        .truncate(!append)
        .write(true)
        .open(&part)
        .map_err(|err| AttemptFail::fatal(err.into()))?;

    loop {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                file.write_all(&chunk)
                    .map_err(|err| AttemptFail::fatal(err.into()))?;
                state.add_inflight(&task.dest, chunk.len() as u64);
                state.publish_throttled(tx);
            }
            Ok(None) => break,
            Err(err) => {
                // обрыв тела — .part остаётся, следующая попытка дозапишет
                return Err(AttemptFail::retryable(err.into()));
            }
        }
    }
    file.flush().map_err(|err| AttemptFail::fatal(err.into()))?;
    drop(file);

    if let Some(expected) = &task.sha1 {
        let actual = cache::sha1_file(&part).map_err(AttemptFail::fatal)?;
        if &actual != expected {
            if let Err(err) = fs::remove_file(&part) {
                tracing::warn!(%err, "failed to drop corrupted .part");
            }
            return Err(AttemptFail::retryable(CoreError::Sha1Mismatch {
                url: task.url.clone(),
                expected: expected.clone(),
                actual,
            }));
        }
    }

    fs::rename(&part, &task.dest).map_err(|err| AttemptFail::fatal(err.into()))?;
    let real_len = fs::metadata(&task.dest).map(|meta| meta.len()).unwrap_or(0);
    state.complete(&task.dest, real_len);
    state.publish(tx);
    Ok(())
}

/// Скачать одну задачу: пропуск валидного файла, иначе до [`DOWNLOAD_ATTEMPTS`] попыток.
async fn download_one(
    client: &reqwest::Client,
    task: &DownloadTask,
    state: &ProgressState,
    tx: &watch::Sender<Progress>,
) -> Result<(), CoreError> {
    if existing_is_valid(&task.dest, task.sha1.as_deref()) {
        let size = fs::metadata(&task.dest).map(|meta| meta.len()).unwrap_or(0);
        state.complete(&task.dest, size);
        state.publish(tx);
        tracing::debug!(path = %task.dest.display(), "file already present, skipping");
        return Ok(());
    }
    if task.dest.exists() {
        if let Err(err) = fs::remove_file(&task.dest) {
            tracing::warn!(%err, "cannot remove outdated file, redownloading");
        }
    }

    let mut last_error = None;
    let mut attempts_made = 0;
    for attempt in 1..=DOWNLOAD_ATTEMPTS {
        attempts_made = attempt;
        match attempt_download(client, task, state, tx).await {
            Ok(()) => return Ok(()),
            Err(fail) => {
                tracing::warn!(
                    attempt,
                    retryable = fail.retryable,
                    error = %fail.error,
                    url = %task.url,
                    "download attempt failed"
                );
                // .part мог измениться (обрыв, удаление брака) — синхронизируем вклад
                state.sync_inflight(&task.dest);
                let retryable = fail.retryable;
                last_error = Some(fail.error);
                if !retryable {
                    break;
                }
                if attempt < DOWNLOAD_ATTEMPTS {
                    tokio::time::sleep(RETRY_BASE * (1 << (attempt - 1))).await;
                }
            }
        }
    }

    let error = last_error.unwrap_or_else(|| CoreError::DownloadFailed {
        url: task.url.clone(),
        attempts: attempts_made,
    });
    // ошибка должна называть реальное число попыток, а не константу
    Err(match error {
        CoreError::DownloadFailed { url, .. } => CoreError::DownloadFailed {
            url,
            attempts: attempts_made,
        },
        other => other,
    })
}

/// Скачать все задачи конкурентно ([`DOWNLOAD_CONCURRENCY`] потоков).
///
/// Идемпотентно: уже лежащие валидные файлы не трогаются. Дубликаты по `dest`
/// (в реальных индексах ассетов один hash встречается под разными именами)
/// схлопываются в одну задачу — иначе два потока писали бы в один `.part`.
/// Первая ошибка отменяет остальные задачи. Прогресс публикуется и при успехе,
/// и при ошибке.
pub async fn download_all(
    client: &reqwest::Client,
    tasks: Vec<DownloadTask>,
    tx: &watch::Sender<Progress>,
) -> Result<(), CoreError> {
    let mut seen = HashSet::with_capacity(tasks.len());
    let tasks: Vec<DownloadTask> = tasks
        .into_iter()
        .filter(|task| {
            let fresh = seen.insert(task.dest.clone());
            if !fresh {
                tracing::warn!(
                    dest = %task.dest.display(),
                    url = %task.url,
                    "duplicate download task dropped"
                );
            }
            fresh
        })
        .collect();

    let state = ProgressState {
        files_total: tasks.len(),
        bytes_total: tasks.iter().map(|task| task.size.unwrap_or(0)).sum(),
        completed_bytes: AtomicU64::new(0),
        files_done: AtomicU64::new(0),
        inflight: Mutex::new(HashMap::new()),
        start: Instant::now(),
        last_publish: Mutex::new(Instant::now()),
    };
    state.publish(tx);

    let mut stream = stream::iter(tasks.into_iter().map(|task| {
        let client = client.clone();
        let state = &state;
        async move { download_one(&client, &task, state, tx).await }
    }))
    .buffer_unordered(DOWNLOAD_CONCURRENCY);

    let mut first_error = None;
    while let Some(result) = stream.next().await {
        if let Err(err) = result {
            first_error = Some(err);
            break;
        }
    }
    drop(stream);

    state.publish(tx);
    match first_error {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn part_path_appends_suffix_without_touching_extension() {
        assert_eq!(
            part_path(Path::new("/data/lib/bar.jar")),
            PathBuf::from("/data/lib/bar.jar.part")
        );
    }

    #[test]
    fn sha1_hex_is_lowercase_two_digits() {
        // пустое содержимое → известный SHA-1
        assert_eq!(
            cache::sha1_hex(b""),
            "da39a3ee5e6b4b0d3255bfef95601890afd80709"
        );
    }
}
