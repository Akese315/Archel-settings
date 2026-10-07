---
phase: 04-connectivity-devices
verified: 2026-10-07T00:00:00Z
status: human_needed
score: 16/16 must-haves verified
covered_files:
  - .planning/phases/04-connectivity-devices/04-01-PLAN.md
  - .planning/phases/04-connectivity-devices/04-02-PLAN.md
  - .planning/phases/04-connectivity-devices/04-03-PLAN.md
  - .planning/phases/04-connectivity-devices/04-04-PLAN.md
  - .planning/phases/04-connectivity-devices/04-01-SUMMARY.md
  - .planning/phases/04-connectivity-devices/04-02-SUMMARY.md
  - .planning/phases/04-connectivity-devices/04-03-SUMMARY.md
  - .planning/phases/04-connectivity-devices/04-04-SUMMARY.md
  - crates/archel-core/src/network.rs
  - crates/archel-core/src/proxy.rs
  - crates/archel-core/src/bluetooth.rs
  - crates/archel-core/src/audio.rs
  - crates/archel-core/src/power.rs
  - crates/archel/src/cmds/net_cmds.rs
  - crates/archel/src/cmds/bt_cmds.rs
  - crates/archel/src/cmds/audio_cmds.rs
  - crates/archel/src/cmds/power_cmds.rs
  - crates/archel-core/src/error.rs
  - crates/archel-core/src/lib.rs
  - crates/archel-core/locales/fr.json
covered_digest: "manual-sha256 (gsd_run unavailable in subagent context; files hashed via sha256sum at verify time)"
behavior_unverified: 0
overrides_applied: 0
human_verification:
  - test: "Live DNS round-trip: `archel net dns set 1.1.1.1` then `net dns show`, then `net dns reset` on a scratch profile"
    expected: "set replaces profile DNS with preview + [o/N] confirm, show reports effective DNS read-back, reset restores DHCP"
    why_human: "Mutates a live NM profile and needs a polkit agent interaction; automation stops at the NeedsTty refusal gate"
  - test: "Live VPN connect: `archel net vpn up <throwaway-connection>` then `down`; config-file import of a test .ovpn/.conf"
    expected: "up/down activate by UUID with confirm; import delegates to nmcli plugin; plugin-missing yields VpnPluginMissing hint"
    why_human: "Mutates live tunnels, needs a real tunnel + openvpn plugin; automation covers everything up to the spawn"
  - test: "Real-peer BT pair: `archel bt scan`, pick a human-owned device, `archel bt pair <pick>` with PIN [o/N] confirm, then connect/disconnect"
    expected: "PIN surfaces for explicit confirmation, pair-then-connect offer appears, no auto-accept"
    why_human: "Needs a human-owned hardware peer; pairing with ambient/unknown peers is unsafe and was deliberately not attempted"
  - test: "Audible audio switch: `archel audio set-output <real-name>` with TTY confirm, listen for route change; then switch back"
    expected: "Default sink switches by stable name with read-back ok line; unknown names fail with candidate list"
    why_human: "Mutates the live session audio route; only human ears confirm the effect"
  - test: "Live governor switch: `archel power set balanced` (expect PolkitDenied hint as non-root, or profile change with privilege)"
    expected: "Typed hint without privilege; backend tag present on every output"
    why_human: "System-wide power-state effect; headless runs stop at the NeedsTty gate by design"
---

# Phase 04: Connectivity & Devices Verification Report

