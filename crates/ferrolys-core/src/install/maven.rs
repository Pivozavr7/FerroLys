//! maven-координаты и распаковка natives (план §7.1).
//!
//! Путь библиотеки внутри `libraries/` строится из координат
//! `group:artifact:version[:classifier][@ext]`; нативные бинарники старого
//! формата распаковываются из classifiers-jar в каталог `natives/`.

use std::fs::{self, File};
use std::io;
use std::path::Path;

use crate::error::CoreError;

/// `group:artifact:version[:classifier][@ext]` → путь внутри `libraries/`.
///
/// ```text
/// com.mojang:brigadier:1.0.18          → com/mojang/brigadier/1.0.18/brigadier-1.0.18.jar
/// org.lwjgl:lwjgl:3.3.3:natives-linux  → org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-linux.jar
/// group:artifact:1.0@zip               → group/artifact/1.0/artifact-1.0.zip
/// ```
pub fn maven_path(name: &str) -> Result<String, CoreError> {
    let (coordinates, extension) = match name.split_once('@') {
        Some((coordinates, extension)) => (coordinates, extension),
        None => (name, "jar"),
    };

    let parts: Vec<&str> = coordinates.split(':').collect();
    let (group, artifact, version, classifier) = match parts.as_slice() {
        [group, artifact, version] => (*group, *artifact, *version, None),
        [group, artifact, version, classifier] if !classifier.is_empty() => {
            (*group, *artifact, *version, Some(*classifier))
        }
        _ => return Err(CoreError::InvalidMaven(name.to_string())),
    };

    if group.is_empty() || artifact.is_empty() || version.is_empty() || extension.is_empty() {
        return Err(CoreError::InvalidMaven(name.to_string()));
    }

    let class = classifier
        .map(|class| format!("-{class}"))
        .unwrap_or_default();
    Ok(format!(
        "{}/{}/{}/{}-{version}{class}.{extension}",
        group.replace('.', "/"),
        artifact,
        version,
        artifact
    ))
}

/// `${arch}` → `x86_64` (единственная подстановка в natives-классификаторах).
pub fn expand_arch(placeholder: &str) -> String {
    placeholder.replace("${arch}", "x86_64")
}

/// Распаковать natives-jar в `dest`, вернуть число записанных файлов.
///
/// Пропускаются каталоги, пути с `META-INF`, пути, содержащие любой из
/// `excludes`, и небезопасные (path traversal) записи — они только
/// логируются. `dest` создаётся при необходимости.
pub fn extract_natives(
    archive: &Path,
    dest: &Path,
    excludes: &[String],
) -> Result<usize, CoreError> {
    fs::create_dir_all(dest)?;

    let file = File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file)?;
    let mut extracted = 0;

    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let name = entry.name().to_string();

        if name.ends_with('/')
            || name.contains("META-INF")
            || excludes.iter().any(|exclude| name.contains(exclude))
        {
            continue;
        }
        let Some(relative) = entry.enclosed_name() else {
            tracing::warn!(%name, "skipping unsafe zip entry");
            continue;
        };

        let out_path = dest.join(relative);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = File::create(&out_path)?;
        io::copy(&mut entry, &mut out)?;
        extracted += 1;
    }

    Ok(extracted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maven_path_keeps_dotted_group() {
        assert_eq!(
            maven_path("com.mojang:datafixerupper:4.0.32").unwrap(),
            "com/mojang/datafixerupper/4.0.32/datafixerupper-4.0.32.jar"
        );
    }

    #[test]
    fn maven_path_rejects_empty_classifier() {
        assert!(matches!(
            maven_path("a:b:1.0:"),
            Err(CoreError::InvalidMaven(_))
        ));
    }
}
