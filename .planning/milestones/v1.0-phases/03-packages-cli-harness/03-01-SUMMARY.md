---
phase: 03-packages-cli-harness
plan: 01
subsystem: packages
tags: [alpm, pacman, yay, aur, clap, duct, insta, rust-i18n]

# Dependency graph
requires:
  - phase: 02-system-insight
    provides: CLI shape precedent (sys group, French tables, global --json, insta pins)
  - phase: 01-foundations-safety-rails
    provides: which-gated duct spawn pattern, typed ArchelError + fr.json catalog, diacritics audit
provides:
  - packages.rs core module (list/search/card composers, LC_ALL=C-pinned queries, epoch-aware sort)
  - pkg CLI group (list/search/info) with French tables and --json on every subcommand
  - PackageNotFound + YayMissing typed errors with catalog French
  - insta pins for pkg list/search/info JSON schemas
affects: [03-02 (mutation plan reuses card composer + error variants), gui-packages-tab]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
# Same estimateTokens scale (chars/4 over the realized diff), never a harness token count.
actuals:
  tokens: 19119
  tasks: 3
  commits: 3

# Tech tracking
tech-stack:
  added: [alpm 5.0.2 with checkver feature]
  patterns: [LC_ALL=C-pinned parse-bound spawns, epoch-aware version-desc sort, card composer in core]

key-files:
  created:
    - crates/archel-core/src/packages.rs
    - crates/archel/src/cmds/pkg_cmds.rs
    - crates/archel/src/cmds/snapshots/archel__cmds__pkg_cmds__tests__pkg_list.snap
    - crates/archel/src/cmds/snapshots/archel__cmds__pkg_cmds__tests__pkg_search.snap
    - crates/archel/src/cmds/snapshots/archel__cmds__pkg_cmds__tests__pkg_info.snap
  modified:
    - crates/archel-core/Cargo.toml
    - crates/archel-core/src/lib.rs
    - crates/archel-core/src/error.rs
    - crates/archel-core/locales/fr.json
    - crates/archel/src/cmds/mod.rs
    - Cargo.lock

key-decisions:
  - "Merged version-desc search ordering with repo-first tie-break (D-07 literal), not repo/AUR blocks"
  - "Blank search query short-circuits to honest-empty (unresolved specless-probe item surfaced, not guessed)"
  - "PackageInfo.source is Option<String>: None for installed list (no source concept), Some for search/card"
  - "AUR source defaults to AUR when yay -Si carries no Repository field (offline-robust)"

patterns-established:
  - "run_capped_query: which-gated duct spawn with .unchecked() + LC_ALL=C + wait_timeout, exit codes flow to a mapper"
  - "Parse functions are pure over fixture strings (parse_search_output, parse_info_fields) — LC_ALL=C samples as unit fixtures"

requirements-completed: [PKG-01, PLAT-02]

# Coverage metadata (#1602) — one entry per shipped deliverable. Drives DETERMINISTIC UAT routing in verify-work.
coverage:
  - id: D1
    description: "archel pkg list shows every installed package with version in a French table"
    requirement: "PKG-01"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/packages.rs#installed_list_reads_from_the_localdb"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/pkg_cmds.rs#list_table_shows_every_name_with_its_version"
        status: pass
      - kind: integration
        ref: "cargo run -p archel -- pkg list (1147 packages, exit 0)"
        status: pass
    human_judgment: false
  - id: D2
    description: "archel pkg search merges repos + AUR version-descending, repos first on ties"
    requirement: "PKG-01"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/packages.rs#hits_sort_version_descending_with_repos_before_aur_on_ties"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/packages.rs#search_parse_reads_repo_headers_skipping_descriptions"
        status: pass
      - kind: integration
        ref: "cargo run -p archel -- pkg search firefox (972 hits, epoch-2 first, exit 0)"
        status: pass
    human_judgment: false
  - id: D3
    description: "archel pkg info shows the D-04 pre-install card (sizes, source, candidate vs installed)"
    requirement: "PKG-01"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/packages.rs#live_card_shows_sizes_source_and_version_compare"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/pkg_cmds.rs#info_table_shows_sizes_source_and_both_versions"
        status: pass
      - kind: integration
        ref: "cargo run -p archel -- pkg info firefox (card + --json parses, exit 0)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Every new pkg subcommand emits one parseable JSON object under --json (D-10)"
    requirement: "PLAT-02"
    verification:
      - kind: unit
        ref: "crates/archel/src/cmds/pkg_cmds.rs#schema_pkg_list_is_pinned + schema_pkg_search_is_pinned + schema_pkg_info_is_pinned"
        status: pass
      - kind: integration
        ref: "pkg list/search/info --json | python3 -c json.load (all three parse)"
        status: pass
    human_judgment: false

