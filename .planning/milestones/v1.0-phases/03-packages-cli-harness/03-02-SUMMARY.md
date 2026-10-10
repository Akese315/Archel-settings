---
phase: 03-packages-cli-harness
plan: 02
subsystem: packages
tags: [pacman, pkexec, polkit, duct, privilege-boundary, confirm-prompt, rust-i18n]

# Dependency graph
requires:
  - phase: 03-packages-cli-harness plan 01
    provides: D-04 card composer, query-spawn pattern, PackageNotFound/YayMissing variants reused by previews
  - phase: 01-foundations-safety-rails
    provides: helper allowlist + pkexec client patterns (126/127 mapping, v1 envelope, diacritics audit)
provides:
  - helper pkg-install/pkg-remove/pkg-upgrade verbs with validator matrix (T-3-04)
  - streaming privileged client (reader tee, transcript for --json) + 3 policy action blocks
  - confirm-then-stream pkg install/remove/upgrade CLI with D-03 partial-upgrade refusal
  - ConfirmationAborted, NeedsTty, InvalidPackageName, PartialUpgradeRefused typed errors with catalog French
affects: [03-03 (AUR review flow reuses confirm gate + streaming client), gui-packages-tab]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
# Same estimateTokens scale (chars/4 over the realized diff), never a harness token count.
actuals:
  tokens: 14652
  tasks: 3
  commits: 4

# Tech tracking
tech-stack:
  added: []
  patterns: [verb-plus-validated-args helper allowlist, last-line v1 envelope over live transcript, reader-tee streaming with callback, TTY-gated default-No confirm, unprivileged --print previews]

key-files:
  created: []
  modified:
    - crates/archel-privileged/src/main.rs
    - crates/archel-core/src/privileged.rs
    - crates/archel-core/src/packages.rs
    - crates/archel-core/src/error.rs
    - crates/archel-core/locales/fr.json
    - crates/archel/src/cmds/pkg_cmds.rs
    - crates/archel/src/cmds/mod.rs
    - policy/org.archel.privileged.policy

key-decisions:
  - "Plain `pacman -R` (not -Rns) for pkg-remove: recursion stays opt-in (A8 resolution) — -Rns also drops configs and orphans, a destructive default for a settings app"
  - "Helper prints the v1 envelope as the LAST stdout line after the live pacman transcript; client parses only the final line, so parse_helper_stdout needed no change"
  - "`pkg upgrade <name>` is the D-03 refusal trigger (closest mappable input): no -Sy constructor exists anywhere, only full -Syu"
  - "--json mutations suppress per-line printing and emit one report object with the transcript array (D-02 live path is the human path; transcript array is the script record)"
  - "Streaming core takes an on_line callback: the bin prints live, tests capture into a vec with zero stdout pollution"

patterns-established:
  - "run_privileged_streaming(verb, packages, action_label, on_line): unchecked reader() + BufRead tee + try_wait code map + last-line envelope parse — Pitfall 7 deadlock avoided"
  - "confirm_action(prompt): non-TTY stdin → NeedsTty before any prompt; o/O/ou/oui-prefix confirms, everything else (empty, EOF, non) → ConfirmationAborted"
  - "validate_package_names (client fast-fail mirror of the helper charset) runs before any preview spawn or pkexec prompt; helper re-validates server-side regardless"

requirements-completed: [PKG-02, PKG-03, PKG-04]

