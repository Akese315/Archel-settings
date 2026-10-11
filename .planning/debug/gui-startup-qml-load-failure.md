---
status: resolved
trigger: ./target/debug/archel settings fails at startup — QQmlApplicationEngine failed to load component (invalidOverride duplicate signals in 3 modals + CountdownModal defaultButton)
created: 2026-10-11
updated: 2026-10-11
reopened: 2026-10-11 — follow-up failure after 022acb2: CountdownModal.qml:32 `onEscPressed` is not a Dialog/Popup signal; same misuse pattern in 5 modals
---

# Debug Session: gui-startup-qml-load-failure

## Symptoms

- Expected: `archel settings` opens the graphical settings window.
- Actual: engine fails to load `Main.qml`; no window. Chain: Main.qml → DisplayTab → CountdownModal (`Cannot assign to non-existent property "defaultButton"` at CountdownModal.qml:89). Warnings: duplicate signal names in PairingModal.qml:26, StreamModal.qml:34, ConfirmModal.qml:17 (`invalid override of property change signal or superclass signal`), plus `Final member result is overridden in StreamModal`.
- Timeline: first GUI run after rebuilding with `--features gui` (previous binary was headless). Never worked in this checkout.
- Reproduction: `./target/debug/archel settings` (also under offscreen platform).

## Evidence

- timestamp: 2026-10-11 — ConfirmModal.qml declares `signal accepted()` / `signal declined()` inside a `Dialog` root, which already provides `accepted()` (Qt Quick Controls Dialog built-in). Same pattern suspected in PairingModal/StreamModal with `result`.
- timestamp: 2026-10-11 — CountdownModal.qml:89 assigns `defaultButton: keepButton`, but Dialog has no `defaultButton` property (fatal load error).
- timestamp: 2026-10-11 — Full audit: `defaultButton:` present in ALL five Dialog modals (Confirm/ Pairing/Stream/Error/Countdown); `signal closed()` in StreamModal+ErrorModal collides with Popup.closed(); `signal accepted()` in Confirm+Pairing collides with Dialog.accepted(); `declined`/`keepRequested`/`revertRequested`/`cancelRequested` verified collision-free (Dialog has rejected/discarded, no declined). Remaining `onAccepted` uses (VpnImportPopup FileDialog, PkgTab, AppearanceTab) are Qt built-in dialogs — untouched.
- timestamp: 2026-10-11 — Fix applied: `accepted`→`confirmed` (ConfirmModal, PairingModal); `result`→`operationResult`, `closed`→`dismissed` (StreamModal); `closed`→`dismissed` (ErrorModal); all `defaultButton: X` removed, `focus: true` on each primary button; Main.qml handlers rewired (onConfirmed×3, onDismissed×2, operationResult binding). No Rust changes (QML signals only connect to shell invokables in QML).
- timestamp: 2026-10-11 — Verification: qmllint clean on all 5 modals + Main.qml + DisplayTab; `cargo build -p archel` ok; killed stale pre-fix instance (PID 146912, exe deleted, holding abstract socket with no window); `QT_QPA_PLATFORM=offscreen timeout 25 ./target/debug/archel settings` → ran full 25s, zero engine errors (exit 124 = timeout kill, healthy event loop); `cargo test -p archel` → 250 passed, 0 failed.
- timestamp: 2026-10-11 — Follow-up failure reproduced the same chain (Main.qml → DisplayTab → CountdownModal): `CountdownModal.qml:32:5: Cannot assign to non-existent property "onEscPressed"`. Popup/Dialog has no such signal. Resume hypothesis (Keys.onEscapePressed) rejected on type grounds: Keys attaches to Item, Popup is QObject-based, so a Keys handler on the Dialog root would be silently dead — the Esc contract (UI-SPEC keyboard row) would verify "clean" but never fire. Correct mechanism is a window-context `Shortcut { sequence: "Escape"; enabled: root.visible }` per modal (enabled-gate preserves the binding-driven visibility contract: Esc emits only the session decision signal, never closes). Applied to all 5 modals (StreamModal keeps its `if (root.done)` guard). Precedent already in-tree: SystemPalette (non-visual QObject) declared directly under Dialog root in ErrorModal/StreamModal, so Shortcut-as-direct-child uses the same contentData path.

## Resolution

- root_cause: Custom QML `signal`/`property` declarations shadowed Qt Quick Controls Dialog/Popup built-ins (`accepted`, `closed`, final `result`), and all five Dialog modals assigned the non-existent `defaultButton` property — the CountdownModal instance (via DisplayTab, eagerly created) made it fatal at startup.
- fix: Renamed colliding members (`confirmed`, `dismissed`, `operationResult`), replaced `defaultButton` with `focus: true` on each primary button, rewired Main.qml handlers; verified via qmllint, offscreen run, and full test suite.
- prevention: why not caught: no gate existed for this class (QML is not compiled; first GUI run in this checkout); guard: qmllint over qml/ + offscreen startup smoke run before GUI-phase sign-off.
- follow-up root_cause: all five Dialog modals assigned non-existent `onEscPressed` (Popup/Dialog has no such signal; Keys-attached Esc handling does not apply to non-Item Popup roots) — the eagerly-created CountdownModal (via DisplayTab) made it fatal at startup, same chain as the defaultButton failure.
- follow-up fix: per-modal window-context `Shortcut { sequence: "Escape"; enabled: root.visible }` emitting the existing session signal (revertRequested/declined/dismissed, StreamModal done-gated); NoAutoClose + binding-driven visibility untouched; verified via qmllint (7 files clean), offscreen 25 s run with zero engine output (exit 124), `cargo test -p archel` 250 passed.

## Eliminated

(none yet)

## Current Focus

- hypothesis: Esc handling verified via Shortcut (Keys rejected — Popup is not an Item); startup chain loads clean.
- test: qmllint + offscreen load of Main.qml after fix.
- expecting: Zero engine errors, window loads.
- next_action: none — fix verified (qmllint clean, offscreen 25 s zero-output run, 250 tests passed); session ready to close
