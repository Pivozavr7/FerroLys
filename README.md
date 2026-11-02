<div align="center">

<img src="assets/icons/app-icon.svg" alt="Ferrolys logo" width="120" height="120">

# Ferrolys

**A fast, honest, privacy-first Minecraft launcher.**

<p>
  <a href="LICENSE"><img alt="License: Apache-2.0" src="https://img.shields.io/github/license/Pivozavr7/FerroLys?style=for-the-badge&logo=apache&logoColor=white&color=D22128"></a>
  <a href="rust-toolchain.toml"><img alt="Rust stable" src="https://img.shields.io/badge/Rust-stable-000000?style=for-the-badge&logo=rust&logoColor=white"></a>
  <a href="https://slint.dev"><img alt="UI: Slint" src="https://img.shields.io/badge/UI-Slint-2379F4?style=for-the-badge&logo=slint&logoColor=white"></a>
  <img alt="Platform: Linux" src="https://img.shields.io/badge/Linux-only_for_now-FCC624?style=for-the-badge&logo=linux&logoColor=black">
  <a href=".github/workflows/ci.yml"><img alt="CI status" src="https://img.shields.io/github/actions/workflow/status/Pivozavr7/FerroLys/ci.yml?branch=master&style=for-the-badge&logo=githubactions&logoColor=white&label=CI"></a>
</p>

Rust + Slint · Material Design 3 · Russian / English · Apache-2.0

</div>

---

> [!WARNING]
> **Early development.** Ferrolys can already download and install Minecraft files, but it **can't launch the game yet**. The first usable release is planned as v0.1.0. Until then, this repo is for people who want to follow along or poke at the code.

## Why another launcher?

Because I wanted a launcher that opens in a fraction of a second, doesn't phone home, and doesn't hide what it's doing.

