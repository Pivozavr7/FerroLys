//! Детект краша по строкам вывода игры (план §7.2, Task 4).

/// Паттерны краша: маркер ванильного лаунчера, исключение главного потока
/// и фатальная ошибка JVM (hs_err не содержит "Game crashed!").
const CRASH_PATTERNS: [&str; 3] = [
    "Game crashed!",
    r#"Exception in thread "main""#,
    "fatal error has been detected by the Java Runtime Environment",
];

/// Первая строка, содержащая один из паттернов краша, — сама строка
/// (для краш-диалога). `None` — краша в срезе нет.
pub fn find_crash_marker(lines: &[String]) -> Option<&str> {
    lines
        .iter()
        .find(|line| CRASH_PATTERNS.iter().any(|pattern| line.contains(pattern)))
        .map(String::as_str)
}
