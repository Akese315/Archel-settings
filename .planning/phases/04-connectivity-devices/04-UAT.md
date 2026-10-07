---
status: testing
phase: 04-connectivity-devices
source: [04-VERIFICATION.md]
started: 2026-10-07T00:00:00Z
updated: 2026-10-07T00:00:00Z
---

## Current Test

number: 1
name: Live DNS round-trip
expected: |
  `archel net dns set 1.1.1.1` on a scratch profile shows old-to-new preview + [o/N] confirm, `net dns show` reports effective DNS read-back, `net dns reset` restores DHCP.
awaiting: user response

## Tests

### 1. Live DNS round-trip
expected: set replaces profile DNS with preview + [o/N] confirm, show reports effective DNS read-back, reset restores DHCP
result: [pending]

### 2. Live VPN connect + import
expected: up/down activate by UUID with confirm; import delegates to nmcli plugin; plugin-missing yields VpnPluginMissing hint
result: [pending]

### 3. Real-peer BT pair
expected: PIN surfaces for explicit confirmation, pair-then-connect offer appears, no auto-accept
result: [pending]

### 4. Audible audio switch
expected: default sink switches by stable name with read-back ok line; unknown names fail with candidate list
result: [pending]

### 5. Live governor switch
expected: typed hint without privilege; backend tag present on every output
result: [pending]

## Summary

total: 5
passed: 0
issues: 0
pending: 5
skipped: 0
blocked: 0

## Gaps
