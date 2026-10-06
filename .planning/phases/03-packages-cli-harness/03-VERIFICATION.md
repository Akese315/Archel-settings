---
phase: 03-packages-cli-harness
verified: 2026-10-06T13:09:16Z
status: passed
score: 15/15 must-haves verified
covered_files:
  - .planning/phases/03-packages-cli-harness/03-01-PLAN.md
  - .planning/phases/03-packages-cli-harness/03-01-SUMMARY.md
  - .planning/phases/03-packages-cli-harness/03-02-PLAN.md
  - .planning/phases/03-packages-cli-harness/03-02-SUMMARY.md
  - .planning/phases/03-packages-cli-harness/03-03-PLAN.md
  - .planning/phases/03-packages-cli-harness/03-03-SUMMARY.md
  - .planning/phases/03-packages-cli-harness/03-04-PLAN.md
  - .planning/phases/03-packages-cli-harness/03-04-SUMMARY.md
  - .planning/phases/03-packages-cli-harness/03-UAT.md
  - crates/archel-core/locales/fr.json
  - crates/archel-core/src/aur.rs
  - crates/archel-core/src/error.rs
  - crates/archel-core/src/lib.rs
  - crates/archel-core/src/packages.rs
  - crates/archel-core/src/privileged.rs
  - crates/archel-privileged/src/main.rs
  - crates/archel/src/cmds/mod.rs
  - crates/archel/src/cmds/pkg_cmds.rs
  - policy/org.archel.privileged.policy

covered_digest: "v2:sha256:f2a5d96d130af7628991019bec3871af1f024429576aaa3cc8306f6f12c2f8cf"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 12/12
  gaps_closed:
    - "G-03-1: `pkg install <AUR-only-name>` stops before confirm/helper with a French redirect to `pkg aur <name>` (03-04 guard, live-proven in UAT test 1)"
  gaps_remaining: []
  regressions: []
---

# Phase 03: Packages & CLI Harness Verification Report

