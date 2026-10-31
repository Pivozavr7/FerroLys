# Contributing / Участие

EN: Ferrolys is a small, wave-driven project. The development plan lives in
`ferrolys-dev-plan.md`. New ideas go to the backlog section (§16) of the plan
first, then into a wave — never straight into the current wave.

RU: Ferrolys — небольшой волновой проект. План разработки лежит в
`ferrolys-dev-plan.md`. Любые новые идеи сначала попадают в backlog (§16)
плана, и только потом — в волну. В текущую волну новое не добавляется.

## Before PR / Перед PR

- `cargo fmt --all`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

## Rules / Правила

1. No own servers, no telemetry (plan §1, принципы 1–2).
2. All UI strings go through the i18n dictionary (`i18n/*.toml`) — RU + EN.
3. `ferrolys-core` must not depend on Slint: domain logic stays testable
   without a UI.
4. Do not copy code from GPL-3.0 projects (e.g. Millida) — ideas only,
   never lines of code (plan §14.2).
5. Rust: stable toolchain, `rust-toolchain.toml` is authoritative.