# Metrics
duration: ~75min
completed: 2026-10-01
status: complete
---

# Phase 03 Plan 01: Read-Side Package Foundation Summary

**alpm localdb installed list, repo+AUR version-descending search, and D-04 pre-install card — all drivable from `archel pkg list|search|info` with French tables and `--json`**

## Performance

- **Duration:** ~75min
- **Started:** 2026-10-01T10:35Z (approx)
- **Completed:** 2026-10-01T12:50Z (approx)
- **Tasks:** 3 of 3
- **Files modified:** 13 (6 created, 7 modified incl. Cargo.lock)

## Accomplishments

- `archel pkg list` end-to-end: alpm localdb read, epoch-aware version-desc sort, French table of 1147 installed packages; `--json` parses
- `archel pkg search <term>`: `LC_ALL=C`-pinned `pacman -Ss` + `yay -Ss` merged version-descending with repos before AUR on ties (972 hits for `firefox`, epoch `2:` versions lead); blank query renders honest-empty; missing yay yields typed French hint
- `archel pkg info <name>`: D-04 card composer in core (download + installed sizes, source repo-or-AUR, candidate vs installed side by side); unknown names yield honest `PackageNotFound` French error, never a zero-filled card
- `PackageNotFound` + `YayMissing` typed errors with `fr.json` catalog keys; `every_variant_renders` covers all variants; diacritics audit green
- insta pins for all three `--json` schemas (`pkg_list`, `pkg_search`, `pkg_info`); full headless workspace suite green (35 + 85 + 5 tests)

## Task Commits

Each task was committed atomically:

1. **Task 1: End-to-end `archel pkg list`** - `db2a577` (feat)
2. **Task 2: Repo+AUR search with version-desc, repos-first ordering** - `35f7c7e` (feat)
3. **Task 3: Pre-install card composer plus JSON snapshot pins** - `4a4c2ff` (feat)

## Files Created/Modified

- `crates/archel-core/src/packages.rs` - PackageInfo/PackageCard types, list/search/card composers, C-locale parsers, D-07 sort
- `crates/archel/src/cmds/pkg_cmds.rs` - run_list/run_search/run_info, French tables, JSON framing, insta pins
- `crates/archel/src/cmds/snapshots/archel__cmds__pkg_cmds__tests__pkg_{list,search,info}.snap` - pinned --json schemas
- `crates/archel-core/Cargo.toml` - alpm 5.0.2 + checkver
- `crates/archel-core/src/lib.rs` - `pub mod packages`
- `crates/archel-core/src/error.rs` - PackageNotFound + YayMissing variants, message arms, variant coverage
- `crates/archel-core/locales/fr.json` - pkg.* table/card labels, errors.package_not_found/yay_missing
- `crates/archel/src/cmds/mod.rs` - Pkg group (List/Search/Info) + dispatch
- `Cargo.lock` - alpm dependency closure

## Decisions Made

