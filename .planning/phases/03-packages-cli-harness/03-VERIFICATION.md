---
phase: 03-packages-cli-harness
verified: 2026-10-01T11:30:45Z
status: human_needed
score: 12/12 must-haves verified
covered_files:
  - .planning/phases/03-packages-cli-harness/03-01-PLAN.md
  - .planning/phases/03-packages-cli-harness/03-01-SUMMARY.md
  - .planning/phases/03-packages-cli-harness/03-02-PLAN.md
  - .planning/phases/03-packages-cli-harness/03-02-SUMMARY.md
  - .planning/phases/03-packages-cli-harness/03-03-PLAN.md
  - .planning/phases/03-packages-cli-harness/03-03-SUMMARY.md
  - .planning/phases/03-packages-cli-harness/deferred-items.md
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
covered_digest: "v2:sha256:dd74e4803595d50f6a4aa9c805d33188a1ef38a58741a4d77bcdc003647084d1"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 12/12
  gaps_closed: []
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Run one real `archel pkg install <tiny-pkg>` on the Hyprland session, confirm at the [o/N] prompt, watch live output"
    expected: "Deps preview renders, [o/N] defaults to No on empty, post-confirm pacman streams live via pkexec, exit 0, package installed"
    why_human: "Sandbox has no polkit agent or TTY; pkexec-mediated mutation cannot run headless. Unit proofs cover preview/validator/streaming only."
  - test: "Run one real `archel pkg remove <same-tiny-pkg>` on the Hyprland session, confirm, watch live output"
    expected: "Same confirm-then-stream flow via helper, plain `pacman -R`, package removed, app itself never ran as root"
    why_human: "Same polkit/TTY sandbox limitation as install"
  - test: "Run one real `archel pkg upgrade` on the Hyprland session (full -Syu), confirm, watch live output"
    expected: "Empty -Qu reports honestly up-to-date with no prompt; non-empty list streams the full -Syu via pkexec"
    why_human: "A real system upgrade is human-gated by definition; deliberately never run in automation"
  - test: "Run one real AUR review-then-abort plus one review-then-confirm on the Hyprland session (`archel pkg aur <tiny-aur-pkg>`: abort at prompt, then confirm once)"
    expected: "Full PKGBUILD + trust signals + deps display; abort builds nothing; confirm streams `yay -S --noconfirm` as the user with native sudo password handling"
    why_human: "Real AUR build mutates the machine through yay/sudo; unit + snapshot proofs cover fetch/validate/gates only"
  - test: "Source each completion script in its shell (`source <(archel completions bash)`, zsh, fish) and tab-complete `archel pkg <TAB>`"
    expected: "All subcommands complete in all three shells"
    why_human: "Non-empty output + full-tree word coverage are unit-pinned; actual shell evaluation needs a human shell"
---

# Phase 03: Packages & CLI Harness Verification Report

