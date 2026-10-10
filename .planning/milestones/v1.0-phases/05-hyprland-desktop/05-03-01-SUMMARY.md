---
phase: 05-hyprland-desktop
plan: 03-01
subsystem: desktop
tags: [hyprland, hyprctl, display, monitors, confirm-rollback, clap, insta]

# Dependency graph
requires:
  - phase: 05-hyprland-desktop plan 02-01
    provides: hardened shared seams (eval-first persist-on-agreement ordering, millis backups, typed timeouts, hermetic planners)
provides:
  - Monitor model with fixture-pure parse, plan_monitor_change with apply/rollback eval legs, and layout validators
  - confirm_action_timed (std-only 15 s window) plus ConfirmTimeout typed error with FR catalog key
  - archel display monitors plus display set CLI surface (FR tables, --json, insta snapshots, completions words)
affects: [05-04-01, 05-05-01, gui-phase]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
actuals:
  tokens: 19828
  tasks: 3
  commits: 3

# Tech tracking
tech-stack:
  added: []
  patterns: [vocabulary-validated-eval, timed-confirm-with-rollback, projected-layout-validation]

key-files:
  created:
    - crates/archel/src/cmds/display_cmds.rs
    - crates/archel/src/cmds/snapshots/archel__cmds__display_cmds__tests__display_monitors.snap
    - crates/archel/src/cmds/snapshots/archel__cmds__display_cmds__tests__display_set_keep.snap
    - crates/archel/src/cmds/snapshots/archel__cmds__display_cmds__tests__display_set_rollback.snap
  modified:
    - crates/archel-core/src/hyprland/monitors.rs
    - crates/archel-core/src/packages.rs
    - crates/archel-core/src/error.rs
    - crates/archel-core/locales/fr.json
    - crates/archel/src/cmds/mod.rs

key-decisions:
  - "Pre-confirm lives core-side in set_monitor (single prompt); the CLI renders the preview then delegates, avoiding a double prompt"
  - "Timeout and refusal both roll back yet exit distinctively (ConfirmTimeout vs ConfirmationAborted)"
  - "Layout projection reuses the plan's resolved mode/scale/position so overlap is caught pre-prompt against the live set"

patterns-established:
  - "Vocabulary-validated eval: every string reaching hl.monitor Lua validates against monitors -j, then renders through lua_quote"
  - "Projected-layout validation: clone the live set, swap in the planned geometry, validate overlap before any prompt"

requirements-completed: [DSP-01, DSP-02, DSP-03, DSP-04]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "User lists every connected monitor with current mode, scale, refresh, and position from hyprctl monitors -j (DSP-01..04 reads)"
    requirement: "DSP-01"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/hyprland/monitors.rs#fixture_parse_reads_both_monitors_with_derived_modes"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/display_cmds.rs#monitors_json_schema_pins_read_fields"
        status: pass
      - kind: e2e
        ref: "live: archel display monitors listed HDMI-A-2 1920x1080@144 scale 1 at 2560x0 plus DP-2 3840x2160@60 scale 1.5 at 0x0 (table and --json)"
        status: pass
    human_judgment: false
  - id: D2
    description: "User changes mode/scale/position per monitor with a 15 s keep-or-rollback re-confirm; reject/timeout restores the snapshot (DSP-01, D-04)"
    requirement: "DSP-01"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/hyprland/monitors.rs#orchestration_timeout_runs_rollback_and_stays_distinctive"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/hyprland/monitors.rs#orchestration_refusal_runs_rollback_with_aborted_exit"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/hyprland/monitors.rs#orchestration_keep_persists_the_snippet_line"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/hyprland/monitors.rs#orchestration_abort_writes_nothing_and_skips_both_legs"
        status: pass
      - kind: e2e
        ref: "live: hyprctl eval no-op re-apply of HDMI-A-2 current values returned ok rc=0 with monitors -j byte-identical before/after"
        status: pass
    human_judgment: false
  - id: D3
    description: "Invalid scale divisibility, unknown modes, typos, and overlapping layouts rejected typed-DesktopInvalid before any prompt, spawn, or write (DSP-02/03/04)"
    requirement: "DSP-02"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/hyprland/monitors.rs#scale_validator_rejects_bad_divisibility_nans_and_garbage"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/hyprland/monitors.rs#two_monitor_fixture_proves_layout_without_live_hardware"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/display_cmds.rs#set_unknown_output_fails_before_any_prompt_or_spawn"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/display_cmds.rs#set_overlapping_layout_fails_before_any_prompt_or_spawn"
        status: pass
      - kind: e2e
        ref: "live: set --output VGA-9, --scale 1.4, and --position 100x0 each failed typed-DesktopInvalid with rc=1 and no prompt"
        status: pass
    human_judgment: false
  - id: D4
    description: "Timed confirm primitive (15 s window, std-only) with NeedsTty gate and catalog-complete ConfirmTimeout error"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/packages.rs#timed_confirm_refuses_without_a_tty"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/error.rs#every_variant_renders_non_empty_french"
        status: pass
    human_judgment: false
  - id: D5
    description: "Kept display change is visibly active on the monitor after the 15 s keep (visual confirmation of the apply path)"
    verification: []
    human_judgment: true
    rationale: "Headless tests prove the eval legs, rollback, and snippet persist; only a human watching the screen can confirm a real mode change lands visibly. Live automated proof stayed a no-op re-apply by D-05 design."

# Metrics
duration: 7min
completed: 2026-10-08
status: complete
---

