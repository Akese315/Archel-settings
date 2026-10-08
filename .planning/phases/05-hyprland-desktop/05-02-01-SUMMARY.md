---
phase: 05-hyprland-desktop
plan: 02-01
subsystem: desktop
tags: [hyprland, hyprpaper, gsettings, font, wallpaper, lua-quote, clap, insta]

# Dependency graph
requires:
  - phase: 05-hyprland-desktop plan 01-01
    provides: hyprland core module (run_hyprctl eval apply, getoption read-back, lua_quote, snippet writer with backup+atomic+require)
provides:
  - font core module (charset validation, gsettings get/set with read-back, snippet misc:font_family, eval apply)
  - wallpaper core module (path validation, hyprpaper slice render, guarded source line, pkill + setsid respawn)
  - hardened shared seams (eval-first ordering, millis backups, atomic require-line, typed timeout)
  - archel appearance font + wallpaper CLI surface (FR tables, --json, insta snapshots)
affects: [05-03-01, 05-04-01, 05-05-01, gui-phase]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
actuals:
  tokens: 25935
  tasks: 3
  commits: 3

# Tech tracking
tech-stack:
  added: []
  patterns: [two-store-font-apply, conf-write-plus-restart, validate-first-read-second, hostile-input-golden]

key-files:
  created:
    - crates/archel-core/src/hyprland/font.rs
    - crates/archel/src/cmds/snapshots/archel__cmds__appearance_cmds__tests__appearance_font.snap
    - crates/archel/src/cmds/snapshots/archel__cmds__appearance_cmds__tests__appearance_font_status.snap
    - crates/archel/src/cmds/snapshots/archel__cmds__appearance_cmds__tests__appearance_wallpaper.snap
    - crates/archel/src/cmds/snapshots/archel__cmds__appearance_cmds__tests__appearance_wallpaper_status.snap
  modified:
    - crates/archel-core/src/hyprland/mod.rs
    - crates/archel-core/src/hyprland/appearance.rs
    - crates/archel-core/src/hyprland/wallpaper.rs
    - crates/archel-core/locales/fr.json
    - crates/archel/src/cmds/appearance_cmds.rs
    - crates/archel/src/cmds/mod.rs

key-decisions:
  - "Eval-then-read-back-first ordering for all apply paths; snippet persists only on agreement (WR-01)"
  - "gsettings failures map to typed HyprlandUnavailable (session-tool family, never panic)"
  - "Trailing numeric font token always claims the size and fails loudly when out of range"
  - "Missing lua entry / user hyprpaper.conf refuses typed instead of planting stray files (WR-06 + analog)"
  - "Wallpaper restart is pkill + setsid only; setsid verified present live so no nohup fallback was needed"

patterns-established:
  - "Two-store mutation: eval + getoption read-back, then second-store set with its own read-back, persist only when both agree"
  - "Hostile-input golden: production statement pinned to lua_quote output plus pre-spawn rejection cases"
  - "Restart-only lifecycle argv builders (pkill/setsid/nohup) with a golden no-hyprctl assertion"

requirements-completed: [APP-01, APP-02]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "User sets system font from the CLI with GTK plus Hyprland live-apply, read-back, and snippet durability (APP-02)"
    requirement: "APP-02"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/hyprland/font.rs#font_eval_routes_through_lua_quote_with_hostile_rejection"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/hyprland/font.rs#apply_persists_only_when_both_legs_agree"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/appearance_cmds.rs#font_confirm_abort_spawns_no_write"
        status: pass
      - kind: e2e
        ref: "live: archel appearance font set 'Cantarell 12' flipped GTK + getoption misc:font_family with read-back (restored to Adwaita Sans 11 / Sans)"
        status: pass
    human_judgment: false
  - id: D2
    description: "User sets wallpaper from the CLI with conf-write plus daemon restart; invalid paths rejected pre-spawn (APP-01)"
    requirement: "APP-01"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/hyprland/wallpaper.rs#lifecycle_argv_is_restart_only_with_no_hyprctl_path"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/hyprland/wallpaper.rs#writer_touches_nothing_outside_the_hypr_trees"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/appearance_cmds.rs#wallpaper_confirm_abort_spawns_no_write"
        status: pass
      - kind: e2e
        ref: "live: archel appearance wallpaper set <winter-forest> rewrote slice + source line and restarted hyprpaper pid 902->68925 (conf restored md5-identical)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Shared seams hardened once for all later waves (WR-01 ordering, WR-05 stamps, WR-06 atomic require-line, WR-07 typed timeout)"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/hyprland/appearance.rs#apply_evals_first_and_persists_only_on_agreement"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/hyprland/mod.rs#back_to_back_writes_keep_distinct_backups"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/hyprland/mod.rs#require_line_refuses_a_missing_entry_atomically"
        status: pass
    human_judgment: false
  - id: D4
    description: "Swapped wallpaper is visibly shown on the desktop (visual confirmation of the restart mechanism)"
    verification: []
    human_judgment: true
    rationale: "Headless session cannot observe the compositor output; mechanism proven by pid change + slice content, visual landing needs a human glance"

