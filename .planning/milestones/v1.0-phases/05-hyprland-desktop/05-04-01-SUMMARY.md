---
phase: 05-hyprland-desktop
plan: 04-01
subsystem: desktop
tags: [hyprland, hyprctl, keybinds, binds, modmask, mouse, input, eval, confirm-gate, clap, insta]

# Dependency graph
requires:
  - phase: 05-hyprland-desktop plan 03-01
    provides: hardened shared seams (eval-first persist-on-agreement ordering, typed errors, confirm gate, snippet writer, write-scope spy shape)
provides:
  - Bind model with modmask codec, binds -j parse, hl.bind/hl.unbind render plus snippet exact-match add/edit/remove
  - Mouse input leaves (sensitivity, natural scroll, scroll factor) with validators, eval live-apply, and read-back
  - archel input keybinds plus archel input mouse CLI surface (FR tables, --json, insta snapshots, completions words)
affects: [05-05-01, gui-phase]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
actuals:
  tokens: 27535
  tasks: 3
  commits: 3

# Tech tracking
tech-stack:
  added: []
  patterns: [snippet-as-truth-exact-match, dsp-path-round-trip, live-unbind-remove]

key-files:
  created:
    - crates/archel/src/cmds/input_cmds.rs
    - crates/archel/src/cmds/snapshots/archel__cmds__input_cmds__tests__input_keybinds.snap
    - crates/archel/src/cmds/snapshots/archel__cmds__input_cmds__tests__input_keybind_add.snap
    - crates/archel/src/cmds/snapshots/archel__cmds__input_cmds__tests__input_keybind_remove.snap
    - crates/archel/src/cmds/snapshots/archel__cmds__input_cmds__tests__input_mouse.snap
    - crates/archel/src/cmds/snapshots/archel__cmds__input_cmds__tests__input_mouse_set.snap
  modified:
    - crates/archel-core/src/hyprland/binds.rs
    - crates/archel-core/src/hyprland/input.rs
    - crates/archel-core/src/hyprland/mod.rs
    - crates/archel-core/locales/fr.json
    - crates/archel/src/cmds/mod.rs

key-decisions:
  - "Bind removals live-unbind via hl.unbind before the snippet line leaves, so no reload note is needed (remove report note is null)"
  - "CLI-facing dispatcher words render to live-verified hl.dsp paths (exec_cmd, window.close, window.fullscreen) with parse-back round-trip; unverified words render as-is and the compositor refusal stays honest"
  - "Edit keeps the current arg when the dispatcher is unchanged, drops it when the dispatcher changes unless --new-arg says otherwise"

patterns-established:
  - "Snippet-as-truth exact-match: add/edit/remove operate on the normalized (mods, key, dispatcher) tuple over app-generated lines only; binds -j stays a display vocabulary, never an edit address"
  - "Dsp-path round-trip: render maps CLI words to live hl.dsp paths, parse maps them back, so snippet exact-match holds across the boundary"

requirements-completed: [INP-01, INP-02, INP-03, INP-04]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "User lists user binds (binds -j with decoded mods) plus app-managed binds (snippet, marked managed) from archel input keybinds list (INP-01)"
    requirement: "INP-01"
    verification:
      - kind: unit
        ref: "crates/archel/src/cmds/input_cmds.rs#list_table_shows_user_binds_plus_managed_marks"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/input_cmds.rs#list_json_schema_pins_managed_flags"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/hyprland/binds.rs#binds_fixture_parses_key_modmask_and_submap"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/hyprland/binds.rs#modmask_codec_roundtrips_the_verified_bits"
        status: pass
    human_judgment: false
  - id: D2
    description: "User adds, edits, and removes keybinds by exact (mods, key, dispatcher) match with confirm gates; misses fail typed before any prompt (INP-02, D-06)"
    requirement: "INP-02"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/hyprland/binds.rs#add_is_idempotent_and_never_touches_siblings"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/hyprland/binds.rs#edit_and_remove_exact_match_with_typed_misses"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/input_cmds.rs#edit_miss_fails_before_any_prompt_or_spawn"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/input_cmds.rs#remove_miss_fails_before_any_prompt_or_spawn"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/input_cmds.rs#add_headless_refusal_writes_nothing"
        status: pass
      - kind: e2e
        ref: "live: hl.bind SUPER+SHIFT+F13 exec_cmd returned ok with modmask 65 present in binds -j, then hl.unbind returned ok with F13 absent (snippet untouched throughout)"
        status: pass
    human_judgment: false
  - id: D3
    description: "User tunes mouse sensitivity, natural scroll, and scroll factor with pre-spawn validation, eval live-apply, read-back, and persist-on-agreement (INP-03/04, D-07)"
    requirement: "INP-03"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/hyprland/input.rs#sensitivity_validator_rejects_nan_inf_and_out_of_range"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/hyprland/input.rs#eval_golden_pins_exact_statements"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/hyprland/input.rs#apply_evals_first_and_persists_only_on_agreement"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/hyprland/input.rs#abort_writes_nothing_and_skips_both_legs"
        status: pass
      - kind: e2e
        ref: "live: sensitivity 0.0 flipped to 0.75 (read-back 0.75) then restored to 0.0 (read-back 0.0)"
        status: pass
    human_judgment: false
  - id: D4
    description: "User bind blocks stay byte-for-byte unchanged; only app-owned snippet paths change (D-08 write-scope spy)"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/hyprland/binds.rs#user_block_sentinel_lines_never_match"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/input_cmds.rs#set_path_touches_nothing_outside_app_owned_files"
        status: pass
    human_judgment: false
  - id: D5
    description: "A newly added keybind visibly fires when the user presses the combo (only a human at the keyboard can confirm the compositor dispatches it)"
    requirement: "INP-02"
    verification: []
    human_judgment: true
    rationale: "Headless tests plus the live bind/unbind round-trip prove the eval legs and snippet persist; only pressing the combo can confirm the compositor actually dispatches the new bind."

