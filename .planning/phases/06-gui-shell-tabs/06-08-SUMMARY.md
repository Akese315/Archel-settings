---
phase: 06-gui-shell-tabs
plan: 08
subsystem: gui-hyprland-writes
status: complete
tags: [write, hyprland, appearance, input, display, countdown]
requires:
  - 06-05 (Appearance/Input/Display read models, tab shell)
  - 06-06 (ModalSession, Confirm/Error global modals, write-slot pattern)
provides:
  - AppearanceModel write invokables (blur/opacity/wallpaper/font) through apply_*_with seams
  - InputModel write invokables (sensitivity/accelerate/keybind add+remove) through apply_input_with + binds line editor
  - DisplayModel plan/keep/revert_change through plan_monitor_change -> apply_monitor_plan + CountdownModal
affects:
  - crates/archel/src/gui/models/appearance.rs
  - crates/archel/src/gui/models/input.rs
  - crates/archel/src/gui/models/display.rs
  - crates/archel/src/gui/bridge_appearance.rs
  - crates/archel/src/gui/bridge_input.rs
  - crates/archel/src/gui/bridge_display.rs
  - crates/archel/src/gui/qml/tabs/AppearanceTab.qml
  - crates/archel/src/gui/qml/tabs/InputTab.qml
  - crates/archel/src/gui/qml/tabs/DisplayTab.qml
  - crates/archel/src/gui/qml/components/CountdownModal.qml
tech-stack:
  added: []
  patterns:
    - "fn-pointer injection seams (AppearanceOps / InputOps / DisplayOps) with PROD_* consts wiring core functions; unit tests pass fakes, workers run on the blocking pool"
    - "validate-first worker order (WR-02): plan fns run before any confirm; decline/errors touch neither IPC nor conf"
    - "per-model Arc<CountdownState> handoff (worker opens/resolves, Qt thread mirrors via countdownTick) instead of extending the shared ModalSession"
requirements: [PLAT-03]
key-files:
  created: []
  modified:
    - crates/archel/src/gui/models/appearance.rs
    - crates/archel/src/gui/models/input.rs
    - crates/archel/src/gui/models/display.rs
    - crates/archel/src/gui/bridge_appearance.rs
    - crates/archel/src/gui/bridge_input.rs
    - crates/archel/src/gui/bridge_display.rs
    - crates/archel/src/gui/qml/tabs/AppearanceTab.qml
    - crates/archel/src/gui/qml/tabs/InputTab.qml
    - crates/archel/src/gui/qml/tabs/DisplayTab.qml
    - crates/archel/src/gui/qml/components/CountdownModal.qml
key-decisions:
  - "set_accelerate drives the scroll-factor leaf (core has no accel leaf; sensitivity is accel-flat semantics)"
  - "Collision hint is prepended to the confirm question (shown before the user confirms); tab-inline validation stays for typos via synchronous pre-check"
  - "Countdown state lives in DisplayModel (Arc<CountdownState>), not ModalSession/ShellModel — avoids colliding with parallel 06-07 edits to modal.rs/shell.rs"
  - "Timeout/decline of a display change is silent + auto-refresh (the visible revert communicates it); only genuine failures open ErrorModal"
requirements-note: "PLAT-03 verified via thin-view contract: no std::process/Command, no hyprctl spawning, no conf-text editing in the GUI crate; forbidden-IPC grep clean; CountdownModal instantiated exactly once"
reconciled: false
estimate:
  tokens: 84000
  raw_tokens: 60000
  tasks: 3
  confidence: low
actuals:
  tokens: 29541
  tasks: 3
  commits: 4
  plan_head_before: 054402b
  plan_head_after: a77f6a4
metrics:
  duration: ~2h 30m
  completed: 2026-10-10
  tasks: 3
  files: 10
  tests-added: 27
  tests-total-gui: 236
---

# Phase 06 Plan 08: Hyprland Write Domains + Countdown Summary

Complete write coverage for the three Hyprland-integrated tabs through the CLI-proven core seams: four Appearance invokables, four Input invokables (pointer leaves + keybind add/remove), and the Display plan-apply-countdown flow with the real CountdownModal — every write confirm-gated, tab-locked while in flight, and failure-routed to the global ErrorModal.

## What Was Built

