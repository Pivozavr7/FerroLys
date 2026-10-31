//! Логирование: stderr + ротация файлов в `~/.local/state/ferrolys/log/`
//! (план §4.3). Уровень можно менять на лету — переключатель в настройках.

use std::sync::OnceLock;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{daily, RollingFileAppender};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter, Registry};

type FilterHandle = tracing_subscriber::reload::Handle<EnvFilter, Registry>;

static FILTER: OnceLock<FilterHandle> = OnceLock::new();

/// Держатель non-blocking writer'а: пока жив — файл пишется.
pub struct Logging {
    _guard: Option<WorkerGuard>,
}

fn build_filter(verbose: bool) -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        if verbose {
            EnvFilter::new("ferrolys=debug,info")
        } else {
            EnvFilter::new("ferrolys=info,warn")
        }
    })
}

/// Инициализировать логирование (вызывается один раз в main).
pub fn init(verbose: bool) -> Logging {
    let (filter, handle) = tracing_subscriber::reload::Layer::new(build_filter(verbose));
    let _ = FILTER.set(handle);

    let registry = tracing_subscriber::registry().with(filter);
    let stderr_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_target(true)
        .with_ansi(true);

    let log_dir = crate::paths::FerrolysPaths::discover().log_dir();
    match std::fs::create_dir_all(&log_dir).map(|_| file_appender(&log_dir)) {
        Ok(appender) => {
            let (writer, guard) = tracing_appender::non_blocking(appender);
            let file_layer = fmt::layer().with_writer(writer).with_ansi(false);
            registry.with(stderr_layer).with(file_layer).init();
            Logging {
                _guard: Some(guard),
            }
        }
        Err(_err) => {
            // Нет прав на каталог логов — работаем только с stderr.
            registry.with(stderr_layer).init();
            Logging { _guard: None }
        }
    }
}

fn file_appender(dir: &std::path::Path) -> RollingFileAppender {
    daily(dir, "ferrolys.log")
}

/// Переключить уровень логирования на лету (настройка «подробное логирование»).
pub fn set_verbose(verbose: bool) {
    if let Some(handle) = FILTER.get() {
        if let Err(err) = handle.reload(build_filter(verbose)) {
            eprintln!("failed to reload log filter: {err}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbose_filter_enables_debug_for_our_target() {
        let subscriber = tracing_subscriber::registry().with(build_filter(true));
        tracing::subscriber::with_default(subscriber, || {
            assert!(tracing::enabled!(target: "ferrolys", tracing::Level::DEBUG));
            assert!(tracing::enabled!(target: "ferrolys", tracing::Level::INFO));
        });
    }

    #[test]
    fn default_filter_hides_debug_for_our_target() {
        let subscriber = tracing_subscriber::registry().with(build_filter(false));
        tracing::subscriber::with_default(subscriber, || {
            assert!(!tracing::enabled!(target: "ferrolys", tracing::Level::DEBUG));
            assert!(tracing::enabled!(target: "ferrolys", tracing::Level::INFO));
            // Чужие цели не фильтруются нашим таргетом (логируются по умолчанию).
            assert!(tracing::enabled!(target: "other_crate", tracing::Level::WARN));
        });
    }
}
