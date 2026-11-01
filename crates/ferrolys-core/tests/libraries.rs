//! Тесты задач загрузки библиотек и раскладки natives (план §7.1, Task 6).

use std::fs;
use std::path::{Path, PathBuf};

use ferrolys_core::install::libraries::{library_tasks, natives_targets};
use ferrolys_core::install::version_json::parse_version_json;

fn parse_fixture(version: &str) -> ferrolys_core::install::version_json::VersionJson {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/versions")
        .join(format!("{version}.json"));
    parse_version_json(&fs::read(path).expect("fixture must parse")).expect("fixture must parse")
}

#[test]
fn library_tasks_for_1_7_10_on_fixture() {
    let vj = parse_fixture("1.7.10");
    let libraries_dir = Path::new("/i/instance/libraries");
    let tasks = library_tasks(&vj, libraries_dir);

    // golden: 29 artifact-задач + 2 natives-классификатора = 31
    assert_eq!(tasks.len(), 31);

    let natives = tasks
        .iter()
        .find(|t| {
            t.dest
                == libraries_dir.join(
                    "org/lwjgl/lwjgl/lwjgl-platform/2.9.1/lwjgl-platform-2.9.1-natives-linux.jar",
                )
        })
        .expect("lwjgl-platform natives-linux task present");
    assert_eq!(
        natives.url,
        "https://libraries.minecraft.net/org/lwjgl/lwjgl/lwjgl-platform/2.9.1/lwjgl-platform-2.9.1-natives-linux.jar"
    );
    assert_eq!(
        natives.sha1.as_deref(),
        Some("aa9aae879af8eb378e22cfc64db56ec2ca9a44d1")
    );
    assert_eq!(natives.size, Some(571_424));

    // обычный artifact-путь
    assert!(tasks
        .iter()
        .any(|t| { t.dest == libraries_dir.join("com/mojang/netty/1.8.8/netty-1.8.8.jar") }));

    // twitch-библиотеки (disallow linux) не попадают в задачи
    assert!(!tasks.iter().any(|t| t.url.contains("tv.twitch")));
    assert!(tasks
        .iter()
        .all(|t| t.url.starts_with("https://libraries.minecraft.net/")));
}

#[test]
fn library_tasks_for_1_21_on_fixture() {
    let vj = parse_fixture("1.21.1");
    let libraries_dir = Path::new("/i/instance/libraries");
    let tasks = library_tasks(&vj, libraries_dir);

    // golden: 56 разрешённых библиотек, у всех artifact (natives входят в classpath)
    assert_eq!(tasks.len(), 56);
    // windows/osx-варианты исключены правилами
    assert!(!tasks
        .iter()
        .any(|t| t.url.contains("natives-windows") || t.url.contains("natives-macos")));
    // контрольный путь maven
    assert!(tasks.iter().any(|t| {
        t.dest == libraries_dir.join("org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-linux.jar")
    }));
    // у modern-библиотек extraction не нужен
    assert!(natives_targets(
        &vj,
        libraries_dir,
        Path::new("/i/instance/versions/1.21.1/natives")
    )
    .is_empty());
}

#[test]
fn library_tasks_for_1_12_2_on_fixture() {
    let vj = parse_fixture("1.12.2");
    let tasks = library_tasks(&vj, Path::new("/lib"));

    // golden: 33 artifact + 3 natives-классификатора
    assert_eq!(tasks.len(), 36);
    assert!(!tasks.iter().any(|t| t.url.contains("2.9.2-nightly")));
}

#[test]
fn natives_targets_have_excludes() {
    let vj = parse_fixture("1.7.10");
    let libraries_dir = Path::new("/i/instance/libraries");
    let natives_dir = Path::new("/i/instance/versions/1.7.10/natives");

    let targets = natives_targets(&vj, libraries_dir, natives_dir);

    // golden: 2 библиотеки со старым форматом natives на linux
    assert_eq!(targets.len(), 2);
    for target in &targets {
        assert_eq!(target.dest, natives_dir);
        assert!(
            target.excludes.iter().any(|e| e.contains("META-INF")),
            "исключение META-INF обязано прийти из extract.exclude: {:?}",
            target.excludes
        );
        assert!(target.archive.starts_with(libraries_dir));
        assert!(target
            .archive
            .to_string_lossy()
            .ends_with("-natives-linux.jar"));
    }
    let lwjgl: Vec<PathBuf> = targets
        .iter()
        .map(|t| t.archive.clone())
        .filter(|p| p.to_string_lossy().contains("lwjgl-platform"))
        .collect();
    assert_eq!(
        lwjgl,
        vec![libraries_dir
            .join("org/lwjgl/lwjgl/lwjgl-platform/2.9.1/lwjgl-platform-2.9.1-natives-linux.jar")]
    );
}
