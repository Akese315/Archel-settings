---
phase: "2"
slug: "system-insight"
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-01"
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (built-in) + `insta` for `--json` snapshots (first use — Wave 0 adds dev-dep) |
| **Config file** | none — Wave 0 creates core modules + CLI wiring |
| **Quick run command** | `cargo test -p archel-core --no-default-features` |
| **Full suite command** | `cargo test --workspace --no-default-features` |
| **Estimated runtime** | ~60 seconds (includes ~1 s CPU sampling tests) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p archel-core --no-default-features`
- **After every plan wave:** Run `cargo test --workspace --no-default-features`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 2-01-01 | 01 | 1 | LAND-01, LAND-02 | — | N/A (read-only version/kernel) | unit | `cargo test -p archel-core landing` | ❌ W0 | ⬜ pending |
| 2-01-02 | 01 | 1 | LAND-03 | T-2-01 | Corrupt cache → recompute, never crash | unit (fixture clock) | `cargo test -p archel-core updates` | ❌ W0 | ⬜ pending |
| 2-02-01 | 02 | 2 | SYS-01..04 | — | N/A (world-readable /proc, sysinfo) | unit (live range asserts) | `cargo test -p archel-core system` | ❌ W0 | ⬜ pending |
| 2-03-01 | 03 | 3 | SYS-05..08 | — | NVML dlopen failure → typed N/A, not crash | unit (live NVIDIA + fixtures) | `cargo test -p archel-core sensors` | ❌ W0 | ⬜ pending |
| 2-03-02 | 03 | 3 | LAND-01..03, SYS-01..08 | — | Tables render catalog French only | integration/snapshot | `cargo test -p archel` (insta --json) | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/archel-core/src/{system,sensors,updates,landing,cache}.rs` (+ unit tests per map above)
- [ ] CLI `sys` group + `landing` subcommand + global `--json` + `--watch`/`--refresh` wiring
- [ ] `insta` dev-dependency (first snapshot use for `--json` schemas)
- [ ] `pacman-contrib` installed on the machine (human step — `checkupdates` missing; LAND-03 degrades to missing-checker state without it)

*Framework install: none beyond cargo itself + new crate deps (sysinfo, nvml-wrapper, directories) declared in plan.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `--watch` live re-render at 500 ms | D-02 | Terminal rendering can't be unit-asserted | `archel sys cpu --watch`, confirm re-render + Ctrl+C exits cleanly |
| Missing-`checkupdates` graceful state | LAND-03 | Depends on machine package state | `archel landing` without pacman-contrib → guided French hint, no blank/crash |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