**Phase Goal:** Users can manage system and AUR packages end-to-end, and every proven API domain is drivable from the command line
**Verified:** 2026-10-06T13:09:16Z
**Status:** passed
**Re-verification:** Yes — after gap-closure plan 03-04 (commits fa4fe80, edcb5d9, d5c9173) plus UAT completion (12/12 pass, G-03-1 resolved, commit a87316e)

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User runs `archel pkg list` and sees every installed package with version in a French table (SC1, PKG-01) | ✓ VERIFIED | `packages.rs` reads alpm localdb (`Alpm::new`, root `/`, dbpath `/var/lib/pacman`), epoch-aware version-desc sort; French labels via catalog keys; UAT test 6 pass |
| 2 | User runs `archel pkg search <term>` and gets repo results first on ties, sorted version-descending (SC1, PKG-01, D-07) | ✓ VERIFIED | Merged `pacman -Ss` + `yay -Ss` under `LC_ALL=C`; pure parsers fixture-tested; blank query honest-empty; yay absent → typed `YayMissing` French hint; UAT test 7 pass |
| 3 | User runs `archel pkg info <name>` and sees the D-04 pre-install card: sizes, source, candidate vs installed (PKG-01) | ✓ VERIFIED | `PackageCard` from `pacman -Si` / `yay -Si` under `LC_ALL=C`; unknown name → honest `PackageNotFound` French, never zero-filled; UAT test 8 pass |
| 4 | User runs `archel pkg install <name>`, sees deps, confirms [o/N], watches live pacman stream via helper, never as root (SC2, PKG-02) | ✓ VERIFIED | `run_install`: `-S --print` preview → TTY-gated `confirm_action` (default-No, non-TTY → `NeedsTty`) → `run_privileged_streaming` (duct `reader()` tee) → helper `pkg-install` → `/usr/bin/pacman -S --noconfirm -- <pkgs>`; 18-test helper matrix + streaming tests green; **live-proven in UAT test 1** (blender reinstall: card, [o/N], pkexec auth, live stream to `installe.`) |
| 5 | User runs `archel pkg remove <name>` through the same confirm-then-stream flow via the pkexec helper (SC2, PKG-03) | ✓ VERIFIED | `run_remove` mirrors install; helper `pkg-remove` → plain `pacman -R --noconfirm`; **live-proven in UAT test 2** (cowsay install then remove, live `pacman -R` to `supprime.`) |
| 6 | User runs `archel pkg upgrade` for full -Syu; any partial-upgrade attempt is refused in French (SC2, PKG-04, D-03) | ✓ VERIFIED | `grep -rn '"-Sy"' crates/` → zero matches (re-checked 2026-10-06); helper `pkg-upgrade` exact-match → `-Syu --noconfirm` only; **live-proven in UAT test 3** (up-to-date → honest no-prompt report; `upgrade firefox` → French D-03 refusal). Full -Syu stream on a non-up-to-date machine untested — dispositioned below, not blocking |
| 7 | A crafted package name (-foo, --overwrite, empty) is rejected and nothing executes | ✓ VERIFIED | Live `pkg install -- -foo` → `InvalidPackageName` French; helper matrix rejects all injection probes with exit 2 + v1 envelope; UAT test 10 pass |
| 8 | User runs an AUR install and must see the full PKGBUILD plus validation before anything builds; abort builds nothing (SC3, PKG-05, D-05) | ✓ VERIFIED | `run_aur`: card pre-check → repo redirect → `review_package` (yay -G tempdir, `bash -n`, danger heuristics warn-only) → display → `confirm_action` → `yay_build`; abort seam unit-proven; **live-proven in UAT test 4** (N → `Opération annulée`, no spawn; o → yay streamed until Ctrl+C, build dir cleaned) |
| 9 | AUR operations run yay unprivileged with native sudo handling, never elevated by Archel (SC3, PKG-05, D-06) | ✓ VERIFIED | Zero `pkexec` in `aur.rs`; `yay_build` spawns `yay -S <name> --noconfirm` as invoking user; which-gated with typed `YayMissing` hint; UAT test 4 pass |
| 10 | User runs `archel completions <bash\|zsh\|fish>` and gets working scripts covering every subcommand (D-09) | ✓ VERIFIED | `clap_complete 4.6.11` via `aot::generate` from single clap definition; all three shells non-empty; **sourcing live-proven in UAT test 5** (bash TAB lists all subcommands + flags) |
| 11 | Every Phase 2–3 capability accepts `--json`; harness close-out matrix green (SC4, PLAT-02) | ✓ VERIFIED | `landing/sys/pkg --json` all parse; mutation fast-fails render typed French; 17-path matrix in 03-03 SUMMARY; UAT tests 9 and 12 pass |
| 12 | `archel pkg install <AUR-only-name>` stops before any confirm prompt or helper spawn and prints a French redirect to `archel pkg aur <name>` (G-03-1, 03-04) | ✓ VERIFIED | `run_install` (pkg_cmds.rs:111-117): `install_preview` → `aur_names_in_cards` via `is_aur_card` → early `return Ok(())` with `print_install_aur_redirect`, placed BEFORE card render (line 122), `confirm_action` (129), and `stream_install`/helper spawn (132) in both human and `--json` branches; catalog keys `aur.is_aur_hint` / `aur.is_aur_hint_list` in fr.json with `is_aur_hint`/`is_aur_hint_list` accessors in aur.rs (439-446); 3 redirect-matrix tests green; **live-proven in UAT test 1 note** (`pkg install hello` → French `pkg aur` hint, exit 0, no prompt/auth; plus `pkg install aur hello` typo → clean French not-found error) |
| 13 | `archel pkg install <repo-name>` behaves exactly as before (card + [o/N] + helper stream) (03-04) | ✓ VERIFIED | Guard falls through only when `aur_names.is_empty()`; repo-only passthrough test `install_guard_passes_repo_only_lists_through` green; regression-proof: full workspace suite 52 + 111 + 18 green 2026-10-06, and UAT test 1's live `blender` reinstall ran WITH the guard in place |
| 14 | A mixed `pkg install <repo-pkg> <aur-pkg>` list installs nothing and redirects listing the AUR names (refuse-all) (03-04) | ✓ VERIFIED | `aur_names_in_cards` partitions cards; any non-empty set triggers the redirect return before any install; `install_guard_refuses_mixed_lists_listing_only_aur_names` green (lists `hello`, never `firefox`); refuse-all documented in `run_install` doc comment (101-106) |
| 15 | French copy for the redirect lives only in fr.json keys, diacritics audit stays green (03-04) | ✓ VERIFIED | No hardcoded French literals in the guard code (all rendering via `is_aur_hint`/`is_aur_hint_list` accessors); fr.json keys use `%{name}`/`%{packages}` interpolation; full workspace suite (includes diacritics audit) green |

