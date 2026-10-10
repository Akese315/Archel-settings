---
phase: "4"
slug: "connectivity-devices"
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-06"
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (built-in) + `insta` snapshots (already dev-dep) |
| **Config file** | none — Wave 0 creates network/proxy/bluetooth/audio/power modules + CLI groups |
| **Quick run command** | `cargo test -p archel-core --no-default-features` |
| **Full suite command** | `cargo test --workspace --no-default-features` |
| **Estimated runtime** | ~90 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p archel-core --no-default-features`
- **After every plan wave:** Run `cargo test --workspace --no-default-features`
- **Before `/gsd-verify-work`:** Full suite must be green + live CLI proof of every write path
- **Max feedback latency:** 90 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 4-01-01 | 01 | 1 | NET-01, NET-02 | — | N/A (read-only state parse) | unit (mock payloads) | `cargo test -p archel-core network` | ❌ W0 | ⬜ pending |
| 4-01-02 | 01 | 1 | NET-03, NET-06 | T-4-01 | DNS key rejects IPv6/hostnames; hop outputs parsed, never shelled | unit | `cargo test -p archel-core network::dns` | ❌ W0 | ⬜ pending |
| 4-02-01 | 02 | 2 | NET-04, NET-05 | T-4-02 | Secrets redacted in snapshots/logs; import validates path | unit + insta | `cargo test -p archel-core proxy` | ❌ W0 | ⬜ pending |
| 4-02-02 | 02 | 2 | NET-05 | T-4-04 | VPN secrets never in CLI args/logs; path-validated import | unit | `cargo test -p archel-core network::vpn` | ❌ W0 | ⬜ pending |
| 4-03-01 | 03 | 3 | BT-01, BT-02 | T-4-03 | PIN shown, explicit confirm; no auto-accept path exists | unit (mock events) | `cargo test -p archel-core bluetooth` | ❌ W0 | ⬜ pending |
| 4-03-02 | 03 | 3 | BT-02 | T-4-05, T-4-06 | No rogue auto-pair; PIN displayed only for user confirm | unit | `cargo test -p archel-core bluetooth` | ❌ W0 | ⬜ pending |
| 4-04-01 | 04 | 4 | SND-01, SND-02, PWR-01 | T-4-07, T-4-08 | name→ID resolve (no stale IDs); governor map + backend tag | unit + insta | `cargo test -p archel-core audio` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/archel-core/src/{network,proxy,bluetooth,audio,power}.rs` with parser unit tests (FR-locale fixtures)
- [ ] Live-bus spike results: NM proxy names + auth-error string + bluer agent signature + `nmcli import` dry-probe
- [ ] FR-locale `wpctl`/`nmcli` fixture files under testdata/
- [ ] `bluetooth.service` enabled on target (human step — daemon inactive; scan/pair need it)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| DNS set/reset live round-trip | NET-03 | Mutates live profile, needs polkit agent | `archel net dns …` on target, verify + reset |
| Proxy mode transitions | NET-04 | Mutates session gsettings | Cycle off/manual/auto, verify each |
| Live BT scan/pair | BT-01/02 | Needs daemon + hardware peer | `archel bt scan`, pair a real device, confirm PIN flow |
| Live audio switch + power switch | SND/PWR-01 | Audible/system-wide effects | Switch sink/source, cycle governor profiles |
| VPN connect + import | NET-05 | Real tunnel + plugin may be missing | Import .ovpn/.conf, connect, verify; OpenVPN-missing hint if absent |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
