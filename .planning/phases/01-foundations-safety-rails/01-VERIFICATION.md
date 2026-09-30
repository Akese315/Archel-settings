---
phase: 01-foundations-safety-rails
verified: 2026-09-30T19:54:15Z
status: human_needed
score: 4/4 must-haves verified
covered_files:
  - .planning/phases/01-foundations-safety-rails/01-01-PLAN.md
  - .planning/phases/01-foundations-safety-rails/01-01-SUMMARY.md
  - .planning/phases/01-foundations-safety-rails/01-02-PLAN.md
  - .planning/phases/01-foundations-safety-rails/01-02-SUMMARY.md
  - Cargo.toml
  - crates/archel-core/Cargo.toml
  - crates/archel-core/locales/fr.json
  - crates/archel-core/src/error.rs
  - crates/archel-core/src/i18n.rs
  - crates/archel-core/src/lib.rs
  - crates/archel-core/src/privileged.rs
  - crates/archel-privileged/Cargo.toml
  - crates/archel-privileged/src/main.rs
  - crates/archel/Cargo.toml
  - crates/archel/build.rs
  - crates/archel/src/cmds/mod.rs
  - crates/archel/src/gui/bridge.rs
  - crates/archel/src/gui/mod.rs
  - crates/archel/src/gui/qml/Spike.qml
  - crates/archel/src/main.rs
  - policy/org.archel.privileged.policy
  - rust-toolchain.toml
covered_digest: "v2:sha256:2d9078d7eee45ec9e8226de53de5b643fe59f101aeb4c3968e9f21fbd32e44c4"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 4/4
  gaps_closed: []
  gaps_remaining: []
  regressions: []
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
**Verified:** 2026-09-30T19:54:15Z
**Status:** human_needed
**Re-verification:** Yes — digest refresh after roadmap/state commits; zero source changes since prior verification (f9361b0)

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User builds the workspace with one cargo command | ✓ VERIFIED | `git diff f9361b0..HEAD -- crates/ policy/ Cargo.toml Cargo.lock rust-toolchain.toml` → empty (no source drift). `cargo test --workspace --no-default-features` re-ran → 16 core + 5 helper passed, 0 failed. Naming variance (below) does not change the one-command-build fact. |
| 2 | User runs a spike GUI window with vertical tabs + live label (or explicit Slint pivot) | ✓ VERIFIED | `Spike.qml`: Column-of-TabButtons rail + StackLayout + 1 Hz `Timer → spike.tick()` (grep-confirmed). `bridge.rs`: `SpikeModel` with `tick_label` NOTIFY + `tick()`. Qt 6.11 link previously proven on the cargo-only path with zero Qt5 paths; no Slint pivot needed. Visual polish remains human-check #1 (dispositioned below). |
| 3 | User invokes the pkexec helper skeleton for an allowlisted dry-run and gets structured JSON back | ✓ VERIFIED | Live re-run: `./target/debug/archel-privileged dry-run` → `{"v":1,"action":"dry-run","ok":true}` exit 0; `install foo` → error JSON exit 2. `archel version` → `Archel version 0.1.0 — moteur archel-core 0.1.0` (bin→core proven live). 5 helper matrix + 16 core tests green. Interactive dialog remains human-check #2 (dispositioned below). |
| 4 | User gets typed `ArchelError` diagnostics (not panics) from any core failure path | ✓ VERIFIED | `error.rs`: `AuthFailed{action,reason}` / `HelperProtocol` / `GuiNotCompiled` / `Internal`, rendered via `message()` from `locales/fr.json` keys only. Full headless suite green incl. every-variant-renders-French + diacritics audit. Only `sudo` mention in core/helper is a "no sudo path" doc comment. |

**Score:** 4/4 truths verified (0 present, behavior-unverified)

