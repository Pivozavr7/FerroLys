//! Запуск игры (план §7.2): сборка команды, спавн процесса, детект краша.

pub mod command;

pub use command::{build_command, AuthContext, CommandSpec, LaunchInput};
