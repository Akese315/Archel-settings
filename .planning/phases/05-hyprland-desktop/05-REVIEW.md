---
phase: 05-hyprland-desktop
reviewed: 2026-10-10T12:30:00Z
depth: standard
files_reviewed: 12
files_reviewed_list:
  - crates/archel-core/src/hyprland/mod.rs
  - crates/archel-core/src/hyprland/appearance.rs
  - crates/archel-core/src/hyprland/binds.rs
  - crates/archel-core/src/hyprland/monitors.rs
  - crates/archel-core/src/hyprland/wallpaper.rs
  - crates/archel-core/src/hyprland/font.rs
  - crates/archel-core/src/lib.rs
  - crates/archel-core/src/error.rs
  - crates/archel-core/src/packages.rs
  - crates/archel/src/cmds/appearance_cmds.rs
  - crates/archel/src/cmds/display_cmds.rs
  - crates/archel/src/cmds/input_cmds.rs
findings:
  critical: 1
  warning: 7
  info: 0
  total: 8
status: issues_found
---

# Phase 05: Code Review Report

**Reviewed:** 2026-10-10T12:30:00Z
**Depth:** standard
**Files Reviewed:** 12
**Status:** issues_found

## Summary

Reviewed the six Hyprland core modules, the core crate surface (`lib.rs`, `error.rs`), `packages.rs`, and the three CLI command modules for Phase 05.

The subprocess boundary is generally solid: every value reaching `hyprctl eval` goes through `lua_quote` (mod.rs:133-146) which escapes `\` and `"` and rejects newline/NUL, so Lua statement injection into `hl.bind(...)`/`hl.monitor(...)` statements is closed. Writes are atomic (tmp + rename), backups are timestamped with collision suffixes and rotated (mod.rs:210-265), and `wallpaper.rs`/`font.rs` validate inputs against strict allowlists with persist-on-agreement semantics.

The main systemic weakness is **substring matching against the shared app snippet**. The snippet `~/.config/hypr/modules/archel.lua` holds the appearance block, appended `hl.monitor(...)` lines, and appended `hl.bind(...)` lines in one file, and three separate places parse or rewrite it with whole-text substring operations (`contains`/`find`) rather than line-shape-scoped matching. One of these (monitor upsert) can destroy a durable config line under realistic hardware naming; the others are user-triggerable misparse/round-trip corruption paths. The confirm/rollback orchestration also silently discards rollback failures, and the keybind edit path leaves the old combo live-bound until restart.

## Critical Issues

### CR-01: `upsert_monitor_line` matches monitor lines by output substring — prefix-colliding outputs destroy each other's persisted lines

**File:** `crates/archel-core/src/hyprland/monitors.rs:552-573`
**Issue:** The replacement condition is `body_line.contains("hl.monitor(") && body_line.contains(output)` (line 556), with first-match-wins in body order. Output names come from live hardware (`current.name`, monitors.rs:381/414) and commonly form prefix pairs: `DP-1` and `DP-1.1` (MST/dock connectors), `DP-1` and `DP-10`, `HDMI-A-1` and `HDMI-A-11`. When the snippet already holds `hl.monitor("DP-1.1", ...)` **before** the `DP-1` line (the snippet is append-order, so this is exactly what happens when DP-1.1 was persisted first), an upsert for `DP-1` matches the `DP-1.1` line via `contains("DP-1")` and overwrites it. Result: the DP-1.1 managed line is destroyed (durable config loss for that monitor), the stale `DP-1` line survives, and the new `DP-1` line is placed at the wrong position. No error is reported — the write "succeeds".
**Fix:** Match the exact quoted first argument instead of a bare substring:
```rust
// monitors.rs:556 — snippet lines are always rendered as
// hl.monitor("<output>", ...) via lua_quote, so anchor on the quoted token.
let needle = format!("hl.monitor(\"{output}\",");
if !replaced && body_line.contains(&needle) {
    // ...replace...
}
```
(Alternatively parse the first quoted argument of each `hl.monitor(` line and compare for equality, which also survives future argument-order changes.)

