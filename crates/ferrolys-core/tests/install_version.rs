//! Тесты оркестратора установки версии (план §7.1, Task 7).

use std::fs;
use std::io::Write;

use ferrolys_core::install::download::Progress;
use ferrolys_core::install::orchestrator::{install_version, InstallOptions};
use ferrolys_core::paths::FerrolysPaths;
use sha1::{Digest, Sha1};
use tokio::sync::watch;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn sha1_hex(bytes: &[u8]) -> String {
    let digest = Sha1::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    fs::read(path).expect("fixture must exist")
}

/// Zip natives-классификатора: один .so и META-INF (должен быть пропущен).
fn natives_jar_bytes() -> Vec<u8> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default();
        writer.start_file("libfoo.so", options).unwrap();
        writer.write_all(b"ELF").unwrap();
        writer.start_file("META-INF/MANIFEST.MF", options).unwrap();
        writer.write_all(b"manifest").unwrap();
        writer.finish().unwrap();
    }
    cursor.into_inner()
}

/// Полное дерево: манифест + version-json + jar + библиотеки + ассеты + логинг.
struct Tree {
    server: MockServer,
    jar_bytes: Vec<u8>,
    lib_a: Vec<u8>,
    lib_b: Vec<u8>,
    natives_bytes: Vec<u8>,
    asset_bodies: Vec<Vec<u8>>,
    log_bytes: Vec<u8>,
    instance: String,
    version: String,
}

