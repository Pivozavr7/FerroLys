//! Запуск игры (план §7.2): сборка команды, спавн процесса, детект краша.

pub mod command;
pub mod process;

pub use command::{build_command, AuthContext, CommandSpec, LaunchInput};
pub use process::{run_game, GameEvent};
