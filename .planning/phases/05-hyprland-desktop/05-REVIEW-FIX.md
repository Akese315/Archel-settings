---
phase: 05
fixed_at: 2026-10-10T00:00:00Z
review_path: .planning/phases/05-hyprland-desktop/05-REVIEW.md
iteration: 1
findings_in_scope: 8
fixed: 8
skipped: 0
status: all_fixed
---

# Phase 05: Code Review Fix Report

**Fixed at:** 2026-10-10
**Source review:** `.planning/phases/05-hyprland-desktop/05-REVIEW.md` (status: issues_found, iteration 1)
**Iteration:** 1

**Summary:**
- Findings in scope: 8 (1 critical + 7 warning; no Info findings exist)
- Fixed: 8
- Skipped: 0

All work happened on an isolated worktree branch (`gsd-reviewfix/05-27215` from `master` @ `cea63df`).
**Verification ran in the isolated worktree** (`/home/axel/Programmation/settings-app/.claude/worktrees/rf-05-27215-1791641663`), so the test totals below are reproducible from that tree only until the branch is merged.

Final full-suite result: `cargo test --offline -p archel-core -p archel` →
**archel-core: 277 passed, 0 failed (1 ignored) · archel: 243 passed, 0 failed.**

Commits (oldest first): `58a0b2a` CR-01 · `3b48eaa` WR-01+WR-02 · `4cb3062` WR-03 · `6ac1bbd` WR-04 · `bd7bdde` WR-05+WR-06 · `fb0408d` WR-07.
(WR-01+WR-02 and WR-05+WR-06 are paired in single commits: one rewrite resolved each pair, both files overlapping.)

## Fixed Issues

### CR-01: `upsert_monitor_line` matches monitor lines by output substring — prefix-colliding outputs destroy each other's persisted lines

**Files modified:** `crates/archel-core/src/hyprland/monitors.rs`
**Commit:** `58a0b2a`
**Applied fix:** The upsert needle is now anchored on the exact quoted output name — `hl.monitor({ output = "{output}"` includes the closing quote, so `DP-1` can no longer match a `DP-1.1` line. Previously the unquoted prefix match let an upsert for one output clobber the line of any output whose name it prefixes.
**Test evidence:** `snippet_upsert_never_clobbers_prefix_colliding_outputs` (upsert `DP-1` against a snippet also containing `DP-1.1`; both lines survive intact). All monitors tests green.

### WR-01: `parse_snippet_state` reads appearance values anywhere in the shared snippet — bind/monitor lines can forge blur/opacity state

**Files modified:** `crates/archel-core/src/hyprland/appearance.rs`
**Commit:** `3b48eaa`
**Applied fix:** `parse_snippet_state` is now scoped to the managed block only: it locates the `MANAGED_BLOCK_SENTINEL` and parses solely within the `sentinel..})` block span. Values on bind or monitor lines outside the block can no longer forge blur/opacity state. The parser and `render_snippet` now share the `MANAGED_BLOCK_SENTINEL` constant so the contract cannot drift.
**Test evidence:** `parse_never_reads_values_outside_the_managed_block` (decoy `active_opacity`/blur keys on non-managed lines are ignored). All 13 appearance tests pass.

### WR-02: `active_opacity = ` is a substring of `inactive_opacity = ` — latent misparse ordered entirely by renderer accident

**Files modified:** `crates/archel-core/src/hyprland/appearance.rs`
**Commit:** `3b48eaa`
**Applied fix:** Key lookup uses a left-boundary-anchored `leaf_tail` matcher: a key only matches when the character before it is not a key-charset character (so `active_opacity` cannot match inside `inactive_opacity`), plus a `leading_number` guard on parsed values. Matching no longer depends on line emission order.
**Test evidence:** `parse_anchors_keys_so_substring_hazards_cannot_shadow` (snippet containing only `inactive_opacity` never yields an `active_opacity` reading, in either line order).

### WR-03: Rollback leg failures are silently swallowed in `apply_monitor_plan`

**Files modified:** `crates/archel-core/src/hyprland/monitors.rs`
**Commit:** `4cb3062`
**Applied fix:** A `run_rollback` helper collects every rollback-leg failure; if any leg fails, the orchestration upgrades the exit from the plain rollback outcome to `ArchelError::HyprlandUnavailable { detail }` naming the failed leg(s). The CLI's "restored" gate in `display_cmds.rs` matches only `ConfirmationAborted | ConfirmTimeout`, so the upgraded error cannot be mistaken for a clean restore — `display_cmds.rs` needed no change.
**Test evidence:** `orchestration_incomplete_rollback_fails_loud_not_silent` (injected failing rollback leg ⇒ typed `HyprlandUnavailable`, not the silent outcome). All 5 orchestration tests pass. *Logic-adjacent change — behavior is test-covered but the exit-code taxonomy is worth a glance.*

### WR-04: `unquote` does not undo `lua_quote` escaping — parse→render round-trip double-escapes exec args

