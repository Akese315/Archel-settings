# Archel

## What This Is

A system settings suite for a personal Arch Linux (Hyprland) distro, built in Rust with a Qt6 GUI. It is split into a shared settings **API** layer that owns all system integrations, with two frontends on top: `archel-cli` (command-line access to every capability) and `archel settings` (a modern, user-friendly GUI with vertical tabs). The API is built first; the GUI and CLI are thin consumers of it.

## Core Value

One reliable API through which every system setting (network, power, bluetooth, appearance, audio, packages, display, input, system stats) can be read and changed on Arch + Hyprland — proven via the CLI before the GUI ever draws it.

## Requirements

### Validated

(None yet — ship to validate)

### Active

**Landing / Overview (home page)**
- [ ] Home page shows the app version, running kernel version, and the number of packages with available updates (pacman repo + yay/AUR)

**Network (Réseau)**
- [ ] User can see connection state, IP address, and gateway
- [ ] User can configure DNS servers
- [ ] User can configure proxy settings
- [ ] User can manage VPN connections
- [ ] User can run network tests (e.g. connectivity/latency checks)

**Power (Énergie)**
- [ ] User can switch power profiles: performance, balanced, power saver

**Bluetooth**
- [ ] User can view, pair, connect, and disconnect Bluetooth devices

**Appearance (Apparence)**
- [ ] User can set the wallpaper
- [ ] User can change the system font
- [ ] User can configure blur and transparency (Hyprland blur/opacity)

**Sound (Sons)**
- [ ] User can select the audio output device
- [ ] User can select the audio input device

**Applications / Packages**
- [ ] User can view installed packages and search them (pacman)
- [ ] User can install, remove, and update packages via pacman
- [ ] User can manage AUR packages via yay
- [ ] User can see which packages are updatable and trigger updates

**Display (Écran)**
- [ ] User can change resolution, scaling, and refresh rate per monitor
- [ ] User can arrange monitor position/layout

**Input**
- [ ] User can view and edit keyboard shortcuts (Hyprland keybinds)
- [ ] User can configure mouse sensitivity and scrolling behavior

**System Information**
- [ ] User can see RAM and disk usage
- [ ] User can see live CPU usage per core (graph) and current clock (GHz)
- [ ] User can see GPU VRAM used and GPU/CPU temperature
- [ ] User can see real-time CPU and GPU power consumption
- [ ] User can see disk usage

**Platform / Architecture**
- [ ] A single settings API exposes every capability above as a programmatic interface
- [ ] `archel-cli` exposes the full API from the command line
- [ ] `archel settings` GUI presents the API with vertical tabs, one tab per domain above (minimum tab set)

### Out of Scope

- Other desktop environments (KDE, GNOME, i3, etc.) — target is Arch + Hyprland only
- Non-Arch package managers (apt, dnf, flatpak, snap) — pacman + yay only
- User account / sudo / polkit administration UI — out of scope for v1 settings surface
- Themes/icon-pack marketplace or community config sharing — config editing only
- systemd unit editor, cron editor, service manager — full system admin creep, not settings

## Context

- **Target machine**: personal Arch Linux distro ("archel"), Hyprland compositor (Wayland). Hostname `archel`, user `axel`.
- **Hyprland implications**: blur/transparency live in `hyprland.conf` (decoration/blur settings, window rules); wallpaper via hyprpaper/hyprpaper-style utility; keybinds and mouse settings also live in `hyprland.conf`; monitor config via `hyprctl` / wlr-output-management protocol.
- **Expected service integrations** (to confirm in research): NetworkManager (via D-Bus/nmcli) for network+VPN, power-profiles-daemon or TLP for power profiles, BlueZ (D-Bus) for Bluetooth, PipeWire/WirePlumber for audio, pacman + yay for packages.
- **System stats needs live sampling**: per-core CPU graphs, GHz, temps, power draw, VRAM — implies a metrics sampling layer (sysfs/hwmon, nvml/rocm or amd equivalent, /proc, nvtop-style data sources).
- **Build order is explicit**: API first, then GUI on top of it. `archel-cli` doubles as the API's test harness.
- User language: French/English mixed; UI copy language to be decided (French labels appear in the original spec — e.g. "Réseau", "Énergie").

## Constraints

- **Tech stack**: Qt6 for the GUI — the user asked for Rust "if possible"; Qt-from-Rust options (Qt for Rust/Qt Rust bindings, cxx-bridge to C++ Qt, or QML-from-Rust) must be validated in research before committing
- **Platform**: Arch Linux + Hyprland (Wayland) — X11-only tools are not acceptable without verification
- **Privilege model**: package operations (pacman/yay) and some network settings require root — privilege escalation strategy (pkexec / sudo / archel daemon) must be decided early
- **Ordering**: the API must exist and be usable via `archel-cli` before the GUI phase begins
- **GUI UX**: modern, user-friendly, vertical tab layout; minimum tab set is fixed (Landing, Network, Power, Bluetooth, Appearance, Sound, Apps/Packages, Display, Input, System Info)

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust + Qt6 for GUI | User preference for Rust; Qt6 requested for modern look | — Pending |
| API-first architecture (`archel-cli` + `archel settings` share one API) | User explicitly wants an API for everything, then the GUI; CLI serves as verification harness | — Pending |
| Target Arch + Hyprland only | Personal distro; keeps appearance/input integrations concrete (hyprland.conf, hyprctl) | — Pending |
| Vertical tabs, minimum 9-tab layout | User's explicit UX requirement ("Les onglets doivent être verticaux") | — Pending |
| pacman + yay as the only package front-ends | Matches user's AUR workflow | — Pending |
| Standard granularity, parallel plan execution, research/plan-check/verifier all on | Configured during initialization (auto mode) | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-09-30 after initialization*