**Phase Goal:** Users can inspect and control network, Bluetooth, audio, and power from one API surface.
**Verified:** 2026-10-07
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `archel net status` shows state, interface, SSID/wired, IP, gateway, effective DNS in one French table or --json (NET-01/02) | ✓ VERIFIED | Live: `net status --json` parses (source=dbus, wlan0, SSID, 10.192.61.76/24, gw, DNS, profile+UUID); unit `status_rows_render_na_for_absent_fields` + insta schema pin |
| 2 | `net dns set` replaces DHCP DNS with IPv4 servers; `net dns reset` restores DHCP, with read-back (NET-03, D-01/D-02/D-03) | ✓ VERIFIED | Units: validator accepts 1.1.1.1/9.9.9.9, rejects ::1/hostnames/999.1.1.1/leading-zero; argv replace touches ipv4 keys only (grep: no "ipv6" in argv builders, only in test assertions); preview+confirm+reactivation code present; live `dns show --json` parses, piped set refuses NeedsTty with no mutation |
| 3 | `archel net test` shows raw per-hop ms for gateway, DNS, internet (NET-06, D-09/D-10) | ✓ VERIFIED | Live: `net test --json` → 3 hops with ms figures (12.315/57.0/16.082); parsers unit-covered over C+FR fixtures; unreachable → N/A/null; no verdict strings in output paths (grep clean — only comments/tests mention the prohibition) |
| 4 | Every 04-01 write path drivable via CLI with --json | ✓ VERIFIED | Dns/Test actions + --json in net_cmds.rs; insta pins for dns report + hop schemas; suite green |
| 5 | Bare `archel net proxy` shows current state in one line + shell export hint (NET-04, D-04) | ✓ VERIFIED | Live: `net proxy --json` parses (mode=none + ignore-hosts); unit `status_line_is_single_line_with_endpoint`; export hint non-empty French |
| 6 | `net proxy off\|manual\|auto` change session proxy mode with exceptions/auth flags | ✓ VERIFIED | Units: argv builder maps modes to exact gsettings sequences; validators reject bad host/port/PAC; piped manual refuses NeedsTty with no mutation; gsettings mode still 'none' after probes |
| 7 | VPN list/up/down/import from CLI with --json (NET-05, D-07) | ✓ VERIFIED | Live: `net vpn list --json` → real wg0 wireguard actif; units: import validator rejects missing/dir/wrong-ext//proc//sys//dev; plugin-missing → VpnPluginMissing; empty list → honest `[]` |
| 8 | VPN/proxy secrets never in logs, snapshots, --json | ✓ VERIFIED | No Serialize on ProxyPlan (manual Debug shows password_set bool); VPN secrets stay in `nmcli --ask`; grep over snapshots for password/secret/passkey: zero hits; insta review clean |
| 9 | `archel bt scan` shows nearby devices as live list with name, MAC, signal (BT-01) | ✓ VERIFIED | Live: `bt scan --json` → 5-device array with address+rssi; units: dedupe-by-MAC, stable signal ordering, honest empty; numbered French table |
| 10 | Pair with code shown for explicit confirmation, pick from scanned list only (BT-02, D-05/D-06) | ✓ VERIFIED | Units: pick validator accepts in-list index/MAC, rejects free-form MAC; pair abort spawns nothing → ConfirmationAborted; agent is confirm-only (`request_confirmation` set, rest default=None=reject, `request_default:false`); grep: no `request_default:true`, no standalone auto-accept constructor (the one `Agent::default()` hit is the struct-update spread for the confirm-only agent) |
| 11 | Connect/disconnect the picked device, each confirm-gated | ✓ VERIFIED | `connect_device`/`disconnect_device` via `resolve_pick` (sole path); pair-then-connect offer reuses confirm_action with NeedsTty when piped |
| 12 | Daemon down → honest enable-hint failure, never hang/crash | ✓ VERIFIED | Every bt command probes org.bluez bus name first → BluetoothUnavailable with enable hint; daemon confirmed active here (hci0 owned), degrade path unit-covered |
| 13 | `archel audio list` shows stable device names for outputs and inputs (SND-01) | ✓ VERIFIED | Live: `audio list --json` → 3 sinks + 2 sources with stable alsa_ names + defaults; units: Audio-tree-gated wpctl parse skips Video loopback + garbage lines; IDs `#[serde(skip)]` (never in --json, never matched across runs) |
| 14 | `audio set-output/set-input <name>` switches default sink/source by stable name (SND-02) | ✓ VERIFIED | Units: name-to-ID resolver tracks names across renumbered fixtures; unknown → AudioDeviceNotFound with candidates; abort spawns nothing; live ghost-name refuses typed, piped real-name refuses NeedsTty |
| 15 | `archel power status` shows active profile + backend; `power set` switches performance/balanced/power-saver (PWR-01) | ✓ VERIFIED | Live: `power status --json` → profile power-saver, backend cpufreq + collapse note; units: governor map + note, EACCES → PolkitDenied (chmod-444 fixture test); PPD probe-then-cpufreq two-tier with backend tag on every output |
| 16 | Every 04-04 action works with --json; CLI completions cover new verbs | ✓ VERIFIED | Insta pins for audio list + power status schemas; completions word-list test covers audio/power/set-output/set-input verbs; suite green |

