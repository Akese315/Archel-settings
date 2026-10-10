---
phase: 03-packages-cli-harness
plan: 04
subsystem: cli
tags: [rust, pacman, aur, yay, i18n, pkg-install, guard]

# Dependency graph
requires:
  - phase: 03-packages-cli-harness (plan 03-03)
    provides: Existing aur-to-repo redirect (run_aur) and is_aur_card predicate this plan mirrors
provides:
  - run_install AUR guard redirecting AUR-only/mixed lists to pkg aur before confirm/helper
  - aur.is_aur_hint / aur.is_aur_hint_list French catalog keys plus accessors
  - Headless redirect-matrix unit tests (all-AUR, mixed refuse-all, repo-only passthrough)
affects: [03-packages-cli-harness UAT test 1, pkg install live flows]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
actuals:
  tokens: 2130
  tasks: 2
  commits: 2

# Tech tracking
tech-stack:
  added: []
  patterns: [refuse-all redirect for mixed repo+AUR install lists, pure card-partition helper for headless coverage]

key-files:
  created: []
  modified: [crates/archel/src/cmds/pkg_cmds.rs, crates/archel-core/src/aur.rs, crates/archel-core/locales/fr.json]

key-decisions:
  - "Mixed repo+AUR install lists refuse-all (install nothing) rather than partial-install the repo subset"
  - "Redirect JSON arm uses {cards + note} framing mirroring the run_aur repo-redirect object"
  - "No live run_install(hello) test headless — pure-helper tests plus pre-confirm placement are the proof"

patterns-established:
  - "Install guard: partition preview cards with is_aur_card before any render/confirm/spawn"
  - "French copy lives only in fr.json keys so the diacritics audit stays green"

requirements-completed: [PKG-02]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "pkg install of an AUR-only package stops before confirm/helper with a French redirect to pkg aur"
    requirement: "PKG-02"
    verification:
      - kind: unit
        ref: "crates/archel/src/cmds/pkg_cmds.rs#install_guard_redirects_all_aur_lists_with_every_name"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/pkg_cmds.rs#aur_repo_package_redirects_without_fetching_or_building"
        status: pass
    human_judgment: false
  - id: D2
    description: "Mixed repo+AUR install lists install nothing and redirect listing only the AUR names"
    requirement: "PKG-02"
    verification:
      - kind: unit
        ref: "crates/archel/src/cmds/pkg_cmds.rs#install_guard_refuses_mixed_lists_listing_only_aur_names"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/pkg_cmds.rs#install_guard_passes_repo_only_lists_through"
        status: pass
    human_judgment: false
  - id: D3
    description: "Live pkg install hello shows the redirect with no prompt, then a repo-package install completes the full confirm-and-stream flow"
    requirement: "PKG-02"
    verification: []
    human_judgment: true
    rationale: "Requires a live Arch machine with pacman/yay, a TTY for the [o/N] gate, and pkexec auth — cannot be proven headless"

# Metrics
duration: 12min
completed: 2026-10-01
status: complete
---

# Phase 03 Plan 04: Install AUR Guard Summary

**`pkg install` now redirects AUR-only and mixed lists to `pkg aur <name>` in French before any confirm prompt or helper spawn, closing UAT gap G-03-1**

## Performance

- **Duration:** 12 min
- **Started:** 2026-10-01T01:20:00Z
- **Completed:** 2026-10-01T01:32:00Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- AUR guard in `run_install`: any AUR-sourced preview card triggers a French redirect to `pkg aur` with exit 0, before card render, `confirm_action`, and helper spawn — in both human and `--json` modes
- Mixed repo+AUR lists use refuse-all semantics: nothing installs, only the AUR names are listed
- French catalog keys `aur.is_aur_hint` (singular) and `aur.is_aur_hint_list` (mixed) plus `is_aur_hint`/`is_aur_hint_list` accessors in `archel-core`
- Headless redirect-matrix tests: all-AUR redirects, mixed refuse-all, repo-only passthrough — full workspace suite green (52 + 111 + 18, includes diacritics audit)

## Task Commits

Each task was committed atomically:

1. **Task 1: AUR guard in run_install with catalog French** - `edcb5d9` (feat)
2. **Task 2: Mirror redirect tests headless-safe + diacritics audit** - `d5c9173` (test)

## Files Created/Modified

- `crates/archel/src/cmds/pkg_cmds.rs` - AUR guard in `run_install`, `aur_names_in_cards` pure helper, `print_install_aur_redirect` (human + JSON), redirect-matrix tests
- `crates/archel-core/src/aur.rs` - `is_aur_hint` / `is_aur_hint_list` catalog accessors, extended catalog test
- `crates/archel-core/locales/fr.json` - `aur.is_aur_hint` + `aur.is_aur_hint_list` French keys

## Decisions Made

- Mixed lists refuse-all rather than partial-install: a partial transaction the user did not confirm as partial would surprise; documented in the `run_install` doc comment
- Redirect JSON arm uses `{"cards" + "note"}` framing (same card-plus-note shape as the `run_aur` repo-redirect object); planner-discretion, no schema pin per plan
- No live `run_install("hello")` test headless: `package_card` spawns live `pacman -Si`/`yay -Si`, so the pure-helper tests plus pre-confirm placement are the headless proof (same pattern as the existing redirect test)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- Executor HEAD was on `master` (protected); created branch `agent-03-04` for the plan commits instead of committing to the default branch. Task files only — orchestrator-owned `.planning/` files left untouched.
- One stray whitespace edit during test authoring was immediately reverted (net-zero diff verified via `git diff`).

## Threat Flags

None - no new trust surface. Guard is read-only over preview cards (T-3-12), reuses the exact `is_aur_card` predicate (T-3-13), and adds no dependencies or spawns.

## Known Stubs

None.

## Self-Check: PASSED

- `crates/archel/src/cmds/pkg_cmds.rs` FOUND (guard + helper + 3 tests)
- `crates/archel-core/src/aur.rs` FOUND (2 accessors + catalog asserts)
- `crates/archel-core/locales/fr.json` FOUND (2 keys)
- Commits `edcb5d9`, `d5c9173` FOUND in `git log`
- `cargo test --workspace` green (52 + 111 + 18 passed, 0 failed)

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- G-03-1 truth holds at the unit level; UAT test 1 can proceed: `pkg install hello` should show the redirect, then `pkg install <tiny-repo-pkg>` completes the full confirm-and-stream flow (human verify-work item D3)
- `run_remove`/`run_upgrade` untouched per plan (removal targets must be installed; upgrade takes no names)

---
*Phase: 03-packages-cli-harness*
*Completed: 2026-10-01*
