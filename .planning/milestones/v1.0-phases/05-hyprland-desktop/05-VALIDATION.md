---
phase: "5"
slug: "hyprland-desktop"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-07"
---

# Phase 5 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust, Phase 4 discipline: fake-transport unit tests + `--no-default-features` gating for live-bus code) |
| **Config file** | none — existing workspace test infra covers this phase |
| **Quick run command** | `cargo test -p archel-core --no-default-features <module>` (per prior-phase pattern; module TBD by planner: appearance/display/input) |
| **Full suite command** | `cargo test --workspace --no-default-features` |
| **Estimated runtime** | ~60 seconds |

*Infrastructure fields are seeded from the Phase 4 proven verify commands; the planner/executor refine per-task commands in the Per-Task Verification Map.*

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p archel-core --no-default-features <touched-module>`
- **After every plan wave:** Run `cargo test --workspace --no-default-features`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 90 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 05-01-01 | 01 | 1 | TBD by planner | T-05-NN / — | TBD | unit | TBD | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Per-module test stubs for APP-01..04, DSP-01..04, INP-01..04 — planner/executor to enumerate once PLAN tasks exist
- [ ] Live-Hyprland probe helpers (hyprctl eval/getoption fixtures) if not already provided by Phase 1 spike infra

*If none: "Existing infrastructure covers all phase requirements."*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Wallpaper visible after hyprpaper kill/respawn | APP-01 | Visual result of wallpaper render cannot be asserted headlessly | Run `archel appearance wallpaper set <file>`, visually confirm wallpaper changed; confirm via UAT |
| Monitor confirm-and-rollback dialog UX behavior | DSP-01 | Rollback timing/visual feedback needs live display | Change resolution via CLI, decline confirm, verify previous mode restored within timeout |

*Planner may add/remove rows during planning.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 90s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
