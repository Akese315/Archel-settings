---
phase: 05-hyprland-desktop
plan: 01-01
subsystem: desktop
tags: [hyprland, hyprctl, lua, appearance, blur, opacity, clap, insta]

# Dependency graph
requires:
  - phase: 04-connectivity-devices
    provides: subprocess discipline (which-gate, LC_ALL=C, wait_timeout), confirm-then-execute gate, FR catalog + insta patterns
provides:
  - hyprland core module (run_hyprctl eval apply, getoption read-back, lua_quote, snippet writer with backup+atomic+require)
  - HyprlandUnavailable / DesktopInvalid typed errors with FR catalog keys
  - archel appearance blur + opacity CLI surface (FR tables, --json, insta snapshots)
  - monitors/binds/wallpaper submodule stubs for plans 05-02-01/03-01/04-01
affects: [05-02-01, 05-03-01, 05-04-01, 05-05-01, gui-phase]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
actuals:
  tokens: 16949
  tasks: 2
  commits: 2

# Tech tracking
tech-stack:
  added: []
  patterns: [hyprctl-eval-live-apply, snippet-as-durability, honest-getoption-read-back, injectable-runner-reader]

key-files:
  created:
    - crates/archel-core/src/hyprland/mod.rs
    - crates/archel-core/src/hyprland/appearance.rs
    - crates/archel-core/src/hyprland/monitors.rs
    - crates/archel-core/src/hyprland/binds.rs
    - crates/archel-core/src/hyprland/wallpaper.rs
    - crates/archel/src/cmds/appearance_cmds.rs
  modified:
    - crates/archel-core/src/lib.rs
    - crates/archel-core/src/error.rs
    - crates/archel-core/locales/fr.json
    - crates/archel/src/cmds/mod.rs

key-decisions:
  - "Live-apply via hyprctl eval hl.config (keyword falsified dead on Hyprland 0.56.2)"
  - "Snippet at ~/.config/hypr/modules/archel.lua with backup-before-every-write, tmp+rename, rotation N=5, idempotent require line"
  - "Opacity floats validated typed-DesktopInvalid pre-spawn; canonical float rendering for eval/snippet text"

patterns-established:
  - "hyprctl runner: HIS-gate + which-gate + LC_ALL=C + wait_timeout/kill + eval error-prefix refusal"
  - "Mutations do snippet-write then eval-apply then getoption read-back; read-back disagreement fails typed, never silent ok"
  - "D-08 write-scope spy test: temp hypr dir + outside sentinel assert only app-owned paths change"

requirements-completed: [APP-03, APP-04]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "User toggles window blur from the CLI with live-apply, read-back, and snippet durability (APP-03)"
    requirement: "APP-03"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/hyprland/appearance.rs#apply_orchestration_writes_snippet_then_evals_then_reads_back"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/appearance_cmds.rs#blur_confirm_abort_spawns_no_write"
        status: pass
      - kind: e2e
        ref: "live: archel appearance blur off/on flipped getoption decoration:blur:enabled with read-back (restored to true)"
        status: pass
    human_judgment: false
  - id: D2
    description: "User sets active/inactive opacity from the CLI with live-apply; invalid floats rejected pre-spawn (APP-04)"
    requirement: "APP-04"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/hyprland/appearance.rs#opacity_validators_reject_garbage_and_accept_boundaries"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/hyprland/appearance.rs#opacity_eval_golden_pins_exact_statements"
        status: pass
      - kind: e2e
        ref: "live: archel appearance opacity set --active 0.9 --inactive 0.8 applied with read-back (restored to 1.0/1.0)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Out-of-session invocation degrades to typed HyprlandUnavailable, never panic or hang"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/hyprland/mod.rs#unset_signature_maps_to_typed_unavailable_without_spawning"
        status: pass
      - kind: e2e
        ref: "live: env -u HYPRLAND_INSTANCE_SIGNATURE archel appearance blur on printed the FR hint with exit 1"
        status: pass
    human_judgment: false
  - id: D4
    description: "D-08 invariant: user config byte-for-byte unchanged except the guarded require line; every write backed up"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/hyprland/mod.rs#writer_touches_nothing_outside_the_hypr_dir"
        status: pass
      - kind: e2e
        ref: "live: require line present exactly once; sibling modules untouched; timestamped backups retained"
        status: pass
    human_judgment: false

# Metrics
duration: 7min
completed: 2026-10-08
status: complete
---

# Phase 05 Plan 01-01: Hyprland Blur + Opacity Summary

**Live blur/opacity control via `hyprctl eval` with app-snippet durability — tracer slice proven live, opacity expanded on the proven shape**

## Performance

- **Duration:** 7 min
- **Started:** 2026-10-08T11:19:04Z
- **Completed:** 2026-10-08T11:26:07Z
- **Tasks:** 2
- **Files modified:** 12

## Accomplishments

- `archel appearance blur on|off` flips `decoration:blur:enabled` live via `hyprctl eval 'hl.config(...)'`, confirms via `getoption -j` read-back, and persists to `~/.config/hypr/modules/archel.lua` (APP-03)
- `archel appearance opacity set --active/--inactive` live-applies both leaves with read-back; bare `opacity` reads both; empty/NaN/inf/out-of-range floats rejected typed-DesktopInvalid before any spawn or write (APP-04)
- Snippet writer is production-quality for all later plans: timestamped backup before every write (rotation N=5), tmp+rename atomicity, idempotent `require("modules.archel")` insertion, D-08 write-scope spy test
- Full workspace headless suite green: 293 passed, 0 failed; both new `--json` surfaces pinned by insta snapshots