# Metrics
duration: 13min
completed: 2026-10-08
status: complete
---

# Phase 05 Plan 02-01: Font + Wallpaper Summary

**System font (GTK gsettings + Hyprland leaf) and wallpaper (hyprpaper slice + kill/respawn) on the CLI with lua_quote on every string path, plus the four shared-seam hardenings later waves inherit**

## Performance

- **Duration:** 13 min
- **Started:** 2026-10-08T11:43:00Z
- **Completed:** 2026-10-08T11:55:56Z
- **Tasks:** 3
- **Files modified:** 11

## Accomplishments

- `archel appearance font` reads both stores (GTK `Adwaita Sans 11` + live `Sans`); `font set '<family> [size]'` live-applies via eval, confirms via getoption read-back, writes GTK with its own read-back, persists the `misc.font_family` snippet leaf (APP-02)
- `archel appearance wallpaper` reads the managed slice path; `wallpaper set <image>` rewrites the app slice, appends one end-anchored `source =` line, `pkill -x hyprpaper` + detached `setsid` respawn — no `hyprctl hyprpaper` anywhere (APP-01)
- Shared seams fixed once: eval-first persist-on-agreement ordering (WR-01), millis + loop-suffix backups (WR-05), atomic refuse-if-missing require-line (WR-06), typed timeout (WR-07); env-gate tests merged (WR-03), headless preconditions asserted not skipped (WR-04)
- Full workspace headless suite green: 339 passed, 0 failed; six new `--json` surfaces pinned by insta snapshots

## Task Commits

Each task was committed atomically:

1. **Task 1: Shared seam hardening** - `0f28adc` (feat)
2. **Task 2: Font slice** - `2d97f4a` (feat)
3. **Task 3: Wallpaper slice** - `7fb51ea` (feat)

## Files Created/Modified

- `crates/archel-core/src/hyprland/mod.rs` - WR-05/06/07 fixes, merged session-gate test, `pub mod font`
- `crates/archel-core/src/hyprland/appearance.rs` - WR-01 reorder both apply paths, hermetic `plan_opacity_set_at`, `font_family` snippet leaf
- `crates/archel-core/src/hyprland/font.rs` - validation, gsettings get/set, eval + read-back, snippet upsert, FR label fns
- `crates/archel-core/src/hyprland/wallpaper.rs` - path validation, slice render/parse, source line, kill/respawn lifecycle, FR label fns
- `crates/archel-core/locales/fr.json` - appearance.font_* + appearance.wallpaper_* keys
- `crates/archel/src/cmds/appearance_cmds.rs` - font + wallpaper runners, FR tables, --json, abort + snapshot tests
- `crates/archel/src/cmds/mod.rs` - Font/Wallpaper clap tree + completions words
- `crates/archel/src/cmds/snapshots/*appearance_{font,font_status,wallpaper,wallpaper_status}.snap` - pinned --json schemas

## Decisions Made

