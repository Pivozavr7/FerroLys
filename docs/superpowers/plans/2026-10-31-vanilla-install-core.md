# Vanilla Install Core (§7.1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Клиент piston-meta, парсер version-JSON (1.7.10→1.21.x), конкурентный движок загрузок с resume/SHA-1/прогрессом, установка assets и libraries в структуру инстанса — всё в `ferrolys-core`, тестируется без UI.

**Architecture:** Новый модуль `install/` в `ferrolys-core`: чистые парсеры (без сети) + тонкая сеть-обёртка (reqwest) + оркестратор установки. Прогресс — `tokio::sync::watch` (план §4.2), UI подключится позже. Файлы — только через `FerrolysPaths` (§4.3): assets общие, libraries/jar — в инстансе.

**Tech Stack:** tokio, reqwest (rustls, stream), futures-util, serde/serde_json, sha1, zip, wiremock (dev).

**Spec:** `ferrolys-dev-plan.md` §7.1, §4.1–4.3, §12.

## Global Constraints

- Зависимости (разрешены пользователем для волны 1): `tokio`, `reqwest`, `futures-util`, `sha1`, `zip` в `[dependencies]` ferrolys-core; `wiremock`, `tokio` — в `[dev-dependencies]`. Всё сверх — спросить.
- Нет `unwrap()`/`expect()` вне тестов. Ошибки домена — `CoreError` (thiserror) в `error.rs`; восстановимые — `tracing::warn!` + fallback, никаких паник.
- Все пути — через `FerrolysPaths`; запрещены абсолютные пути и хардкод `~`.
- `ferrolys-core` не зависит от Slint.
- Итерация: `cargo check`; перед каждым коммитом: `cargo build` + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo test --workspace`.
- Минимальные диффы; строк UI в этой задаче нет (экран установки — отдельная задача волны).
- Коммит на каждый выполненный таск.

## Review Focus

Пять входов/отказов, которые spec подразумевает, но не каждым тестом покрыты:

1. **Сервер отдаёт байты, SHA-1 не сходится никогда** (битый/подменённый контент) → после N ретраев — ошибка, не вечный цикл и не записанный бракованный файл. → Task 4, тест `sha1_mismatch_retries_then_fails`.
2. **Обрыв тела ответа посреди передачи** → resume через `Range`, файл докачивается, `.part` не превращается в финальный бракованный. → Task 4, тест `resumes_partial_download_via_range`.
3. **Уже установленный файл существует с верным SHA-1** (повторный запуск после сбоя посреди установки) → сетевой вызов не делается (идемпотентность). → Task 4, тест `skips_existing_file_with_matching_sha1`.
4. **Старый манифест 1.7.10** (`minecraftArguments` строка, rules с `disallow`, natives через classifiers) **и новый 1.21.x** (`arguments.game/jvm` — массивы строк/объектов, `javaVersion`) → оба парсятся одинаковым кодом. → Task 2, тесты `parses_legacy_1_7_10` и `parses_modern_1_21`.
5. **Нет сети / нет прав на каталог** → ошибка без паники, частично скачанное не мешает следующему запуску. → Task 4, тест `io_error_returns_typed_error` (каталог назначения = файл) — маппинг в CoreError::Io.

---

### Task 1: Клиент piston-meta с кэшем манифеста версий

**Files:**
- Create: `crates/ferrolys-core/src/install/mod.rs`
- Create: `crates/ferrolys-core/src/install/version_manifest.rs`
- Modify: `crates/ferrolys-core/src/lib.rs` (подключить `pub mod install;`)
- Modify: `crates/ferrolys-core/src/error.rs`
- Modify: `crates/ferrolys-core/Cargo.toml` (deps: tokio, reqwest, futures-util; dev: wiremock, tokio)
- Test: `crates/ferrolys-core/tests/install_manifest.rs`
- Fixture: `crates/ferrolys-core/tests/fixtures/version_manifest_v2.json`

**Interfaces:**
- Consumes: `FerrolysPaths::cache_dir()`.
- Produces (для Task 7):
  - `pub const PISTON_MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";`
  - `pub enum VersionType { Release, Snapshot, OldBeta, OldAlpha, Preview }`
  - `pub struct ManifestEntry { pub id: String, pub kind: VersionType, pub url: String, pub sha1: String, pub release_time: String }`
  - `pub struct VersionManifest { pub latest_release: String, pub latest_snapshot: String, pub versions: Vec<ManifestEntry> }`
  - `impl VersionManifest { pub fn get(&self, id: &str) -> Option<&ManifestEntry> }`
  - `pub async fn fetch_manifest(client: &reqwest::Client, paths: &FerrolysPaths) -> Result<VersionManifest, CoreError>;`
  - `pub const MANIFEST_TTL_SECS: u64 = 600;`