**Phase Goal:** Users can manage system and AUR packages end-to-end, and every proven API domain is drivable from the command line
**Verified:** 2026-10-01T11:30:45Z
**Status:** human_needed
**Re-verification:** Yes — digest refresh after roadmap/state bookkeeping commits (1bcc1a5, 579d4a0); zero source changes since a841219 (`git diff a841219 HEAD -- crates/ policy/` empty). All 12 truths re-checked against live code with a fresh headless suite run.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User runs `archel pkg list` and sees every installed package with version in a French table (SC1a, PKG-01) | ✓ VERIFIED | Re-run 2026-10-01: `pkg list --json` parses, 1147 rows; `packages.rs` reads alpm localdb (`Alpm::new`, root `/`, dbpath `/var/lib/pacman`), epoch-aware version-desc sort; French labels via `rust_i18n::t!("pkg.*")` catalog keys |
| 2 | User runs `archel pkg search <term>` and gets repo results first on ties, sorted version-descending (SC1b, PKG-01, D-07) | ✓ VERIFIED | Live `pkg search firefox --json` → 972 hits, epoch `2:` leads; every parse-bound spawn pins `LC_ALL=C`; pure parsers `parse_search_output` fixture-tested; blank query short-circuits honest-empty; yay absent → typed `YayMissing` French hint |
| 3 | User runs `archel pkg info <name>` and sees the D-04 pre-install card: sizes, source, candidate vs installed (PKG-01) | ✓ VERIFIED | Re-run 2026-10-01: `pkg info firefox --json` → keys `name/candidate_version/installed_version/source/download_size/installed_size`; unknown name → honest `PackageNotFound` French, never zero-filled card; `source: Option<String>` (None = installed list, honest null) |
| 4 | Every new pkg subcommand emits one parseable JSON object under `--json` (D-10) | ✓ VERIFIED | `pkg list/search/info --json` all `json.load` clean (re-run); insta pins `pkg_list/pkg_search/pkg_info` pass in the green suite (49 archel tests) |
| 5 | User runs `archel pkg install <name>`, sees deps, confirms [o/N], watches live pacman output stream (SC2, PKG-02, D-01/D-02) | ✓ VERIFIED | `run_install`: unprivileged `-S --print` preview → TTY-gated `confirm_action` (o/O/oui-prefix, default-No, non-TTY → `NeedsTty`) → `run_privileged_streaming` (duct `reader()` tee, transcript for --json) → helper `pkg-install` → `/usr/bin/pacman -S --noconfirm -- <pkgs>`; 18-test helper matrix + 8 streaming-client tests green (re-run). **Live pkexec run is human-gated (see Human Verification)** |
| 6 | User runs `archel pkg remove <name>` through the same confirm-then-stream flow via the pkexec helper, never as root (SC2, PKG-03) | ✓ VERIFIED | `run_remove` mirrors install; helper `pkg-remove` → plain `pacman -R --noconfirm` (A8: recursion opt-in, not `-Rns`); preview `-R --print` matches execution flags; `live_remove_preview_requires_installed_targets` green. **Live pkexec run is human-gated** |
| 7 | User runs `archel pkg upgrade` for full -Syu; any partial-upgrade attempt is refused in French (SC2, PKG-04, D-03) | ✓ VERIFIED | Re-run 2026-10-01: `grep -rn '"-Sy"' crates/` → zero matches (no bare `-Sy` constructor anywhere); helper `pkg-upgrade` exact-match → `-Syu --noconfirm` only; live `pkg upgrade firefox` → French D-03 refusal; `upgrade_with_a_package_name_is_the_refused_partial_path` + `upgrade_argv_is_the_full_sync_upgrade_only` green |
| 8 | A crafted package name (-foo, --overwrite, empty) is rejected and nothing executes (T-3-04) | ✓ VERIFIED | Live `pkg install -- -foo` → `InvalidPackageName` French exit 1, no pkexec prompt; helper matrix rejects `-foo/--overwrite/empty/smuggled-flag/zero-packages` with exit 2 + v1 envelope; client `validate_package_names` fast-fail mirrors helper before any spawn |
| 9 | User runs an AUR install and must see the full PKGBUILD plus validation before anything builds; abort builds nothing (SC3, PKG-05, D-05) | ✓ VERIFIED | `run_aur` order is card pre-check → repo redirect → `review_package` (yay -G tempdir, `bash -n <file>` with load-bearing arg-order comment, danger heuristics warn-only) → display → `confirm_action` → `yay_build`; `yay -S` reachable only past the gate; abort seam unit-proven with spawn probe (`aur_confirm_abort_spawns_no_build`, zero builds) |
| 10 | AUR operations run yay unprivileged with native sudo handling, never elevated by Archel (SC3, PKG-05, D-06) | ✓ VERIFIED | Zero `pkexec` occurrences in `aur.rs`; `yay_build` spawns `yay -S <name> --noconfirm` as the invoking user (A1/A2: `--noconfirm` answers only yay's own menus, sudo prompting untouched); which-gated with typed `YayMissing` French hint; live `pkg aur pacman` → repo redirect exit 0; `pkg aur --json -- -foo` → `InvalidPackageName` French, no spawn |
| 11 | User runs `archel completions <bash\|zsh\|fish>` and gets working scripts covering every subcommand (D-09) | ✓ VERIFIED | Re-run 2026-10-01: `completions bash \| grep -c archel` → 206, non-empty; `clap_complete 4.6.11` via `aot::generate` from the single clap definition (no deprecated root re-exports); `completions_cover_the_full_command_tree` pins per-shell word coverage. **Shell sourcing is human-gated** |
| 12 | Every Phase 2–3 capability accepts `--json`; harness close-out matrix green (SC4, PLAT-02) | ✓ VERIFIED | Spot-checked: `landing --json`, `sys cpu --json`, `pkg list/search/info --json` all parse; mutation fast-fails render typed French; 17-path matrix recorded in 03-03 SUMMARY D4; bare `pkg upgrade --json` deliberately never run live (would start a real pkexec upgrade on a non-empty list) — rests on clap global-arg mechanics + the `upgrade <name> --json` refusal probe. **Full-upgrade live JSON path rides with the human upgrade check** |

**Score:** 12/12 truths verified (5 carry a human-gated live-execution tail — code present, wired, unit-proven; the privileged/live half needs a desktop session)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/archel-core/src/packages.rs` | list/search/card composers, LC_ALL=C parsers, previews, confirm gate | ✓ VERIFIED | alpm localdb read; `run_capped_query` with `.unchecked()` + `LC_ALL=C`; `validate_package_names`, `confirm_action`, install/remove/upgrade previews |
| `crates/archel-core/src/aur.rs` | fetch/validate/review/trust-signals/unprivileged build | ✓ VERIFIED | `YAY_BUILD_FLAGS = ["--noconfirm"]`; `bash -n` before filename (trust boundary comment); no pkexec; `is_aur_card` case-insensitive |
| `crates/archel-core/src/privileged.rs` | streaming privileged client | ✓ VERIFIED | `pkg_install/pkg_remove/pkg_upgrade` reuse `locate_helper`/`map_pkexec_exit`/`parse_helper_stdout`; `run_privileged_streaming` via `reader()`+BufRead tee with `on_line` callback; 126/127 → distinct French |
| `crates/archel-privileged/src/main.rs` | 3 verb allowlist + validator matrix | ✓ VERIFIED | `pkg-upgrade` exact-match; install/remove charset+leading-dash+`--` separator; absolute `/usr/bin/pacman`; v1 envelope printed last; 18-test matrix green |
| `crates/archel/src/cmds/pkg_cmds.rs` | run_list/search/info/install/remove/upgrade/aur + renderers | ✓ VERIFIED | All 7 runners + `run_aur_confirmed` seam + `render_aur_review`; French via catalog fns; mutation `--json` → single report object with transcript array |
| `crates/archel/src/cmds/mod.rs` | Pkg group + Completions dispatch | ✓ VERIFIED | `PkgAction::{List,Search,Info,Install,Remove,Upgrade,Aur}` dispatched; `Command::Completions{shell: Shell}` via `aot::generate`; full-tree coverage test |
| `policy/org.archel.privileged.policy` | 3 least-privilege action blocks | ✓ VERIFIED | `pkg-install/pkg-remove/pkg-upgrade` ids present; `auth_admin_keep` triples, never `yes`; XML parses |
| insta snapshots (5: pkg_list/search/info, aur_review/result) | pinned --json schemas | ✓ VERIFIED | All 5 files exist; suite green means pins hold |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `pkg_cmds` runners | `archel-core` composers | direct fn calls, bin owns no French copy | WIRED | `run_list/search/info` render `PackageInfo`/`PackageCard` composed in core; labels via `t!` catalog keys; diacritics audit clean |
| helper verbs | `/usr/bin/pacman` | absolute path, server-side flags, `--` separator, no shell | WIRED | argv asserted exactly in tests (`-S/--noconfirm/--`, `-R/--noconfirm/--`, `-Syu/--noconfirm`); helper re-validates every invocation, never trusts CLI |
| streaming client | helper stdout | `reader()` tee → live print + transcript; last-line v1 envelope parse | WIRED | `parse_helper_stdout` unchanged (envelope is the last transcript line); `collect_streaming` unit-tested with vec collector |
| `run_aur` | `yay_build` | reachable only past review display + `confirm_action` | WIRED | Code order verified (review → print → `run_aur_confirmed` → confirm → build closure); `--noconfirm` appears only in `yay_build`; abort path spawns nothing (probe test) |
| `Completions` | clap definition | `clap_complete::aot::generate` to stdout | WIRED | Single definition source; all three shells non-empty with full-tree words |
| alpm localdb → list output | user display | typed error, never panic | WIRED | Open failure → `ArchelError::Internal` + catalog French; empty DB → honest empty table |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `pkg list` table/JSON | `PackageInfo{name, version}` | alpm localdb (`localdb.pkgs()`) | ✓ FLOWING | Live 1147 real rows (re-run) |
| `pkg search` table/JSON | merged `-Ss` hits | `pacman -Ss` + `yay -Ss` under `LC_ALL=C` | ✓ FLOWING | Live 972 hits for `firefox` |
| `pkg info` card | `PackageCard` fields | `pacman -Si` / `yay -Si` under `LC_ALL=C` | ✓ FLOWING | Live `firefox` card with sizes + versions (re-run) |
| `pkg aur` review | `AurReview{pkgbuild_text, trust, deps}` | `yay -G` tempdir + `yay -Si` fields | ✓ FLOWING | Live `hello` PKGBUILD rendered during 03-03 execution (fetch proven; sandbox DNS flaked later, not a code issue) |
| mutation transcript | `transcript: Vec<String>` | live helper/yay stdout lines | ✓ FLOWING (unit) | Collector test tees live lines; live transcript awaits human pkexec/yay run |

### Behavioral Spot-Checks (re-run 2026-10-01 for this re-verification)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| workspace suite green | `cargo test -p archel-core -p archel -p archel-privileged --no-default-features` | 49 + 111 + 18 passed, 0 failed | ✓ PASS |
| list --json parses | `pkg list --json \| python3 json.load` | parses, 1147 rows | ✓ PASS |
| info --json parses | `pkg info firefox --json \| json.load` | 6 keys incl. sizes + versions | ✓ PASS |
| partial-upgrade refusal | `pkg upgrade firefox` | French D-03 refusal | ✓ PASS |
| no bare -Sy | `grep -rn '"-Sy"' crates/` | zero matches (exit 1) | ✓ PASS |
| completions non-empty | `completions bash \| grep -c archel` | 206 | ✓ PASS |

### Probe Execution

No `scripts/*/tests/probe-*.sh` probes declared by any of the three plans; plan `<verify>` blocks are cargo-test / grep-gate / live-run commands, all executed above. Nothing to run here.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| PKG-01 | 03-01 | List and search installed packages (read-only) | ✓ SATISFIED | Live list + search + info card; D-07 ordering; D-08 installed-only list; D-04 card |
| PKG-02 | 03-02 | Install via pacman through helper | ✓ SATISFIED (live run human-gated) | Verbs + streaming + confirm; validator matrix; honest no-mutation paths live |
| PKG-03 | 03-02 | Remove via pacman through helper | ✓ SATISFIED (live run human-gated) | Same as PKG-02 with plain `-R` |
| PKG-04 | 03-02 | Full `-Syu`, never `-Sy` | ✓ SATISFIED (live upgrade human-gated) | Negative grep gate; D-03 refusal live; up-to-date path live exit 0 |
| PKG-05 | 03-03 | AUR via yay as user, with review step | ✓ SATISFIED (live build human-gated) | D-05 review intact and live-rendered; D-06 unprivileged proven by absence of elevation + which-gate |
| PLAT-02 | 03-01 + 03-03 | CLI exposes full API surface (harness) | ✓ SATISFIED (upgrade-live + sourcing human-gated) | All Phase 2–3 paths accept `--json`; 3-shell completions; 17-path matrix |

No orphaned requirements: ROADMAP maps exactly PKG-01..05 + PLAT-02 to Phase 3, and the three plans claim all six.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | `TBD/FIXME/XXX/placeholder/console.log-only` | — | None found across the five implementation files |
| — | — | Hardcoded French in `.rs` | — | None (French literals only in tests/comments; all render paths use `fr.json` catalog keys) |

### Human Verification Required

Five desktop-session checks, explicitly dispositioned (not dropped — each maps to a plan-flagged `human_judgment: true` carry-forward):

1. **Live install** — one real `pkg install`, confirm at `[o/N]`, watch the stream. Disposition: required before the install path is called proven; unit proofs (preview/validator/streaming) hold the fort until then.
2. **Live remove** — one real `pkg remove` through the helper. Disposition: same as install.
3. **Live upgrade** — one real full `-Syu` (includes the `--json` live path deliberately never run in automation). Disposition: required; automation must never run this.
4. **Live AUR review-then-abort + review-then-confirm** — abort builds nothing; confirm streams with native sudo. Disposition: required; sandbox DNS flaked during 03-03 so even the fetch half deserves a desktop re-proof.
5. **Completion sourcing** — `source <(archel completions …)` in bash/zsh/fish + tab-complete. Disposition: required; output shape is pinned, shell evaluation is not.

### Known Deferred Item (acknowledged, not dropped)

`updates.rs::run_checker` spawns without duct `.unchecked()`, so `checkupdates` exit 2 (up-to-date system) surfaces as `Internal` via `wait_timeout` instead of reaching `map_update_exit`'s explicit `("checkupdates", Some(2))` → `Ok` arm. Re-confirmed open in this re-verification: `grep -n unchecked crates/archel-core/src/updates.rs` returns nothing, while `packages.rs:177` carries `.unchecked()` plus the load-bearing comment. Effect: on a fully updated machine with `checkupdates` installed, the repo half degrades to stale/unavailable instead of `Fresh(0)`. Fix is one word but `updates.rs` is Phase-2-owned — still logged in `.planning/phases/03-packages-cli-harness/deferred-items.md` (status: open) and routed to a future plan touching that file. No Phase 3 path shares the bug. This item does not block the Phase 3 goal (LAND-03 counts still work; worst case is a stale-cache fallback on fully-clean machines).

### Gaps Summary

No code gaps. All 12 must-have truths hold in the codebase with live behavioral evidence for every headless-provable path (178 tests green: 49 archel + 111 core + 18 helper, re-run 2026-10-01; live list/info/refusal/completions re-exercised against the real system in this re-verification). The only unverifiable-in-sandbox remainder is live privileged execution (pkexec install/remove/upgrade, yay build with sudo, shell sourcing) — the plans themselves designed these as verify-work items, the summaries flagged them `human_judgment: true`, and they are dispositioned above as five explicit human checks. Status is therefore `human_needed`, not `passed`.

---

_Verified: 2026-10-01T11:30:45Z_
_Verifier: the agent (gsd-verifier)_
