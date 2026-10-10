---
phase: 04-connectivity-devices
verified: 2026-10-10T14:30:00Z
status: human_needed
score: 19/19 goal-backward checks satisfied
behavior_unverified: 0
overrides_applied: 0
covered_files:
  - .planning/phases/04-connectivity-devices/04-01-PLAN.md
  - .planning/phases/04-connectivity-devices/04-01-SUMMARY.md
  - .planning/phases/04-connectivity-devices/04-02-PLAN.md
  - .planning/phases/04-connectivity-devices/04-02-SUMMARY.md
  - .planning/phases/04-connectivity-devices/04-03-PLAN.md
  - .planning/phases/04-connectivity-devices/04-03-SUMMARY.md
  - .planning/phases/04-connectivity-devices/04-04-PLAN.md
  - .planning/phases/04-connectivity-devices/04-04-SUMMARY.md
  - .planning/phases/04-connectivity-devices/04-05-PLAN.md
  - .planning/phases/04-connectivity-devices/04-05-SUMMARY.md
  - Cargo.lock
  - crates/archel-core/Cargo.toml
  - crates/archel-core/locales/fr.json
  - crates/archel-core/src/audio.rs
  - crates/archel-core/src/bluetooth.rs
  - crates/archel-core/src/error.rs
  - crates/archel-core/src/lib.rs
  - crates/archel-core/src/network.rs
  - crates/archel-core/src/power.rs
  - crates/archel-core/src/proxy.rs
  - crates/archel/Cargo.toml
  - crates/archel/src/cmds/audio_cmds.rs
  - crates/archel/src/cmds/bt_cmds.rs
  - crates/archel/src/cmds/mod.rs
  - crates/archel/src/cmds/net_cmds.rs
  - crates/archel/src/cmds/power_cmds.rs
  - crates/archel/src/cmds/snapshots/archel__cmds__net_cmds__tests__proxy_report_json_schema_pins_apply_fields_without_secrets.snap
  - crates/archel/src/cmds/snapshots/archel__cmds__net_cmds__tests__vpn_action_report.snap
  - crates/archel/src/cmds/snapshots/archel__cmds__net_cmds__tests__vpn_import_report.snap
  - crates/archel/src/cmds/snapshots/archel__cmds__net_cmds__tests__vpn_list_json_schema_pins_connection_fields.snap
  - crates/archel/src/main.rs
covered_digest: "v2:sha256:282e3287bdcf087b764d46d163c505e3e5b5ad4c65201f0f4eee42e86a3386dc"
re_verification:
  previous_status: human_needed
  previous_score: 16/16
  previous_defect: "stale report — hand-written covered_digest (gate fails closed) and covered_files omitted the 04-05 PLAN/SUMMARY pair"
  gaps_closed: []
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Live DNS round-trip: `archel net dns set 1.1.1.1` then `net dns show`, then `net dns reset` on a scratch profile"
    expected: "set replaces profile DNS with preview + [o/N] confirm, show reports effective DNS read-back, reset restores DHCP"
    why_human: "Mutates a live NM profile and needs a polkit agent interaction; automation stops at the NeedsTty refusal gate (re-confirmed this run: piped set refuses 'annulée sans rien modifier')"
  - test: "Live VPN up/down/import: `archel net vpn up <throwaway>` then `down`; config-file import of a test .ovpn/.conf"
    expected: "up/down activate by UUID with confirm; ephemeral down preview warns about tracking loss with import hint; import delegates to nmcli plugin; plugin-missing yields VpnPluginMissing hint"
    why_human: "Mutates live tunnels (downing an ephemeral profile drops NM tracking — a real tunnel must be sacrificed); automation covers everything up to the spawn"
  - test: "Real-peer BT pair: `archel bt scan`, pick a human-owned device, `archel bt pair <pick>` with PIN [o/N] confirm, then connect/disconnect"
    expected: "PIN surfaces for explicit confirmation, pair-then-connect offer appears, no auto-accept"
    why_human: "Needs a human-owned hardware peer; pairing with ambient/unknown peers is unsafe and was deliberately not attempted (live scan confirmed working: 5+ devices with RSSI)"
  - test: "Audible audio switch: `archel audio set-output <real-name>` with TTY confirm, listen for route change; then switch back"
    expected: "Default sink switches by stable name with read-back ok line; unknown names fail with candidate list"
    why_human: "Mutates the live session audio route; only human ears confirm the effect (unknown-name typed refusal re-confirmed live this run)"
  - test: "Live governor switch: `archel power set balanced` (expect PolkitDenied hint as non-root, or profile change with privilege)"
    expected: "Typed hint without privilege; backend tag present on every output"
    why_human: "System-wide power-state effect; headless runs stop at the NeedsTty gate by design (live `power status --json` confirmed: profile + backend=cpufreq + collapse note)"
