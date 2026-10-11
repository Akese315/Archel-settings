---
phase: 07-gui-polish-fixes
plan: "03"
subsystem: gui-modal-session
tags: [modal-session, confirm-stack, qml, cxx-qt, wr-03, gui-03]
requires: [phase-6-modal-session, phase-6-sync-session-mirror]
provides: [confirm-stack, stacked-confirm-rendering]
affects: [07-04]
actuals:
  tokens: 4162
  tasks: 2
  commits: 2
tech-stack:
  added: []
  patterns: [generation-tagged-confirm-stack, top-only-resolution, stacked-modal-mirror]
key-files:
  created: []
  modified: [crates/archel/src/gui/modal.rs, crates/archel/src/gui/shell.rs, crates/archel/src/gui/qml/Main.qml]
key-decisions: ["Fixed two-modal rendering shape with a hard cap of 2 stacked confirms; overflow past the cap declines the newest through the standard false path", "New QML bindings use camelCase per the D-13 going-forward rule; the pre-existing snake_case base-modal bindings are left for the 07-04 casing pass"]
patterns-established: ["Pending-confirm stack (Vec of generation-tagged entries behind the existing Mutex discipline) as the session pattern for parallel worker waits", "Top-entry mirror (question_top plus confirm_count) beside the index-0 question as the stacked-rendering shape"]
requirements-completed: [GUI-03]
coverage:
  - id: GUI-03-parallel-confirms
    description: Two tabs can each hold a pending confirm at the same time with neither silently declined; each waiter keeps its own receiver until its own entry resolves
    requirement: GUI-03
    verification:
      - kind: automated
        ref: cargo test --offline -p archel --features gui modal — 18 passed including two_concurrent_confirms_coexist
        status: pass
      - kind: manual
        ref: 07-VALIDATION.md GUI-03 manual row (arm writes in two tabs, confirm both modals resolve independently)
        status: pending-human
    human_judgment: true
    rationale: Unit tests lock session independence via receivers; two live tabs resolving through two live modals needs a Hyprland session per VALIDATION.md
  - id: GUI-03-stacked-rendering
    description: A second confirm modal stacks above the first (z 105 over z 100) and each resolves independently through top-resolving invokables
    requirement: GUI-03
    verification:
      - kind: automated
        ref: qmllint exits 0 on Main.qml and ConfirmModal.qml; grep shows two ConfirmModal instances with the second at higher z
        status: pass
      - kind: manual
        ref: 07-VALIDATION.md GUI-03 manual row (stacked modals, top-only cancel leaves older pending)
        status: pending-human
    human_judgment: true
    rationale: qmllint proves syntax only; stacked-modal layering and independent resolution need a live GUI per VALIDATION.md
  - id: GUI-03-top-only-cancel
    description: Cancelling the top modal removes exactly the newest entry while older pending confirms stay stacked and resolvable
    requirement: GUI-03
    verification:
      - kind: automated
        ref: cargo test --offline -p archel --features gui modal — top_only_resolve_leaves_the_older_pending and stacked_decline_resolves_top_false_and_keeps_the_older pass
        status: pass
      - kind: manual
        ref: 07-VALIDATION.md GUI-03 manual row (cancel the top, verify the older stays pending)
        status: pending-human
    human_judgment: true
    rationale: Unit tests lock pop-newest-only semantics; the cancel-top-then-older-stays drill needs a live GUI per VALIDATION.md
  - id: GUI-03-error-voice
    description: Write failures still surface through the single global error modal; the error path and pairing session are untouched
    requirement: GUI-03
    verification:
      - kind: automated
        ref: cargo test --offline -p archel --features gui modal — error_session_carries_readable_text and both pairing tests pass; full gui suite 248 passed
        status: pass
    human_judgment: false
    rationale: Fully determined by automated tests — reset() still clears pending waits with identical per-waiter decline outcome, and open_error is byte-identical
duration: 5m
completed: "2026-10-11T18:43:00Z"
status: complete
plan_head_before: 6f50939eac30cc77b4e6d5bdad312b3225e41e73
plan_head_after: e0a96c594d39345a012fdb30fd9e763751d669a6
commits: 2
---

# Phase 7 Plan 03: Parallel Confirm Sessions Summary

Expansion slice (per D-05, D-06, D-07, D-08): parallel per-tab confirm sessions replacing the single-slot reset behavior. A second tab confirm no longer orphans the first waiter; the second modal stacks above the first and each resolves independently.

## Performance

- Duration: ~5m (2026-10-11)
- Tasks: 2/2, commits: 2, files touched: 3 (2 Rust bridge/session, 1 QML shell)

## Accomplishments

