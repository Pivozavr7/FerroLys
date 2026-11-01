//! Тесты движка загрузок (план §7.1, Task 4).

use std::fs;
use std::path::PathBuf;

use ferrolys_core::install::download::{download_all, DownloadTask, Progress, DOWNLOAD_ATTEMPTS};
use sha1::{Digest, Sha1};
use tokio::sync::watch;
use wiremock::matchers::{header, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn sha1_hex(bytes: &[u8]) -> String {
    let digest = Sha1::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn task(url: &str, dest: PathBuf, body: &[u8]) -> DownloadTask {
    DownloadTask {
        url: url.to_string(),
        dest,
        sha1: Some(sha1_hex(body)),
        size: Some(body.len() as u64),
    }
}

#[tokio::test]
async fn downloads_and_verifies_sha1() {
    // два url с разными телами
    let server = MockServer::start().await;
    let body_a: &[u8] = b"first file contents";
    let body_b: &[u8] = b"second file contents, a bit longer";
    Mock::given(method("GET"))
        .and(wiremock::matchers::path("/a"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body_a))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(wiremock::matchers::path("/b"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body_b))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let tasks = vec![
        task(
            &format!("{}/a", server.uri()),
            dir.path().join("a.jar"),
            body_a,
        ),
        task(
            &format!("{}/b", server.uri()),
            dir.path().join("b.jar"),
            body_b,
        ),
    ];
    let (tx, rx) = watch::channel(Progress::default());

    download_all(&reqwest::Client::new(), tasks, &tx)
        .await
        .expect("both files must download");

    assert_eq!(fs::read(dir.path().join("a.jar")).unwrap(), body_a);
    assert_eq!(fs::read(dir.path().join("b.jar")).unwrap(), body_b);

    let final_progress = rx.borrow().clone();
    assert_eq!(final_progress.files_done, 2);
    assert_eq!(final_progress.files_total, 2);
    assert_eq!(
        final_progress.bytes_done,
        (body_a.len() + body_b.len()) as u64
    );
    assert_eq!(final_progress.bytes_total, final_progress.bytes_done);
    assert!(final_progress.eta_secs == Some(0));
}

#[tokio::test]
async fn skips_existing_file_with_matching_sha1() {
    let server = MockServer::start().await;
    let body: &[u8] = b"already downloaded";
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
        .expect(0)
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("cached.jar");
    fs::write(&dest, body).unwrap();

    let (tx, _rx) = watch::channel(Progress::default());
    let tasks = vec![task(&format!("{}/x", server.uri()), dest.clone(), body)];

    download_all(&reqwest::Client::new(), tasks, &tx)
        .await
        .expect("no network needed");

    assert_eq!(fs::read(&dest).unwrap(), body);
    server.verify().await; // ровно 0 GET
}

#[tokio::test]
async fn resumes_partial_download_via_range() {
    let full: Vec<u8> = (0..1000u32).map(|i| (i % 251) as u8).collect();
    let server = MockServer::start().await;
    // общий ответ (если Range не запрашивался)
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(full.clone()))
        .mount(&server)
        .await;
    // приоритет 1: иначе общий мок (GET без условий) перехватит Range-запрос
    Mock::given(method("GET"))
        .and(header("range", "bytes=400-"))
        .respond_with(ResponseTemplate::new(206).set_body_bytes(full[400..].to_vec()))
        .with_priority(1)
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("resumed.jar");
    fs::write(dest.with_file_name("resumed.jar.part"), &full[..400]).unwrap();

    let (tx, _rx) = watch::channel(Progress::default());
    let tasks = vec![task(&format!("{}/r", server.uri()), dest.clone(), &full)];

    download_all(&reqwest::Client::new(), tasks, &tx)
        .await
        .expect("resume must succeed");

    assert_eq!(fs::read(&dest).unwrap(), full, "файл докачан целиком");
    let requests = server.received_requests().await.unwrap();
    assert!(
        requests.iter().any(|r| {
            r.headers
                .get("range")
                .map(|v| v.to_str().unwrap_or("") == "bytes=400-")
                .unwrap_or(false)
        }),
        "сервер должен был получить Range-заголовок"
    );
}

#[tokio::test]
async fn sha1_mismatch_retries_then_fails() {
    let garbage: &[u8] = b"corrupted payload";
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(garbage))
        .expect(u64::from(DOWNLOAD_ATTEMPTS))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("bad.jar");
    let mut wrong = task(&format!("{}/b", server.uri()), dest.clone(), garbage);
    wrong.sha1 = Some("0000000000000000000000000000000000000000".to_string());

    let (tx, _rx) = watch::channel(Progress::default());
    let err = download_all(&reqwest::Client::new(), vec![wrong], &tx)
        .await
        .expect_err("bit content must fail after retries");

    assert!(
        matches!(err, ferrolys_core::CoreError::Sha1Mismatch { .. }),
        "got: {err:?}"
    );
    assert!(!dest.exists(), "битый файл не должен попасть в dest");
    assert!(
        !dest.with_file_name("bad.jar.part").exists(),
        ".part должен быть удалён"
    );
    server.verify().await; // ровно DOWNLOAD_ATTEMPTS попыток
}

#[tokio::test]
async fn io_error_returns_typed_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"x"))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("blocker");
    fs::write(&blocker, b"i am a file, not a directory").unwrap();

    let (tx, _rx) = watch::channel(Progress::default());
    let tasks = vec![task(
        &format!("{}/x", server.uri()),
        blocker.join("nested.jar"),
        b"x",
    )];

    let err = download_all(&reqwest::Client::new(), tasks, &tx)
        .await
        .expect_err("parent-is-file must be a typed io error");

    assert!(
        matches!(err, ferrolys_core::CoreError::Io(_)),
        "got: {err:?}"
    );
}

#[tokio::test]
async fn progress_reports_speed_and_eta() {
    let server = MockServer::start().await;
    let body: &[u8] = b"progress payload";
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let (tx, rx) = watch::channel(Progress::default());
    let tasks = vec![task(
        &format!("{}/p", server.uri()),
        dir.path().join("p.jar"),
        body,
    )];

    download_all(&reqwest::Client::new(), tasks, &tx)
        .await
        .expect("download must succeed");

    let progress = rx.borrow().clone();
    assert_eq!(progress.files_done, progress.files_total);
    assert_eq!(progress.bytes_done, progress.bytes_total);
    assert_eq!(progress.eta_secs, Some(0));
    assert!(
        progress.speed_bps > 0,
        "средняя скорость должна быть посчитана"
    );
}
