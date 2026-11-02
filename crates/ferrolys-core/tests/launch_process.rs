//! Тесты спавна процесса игры (план §7.2, Task 3).

use std::path::PathBuf;

use ferrolys_core::error::CoreError;
use ferrolys_core::launch::{run_game, CommandSpec, GameEvent};
use tokio::sync::mpsc;

fn sh_spec(args: &[&str], cwd: PathBuf) -> CommandSpec {
    CommandSpec {
        program: PathBuf::from("/bin/sh"),
        args: args.iter().map(|arg| (*arg).to_string()).collect(),
        cwd,
    }
}

#[tokio::test]
async fn streams_stdout_and_stderr_and_reports_exit_code() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (tx, mut rx) = mpsc::unbounded_channel();

    let spec = sh_spec(
        &["-c", "echo out; echo err 1>&2; exit 3"],
        tmp.path().to_path_buf(),
    );
    let task = tokio::spawn(run_game(spec, tx));

    let mut lines = Vec::new();
    let mut code = None;
    while let Some(event) = rx.recv().await {
        match event {
            GameEvent::Line(line) => lines.push(line),
            GameEvent::Exited { code: exited } => {
                code = exited;
                break;
            }
        }
    }

    task.await
        .expect("run_game task must join")
        .expect("run_game must succeed");
    assert!(lines.iter().any(|line| line == "out"), "got {lines:?}");
    assert!(lines.iter().any(|line| line == "err"), "got {lines:?}");
    assert_eq!(code, Some(3));
}

#[tokio::test]
async fn missing_program_returns_io_error() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (tx, _rx) = mpsc::unbounded_channel();

    let spec = CommandSpec {
        program: PathBuf::from("/nonexistent/java-xyz"),
        args: Vec::new(),
        cwd: tmp.path().to_path_buf(),
    };

    let err = run_game(spec, tx)
        .await
        .expect_err("spawn of missing program must fail");
    assert!(matches!(err, CoreError::Io(_)), "got {err:?}");
}