**Score:** 15/15 truths verified (12 carried from prior verification + 3 new 03-04 guard truths; prior truth 5/6/7/8/10 live tails now closed by UAT 12/12)

### UAT Reconciliation

UAT file status: `complete`, 12/12 pass, 0 issues, 0 pending, 0 skipped. Gap G-03-1: `resolved` via 03-04-SUMMARY.md on 2026-10-01. Automated-coverage tests 6–12 map to SUMMARY coverage IDs D1–D4; live tests 1–5 were executed on the Arch desktop and each note records the observed behavior (redirect, reinstall stream, remove stream, AUR abort+confirm, bash completions). No contradiction between UAT claims and code: every live observation has a matching code path verified above.

### Residual Manual Note — Dispositioned (Not Blocking)

UAT test 3 notes the full `-Syu` stream was not exercised because the machine was fully up-to-date. Disposition: **accepted limitation, not a gap.** The `-Syu` stream uses the identical `run_privileged_streaming` + helper-envelope machinery live-proven twice (install in test 1, remove in test 2); the only delta is the server-side argv (`-Syu --noconfirm`), which is unit-pinned (`upgrade_argv_is_the_full_sync_upgrade_only`) and can only be produced by the exact-match `pkg-upgrade` verb. Forcing a real upgrade on demand would require a deliberately outdated machine; the next natural `-Syu` on the user's desktop exercises it. No code change required.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/archel-core/src/packages.rs` | list/search/card composers, LC_ALL=C parsers, previews, confirm gate | ✓ VERIFIED | alpm localdb read; `run_capped_query` with `.unchecked()` + `LC_ALL=C`; `validate_package_names`, `confirm_action`, install/remove/upgrade previews — unchanged since prior verification, suite green |
| `crates/archel-core/src/aur.rs` | fetch/validate/review/trust-signals/unprivileged build + redirect accessors | ✓ VERIFIED | `is_aur_hint`/`is_aur_hint_list` accessors (439-446) + catalog asserts; no pkexec; `is_aur_card` case-insensitive — reused unchanged by the guard |
| `crates/archel-core/src/privileged.rs` | streaming privileged client | ✓ VERIFIED | `pkg_install/pkg_remove/pkg_upgrade` via `reader()` tee; 126/127 → distinct French — unchanged, suite green |
| `crates/archel-privileged/src/main.rs` | 3 verb allowlist + validator matrix | ✓ VERIFIED | 18-test matrix green (re-run 2026-10-06) |
| `crates/archel/src/cmds/pkg_cmds.rs` | 7 runners + AUR guard + pure helper + redirect-matrix tests | ✓ VERIFIED | Guard (111-117), `aur_names_in_cards` (142-148), `print_install_aur_redirect` (154-171), 3 new tests (889-944) — all present, substantive, wired |
| `crates/archel/src/cmds/mod.rs` | Pkg group + Completions dispatch | ✓ VERIFIED | Unchanged, suite green |
| `crates/archel-core/locales/fr.json` | catalog incl. `aur.is_aur_hint` + `aur.is_aur_hint_list` | ✓ VERIFIED | Keys present (lines 85-86), interpolation `%{name}`/`%{packages}` |
| `policy/org.archel.privileged.policy` | 3 least-privilege action blocks | ✓ VERIFIED | Unchanged since prior verification |
| insta snapshots (5) | pinned --json schemas | ✓ VERIFIED | Suite green means pins hold; no new snapshots per plan (redirect note is a hint string, not a schema) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `run_install` | `is_aur_card` / redirect | `aur_names_in_cards(&preview.cards)` checked before render/confirm/helper (pkg_cmds.rs:113-117) | WIRED | Early `return Ok(())`; original `packages` slice untouched on fall-through (T-3-12); same predicate as `run_aur` redirect (T-3-13, line 285) |
| `print_install_aur_redirect` | fr.json catalog | `is_aur_hint` / `is_aur_hint_list` accessors | WIRED | Singular vs list branch on `aur_names.len()`; JSON arm emits `{cards + note}` mirroring `run_aur` repo-redirect shape |
| `pkg_cmds` runners | `archel-core` composers | direct fn calls, bin owns no French copy | WIRED | Re-confirmed; diacritics audit green |
| helper verbs | `/usr/bin/pacman` | absolute path, server-side flags, `--` separator, no shell | WIRED | Re-confirmed; 18-test matrix green |
| streaming client | helper stdout | `reader()` tee → live print + transcript | WIRED | Re-confirmed |
| `run_aur` | `yay_build` | reachable only past review + `confirm_action` | WIRED | Re-confirmed |
| `Completions` | clap definition | `clap_complete::aot::generate` to stdout | WIRED | Re-confirmed |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `pkg install` redirect note | `aur_names` | `preview.cards[].source` via `is_aur_card` (live `pacman -Si`/`yay -Si` provenance) | ✓ FLOWING | Live `hello` → AUR card → French hint observed in UAT; no static fallback |
| `pkg install` redirect JSON | `{cards, note}` | preview cards + catalog hint | ✓ FLOWING | Real cards echoed, not stubbed |
| `pkg list/search/info` | rows / hits / card | alpm localdb / `-Ss` / `-Si` | ✓ FLOWING | Re-confirmed from prior verification (no source changes) |

