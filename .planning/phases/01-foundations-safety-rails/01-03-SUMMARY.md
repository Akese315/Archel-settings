---
phase: 01-foundations-safety-rails
plan: 03
subsystem: ui
tags: [rust, cxx-qt, qml, qt6, qrc, gap-closure]

# Dependency graph
requires:
  - phase: 01-02
    provides: "CXX-Qt 0.10 spike window (vertical tabs + 1 Hz live label) with a mismatched qrc load URL"
provides:
  - "Spike window loading its QML from the qrc alias registered by cxx-qt-build (G-01-1 closed on the automated side)"
affects: [gui-phase, verify-work, uat-test-1]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
actuals:
  tokens: 30
  tasks: 1
  commits: 1

# Tech tracking
tech-stack:
  added: []
  patterns: [qrc-alias-rederived-from-generated-qrc]

key-files:
  created: []
  modified: [crates/archel/src/gui/mod.rs]

key-decisions:
  - "Trust the generated qrc, not the plan's memory: alias confirmed by grepping qml_module_resources_org_archel_spike.qrc before editing"

patterns-established:
  - "QML load URLs must be re-derived from the freshly generated qrc after every qml_file path change — the verify diff fails loudly instead of at window-open time"

requirements-completed: [PLAT-04]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "Spike window loads QML from the qrc alias registered by cxx-qt-build (no component-load errors)"
    requirement: "PLAT-04"
    verification:
      - kind: other
        ref: "grep generated qrc alias vs mod.rs load URL — both src/gui/qml/Spike.qml"
        status: pass
      - kind: other
        ref: "QT_QPA_PLATFORM=offscreen timeout 8 ./target/debug/archel settings — 0x 'failed to load component|No such file'"
        status: pass
      - kind: other
        ref: "cargo build -p archel --no-default-features — compiles Qt-free"
        status: pass
    human_judgment: true
    rationale: "Offscreen proves the QML loads and the loop holds, but UAT test 1 (vertical tab rail looks right + label ticks at 1 Hz) needs eyes on the live Hyprland session"

# Metrics
duration: 8min
completed: 2026-09-30
status: complete
---

# Phase 01 Plan 03: Spike QML Load URL Correction Summary

**One-line qrc URL fix (`qml/Spike.qml` → `src/gui/qml/Spike.qml`) confirmed against the generated qrc, proven by Qt rebuild + zero-error offscreen load + Qt-free headless build**

## Performance

- **Duration:** 8 min
- **Started:** 2026-09-30T20:00:00Z
- **Completed:** 2026-09-30T20:08:00Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- Closed gap G-01-1 on the automated side: `run_settings()` now loads `qrc:/qt/qml/org/archel/spike/src/gui/qml/Spike.qml`, exactly matching the `src/gui/qml/Spike.qml` alias registered in the generated `qml_module_resources_org_archel_spike.qrc`
- Qt build (`QMAKE=/usr/bin/qmake6 cargo build -p archel`) links clean against system Qt 6.11
- Offscreen `archel settings` run holds with zero `failed to load component` / `No such file` errors (was the exact UAT-reported failure before the fix)
- Headless `cargo build -p archel --no-default-features` still compiles with zero Qt involvement — no regression on the Qt-free invariant

## Task Commits

Each task was committed atomically:

1. **Task 1: Correct the spike QML load URL and re-verify Qt + headless builds** - `81308d6` (fix)

## Files Created/Modified

- `crates/archel/src/gui/mod.rs` - corrected the `QUrl` in `run_settings()` to the registered qrc alias (1 line)

## Decisions Made

- **Trust the generated qrc, not memory:** grepped `target/debug/build/archel-*/out/qt-build-utils/qml_modules/org/archel/spike/*.qrc` before editing and used exactly the alias found there (`src/gui/qml/Spike.qml`). The plan's diagnosis matched the on-disk artifact, so no deviation was needed.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- **`cargo` not on PATH in the execution shell:** bare `cargo` resolved to "commande introuvable" (Rust toolchain is user-local at `~/.cargo/bin`, per the 01-02 summary's environment note). Fixed by exporting `PATH="$HOME/.cargo/bin:$PATH"` for the verify commands — environment setup, not a code issue.
- **Commits land on `master`:** the repo's GSD config uses `branching_strategy: "none"` and every prior plan commit (`d0c85c6`, `265624c`, `eecad6c`, `ce07b08`, plus docs commits) is on `master`. Committed on `master` to match established project practice, per the orchestrator's explicit commit instruction.

## Threat Flags

None — one-line string correction to an already-reviewed local qrc URL, per the plan's threat model. No new surface.

## Known Stubs

None.

## User Setup Required

None - no external service configuration required. (Environment note: Qt builds require `QMAKE=/usr/bin/qmake6`; `~/.cargo/bin` must be on PATH. Human-check outstanding: re-run `QMAKE=/usr/bin/qmake6 cargo run -p archel --features gui -- settings` on the Hyprland session and confirm the vertical tab rail + 1 Hz ticking label per UAT test 1.)

## Next Phase Readiness

- G-01-1 is closed on the automated side; only the UAT test 1 human visual confirm remains (owned by verify-work, not a code blocker)
- No regression risk for later phases: headless stays Qt-free, and the qrc-alias re-derivation pattern guards any future `qml_file` path change

## Self-Check: PASSED

- `crates/archel/src/gui/mod.rs` FOUND with the corrected URL string
- Commit `81308d6` verified in git history (`git log`)
- No file deletions in the commit (`git diff --diff-filter=D` empty)
- No stray untracked files from this task (remaining `??` entries are orchestrator-owned `.planning/` files and `prompt.md`, left untouched)

---
*Phase: 01-foundations-safety-rails*
*Completed: 2026-09-30*
