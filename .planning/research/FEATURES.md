# Feature Research: Archel (Arch + Hyprland Settings Suite)

**Domain:** Linux desktop system settings (GNOME Control Center / KDE System Settings class)
**Researched:** 2026-09-30
**Confidence:** MEDIUM (competitor behavior from stable, long-lived tools + ArchWiki verification of power-profiles-daemon; live web search degraded so no fresh release-note verification)

## Feature Landscape

### Table Stakes (Users Expect These)

Missing any of these makes the app feel broken vs. GNOME Settings / KDE System Settings. Every one maps to a tab the user already mandated.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Wi-Fi / wired connection list + connect/disconnect | GNOME "Wi-Fi" panel and nm-applet both do this; it is the #1 reason anyone opens a settings app | LOW | Thin frontend over NetworkManager D-Bus (`org.freedesktop.NetworkManager`). Never reimplement scanning — use `nmcli`/D-Bus. `networkmanager` + `nm-applet` already on most Arch systems. |
| IP address / gateway / connection state display | nm-applet "Connection Information" and GNOME Network panel show this; user explicitly requires it | LOW | Read-only NM D-Bus properties (`Ip4Config`). Trivial once NM integration exists. |
| DNS server configuration | GNOME Network panel (IPv4/IPv6 → DNS) and nm-connection-editor both expose per-connection DNS; user requires it | MEDIUM | Per-connection DNS lives in NM connection profiles (`ipv4.dns`). Overlaps with VPN profiles (VPN pushes its own DNS). Must decide: edit the active profile vs. global `systemd-resolved`. Recommend: per-connection via NM only. |
| Audio output/input device selection + per-device volume | `pavucontrol` and GNOME Sound panel do exactly this; user requires output/input selection | LOW | PipeWire via `wpctl` / `libpipewire`. Enumerate sinks/sources, set default. Do NOT build a mixer (per-app volumes) for v1 — pavucontrol already exists. |
| Bluetooth scan / pair / connect / disconnect / trust | GNOME Bluetooth panel and `blueman` do this; user requires view+pair+connect+disconnect | MEDIUM | BlueZ over D-Bus (`org.bluez`: Adapter1.StartDiscovery, Device1.Pair/Connect). Pairing needs an agent for PIN/passkey — the classic trap. Use `bluetui`-style flow or depend on `bluez-utils`. Auto-reconnect + audio profile (A2DP vs HSP) switching is where MEDIUM comes from. |
| Display resolution + refresh rate per monitor | GNOME Displays and KDE KScreen do this; user requires it | MEDIUM | On Hyprland: `hyprctl monitors` to read, `hyprctl keyword monitor` to apply. Modes come from the compositor, not xrandr. Persist into `hyprland.conf` `monitor=` lines. No KScreen equivalent on Hyprland — this is genuinely needed. |
| Monitor scaling + position/layout | GNOME Displays drag-to-arrange; KDE KScreen; user requires scaling + position | MEDIUM | Same `monitor=` line (`monitor=DP-1,2560x1440@144,0x0,1.25`). Fractional scaling on Wayland/Hyprland is a known sharp-edge (blurry XWayland). Arrangement UI = drag canvas writing offsets. |
| Wallpaper setting | nwg-look-adjacent tools, GNOME Background, hyprpanel wallpaper modules; user requires it | LOW | On Hyprland wallpaper is NOT a gsetting — it's `hyprpaper` (or `swww`/`mpvpaper`). Setting wallpaper = writing hyprpaper config + IPC reload. Per-monitor wallpapers multiply complexity slightly. |
| Keyboard shortcut viewing | GNOME Keyboard panel lists all shortcuts; Hyprland users constantly forget their own binds | MEDIUM | Parse `bind=` lines from `hyprland.conf` (+ sourced files). Read path is easy; the trap is `$mod` variable resolution and multi-file `source=` chains. Display-only for v1 is LOW; editing is HIGH (see Differentiators). |
| Mouse sensitivity + scroll behavior | GNOME Mouse & Touchpad panel; user requires sensitivity + scroll | LOW | Hyprland `input:` section (`sensitivity`, `scroll_factor`, `natural_scroll`, `touchpad:*`). Write via `hyprctl keyword` for live apply + persist to conf. |
| Installed-package list + search + install/remove/update (pacman) | pamac and octopi both do this; Arch users expect a GUI package view; user requires it | MEDIUM | Shell out to `pacman -Q`, `-Ss`, `-S/-R`. Requires privilege escalation for install/remove (see Dependencies: privilege model). Query path is LOW; transaction path with progress reporting is MEDIUM. |
| Updatable-package count + "update all" (repo + AUR) | pamac/octopi update badges; user requires count on Landing + trigger | MEDIUM | `checkupdates` (pacman) + `yay -Qua` (AUR). Counting must be async/cached — `yay -Qua` can take 10+s. Landing page must never block on this. |
| System info: CPU/RAM/disk overview | GNOME About + KDE Info Center; every settings suite has an About page; user requires Landing + System Info | LOW | Static facts: `/proc/cpuinfo`, `uname -r`, `free`, `df`, GPU via `lspci`/sysfs. Read-only, no daemon needed. |
| Power profile switch (performance/balanced/power-saver) | GNOME Power panel "Power Mode" does exactly this; user requires it | LOW | `power-profiles-daemon` over D-Bus (`net.hadess.PowerProfiles`: `ActiveProfile`). One D-Bus property write. LOW **iff** PPD is the chosen backend — do NOT also support TLP (they conflict; ArchWiki: run only one). Recommend PPD, hold `balanced` default. |