- `CoreError` пополняется: `Http(#[from] reqwest::Error)`, `Json(#[from] serde_json::Error)`, `Sha1Mismatch { expected: String, actual: String }`(текст статический, без i18n в core), `DownloadFailed { url: String, attempts: u32 }`.

Логика кэша: файл `<cache_dir>/version_manifest_v2.json`; свежесть — по mtime (< `MANIFEST_TTL_SECS` → читать с диска; иначе GET, проверить SHA-1 от тела, атомарно записать `.tmp` → rename, затем распарсить). Сетевая ошибка при просроченном, но имеющемся кэше → `tracing::warn!` + отдать кэш (fallback, не ошибка).

- [ ] **Step 1: Написать фикстуру манифеста**

Скачать реальный `version_manifest_v2.json` в `tests/fixtures/` (один раз, закоммитить).

- [ ] **Step 2: Написать падающий тест**

```rust
#[tokio::test]
async fn fetch_manifest_parses_fixture_from_wiremock() {
    // wiremock отдаёт фикстуру; fetch_manifest →
    // versions.len() == <из фикстуры>, latest_release == "1.21.4" (по факту из файла),
    // get("1.7.10") == Some, get("nope") == None
}

#[tokio::test]
async fn fetch_manifest_uses_cache_within_ttl() {
    // первый вызов → 1 GET; второй вызов → сервер всё ещё видит ровно 1 hit
}

#[tokio::test]
async fn fetch_manifest_serves_stale_cache_when_network_fails() {
    // кэш записан, mtime старше TTL, сервер отдаёт 500 → Ok из кэша + warn
}
```

- [ ] **Step 3: Запустить → FAIL** (нет модуля `install`): `cargo test -p ferrolys-core --test install_manifest`

- [ ] **Step 4: Реализовать `version_manifest.rs` + пополнить `CoreError`**

Тело кэша/сети — по сигнатурам выше; serde-теги для `type` → `VersionType` (`rename_all = "lowercase"`-маппинг, `Preview` ← `preview`).

- [ ] **Step 5: Запустить → PASS**, затем `cargo clippy --workspace --all-targets -- -D warnings`

- [ ] **Step 6: Коммит** `feat(install): piston-meta version manifest client with TTL cache`

---

### Task 2: Парсер version-JSON и правила библиотек

**Files:**
- Create: `crates/ferrolys-core/src/install/version_json.rs`
- Fixtures: `crates/ferrolys-core/tests/fixtures/versions/1.7.10.json`, `1.12.2.json`, `1.20.1.json`, `1.21.1.json` (реальные, скачаны с piston-meta)
- Test: `crates/ferrolys-core/tests/version_json.rs`

