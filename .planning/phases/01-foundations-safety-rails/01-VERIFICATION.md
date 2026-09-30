---
phase: 01-foundations-safety-rails
verified: 2026-09-30T20:05:00Z
status: human_needed
score: 4/4 must-haves verified
covered_files:
  - .planning/phases/01-foundations-safety-rails/01-01-PLAN.md
  - .planning/phases/01-foundations-safety-rails/01-02-PLAN.md
  - .planning/phases/01-foundations-safety-rails/01-01-SUMMARY.md
  - .planning/phases/01-foundations-safety-rails/01-02-SUMMARY.md
  - Cargo.toml
  - rust-toolchain.toml
  - crates/archel-core/src/lib.rs
  - crates/archel-core/src/error.rs
  - crates/archel-core/src/i18n.rs
  - crates/archel-core/src/privileged.rs
  - crates/archel-core/locales/fr.json
  - crates/archel-core/Cargo.toml
  - crates/archel/src/main.rs
  - crates/archel/src/cmds/mod.rs
  - crates/archel/src/gui/mod.rs
  - crates/archel/src/gui/bridge.rs
  - crates/archel/src/gui/qml/Spike.qml
  - crates/archel/build.rs
  - crates/archel/Cargo.toml
  - crates/archel-privileged/src/main.rs
  - crates/archel-privileged/Cargo.toml
  - policy/org.archel.privileged.policy
covered_digest: "v2:sha256:43a0c9c5571828a0bdafc601cfe993481b121fcf0069e02b9f1b6f19d008f4a7"
behavior_unverified: 0
overrides_applied: 0
human_verification:
  - test: "Run `QMAKE=/usr/bin/qmake6 cargo run -p archel --features gui -- settings` on the live Hyprland session and look at the window"
    expected: "Window shows a left vertical tab rail (Reseau, Alimentation, Bluetooth, Apparence, Audio) and a label whose counter updates every second"
    why_human: "Offscreen run proves QML loads and the event loop ticks with zero errors, but pixel-level adequacy (tabs look vertical, label visibly ticks at 1 Hz) needs a human eye; no screenshot tooling exists in this shell"
  - test: "Run `pkexec ./target/debug/archel-privileged dry-run` on the live desktop session; allow once, cancel once"
    expected: "Desktop polkit auth dialog appears; allow yields v1 JSON exit 0; cancel yields the friendly French cancelled error naming the action (never raw polkit text)"
    why_human: "Headless shell has no polkit agent (pkexec exits 127 with 'No such device'); the interactive dialog path cannot be exercised without a desktop session"
---

# Phase 01: Foundations & Safety Rails Verification Report

**Phase Goal:** All downstream work builds on decided foundations — workspace compiles, privilege boundary is designed, Qt viability is proven or rejected
**Verified:** 2026-09-30T20:05:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User builds the workspace with one cargo command | ✓ VERIFIED | `cargo build --workspace --no-default-features` → exit 0 (re-ran). `QMAKE=/usr/bin/qmake6 cargo build -p archel` → exit 0, zero Qt5 paths. Naming variance (below) does not change the one-command-build fact. |
| 2 | User runs a spike GUI window with vertical tabs + live label (or explicit Slint pivot) | ✓ VERIFIED | `Spike.qml`: Column-of-TabButtons rail + StackLayout + 1 Hz `Timer → spike.tick()`; `bridge.rs`: `SpikeModel` with `tick_label` NOTIFY + `tick()`. Offscreen `archel settings` holds the event loop 8 s with zero QML errors/warnings (verifier-ran). No Slint pivot needed — Qt 6.11 link succeeded on the cargo-only path. Visual polish remains a human check. |
| 3 | User invokes the pkexec helper skeleton for an allowlisted dry-run and gets structured JSON back | ✓ VERIFIED | Helper: exact-match `["dry-run"]` → `{"v":1,"action":"dry-run","ok":true}` exit 0; all else → error JSON exit 2 (verifier-ran both live). `archel privileged dry-run` → French success line, exit 0 (verifier-ran). 5 helper matrix + 11 core client tests green. Interactive dialog remains a human check. |
| 4 | User gets typed `ArchelError` diagnostics (not panics) from any core failure path | ✓ VERIFIED | `error.rs`: `AuthFailed{action,reason}` / `HelperProtocol` / `GuiNotCompiled` / `Internal`, all rendered via `message()` from `locales/fr.json` keys only. 16/16 `archel-core` tests green incl. every-variant-renders-French + diacritics audit. Only `expect()` occurrences in shipped code are inside `#[cfg(test)]` modules or the audit test itself — zero panics on production failure paths. |

