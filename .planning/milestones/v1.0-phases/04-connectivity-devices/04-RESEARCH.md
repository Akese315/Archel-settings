# Phase 4: Connectivity & Devices - Research

**Researched:** 2026-10-06
**Domain:** NetworkManager / BlueZ / PipeWire-WirePlumber / power-profiles + CPUfreq / session proxy on Arch + Hyprland (Rust, headless CLI proof, no GUI)
**Confidence:** MEDIUM (live-machine probes + registry verification in-session; crate API shapes from training knowledge flagged ASSUMED)

## Summary

Phase 4 adds inspect-and-control for network, Bluetooth, audio, and power through `archel-core` modules consumed by new `archel` CLI groups (`net`, `bt`, `audio`, `power`), with every write path proven headlessly. The live Archel machine (probed 2026-10-06) dictates four load-bearing facts the plan must respect: (1) NetworkManager 1.58.1 is running with an active Wi-Fi profile and native WireGuard support, but the OpenVPN NM plugin is **not** installed; (2) **power-profiles-daemon is absent** — PWR-01 must ship a cpufreq-governor fallback (only `performance`/`powersave` governors exist); (3) BlueZ 5.87 is installed with an `hci0` adapter present, but `bluetooth.service` is **inactive** — the stack must start/detect-gracefully; (4) PipeWire 1.6.9 + WirePlumber run on the session bus with **no named `org.wireplumber` service**, so audio switching goes through `wpctl` subprocess, not D-Bus.

**Primary recommendation:** Reads via `zbus` 5.x NM proxies + `wpctl`/`nmcli` subprocesses where D-Bus has no named endpoint; all NM writes in-process through NM's own D-Bus polkit (never the `archel-privileged` helper — that helper is pacman-scoped); proxy state in `gsettings org.gnome.system.proxy` (schema verified present, auth keys included); Bluetooth via `bluer` 0.17 with an in-process pairing agent that surfaces PINs to the CLI for confirmation (never auto-accept); introduce `tokio` to `archel-core` now (bluer/zbus require it) with minimal features.

## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Custom DNS servers replace the DHCP-provided ones on the profile (ignore DHCP), not prepended.
- **D-02:** IPv4 DNS only — no IPv6 DNS setting in this phase.
- **D-03:** A reset command restores automatic (DHCP) DNS on the profile.
- **D-04:** Three modes under one command: `net proxy off`, `net proxy manual <host> <port>` (exceptions optional), `net proxy auto <pac-url>`; bare `net proxy` prints current state in one line.
- **D-05:** Pairing requests surface the code in the CLI and ask for confirmation — never auto-accept.
- **D-06:** Connect flow is scan-then-pick from the live list, not direct MAC entry.
- **D-07:** Multi-protocol: OpenVPN and WireGuard at minimum; connections importable from config files (`.ovpn`, WireGuard `.conf`) in the CLI.
- **D-08:** The popup GUI form (username, password, certificate, key, or config file) is recorded for the GUI phase — CLI takes file import + fields; no GUI work now.
- **D-09:** `net test` checks all three hops: gateway ping, DNS resolution, internet reachability.
- **D-10:** Results show raw millisecond figures, not an auto-diagnosis verdict.

### Agent's Discretion
- Exact `net`/`bt`/`audio`/`power` subcommand inventory (EN), audio device identification (name vs index), power-profile switching mechanics, proxy storage mechanism (env vs gsettings vs file — researcher decides per session-layer constraint), VPN secret handling (keyring vs prompt), ping counts/timeouts, table layouts, JSON field names.

### Deferred Ideas (OUT OF SCOPE)
- VPN popup GUI form (username/password/certificate/key/file) — GUI phase, fields recorded above.
- Authenticated-proxy credentials (user/password) — left open for researcher (only if trivially supported by chosen mechanism).
- Speedtest history, DNS-leak detection, governor tuning, BT battery, per-app mixer — out of scope (phase boundary).

