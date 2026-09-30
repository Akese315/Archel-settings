---
phase: 01-foundations-safety-rails
plan: 01
subsystem: infra
tags: [rust, cargo-workspace, clap, thiserror, rust-i18n, edition-2024]

# Dependency graph
requires: []
provides:
  - "Compiling headless Cargo workspace: archel bin + archel-core lib + archel-privileged stub"
  - "Typed ArchelError taxonomy with French catalog rendering (rust-i18n)"
  - "Bin -> core -> French-output wiring proven by archel version"
affects: [privilege-boundary, cxx-qt-spike, all-domain-phases]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
actuals:
  tokens: 4000
  tasks: 3
  commits: 2

# Tech tracking
tech-stack:
  added: [clap-4.6-derive, thiserror-2.0, rust-i18n-4.2]
  patterns: [single-binary-with-gui-feature, catalog-only-french, typed-errors-no-panic]

key-files:
  created: [rust-toolchain.toml, Cargo.toml, crates/archel-core/src/lib.rs, crates/archel-core/src/error.rs, crates/archel-core/src/i18n.rs, crates/archel-core/locales/fr.json, crates/archel/src/main.rs, crates/archel/src/cmds/mod.rs, crates/archel-privileged/src/main.rs, .gitignore]
  modified: [crates/archel-core/Cargo.toml, crates/archel/Cargo.toml]

key-decisions:
  - "D-01 single-binary layout approved (pre-approved checkpoint): archel bin + archel-core lib + archel-privileged helper"
  - "User-local rustup install instead of sudo pacman (sudo needs a password)"
  - "English clap help literals; all runtime strings via French catalog"
  - "thiserror English Display (log-oriented) + message() French renderer per Pattern 4"

patterns-established:
  - "Bin renders core errors via message() on stderr with non-zero exit; Display never reaches the terminal"
  - "gui feature (default) isolates all future Qt deps; headless builds use --no-default-features"
  - "Diacritics audit test guards D-07 repo-wide (scans crates/*/src for accented chars)"

requirements-completed: [PLAT-01]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "One cargo command builds all three crates headless without Qt"
    requirement: "PLAT-01"
    verification:
      - kind: other
        ref: "cargo build --workspace --no-default-features"
        status: pass
    human_judgment: false
  - id: D2
    description: "archel version prints a French line sourced from archel-core"
    requirement: "PLAT-01"
    verification:
      - kind: other
        ref: "cargo run -q -p archel --no-default-features -- version"
        status: pass
    human_judgment: false
  - id: D3
    description: "Every ArchelError variant renders non-empty French from the catalog; failure paths return Err"
    requirement: "PLAT-01"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/error.rs#every_variant_renders_non_empty_french + 4 companion tests"
        status: pass
    human_judgment: false
  - id: D4
    description: "No hardcoded French diacritics in any .rs file outside locales/"
    requirement: "PLAT-01"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/i18n.rs#no_hardcoded_french_diacritics_in_rust_sources"
        status: pass
    human_judgment: false

# Metrics
duration: 6min
completed: 2026-09-30
status: complete
---

# Phase 01 Plan 01: Foundations Tracer + Typed Errors Summary

**Compiling headless Rust workspace (archel bin + archel-core lib + privileged stub) with `archel version` proving the bin-to-core link and a typed ArchelError taxonomy rendered from a French rust-i18n catalog**

## Performance

- **Duration:** 6 min
- **Started:** 2026-09-30T19:28:22Z
- **Completed:** 2026-09-30T19:34:29Z
- **Tasks:** 3 (1 checkpoint pre-approved + 1 tracer + 1 auto)
- **Files modified:** 13

## Accomplishments

- Headless workspace builds with one cargo command; `archel version` prints `Archel version 0.1.0 — moteur archel-core 0.1.0` via `archel-core::version_line()`
- Typed `ArchelError` (AuthFailed{action,reason}, HelperProtocol, GuiNotCompiled, Internal) with D-12 cancelled-vs-denied French strings naming the rejected action
- All user-facing French lives in `crates/archel-core/locales/fr.json`; 6 unit tests green including the repo-wide diacritics audit

## Task Commits

Each task was committed atomically:

1. **Task 1: Confirm single-binary layout (checkpoint:decision)** — pre-approved by user on 2026-09-30 (D-01: `archel` bin + `archel-core` lib + `archel-privileged` helper); no commit, approval recorded here
2. **Task 2: End-to-end toolchain + workspace tracer** - `61f1d1d` (feat)
3. **Task 3: Typed ArchelError + FR catalog behind i18n keys** - `65527cd` (feat)

## Files Created/Modified

- `rust-toolchain.toml` - pins stable toolchain
- `Cargo.toml` / `Cargo.lock` - workspace (3 members, resolver 2, edition 2024); lock committed
- `crates/archel-core/src/lib.rs` - `library_version()`, `version_line()`, module re-exports, i18n backend
- `crates/archel-core/src/error.rs` - ArchelError taxonomy + `message()` catalog renderer + 5 tests
- `crates/archel-core/src/i18n.rs` - locale helpers + diacritics audit test
- `crates/archel-core/locales/fr.json` - sole source of French strings (version line + 5 error keys)
- `crates/archel/Cargo.toml` - clap derive dep; `gui` feature (default) with cxx-qt trio pinned =0.10.0, no Qt code yet
- `crates/archel/src/main.rs` + `src/cmds/mod.rs` - clap root; `version` calls core; `settings` returns typed GuiNotCompiled; French stderr + exit 1 on failure, no panics
- `crates/archel-privileged/` - stub binary refusing every invocation with exit 2 (plan 02 replaces it)
- `.gitignore` - excludes `/target/`