### Differentiators (Competitive Advantage)

Nobody in the Hyprland ecosystem does these well in one place. This is where Archel wins — aligned with the Core Value (one API for everything on Arch+Hyprland).

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Live per-core CPU graph + GHz | GNOME Mission Center extension / KDE System Monitor do this as separate apps; no settings suite bundles it. User requires it | MEDIUM | Sampling layer reading `/proc/stat` per core + `/proc/cpuinfo` MHz (or `scaling_cur_freq` per cpu in sysfs) at 1Hz. Needs a poller thread + ring buffer in the API, not the GUI — GUI just subscribes. Design the sampler once, reuse for temps/power/network. |
| GPU VRAM + temperature + real-time CPU/GPU power draw | `nvtop`/`btop` territory, never integrated into a settings app. User explicitly requires it — highest "wow" density on the System Info tab | HIGH | Vendor-fragmented: NVIDIA exposes power via NVML (`nvidia-smi`, `libnvidia-ml`); AMD via `hwmon` (`power1_average` under `/sys/class/hwmon` amdgpu) + `radeontop`-style VRAM (`mem_info_vram_used`); Intel via RAPL (`/sys/class/powercap/intel-rapl`). Must do GPU-vendor detection first and degrade gracefully (show "N/A — unsupported GPU" not a crash). Single hardest System Info feature; isolate behind a `gpu` provider trait. |
| Network self-tests (ping gateway, ping internet, DNS resolve check, speed hint) | Neither GNOME nor nm-applet offers diagnostics; users drop to terminal. User requires it | LOW | `ping -c4 <gateway>`, `ping -c4 1.1.1.1`, resolve test via `getent hosts`, latency display. Pure orchestration of existing tools; no daemon. Cheap to build, high perceived competence. |
| VPN management integrated with Network panel | GNOME buries VPN under Network → "+"; nm-connection-editor is a separate ugly GTK3 dialog. One clean VPN section inside Network is a real UX win | MEDIUM | VPN = NM connection profiles of type wireguard/openvpn (`nmcli connection import` / `vpn.data`). Needs at least WireGuard + OpenVPN plugin support (`networkmanager-wireguard`, `networkmanager-openvpn`). Import-from-file (.conf/.ovpn) is the killer sub-feature — LOW cost, huge value. |
| Proxy configuration (env + GNOME-style manual proxy) | GNOME Network has a Proxy section; Hyprland/Arch has nothing GUI. User requires it | LOW | No system daemon owns "the proxy" on Arch — write `http_proxy/https_proxy` to `/etc/environment` (system) and/or `gsettings`-equivalent per-app is out of scope. Recommend: system-wide env file + per-session export note. Keep it a simple manual form (host/port/bypass), not PAC/WPAD auto-config (that way lies HIGH complexity). |
| Hyprland keybind *editing* (not just viewing) | No GUI tool edits hyprland.conf binds safely; users hand-edit. Editing = genuine moat | HIGH | Requires: full `source=` chain resolution, `$var` expansion, preserving comments/formatting on write (round-trip parser, not regex replace), conflict detection (two actions, one bind), live apply via `hyprctl keyword bind`. Recommend v1 = view + "copy bind" + single-bind reassign for `bind=` SUPER+key lines only; full editor is v2+. |
| Hyprland blur/transparency controls | nwg-look handles GTK theme only; nothing GUI-edits Hyprland `decoration:` blur/opacity. User requires it | MEDIUM | `decoration:blur:enabled/size/passes`, `active_opacity`/`inactive_opacity`, `blurls=` layer rules. Live-apply via `hyprctl keyword`; persist to conf. Sliders + live preview = high delight, moderate cost. Couple with a "compositor reload" safety (validate before write; keep a backup of hyprland.conf). |
| System font selection (GTK + Qt + terminal coherence) | nwg-look edits GTK settings.ini only; KDE ignores GTK. One font picker writing GTK + Qt + fontconfig is an Arch-Hyprland gap | MEDIUM | Write `gtk-font-name` to `~/.config/gtk-3.0/settings.ini` + `~/.config/gtk-4.0`, Qt via `qt6ct`/`KDE_SESSION` fontconfig, `fc-list` for enumeration. The trap: three toolkits, three config locations, live apps need restart notice. Show "restart apps to apply" honestly. |
| AUR management via yay (search/install/update/orphans) | pamac supports AUR but pulls heavy deps (pamac is Manjaro-flavored, pulls its own polkit/tray stack); octopi is Qt but dated. A clean yay frontend is wanted by exactly this user profile | MEDIUM | `yay -Ss/-S/-R/-Qua`. Build prompts are the trap (PKGBUILD diff/review on install — yay is interactive; wrapping needs `--noconfirm` policy + showing the PKGBUILD diff before confirm). Orphan cleanup (`yay -Yc`) is a cheap, loved sub-feature. |
| Landing/overview dashboard (kernel, app version, update count, quick status) | GNOME has no landing page (opens on last panel); KDE has a welcome-ish home. A status dashboard is Archel's identity | LOW | Aggregates other tabs' data (kernel from System, updates from Packages, network state, power profile). Pure composition — but architecturally it forces the API to expose cheap cached getters for everything, which is good pressure on API design. |
| `archel-cli` exposing every capability | None of the competitors are API-first with a CLI twin. Scriptability (`archel-cli wifi list`, `archel-cli power set performance`) is unique | MEDIUM | Cost is architectural, not per-feature: every API method needs CLI arg parsing + JSON/text output. Pays back as the test harness (PROJECT.md build order). Enforce: no GUI feature without a CLI equivalent — keeps the API honest. |