**Researcher rulings on discretion/deferred items:**
- **Audio identification: stable node names, never numeric IDs.** `wpctl` sink/source IDs are session-local and renumber on every restart [VERIFIED: live `wpctl status` shows IDs 47/59/61 that are allocation-order dependent]. Resolve `node.name` (e.g. `alsa_output.usb-...analog-stereo`) via `wpctl inspect`, present names in CLI, map name→current ID at switch time.
- **Proxy storage: `gsettings org.gnome.system.proxy` is canonical.** Schema verified present on this machine with `mode`, `autoconfig-url`, `ignore-hosts`, per-scheme `host`/`port`, and `authentication-user`/`authentication-password`/`use-authentication` keys [VERIFIED: live `gsettings list-keys`]. No daemon needed (dconf file backend works headless); consumed by Chromium/Electron/GTK apps. Hyprland caveat (document, don't solve): terminal tools read `$http_proxy` env, not gsettings — `net proxy` prints an `export` hint line alongside the one-line status.
- **Authenticated proxy: INCLUDE optional `--user/--password` flags.** Trivially supported by the chosen mechanism (dedicated gsettings keys exist) — deferred item resolved as in-scope-cheap.
- **VPN secrets: prompt-at-import, never stored by Archel.** `nmcli connection import` delegates secret handling to the NM plugin/agent; where a password is needed interactively, read via `rpassword` (no-echo stdin, never logged, never snapshotted). No keyring integration in v1 (no daemon to hold it — consistent with "no resident root daemon" out-of-scope rule).
- **Ping counts/timeouts: `ping -c 3 -W 2`, DNS via `dig +time=2 +tries=1`, internet via `ping -c 3 -W 2 1.1.1.1`.** Fast (<10 s worst case), raw ms preserved for D-10.
- **Power mechanics: two-tier — PPD if present, cpufreq-governor fallback otherwise.** This machine has no PPD, so the fallback is the shipped path here: `performance→performance`, `power-saver→powersave`, `balanced→powersave` + honest note (only two governors exist on this kernel). Table/JSON expose `backend: "power-profiles-daemon" | "cpufreq"` so the GUI phase can render provenance.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| NET-01 | See connection state (connected/disconnected, interface, SSID/wired) | NM D-Bus `GetDevices`/`ActiveConnections` via zbus; `nmcli` field names verified live |
| NET-02 | See IP address and gateway of the active connection | `IP4.ADDRESS`/`IP4.GATEWAY` fields verified live on `wlan0` |
| NET-03 | Configure DNS servers per NM connection profile | `ipv4.dns` + `ipv4.ignore-auto-dns` keys verified live; D-01..D-03 map to exact `nmcli connection modify` sequences |
| NET-04 | Configure proxy (system/session layer, separate from NM) | gsettings schema verified; 3-mode command design |
| NET-05 | Manage VPN (list, connect, disconnect via NM) + file import | `nmcli connection import type <type> file` verified (plugin-gated); WireGuard native, OpenVPN plugin missing → graceful error |
| NET-06 | Network tests (ping gateway/DNS, DNS resolution, latency) | ping/dig/getent all present; triple-hop design per D-09/D-10 |
| BT-01 | Scan and list nearby Bluetooth devices | bluer 0.17 (OK); hci0 present, daemon inactive → start-or-degrade path |
| BT-02 | Pair, connect, disconnect BT devices | bluer in-process agent for PIN-confirm (D-05); scan-then-pick (D-06) |
| SND-01 | Select audio output device (default sink) | `wpctl set-default` verified; name-based identification |
| SND-02 | Select audio input device (default source) | Same mechanism, `set-default <source-id>` |
| PWR-01 | Switch power profile (performance/balanced/power-saver) | PPD absent → cpufreq fallback (`scaling_governor` verified writable path pattern, governors `performance powersave` verified) |

## Project Constraints (from AGENTS.md)

Single `archel` binary (clap derive, EN subcommand/flag names; FR display only via `archel-core` rust-i18n catalog — the binary owns no user-facing copy). pkexec is the **sole** escalation mechanism — no sudo path anywhere in core or GUI [VERIFIED: `crates/archel-core/src/privileged.rs:1-12`]. `archel-privileged` helper is pacman-scoped (exact-argv allowlist, v1 JSON envelope); **NM writes must NOT go through it** — ROADMAP locks "NM writes via its own D-Bus polkit". Reads unprivileged in-process; mutations confirmed `[o/N]` before execution (Phase 3 D-01 precedent) — applies to DNS/proxy/VPN/BT/audio/power writes. Global `--json` + insta snapshots pin every machine schema. `LC_ALL=C` on all parse-bound spawns (`nmcli` output on this machine is French-localized — verified: `nmcli` prints `connecté`, `Syntaxe :`). Headless `--no-default-features` gates: no Qt outside `gui/` feature. Typed `ArchelError`, never panics on failure paths. No GUI work in this phase; CLI proof gate on every write path.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| NM state/IP/DNS reads | API (archel-core, in-process) | — | D-Bus system bus, no privilege needed for reads |
| DNS per-profile write | API → NM D-Bus (polkit-mediated) | — | NM's own auth; helper must not be involved |
| Session proxy | API → dconf/gsettings (user session) | — | Session layer, independent of NM (REQ NET-04 explicit) |
| VPN manage/import | API → `nmcli` subprocess | — | Import is plugin-implemented; only nmcli exposes it uniformly |
| Triple self-tests | API → ping/dig subprocesses | — | which-gated, LC_ALL=C, timeout-bounded (updates.rs pattern) |
| BT scan/pair/connect | API (bluer, tokio) | — | In-process BlueZ D-Bus; needs bluetooth.service running |
| Audio sink/source select | API → `wpctl` subprocess | — | No named WP D-Bus endpoint on session bus (verified) |
| Power profile switch | API → PPD D-Bus **or** sysfs cpufreq | — | PPD absent on target; sysfs is stable ABI, root-owned writes need privilege path decision (see Pitfalls) |

## Standard Stack

### Core (new dependencies for archel-core)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tokio | 1.53.2 [VERIFIED: crates.io via `cargo search`] | Async runtime for zbus + bluer | Non-negotiable: bluer is tokio-only; zbus 5.x is async-first. Minimal features: `rt-multi-thread`, `time`, `macros`. First introduction of async into core — CLI needs `#[tokio::main]`. |
| zbus | 5.19.0 [VERIFIED: crates.io via `cargo search`; legitimacy OK] | NM D-Bus proxies (state, DNS, VPN list/up/down) + PPD proxy (when present) | STACK-blessed single D-Bus stack; `zbus::proxy` macro, 2.2M weekly downloads, repo github.com/z-galaxy/zbus. |
| bluer | 0.17.4 [VERIFIED: crates.io; legitimacy OK] | BlueZ adapter/discovery/pairing/connect | Official BlueZ Rust bindings, tokio-native, streaming discovery fits scan-then-pick. |
| rpassword | 7.x [ASSUMED version — legitimacy OK, 773k/wk] | No-echo password prompt for VPN/proxy auth | Only if needed at import time; tiny, no async, no D-Bus. Planner: verify exact version via `cargo search rpassword` at plan time. |

### Supporting (already in workspace — reuse, no new deps)

| Library | Purpose in Phase 4 | Pattern source |
|---------|-------------------|----------------|
| duct + which | `nmcli`, `wpctl`, `gsettings`, `ping`, `dig`, `systemctl` spawns; capability detection | `updates.rs` which-gated + LC_ALL=C + timeout |
| serde/serde_json | `--json` schemas for net/bt/audio/power; insta pins | Phase 2–3 precedent |
| thiserror + rust-i18n FR catalog | New typed variants + catalog keys per user-visible string | `error.rs` + `locales/fr.json` |
| gsettings CLI (system tool, present) | Proxy read/write via `gsettings set org.gnome.system.proxy …` | Avoids dconf-rs crate entirely |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| zbus-direct NM proxies | `networkmanager` crate 0.5.0 [WARNING: flagged as suspicious — verify before using.] | Saves proxy boilerplate IF its API covers Settings/Connection/VPN. But legitimacy gate returned **SUS (630/wk, low-downloads)** → planner must add `checkpoint:human-verify` before use; default stays **zbus-direct**. Time-boxed spike only. |
| `wpctl` subprocess for audio | pipewire-rs 0.10.1 enumeration + WP D-Bus switch | Rejected: no named WirePlumber D-Bus service on this machine's session bus [VERIFIED: live `busctl --user list`]; `wpctl set-default` is the WP-blessed path and is parse-light (IDs only). |
| gsettings for proxy | env-var file (`~/.config/archel/proxy.env` + shell sourcing) | Rejected as canonical: nothing consumes it automatically; gsettings is consumed by Chromium/Electron/GTK today. Print `export` hint for shells instead. |
| PPD-only power switching | cpufreq sysfs fallback (shipped) | PPD absent on target machine — PPD-only would make PWR-01 untestable. Ship fallback; prefer PPD when its D-Bus name appears. |
| keyring (secret-service) for VPN secrets | Prompt-at-use via rpassword | Rejected for v1: no daemon/session-unlock story; prompt keeps secrets out of disk entirely. |

**Installation:**
```bash
cargo add tokio --features rt-multi-thread,time,macros zbus bluer -p archel-core
cargo add rpassword -p archel-core   # only if VPN/proxy-auth prompt lands in scope
```

**Version verification:** `cargo search` 2026-10-06 returned `zbus = "5.19.0"`, `bluer = "0.17.4"`, `tokio = "1.53.2"`, `networkmanager = "0.5.0"` — matches STACK.md pins (tokio 1.53.1→1.53.2 patch drift, harmless).

## Package Legitimacy Audit

> Run via `gsd-tools package-legitimacy check --ecosystem crates …` 2026-10-06.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| zbus | crates.io | 6 yrs (2020) | 2.26M/wk | github.com/z-galaxy/zbus | OK | Approved |
| bluer | crates.io | 5 yrs (2021) | 53k/wk | github.com/bluez/bluer | OK | Approved |
| tokio | crates.io | 10 yrs (2016) | 19.2M/wk | github.com/tokio-rs/tokio | OK | Approved |
| rpassword | crates.io | 11 yrs (2015) | 773k/wk | github.com/conradkleinespel/rpassword | OK | Approved (version ASSUMED — re-verify at plan time) |
| networkmanager | crates.io | 6 yrs (2020) | 630/wk | github.com/zibebe/networkmanager-rs | SUS | Flagged — spike-only, planner must add checkpoint:human-verify before adding |

**Packages removed due to SLOP verdict:** none.
**Packages flagged as suspicious (SUS):** `networkmanager` — planner inserts `checkpoint:human-verify` before any `cargo add networkmanager`; default implementation path must not depend on it.

## Architecture Patterns

### System Architecture Diagram

```
archel net|bt|audio|power (clap groups, EN names)
   │  confirm [o/N] on writes → FR tables / --json
   ▼
archel-core modules: network.rs │ proxy.rs │ bluetooth.rs │ audio.rs │ power.rs
   │ typed ArchelError + FR catalog keys (new variants, see below)
   ├─ zbus system bus ──► org.freedesktop.NetworkManager (1.58.1) [reads, DNS write, VPN up/down]
   ├─ duct+which ──► nmcli (VPN import: plugin-implemented, WireGuard native / OpenVPN needs plugin)
   ├─ duct+which ──► gsettings org.gnome.system.proxy (proxy store; export-hint for shells)
   ├─ duct+which ──► ping/dig/getent (triple self-test, LC_ALL=C, timeouts)
   ├─ bluer/tokio ──► org.bluez (needs bluetooth.service active; in-process pairing agent)
   ├─ duct+which ──► wpctl status|inspect (list) / set-default (switch, name→ID resolution)
   └─ zbus?/sysfs ──► net.hadess.PowerProfiles (if present) else /sys/.../scaling_governor
```

### Recommended Project Structure

```
crates/archel-core/src/
├── network.rs     # NM state/IP/DNS/VPN reads+writes, net test triple (zbus + nmcli/ping/dig)
├── proxy.rs       # gsettings proxy get/set, one-line status + export hint
├── bluetooth.rs   # bluer session, scan stream, agent (PIN-confirm), pair/connect/disconnect
├── audio.rs       # wpctl list/inspect parse, name→ID resolve, set-default
└── power.rs       # PPD probe → PPD set; else cpufreq governor map + backend tag
crates/archel/src/cmds/
├── net_cmds.rs    # status, dns (show|set|reset), proxy, vpn (list|up|down|import), test
├── bt_cmds.rs     # scan, pair, connect, disconnect, devices
├── audio_cmds.rs  # list, set-output, set-input
└── power_cmds.rs  # status, set <performance|balanced|power-saver>
```

New `ArchelError` variants (planner: add + catalog keys): `NetworkUnavailable`, `ProfileNotFound`, `DnsInvalid`, `VpnPluginMissing { plugin }`, `VpnImportFailed`, `BluetoothUnavailable` (daemon down), `DeviceNotFound`, `PairingRejected`/`PairingTimeout`, `AudioDeviceNotFound`, `PowerBackendMissing`, `PolkitDenied { action }` (NM D-Bus auth failure — NOT AuthFailed, which is helper-scoped), `NeedsTty` reuse for blind mutations.

### Pattern 1: NM reads via zbus proxy, writes via Settings.Update
**What:** Single `org.freedesktop.NetworkManager` connection; reads for NET-01/02, `Settings.GetConnectionByUuid` + `Connection.Update` for DNS (D-01: set `ipv4.dns=[…]` + `ipv4.ignore-auto-dns=true`; D-03: clear both), then `ActivateConnection` reapply. Touch **only** `ipv4.*` keys (D-02). Auth denial (`org.freedesktop.NetworkManager.Settings.NotAuthorized` or polkit `AccessDenied`) → `PolkitDenied`, never a helper call [ASSUMED exact error string — verify against live bus in Wave 0].
**When to use:** All NET-01/02/03/05 (except import).

### Pattern 2: VPN import shells to `nmcli connection import`
**What:** `nmcli connection import type wireguard file <path>` works natively (wg0 proves it); `type openvpn` requires the `NetworkManager-openvpn` plugin package (not installed → typed `VpnPluginMissing` with `pacman -S networkmanager-openvpn` hint). Validate path client-side (exists, `.ovpn`/`.conf` extension, no shell). Never parse .ovpn/WireGuard formats in Rust [ASSUMED nmcli type names `wireguard`/`openvpn` — standard, verify via `nmcli connection import type <t> file /dev/null` dry probe in Wave 0 without mutating].
**When to use:** NET-05 import only. List/up/down stay on zbus/nmcli-read path.

### Pattern 3: bluer session + short discovery + registered agent
**What:** `Bluer::Session::new().await` → `adapter.set_powered(true)` → timed discovery (~10–15 s, STREAM events into live list for scan-then-pick, D-06) → selected device `pair()` with a pre-registered in-process agent whose `request_pin_code`/`display_pin_code` callback prints the code and asks `[o/N]` (D-05, never auto-accept) → `connect()`. If `org.bluez` is not on the system bus: `BluetoothUnavailable` with `sudo systemctl start bluetooth` hint (daemon inactive verified live; adapter hci0 exists in sysfs).
**When to use:** BT-01/02. API shape [ASSUMED] — planner: spike agent callback signature first (bluer 0.17 docs), it's the highest-risk API surface in the phase.

### Pattern 4: wpctl name-based audio switching
**What:** `wpctl status` (LC_ALL=C) → parse Sinks/Sources lines → `ID → {name via wpctl inspect <id>}` map; CLI shows names; `set-output <name>` resolves to current ID → `wpctl set-default <id>`. Fallback order if `wpctl` missing: typed error (no second fallback — PipeWire is a hard platform dep).
**When to use:** SND-01/02.

### Anti-Patterns to Avoid
- **Routing NM writes through `archel-privileged`.** The helper is pacman-scoped (exact-argv allowlist); NM has its own polkit. Cross-wiring breaks the privilege model.
- **Storing VPN/proxy secrets in snapshots, logs, or `--json`.** Insta pins must redact secrets; `Display` stays English/log-only and secret-free (error.rs T-1-02 rule).
- **Matching audio devices by numeric ID across runs.** IDs renumber; always resolve by `node.name`.
- **Writing `ipv6.*` keys alongside DNS changes.** D-02 forbids it; even "harmless" ipv6 touches risk breaking working v6 on this network (systemd-resolved stub is live).
- **Auto-accepting pairing or defaulting DNS on ambiguity.** No-default-ok rule (T-1-01): decline/ask, never assume.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| NM profile parsing/writing | Keyfile editor for system-connections | D-Bus `Connection.Update` / `nmcli connection modify` | Keyfiles are root-owned, format-versioned, and bypass polkit audit |
| OpenVPN/WireGuard config parsing | .ovpn/.conf parser | `nmcli connection import` (plugin-implemented) | Crypto/option matrix + secret flags; a partial parser silently drops options |
| BT pairing crypto/agent protocol | Custom BlueZ agent | bluer's agent registration | Passkey/PIN flows have timeout/retry semantics tied to the daemon |
| Default-sink switching | pactl-style direct socket writes | `wpctl set-default` | WP owns routing policy; bypassing it desyncs volumes/streams |
| Latency verdicts | "diagnosis" strings | Raw ms (D-10) | Locked decision; verdicts also need baselines we don't have |
| Speedtest / DNS-leak / governor tuning | Any implementation | Nothing (out of scope) | Explicitly deferred; don't smuggle in |

**Key insight:** Every Phase 4 write is a call into a daemon that already owns policy, auth, and rollback (NM, BlueZ, WP, PPD). Archel's job is typed intent + confirmation + honest errors — never reimplementing daemon logic.

## Common Pitfalls

### Pitfall 1: No polkit agent in headless/Hyprland sessions → NM system-profile writes fail
**What goes wrong:** `Connection.Update` on a system-owned profile returns NotAuthorized even for a sudo-capable user.
**Why it happens:** Verified live — no `*polkit*agent*` process runs on this machine. NM system settings need `org.freedesktop.NetworkManager.settings.modify.system`; without an agent the auth dialog can never complete.
**How to avoid:** Map to `PolkitDenied` with a hint (`start hyprpolkitagent` / run from a session with an agent). Reads and user-scoped operations keep working. Planner: Wave 0 must attempt one harmless write path probe and record the outcome.
**Warning signs:** `busctl` shows NM on system bus but every write fails while reads succeed.

### Pitfall 2: power-profiles-daemon missing → PWR-01 untestable via PPD
**What goes wrong:** Plan assumes `net.hadess.PowerProfiles` exists; it doesn't (verified absent; package not installed).
**Why it happens:** Minimal Arch installs often skip PPD. Only UPower (battery/uptime, not profiles) is present.
**How to avoid:** Ship the cpufreq fallback as the primary path here: map `performance→performance`, `balanced→powersave`, `power-saver→powersave`, tag output with `backend` + note. Governor writes are root-owned (`/sys/.../scaling_governor` is 644 root) → needs privilege decision: document as `PolkitDenied`-style typed error suggesting `pkgs` helper extension is OUT of scope; recommend user runs `archel power set` in a session where a udev rule or PPD exists. Planner must decide: read-only-if-unprivileged vs error-with-hint (researcher recommends error-with-hint, consistent with honest-N/A precedent from Phase 2 D-10).
**Warning signs:** `powerprofilesctl: commande introuvable` (seen live).

### Pitfall 3: bluetooth.service inactive despite adapter present
**What goes wrong:** bluer `Session::new()` fails with "no such service".
**Why it happens:** `org.bluez` is activatable but activation fails (unknown unit state seen live); adapter `hci0` exists in sysfs but no daemon owns it.
**How to avoid:** `BluetoothUnavailable` + `sudo systemctl enable --now bluetooth` hint; every `bt` command checks daemon presence first (bus name probe, cheap). Never attempt auto-start from core (service management is out of scope).
**Warning signs:** `busctl tree org.bluez` → activation failure.

### Pitfall 4: French-localized `nmcli` breaks naive parsers
**What goes wrong:** `nmcli` speaks French on this machine (`connecté`, `Syntaxe :`).
**Why it happens:** No `LC_ALL=C` on the spawn.
**How to avoid:** Phase 1–3 rule — `LC_ALL=C` (or `-t` terse + `-f` explicit fields) on every parse-bound spawn. Prefer `-t -f FIELD` over human tables. [VERIFIED: live output is localized].

### Pitfall 5: DNS change without reapply looks like success but changes nothing
**What goes wrong:** Profile updated, effective DNS unchanged until reconnect.
**Why it happens:** NM applies profile DNS on (re)activation.
**How to avoid:** After `Update`, reactivate the profile (`ActivateConnection`); report effective DNS read-back in the success line. D-03 reset follows the same reapply.

### Pitfall 6: tokio introduced into a sync core — executor/mealtime hazards
**What goes wrong:** Blocking duct spawns inside async contexts stall the runtime; two runtimes (core + bluer) conflict.
**Why it happens:** First async dependency in `archel-core`.
**How to avoid:** One `#[tokio::main]` at the binary; core exposes `async fn`s; wrap blocking spawns in `spawn_blocking` or keep subprocess helpers sync and call them from async via `tokio::task::spawn_blocking`. Never `block_on` inside core.

### Pitfall 7: `wg0` is already an active NM WireGuard connection
**What goes wrong:** Self-tests or VPN up/down during development drop the machine's tunnel.
**Why it happens:** Verified live — `wg0:wireguard:wg0` is active.
**How to avoid:** Never `down`/`delete` profiles in tests; test VPN flows with a `--temporary` import or a throwaway profile; document the hazard in plan verification steps.

## Code Examples

### NM active-connection read (zbus proxy shape)

```rust
// Source: zbus 5.x `proxy` macro pattern (STACK-blessed); exact NM member
// names [ASSUMED] — verify in Wave 0 spike against org.freedesktop.NetworkManager 1.58.1.
use zbus::proxy;

#[proxy(
    interface = "org.freedesktop.NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
trait NetworkManager {
    fn get_devices(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;
    #[zbus(property)]
    fn primary_connection(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
}
```

### DNS replace / reset via nmcli (exact sequences for D-01/D-03)

```bash
# D-01 replace (IPv4 only, D-02): values verified as live keys on the active profile
nmcli connection modify "<profile>" ipv4.dns "1.1.1.1 9.9.9.9" ipv4.ignore-auto-dns yes
nmcli connection up "<profile>"   # reapply (Pitfall 5)
# D-03 reset to DHCP:
nmcli connection modify "<profile>" ipv4.dns "" ipv4.ignore-auto-dns no
nmcli connection up "<profile>"
```
Field existence [VERIFIED: live `nmcli -t -f ipv4.dns,ipv4.ignore-auto-dns,ipv4.method connection show`]; reapply semantics [ASSUMED standard NM behavior].

### Proxy get/set via gsettings (schema verified live)

```bash
# Read one-line status (D-04 bare `net proxy`):
gsettings get org.gnome.system.proxy mode            # 'none' (verified current value)
# Manual + exceptions:
gsettings set org.gnome.system.proxy mode 'manual'
gsettings set org.gnome.system.proxy.http host '"proxy.example"'
gsettings set org.gnome.system.proxy.http port 8080
gsettings set org.gnome.system.proxy ignore-hosts "['localhost', '127.0.0.0/8']"
# Auto PAC:
gsettings set org.gnome.system.proxy mode 'auto'
gsettings set org.gnome.system.proxy autoconfig-url '"http://example/proxy.pac"'
# Off:
gsettings set org.gnome.system.proxy mode 'none'
# Optional auth (deferred item, trivially supported — keys verified present):
gsettings set org.gnome.system.proxy.http use-authentication true
gsettings set org.gnome.system.proxy.http authentication-user '"user"'
```
Keys [VERIFIED: live `gsettings list-keys` incl. `authentication-user/password`, `use-authentication`]. Password via rpassword prompt, passed on stdin — never argv (argv leaks to `ps`).

### VPN import (plugin-gated)

```bash
# Works natively (no plugin):
nmcli connection import [--temporary] type wireguard file <path.conf>
# Requires NetworkManager-openvpn plugin (NOT installed → VpnPluginMissing + install hint):
nmcli connection import type openvpn file <path.ovpn>
```
Import-type/plugin model [VERIFIED: live `nmcli connection import --help`: "Seules les configurations VPN… importée par les greffons de VPN"]; exact type strings [ASSUMED].

### Audio switch (name-based, IDs verified unstable)

```bash
LC_ALL=C wpctl status        # parse Sinks:/Sources: lines → IDs (verified format live)
wpctl inspect <id>           # → node.name for stable identification
wpctl set-default <id>       # switch (verified subcommand in `wpctl --help`)
```

### Power fallback (cpufreq)

```bash
cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_available_governors  # "performance powersave" [VERIFIED]
echo powersave | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor  # needs privilege (Pitfall 2)
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `dbus` crate blocking API | zbus 5.x async | 2023–2024 | Don't mix; tokio-only in core |
| `qmetaobject`/GTK proxy applets | gsettings CLI for proxy | — | No new crate; dconf file backend works headless |
| PPD as only power API | PPD-probe → cpufreq fallback | This research (target lacks PPD) | PWR-01 shippable on minimal Arch |
| `pactl set-default-sink` | `wpctl set-default` | PipeWire era | `pactl` targets PulseAudio server semantics; WP owns routing now |
| `networkmanager` 0.5 wrapper as default | zbus-direct default, wrapper spike-gated | This research (SUS verdict) | Avoids a 630/wk dependency on the critical path |
| MAC-address CLI args for BT | scan-then-pick (D-06) | Locked | Better UX + avoids typo'd MAC writes |

**Deprecated/outdated:**
- `wireplumber` crate 0.0.1 (2023 placeholder) — use `wpctl`.
- `nmcli` human-table parsing without `LC_ALL=C`/`-t -f` — locale breakage (verified live).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | NM D-Bus exact member names (`GetDevices`, `ActivateConnection`, `Settings.GetConnectionByUuid`, `Connection.Update`, `AccessDenied/NotAuthorized` strings) | Patterns 1–2 | Build failure or unmapped auth error — mitigate with Wave 0 live-bus spike |
| A2 | bluer 0.17 `Session`/`Adapter`/`Agent` API shape (agent callback for PIN) | Pattern 3 | Highest-risk surface — spike first, before CLI wiring |
| A3 | `nmcli connection import type wireguard\|openvpn` exact type strings | Pattern 2 | Import fails — dry-probe in Wave 0 (`file /dev/null` expect clean error, no mutation) |
| A4 | NM DNS reapply requires profile reactivation | Pitfall 5 | Success line lies — verify by DNS read-back in tests |
| A5 | rpassword version 7.x (legitimacy OK, version not pinned) | Stack | Minor — `cargo search rpassword` at plan time |
| A6 | Balanced→powersave governor mapping acceptable to user | Ruling | UX decision — confirm in discuss/verify (one question) |
| A7 | gsettings proxy consumed by user's apps on Hyprland (Chromium/Electron/GTK yes; Firefox uses own prefs; terminals need env export) | Proxy ruling | Proxy "does nothing" for some apps — the export-hint + docs mitigate; no code fix possible |
| A8 | Governor sysfs writes need privilege → error-with-hint (no helper extension) | Pitfall 2 | If user expects one-shot escalation, needs Phase 1 helper extension — confirm in planning |

## Open Questions

1. **Power-privilege path**: cpufreq writes are root-owned. Options: (a) typed error + hint (recommended, no scope creep), (b) extend `archel-privileged` allowlist (reopens Phase 1 security surface). Recommendation: (a); confirm with user.
2. **Balanced governor mapping**: with only `performance`/`powersave` available, balanced→powersave + note. Confirm, or install PPD (`pacman -S power-profiles-daemon`) to get real three-profile switching.
3. **BT daemon auto-start**: `bt` commands error with enable-hint vs attempting `systemctl --user`/system start. Recommendation: hint only (service management out of scope).
4. **`net test` internet endpoint**: `ping 1.1.1.1` (ICMP, may be filtered) vs TCP/HTTP probe. Recommendation: ICMP primary (raw ms, no verdict), note filtered-ICMP caveat in output.

## Environment Availability

> Probed live on the Archel machine 2026-10-06 (all `timeout`-bounded; `bluetoothctl show` hangs without a daemon — use `busctl` probes instead).

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| NetworkManager (system bus) | NET-01/02/03/05 | ✓ | 1.58.1 (`org.freedesktop.NetworkManager` @ :1.6) | — |
| nmcli | VPN import, DNS write alt-path | ✓ | — (FR locale: use LC_ALL=C) | zbus-direct |
| Active Wi-Fi profile + WireGuard wg0 | NET reads, VPN list | ✓ | `KLEY Rennes privée` (wlan0, 10.192.61.76/24, gw+DND 10.192.61.1) | — (do not disturb wg0 in tests) |
| NetworkManager-openvpn plugin | NET-05 OpenVPN import | ✗ | — | Typed `VpnPluginMissing` + install hint; WireGuard works now |
| power-profiles-daemon | PWR-01 preferred backend | ✗ | — (only UPower 1.91.4) | cpufreq sysfs fallback (governors verified) |
| BlueZ daemon (org.bluez) | BT-01/02 | ✗ (inactive) | bluez 5.87 installed, hci0 present | `BluetoothUnavailable` + enable-hint |
| PipeWire + WirePlumber | SND-01/02 | ✓ | 1.6.9 / 0.5.17, sinks+sources present | — |
| wpctl | audio list/switch | ✓ (`set-default`, `inspect` verified) | — | — |
| gsettings + dconf | NET-04 store | ✓ (mode `'none'` current) | schema + auth keys verified | — |
| polkit agent | NM system writes | ✗ (none running) | — | `PolkitDenied` + agent hint; reads unaffected |
| ping/dig/getent | NET-06 | ✓ | — | — |
| gsettings consumer on Hyprland | NET-04 effectiveness | partial | Chromium/Electron/GTK yes; terminals need env | export-hint line |
| rustc/cargo | build | ✓ | 1.98.1 (edition 2024 OK) | — |

**Missing dependencies with no fallback:** none blocking (every gap has a typed-error + hint path).
**Missing with fallback:** PPD→cpufreq; OpenVPN plugin→WireGuard-now + hint; BT daemon→hint; polkit agent→hint.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test + insta 1.48 (`json` feature, already dev-dep) |
| Config file | none — see Wave 0 |
| Quick run command | `cargo test -p archel-core --no-default-features` |
| Full suite command | `cargo test --workspace --no-default-features` (headless gate: no Qt) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| NET-01/02 | NM state parse → struct | unit (pure parser, mocked D-Bus payload) | `cargo test -p archel-core network::` | ❌ Wave 0 |
| NET-03 | DNS key validation (IPv4 addrs, reject v6/hostnames) | unit | `cargo test -p archel-core network::dns` | ❌ Wave 0 |
| NET-03 | DNS set/reset live round-trip | manual-only (mutates live profile, needs polkit agent) | `archel net dns …` on target | n/a — justify: mutates user network |
| NET-04 | Proxy mode transitions | unit (gsettings argv builder) + live manual | `cargo test -p archel-core proxy::` | ❌ Wave 0 |
| NET-05 | VPN list parse; import arg validation | unit | `cargo test -p archel-core network::vpn` | ❌ Wave 0 |
| NET-06 | Hop-output ms parsing | unit (mock ping/dig stdout, FR + C locales) | `cargo test -p archel-core network::test` | ❌ Wave 0 |
| BT-01/02 | Discovery-event → device-row mapping | unit (mock events) | `cargo test -p archel-core bluetooth::` | ❌ Wave 0 |
| BT-01/02 | Live scan/pair | manual-only (needs daemon + hardware peer) | `archel bt scan` | n/a — justify: hardware |
| SND-01/02 | `wpctl status` parse (multi-locale fixtures) + name→ID resolve | unit + insta snapshot | `cargo test -p archel-core audio::` | ❌ Wave 0 |
| PWR-01 | Governor map + backend tag | unit | `cargo test -p archel-core power::` | ❌ Wave 0 |
| ALL writes | CLI `--json` schemas | insta snapshots (secrets redacted fixtures) | `cargo insta test -p archel` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p archel-core --no-default-features`
- **Per wave merge:** `cargo test --workspace --no-default-features`
- **Phase gate:** Full suite green + live CLI proof of every write path before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/archel-core/src/network.rs` (+ `#[cfg(test)]` parser tests with FR-locale fixtures)
- [ ] `crates/archel-core/src/{proxy,bluetooth,audio,power}.rs` (+ unit tests)
- [ ] Live-bus spike: NM proxy names + auth-error string + `nmcli import` dry-probe + bluer agent signature
- [ ] Framework/config: none new (insta already present); add FR-locale `wpctl`/`nmcli` fixture files under testdata/

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | partial | No app credentials; VPN/proxy passwords prompted (rpassword), never stored; NM auth via system polkit (no bypass) |
| V3 Session Management | no | No sessions (one-shot CLI) |
| V4 Access Control | yes | Never run app as root; NM writes use caller's polkit; helper untouched; `PolkitDenied` typed, no retry-as-root fallback |
| V5 Input Validation | yes | IPv4-only DNS validator (reject v6, hostnames, leading zeros ambiguity); MAC `XX:..` strict parse; proxy host/port range; import path confined (exists, is-file, `.ovpn`/`.conf`, no `/proc|/sys|/dev`); SSID/device names treated as untrusted display data |
| V6 Cryptography | partial | No crypto implemented; VPN crypto stays inside NM plugins; never log secrets |
| V7 Logging | yes | FR catalog only; `Display`/logs English, secret-free; insta fixtures redacted |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Password in argv/env visible via `ps` | Info disclosure | rpassword stdin prompt; gsettings write via stdin/`--password`? (gsettings takes values as argv — acceptable: local session bus, transient; document) |
| Malicious `.ovpn` `auth-user-pass` path exfil | Tampering | Import via nmcli (plugin-sandboxed); validate path client-side; never cat secrets to terminal |
| Rogue BT device spoofing name (phase boundary: no allowlist) | Spoofing | Show MAC + name + signal; confirm-before-pair; never auto-accept (D-05) |
| DNS hijack via profile confusion (wrong profile modified) | Tampering | Operate on active-profile UUID by default; `--profile` requires exact name/UUID match; confirm shows profile + old→new |
| Secret leak into insta snapshots / `--json` | Info disclosure | Redaction helper for secrets in Debug/Serialize test paths; review snapshots for key material |

## Sources

### Primary (tool-confirmed in-session)
- Live machine probes 2026-10-06: `nmcli general status` + `connection show --active` + `-f IP4.* device show wlan0`; `busctl list/get-property/tree` (NM 1.58.1, UPower 1.91.4, no PPD, org.bluez inactive); `wpctl status/--help`; `gsettings list-keys/get` (proxy schema + auth keys, mode 'none'); `nmcli connection import --help`; sysfs (`/sys/class/bluetooth/hci0`, cpufreq governors); `pacman -Q` (pipewire/wireplumber/bluez/openvpn/wireguard-tools; PPD + NM-openvpn-plugin absent); `cargo search` (zbus 5.19.0, bluer 0.17.4, tokio 1.53.2, networkmanager 0.5.0)
- `gsd-tools package-legitimacy check --ecosystem crates` (zbus/bluer/tokio/rpassword OK; networkmanager SUS)
- In-repo reads: `crates/archel-core/src/privileged.rs:1-12` (pkexec-only), `error.rs` (error taxonomy rules), `lib.rs` (module pattern), `crates/archel/src/cmds/mod.rs` (CLI shape)

### Secondary
- STACK.md (2026-09-30, crates.io/docs.rs/KDAB/archlinux.org verified): zbus/bluer/wpctl-fallback/`networkmanager`-spike guidance — spike verdict updated by today's SUS result
- CONTEXT.md Phase 4 + ROADMAP (NM-via-own-polkit, pkexec-helper pacman-scoped)

### Tertiary ([ASSUMED], need Wave 0 spike)
- NM D-Bus member names, bluer 0.17 agent API shape, nmcli import type strings, DNS reapply semantics, rpassword exact version

## Metadata

**Confidence breakdown:**
- Standard stack: MEDIUM — versions + legitimacy tool-verified; crate API shapes assumed (spike-gated)
- Architecture: MEDIUM — backend choices grounded in live probes (no-PPD, no-WP-bus-name, gsettings-present); write-path auth strings need live verification
- Pitfalls: HIGH — every major pitfall observed directly on the target machine this session
- Proxy/audio/VPN mechanics: MEDIUM — CLIs/schemas verified live, exact import type strings assumed

**Research date:** 2026-10-06
**Valid until:** 2026-11-05 (stable domain; re-probe BT daemon + PPD + polkit-agent presence at execution — all three can change with one pacman/systemctl command)
