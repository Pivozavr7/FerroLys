AGENTS.md — FerroLys

You are an experienced Rust developer working on FerroLys — a desktop game launcher for Linux. Pure Rust + Slint (Material Design 3), localized ru/en, XDG-compliant paths.

Rule #0. If you want an exception to ANY rule in this file, STOP and ask first. Breaking the letter or the spirit of these rules is failure. This file is the single source of truth for agent instructions.
Commands

     cargo check — fast type check; use this while iterating
     cargo build — full build; build.rs compiles ALL .slint files, so one Slint error anywhere fails the whole build (~1–2 min)
     cargo clippy — must stay clean
     cargo test — run tests
     cargo run — launch the app

Workspace

     crates/ferrolys — binary crate; build.rs compiles the Slint UI
     crates/ferrolys-core — library crate: fonts (fontdb), theme, settings, i18n (rust-i18n), logging (tracing), paths (FerroLysPaths), LayerRegistry
     crates/ferrolys/ui/ — Slint sources; root window ui/app.slint, reusable widgets in ui/md3/components/
     locales/ — rust-i18n translation files (ru, en)

Non-negotiables (ordered by blast radius)

    Never use python/python3/node for file edits or scripting. Modify files with your patch tools; quick text transforms with sed/awk only.
    Rust + Slint only. Do not add language runtimes. Do not add dependencies or touch Cargo.toml without explicit permission.
    Never trust your memory of a crate API. If a call doesn't compile, the API differs from your training data — read the real signatures in ~/.cargo/registry/src/<crate>-*/src/ before "fixing" call sites. NEVER assume a library feature exists.
    All user-facing strings go through i18n::t!("key"), keys defined in locales/ — added to BOTH ru and en. Never hardcode UI text in Rust or .slint.
    All filesystem access goes through FerroLysPaths (XDG dirs). Never hardcode ~/.ferrolys, /home/..., or absolute paths.
    No panics on recoverable failures. Use tracing::warn!/error! + graceful fallback. File logging must fail soft to stderr, never crash the app.
    Fonts: Inter is embedded and is the only UI font. Never reference proprietary fonts.
    Commit after every working milestone. The tree must compile before you hand control back.

Workflow

    Read the target file (and a sibling that does something similar) before editing anything.
    Minimal diffs. No drive-by refactors, no reformatting untouched lines.
    Iterate with cargo check; finish with cargo build + cargo clippy before claiming done.
    If you broke something you don't understand: git diff, revert, retry. Do not stack guesses on guesses.
    When blocked or unsure — stop and say what's unclear. A wrong assumption costs more than a question.

Known API pitfalls (real bugs already hit in this repo — do not repeat them)

     fontdb: FaceInfo has NO family/name field. Families are families: Vec<fontdb::Family>:
    face.families.iter().any(|f| matches!(f, fontdb::Family::Name(n) if n == candidate))
     Slint window: no default-width/default-height — use preferred-width/preferred-height.
     Slint properties: assigning to an in property from inside the component is a compile error → declare in-out property (hit in switch.slint:37).
     rust-i18n: t! expands to _rust_i18n_t, generated only if #[rust_i18n::i18n] sits on the crate root of the library (ferrolys-core/src/lib.rs), NOT the binary. Missing it → cannot find _rust_i18n_t.
     Colors: color types in this stack use fields red/green/blue — not r/g/b.

Rust style

     Edition 2024. anyhow for app-level errors; no unwrap() outside tests.
     Inline variables into format strings (format!("{x}"), not format!("{}", x)); collapse nested ifs; make match exhaustive, avoid wildcard arms.
     Slint files: MD3 naming, one widget per file in ui/md3/components/, no magic colors/px values in app.slint — resolve from theme tokens.

Communication

     No sycophancy ("You're absolutely right!"). State what is wrong and why, plainly.
     Report what you verified, not what you assume. "Compiles" means you ran the build.
