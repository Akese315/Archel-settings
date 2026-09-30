# Roadmap: Archel

## Overview

API first, GUI last: foundations and safety rails (workspace, error model, privilege helper, Qt spike gate), then end-to-end domain slices each proven through `archel-cli` — system insight, packages, connectivity/devices, Hyprland desktop — closing with the Qt6/QML GUI that composes the already-proven API into vertical tabs.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Foundations & Safety Rails** - Workspace, error model, privilege helper skeleton, Qt spike gate
- [ ] **Phase 2: System Insight** - Landing dashboard data, live stats sampler, system facts
- [ ] **Phase 3: Packages & CLI Harness** - pacman/yay management plus full CLI surface for proven domains
- [ ] **Phase 4: Connectivity & Devices** - Network, Bluetooth, sound, and power control
- [ ] **Phase 5: Hyprland Desktop** - Appearance, display, and input via snippet-file writes
- [ ] **Phase 6: GUI Shell & Tabs** - archel-settings vertical-tab GUI over the proven API

## Phase Details

### Phase 1: Foundations & Safety Rails
**Goal**: All downstream work builds on decided foundations — workspace compiles, privilege boundary is designed, Qt viability is proven or rejected
**Mode:** mvp
**Depends on**: Nothing (first phase)
**Requirements**: PLAT-01, PLAT-04
**Success Criteria** (what must be TRUE):
  1. User can build the workspace (`archel-core`, `archel-cli`, `archel-settings`, `archel-privileged` skeletons) with one cargo command
  2. User can run a spike GUI window with vertical tabs and a live-updating label (or sees an explicit Slint pivot decision)
  3. User can invoke the pkexec helper skeleton for an allowlisted dry-run action and gets structured JSON back
  4. User gets typed `ArchelError` diagnostics (not panics) from any core failure path
**Plans**: 2 plans

Plans:
- [x] 01-01-PLAN.md — Core + CLI + i18n + typed errors tracer-first (PLAT-01)
- [x] 01-02-PLAN.md — Privileged helper + policy + CXX-Qt spike (PLAT-04)

### Phase 2: System Insight
**Goal**: Users can see at a glance what their machine is doing — landing facts plus live system telemetry
**Mode:** mvp
**Depends on**: Phase 1
**Requirements**: LAND-01, LAND-02, LAND-03, SYS-01, SYS-02, SYS-03, SYS-04, SYS-05, SYS-06, SYS-07, SYS-08
**Success Criteria** (what must be TRUE):
  1. User can see app version, running kernel version, and pending update count (pacman + AUR) in one landing snapshot
  2. User can see RAM usage, disk usage per mount, and live per-core CPU graph with current clock speed
  3. User can see GPU VRAM used/total, CPU/GPU temperatures, and real-time CPU/GPU power draw — with honest N/A where hardware has no sensor
  4. User can fetch every value above headlessly via `archel-cli` (JSON output), proving the read API before any GUI exists
**Plans**: TBD

### Phase 3: Packages & CLI Harness
**Goal**: Users can manage system and AUR packages end-to-end, and every proven API domain is drivable from the command line
**Mode:** mvp
**Depends on**: Phase 2
**Requirements**: PKG-01, PKG-02, PKG-03, PKG-04, PKG-05, PLAT-02
**Success Criteria** (what must be TRUE):
  1. User can list and search installed packages from the terminal
  2. User can install, remove, and fully upgrade (`-Syu`) system packages through the pkexec helper without ever running the app as root
  3. User can manage AUR packages via yay running unprivileged, with a PKGBUILD review step before building
  4. User can drive every Phase 2–3 capability through `archel-cli` with `--json` output and shell completions
**Plans**: TBD

### Phase 4: Connectivity & Devices
**Goal**: Users can inspect and control network, Bluetooth, audio, and power from one API surface
**Mode:** mvp
**Depends on**: Phase 3
**Requirements**: NET-01, NET-02, NET-03, NET-04, NET-05, NET-06, BT-01, BT-02, SND-01, SND-02, PWR-01
**Success Criteria** (what must be TRUE):
  1. User can see connection state, IP/gateway, and effective DNS, and can change DNS per NetworkManager profile, manage VPN connections, and run connectivity/latency self-tests
  2. User can configure proxy settings at the session layer, independent of NetworkManager profiles
  3. User can scan, pair, connect, and disconnect Bluetooth devices including a working pairing flow
  4. User can select audio output and input devices and switch power profiles (performance/balanced/power-saver)
  5. User can perform every action above through `archel-cli`, proving each write path before GUI work starts
**Plans**: TBD

### Phase 5: Hyprland Desktop
**Goal**: Users can personalize and configure their Hyprland desktop — look, monitors, and inputs — safely
**Mode:** mvp
**Depends on**: Phase 4
**Requirements**: APP-01, APP-02, APP-03, APP-04, DSP-01, DSP-02, DSP-03, DSP-04, INP-01, INP-02, INP-03, INP-04
**Success Criteria** (what must be TRUE):
  1. User can set wallpaper, change system font, and adjust window blur and transparency with live-apply
  2. User can change monitor resolution, scaling, refresh rate, and layout — with a confirm-and-rollback dialog on apply
  3. User can view keyboard shortcuts and add, edit, or remove them, plus configure mouse sensitivity and scrolling
  4. User's main hyprland.conf is never rewritten — all persistence lands in the app-owned `source=` snippet with backup, and every action above works via `archel-cli`
**Plans**: TBD

### Phase 6: GUI Shell & Tabs
**Goal**: Users get a modern vertical-tab GUI presenting every proven capability
**Mode:** mvp
**Depends on**: Phase 5
**Requirements**: PLAT-03
**Success Criteria** (what must be TRUE):
  1. User can open `archel settings` and navigate Landing plus all 9 domain tabs in a vertical-tab layout
  2. User can see live system graphs that update at 1 Hz without freezing the interface
  3. User can perform every write action (network, packages with progress + cancel, appearance, display with confirm dialog, input) from the GUI with spinner/disabled states and readable error messages
  4. User can do nothing in the GUI that isn't already CLI-proven — each tab is a thin view over the proven core
**Plans**: TBD
**UI hint**: yes

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundations & Safety Rails | 0/TBD | Not started | - |
| 2. System Insight | 0/TBD | Not started | - |
| 3. Packages & CLI Harness | 0/TBD | Not started | - |
| 4. Connectivity & Devices | 0/TBD | Not started | - |
| 5. Hyprland Desktop | 0/TBD | Not started | - |
| 6. GUI Shell & Tabs | 0/TBD | Not started | - |
