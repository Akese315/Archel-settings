---
status: complete
phase: 05-hyprland-desktop
source: [05-01-01-SUMMARY.md, 05-02-01-SUMMARY.md, 05-03-01-SUMMARY.md, 05-04-01-SUMMARY.md]
started: 2026-10-08T14:05:00Z
updated: 2026-10-08T14:20:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Wallpaper visibly shown on the desktop
expected: After `archel appearance wallpaper set <valid-image>` the desktop background visibly changes to the chosen image (hyprpaper restart mechanism works end to end). Revert afterwards.
result: skipped
reason: "User chose to advance to next phase without live visual UAT (2026-10-08); mechanism is covered by automated tests (test 9) plus the executor's live spot-check with state restore."

### 2. Kept display change visibly active
expected: After `archel display set` and confirming KEEP within the 15 s window, the monitor is visibly at the new mode/scale/position. Rejecting or timing out visibly restores the previous configuration.
result: skipped
reason: "User chose to advance to next phase without live visual UAT (2026-10-08); orchestration covered by automated tests (tests 11-14) plus D-05 no-op live proof on HDMI-A-2 recorded in 05-03-01-SUMMARY.md."

### 3. Added keybind visibly fires
expected: After `archel input keybind add` registers a new combo (e.g. SUPER+SHIFT+F13 -> a dispatcher), physically pressing the combo actually performs the action on the live compositor. Remove the bind afterwards.
result: skipped
reason: "User chose to advance to next phase without live visual UAT (2026-10-08); covered by automated tests (tests 15-18) plus executor F13 add/unbind live round-trip recorded in 05-04-01-SUMMARY.md."

### 4. Blur toggle end-to-end
expected: User toggles window blur from the CLI with live-apply, read-back, and snippet durability (APP-03)
result: pass
source: automated
coverage_id: D-05-01-01-a

### 5. Opacity set end-to-end
expected: User sets active/inactive opacity from the CLI with live-apply; invalid floats rejected pre-spawn (APP-04)
result: pass
source: automated
coverage_id: D-05-01-01-b

### 6. Out-of-session degradation
expected: Out-of-session invocation degrades to typed HyprlandUnavailable, never panic or hang
result: pass
source: automated
coverage_id: D-05-01-01-c

### 7. D-08 write-scope invariant
expected: D-08 invariant: user config byte-for-byte unchanged except the guarded require line; every write backed up
result: pass
source: automated
coverage_id: D-05-01-01-d

### 8. Font set end-to-end
expected: User sets system font from the CLI with GTK plus Hyprland live-apply, read-back, and snippet durability (APP-02)
result: pass
source: automated
coverage_id: D-05-02-01-a

### 9. Wallpaper set end-to-end (mechanics)
expected: User sets wallpaper from the CLI with conf-write plus daemon restart; invalid paths rejected pre-spawn (APP-01)
result: pass
source: automated
coverage_id: D-05-02-01-b

### 10. Shared seams hardened
expected: Shared seams hardened once for all later waves (WR-01 ordering, WR-05 stamps, WR-06 atomic require-line, WR-07 typed timeout)
result: pass
source: automated
coverage_id: D-05-02-01-c

### 11. Monitor list
expected: User lists every connected monitor with current mode, scale, refresh, and position from hyprctl monitors -j (DSP-01..04 reads)
result: pass
source: automated
coverage_id: D-05-03-01-a

### 12. Display set with 15 s keep-or-rollback
expected: User changes mode/scale/position per monitor with a 15 s keep-or-rollback re-confirm; reject/timeout restores the snapshot (DSP-01, D-04)
result: pass
source: automated
coverage_id: D-05-03-01-b

### 13. Display validators reject before any side effect
expected: Invalid scale divisibility, unknown modes, typos, and overlapping layouts rejected typed-DesktopInvalid before any prompt, spawn, or write (DSP-02/03/04)
result: pass
source: automated
coverage_id: D-05-03-01-c

### 14. Timed confirm primitive
expected: Timed confirm primitive (15 s window, std-only) with NeedsTty gate and catalog-complete ConfirmTimeout error
result: pass
source: automated
coverage_id: D-05-03-01-d

### 15. Keybind list
expected: User lists user binds (binds -j with decoded mods) plus app-managed binds (snippet, marked managed) from archel input keybinds list (INP-01)
result: pass
source: automated
coverage_id: D-05-04-01-a

### 16. Keybind add/edit/remove exact-match
expected: User adds, edits, and removes keybinds by exact (mods, key, dispatcher) match with confirm gates; misses fail typed before any prompt (INP-02, D-06)
result: pass
source: automated
coverage_id: D-05-04-01-b

### 17. Mouse leaves end-to-end
expected: User tunes mouse sensitivity, natural scroll, and scroll factor with pre-spawn validation, eval live-apply, read-back, and persist-on-agreement (INP-03/04, D-07)
result: pass
source: automated
coverage_id: D-05-04-01-c

### 18. User bind blocks untouched
expected: User bind blocks stay byte-for-byte unchanged; only app-owned snippet paths change (D-08 write-scope spy)
result: pass
source: automated
coverage_id: D-05-04-01-d

## Summary

total: 18
passed: 15
issues: 0
pending: 0
skipped: 3

## Gaps

[none yet]
