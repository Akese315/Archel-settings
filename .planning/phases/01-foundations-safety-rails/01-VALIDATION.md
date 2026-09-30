---
phase: "1"
slug: "foundations-safety-rails"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-09-30"
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (built-in) — no extra harness for Phase 1 |
| **Config file** | none — Wave 0 creates workspace + rust-toolchain.toml |
| **Quick run command** | `cargo test --workspace --no-default-features` |
| **Full suite command** | `QMAKE=/usr/bin/qmake6 cargo test --workspace` |
| **Estimated runtime** | ~120 seconds (post-Qt build; headless quick run ~30s) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --workspace --no-default-features`
- **After every plan wave:** Run `QMAKE=/usr/bin/qmake6 cargo test --workspace`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 120 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 1-01-01 | 01 | 0 | PLAT-01 | — | N/A (toolchain + workspace skeleton) | build | `cargo build --workspace` | ❌ W0 | ⬜ pending |
| 1-01-02 | 01 | 1 | PLAT-01 | — | Typed errors, no panics | unit | `cargo test -p archel-core error::tests` | ❌ W0 | ⬜ pending |
| 1-01-03 | 01 | 1 | PLAT-01 | — | No hardcoded French outside locales/ | unit/lint | `cargo test -p archel-core i18n_audit` (diacritics grep test) | ❌ W0 | ⬜ pending |
| 1-02-01 | 02 | 1 | PLAT-04 | T-1-01 | Allowlist: dry-run → v1 JSON exit 0; else exit 2 + error JSON | unit + integration | `cargo test -p archel-privileged` (argv matrix, no root) | ❌ W0 | ⬜ pending |
| 1-02-02 | 02 | 1 | PLAT-04 | T-1-02 | pkexec 126/127 → AuthFailed naming action (D-12) | unit (mocked exit codes) | `cargo test -p archel-core privileged::tests` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `rust-toolchain.toml` + workspace `Cargo.toml` — toolchain pin + members (blocks everything; bootstrap via `sudo pacman -S rustup && rustup toolchain install stable`)
- [ ] `crates/archel-core/src/{lib,error,i18n,privileged}.rs` + `locales/fr.json`
- [ ] `crates/archel/src/main.rs` (clap root) + `gui/` spike module + `qml/Spike.qml`
- [ ] `crates/archel-privileged/src/main.rs` (allowlist + v1 JSON)
- [ ] `policy/org.archel.privileged.policy` draft

*Framework install: `sudo pacman -S rustup && rustup toolchain install stable` — no test framework beyond cargo itself.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Spike window: vertical tabs + 1 Hz live label (D-14) | Spike bar | GUI rendering can't be unit-asserted | `QMAKE=/usr/bin/qmake6 cargo run --features gui -- settings` + visual confirm (screenshot/run log as evidence) |
| pkexec dry-run JSON capture | PLAT-04 | Requires interactive polkit auth | `pkexec archel-privileged dry-run` → versioned JSON exit 0; screenshot/log capture |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