- Eval-then-read-back-first for every apply path (blur, opacity, font); snippet persists only on agreement; disagreement returns typed `HyprlandUnavailable` with the prior snippet untouched (WR-01 fix shape, extended to the two-store font path)
- `gsettings` failures map to `HyprlandUnavailable` (session-tool family): glib2 ships it on every Arch desktop, and the typed variant keeps the honest hint instead of a generic Internal
- Trailing numeric font token always claims the size: `Adwaita Sans 0` fails loudly as an invalid size instead of silently becoming a family name; control characters rejected upfront in the raw spec
- Missing lua entry / missing user hyprpaper.conf refuse typed instead of planting stray files the compositor never loads (WR-06 + wallpaper analog)
- `setsid` verified present live (`/usr/bin/setsid`), so the respawn uses the primary shim; the `nohup` shape stays coded and unit-tested as fallback
- Tilde paths canonicalize to absolute before slice render (live: `~/.config/...` became `/home/axel/.config/...`), so the daemon resolves them regardless of working directory

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Created font.rs stub in Task 1 so `pub mod font;` compiles**
- **Found during:** Task 1 (shared seam hardening)
- **Issue:** The plan orders `pub mod font;` in Task 1 but the module file only lands in Task 2 — declaring it without the file breaks the build and the per-task green gate
- **Fix:** Created `font.rs` as a stub in the Task 1 commit (same shape as the tracer's `wallpaper.rs` stub); Task 2 replaced the body
- **Files modified:** crates/archel-core/src/hyprland/font.rs
- **Verification:** `cargo test -p archel-core --no-default-features hyprland` green (20 passed)
- **Committed in:** 0f28adc (part of task commit)

**2. [Rule 2 - Missing Critical] Fixed the opacity planner validate-first shape alongside the four listed warnings**
- **Found during:** Task 1 (shared seam hardening)
- **Issue:** `plan_opacity_set` read the live home dir before validating inputs (WR-02) with no injectable base — same seam family as the mandated fixes, and its unit test touched the real home directory
- **Fix:** Added `plan_opacity_set_at(base, ...)` (validate first, read second), kept `plan_opacity_set` as the live-dir wrapper; hermetic test asserts garbage fails on a missing base
- **Files modified:** crates/archel-core/src/hyprland/appearance.rs
- **Verification:** `opacity_plan_requires_at_least_one_leaf` hermetic and green
- **Committed in:** 0f28adc (part of task commit)

**3. [Rule 1 - Bug] Fixed two font strictness gaps found by the new tests**
- **Found during:** Task 2 (font implementation)
- **Issue:** `Adwaita Sans 0` silently parsed as family `"Adwaita Sans 0"` (trailing zero swallowed); `"fam\nily"` whitespace-normalized to `"fam ily"` instead of failing
- **Fix:** Trailing numeric tokens always claim the size (out-of-range fails loudly); raw control characters rejected upfront in `parse_font_spec`
- **Files modified:** crates/archel-core/src/hyprland/font.rs
- **Verification:** `spec_parser_splits_optional_trailing_size` + hostile cases green
- **Committed in:** 2d97f4a (part of task commit)

**4. [Rule 1 - Bug] Fixed struct literals and imports after the AppearanceState extension**
- **Found during:** Task 2 (font leaf in snippet state)
- **Issue:** Three `AppearanceState` literals missed the new `font_family` field (E0063); `ENTRY_FILE` import unused outside tests; `read_option` unused in font.rs
- **Fix:** Added `font_family: None` to literals, moved `ENTRY_FILE` to test-only import, dropped unused import
- **Files modified:** crates/archel-core/src/hyprland/appearance.rs, crates/archel-core/src/hyprland/font.rs
- **Verification:** Scoped suites green, zero warnings
- **Committed in:** 2d97f4a (part of task commit)

---

**Total deviations:** 4 auto-fixed (2 correctness bugs, 1 missing-critical, 1 blocking)
**Impact on plan:** All necessary for compilation, test hermeticity, or injection strictness. No scope creep, no architectural change, zero new dependencies.

## Issues Encountered

- `packages::tests::live_unknown_name_is_the_honest_not_found_error` (and once its `live_install_preview` sibling) time out under full-workspace parallel load (`'yay' exceeded the query timeout`, 15 s) but pass in isolation in ~1 s; files untouched by this plan (pre-existing live-network flake, also noted in 05-01-01). Full suite re-run green: 339 passed, 0 failed.
- Clippy `-D warnings` stays red on the same 22 pre-existing lints outside plan files (logged in 05-01-01, untouched per scope boundary).
- Live restores were done by hand after each spot-check (snippet `cp` back + eval/aread restore) so the machine is byte-identical to pre-test; the tool's own backup cycle was exercised during the flips (millis-stamped backups observed live).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Shared seams ready for 05-03-01 (monitors + timed confirm) and 05-04-01 (binds/input): eval-first ordering, millis backups, atomic refuse-if-missing insertions, typed timeout, hermetic planners — later waves copy these shapes instead of the tracer's
- `AppearanceState.font_family` composes with blur/opacity in one managed block; snippet round-trips all three leaves
- Zero new dependencies (RESEARCH zero-new-deps gate held); no `hyprctl keyword/reload/hyprpaper` spawns (doc-comment mentions + golden no-hyprctl assertion only)
- Live compositor state left exactly as found (font Adwaita Sans 11 / Sans, user hyprpaper.conf md5-identical, require line exactly once)

## Self-Check: PASSED

- All 5 created files exist on disk; 4 new snapshot files pinned and content-reviewed
- Commits `0f28adc`, `2d97f4a`, `7fb51ea` present in `git log`
- Workspace suite: 339 passed, 0 failed (`cargo test --workspace --no-default-features`)
- Prohibitions honored: no keyword/reload/hyprpaper spawns (`wallpaper.rs` never calls `run_hyprctl`), no new deps, no writes outside app-owned paths (spy tests + live md5 evidence), no French diacritics in any `.rs` file

---
*Phase: 05-hyprland-desktop*
*Completed: 2026-10-08*
