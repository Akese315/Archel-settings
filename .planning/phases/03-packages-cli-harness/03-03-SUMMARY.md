---
phase: 03-packages-cli-harness
plan: 03
subsystem: packages
tags: [aur, yay, pkgbuild, review-before-build, clap_complete, completions, insta, privilege-boundary, rust-i18n]

# Dependency graph
requires:
  - phase: 03-packages-cli-harness plan 01
    provides: D-04 card composer, LC_ALL=C query-spawn pattern, PackageCard schema reused by the review
  - phase: 03-packages-cli-harness plan 02
    provides: TTY-gated confirm gate, reader-tee streaming shape, mutation report JSON framing reused by the AUR build
provides:
  - aur.rs core module (yay -G fetch, bash -n gate, danger heuristics, -Si trust signals + dep preview, unprivileged yay -S streaming)
  - pkg aur review-then-build CLI with repo redirect and abort-builds-nothing gates
  - archel completions <bash|zsh|fish> generated from the single clap definition
  - PkgbuildRejected typed error with catalog French
  - insta pins for the AUR review/result JSON schemas
affects: [gui-packages-tab (PKG-06 diff review builds on the AurReview shape), verify-work (live AUR review-then-build)]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
# Same estimateTokens scale (chars/4 over the realized diff), never a harness token count.
actuals:
  tokens: 18400
  tasks: 3
  commits: 4

# Tech tracking
tech-stack:
  added: [clap_complete 4.6.11 (aot::generate path)]
  patterns: [review-before-build gate (fetch + validate + display + confirm + build), injectable build-closure seam for abort-path tests, full-PKGBUILD-text-in-review JSON]

key-files:
  created:
    - crates/archel-core/src/aur.rs
    - crates/archel/src/cmds/snapshots/archel__cmds__pkg_cmds__tests__aur_review.snap
    - crates/archel/src/cmds/snapshots/archel__cmds__pkg_cmds__tests__aur_result.snap
  modified:
    - crates/archel-core/src/lib.rs
    - crates/archel-core/src/error.rs
    - crates/archel-core/locales/fr.json
    - crates/archel/Cargo.toml
    - crates/archel/src/cmds/mod.rs
    - crates/archel/src/cmds/pkg_cmds.rs
    - Cargo.lock

key-decisions:
  - "Post-confirm build flags are exactly `yay -S <name> --noconfirm` (A1/A2 resolution): --noconfirm answers only yay's own menus, all superseded by Archel's D-05 review; sudo password prompting is untouched (comes from sudo itself); --answerdiff/--answeredit combo and --sudoloop deliberately not used"
  - "Heuristic danger patterns warn, never reject — bash -n failure is the only automatic reject (user validates per D-05)"
  - "TTY pre-refusal is structural: confirm_action gates before any yay -S spawn, and the abort seam is unit-proven with a spawn probe (no network in tests)"
  - "Repo packages through `pkg aur` redirect to `pkg install` after the card pre-check, before any yay -G fetch"

patterns-established:
  - "run_aur_confirmed(name, json, build-closure): injectable post-review seam — abort path unit-covered headless with an AtomicUsize spawn probe"
  - "completions_for(shell) test helper: generate-to-buffer per shell, assert non-empty + archel reference + full-tree word coverage"

requirements-completed: [PKG-05, PLAT-02]

# Coverage metadata (#1602) — one entry per shipped deliverable. Drives DETERMINISTIC UAT routing in verify-work.
coverage:
  - id: D1
    description: "archel pkg aur <name> shows the full PKGBUILD plus trust signals and builds nothing without explicit validation"
    requirement: "PKG-05"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/aur.rs#abort_decision_returns_the_abort_variant_without_spawning_a_build"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/pkg_cmds.rs#aur_confirm_abort_spawns_no_build"
        status: pass
      - kind: integration
        ref: "cargo run -p archel -- pkg aur hello (live review fetched + rendered, stopped at NeedsTty, exit 1)"
        status: pass
    human_judgment: true
    rationale: "Real AUR review-then-build mutates the dev machine through yay/sudo: automation proves fetch/validate/gates, a human must run one real review-then-abort on the Hyprland session"
  - id: D2
    description: "yay runs unprivileged with native sudo handling; Archel never orchestrates elevation on the AUR path"
    requirement: "PKG-05"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/aur.rs#missing_yay_maps_to_the_typed_hint"
        status: pass
      - kind: integration
        ref: "pkg aur pacman (repo redirect, exit 0) + pkg aur --json -- -foo (InvalidPackageName French, exit 1, no spawn)"
        status: pass
    human_judgment: true
    rationale: "End-to-end sudo interplay on a real build stays manual verify-work by plan"
  - id: D3
    description: "archel completions <bash|zsh|fish> emits working scripts covering every subcommand"
    requirement: "PLAT-02"
    verification:
      - kind: unit
        ref: "crates/archel/src/cmds/mod.rs#completions_cover_the_full_command_tree"
        status: pass
      - kind: integration
        ref: "completions bash/zsh/fish | grep -c archel (206/233/128, all non-empty)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Every Phase 2-3 capability accepts --json (reads emit parseable objects, mutations fast-fail typed)"
    requirement: "PLAT-02"
    verification:
      - kind: integration
        ref: "17-path --json matrix green (landing, 7 sys metrics, list/search/info/aur reads parse; install/remove/aur fast-fails + upgrade refusal render typed French)"
        status: pass
    human_judgment: false

