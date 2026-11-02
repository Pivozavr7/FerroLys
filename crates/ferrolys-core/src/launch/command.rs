//! Сборка команды запуска игры (план §7.2, Task 1).
//!
//! Чистая функция без I/O: собирает `CommandSpec` из распарсенного
//! version-JSON, путей инстанса и контекста авторизации. Все плейсхолдеры
//! (`${auth_player_name}` и др.) подставляются здесь; неизвестный ключ
//! заменяется на пустую строку с `tracing::warn!`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::install::maven::maven_path;
use crate::install::version_json::{library_allowed, VersionJson};
use crate::paths::FerrolysPaths;
use crate::APP_VERSION;

/// Имя лаунчера для `${launcher_name}`.
const LAUNCHER_NAME: &str = "Ferrolys";

/// Разрешённые плейсхолдеры: `${key}` → значение.
type Placeholders = BTreeMap<&'static str, String>;

/// Оффлайн-контекст авторизации (§7.4 подставит настоящий).
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub player_name: String,
    /// UUID с дефисами.
    pub uuid: String,
    pub access_token: String,
    /// `"legacy"` (оффлайн) | `"ely"` (§7.4).
    pub user_type: String,
}

/// Входные данные для [`build_command`].
#[derive(Debug, Clone)]
pub struct LaunchInput<'a> {
    pub paths: &'a FerrolysPaths,
    pub vj: &'a VersionJson,
    pub instance_id: &'a str,
    pub auth: &'a AuthContext,
    /// Путь к java; §7.3 детектит рантайм, пока — `"java"`.
    pub java: &'a Path,
    pub xmx_mb: u32,
    /// Значение `${version_type}` (`vj.kind.as_str()`).
    pub version_type: &'a str,
}

/// Готовая команда запуска: без shell, аргументы — отдельные элементы.
#[derive(Debug, Clone)]
pub struct CommandSpec {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

/// Classpath: артефакты разрешённых библиотек в порядке манифеста,
/// склеенные `:`. Ошибка maven-координат — `warn` + пропуск (как в
/// `library_tasks`).
fn classpath(libraries_dir: &Path, vj: &VersionJson) -> String {
    let mut entries = Vec::new();
    for library in &vj.libraries {
        if !library_allowed(library) || library.artifact.is_none() {
            continue;
        }
        match maven_path(&library.name) {
            Ok(relative) => entries.push(libraries_dir.join(relative).display().to_string()),
            Err(err) => {
                tracing::warn!(name = %library.name, %err, "cannot build maven path");
            }
        }
    }
    entries.join(":")
}

/// Таблица подстановок для одного запуска.
fn placeholders(input: &LaunchInput<'_>, classpath: &str) -> Placeholders {
    let instance_dir = input.paths.instances_dir().join(input.instance_id);
    let natives_dir = instance_dir
        .join("versions")
        .join(&input.vj.id)
        .join("natives");

    let mut map: Placeholders = BTreeMap::new();
    map.insert("auth_player_name", input.auth.player_name.clone());
    map.insert("auth_uuid", input.auth.uuid.clone());
    map.insert("auth_access_token", input.auth.access_token.clone());
    map.insert("user_type", input.auth.user_type.clone());
    map.insert("user_properties", "{}".to_string());
    map.insert("game_directory", instance_dir.display().to_string());
    map.insert(
        "assets_root",
        input.paths.assets_dir().display().to_string(),
    );
    map.insert("assets_index_name", input.vj.asset_index.id.clone());
    map.insert("version_name", input.vj.id.clone());
    map.insert("version_type", input.version_type.to_string());
    map.insert("classpath", classpath.to_string());
    map.insert("natives_directory", natives_dir.display().to_string());
    map.insert("launcher_name", LAUNCHER_NAME.to_string());
    map.insert("launcher_version", APP_VERSION.to_string());
    // Оффлайн-заглушка до §7.4.
    map.insert("clientid", String::new());
    map.insert("auth_xuid", String::new());
    map.insert("resolution_width", "854".to_string());
    map.insert("resolution_height", "480".to_string());
    map
}

/// Заменить все `${key}` в аргументе. Неизвестный ключ → `""` + `warn`;
/// незакрытая скобка `${key` остаётся как есть.
fn substitute(arg: &str, map: &Placeholders) -> String {
    let mut out = String::with_capacity(arg.len());
    let mut rest = arg;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else {
            out.push_str("${");
            rest = after;
            break;
        };
        let key = &after[..end];
        match map.get(key) {
            Some(value) => out.push_str(value),
            None => tracing::warn!(key, "unknown launch placeholder"),
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

/// Собрать команду запуска. Инфаллибельно: ошибки координат деградируют в
/// `warn` и пропуск, неизвестные плейсхолдеры — в пустую строку.
pub fn build_command(input: &LaunchInput<'_>) -> CommandSpec {
    let instance_dir = input.paths.instances_dir().join(input.instance_id);
    let libraries_dir = instance_dir.join("libraries");
    let classpath = classpath(&libraries_dir, input.vj);
    let map = placeholders(input, &classpath);

    let mut args = vec![format!("-Xmx{}M", input.xmx_mb)];

    // log4j-конфиг: `${path}` → локальный скачанный файл (§7.1 ставит его
    // в assets/log_configs). Подставляется до общей замены: в общей карте
    // ключа `${path}` намеренно нет.
    if let Some(logging) = &input.vj.logging_client {
        let local = input
            .paths
            .assets_dir()
            .join("log_configs")
            .join(&logging.file_id);
        args.push(
            logging
                .argument
                .replace("${path}", &local.display().to_string()),
        );
    }

    // JVM-аргументы манифеста; у старых версий (1.7.10) секции нет —
    // дефолты лаунчера, без которых игра не стартует.
    let jvm: Vec<String> = if input.vj.jvm_arguments.is_empty() {
        vec![
            "-Djava.library.path=${natives_directory}".to_string(),
            "-cp".to_string(),
            "${classpath}".to_string(),
        ]
    } else {
        input.vj.jvm_arguments.clone()
    };
    args.extend(jvm.into_iter().map(|arg| substitute(&arg, &map)));

    args.push(input.vj.main_class.clone());
    args.extend(input.vj.arguments.iter().map(|arg| substitute(arg, &map)));

    CommandSpec {
        program: input.java.to_path_buf(),
        args,
        cwd: instance_dir,
    }
}
