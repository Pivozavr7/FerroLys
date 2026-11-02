//! Тесты индекса ассетов (план §7.1, Task 5).

use std::fs;

use ferrolys_core::install::assets::{asset_tasks, asset_tasks_with_base, parse_asset_index};
use ferrolys_core::install::download::{download_all, Progress};
use sha1::{Digest, Sha1};
use tokio::sync::watch;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

fn sha1_hex(bytes: &[u8]) -> String {
    let digest = Sha1::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn fixture() -> Vec<u8> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/asset_index_1_21.json");
    fs::read(path).expect("fixture must exist")
}

#[test]
fn parses_asset_index() {
    let index = parse_asset_index(&fixture()).expect("fixture must parse");
    assert_eq!(index.objects.len(), 8);
    assert_eq!(index.total_size, 419_527);
    let first = &index.objects[0];
    assert_eq!(first.hash, "b62ca8ec10d07e6bf5ac8dae0c8c1d2e6a1e3356");
    assert_eq!(first.size, 9101);
}

#[test]
fn asset_tasks_have_hash_paths() {
    let index = parse_asset_index(&fixture()).unwrap();
    let assets_dir = std::path::Path::new("/games/assets");
    let tasks = asset_tasks(&index, assets_dir);

    assert_eq!(tasks.len(), 8);
    let task = &tasks[0];
    assert_eq!(
        task.dest,
        assets_dir.join("objects/b6/b62ca8ec10d07e6bf5ac8dae0c8c1d2e6a1e3356")
    );
    assert_eq!(
        task.url,
        "https://resources.download.minecraft.net/b6/b62ca8ec10d07e6bf5ac8dae0c8c1d2e6a1e3356"
    );
    assert_eq!(
        task.sha1.as_deref(),
        Some("b62ca8ec10d07e6bf5ac8dae0c8c1d2e6a1e3356")
    );
    assert_eq!(task.size, Some(9101));
    // все url — с одним и тем же базовым хостом
    assert!(tasks.iter().all(|t| t
        .url
        .starts_with("https://resources.download.minecraft.net/")));
}

#[tokio::test]
async fn installs_assets_on_wiremock() {
    // Индекс из байтов, которые отдаёт сервер: хеш = sha1 тела
    let bodies: Vec<(&str, Vec<u8>)> = vec![
        ("icons/gui.png", b"png-bytes-1".to_vec()),
        (
            "minecraft/sounds/levelup.ogg",
            b"ogg-bytes-longer-2".to_vec(),
        ),
        ("lang/ru_ru.json", "{}".as_bytes().to_vec()),
    ];
    let server = MockServer::start().await;
    let mut expected_bytes = 0u64;
    for (_, body) in &bodies {
        let hash = sha1_hex(body);
        Mock::given(method("GET"))
            .and(wiremock::matchers::path(format!(
                "/{}/{}",
                &hash[..2],
                hash
            )))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body.clone()))
            .mount(&server)
            .await;
        expected_bytes += body.len() as u64;
    }

    let index = ferrolys_core::install::assets::AssetIndex {
        objects: bodies
            .iter()
            .map(|(_, body)| ferrolys_core::install::assets::AssetObject {
                hash: sha1_hex(body),
                size: body.len() as u64,
            })
            .collect(),
        total_size: expected_bytes,
    };

    let dir = tempfile::tempdir().unwrap();
    let tasks = asset_tasks_with_base(&index, dir.path(), &server.uri());
    let (tx, rx) = watch::channel(Progress::default());

    download_all(&reqwest::Client::new(), tasks, &tx)
        .await
        .expect("assets must install");

    for (_, body) in &bodies {
        let hash = sha1_hex(body);
        let path = dir.path().join(format!("objects/{}/{}", &hash[..2], hash));
        assert_eq!(fs::read(&path).unwrap(), *body, "объект {hash} на месте");
    }

    let progress = rx.borrow().clone();
    assert_eq!(progress.files_done, 3);
    assert_eq!(progress.bytes_done, expected_bytes);
    assert_eq!(progress.bytes_total, expected_bytes);
    assert_eq!(progress.eta_secs, Some(0));
}
