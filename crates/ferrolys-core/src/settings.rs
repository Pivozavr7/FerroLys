//! Настройки лаунчера: `~/.config/ferrolys/settings.toml` (план §4.3, §5.5).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::paths::FerrolysPaths;
use crate::theme::DEFAULT_ACCENT_SEED;

/// Поддерживаемые языки UI (RU + EN с первого дня).
pub const LOCALES: &[&str] = &["en", "ru"];

/// Настройки приложения. Неизвестные поля из будущих версий игнорируются,
/// отсутствующие поля получают значения по умолчанию (`#[serde(default)]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Код локали из [`LOCALES`]: `"en"` | `"ru"`.
    pub locale: String,
    /// Seed акцентного цвета, 0xRRGGBB (см. [`crate::theme`]).
    pub accent_seed: u32,
    /// Семейство шрифта UI: `"Inter"` (встроенный) или системное.
    pub font_family: String,
    /// Подробное логирование (уровень debug).
    pub verbose_logging: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            locale: guess_locale(),
            accent_seed: DEFAULT_ACCENT_SEED,
            font_family: crate::fonts::EMBEDDED_DEFAULT.to_string(),
            verbose_logging: false,
        }
    }
}

impl Settings {
    /// Загрузить настройки; при любой ошибке — значения по умолчанию
    /// (лаунчер никогда не должен не стартовать из-за битого файла).
    pub fn load(paths: &FerrolysPaths) -> Self {
        Self::load_from(&paths.settings_file())
    }

    /// Загрузка из конкретного файла (используется и в тестах).
    pub fn load_from(file: &Path) -> Self {
        let mut settings = std::fs::read_to_string(file)
            .ok()
            .and_then(|raw| toml::from_str::<Settings>(&raw).ok())
            .unwrap_or_default();
        settings.sanitize();
        settings
    }

    /// Сохранить настройки (атомарно: tmp + rename). Ошибка не фатальна —
    /// вызывающий логирует её.
    pub fn save(&self, paths: &FerrolysPaths) -> Result<(), CoreError> {
        self.save_to(&paths.settings_file())
    }

    /// Сохранение в конкретный файл (используется и в тестах).
    pub fn save_to(&self, file: &Path) -> Result<(), CoreError> {
        let dir = file.parent().unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(dir)?;
        let raw = toml::to_string_pretty(self)?;
        let tmp = file.with_extension("toml.tmp");
        std::fs::write(&tmp, raw)?;
        std::fs::rename(&tmp, file)?;
        Ok(())
    }

    /// Привести значения из файла к поддерживаемым (миграции/битые записи).
    pub fn sanitize(&mut self) {
        if !LOCALES.contains(&self.locale.as_str()) {
            self.locale = guess_locale();
        }
        if self.font_family.trim().is_empty() {
            self.font_family = crate::fonts::EMBEDDED_DEFAULT.to_string();
        }
        // Акцент — только из пресетов, чтобы не потерять совместимость.
        if !crate::theme::ACCENT_PRESETS
            .iter()
            .any(|p| p.seed == self.accent_seed)
        {
            self.accent_seed = DEFAULT_ACCENT_SEED;
        }
    }
}

/// Определить локаль из окружения (`LC_ALL` → `LC_MESSAGES` → `LANG`).
/// Поддерживается только русский/английский; по умолчанию — English.
pub fn guess_locale() -> String {
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(value) = std::env::var(key) {
            let lower = value.to_lowercase();
            if lower.starts_with("ru") {
                return "ru".to_string();
            }
            if lower.starts_with("en") {
                return "en".to_string();
            }
        }
    }
    "en".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::load_from(&dir.path().join("nope.toml"));
        assert!(LOCALES.contains(&settings.locale.as_str()));
        assert_eq!(settings.font_family, "Inter");
        assert_eq!(settings.accent_seed, DEFAULT_ACCENT_SEED);
        assert!(!settings.verbose_logging);
    }

    #[test]
    fn roundtrip_preserves_values() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.toml");
        let settings = Settings {
            locale: "ru".into(),
            accent_seed: 0x0061A4,
            font_family: "Google Sans".into(),
            verbose_logging: true,
        };
        settings.save_to(&file).unwrap();

        let loaded = Settings::load_from(&file);
        assert_eq!(loaded, settings);
    }

    #[test]
    fn corrupt_file_falls_back_to_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.toml");
        std::fs::write(&file, "locale = [this is not toml").unwrap();
        let loaded = Settings::load_from(&file);
        assert_eq!(loaded, Settings::default());
    }

    #[test]
    fn sanitize_fixes_unknown_locale_and_font() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.toml");
        std::fs::write(&file, "locale = \"de\"\nfont_family = \"\"\n").unwrap();
        let loaded = Settings::load_from(&file);
        assert!(LOCALES.contains(&loaded.locale.as_str()));
        assert_eq!(loaded.font_family, "Inter");
    }

    #[test]
    fn sanitize_resets_unknown_accent_seed() {
        let mut settings = Settings {
            accent_seed: 0xDEAD00,
            ..Settings::default()
        };
        settings.sanitize();
        assert_eq!(settings.accent_seed, DEFAULT_ACCENT_SEED);
    }

    #[test]
    fn unknown_fields_from_future_versions_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.toml");
        std::fs::write(&file, "locale = \"ru\"\nsome_future_field = 42\n").unwrap();
        let loaded = Settings::load_from(&file);
        assert_eq!(loaded.locale, "ru");
    }
}