# Coverage metadata (#1602) — one entry per shipped deliverable. Drives DETERMINISTIC UAT routing in verify-work.
coverage:
  - id: D1
    description: "archel pkg install <name> shows deps, asks [o/N], and streams live pacman output after confirm"
    requirement: "PKG-02"
    verification:
      - kind: unit
        ref: "crates/archel-privileged/src/main.rs#install_argv_hardcodes_flags_with_separator_before_packages"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/packages.rs#live_install_preview_lists_cards_and_dependencies"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/privileged.rs#collector_tees_live_lines_and_accumulates_the_transcript"
        status: pass
    human_judgment: true
    rationale: "Live install mutates the dev machine through pkexec (needs polkit agent + TTY): automation proves preview/validator/streaming, a human must run one real install"
  - id: D2
    description: "archel pkg remove <name> follows the same confirm-then-stream flow via the helper, never as root"
    requirement: "PKG-03"
    verification:
      - kind: unit
        ref: "crates/archel-privileged/src/main.rs#remove_argv_uses_plain_recursive_free_remove"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/packages.rs#live_remove_preview_requires_installed_targets"
        status: pass
    human_judgment: true
    rationale: "Live remove mutates the dev machine: same human-gated pkexec proof as install"
  - id: D3
    description: "archel pkg upgrade runs full -Syu; pkg upgrade <name> is refused with the French partial-upgrade message"
    requirement: "PKG-04"
    verification:
      - kind: unit
        ref: "crates/archel-privileged/src/main.rs#upgrade_argv_is_the_full_sync_upgrade_only"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/pkg_cmds.rs#upgrade_with_a_package_name_is_the_refused_partial_path"
        status: pass
      - kind: integration
        ref: "cargo run -p archel -- pkg upgrade firefox (French D-03 refusal, exit 1) + pkg upgrade (up-to-date, exit 0)"
        status: pass
    human_judgment: true
    rationale: "Full -Syu on the dev machine is a real system upgrade: human-gated by definition"
  - id: D4
    description: "Crafted package names (-foo, --overwrite, empty) are rejected with exit 2 and nothing executes"
    requirement: "PKG-02"
    verification:
      - kind: unit
        ref: "crates/archel-privileged/src/main.rs#crafted_package_names_are_rejected_with_exit_two"
        status: pass
      - kind: unit
        ref: "crates/archel-privileged/src/main.rs#smuggled_extra_flag_after_a_valid_package_is_rejected"
        status: pass
      - kind: integration
        ref: "cargo run -p archel -- pkg install -- -foo (InvalidPackageName French, exit 1, no pkexec prompt)"
        status: pass
    human_judgment: false

# Metrics
duration: ~50min
completed: 2026-10-01
status: complete
---

# Phase 03 Plan 02: Privileged Pacman Mutations Summary

**Helper pkg-install/remove/upgrade verbs with injection-proof validator, reader-tee streaming pkexec client, and confirm-then-stream CLI with French D-03 partial-upgrade refusal — PKG-02/03/04 without ever running Archel as root**

## Performance

- **Duration:** ~50min
- **Started:** 2026-10-01T10:53Z
- **Completed:** 2026-10-01T11:45Z (approx)
- **Tasks:** 3 of 3
- **Files modified:** 8 (0 created, 8 modified)

## Accomplishments

- Helper speaks three mutation verbs: `pkg-upgrade` exact-match → `pacman -Syu --noconfirm`, `pkg-install` → `-S`, `pkg-remove` → plain `-R`, every package arg charset+leading-dash gated, `--` separator, absolute `/usr/bin/pacman`, v1 envelope printed last — 18-test matrix green including all injection cases
- Streaming client (`pkg_install`/`pkg_remove`/`pkg_upgrade`) tees every helper line live via callback plus transcript for `--json`, reusing `locate_helper`/`map_pkexec_exit`/`parse_helper_stdout` verbatim; 126/127 render distinct French naming the catalog action
- Policy carries four least-privilege blocks (dry-run + 3 verbs), `auth_admin_keep` triple, never `yes`, validates as XML
- `archel pkg install|remove|upgrade` run preview → TTY-gated `[o/N]` (o/O/oui-prefix, default-No) → pkexec stream; `upgrade <name>` hits the D-03 French refusal; empty `-Qu` reports honestly up-to-date with no prompt
- 157 headless tests green (39 archel + 100 core + 18 helper), zero warnings, `rg '"-Sy"'` negative gate clean, diacritics audit green

## Task Commits

Each task was committed atomically:

1. **Task 1: Helper verb allowlist plus validator matrix (TDD)** - `85f4b26` (test, RED) + `7b313ec` (feat, GREEN)
2. **Task 2: Streaming privileged client plus policy action blocks** - `1075346` (feat)
3. **Task 3: Confirm-then-stream install/remove/upgrade CLI** - `fad49e2` (feat)