# Metrics
duration: ~90min
completed: 2026-10-01
status: complete
plan_head_before: 734c32a2d645541ef0a01d12a3060c3a13db5a88
plan_head_after: a841219870bbba5b969768acdb6f0cc6d71268fd
---

# Phase 03 Plan 03: AUR Review plus CLI Harness Summary

**AUR review-before-build with the D-05 trust boundary intact, unprivileged yay with native sudo handling, and shell completions closing the PLAT-02 harness — PKG-05 delivered, every Phase 2-3 capability terminal-drivable**

## Performance

- **Duration:** ~90min
- **Started:** 2026-10-01T11:07Z
- **Completed:** 2026-10-01T12:40Z (approx)
- **Tasks:** 3 of 3
- **Files modified:** 10 (3 created, 7 modified incl. Cargo.lock)

## Accomplishments

- `archel-core::aur`: `yay -G` temp-dir fetch as the invoking user, `bash -n` static gate mapping to typed `PkgbuildRejected`, greppable danger heuristics (pipe-to-shell, sudo-in-build, forced-remove) as warnings, trust signals + dep preview from `yay -Si` fields under `LC_ALL=C`, unprivileged `yay -S --noconfirm` streaming via `reader()` tee
- `archel pkg aur <name>`: name fast-fail, D-04 card pre-check with repo redirect to `pkg install`, full review display (card + trust + deps + warnings + ENTIRE PKGBUILD), TTY-gated `[o/N]` confirm, then the confirmed build with live output; abort at any gate builds nothing
- `archel completions <bash|zsh|fish>`: generated from the single clap definition via `clap_complete::aot::generate` to stdout (never deprecated root re-exports); all three shells non-empty with 206/233/128 archel references; full-tree word coverage pinned
- insta pins for the AUR review/result `--json` schemas; PLAT-02 close-out matrix green on 17 read/fast-fail paths
- 178 headless tests green (49 archel + 111 core + 18 helper), zero warnings, diacritics audit green

## Task Commits

Each task was committed atomically:

1. **Task 1: AUR fetch plus PKGBUILD validation core (TDD)** - `39cf5b8` (test, RED) + `acec024` (feat, GREEN)
2. **Task 2: AUR review-then-build CLI flow** - `3262165` (feat)
3. **Task 3: Completions for three shells plus harness close-out** - `a841219` (feat)

_TDD task 1 has two commits (RED matrix then GREEN implementation) per the RED-GREEN contract_

## Files Created/Modified

- `crates/archel-core/src/aur.rs` - AurReview/TrustSignals/AurDecision types, fetch/validate/parse/build fns, catalog label fns, 13 tests
- `crates/archel-core/src/error.rs` - `PkgbuildRejected{name, reason}` variant, message arm (reason stays log-only), coverage entry
- `crates/archel-core/locales/fr.json` - `aur.*` review keys, `errors.pkgbuild_rejected`; removed a pre-existing duplicate `invalid_package_name` key
- `crates/archel-core/src/lib.rs` - `pub mod aur` wiring
- `crates/archel/Cargo.toml` - clap_complete 4.6.11
- `crates/archel/src/cmds/mod.rs` - `PkgAction::Aur`, top-level `Completions{shell: Shell}`, completions coverage tests
- `crates/archel/src/cmds/pkg_cmds.rs` - `run_aur` + `run_aur_confirmed` seam + `render_aur_review`, 8 new tests, AUR schema pins
- `crates/archel/src/cmds/snapshots/archel__cmds__pkg_cmds__tests__aur_{review,result}.snap` - pinned AUR JSON schemas
- `Cargo.lock` - clap_complete dependency closure

