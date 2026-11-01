//! Assets по индексу (план §7.1): `objects/<hh>/<hash>` из
//! `resources.download.minecraft.net`.

use std::path::Path;

use serde::Deserialize;

use crate::error::CoreError;
use crate::install::download::DownloadTask;

/// Базовый URL хранилища ассетов.
pub const ASSETS_BASE_URL: &str = "https://resources.download.minecraft.net";

/// Объект ассета в индексе.
#[derive(Debug, Clone)]
pub struct AssetObject {
    /// SHA-1 содержимого (и одновременно имя файла).
    pub hash: String,
    pub size: u64,
}

/// Распарсенный индекс ассетов.
#[derive(Debug, Clone)]
pub struct AssetIndex {
    pub objects: Vec<AssetObject>,
    /// Сумма размеров всех объектов.
    pub total_size: u64,
}

#[derive(Deserialize)]
struct RawObject {
    hash: String,
    size: u64,
}

#[derive(Deserialize)]
struct RawIndex {
    objects: std::collections::BTreeMap<String, RawObject>,
}

/// Распарсить asset-index (`objects` — карта имя → `{hash, size}`).
///
/// Порядок объектов — по имени файла (BTreeMap), чтобы набор задач был
/// детерминированным.
pub fn parse_asset_index(bytes: &[u8]) -> Result<AssetIndex, CoreError> {
    let raw: RawIndex = serde_json::from_slice(bytes)?;
    let objects: Vec<AssetObject> = raw
        .objects
        .into_values()
        .map(|object| AssetObject {
            hash: object.hash,
            size: object.size,
        })
        .collect();
    let total_size = objects.iter().map(|object| object.size).sum();
    Ok(AssetIndex {
        objects,
        total_size,
    })
}

fn tasks_from(index: &AssetIndex, assets_dir: &Path, base: &str) -> Vec<DownloadTask> {
    let base = base.trim_end_matches('/');
    index
        .objects
        .iter()
        .map(|object| {
            let hash = &object.hash;
            let prefix = hash.get(..2).unwrap_or("");
            DownloadTask {
                url: format!("{base}/{prefix}/{hash}"),
                dest: assets_dir.join("objects").join(prefix).join(hash),
                sha1: Some(hash.clone()),
                size: Some(object.size),
            }
        })
        .collect()
}

/// Задачи загрузки всех объектов индекса в `assets_dir` (боевой CDN).
pub fn asset_tasks(index: &AssetIndex, assets_dir: &Path) -> Vec<DownloadTask> {
    tasks_from(index, assets_dir, ASSETS_BASE_URL)
}

/// Как [`asset_tasks`], но с произвольным базовым URL (тесты, зеркала).
pub fn asset_tasks_with_base(
    index: &AssetIndex,
    assets_dir: &Path,
    base_url: &str,
) -> Vec<DownloadTask> {
    tasks_from(index, assets_dir, base_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_prefix_uses_two_chars() {
        let index = AssetIndex {
            objects: vec![AssetObject {
                hash: "ab".to_string(),
                size: 1,
            }],
            total_size: 1,
        };
        let tasks = tasks_from(&index, Path::new("/a"), "http://x/");
        assert_eq!(tasks[0].url, "http://x/ab/ab");
        assert_eq!(tasks[0].dest, Path::new("/a/objects/ab/ab"));
    }
}
