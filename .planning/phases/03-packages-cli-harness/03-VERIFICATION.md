---
phase: 03-packages-cli-harness
verified: 2026-10-10T18:52:38Z
status: human_needed
score: 10/10 goal-backward checks satisfied
behavior_unverified: 0
overrides_applied: 0
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
  - Cargo.lock
  - crates/archel-core/Cargo.toml
  - crates/archel-core/locales/fr.json
  - crates/archel-core/src/aur.rs
  - crates/archel-core/src/error.rs
  - crates/archel-core/src/lib.rs
  - crates/archel-core/src/packages.rs
  - crates/archel-core/src/privileged.rs
  - crates/archel-privileged/src/main.rs
  - crates/archel/Cargo.toml
  - crates/archel/src/cmds/mod.rs
  - crates/archel/src/cmds/pkg_cmds.rs
  - policy/org.archel.privileged.policy
covered_digest: "v2:sha256:f7b9bf63df2f2bf3f696e2be836163b80990a9ea7cb64af1b130bfca45a6ab61"
re_verification:
  previous_status: passed
  previous_score: 15/15
  previous_verified: 2026-10-06T13:09:16Z
  reason: "Covered source files legitimately changed during later phases (05/06 work and code-review fixes touched shared core files, including packages.rs — the phase-05 pacman `--` terminator and InvalidPackageName validator landed in this territory). Regenerating the report against the current codebase; plans were NOT re-executed."
  gaps_closed: []
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Live pkexec elevation for install/remove/upgrade: run `archel pkg install <small-repo-pkg>` then `archel pkg remove <same-pkg>` and `archel pkg upgrade` on a desktop session, accepting the polkit dialog."
    expected: "Polkit dialog appears, pacman output streams live into the terminal, command succeeds with the French OK line; declining the dialog yields the typed French AuthFailed (pkexec 126/127 mapping) and nothing changes on disk. The app process never runs as root (verify with `ps -o user= -p <pid>`)."
    why_human: "Requires an interactive polkit agent plus a TTY confirm prompt; headless CI cannot answer the pkexec dialog, so the elevation round-trip cannot be exercised here."
  - test: "Live AUR review-before-build: `archel pkg aur <aur-only-pkg>` on a desktop session, abort at the review prompt, then re-run and allow the build."
    expected: "Full PKGBUILD text plus `bash -n` validation and danger-heuristic warnings render BEFORE any build; aborting prints the French cancellation and spawns nothing (no build directory left behind); confirming streams `yay -S <name> --noconfirm` as the invoking user, with yay performing its own sudo handling."
    why_human: "The build path needs a real yay invocation, network access to the AUR, and an interactive TTY confirm; none available headlessly."
  - test: "Full `-Syu` upgrade stream on a genuinely out-of-date machine."
    expected: "`archel pkg upgrade` streams the complete pacman `-Syu` transaction through the helper and finishes with the French success report."
    why_human: "The dev machine is fully up-to-date and `--offline` is required for this verification run, so the sync-upgrade stream cannot be produced. Its argv (`-Syu --noconfirm`) is unit-pinned and the identical streaming path is live-proven twice for install and remove."
---

# Phase 03: Packages & CLI Harness Verification Report

**Phase Goal:** Users can manage system and AUR packages end-to-end, and every proven API domain is drivable from the command line
**Verified:** 2026-10-10T18:52:38Z
**Status:** human_needed
**Re-verification:** Yes — initial claims re-checked against the current codebase after phases 05/06 and code-review fixes touched shared files

## Command Results