**Files modified:** `crates/archel-core/src/hyprland/binds.rs`
**Commit:** `6ac1bbd`
**Applied fix:** `unquote` rewritten as the exact inverse of `lua_quote`: only `\\`→`\` and `\"`→`"` are unescaped (`lua_quote` rejects raw newlines, so `\n`/`\t` unescaping would have been an over-broad inverse that corrupted literal-backslash args); fast path when no backslash is present; trailing/unknown backslash sequences kept verbatim. The stale doc comment describing the old behavior was corrected.
**Test evidence:** `unquote_is_the_exact_inverse_of_lua_quote` (7 hostile strings including `a"b\c` and trailing backslash: `unquote(lua_quote(s)) == s` for all accepted inputs) and `bind_line_round_trips_hostile_exec_args` (parse→render of a bind line preserves the arg byte-for-byte). All 14 binds tests pass.

### WR-05: `search` and `package_card` pass unvalidated user strings to pacman/yay — flag injection on read paths

**Files modified:** `crates/archel-core/src/packages.rs`
**Commit:** `bd7bdde`
**Applied fix:** `search` now rejects any query whose trimmed form starts with `-` with typed `ArchelError::InvalidPackageName` before any spawn; `package_card` keeps its blank-name → `PackageNotFound` behavior first (preserving the existing not-found contract), then gates on `!is_valid_package_name(name)` → `InvalidPackageName` before any spawn — the same validator the mutation paths use. The two **pacman** query calls additionally pass a `--` end-of-options terminator (`-Ss --`, `-Si --`).
**Deviation from the review suggestion (recorded deliberately):** the fix initially added `--` to the **yay** calls too; a controlled experiment in this worktree showed `yay -Si -- <name>` deterministically hangs the suite's two live AUR tests against the 15 s query cap (both fail with `Internal("'yay' exceeded the query timeout")` with `--`, pass in ~1.6 s without — AUR-cache warm). yay therefore relies on the typed validator alone; pacman keeps the belt-and-braces terminator.
**Test evidence:** `search_rejects_flag_injection_queries_typed` and `package_card_rejects_flag_injection_names_typed` (hostile inputs `--config=…`, `-Sy`, `--ask`, padded `  --noconfirm`, `foo bar`, `foo/bar`, `foo;rm` — each refused typed, no spawn). Full packages module: 34 tests pass including the live-machine alpm/`pacman -Si`/AUR tests.

### WR-06: `confirm_action_timed` leaks the blocked stdin-reader thread on timeout

**Files modified:** `crates/archel-core/src/packages.rs`
**Commit:** `bd7bdde`
**Applied fix:** On the timeout path the receiving end of the mpsc channel is explicitly dropped, so a reader thread that wakes late finds its `tx.send` failed and exits — no thread lingers holding the channel after the window lapses. The caller contract is documented on the function: the process should end promptly after `ConfirmTimeout` (the display orchestration does), because a long-lived caller otherwise leaves the stale reader blocked on stdin where it could consume one line from a later prompt. Std-only, no new deps.
**Test evidence:** The TTY-gated body is not exercisable headless (the `NeedsTty` gate fires first, which the existing timed tests assert); the change is structural (an explicit `drop`) plus doc. *Runtime behavior of the timeout path requires a TTY to observe — spot-check once interactively: `ConfirmTimeout` fires and no reader thread survives (the process exits shortly after anyway per the documented contract).*

### WR-07: Keybind edit applies the new combo live but never unbinds the old combo

**Files modified:** `crates/archel/src/cmds/input_cmds.rs`
**Commit:** `fb0408d`
**Applied fix:** `run_keybind_edit_with` gains a second injected closure `unbind` (mirroring the removal path's injectable-eval shape). When the edit changes the chord identity — `old.mods != new.mods || old.key != new.key`, extracted into the testable `bind_identity_changed` helper — `unbind(old)` runs **before** `eval(new)`, using the same `hl.unbindkey` statement path that `run_keybind_remove` already uses (`core_binds::unbind_eval_statement` + `run_hyprctl_eval` in the production closure). Dispatcher/arg-only edits skip the unbind (the follow-up eval overwrites them in place). A refused unbind fails typed with the snippet publish never reached, matching the removal path's failure semantics. The GUI (`gui/models/input.rs`) has its own InputOps wiring and was deliberately left out of scope (CLI-only finding).
**Test evidence:** `rekey_decision_fires_only_on_mods_or_key_change` (mods/key changes rekey; dispatcher/arg-only and identical specs do not) and `rekey_edit_headless_refusal_runs_neither_eval_nor_unbind` (headless confirm refusal ⇒ neither live leg runs); the existing miss test now also spies that unbind never runs pre-confirm. All 243 archel-bin tests pass. *The live ordering (old chord actually gone from the session after a rekey edit) needs one interactive check under Hyprland: edit a combo's mods and confirm the old chord no longer fires.*

---

_Fixed: 2026-10-10_
_Fixer: gsd-code-fixer_
_Iteration: 1_
_Verification: `cargo test --offline -p archel-core -p archel` run in the isolated worktree (see note above); 520 passed / 0 failed / 1 ignored across both crates._
