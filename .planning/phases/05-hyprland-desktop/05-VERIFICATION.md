---
phase: 05-hyprland-desktop
verified: 2026-10-10T18:05:00Z
status: human_needed
score: 4/4 goal-backward checks satisfied (0 partial, 0 missing)
behavior_unverified: 0
overrides_applied: 0
covered_files:
  - crates/archel-core/src/hyprland/mod.rs
  - crates/archel-core/src/hyprland/appearance.rs
  - crates/archel-core/src/hyprland/monitors.rs
  - crates/archel-core/src/hyprland/binds.rs
  - crates/archel-core/src/hyprland/wallpaper.rs
  - crates/archel-core/src/hyprland/font.rs
  - crates/archel-core/src/hyprland/input.rs
  - crates/archel/src/cmds/appearance_cmds.rs
  - crates/archel/src/cmds/display_cmds.rs
  - crates/archel/src/cmds/input_cmds.rs
  - crates/archel-core/src/packages.rs
  - crates/archel-core/src/error.rs
  - crates/archel-core/src/lib.rs
  - crates/archel/src/cmds/mod.rs
  - crates/archel-core/locales/fr.json
covered_digest: "v3:sha256:ab700a91db8860522c070c7bb7fd7b8f8f6be5d3f75498cf840ba55adcabee17"
human_verification:
  - test: "After `archel appearance wallpaper set <valid-image>`, the desktop background visibly changes (hyprpaper conf-slice rewrite + pkill/respawn end to end). Revert afterwards."
    expected: "Wallpaper visibly changes on the live Hyprland desktop; `wallpaper status` reads back the managed path."
    why_human: "Needs a live Hyprland session with hyprpaper; skipped as UAT test 1 (user chose to advance 2026-10-08). Automated mechanics covered (UAT test 9)."
  - test: "After `archel display set --output ... --mode ...` and confirming KEEP within the 15 s window, the monitor is visibly at the new mode/scale/position; rejecting or timing out visibly restores the previous configuration."
    expected: "Live monitor visibly changes on keep; snapshot rollback visibly restores on reject/timeout."
    why_human: "Needs a real monitor under a live Hyprland session; skipped as UAT test 2. Orchestration covered by automated rollback/keep tests (UAT 12-14) plus the D-05 no-op live proof on HDMI-A-2."
  - test: "After `archel input keybind add` registers a new combo (e.g. SUPER+SHIFT+F13 -> dispatcher), physically pressing the combo performs the action on the live compositor. Remove the bind afterwards."
    expected: "The added keybind actually fires on the live compositor."
    why_human: "Needs physical keypresses in a live Hyprland session; skipped as UAT test 3. Executor F13 add/unbind round-trip recorded in 05-04-01-SUMMARY.md; logic covered by UAT 15-18."
  - test: "WR-06 follow-up: trigger the timed-confirm timeout on a TTY and confirm the spawned reader thread exits (receiver dropped so the late send fails)."
    expected: "No lingering reader thread after timeout; process exits cleanly."
    why_human: "Thread lifecycle after timeout is TTY-only observable; logic-level tests exist (confirm_action_timed, timed_confirm_refuses_without_a_tty) but thread exit itself needs a live TTY."
  - test: "WR-07 follow-up: run `archel input keybinds edit` changing the chord (mods/key) under a live Hyprland session and confirm the old chord stops firing while the new chord fires."
    expected: "Live `hl.unbindkey` of the old chord takes effect before the new bind is eval-applied; old combo dead, new combo live."
    why_human: "Live unbind ordering under the real compositor cannot be seen by grep or unit tests; decision + refusal-gate tests exist (rekey_decision_fires_only_on_mods_or_key_change)."
---

# Phase 5: Hyprland Desktop Verification Report