| Command | Result |
| --- | --- |
| `cargo test --offline -p archel-core --no-default-features` | ok — 277 passed; 0 failed; 1 ignored |
| `cargo test --offline -p archel --no-default-features` | ok — 125 passed; 0 failed; 0 ignored |
| `cargo build --offline -p archel --no-default-features` | ok — `target/debug/archel` runnable |
| `archel completions bash \| wc -c` | 162084 (non-empty) |
| `archel completions zsh \| wc -c` | 130886 (non-empty) |
| `archel completions fish \| wc -c` | 55601 (non-empty) |
| `archel pkg list --json` | Parseable JSON array of installed packages with epoch-bearing versions (`blender 17:5.2.2-2`, …) |
| `archel pkg install -- -foo` | `Nom de paquet invalide : « -foo » — rien n'a été exécuté.` — rejected before any spawn |
| `archel pkg upgrade firefox` | French D-03 partial-upgrade refusal naming `archel pkg upgrade` |
| `archel pkg info nonexistent-pkg-xyz` | `Paquet « … » introuvable dans les dépôts ni dans AUR.` — honest not-found, never zero-filled |
| `archel pkg search <flag-like query>` | `InvalidPackageName` via unit assertions (`packages.rs:722-740`) |
| `archel landing --json` / `sys ram --json` | Parseable JSON objects (Phase 2 harness close-out) |

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User can list and search installed packages from the terminal (SC1, PKG-01) | ✓ VERIFIED | `packages.rs` reads the alpm localdb and renders a French table; live `pkg list --json` returned real installed packages with epoch versions. `search_installed` merges `pacman -Ss --` and `yay -Ss --` under `LC_ALL=C` (`packages.rs:103,189`) with repo-first, version-descending merge and blank-query honest-empty. Unit matrix `packages.rs:722-740` rejects flag-like queries/names as `InvalidPackageName`. |
| 2 | User can install, remove and fully upgrade (`-Syu`) through the pkexec helper, never as root (SC2, PKG-02/03/04) | ✓ VERIFIED (code) | CLI side is fully proven headlessly: `run_install`/`run_remove` gate on `confirm_action` then `stream_install` → `run_privileged_streaming` (`privileged.rs:234` `reader()` tee, no capture deadlock). Helper `crates/archel-privileged/src/main.rs` exposes exactly `pkg-install` / `pkg-remove` / `pkg-upgrade`; flags are hardcoded server-side and user names are appended after a `--` terminator (`main.rs:95`, asserted `main.rs:311,326`). `pkg-upgrade` is exact-match argv `["-Syu","--noconfirm"]` (`main.rs:73-76`, `:338`). `grep -rn '"-Sy"' crates/` matches only the two injection-probe test lists (`packages.rs:722,735`) — no bare-sync path exists. pkexec 126/127 map to typed French cancel/deny (`privileged.rs:112-113`). The 18-test helper matrix rejects every injection probe with exit 2. Live polkit round-trip: human item 1. |
| 3 | User can manage AUR packages via yay unprivileged, with a PKGBUILD review step before building (SC3, PKG-05) | ✓ VERIFIED (code) | `aur.rs` contains no `pkexec` (the single match is the D-06 doc comment at `:28-30`); `yay_build` spawns `yay -S <name> --noconfirm` as the invoking user (`aur.rs:355`, `:408`). `review_package` (`:312`) fetches the PKGBUILD into a tempdir via `yay -G`, validates with `bash -n` — never executes or sources it — and runs warn-only danger heuristics; `--noconfirm` is applied only post-review (`YAY_BUILD_FLAGS` `:343`), and the confirm at `pkg_cmds.rs:331` gates the spawn. Live build/abort path: human item 2. |
| 4 | A crafted package name is rejected by the helper with exit 2 and nothing executes (SC2 hardening) | ✓ VERIFIED | Double gate: the CLI validator rejects first (live `pkg install -- -foo` → French `InvalidPackageName`, exit 0, "rien n'a été exécuté"), and the helper independently re-validates (`main.rs:79-100`) and rejects with exit 2. Phase-05 code-review fix `bd7bdde` (WR-05/WR-06) extended the same rejection to flag-injection search queries. |
| 5 | A crafted query/name is rejected at the core boundary before any subprocess | ✓ VERIFIED | `InvalidPackageName` raised at `packages.rs:98` (search), `:268` (info), `:382,388` (install/remove); `pacman -Ss/-Si` invoked with a `--` terminator (`packages.rs:103,273`); 12 assertions in the `packages.rs:722-740` probe matrix. |
| 6 | User can drive every Phase 2–3 capability through `archel-cli` with `--json` output (SC4, PLAT-02) | ✓ VERIFIED | Live `pkg list/info/search --json`, `landing --json`, `sys ram --json` all returned parseable JSON in this run; 125 green `archel` tests include pinned schema snapshots per command. |
| 7 | User gets working bash/zsh/fish completions covering every subcommand (D-09) | ✓ VERIFIED | `clap_complete 4.6.x` `aot::generate` from the single clap definition (`cmds/mod.rs:9,108-111`), never hand-written. Live byte counts 162084 / 130886 / 55601; tests `completions_are_non_empty_and_name_archel_per_shell` and `completions_cover_the_full_command_tree` pass. |
| 8 | AUR-only name in `pkg install` stops before any confirm prompt or helper spawn and prints a French redirect to `pkg aur` (G-03-1, 03-04) | ✓ VERIFIED | `run_install` (`pkg_cmds.rs:115-121`): `install_preview` → `aur_names_in_cards` → `print_install_aur_redirect` → `return Ok(())`, positioned BEFORE `stream_install` (`:123`) and `confirm_action` (`:133`) in both the `--json` and human branches. Tests `aur_repo_package_redirects_without_fetching_or_building` and the 03-04 redirect matrix pass. |
| 9 | Repo-only installs behave exactly as before the 03-04 guard (03-04) | ✓ VERIFIED | The guard returns only when `aur_names` is non-empty; repo-only passthrough test green and the original `packages` slice reaches the install path unchanged (T-3-12). |
| 10 | Mixed repo+AUR lists install nothing and redirect listing the AUR names (refuse-all) (03-04) | ✓ VERIFIED | `aur_names_in_cards` partitions the preview cards and any non-empty AUR set triggers the early return; `install_guard_refuses_mixed_lists_listing_only_aur_names` passes. |