## Warnings

### WR-01: `parse_snippet_state` reads appearance values anywhere in the shared snippet — bind/monitor lines can forge blur/opacity state

**File:** `crates/archel-core/src/hyprland/appearance.rs:117-131`
**Issue:** The snippet is shared: appearance block first, then `hl.monitor(...)`/`hl.bind(...)` lines appended by monitors.rs/binds.rs. `text.contains("enabled = false")` (line 119) and `text.find(marker)` for the opacity markers (line 126) search the **whole file**, not just the managed appearance block. Opaque exec args pass through `lua_quote`, which only escapes `"` and `\` — plain text survives verbatim. So `archel input keybinds add --action exec --argument 'grep "enabled = false" cfg'` (or any bind/workspace/monitor line containing `enabled = false` or `active_opacity = 0.5`) makes `get_display()` report blur off / forged opacity, and the next `appearance set` re-renders the forged value into the durable appearance block. User-triggered config corruption.
**Fix:** Scope parsing to the managed appearance block only — slice the text between the appearance block sentinels (the same markers `render_appearance_block` emits) before `contains`/`find`, or parse line-by-line requiring the trimmed line to *start with* the key (e.g. `line.trim_start().starts_with("enabled = false")`) within the block region.

### WR-02: `active_opacity = ` is a substring of `inactive_opacity = ` — latent misparse ordered entirely by renderer accident

**File:** `crates/archel-core/src/hyprland/appearance.rs:122-132`
**Issue:** `text.find("active_opacity = ")` (line 126, marker loop at 123) also matches inside `inactive_opacity = ` because the former is a literal substring of the latter. Today it is only harmless because the renderer (appearance.rs:74-89) always emits `active_opacity` before `inactive_opacity`, so `find` hits the real line first. Any change to render order, a hand-touched snippet missing the active line, or the WR-01 injection path makes the active opacity silently inherit the inactive value and get re-persisted.
**Fix:** Anchor the match to line start: iterate `text.lines()` and match `line.trim_start().starts_with(marker)`, or search for `"\nactive_opacity = "` plus an explicit first-line check.

### WR-03: Rollback leg failures are silently swallowed in `apply_monitor_plan`

**File:** `crates/archel-core/src/hyprland/monitors.rs:619-621, 631-633, 637-639`
**Issue:** All three rollback loops use `let _ = rollbacker(rollback_leg);`. If a rollback leg itself fails (hyprctl dead, eval refused), the monitor stays in the half-applied broken state (possibly off/black) while the CLI reports only `ConfirmationAborted`/`ConfirmTimeout` — display_cmds.rs:119-125 even prints "restored" wording (line 124) in that case, which is actively false. The user is told the rollback completed when it may not have run at all.
**Fix:** Collect failures and surface them:
```rust
let mut rb_failed: Vec<String> = Vec::new();
for rollback_leg in &plan.rollback {
    if let Err(err) = rollbacker(rollback_leg) {
        rb_failed.push(format!("{rollback_leg}: {err}"));
    }
}
if !rb_failed.is_empty() {
    eprintln!("warning: rollback legs failed, monitor may be in an inconsistent state: {rb_failed:?}");
}
```
and gate the "restored" message in display_cmds.rs on rollback success.

### WR-04: `unquote` does not undo `lua_quote` escaping — parse→render round-trip double-escapes exec args

**File:** `crates/archel-core/src/hyprland/binds.rs:411-415` (vs `lua_quote` at mod.rs:133-146)
**Issue:** `unquote` strips one layer of double quotes but never unescapes `\"`/`\\` (the doc comment's claim at lines 408-410 that "rendered args never contain raw quotes" is wrong: `lua_quote` deliberately produces `\"` and `\\` sequences). Therefore `parse_bind_line(render_bind_line(spec)).arg != spec.arg` whenever the arg contains quotes or backslashes. Concrete corruption path: `run_keybind_edit` keeps the old arg via `old_arg_of` (input_cmds.rs:133, 150-164), which returns the **escaped** text; feeding that back through `plan_bind` → `render_bind_line` → `lua_quote` re-escapes it (`\"` → `\\\"`), so editing `--new-key` on an existing bind whose exec command contains quotes corrupts the command both live and in the snippet, on every edit generation.
**Fix:** Make `unquote` the true inverse of `lua_quote`:
```rust
fn unquote(raw: &str) -> Option<String> {
    let inner = raw.strip_prefix('"').and_then(|s| s.strip_suffix('"'))?;
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.push(chars.next()?),
            c => out.push(c),
        }
    }
    Some(out)
}
```
Also correct the stale doc comment.

