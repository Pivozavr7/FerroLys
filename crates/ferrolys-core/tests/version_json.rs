//! Тесты парсера version-JSON (план §7.1, Task 2).
//!
//! Фикстуры — реальные манифесты, скачаны с piston-meta и проверены по SHA-1
//! из `version_manifest_v2.json`.

use std::fs;

use ferrolys_core::install::version_json::{library_allowed, parse_version_json, Library};

fn fixture(version: &str) -> Vec<u8> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/versions")
        .join(format!("{version}.json"));
    fs::read(path).expect("fixture must exist")
}

fn parse_fixture(version: &str) -> ferrolys_core::install::version_json::VersionJson {
    parse_version_json(&fixture(version)).expect("fixture must parse")
}

fn allowed_count(vj: &ferrolys_core::install::version_json::VersionJson) -> usize {
    vj.libraries.iter().filter(|lib| library_allowed(lib)).count()
}

fn lib_from_json(json: &str) -> Library {
    serde_json::from_str(json).expect("test library must deserialize")
}

#[test]
fn parses_legacy_1_7_10() {
    let vj = parse_fixture("1.7.10");
    assert_eq!(vj.id, "1.7.10");
    assert_eq!(vj.main_class, "net.minecraft.client.main.Main");
    assert_eq!(vj.java_version, Some(8));
    // minecraftArguments разбит по пробелам, arguments-секции нет
    assert_eq!(vj.arguments.first().map(String::as_str), Some("--username"));
    assert!(vj.arguments.iter().any(|a| a == "${auth_player_name}"));
    assert!(vj.jvm_arguments.is_empty());
    // golden: 33 библиотеки, 31 разрешена на linux x86_64
    assert_eq!(vj.libraries.len(), 33);
    assert_eq!(allowed_count(&vj), 31);
    assert!(!vj
        .libraries
        .iter()
        .any(|l| l.name == "tv.twitch:twitch-platform:5.16" && library_allowed(l)));
    // natives через classifiers + extract excludes
    let natives = vj
        .libraries
        .iter()
        .find(|l| l.name == "org.lwjgl.lwjgl:lwjgl-platform:2.9.1")
        .expect("lwjgl-platform present");
    assert_eq!(natives.natives_linux.as_deref(), Some("natives-linux"));
    assert!(natives.extract_excludes.iter().any(|e| e.contains("META-INF")));
    assert!(natives
        .classifiers
        .iter()
        .any(|(name, art)| name == "natives-linux" && art.sha1.len() == 40));
    // assetIndex и client jar
    assert_eq!(vj.asset_index.id, "1.7.10");
    assert_eq!(vj.asset_index.sha1.len(), 40);
    let client = vj.downloads.client.as_ref().expect("client jar");
    assert!(client.url.contains("launcher.mojang.com"));
    // logging.client
    let logging = vj.logging_client.as_ref().expect("logging config");
    assert_eq!(logging.file_id, "client-1.7.xml");
    assert!(logging.argument.contains("${path}"));
}

#[test]
fn parses_modern_1_21() {
    let vj = parse_fixture("1.21.1");
    assert_eq!(vj.id, "1.21.1");
    assert_eq!(vj.java_version, Some(21));
    // arguments.game: только linux-применимые (объекты с osx/фичами отфильтрованы)
    assert_eq!(vj.arguments.len(), 22);
    assert!(vj.arguments.iter().any(|a| a == "--username"));
    assert!(!vj.arguments.iter().any(|a| a == "--demo"));
    // arguments.jvm: отфильтрованы osx-флаги вроде -XstartOnFirstThread
    assert_eq!(vj.jvm_arguments.len(), 8);
    assert!(vj
        .jvm_arguments
        .iter()
        .any(|a| a == "-Djava.library.path=${natives_directory}"));
    assert!(vj.jvm_arguments.iter().any(|a| a == "-cp"));
    assert!(!vj
        .jvm_arguments
        .iter()
        .any(|a| a.contains("XstartOnFirstThread")));
    // golden: 97 библиотек, 56 разрешено
    assert_eq!(vj.libraries.len(), 97);
    assert_eq!(allowed_count(&vj), 56);
    assert_eq!(vj.asset_index.id, "17");
    assert_eq!(
        vj.logging_client.as_ref().map(|l| l.file_id.as_str()),
        Some("client-1.12.xml")
    );
}

#[test]
fn parses_1_12_2() {
    let vj = parse_fixture("1.12.2");
    assert_eq!(vj.java_version, Some(8));
    assert_eq!(vj.arguments.first().map(String::as_str), Some("--username"));
    // golden: 39 библиотек, 34 (osx-вариант lwjgl 2.9.2 исключён)
    assert_eq!(vj.libraries.len(), 39);
    assert_eq!(allowed_count(&vj), 34);
    assert_eq!(vj.asset_index.id, "1.12");
}

#[test]
fn parses_1_20_1() {
    let vj = parse_fixture("1.20.1");
    assert_eq!(vj.java_version, Some(17));
    assert_eq!(vj.arguments.len(), 22);
    assert!(vj
        .jvm_arguments
        .iter()
        .any(|a| a == "-Djava.library.path=${natives_directory}"));
    // golden: 88 библиотек, 52
    assert_eq!(vj.libraries.len(), 88);
    assert_eq!(allowed_count(&vj), 52);
    assert_eq!(vj.asset_index.id, "5");
}

#[test]
fn rules_disallow_without_os_blocks_everything() {
    let lib = lib_from_json(r#"{"name":"a:b:1","rules":[{"action":"disallow"}]}"#);
    assert!(!library_allowed(&lib));
}

#[test]
fn rules_feature_mismatch_disallows() {
    let lib = lib_from_json(
        r#"{"name":"a:b:1","rules":[{"action":"allow","features":{"is_demo_user":true}}]}"#,
    );
    assert!(!library_allowed(&lib));
}

#[test]
fn missing_rules_means_allowed() {
    let lib = lib_from_json(r#"{"name":"a:b:1"}"#);
    assert!(library_allowed(&lib));
}

#[test]
fn last_matching_rule_wins() {
    // Паттерн twitch из 1.7.10: allow + disallow linux → на linux не ставим
    let linux_disallowed = lib_from_json(
        r#"{"name":"a:b:1","rules":[{"action":"allow"},{"action":"disallow","os":{"name":"linux"}}]}"#,
    );
    assert!(!library_allowed(&linux_disallowed));
    // allow + disallow osx → на linux ставим
    let osx_disallowed = lib_from_json(
        r#"{"name":"a:b:1","rules":[{"action":"allow"},{"action":"disallow","os":{"name":"osx"}}]}"#,
    );
    assert!(library_allowed(&osx_disallowed));
    // только allow windows → на linux не ставим
    let windows_only =
        lib_from_json(r#"{"name":"a:b:1","rules":[{"action":"allow","os":{"name":"windows"}}]}"#);
    assert!(!library_allowed(&windows_only));
}