### Task 1 — Appearance writes (commits `8a8b8b8`, appearance part)
- `AppearanceOps` fn-pointer seam + production wrappers around `apply_blur_with` / `apply_opacity_with` / `apply_wallpaper_with` / `apply_font_with` with the real hyprctl/gsettings/hyprpaper legs (same legs the CLI twins pass), parameterized by `base` for hermetic tests.
- Workers `blur_worker` / `opacity_worker` / `wallpaper_worker` / `font_worker`: validate-plus-plan first (WR-02, typos fail before any confirm), core-verbatim confirm question (`confirm_blur_prompt`, `confirm_opacity_prompt`, `confirm_wallpaper_prompt`, `confirm_font_prompt`), injected apply, `ArchelError::message()` to ErrorModal on failure. Decline touches neither IPC nor conf.
- Bridge `setBlur(bool)` / `setOpacity(active, inactive)` / `setWallpaper(path)` / `setFont(family, size)` + `operationBusy`/`operationKind` single-operation slot (D-15); blank inputs are no-ops before any session is touched.
- AppearanceTab: blur Switch (writes on click), active/inactive opacity sliders with % labels + Appliquer, wallpaper path field + FileDialog (typed-path fallback, image preview when the path exists), font family/size fields, BusyIndicator + tab-wide disable while busy.
- 10 worker tests: decline asks the core question verbatim and applies nothing (blur, opacity), invalid opacity/wallpaper-path/font-spec rejected before any confirm, apply failure surfaces the core French message, plus conf-transformation asserts (opacity leaf render, wallpaper slice render, font family render with sibling leaves intact).

### Task 2 — Input writes (commits `4488b4b`, `97b48cf`)
- `InputOps` seam (apply_input / live-binds read / bind-line eval / unbind eval / snippet read+write at injected base); production `apply_input` passes a no-op core confirm (the modal already confirmed — no second TTY prompt can appear).
- `sensitivity_worker` / `accelerate_worker` (scroll-factor leaf) via `plan_input_set_at` + verbatim `confirm_input_prompt`.
- Keybind add: `split_combo` (`SUPER + T`, bare `F1`; dangling mods rejected) + `split_action` (`exec kitty --hold` keeps the opaque arg) + `plan_bind` validation (T-06-35); the add invokable pre-validates synchronously so typos render inline in `keybindError` and never reach a modal; the worker reads the live vocabulary, prepends `collision_hint` to the confirm question, then live-evals plus `add_bind_line`. Decline writes nothing.
- Keybind remove: exact-match `remove_bind_line` miss check BEFORE any confirm, always confirmed with `confirm_bind_remove_prompt`, then live unbind plus snippet removal.
- `binds_json` rows for managed binds now carry their removal address (`mods`/`key`/`dispatcher`/`arg`); user binds carry none and show no trash button (D-06 display-only).
- InputTab: sensitivity slider + Appliquer, acceleration (scroll-factor) field + Appliquer, add row (combo + action fields, inline red error label, Ajouter), per-row red Supprimer on managed rows only, elide-middle + ToolTip rows (long-text-keybind backstop).
- 9 worker tests: splitter rejections, sensitivity decline/invalid, accelerate leaf value, clean add (bare core prompt + merged body), collision warning ahead of confirm with decline writing nothing, remove miss/accept (unbind-then-line-removal order).

### Task 3 — Display plan-apply-countdown (commit `a77f6a4`)
- `DisplayOps` seam (inventory read + apply/rollback eval legs); `display_set_worker` runs read-only planning (lookup, `plan_monitor_change`, `project_layout` + `validate_layout`) BEFORE any prompt, then `apply_monitor_plan` with the modal pre-confirm and the countdown as the timed confirm. `compose_mode` maps the tab fields (blank keeps current).
- `CountdownState` (Arc, Mutex, mpsc): worker `open()`s it for the output, `recv_timeout` owns the real 15 s expiry (Timeout → `ConfirmTimeout`, disconnect → `ConfirmationAborted` — core rolls back in both cases); `keep_change`/`revert_change` resolve it; `countdown_tick` mirrors (visible, seconds, monitor) onto QML each second. No Hyprland IPC verb besides the eval legs is reachable — forbidden-verb grep clean, `set_monitor` never called.
- Bridge `planChange(monitor, resolution, refresh, scale)` / `keepChange` / `revertChange` / `countdownTick` + busy and countdown properties.
- CountdownModal real implementation (frozen api `{monitorName, secondsLeft, totalSeconds, keepRequested, revertRequested}`): "Conserver ces paramètres ? / Annulation automatique dans {s} s.", 15 s ProgressBar, Conserver (default) / Annuler, NoAutoClose + Esc=revert — instantiated ONLY in DisplayTab.qml (count == 1).
- DisplayTab: per-monitor cards with mode ComboBox (core `available_modes` vocabulary, split into resolution/refresh at apply time), scale field, Appliquer; countdown mirror rides the 1 s tick.
- 6 worker tests with fake clock: decline applies nothing and never opens the countdown, keep prevents revert + persists the snippet naming the output, zero-second expiry triggers rollback with no snippet write, unknown output/off-vocabulary mode fail before any confirm, countdown tick/resolve unit behavior.

## Verification

