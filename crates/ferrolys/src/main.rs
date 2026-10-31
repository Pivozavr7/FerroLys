//! Ferrolys — точка входа (волна 0): bootstrap, i18n/тема, async-мост,
//! разводка callback'ов UI ⇄ core, panic hook → краш-диалог.

slint::include_modules!();

mod bridge;
mod ui;

use std::sync::{Arc, Mutex, OnceLock};

use anyhow::Result;
use slint::{Global, Weak};

use ferrolys_core::fonts;
use ferrolys_core::i18n;
use ferrolys_core::logging;
use ferrolys_core::{FerrolysPaths, Settings, APP_VERSION};

/// Weak-хэндл окна для показа краш-диалога из panic hook.
static UI_WEAK: OnceLock<Mutex<Weak<AppWindow>>> = OnceLock::new();

fn main() -> Result<()> {
    // — Пути и настройки (до логирования: уровень берётся из настроек) —
    let paths = FerrolysPaths::discover();
    let mut settings = Settings::load(&paths);

    // Шрифт из настроек мог пропасть из системы — валидируем.
    if !fonts::is_ui_font_available(&settings.font_family) {
        settings.font_family = fonts::EMBEDDED_DEFAULT.to_string();
    }

    let _logging = logging::init(settings.verbose_logging);
    i18n::set_locale(&settings.locale);
    install_panic_hook(&paths);

    tracing::info!(
        version = APP_VERSION,
        locale = %i18n::current_locale(),
        accent_seed = format_args!("0x{:06X}", settings.accent_seed),
        "boot"
    );

    // — Один глобальный tokio-рантайм (план §4.2) —
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let ui = AppWindow::new()?;

    // — Токены темы и строки (живое переключение — из callback'ов ниже) —
    Theme::get(&ui).set_colors(ui::theme_colors(settings.accent_seed));
    Theme::get(&ui).set_font_family(settings.font_family.clone().into());
    Tr::get(&ui).set_s(ui::ui_strings());

    // — Разделяемое состояние настроек для callback'ов —
    let settings = Arc::new(Mutex::new(settings));
    {
        let guard = settings.lock().unwrap();

        ui.set_locale_options(ui::locale_options());
        ui.set_current_locale(ui::locale_index(&guard.locale));
        ui.set_accent_swatches(ui::accent_swatches());
        ui.set_current_accent(ui::accent_index(guard.accent_seed));
        ui.set_font_choices(ui::font_choices());
        ui.set_current_font(guard.font_family.clone().into());
        ui.set_verbose_logging(guard.verbose_logging);
    }
    ui.set_path_data(paths.instances_dir().display().to_string().into());
    ui.set_path_config(paths.config_dir().display().to_string().into());
    ui.set_path_logs(paths.log_dir().display().to_string().into());
    ui.set_version(APP_VERSION.into());

    // — Смена языка: словарь + пересборка строк, на лету —
    let weak = ui.as_weak();
    let shared = settings.clone();
    let save_paths = paths.clone();
    ui.on_locale_selected(move |index| {
        let Some(ui) = weak.upgrade() else { return };
        let code = ui::locale_code(index);
        i18n::set_locale(&code);
        ui.set_current_locale(index);
        Tr::get(&ui).set_s(ui::ui_strings());
        let mut guard = shared.lock().unwrap();
        guard.locale = code;
        persist(&guard, &save_paths);
    });

    // — Смена акцента: пересчёт MD3-схемы из seed —
    let weak = ui.as_weak();
    let shared = settings.clone();
    let save_paths = paths.clone();
    ui.on_accent_selected(move |index| {
        let Some(ui) = weak.upgrade() else { return };
        let seed = ui::accent_seed(index);
        Theme::get(&ui).set_colors(ui::theme_colors(seed));
        ui.set_current_accent(index);
        let mut guard = shared.lock().unwrap();
        guard.accent_seed = seed;
        persist(&guard, &save_paths);
    });

    // — Смена шрифта —
    let weak = ui.as_weak();
    let shared = settings.clone();
    let save_paths = paths.clone();
    ui.on_font_selected(move |family| {
        let Some(ui) = weak.upgrade() else { return };
        if !fonts::is_ui_font_available(&family) {
            return;
        }
        Theme::get(&ui).set_font_family(family.clone());
        ui.set_current_font(family.clone());
        let mut guard = shared.lock().unwrap();
        guard.font_family = family.to_string();
        persist(&guard, &save_paths);
    });

    // — Подробное логирование (tracing reload на лету) —
    let weak = ui.as_weak();
    let shared = settings.clone();
    let save_paths = paths.clone();
    ui.on_verbose_toggled(move |verbose| {
        let Some(ui) = weak.upgrade() else { return };
        logging::set_verbose(verbose);
        ui.set_verbose_logging(verbose);
        let mut guard = shared.lock().unwrap();
        guard.verbose_logging = verbose;
        persist(&guard, &save_paths);
    });

    // — Демо async-моста —
    let weak = ui.as_weak();
    let handle = rt.handle().clone();
    ui.on_run_demo(move || {
        bridge::start(&handle, weak.clone());
    });

    // Для panic hook.
    let _ = UI_WEAK.set(Mutex::new(ui.as_weak()));

    ui.run()?;
    drop(rt);
    Ok(())
}

fn persist(settings: &Settings, paths: &FerrolysPaths) {
    if let Err(err) = settings.save(paths) {
        tracing::error!(%err, "failed to save settings");
    }
}

/// Panic hook: лог + файл panic.log + попытка показать краш-диалог в UI.
fn install_panic_hook(paths: &FerrolysPaths) {
    let panic_log = paths.log_dir().join("panic.log");
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let message = payload
            .downcast_ref::<&str>()
            .map(|text| (*text).to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown panic".to_string());
        let location = info
            .location()
            .map(|loc| loc.to_string())
            .unwrap_or_else(|| "<unknown location>".to_string());
        let backtrace = std::backtrace::Backtrace::force_capture();
        let report = format!("panic at {location}: {message}\n\n{backtrace}");

        tracing::error!(panic = %message, %location, "panic in Ferrolys");
        eprintln!("{report}");

        if let Some(parent) = panic_log.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&panic_log, &report);

        // Показать диалог, если окно ещё живо (best effort).
        let weak = UI_WEAK
            .get()
            .and_then(|cell| cell.lock().ok())
            .map(|guard| guard.clone());
        if let Some(weak) = weak {
            let _ = weak.upgrade_in_event_loop(move |ui| {
                ui.set_crash_message(message.into());
            });
        }
    }));
}
