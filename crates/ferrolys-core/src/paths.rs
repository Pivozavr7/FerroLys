//! XDG-пути Ferrolys (план §4.3).
//!
//! | Что                    | Где                                        |
//! |------------------------|--------------------------------------------|
//! | Инстансы               | `$XDG_DATA_HOME/ferrolys/instances/`       |
//! | Общие assets           | `$XDG_DATA_HOME/ferrolys/assets/`          |
//! | Java-рантаймы          | `$XDG_DATA_HOME/ferrolys/runtimes/`        |
//! | Настройки              | `$XDG_CONFIG_HOME/ferrolys/settings.toml`  |
//! | HTTP-кэш               | `$XDG_CACHE_HOME/ferrolys/`                |
//! | Логи лаунчера          | `$XDG_STATE_HOME/ferrolys/log/`            |

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Разрешённые каталоги приложения.
///
/// Базовые каталоги (`$XDG_*` или fallback под home) вычисляются один раз
/// при создании — `with_home` намеренно **не читает окружение**, чтобы
/// тесты были изолированы от реального `XDG_*` пользователя.
#[derive(Debug, Clone)]
pub struct FerrolysPaths {
    config_base: PathBuf,
    data_base: PathBuf,
    cache_base: PathBuf,
    state_base: PathBuf,
}

impl FerrolysPaths {
    /// Определить пути из окружения (`XDG_*` + `$HOME`).
    pub fn discover() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        Self {
            config_base: resolve_base(std::env::var_os("XDG_CONFIG_HOME"), home.join(".config")),
            data_base: resolve_base(std::env::var_os("XDG_DATA_HOME"), home.join(".local/share")),
            cache_base: resolve_base(std::env::var_os("XDG_CACHE_HOME"), home.join(".cache")),
            state_base: resolve_base(
                std::env::var_os("XDG_STATE_HOME"),
                home.join(".local/state"),
            ),
        }
    }

    /// Конструктор для тестов: пути строятся строго относительно заданного
    /// home, реальное окружение `XDG_*` игнорируется.
    pub fn with_home(home: impl Into<PathBuf>) -> Self {
        let home = home.into();
        Self {
            config_base: home.join(".config"),
            data_base: home.join(".local/share"),
            cache_base: home.join(".cache"),
            state_base: home.join(".local/state"),
        }
    }

    /// `~/.config/ferrolys` — настройки.
    pub fn config_dir(&self) -> PathBuf {
        self.config_base.join("ferrolys")
    }

    /// `~/.local/share/ferrolys` — инстансы, assets, рантаймы.
    pub fn data_dir(&self) -> PathBuf {
        self.data_base.join("ferrolys")
    }

    /// `~/.cache/ferrolys` — HTTP-кэш.
    pub fn cache_dir(&self) -> PathBuf {
        self.cache_base.join("ferrolys")
    }

    /// `~/.local/state/ferrolys` — логи.
    pub fn state_dir(&self) -> PathBuf {
        self.state_base.join("ferrolys")
    }

    /// `…/instances/<id>/` — корень классической структуры `.minecraft`.
    pub fn instances_dir(&self) -> PathBuf {
        self.data_dir().join("instances")
    }

    /// Общие на все инстансы assets.
    pub fn assets_dir(&self) -> PathBuf {
        self.data_dir().join("assets")
    }

    /// Java-рантаймы (Temurin и др.).
    pub fn runtimes_dir(&self) -> PathBuf {
        self.data_dir().join("runtimes")
    }

    /// Файл настроек.
    pub fn settings_file(&self) -> PathBuf {
        self.config_dir().join("settings.toml")
    }

    /// Каталог логов лаунчера (ротация — в `logging`).
    pub fn log_dir(&self) -> PathBuf {
        self.state_dir().join("log")
    }
}

/// `$XDG_*`, если задан и не пуст, иначе fallback относительно home.
fn resolve_base(env_value: Option<OsString>, fallback: PathBuf) -> PathBuf {
    match env_value {
        Some(value) if !value.is_empty() => PathBuf::from(value),
        _ => fallback,
    }
}

/// Проверить, что путь лежит под base (для тестов структуры).
pub fn is_under(path: &Path, base: &Path) -> bool {
    path.starts_with(base)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_base_prefers_env() {
        let got = resolve_base(
            Some(OsString::from("/custom/xdg")),
            PathBuf::from("/home/u/.config"),
        );
        assert_eq!(got, PathBuf::from("/custom/xdg"));
    }

    #[test]
    fn resolve_base_ignores_empty_env() {
        let got = resolve_base(Some(OsString::new()), PathBuf::from("/home/u/.config"));
        assert_eq!(got, PathBuf::from("/home/u/.config"));
    }

    #[test]
    fn resolve_base_fallback_when_unset() {
        let got = resolve_base(None, PathBuf::from("/home/u/.local/share"));
        assert_eq!(got, PathBuf::from("/home/u/.local/share"));
    }

    #[test]
    fn layout_matches_plan() {
        let paths = FerrolysPaths::with_home("/home/tester");
        assert_eq!(
            paths.instances_dir(),
            Path::new("/home/tester/.local/share/ferrolys/instances")
        );
        assert_eq!(
            paths.assets_dir(),
            Path::new("/home/tester/.local/share/ferrolys/assets")
        );
        assert_eq!(
            paths.runtimes_dir(),
            Path::new("/home/tester/.local/share/ferrolys/runtimes")
        );
        assert_eq!(
            paths.settings_file(),
            Path::new("/home/tester/.config/ferrolys/settings.toml")
        );
        assert_eq!(paths.cache_dir(), Path::new("/home/tester/.cache/ferrolys"));
        assert_eq!(
            paths.log_dir(),
            Path::new("/home/tester/.local/state/ferrolys/log")
        );
    }
}
