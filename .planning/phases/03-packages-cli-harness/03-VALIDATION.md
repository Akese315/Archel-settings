---
phase: "3"
slug: "packages-cli-harness"
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-01"
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (built-in) + `insta` snapshot pins (Phase 2 precedent) |
| **Config file** | none — Wave 0 creates packages/aur modules + pkg CLI wiring |
| **Quick run command** | `cargo test -p archel-core -p archel --no-default-features` |
| **Full suite command** | `cargo test --workspace --no-default-features` |
| **Estimated runtime** | ~90 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p archel-core -p archel -p archel-privileged --no-default-features`
- **After every plan wave:** Run full headless suite + `rg '"-Sy"' crates/` negative gate + diacritics audit
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 90 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 3-01-01 | 01 | 1 | PKG-01 | — | Parse under LC_ALL=C only | unit | `cargo test -p archel-core packages` | ❌ W0 | ⬜ pending |
| 3-02-01 | 02 | 2 | PKG-02, PKG-03 | T-3-01 | Validator rejects `-foo`/`--overwrite`/empty | unit (matrix) | `cargo test -p archel-privileged` | ❌ W0 | ⬜ pending |
| 3-02-02 | 02 | 2 | PKG-04 | T-3-02 | No `-Sy` constructor exists (grep gate) | unit + grep | `cargo test -p archel-core` + `rg '"-Sy"'` | ❌ W0 | ⬜ pending |
| 3-03-01 | 03 | 3 | PKG-05 | T-3-03 | Abort path never builds; `bash -n` gate | unit (abort) | `cargo test -p archel-core aur` | ❌ W0 | ⬜ pending |
| 3-03-02 | 03 | 3 | PLAT-02 | — | Completions non-empty per shell, contain `archel` | unit + snapshot | `cargo test -p archel` (insta) | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/archel-core/src/packages.rs` + `aur.rs` (or same file) with unit tests
- [ ] `crates/archel/src/cmds/pkg_cmds.rs` (TTY-gate, `--json` framing, completions)
- [ ] Helper argv validator matrix in `archel-privileged` (3 verbs + injection cases)
- [ ] `insta` pins for new `--json` schemas (card, search, list)
- [ ] `cargo add -p archel-core alpm --features checkver && cargo add -p archel clap_complete`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Live install of a trivial package streams + exits 0 | PKG-02 | Real mutation on dev machine | `archel pkg install <tiny>` with confirmation, observe live stream |
| Live remove + full `-Syu` upgrade | PKG-03, PKG-04 | Real mutations | Remove trivial pkg; run upgrade; confirm refusal of partial `-Sy` |
| Real AUR review then abort (no build) | PKG-05 | Network + human judgment | Fetch real PKGBUILD, display, abort — confirm nothing builds |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
