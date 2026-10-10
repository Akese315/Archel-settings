---
phase: 06-gui-shell-tabs
reviewed: 2026-10-10T00:00:00Z
depth: standard
files_reviewed: 20
files_reviewed_list:
  - crates/archel/src/gui/models/power.rs
  - crates/archel/src/gui/models/audio.rs
  - crates/archel/src/gui/models/net.rs
  - crates/archel/src/gui/models/bt.rs
  - crates/archel/src/gui/models/appearance.rs
  - crates/archel/src/gui/models/input.rs
  - crates/archel/src/gui/models/display.rs
  - crates/archel/src/gui/modal.rs
  - crates/archel/src/gui/shell.rs
  - crates/archel/src/gui/bridge_net.rs
  - crates/archel/src/gui/bridge_bt.rs
  - crates/archel/src/gui/bridge_display.rs
  - crates/archel/src/gui/bridge_input.rs
  - crates/archel/src/thin_view_audit.rs
  - crates/archel-core/src/network.rs
  - crates/archel/src/gui/qml/tabs/NetTab.qml
  - crates/archel/src/gui/qml/tabs/BtTab.qml
  - crates/archel/src/gui/qml/tabs/DisplayTab.qml
  - crates/archel/src/gui/qml/tabs/InputTab.qml
  - crates/archel/src/gui/qml/components/VpnImportPopup.qml
  - crates/archel/src/gui/qml/components/PairingModal.qml
  - crates/archel/src/gui/qml/components/CountdownModal.qml
  - crates/archel/src/gui/qml/tabs/AppearanceTab.qml
  - justfile
findings:
  critical: 2
  warning: 6
  info: 3
  total: 11
status: issues_found
---

# Phase 06 (waves 6-7): Code Review Report — GUI write paths + verification harness

**Reviewed:** 2026-10-10
**Depth:** standard
**Files Reviewed:** 20+
**Status:** issues_found
**Scope:** git range `054402b..HEAD` (plans 06-07 system-domain writes, 06-08 hyprland writes, 06-09 justfile + thin-view audit).
**Noted:** open broken window #1 (camelCase vs snake_case QML bindings) is pre-existing and NOT re-reported below.

## Summary

The write-path choreography is consistently structured (validate/plan → core-verbatim confirm → apply → read-back, all mutations in `archel-core`, GUI holds no child/pid/signal) and the VPN `stdin_null` split, BT decline-unpairs rule, and display countdown/rollback are sound in the happy path. Two trust/data-loss defects must be fixed before ship (swallowed unpair error after an explicit pairing refusal; stale-body snippet overwrite in keybind removal), plus six warnings led by the 1 Hz tick fighting the Input tab's editable fields. No hardcoded secrets, no injection vectors (dispatcher vocabulary is allowlisted in core `plan_bind`, VPN path re-validated inside core import), no subprocess/IPC primitives in `gui/`.

## Critical Issues

### CR-01: Pairing-decline unpair failure is swallowed — refused device may stay bonded

**File:** `crates/archel/src/gui/models/bt.rs:464-467`
**Issue:** When the user declines the BlueZ pairing code, the worker does `let _ = (ops.unpair)(&device);` and returns silently. If that best-effort unpair fails (daemon race, permission shape, transport hiccup), the half-bond persists while the UI reports nothing — the user explicitly refused trust, yet the device remains paired with zero feedback. This is a trust-violation shape: a fail-closed design must surface the failure, not discard it.
**Fix:**
```rust
if declined.load(Ordering::SeqCst) {
    if let Err(err) = (ops.unpair)(&device) {
        sess.open_error(&err.message());
    }
    return;
}
```
(Decline itself stays silent on success; only the cleanup failure gets a modal.)

### CR-02: Keybind removal writes a pre-confirm snapshot — external edits made during the confirm are clobbered

