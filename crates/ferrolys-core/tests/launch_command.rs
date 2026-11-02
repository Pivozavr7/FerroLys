//! Тесты генератора команды запуска (план §7.2, Task 1–2): плейсхолдеры,
//! classpath, golden-argv по фикстурам.

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

// — Task 2: golden-тесты —

/// Независимый контекст подстановок: своя таблица и свой алгоритм
/// (построчная замена, а не сканер) — намеренно не из кода SUT.
struct Ctx<'a> {
    paths: &'a FerrolysPaths,
    vj: &'a VersionJson,
    auth: &'a AuthContext,
}

impl Ctx<'_> {
    fn instance_dir(&self) -> PathBuf {
        self.paths.instances_dir().join("main")
    }

    fn natives_dir(&self) -> PathBuf {
        self.instance_dir()
            .join("versions")
            .join(&self.vj.id)
            .join("natives")
    }

    fn classpath(&self) -> String {
        let libraries_dir = self.instance_dir().join("libraries");
        self.vj
            .libraries
            .iter()
            .filter(|lib| library_allowed(lib) && lib.artifact.is_some())
            .filter_map(|lib| {
                maven_path(&lib.name)
                    .ok()
                    .map(|relative| libraries_dir.join(relative).display().to_string())
            })
            .collect::<Vec<String>>()
            .join(":")
    }

    fn expand(&self, arg: &str) -> String {
        let table: [(&str, &str); 18] = [
            ("auth_player_name", &self.auth.player_name),
            ("auth_uuid", &self.auth.uuid),
            ("auth_access_token", &self.auth.access_token),
            ("user_type", &self.auth.user_type),
            ("user_properties", "{}"),
            ("game_directory", &self.instance_dir().display().to_string()),
            (
                "assets_root",
                &self.paths.assets_dir().display().to_string(),
            ),
            ("assets_index_name", &self.vj.asset_index.id),
            ("version_name", &self.vj.id),
            ("version_type", "release"),
            ("classpath", &self.classpath()),
            (
                "natives_directory",
                &self.natives_dir().display().to_string(),
            ),
            ("launcher_name", "Ferrolys"),
            ("launcher_version", ferrolys_core::APP_VERSION),
            ("clientid", ""),
            ("auth_xuid", ""),
            ("resolution_width", "854"),
            ("resolution_height", "480"),
        ];
        let mut out = arg.to_string();
        for (key, value) in table {
            out = out.replace(&format!("${{{key}}}"), value);
        }
        assert!(
            !out.contains("${"),
            "unsubstituted placeholder left in {out:?}"
        );
        out
    }
}

/// Ожидаемый argv по секциям: Xmx → logging → JVM → mainClass → игра.
fn expected_sections(ctx: &Ctx<'_>) -> Vec<String> {
    let mut args = vec!["-Xmx2048M".to_string()];

    if let Some(logging) = &ctx.vj.logging_client {
        let local = ctx
            .paths
            .assets_dir()
            .join("log_configs")
            .join(&logging.file_id);
        args.push(
            logging
                .argument
                .replace("${path}", &local.display().to_string()),
        );
    }

    if ctx.vj.jvm_arguments.is_empty() {
        args.push(format!(
            "-Djava.library.path={}",
            ctx.natives_dir().display()
        ));
        args.push("-cp".to_string());
        args.push(ctx.classpath());
    } else {
        args.extend(ctx.vj.jvm_arguments.iter().map(|arg| ctx.expand(arg)));
    }

    args.push(ctx.vj.main_class.clone());
    args.extend(ctx.vj.arguments.iter().map(|arg| ctx.expand(arg)));
    args
}

#[test]
fn golden_argv_1_7_10() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let paths = FerrolysPaths::with_home(tmp.path());
    let a = auth();
    let vj = parse_fixture("1.7.10");
    let ctx = Ctx {
        paths: &paths,
        vj: &vj,
        auth: &a,
    };
    let spec = build_command(&input(&paths, &vj, Path::new("java"), &a));

    // Литеральный argv: legacy-версия без секции arguments.jvm → дефолты
    // лаунчера, игра — из minecraftArguments.
    let expected: Vec<String> = [
        "-Xmx2048M".to_string(),
        format!(
            "-Dlog4j.configurationFile={}",
            paths
                .assets_dir()
                .join("log_configs")
                .join("client-1.7.xml")
                .display()
        ),
        format!("-Djava.library.path={}", ctx.natives_dir().display()),
        "-cp".to_string(),
        ctx.classpath(),
        "net.minecraft.client.main.Main".to_string(),
        "--username".to_string(),
        "Steve".to_string(),
        "--version".to_string(),
        "1.7.10".to_string(),
        "--gameDir".to_string(),
        ctx.instance_dir().display().to_string(),
        "--assetsDir".to_string(),
        paths.assets_dir().display().to_string(),
        "--assetIndex".to_string(),
        "1.7.10".to_string(),
        "--uuid".to_string(),
        a.uuid.clone(),
        "--accessToken".to_string(),
        a.access_token.clone(),
        "--userProperties".to_string(),
        "{}".to_string(),
        "--userType".to_string(),
        a.user_type.clone(),
    ]
    .to_vec();

    assert_eq!(spec.args, expected);
}

#[test]
fn golden_sections_modern() {
    for version in ["1.12.2", "1.20.1", "1.21.1"] {
        let tmp = tempfile::tempdir().expect("tempdir");
        let paths = FerrolysPaths::with_home(tmp.path());
        let a = auth();
        let vj = parse_fixture(version);
        let ctx = Ctx {
            paths: &paths,
            vj: &vj,
            auth: &a,
        };
        let spec = build_command(&input(&paths, &vj, Path::new("java"), &a));

        assert_eq!(
            spec.args,
            expected_sections(&ctx),
            "{version}: argv diverged from sections"
        );
    }
}

#[test]
fn classpath_with_spaces_stays_single_arg() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let a = auth();
    let vj = parse_fixture("1.7.10");

    let spaced = FerrolysPaths::with_home(tmp.path().join("My Games"));
    let plain = FerrolysPaths::with_home(tmp.path().join("plain"));
    let spec_spaced = build_command(&input(&spaced, &vj, Path::new("java"), &a));
    let spec_plain = build_command(&input(&plain, &vj, Path::new("java"), &a));

    // Пробел в пути не разрывает argv: столько же аргументов, что и без него.
    assert_eq!(spec_spaced.args.len(), spec_plain.args.len());

    let classpath = arg_after(&spec_spaced.args, "-cp").expect("-cp must be present");
    assert!(
        classpath.contains("My Games"),
        "classpath must carry the spaced root: {classpath}"
    );
    assert!(classpath.ends_with(".jar"));
}