**Score:** 10/10 goal-backward checks verified. Truth 2 and 3 are code-verified headlessly with their live polkit / yay-build tails routed to human verification (the only remaining way to prove them in a headless environment), not guessed.

### UAT Reconciliation

`03-UAT.md` status: `complete` — 12/12 pass, 0 issues, 0 pending, 0 skipped. Gap G-03-1 resolved by plan 03-04 (UAT test 1 note: `pkg install hello` → French `pkg aur` redirect, exit 0, no prompt or auth). Live tests 1–5 recorded observed desktop behaviour (blender reinstall stream, cowsay remove stream, `-Syu` honest up-to-date report, AUR abort/confirm, bash TAB completions). No UAT claim contradicts the current code: every observed behaviour above has a matching, still-present code path verified in this pass.

### Prior-Report Notes

The previous report (2026-10-06, 15/15) enumerated 15 rows, of which four (helper validation, CLI crafted-name rejection, query/name `InvalidPackageName`, mixed-list refuse-all) were restatements of the same input-validation boundary and two restated the `pkg install` redirect flow. This pass collapses them into the 10 distinct goal-backward checks above; no check was dropped. It also recorded one residual non-blocking note: the full `-Syu` stream was never exercised because the machine was up-to-date. That disposition still holds — the argv is unit-pinned and the streaming machinery is live-proven twice via install and remove — so it is carried here as human item 3 rather than a gap.

### Required Artifacts

