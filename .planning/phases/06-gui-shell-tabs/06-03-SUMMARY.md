---
phase: 06-gui-shell-tabs
plan: 03
subsystem: gui
tags: [polling, cxx-qt, qml, system-info, landing, sparkline, in-flight-guard, archel]
requires: [06-01-shell-window]
provides:
  - gui/poll.rs (PollGuard in-flight skip + sequence stale-drop, BannerPolicy D-05 rule)
  - gui/models/ architecture (bridge shim in src/gui/ + logic in models/<tab>.rs)
  - SystemModel (1 Hz CPU/RAM sparklines, per-core expander, representative temps)
  - LandingModel (cached update counts via landing::snapshot(false))
  - StatField / PollBanner / Sparkline components filled from placeholders
affects: [06-04-read-net-power-audio-bt, 06-05-read-hyprland-pkg, 06-06-modal-infra-pkg-write, 06-07-write-system-domains, 06-08-write-hyprland-domains]
tech-stack:
  added: []
  patterns: [bridge-shim-plus-models-logic-split, queue-closure-is-the-only-property-writer, unconditional-1hz-timer]
key-files:
  created:
    - crates/archel/src/gui/poll.rs
    - crates/archel/src/gui/models/mod.rs
    - crates/archel/src/gui/models/system.rs
    - crates/archel/src/gui/models/landing.rs
    - crates/archel/src/gui/bridge_system.rs
    - crates/archel/src/gui/bridge_landing.rs
  modified:
    - crates/archel/src/gui/mod.rs
    - crates/archel/build.rs
    - crates/archel/src/gui/qml/components/StatField.qml
    - crates/archel/src/gui/qml/components/PollBanner.qml
    - crates/archel/src/gui/qml/components/Sparkline.qml
    - crates/archel/src/gui/qml/tabs/SystemTab.qml
    - crates/archel/src/gui/qml/tabs/LandingTab.qml
  deleted:
    - crates/archel/src/gui/bridge.rs
key-decisions:
  - "Bridge shim + models logic split: QTBUG-93443 allows one bridge directory per QmlModule and cxx-qt-gen demands super::T aliases, so each tab gets a thin bridge_<tab>.rs in src/gui/ (shape + re-export) with the struct and all behavior in models/<tab>.rs; later tab plans copy the split"
  - "Queue closure calls generated setters, never struct fields: setting Rust fields skips NOTIFY, so apply_* returns owned strings and the closure feeds every property through set_* (the only property-mutating code)"
  - "Kernel-unreadable is the only whole-tick failure: partial sensor gaps stay None fields rendered N/A inside an otherwise live tick; matches landing::snapshot semantics"
  - "BannerPolicy gained wants_visible(): consecutive failures keep the banner on screen without re-signalling, dismiss hides until success, success re-arms"
requirements-completed: [PLAT-03]
actuals:
  tokens: 14250
  tasks: 3
  commits: 4
coverage:
  - id: D1
    description: "PollGuard skips a tick while one is in flight and drops stale sequences; BannerPolicy shows the banner once per episode (PLAT-03)"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "gui::poll::tests (5 passed: guard skip, stale drop, banner once, dismiss, wants_visible)"
        status: pass
  - id: D2
    description: "System tab shows live 1 Hz CPU/RAM sparklines over a 60 s rolling window, per-core expander hidden on 1-core, representative temps only, ellipsis pre-tick and N/A on failure (PLAT-03)"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "gui::models::system::tests (5 passed: series cap 60, N/A never zero, banner flow, aggregate, live collect)"
        status: pass
      - kind: manual_procedural
        ref: "QT_QPA_PLATFORM=offscreen timeout 10 ./target/debug/archel settings -> rc 124, zero qml errors"
        status: pass
      - kind: lint
        ref: "qmllint over all 21 QML files, exit 0"
        status: pass
  - id: D3
    description: "Landing renders cached update counts within one tick, never forces an alpm refresh; banner once on first failure, silent afterwards (PLAT-03)"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "gui::models::landing::tests (3 passed: N/A + banner once, success fills counts, live cached snapshot)"
        status: pass
      - kind: manual_procedural
        ref: "QT_QPA_PLATFORM=offscreen timeout 10 ./target/debug/archel settings -> rc 124, zero qml errors"
        status: pass
  - id: D4
    description: "Spike fully retired; headless builds compile no Qt and stay green (PLAT-03)"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "cargo test -p archel --no-default-features (120 passed); no SpikeModel refs in crates/archel/src"
        status: pass
      - kind: unit
        ref: "cargo test -p archel-core --no-default-features (269 passed)"
        status: pass