### Anti-Features (Deliberately NOT Building)

Already partly captured in PROJECT.md Out of Scope — this table adds the *why problematic* rationale from competitor observation.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Full systemd unit editor / service manager | "It's system settings, manage services too" | Stray into sysadmin tooling; `systemctl` semantics (enable vs start, user vs system units, failed-state triage) are a whole product. KDE doesn't do it either | Deep-link/open `systemctl` in terminal for the rare case; at most show "required services running?" health dots (NM, PPD, bluetooth) on Landing |
| User account / sudo / polkit admin UI | GNOME Users panel exists, so parity tempts | User management on Arch (wheel group, sudoers, polkit rules) risks lockout bugs; explicitly out of scope for v1 | Document that Archel runs as the user + escalates per-operation; revisit only if multi-user need appears |
| Theme/icon-pack marketplace or community config sharing | Looks like differentiation ("ricing hub") | Moderation, trust (arbitrary CSS/config execution), maintenance sink; nwg-look succeeds precisely by NOT doing this | Config *editing* only (blur, opacity, font, wallpaper) — local files, no downloads |
| Flatpak/snap/appimage store section | "Universal app management" | Non-Arch packaging; user uses pacman+yay only. pamac's bulk comes from exactly this sprawl | pacman + yay only, stated in PROJECT.md |
| Per-application firewall editor (ufw/iptables GUI) | "Network tab should have firewall" | GNOME doesn't do it; firewall rule semantics + lockout risk (cut own SSH/WG access) deserve a dedicated tool, not a tab section | Show firewall status (active/inactive via `ufw status`) read-only; link to terminal for changes |
| Full network connection editor parity (every NM property: 802.1X, bridges, bonds, VLANs) | nm-connection-editor exposes ~everything | nm-connection-editor already exists and is fine for exotic setups; cloning its 30-tab dialog burns months | Cover Wi-Fi/ethernet/VPN/DNS/proxy (90% use); "Advanced…" button shells out to `nm-connection-editor` for the rest |
| Real-time process manager (task manager) | KDE has KSysGuard adjacent; System Info tempts it | Separate product (btop/mission-center exist); process kill semantics + authorization add risk | Resource graphs only; "Open btop" affordance |
| Email/calendar/online-accounts integration | GNOME Online Accounts exists | Needs per-provider OAuth (Google/Nextcloud), token storage (libsecret/keyring) — enormous surface, zero relevance to Hyprland settings | Skip entirely; not on any tab |
| Printer management | GNOME Printers panel exists | CUPS on a personal Hyprland rig is near-zero use; `system-config-printer` exists if needed | Skip for v1; cheapest P3 ever |
| Auto-theming / wallpaper-to-palette (Material You style) | Eye-candy differentiation | Requires palette extraction + coordinated writes to GTK/Qt/Hyprland/terminal configs; fragile across toolkits; pywal exists standalone | Manual accent control at most (v2+); ship curated static coherence first |

