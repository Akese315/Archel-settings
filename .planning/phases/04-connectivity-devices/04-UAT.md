---
status: complete
phase: 04-connectivity-devices
source: [04-VERIFICATION.md]
started: 2026-10-07T00:00:00Z
updated: 2026-10-07T00:00:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Live DNS round-trip
expected: set replaces profile DNS with preview + [o/N] confirm, show reports effective DNS read-back, reset restores DHCP
result: pass

### 2. Live VPN connect + import
expected: up/down activate by UUID with confirm; import delegates to nmcli plugin; plugin-missing yields VpnPluginMissing hint
result: issue
reported: "vpn down wg0 succeeded but deleted the NM profile instead of deactivating it — nmcli connection show no longer lists wg0, vpn list shows empty, and vpn up fails for both name and UUID"
severity: blocker

### 3. Real-peer BT pair
expected: PIN surfaces for explicit confirmation, pair-then-connect offer appears, no auto-accept
result: pass
note: "MAC pick accepted after name pick correctly rejected; pair [o/N] then pair-then-connect [o/N] offer both shown; WH-1000XM4 paired and connected"

### 4. Audible audio switch
expected: default sink switches by stable name with read-back ok line; unknown names fail with candidate list
result: pass
note: "list shows 4 sinks + 2 sources with stable names; set-output to HDMI then WH-1000XM4 both confirmed with read-back; unknown name cleanly rejected with hint"

### 5. Live governor switch
expected: typed hint without privilege; backend tag present on every output
result: pass
note: "status shows power-saver/cpufreq + collapse note; set balanced as non-root yields typed PolkitDenied hint with hyprpolkitagent guidance, no bypass"

## Summary

total: 5
passed: 4
issues: 1
pending: 0
skipped: 0
blocked: 0

## Gaps

- gap_id: G-04-2
  truth: "vpn down deactivates the tunnel, profile remains configured and re-activatable via vpn up"
  status: failed
  reason: "User reported: vpn down wg0 dropped NM tracking (vpn list empty, vpn up fails for name and UUID). Diagnosis: wg0 was an ephemeral NM-assumed profile for an externally-created WireGuard iface (wg-quick, managed-type external, no keyfile, journal shows only connection-deactivate success, zero connection-delete). No delete path exists in codebase (vpn_down builds connection/down/<uuid>). True defect: vpn list hides provenance and vpn down warns nothing about ephemeral profiles."
  severity: blocker
  test: 2
  artifacts: ["crates/archel-core/src/network.rs (parse_vpn_connections, vpn_list, vpn_down)", "crates/archel/src/cmds/net_cmds.rs (run_vpn_down)"]
  missing: ["FILENAME provenance column in vpn list query + VpnConnection persistence flag", "ephemeral-profile warning in down confirm preview", "post-deactivate read-back instead of hardcoded active:false", "vpn_down routed through vpn_down_argv single builder + regression tests"]