_TDD task 1 has two commits (RED matrix-then-GREEN mapping) per the RED-GREEN contract_

## Files Created/Modified

- `crates/archel-privileged/src/main.rs` - Decision enum, charset validator, 3 verb argv maps, run_pacman with inherited stdio + last-line envelope, 18-test matrix
- `crates/archel-core/src/privileged.rs` - pkg_* verbs, argv builder, stream_envelope_line, collect_streaming, run_privileged_streaming, 8 new tests
- `crates/archel-core/src/packages.rs` - validate_package_names, install/remove/upgrade previews, parse_confirm_answer, confirm_action, catalog label fns, 9 new tests
- `crates/archel-core/src/error.rs` - ConfirmationAborted, NeedsTty, InvalidPackageName, PartialUpgradeRefused variants + arms + coverage
- `crates/archel-core/locales/fr.json` - pkg confirm/preview/success/action keys, 4 new errors.* keys
- `crates/archel/src/cmds/pkg_cmds.rs` - run_install/run_remove/run_upgrade, preview rendering, mutation report JSON, 4 new tests
- `crates/archel/src/cmds/mod.rs` - PkgAction Install/Remove/Upgrade + dispatch
- `policy/org.archel.privileged.policy` - pkg-install/remove/upgrade action blocks

## Decisions Made

- Plain `pacman -R` for remove (A8 resolution): `-Rns` drops configs and orphans — destructive default the plan-check questioned; recursion becomes an opt-in flag in a later pass, and the preview (`-R --print`) matches the execution flags exactly
- Last-line v1 envelope: helper inherits stdio for D-02 liveness, then prints the envelope; client parses only the final transcript line — `parse_helper_stdout` untouched, dry-run path behavior-identical (whole stdout is the envelope)
- `pkg upgrade <name>` is the D-03 refusal trigger: the only user-reachable partial-upgrade-shaped input, refused in French before any spawn; verified live (`pkg upgrade firefox` → refusal exit 1, `pkg upgrade` → up-to-date exit 0)
- `--json` mutations stay single-object: live printing suppressed, transcript array inside the report — D-02 streaming is the human path, the array is the script record (report schema: action/packages/ok/transcript/note)
- Callback-based streaming (`on_line: &mut dyn FnMut(&str)`): bin passes the printer, tests pass a vec collector — no stdout pollution in the suite, GUI-ready later

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added InvalidPackageName + PartialUpgradeRefused variants (error.rs not in task 3 file list)**
- **Found during:** Task 3 (CLI fast-fail + D-03 refusal need typed homes)
- **Issue:** Plan assigned only ConfirmationAborted/NeedsTty (task 2); crafted names at CLI level and the D-03 refusal had no typed variant — without them the CLI would surface generic HelperProtocol French for input errors
- **Fix:** Added both variants with message arms, every_variant entries, and fr.json keys (invalid_package_name key staged in task 2 commit alongside its siblings)
- **Files modified:** crates/archel-core/src/error.rs, crates/archel-core/locales/fr.json
- **Verification:** every_variant_renders covers 14 variants; live `pkg install -- -foo` renders the InvalidPackageName French
- **Committed in:** fad49e2 (part of task commit)

**2. [Rule 1 - Bug] Corrected the plan's policy count gate (9 → 12 element occurrences)**
- **Found during:** Task 2 verification
- **Issue:** Plan's `fails_when` demanded `auth_admin_keep` count of 9 (3 blocks × 3), forgetting the Phase-1 dry-run block still in the file; raw `grep -c` also matches the pre-existing XML comment line (total 13). Enforcing 9 would mean deleting the dry-run block — breaking Phase 1
- **Fix:** Kept all 4 blocks; gate counts element occurrences only: 12 (4 × 3), no `yes`, minidom-valid XML
- **Files modified:** policy/org.archel.privileged.policy
- **Verification:** `grep -c '<allow_.*>auth_admin_keep</allow_.*>'` → 12; 4 action ids; XML parses
- **Committed in:** 1075346 (part of task commit)