- Merged version-desc search ordering with repo-first tie-break (D-07 literal + user's "version puis serveur"), not repo/AUR display blocks — an AUR hit with a genuinely higher version sorts first; ties (same package both sides) always lead with the repo record
- Blank search query short-circuits to `Ok(empty)` without spawning — the PKG-01 empty-query semantics are an unresolved specless-probe item, surfaced here instead of guessed (no full-DB dump)
- `PackageInfo.source: Option<String>` — `None` for the installed list (no source concept per D-08), `Some(repo|AUR)` for search/card; serializes as honest null, not an empty-string stub
- AUR source defaults to `"AUR"` when `yay -Si` carries no `Repository` field — robust to yay's AUR record shape without network access to verify it
- `pacman -Ss/-Si` exit 1 maps to empty output (verified live), not an error — no-match renders the honest empty table/card-fallback

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added `pkg.*` catalog keys in task 1 (fr.json not in task 1 file list)**
- **Found during:** Task 1 (tracer table labels)
- **Issue:** The tracer renders French labels through `list_table_labels()`; without catalog keys the code would either hardcode French (failing the diacritics audit) or have no labels
- **Fix:** Added `pkg.list_title/list_name/list_version` to fr.json in the task 1 commit
- **Files modified:** crates/archel-core/locales/fr.json
- **Verification:** diacritics audit passes; table renders "Paquets installés"
- **Committed in:** db2a577 (part of task commit)

**2. [Rule 1 - Bug] Added duct `.unchecked()` to the query spawn**
- **Found during:** Task 3 (`live_unknown_name_is_the_honest_not_found_error` failed)
- **Issue:** duct's `Handle::wait_timeout` returns Err on ANY non-zero exit unless the expression is `.unchecked()` (proven with a minimal probe crate) — so pacman's exit-1 no-match never reached `map_query_exit` and surfaced as `Internal` instead of honest-empty/not-found. The copied `updates.rs` spawn shape has the same latent shape
- **Fix:** `.unchecked()` on the `run_capped_query` spawn plus a load-bearing comment; exit codes now flow to `map_query_exit`
- **Files modified:** crates/archel-core/src/packages.rs
- **Verification:** no-match search renders honest empty (exit 0); unknown info yields PackageNotFound (exit 1); full suite green
- **Committed in:** 4a4c2ff (part of task commit)

**3. [Rule 2 - Missing Critical] Extended `every_variant_renders` coverage in task 2 (plan assigned it to task 3)**
- **Found during:** Task 2 (new error variants added)
- **Issue:** New `PackageNotFound`/`YayMissing` variants would ship without render coverage until task 3
- **Fix:** Added both variants to `every_variant()` immediately
- **Files modified:** crates/archel-core/src/error.rs
- **Verification:** `every_variant_renders_non_empty_french` passes with 10 variants
- **Committed in:** 35f7c7e (part of task commit)

---

**Total deviations:** 3 auto-fixed (1 bug, 2 missing-critical)
**Impact on plan:** All auto-fixes necessary for correctness/audit-compliance. No scope creep. No architectural changes.

## Issues Encountered

- `yay -Si` on AUR packages could not be verified live: this sandbox has no external DNS (`aur.archlinux.org` unresolvable), so the AUR card path is proven by fixture tests (parse + composer + `AUR` source default) but not by a live AUR record. Repo card path verified live (`pkg info firefox`). Live AUR review belongs to verify-work on the desktop.
- `yay -Ss` also returns repo hits, so search output contains adjacent duplicates (same package from both sources, e.g. `otf-fira-mono` twice). This is the plan-flagged PKG-01 duplicate-adjacency specless-probe item — carried forward unresolved, not dismissed.
- Pre-existing (out of scope, NOT fixed): `updates.rs::run_checker` shares the checked-spawn shape — `checkupdates` exit 2 (clean system) can never reach `map_update_exit`'s explicit Ok arm and degrades to stale/unavailable instead of `Fresh(0)`. Logged in `.planning/phases/03-packages-cli-harness/deferred-items.md` (untracked, for the orchestrator to route).

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. `source: None` on list entries is a modeled absence (D-08: installed list has no source concept), serialized as honest null and never rendered.

## Threat Flags

None beyond the plan's threat model. New surface introduced and mitigated per register:
- T-3-01 (localized parse): every parse-bound spawn pins `LC_ALL=C` via duct `.env()`; fixture tests feed C-locale samples; stderr stays log-only
- T-3-03 (localdb open/read): open failure maps to typed `ArchelError::Internal` with catalog French; never panics; empty DB renders honest empty table
- No bare `-Sy` or sync-without-upgrade path exists (negative grep gate passes); no package-mutation French copy outside `fr.json` keys

## Next Phase Readiness

- PKG-01 delivered (installed-only list per D-08, global search per D-07); D-04 card composer proven and reusable by mutation plan 03-02; D-10 JSON framing on all three subcommands
- Foundation ready for 03-02 (privileged install/remove/upgrade + AUR review flow): card composer, error variants, and query-spawn pattern are the exact reuse points
- Open carry-forwards for 03-02/verify-work: PKG-01 empty-query semantics, duplicate repo/AUR adjacency, live AUR card on a networked machine, `updates.rs` exit-2 deferred item

---
*Phase: 03-packages-cli-harness*
*Completed: 2026-10-01*