**Score:** 16/16 truths verified (0 present-but-behavior-unverified; mutation *effects* routed to human verification below, code paths fully proven)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/archel-core/src/network.rs` | NM status/DNS/test/VPN core | ✓ VERIFIED | 128 fns, substantive, wired to net_cmds |
| `crates/archel-core/src/proxy.rs` | gsettings proxy store | ✓ VERIFIED | 42 fns, substantive, wired |
| `crates/archel-core/src/bluetooth.rs` | bluer scan/pair/connect | ✓ VERIFIED | 40 fns, substantive, wired to bt_cmds |
| `crates/archel-core/src/audio.rs` | wpctl parse + name-keyed set | ✓ VERIFIED | 36 fns, substantive, wired |
| `crates/archel-core/src/power.rs` | PPD/cpufreq two-tier | ✓ VERIFIED | 35 fns, substantive, wired |
| `crates/archel/src/cmds/net_cmds.rs` | net CLI group | ✓ VERIFIED | status/dns/test/proxy/vpn runners, --json, confirm gates (18 NeedsTty/confirm hits across 4 cmd files) |
| `crates/archel/src/cmds/bt_cmds.rs` | bt CLI group | ✓ VERIFIED | scan/pair/connect/disconnect runners |
| `crates/archel/src/cmds/audio_cmds.rs` | audio CLI group | ✓ VERIFIED | list/set-output/set-input + insta pins |
| `crates/archel/src/cmds/power_cmds.rs` | power CLI group | ✓ VERIFIED | status/set + insta pins |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| net status CLI | archel-core::network | NM D-Bus or nmcli fallback | WIRED | live source=dbus |
| net dns set/reset | NM profile update | reactivation read-back of effective DNS | WIRED | plan-preview-confirm-apply + `connection up` + read-back code |
| net test | ping/dig subprocesses | raw ms, never shelled | WIRED | exact argv, LC_ALL=C, timeout+kill |
| net proxy CLI | archel-core::proxy | gsettings org.gnome.system.proxy | WIRED | live mode=none read |
| net vpn import | path validation | nmcli connection import (plugin) | WIRED | confinement + delegation, no .ovpn/.conf parsing in Rust |
| vpn up/down | NM activation | confirm-then-execute, secret redaction | WIRED | UUID-resolved pre-confirm, --ask delegation |
| bt scan CLI | archel-core::bluetooth | bluer discovery stream | WIRED | live 5-device sweep |
| bt pair | in-process agent PIN | explicit [o/N] confirm | WIRED | confirm-only agent, no auto-accept |
| daemon-absent probe | BluetoothUnavailable | enable-hint on every bt command | WIRED | bus-name probe first |
| audio list CLI | wpctl status/inspect parse | stable node.name rows | WIRED | live 3+2 endpoints |
| audio set | name-to-current-ID resolve | wpctl set-default + read-back | WIRED | resolve at switch time, read-back before ok |
| power status/set | PPD probe or cpufreq fallback | backend tag | WIRED | live backend=cpufreq + note |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| net status | state/interface/SSID/IP/gateway/DNS | live D-Bus (source=dbus) | ✓ live wlan0 data | ✓ FLOWING |
| net test | per-hop ms | live ping/dig | ✓ 12.315/57.0/16.082 ms | ✓ FLOWING |
| net vpn list | tunnels | live nmcli terse | ✓ real wg0 row | ✓ FLOWING |
| net proxy | mode/hosts | live gsettings | ✓ mode=none | ✓ FLOWING |
| bt scan | devices | live BlueZ sweep | ✓ 5-device array | ✓ FLOWING |
| audio list | sinks/sources | live wpctl | ✓ 3+2 stable names | ✓ FLOWING |
| power status | profile/backend | live cpufreq sysfs | ✓ power-saver/cpufreq | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| workspace suite green | `cargo test --workspace --no-default-features` | 183 + 83 + 18 passed, 0 failed | ✓ PASS |
| net/audio/power/bt --json parse | 7 live CLI invocations above | all parse, rc=0 | ✓ PASS |
| refusal gates leave no mutation | piped dns-set / proxy-manual / audio-set / power-set / vpn-up | NeedsTty refusal, state unchanged (gsettings mode still none, governor still powersave) | ✓ PASS |
| prohibitions hold | greps: secrets-in-snapshots (0), auto-accept (confirm-only), ipv6-in-dns-argv (test-only), verdict-strings (comments-only) | clean | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| NET-01 | 04-01 | connection state/iface/SSID | ✓ SATISFIED | live status table+JSON |
| NET-02 | 04-01 | IP + gateway | ✓ SATISFIED | live 10.192.61.76/24 + gw |
| NET-03 | 04-01 | per-profile DNS | ✓ SATISFIED | validator+argv+confirm+read-back units; live round-trip → human |
| NET-04 | 04-02 | session proxy | ✓ SATISFIED | 3-mode + one-liner; live cycling → human |
| NET-05 | 04-02 | VPN manage + import | ✓ SATISFIED | list/up/down/import; live connect/import → human |
| NET-06 | 04-01 | triple self-test raw ms | ✓ SATISFIED | live 3-hop figures |
| BT-01 | 04-03 | scan/list devices | ✓ SATISFIED | live 5-device sweep |
| BT-02 | 04-03 | pair/connect/disconnect | ✓ SATISFIED | confirm-gated code paths; real-peer pair → human |
| SND-01 | 04-04 | output device select | ✓ SATISFIED | name-keyed list+set; audible proof → human |
| SND-02 | 04-04 | input device select | ✓ SATISFIED | set-input by name; audible proof → human |
| PWR-01 | 04-04 | 3-profile switching | ✓ SATISFIED | backend-tagged status/set; governor switch → human |

All 11 Phase 4 requirement IDs accounted for. No orphans (NET-07/08, PWR-02, BT-03, SND-03 are v2, out of phase scope per CONTEXT).

### Context Decisions (D-01..D-10) Honored

| Decision | Status | Evidence |
|----------|--------|----------|
| D-01 replace-not-prepend | ✓ | `ipv4.dns` + `ignore-auto-dns yes` argv unit |
| D-02 IPv4 only | ✓ | strict validator rejects ::1; no ipv6 keys in argv builders |
| D-03 reset restores DHCP | ✓ | reset argv (empty + no) unit |
| D-04 3-mode proxy + one-liner | ✓ | off/manual/auto + single-line status + export hint |
| D-05 never auto-accept | ✓ | confirm-only agent, abort-spawns-nothing unit |
| D-06 scan-then-pick | ✓ | resolve_pick sole path to all 3 mutations |
| D-07 OpenVPN+WireGuard + file import, D-08 popup deferred | ✓ | .ovpn/.conf delegation, no Rust parsing; no GUI code |
| D-09 triple hop, D-10 raw ms no verdict | ✓ | live 3-hop ms; verdict-free units + grep |

### Anti-Patterns Found

None blocking. Workspace suite 183+83+18 green, diacritics guard clean, no stub patterns (no `todo!`/`unimplemented!` on Phase 4 paths per summaries; all tables render live data, empty states honest).

### Human Verification Required

5 items (see frontmatter `human_verification`): DNS round-trip, VPN connect+import, real-peer pair, audible switch, governor switch. All are live-mutation proofs the plans explicitly queued as manual verify-work — automation proves planning/validation/refusal, humans prove the effect.

### Gaps Summary

No gaps. All 16 must-haves verified against the codebase with unit + live-CLI evidence. Status is `human_needed` solely because 5 live-mutation effects require a human hand on real state (DNS profile, VPN tunnel, BT peer, speakers, governor) — the code paths behind each are fully proven headlessly.

---
_Verified: 2026-10-07_
_Verifier: the agent (gsd-verifier)_

## Gap Re-verification (G-04-2)

**Scope:** plan 04-05 (commits a29ddf5, 3fb5336), gap G-04-2 only. Full-phase 16/16 verdict above untouched.

| # | Item | Status | Evidence |
|---|------|--------|----------|
| 1 | vpn list surfaces provenance (stored vs ephemeral) in table + --json | ✓ closed | `vpn_list()` queries `-f NAME,UUID,TYPE,FILENAME` on both full + `--active` (network.rs:2273-2289); `is_persistent_filename` classifies empty/`--`→false, `/run/`→false, else true (2168-2177); `connection_rows` adds Provenance row stocké/éphémère (2698-2717); `VpnConnection.persistent: bool` serializes to --json; tests `provenance_parser_classifies_empty_dash_run_and_keyfile` + `vpn_list_table_names_every_tunnel_and_state` + `vpn_list_json_schema_pins_connection_fields` pass (6/6 core vpn, 5/5 cli vpn) |
| 2 | vpn down on ephemeral warns tracking-loss + import hint through confirm gate | ✓ closed | `run_vpn_down` passes `down_preview_warning_rows(&target)` as extra preview rows + `confirm_vpn_down_prompt(&target)` as prompt (net_cmds.rs:257-274); ephemeral → `(Avertissement, <warning>)` row + warned prompt, persistent → empty/plain (network.rs:2654-2675); warning text carries `vpn import` hint (fr.json `vpn_ephemeral_warning`, test asserts `contains("vpn import")`); warn-and-continue through existing `[o/N]` gate, no new error type |
| 3 | vpn_down reports post-deactivate read-back | ✓ closed | `vpn_down` routes deactivate then re-queries `vpn_list()` and reports `any(uuid match && active)`, degrading to `active:false` on read-back failure instead of hardcoded flag (network.rs:2382-2401) |
| 4 | single argv builder, no delete verb | ✓ closed | `vpn_down` calls `vpn_down_argv(&target.uuid)` (2385); builder pinned to exactly `[connection, down, uuid]` by test `down_argv_is_exactly_connection_down_uuid_with_no_delete_verb`; grep: `delete` appears only in doc/test/defect comments, zero in any argv builder |
| 5 | regression tests cover the above | ✓ closed | `cargo test -p archel-core --no-default-features network::vpn` 6/6 pass; `cargo test -p archel --no-default-features vpn` 5/5 pass; fixtures pure headless (never spawn nmcli, never touch wg0) |
| 6 | prohibitions hold (no delete path, no secrets, no live wg0 mutation) | ✓ closed | No delete path added; `VpnConnection` carries no key material, only derived `persistent` flag surfaces (raw FILENAME never leaves parser — T-4-05); snapshot diff contains only new `persistent` field, secret-free; tests use fixtures only |

**Verdict: gap-closed.** G-04-2 closure confirmed in code. Live-UAT caveat stands: re-running UAT test 2 against a real ephemeral tunnel still needs a human with a tunnel to sacrifice (automation cannot down a live tunnel).
