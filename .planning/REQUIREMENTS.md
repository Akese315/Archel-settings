# Requirements: Archel

**Defined:** 2026-09-30
**Core Value:** One reliable API through which every system setting can be read and changed on Arch + Hyprland — proven via the CLI before the GUI ever draws it.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Landing

- [x] **LAND-01**: User sees the app version on the home page
- [x] **LAND-02**: User sees the running kernel version on the home page
- [x] **LAND-03**: User sees the number of packages with available updates (pacman repo + yay/AUR) on the home page

### Network

- [x] **NET-01**: User can see connection state (connected/disconnected, interface, SSID/wired)
- [x] **NET-02**: User can see IP address and gateway of the active connection
- [x] **NET-03**: User can configure DNS servers per NetworkManager connection profile
- [x] **NET-04**: User can configure proxy settings (system/session layer, separate from NM)
- [x] **NET-05**: User can manage VPN connections (list, connect, disconnect via NetworkManager)
- [x] **NET-06**: User can run network tests (ping gateway/DNS, DNS resolution check, latency)

### Power

- [x] **PWR-01**: User can switch power profile between performance, balanced, and power-saver (power-profiles-daemon)

### Bluetooth

- [x] **BT-01**: User can scan and list nearby Bluetooth devices
- [x] **BT-02**: User can pair, connect, and disconnect Bluetooth devices

### Appearance

- [x] **APP-01**: User can set the desktop wallpaper (Hyprland backend: hyprpaper/swww, decided in stack spike)
- [x] **APP-02**: User can change the system font
- [x] **APP-03**: User can configure window blur (Hyprland decoration settings)
- [x] **APP-04**: User can configure window transparency/opacity (Hyprland rules)

### Sound

- [x] **SND-01**: User can select the audio output device (PipeWire/WirePlumber default sink)
- [x] **SND-02**: User can select the audio input device (PipeWire/WirePlumber default source)

### Packages

- [x] **PKG-01**: User can list and search installed packages (pacman metadata, read-only)
- [x] **PKG-02**: User can install packages via pacman (through privileged helper)
- [x] **PKG-03**: User can remove packages via pacman (through privileged helper)
- [x] **PKG-04**: User can update system packages (full `-Syu`, never `-Sy` alone)
- [x] **PKG-05**: User can manage AUR packages via yay (run as user, with AUR review step)

### Display

- [x] **DSP-01**: User can change monitor resolution (applied with confirm-and-rollback)
- [x] **DSP-02**: User can change monitor scaling per monitor
- [x] **DSP-03**: User can change monitor refresh rate
- [x] **DSP-04**: User can arrange monitor position/layout

### Input

- [x] **INP-01**: User can view current keyboard shortcuts (Hyprland binds)
- [x] **INP-02**: User can add, edit, and remove keyboard shortcuts
- [x] **INP-03**: User can configure mouse sensitivity
- [x] **INP-04**: User can configure mouse scrolling behavior

### System Info

- [x] **SYS-01**: User can see RAM usage (used/total)
- [x] **SYS-02**: User can see disk usage per mount
- [x] **SYS-03**: User can see live per-core CPU usage graph
- [x] **SYS-04**: User can see current CPU clock speed (GHz)
- [x] **SYS-05**: User can see GPU VRAM used/total
- [x] **SYS-06**: User can see CPU and GPU temperature
- [x] **SYS-07**: User can see real-time CPU power consumption (W)
- [x] **SYS-08**: User can see real-time GPU power consumption (W)

### Platform

- [x] **PLAT-01**: A shared Rust API library (`archel-core`) exposes every domain above as a programmatic interface
- [x] **PLAT-02**: `archel-cli` exposes the full API surface from the command line (doubles as API test harness)
- [x] **PLAT-03**: `archel settings` GUI presents the API with vertical tabs, one tab per domain (Landing + 9 domains)
- [x] **PLAT-04**: Privileged operations (pacman transactions) go through an allowlisted pkexec helper; yay always runs unprivileged

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Network