**Interfaces:**
- Consumes: `CoreError::Json`.
- Produces (для Task 3, 6, 7):
  - `pub struct VersionJson { pub id: String, pub main_class: String, pub java_version: Option<i32>, pub arguments: Vec<String>, pub jvm_arguments: Vec<String>, pub asset_index: AssetIndexRef, pub downloads: Downloads, pub libraries: Vec<Library>, pub logging_client: Option<LoggingConfig> }`
  - `pub struct AssetIndexRef { pub id: String, pub url: String, pub sha1: String, pub size: u64 }`
  - `pub struct Downloads { pub client: Option<Artifact>, pub server: Option<Artifact> }`
  - `pub struct Artifact { pub url: String, pub sha1: String, pub size: u64 }`
  - `pub struct Library { pub name: String, pub artifact: Option<Artifact>, pub classifiers: Vec<(String, Artifact)>, pub natives_linux: Option<String>, pub rules: Vec<Rule>, pub extract_excludes: Vec<String> }`
  - `pub struct Rule { pub action: Action /* Allow | Disallow */, pub os_name: Option<String>, pub os_arch: Option<String>, pub features: Vec<(String, bool)> }`
  - `pub struct LoggingConfig { pub argument: String, pub file_id: String, pub file: Artifact }` (`file_id` — имя файла конфига, например `client-1.12.xml`)
  - `pub fn parse_version_json(bytes: &[u8]) -> Result<VersionJson, CoreError>;`
  - `pub fn library_allowed(lib: &Library) -> bool` — правила для Linux x86_64; `features` — все пары должны совпасть с ожидаемыми значениями окружения по умолчанию (никаких фич не считаем включёнными); отсутствие `rules` = разрешено; `disallow` без `os` = запрет всем.

Правила аргументов: `arguments.game/jvm` — строка **или** массив смешанного содержимого (строка | объект `{value, rules}`); объекты фильтруются теми же `Rule`. Склеивание: game-аргументы добавляются к `minecraftArguments` старых версий (или заменяют его), jvm-аргументы — отдельно.

- [ ] **Step 1: Скачать 4 реальных version-JSON в fixtures**

- [ ] **Step 2: Падающие тесты** (`cargo test -p ferrolys-core --test version_json`):

```rust
#[test] fn parses_legacy_1_7_10();      // main_class=="net.minecraft.client.main.Minecraft", java_version==None (нет ключа), есть minecraftArguments-токены в arguments, библиотека с natives classifier присутствует
#[test] fn parses_modern_1_21();        // java_version==Some(21), arguments содержат "-Djava.library.path=${natives_directory}", rules-объекты отфильтрованы
#[test] fn parses_1_12_2();             // java_version==Some(8), rules с os.name=="windows" → library_allowed()==false для windows-правил (проверка на библиотеке, у которой правила только для windows)
#[test] fn rules_disallow_without_os_blocks_everything();  // рукописный JSON
#[test] fn rules_feature_mismatch_disallows();             // рукописный JSON
#[test] fn missing_rules_means_allowed();                  // рукописный JSON
```

- [ ] **Step 3: FAIL подтверждён** (модуля нет)

- [ ] **Step 4: Реализовать `version_json.rs`**

- [ ] **Step 5: PASS + clippy** — `cargo test -p ferrolys-core --test version_json && cargo clippy --workspace --all-targets -- -D warnings`

- [ ] **Step 6: Коммит** `feat(install): version json parser with library rules (1.7.10 → 1.21)`

---

### Task 3: Maven-координаты → путь и natives

**Files:**
- Create: `crates/ferrolys-core/src/install/maven.rs`
- Test: `crates/ferrolys-core/tests/maven.rs`
- Modify: `crates/ferrolys-core/Cargo.toml` (dep: `zip`)

**Interfaces:**
- Consumes: `Library`, `Artifact` из Task 2.
- Produces (для Task 6):
  - `pub fn maven_path(name: &str) -> Result<String, CoreError>;` — `"group:artifact:version"` / `…:classifier` / `…:ext@jar` → `group/artifact/version/artifact-version[-classifier].ext`
  - `pub fn expand_arch(placeholder: &str) -> String;` — `${arch}` → `x86_64`
  - `pub fn extract_natives(archive: &Path, dest: &Path) -> Result<usize, CoreError>;` — распаковка в `dest`, пропуск путей, содержащих любой из `extract_excludes` (передаётся списком) и `META-INF`; возвращает число файлов. Создание `dest` — включено.
  - Сигнатура точнее: `pub fn extract_natives(archive: &Path, dest: &Path, excludes: &[String]) -> Result<usize, CoreError>;`

