---
phase: 06-gui-shell-tabs
plan: 05
subsystem: gui
tags: [thin-view, cxx-qt, qml, appearance, display, input, packages, keybinds, archel]

# Dependency graph
requires: [06-03-polling-system-landing, 06-04-read-net-power-audio-bt]
provides:
  - gui/models/appearance.rs (AppearanceModel: blur/snippet-opacity/wallpaper/font reads)
  - gui/models/display.rs (DisplayModel: read_monitors with core-derived mode + option lists)
  - gui/models/input.rs (InputModel: read_input + merged binds via keybind_rows)
  - gui/models/pkg.rs (PkgModel read half: installed list, per-query search, card select)
  - bridge_appearance/display/input/pkg.rs shims + AppearanceTab/DisplayTab/InputTab/PkgTab populated QML
affects: [06-06-modal-infra-pkg-write, 06-08-write-hyprland-domains]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
actuals:
  tokens: 19500
  tasks: 2
  commits: 2

# Tech tracking
tech-stack:
  added: []
  patterns: [per-query-sequence-tag-for-search, blank-query-clears-without-spawn, na-card-on-unknown-package]

key-files:
  created:
    - crates/archel/src/gui/models/appearance.rs
    - crates/archel/src/gui/models/display.rs
    - crates/archel/src/gui/models/input.rs
    - crates/archel/src/gui/models/pkg.rs
    - crates/archel/src/gui/bridge_appearance.rs
    - crates/archel/src/gui/bridge_display.rs
    - crates/archel/src/gui/bridge_input.rs
    - crates/archel/src/gui/bridge_pkg.rs
  modified:
    - crates/archel/src/gui/models/mod.rs
    - crates/archel/src/gui/mod.rs
    - crates/archel/build.rs
    - crates/archel/src/gui/qml/tabs/AppearanceTab.qml
    - crates/archel/src/gui/qml/tabs/DisplayTab.qml
    - crates/archel/src/gui/qml/tabs/InputTab.qml
    - crates/archel/src/gui/qml/tabs/PkgTab.qml

key-decisions:
  - "Blur+snippet failure fails the whole Appearance tick; font/wallpaper gaps degrade per-field without tripping the banner"
  - "Display rows serialize name/summary/mode/scale/position/disabled/available_modes via current_mode and summarize_monitor, never re-derived"
  - "Binds failures degrade to the honest empty list; only read_input failure reads N/A everywhere"
  - "Blank search clears to the installed list without spawning; card errors read the N/A card, never a zero-filled one"

requirements-completed: [PLAT-03]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "Appearance tab reads blur/opacity/wallpaper/font through the CLI-proven core reads; Display lists Hyprland monitors via read_monitors; Input lists mouse leaves via read_input and binds via parse_binds_json/parse_snippet_binds rendered with keybind_rows (PLAT-03)"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "gui::models::appearance::tests (3 passed: N/A + banner once, core field fill, per-field degrade)"
        status: pass
      - kind: unit
        ref: "gui::models::display::tests (2 passed: empty + banner once, mode/option row shape)"
        status: pass
      - kind: unit
        ref: "gui::models::input::tests (3 passed: N/A + banner once, verbatim core rows, mouse fill)"
        status: pass
      - kind: lint
        ref: "qmllint AppearanceTab.qml DisplayTab.qml InputTab.qml, exit 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "Packages tab lists installed packages, live search results, and the package card through list_installed/search/package_card; rapid typing never flashes stale hits; zero-hit refinement copy and keybind empty-state copy with inert Ajouter affordance (PLAT-03)"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "gui::models::pkg::tests (5 passed: stale drop, zero-hit flag, N/A card, list order, label set)"
        status: pass
      - kind: lint
        ref: "qmllint PkgTab.qml, exit 0"
        status: pass
      - kind: manual_procedural
        ref: "QT_QPA_PLATFORM=offscreen timeout 10 ./target/debug/archel settings -> rc 124, zero qml errors"
        status: pass
    human_judgment: false
  - id: D3
    description: "All ten tabs navigate with live reads; all four new tabs poll unconditionally at 1 Hz with ellipsis-until-first-tick and N/A on failure (PLAT-03)"
    requirement: "PLAT-03"
    verification:
      - kind: manual_procedural
        ref: "QML source review: unconditional Timers + Component.onCompleted tick in all four tabs; StatField ellipsis fallback"
        status: pass
      - kind: unit
        ref: "cargo test -p archel-core (269) + headless archel (120) + gui (164), qmllint sweep over all QML"
        status: pass
    human_judgment: true

