---
phase: 07-gui-polish-fixes
plan: "02"
subsystem: gui-pickers-keybind
tags: [qml, file-dialog, toLocalFile, keybind, persist-then-eval, wr-02, wr-04, gui-02, gui-04]
requires: [phase-6-inputtab-keybind-workers, phase-6-picker-popup-d20]
provides: [picker-path-decoding, keybind-persist-ordering]
affects: [07-03, 07-04]
actuals:
  tokens: 1013
  tasks: 2
  commits: 2
tech-stack:
  added: []
  patterns: [qurl-tolocalfile-decode, persist-then-eval-worker]
key-files:
  created: []
  modified: [crates/archel/src/gui/qml/components/VpnImportPopup.qml, crates/archel/src/gui/qml/tabs/AppearanceTab.qml, crates/archel/src/gui/models/input.rs]
key-decisions: ["Empty toLocalFile result keeps prior text via an explicit guard (never clears the field)", "Persist-then-eval lean adopted for WR-04 (no compensating unbind)", "Wallpaper preview file-scheme-prefix line left untouched"]
patterns-established: ["Picker decode guard (toLocalFile plus empty-check) as the copy-target for any future FileDialog site", "Failing-write_body fake asserting zero eval calls as the worker-ordering regression shape"]
requirements-completed: [GUI-02, GUI-04]
coverage:
  - id: GUI-02-picker-decode
    description: VPN and wallpaper pickers decode percent-encoded selections to real paths; typed paths unchanged; worker-side validation still gates
    requirement: GUI-02
    verification:
      - kind: automated
        ref: qmllint exits 0 on both files; grep for the file-scheme-strip expression absent in both files
        status: pass
      - kind: manual
        ref: 07-VALIDATION.md GUI-02 manual row (space plus accent pick in each picker)
        status: pending-human
    human_judgment: true
    rationale: qmllint proves syntax only; decoded-characters rendering needs a live GUI per VALIDATION.md
  - id: GUI-04-persist-order
    description: Keybind-add persists the snippet merge before live eval; a persist failure surfaces the error modal with zero live-compositor mutation
    requirement: GUI-04
    verification:
      - kind: automated
        ref: cargo test --offline -p archel --features gui input — 29 passed including new add_persist_failure_never_touches_the_live_compositor
        status: pass
      - kind: manual
        ref: 07-VALIDATION.md GUI-04 live-fire drill (forced persist failure, confirm no stray live bind)
        status: pending-human
    human_judgment: true
    rationale: Unit test locks ordering via fakes; the live-compositor absence needs a Hyprland session per VALIDATION.md
duration: 3m
completed: "2026-10-11T18:38:00Z"
status: complete
plan_head_before: e22403b28e99849506e2ee4977e324b4eb7ee8fe
plan_head_after: 788c4c31d94305b4309ad05172656aecae7206b4c
---

# Phase 7 Plan 02: Picker Decoding + Keybind Persist Ordering Summary

Expansion slice: `toLocalFile()` decoding at both FileDialog picker sites (GUI-02/WR-02) and persist-then-eval ordering in `add_keybind_worker` with a failing-write regression test (GUI-04/WR-04).

## Performance

- Duration: ~3m (2026-10-11)
- Tasks: 2/2, commits: 2, files touched: 3 (2 QML-only bindings, 1 Rust worker plus test)

## Accomplishments

- Picker decoding: `VpnImportPopup.qml` and `AppearanceTab.qml` `onAccepted` handlers now source the path from `selectedFile.toLocalFile()`, so percent-encoded selections (spaces, accents) resolve to real filesystem paths. Empty-selection guard keeps the prior field text instead of clearing it (flagged-assumption boundary contract). Typed-path fields untouched (already raw paths); wallpaper preview `file://`-prefix line untouched; popup frozen api (`busy`, `importRequested(path)`, `dismissed`) preserved with no filesystem/nmcli touch in QML. Core `validate_vpn_import_path` stays the gate (T-7-02).
- Keybind ordering: `add_keybind_worker` now runs read_body, `add_bind_line`, write_body before `eval_bind_line`, so a persist failure returns through the existing `sess.open_error` path before any live-compositor mutation (T-7-03). Validate, plan, collision-hint, confirm, and PollGuard/CxxQtThread discipline untouched. New `add_persist_failure_never_touches_the_live_compositor` test (failing `write_body` fake) asserts eval op count is zero and the error-modal path is taken. Input suite: 29 passed, 0 failed.
- Idempotency finding (flagged assumption): core `add_bind_line` dedupes — an existing app line with the same (mods, key, dispatcher) address is replaced in place (`binds.rs:481-504`), so running add_keybind twice on the same combo yields one snippet line. No new locking added; parallel adds serialize through the existing worker discipline.

## Task Commits

- `f8eec76` feat(7-02): picker path decoding via toLocalFile at both sites
- `788c4c3` feat(7-02): keybind-add persist-then-eval reorder with regression test

## Files Created/Modified

- Modified: `crates/archel/src/gui/qml/components/VpnImportPopup.qml` — `onAccepted` decodes via `toLocalFile()` with empty-guard
- Modified: `crates/archel/src/gui/qml/tabs/AppearanceTab.qml` — wallpaper picker `onAccepted` decodes via `toLocalFile()` with empty-guard
- Modified: `crates/archel/src/gui/models/input.rs` — persist-then-eval reorder in `add_keybind_worker` plus failing-write regression test
- Created: `.planning/phases/07-gui-polish-fixes/07-02-SUMMARY.md` (this file)

## Decisions Made

- Empty `toLocalFile` result keeps prior text via an explicit guard (never clears the field): implements the flagged-assumption boundary contract (empty accept = keep prior text, one step either side of a valid decode).
- Persist-then-eval lean adopted for WR-04 (no compensating unbind): keeps failures pre-live with nothing to compensate, per the research lean and CONTEXT discretion.
- Wallpaper preview `file://`-prefix line left untouched: the decode needed no adjustment there.

## Deviations from Plan

None - plan executed exactly as written (no package installs per T-7-SC; byte-faithful decoding shows no mangling at the lint level, so no manual-decode fallback was layered).

## Issues Encountered

- None blocking. qmllint passed on the first run; cargo input suite green on the first run including the new test.

## Threat Flags

None beyond the plan's threat register — T-7-02 and T-7-03 mitigations hold (validation unbypassed, failures pre-live); no new network endpoints, auth paths, or schema changes.

## Known Stubs

None.

## User Setup Required

- None. Manual UAT rows (space/accent picks per GUI-02, live-fire persist-failure drill per GUI-04) are harvested at phase verification — not run here (no live Hyprland session in this environment).

## Next Phase Readiness

- Picker and keybind-ordering expansion slices land cleanly beside the 07-01 tracer with no shared-file overlap (07-01 touched InputTab.qml only). Plans 07-03/07-04 can proceed. GUI-02/GUI-04 await their live-GUI manual rows at phase verification.

## Self-Check: PASSED

- Both QML files contain `toLocalFile()` with no `replace("file://"` occurrence — verified by grep plus qmllint exit 0.
- `write_body` (line 668) precedes `eval_bind_line` (line 672) in `add_keybind_worker` source order — verified by line-number grep.
- Commits `f8eec76`, `788c4c3` present in `git log`; plan ledger counts 2 commits.