- **NET-07**: User can run bandwidth speed tests with history graph
- **NET-08**: User gets automated DNS-leak detection on VPN connections

### Power

- **PWR-02**: User can fine-tune power settings (CPU governor, frequency caps) beyond the three profiles

### Bluetooth

- **BT-03**: User can see Bluetooth device battery levels

### Sound

- **SND-03**: User can adjust per-application volume mixer

### Packages

- **PKG-06**: User can review AUR PKGBUILD diffs inside the GUI before building

### Display

- **DSP-05**: User can schedule night-light / blue-light filter

### System Info

- **SYS-09**: User can see historical graphs (power, temperature over time) and export them

### Platform

- **PLAT-05**: User can switch UI language (French/English)

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Other desktop environments (KDE, GNOME, i3, Sway) | Personal Arch + Hyprland distro only; keeps integrations concrete |
| Non-Arch package managers (apt, dnf, flatpak, snap) | pacman + yay match the user's workflow |
| User account / sudo / polkit administration UI | System-admin creep, not a settings surface |
| systemd unit / cron / service manager | Full admin tooling, out of scope for v1 settings |
| Theme/icon-pack marketplace or config sharing | Config editing only in v1 |
| Resident root daemon for privileged ops | pkexec one-shot helper is sufficient and safer for v1 |
| Keybind editing beyond Hyprland binds | Deferred with Input editing scope; compositor-agnostic editing is v2+ |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| LAND-01 | Phase 2 | Complete |
| LAND-02 | Phase 2 | Complete |
| LAND-03 | Phase 2 | Complete |
| NET-01 | Phase 4 | Complete |
| NET-02 | Phase 4 | Complete |
| NET-03 | Phase 4 | Complete |
| NET-04 | Phase 4 | Complete |
| NET-05 | Phase 4 | Complete |
| NET-06 | Phase 4 | Complete |
| PWR-01 | Phase 4 | Complete |
| BT-01 | Phase 4 | Complete |
| BT-02 | Phase 4 | Complete |
| APP-01 | Phase 5 | Pending phase verification |
| APP-02 | Phase 5 | Pending phase verification |
| APP-03 | Phase 5 | Pending phase verification |
| APP-04 | Phase 5 | Pending phase verification |
| SND-01 | Phase 4 | Complete |
| SND-02 | Phase 4 | Complete |
| PKG-01 | Phase 3 | Complete |
| PKG-02 | Phase 3 | Complete |
| PKG-03 | Phase 3 | Complete |
| PKG-04 | Phase 3 | Complete |
| PKG-05 | Phase 3 | Complete |
| DSP-01 | Phase 5 | Pending phase verification |
| DSP-02 | Phase 5 | Pending phase verification |
| DSP-03 | Phase 5 | Pending phase verification |
| DSP-04 | Phase 5 | Pending phase verification |
| INP-01 | Phase 5 | Pending phase verification |
| INP-02 | Phase 5 | Pending phase verification |
| INP-03 | Phase 5 | Pending phase verification |
| INP-04 | Phase 5 | Pending phase verification |
| SYS-01 | Phase 2 | Complete |
| SYS-02 | Phase 2 | Complete |
| SYS-03 | Phase 2 | Complete |
| SYS-04 | Phase 2 | Complete |
| SYS-05 | Phase 2 | Complete |
| SYS-06 | Phase 2 | Complete |
| SYS-07 | Phase 2 | Complete |
| SYS-08 | Phase 2 | Complete |
| PLAT-01 | Phase 1 | Complete |
| PLAT-02 | Phase 3 | Complete |
| PLAT-03 | Phase 6 | Complete |
| PLAT-04 | Phase 1 | Complete |

**Coverage:**
- v1 requirements: 43 total
- Mapped to phases: 43 (roadmap 2026-09-30)
- Unmapped: 0

---
*Requirements defined: 2026-09-30*
*Last updated: 2026-10-10 after v1.0 milestone audit (3-source cross-reference)*