# Metrics
duration: 50min
completed: 2026-10-09
status: complete
commits: 2
plan_head_before: df0c5e598e077f0cddab90c6075fd9a879736b17
plan_head_after: 2a4ee3b9415c9887e2fa60fb804add4a2fd000c8
---

# Phase 06 Plan 05 Summary

**The last four read tabs over CLI-proven core entry points: Appearance, Display, Input, and Packages — all ten tabs now navigate with live content.**

## Performance

- **Duration:** ~50min
- **Tasks:** 2 completed
- **Files modified:** 15 (8 created Rust, 7 modified Rust/QML/build)
- **Tests added:** 13 (3 appearance + 2 display + 3 input + 5 pkg)

## Accomplishments

- `AppearanceModel`: `tick()` runs `spawn_blocking` over `read_blur()` + `current_snippet_state(hypr_dir())` + `read_wallpaper_at(hypr_dir())` + `read_font()`; blur on/off text, `active / inactive` opacity pair, wallpaper path, `family size` font line. AppearanceTab: disabled Switch, 0..1 Slider parsed from the active leaf, path/font StatFields.
- `DisplayModel`: `tick()` runs `spawn_blocking(read_monitors())` — the same read `run_monitors` calls; rows carry name/summary/mode/scale/position/disabled plus the core `available_modes` vocabulary. DisplayTab: per-monitor GroupBox with read-only resolution/refresh ComboBoxes populated from the option lists, disabled until plan 08.
- `InputModel`: `tick()` runs `spawn_blocking` over `read_input()` plus `binds -j` merged with `parse_snippet_binds` and rendered through `keybind_rows` verbatim (managed marker / opaque dispatcher). InputTab: disabled sensitivity Slider (-1..1) + natural-scroll Switch, binds ListView with `combo — detail` rows, ElideMiddle + ToolTip, exact `Aucun raccourci personnalisé` empty copy with inert `Ajouter` button (plan 08 wires it).
- `PkgModel` (read half; plan 06 adds write invokables to this file): `tick()` lists installed with the PollGuard sequence tag; `search(query)` spawns `packages::search` with a per-query tag so stale hits drop (T-06-26); `select(name)` spawns `package_card`. PkgTab: search TextField (accepted text), BusyIndicator while searching, installed/search ListView with ElideMiddle + ToolTip, `Aucun paquet trouvé` refinement copy gated on the `zero_hit` flag, detail Pane rendering the six core card fields. No Install/Remove/Mettre à jour buttons (plan 06).
- Wiring: four `bridge_<tab>.rs` shims (proven split), `models/mod.rs` + `gui/mod.rs` + `build.rs` appends only. No new dependencies (T-06-SC holds); no hyprland.conf parsing or hyprctl dispatch in GUI code (prohibition holds — all reads go through `archel-core::hyprland` and `packages`).

## Task Commits

Each task was committed atomically:

1. **Task 1: AppearanceModel + DisplayModel + InputModel (+ tabs)** - `a7a53d0` (feat)
2. **Task 2: PkgModel reads + Packages tab (list/search/card)** - `2a4ee3b` (feat)

## Files Created/Modified

- `crates/archel/src/gui/models/{appearance,display,input,pkg}.rs` - view-model logic + inline tests
- `crates/archel/src/gui/bridge_{appearance,display,input,pkg}.rs` - bridge shims + Threading + re-exports
- `crates/archel/src/gui/models/mod.rs`, `gui/mod.rs`, `build.rs` - registration appends
- `crates/archel/src/gui/qml/tabs/{Appearance,Display,Input,Pkg}Tab.qml` - populated tab pages

## Decisions Made