## Task Commits

Each task was committed atomically:

1. **Task 1: End-to-end blur toggle** - `552d708` (feat)
2. **Task 2: Opacity expansion** - `88e99b5` (feat)

## Files Created/Modified

- `crates/archel-core/src/hyprland/mod.rs` - run_hyprctl runner, eval refusal, getoption parse, lua_quote, snippet writer + backups + require line
- `crates/archel-core/src/hyprland/appearance.rs` - blur/opacity plan/apply/read-back, validators, FR label fns
- `crates/archel-core/src/hyprland/monitors.rs` - stub for 05-03-01
- `crates/archel-core/src/hyprland/binds.rs` - stub for 05-04-01
- `crates/archel-core/src/hyprland/wallpaper.rs` - stub for 05-02-01
- `crates/archel-core/src/lib.rs` - `pub mod hyprland`
- `crates/archel-core/src/error.rs` - HyprlandUnavailable + DesktopInvalid (constructors, message arms, every_variant)
- `crates/archel-core/locales/fr.json` - appearance.* + errors.hyprland_unavailable/desktop_invalid keys
- `crates/archel/src/cmds/appearance_cmds.rs` - blur/opacity runners, FR tables, --json, abort + snapshot tests
- `crates/archel/src/cmds/mod.rs` - Appearance/Opacity clap tree + completions words

## Decisions Made

- Live-apply via `hyprctl eval 'hl.config({...})'`: D-03/D-07 say "keyword" but keyword is dead on Hyprland 0.56.2 (returns "use eval", exits 0) — same intent, validated substitute per plan deviation note
- Opacity CLI takes `Option<String>` (not f64) so the typed DesktopInvalid validator — not a clap parse error — names every rejected value pre-spawn
- `plan_opacity_set` reads old leaves from the live snippet for the preview; bare `opacity` reads live compositor values directly (no confirm, no write)
- Live session state restored after every spot-check (blur true, opacities 1.0); user modules untouched, require line exactly once

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed duplicate blur helper definitions after opacity-section edit**
- **Found during:** Task 2 (opacity expansion)
- **Issue:** The opacity-section insertion left `blur_ok_line`/`blur_word` defined twice (E0428), blocking compilation
- **Fix:** Removed the duplicated copies, keeping the originals
- **Files modified:** crates/archel-core/src/hyprland/appearance.rs
- **Verification:** `cargo test -p archel-core --no-default-features appearance` green (11 passed)
- **Committed in:** 88e99b5 (part of task commit)

**2. [Rule 1 - Bug] Fixed Cell<String> and double-deref in new unit tests**
- **Found during:** Task 1 (tracer test compilation)
- **Issue:** `Rc<Cell<String>>` requires Copy; `**slot` over-dereferenced `&mut f64`
- **Fix:** Switched to `Rc<RefCell<String>>`; single deref assignment
- **Files modified:** crates/archel-core/src/hyprland/appearance.rs (tests only)
- **Verification:** `cargo test -p archel-core --no-default-features hyprland` green (14 passed)
- **Committed in:** 552d708 (part of task commit)

---

**Total deviations:** 2 auto-fixed (2 correctness bugs in new test/scaffold code)
**Impact on plan:** Both necessary for compilation; no scope creep, no architectural change.

## Issues Encountered

- `cargo clippy -p archel-core --all-features -- -D warnings` (plan verify leg) fails on **22 pre-existing lints** in updates.rs, sensors.rs, network.rs, bluetooth.rs, audio.rs — verified identical on the stashed baseline (zero lints reference any file touched by this plan). Left unfixed per scope boundary (pre-existing, unrelated); logged here instead of fixed.
- Test-only env hazard: `unset_signature_maps_to_typed_unavailable_without_spawning` mutates `HYPRLAND_INSTANCE_SIGNATURE` process-wide while tests run in parallel; it restores the value and no other test spawns hyprctl, so no interference observed — flagged for later plans adding session-dependent tests.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Shared seams ready for 05-02-01 (font/wallpaper), 05-03-01 (monitors + timed confirm), 05-04-01 (binds/input): runner, snippet writer, error variants, clap registration pattern, fr.json layout all established — later waves touch only their own files
- Zero new dependencies added (RESEARCH zero-new-deps gate held); no `hyprctl keyword/reload/hyprpaper` invocations anywhere (doc-comment mentions + a golden test asserting their absence only)
- Live compositor state left exactly as found (blur on, opacities 1.0/1.0)

## Self-Check: PASSED

- All 6 created files exist on disk; snapshot files pinned (appearance_blur, appearance_opacity)
- Commits `552d708` and `88e99b5` present in `git log`
- Workspace suite: 293 passed, 0 failed (`cargo test --workspace --no-default-features`)
- Prohibitions honored: no keyword/reload/hyprpaper spawns, no new deps, no writes outside app-owned paths (spy test + live md5 evidence)

---
*Phase: 05-hyprland-desktop*
*Completed: 2026-10-08*
