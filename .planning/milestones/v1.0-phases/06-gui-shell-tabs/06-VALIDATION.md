---
phase: "06"
slug: "gui-shell-tabs"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-09"
---

# Phase 06 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (workspace) + qmllint + offscreen smoke; insta (1.48) for geometry/streaming snapshot shapes |
| **Config file** | none extra (workspace Cargo); spike precedent |
| **Quick run command** | `cargo test -p archel-core --no-default-features && cargo test -p archel --no-default-features` |
| **Full suite command** | `cargo clippy -p archel-core --all-features -- -D warnings && cargo test -p archel-core --no-default-features && cargo test -p archel --no-default-features && QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui` |
| **Estimated runtime** | ~120 seconds |

---

## Sampling Rate

- **After every task commit:** Quick run command + targeted `--features gui` test for the task
- **After every plan wave:** Full suite command
- **Before `/gsd-verify-work`:** Full suite green + offscreen smoke green
- **Max feedback latency:** ~30 seconds (quick command)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 06-01-T2 | 06-01 | 1 | D-23 | — | Geometry JSON serde-strict + clamps | unit + insta | `cargo test -p archel --features gui geometry` | ❌ W0 | ⬜ pending |
| 06-01-T3 | 06-01 | 1 | D-26 | — | Second bind fails cleanly | unit | `cargo test -p archel --features gui single_instance` | ❌ W0 | ⬜ pending |
| 06-02-T1 | 06-02 | 1 | D-13/D-14 | T-06-02 | Cancel → one-shot SIGINT + drain to EOF; exit status reported, never hard-kill | unit (fake child) | `cargo test -p archel-core privileged` | ❌ W0 (new variant) | ⬜ pending |
| 06-03-T1 | 06-03 | 2 | D-01/D-02 | — | One in-flight refresh per model; stale results dropped by sequence tag | unit | `cargo test -p archel --features gui poll` | ❌ W0 | ⬜ pending |
| 06-03-T1 | 06-03 | 2 | D-04/D-05 | — | Poll failure banner + retry cadence | unit | `cargo test -p archel --features gui poll_failures` | ❌ W0 | ⬜ pending |
| 06-03-T2 | 06-03 | 2 | D-03/D-06 | — | Timers tick while hidden/minimized; data fresh on return | unit | `cargo test -p archel --features gui system` | ❌ W0 | ⬜ pending |
| 06-03-T3 | 06-03 | 2 | D-07 | — | Landing snapshot via `landing::snapshot(false)` | unit | `cargo test -p archel-core updates` | ✅ (extend if needed) | ⬜ pending |
| 06-04-T2 | 06-04 | 3 | D-08 | — | 12 s burst scan, populate-at-end, own scan-in-flight flag | unit | `cargo test -p archel --features gui bt` | ❌ W0 | ⬜ pending |
| 06-06-T3 | 06-06 | 5 | D-19 | T-06-29 | Valider disabled until PKGBUILD expander opened (pkgbuildSeen) AND trust checked | unit (flag seam) | `cargo test -p archel --features gui aur` | ❌ W0 | ⬜ pending |
| 06-08-T3 | 06-08 | 6 | D-16 | — | Countdown keep/timeout auto-rollback; no direct set_monitor/dispatch | unit | `cargo test -p archel-core monitors` | ✅ extend | ⬜ pending |
| 06-09-T1 | 06-09 | 7 | All QML | — | N/A | lint | `qmllint $(find crates/archel/src/gui/qml -name '*.qml')` (or `just lint-qml`) | ❌ W0 (CI step) | ⬜ pending |
| 06-09-T1 | 06-09 | 7 | Shell | — | N/A | smoke | `QT_QPA_PLATFORM=offscreen timeout 10 ./target/debug/archel settings` exit 124 / no `qrc: load` errors | ❌ W0 | ⬜ pending |
| 06-09-T2 | 06-09 | 7 | PLAT-03 | T-06-01+ | GUI never concatenates command strings; no subprocess/IPC literals in gui/; every cmds command annotated | unit (audit) | `cargo test -p archel --no-default-features gui_thin_view` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/archel/tests/gui_viewmodels.rs` (or unit modules) — tick/N-A/in-flight/single-instance/geometry
- [ ] Cancel-aware test harness in `archel-core` privileged tests (fake streaming child emitting lines + responding to SIGINT)
- [ ] `just gui-check` (or shell script): QMAKE pin + qmllint sweep + offscreen smoke, wired alongside existing headless jobs

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Visual vertical-tab rail with icons + French labels | D-25 | Rendering fidelity needs eyes | Launch `archel settings` on Hyprland, screenshot rail |
| Modal modality (confirm/countdown/streaming log) feel | D-12/D-13/D-16 | UX perception | Exercise install, display apply, BT pairing on live session |
| Theme follows system style hint | D-24 | Depends on live QT_QPA_PLATFORMTHEME env | Compare app chrome against system Fusion theme |
| pkexec prompt inside confirm modal | D-18 | polkit agent interaction outside test reach | Trigger a privileged write, observe prompt timing |

*All data-behavior checks have automated verification; only visual/UX perceptions are manual.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
