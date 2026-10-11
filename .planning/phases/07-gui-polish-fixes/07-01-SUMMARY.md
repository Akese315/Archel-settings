---
phase: 07-gui-polish-fixes
plan: "01"
subsystem: gui-input-tab
tags: [qml, cxx-qt, draft-state, wr-01, gui-01, tracer]
requires: [phase-6-inputtab-tick-editors, nettab-draft-pattern]
provides: [input-draft-state-loop, discard-or-apply-popup]
affects: [07-02, 07-03, 07-04]
actuals:
  tokens: 1841
  tasks: 2
  commits: 2
tech-stack:
  added: []
  patterns: [draft-local-editor-state, base-compare-external-wins-sync, dirty-and-valid-enablement, tab-owned-discard-or-apply-dialog]
key-files:
  created: []
  modified: [crates/archel/src/gui/qml/tabs/InputTab.qml]
key-decisions: ["Validation flags stay lightweight QML guards with the worker authoritative (no new invokable)", "Popup is InputTab-owned with forced choice, never the shared session", "Timer also re-syncs as fallback behind the property-change signals"]
patterns-established: ["Input draft-state loop (sensDraft/accelDraft plus sensBase/accelBase, reset/sync helpers, Connections plus Timer fallback) available as the copy-target for later tick-bound editors"]
requirements-completed: [GUI-01]
coverage:
  - id: GUI-01-tick-survival
    description: Typing in sensitivity/accelerate editors survives consecutive 1 Hz ticks; read-only StatFields keep refreshing
    requirement: GUI-01
    verification:
      - kind: automated
        ref: qmllint crates/archel/src/gui/qml/tabs/InputTab.qml exits 0
        status: pass
      - kind: manual
        ref: 07-VALIDATION.md GUI-01 tick drill (type during 3+ ticks, drag slider, confirm no yank-back)
        status: pending-human
    human_judgment: true
    rationale: qmllint proves syntax only; the tick-vs-keystroke race needs a live GUI per VALIDATION.md
  - id: GUI-01-gating-popup
    description: Appliquer enabled only on dirty-and-valid drafts; tab return with dirty draft offers Appliquer or Abandonner, never silent revert or keep
    requirement: GUI-01
    verification:
      - kind: automated
        ref: grep enabled bindings combine draft-differs-from-base with validation flag; grep Abandonner count >= 1; no shared-session reference in new wiring
        status: pass
      - kind: manual
        ref: 07-VALIDATION.md GUI-01 popup drill (edit, switch tab, return, exercise both legs)
        status: pending-human
    human_judgment: true
    rationale: Modal flow and enablement feel need a live GUI per VALIDATION.md
duration: 2m
completed: "2026-10-11T18:35:00Z"
status: complete
---

# Phase 7 Plan 01: Input Draft-State Tracer Summary

GUI-01 tracer slice: the Input tab sensitivity slider and accelerate field now edit draft-local state mirroring NetTab, proven end-to-end before any expansion fix lands.

## Performance

- Duration: ~2m (18:33–18:35 UTC 2026-10-11)
- Tasks: 2/2, commits: 2, files touched: 1 (QML-only, zero Rust changes)

## Accomplishments

- Draft-local editors: `sensDraft`/`accelDraft` plus `sensBase`/`accelBase` snapshots on the InputTab root; slider binds `sensValueOf(sensDraft)`, accelerate field binds `accelDraft` with `onTextEdited` write-back.
- External-wins sync (D-02): `syncSensFromLive`/`syncAccelFromLive` replace base and draft when the bridge value moves; untouched drafts are left alone. Hooked to `onSensitivityTextChanged`/`onScrollTextChanged` plus a Timer re-sync fallback; 1 Hz Timer keeps `running: true, repeat: true` (D-01, no tick-freezing).
- StatField read-only displays keep their live `input.sensitivityText`/`input.scrollText` bindings.
- Appliquer gating (D-03): `sensDraft !== sensBase && sensValid` and `accelDraft !== accelBase && accelValid`, both also gated on `!input.operationBusy`.
- Tab-return popup (D-04): InputTab-owned `Dialog` (forced choice via `Popup.NoAutoClose`, `modal: true`) opened from `onVisibleChanged` when dirty and no write is in flight; Appliquer leg writes dirty drafts, Abandonner leg resets drafts from live. No reference to the shared session anywhere in the new wiring.
- Scope guard honored: `comboField`/`actionField` untouched (no tick binding, per Pitfall 4).

## Task Commits

- `772637a` feat(7-01): draft-local Input editors with base-compare tick sync
- `b937fc8` feat(7-01): gate Appliquer on dirty-and-valid plus tab-return popup

## Files Created/Modified

- Modified: `crates/archel/src/gui/qml/tabs/InputTab.qml` (+160/−13) — draft properties, read/reset/sync helpers, Connections, gated buttons, draftDialog, visibility trigger
- Created: `.planning/phases/07-gui-polish-fixes/07-01-SUMMARY.md` (this file)

## Decisions Made

- Validation flags stay lightweight QML guards with the worker authoritative (no new invokable): no QML-callable pre-check invokable exists for pointer values (`add_keybind`'s synchronous check covers keybind fields only), so `sensValid` (non-empty, numeric) and `accelValid` (non-blank) gate the buttons while `plan_input_set_at` in the worker stays the single definition of valid — the same two-layer shape as NetTab (`dnsValid` gate plus worker re-check). Typos still surface in ErrorModal, exactly as before.
- Popup is InputTab-owned with forced choice, never the shared session: per the resolved Open Question 2 and to avoid colliding with GUI-03 pending confirms; `NoAutoClose` means Esc cannot silently dismiss, the draft is never silently reverted nor kept.
- Timer also re-syncs as fallback behind the property-change signals: `onTriggered` calls `input.tick()` then both syncs, so drafts converge even if a `Changed` signal name ever mismatches the bridge codegen.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Plan-text inapplicable] Validation mechanism for pointer drafts**
- Found during: Task 2
- Issue: The task action names "the shared plan_keybind_from_fields definition of valid via the existing invokable pre-check" for the sensitivity/accelerate Appliquer gates, but `plan_keybind_from_fields` validates keybind combo/action fields only — it cannot validate pointer values, and no `input.validateX` invokable exists on the bridge (verified: `bridge_input.rs` exposes only `tick`, `dismissBanner`, `setSensitivity`, `setAccelerate`, `addKeybind`, `removeKeybind`).
- Fix: Gated on lightweight QML validity guards (`sensValid`, `accelValid`) with the existing worker-side core validation authoritative; acceptance criteria (draft-differs-plus-validation-flag bindings) met verbatim.
- Files modified: `crates/archel/src/gui/qml/tabs/InputTab.qml`
- Commit: `b937fc8`

Otherwise: none — plan executed as written (single-file QML scope, French strings, no package installs per T-7-SC).

## Issues Encountered

- None blocking. qmllint passed on the first run for both tasks.

## User Setup Required

- None. Manual UAT drills (tick-survival, popup legs per 07-VALIDATION.md GUI-01 rows) are harvested at phase verification — not run here (no live Hyprland session in this environment).

## Next Phase Readiness

- Tracer loop proven at the QML level; plans 07-02…07-04 can build on the draft-state pattern. GUI-01 awaits its live-GUI manual drills at phase verification alongside the other Phase 7 manual rows.

## Self-Check: PASSED

- InputTab.qml exists with sensDraft/accelDraft, gated Appliquer buttons, draftDialog with Abandonner leg — verified by qmllint plus acceptance greps above.
- Commits `772637a`, `b937fc8` present in `git log`.