**Phase Goal:** Users can personalize and configure their Hyprland desktop — look, monitors, and inputs — safely
**Verified:** 2026-10-10T18:05:00Z (HEAD e8c9a7a)
**Status:** human_needed
**Re-verification:** No — initial verification
**Mode:** mvp-presentation, core+CLI phase (no QML/GUI files touched — no UI-SPEC gate applicable)

## Goal Achievement — Success Criteria (goal-backward)

| # | Criterion | Status | Evidence |
| --- | --- | --- | --- |
| 1 | Wallpaper, system font, blur, transparency with live-apply | ✓ SATISFIED | `archel appearance blur/opacity/font/wallpaper` (cmds/mod.rs `AppearanceAction`); core: `appearance.rs` (blur + active/inactive opacity via `hyprctl eval` + `getoption` read-back), `font.rs` (gsettings + snippet, D-02), `wallpaper.rs` (hyprpaper slice + pkill/respawn). UAT 4,5,8,9 automated pass. |
| 2 | Resolution, scaling, refresh, layout with confirm-and-rollback | ✓ SATISFIED | `archel display monitors` + `display set --output --mode(WxH@Hz) --scale --position --disable` (`DisplayAction`); `monitors.rs` snapshot → apply → 15 s keep-or-rollback (`confirm_action_timed`, packages.rs:532) → `run_rollback` collects leg failures and upgrades to `HyprlandUnavailable` naming the leg (WR-03 fix, commit 4cb3062). Validators reject typed-DesktopInvalid pre-spawn. Ran `orchestration_refusal_runs_rollback_with_aborted_exit` — ok. UAT 11–14 pass. |
| 3 | Keybinds view/add/edit/remove + mouse sensitivity/scrolling | ✓ SATISFIED | `InputAction::Keybinds {List,Add,Edit,Remove}` + `Mouse` (`input_cmds.rs`, 806 lines); `binds.rs` exact-(mods,key,dispatcher) match, `hl.bind`/`hl.unbind` eval path, WR-07 live-unbind-old-chord on rekey (`bind_identity_changed`, input_cmds.rs:180, commit fb0408d); `input.rs` SENSITIVITY/NATURAL_SCROLL/SCROLL_FACTOR keys with pre-spawn float validation. Ran `rekey_decision_fires_only_on_mods_or_key_change` — ok. UAT 15–18 pass. |
| 4 | hyprland.conf never rewritten; app-owned snippet + backup; all via CLI | ✓ SATISFIED | `mod.rs`: snippet at `modules/archel.lua`, `backup_before_write` (tmp+rename, rotation N=5, prune), one-time guarded `require("modules.archel")` via `ensure_require_line_at`; D-08 sentinel spy tests (mod.rs:516, wallpaper.rs:600) assert user tree + outside sentinel byte-for-byte unchanged. Require-line idempotency + atomic-refusal tests executed — ok. All 12 commands exposed via clap in `archel` binary. |

**Score:** 4/4 satisfied, 0 partial, 0 missing.

## Required Artifacts

All SUMMARY key-files exist and are substantive: hyprland module 7 files (mod 550, appearance 843, monitors 1362, binds 989, wallpaper 662, font 680, input 837 lines), 3 CLI command modules (appearance 653, display 441, input 806 lines), 14 insta snapshots. No stubs.

## Review-Fix Cross-Check (claimed vs actual — all 8 landed)