## Feature Dependencies

```
Landing dashboard
    ├──requires──> Packages update-count provider (async, cached)
    ├──requires──> System static facts (kernel, CPU model)
    └──requires──> Status getters (network state, power profile)

Network panel
    ├──requires──> NetworkManager D-Bus integration (foundation for ALL network features)
    │                   ├──enables──> Wi-Fi list/connect, IP/gateway display
    │                   ├──enables──> Per-connection DNS editing
    │                   └──enables──> VPN profiles (WireGuard/OpenVPN plugins installed)
    ├──requires──> Proxy writer (/etc/environment — needs privilege escalation)
    └──independent──> Network self-tests (only needs gateway IP + ping; works even if NM editing fails)

Power panel
    ├──requires──> power-profiles-daemon installed AND TLP/mask-conflict resolved
    └──conflicts──> TLP / laptop-mode-tools (run exactly one; PPD recommended)

Bluetooth panel
    ├──requires──> BlueZ + adapter present (graceful "no adapter" state on desktops)
    └──requires──> Pairing agent (PIN/passkey flow — hardest part of Bluetooth)

Appearance panel
    ├──requires──> hyprland.conf safe read/write layer (shared with Input + Display!)
    │                   ├──enables──> Blur/opacity settings
    │                   ├──enables──> Keybind view/edit
    │                   └──enables──> Mouse/input settings
    ├──requires──> hyprpaper (or chosen wallpaper backend) for wallpaper
    └──requires──> GTK3/4 settings.ini + fontconfig writers for font/mouse-gtk

Sound panel
    └──requires──> PipeWire/WirePlumber running (pw-cli/wpctl present; else disabled state)

Packages panel
    ├──requires──> Privilege escalation strategy (pacman -S/-R, yay — the ONE decision blocking all mutations)
    ├──requires──> PKGBUILD review flow for AUR installs (trust decision)
    └──enables──> Landing update-count badge

Display panel
    ├──requires──> hyprland.conf monitor-line writer (shared layer above)
    └──requires──> live `hyprctl` apply + revert-on-timeout (avoid black-screen lock-in: apply, confirm in 10s, else rollback)

System Info panel
    ├──requires──> Metrics sampler core (1Hz poller + ring buffers — shared by CPU graph, temps, power, network rates)
    │                   ├──requires──> GPU vendor detection (NVIDIA NVML vs AMD hwmon vs Intel RAPL) for power/VRAM
    │                   └──degrades──> graceful N/A per sensor (never crash on missing hwmon node)
    └──independent──> Static facts (no sampler needed)

archel-cli
    └──requires──> API surface per domain (CLI is generated/thin over API — no direct system calls from CLI)
```

### Dependency Notes