- [ ] **Step 1: Падающие тесты** (`cargo test -p ferrolys-core --test maven`):

```rust
#[test] fn maven_path_plain();          // "com.mojang:brigadier:1.0.18" → "com/mojang/brigadier/1.0.18/brigadier-1.0.18.jar"
#[test] fn maven_path_with_classifier();// "org.lwjgl:lwjgl:3.3.3:natives-linux" → "…/lwjgl-3.3.3-natives-linux.jar"
#[test] fn maven_path_with_extension(); // "io.netty:netty-all:4.0.15.Final:shaded" (и "@"-вариант "group:a:v@zip")
#[test] fn maven_path_invalid_returns_error(); // "не-координаты" → Err
#[test] fn extract_natives_skips_meta_inf_and_excludes(); // zip из памяти (tempfile) с файлами: natives/libfoo.so, natives/META-INF/MANIFEST.MF, natives/META-INF/versions/x → число==1, файл существует
```

- [ ] **Step 2: FAIL**

- [ ] **Step 3: Реализовать `maven.rs`** (`zip::ZipArchive` по `File`)

- [ ] **Step 4: PASS + clippy**

- [ ] **Step 5: Коммит** `feat(install): maven coordinate paths and natives extraction`

---

### Task 4: Движок загрузок (конкурентность, resume, SHA-1, прогресс)

**Files:**
- Create: `crates/ferrolys-core/src/install/download.rs`
- Test: `crates/ferrolys-core/tests/download.rs`

**Interfaces:**
- Consumes: `CoreError`, `FerrolysPaths` (для тестов не нужен — цели назначения в tempdir).
- Produces (для Task 5, 6, 7):
  - `pub struct DownloadTask { pub url: String, pub dest: PathBuf, pub sha1: Option<String>, pub size: Option<u64> }`
  - `pub struct Progress { pub files_total: usize, pub files_done: usize, pub bytes_total: u64, pub bytes_done: u64, pub speed_bps: u64, pub eta_secs: Option<u64> }`
  - `pub async fn download_all(client: &reqwest::Client, tasks: Vec<DownloadTask>, tx: &tokio::sync::watch::Sender<Progress>) -> Result<(), CoreError>;`
  - `pub const DOWNLOAD_CONCURRENCY: usize = 6;`
  - `pub const DOWNLOAD_ATTEMPTS: u32 = 3;`
- Поведение:
  - Пропуск: `dest` существует и (sha1==None → пропуск | sha1 есть → совпадает) — без сетевых вызовов; битый существующий — удалить и скачать заново.
  - Качаем в `<dest>.part`; при наличии `.part` — GET с заголовком `Range: bytes=<len>-`; ответ 206 — дописываем (`Mode::Append`), 200 — начинаем заново (сервер проигнорировал Range).
  - SHA-1 после докачки: не совпал → удалить `.part`, ретрай (счётчик общий, `DOWNLOAD_ATTEMPTS`), исчерпан → `CoreError::Sha1Mismatch`. Совпадает → rename `.part` → `dest`.
  - Ретраи: сетевые ошибки и 5xx — с бэкоффом `200ms * 2^n` (тесты уменьшают через параметр? нет — бэкофф вынесен в `tokio::time::sleep`, в тестах ок; держим константой, тесты терпят ~1.4s).
  - 4xx без тела-файла → ошибка сразу (ретрай бессмыслен).
  - Прогресс: `watch::Sender` обновляется на каждый завершённый файл и раз в ~100мс по байтам (drain-через `try_send`-аналог для watch — `send_replace`); `speed_bps` — среднее с начала, `eta_secs` — остаток/скорость.
  - `futures_util::stream::iter` + `buffer_unordered(DOWNLOAD_CONCURRENCY)`.
  - Ошибка первой задачи → отмена остальных, возврат `Err`.

