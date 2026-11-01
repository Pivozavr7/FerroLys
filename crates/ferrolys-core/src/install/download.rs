//! Конкурентный движок загрузок (план §7.1): resume через Range, SHA-1
//! каждого файла, ретраи с бэкоффом и общий прогресс через `watch`.
//!
//! Каждый файл скачивается в `<dest>.part` и переименовывается только после
//! успешной проверки SHA-1 — бракованный контент никогда не попадает в `dest`.

use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use futures_util::stream::{self, StreamExt};
use sha1::{Digest, Sha1};
use tokio::sync::watch;

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
struct ProgressState {
    files_total: usize,
    bytes_total: u64,
    bytes_done: AtomicU64,
    files_done: AtomicU64,
    start: Instant,
    last_publish: Mutex<Instant>,
}

impl ProgressState {
    fn publish(&self, tx: &watch::Sender<Progress>) {
        let bytes_done = self.bytes_done.load(Ordering::Relaxed);
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

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// SHA-1 файла по содержимому (hex, нижний регистр).
fn sha1_file(path: &Path) -> Result<String, CoreError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha1::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(hex_encode(&hasher.finalize()))
}

/// Существующий файл уже валиден? (sha1 == None → считаем валидным).
fn existing_is_valid(dest: &Path, sha1: Option<&str>) -> bool {
    if !dest.exists() {
        return false;
    }
    match sha1 {
        None => true,
        Some(expected) => match sha1_file(dest) {
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
                state
                    .bytes_done
                    .fetch_add(chunk.len() as u64, Ordering::Relaxed);
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
        let actual = sha1_file(&part).map_err(AttemptFail::fatal)?;
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
    state.files_done.fetch_add(1, Ordering::Relaxed);
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
        state.bytes_done.fetch_add(size, Ordering::Relaxed);
        state.files_done.fetch_add(1, Ordering::Relaxed);
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
    for attempt in 1..=DOWNLOAD_ATTEMPTS {
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

    Err(last_error.unwrap_or_else(|| CoreError::DownloadFailed {
        url: task.url.clone(),
        attempts: DOWNLOAD_ATTEMPTS,
    }))
}

/// Скачать все задачи конкурентно ([`DOWNLOAD_CONCURRENCY`] потоков).
///
/// Идемпотентно: уже лежащие валидные файлы не трогаются. Первая ошибка
/// отменяет остальные задачи. Прогресс публикуется и при успехе, и при ошибке.
pub async fn download_all(
    client: &reqwest::Client,
    tasks: Vec<DownloadTask>,
    tx: &watch::Sender<Progress>,
) -> Result<(), CoreError> {
    let state = ProgressState {
        files_total: tasks.len(),
        bytes_total: tasks.iter().map(|task| task.size.unwrap_or(0)).sum(),
        bytes_done: AtomicU64::new(0),
        files_done: AtomicU64::new(0),
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
    fn hex_encode_is_lowercase_two_digits() {
        assert_eq!(hex_encode(&[0x00, 0x0f, 0xff]), "000fff");
    }
}