| Artifact | Expected | Status | Details |
| --- | --- | --- | --- |
| `crates/archel-core/src/packages.rs` | alpm localdb list, merged search, pre-install card, validators | ✓ VERIFIED | Substantive; live `--json` output; LC_ALL=C + `--` pinned |
| `crates/archel-core/src/aur.rs` | yay queries, PKGBUILD review, unprivileged build, redirect copy accessors | ✓ VERIFIED | `bash -n` only; zero pkexec; `is_aur_hint`/`is_aur_hint_list` at `:486-493` |
| `crates/archel-core/src/privileged.rs` | streaming client, pkexec exit-code mapping | ✓ VERIFIED | `reader()` tee at `:234`; 126/127 at `:112-113` |
| `crates/archel-core/src/error.rs` | typed `ArchelError` incl. `InvalidPackageName`, `AuthFailed`, `YayMissing` | ✓ VERIFIED | Matched by helper and CLI tests |
| `crates/archel-core/locales/fr.json` | All French copy incl. redirect keys | ✓ VERIFIED | `aur.is_aur_hint` / `is_aur_hint_list` present with `%{name}`/`%{packages}` |
| `crates/archel-privileged/src/main.rs` | Three allowlisted verbs, server-side flags | ✓ VERIFIED | 18-test matrix; `--` terminator `:95` |
| `policy/org.archel.privileged.policy` | polkit policy for the helper | ✓ VERIFIED | Present, 2749 bytes |
| `crates/archel/src/cmds/pkg_cmds.rs` | install/remove/upgrade/aur commands, AUR guard | ✓ VERIFIED | Guard ordering confirmed at `:115-137` |
| `crates/archel/src/cmds/mod.rs` | clap surface + completions generation | ✓ VERIFIED | `aot::generate`, live non-empty per shell |

### Key Link Verification

| From | To | Via | Status | Details |
| --- | --- | --- | --- | --- |
| `pkg_cmds.rs` | `archel_core::packages` | `install_preview`, `confirm_action`, `stream_install` — bin owns no French copy or parsing | ✓ WIRED | Confirmed `:115-137` |
| `pkg_cmds.rs` | `archel-privileged` helper | `run_privileged_streaming` → pkexec → pacman with hardcoded flags + `--` | ✓ WIRED | Streaming tee at `privileged.rs:234` |
| `aur.rs` | yay subprocess | `yay -G` / `yay -Si` / `yay -S --noconfirm` as invoking user | ✓ WIRED | `aur.rs:148,355,408`; no elevation path |
| `pkg_cmds.rs` redirect | `fr.json` keys | `is_aur_hint` / `is_aur_hint_list` accessors | ✓ WIRED | `aur.rs:486-493` |
| alpm localdb | `pkg list` output | typed error on unavailable db, never panic | ✓ WIRED | 277-test core suite green incl. live localdb tests |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| --- | --- | --- | --- | --- |
| `pkg list --json` | package array | alpm localdb read | Yes — live run returned real epoch-bearing versions | ✓ FLOWING |
| `pkg search --json` | merged hits | `pacman -Ss --` + `yay -Ss --` under `LC_ALL=C` | Yes (live network/AUR tail is environment-bound) | ✓ FLOWING |
| `pkg info --json` | `PackageCard` | `pacman -Si --` / `yay -Si` | Yes — honest not-found instead of zero-fill | ✓ FLOWING |
| `landing --json` / `sys ram --json` | snapshot fields | Phase 2 update cache + sysinfo | Yes — `repo.fresh`/`aur.fresh`, byte counts | ✓ FLOWING |
| completions | script text | `clap_complete::aot::generate` over the live clap tree | Yes | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| --- | --- | --- | --- |
| List installed packages | `archel pkg list --json` | Non-empty JSON array, real packages | ✓ PASS |
| Crafted-name rejection | `archel pkg install -- -foo` | French `InvalidPackageName`, no execution | ✓ PASS |
| Partial-upgrade refusal | `archel pkg upgrade firefox` | French D-03 refusal | ✓ PASS |
| Honest not-found card | `archel pkg info nonexistent-pkg-xyz` | French "introuvable dans les dépôts ni dans AUR" | ✓ PASS |
| Completions per shell | `archel completions bash/zsh/fish` | 162084 / 130886 / 55601 bytes | ✓ PASS |
| Phase 2 `--json` close-out | `landing --json`, `sys ram --json` | Parseable JSON | ✓ PASS |
| Bare `-Sy` prohibition | `grep -rn '"-Sy"' crates/` | Only test injection-probe lists | ✓ PASS |

### Probe Execution

