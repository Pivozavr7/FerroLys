//! Общие HTTP/кэш-хелперы установки: GET с User-Agent, атомарная запись
//! кэша, SHA-1 (hex и по файлу) — используются манифестом версий,
//! оркестратором и движком загрузок.

use std::fs;
use std::io::Read;
use std::path::Path;

use sha1::{Digest, Sha1};

use crate::error::CoreError;

/// GET с `User-Agent` приложения → байты тела (любой не-2xx → ошибка).
pub(crate) async fn fetch_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, CoreError> {
    let response = client
        .get(url)
        .header(reqwest::header::USER_AGENT, crate::APP_USER_AGENT)
        .send()
        .await?
        .error_for_status()?;
    Ok(response.bytes().await?.to_vec())
}

/// Атомарная запись кэша: `<file>.tmp` → rename.
pub(crate) fn write_cache(file: &Path, bytes: &[u8]) -> Result<(), CoreError> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = file.with_extension("json.tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, file)?;
    Ok(())
}

/// Закодировать байты в hex (нижний регистр, без алокаций на каждый байт).
fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// SHA-1 содержимого → hex (нижний регистр).
pub(crate) fn sha1_hex(bytes: &[u8]) -> String {
    hex_encode(&Sha1::digest(bytes))
}

/// SHA-1 файла по содержимому → hex.
pub(crate) fn sha1_file(path: &Path) -> Result<String, CoreError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha1::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(hex_encode(&hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Регрессия: `sha1_file` не должен хешировать дайдест дважды.
    /// `sha1_hex` кодирует вход как есть, `hex_encode` — чистое hex-кодирование.
    #[test]
    fn sha1_file_matches_sha1_hex_of_same_bytes() {
        let dir = std::env::temp_dir().join(format!("cache-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("x.bin");
        std::fs::write(&f, b"already downloaded").unwrap();

        let want = sha1_hex(b"already downloaded");
        assert_eq!(want, "1d121c9cd73266a077c73bd49747eb455e220660");
        assert_eq!(sha1_file(&f).unwrap(), want);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hex_encode_is_lowercase_two_digits() {
        assert_eq!(hex_encode(&[0x00, 0x0f, 0xff]), "000fff");
    }
}