# Metrics
duration: 80min
completed: 2026-10-08
status: complete
---

# Phase 05 Plan 04-01: Input Keybinds Plus Mouse Summary

**`archel input keybinds` (list/add/edit/remove on snippet exact-match with live hl.bind/hl.unbind) plus `archel input mouse` (validated sensitivity/scroll leaves with eval live-apply and read-back) — Phase 05 requirement coverage complete**

## Performance

- **Duration:** ~80 min wall-clock across the interrupted session plus resume (Tasks 1-2 prior executor, Task 3 completion plus SUMMARY this session)
- **Started:** 2026-10-08T12:10:00Z
- **Completed:** 2026-10-08T13:31:20Z
- **Tasks:** 3
- **Files modified:** 11

## Accomplishments

- `archel input keybinds list` shows user binds from `binds -j` (decoded SUPER/CTRL/ALT/SHIFT, key, submap, dispatcher opaque) plus app-managed binds from the snippet marked managed, as an FR table or `--json` (INP-01)
- `archel input keybinds add|edit|remove` address binds by exact (mods, key, dispatcher) match over app-snippet lines only: FR preview with collision hint, confirm gate, `hl.bind`/`hl.unbind` eval live-apply, snippet merge, FR ok-line plus `--json` (INP-02, D-06, T-05-11)
- `archel input mouse` reads all three leaves read-only; `archel input mouse set` validates pre-spawn (sensitivity -1..1, scroll factor bounds, on/off words), previews, delegates confirm-plus-apply to core (eval first, `getoption -j` read-back, persist only on agreement) (INP-03/04, D-07)
- Modmask codec over the live-verified bits (SHIFT=1, CTRL=4, ALT=8, SUPER=64) with round-trip tests; dispatcher words render to live-verified `hl.dsp` paths (`exec_cmd`, `window.close`, `window.fullscreen`) and parse back, so snippet exact-match holds across the boundary
- Full workspace headless suite green: 404 passed (120 archel bin + 266 archel-core lib + 18 doc/other), 0 failed; five new `--json` surfaces pinned by insta snapshots; completions cover `input`, `keybinds`, `mouse`, `add`, `edit`, `remove`

## Task Commits

Each task was committed atomically:

1. **Task 1: Binds pure layer** - `76277b8` (feat)
2. **Task 2: Mouse input leaves** - `a29cefc` (feat)
3. **Task 3: Input CLI** - `4830092` (feat)

## Files Created/Modified

- `crates/archel-core/src/hyprland/binds.rs` - binds -j parse, modmask codec, hl.bind/hl.unbind render/parse, snippet exact-match add/edit/remove, FR label fns
- `crates/archel-core/src/hyprland/input.rs` - mouse validators, eval live-apply with read-back, snippet input leaves, FR label fns
- `crates/archel-core/src/hyprland/mod.rs` - `pub mod input;` declaration (only mod.rs edit in core)
- `crates/archel-core/locales/fr.json` - `input.*` keys (titles, confirm prompts, ok lines, collision hint, mouse labels)
- `crates/archel/src/cmds/input_cmds.rs` - keybinds/mouse runners, FR tables, --json, fail-fast plus abort plus write-scope tests
- `crates/archel/src/cmds/mod.rs` - InputAction/KeybindAction/MouseAction clap tree, dispatch, requirement-ID doc comments, completions words
- `crates/archel/src/cmds/snapshots/*input_{keybinds,keybind_add,keybind_remove,mouse,mouse_set}.snap` - pinned --json schemas

## Decisions Made

