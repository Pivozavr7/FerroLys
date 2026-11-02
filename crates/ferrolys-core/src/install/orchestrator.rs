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
use crate::install::cache;
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

/// Кэш-файл с проверкой SHA-1: читать с диска, но только если содержимое
/// совпадает с `expected_sha1` из манифеста; иначе — скачать заново.
///
/// Кэш без проверки опасен: каптив-портал/прокси может записать HTML вместо
/// JSON, и установка падала бы вечно (отравленный кэш не самоизлечивался).
async fn cached_json(
    client: &reqwest::Client,
    cache_file: &Path,
    url: &str,
    expected_sha1: &str,
) -> Result<Vec<u8>, CoreError> {
    if let Ok(bytes) = fs::read(cache_file) {
        let actual = cache::sha1_hex(&bytes);
        if actual == expected_sha1 {
            return Ok(bytes);
        }
        tracing::warn!(
            path = %cache_file.display(),
            expected = expected_sha1,
            actual,
            "corrupt cache entry (sha1 mismatch), refetching"
        );
        if let Err(err) = fs::remove_file(cache_file) {
            tracing::warn!(%err, "failed to drop corrupt cache entry");
        }
    }

    let bytes = cache::fetch_bytes(client, url).await?;
    let actual = cache::sha1_hex(&bytes);
    if actual != expected_sha1 {
        return Err(CoreError::Sha1Mismatch {
            url: url.to_string(),
            expected: expected_sha1.to_string(),
            actual,
        });
    }
    if let Err(err) = cache::write_cache(cache_file, &bytes) {
        tracing::warn!(%err, path = %cache_file.display(), "failed to write cache");
    }
    Ok(bytes)
}

/// Установить версию `version_id` в инстанс `instance_id`.
///
/// Фазы: version-json (кэш с проверкой SHA-1) → сбор ВСЕХ задач (client jar,
/// libraries, assets, logging) в один `download_all` → распаковка natives.
/// Один `download_all` даёт кумулятивный прогресс: счётчики не сбрасываются
/// между «фазами», UI не показывает регрессию.
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
    let vj_bytes = cached_json(
        &opts.client,
        &vj_cache,
        &rewrite(&entry.url, &opts.url_map),
        &entry.sha1,
    )
    .await?;
    let vj = parse_version_json(&vj_bytes)?;
    tracing::debug!(version = %vj.id, "version json loaded");

    let instance_dir = paths.instances_dir().join(&opts.instance_id);
    let version_dir = instance_dir.join("versions").join(&opts.version_id);
    let libraries_dir = instance_dir.join("libraries");
    let natives_dir = version_dir.join("natives");

    // 2. client jar
    let mut tasks: Vec<DownloadTask> = Vec::new();
    if let Some(client_artifact) = &vj.downloads.client {
        tasks.push(DownloadTask {
            url: rewrite(&client_artifact.url, &opts.url_map),
            dest: version_dir.join(format!("{}.jar", opts.version_id)),
            sha1: Some(client_artifact.sha1.clone()),
            size: Some(client_artifact.size),
        });
    }

    // 3. libraries (артефакты + natives-классификаторы старого формата)
    for mut task in library_tasks(&vj, &libraries_dir) {
        task.url = rewrite(&task.url, &opts.url_map);
        tasks.push(task);
    }

    // 4. assets (индекс в кэше — с проверкой SHA-1)
    let asset_index = &vj.asset_index;
    let ai_cache = paths
        .cache_dir()
        .join("asset_indexes")
        .join(format!("{}.json", asset_index.id));
    let ai_bytes = cached_json(
        &opts.client,
        &ai_cache,
        &rewrite(&asset_index.url, &opts.url_map),
        &asset_index.sha1,
    )
    .await?;
    let index = parse_asset_index(&ai_bytes)?;
    let assets_base = rewrite(ASSETS_BASE_URL, &opts.url_map);
    tasks.extend(asset_tasks_with_base(
        &index,
        &paths.assets_dir(),
        &assets_base,
    ));

    // 5. logging.client: сам конфиг (подмена JVM-аргумента — задача §7.2)
    if let Some(logging) = &vj.logging_client {
        tasks.push(DownloadTask {
            url: rewrite(&logging.file.url, &opts.url_map),
            dest: paths
                .assets_dir()
                .join("log_configs")
                .join(&logging.file_id),
            sha1: Some(logging.file.sha1.clone()),
            size: Some(logging.file.size),
        });
    }

    // 6. скачивание всех файлов одной пачкой (кумулятивный прогресс)
    download_all(&opts.client, tasks, tx).await?;

    // 7. natives: распаковка скачанных классификаторов
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
