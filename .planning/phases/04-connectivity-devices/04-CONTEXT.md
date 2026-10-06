# Phase 4: Connectivity & Devices - Context

**Gathered:** 2026-10-06
**Status:** Ready for planning

## Phase Boundary

Phase 4 delivers inspect-and-control for network, Bluetooth, audio, and power through one API surface, proven headlessly: connection state + IP/gateway/DNS, per-profile DNS override, session proxy, VPN manage + import (OpenVPN/WireGuard), triple self-tests, BT scan/pair/connect, audio sink/source selection, power-profiles switching. No GUI work in this phase; the CLI proof gate applies to every write path. Speedtest history, DNS-leak detection, governor tuning, BT battery, per-app mixer are explicitly out of scope.

## Implementation Decisions

### DNS per profile (NET-03)
- **D-01:** Custom DNS servers replace the DHCP-provided ones on the profile (ignore DHCP), not prepended.
- **D-02:** IPv4 DNS only — no IPv6 DNS setting in this phase.
- **D-03:** A reset command restores automatic (DHCP) DNS on the profile.

### Session proxy (NET-04)
- **D-04:** Three modes under one command (agent proposal, user-approved): `net proxy off`, `net proxy manual <host> <port>` (exceptions optional), `net proxy auto <pac-url>`; bare `net proxy` prints current state in one line.

### Bluetooth pairing (BT-01/BT-02)
- **D-05:** Pairing requests surface the code in the CLI and ask for confirmation — never auto-accept.
- **D-06:** Connect flow is scan-then-pick from the live list, not direct MAC entry.

### VPN (NET-05)
- **D-07:** Multi-protocol: OpenVPN and WireGuard at minimum (researcher confirms exact NM-supported set); connections importable from config files (`.ovpn`, WireGuard `.conf`) in the CLI.
- **D-08:** The popup GUI form (username, password, certificate, key, or config file) is recorded for the GUI phase — CLI takes file import + fields; no GUI work now.

### Network self-tests (NET-06)
- **D-09:** `net test` checks all three hops: gateway ping, DNS resolution, internet reachability.
- **D-10:** Results show raw millisecond figures, not an auto-diagnosis verdict.

### the agent's Discretion
- Exact `net`/`bt`/`audio`/`power` subcommand inventory (EN), audio device identification (name vs index), power-profile switching mechanics, proxy storage mechanism (env vs gsettings vs file — researcher decides per session-layer constraint), VPN secret handling (keyring vs prompt), ping counts/timeouts, table layouts, JSON field names.

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

No external specs — requirements fully captured in decisions above. Phase sources:
- `.planning/REQUIREMENTS.md` — NET-01..06, PWR-01, BT-01..02, SND-01..02 (NET-07/08, PWR-02, BT-03, SND-03 explicitly out of scope)
- `.planning/ROADMAP.md` — Phase 4 goal + 5 success criteria
- `.planning/phases/01-foundations-safety-rails/01-CONTEXT.md` — locked D-01..D-15 (single binary, FR/EN split, pkexec-only, i18n keys)
- `.planning/phases/02-system-insight/02-CONTEXT.md` — CLI shape precedent (`sys` group, French tables, global `--json`, snapshot + `--watch`)
- `.planning/phases/03-packages-cli-harness/03-CONTEXT.md` — confirmation-before-mutation precedent (D-01), streaming output, N/A honesty

## Existing Code Insights

### Reusable Assets
- `crates/archel-core/src/privileged.rs` (spawn/parse/map, 126/127 → AuthFailed) — if any network write needs elevation (researcher decides NM auth path)
- `crates/archel-core/src/updates.rs` (which-gated duct spawn + LC_ALL=C + timeout) — pattern for subprocess probes (ping, etc.)
- `crates/archel/src/cmds/` (clap derive groups + French tables + global `--json` + insta pins) — where `net`/`bt`/audio/power groups plug in
- `crates/archel-core/src/error.rs` + `fr.json` — new typed variants + catalog keys for every user-visible string

### Established Patterns
- Reads unprivileged in-process; mutations confirmed `[o/N]` then executed; privileged mutations via helper
- D-Bus via zbus 5.x is the blessed stack (STACK.md: NetworkManager, power-profiles-daemon, WirePlumber proxies); bluer 0.17 for Bluetooth; pipewire-rs + WP D-Bus for audio
- Headless `--no-default-features` gates; no Qt code outside `gui/`

### Integration Points
- New core modules (e.g. `network.rs`, `bluetooth.rs`, `audio.rs`, `power.rs`) exposed through `lib.rs`, consumed by new CLI groups
- NM writes may need auth — researcher determines user-session vs pkexec-helper path

## Specific Ideas

- User's proxy ask: "propose moi la maniere la plus ergonomique intuitive et fluide" — answered with GNOME-like 3-mode proposal, approved as-is
- User's VPN ask: "plusieurs protocoles comme openvpn ou wireguard etc... une fenetre popup qui m'affiche un formulaire, mdp utilisateurs certificat cle etc... ou meme fichier config" — multi-protocol + file import locked for CLI; popup form recorded for GUI phase
- Streaming/preview habits from Phase 3 (live output, confirm-with-deps) carry over to mutation flows here

## Deferred Ideas

- VPN popup GUI form (username/password/certificate/key/file) — GUI phase, fields recorded above
- Authenticated-proxy credentials (user/password) — user asked whether to add; left open for researcher (only if trivially supported by chosen mechanism)

---

*Phase: 4-Connectivity & Devices*
*Context gathered: 2026-10-06*