- Bind removals live-unbind via `hl.unbind` before the snippet line leaves (verified `ok` live on the F13 probe bind): no reload note is needed, so the remove report carries `note: null` and the FR ok-line names the bind only
- CLI-facing dispatcher words render to live-verified `hl.dsp` paths (`exec` to `exec_cmd`, `killactive` to `window.close`, `fullscreen` to `window.fullscreen`, verified via `assert(hl.dsp.<path>)` probes); remaining plan words render as-is and a compositor refusal fails typed with the snippet untouched — the compositor stays the final arbiter
- Edit keeps the current arg when the dispatcher is unchanged (read out of the snippet), drops the stale arg when the dispatcher changes unless `--new-arg` says otherwise

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Stale killactive golden after live-verified dsp-path render**
- **Found during:** Task 3 completion (workspace suite red on `bind_line_renders_and_parses_back`)
- **Issue:** The WIP `dsp_path()` render (killactive to `window.close`, live-verified) left one golden expecting the old `hl.dsp.killactive()` shape; the exec golden had already been updated to `exec_cmd`
- **Fix:** Updated the golden to `hl.bind("F1", hl.dsp.window.close())`; round-trip still holds via `from_dsp_path`
- **Files modified:** crates/archel-core/src/hyprland/binds.rs
- **Verification:** `cargo test --workspace --no-default-features` green
- **Committed in:** 4830092 (Task 3 commit)

**2. [Rule 1 - Bug] Stale remove-note snapshot after live-unbind behavior change**
- **Found during:** Task 3 completion (workspace suite red on `remove_report_pins_action_bind_and_ok`)
- **Issue:** The pinned snap carried the old reload note while code plus test assert `note: null` (live unbind needs no reload)
- **Fix:** Reviewed the `.snap.new` content (matches the intended live-unbind behavior, transcript names the removed bind), accepted it over the stale snap
- **Files modified:** crates/archel/src/cmds/snapshots/archel__cmds__input_cmds__tests__input_keybind_remove.snap
- **Verification:** suite green; test asserts `report["note"].is_null()`
- **Committed in:** 4830092 (Task 3 commit)

**3. [Rule 1 - Bug] Dead KNOWN_DISPATCHERS const plus redundant closure (new clippy warnings)**
- **Found during:** Task 3 completion (clippy on plan files)
- **Issue:** The WIP refactor inlined the dispatcher vocabulary into `parse` match arms, orphaning `KNOWN_DISPATCHERS` (dead_code); `run_mouse_set` wrapped `confirm_action` in a redundant closure
- **Fix:** Removed the const and its now-redundant test assert (the `dispatcher_vocabulary_maps_cli_words_to_live_dsp_paths` test covers the mapping); passed `confirm_action` directly
- **Files modified:** crates/archel-core/src/hyprland/binds.rs, crates/archel/src/cmds/input_cmds.rs
- **Verification:** clippy clean on both files; suite green
- **Committed in:** 4830092 (Task 3 commit)

---

**Total deviations:** 3 auto-fixed (all Rule 1 completion-of-WIP bugs)
**Impact on plan:** All three were needed to land the WIP Task 3 work green; no scope creep, no architectural change.

(Process note: Tasks 1-2 were committed by the prior executor before interruption (`76277b8`, `a29cefc` — verified present, not redone); the resume adopted the uncommitted Task 3 WIP, fixed the three issues above forward per plan, and committed. No WIP contradicted the plan.)

## Issues Encountered

- Single stale-snapshot failure plus single stale-golden failure on first full-suite run after adopting the WIP — both traced to intended behavior changes (live unbind, dsp-path render), fixed as deviations 1-2 above rather than reverted
- Remaining clippy warnings (32 in `archel`, plus `audio.rs`/`network.rs`/`sensors.rs`/`monitors.rs`) are all in files outside this plan — left untouched per scope boundary

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 05 requirement coverage complete: APP-01..04 (01-01, 02-01), DSP-01..04 (03-01), INP-01..04 (04-01) — all drivable via `archel` CLI with tables plus `--json`
- `archel input` composes with the appearance/display managed blocks as raw snippet text; the GUI phase consumes `binds`/`input` exactly like the other `hyprland::` modules (pure planners plus confirm-gated apply)
- Zero new dependencies (zero-new-deps gate held for the whole phase); no `hyprctl keyword/reload/hyprpaper` spawns; no writes outside app-owned paths (spy tests green)
- Live compositor state left exactly as found (test bind added then unbound, F13 absent from binds -j; sensitivity flipped to 0.75 then restored to 0.0 with read-back both ways; snippet never touched by the spot-checks)
- Visible keybind dispatch (pressing a new combo fires the action) recorded for live UAT — coverage D5 carries `human_judgment: true`

## Self-Check: PASSED

- All 6 created files exist on disk; 5 new snapshot files pinned and content-reviewed
- Commits `76277b8`, `a29cefc`, `4830092` present in `git log`
- Workspace suite: 404 passed, 0 failed (`cargo test --workspace --no-default-features`)
- Prohibitions honored: no keyword/reload/hyprpaper spawns, no new deps, no writes outside app-owned paths (spy tests green), no French diacritics in any `.rs` file (em-dash/ellipsis precedent from Task 1 holds; diacritic grep clean)

---
*Phase: 05-hyprland-desktop*
*Completed: 2026-10-08*