- Blur-or-snippet failure fails the whole Appearance tick (both are the tab's core truth); font (gsettings) and wallpaper (slice file) failures degrade per-field — they must not blank live compositor state
- Keybind rows use `keybind_rows` `(combo, marker)` pairs mapped to `{combo, detail}` JSON — the exact strings `run_keybinds_list` prints, no GUI reformatting
- Binds-read failure degrades to the honest empty list (empty-state copy covers it); only `read_input` failure trips N/A + banner
- Blank search clears `searchJson`/`zeroHit`/`searchBusy` synchronously without spawning — clearing the field always returns to the installed list
- Card errors render the N/A card (name survives, every value N/A) rather than emptying `cardJson` — the detail pane stays informative
- Empty-state copy uses the exact UI-SPEC French rows (`Aucun paquet trouvé` / `Vérifiez l'orthographe…`, `Aucun raccourci personnalisé` / `Ajoutez votre premier…`); the plan text dropped diacritics but QML carries them

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Stale-search test asserted the wrong tag order**
- **Found during:** Task 2 verification (1 test failed)
- **Issue:** The test set `search_seq = 1` then applied tag 1 (which correctly applied as newest), so the "stale" assertion failed — a stale arrival must come after the counter already advanced
- **Fix:** Set `search_seq = 2` before applying tag 1 (dropped), then tag 2 (applied) — matches the real rapid-typing interleaving
- **Files modified:** `crates/archel/src/gui/models/pkg.rs` (test only)
- **Commit:** `2a4ee3b`

**2. [Rule 3 - Blocking] `//!` continuation comment rejected by rustc**
- **Found during:** Task 1 build (`E0753: expected outer doc comment`)
- **Issue:** A `//!` line placed after `///` lines inside the `monitors_json_for` doc block — inner doc comments may only precede items
- **Fix:** Changed the continuation line to `///`
- **Files modified:** `crates/archel/src/gui/models/display.rs`
- **Commit:** `a7a53d0`

**3. [Rule 3 - Blocking] Plan verify filters match zero tests**
- **Found during:** Task 1 verification (`cargo test` takes one TESTNAME filter; three filters rejected)
- **Issue:** Same adjustment 06-03/06-04 documented — tests live at `gui::models::<tab>::tests`, so the plan's `appearance_model::`/`display_model::`/`input_model::` filters match nothing
- **Fix:** Ran the `models::` filter (28 passed incl. 8 new), then `models::pkg` for task 2 (5 passed)
- **Commit:** n/a (verification only)

---

**Total deviations:** 3 auto-fixed (1 test bug, 1 compile blocker, 1 verify-filter). No scope creep; no new dependencies; every read traces 1:1 to a CLI entry point (`appearance blur/opacity/wallpaper/font` reads, `run_monitors`, `run_keybinds_list` + `read_input`, `run_list`/`run_search`/`run_info`).

## Issues Encountered

- `cargo test -p archel-core --no-default-features` showed 2 failures on the first post-task run (267 passed / 2 failed), then went fully green on two consecutive reruns (269 passed / 0 failed, 1 ignored). No core file was touched by this plan — the failures are flaky pre-existing timing-sensitive tests, not a regression. Documented here for the phase verifier; rerun to confirm if the gate trips on them.
- The `>=60-char name` ElideMiddle visual check (UI-SPEC backstop row) is code-reviewed (ElideMiddle + ToolTip in every new list delegate) but not visually rendered — headless environment has no display; flagged for UAT (same note as 06-04).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All ten tabs now show live reads; the read half of the phase is complete (success criteria 1 and 4 for reads)
- Plan 06 consumes: `PkgModel` in `models/pkg.rs` (add install/remove/upgrade invokables + streaming modal glue to the same file) and the `searchBusy`/`zeroHit` slots
- Plan 08 consumes: the disabled Appearance controls (Switch/Slider), the Display ComboBoxes with their `available_modes` option lists, and the inert `Ajouter` affordance (InputTab `onClicked: {}` placeholder)

## Self-Check: PASSED

- All 15 files exist on disk; commits `a7a53d0`, `2a4ee3b` verified in `git log`
- Full verification green at commit time (core 269 / headless 120 / gui 164, qmllint sweep clean, offscreen smoke rc 124 with zero QML errors)
- No `.planning/` or `prompt.md` files staged in task commits; deletions check clean on both commits

---
*Phase: 06-gui-shell-tabs*
*Completed: 2026-10-09*
