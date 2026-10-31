<div align="center">

<img src="assets/icons/app-icon.svg" width="96" height="96" alt="Ferrolys logo">

# Ferrolys

**Быстрый, честный, приватный лаунчер Minecraft.**
**A fast, honest, privacy-first Minecraft launcher.**

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable%201.99-orange.svg)](rust-toolchain.toml)
[![Slint](https://img.shields.io/badge/UI-Slint%201.18-29ADFF.svg)](https://slint.dev)
[![Platform](https://img.shields.io/badge/platform-Linux-lightgrey.svg)]()
[![CI](https://img.shields.io/badge/CI-GitHub%20Actions-2088FF.svg)](.github/workflows/ci.yml)

Rust + Slint · Material Design 3 · ru/en · Apache-2.0

</div>

---

## О проекте

Ferrolys — нативный лаунчер Minecraft, который запускается за доли секунды,
ставит ваниллу и лоадеры (Fabric, Quilt, NeoForge, Forge), работает с
аккаунтами **Ely.by** и оффлайн-профилями. Дальше по плану: каталог Modrinth,
ИИ-сборщик модпаков (BYOK) и рантайм Horizon.

### Принципы

|  | Принцип | Как это работает |
|---|---------|------------------|
| 🚫 | **Ноль своих серверов** | Всё качается напрямую с официальных источников: Mojang, метаданные лоадеров, Modrinth, GitHub Releases |
| 📵 | **Ноль телеметрии** | Сеть видит только то, что пользователь сам попросил скачать |
| 🔒 | **Приватность по умолчанию** | Токены Ely.by уходят только на `authserver.ely.by`, ключи ИИ хранятся локально (BYOK) |
| 🪶 | **Лёгкость** | Один бинарник, RAM в десятках МБ, никакого webview и JVM внутри лаунчера |
| 🔓 | **Открытость** | Apache-2.0, публичный репозиторий, релизы на GitHub Releases |

### Статус

- [x] **Волна 0 — каркас** — UI-скелет на Slint с токенами MD3, i18n RU/EN,
  асинхронный мост tokio ⇄ Slint, рабочие настройки, XDG-пути, логи с
  ротацией, CI
- [ ] **Волна 1 — MVP** *(v0.1.0)* — ванилла + лоадеры + Ely.by/оффлайн + запуск
- [ ] **Волна 2** *(v0.2.0)* — Modrinth + Horizon
- [ ] **Волна 3** *(v0.3.0)* — ИИ BYOK
- [ ] **Волна 4** *(v0.4.0)* — Windows, Microsoft-аккаунты, CurseForge

Полный план — в [ferrolys-dev-plan.md](ferrolys-dev-plan.md).

---

## Архитектура

```
Ferrolys/
├── crates/
│   ├── ferrolys/          # бинарник: окно, сборка UI (build.rs → Slint)
│   │   ├── ui/            #   исходники .slint (корень app.slint)
│   │   │   ├── md3/       #   токены, строки i18n, виджеты MD3
│   │   │   └── screens/   #   экраны: профили, аккаунты, настройки
│   │   └── src/           #   main, bridge (tokio ⇄ Slint), ui-модели
│   └── ferrolys-core/     # библиотека: доменная логика без UI
│       └── src/           #   paths (XDG), settings, theme, fonts,
│                          #   i18n (rust-i18n), logging (tracing)
├── i18n/                  # словари ru / en
├── assets/                # Inter (OFL), иконки
└── .github/workflows/     # CI
```

Правила разработки — в [AGENTS.md](AGENTS.md) (единый источник истины
для агентов и контрибьюторов).

---

## Сборка

```sh
cargo run --release
```

Требуется Rust stable (см. `rust-toolchain.toml`) и системные библиотеки
оконного рендера (Linux): `pkg-config`, `libxkbcommon-dev`,
`libxcb-render0-dev`, `libxcb-shape0-dev`, `libxcb-xfixes0-dev`,
`libwayland-dev`, `libfontconfig-dev`.

Команды разработки:

| Команда | Зачем |
|---------|-------|
| `cargo check` | быстрая проверка типов во время итераций |
| `cargo build` | полная сборка (build.rs компилирует все `.slint`) |
| `cargo clippy` | линтер, должен быть чистым |
| `cargo test` | тесты |
| `cargo run` | запуск приложения |

---

## About this project (EN)

Ferrolys is a native Minecraft launcher that starts in a fraction of a second,
installs vanilla and loaders (Fabric, Quilt, NeoForge, Forge), works with
**Ely.by** accounts and offline profiles. Coming later: a Modrinth catalog,
an AI modpack builder (BYOK) and the Horizon runtime.

**Principles:** no own servers · no telemetry · private by default ·
lightweight (one binary, no webview/JVM) · Apache-2.0.

**Status:** Wave 0 (UI skeleton: MD3 tokens, ru/en i18n, async tokio ⇄ Slint
bridge, settings, XDG paths, rotating logs, CI) — done. See the roadmap above
and [ferrolys-dev-plan.md](ferrolys-dev-plan.md) for what's next.

**Build:** `cargo run --release` — requires Rust stable and Linux windowing
libraries (see above).

---

## Лицензия / License

Apache-2.0 — см. [LICENSE](LICENSE) и [NOTICE](NOTICE).
Inter — SIL OFL 1.1 (`assets/fonts/Inter-OFL.txt`). Slint — royalty-free
лицензия Slint Dev.

**Not affiliated with Mojang, Microsoft or Ely.by.**
Minecraft — торговая марка Mojang Studios.