duration: 10min
completed: 2026-10-09
status: complete
---

# Phase 06 Plan 03 Summary

**Reusable polling layer plus the first two live tabs: System Info graphs at 1 Hz and Landing from the update-count cache.**

## Performance

- **Duration:** ~10min
- **Started:** 2026-10-09T18:23Z
- **Completed:** 2026-10-09T18:33Z
- **Tasks:** 3 completed
- **Files modified:** 14 (6 created Rust, 2 created shims counted in created, 5 QML filled, 2 build/wiring, 1 deleted)

## Accomplishments

- `gui/poll.rs`: `PollGuard` (compare-exchange in-flight flag, monotonic sequence tag, `is_newest` stale-drop) and `BannerPolicy` (first-failure show, dismiss, success re-arm, `wants_visible` stickiness) — Qt-free, 5 inline tests
- `gui/models/` architecture: `models/mod.rs` module list, per-tab logic files, thin `bridge_<tab>.rs` shims holding the `#[cxx_qt::bridge]` shape plus a `super::T` re-export; `SpikeModel` and `bridge.rs` fully deleted
- `SystemModel`: `Arc<Mutex<Monitor>>` + `Arc<Sensors>` shared handles, `spawn_blocking` workers, queue-only property writes; CPU/RAM 60-sample series, aggregate + max-clock, per-core JSON rows, representative temps, GPU line only with data, kernel; honest N/A, ellipsis pre-tick (QML-side `""` fallback)
- `LandingModel`: `landing::snapshot(false)` per tick (15-min TTL owned by core), `updates_cell` + `checker_notes` rendering, version/kernel fields
- Three components filled: `StatField` (N/A at 40% opacity, ellipsis), `PollBanner` (locked D-05 sentence, dismiss signal), `Sparkline` (Canvas 2px polyline, color param, empty-guard)
- Two live tabs with unconditional 1 Hz Timers + immediate first tick; SystemTab sections per D-09/D-10/D-11 with the `coreCount !== 1` expander rule; LandingTab hero block with 2xl top spacing

## Task Commits

Each task was committed atomically:

1. **Task 1: Polling layer + model architecture + component shells** - `f07b358` (feat)
2. **Task 2: SystemModel + SystemTab (1 Hz graphs)** - `d90acba` (feat)
3. **Task 3: LandingModel + LandingTab (cached update counts)** - `a99a8e8` (feat)

Plus `b6bc731` (style: em-dash cleanup in model comments, ASCII convention).

## Files Created/Modified

- `crates/archel/src/gui/poll.rs` - PollGuard + BannerPolicy + 5 inline tests
- `crates/archel/src/gui/models/mod.rs` - `pub mod landing; pub mod system;`
- `crates/archel/src/gui/models/system.rs` - SystemModelRust, TickOutcome, collect/apply, 5 tests
- `crates/archel/src/gui/models/landing.rs` - LandingModelRust, apply, 3 tests
- `crates/archel/src/gui/bridge_system.rs` / `bridge_landing.rs` - bridge declarations + Threading + re-exports
- `crates/archel/src/gui/mod.rs` / `crates/archel/build.rs` - module + bridge-file registration
- `crates/archel/src/gui/bridge.rs` - deleted (SpikeModel retired)
- 5 QML files filled (3 components, 2 tabs)

## Decisions Made

