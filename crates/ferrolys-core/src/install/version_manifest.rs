//! Клиент piston-meta: манифест списка версий с TTL-кэшем (план §7.1).
//!
//! Манифест кэшируется в `$XDG_CACHE_HOME/ferrolys/` на [`MANIFEST_TTL_SECS`].
//! Свежий кэш отдаётся без сети; просроченный кэш используется как fallback,
//! если сеть недоступна (с `tracing::warn!`, без паник).

use std::fs;
use std::path::Path;
use std::time::Duration;

use serde::Deserialize;

use crate::error::CoreError;
use crate::paths::FerrolysPaths;

/// Базовый URL манифеста версий (piston-meta).
pub const PISTON_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

/// Имя файла кэша внутри `cache_dir()`.
const CACHE_FILE: &str = "version_manifest_v2.json";

/// TTL свежести кэша манифеста.
pub const MANIFEST_TTL_SECS: u64 = 600;

/// Тип версии в манифесте (`type` в JSON).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionType {
    Release,
    Snapshot,
    OldBeta,
    OldAlpha,
    Preview,
    /// Неизвестный тип из будущих версий манифеста — не ошибка парсинга.
    Unknown,
}

impl VersionType {
    fn from_raw(raw: &str) -> Self {
        match raw {
            "release" => Self::Release,
            "snapshot" => Self::Snapshot,
            "old_beta" => Self::OldBeta,
            "old_alpha" => Self::OldAlpha,
            "preview" => Self::Preview,
            _ => Self::Unknown,
        }
    }
}

impl<'de> Deserialize<'de> for VersionType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Ok(Self::from_raw(&raw))
    }
}

/// Строка версии в манифесте.
#[derive(Debug, Clone, Deserialize)]
pub struct ManifestEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: VersionType,
    pub url: String,
    pub sha1: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

/// Распарсенный манифест версий.
#[derive(Debug, Clone)]
pub struct VersionManifest {
    pub latest_release: String,
    pub latest_snapshot: String,
    pub versions: Vec<ManifestEntry>,
}

impl VersionManifest {
    /// Найти версию по идентификатору.
    pub fn get(&self, id: &str) -> Option<&ManifestEntry> {
        self.versions.iter().find(|entry| entry.id == id)
    }
}

#[derive(Debug, Deserialize)]
struct RawLatest {
    release: String,
    snapshot: String,
}

#[derive(Debug, Deserialize)]
struct RawManifest {
    latest: RawLatest,
    versions: Vec<ManifestEntry>,
}

fn parse(bytes: &[u8]) -> Result<VersionManifest, CoreError> {
    let raw: RawManifest = serde_json::from_slice(bytes)?;
    Ok(VersionManifest {
        latest_release: raw.latest.release,
        latest_snapshot: raw.latest.snapshot,
        versions: raw.versions,
    })
}

/// Свежий ли кэш (по mtime, младше [`MANIFEST_TTL_SECS`]).
fn cache_is_fresh(file: &Path) -> bool {
    let Ok(meta) = fs::metadata(file) else {
        return false;
    };
    let Ok(modified) = meta.modified() else {
        return false;
    };
    modified
        .elapsed()
        .map(|age| age < Duration::from_secs(MANIFEST_TTL_SECS))
        .unwrap_or(false)
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

async fn fetch_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, CoreError> {
    let response = client.get(url).send().await?.error_for_status()?;
    let bytes = response.bytes().await?;
    Ok(bytes.to_vec())
}

/// Скачать манифест [`PISTON_MANIFEST_URL`] (через TTL-кэш).
pub async fn fetch_manifest(
    client: &reqwest::Client,
    paths: &FerrolysPaths,
) -> Result<VersionManifest, CoreError> {
    fetch_manifest_from(client, paths, PISTON_MANIFEST_URL).await
}

/// Как [`fetch_manifest`], но с произвольным URL (для тестов и зеркал).
///
/// Порядок: свежий кэш → сеть → запись кэша; при сетевой ошибке — fallback
/// на имеющийся (пусть и просроченный) кэш.
pub async fn fetch_manifest_from(
    client: &reqwest::Client,
    paths: &FerrolysPaths,
    url: &str,
) -> Result<VersionManifest, CoreError> {
    let cache_file = paths.cache_dir().join(CACHE_FILE);

    if cache_is_fresh(&cache_file) {
        match fs::read(&cache_file) {
            Ok(bytes) => match parse(&bytes) {
                Ok(manifest) => return Ok(manifest),
                Err(err) => tracing::warn!(%err, "corrupt version manifest cache, refetching"),
            },
            Err(err) => tracing::warn!(%err, "unreadable version manifest cache, refetching"),
        }
    }

    match fetch_bytes(client, url).await {
        Ok(bytes) => {
            let manifest = parse(&bytes)?;
            if let Err(err) = write_cache(&cache_file, &bytes) {
                tracing::warn!(%err, "failed to write version manifest cache");
            }
            Ok(manifest)
        }
        Err(net_err) => {
            if cache_file.exists() {
                tracing::warn!(error = %net_err, "manifest fetch failed, serving cached copy");
                let bytes = fs::read(&cache_file)?;
                parse(&bytes)
            } else {
                Err(net_err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_type_maps_known_and_unknown_strings() {
        assert_eq!(VersionType::from_raw("release"), VersionType::Release);
        assert_eq!(VersionType::from_raw("old_beta"), VersionType::OldBeta);
        assert_eq!(VersionType::from_raw("old_alpha"), VersionType::OldAlpha);
        assert_eq!(VersionType::from_raw("preview"), VersionType::Preview);
        assert_eq!(VersionType::from_raw("quantum"), VersionType::Unknown);
    }

    #[test]
    fn cache_is_fresh_false_when_missing() {
        assert!(!cache_is_fresh(Path::new("/nonexistent/manifest.json")));
    }

    #[test]
    fn parse_rejects_garbage() {
        assert!(matches!(parse(b"not json"), Err(CoreError::Json(_))));
    }
}
