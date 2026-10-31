# Ferrolys <img src="assets/icons/app-icon.svg" width="64" height="64" alt="">

**Быстрый, честный, приватный лаунчер Minecraft. / A fast, honest, privacy-first Minecraft launcher.**

Rust + Slint · Linux first · Material Design 3 · Apache-2.0

---

## RU

Ferrolys — нативный лаунчер Minecraft, который запускается за доли секунды,
ставит ваниллу и лоадеры (Fabric, Quilt, NeoForge, Forge), работает с
аккаунтами **Ely.by** и оффлайн-профилями. Дальше по плану: каталог Modrinth,
ИИ-сборщик модпаков (BYOK) и рантайм Horizon.

### Принципы

1. **Ноль своих серверов.** Всё качается напрямую с официальных источников:
   Mojang, метаданные лоадеров, Modrinth, GitHub Releases.
2. **Ноль телеметрии.** Сеть видит только то, что пользователь сам попросил
   скачать.
3. **Приватность по умолчанию.** Токены Ely.by уходят только на
   `authserver.ely.by`, ключи ИИ хранятся локально (BYOK).
4. **Лёгкость.** Один бинарник, RAM в десятках МБ, никакого webview и JVM
   внутри лаунчера.
5. **Открытость.** Apache-2.0, публичный репозиторий, релизы на GitHub
   Releases.

### Статус

Волна 0 (каркас): UI-скелет на Slint с токенами MD3, i18n RU/EN, асинхронный
мост tokio ⇄ Slint, рабочие настройки, XDG-пути, логи с ротацией, CI.

- [ ] Волна 0 — каркас
- [ ] Волна 1 — MVP: ванилла + лоадеры + Ely.by/оффлайн + запуск (v0.1.0)
- [ ] Волна 2 — Modrinth + Horizon (v0.2.0)
- [ ] Волна 3 — ИИ BYOK (v0.3.0)
- [ ] Волна 4 — Windows, Microsoft-аккаунты, CurseForge (v0.4.0)

### Сборка

```sh
cargo run --release
```

Требуется Rust stable (см. `rust-toolchain.toml`) и системные библиотеки для
оконного рендера (Linux: `pkg-config`, `libxkbcommon-dev`,
`libxcb-render0-dev`, `libxcb-shape0-dev`, `libxcb-xfixes0-dev`,
`libwayland-dev`, `libfontconfig-dev`).

### Лицензия

Apache-2.0 (см. `LICENSE` и `NOTICE`). Inter — SIL OFL 1.1. Slint —
royalty-free лицензия Slint Dev.

**Not affiliated with Mojang, Microsoft или Ely.by.** Minecraft — торговая
марка Mojang Studios.

---

## EN

Ferrolys is a native Minecraft launcher that starts in a fraction of a second,
installs vanilla and loaders (Fabric, Quilt, NeoForge, Forge), works with
**Ely.by** accounts and offline profiles. Coming later: a Modrinth catalog, an
AI modpack builder (BYOK) and the Horizon runtime.

### Principles

1. **No own servers.** Everything is downloaded directly from official
   sources: Mojang, loader metadata, Modrinth, GitHub Releases.
2. **No telemetry.** The network only ever sees what the user asked to
   download.
3. **Private by default.** Ely.by tokens go only to `authserver.ely.by`;
   AI keys stay local (BYOK).
4. **Lightweight.** One binary, tens of MB of RAM, no webview or JVM inside
   the launcher.
5. **Open.** Apache-2.0, public repository, releases on GitHub Releases.

### Status

Wave 0 (skeleton): Slint UI shell with MD3 tokens, RU/EN i18n, a tokio ⇄ Slint
async bridge, working settings, XDG paths, rotating logs, CI.

### Build

```sh
cargo run --release
```

Requires Rust stable (see `rust-toolchain.toml`) and windowing system
libraries on Linux (see above).

### License

Apache-2.0 (see `LICENSE` and `NOTICE`). Inter — SIL OFL 1.1. Slint — royalty-free
license by Slint Dev.

**Not affiliated with Mojang, Microsoft or Ely.by.** Minecraft is a trademark
of Mojang Studios.