| Gate | Result |
|------|--------|
| `cargo test -p archel --features gui` | 236 passed, 0 failed (baseline 192 + 27 new this plan + 17 from parallel 06-07) |
| `cargo test -p archel --no-default-features` | 120 passed |
| `cargo test -p archel-core --no-default-features hyprland` | 79 passed |
| `cargo test -p archel-core --no-default-features binds` | 12 passed |
| `cargo clippy -p archel --features gui --all-targets` | zero warnings in files changed by this plan |
| `qmllint` on the 4 QML files | rc=0, no findings |
| `QT_QPA_PLATFORM=offscreen timeout 10 archel settings` | rc=124 (ran to timeout), 0 errors / `has no property` warnings |
| `! grep -nE '"(dispatch\|set_monitor)"' display.rs` | clean (Pitfall 5) |
| `grep -c "CountdownModal {" qml/` | 1 (DisplayTab only) |
| ASCII check on all touched `.rs` files | clean (D-07: French lives in QML/catalog) |

## Deviations from Plan

### Auto-fixed Issues

1. **[Rule 1 - Bug] `set_keybind_error` needs no trait import** — removed two unused `use cxx_qt::CxxQtType` lines the compiler flagged in `add_keybind`/`remove_keybind` (commit `97b48cf`, with a `from_ref` borrow fix for a clippy `redundant_clone`).
2. **[Rule 1 - Bug] `split_combo("SUPER + ")` treated SUPER as a modeless key** — a raw string containing `+` with a single surviving token is now a typed error (modifier without a key).
3. **[Rule 1 - Bug] Appearance blur Switch used `onToggled`** — programmatic tick updates could re-fire writes; switched to `onClicked` (user gesture only).
4. **[Rule 1 - Bug] DisplayTab passed unknown props to StatField** — `valueElide`/`valueToolTip` do not exist on StatField (would log `has no property`); removed (StatField already elides middle).
5. **[Rule 3 - Blocking] Parallel-agent contamination** — Task 1's commit swept 8 already-staged 06-07 files (audio/power) into `8a8b8b8`; all later commits in this plan use pathspec (`git commit -m … -- <own files>`) and contain only plan-08 files. The shared-tree compile was also intermittently broken by 06-07's in-progress `bt.rs`; display verification ran after their fix commit landed.
6. **[Rule 3 - Blocking] `archel-core` full-suite red is not mine** — `i18n::no_hardcoded_french_diacritics_in_rust_sources` fails on 06-07's `bluetooth.rs`; left untouched per scope boundary.

### Documented plan-wording deviations (behavior-equivalent or core-limited)

- **Blur passes/size sliders omitted**: core `AppearanceState` exposes only the blur switch + opacity leaves — no passes/size seam exists, and the GUI invents no domain logic (phase rule). Switch only.
- **`set_accelerate` = scroll-factor leaf**: core models no accel leaf; the UI labels it "Accélération" over the scroll-factor store. The natural-scroll switch stays display-only (artifacts list exactly two pointer setters).
- **Collision hint placement**: the hint is prepended to the confirm question (shown before the user confirms) rather than tab-inline — the worker cannot touch QObject properties; tab-inline display is reserved for typos via the synchronous pre-check.
- **Countdown state placement**: per-model `Arc<CountdownState>` instead of extending `ModalSession`/`ShellModel` — 06-07 was concurrently editing both files in the same tree; the local state keeps the plans file-disjoint.
- **Timeout/decline silence**: an expired or declined display change refreshes the tab without opening ErrorModal (rollback is the expected outcome, and the visible revert + closing countdown communicate it).

## Authentication Gates

None — no privilege paths in this plan (all Hyprland IPC runs unprivileged; no pkexec surface).

## Known Stubs

None. Every invokable is wired end-to-end to a core seam with fake-injected tests; the wallpaper preview degrades to empty (not placeholder) for missing paths; the natural-scroll switch is intentionally read-only (documented above, no future plan needed — core exposes no setter contract for it beyond `apply_input_with`, which this plan already drives).

## Threat Flags

None new. T-06-34: the 15 s auto-revert lives in core `apply_monitor_plan`; the GUI cannot disable it and always offers Annuler (plus Esc). T-06-35: crafted keybind/action strings pass `plan_bind` validation and core `lua_quote` rendering before any eval or conf write; conf writes go only through `add_bind_line`/`remove_bind_line`. T-06-36: accepted as planned (confirm quotes the exact change; notify watcher reloads external edits). No new packages (T-06-SC N/A).

## Self-Check: PASSED

- Files: all 10 modified files exist on disk.
- Commits: `8a8b8b8` (appearance + 8 swept 06-07 files, documented), `4488b4b` (input), `97b48cf` (input clippy), `a77f6a4` (display) — all present in `git log`.
- Wave-6 parallel work (06-07 commits `8d41e7d`, `53dd31e`, `b3ebd03`) untouched by plan-08 commits after `8a8b8b8`.