**Score:** 4/4 truths verified (0 present, behavior-unverified)

**Naming variance (not a gap):** ROADMAP SC-1 names crates `archel-cli` / `archel-settings`; the locked D-01/D-02 decision (pre-approved, recorded in 01-01-SUMMARY) implements them as one `archel` binary with subcommands (`version`, `privileged dry-run`, `settings`) plus `archel-core` lib and `archel-privileged` helper. One-command build, CLI provability, and GUI entry point are all observably present — intent met under the approved layout.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `rust-toolchain.toml` | pins stable | ✓ VERIFIED | exists, stable channel |
| `Cargo.toml` | 3-member workspace, resolver 2, edition 2024 | ✓ VERIFIED | members archel / archel-core / archel-privileged; `Cargo.lock` committed |
| `crates/archel-core/src/lib.rs` | lib seams + i18n backend | ✓ VERIFIED | `version_line()`, `pub mod privileged`, re-exports |
| `crates/archel-core/src/error.rs` | typed taxonomy + catalog renderer | ✓ VERIFIED | 4 variants, `message()` only via `t!` keys; substantive (79+ lines + tests) |
| `crates/archel-core/src/i18n.rs` | locale helpers + D-07 guard | ✓ VERIFIED | diacritics audit test green |
| `crates/archel-core/locales/fr.json` | sole French source | ✓ VERIFIED | keys `app`, `privileged`, `errors`; all variants render non-empty (test) |
| `crates/archel-core/src/privileged.rs` | spawn/parse/map client, no sudo | ✓ VERIFIED | `parse_helper_stdout` v-gate, `map_pkexec_exit` 0/126/127/other, stderr log-only; only `sudo` mention is a "no sudo path" doc comment |
| `crates/archel/src/main.rs` + `cmds/mod.rs` | clap bin, bin→core calls, `message()` rendering | ✓ VERIFIED | `version` prints `version_line()`; `settings` → `gui::run_settings()` or `GuiNotCompiled`; clap only in bin |
| `crates/archel-privileged/src/main.rs` | exact-match allowlist, v1 JSON, exit 0/2, zero deps | ✓ VERIFIED | pure `handle_args`, 5 matrix tests incl. prefix-rejection |
| `crates/archel/build.rs` | cfg-gated CXX-Qt build | ✓ VERIFIED | no-op without `gui`; optional build-dep so headless never compiles Qt tooling |
| `crates/archel/src/gui/bridge.rs` | SpikeModel QObject | ✓ VERIFIED | `tick_label`/`tick_count` Q_PROPERTYs + `tick()` invokable (note: code uses `core::pin::Pin` path and still codegens clean — SUMMARY deviation-3 narrative is stale but harmless) |
| `crates/archel/src/gui/qml/Spike.qml` | vertical tabs + 1 Hz label | ✓ VERIFIED | TabButton column + StackLayout, French labels, static QML |
| `policy/org.archel.privileged.policy` | least-privilege draft | ✓ VERIFIED | 1 dry-run action, 4x `auth_admin_keep` (3 defaults + comment), 0 `yes` rules; mirrors hostname1.policy shape |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| archel bin | archel-core lib | path dep; `version` → `version_line()`, `privileged dry-run` → `dry_run_direct()` + `dry_run_success_line()` | ✓ WIRED | live CLI output proves both calls end-to-end |
| error Display/message | locales/fr.json | `t!` keys, sole French source | ✓ WIRED | diacritics audit + every-variant test green; no French literals in `.rs` |
| archel bin | pkexec → helper → v1 JSON → core parser | `run_action_via_pkexec`, `v == 1` gate | ✓ WIRED | direct-spawn path exercised live; pkexec path unit-covered (0/126/127/other); interactive dialog is human-check #2 |
| settings subcommand | gui feature → SpikeModel → Spike.qml | `QGuiApplication` + engine load of `qrc:/…Spike.qml`; Timer→tick→NOTIFY | ✓ WIRED | Qt build links Qt6 only; offscreen run holds loop, zero QML errors |
| pkexec 126/127 | AuthFailed → fr.json | `map_pkexec_exit` + cancelled/denied catalog strings | ✓ WIRED | distinct French strings naming the action asserted in tests; no `GDBus` leakage asserted |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `archel version` line | `version_line()` | `archel-core` lib constant via catalog key | ✓ FLOWING | live output `Archel version 0.1.0 — moteur archel-core 0.1.0` |
| `privileged dry-run` line | helper stdout JSON → `parse_helper_stdout` → catalog line | live `archel-privileged` process spawn | ✓ FLOWING | live output `Module privilégié vérifié…`, not a static print |
| spike label | `spike.tick_label` | Rust `SpikeModel.tick()` via NOTIFY | ✓ WIRED (visual tick is human-check #1) | binding present; offscreen proves loop runs |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| workspace builds headless | `cargo build --workspace --no-default-features` | Finished, exit 0 | ✓ PASS |
| Qt build links Qt6 | `QMAKE=/usr/bin/qmake6 cargo build -p archel` | Finished, exit 0, no Qt5 paths | ✓ PASS |
| headless test suite | `cargo test --workspace --no-default-features` | 16 core + 5 helper passed, 0 failed | ✓ PASS |
| `archel version` proves bin→core | `cargo run -q -p archel --no-default-features -- version` | `Archel version 0.1.0 — moteur archel-core 0.1.0` | ✓ PASS |
| helper allowlist live | `./target/debug/archel-privileged dry-run` / `install foo` | v1 ok-JSON exit 0 / error-JSON exit 2 | ✓ PASS |
| spike boots, QML clean | `QT_QPA_PLATFORM=offscreen timeout 8 ./target/debug/archel settings` | loop held full 8 s, zero QML errors/warnings | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| PLAT-01 | 01-01 | shared `archel-core` lib as programmatic interface | ✓ SATISFIED | lib exposes version/error/i18n/privileged seams; bin consumes them (no orphaned requirement) |
| PLAT-04 | 01-02 | privileged ops via allowlisted pkexec helper; yay unprivileged | ✓ SATISFIED (Phase 1 scope) | allowlist + v1 JSON + 126/127 mapping + policy draft; no sudo path; real package actions land in Phase 3 |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | none | — | No TBD/FIXME/XXX/TODO/placeholder/console.log/`return null` in any phase file; `.qmlls.ini` generator output correctly git-ignored |

### Human Verification Required

Deferred-by-design from both plans (`<human-check>` blocks harvested, deduplicated to 2 items — see frontmatter for test/expected/why):

1. **Spike visual on live Hyprland** — vertical tab rail + 1 Hz ticking label need a human eye (offscreen proves load + loop, not pixels).
2. **pkexec desktop dialog** — allow/cancel paths need a desktop polkit agent (headless shell provably has none; unit matrix covers the mapping logic).

### Gaps Summary

No gaps. All 4 roadmap success criteria are observably true in the codebase with live behavioral evidence. The two human items are plan-deferred checks requiring a desktop session, not implementation deficits — automated checks passed; awaiting human verification.

---
_Verified: 2026-09-30T20:05:00Z_
_Verifier: the agent (gsd-verifier)_
