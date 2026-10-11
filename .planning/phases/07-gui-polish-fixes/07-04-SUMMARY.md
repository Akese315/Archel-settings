---
phase: 07-gui-polish-fixes
plan: "04"
subsystem: gui-dns-reset-casing
tags: [dns-reset, automatiquemode, camelcase, binding-spell-gate, gui-05, gui-06, wr-05]
requires: [07-01, 07-02, 07-03, core-plan-dns-reset-leg]
provides: [dns-reset-affordance, qml-camelcase-convention, binding-spell-gate]
affects: [phase-8-hygiene, phase-9-live-uat]
actuals:
  tokens: 6813
  tasks: 3
  commits: 3
tech-stack:
  added: []
  patterns: [reset-worker-without-validation, radio-mode-with-session-memory, binding-spell-grep-gate]
key-files:
  created: []
  modified: [crates/archel/src/gui/models/net.rs, crates/archel/src/gui/bridge_net.rs, crates/archel/src/gui/qml/tabs/NetTab.qml, crates/archel/src/gui/qml/tabs/PkgTab.qml, justfile, .planning/WINDOWS.md]
key-decisions: ["Automatique visual shape is an Automatique/Manuel radio pair at the top of the Serveurs DNS group (cheapest button-row-consistent shape)", "Manual row edits flip the mode back to Manuel so a declined reset confirm self-heals", "Main.qml shell.modal_* snake_case bindings left untouched (shell prefix outside the D-15 model-prefix gate scope)"]
patterns-established: ["Reset worker shape (plan_reset plus confirm plus apply, validation skipped) as the copy-target for future no-input writes", "Binding-spell grep gate (snake model-prefix pattern piped through cxx_name exclusion, negated) as the merge-blocking convention check"]
requirements-completed: [GUI-05, GUI-06]
coverage:
  - id: GUI-05-reset-worker
    description: dns_reset_worker plans DHCP restore against the active profile, confirms with the core-verbatim reset question, applies via apply_dns_plan; decline writes nothing
    requirement: GUI-05
    verification:
      - kind: automated
        ref: cargo test --offline -p archel --features gui net — 26 passed including dns_reset_accept_applies_the_dhcp_restore and dns_reset_decline_writes_nothing
        status: pass
      - kind: manual
        ref: 07-VALIDATION.md GUI-05 manual row (switch auto through confirm, back to manual, verify pre-fill)
        status: pending-human
    human_judgment: true
    rationale: Unit tests lock plan-confirm-apply via fakes; the live NM write plus modal flow needs a Hyprland session per VALIDATION.md
  - id: GUI-05-automatique-ui
    description: NetTab Serveurs DNS group offers an Automatique choice through the standard confirm modal with lastManualServers session pre-fill on return to manual
    requirement: GUI-05
    verification:
      - kind: automated
        ref: qmllint exits 0 on NetTab.qml; grep finds lastManualServers and dnsReset call site
        status: pass
      - kind: manual
        ref: 07-VALIDATION.md GUI-05 manual row (Automatique through confirm modal, manual pre-fill)
        status: pending-human
    human_judgment: true
    rationale: qmllint proves syntax only; mode radios plus confirm-modal flow need a live GUI per VALIDATION.md
  - id: GUI-06-casing-gate
    description: All tab bindings camelCase with zero snake_case model-property bindings; lint-qml binding-spell check fails the build on violation; WINDOWS.md id-1 fixed
    requirement: GUI-06
    verification:
      - kind: automated
        ref: qmllint sweep over all QML exits 0; snake_case binding grep empty; gate proven both directions (clean passes, snake fixture flagged, cxx_name comment excluded); WINDOWS.md id-1 status fixed with resolved date
        status: pass
      - kind: manual
        ref: 07-VALIDATION.md GUI-06 manual row (offscreen smoke plus click through all 10 tabs, AUR review modal renders)
        status: pending-human
    human_judgment: true
    rationale: Lint plus grep prove spelling only; Qt case-folding resolution of the renamed pkg bindings plus the AUR alias chain need the live smoke per VALIDATION.md