async fn mounted_server() -> Tree {
    let server = MockServer::start().await;

    let jar_bytes = b"client-jar-bytes".to_vec();
    let lib_a = b"library-a-jar".to_vec();
    let lib_b = b"library-b-jar".to_vec();
    let natives_bytes = natives_jar_bytes();
    let asset_bodies: Vec<Vec<u8>> = vec![
        b"asset-one".to_vec(),
        b"asset-two-longer".to_vec(),
        b"asset-three".to_vec(),
        b"asset-four-again".to_vec(),
    ];
    let log_bytes = b"<Configuration/>".to_vec();

    // индекс ассетов: хеши от тел, которые отдаёт сервер
    let objects: Vec<String> = asset_bodies
        .iter()
        .enumerate()
        .map(|(i, body)| {
            let hash = sha1_hex(body);
            format!(
                "\"obj{i}\": {{\"hash\": \"{hash}\", \"size\": {}}}",
                body.len()
            )
        })
        .collect();
    let asset_index = format!("{{\"objects\": {{{}}}}}", objects.join(","));
    let asset_index_bytes = asset_index.into_bytes();

    // version-json: url шлём прямо на тестовый сервер
    let base = server.uri();
    let version_json = serde_json::to_vec(&serde_json::json!({
        "id": "1.21.1",
        "mainClass": "net.minecraft.client.main.Main",
        "javaVersion": {"component": "java-runtime-delta", "majorVersion": 21},
        "arguments": {
            "game": ["--username", "${auth_player_name}"],
            "jvm": ["-Djava.library.path=${natives_directory}"]
        },
        "assetIndex": {
            "id": "test-index",
            "url": format!("{base}/assets.json"),
            "sha1": sha1_hex(&asset_index_bytes),
            "size": asset_index_bytes.len()
        },
        "downloads": {
            "client": {
                "url": format!("{base}/client.jar"),
                "sha1": sha1_hex(&jar_bytes),
                "size": jar_bytes.len()
            }
        },
        "libraries": [
            {
                "name": "com.example:lib-a:1.0",
                "downloads": {"artifact": {
                    "path": "com/example/lib-a/1.0/lib-a-1.0.jar",
                    "url": format!("{base}/libs/lib-a.jar"),
                    "sha1": sha1_hex(&lib_a),
                    "size": lib_a.len()
                }}
            },
            {
                "name": "com.example:lib-b:2.0",
                "rules": [{"action": "allow"}, {"action": "disallow", "os": {"name": "osx"}}],
                "downloads": {"artifact": {
                    "path": "com/example/lib-b/2.0/lib-b-2.0.jar",
                    "url": format!("{base}/libs/lib-b.jar"),
                    "sha1": sha1_hex(&lib_b),
                    "size": lib_b.len()
                }}
            },
            {
                "name": "com.example:lib-win:3.0",
                "rules": [{"action": "allow", "os": {"name": "windows"}}],
                "downloads": {"artifact": {
                    "path": "com/example/lib-win/3.0/lib-win-3.0.jar",
                    "url": format!("{base}/libs/lib-win.jar"),
                    "sha1": "0000000000000000000000000000000000000000",
                    "size": 1
                }}
            },
            {
                "name": "org.lwjgl.lwjgl:lwjgl-platform:2.9.1",
                "natives": {"linux": "natives-linux"},
                "extract": {"exclude": ["META-INF/"]},
                "downloads": {"classifiers": {"natives-linux": {
                    "url": format!("{base}/cls.jar"),
                    "sha1": sha1_hex(&natives_bytes),
                    "size": natives_bytes.len()
                }}}
            }
        ],
        "logging": {"client": {
            "argument": "-Dlog4j.configurationFile=${path}",
            "file": {
                "id": "client-1.21.xml",
                "url": format!("{base}/log.xml"),
                "sha1": sha1_hex(&log_bytes),
                "size": log_bytes.len()
            }
        }}
    }))
    .expect("version json serializes");

    // манифест — реальный fixture; путь version-json берём из его entry
    let manifest_bytes = fixture("version_manifest_v2.json");
    let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes).unwrap();
    let entry_url = manifest["versions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["id"] == "1.21.1")
        .expect("1.21.1 in manifest")["url"]
        .as_str()
        .unwrap()
        .to_string();
    let version_path = entry_url
        .strip_prefix("https://piston-meta.mojang.com")
        .expect("piston-meta url")
        .to_string();

    Mock::given(method("GET"))
        .and(path("/mc/game/version_manifest_v2.json"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(manifest_bytes))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(version_path))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(version_json))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/client.jar"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(jar_bytes.clone()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/libs/lib-a.jar"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(lib_a.clone()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/libs/lib-b.jar"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(lib_b.clone()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/libs/lib-win.jar"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"win"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/cls.jar"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(natives_bytes.clone()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/assets.json"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(asset_index_bytes))
        .mount(&server)
        .await;
    for body in &asset_bodies {
        let hash = sha1_hex(body);
        Mock::given(method("GET"))
            .and(path(format!("/{}/{}", &hash[..2], hash)))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body.clone()))
            .mount(&server)
            .await;
    }
    Mock::given(method("GET"))
        .and(path("/log.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(log_bytes.clone()))
        .mount(&server)
        .await;

    Tree {
        server,
        jar_bytes,
        lib_a,
        lib_b,
        natives_bytes,
        asset_bodies,
        log_bytes,
        instance: "main".to_string(),
        version: "1.21.1".to_string(),
    }
}

fn options(tree: &Tree) -> InstallOptions {
    let mut opts = InstallOptions::new(&tree.instance, &tree.version, reqwest::Client::new());
    opts.url_map = vec![
        (
            "https://piston-meta.mojang.com".to_string(),
            tree.server.uri(),
        ),
        (
            "https://resources.download.minecraft.net".to_string(),
            tree.server.uri(),
        ),
    ];
    opts
}

#[tokio::test]
async fn full_install_builds_instance_tree() {
    let tree = mounted_server().await;
    let tmp = tempfile::tempdir().unwrap();
    let paths = FerrolysPaths::with_home(tmp.path());
    let (tx, rx) = watch::channel(Progress::default());

    install_version(&paths, &options(&tree), &tx)
        .await
        .expect("full install must succeed");

    let instance = paths.instances_dir().join(&tree.instance);
    let version_dir = instance.join("versions").join(&tree.version);

    // client jar
    let jar = version_dir.join(format!("{}.jar", tree.version));
    assert_eq!(fs::read(&jar).unwrap(), tree.jar_bytes);

    // библиотеки: lib-win (windows-only) не скачан
    let lib_a = instance.join("libraries/com/example/lib-a/1.0/lib-a-1.0.jar");
    assert_eq!(fs::read(&lib_a).unwrap(), tree.lib_a);
    let lib_b = instance.join("libraries/com/example/lib-b/2.0/lib-b-2.0.jar");
    assert_eq!(fs::read(&lib_b).unwrap(), tree.lib_b);
    let lib_win = instance.join("libraries/com/example/lib-win/3.0/lib-win-3.0.jar");
    assert!(
        !lib_win.exists(),
        "windows-only библиотека не должна ставиться"
    );

    // natives: .so распакован, META-INF — нет
    let natives_dir = version_dir.join("natives");
    assert_eq!(
        fs::read(natives_dir.join("libfoo.so")).unwrap(),
        b"ELF".to_vec()
    );
    assert!(!natives_dir.join("META-INF").exists());
    assert!(
        instance
            .join("libraries/org/lwjgl/lwjgl/lwjgl-platform/2.9.1/lwjgl-platform-2.9.1-natives-linux.jar")
            .exists(),
        "классификатор скачан в libraries"
    );
    assert_eq!(
        fs::read(instance.join(
            "libraries/org/lwjgl/lwjgl/lwjgl-platform/2.9.1/lwjgl-platform-2.9.1-natives-linux.jar"
        ))
        .unwrap(),
        tree.natives_bytes,
        "классификатор лежит в libraries байт-в-байт"
    );

    // ассеты: objects/<hh>/<hash>
    for body in &tree.asset_bodies {
        let hash = sha1_hex(body);
        let object = paths
            .assets_dir()
            .join("objects")
            .join(&hash[..2])
            .join(&hash);
        assert_eq!(fs::read(&object).unwrap(), *body, "asset {hash}");
    }

    // логинг-конфиг
    assert_eq!(
        fs::read(paths.assets_dir().join("log_configs/client-1.21.xml")).unwrap(),
        tree.log_bytes
    );

    // кэши
    assert!(
        paths.cache_dir().join("versions/1.21.1.json").exists(),
        "version-json закэширован"
    );
    assert!(
        paths
            .cache_dir()
            .join("asset_indexes/test-index.json")
            .exists(),
        "asset-index закэширован"
    );

    // прогресс финальной фазы сходится
    let progress = rx.borrow().clone();
    assert_eq!(progress.files_done, progress.files_total);
    assert!(progress.files_total > 0);
    assert_eq!(progress.bytes_done, progress.bytes_total);
    assert_eq!(progress.eta_secs, Some(0));
}

#[tokio::test]
async fn second_install_is_noop_on_network() {
    let tree = mounted_server().await;
    let tmp = tempfile::tempdir().unwrap();
    let paths = FerrolysPaths::with_home(tmp.path());
    let (tx, _rx) = watch::channel(Progress::default());
    let client = reqwest::Client::new();

    install_version(
        &paths,
        &InstallOptions {
            instance_id: tree.instance.clone(),
            version_id: tree.version.clone(),
            client: client.clone(),
            url_map: options(&tree).url_map,
        },
        &tx,
    )
    .await
    .expect("first install");

    let after_first = tree.server.received_requests().await.unwrap().len();

    install_version(
        &paths,
        &InstallOptions {
            instance_id: tree.instance.clone(),
            version_id: tree.version.clone(),
            client,
            url_map: options(&tree).url_map,
        },
        &tx,
    )
    .await
    .expect("second install");

    let after_second = tree.server.received_requests().await.unwrap().len();
    assert_eq!(
        after_first, after_second,
        "повторный запуск не должен ходить в сеть ({after_first} → {after_second})"
    );
    // natives перераспакованы без сети — файл на месте
    let so = paths
        .instances_dir()
        .join(&tree.instance)
        .join("versions")
        .join(&tree.version)
        .join("natives/libfoo.so");
    assert!(so.exists());
}
