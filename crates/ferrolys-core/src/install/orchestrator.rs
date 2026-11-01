//! Оркестратор установки версии (план §7.1): version-json, client jar,
//! libraries, natives, assets, logging-конфиг.
//!
//! Все фазы идемпотентны: уже скачанное с верным SHA-1 не трогается
//! ([`download_all`]), version-json и индекс ассетов кэшируются на диске —
//! повторный запуск не ходит в сеть.

use std::fs;
use std::path::Path;

use tokio::sync::watch;

use crate::error::CoreError;
use crate::install::assets::{asset_tasks_with_base, parse_asset_index, ASSETS_BASE_URL};
use crate::install::download::{download_all, DownloadTask, Progress};
use crate::install::libraries::{library_tasks, natives_targets};
use crate::install::maven::extract_natives;
use crate::install::version_json::parse_version_json;
use crate::install::version_manifest::{fetch_manifest_from, PISTON_MANIFEST_URL};
use crate::paths::FerrolysPaths;

/// Параметры установки версии в инстанс.
#[derive(Debug, Clone)]
pub struct InstallOptions {
    pub instance_id: String,
    pub version_id: String,
    pub client: reqwest::Client,
    /// Подмена URL-префиксов (тесты, зеркала): (боевой префикс → свой).
    pub url_map: Vec<(String, String)>,
}

impl InstallOptions {
    /// Боевые адреса Mojang, без подмен.
    pub fn new(
        instance_id: impl Into<String>,
        version_id: impl Into<String>,
        client: reqwest::Client,
    ) -> Self {
        Self {
            instance_id: instance_id.into(),
            version_id: version_id.into(),
            client,
            url_map: Vec::new(),
        }
    }
}

/// Подменить первый подходящий префикс URL (или вернуть как есть).
fn rewrite(url: &str, map: &[(String, String)]) -> String {
    for (from, to) in map {
        if let Some(rest) = url.strip_prefix(from.as_str()) {
            return format!("{to}{rest}");
        }
    }
    url.to_string()
}

async fn fetch_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, CoreError> {
    let response = client
        .get(url)
        .header(reqwest::header::USER_AGENT, crate::APP_USER_AGENT)
        .send()
        .await?
        .error_for_status()?;
    Ok(response.bytes().await?.to_vec())
}

/// Кэш-файл: читать с диска, иначе скачать по url и положить в кэш.
async fn cached_json(
    client: &reqwest::Client,
    cache_file: &Path,
    url: &str,
) -> Result<Vec<u8>, CoreError> {
    if let Ok(bytes) = fs::read(cache_file) {
        return Ok(bytes);
    }
    let bytes = fetch_bytes(client, url).await?;
    if let Err(err) = write_cache(cache_file, &bytes) {
        tracing::warn!(%err, path = %cache_file.display(), "failed to write cache");
    }
    Ok(bytes)
}

/// Атомарная запись кэша: `<file>.tmp` → rename.
fn write_cache(file: &Path, bytes: &[u8]) -> Result<(), CoreError> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = file.with_extension("json.tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, file)?;
    Ok(())
}

/// Скачать задачи одной фазы (пустая фаза — no-op).
async fn run_phase(
    client: &reqwest::Client,
    tasks: Vec<DownloadTask>,
    tx: &watch::Sender<Progress>,
) -> Result<(), CoreError> {
    download_all(client, tasks, tx).await
}

