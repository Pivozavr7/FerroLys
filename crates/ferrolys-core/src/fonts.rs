//! Шрифты UI: встроенный Inter (OFL) + обнаружение кандидатов среди
//! системных шрифтов (план §5.4, решение по Google Sans: только как
//! системный шрифт пользователя — распространять его мы не можем).

use std::sync::OnceLock;

/// Шрифт, встроенный в бинарник (assets/fonts/). Всегда доступен.
pub const EMBEDDED_DEFAULT: &str = "Inter";

/// Кандидаты в настройках: встроенный + популярные системные гротески
/// с кириллицей. Проверяются по наличию в системе.
const CANDIDATES: &[&str] = &[
    EMBEDDED_DEFAULT,
    "Google Sans",
    "Roboto",
    "Golos Text",
    "Onest",
    "Noto Sans",
    "Open Sans",
    "DejaVu Sans",
    "Ubuntu",
    "Cantarell",
    "Fira Sans",
    "Manrope",
];

/// Доступные шрифты UI: встроенный Inter первым, затем найденные в системе.
/// Результат кэшируется (fontdb грузит системные шрифты на каждый вызов).
pub fn available_ui_fonts() -> Vec<String> {
    static CACHE: OnceLock<Vec<String>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let mut found = vec![EMBEDDED_DEFAULT.to_string()];
            let mut db = fontdb::Database::new();
            db.load_system_fonts();
            for candidate in CANDIDATES {
                if *candidate == EMBEDDED_DEFAULT {
                    continue;
                }
                let exists = db.faces().any(|face| {
                    face.families
                        .iter()
                        .any(|(name, _)| name.as_str() == *candidate)
                });
                if exists {
                    found.push((*candidate).to_string());
                }
            }
            found
        })
        .clone()
}

/// Есть ли шрифт в списке доступных (для валидации настроек).
pub fn is_ui_font_available(family: &str) -> bool {
    available_ui_fonts().iter().any(|f| f == family)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_font_is_always_available() {
        let fonts = available_ui_fonts();
        assert_eq!(fonts.first().map(String::as_str), Some(EMBEDDED_DEFAULT));
    }

    #[test]
    fn default_font_is_available() {
        assert!(is_ui_font_available(EMBEDDED_DEFAULT));
    }

    #[test]
    fn no_duplicates() {
        let fonts = available_ui_fonts();
        let unique: std::collections::HashSet<_> = fonts.iter().collect();
        assert_eq!(fonts.len(), unique.len());
    }
}