Ferrolys is a native Rust app. No Electron, no bundled JVM, no webview. It installs vanilla Minecraft and the main loaders (Fabric, Quilt, NeoForge, Forge), signs in through [Ely.by](https://ely.by), and also supports offline profiles. Later on: a Modrinth catalog, an AI modpack builder where you bring your own key, and the Horizon runtime.

### Principles I won't break

| | Principle | What it means in practice |
|---|---|---|
| 🚫 | **No servers of our own** | Everything is downloaded straight from official sources: Mojang, loader metadata, Modrinth, GitHub Releases. |
| 📵 | **No telemetry** | The network only sees what you asked it to download. Nothing else. |
| 🔒 | **Private by default** | Ely.by tokens go only to `authserver.ely.by`. AI keys never leave your machine (BYOK). |
| 🪶 | **Lightweight** | One binary, tens of MB of RAM, nothing heavy hiding inside. |
| 🔓 | **Open** | Apache-2.0, public repo, releases on GitHub Releases. |

## Honest note: this project is vibe-coded

A big part of Ferrolys is written together with AI coding agents. I decide what gets built, how it's structured and what the rules are (they live in `AGENTS.md`). The agent writes a lot of the code. I run it, look at the changes, and keep `cargo clippy` and `cargo test` green.

That's the deal, and I'd rather say it out loud than have you find out from the commit history. What it means for you:

- **Expect bugs.** Tests help, but they don't prove the code is right. Treat Ferrolys like any young project and read the code before you trust it, especially the parts that handle downloads, archives and credentials.
- **The code can look uneven** in places. I clean it up as I go.
- **Review is very welcome.** Bug reports, failing test cases and critical code review help more than stars.
- **AI-assisted pull requests are fine**, as long as you understand what you're submitting and have actually run it. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Where it stands

- [x] **Wave 0: skeleton.** Slint UI with MD3 tokens, ru/en i18n, the async tokio ⇄ Slint bridge, working settings, XDG paths, rotating logs, CI.
- [x] **Wave 1: core install.** piston-meta client, version JSON parser (1.7.10 → 1.21.x), concurrent download engine (resume, SHA-1, progress), assets / libraries / natives install.
- [ ] **Wave 1: the rest of the MVP** *(v0.1.0)*. Game launch, Java, Ely.by and offline accounts, loaders.
- [ ] **Wave 2** *(v0.2.0)*. Modrinth + Horizon.
- [ ] **Wave 3** *(v0.3.0)*. AI modpack builder (BYOK).
- [ ] **Wave 4** *(v0.4.0)*. Windows, Microsoft accounts, CurseForge.

## Architecture

```
Ferrolys/
├── crates/
│   ├── ferrolys/          # binary: window, UI build (build.rs → Slint)
│   │   ├── ui/            #   .slint sources (app.slint at the root)
│   │   │   ├── md3/       #   tokens, i18n strings, MD3 widgets
│   │   │   └── screens/   #   screens: profiles, accounts, settings
│   │   └── src/           #   main, bridge (tokio ⇄ Slint), UI models
│   └── ferrolys-core/     # domain library, no UI inside
│       └── src/           #   paths (XDG), settings, theme, fonts,
│                          #   i18n (rust-i18n), logging (tracing),
│                          #   install/ (manifests, downloads, assets…)
├── i18n/                  # ru / en dictionaries
├── assets/                # Inter (OFL), icons
└── .github/workflows/     # CI
```

The project rules for agents and contributors live in `AGENTS.md`.

## Building

```bash
cargo run --release
```

You need Rust stable (see `rust-toolchain.toml`) and the usual Linux windowing libraries:
`pkg-config`, `libxkbcommon-dev`, `libxcb-render0-dev`, `libxcb-shape0-dev`, `libxcb-xfixes0-dev`, `libwayland-dev`, `libfontconfig-dev`.

Everyday commands:

| Command | What it's for |
|---|---|
| `cargo check` | Quick type check while you iterate |
| `cargo build` | Full build (`build.rs` compiles every `.slint`) |
| `cargo clippy` | Linter. It has to stay clean |
| `cargo test` | Tests |
| `cargo run` | Start the app |

## Special thanks

Ferrolys stands on the shoulders of some great open-source work:

- [**Slint**](https://slint.dev): the UI toolkit. Native, declarative, and a pleasure to write `.slint` files in.
- [**tokio**](https://tokio.rs): the async runtime under the hood.
- [**reqwest**](https://github.com/seanmonstar/reqwest): HTTP client with rustls.
- [**futures-util**](https://docs.rs/futures-util): `buffer_unordered` powers the download engine.
- [**serde**](https://serde.rs), [**serde_json**](https://github.com/serde-rs/json), [**toml**](https://github.com/toml-rs/toml): everything that needs parsing and everything that needs saving.
- [**thiserror**](https://github.com/dtolnay/thiserror): typed errors in the core.
- [**anyhow**](https://github.com/dtolnay/anyhow): error handling at the app level.
- [**tracing**](https://github.com/tokio-rs/tracing) with `tracing-subscriber` and `tracing-appender`: logs that rotate and never crash the app.
- [**rust-i18n**](https://github.com/chenyukang/rust-i18n): ru/en localization.
- [**material-colors**](https://github.com/nicklimmm/material-colors): Material Design 3 palettes from a seed color.
- [**material-color-utilities**](https://github.com/material-foundation/material-color-utilities): the M3 color science behind those palettes.
- [**fontdb**](https://github.com/RazrFalcon/fontdb): system font detection (Inter itself is embedded).
- [**dirs**](https://github.com/dirs-dev/dirs-rs): XDG directories.
- [**sha1**, **sha2**, **md-5**](https://github.com/RustCrypto/hashes): Mojang and Modrinth ask for hashes, so we speak hash.
- [**zip**](https://github.com/zip-rs/zip2): natives jars and, soon, `.mrpack` imports.
- [**semver**](https://github.com/dtolnay/semver): version comparison.
- [**wiremock**](https://github.com/LukeMathWalker/wiremock): makes network tests a joy to write.
- [**keyring**](https://github.com/open-source-cooperative/keyring-rs) *(planned)*: credentials in the Secret Service, not in a file.
- [**rfd**](https://github.com/PolyMeilex/rfd) *(planned)*: native file dialogs.
- [**Inter**](https://rsms.me/inter/) by Rasmus Andersson: the UI font, embedded under the SIL OFL 1.1.
- [**Ely.by**](https://ely.by): the auth service this launcher is built around, and a big thank-you to its community.

If your project powers Ferrolys and you're not on this list, open an issue and I'll fix that.

## License

Apache-2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
Inter is licensed under the SIL OFL 1.1 (`assets/fonts/Inter-OFL.txt`).
The UI is built with [Slint](https://slint.dev), used under the royalty-free license.

**Not affiliated with Mojang, Microsoft or Ely.by.** Minecraft is a trademark of Mojang Studios.