- Confirm stack: `ModalSession::request_confirm` pushes a generation-tagged `PendingConfirm` entry (generation id, question string, sender) onto a `Vec` behind the existing Mutex discipline and never resets siblings; each waiter keeps its own receiver until its own entry resolves. `resolve_confirm` pops exactly the newest entry (top-only, D-08); when entries remain the kind stays `Confirm` with `question` re-pointed at index 0, and only the last resolution closes the session.
- Overflow contract (flagged-assumption resolution): the stack is capped at `MAX_STACKED_CONFIRMS = 2` matching the fixed two-modal rendering; a request past the cap is declined immediately through the standard decline path (its worker sees `false`, writes nothing, shows nothing — D-12-compliant, no error modal).
- Error path byte-identical (D-07): `open_error` still calls `reset()`, and `reset()` draining the stack drops every pending sender — each waiter observes the same decline (`recv` Err → `unwrap_or(false)`) it saw under the old single-slot reset. Pairing session (`begin_pairing`/`pairing_request_code`/`resolve_pairing`/`end_pairing`) untouched.
- Stacked mirror: `sync_session` publishes `modal_question_top` (newest entry question) and `modal_confirm_count` beside the index-0 `modal_question`; all property writes stay inside `sync_session` on the Qt thread via the 120 ms Timer, with no worker-side queue calls for confirm state. New `confirmTopAccepted`/`confirmTopDeclined` invokables resolve through the same top-only session path; `confirmAccepted`/`confirmDeclined` keep working unchanged.
- Second `ConfirmModal` in Main.qml at `z: 105` above the base confirm (`z: 100`), bound to `shell.modalQuestionTop` with visibility `shell.modalKind === "confirm" && shell.modalConfirmCount > 1` — derived only from mirrored state, no imperative close (ConfirmModal.qml NoAutoClose rule). ErrorModal stays the single global instance.
- Regression tests: `two_concurrent_confirms_coexist`, `top_only_resolve_leaves_the_older_pending`, `stacked_decline_resolves_top_false_and_keeps_the_older`, `confirm_overflow_past_two_declines_the_newest` — modal suite 18/18 green, full GUI suite 248/248 green, qmllint green on both QML files.

## Task Commits

- `6f88466` feat(7-03): confirm stack in ModalSession with regression tests
- `e0a96c5` feat(7-03): stacked mirror plus second ConfirmModal rendering

## Files Created/Modified

- Modified: `crates/archel/src/gui/modal.rs` (+169/−14) — `PendingConfirm` stack, `MAX_STACKED_CONFIRMS`, push-never-reset `request_confirm`, top-only `resolve_confirm`, `snapshot_question_top`/`snapshot_confirm_count`, 4 new tests
- Modified: `crates/archel/src/gui/shell.rs` (+35) — `modal_question_top`/`modal_confirm_count` qproperties, snapshot fields, sync setters, `confirmTopAccepted`/`confirmTopDeclined` invokables
- Modified: `crates/archel/src/gui/qml/Main.qml` (+14) — second stacked ConfirmModal at z 105
- Created: `.planning/phases/07-gui-polish-fixes/07-03-SUMMARY.md` (this file)

## Decisions Made

- Fixed two-modal rendering shape with a hard cap of 2 stacked confirms; overflow past the cap declines the newest through the standard false path: implements the flagged-assumption boundary contract (two concurrent confirms is the practical max; the newest-overflow decline is D-12-compliant since decline writes nothing and shows nothing).
- New QML bindings use camelCase (`modalQuestionTop`, `modalConfirmCount`, `modalKind`) per the D-13 going-forward rule; the pre-existing snake_case base-modal bindings are left untouched for the 07-04 casing pass (no opportunistic rename, keeps the diff reviewable).

## Deviations from Plan

None - plan executed exactly as written (no package installs per T-7-SC; Mutex-only workers plus Qt-thread-only property writes preserved per T-7-04; questions mirrored from the Rust session verbatim per T-7-05).

## Threat Flags

None beyond the plan's threat register — T-7-04 holds (generation-tagged entries, per-entry receivers, top-only resolution, Mutex-only worker access with Qt-thread-only property writes); T-7-05 holds (second modal renders the mirrored session question verbatim, QML owns no session truth).

## Known Stubs

None.

## Issues Encountered

- None blocking. Modal suite green on the first run including all 4 new tests; qmllint green on the first run; full GUI suite (248 tests, covering every worker that shares the session) green with no regressions.

## User Setup Required

- None. The manual stacked-confirm drill (arm writes in two tabs, confirm both modals resolve independently, cancel the top and verify the older stays pending per 07-VALIDATION.md GUI-03 row) is harvested at phase verification — not run here (no live Hyprland session in this environment).

## Next Phase Readiness

- GUI-03 lands cleanly beside the 07-01/07-02 slices with no shared-file overlap (07-01 touched InputTab.qml only; 07-02 touched pickers plus input.rs). Plan 07-04 (DNS Automatique plus casing pass) can proceed; its casing pass should convert the pre-existing snake_case base-modal bindings in Main.qml alongside the tab files. GUI-03 awaits its live-GUI manual row at phase verification.

## Self-Check: PASSED

- `request_confirm` contains no `reset()` call and no `confirm_tx` remnant — verified by grep.
- 4 new stack tests listed and passing in `cargo test --offline -p archel --features gui modal` (18/18); full gui suite 248/248.
- Main.qml contains two `ConfirmModal` instances (z 100 and z 105); qmllint exits 0 on both QML files.
- Commits `6f88466`, `e0a96c5` present in `git log`; plan ledger counts 2 commits.