**Naming variance (not a gap):** ROADMAP SC-1 names crates `archel-cli` / `archel-settings`; the locked D-01/D-02 decision (pre-approved, recorded in 01-01-SUMMARY) implements them as one `archel` binary with subcommands (`version`, `privileged dry-run`, `settings`) plus `archel-core` lib and `archel-privileged` helper. One-command build, CLI provability, and GUI entry point are all observably present — intent met under the approved layout.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `rust-toolchain.toml` | pins stable | ✓ VERIFIED | exists, stable channel |
| `Cargo.toml` | 3-member workspace, resolver 2, edition 2024 | ✓ VERIFIED | members archel / archel-core / archel-privileged; `Cargo.lock` committed |
| `crates/archel-core/src/lib.rs` | lib seams + i18n backend | ✓ VERIFIED | `version_line()`, `pub mod privileged`, re-exports |
| `crates/archel-core/src/error.rs` | typed taxonomy + catalog renderer | ✓ VERIFIED | 4 variants, `message()` only via `t!` keys; substantive + tests |
| `crates/archel-core/src/i18n.rs` | locale helpers + D-07 guard | ✓ VERIFIED | diacritics audit test green (in 16-test suite) |
| `crates/archel-core/locales/fr.json` | sole French source | ✓ VERIFIED | `app` + `privileged` + `errors` keys; every-variant test green |
| `crates/archel-core/src/privileged.rs` | spawn/parse/map client, no sudo | ✓ VERIFIED | `parse_helper_stdout` v-gate, `map_pkexec_exit` 0/126/127/other, stderr log-only |
| `crates/archel/src/main.rs` + `cmds/mod.rs` | clap bin, bin→core calls, `message()` rendering | ✓ VERIFIED | `version` prints `version_line()` (live); `settings` → `gui::run_settings()` or `GuiNotCompiled`; clap only in bin |
| `crates/archel-privileged/src/main.rs` | exact-match allowlist, v1 JSON, exit 0/2, zero deps | ✓ VERIFIED | exercised live both paths; 5 matrix tests incl. prefix-rejection |
| `crates/archel/build.rs` | cfg-gated CXX-Qt build | ✓ VERIFIED | no-op without `gui`; optional build-dep so headless never compiles Qt tooling |
| `crates/archel/src/gui/bridge.rs` | SpikeModel QObject | ✓ VERIFIED | `tick_label`/`tick_count` Q_PROPERTYs + `tick()` invokable |
| `crates/archel/src/gui/qml/Spike.qml` | vertical tabs + 1 Hz label | ✓ VERIFIED | TabButton column + StackLayout, French labels, static QML (grep-confirmed) |
| `policy/org.archel.privileged.policy` | least-privilege draft | ✓ VERIFIED | 4x `auth_admin_keep`, 0 `yes` rules (grep-confirmed); single dry-run action |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| archel bin | archel-core lib | path dep; `version` → `version_line()`, `privileged dry-run` → core client | ✓ WIRED | live CLI output proves both calls end-to-end |
| error Display/message | locales/fr.json | `t!` keys, sole French source | ✓ WIRED | diacritics audit + every-variant test green |
| archel bin | pkexec → helper → v1 JSON → core parser | `run_action_via_pkexec`, `v == 1` gate | ✓ WIRED | direct-spawn path exercised live; pkexec path unit-covered (0/126/127/other); interactive dialog is human-check #2 |
| settings subcommand | gui feature → SpikeModel → Spike.qml | `QGuiApplication` + engine load of `qrc:/…Spike.qml`; Timer→tick→NOTIFY | ✓ WIRED | Qt build links Qt6 only; offscreen run previously held loop with zero QML errors |
| pkexec 126/127 | AuthFailed → fr.json | `map_pkexec_exit` + cancelled/denied catalog strings | ✓ WIRED | distinct French strings naming the action asserted in tests |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `archel version` line | `version_line()` | `archel-core` lib constant via catalog key | ✓ FLOWING | live output `Archel version 0.1.0 — moteur archel-core 0.1.0` |
| `privileged dry-run` line | helper stdout JSON → `parse_helper_stdout` → catalog line | live `archel-privileged` process spawn | ✓ FLOWING | live JSON verified, not a static print |
| spike label | `spike.tick_label` | Rust `SpikeModel.tick()` via NOTIFY | ✓ WIRED (visual tick is human-check #1) | binding present; offscreen previously proved loop runs |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| headless test suite | `cargo test --workspace --no-default-features` | 16 core + 5 helper passed, 0 failed | ✓ PASS |
| `archel version` proves bin→core | `cargo run -q -p archel --no-default-features -- version` | `Archel version 0.1.0 — moteur archel-core 0.1.0` | ✓ PASS |
| helper allow | `./target/debug/archel-privileged dry-run` | v1 ok-JSON, exit 0 | ✓ PASS |
| helper reject | `./target/debug/archel-privileged install foo` | error-JSON, exit 2 | ✓ PASS |

(Qt link + offscreen spike checks carried from prior verification — Qt artifacts untouched since, `git diff` empty on all spike files; not re-run to keep this refresh fast.)

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| PLAT-01 | 01-01 | shared `archel-core` lib as programmatic interface | ✓ SATISFIED | lib exposes version/error/i18n/privileged seams; bin consumes them (no orphaned requirement) |
| PLAT-04 | 01-02 | privileged ops via allowlisted pkexec helper; yay unprivileged | ✓ SATISFIED (Phase 1 scope) | allowlist + v1 JSON + 126/127 mapping + policy draft; no sudo path; real package actions land in Phase 3 |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | none | — | No TBD/FIXME/XXX/TODO/placeholder in any phase file (unchanged since prior scan; no source drift) |

### Human Verification Required

Deferred-by-design from both plans (`<human-check>` blocks, deduplicated to 2 items — see frontmatter for test/expected/why). Disposition:

1. **Spike visual on live Hyprland** — DISPOSITION: stays open, non-blocking for Phase 2. Offscreen evidence proves the bridge seam every later tab reuses; pixels need eyes on the live session. Carry to verify-work on the Archel machine.
2. **pkexec desktop dialog** — DISPOSITION: stays open, non-blocking for Phase 2. Exit-code mapping logic is unit-covered (0/126/127/other); only the interactive polkit-agent UX needs a desktop session. Carry to verify-work on the Archel machine.

Neither item reflects an implementation deficit — automated checks are green. They gate final user acceptance, not downstream phase work (Phase 2 is read-only telemetry, no privilege or GUI dependency).

### Gaps Summary

No gaps. All 4 roadmap success criteria are observably true in the codebase with live behavioral evidence re-run in this refresh. Source tree is byte-identical to the prior verification on all covered paths (`git diff` empty); only `.planning/ROADMAP.md`, `.planning/STATE.md`, and `.planning/config.json` changed (phase-close bookkeeping, out of scope for this report — STATE.md/ROADMAP.md intentionally untouched). The two human items are plan-deferred checks requiring a desktop session, explicitly dispositioned above as non-blocking carries to verify-work.

---
_Verified: 2026-09-30T19:54:15Z_
_Verifier: the agent (gsd-verifier)_