- Bridge shim + models logic split (QTBUG-93443 one-directory rule + `super::T` alias rule force it); later tab plans copy the split verbatim
- Queue closures feed owned strings through generated setters — field writes skip NOTIFY (caught during implementation, fixed before commit)
- Kernel-unreadable is the sole whole-tick failure; disk/GPU/temp gaps are per-field N/A
- `BannerPolicy::wants_visible` added so consecutive failures keep the banner without re-signalling
- Plan verify filters `system_model`/`landing` adjusted to real module paths (`models::system`, `models::landing`) — filters must match the `gui::models::<tab>::tests` path

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] bridge.rs deleted instead of emptied**
- **Found during:** Task 1 (cxx-qt-build rejects a listed file with no `#[cxx_qt::bridge]` module)
- **Issue:** The plan said "bridge.rs keeps only ShellModel", but ShellModel lives in shell.rs — an emptied bridge.rs breaks the build
- **Fix:** Deleted the file, removed `mod bridge` and its build.rs entry; zero SpikeModel references remain
- **Files modified:** `bridge.rs` (deleted), `mod.rs`, `build.rs`
- **Commit:** `f07b358`

**2. [Rule 3 - Blocking] Bridge shim split (QTBUG-93443 + super::T rule)**
- **Found during:** Task 1 (build failure: "Only one directory is supported per QmlModule"; then "Type alias path must have at exactly two segments")
- **Issue:** `models/*.rs` cannot hold `#[cxx_qt::bridge]` modules (different directory), and bridges cannot name `crate::...` struct paths
- **Fix:** Thin `bridge_<tab>.rs` shims in `src/gui/` (shape + `pub use` re-export for the `super::T` alias); all logic stays in `models/`. New files beyond `files_modified`: `bridge_system.rs`, `bridge_landing.rs`
- **Commit:** `f07b358`

**3. [Rule 1 - Bug] Queue closure must call generated setters**
- **Found during:** Task 2 (review before build)
- **Issue:** First draft mutated Rust struct fields in the queue closure, which never emits NOTIFY — QML would never re-render
- **Fix:** `apply_outcome` returns owned `AppliedTick` strings; the closure feeds each through its `set_*` invokable
- **Commit:** `d90acba`

**4. [Rule 2 - Correctness] Backing fields need pub(crate)**
- **Found during:** Task 1 (E0616: generated accessors in the shim module cannot see private fields)
- **Issue:** Split-file layout breaks the same-module field privacy the shell.rs precedent relied on
- **Fix:** `pub(crate)` on all qproperty backing fields
- **Commit:** `f07b358`

---

**Total deviations:** 4 auto-fixed (2 toolchain constraints, 1 correctness, 1 privacy). No scope creep; no new dependencies (pure cxx-qt/tokio/serde reuse — T-06-SC holds).

## Issues Encountered

- None beyond the deviations above. Full suite green: core 269, headless 120, gui 139 (includes 13 new: 5 poll + 5 system + 3 landing).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plans 06-04 through 06-08 copy the proven per-tab recipe: `models/<tab>.rs` (struct + collect/apply + tests) + `bridge_<tab>.rs` (shape + re-export + Threading) + one line in `models/mod.rs` + tab QML with unconditional Timer + StatField/PollBanner/Sparkline reuse
- Shared `Sensors` handle pattern (`Arc<Sensors>`, `&self` reads) and `Monitor` pattern (`Arc<Mutex<Monitor>>`) verified `Send + Sync` by the compiler — reuse directly for graphs/telemetry tabs
- Async core fns (network/power/bt) use the same tick shape with `runtime.spawn(async)` instead of `spawn_blocking`

## Self-Check: PASSED

- All 14 files exist on disk; commits `f07b358`, `d90acba`, `a99a8e8`, `b6bc731` verified in `git log`
- Full verification green at commit time (core 269 / headless 120 / gui 139, qmllint sweep, offscreen smoke rc 124 with zero QML errors)

---
*Phase: 06-gui-shell-tabs*
*Completed: 2026-10-09*