- **hyprland.conf safe read/write layer is the keystone for Appearance + Display + Input.** Three tabs depend on one parser/writer that must round-trip comments, resolve `source=` includes, expand `$variables`, and keep a backup. Build it once in the API phase; every Hyprland-facing tab blocks on it. This is the single highest-leverage ordering decision.
- **Privilege escalation strategy blocks all package mutations + proxy + DNS-global changes.** Decide early (pkexec per-op vs. a root daemon vs. sudo askpass in GUI). Until decided, Packages/Network/Appearance-font work is read-only. Recommend: `pkexec pacman/...` per-operation for v1 (simplest, matches pamac behavior), revisit daemon only if UX suffers.
- **Metrics sampler core precedes all live graphs.** One poller design (interval, subscription, history buffer) serves CPU/GHz/temps/power/network-throughput. Build the sampler + static facts first (System Info v1), then hang GPU power (HIGH) on it last.
- **GPU vendor detection gates power/VRAM.** NVIDIA/AMD/Intel need three backends behind one trait; detection runs once at startup. Missing sensors must render N/A — Arch hardware varies wildly, and hwmon node names differ per kernel/driver.
- **VPN/DNS overlap:** VPN profiles push DNS; editing connection DNS while a VPN is active is silently overridden. UI must show "DNS managed by VPN <name>" state instead of pretending the edit applies. Same class of issue as GNOME's.
- **Display apply needs revert-on-timeout.** A bad mode line = black screen with no way back. GNOME/KDE both confirm-then-revert; copy that pattern (`hyprctl` apply → 10s confirm dialog → rollback to previous `monitor=` line).
- **PPD vs TLP is exclusive.** Detect at startup: if TLP active, either refuse Power tab with an explanatory message or offer one-click migration. Never let both write CPU governors simultaneously.

## MVP Definition

Ruthless cut: the API + CLI prove every domain; the GUI proves the shell. Anything HIGH complexity that isn't explicitly required by the user defers.

### Launch With (v1)

- [ ] Settings API skeleton + `archel-cli` covering: network state/IP/DNS read, power get/set, bluetooth list, wallpaper set, sound default set, pacman query/install/remove, display read, keybind list, system static facts — CLI-first proves the integration layer
- [ ] Landing dashboard (version, kernel, cached update count, service health dots) — Archel's identity, cheap composition
- [ ] Network: state/IP/gateway, Wi-Fi connect, DNS per-connection, proxy manual form, VPN list/connect + WireGuard import, self-tests — the full required set; VPN import is the cheap win
- [ ] Power: 3-profile switch via PPD — one D-Bus property, huge parity value
- [ ] Bluetooth: scan/pair/connect/disconnect with working pairing agent — MEDIUM but required
- [ ] Appearance: wallpaper, font, blur/opacity sliders with live apply + conf backup — required set, no auto-theming
- [ ] Sound: output/input select + volume — required set, no per-app mixer
- [ ] Packages: pacman search/install/remove/update + yay AUR install/update + update count + orphan cleanup — required set with PKGBUILD confirm
- [ ] Display: resolution/refresh/scale/position with apply-confirm-rollback — required set
- [ ] Input: keybind *view* + mouse sensitivity/scroll — viewing only for v1 (editing is HIGH, defer)
- [ ] System Info: static facts + per-core CPU graph + GHz + temps + disk/RAM — sampler core proven; GPU power/VRAM included only if vendor backend lands cleanly, else N/A placeholders

### Add After Validation (v1.x)

- [ ] Keybind single-bind reassign (first step of editing) — trigger: v1 keybind view adopted without parser bugs
- [ ] GPU power/VRAM backends for remaining vendors — trigger: first vendor backend stable
- [ ] A2DP/HSP Bluetooth profile switching + auto-reconnect rules — trigger: basic Bluetooth pairing reliable
- [ ] Per-app audio volumes (pavucontrol parity) — trigger: users actually ask; pavucontrol exists meanwhile
- [ ] "Advanced…" escape hatches (nm-connection-editor, btop) — trigger: support requests for exotic setups

### Future Consideration (v2+)

