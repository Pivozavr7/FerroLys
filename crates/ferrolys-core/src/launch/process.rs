//! Спавн процесса игры и live-вывод его строк (план §7.2, Task 3).
//!
//! Строки stdout и stderr читаются потоками внутри `spawn_blocking`
//! (`std::process`: фича `tokio/process` в воркспейсе не включена, а
//! Cargo.toml по правилу #2 не трогаем). `kill_on_drop` не ставится — игра
//! переживает закрытие лаунчера.

use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};

use tokio::sync::mpsc;

use crate::error::CoreError;
use crate::launch::CommandSpec;

/// Событие жизненного цикла игры.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameEvent {
    /// Одна строка stdout или stderr.
    Line(String),
    /// Процесс завершился; `code: None` — убит сигналом.
    Exited { code: Option<i32> },
}

/// Запустить игру и слать её вывод в канал до [`GameEvent::Exited`].
///
/// Блокирующий вызов живёт в `spawn_blocking`; ошибки чтения строк —
/// `warn` и остановка чтения этого потока, не фатал.
pub async fn run_game(
    spec: CommandSpec,
    tx: mpsc::UnboundedSender<GameEvent>,
) -> Result<(), CoreError> {
    match tokio::task::spawn_blocking(move || run_blocking(spec, tx)).await {
        Ok(result) => result,
        Err(err) => {
            tracing::error!(%err, "game task failed");
            Err(CoreError::Io(std::io::Error::other("game task failed")))
        }
    }
}

/// Дочерний процесс: два потока-насоса строк, затем статус выхода.
fn run_blocking(spec: CommandSpec, tx: mpsc::UnboundedSender<GameEvent>) -> Result<(), CoreError> {
    let mut child = Command::new(&spec.program)
        .args(&spec.args)
        .current_dir(&spec.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // С `Stdio::piped()` оба потока всегда Some; else-ветка — без паники.
    let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
        tracing::error!("piped stdio missing");
        let _ = child.kill();
        return Err(CoreError::Io(std::io::Error::other("piped stdio missing")));
    };

    let tx_out = tx.clone();
    let tx_err = tx.clone();
    let out_thread = std::thread::spawn(move || pump(stdout, tx_out));
    let err_thread = std::thread::spawn(move || pump(stderr, tx_err));

    let status = child.wait()?;
    for (name, thread) in [("stdout", out_thread), ("stderr", err_thread)] {
        if thread.join().is_err() {
            tracing::warn!(name, "game output reader panicked");
        }
    }

    let _ = tx.send(GameEvent::Exited {
        code: status.code(),
    });
    Ok(())
}

/// Читать строки до EOF и слать их в канал. Ресивер жив — продолжаем
/// читать, даже если слать некуда (иначе забьётся pipe и игра встанет).
fn pump<R: Read>(stream: R, tx: mpsc::UnboundedSender<GameEvent>) {
    for line in BufReader::new(stream).lines() {
        match line {
            Ok(line) => {
                let _ = tx.send(GameEvent::Line(line));
            }
            Err(err) => {
                tracing::warn!(%err, "game output line unreadable");
                break;
            }
        }
    }
}
