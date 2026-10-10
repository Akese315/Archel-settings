# Archel

## What This Is

A system settings suite for a personal Arch Linux (Hyprland) distro, built in Rust with a Qt6 GUI. It is split into a shared settings **API** layer (`archel-core`) that owns all system integrations, with two frontends on top: the `archel` CLI (command-line access to every capability) and `archel settings` (a modern, user-friendly GUI with vertical tabs, CXX-Qt/QML). The API was built first and proven via the CLI; the GUI and CLI are thin consumers of it. Shipped as v1.0 (2026-10-11).

## Core Value

One reliable API through which every system setting (network, power, bluetooth, appearance, audio, packages, display, input, system stats) can be read and changed on Arch + Hyprland — proven via the CLI before the GUI ever draws it.

## Requirements

### Validated

All v1 requirements shipped and verified in v1.0 (2026-10-11; tag `v1.0`; 43/43 in the archived `.planning/milestones/v1.0-REQUIREMENTS.md`, per-phase evidence in `.planning/milestones/v1.0-phases/*/…-VERIFICATION.md`):

- ✓ Home page shows app version, running kernel, and pending update counts (pacman repo + yay/AUR) — v1.0
- ✓ Connection state, IP address, and gateway; DNS, proxy, VPN management; network tests — v1.0
- ✓ Power profile switching: performance / balanced / power saver — v1.0
- ✓ Bluetooth view / pair / connect / disconnect — v1.0
- ✓ Wallpaper, system font, blur/transparency (Hyprland) — v1.0
- ✓ Audio output + input device selection — v1.0
- ✓ Installed package view + search (pacman); install / remove / update via pacman through the pkexec helper; AUR via yay with review-before-build; updatable list + upgrade trigger — v1.0
- ✓ Resolution, scaling, refresh rate per monitor; monitor layout — v1.0
- ✓ Hyprland keybinds editor; mouse sensitivity and scrolling — v1.0
- ✓ RAM/disk usage, per-core CPU + clock, VRAM + CPU/GPU temps, CPU/GPU power draw (honest N/A where root-gated) — v1.0
- ✓ Single settings API (`archel-core`) exposes every capability; `archel` CLI exposes the full API; `archel settings` GUI with vertical tabs over the proven API — v1.0
- ✓ French-first localized UI (bonus over spec) + shell completions — v1.0

### Active

v1.1 candidates (carried from v1.0 close — see STATE.md `## Deferred Items` and per-phase `deferred-items.md` ledgers):

