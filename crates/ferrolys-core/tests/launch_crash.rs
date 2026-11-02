//! Тесты детекта краша по строкам лога (план §7.2, Task 4).

use ferrolys_core::launch::find_crash_marker;

fn lines(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_string()).collect()
}

#[test]
fn matches_game_crashed_marker() {
    let captured = lines(&[
        "[12:00:00] [main/INFO]: Setting user: Steve",
        "Game crashed!",
        "Stopping!",
    ]);
    assert_eq!(find_crash_marker(&captured), Some("Game crashed!"));
}

#[test]
fn matches_exception_in_main_thread() {
    let captured = lines(&[
        "[12:00:00] [main/INFO]: Setting user: Steve",
        r#"Exception in thread "main" java.lang.NoClassDefFoundError: net/minecraft/client/main/Main"#,
    ]);
    assert_eq!(
        find_crash_marker(&captured),
        Some(
            r#"Exception in thread "main" java.lang.NoClassDefFoundError: net/minecraft/client/main/Main"#
        )
    );
}

#[test]
fn matches_jvm_fatal_error() {
    // JVM-краш (hs_err) не содержит "Game crashed!" — ловим и его.
    let captured = lines(&[
        "#",
        "# A fatal error has been detected by the Java Runtime Environment:",
        "#  SIGSEGV (0xb) at pc=0x00007f0000000000",
    ]);
    assert_eq!(
        find_crash_marker(&captured),
        Some("# A fatal error has been detected by the Java Runtime Environment:")
    );
}

#[test]
fn returns_first_matching_line() {
    let captured = lines(&["Game crashed!", r#"Exception in thread "main" X"#]);
    assert_eq!(find_crash_marker(&captured), Some("Game crashed!"));
}

#[test]
fn clean_log_has_no_marker() {
    let captured = lines(&[
        "[12:00:00] [main/INFO]: Setting user: Steve",
        "[12:00:01] [main/INFO]: Created: 1024x512 atlas",
    ]);
    assert_eq!(find_crash_marker(&captured), None);
}

#[test]
fn empty_log_has_no_marker() {
    assert_eq!(find_crash_marker(&[]), None);
}
