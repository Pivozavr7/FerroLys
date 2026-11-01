//! Тесты клиента piston-meta (план §7.1, Task 1).

use std::fs;
use std::time::{Duration, SystemTime};

use ferrolys_core::install::version_manifest::{
    fetch_manifest_from, VersionType, MANIFEST_TTL_SECS,
};
use ferrolys_core::paths::FerrolysPaths;
use ferrolys_core::CoreError;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    fs::read(path).expect("fixture must exist")
}

#[tokio::test]
async fn fetch_manifest_parses_fixture_from_wiremock() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200).set_body_bytes(fixture("version_manifest_v2.json")),
        )
        .mount(&server)
        .await;

    let tmp = tempfile::tempdir().unwrap();
    let paths = FerrolysPaths::with_home(tmp.path());
    let client = reqwest::Client::new();

    let manifest = fetch_manifest_from(&client, &paths, &format!("{}/manifest.json", server.uri()))
        .await
        .unwrap();

    assert_eq!(manifest.versions.len(), 921);
    assert_eq!(manifest.latest_release, "26.3");
    let entry = manifest.get("1.7.10").expect("1.7.10 present");
    assert_eq!(entry.kind, VersionType::Release);
    assert!(!entry.url.is_empty());
    assert_eq!(entry.sha1.len(), 40);
    assert_eq!(
        manifest.get("b1.8.1").map(|e| e.kind),
        Some(VersionType::OldBeta)
    );
    assert_eq!(
        manifest.get("a1.2.6").map(|e| e.kind),
        Some(VersionType::OldAlpha)
    );
    assert!(manifest.get("nope").is_none());
}

#[tokio::test]
async fn fetch_manifest_uses_cache_within_ttl() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200).set_body_bytes(fixture("version_manifest_v2.json")),
        )
        .expect(1)
        .mount(&server)
        .await;

    let tmp = tempfile::tempdir().unwrap();
    let paths = FerrolysPaths::with_home(tmp.path());
    let client = reqwest::Client::new();
    let url = format!("{}/manifest.json", server.uri());

    let first = fetch_manifest_from(&client, &paths, &url).await.unwrap();
    let second = fetch_manifest_from(&client, &paths, &url).await.unwrap();

    assert_eq!(first.versions.len(), second.versions.len());
    assert!(paths.cache_dir().join("version_manifest_v2.json").exists());
    server.verify().await; // ровно один GET за оба вызова
}

#[tokio::test]
async fn fetch_manifest_serves_stale_cache_when_network_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = FerrolysPaths::with_home(tmp.path());
    fs::create_dir_all(paths.cache_dir()).unwrap();
    let cache_file = paths.cache_dir().join("version_manifest_v2.json");
    fs::write(&cache_file, fixture("version_manifest_v2.json")).unwrap();
    let stale = SystemTime::now() - Duration::from_secs(MANIFEST_TTL_SECS + 60);
    fs::File::options()
        .write(true)
        .open(&cache_file)
        .unwrap()
        .set_modified(stale)
        .unwrap();

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let client = reqwest::Client::new();
    let manifest = fetch_manifest_from(&client, &paths, &format!("{}/m", server.uri()))
        .await
        .expect("stale cache must be served as fallback");

    assert_eq!(manifest.versions.len(), 921);
}

#[tokio::test]
async fn fetch_manifest_errors_without_cache_on_http_failure() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let tmp = tempfile::tempdir().unwrap();
    let paths = FerrolysPaths::with_home(tmp.path());
    let client = reqwest::Client::new();

    let err = fetch_manifest_from(&client, &paths, &format!("{}/m", server.uri()))
        .await
        .expect_err("no cache + HTTP failure must be an error");

    assert!(matches!(err, CoreError::Http(_)), "got: {err:?}");
}