## Decisions Made

- Post-confirm build flags are exactly `yay -S <name> --noconfirm` (A1/A2 resolution): `--noconfirm` answers only yay's own diff/edit/clean/proceed menus, every one superseded by Archel's D-05 review plus the Archel-side confirm; sudo password prompting is untouched because it comes from sudo(8) itself, which yay invokes natively per D-06 and never pre-answers. The finer `--answerdiff/--answeredit` combo is unnecessary (blanket post-review `--noconfirm` covers exactly the replaced menus); `--sudoloop` deliberately absent (no backgrounded sudo). Live-probed `yay -S --help` confirms the flag surface; end-to-end sudo interplay on a real build stays manual verify-work
- Heuristic danger patterns warn, never reject: only `bash -n` failure auto-rejects; the human validates everything else per D-05 (avoids over-blocking legit PKGBUILDs and false confidence alike)
- `bash -n` arg order is the trust boundary: `bash -n <file>` checks, `bash <file> -n` would EXECUTE — load-bearing comment at the spawn site (caught during implementation before any test ran)
- Abort seam takes an injectable build closure (`run_aur_confirmed`): the confirm-abort path is unit-proven headless with a spawn probe instead of depending on sandbox network (which flaked twice mid-phase: DNS ok then IPv6 unreachable)
- Repo packages through `pkg aur` redirect after the card pre-check, before any `yay -G` clone: no pointless network fetch on user error
- `Shell` implements `ValueEnum` + `FromStr` (A7 resolved by reading the vendored 4.6.11 source) — no wrapper enum needed; `#[arg(value_enum)] shell: Shell` directly
- Completions emit to stdout only; install paths (user-local vs system-wide) stay packaging-phase discretion per plan

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed `bash -n` argument order before any test ran**
- **Found during:** Task 1 GREEN implementation (code review of own spawn)
- **Issue:** `duct::cmd(&bash, [&probe]).arg("-n")` would have produced `bash <file> -n`, which EXECUTES the PKGBUILD instead of checking it — the exact violation D-05 exists to prevent. (duct 1.x `Expression` has no `.arg()` anyway — compile error surfaced it too)
- **Fix:** `duct::cmd(&bash, ["-n"])` plus `OsString` probe arg, with a load-bearing comment marking the order as the trust boundary
- **Files modified:** crates/archel-core/src/aur.rs
- **Verification:** `invalid_bash_maps_to_the_typed_rejection_with_french` passes; no PKGBUILD is ever executed
- **Committed in:** acec024 (part of task commit)

**2. [Rule 1 - Bug] Unique probe path per validation (parallel-test race)**
- **Found during:** Task 1 GREEN verification (2 test failures)
- **Issue:** All `validate_pkgbuild_source` calls shared one `/tmp/...-validate/PKGBUILD.probe` path; parallel tests raced write-vs-`bash -n`-read, so the danger-pattern test read the invalid-bash fixture and falsely rejected
- **Fix:** `probe_workdir()` with a process-wide atomic counter — unique path per call, still cleaned up after each validation
- **Files modified:** crates/archel-core/src/aur.rs
- **Verification:** full aur suite green under parallel execution
- **Committed in:** acec024 (part of task commit)

**3. [Rule 2 - Missing Critical] Removed duplicate `invalid_package_name` key in fr.json**
- **Found during:** Task 1 GREEN (catalog edit)
- **Issue:** Pre-existing duplicate key (serde_json silently takes the last); leaving it while adding sibling keys risks masking future catalog drift
- **Fix:** Removed the second occurrence, added `errors.pkgbuild_rejected`
- **Files modified:** crates/archel-core/locales/fr.json
- **Verification:** `every_variant_renders_non_empty_french` passes with 15 variants; diacritics audit green
- **Committed in:** acec024 (part of task commit)

**4. [Rule 1 - Bug] Repaired an edit slip that deleted `print_preview_list`'s signature**
- **Found during:** Task 2 implementation (immediate compile check)
- **Issue:** Inserting `run_aur` overwrote the `print_preview_list` fn signature line, orphaning its body
- **Fix:** Re-added the signature verbatim
- **Files modified:** crates/archel/src/cmds/pkg_cmds.rs
- **Verification:** compiles; install/remove/upgrade preview tests still pass
- **Committed in:** 3262165 (part of task commit)