**Live-session verification (needs user's Hyprland session)**
- [ ] UAT-confirm the ~13 deferred live items: phase-05 display/input/appearance live checks, pkexec escalation dialog, GUI visual pass, VPN `down` re-test after the 04-05 fix

**GUI polish (acknowledged code-review debt)**
- [ ] WR-01: decouple the 1 Hz tick from in-progress Input tab edits (local write-slot state like NetTab)
- [ ] WR-02: FileDialog percent-encoded path handling (`toLocalFile`) for VPN + wallpaper pickers
- [ ] WR-03: per-tab confirm sessions instead of one global confirm multiplexer
- [ ] WR-04: persist-then-eval (or compensating unbind) for keybind-add
- [ ] WR-05: GUI affordance for automatic DNS (maps the CLI `reset` leg)
- [ ] camelCase/snake_case QML binding convention — adopt a hard camelCase style rule or move to `Q_PROPERTY`-style names (mirrored as the open `.planning/WINDOWS.md` ship-gate)

**Hygiene**
- [ ] Clear the 29 pre-existing `archel-core` clippy findings so `-D warnings` is clean repo-wide
- [ ] Stabilize `live_*` timing tests under heavy parallel load (duration thresholds)
- [ ] Nyquist validation draft debt for phases 01–06 (`/gsd-validate-phase`)

### Out of Scope

- Other desktop environments (KDE, GNOME, i3, etc.) — target is Arch + Hyprland only
- Non-Arch package managers (apt, dnf, flatpak, snap) — pacman + yay only
- User account / sudo / polkit administration UI — out of scope for v1 settings surface
- Themes/icon-pack marketplace or community config sharing — config editing only
- systemd unit editor, cron editor, service manager — full system admin creep, not settings
- Reimplementing alpm transactions in Rust — confirmed in v1: subprocess `pacman`/`yay` inherits hooks, signing, and progress semantics for free; alpm stays read-side only
- Background daemon / event subscriptions (tray, auto-update checks) — confirmed in v1: snapshot polling (1 Hz GUI ticker) meets requirements; revisit only in v2

## Context

- **Target machine**: personal Arch Linux distro ("archel"), Hyprland compositor (Wayland). Hostname `archel`, user `axel`.
- **v1.0 shipped 2026-10-11** (tag `v1.0`): 6 phases, 28 plans, 206 commits in ~11 days; 43/43 requirements; all phases `phase_complete` + `verification: passed`; closeout `override_closeout` (9 verified-fixed, 9 acknowledged as v1.1 debt).
- **Workspace as built**: cargo workspace (edition 2024) — `crates/archel-core` lib (~34.3k lines Rust incl. tests; 17 domain modules) + `crates/archel` bin (CLI `archel` + CXX-Qt GUI, ~3.1k lines QML: Main shell + 13 tab files).
- **Stack as built**: tokio + zbus 5 (NetworkManager, power-profiles-daemon, UPower), bluer (BlueZ), hyprland-rs 0.3 IPC, alpm read-only + pacman/yay subprocess mutations, pipewire + `wpctl` fallback, custom hwmon/RAPL sysfs sensors + nvml-wrapper (dlopen-gated), serde/toml, tracing, insta snapshots for conf/CLI regressions.
- **GUI**: CXX-Qt 0.10 — one `#[qobject]` Model, snake_case Q_PROPERTYs (QML binds camelCase), confirm-token mutation pattern, 1 Hz/500 ms tickers; French-first i18n via rust-i18n catalogs.
- **Privilege model (delivered)**: allowlisted pkexec helper (`archel-privileged`, v1 JSON protocol, D-12 exit-code mapping) + least-privilege polkit policy; typed `NeedsPrivilege` errors so both frontends render escalation uniformly. Archel never runs as root.
- **Tests**: 278 core + 125 bin tests green (`--offline`); insta-pinned `--json` schemas.
- **Known debt**: the Active list above; 9 items formally acknowledged at close via the audit-open writer (self-invalidating if artifacts change).

## Constraints

- **Tech stack**: Qt6 via CXX-Qt 0.10 + QML — delivered and proven on Arch Qt 6.11; escape hatch (Slint) retired after the Phase-1 spike gate passed.
- **Platform**: Arch Linux + Hyprland (Wayland) — no X11-only tools anywhere.
- **Privilege model**: pkexec + polkit allowlist through the dedicated `archel-privileged` helper; API returns typed `NeedsPrivilege` for frontends to render.
- **Offline machine**: no network at build/test time — `cargo test --offline` is mandatory; live HTTP is allowed only at runtime.
- **GUI UX**: modern, user-friendly, vertical tab layout; minimum tab set delivered (Landing, Network, Power, Bluetooth, Appearance, Sound, Apps/Packages, Display, Input, System Info).

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust + Qt6 for GUI | User preference for Rust; Qt6 requested for modern look | ✓ Good — shipped via CXX-Qt 0.10/QML on Arch Qt 6.11 |
| API-first architecture (`archel` CLI + `archel settings` GUI share one API) | User explicitly wants an API for everything, then the GUI; CLI serves as verification harness | ✓ Good — CLI-proven API let GUI land in 8 plans with zero rework |
| Target Arch + Hyprland only | Personal distro; keeps appearance/input integrations concrete (hyprland.conf, hyprctl) | ✓ Good |
| Vertical tabs, minimum 9-tab layout | User's explicit UX requirement ("Les onglets doivent être verticaux") | ✓ Good — 10-tab shell shipped |
| pacman + yay as the only package front-ends | Matches user's AUR workflow | ✓ Good — alpm read-side + subprocess mutations, review-before-build (D-05) |
| Standard granularity, parallel plan execution, research/plan-check/verifier all on | Configured during initialization (auto mode) | ✓ Good — 28 plans, wave-parallel execution; verifier caught real gaps (G-04-2, GAP-1) |
| pkexec + polkit helper over daemon/setuid | D-12/D-13: never run archel as root; typed NeedsPrivilege | ✓ Good — helper + policy shipped and tested |
| Line-preserving `hyprland.conf` block editor (snippet files) | D-09/D-26: user annotations/comments must survive writes | ✓ Good — atomic, validated, watch-driven reload |
| Snapshot polling over D-Bus/PipeWire signal subscriptions | D-11/D-05: simpler lifecycle, matches CLI parity | ⚠️ Revisit — GUI feels tick-adjacent (1 Hz caps); event-driven updates would cut latency and CPU |
| CXX-Qt snake_case Q_PROPERTYs bound as camelCase in QML | CXX-Qt codegen choice | ⚠️ Revisit — works via Qt case-folding but 5 code-review findings wrote binding bugs into QML; Windows-gate open, style rule or rename planned v1.1 |

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
*Last updated: 2026-10-11 after v1.0 milestone completion (full section review)*