**File:** `crates/archel/src/gui/models/input.rs:693-716`
**Issue:** `remove_keybind_worker` reads the snippet body and computes `merged` *before* `await_confirm`, then writes that stale `merged` after the user confirms. The confirm window is unbounded user think time, and this app explicitly supports external `hyprland.conf` edits (filesystem watcher per STACK). Any concurrent edit (CLI, another editor, file sync) between the read and the write is silently lost. The sibling `add_keybind_worker` (same file, lines 650-668) does it right: it re-reads the body *after* the confirm.
**Fix:** mirror the add path — move `(ops.read_body)` + `remove_bind_line` to after `await_confirm`, re-failing typed on miss drift:
```rust
if !await_confirm(sess, &core_binds::confirm_bind_remove_prompt(&target)) {
    return;
}
let body = match (ops.read_body)(base) { Ok(b) => b, Err(err) => { sess.open_error(&err.message()); return; } };
let merged = match core_binds::remove_bind_line(&body, &target) { Ok(m) => m, Err(err) => { sess.open_error(&err.message()); return; } };
```

## Warnings

### WR-01: 1 Hz tick overwrites in-progress user input on the Input tab

**File:** `crates/archel/src/gui/qml/tabs/InputTab.qml` (accelerateField `text:` binding; sensitivitySlider `value:` binding)
**Issue:** `accelerateField` binds `text: input.scrollText === "N/A" ? "" : input.scrollText` and the sensitivity `Slider` binds `value:` to the live model text. The tab timer ticks every second, so each tick resets whatever the user is mid-typing/dragging (e.g. typing "2.0" gets clobbered back to "1.50" after at most 1 s; slider drags fight the binding). The NetTab DNS editor in this same range solved this correctly with local `ListModel` state decoupled from the tick. The DisplayTab scale field also does it right (placeholder-only, no `text:` binding).
**Fix:** decouple like NetTab — hold editor state locally (plain `property string` / local var initialized from the model on completed + explicit sync button), write back only on Appliquer.

### WR-02: FileDialog `file://` strip breaks on spaces/non-ASCII (VPN import + wallpaper picker)

**File:** `crates/archel/src/gui/qml/components/VpnImportPopup.qml:33`, `crates/archel/src/gui/qml/tabs/AppearanceTab.qml:32`
**Issue:** Both pickers do `selectedFile.toString().replace("file://", "")`. A `file://` URL percent-encodes (`/home/u/mon%20bureau.ovpn`), so any path with spaces or non-ASCII decodes to a nonexistent literal and core validation rejects it. Fail-closed (no traversal — core re-validates), but the picker is broken for exactly the users who need it. `replace` with a string arg also strips only the first occurrence, which happens to be correct here but is fragile.
**Fix:** resolve to a local path instead of string-stripping, e.g. use the dialog's `selectedFile` → `Qt.resolvedUrl`/`toLocalFile()` equivalent for the Qt version in use, or `decodeURIComponent(path)`.

### WR-03: Single global confirm session multiplexes all tabs — cross-tab writes orphan each other's confirms