/// Установить версию `version_id` в инстанс `instance_id`.
///
/// Фазы: version-json (кэш) → client jar → libraries → natives (распаковка)
/// → assets (индекс в кэше) → logging-конфиг. Прогресс каждой фазы шлётся
/// в один `watch`.
pub async fn install_version(
    paths: &FerrolysPaths,
    opts: &InstallOptions,
    tx: &watch::Sender<Progress>,
) -> Result<(), CoreError> {
    // 1. version-json
    let manifest_url = rewrite(PISTON_MANIFEST_URL, &opts.url_map);
    let manifest = fetch_manifest_from(&opts.client, paths, &manifest_url).await?;
    let entry = manifest
        .get(&opts.version_id)
        .ok_or_else(|| CoreError::VersionNotFound(opts.version_id.clone()))?;

    let vj_cache = paths
        .cache_dir()
        .join("versions")
        .join(format!("{}.json", opts.version_id));
    let vj_bytes =
        cached_json(&opts.client, &vj_cache, &rewrite(&entry.url, &opts.url_map)).await?;
    let vj = parse_version_json(&vj_bytes)?;
    tracing::debug!(version = %vj.id, "version json loaded");

    let instance_dir = paths.instances_dir().join(&opts.instance_id);
    let version_dir = instance_dir.join("versions").join(&opts.version_id);
    let libraries_dir = instance_dir.join("libraries");
    let natives_dir = version_dir.join("natives");

    // 2. client jar
    let mut phase: Vec<DownloadTask> = Vec::new();
    if let Some(client_artifact) = &vj.downloads.client {
        phase.push(DownloadTask {
            url: rewrite(&client_artifact.url, &opts.url_map),
            dest: version_dir.join(format!("{}.jar", opts.version_id)),
            sha1: Some(client_artifact.sha1.clone()),
            size: Some(client_artifact.size),
        });
    }
    run_phase(&opts.client, phase, tx).await?;

    // 3. libraries (артефакты + natives-классификаторы старого формата)
    let mut phase = library_tasks(&vj, &libraries_dir);
    for task in &mut phase {
        task.url = rewrite(&task.url, &opts.url_map);
    }
    run_phase(&opts.client, phase, tx).await?;

    // 4. natives: распаковка скачанных классификаторов
    for target in natives_targets(&vj, &libraries_dir, &natives_dir) {
        if !target.archive.exists() {
            tracing::warn!(
                archive = %target.archive.display(),
                "natives archive missing, skipping extraction"
            );
            continue;
        }
        let extracted = extract_natives(&target.archive, &target.dest, &target.excludes)?;
        tracing::debug!(
            dest = %target.dest.display(),
            extracted,
            "natives extracted"
        );
    }

    // 5. assets
    let asset_index = &vj.asset_index;
    let ai_cache = paths
        .cache_dir()
        .join("asset_indexes")
        .join(format!("{}.json", asset_index.id));
    let ai_bytes = cached_json(
        &opts.client,
        &ai_cache,
        &rewrite(&asset_index.url, &opts.url_map),
    )
    .await?;
    let index = parse_asset_index(&ai_bytes)?;
    let assets_base = rewrite(ASSETS_BASE_URL, &opts.url_map);
    run_phase(
        &opts.client,
        asset_tasks_with_base(&index, &paths.assets_dir(), &assets_base),
        tx,
    )
    .await?;

    // 6. logging.client: сам конфиг (подмена JVM-аргумента — задача §7.2)
    if let Some(logging) = &vj.logging_client {
        let task = DownloadTask {
            url: rewrite(&logging.file.url, &opts.url_map),
            dest: paths
                .assets_dir()
                .join("log_configs")
                .join(&logging.file_id),
            sha1: Some(logging.file.sha1.clone()),
            size: Some(logging.file.size),
        };
        run_phase(&opts.client, vec![task], tx).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrite_replaces_first_matching_prefix() {
        let map = vec![(
            "https://cdn.example".to_string(),
            "http://127.0.0.1:8080".to_string(),
        )];
        assert_eq!(
            rewrite("https://cdn.example/a/b.jar", &map),
            "http://127.0.0.1:8080/a/b.jar"
        );
        assert_eq!(
            rewrite("https://other.example/a.jar", &map),
            "https://other.example/a.jar"
        );
        assert_eq!(
            rewrite("https://cdn.example", &map),
            "http://127.0.0.1:8080"
        );
    }

    #[test]
    fn rewrite_empty_map_is_identity() {
        assert_eq!(rewrite("https://x/y", &[]), "https://x/y");
    }
}