- [ ] **Step 1: Падающие тесты** (`cargo test -p ferrolys-core --test download`):

```rust
#[tokio::test] async fn downloads_and_verifies_sha1();           // 2 файла, оба ок, файлы на месте, watch получил bytes_done==total
#[tokio::test] async fn skips_existing_file_with_matching_sha1(); // файл уже лежит → hits сервера == 0
#[tokio::test] async fn resumes_partial_download_via_range();     // .part из половины → сервер видит Range-заголовок (wiremock assert), файл после == полному, sha1 сходится
#[tokio::test] async fn sha1_mismatch_retries_then_fails();       // сервер всегда отдаёт битые байты → Err(Sha1Match…), на диске нет ни dest, ни .part
#[tokio::test] async fn io_error_returns_typed_error();           // dest путь лежит внутри существующего файла → Err(CoreError::Io)
#[tokio::test] async fn progress_reports_speed_and_eta();         // после завершения: files_done==files_total, eta_secs==Some(0) или None — фиксируем Some(0)
```

- [ ] **Step 2: FAIL**

- [ ] **Step 3: Реализовать `download.rs`**

- [ ] **Step 4: PASS + clippy** — тест resume убедиться, что реально шёл через Range (wiremock-хиты с заголовком)

- [ ] **Step 5: Коммит** `feat(install): concurrent download engine with resume, sha1 and progress`

---

### Task 5: Assets по индексу

**Files:**
- Create: `crates/ferrolys-core/src/install/assets.rs`
- Fixture: `crates/ferrolys-core/tests/fixtures/asset_index_1_21.json` (обрезанный реальный индекс: 5–10 объектов)
- Test: `crates/ferrolys-core/tests/assets.rs`

**Interfaces:**
- Consumes: `parse_*`, `DownloadTask`, `download_all` (Task 2, 4), `FerrolysPaths::assets_dir()`.
- Produces (для Task 7):
  - `pub struct AssetIndex { pub objects: Vec<AssetObject>, pub total_size: u64 }`
  - `pub struct AssetObject { pub hash: String, pub size: u64 }`
  - `pub fn parse_asset_index(bytes: &[u8]) -> Result<AssetIndex, CoreError>;`
  - `pub fn asset_tasks(index: &AssetIndex, assets_dir: &Path) -> Vec<DownloadTask>;` — url `https://resources.download.minecraft.net/{h[0..2]}/{h}`, dest `<assets_dir>/objects/{h[0..2]}/{h}`

- [ ] **Step 1: Падающие тесты** (`cargo test -p ferrolys-core --test assets`):

```rust
#[test] fn parses_asset_index();            // objects.len()==N из фикстуры, total_size посчитан
#[test] fn asset_tasks_have_hash_paths();   // dest == objects/ab/abcdef…, url содержит тот же хеш
#[tokio::test] async fn installs_assets_on_wiremock(); // wiremock отдаёт байты (sha1 считаем от них самих) → download_all → файлы на месте, progress байты сошлись
```

- [ ] **Step 2: FAIL → Step 3: реализовать → Step 4: PASS + clippy**

- [ ] **Step 5: Коммит** `feat(install): asset index parsing and download tasks`

---

### Task 6: Libraries: задачи загрузки и раскладка

**Files:**
- Create: `crates/ferrolys-core/src/install/libraries.rs`
- Test: `crates/ferrolys-core/tests/libraries.rs`

**Interfaces:**
- Consumes: Task 2 (`VersionJson`, `library_allowed`), Task 3 (`maven_path`, `expand_arch`, `extract_natives`), Task 4 (`DownloadTask`).
- Produces (для Task 7):
  - `pub fn library_tasks(vj: &VersionJson, libraries_dir: &Path) -> Vec<DownloadTask>;` — артефакты разрешённых (linux) библиотек; для каждой: `artifact` → путь из `maven_path(name)`; классификаторы natives-linux → те же пути + `-natives-linux`; url берём из `Artifact.url` (Mojang отдаёт готовые), отсутствующий url → пропуск с `warn!`.
  - `pub fn natives_targets(vj: &VersionJson, natives_dir: &Path) -> Vec<(PathBuf, Vec<String>)>;` — пары (куда распаковывать `<instances>/<id>/versions/<vj.id>/natives`, excludes библиотеки) для библиотек с `natives_linux`.

