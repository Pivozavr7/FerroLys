//! Тесты maven-координат и распаковки natives (план §7.1, Task 3).

use std::io::Write;
use std::path::PathBuf;

use ferrolys_core::install::maven::{expand_arch, extract_natives, maven_path};

#[test]
fn maven_path_plain() {
    assert_eq!(
        maven_path("com.mojang:brigadier:1.0.18").unwrap(),
        "com/mojang/brigadier/1.0.18/brigadier-1.0.18.jar"
    );
}

#[test]
fn maven_path_with_classifier() {
    assert_eq!(
        maven_path("org.lwjgl:lwjgl:3.3.3:natives-linux").unwrap(),
        "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-linux.jar"
    );
}

#[test]
fn maven_path_with_extension() {
    // 4-я часть после «:» — классификатор, расширение остаётся jar
    assert_eq!(
        maven_path("io.netty:netty-all:4.0.15.Final:shaded").unwrap(),
        "io/netty/netty-all/4.0.15.Final/netty-all-4.0.15.Final-shaded.jar"
    );
    // «@ext» переопределяет расширение
    assert_eq!(
        maven_path("group:artifact:1.0@zip").unwrap(),
        "group/artifact/1.0/artifact-1.0.zip"
    );
    // классификатор + @ext
    assert_eq!(
        maven_path("group:artifact:1.0:cls@zip").unwrap(),
        "group/artifact/1.0/artifact-1.0-cls.zip"
    );
}

#[test]
fn maven_path_invalid_returns_error() {
    assert!(maven_path("не-координаты").is_err());
    assert!(maven_path("a:b").is_err(), "нет версии — ошибка");
    assert!(maven_path("a::c").is_err(), "пустая часть — ошибка");
    assert!(maven_path("a:b:c@").is_err(), "пустое расширение — ошибка");
}

#[test]
fn expand_arch_substitutes_placeholder() {
    assert_eq!(expand_arch("${arch}"), "x86_64");
    assert_eq!(expand_arch("natives-linux"), "natives-linux");
}

/// Собрать zip во временном файле из пар (имя, содержимое).
fn make_zip(entries: &[(&str, &str)]) -> tempfile::TempPath {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut writer = zip::ZipWriter::new(file.reopen().unwrap());
    for (name, content) in entries {
        writer
            .start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(content.as_bytes()).unwrap();
    }
    writer.finish().unwrap();
    file.into_temp_path()
}

#[test]
fn extract_natives_skips_meta_inf_and_excludes() {
    let archive = make_zip(&[
        ("natives/libfoo.so", "ELF"),
        ("natives/META-INF/MANIFEST.MF", "manifest"),
        ("natives/META-INF/versions/9/module-info.class", "class"),
        ("other/keep.txt", "no"),
    ]);
    let dest = tempfile::tempdir().unwrap();
    let dest: PathBuf = dest.path().join("natives");

    let extracted = extract_natives(&archive, &dest, &["other/".to_string()]).unwrap();

    assert_eq!(
        extracted, 1,
        "только libfoo.so, META-INF и other/ пропущены"
    );
    assert!(dest.join("natives/libfoo.so").exists());
    assert!(!dest.join("other").exists());
    // dest создан даже если не существовал
    assert!(dest.exists());
}

#[test]
fn extract_natives_skips_zip_slip_entries() {
    let archive = make_zip(&[("../evil.txt", "escape"), ("good.so", "ELF")]);
    let dest = tempfile::tempdir().unwrap();

    let extracted = extract_natives(&archive, dest.path(), &[]).unwrap();

    assert_eq!(extracted, 1, "path-traversal записан быть не должен");
    assert!(!dest.path().parent().unwrap().join("evil.txt").exists());
    assert!(dest.path().join("good.so").exists());
}

#[test]
fn extract_natives_missing_archive_is_typed_error() {
    let dest = tempfile::tempdir().unwrap();
    assert!(extract_natives(
        PathBuf::from("/nonexistent/natives.jar").as_path(),
        dest.path(),
        &[]
    )
    .is_err());
}
