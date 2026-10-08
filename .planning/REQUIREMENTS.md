# Requirements: Archel

**Defined:** 2026-09-30
**Core Value:** One reliable API through which every system setting can be read and changed on Arch + Hyprland — proven via the CLI before the GUI ever draws it.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Landing

- [ ] **LAND-01**: User sees the app version on the home page
- [ ] **LAND-02**: User sees the running kernel version on the home page
- [ ] **LAND-03**: User sees the number of packages with available updates (pacman repo + yay/AUR) on the home page

### Network

- [ ] **NET-01**: User can see connection state (connected/disconnected, interface, SSID/wired)
- [ ] **NET-02**: User can see IP address and gateway of the active connection
- [ ] **NET-03**: User can configure DNS servers per NetworkManager connection profile
- [ ] **NET-04**: User can configure proxy settings (system/session layer, separate from NM)
- [ ] **NET-05**: User can manage VPN connections (list, connect, disconnect via NetworkManager)
- [ ] **NET-06**: User can run network tests (ping gateway/DNS, DNS resolution check, latency)

### Power

- [ ] **PWR-01**: User can switch power profile between performance, balanced, and power-saver (power-profiles-daemon)

### Bluetooth

- [ ] **BT-01**: User can scan and list nearby Bluetooth devices
- [ ] **BT-02**: User can pair, connect, and disconnect Bluetooth devices

### Appearance

- [x] **APP-01**: User can set the desktop wallpaper (Hyprland backend: hyprpaper/swww, decided in stack spike)
- [x] **APP-02**: User can change the system font
- [x] **APP-03**: User can configure window blur (Hyprland decoration settings)
- [x] **APP-04**: User can configure window transparency/opacity (Hyprland rules)

### Sound

- [ ] **SND-01**: User can select the audio output device (PipeWire/WirePlumber default sink)
- [ ] **SND-02**: User can select the audio input device (PipeWire/WirePlumber default source)

### Packages

- [ ] **PKG-01**: User can list and search installed packages (pacman metadata, read-only)
- [ ] **PKG-02**: User can install packages via pacman (through privileged helper)
- [ ] **PKG-03**: User can remove packages via pacman (through privileged helper)
- [ ] **PKG-04**: User can update system packages (full `-Syu`, never `-Sy` alone)
- [ ] **PKG-05**: User can manage AUR packages via yay (run as user, with AUR review step)

### Display

- [ ] **DSP-01**: User can change monitor resolution (applied with confirm-and-rollback)
- [ ] **DSP-02**: User can change monitor scaling per monitor
- [ ] **DSP-03**: User can change monitor refresh rate
- [ ] **DSP-04**: User can arrange monitor position/layout

### Input

- [ ] **INP-01**: User can view current keyboard shortcuts (Hyprland binds)
- [ ] **INP-02**: User can add, edit, and remove keyboard shortcuts
- [ ] **INP-03**: User can configure mouse sensitivity
- [ ] **INP-04**: User can configure mouse scrolling behavior

### System Info

- [ ] **SYS-01**: User can see RAM usage (used/total)
- [ ] **SYS-02**: User can see disk usage per mount
- [ ] **SYS-03**: User can see live per-core CPU usage graph
- [ ] **SYS-04**: User can see current CPU clock speed (GHz)
- [ ] **SYS-05**: User can see GPU VRAM used/total
- [ ] **SYS-06**: User can see CPU and GPU temperature
- [ ] **SYS-07**: User can see real-time CPU power consumption (W)
- [ ] **SYS-08**: User can see real-time GPU power consumption (W)

### Platform

- [ ] **PLAT-01**: A shared Rust API library (`archel-core`) exposes every domain above as a programmatic interface
- [ ] **PLAT-02**: `archel-cli` exposes the full API surface from the command line (doubles as API test harness)
- [ ] **PLAT-03**: `archel settings` GUI presents the API with vertical tabs, one tab per domain (Landing + 9 domains)
- [ ] **PLAT-04**: Privileged operations (pacman transactions) go through an allowlisted pkexec helper; yay always runs unprivileged

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
| LAND-01 | Phase 2 | Pending |
| LAND-02 | Phase 2 | Pending |
| LAND-03 | Phase 2 | Pending |
| NET-01 | Phase 4 | Complete |
| NET-02 | Phase 4 | Complete |
| NET-03 | Phase 4 | Complete |
| NET-04 | Phase 4 | Complete |
| NET-05 | Phase 4 | Complete |
| NET-06 | Phase 4 | Complete |
| PWR-01 | Phase 4 | Complete |
| BT-01 | Phase 4 | Complete |
| BT-02 | Phase 4 | Complete |
| APP-01 | Phase 5 | Complete |
| APP-02 | Phase 5 | Complete |
| APP-03 | Phase 5 | Complete |
| APP-04 | Phase 5 | Complete |
| SND-01 | Phase 4 | Complete |
| SND-02 | Phase 4 | Complete |
| PKG-01 | Phase 3 | Pending |
| PKG-02 | Phase 3 | Pending |
| PKG-03 | Phase 3 | Pending |
| PKG-04 | Phase 3 | Pending |
| PKG-05 | Phase 3 | Pending |
| DSP-01 | Phase 5 | Pending |
| DSP-02 | Phase 5 | Pending |
| DSP-03 | Phase 5 | Pending |
| DSP-04 | Phase 5 | Pending |
| INP-01 | Phase 5 | Pending |
| INP-02 | Phase 5 | Pending |
| INP-03 | Phase 5 | Pending |
| INP-04 | Phase 5 | Pending |
| SYS-01 | Phase 2 | Pending |
| SYS-02 | Phase 2 | Pending |
| SYS-03 | Phase 2 | Pending |
| SYS-04 | Phase 2 | Pending |
| SYS-05 | Phase 2 | Pending |
| SYS-06 | Phase 2 | Pending |
| SYS-07 | Phase 2 | Pending |
| SYS-08 | Phase 2 | Pending |
| PLAT-01 | Phase 1 | Pending |
| PLAT-02 | Phase 3 | Pending |
| PLAT-03 | Phase 6 | Pending |
| PLAT-04 | Phase 1 | Pending |

**Coverage:**
- v1 requirements: 43 total
- Mapped to phases: 43 (roadmap 2026-09-30)
- Unmapped: 0

---
*Requirements defined: 2026-09-30*
*Last updated: 2026-09-30 after initial definition*