| Finding | Commit | Verified in code | Test executed |
| --- | --- | --- | --- |
| CR-01 exact-quoted-output upsert anchor | 58a0b2a | monitors.rs:1065 | `snippet_upsert_never_clobbers_prefix_colliding_outputs` — ok |
| WR-01/02 managed-block-scoped parsing, boundary-anchored keys | 3b48eaa | appearance.rs (span + left-boundary anchor, test at :612) | ok (filter run) |
| WR-03 rollback-leg failure collection | 4cb3062 | monitors.rs:602–633 `run_rollback` | rollback tests — ok (5 passed) |
| WR-04 unquote = exact inverse of lua_quote | 6ac1bbd | binds.rs:414, test :836 | `unquote_is_the_exact_inverse_of_lua_quote` — ok |
| WR-05 InvalidPackageName pre-spawn guards | bd7bdde | packages.rs:98,268,382,388 | flag-like query guards — ok |
| WR-06 drop timed-confirm receiver on timeout | bd7bdde | packages.rs:552–556 `drop(rx)` | logic tests present (TTY exit → human) |
| WR-07 live-unbind old chord on rekey | fb0408d | input_cmds.rs:178–210 | `rekey_decision_fires_only_on_mods_or_key_change` — ok |

Regression gate: `cargo test --offline -p archel-core/-p archel --no-default-features` green at e8c9a7a (per orchestrator); targeted named-test runs above all pass in this verification.

## Gate Artifacts (must_create)

| Gate | Status | Coherence |
| --- | --- | --- |
| 05-REVIEW.md | `status: issues_found`, standard depth, 1 critical + 7 warning | ✓ coherent with disposition |
| 05-REVIEW-FIX.md | `status: all_fixed`, iteration 1, 8/8 fixed, 0 skipped | ✓ matches 6 fix commits on disk |
| 05-REVIEW-DISPOSITION.md | 8 current fixed with commit hashes; 8 carried rows open/untriaged | ✓ coherent, honestly labeled |
| 05-SECURITY.md | `status: verified`, threats_open: 0 | ✓ coherent; created 2026-10-08 predates fix commits, but all 6 fixes are hardening (additive mitigations), no new attack surface → gate stands |
| 05-VALIDATION.md | exists, `status: draft`, nyquist_compliant: false | ⚠️ seeded-only; Nyquist validation never run despite 18 UAT coverage IDs — advisory, not goal-blocking |

## Requirements Coverage

All 12 requirements (APP-01..04, DSP-01..04, INP-01..04) claimed by the 4 plans' `requirements-completed`, all `[x]` in REQUIREMENTS.md, each mapped to implemented + tested code above. No orphaned requirements.

## Carried Disposition Rows Assessment (WR-08/09, IN-01..06 — informational only, non-blocking)

- **IN-02** (lua_quote test-only): resolved in production by binds.rs:370–598 + appearance.rs:84 callers.
- **IN-06** (substring parse): resolved by WR-01/WR-02 boundary-anchored parsing fix.
- **IN-01** (dead catalog keys `blur_title`/`opacity_ok`): still dead in fr.json — cosmetic.
- **IN-04** (`hypr_dir()` /tmp fallback on no-HOME): still present — degenerate-edge defense, info-level.
- **WR-08/09, IN-03/05**: stale-review quality/UX nits not re-reported by the current review against current source. None blocks any success criterion. Recommend triaging to `deferred`/`skipped` in the disposition ledger.

## Anti-Patterns / Debt Markers

No TBD/FIXME/XXX markers in any phase-modified file. No stub returns, no hardcoded-empty-data patterns found in the 15 covered files.

## Spot-Check Notes

- Live-Hyprland behaviors (wallpaper visible, monitor mode visibly changed, keybind physically fires) cannot be exercised in this environment — routed to human verification, matching the 3 previously-skipped UAT items.
- MVP_NOT_IMPLEMENTED check: not applicable — standard mvp-presentation core+CLI phase, no GUI files touched.

## Human Verification Required

5 items — see `human_verification` frontmatter: (1) wallpaper live-apply, (2) display keep-or-rollback live dialog against a real monitor, (3) added keybind physically fires, (4) WR-06 reader-thread exit after timeout on a TTY, (5) WR-07 live unbind ordering for `keybinds edit`.

## Gaps

None. All automated checks green; status is `human_needed` solely for the 5 live-session items above.

---

_Verified: 2026-10-10T18:05:00Z_
_Verifier: the agent (gsd-verifier)_