- [ ] Full keybind editor with conflict detection — why defer: HIGH complexity, round-trip parser risk; validate demand via v1 view first
- [ ] Accent-color / coordinated theming — why defer: fragile across 3 toolkits; static coherence first
- [ ] Firewall status → management, printer support — why defer: near-zero use on target machine
- [ ] Daemon-based privilege model — why defer: only if pkexec-per-op UX proves annoying

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Wi-Fi connect + IP/gateway/DNS | HIGH | LOW–MED | P1 |
| Power profiles (PPD) | HIGH | LOW | P1 |
| Sound output/input select | HIGH | LOW | P1 |
| Wallpaper + font + blur/opacity | HIGH | LOW–MED | P1 |
| Packages pacman+yay + update count | HIGH | MED | P1 |
| Display modes + layout + rollback | HIGH | MED | P1 |
| Landing dashboard | HIGH | LOW | P1 |
| hyprland.conf safe R/W layer | HIGH (enabler) | MED–HIGH | P1 (first) |
| Privilege escalation decision | HIGH (enabler) | LOW–MED | P1 (first) |
| Metrics sampler + static facts | HIGH (enabler) | MED | P1 |
| Bluetooth pair/connect + agent | HIGH | MED | P1 |
| Network self-tests | MED | LOW | P1 (cheap win) |
| VPN list/connect + WG import | MED–HIGH | MED | P1 |
| Proxy manual form | MED | LOW | P1 |
| Per-core CPU graph + GHz + temps | MED–HIGH | MED | P1 |
| Mouse sensitivity/scroll | MED | LOW | P1 |
| Keybind viewing | MED | MED | P1 |
| GPU power/VRAM live | MED–HIGH | HIGH | P2 (or P1 if backend lands) |
| Keybind single-bind reassign | MED | HIGH | P2 |
| Bluetooth audio-profile switching | LOW–MED | MED | P2 |
| Per-app mixer | LOW | MED | P3 |
| Full keybind editor + conflicts | MED | HIGH | P3 (v2+) |
| Firewall mgmt / printers / online accounts / task manager / marketplace / systemd editor | LOW (for target) | HIGH | P3 (anti-features; don't schedule) |

**Priority key:**
- P1: Must have for launch
- P2: Should have, add when possible
- P3: Nice to have, future consideration

## Competitor Feature Analysis

| Feature | GNOME Control Center | KDE System Settings | Hyprland-ecosystem (nwg-look / hyprpanel) | Our Approach |
|---------|---------------------|---------------------|-------------------------------------------|--------------|
| Network/Wi-Fi | Wi-Fi + Network panels over NM, polished | KCM networkmanagement, full NM parity | None (hyprpanel shows status only) | Match GNOME scope: connect + DNS + VPN + proxy + self-tests (tests = our edge) |
| Power | Power Mode via PPD | PowerDevil (own daemon, richer) | None | PPD only (GNOME model, not PowerDevil); 3 profiles |
| Bluetooth | Panel over BlueZ | Bluedevil over BlueZ | None | Match GNOME panel; blueman remains the escape hatch |
| Appearance | Background/Appearance (GTK/libadwaita) | Global Theme (deep) | nwg-look: GTK theme/font/cursor only; no Hyprland decoration control | Wallpaper + font + Hyprland blur/opacity — goes where nwg-look stops |
| Sound | Sound panel (PipeWire) | Audio KCM (rich: per-app, profiles) | hyprpanel volume widget only | v1 = device select (GNOME-level); per-app = v1.x |
| Packages | GNOME Software (flatpak-centric, irrelevant) | Discover (same) | pamac (heavy, Manjaro) / octopi (dated Qt) | Clean pacman+yay Qt frontend; PKGBUILD review; no flatpak |
| Display | Displays (Mutter) | KScreen (best-in-class) | `hyprctl`/wlr protocol, no GUI | KScreen-lite for Hyprland: modes + layout + confirm-rollback |
| Input/keybinds | Keyboard/Mouse panels (static Mutter binds) | Shortcuts KCM (full editor) | Hand-edited hyprland.conf everywhere | v1 view (GNOME-level) → v2 editor (KDE-level); nobody else does Hyprland binds GUI |
| System info | About (static) | Info Center (rich) | Mission-center-ish widgets, standalone | About + live graphs in one tab (Info Center-lite with power draw edge) |
| CLI/API twin | None (`gsettings` is incidental) | None | None | Unique: every capability in `archel-cli`; no competitor does this |

## Sources

- ArchWiki "Power management" — confirms power-profiles-daemon as the D-Bus power-profile standard on Arch (scraped 2026-09-30, HIGH confidence for PPD recommendation)
- nwg-look GitHub (nwg-piotr/nwg-look, ~1k stars) — GTK3 settings editor scope: theme/font/cursor, no Hyprland decoration control (scraped 2026-09-30, MEDIUM)
- Curated domain knowledge: GNOME Control Center panel set, KDE System Settings/KScreen/PowerDevil/Bluedevil scope, nm-connection-editor/nm-applet behavior, pamac/octopi scope, BlueZ D-Bus API shape, PipeWire/wpctl, pacman/yay CLI surface, Hyprland `hyprctl`/`monitor=`/`decoration:`/`bind=` semantics (MEDIUM — stable tools, no fresh release-note check; live search was unavailable during research)
- PROJECT.md required tab set + Out of Scope list (HIGH — primary source for must-have vs anti-feature boundary)

---
*Feature research for: Archel — Arch + Hyprland settings suite*
*Researched: 2026-09-30*
