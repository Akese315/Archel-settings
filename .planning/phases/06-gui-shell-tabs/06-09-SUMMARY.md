---
phase: 06-gui-shell-tabs
plan: 09
subsystem: testing
tags: [validation, harness, coverage-audit, just, plat-03, thin-view, ci]

# Dependency graph
requires:
  - phase: 06-gui-shell-tabs (plans 06-01 through 06-08)
    provides: ShellModel + 10 tab view-models, QML shell, cmds twins
provides:
  - Single-command GUI verification (`just gui-check`: Qt build + qmllint sweep + offscreen smoke rc=124 + gui-feature suite)
  - Executable PLAT-03 thin-view guarantee (47-entry tab<->CLI map + no-subprocess-in-gui gate + annotation drift gate, green in both feature suites)
affects: [gsd-verify-work, CI wiring, phase-07+ (annotation convention for new CLI capabilities)]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
actuals:
  tokens: 12000
  tasks: 2
  commits: 4

# Tech tracking
tech-stack:
  added: [justfile harness (system `just` tool, no new cargo/crates)]
  patterns: [source-text audit tests (std-only, #[cfg(test)] bin module), // gui: vs // cli-only: capability annotations with two-way map agreement]

key-files:
  created: [justfile, crates/archel/src/thin_view_audit.rs]
  modified: [crates/archel/src/cmds/*_cmds.rs, crates/archel/src/cmds/landing_cmd.rs (53 annotations, comment-only), crates/archel/src/main.rs (test module wiring), crates/archel/src/gui/geometry.rs + models/{input,appearance,display}.rs (test-helper pid removal)]

key-decisions:
  - "Audit lives in src/thin_view_audit.rs as a #[cfg(test)] bin module, not tests/: binary-only crate integration targets inherit the build.rs Qt link set without bin bridge symbols and can never link under --features gui"
  - "13 CLI entry points annotated // cli-only: with reasons (session-scope proxy, VPN up/down toggles, net self-test, DNS reset, keybind edit composed via remove+add) — explicit v1 gaps, not rubber-stamps"
  - "Forbidden-literal gate allows archel_core::hyprland paths (core-backed, CLI-proven) and tokio::net::UnixListener (D-26 lifecycle); forbids std::process, Command::new, tokio::process, duct, zbus, direct hyprland-crate use with no whitelists"
  - "just binary absent on the box: verified via the exact shell mirrors from the justfile header (same fixed literals)"

patterns-established:
  - "Capability annotation: every pub run_*/begin_* fn in cmds/ carries // gui: <tab>.<invokable>[, ...] or // cli-only: <reason>; the audit fails on unannotated, doubly-annotated, or map-disagreeing entries"
  - "gui/ stays subprocess-free: test helpers use nanos stamps, never process-handle APIs"

requirements-completed: [PLAT-03]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "justfile with gui-check (Qt build + qmllint sweep + offscreen smoke rc=124 + gui-feature suite) and gui-check-fast alias"
    requirement: "PLAT-03"
    verification:
      - kind: other
        ref: "QMAKE=/usr/bin/qmake6 cargo build -p archel --features gui (24s, only pre-existing Qt-header C++ warnings)"
        status: pass
      - kind: other
        ref: "qmllint sweep: 20/20 QML files rc=0"
        status: pass
      - kind: e2e
        ref: "QT_QPA_PLATFORM=offscreen timeout 10 ./target/debug/archel settings -> rc=124, zero-error log"
        status: pass
      - kind: unit
        ref: "QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui -> 239 passed, 0 failed"
        status: pass
    human_judgment: false
  - id: D2
    description: "Thin-view coverage audit: 47-entry tab<->CLI map, mapped-invokable existence, no-subprocess/IPC-in-gui, annotation drift gate"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "cargo test -p archel --no-default-features thin_view_audit -> 3 passed"
        status: pass
      - kind: unit
        ref: "cargo test -p archel --features gui -> 239 passed including thin_view_audit::mapped_invokables_exist, no_subprocess_or_ipc_in_gui, annotation_drift_gate"
        status: pass
    human_judgment: false
  - id: D3
    description: "53 // gui: / // cli-only: annotations across all cmds twins (comment-only, no behaviour change)"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "annotation_drift_gate two-way annotation<->MAP agreement (fails on unannotated)"
        status: pass
    human_judgment: false

# Metrics
duration: ~25min
completed: 2026-10-10
status: complete
---

# Phase 06 Plan 09: gui-check Harness + Thin-View Audit Summary

**Single-command GUI verification plus the executable PLAT-03 guarantee: `just gui-check` (build + 20/20 qmllint + offscreen smoke rc=124 + 239/239 gui tests) and a 47-entry tab<->CLI coverage audit green in both feature suites.**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-10-10T10:37:00Z
- **Completed:** 2026-10-10T11:02:00Z
- **Tasks:** 2
- **Files modified:** 19 (2 created, 17 touched incl. SUMMARY)

## Accomplishments

- Root `justfile` landing `test`, `gui-build` (QMAKE pin baked in), `lint-qml`, `smoke` (rc=124 gate), `gui-check`, and `gui-check-fast` recipes — fixed literals only, shell mirrors in the header.
- `crates/archel/src/thin_view_audit.rs`: three gates (mapped invokables exist / no subprocess-or-IPC in gui / annotation drift with two-way map agreement) over a curated 47-entry map covering all 53 CLI entry points.
- 53 `// gui:` / `// cli-only:` annotations across the 10 cmds files, comment-only; 13 honest cli-only gaps with reasons (proxy session scope, VPN up/down, net self-test, DNS reset, keybind edit).
- Removed every `std::process` occurrence from gui/ (4 test helpers → nanos stamps) so the prohibition holds literally with no whitelists.

## Task Commits

Each task was committed atomically:

1. **Task 1: root justfile with gui-check recipe** - `e39b6a7` (feat)
2. **Task 2: thin-view coverage-audit test (PLAT-03 gate)** - `45dad26` (test)
3. **Rule 1 fix: audit moved into bin unit-test tree** - `3514fb8` (fix)

**Plan metadata:** `docs(06-09)` SUMMARY commit follows (this file).

## Files Created/Modified

- `justfile` - GUI verification harness (gui-check = build + lint + smoke + gui-feature tests)
- `crates/archel/src/thin_view_audit.rs` - PLAT-03 audit: MAP table + 3 test gates, std-only
- `crates/archel/src/cmds/{appearance,audio,bt,display,input,landing,net,pkg,power,sys}*.rs` - 53 capability annotations (comment-only)
- `crates/archel/src/main.rs` - `#[cfg(test)] mod thin_view_audit;` wiring
- `crates/archel/src/gui/geometry.rs`, `models/{input,appearance,display}.rs` - test helpers off `std::process::id()` onto nanos stamps
- `.planning/phases/06-gui-shell-tabs/deferred-items.md` - logged pre-existing archel-core i18n failure

## Decisions Made

- Audit placed in `src/` unit-test tree instead of the planned `tests/` path: a binary-only crate's integration-test target inherits build.rs Qt link directives without the bin target's bridge symbols, so `tests/gui_thin_view.rs` linked headless but could never link under `--features gui` — which is exactly what `gui-check` runs. Same three gates, both suites green.
- cli-only annotations used for 13 genuinely GUI-less v1 capabilities rather than forced mappings (showing state is not the capability); each carries a reason so a future tab plan can claim it by flipping the annotation + extending MAP.
- `tokio::net::UnixListener` (D-26) and `archel_core::hyprland::*` paths explicitly allowed in the audit with documented rationale; everything else subprocess/IPC-shaped is forbidden without exception.
- `just` binary absent on this box: verified each recipe via its header-documented shell mirror (identical literals), noted so CI (which has `just`) runs the real thing.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] tests/gui_thin_view.rs cannot link under --features gui**
- **Found during:** Task 1 verification (`cargo test -p archel --features gui` after Task 2 landed)
- **Issue:** Plan placed the audit at `crates/archel/tests/gui_thin_view.rs` as an integration test. `crates/archel` is binary-only: the integration target inherits build.rs Qt/cx-qt static link args but not the bin target's Rust bridge symbols → `undefined reference to cxx_qt_init_crate_* / ShellModel$*` at link. This broke the plan's own gui-check gate.
- **Fix:** Moved the identical audit to `crates/archel/src/thin_view_audit.rs` as `#[cfg(test)]` module wired from `main.rs` (std-only, compiles under both feature sets); deleted `tests/`. Doc header records why tests/ is structurally wrong for this crate.
- **Files modified:** `crates/archel/src/thin_view_audit.rs` (new), `crates/archel/src/main.rs`, deleted `crates/archel/tests/gui_thin_view.rs`
- **Verification:** `cargo test -p archel --no-default-features` → 123/123; `QMAKE=... cargo test -p archel --features gui` → 239/239 (audit included)
- **Committed in:** `3514fb8` (fix)

**2. [Rule 2 - Correctness] Scrubbed std::process from gui/ test helpers**
- **Found during:** Task 2 (forbidden-literal gate design)
- **Issue:** Four test helpers (`geometry.rs` scratch dir, `input`/`appearance`/`display` `test_base`) used `std::process::id()` for uniqueness; the plan prohibition forbids `std::process` in gui/** with no whitelists, and a literal scan cannot distinguish test-only use.
- **Fix:** Nanos-timestamp (+ existing sequence) uniqueness, same collision behaviour for parallel runs; reworded one comment that named the literal.
- **Files modified:** `crates/archel/src/gui/geometry.rs`, `models/input.rs`, `models/appearance.rs`, `models/display.rs`
- **Verification:** `grep -rn "std::process" crates/archel/src/gui/` clean; headless suite still 123/123
- **Committed in:** `45dad26` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 blocking bug, 1 correctness)
**Impact on plan:** Both required for the plan's own gates to hold (gui-check green, literal prohibition). Artifact path differs from plan (src/ vs tests/) — documented above; no scope creep.

## Issues Encountered

- Pre-existing `archel-core` failure `i18n::tests::no_hardcoded_french_diacritics_in_rust_sources` (hardcoded diacritics in `archel-core/src/bluetooth.rs`, introduced by 06-07): `crates/archel-core/` is git-clean under this plan, so the failure is independent of this diff. Out of scope (no archel-core edits permitted), logged to `deferred-items.md`. The same test scans all of `crates/` and passes on every file this plan touched (annotations are ASCII-only).
- `just` not installed on this box: used header shell mirrors for every recipe step; the justfile itself is the shipped artifact for CI/dev boxes with `just`.

## Verification Results

| Check | Command | Result |
|---|---|---|
| Qt build | `QMAKE=/usr/bin/qmake6 cargo build -p archel --features gui` | green 24s (only pre-existing Qt-header C++ warnings) |
| QML lint | qmllint over `src/gui/qml/**/*.qml` | 20/20 rc=0 |
| Smoke | `QT_QPA_PLATFORM=offscreen timeout 10 ./target/debug/archel settings` | rc=124, zero-error log |
| Headless suite | `cargo test -p archel --no-default-features` | 123 passed, 0 failed |
| GUI suite | `QMAKE=... cargo test -p archel --features gui` | 239 passed, 0 failed |
| Core suite | `cargo test -p archel-core --no-default-features` | 268 passed, 1 pre-existing failure (i18n diacritics, deferred) |

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `just gui-check` is the executable phase-goal definition for `/gsd-verify-work` and CI: build + lint + smoke + audit in one command.
- Any new CLI capability must carry a `// gui:` or `// cli-only:` annotation or the drift gate goes red — the convention is self-enforcing from here.
- Known v1 GUI gaps are enumerated as `// cli-only:` reasons (proxy, VPN up/down, net self-test, DNS reset, keybind edit) — ready-made scope for follow-up tab plans.
- Deferred: archel-core i18n diacritics failure (owning cleanup, not GUI).

---
*Phase: 06-gui-shell-tabs*
*Completed: 2026-10-10*

## Self-Check: PASSED

- `justfile` FOUND; `crates/archel/src/thin_view_audit.rs` FOUND
- Commits `e39b6a7`, `45dad26`, `3514fb8` all present in `git log`
- All verification commands above re-run post-commit with the results shown
