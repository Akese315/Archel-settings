---
phase: "7"
slug: "gui-polish-fixes"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-11"
---

# Phase 7 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from 07-RESEARCH.md ## Validation Architecture (Nyquist enabled; GUI-fix phase — heavy manual-visual surface, QML lint gates).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (core: 278 tests; bin: 125 tests) + qmllint + offscreen smoke |
| **Config file** | Cargo workspace root; `justfile` recipes (note: `just` binary absent on this machine — mirror recipes by hand) |
| **Quick run command** | `cargo test --offline -p archel-core <module>` (scoped per fix) |
| **Full suite command** | `cargo test --offline` (full) ; GUI gate: qmllint over `crates/archel/src/gui/qml/` + offscreen smoke + gui tests |
| **Estimated runtime** | ~120 seconds (full core+bin); qmllint seconds |

---

## Sampling Rate

- **After every task commit:** Run scoped `cargo test --offline` for the touched module + qmllint on touched QML
- **After every plan wave:** Run full `cargo test --offline` + full qmllint pass
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 180 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 7-01-01 | 01 | 1 | GUI-01 | — | N/A (no trust boundary crossed) | qml-lint + manual | `qmllint crates/archel/src/gui/qml/tabs/InputTab.qml` | ✅ | ⬜ pending |
| 7-01-02 | 01 | 1 | GUI-03 | — | N/A | unit (rust) | `cargo test --offline -p archel-core modal` | ✅ | ⬜ pending |
| 7-02-01 | 02 | 1 | GUI-02 | — | N/A (local file pick, no secret) | qml-lint + manual | `qmllint` on VpnImportPopup.qml + AppearanceTab.qml | ✅ | ⬜ pending |
| 7-02-02 | 02 | 1 | GUI-04 | — | N/A | unit | `cargo test --offline -p archel-core input` | ✅ | ⬜ pending |
| 7-03-01 | 03 | 2 | GUI-05 | — | N/A (reset leg already CLI-proven) | unit + manual | `cargo test --offline -p archel-core dns` | ✅ | ⬜ pending |
| 7-03-02 | 03 | 2 | GUI-06 | — | N/A | qml-lint + binding-spell check | `just gui-check` equivalent (qmllint all tabs) | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements — no Wave 0 stubs needed. Core legs (dns reset, keybind persist, modal session) are CLI-proven with existing tests; QML fixes verify via qmllint + offscreen smoke + manual tick/confirm drills.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Type in Input tab while 1 Hz tick fires; draft survives | GUI-01 | Tick-vs-keystroke race needs a live GUI | Open Input tab, type a keybind chord, watch 3+ ticks, confirm draft intact; apply and verify |
| Unapplied-draft popup on tab return (discard/apply) | GUI-01 | Modal flow needs a live GUI | Edit Input, switch tab, return, exercise both popup legs |
| Pick VPN/wallpaper path with spaces/accents | GUI-02 | FileDialog needs a live session | Pick such a file in both pickers; verify resolved path |
| Two tabs confirm in parallel; stacked modals; top-only cancel | GUI-03 | Multi-modal stacking needs a live GUI | Arm writes in two tabs, confirm both, cancel top, verify independence |
| Keybind-add fires live; failure leaves no stale snippet | GUI-04 | Live compositor behavior | Add combo, press it, verify fire; force persist failure path |
| "Automatique" DNS via confirm modal; remembers last manual | GUI-05 | Live NM + modal flow | Switch auto (confirm), back to manual, verify pre-fill |
| All tabs render with camelCase bindings, no ReferenceError | GUI-06 | QML runtime binding errors surface live | Offscreen smoke + click through 10 tabs |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 180s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
