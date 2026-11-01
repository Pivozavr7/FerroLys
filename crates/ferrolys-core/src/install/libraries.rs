//! Библиотеки: задачи загрузки и раскладка natives (план §7.1).
//!
//! Библиотеки фильтруются правилами для linux/x86_64 ([`library_allowed`]).
//! Старый формат (`natives` + `downloads.classifiers`) даёт дополнительную
//! задачу на скачивание классификатора и пару [`NativesTarget`] для
//! распаковки; у новых версий natives-джары входят в classpath как обычные
//! артефакты и не распаковываются.

use std::path::{Path, PathBuf};

use crate::install::download::DownloadTask;
use crate::install::maven::{expand_arch, maven_path};
use crate::install::version_json::{library_allowed, Artifact, VersionJson};

/// Куда и что распаковывать из natives-классификатора.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativesTarget {
    /// Скачанный классификатор-jar (внутри `libraries_dir`).
    pub archive: PathBuf,
    /// Каталог распаковки (`…/versions/<id>/natives`).
    pub dest: PathBuf,
    /// Префиксы путей, которые пропускаются (`extract.exclude` библиотеки).
    pub excludes: Vec<String>,
}

/// Добавить классификатор к maven-пути: `…/a-1.0.jar` → `…/a-1.0-cls.jar`.
fn with_classifier(maven: &str, classifier: &str) -> String {
    match maven.rsplit_once('.') {
        Some((stem, ext)) => format!("{stem}-{classifier}.{ext}"),
        None => format!("{maven}-{classifier}"),
    }
}

fn task_for(artifact: &Artifact, dest: PathBuf) -> Option<DownloadTask> {
    if artifact.url.is_empty() {
        tracing::warn!(
            path = %dest.display(),
            "library artifact has no url, skipping"
        );
        return None;
    }
    Some(DownloadTask {
        url: artifact.url.clone(),
        dest,
        sha1: Some(artifact.sha1.clone()),
        size: Some(artifact.size),
    })
}

/// Задачи загрузки всех разрешённых (linux) библиотек в `libraries_dir`.
pub fn library_tasks(vj: &VersionJson, libraries_dir: &Path) -> Vec<DownloadTask> {
    let mut tasks = Vec::new();
    for library in &vj.libraries {
        if !library_allowed(library) {
            continue;
        }

        if let Some(artifact) = &library.artifact {
            match maven_path(&library.name) {
                Ok(relative) => {
                    let dest = libraries_dir.join(relative);
                    tasks.extend(task_for(artifact, dest));
                }
                Err(err) => {
                    tracing::warn!(name = %library.name, %err, "cannot build maven path");
                }
            }
        }

        // Старый формат: natives.linux → классификатор из downloads.classifiers
        if let Some(classifier) = &library.natives_linux {
            let classifier = expand_arch(classifier);
            let Some((_, artifact)) = library
                .classifiers
                .iter()
                .find(|(name, _)| *name == classifier)
            else {
                tracing::warn!(name = %library.name, classifier, "missing natives classifier artifact");
                continue;
            };
            let Ok(relative) = maven_path(&library.name) else {
                tracing::warn!(name = %library.name, "cannot build maven path for natives");
                continue;
            };
            let dest = libraries_dir.join(with_classifier(&relative, &classifier));
            tasks.extend(task_for(artifact, dest));
        }
    }
    tasks
}

/// Библиотеки со старым форматом natives, разрешённые на linux:
/// что распаковать и куда.
pub fn natives_targets(
    vj: &VersionJson,
    libraries_dir: &Path,
    natives_dir: &Path,
) -> Vec<NativesTarget> {
    vj.libraries
        .iter()
        .filter(|library| library_allowed(library))
        .filter(|library| library.natives_linux.is_some())
        .filter_map(|library| {
            let relative = maven_path(&library.name).ok()?;
            let classifier = expand_arch(library.natives_linux.as_deref()?);
            Some(NativesTarget {
                archive: libraries_dir.join(with_classifier(&relative, &classifier)),
                dest: natives_dir.to_path_buf(),
                excludes: library.extract_excludes.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::install::version_json::Library;

    #[test]
    fn with_classifier_inserts_before_extension() {
        assert_eq!(
            with_classifier("a/b/1.0/b-1.0.jar", "natives-linux"),
            "a/b/1.0/b-1.0-natives-linux.jar"
        );
        // версия с точкой в конце не ломает вставку
        assert_eq!(
            with_classifier("io/netty/4.0.15.Final/netty-4.0.15.Final.jar", "shaded"),
            "io/netty/4.0.15.Final/netty-4.0.15.Final-shaded.jar"
        );
    }

    #[test]
    fn task_for_skips_empty_url() {
        let artifact = Artifact {
            url: String::new(),
            sha1: "0".repeat(40),
            size: 1,
        };
        assert!(task_for(&artifact, PathBuf::from("/x.jar")).is_none());
    }

    #[test]
    fn library_without_artifact_and_natives_adds_nothing() {
        let library: Library = serde_json::from_str(r#"{"name":"a:b:1.0"}"#).unwrap();
        let vj = VersionJson {
            id: "test".into(),
            main_class: "Main".into(),
            java_version: None,
            arguments: vec![],
            jvm_arguments: vec![],
            asset_index: serde_json::from_str(
                r#"{"id":"1","url":"http://x","sha1":"00","size":1}"#,
            )
            .unwrap(),
            downloads: serde_json::from_str("{}").unwrap(),
            libraries: vec![library],
            logging_client: None,
        };
        assert!(library_tasks(&vj, Path::new("/lib")).is_empty());
        assert!(natives_targets(&vj, Path::new("/lib"), Path::new("/nat")).is_empty());
    }
}