# Phase 05 Plan 03-01: Display Live-Apply Summary

**Per-monitor resolution/scale/refresh/layout via `hl.monitor` eval with 15 s confirm-and-rollback, vocabulary-validated planning, and CLI tables plus JSON — live-proven with a no-op re-apply on HDMI-A-2**

## Performance

- **Duration:** 7 min
- **Started:** 2026-10-08T11:59:53Z
- **Completed:** 2026-10-08T12:06:54Z
- **Tasks:** 3
- **Files modified:** 9

## Accomplishments

- `archel display monitors` lists every connected monitor with derived mode, scale, and scaled-space position from `hyprctl monitors -j`, as an FR table or `--json` (DSP-01..04 reads)
- `archel display set --output/--mode/--scale/--position/--disable` validates against the live vocabulary, projects the layout for overlap, previews old-arrow-new, pre-confirms, applies eval legs, then holds a 15 s timed re-confirm — reject or timeout re-applies the snapshot verbatim and exits distinctively (D-04)
- `confirm_action_timed` (std thread plus `mpsc::recv_timeout`, zero new deps) beside `confirm_action`, reusing `parse_confirm_answer` and the `NeedsTty` gate; new `ConfirmTimeout` typed error with FR catalog key enforced by the completeness test
- Full workspace headless suite green: 368 passed, 0 failed; three new `--json` surfaces pinned by insta snapshots; completions cover `display` and `monitors`

## Task Commits

Each task was committed atomically:

1. **Task 1: Pure monitor plan** - `7b0c388` (feat)
2. **Task 2: Timed confirm plus apply orchestration with rollback** - `b9bd136` (feat)
3. **Task 3: Display CLI** - `cf8b02b` (feat)

## Files Created/Modified

- `crates/archel-core/src/hyprland/monitors.rs` - Monitor model, fixture-pure parse, validators, plan/rollback statements, layout projection, timed-confirm orchestration, FR label fns
- `crates/archel-core/src/packages.rs` - `confirm_action_timed` plus headless gate test
- `crates/archel-core/src/error.rs` - `ConfirmTimeout` variant, message arm, every_variant registration
- `crates/archel-core/locales/fr.json` - `display.*` keys plus `errors.confirm_timeout`
- `crates/archel/src/cmds/display_cmds.rs` - monitors/set runners, FR tables, --json, fail-fast plus abort plus write-scope tests
- `crates/archel/src/cmds/mod.rs` - DisplayAction clap tree, dispatch, completions words
- `crates/archel/src/cmds/snapshots/*display_{monitors,set_keep,set_rollback}.snap` - pinned --json schemas

## Decisions Made

- Pre-confirm lives core-side in `set_monitor` (single prompt): the CLI renders the FR preview from its own read-only plan, then delegates confirm-plus-apply to core — a CLI-side confirm would have prompted twice for one change
- Timeout and refusal both run the rollback legs yet exit distinctively (`ConfirmTimeout` vs `ConfirmationAborted`) so triage tells a missed window from an explicit no
- Layout projection reuses the plan's resolved mode/scale/position (specials resolve to current geometry) so a planned overlap against the live two-monitor set fails pre-prompt — proven live against DP-2
- The CLI prints the FR rollback line naming the restored output on reject/timeout (non-JSON), since core returns `Err` and carries no report object for that path

## Deviations from Plan

None - plan executed exactly as written. (Process note: orchestration code drafted during Task 1 was temporarily cut back out to a scratch file so the Task 1 commit held only the pure layer with a green tree, then re-inserted verbatim for the Task 2 commit — no code difference, recorded here for auditability.)

## Issues Encountered

- First `archel` test run failed only on the two new insta snapshots (first-run pinning, expected): accepted via `INSTA_UPDATE=always`, content-reviewed (DP-2/HDMI-A-2 shapes, kept/rolled-back flags), stale `.snap.new` leftovers removed, suite re-run green
- The CLI abort-gate test could not mirror the appearance `apply-never-runs` shape because the pre-confirm lives inside core's `set_monitor`: the test instead proves the delegated apply runs its gate first and propagates typed `NeedsTty` before any mutation

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Shared seams ready for 05-04-01 (binds/input): vocabulary-validated planning, injectable confirm/apply/rollback orchestration, hermetic planners, write-scope spy shape — later waves copy these instead of inventing new ones
- `upsert_monitor_line` composes with the blur/opacity/font managed block as raw text, so monitor lines persist without touching `appearance.rs` or the hardened `hyprland/mod.rs` runner
- Zero new dependencies (RESEARCH zero-new-deps gate held); no `hyprctl keyword/reload/hyprpaper` spawns (golden test asserts their absence)
- Live compositor state left exactly as found (no-op eval only; monitors -j byte-identical; user modules untouched)
- Rollback UX (visible mode change plus 15 s keep) recorded for live UAT — coverage D5 carries `human_judgment: true`

## Self-Check: PASSED

- All 4 created files exist on disk; 3 new snapshot files pinned and content-reviewed
- Commits `7b0c388`, `b9bd136`, `cf8b02b` present in `git log`
- Workspace suite: 368 passed, 0 failed (`cargo test --workspace --no-default-features`)
- Prohibitions honored: no keyword/reload/hyprpaper spawns, no new deps, no writes outside app-owned paths (spy tests + live md5 evidence), no French diacritics in any `.rs` file

---
*Phase: 05-hyprland-desktop*
*Completed: 2026-10-08*
