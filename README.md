<div align="center">

<img src="assets/icons/app-icon.svg" width="96" height="96" alt="Ferrolys logo">

# Ferrolys

**A fast, honest, privacy-first Minecraft launcher.**

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable%201.99-orange.svg)](rust-toolchain.toml)
[![Slint](https://img.shields.io/badge/UI-Slint%201.18-29ADFF.svg)](https://slint.dev)
[![Platform](https://img.shields.io/badge/platform-Linux-lightgrey.svg)]()
[![CI](https://img.shields.io/badge/CI-GitHub%20Actions-2088FF.svg)](.github/workflows/ci.yml)

Rust + Slint · Material Design 3 · ru/en · Apache-2.0

</div>

---

## Why another launcher?

Honestly? Because we wanted a launcher that starts in a fraction of a second,
doesn't phone home, and doesn't hide what it's doing. Ferrolys is a native
Rust app — no Electron, no bundled JVM, no webview. It installs vanilla
Minecraft and loaders (Fabric, Quilt, NeoForge, Forge), talks to
[Ely.by](https://ely.by) for accounts and supports offline profiles.
Down the road: a Modrinth catalog, an AI modpack builder you bring your own
key to, and the Horizon runtime.

### The principles we don't break

|  | Principle | In practice |
|---|-----------|-------------|
| 🚫 | **No servers of our own** | Everything downloads straight from the official sources: Mojang, loader metadata, Modrinth, GitHub Releases |
| 📵 | **No telemetry** | The network only ever sees what you asked it to download |
| 🔒 | **Private by default** | Ely.by tokens go only to `authserver.ely.by`, AI keys never leave your machine (BYOK) |
| 🪶 | **Lightweight** | One binary, tens of MB of RAM, nothing heavy inside the launcher |
| 🔓 | **Open** | Apache-2.0, public repo, releases on GitHub Releases |

### Where it stands

- [x] **Wave 0 — skeleton** — Slint UI with MD3 tokens, ru/en i18n, the
  async tokio ⇄ Slint bridge, working settings, XDG paths, rotating logs, CI
- [x] **Wave 1 — core install** — piston-meta client, version-JSON parser
  (1.7.10 → 1.21.x), concurrent download engine (resume, SHA-1, progress),
  assets / libraries / natives install
- [ ] **Wave 1 — rest of the MVP** *(v0.1.0)* — launch, Java, Ely.by/offline
  accounts, loaders
- [ ] **Wave 2** *(v0.2.0)* — Modrinth + Horizon
- [ ] **Wave 3** *(v0.3.0)* — AI modpack builder (BYOK)
- [ ] **Wave 4** *(v0.4.0)* — Windows, Microsoft accounts, CurseForge

The full plan lives in [ferrolys-dev-plan.md](ferrolys-dev-plan.md).

---

## Architecture

```
Ferrolys/
├── crates/
│   ├── ferrolys/          # binary: window, UI build (build.rs → Slint)
│   │   ├── ui/            #   .slint sources (app.slint at the root)
│   │   │   ├── md3/       #   tokens, i18n strings, MD3 widgets
│   │   │   └── screens/   #   screens: profiles, accounts, settings
│   │   └── src/           #   main, bridge (tokio ⇄ Slint), UI models
│   └── ferrolys-core/     # domain library, UI-free
│       └── src/           #   paths (XDG), settings, theme, fonts,
│                          #   i18n (rust-i18n), logging (tracing),
│                          #   install/ (manifests, downloads, assets…)
├── i18n/                  # ru / en dictionaries
├── assets/                # Inter (OFL), icons
└── .github/workflows/     # CI
```

Development rules live in `AGENTS.md` — the single source of truth for
agents and contributors alike.

---

## Building

```sh
cargo run --release
```

You'll need Rust stable (see `rust-toolchain.toml`) and the usual Linux
windowing libraries: `pkg-config`, `libxkbcommon-dev`,
`libxcb-render0-dev`, `libxcb-shape0-dev`, `libxcb-xfixes0-dev`,
`libwayland-dev`, `libfontconfig-dev`.

Everyday commands:

| Command | What it's for |
|---------|---------------|
| `cargo check` | fast type check while iterating |
| `cargo build` | full build (build.rs compiles every `.slint`) |
| `cargo clippy` | linter — it must stay clean |
| `cargo test` | tests |
| `cargo run` | launch the app |

---

## Special thanks

Ferrolys stands on the shoulders of some genuinely great open-source work.
In no particular order:

- **[Slint](https://slint.dev)** — the UI toolkit; native, declarative,
  and a pleasure to write `.slint` files for
- **[tokio](https://tokio.rs)** — async runtime under the hood
- **[reqwest](https://github.com/seanmonstar/reqwest)** — HTTP client with
  rustls
- **[futures-util](https://docs.rs/futures-util)** — `buffer_unordered`
  powers the download engine
- **[serde](https://serde.rs)** / **[serde_json](https://github.com/serde-rs/json)**
  / **[toml](https://github.com/toml-rs/toml)** — everything that needs
  parsing, and everything that needs saving
- **[thiserror](https://github.com/dtolnay/thiserror)** — typed errors in
  the core
- **[anyhow](https://github.com/dtolnay/anyhow)** — error handling at the
  app level
- **[tracing](https://github.com/tokio-rs/tracing)** /
  **[tracing-subscriber](https://github.com/tokio-rs/tracing)** /
  **[tracing-appender](https://github.com/tokio-rs/tracing)** — logs that
  rotate and never crash the app
- **[rust-i18n](https://github.com/chenyukang/rust-i18n)** — ru/en
  localization
- **[material-colors](https://github.com/nicklimmm/material-colors)** —
  Material Design 3 palettes from a seed accent
- **[fontdb](https://github.com/RazrFalcon/fontdb)** — system font
  detection (Inter itself is embedded)
- **[dirs](https://github.com/dirs-dev/dirs-rs)** — XDG directories
- **[sha1](https://github.com/RustCrypto/hashes)** /
  **[sha2](https://github.com/RustCrypto/hashes)** /
  **[md-5](https://github.com/RustCrypto/hashes)** — Mojang and Modrinth
  ask for hashes, so we speak hash
- **[zip](https://github.com/zip-rs/zip2)** — natives jars and (soon)
  `.mrpack` imports
- **[semver](https://github.com/dtolnay/semver)** — version comparisons
- **[keyring](https://github.com/open-source-cooperative/keyring-rs)** —
  credentials in the Secret Service, not in a file *(planned)*
- **[rfd](https://github.com/PolyMeilex/rfd)** — native file dialogs
  *(planned)*
- **[wiremock](https://github.com/LukeMathWalker/wiremock)** — makes the
  network tests a joy to write
- **[material-color-utilities](https://github.com/material-foundation/material-color-utilities)**
  — the M3 color science this palette derives from
- **[Inter](https://rsms.me/inter/)** by Rasmus Andersson — the UI font,
  embedded under the SIL OFL 1.1
- **[Ely.by](https://ely.by)** — the auth service this launcher is built
  around, and a big thank you to the community there

If your project powers Ferrolys and you're not on this list — open an issue
and we'll fix that.

---

## License

Apache-2.0 — see [LICENSE](LICENSE) and [NOTICE](NOTICE).
Inter is licensed under the SIL OFL 1.1 (`assets/fonts/Inter-OFL.txt`).
Slint is used under the royalty-free [Slint Dev license](https://slint.dev/community.html).

**Not affiliated with Mojang, Microsoft or Ely.by.**
Minecraft is a trademark of Mojang Studios.