**5. [Rule 3 - Blocking] Replaced the network-dependent abort test with an injectable seam**
- **Found during:** Task 2 verification (sandbox IPv6 route died mid-phase: `yay -Si` failed with network-unreachable)
- **Issue:** `aur_review_then_no_tty_builds_nothing` needed live AUR network, which proved flaky twice in one phase — the abort gate must be provable headless
- **Fix:** Split `run_aur_confirmed(name, json, build-closure)` out of `run_aur`; the test passes a spawn-probe closure and asserts `NeedsTty` + zero builds with no network. (A live run DID succeed earlier: `pkg aur hello` fetched and rendered the real PKGBUILD, stopping at the TTY gate — recorded as integration evidence in D1)
- **Files modified:** crates/archel/src/cmds/pkg_cmds.rs
- **Verification:** `aur_confirm_abort_spawns_no_build` green, deterministic
- **Committed in:** 3262165 (part of task commit)

---

**Total deviations:** 5 auto-fixed (3 bugs, 1 missing-critical, 1 blocking-issue fix + 1 editing slip recovered)
**Impact on plan:** All auto-fixes necessary for correctness/audit-compliance. No scope creep. No architectural changes.

## Issues Encountered

- Sandbox network flaked twice (DNS resolving then IPv6 unreachable), so no live AUR `-Si`/`-G` assertions survive in the suite: trust-signal and dep-preview parsing are proven by live-shape fixtures (field shapes captured during the working window), and the full live review was proven once manually (`yay -Si yay-bin` fields, `yay -G hello` fetch, `pkg aur hello` render-to-TTY-gate). Real review-then-abort on the desktop is the recorded verify-work item
- `yay -Si` AUR records DO carry `Repository: aur` (lowercase, verified live) — `is_aur_card` matches case-insensitively and display keeps yay's spelling verbatim rather than inventing normalization
- clap consumes everything after `--` as positionals, so `--json` must precede `--` (`pkg aur --json -- -foo`); verified correct placement on all four mutation fast-fail paths
- Bare `pkg upgrade --json` was deliberately NOT run live for close-out: a non-empty pending list would start a real pkexec upgrade. Coverage rests on clap's global-arg mechanics (identical parse path as all proven subcommands) plus the `upgrade <name> --json` refusal probe

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. `TrustSignals` fields are `Option` by design (missing signals render honest N/A, never zero-filled); empty `dependencies`/`warnings` render as label-alone or section-skipped, both honest.

## Threat Flags

None beyond the plan's threat model. Register coverage delivered:
- T-3-08 (PKGBUILD rendering): display-only + `bash -n` gate + explicit confirm; Archel never executes or sources the file (arg-order bug caught pre-test); build runs as user via yay, never root
- T-3-09 (--noconfirm before review): flag-ordering holds — `--noconfirm` appears only in `yay_build`, reachable solely post-review plus post-confirm; A1/A2 flag set recorded above
- T-3-10 (yay resolution): which-gated on the unprivileged side with typed `YayMissing` + French hint; `bash` falls back to the absolute `/usr/bin/bash`, never a PATH game in any privileged path (no privileged path exists here at all)
- T-3-11 (-Si output injection): all `-Si`/PKGBUILD text treated as untrusted display strings; `PkgbuildRejected.reason` (raw stderr) travels in logs only, never the terminal; version compare via structured fields

## Next Phase Readiness

- PKG-05 delivered with mandatory D-05 review intact; D-06 unprivileged yay holds; D-09 completions cover all subcommands; PLAT-02 harness complete (success criterion 4) — every Phase 2-3 capability verified drivable with `--json`
- Ready for Phase 4 (connectivity/devices): the confirm/stream/report/completions harness is the exact reuse shape for NM/Bluetooth mutations
- Carry-forwards for verify-work (desktop, human-gated): one real AUR review-then-abort (`pkg aur <tiny-aur-pkg>`, abort at the prompt, then confirm once and check `--noconfirm` + native sudo interplay); one real install/remove/upgrade from 03-02; completion script sourcing in bash/zsh/fish (`source <(archel completions bash)`)

---
*Phase: 03-packages-cli-harness*
*Completed: 2026-10-01*

## Self-Check: PASSED

- SUMMARY.md exists on disk; all 4 task commits verified in git log (39cf5b8, acec024, 3262165, a841219)
- Headless suite green at commit time (49 archel + 111 core + 18 helper, zero warnings); completions non-empty on all three shells (206/233/128 archel references); diacritics audit green