No `scripts/*/tests/probe-*.sh` probes exist for this phase and none are declared in its PLAN or SUMMARY artifacts. Step 7c: SKIPPED (no probes declared or conventional).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| --- | --- | --- | --- | --- |
| PKG-01 | 03-01 | List/search/info read side | ✓ SATISFIED | Truths 1, 5 |
| PKG-02 | 03-02, 03-04 | Install via pkexec helper | ✓ SATISFIED | Truths 2, 4, 8-10 |
| PKG-03 | 03-02 | Remove via pkexec helper | ✓ SATISFIED | Truth 2 |
| PKG-04 | 03-02 | Full `-Syu` upgrade only | ✓ SATISFIED | Truth 2 (`-Syu` argv pinned, no bare sync) |
| PKG-05 | 03-03 | AUR with review-before-build | ✓ SATISFIED | Truth 3 |
| PLAT-02 | 03-01, 03-03 | CLI harness: `--json` + completions | ✓ SATISFIED | Truths 6, 7 |

No orphaned requirements: ROADMAP Phase 3 maps exactly PKG-01..05 + PLAT-02, all claimed by plans.

### Prohibitions (must-NOT checks)

| Prohibition | Verification | Status | Evidence |
| --- | --- | --- | --- |
| Never emit bare `-Sy` / any sync-without-upgrade path | test | ✓ HELD | Grep returns only test probes; helper's sole refresh verb is exact-match `-Syu --noconfirm` |
| Never parse pacman/yay output without `LC_ALL=C` | judgment | ✓ HELD | `.env("LC_ALL","C")` at `packages.rs:189` + `aur.rs` yay queries |
| Never render mutation French outside `fr.json` | judgment | ✓ HELD | `i18n::tests::no_hardcoded_french_diacritics_in_rust_sources` green; bin renders via catalog accessors |
| Never forward user-influenced flags (`--overwrite`/`--needed`/`-y`) to the helper | test | ✓ HELD | Helper flags hardcoded server-side, names after `--`; probe matrix exits 2 |
| Never run the app itself as root | judgment | ✓ HELD | Mutations only via pkexec helper; zero elevation in `aur.rs` (D-06 comment at `:28-30`) |
| Never execute or source a PKGBUILD | judgment | ✓ HELD | `bash -n` only, `-n` first-arg note at `aur.rs:219`; display path |
| Never pass `--noconfirm` to yay before review + Archel confirm | judgment | ✓ HELD | `YAY_BUILD_FLAGS` applied only in `yay_build`, after `review_package` and `confirm_action` (`pkg_cmds.rs:331`) |

### Anti-Patterns Found

None. No `TBD` / `FIXME` / `XXX` markers in any phase-03 implementation file; the single `placeholder` hit (`pkg_cmds.rs:500`) is a doc comment describing the *absence* of placeholder rendering. No stub returns or hardcoded-empty data flows in the package path.

### Human Verification Required

Three items, all requiring a live desktop session or network — see `human_verification` frontmatter. Automated checks passed for everything reachable headlessly.

1. **Live pkexec elevation** (install/remove/upgrade, dialog accept + decline, non-root check) — code path fully verified headlessly; the interactive polkit round-trip cannot run here.
2. **Live AUR review-before-build** (PKGBUILD render, `bash -n` result, abort builds nothing, confirm streams yay unprivileged) — gating logic verified; the real yay build needs network + TTY.
3. **Full `-Syu` stream on an out-of-date machine** — carried from the prior report's non-blocking note; argv pinned, streaming machinery live-proven twice.

### Gaps Summary

No gaps. No BLOCKERs, no stubs, no missing artifacts, no broken key links, no unresolved debt markers, no failed prohibition. Every goal-backward check is satisfied by automated evidence: 402 green tests across the two crates (277 core + 125 bin), live `--json` output for every Phase 2–3 read surface, live rejections for crafted names and partial upgrades, and non-empty completions for all three shells. The three outstanding items are strictly live-session checks and cannot be exercised in this headless, `--offline` environment, so the status is `human_needed` rather than `passed`.

---

_Verified: 2026-10-10T18:52:38Z_
_Verifier: the agent (gsd-verifier) — re-verification after later-phase drift in shared core files_