---

# Phase 04: Connectivity & Devices Verification Report

**Phase Goal:** Users can inspect and control network, Bluetooth, audio, and power from one API surface
**Verified:** 2026-10-10T14:30:00Z
**Status:** human_needed
**Re-verification:** Yes — regenerated report (prior report structurally invalid: hand-written `covered_digest`, 04-05 pair omitted from `covered_files`). No gaps were open; all five plans remain complete with SUMMARYs.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `archel net status` shows state, interface, SSID/wired, IP, gateway, effective DNS in one French table or --json (NET-01/02) | ✓ VERIFIED | This run live: `net status --json` → source=dbus, wlan0, SSID "KLEY Rennes privée", 10.192.61.76/24, gw 10.192.61.1, DNS, profile+UUID, connection_type |
| 2 | `net dns set` replaces DHCP DNS with IPv4 servers; `net dns reset` restores DHCP with read-back (NET-03, D-01/D-02/D-03) | ✓ VERIFIED | This run live: piped `dns set 1.1.1.1` printed preview (profil/ancien/nouveau) then refused "aucun terminal détecté — annulée sans rien modifier". Units: IPv4 validator, argv builder touches ipv4 keys only (grep: "ipv6" appears only in negative test assertions at network.rs:1324/1345) |
| 3 | `archel net test` shows raw per-hop ms for gateway, DNS, internet (NET-06, D-09/D-10) | ✓ VERIFIED | This run live: `net test --json` → 3 hops with raw ms (1.342 / 55.0 / 9.023), no verdict strings in output; `run_net_test` + `format_hop_ms` + parsers unit-covered |
| 4 | Every 04-01 write path drivable via CLI with --json | ✓ VERIFIED | Dns/Test actions wired in mod.rs dispatch (lines 636-644); insta schema pins; suite green (277+125) |
| 5 | Bare `archel net proxy` shows current state in one line + shell export hint (NET-04, D-04) | ✓ VERIFIED | This run live: `net proxy --json` → mode=none + ignore-hosts; unit `status_line_is_single_line_with_endpoint`; export hint non-empty French |
| 6 | `net proxy off\|manual\|auto` change session proxy mode with exceptions/auth flags | ✓ VERIFIED | This run live: piped `proxy manual proxy.test 8080` printed preview then refused without mutation; `gsettings get org.gnome.system.proxy mode` still 'none' after probes; argv-builder units pin exact gsettings sequences; validators reject bad host/port/PAC |
| 7 | VPN list/up/down/import from CLI with --json (NET-05, D-07) | ✓ VERIFIED | This run live: `net vpn list --json` → real wireguard "Maison" with persistent:true; import validator rejects missing/dir/wrong-ext//proc//sys//dev; plugin-missing → VpnPluginMissing; nmcli --ask delegation argv pinned by unit (network.rs:2461) |
| 8 | VPN/proxy secrets never in logs, snapshots, --json | ✓ VERIFIED | ProxyPlan carries password in-memory with NO Serialize derive, Debug prints `password_set` bool (proxy.rs:253-286); VPN secrets stay in `nmcli --ask`; 4 snapshot files exist and contain no secret fields (schema pins run green in suite) |
| 9 | `archel bt scan` shows nearby devices as live list with name, MAC, signal (BT-01) | ✓ VERIFIED | This run live: `bt scan --json` → 5+ device array with address+rssi; French numbered table also observed via `bt pair` sweep; units: dedupe-by-MAC, stable signal ordering |
| 10 | Pair with code shown for explicit confirmation, pick from scanned list only (BT-02, D-05/D-06) | ✓ VERIFIED | `resolve_pick` (bluetooth.rs:304) is sole path — in-list index/MAC only, else DeviceNotFound (unit at :849); confirm-only agent `request_confirmation: Some(...)` (:480), `request_default` stays false; no auto-accept constructor |
| 11 | Connect/disconnect the picked device, each confirm-gated | ✓ VERIFIED | `connect_device`/`disconnect_device` via `resolve_pick`; pair-then-connect offer reuses confirm gate with NeedsTty when piped |
| 12 | Daemon down → honest enable-hint failure, never hang/crash | ✓ VERIFIED | Every bt command probes org.bluez first → BluetoothUnavailable with enable hint; daemon active here (scan works); degrade path unit-covered |
| 13 | `archel audio list` shows stable device names for outputs and inputs (SND-01) | ✓ VERIFIED | This run live: `audio list --json` → 3 sinks with stable alsa_ names + defaults; session IDs `#[serde(skip)]` (audio.rs:46) — never in --json, never matched across runs |
| 14 | `audio set-output/set-input <name>` switches default sink/source by stable name (SND-02) | ✓ VERIFIED | This run live: unknown name → typed "introuvable : utilisez le nom exact affiché par archel audio list"; name-to-ID resolver + zero-spawn abort units; live route change routed to human item #4 |
| 15 | `archel power status` shows active profile + backend; `power set` switches performance/balanced/power-saver (PWR-01) | ✓ VERIFIED | This run live: `power status --json` → profile=power-saver, backend=cpufreq, honest collapse note in French; PPD-probe-then-cpufreq two-tier with backend tag on every output; EACCES → PolkitDenied unit (chmod-444 fixture) |
| 16 | Every 04-04 action works with --json; CLI completions cover new verbs | ✓ VERIFIED | Insta pins for audio/power schemas; `completions_cover_the_full_command_tree` + `completions_are_non_empty_and_name_archel_per_shell` pass in suite |
| 17 | `net vpn list` distinguishes persistent vs ephemeral (assumed/generated) profiles (04-05, G-04-2) | ✓ VERIFIED | Live: `--json` carries `persistent: true` for stored keyfile profile; unit `provenance_parser_classifies_empty_dash_run_and_keyfile` passes (empty/`--`//run/ → ephemeral, keyfile → stored, escaped-colon names handled) |
| 18 | `vpn down <ephemeral>` preview warns that deactivation drops NM tracking with `vpn import` hint (04-05) | ✓ VERIFIED | Unit `down_preview_warns_for_ephemeral_and_stays_quiet_for_stored` passes; `vpn_ephemeral_warning`/`down_preview_warning_rows`/`confirm_vpn_down_prompt` exist in network.rs; live down still needs human (item #2) |
| 19 | `vpn down <persistent>` deactivates with post-deactivate read-back; no delete verb anywhere (04-05) | ✓ VERIFIED | `vpn_down` routes through single `vpn_down_argv` builder (network.rs:2323); unit `down_argv_is_exactly_connection_down_uuid_with_no_delete_verb` passes (asserts no "delete" arg); read-back degrades honestly to active:false |

**Score:** 19/19 goal-backward checks satisfied (0 present-but-behavior-unverified; mutation *effects* on live hardware routed to human verification — code paths, gates, and argv fully proven by live probes + unit tests)

### Roadmap Success Criteria Mapping

| # | Success Criterion | Covering truths | Status |
|---|-------------------|-----------------|--------|
| 1 | Connection state, IP/gateway, DNS per NM profile, VPN management, self-tests | 1,2,3,4,7,8,17,18,19 | ✓ |
| 2 | Session-layer proxy independent of NM profiles | 5,6,8 | ✓ |
| 3 | BT scan/pair/connect/disconnect with working pairing flow | 9,10,11,12 | ✓ (live pairing flow → human item #3) |
| 4 | Audio output/input selection + power profile switching | 13,14,15,16 | ✓ (audible/governor effect → human items #4,#5) |
| 5 | Every action through archel-cli proving write paths before GUI | 4,16 + live gate probes | ✓ |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/archel-core/src/network.rs` | NM status/DNS/test/VPN core + 04-05 provenance | ✓ VERIFIED | 2755 lines, substantive, wired to net_cmds |
| `crates/archel-core/src/proxy.rs` | gsettings proxy store | ✓ VERIFIED | 894 lines, substantive, wired |
| `crates/archel-core/src/bluetooth.rs` | bluer scan/pair/connect, confirm-only agent | ✓ VERIFIED | 905 lines, substantive, wired to bt_cmds |
| `crates/archel-core/src/audio.rs` | wpctl parse + name-keyed set | ✓ VERIFIED | 712 lines, substantive, wired |
| `crates/archel-core/src/power.rs` | PPD/cpufreq two-tier + backend tag | ✓ VERIFIED | 591 lines, substantive, wired |
| `crates/archel/src/cmds/net_cmds.rs` | net CLI group incl. 04-05 down preview | ✓ VERIFIED | 884 lines; dispatch verified in mod.rs:636-659 |
| `crates/archel/src/cmds/bt_cmds.rs` | bt CLI group | ✓ VERIFIED | 287 lines, live scan + sweep observed |
| `crates/archel/src/cmds/audio_cmds.rs` | audio CLI group | ✓ VERIFIED | 246 lines, live list + typed unknown-name refusal |
| `crates/archel/src/cmds/power_cmds.rs` | power CLI group | ✓ VERIFIED | 165 lines, live status |
| 4 insta snapshots (proxy/vpn schemas) | secret-free schema pins | ✓ VERIFIED | All 4 files exist, pin tests green |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| net CLI group | archel-core::network | mod.rs dispatch → core fns | WIRED | live source=dbus in status output |
| net dns set/reset | NM profile update | preview → confirm gate → reactivation read-back | WIRED | live refusal, no mutation |
| net test | ping/dig subprocesses | exact argv, raw ms | WIRED | live 3-hop ms output |
| net proxy CLI | gsettings org.gnome.system.proxy | argv builders | WIRED | mode stayed 'none' after gated probe |
| vpn down | nmcli connection down UUID | single `vpn_down_argv` builder | WIRED | no-delete-verb unit passes |
| bt pair | bluer confirm-only agent | `request_confirmation: Some(...)` | WIRED | agent constructed at bluetooth.rs:479 |
| audio set | name-to-current-ID → wpctl set-default | resolver + gate | WIRED | live typed refusal |
| power status/set | PPD probe → cpufreq fallback | backend tag | WIRED | live backend=cpufreq |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| net status --json | NetStatus | NM D-Bus (source tag "dbus") | Yes — live SSID/IP/gw/DNS | ✓ FLOWING |
| net vpn list --json | Vec\<VpnConnection\> | nmcli NAME,UUID,TYPE,FILENAME query | Yes — real wg profile + persistent flag | ✓ FLOWING |
| audio list --json | sinks/sources | wpctl status parse (Audio-tree gated) | Yes — 3 real alsa_ sinks | ✓ FLOWING |
| power status --json | PowerStatus | governor sysfs / PPD | Yes — power-saver + cpufreq | ✓ FLOWING |
| net test --json | Vec\<HopResult\> | ping/dig subprocess output | Yes — 3 real ms figures | ✓ FLOWING |
| bt scan --json | Vec\<BtDevice\> | bluer discovery stream | Yes — 5+ live peers with RSSI | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full core suite | `cargo test --offline -p archel-core --no-default-features` | 277 passed, 0 failed, 1 ignored | ✓ PASS |
| Full CLI suite | `cargo test --offline -p archel --no-default-features` | 125 passed, 0 failed | ✓ PASS |
| 04-05 G-04-2 regressions | `cargo test ... vpn_provenance_tests` | 3/3 passed (provenance parser, no-delete argv, ephemeral preview warn) | ✓ PASS |
| Live read paths | `net status/proxy/vpn list/test --json`, `audio list`, `power status`, `bt scan` | all real data (see table above) | ✓ PASS |
| Confirm gates no-mutation | piped `dns set`, piped `proxy manual` | refused "sans rien modifier"; gsettings mode still 'none' | ✓ PASS |
| Typed unknown-device refusal | `audio set-output ghost.device-name` | French typed error pointing to `audio list` | ✓ PASS |

### Probe Execution

No probe scripts declared by the phase PLAN/SUMMARY files (`scripts/*/tests/probe-*.sh` not used in this phase). Step 7c: N/A — replaced by live CLI spot-checks above.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| NET-01 | 04-01 | Connection state, interface, SSID/wired | ✓ SATISFIED | truth 1 (live) |
| NET-02 | 04-01 | IP address and gateway | ✓ SATISFIED | truth 1 (live) |
| NET-03 | 04-01 | DNS per NM profile | ✓ SATISFIED | truth 2 (gate live, apply → human #1) |
| NET-04 | 04-02 | Session-layer proxy | ✓ SATISFIED | truths 5-6 |
| NET-05 | 04-02, 04-05 | VPN list/connect/disconnect | ✓ SATISFIED | truths 7,17-19 (live toggle → human #2) |
| NET-06 | 04-01 | Network self-tests | ✓ SATISFIED | truth 3 (live) |
| BT-01 | 04-03 | BT scan/list | ✓ SATISFIED | truth 9 (live) |
| BT-02 | 04-03 | Pair/connect/disconnect | ✓ SATISFIED | truths 10-12 (live peer → human #3) |
| SND-01 | 04-04 | Audio output selection | ✓ SATISFIED | truths 13-14 (ears → human #4) |
| SND-02 | 04-04 | Audio input selection | ✓ SATISFIED | truth 14 |
| PWR-01 | 04-04 | Power profile switching | ✓ SATISFIED | truth 15 (privilege → human #5) |

All 11 roadmap requirements for Phase 4 are declared in plans; no orphaned requirements.

### Anti-Patterns Found

None. Debt-marker scan (TBD/FIXME/XXX) across all phase implementation files: clean. `PLACEHOLDER` grep hits in network.rs are a legitimate NUL-const for escaped-colon masking in nmcli parsers, not stubs. No empty returns, hardcoded empty data, or console-only implementations found in modified files.

### Prohibitions Check (ADR-550 D3 tier)

| Prohibition | Verification | Verdict |
|-------------|--------------|---------|
| No auto-verdict strings in net test output | judgment | ✓ resolved — live --json contains only hop/target/ms |
| No IPv6 DNS keys touched | judgment | ✓ resolved — "ipv6" appears only in negative test assertions |
| No secrets in argv/logs/snapshots/--json | judgment | ✓ resolved — ProxyPlan no-Serialize, nmcli --ask, secret-free snapshots |
| No .ovpn parsing in Rust | judgment | ✓ resolved — import delegates to nmcli plugins |
| No auto-accept pairing | judgment | ✓ resolved — confirm-only agent, request_default false |
| No direct-MAC connect bypassing scan-then-pick | judgment | ✓ resolved — resolve_pick is sole path |
| No numeric-ID audio matching across runs | judgment | ✓ resolved — IDs `#[serde(skip)]`, names only |
| No delete verb in VPN down path | judgment | ✓ resolved — no-delete unit passes |

### Human Verification Required

5 items — all live mutation *effects* that automation deliberately refuses (gates re-confirmed working this run). Identical to the previous report's set plus the ephemeral-VPN-down case folded into item #2:

1. **Live DNS round-trip** — `net dns set 1.1.1.1` → `dns show` → `dns reset` on a scratch profile (polkit agent + TTY confirm needed)
2. **Live VPN up/down/import** — real tunnel activation, ephemeral-down tracking-loss warning observed in practice, .ovpn import via nmcli plugin
3. **Real-peer BT pair** — PIN confirm flow + pair-then-connect against a human-owned device
4. **Audible audio switch** — default-sink switch confirmed by ear with read-back ok line
5. **Live governor switch** — `power set balanced` effect with privilege; PolkitDenied hint without

### Gaps Summary

No gaps. All 19 goal-backward checks satisfied by automated evidence (two green test suites, 3/3 named 04-05 regression tests) plus live read-path smoke tests and no-mutation gate probes against the running system. The only outstanding items are live-session hardware/privilege/ear effects by design. The prior report's structural defects (hand-written digest, missing 04-05 pair) are corrected: digest regenerated via `gsd-tools query verification fingerprint`, all 10 plan/summary artifacts and all implementation files from the plans' `files_modified` lists are covered.

---

_Verified: 2026-10-10T14:30:00Z_
_Verifier: the agent (gsd-verifier)_