- [ ] **Step 1: Падающие тесты** (`cargo test -p ferrolys-core --test libraries`):

```rust
#[test] fn library_tasks_for_1_7_10_on_fixture();   // рукописные ожидания: количество задач, контрольный путь "net/lwjgl/lwjgl/lwjgl/2.9.4-nightly-20150209/lwjgl-2.9.4-nightly-20150209.jar" (сверить с фактическим fixture-контентом!)
#[test] fn library_tasks_for_1_21_on_fixture();     // windows-only библиотеки отсутствуют; natives-классификаторы на месте
#[test] fn natives_targets_have_excludes();         // META-INF идут в excludes из Library.extract_excludes
```

- [ ] **Step 2: FAIL → Step 3: реализовать → Step 4: PASS + clippy**

- [ ] **Step 5: Коммит** `feat(install): library download tasks and natives layout`

---

### Task 7: Оркестратор установки версии

**Files:**
- Create: `crates/ferrolys-core/src/install/orchestrator.rs`
- Test: `crates/ferrolys-core/tests/install_version.rs`
- Fixture-дерево: сервер из wiremock, отдающий version-manifest + version-json + jar + libraries + assets (по фикстурам)

**Interfaces:**
- Consumes: Task 1–6 полностью.
- Produces (для UI-волны и 7.2):
  - `pub struct InstallOptions { pub instance_id: String, pub version_id: String, pub client: reqwest::Client }`
  - `pub async fn install_version(paths: &FerrolysPaths, opts: &InstallOptions, tx: &watch::Sender<Progress>) -> Result<(), CoreError>;`
- Фазы (все внутри одного `download_all`-потока прогресса, последовательно):
  1. получить version-json по URL из манифеста (кэш `<cache_dir>/versions/<id>.json`, повторный запуск читает кэш);
  2. client jar → `<instances>/<instance_id>/versions/<version_id>/<version_id>.jar`;
  3. libraries → `<instances>/<instance_id>/libraries/…`;
  4. natives: скачать классификаторы, `extract_natives` → `…/versions/<version_id>/natives/`;
  5. assets: индекс (кэш) → `objects/…`;
  6. logging: файл конфига → `<assets_dir>/log_configs/<file.id>` (сама подмена аргумента — в 7.2).
- Атомарность: каждая фаза идемпотентна через механизм Task 4.

- [ ] **Step 1: Падающий тест** (`cargo test -p ferrolys-core --test install_version`):

```rust
#[tokio::test]
async fn full_install_builds_instance_tree();     // wiremock-дерево (fixture-манифест, fixture-json, jar, 3 библиотеки, 4 assets): install_version → проверить existence+sha1 всех файлов по списку, progress.files_done==files_total

#[tokio::test]
async fn second_install_is_noop_on_network();      // повторный вызов → hits сервера не растут (кэш+идемпотентность)
```

- [ ] **Step 2: FAIL → Step 3: реализовать `orchestrator.rs` → Step 4: PASS + clippy**

- [ ] **Step 5: Полный гейт:** `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

- [ ] **Step 6: Коммит** `feat(install): version install orchestrator (jar, libraries, natives, assets)`

---

## Definition of Done (§7.1)

- [ ] `cargo test --workspace` — все тесты (24 старых + новые) зелёные
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` чисто
- [ ] Фикстуры 1.7.10 / 1.12.2 / 1.20.1 / 1.21.1 парсятся, правила библиотек проверены
- [ ] `cargo build` без предупреждений; коммиты на каждый таск
