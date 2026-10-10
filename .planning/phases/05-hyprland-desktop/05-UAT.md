---
status: complete
phase: 05-hyprland-desktop
source: [05-01-01-SUMMARY.md, 05-02-01-SUMMARY.md, 05-03-01-SUMMARY.md, 05-04-01-SUMMARY.md, 05-VERIFICATION.md]
started: 2026-10-08T14:05:00Z
updated: 2026-10-10T14:30:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Wallpaper visibly shown on the desktop
expected: After `archel appearance wallpaper set <valid-image>` the desktop background visibly changes to the chosen image (hyprpaper restart mechanism works end to end). Revert afterwards.
result: skipped
reason: "Deferred follow-up: User chose to advance to next phase without live visual UAT (2026-10-08); mechanism is covered by automated tests (test 9) plus the executor's live spot-check with state restore."

### 2. Kept display change visibly active
expected: After `archel display set` and confirming KEEP within the 15 s window, the monitor is visibly at the new mode/scale/position. Rejecting or timing out visibly restores the previous configuration.
result: skipped
reason: "Deferred follow-up: User chose to advance to next phase without live visual UAT (2026-10-08); orchestration covered by automated tests (tests 11-14) plus D-05 no-op live proof on HDMI-A-2 recorded in 05-03-01-SUMMARY.md."

### 3. Added keybind visibly fires
expected: After `archel input keybind add` registers a new combo (e.g. SUPER+SHIFT+F13 -> a dispatcher), physically pressing the combo actually performs the action on the live compositor. Remove the bind afterwards.
result: skipped
reason: "Deferred follow-up: User chose to advance to next phase without live visual UAT (2026-10-08); covered by automated tests (tests 15-18) plus executor F13 add/unbind live round-trip recorded in 05-04-01-SUMMARY.md."

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

### 19. Timed-confirm reader thread exits after timeout
expected: After `archel display set` times out without an answer (no KEEP within 15 s on a real TTY), the blocked stdin-reader thread exits rather than lingering in the process (WR-06 fix, fix bd7bdde; receiver dropped so late send fails and thread returns).
result: skipped
reason: "Deferred follow-up: deferred from verification 2026-10-10 — TTY-only observable; headless environment gates the confirm path on NeedsTty. Logic-level tests exist."

### 20. Keybind edit rekey leaves only the new chord live
expected: After `archel input keybind edit` changes a combo's mods/key under a live compositor, the OLD chord no longer triggers (live `hl.unbindkey`) and only the NEW chord fires, without restarting Hyprland (WR-07 fix, fix fb0408d).
result: skipped
reason: "Deferred follow-up: deferred from verification 2026-10-10 — live unbind ordering needs a real Hyprland session. Logic-level decision + refusal-gate tests exist."

## Summary

total: 20
passed: 15
issues: 0
pending: 0
skipped: 5

## Deferred Follow-Ups

```yaml
- test: 1
  idea: "Live wallpaper change on the Hyprland desktop (archel appearance wallpaper set) — verify visibly, then revert."
  deferred_at: 2026-10-08
- test: 2
  idea: "Live display set keep-or-rollback on a real monitor (KEEP within 15 s; reject/timeout restores)."
  deferred_at: 2026-10-08
- test: 3
  idea: "Physically press an added keybind (archel input keybind add) and confirm it fires on the live compositor; remove afterwards."
  deferred_at: 2026-10-08
- test: 19
  idea: "Timed-confirm reader thread exits after timeout on a real TTY (WR-06, fix bd7bdde)."
  deferred_at: 2026-10-10
- test: 20
  idea: "Keybinds edit rekey leaves only the new chord live under a real Hyprland session (WR-07, fix fb0408d)."
  deferred_at: 2026-10-10
```

## Gaps

[none yet]
