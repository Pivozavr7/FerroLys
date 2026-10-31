//! Ferrolys core — доменная библиотека лаунчера без UI.
//!
//! Правило (план §4.1): этот крейт не знает про Slint. Любая логика, которую
//! можно покрыть тестами без компиляции интерфейса, живёт здесь.
//!
//! Модули волны 0:
//! - [`paths`]    — XDG-пути (план §4.3)
//! - [`settings`] — `settings.toml` (настройки, персистентные)
//! - [`i18n`]     — словарь RU/EN через rust-i18n
//! - [`logging`]  — tracing: stderr + ротация файлов, runtime-уровень
//! - [`theme`]    — тёмные MD3-палитры из seed-акцента (material-colors)
//! - [`fonts`]    — встроенный Inter + обнаружение системных шрифтов
//!
//! Файлы локалей — в `i18n/` корня воркспейса; инициализация rust-i18n —
//! строго в корне крейта (там генерируется хелпер для `t!`).
rust_i18n::i18n!("../../i18n", fallback = "en");

pub mod error;
pub mod fonts;
pub mod i18n;
pub mod logging;
pub mod paths;
pub mod settings;
pub mod theme;

pub use error::CoreError;
pub use paths::FerrolysPaths;
pub use settings::Settings;

/// Версия лаунчера (из Cargo, для «О программе» и заголовков HTTP).
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Имя пользователя-агента для прямых загрузок (Mojang и др.).
pub const APP_USER_AGENT: &str = concat!("Ferrolys/", env!("CARGO_PKG_VERSION"));
