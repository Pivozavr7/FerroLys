//! Парсер version-JSON (план §7.1): downloads, assetIndex, libraries с
//! правилами `rules`, аргументы (старый `minecraftArguments` и новый
//! `arguments.game/jvm`), logging, javaVersion.
//!
//! Подмножество правил под окружение запуска Ferrolys: linux, x86_64, ни одна
//! игровая фича не включена. Семантика — как у ванильного лаунчера: без
//! `rules` — разрешено, иначе решающее последнее совпавшее правило.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::error::CoreError;

use super::version_manifest::VersionType;

/// Действие правила.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Allow,
    Disallow,
}

impl<'de> Deserialize<'de> for Action {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        match raw.as_str() {
            "allow" => Ok(Self::Allow),
            "disallow" => Ok(Self::Disallow),
            other => Err(serde::de::Error::unknown_variant(
                other,
                &["allow", "disallow"],
            )),
        }
    }
}

/// Правило установки библиотеки или аргумента.
#[derive(Debug, Clone)]
pub struct Rule {
    pub action: Action,
    pub os_name: Option<String>,
    pub os_arch: Option<String>,
    /// Пары (фича, ожидаемое значение); сравниваются с окружением запуска
    /// (у нас ни одна фича не включена).
    pub features: Vec<(String, bool)>,
}

impl<'de> Deserialize<'de> for Rule {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct RawOs {
            #[serde(default)]
            name: Option<String>,
            #[serde(default)]
            arch: Option<String>,
        }

        #[derive(Deserialize)]
        struct Raw {
            action: Action,
            #[serde(default)]
            os: Option<RawOs>,
            #[serde(default)]
            features: BTreeMap<String, bool>,
        }

        let raw = Raw::deserialize(deserializer)?;
        Ok(Self {
            action: raw.action,
            os_name: raw.os.as_ref().and_then(|os| os.name.clone()),
            os_arch: raw.os.and_then(|os| os.arch),
            features: raw.features.into_iter().collect(),
        })
    }
}

/// Скачиваемый артефакт (jar, xml…).
#[derive(Debug, Clone, Deserialize)]
pub struct Artifact {
    pub url: String,
    pub sha1: String,
    pub size: u64,
}

/// Ссылка на индекс ассетов.
#[derive(Debug, Clone, Deserialize)]
pub struct AssetIndexRef {
    pub id: String,
    pub url: String,
    pub sha1: String,
    pub size: u64,
}

/// Клиентские/серверские скачиваемые файлы версии.
#[derive(Debug, Clone, Deserialize)]
pub struct Downloads {
    #[serde(default)]
    pub client: Option<Artifact>,
    #[serde(default)]
    pub server: Option<Artifact>,
}

/// Подмена log4j-конфига (`logging.client`).
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub argument: String,
    pub file_id: String,
    pub file: Artifact,
}

/// Библиотека version-JSON.
#[derive(Debug, Clone)]
pub struct Library {
    pub name: String,
    pub artifact: Option<Artifact>,
    /// `downloads.classifiers` (объект) → пары, отсортированы по имени.
    pub classifiers: Vec<(String, Artifact)>,
    /// `natives.linux` — классификатор для Linux, если у библиотеки есть
    /// нативные бинарники старого формата.
    pub natives_linux: Option<String>,
    pub rules: Vec<Rule>,
    /// `extract.exclude`.
    pub extract_excludes: Vec<String>,
}

impl<'de> Deserialize<'de> for Library {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct RawLibraryDownloads {
            #[serde(default)]
            artifact: Option<Artifact>,
            #[serde(default)]
            classifiers: Option<BTreeMap<String, Artifact>>,
        }

        #[derive(Deserialize)]
        struct RawExtract {
            #[serde(default)]
            exclude: Vec<String>,
        }

        #[derive(Deserialize)]
        struct Raw {
            name: String,
            #[serde(default)]
            downloads: Option<RawLibraryDownloads>,
            #[serde(default)]
            natives: Option<BTreeMap<String, String>>,
            #[serde(default)]
            rules: Vec<Rule>,
            #[serde(default)]
            extract: Option<RawExtract>,
        }

        let raw = Raw::deserialize(deserializer)?;
        let (artifact, classifiers) = raw
            .downloads
            .map(|downloads| (downloads.artifact, downloads.classifiers))
            .unwrap_or_default();
        Ok(Self {
            name: raw.name,
            artifact,
            classifiers: classifiers
                .map(|map| map.into_iter().collect())
                .unwrap_or_default(),
            natives_linux: raw
                .natives
                .and_then(|natives| natives.get("linux").cloned()),
            rules: raw.rules,
            extract_excludes: raw
                .extract
                .map(|extract| extract.exclude)
                .unwrap_or_default(),
        })
    }
}

/// Распарсенный version-JSON.
#[derive(Debug, Clone)]
pub struct VersionJson {
    pub id: String,
    pub main_class: String,
    /// Тип версии из JSON (`"type"`), `Unknown` если ключа нет.
    pub kind: VersionType,
    /// `javaVersion.majorVersion` (8, 17, 21…), `None` если ключа нет.
    pub java_version: Option<i32>,
    /// Игровые аргументы, отфильтрованные по правилам для linux/x86_64.
    pub arguments: Vec<String>,
    /// JVM-аргументы манифеста, отфильтрованные; пусто у старых версий.
    pub jvm_arguments: Vec<String>,
    pub asset_index: AssetIndexRef,
    pub downloads: Downloads,
    pub libraries: Vec<Library>,
    pub logging_client: Option<LoggingConfig>,
}