duration: 4m
completed: "2026-10-11T18:48:29Z"
status: complete
plan_head_before: 6c72a2bece133badfa07278963798739ec777273
plan_head_after: ae9d147bf33198cd4571942111d998b16ffedeba
commits: 3
---

# Phase 7 Plan 04: Automatic DNS + Casing Convention Summary

Closing slice (per D-09…D-15): the NetTab Automatique affordance over the CLI-proven reset leg with session memory, plus the all-tab camelCase pass with a merge-blocking binding-spell gate and WINDOWS.md closure.

## Performance

- Duration: ~4m (2026-10-11 18:44–18:48 UTC)
- Tasks: 3/3, commits: 3, files touched: 6 (2 Rust, 2 QML, justfile, WINDOWS.md)

## Accomplishments

- Reset worker: `NetDnsOps` gains `plan_reset` wired to core `plan_dns_reset`; `dns_reset_worker` mirrors `dns_worker` with validation skipped (reset takes no servers) — plan against the active profile (`None`), `request_confirm` with the core `confirm_dns_prompt` question verbatim, `apply_dns_plan` which branches on `plan.reset` to the DHCP restore. Same privilege gate as set (`run_nm_write`), no new escalation path (T-7-06).
- Bridge invokable: `dnsReset` (`cxx_name`) beside `dnsApply` with the identical busy-guard plus `set_dns_busy` plus `spawn_blocking` plus `qt_thread` queue reset plus `tick()` shape.
- Reset tests: `dns_reset_accept_applies_the_dhcp_restore` (accept applies, asserts the reset flag reaches apply, asserts the set/validate legs never run) and `dns_reset_decline_writes_nothing` (decline applies zero times). Net suite: 26 passed, 0 failed.
- Automatique UI: `Automatique`/`Manuel` radio pair at the top of the Serveurs DNS GroupBox (cheapest shape consistent with the button row, per the flagged assumption). Automatique stashes non-empty manual rows into `lastManualServers`, sets `dnsAutomatic`, calls `net.dnsReset()` through the standard confirm modal (D-10, no inline-apply exception). Manuel pre-fills `dnsRows` from `lastManualServers` (D-11). Active profile only (D-12, no selector change). French labels; no nmcli argv in QML.
- Casing pass: PkgTab converted snake→camel (27 binding lines: `bannerVisible`, `searchBusy`, `operationBusy`, `searchJson`, `installedJson`, `zeroHit`, `operationKind`, `cardJson`) as the exemplar, header comment rewritten to state the D-13 convention. The other seven tab files were already camelCase — each lint-verified individually rather than assumed. Invokable call sites (`tick`, `dnsApply`, `aurBuildStart`, the `pkgTab.pkg.aurBuildStart()` chain via the PkgTab line-21 alias) untouched (pitfall 5).
- Gate: `lint-qml` gains the D-15 binding-spell check (`! grep -rEn '<model-prefixes>\.[a-z]+_[a-z>' tabs/ | grep -v 'cxx_name'`), proven green-on-clean, red-on-snake-fixture, and passing on `cxx_name` comment lines. Hand-mirror comment updated (just is not installed). The gate rides the existing `gui-check` merge path.
- WINDOWS.md id-1 closed via `windows fixed` (status `fixed`, resolved 2026-10-11); ledger now `open_count: 0`.

## Task Commits

- `ede3ac3` feat(7-04): dnsReset worker plus bridge invokable via core reset leg
- `9c70e61` feat(7-04): Automatique DNS choice with session memory in NetTab
- `ae9d147` feat(7-04): camelCase casing pass plus binding-spell gate plus WINDOWS closure

## Files Created/Modified

