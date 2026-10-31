//! i18n: словарь RU/EN через rust-i18n, файлы лежат в `i18n/` корня
//! воркспейса (план §6: «все строки — только через словарь с первого дня»).
//!
//! В `.slint` строки попадают структурой `UiStrings` (см. `ui/md3/i18n.slint`),
//! которую собирает бинарник. Здесь — только источник правды про ключи.

use rust_i18n::t;

use crate::error::CoreError;
use crate::settings::LOCALES;

/// Переключить глобальную локаль (валидируется по [`LOCALES`]).
pub fn set_locale_checked(locale: &str) -> Result<(), CoreError> {
    if !LOCALES.contains(&locale) {
        return Err(CoreError::UnsupportedLocale(locale.to_string()));
    }
    rust_i18n::set_locale(locale);
    Ok(())
}

/// Переключить локаль без проверки (нет-нот-пантик; неизвестное → fallback en).
pub fn set_locale(locale: &str) {
    if LOCALES.contains(&locale) {
        rust_i18n::set_locale(locale);
    } else {
        rust_i18n::set_locale("en");
    }
}

/// Текущая активная локаль.
pub fn current_locale() -> String {
    rust_i18n::locale().to_string()
}

/// Доступные локали UI.
pub fn available_locales() -> Vec<&'static str> {
    LOCALES.to_vec()
}

/// Перевод по ключу, например `"nav.profiles"`.
pub fn tr(key: &str) -> String {
    t!(key).to_string()
}

/// Перевод с одним числовым аргументом: `tr_count("demo.done", 123)`
/// подставит `%{count}`.
pub fn tr_count(key: &str, count: usize) -> String {
    t!(key, count = count).to_string()
}

/// Строка скорости/ETA демо-загрузки: `%{speed}`, `%{eta}`.
pub fn tr_speed_eta(speed: &str, eta: &str) -> String {
    t!("demo.speed-eta", speed = speed, eta = eta).to_string()
}

/// Строка ошибки загрузки: `%{error}`.
pub fn tr_error(message: &str) -> String {
    t!("demo.error", error = message).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locales_files_cover_en_and_ru() {
        set_locale("en");
        assert_eq!(tr("nav.profiles"), "Profiles");
        set_locale("ru");
        assert_eq!(tr("nav.profiles"), "Профили");
        set_locale("en");
    }

    #[test]
    fn interpolation_works() {
        set_locale("en");
        let line = tr_count("demo.done", 1234);
        assert!(line.contains("1234"), "got: {line}");
        set_locale("ru");
    }

    #[test]
    fn missing_key_falls_back_to_en() {
        set_locale("ru");
        let line = tr("definitely.absent.key");
        assert!(!line.is_empty());
        set_locale("en");
    }

    #[test]
    fn set_locale_checked_rejects_unknown() {
        assert!(set_locale_checked("de").is_err());
        assert!(set_locale_checked("ru").is_ok());
        set_locale("en");
    }
}