### Behavioral Spot-Checks (re-run 2026-10-06 for this re-verification)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| workspace suite green | `cargo test --workspace` | 52 + 111 + 18 passed, 0 failed | ✓ PASS |
| guard matrix green | `cargo test -p archel pkg_cmds::tests::install_guard` | 3 passed | ✓ PASS |
| aur module green | `cargo test -p archel-core aur` | 15 passed | ✓ PASS |
| no bare -Sy | `grep -rn '"-Sy"' crates/` (excluding refusal/-Syu contexts) | zero matches | ✓ PASS |
| redirect keys present | `grep -n is_aur_hint fr.json aur.rs` | 2 keys + 2 accessors + asserts | ✓ PASS |

### Probe Execution

No `scripts/*/tests/probe-*.sh` probes declared by any of the four plans; plan `<verify>` blocks are cargo-test / grep-gate / live-run commands, all executed above. Nothing to run here.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| PKG-01 | 03-01 | List and search installed packages (read-only) | ✓ SATISFIED | Unchanged; UAT tests 6–8 pass |
| PKG-02 | 03-02 + 03-04 | Install via pacman through helper + AUR-install redirect guard | ✓ SATISFIED | Guard closes the routing hole; live redirect + live reinstall both proven (UAT test 1) |
| PKG-03 | 03-02 | Remove via pacman through helper | ✓ SATISFIED | Live-proven (UAT test 2) |
| PKG-04 | 03-02 | Full `-Syu`, never `-Sy` | ✓ SATISFIED | Negative gate + live refusal/up-to-date paths (UAT test 3); residual full-stream note dispositioned above |
| PKG-05 | 03-03 | AUR via yay as user, with review step | ✓ SATISFIED | Live-proven abort + confirm (UAT test 4) |
| PLAT-02 | 03-01 + 03-03 | CLI exposes full API surface (harness) | ✓ SATISFIED | UAT tests 5, 9, 11, 12 pass |

No orphaned requirements: ROADMAP maps exactly PKG-01..05 + PLAT-02 to Phase 3, and the four plans claim all six.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | `TBD/FIXME/XXX/TODO/HACK/PLACEHOLDER` in `pkg_cmds.rs`, `aur.rs` | — | None found (grep clean 2026-10-06) |
| — | — | Hardcoded French in `.rs` | — | None (render paths use catalog keys; French literals only in tests/comments) |
| — | — | Empty handlers / null returns / console.log-only | — | None |

### Gaps Summary

No gaps. Gap G-03-1 (the single gap from UAT) is closed: the redirect guard is present, substantive, wired before confirm/helper in both human and `--json` branches, unit-covered by the headless redirect matrix, and live-proven on the desktop (`pkg install hello` → French `pkg aur` hint, exit 0, no prompt/auth). UAT stands at 12/12 pass with 0 issues. The one residual note (full `-Syu` stream on a non-up-to-date machine) is dispositioned above as an accepted limitation sharing the twice-live-proven streaming path — it does not block the phase goal.

### Known Deferred Item (acknowledged, not dropped, carried from prior verification)

`updates.rs::run_checker` spawns without duct `.unchecked()`, so `checkupdates` exit 2 (up-to-date system) surfaces as `Internal` instead of reaching `map_update_exit`'s `Ok` arm. `updates.rs` is Phase-2-owned; logged in `.planning/phases/03-packages-cli-harness/deferred-items.md` (status: open). No Phase 3 path shares the bug. Does not block the Phase 3 goal.

---

_Verified: 2026-10-06T13:09:16Z_
_Verifier: the agent (gsd-verifier)_