- Modified: `crates/archel/src/gui/models/net.rs` — `plan_reset` seam entry, `dns_reset` invokable body, `dns_reset_worker`, 2 new tests (+3 existing literals extended)
- Modified: `crates/archel/src/gui/bridge_net.rs` — `dnsReset` qinvokable beside `dnsApply`
- Modified: `crates/archel/src/gui/qml/tabs/NetTab.qml` (+59/−1) — `lastManualServers`/`dnsAutomatic` properties, mode radio row, manual-edit mode flip-backs
- Modified: `crates/archel/src/gui/qml/tabs/PkgTab.qml` — 27-line camelCase conversion plus convention header rewrite
- Modified: `justfile` — D-15 binding-spell check in `lint-qml` plus hand-mirror comment
- Modified: `.planning/WINDOWS.md` — id-1 row `fixed` with resolved date (via CLI verb)
- Created: `.planning/phases/07-gui-polish-fixes/07-04-SUMMARY.md` (this file)

## Decisions Made

- Automatique visual shape is an Automatique/Manuel radio pair at the top of the Serveurs DNS group (cheapest button-row-consistent shape): resolves the flagged GUI-05 assumption; behavior contract D-09…D-12 fully locked regardless of shape.
- Manual row edits flip the mode back to Manuel so a declined reset confirm self-heals: the worker reports no outcome to QML, so any typing/Ajouter/Retirer/Reinitialiser after a decline returns the radios to Manuel instead of leaving a false Automatique state.
- Main.qml `shell.modal_*` snake_case bindings left untouched (shell prefix outside the D-15 model-prefix gate scope): converting the shell mirror would widen the diff into the 07-03 stacked-modal surface with no gate requirement behind it; recorded as out-of-scope, not a stub.

## Deviations from Plan

None - plan executed exactly as written (no package installs per T-7-SC; ASCII-only Rust sources; French QML labels). Two scope findings, neither a deviation: the seven non-PkgTab files needed zero edits (already camelCase — each still lint-verified individually per the per-file atomicity rule); Main.qml shell bindings intentionally out of gate scope (see Decisions).

## Threat Flags

None beyond the plan's threat register — T-7-06 holds (reset routes through `apply_dns_plan` into `run_nm_write`, confirm-first preserved); T-7-07 holds (property-only rename, invokable call sites pinned and grepped intact, gate plus full sweep green; runtime case-folding of the renamed pkg bindings awaits the live smoke in the GUI-06 manual row).

## Known Stubs

None.

## Issues Encountered

- None blocking. Net suite green on the first run including both new tests; qmllint green on the first run for every touched file.

## User Setup Required

- None. Manual UAT rows (Automatique confirm plus pre-fill drill per GUI-05, 10-tab click-through plus AUR review modal per GUI-06) are harvested at phase verification — not run here (no live Hyprland session in this environment; offscreen smoke likewise deferred).

## Next Phase Readiness

- Phase 7 is now 4/4 plans complete: GUI-01…GUI-06 land with automated gates green and manual rows harvested for Phase 9 live UAT. Phase 8 (hygiene) can proceed; the `lint-qml` gate now blocks merge on casing regressions.

## Self-Check: PASSED

- `dns_reset_worker` present in models/net.rs calling `plan_reset` then `request_confirm` then apply; `dnsReset` invokable beside `dnsApply` in bridge_net.rs; both new reset tests pass in the 26-test net suite.
- NetTab.qml contains the Automatique radio, `lastManualServers`, and the `dnsReset` call site; qmllint exits 0.
- Snake_case binding grep over tabs/ empty outside `cxx_name` lines; justfile lint path contains the binding-spell check; WINDOWS.md id-1 reads `fixed` with resolved date; full qmllint sweep exits 0.
## Self-Check: PASSED

- `dns_reset_worker` present in models/net.rs calling `plan_reset` then `request_confirm` then apply; `dnsReset` invokable beside `dnsApply` in bridge_net.rs; both new reset tests pass in the 26-test net suite.
- NetTab.qml contains the Automatique radio, `lastManualServers`, and the `dnsReset` call site; qmllint exits 0.
- Snake_case binding grep over tabs/ empty outside `cxx_name` lines; justfile lint path contains the binding-spell check; WINDOWS.md id-1 reads `fixed` with resolved date; full qmllint sweep exits 0.
- Commits `ede3ac3`, `9c70e61`, `ae9d147` present in `git log`; rev-list from plan base counts 3.
