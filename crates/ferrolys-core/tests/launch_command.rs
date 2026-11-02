//! Тесты генератора команды запуска (план §7.2, Task 1).

use std::fs;
use std::path::{Path, PathBuf};

use ferrolys_core::install::maven::maven_path;
use ferrolys_core::install::version_json::{library_allowed, parse_version_json, VersionJson};
use ferrolys_core::launch::{build_command, AuthContext, LaunchInput};
use ferrolys_core::paths::FerrolysPaths;

fn fixture(version: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/versions")
        .join(format!("{version}.json"));
    fs::read(path).expect("fixture must exist")
}

fn parse_fixture(version: &str) -> VersionJson {
    parse_version_json(&fixture(version)).expect("fixture must parse")
}

fn auth() -> AuthContext {
    AuthContext {
        player_name: "Steve".into(),
        uuid: "069a79f4-44e9-4726-a5be-fca90e38aaf5".into(),
        access_token: "token".into(),
        user_type: "legacy".into(),
    }
}

fn input<'a>(
    paths: &'a FerrolysPaths,
    vj: &'a VersionJson,
    java: &'a Path,
    auth: &'a AuthContext,
) -> LaunchInput<'a> {
    LaunchInput {
        paths,
        vj,
        instance_id: "main",
        auth,
        java,
        xmx_mb: 2048,
        version_type: "release",
    }
}

fn arg_after<'a>(args: &'a [String], needle: &str) -> Option<&'a str> {
    args.iter()
        .position(|arg| arg == needle)
        .and_then(|idx| args.get(idx + 1))
        .map(String::as_str)
}

#[test]
fn classpath_joins_allowed_artifacts_with_colon() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let paths = FerrolysPaths::with_home(tmp.path());
    let vj = parse_fixture("1.21.1");
    let spec = build_command(&input(&paths, &vj, Path::new("java"), &auth()));

    let classpath = arg_after(&spec.args, "-cp").expect("-cp argument must exist");
    let libraries_dir = paths.instances_dir().join("main").join("libraries");
    let expected: Vec<String> = vj
        .libraries
        .iter()
        .filter(|lib| library_allowed(lib) && lib.artifact.is_some())
        .filter_map(|lib| {
            maven_path(&lib.name)
                .ok()
                .map(|relative| libraries_dir.join(relative).display().to_string())
        })
        .collect();

    assert!(!expected.is_empty(), "fixture must have allowed libraries");
    assert_eq!(classpath, expected.join(":"));
    assert!(classpath.ends_with(".jar"));
}

#[test]
fn legacy_version_gets_default_jvm_args() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let paths = FerrolysPaths::with_home(tmp.path());
    let vj = parse_fixture("1.7.10");
    assert!(
        vj.jvm_arguments.is_empty(),
        "1.7.10 has no arguments.jvm section"
    );

    let spec = build_command(&input(&paths, &vj, Path::new("java"), &auth()));

    let natives = paths
        .instances_dir()
        .join("main")
        .join("versions")
        .join(&vj.id)
        .join("natives");
    let library_path = arg_after(&spec.args, "-Djava.library.path")
        .map(str::to_string)
        .or_else(|| {
            spec.args
                .iter()
                .find(|arg| arg.starts_with("-Djava.library.path="))
                .map(|arg| arg.trim_start_matches("-Djava.library.path=").to_string())
        })
        .expect("java.library.path must be set");
    assert_eq!(library_path, natives.display().to_string());

    let classpath = arg_after(&spec.args, "-cp").expect("-cp must be present");
    assert!(!classpath.is_empty());
}

#[test]
fn logging_argument_uses_local_config_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let paths = FerrolysPaths::with_home(tmp.path());
    let vj = parse_fixture("1.21.1");
    let spec = build_command(&input(&paths, &vj, Path::new("java"), &auth()));

    let file_id = vj
        .logging_client
        .as_ref()
        .map(|logging| logging.file_id.clone())
        .expect("1.21.1 has logging.client");
    let expected = paths
        .assets_dir()
        .join("log_configs")
        .join(&file_id)
        .display()
        .to_string();

    assert!(
        spec.args
            .iter()
            .any(|arg| arg == &format!("-Dlog4j.configurationFile={expected}")),
        "expected log4j argument pointing at {expected}, got {:?}",
        spec.args
    );
    assert!(!spec.args.iter().any(|arg| arg.contains("${path}")));
}

#[test]
fn no_placeholders_remain_in_argv() {
    for version in ["1.7.10", "1.12.2", "1.20.1", "1.21.1"] {
        let tmp = tempfile::tempdir().expect("tempdir");
        let paths = FerrolysPaths::with_home(tmp.path());
        let vj = parse_fixture(version);
        let spec = build_command(&input(&paths, &vj, Path::new("java"), &auth()));

        let leftovers: Vec<&str> = spec
            .args
            .iter()
            .filter(|arg| arg.contains("${"))
            .map(String::as_str)
            .collect();
        assert!(
            leftovers.is_empty(),
            "{version}: unsubstituted placeholders: {leftovers:?}"
        );
    }
}

#[test]
fn xmx_and_main_class_positions() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let paths = FerrolysPaths::with_home(tmp.path());
    let vj = parse_fixture("1.21.1");
    let spec = build_command(&input(&paths, &vj, Path::new("java"), &auth()));

    assert_eq!(spec.args.first().map(String::as_str), Some("-Xmx2048M"));

    let main_pos = spec
        .args
        .iter()
        .position(|arg| arg == &vj.main_class)
        .expect("main class must be in argv");
    assert_eq!(
        spec.args.get(main_pos + 1).map(String::as_str),
        Some("--username")
    );
}