**3. [Rule 1 - Bug] Wired pkg_helper_argv into run_privileged_streaming (dead-code warning)**
- **Found during:** Task 3 (full-suite run surfaced `pkg_helper_argv is never used`)
- **Issue:** The runner built its PathBuf argv inline, leaving the tested builder dead in non-test builds
- **Fix:** Runner now extends from `pkg_helper_argv(verb, packages)` — one argv construction site, still covered by the builder tests
- **Files modified:** crates/archel-core/src/privileged.rs
- **Verification:** zero warnings; full suite green
- **Committed in:** fad49e2 (part of task commit)

**4. [Rule 1 - Bug] Restored run_info after an overwrite slip**
- **Found during:** Task 3 (replaced `run_info` instead of appending after it)
- **Issue:** Edit tooling replaced the run_info block with the new orchestrators — `pkg info` would have vanished from the binary
- **Fix:** Re-added run_info verbatim before run_install; confirmed via grep + full suite (info snapshot tests pass)
- **Files modified:** crates/archel/src/cmds/pkg_cmds.rs
- **Verification:** `schema_pkg_info_is_pinned` + info table tests pass; `pkg info` dispatch intact in mod.rs
- **Committed in:** fad49e2 (part of task commit)

---

**Total deviations:** 4 auto-fixed (2 bugs, 1 missing-critical, 1 correctness-gate fix + 1 editing slip recovered)
**Impact on plan:** All auto-fixes necessary for correctness/audit-compliance. No scope creep. No architectural changes.

## Issues Encountered

- clap intercepts leading-dash package args before they reach any Archel code (`pkg install -foo` → clap exit 2 usage error). The helper/client gates still matter: `pkg install -- -foo` passes clap and hits the InvalidPackageName fast-fail (verified live, exit 1, no pkexec prompt) — defense in depth across three layers (clap, client, helper)
- Live install/remove/upgrade through pkexec remain manual verify-work items: this shell has no polkit agent or TTY, so streaming is proven by unit (collector + duct reader pattern + envelope extraction) and by the honest no-mutation paths live (refusal, invalid name, up-to-date). A human must run one real install/remove/upgrade on the Hyprland session
- PKG-02 already-installed reinstall wording and PKG-03/04 idempotency use honest already-there output per plan (card shows installed==candidate; pacman reports reinstalling) — no fake-zero states, no special-casing

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. `note: null` on mutation reports and empty `transcript: []` on the up-to-date path are modeled absences (nothing to transcribe), serialized honestly and never rendered as fake data.

## Threat Flags

None beyond the plan's threat model. Register coverage delivered:
- T-3-04 (flag injection): charset + leading-dash reject + `--` + server-side flags, matrix-tested on both helper and client mirror; no shell anywhere; absolute pacman path
- T-3-05 (verb surface): 3 verbs, one policy action each, auth_admin_keep triple never yes; helper re-validates every invocation
- T-3-06 (partial upgrade): no `-Sy` constructor (negative grep gate clean); DB refresh only inside `-Syu`; D-03 French refusal on `pkg upgrade <name>`
- T-3-07 (transcript vs stream, accepted): transcript accumulates exactly the lines delivered live; stderr stays log-only

## Next Phase Readiness

- PKG-02/03/04 delivered behind the Phase 1 privilege boundary with D-01 confirmation, D-02 streaming, D-03 refusal; headless proofs green, live pkexec mutations queued for verify-work on the desktop
- Ready for 03-03 (AUR review flow): confirm gate, streaming client, and report framing are the exact reuse points; yay stays unprivileged per D-06
- Carry-forward: A8 recursion opt-in flag for remove (plain -R ships as the safe default)

---
*Phase: 03-packages-cli-harness*
*Completed: 2026-10-01*

## Self-Check: PASSED

- SUMMARY.md exists on disk; all 4 task commits verified in git log (85f4b26, 7b313ec, 1075346, fad49e2)
- Headless suite green at commit time (39 archel + 100 core + 18 helper, zero warnings); negative `rg '"-Sy"'` gate clean; diacritics audit green
