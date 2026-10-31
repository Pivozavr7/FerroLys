//! Сборка структур для `.slint`: строки (i18n) и цвета (material-colors).

use ferrolys_core::fonts;
use ferrolys_core::i18n::tr;
use ferrolys_core::theme;
use slint::{Color, ModelRc, VecModel};

use crate::{AccentSwatch, LocaleOption, ThemeColors, UiStrings};

/// Все строки UI в текущей локали (вызывается при старте и при смене языка).
pub fn ui_strings() -> UiStrings {
    UiStrings {
        app: crate::AppStrings {
            title: tr("app.title").into(),
            tagline: tr("app.tagline").into(),
        },
        nav: crate::NavStrings {
            profiles: tr("nav.profiles").into(),
            accounts: tr("nav.accounts").into(),
            settings: tr("nav.settings").into(),
        },
        common: crate::CommonStrings {
            cancel: tr("common.cancel").into(),
            close: tr("common.close").into(),
            back: tr("common.back").into(),
            open_log_folder: tr("common.open-log-folder").into(),
            data_folder: tr("common.data-folder").into(),
            loading: tr("common.loading").into(),
        },
        profiles: crate::ProfilesStrings {
            title: tr("profiles.title").into(),
            empty_title: tr("profiles.empty-title").into(),
            empty_body: tr("profiles.empty-body").into(),
        },
        accounts: crate::AccountsStrings {
            title: tr("accounts.title").into(),
            empty_title: tr("accounts.empty-title").into(),
            empty_body: tr("accounts.empty-body").into(),
        },
        settings: crate::SettingsStrings {
            title: tr("settings.title").into(),
            appearance: tr("settings.appearance").into(),
            language: tr("settings.language").into(),
            accent: tr("settings.accent").into(),
            font: tr("settings.font").into(),
            network: tr("settings.network").into(),
            verbose_logging: tr("settings.verbose-logging").into(),
            paths: tr("settings.paths").into(),
            about: tr("settings.about").into(),
            version: tr("settings.version").into(),
            path_data: tr("settings.path-data").into(),
            path_config: tr("settings.path-config").into(),
            path_logs: tr("settings.path-logs").into(),
        },
        demo: crate::DemoStrings {
            title: tr("demo.title").into(),
            hint: tr("demo.hint").into(),
            run: tr("demo.run").into(),
            running: tr("demo.running").into(),
        },
        crash: crate::CrashStrings {
            title: tr("crash.title").into(),
            body: tr("crash.body").into(),
        },
    }
}

/// Цвета тёмной MD3-схемы для seed-акцента.
pub fn theme_colors(seed: u32) -> ThemeColors {
    let s = theme::dark_scheme(seed);
    ThemeColors {
        primary: rgb(s.primary),
        on_primary: rgb(s.on_primary),
        primary_container: rgb(s.primary_container),
        on_primary_container: rgb(s.on_primary_container),
        inverse_primary: rgb(s.inverse_primary),
        secondary: rgb(s.secondary),
        on_secondary: rgb(s.on_secondary),
        secondary_container: rgb(s.secondary_container),
        on_secondary_container: rgb(s.on_secondary_container),
        tertiary: rgb(s.tertiary),
        on_tertiary: rgb(s.on_tertiary),
        tertiary_container: rgb(s.tertiary_container),
        on_tertiary_container: rgb(s.on_tertiary_container),
        error: rgb(s.error),
        on_error: rgb(s.on_error),
        error_container: rgb(s.error_container),
        on_error_container: rgb(s.on_error_container),
        surface_dim: rgb(s.surface_dim),
        surface: rgb(s.surface),
        surface_bright: rgb(s.surface_bright),
        surface_container_lowest: rgb(s.surface_container_lowest),
        surface_container_low: rgb(s.surface_container_low),
        surface_container: rgb(s.surface_container),
        surface_container_high: rgb(s.surface_container_high),
        surface_container_highest: rgb(s.surface_container_highest),
        on_surface: rgb(s.on_surface),
        on_surface_variant: rgb(s.on_surface_variant),
        outline: rgb(s.outline),
        outline_variant: rgb(s.outline_variant),
        inverse_surface: rgb(s.inverse_surface),
        inverse_on_surface: rgb(s.inverse_on_surface),
        scrim: rgb(s.scrim),
    }
}

fn rgb(value: u32) -> Color {
    Color::from_rgb_u8((value >> 16) as u8, (value >> 8) as u8, value as u8)
}

/// Опции локалей: код + самоназвание (не переводится — «English»/«Русский»).
pub fn locale_options() -> ModelRc<LocaleOption> {
    let options = vec![
        LocaleOption {
            code: "en".into(),
            name: "English".into(),
        },
        LocaleOption {
            code: "ru".into(),
            name: "Русский".into(),
        },
    ];
    ModelRc::new(VecModel::from(options))
}

/// Индекс локали в опциях (для current-locale).
pub fn locale_index(code: &str) -> i32 {
    match code {
        "ru" => 1,
        _ => 0,
    }
}

/// Код локали по индексу опций.
pub fn locale_code(index: i32) -> String {
    match index {
        1 => "ru".to_string(),
        _ => "en".to_string(),
    }
}

/// Пресеты акцента как кружки для настроек.
pub fn accent_swatches() -> ModelRc<AccentSwatch> {
    let swatches = theme::ACCENT_PRESETS
        .iter()
        .map(|preset| AccentSwatch {
            name: preset.id.into(),
            color: rgb(preset.seed),
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(swatches))
}

/// Индекс пресета акцента по seed (для current-accent).
pub fn accent_index(seed: u32) -> i32 {
    theme::ACCENT_PRESETS
        .iter()
        .position(|preset| preset.seed == seed)
        .unwrap_or(0) as i32
}

/// Seed пресета по индексу (или seed по умолчанию).
pub fn accent_seed(index: i32) -> u32 {
    theme::ACCENT_PRESETS
        .get(index as usize)
        .map(|preset| preset.seed)
        .unwrap_or(theme::DEFAULT_ACCENT_SEED)
}

/// Доступные шрифты: встроенный Inter + найденные системные.
pub fn font_choices() -> ModelRc<slint::SharedString> {
    let choices = fonts::available_ui_fonts()
        .into_iter()
        .map(slint::SharedString::from)
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(choices))
}