#[derive(Deserialize)]
struct RawJavaVersion {
    #[serde(rename = "majorVersion")]
    major: i32,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawArg {
    Plain(String),
    Conditional { rules: Vec<Rule>, value: Value },
}

#[derive(Deserialize)]
struct RawArguments {
    #[serde(default)]
    game: Vec<RawArg>,
    #[serde(default)]
    jvm: Vec<RawArg>,
}

#[derive(Deserialize)]
struct RawLogFile {
    id: String,
    sha1: String,
    size: u64,
    url: String,
}

#[derive(Deserialize)]
struct RawLoggingClient {
    argument: String,
    file: RawLogFile,
}

#[derive(Deserialize)]
struct RawLogging {
    client: RawLoggingClient,
}

#[derive(Deserialize)]
struct RawVersion {
    id: String,
    #[serde(rename = "mainClass")]
    main_class: String,
    /// Тип версии (`"type": "release"`); у старых манифестов отсутствует.
    #[serde(default, rename = "type")]
    kind: Option<String>,
    #[serde(default, rename = "javaVersion")]
    java_version: Option<RawJavaVersion>,
    #[serde(default)]
    arguments: Option<RawArguments>,
    #[serde(default, rename = "minecraftArguments")]
    minecraft_arguments: Option<String>,
    #[serde(rename = "assetIndex")]
    asset_index: AssetIndexRef,
    downloads: Downloads,
    libraries: Vec<Library>,
    #[serde(default)]
    logging: Option<RawLogging>,
}

/// Совпадает ли условие правила с окружением запуска (linux, x86_64, без фич).
fn rule_matches(rule: &Rule) -> bool {
    let os_ok = rule.os_name.as_deref().is_none_or(|name| name == "linux")
        && rule.os_arch.as_deref().is_none_or(|arch| arch == "x86_64");
    // У нас ни одна фича не включена → подходит только ожидание false.
    let features_ok = rule.features.iter().all(|(_, expected)| !expected);
    os_ok && features_ok
}

/// Оценка списка правил: решающее последнее совпавшее правило,
/// без совпадений — запрет.
fn rules_eval(rules: &[Rule]) -> bool {
    let mut allowed = false;
    for rule in rules {
        if rule_matches(rule) {
            allowed = rule.action == Action::Allow;
        }
    }
    allowed
}

/// Разрешена ли библиотека на linux/x86_64. Нет `rules` → разрешена.
pub fn library_allowed(library: &Library) -> bool {
    if library.rules.is_empty() {
        return true;
    }
    rules_eval(&library.rules)
}

/// Отфильтровать аргументы: строки — как есть, объекты — по правилам.
fn filter_args(args: Vec<RawArg>) -> Vec<String> {
    let mut out = Vec::new();
    for arg in args {
        match arg {
            RawArg::Plain(value) => out.push(value),
            RawArg::Conditional { rules, value } => {
                if !rules_eval(&rules) {
                    continue;
                }
                match value {
                    Value::String(value) => out.push(value),
                    Value::Array(items) => out.extend(
                        items
                            .into_iter()
                            .filter_map(|item| item.as_str().map(String::from)),
                    ),
                    other => tracing::warn!(?other, "unexpected argument value shape"),
                }
            }
        }
    }
    out
}

/// Распарсить version-JSON версии игры.
pub fn parse_version_json(bytes: &[u8]) -> Result<VersionJson, CoreError> {
    let raw: RawVersion = serde_json::from_slice(bytes)?;

    let (game_args, jvm_args) = match raw.arguments {
        Some(arguments) => (arguments.game, arguments.jvm),
        None => (Vec::new(), Vec::new()),
    };
    let mut arguments = filter_args(game_args);
    if let Some(legacy) = raw.minecraft_arguments {
        arguments.extend(legacy.split_whitespace().map(String::from));
    }
    let jvm_arguments = filter_args(jvm_args);

    Ok(VersionJson {
        id: raw.id,
        main_class: raw.main_class,
        kind: raw
            .kind
            .as_deref()
            .map(VersionType::from_raw)
            .unwrap_or(VersionType::Unknown),
        java_version: raw.java_version.map(|java| java.major),
        arguments,
        jvm_arguments,
        asset_index: raw.asset_index,
        downloads: raw.downloads,
        libraries: raw.libraries,
        logging_client: raw.logging.map(|logging| {
            let client = logging.client;
            LoggingConfig {
                argument: client.argument,
                file_id: client.file.id,
                file: Artifact {
                    url: client.file.url,
                    sha1: client.file.sha1,
                    size: client.file.size,
                },
            }
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_deserializes_from_json_shape() {
        let rule: Rule =
            serde_json::from_str(r#"{"action":"allow","os":{"name":"linux","arch":"x86"}}"#)
                .unwrap();
        assert_eq!(rule.action, Action::Allow);
        assert_eq!(rule.os_name.as_deref(), Some("linux"));
        assert_eq!(rule.os_arch.as_deref(), Some("x86"));
        assert!(rule.features.is_empty());
    }

    #[test]
    fn rule_with_no_os_matches_any_os() {
        let rule: Rule = serde_json::from_str(r#"{"action":"allow"}"#).unwrap();
        assert!(rule_matches(&rule));
    }
}
