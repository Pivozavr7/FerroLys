//! Async-мост tokio ⇄ Slint (план §4.2, §6): демо-задача волны 0 —
//! скачивание манифеста версий Mojang с прогрессом.
//!
//! Принцип: UI не дёргается на каждый чанк — `watch`-канал + тик каждые
//! 100 мс обновляет property'и через `Weak::upgrade`.

use std::time::{Duration, Instant};

use futures_util::StreamExt;
use slint::Weak;
use tokio::sync::watch;

use ferrolys_core::i18n;

use crate::AppWindow;

/// Небольшой, но «настоящий» файл: официальный манифест версий Mojang.
const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

/// Как часто обновлять UI (план §4.2: не чаще, чем нужно).
const TICK: Duration = Duration::from_millis(100);

#[derive(Debug, Clone)]
enum State {
    Running {
        done: u64,
        total: u64,
        started: Instant,
    },
    Done {
        count: usize,
    },
    Error {
        message: String,
    },
}

/// Запустить демо-задачу (вызывается из callback кнопки).
pub fn start(handle: &tokio::runtime::Handle, weak: Weak<AppWindow>) {
    if let Some(ui) = weak.upgrade() {
        ui.set_demo_running(true);
        ui.set_demo_progress(0.0);
        ui.set_demo_indeterminate(true);
        ui.set_demo_indeterminate_pos(0.0);
        ui.set_demo_status(i18n::tr("demo.running").into());
        ui.set_demo_detail("".into());
    }

    let (tx, rx) = watch::channel(State::Running {
        done: 0,
        total: 0,
        started: Instant::now(),
    });
    handle.spawn(download(tx));
    handle.spawn(tick(rx, weak));
}

/// Задача загрузки: стримим байты, шлём прогресс в watch.
async fn download(tx: watch::Sender<State>) {
    let started = Instant::now();
    let outcome = run(&tx, started).await;
    let state = match outcome {
        Ok(count) => State::Done { count },
        Err(message) => State::Error { message },
    };
    let _ = tx.send(state);
}

async fn run(tx: &watch::Sender<State>, started: Instant) -> Result<usize, String> {
    let client = reqwest::Client::builder()
        .user_agent(ferrolys_core::APP_USER_AGENT)
        .build()
        .map_err(err_text)?;

    let response = client.get(MANIFEST_URL).send().await.map_err(err_text)?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    let total = response.content_length().unwrap_or(0);

    let mut stream = response.bytes_stream();
    let mut buffer: Vec<u8> = Vec::with_capacity(total as usize);
    let mut done: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(err_text)?;
        buffer.extend_from_slice(&chunk);
        done += chunk.len() as u64;
        let _ = tx.send_replace(State::Running {
            done,
            total,
            started,
        });
    }

    let value: serde_json::Value =
        serde_json::from_slice(&buffer).map_err(|err| format!("bad json: {err}"))?;
    Ok(value
        .get("versions")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len))
}

/// Тик каждые 100 мс: читает последнее состояние → обновляет UI.
async fn tick(mut rx: watch::Receiver<State>, weak: Weak<AppWindow>) {
    loop {
        tokio::time::sleep(TICK).await;
        let state = rx.borrow_and_update().clone();
        if apply(&weak, &state) {
            break;
        }
    }
}

/// Возвращает `true`, когда задача завершена (или окно закрыто).
fn apply(weak: &Weak<AppWindow>, state: &State) -> bool {
    let Some(ui) = weak.upgrade() else {
        return true; // окно закрыто
    };
    match state {
        State::Running {
            done,
            total,
            started,
        } => {
            let elapsed = started.elapsed().as_secs_f64().max(0.001);
            let speed = *done as f64 / elapsed;
            if *total > 0 {
                ui.set_demo_indeterminate(false);
                ui.set_demo_progress(*done as f32 / *total as f32);
                let eta = (*total).saturating_sub(*done) as f64 / speed.max(1.0);
                ui.set_demo_detail(
                    i18n::tr_speed_eta(&bytes_per_second(speed), &short_duration(eta)).into(),
                );
            } else {
                // Размер неизвестен — индетерминированный прогресс.
                ui.set_demo_indeterminate_pos(triangle(*started));
            }
            false
        }
        State::Done { count } => {
            ui.set_demo_running(false);
            ui.set_demo_indeterminate(false);
            ui.set_demo_progress(1.0);
            ui.set_demo_status(i18n::tr_count("demo.done", *count).into());
            ui.set_demo_detail("".into());
            true
        }
        State::Error { message } => {
            ui.set_demo_running(false);
            ui.set_demo_indeterminate(false);
            ui.set_demo_status(i18n::tr_error(message).into());
            ui.set_demo_detail("".into());
            true
        }
    }
}

/// Пилообразная позиция 0..1..0 для индетерминированного индикатора.
fn triangle(started: Instant) -> f32 {
    let phase = started.elapsed().as_secs_f64() / 1.4;
    let frac = phase - phase.floor();
    (if frac < 0.5 {
        frac * 2.0
    } else {
        2.0 - frac * 2.0
    }) as f32
}

fn bytes_per_second(speed: f64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    if speed >= MIB {
        format!("{:.1} MiB", speed / MIB)
    } else if speed >= KIB {
        format!("{:.1} KiB", speed / KIB)
    } else {
        format!("{speed:.0} B")
    }
}

fn short_duration(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    format!("{}:{:02}", total / 60, total % 60)
}

fn err_text(err: reqwest::Error) -> String {
    err.without_url().to_string()
}