## Decisions Made

- **D-01 layout (pre-approved):** single `archel` binary with subcommands, `archel-core` lib per REQUIREMENTS.md PLAT-01 wording, separate `archel-privileged` helper. No re-prompt per orchestrator instruction.
- **English clap help literals:** clap-derive `about`/doc comments must be string literals and cannot route through `t!`; help stays English while every runtime string (version line, errors) renders from the French catalog. Satisfies must-have truth "no hardcoded French" without fighting the derive macro.
- **thiserror English Display + `message()` renderer:** per RESEARCH Pattern 4 fallback — `Display` is log-oriented English (raw detail allowed), `message()` is the sole terminal renderer (curated catalog French). The binary only ever prints `message()`.
- **rust-i18n backend at crate root, helpers in i18n.rs:** follows the documented convention so `t!` resolves crate-wide; keeps the D-07 guard beside the seam it protects.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Toolchain bootstrap without sudo**
- **Found during:** Task 2 (tracer, step zero)
- **Issue:** Plan prescribed `sudo pacman -S rustup`, but `sudo -n` fails — sudo demands a password the executor cannot provide; no cargo/rustc/rustup on the machine
- **Fix:** Installed rustup user-local via the official rustup-init script (network verified reachable), yielding stable cargo/rustc 1.98.1; system Qt6 (6.11.2) and g++ 16.2.1 already present so no other pacman step was needed
- **Files modified:** none (environment only)
- **Verification:** `cargo --version` → 1.98.1; full workspace build green
- **Committed in:** n/a (environment fix, reflected in `61f1d1d` build success)

**2. [Rule 1 - Bug] Wrong clap derive import**
- **Found during:** Task 2 (first workspace build)
- **Issue:** `use clap::{Parser, Subcommands}` — no such export; the derive is `Subcommand` (singular), breaking `FromArgMatches` bounds
- **Fix:** Corrected import and `#[derive(Subcommand)]`
- **Files modified:** crates/archel/src/cmds/mod.rs
- **Verification:** `cargo build --workspace --no-default-features` green
- **Committed in:** `61f1d1d` (part of task commit)

**3. [Rule 2 - Missing Critical] Bin migrated to catalog in Task 3 (files outside the task's list)**
- **Found during:** Task 3 (must-have truth check)
- **Issue:** Task 3's `<files>` listed core files only, but the tracer left French literals in `crates/archel/src/cmds/mod.rs` — violating must-have truth "no Rust source file outside locales/ contains hardcoded French"
- **Fix:** `version` now prints `archel_core::version_line()` (catalog key), `settings` returns `Err(ArchelError::GuiNotCompiled)`, `main` renders `err.message()`; no French literals or diacritics remain in any `.rs`
- **Files modified:** crates/archel/src/main.rs, crates/archel/src/cmds/mod.rs
- **Verification:** diacritics unit test + repo grep both clean
- **Committed in:** `65527cd` (part of task commit)

**4. [Rule 2 - Missing Critical] Added `.gitignore` for `/target/`**
- **Found during:** Task 3 (post-commit untracked-file check)
- **Issue:** `cargo build` leaves `target/` untracked in the repo root
- **Fix:** `.gitignore` with `/target/`
- **Files modified:** .gitignore
- **Verification:** `git status` clean of build output
- **Committed in:** `65527cd` (part of task commit)

---

**Total deviations:** 4 auto-fixed (1 blocking, 1 bug, 2 missing-critical)
**Impact on plan:** All required for correctness or plan-truth compliance. No scope creep — no domain logic, no GUI code, no privilege logic added.

## Issues Encountered

None blocking. `sudo` unavailability was routed around via user-local rustup (deviation 1). Default-feature (`gui`) build with cxx-qt was deliberately not exercised — no bridge code or build.rs exists yet; plan 02 owns the Qt spike with `QMAKE=/usr/bin/qmake6`.

## Threat Flags

None beyond the plan's threat model. Implemented as specified: failure paths return typed `ArchelError` (T-1-01 no-default-ok, tested); catalog strings contain only curated French plus the action name, raw detail stays in English `Display`/logs (T-1-02 accept-with-control); helper stub refuses everything with exit 2 so it cannot be mistaken for a working elevation path.

## User Setup Required

None - no external service configuration required. (Environment note: Rust toolchain now lives user-local at `~/.cargo/bin` + `~/.rustup`; ensure `~/.cargo/bin` is on PATH — e.g. `export PATH="$HOME/.cargo/bin:$PATH"`.)

## Next Phase Readiness

- Ready for plan 01-02 (privilege boundary + CXX-Qt spike): workspace layout, error taxonomy, and i18n seam are in place; `privilege_cancelled`/`privilege_denied` constructors already map the pkexec 126/127 semantics from RESEARCH
- Watch item: plan 02's helper replaces the exit-2 stub with the allowlisted dry-run + versioned JSON; core's `HelperProtocol` variant is ready to receive the parser
- Watch item: `archel` default features pull cxx-qt 0.10.0 — first default-feature build happens in the spike; headless CI stays on `--no-default-features`

## Self-Check: PASSED

All 13 files FOUND on disk; commits `61f1d1d` and `65527cd` verified in git history; no unintended deletions (`git diff --diff-filter=D` empty for the plan range).

---
*Phase: 01-foundations-safety-rails*
*Completed: 2026-09-30*