### WR-05: `search` and `package_card` pass unvalidated user strings to pacman/yay — flag injection on read paths

**File:** `crates/archel-core/src/packages.rs:87-103, 246-258`
**Issue:** Install/remove/aur paths validate package-name charset, but `search` passes `query` straight into `pacman -Ss <query>` / `yay -Ss <query>` (lines 92, 101) and `package_card` passes `name` into `pacman -Si <name>` / `yay -Si <name>` (lines 253, 258) after only an emptiness check. A leading-dash value (`--config=/tmp/evil.conf`, `--nocenter`, etc.) is parsed as an **option** by the subprocess, not a pattern — pacman will read an attacker-/typo-chosen config file, and yay (which shells out to makepkg paths for some options) is a wider surface than pacman. Subprocess-arg arrays prevent shell injection, but not option injection.
**Fix:** Insert the option terminator and/or reuse the existing charset validator:
```rust
run_capped_query("pacman", &["-Ss", "--", query], ArchelError::Internal)
```
plus reject `query`/`name` starting with `-` (or apply the same name-validation helper used by install/remove) with a typed `DesktopInvalid`/`PackageNotFound` error.

### WR-06: `confirm_action_timed` leaks the blocked stdin-reader thread on timeout

**File:** `crates/archel-core/src/packages.rs:516-527`
**Issue:** On timeout (line 525) the function returns while the spawned thread (517-522) remains parked in `stdin().read_line()` forever. Harmless in the short-lived CLI (process exit reaps it), but `archel-core` is the API layer the GUI consumes — in a long-lived process every timed-out confirmation permanently leaks a thread plus an unconsumed channel sender, and a later unrelated stdin read gets silently swallowed by the zombie reader.
**Fix:** Document the single-shot CLI-only contract, or make the reader detachable: send the answer through the channel and have the reader exit when the receiver is dropped (`tx.send(...)` returning `Err` is the signal), e.g. drop-based guard, or move the timed variant onto a cancellable line-buffered input abstraction before GUI reuse.

### WR-07: Keybind edit applies the new combo live but never unbinds the old combo

**File:** `crates/archel/src/cmds/input_cmds.rs:169-196`
**Issue:** `run_keybind_edit_with` evaluates only `render_bind_line(new)` (line 187) and publishes the edited snippet, but never runs `unbind_eval_statement(old)` when `--new-mods`/`--new-key` changed the combo. After `keybinds edit --mods SUPER --key T --new-key U`, SUPER+T stays live-bound to the old action until the next Hyprland restart (where it then disappears — the inverse of what the user just asked for). Live state and snippet disagree, violating the phase's live-apply contract that `remove` honors (line 236 runs the unbind leg).
**Fix:** When `old.mods != new.mods || old.key != new.key`, evaluate the unbind first:
```rust
if old.mods != new.mods || old.key != new.key {
    let stmt = core_binds::unbind_eval_statement(old)?;
    archel_core::hyprland::run_hyprctl_eval("hyprctl", &stmt)?;
}
eval(new)?;
```

## Info

_No info-level findings._

---

_Reviewed: 2026-10-10T12:30:00Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