**File:** `crates/archel/src/gui/modal.rs:183-190` (`request_confirm` calls `reset()`)
**Issue:** Each tab model has its own `operation_busy` slot, but all workers share one process-global `ModalSession` confirm channel. Starting a write on tab B while tab A's confirm is pending silently drops A's waiter (`reset()` replaces `confirm_tx`; A's `recv()` returns `None` → default-No decline) and swaps the visible question. Fail-closed, but the user experiences a vanishing dialog and a silently cancelled operation, and a stale QML Cancel can resolve the *wrong* worker's receiver. `resolve_confirm` (line 201-205) also unconditionally sets `kind = None`, so a late resolve clears a newer modal while its worker still waits.
**Fix (minimum):** add a global write-slot guard (second write attempt while any confirm is pending is rejected with a busy signal), or generation-tag confirms so stale resolves are no-ops.

### WR-04: Keybind-add live-eval-before-persist can leave live/compositor and conf diverged

**File:** `crates/archel/src/gui/models/input.rs:650-670`
**Issue:** `add_keybind_worker` runs `(ops.eval_bind_line)` (live compositor) *before* `(ops.read_body)` + `write_body` (persist). If the read, merge, or write fails after a successful live eval, the bind is active in the compositor but absent from the snippet — it silently vanishes on the next reload, and the error modal gives no hint of the split-brain. (Failure of the eval itself is fine — nothing was applied.)
**Fix:** on any post-eval persist failure, attempt a compensating unbind-eval and include the divergence in the error text, or persist-then-eval with rollback of the conf edit.

### WR-05: GUI has no path back to automatic DNS (CLI `reset` leg unmapped)

**File:** `crates/archel/src/gui/models/net.rs:247-271`, `crates/archel/src/gui/qml/tabs/NetTab.qml` (`dnsValid` requires non-empty)
**Issue:** The core `DnsPlan` carries a `reset` leg and the CLI exposes clearing custom DNS, but the GUI editor disables Appliquer for an empty list (`dnsValid = err === "" && servers.length > 0`) and Réinitialiser only re-syncs the editor rows. A user who set custom DNS via the GUI can never revert to automatic via the GUI — a one-way door for a network-critical setting.
**Fix:** add an explicit "Automatique" affordance that plans with `reset: true` through the same confirm-gated worker, or document as intentional scope cut with a cli-only annotation so the thin-view audit stays honest.

### WR-06: `just lint-qml` word-splits paths and fails cryptically without qmllint

**File:** `justfile:31`
**Issue:** `for f in $(find ...)` splits on whitespace (breaks on spaces in checkout paths), and if neither `qmllint-qt6` nor `qmllint` exists the recipe runs `"" "$f"` with an inscrutable error. This is the PLAT-03 gate — it should fail loudly and clearly.
**Fix:**
```sh
command -v qmllint-qt6 >/dev/null || command -v qmllint >/dev/null || { echo "qmllint not found"; exit 1; }
find crates/archel/src/gui/qml -name '*.qml' -print0 | while IFS= read -r -d '' f; do "..." "$f" || exit 1; done
```

## Info

### IN-01: Audio read-back matches on device name, not id — duplicate names can false-positive

**File:** `crates/archel/src/gui/models/audio.rs:270-275`, `296-302`
**Issue:** The `stuck` closures prove the switch with `sink.name == *target && sink.default`. If two sinks ever share a name, the read-back proof can pass for the wrong device. Core resolves by id (`resolve_sink_id`); carrying the resolved `id` into the `stuck` check instead of the name closes it.
**Fix:** capture `id` and check `sink.id == id && sink.default`.

### IN-02: NetTab hardcodes the `"N/A"` literal instead of sharing the model constant

**File:** `crates/archel/src/gui/qml/tabs/NetTab.qml` (`server !== "N/A"` in `syncDnsFromCurrent`)
**Issue:** Duplicates `NOT_AVAILABLE` (`models/system.rs:22`). Benign today (same value, fail-safe direction — worst case an extra empty row), but silent drift if the token ever changes or localizes.
**Fix:** expose the token as a model property or compare against `net.dnsText` emptiness instead.

### IN-03: Thin-view audit's Cargo.toml gate parses the manifest with string splits

**File:** `crates/archel/src/thin_view_audit.rs:231-244`
**Issue:** The `[dependencies]` section is extracted via `split("[dependencies]").nth(1).split('[').next()`, which misses target-gated deps (`[target.'cfg(...)'.dependencies]`) and dev/build sections — a future IPC dep smuggled in via those sections would pass the gate. The `FORBIDDEN` line-scan also fires on comments (false-positive-prone, though currently green).
**Fix:** parse with `cargo_metadata` or the `toml` crate already in the workspace, and strip `//` comments before the literal scan.

---

_Reviewed: 2026-10-10_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
