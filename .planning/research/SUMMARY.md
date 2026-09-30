# Project Research Summary

**Project:** Archel — Arch + Hyprland system settings suite (Rust API + CLI + Qt6 GUI)
**Domain:** Linux desktop system settings (GNOME Control Center / KDE System Settings class, Hyprland-native)
**Researched:** 2026-09-30
**Confidence:** HIGH on stack versions, MEDIUM on features/architecture/pitfalls

## Executive Summary

Archel is a library-first system settings suite for Arch Linux + Hyprland: a single `archel-core` Rust library owns every system integration, consumed in-process by a thin `archel-cli` (clap) and a Qt6/QML GUI (`archel-settings`). Experts build this class of app as an unprivileged client over privileged system services — NetworkManager, BlueZ, power-profiles-daemon, PipeWire/WirePlumber own their state over D-Bus; the app never reimplements them. Flat-file targets (hyprland.conf) get live-apply via `hyprctl` plus persistence in app-owned `source=` snippets, never a rewrite of the user's main file.

The unanimous recommendation across all four research tracks: Rust stable (edition 2024) + tokio throughout, `zbus` 5.x as the single D-Bus stack, CXX-Qt 0.10.0 + QML as the only viable Qt-from-Rust path (gated by a time-boxed GUI spike, Slint 1.18.1 as escape hatch), and build order API → CLI proof → GUI. Privilege stays minimal: pkexec one-shot helper for pacman only, yay/AUR builds always as the user, NM writes via its own D-Bus polkit. The key risks are the Qt binding stalling the GUI phase, destroying hyprland.conf with naive rewrites, pacman concurrency/partial-upgrade breakage, GPU vendor fragmentation, and blocking the Qt event loop — each with a concrete prevention pattern and a phase gate below.

## Key Findings

### Recommended Stack

Rust is the only sane choice: every integration (D-Bus, Hyprland IPC, libalpm, BlueZ, PipeWire, NVML) has its best-maintained binding in Rust, and one language serves API + CLI + GUI backend. tokio 1.53 is non-negotiable (zbus 5.19 + bluer 0.17 both require it). CXX-Qt 0.10.0 (KDAB, released 2026-08-24) + Qt 6.11 + QML is the key decision — write normal Rust `#[qobject]` view-models, normal QML views, isolate all Qt build cost in the GUI crate so API/CLI build headless. See STACK.md for full version table.

**Core technologies:**
- Rust stable ≥1.85, edition 2024 — memory safety + best Linux integration crates, single language for all crates
- CXX-Qt 0.10.0 + Qt6 6.11 + QML — only production-viable Qt-from-Rust path; QML-first, not Widgets
- tokio 1.53 + zbus 5.19 — one async runtime and one D-Bus stack for NM, BlueZ, PPD, WirePlumber
- hyprland 0.3.13 (stable, NOT 0.4-beta) — monitors, keyword dispatch, live-apply; raw socket fallback
- clap 4.6 + clap_complete — thin CLI mirroring the API surface, shell completions for the distro
- alpm 5.0.2 (reads only) + pacman/yay subprocess for mutations — never reimplement transactions
- sysinfo 0.39 + hand-rolled hwmon sysfs module + nvml-wrapper 0.13 — CPU/RAM/disks + temps/power/VRAM per vendor
- anyhow/thiserror + tracing + notify + serde/toml — typed API errors, structured logging, conf watching

### Expected Features

Match GNOME scope per tab, win where the Hyprland ecosystem has nothing: Hyprland decoration control, keybind viewing, VPN/proxy integration, network self-tests, GPU power/VRAM, and the unique CLI twin. hyprland.conf safe R/W layer and the privilege model are the two enabler decisions that unblock everything downstream.

**Must have (table stakes):**
- Wi-Fi/wired connect + IP/gateway/DNS per-connection (NM D-Bus) — #1 reason anyone opens settings
- Power profile switch via PPD (one D-Bus property; TLP excluded) — GNOME parity
- Sound output/input select + volume (PipeWire/WP; no v1 mixer) — pavucontrol covers the rest
- Display modes/scale/position with apply-confirm-rollback — no KScreen exists on Hyprland
- Wallpaper (hyprpaper) + font (GTK/Qt/fontconfig) + blur/opacity (decoration keywords) — past where nwg-look stops
- Packages: pacman query/install/remove + yay AUR + cached update count + orphan cleanup, PKGBUILD review gate
- Bluetooth scan/pair/connect/disconnect with working pairing agent — required, MEDIUM cost
- Landing dashboard (kernel, version, cached updates, service health) + System Info (static facts + CPU graph + temps)
- Keybind *viewing* + mouse sensitivity/scroll — viewing only for v1
- Network self-tests, VPN list/connect + WireGuard import, manual proxy form — cheap wins, real differentiation

**Should have (competitive):**
- GPU VRAM + temperature + CPU/GPU power draw behind a `GpuBackend` trait — highest wow density, HIGH cost, vendor-gated
- Per-core CPU graph + GHz via a shared 1 Hz sampler — sampler built once, reused everywhere
- `archel-cli` exposing every capability — architectural cost, pays back as the test harness; gate rule: no GUI without CLI proof

**Defer (v2+):**
- Full keybind editor with conflict detection (v1 = view + single-bind reassign at most) — HIGH parser risk
- Accent/coordinated theming, per-app mixer, firewall management, printers, online accounts, task manager, systemd editor, flatpak — anti-features, explicitly out of scope

### Architecture Approach

Single library crate consumed in-process by both frontends, no custom daemon in v1 — IPC only pays off when two processes share live state, and here the GUI links `archel-core` directly with zero serialization cost. Qt thread never awaits: tokio runtime on a worker thread, results cross via queued signals. Two write strategies by target: D-Bus for everything with a daemon (never hand-edit its files), managed-block/snippet surgery + `hyprctl keyword` live-apply for hyprland.conf. Privilege crosses exactly one boundary: pkexec one-shot helper for pacman, everything else user-context or the daemon's own polkit.

**Major components:**
1. `archel-core` lib — all domain logic (network, bluetooth, audio, power, packages, display, input, appearance, stats, hyprland_conf editor); zero Qt dependency
2. `archel-cli` — clap tree mirroring core 1:1; the API conformance harness, must pass before any write path or GUI tab
3. `archel-settings` GUI — QML views per tab + Rust QObject view-models over cxx-qt bridge; no business logic in QML
4. `archel-privileged` helper + polkit policy — argv-allowlisted one-shot root actions (install/remove/upgrade), JSON over stdout; yay builds never escalate
5. Stats sampler — 1 Hz poller + ring buffers + watch channels owned by core; GUI subscribes, CLI polls once

### Critical Pitfalls

1. **Qt-for-Rust binding stall** — time-boxed spike (window + vertical tabs + live label) gates all GUI phases; isolate Qt in one crate; Slint escape hatch; pin qt6-base + cxx-qt versions.
2. **Destroying hyprland.conf** — never rewrite the main file; live-apply via `hyprctl keyword`, persist in app-owned `source=` snippet with atomic write + backup + reload-verify.
3. **pacman/yay concurrency + partial upgrades** — single op queue, pre-flight `db.lck` check, always full `-Syu` (never `-Sy` alone), yay never as root, AUR PKGBUILD review mandatory.
4. **GPU vendor fragmentation** — per-vendor `GpuBackend` trait returning `Option` per sensor, unit conversion at the boundary (µW→W, m°C→°C), discover-by-name not hardcoded paths, honest `n/a` states.
5. **Blocking the Qt event loop** — hard rule: no I/O on the GUI thread; async zbus + tokio workers + timeouts + spinners; sampler pushes, GUI never pulls synchronously.
6. **Running the GUI as root** — app always runs as the user; only narrow ops escalate; helper never writes `$HOME` files.
7. **X11-tool assumptions** — no xrandr/xinput/Xresources; displays via `hyprctl monitors` + `monitor=` lines; session assertion at startup.
8. **NM DNS/proxy model** — DNS is per-active-profile (`ipv4.dns` + `ignore-auto-dns` + `dns-priority`, `-100` VPN preset); proxy is a separate env/desktop layer, not an NM property; always show effective DNS + leak check.

## Implications for Roadmap

Based on research, suggested phase structure:

### Phase 1: Foundation (toolchain, error model, privilege + op-queue design, async boundary, GUI spike gate)
**Rationale:** Every downstream phase depends on these decisions; privileges and the Qt spike are the two highest-risk unknowns and must be retired first.
**Delivers:** Workspace (`archel-core`, `archel-cli`, `archel-settings`, `archel-privileged` skeletons), `ArchelError` taxonomy, pkexec helper skeleton + polkit policy draft, single package-op queue design, tokio-worker + queued-delivery pattern proven, CXX-Qt spike pass/fail (window + live label), Qt toolchain pins, "no X11 tools" constraint + session assertion, shared zbus connection rule.
**Addresses:** Privilege escalation enabler, hyprland.conf strategy decision, async boundary.
**Avoids:** Pitfalls 1 (binding stall), 5 (event-loop blocking), 6 (GUI-as-root), 7 (X11 tools).

### Phase 2: Read-only core (hyprland_conf reader, stats sampler + static facts, package queries, Hyprland + D-Bus reads)
**Rationale:** Dependency-free modules first for momentum; feeds the Landing tab; establishes sampler and conf-reader shared infra before any writer exists.
**Delivers:** `hyprland_conf.rs` managed-block reader, stats sampler (sysinfo + hwmon discovery, 1 Hz, watch channels, `--benchmark`), alpm read-only queries, display/input/appearance reads, network/bluetooth/audio/power reads via zbus proxies.
**Uses:** sysinfo, hwmon sysfs, hyprland 0.3.13 reads, zbus proxies, alpm.
**Implements:** `archel-core` read surface for all domains.
**Avoids:** Pitfalls 4 (GPU — trait + fixtures here), 8 (sampler discipline), 3 (reads never touch the op queue).

### Phase 3: CLI proof gate (full `archel-cli` over every read + JSON output)
**Rationale:** PROJECT.md build order — CLI proves the integration layer headless; no write path and no GUI tab starts before its domain is CLI-proven.
**Delivers:** `archel-cli` covering network state/IP/DNS read, power get, bluetooth list, wallpaper/sound/display reads, pacman queries, keybind list, stats snapshot; snapshot tests (insta) on conf transforms + `--json` outputs.
**Addresses:** CLI-twin differentiator (read half), test harness for all later phases.
**Avoids:** Pitfall 5 (async CLI surface validated before GUI amplifies mistakes).

### Phase 4: Privileged writes (helper hardening + package mutations + cached update count)
**Rationale:** First write path; unblocks Landing badge and proves the privilege boundary end-to-end as an unprivileged user.
**Delivers:** Hardened helper (argv allowlist, JSON protocol, audit-surface review), `install/remove/upgrade` via pkexec, op queue + `db.lck` UX, yay-as-user wrapper + PKGBUILD review flow, cached update count with TTL, orphan cleanup.
**Addresses:** Packages tab (full), Landing update badge.
**Avoids:** Pitfalls 3 (concurrency/partial-upgrade), 6 (root discipline — e2e as user, no root-owned `$HOME` files).

### Phase 5: Daemon-mediated writes (network DNS/VPN/proxy, bluetooth pairing agent, audio switching, power set)
**Rationale:** Each needs its Phase 2 read path + Phase 3 CLI proof; NM DNS/proxy semantics are the subtlest domain and deserve an isolated phase.
**Delivers:** Per-active-profile DNS + `dns-priority` VPN preset + effective-DNS display + leak check, VPN list/connect + WireGuard import, separate proxy layer (session env + desktop settings + honest copy), BlueZ pairing agent, default-sink/source switching, PPD profile set + TLP-conflict handling.
**Uses:** zbus NM/BlueZ/PPD proxies, WirePlumber D-Bus / `wpctl` fallback.
**Implements:** `core::network/bluetooth/audio/power` write surface.
**Avoids:** Pitfall 9 (DNS/proxy model), plus BlueZ signal-subscription discipline.

### Phase 6: Hyprland writes (appearance, display, input)
**Rationale:** Establishes the snippet-file + atomic-write + backup + reload-verify infra once (appearance first), then reuses it for display/input; display rollback is safety-critical.
**Delivers:** App-owned `archel.conf` snippet + `source=` install (once, with backup), wallpaper/font/blur-opacity with live-apply + persist, monitor modes/layout with apply-confirm-rollback validated against `hyprctl monitors`, keybind view + mouse/scroll writes.
**Addresses:** Appearance, Display, Input tabs (v1 scope: keybind view only).
**Avoids:** Pitfalls 2 (conf destruction), 7 (X11 tools — compositor-owned outputs/inputs only).

### Phase 7: GUI shell + read-only tabs (vertical tabs, Landing, System Info)
**Rationale:** GUI starts only after the spike passed AND the full read API is CLI-proven; read-only tabs first prove the tokio-worker + queued-delivery pattern under real load.
**Delivers:** `archel-settings` shell with vertical tabs, Landing dashboard (composition of cached getters), System Info with live graphs (1 Hz model updates, hidden-tab backoff, decimated repaints), `QAbstractListModel` graph bindings.
**Uses:** CXX-Qt 0.10 + QML, watch-channel subscriptions, 1 s QTimer view-model pump.
**Implements:** GUI shell + first two tabs.
**Avoids:** Pitfalls 1 (post-spike execution), 5 (no I/O on GUI thread — delayed-D-Bus test), 8 (observer CPU budget).

### Phase 8: GUI write tabs (one per proven domain)
**Rationale:** Tabs land as their domain's write paths complete; each tab is thin (properties + invokables) over an already-proven core.
**Delivers:** Network, Bluetooth, Sound, Power, Packages (progress stream + cancel), Appearance, Display (confirm dialog), Input tabs; spinner/disabled states on every I/O action; error toasts from typed core errors.
**Addresses:** Remaining GUI tabs to v1 scope.
**Avoids:** All UX pitfalls (effective-DNS display, `n/a`-with-reason GPU states, progress + cancel everywhere).

### Phase Ordering Rationale

- **Foundations before features:** privilege model, op queue, async boundary, and conf strategy are decided in Phase 1 because Packages/Network/Hyprland phases all reuse them — deciding late means re-implementing.
- **Reads before writes, CLI before GUI:** every write path depends on its read path plus CLI proof; the GUI shell depends on the full read API. This is the PROJECT.md gate rule and the single highest-leverage sequencing decision.
- **Sampler and conf-reader are shared infra:** built once in Phase 2 (reads) and Phase 6 (writes) respectively; no phase builds its own poller or parser.
- **Riskiest unknowns earliest:** Qt spike and privilege design in Phase 1 (pivot is cheap), GPU vendor backends in Phase 2 (fixtures before graphs), NM DNS semantics isolated in Phase 5.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 1 (Foundation/GUI spike):** `--research-phase` recommended — CXX-Qt 0.10 vs Arch Qt 6.11 compatibility, moc/build.rs setup, and pkexec-vs-polkit-service tradeoff need live verification on the Archel machine.
- **Phase 5 (Network writes):** `--research-phase` recommended — per-profile DNS + `dns-priority` VPN behavior + systemd-resolved interaction must be verified against the live machine; proxy layer scope (env vs gsettings) needs confirmation.
- **Phase 2 (Stats/GPU reads):** `--research-phase` light — hwmon node names, NVML presence, and RAPL paths on Axel's actual hardware need discovery; vendor fixtures built from live sysfs.
- **Phase 6 (Hyprland writes):** `--research-phase` light — snippet-file + `source=` chain behavior and `hyprctl keyword` coverage for needed keys verified on the live compositor.

Phases with standard patterns (skip research-phase):
- **Phase 3 (CLI):** clap derive over an existing API is fully standard.
- **Phase 4 (Package mutations):** libalpm + pkexec + op-queue pattern is well-documented; only machine-specific check is `db.lck`/yay presence.
- **Phase 7–8 (GUI tabs):** once the spike + async pattern are proven, tabs are repetitive view-model work; per-tab questions are discuss-phase material, not research-phase.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Versions verified live via crates.io registry API, docs.rs, KDAB docs, archlinux.org (2026-09-30); single-researcher judgment on architecture calls |
| Features | MEDIUM | Competitor behavior from stable long-lived tools + ArchWiki PPD verification; live search degraded so no fresh release-note check |
| Architecture | MEDIUM | Qt/D-Bus claims verified against official docs (CXX-Qt repo+book, zbus v5); tokio/Qt-bridge, polkit, hwmon details from curated knowledge (LOW–MEDIUM) |
| Pitfalls | MEDIUM | Stable platform facts from domain knowledge; fast-moving items (cxx-qt releases, Qt terms) flagged for re-verification in GUI spike |

**Overall confidence:** MEDIUM-HIGH — stack pins are solid; domain judgments converge across all four researchers and need only live-machine validation during implementation.

### Gaps to Address

- **Live-machine verification:** D-Bus service names, PPD-vs-TLP state, hwmon/NVML presence, Hyprland socket version, yay presence — validate in Phase 1–2 on Axel's machine, not from docs.
- **CXX-Qt 0.10 vs Qt 6.11 friction:** re-verify at spike time; Arch moves fast (Qt 6.12+); fallback decision (Slint) is explicit, not drift.
- **`networkmanager` crate 0.5.0 vs zbus-direct:** spike in Network phase; default stays zbus-direct.
- **UI locale (FR/EN) strategy:** decide in foundation (Qt Linguist from day one vs single-language v1) — flagged in PITFALLS.md UX section, no research covered it.
- **qobject view-model per-tab API freeze:** tab-per-module parallelization assumes frozen per-domain surfaces — enforce at Phase 3 gate.

## Sources

### Primary (HIGH confidence)
- crates.io registry API (live 2026-09-30) — exact versions + dates for cxx-qt 0.10.0, zbus 5.19.0, bluer 0.17.4, hyprland 0.3.13, tokio 1.53.1, alpm 5.0.2, clap 4.6, sysinfo 0.39.6, pipewire 0.10.1, nvml-wrapper 0.13.0, slint 1.18.1
- docs.rs — hyprland stable line, bluer official BlueZ bindings, alpm checkver feature
- KDAB CXX-Qt book + repo — bridge design, QObject macros, CI platforms
- archlinux.org packages — qt6-base 6.11.2-3 in extra
- ArchWiki Power management — PPD as the D-Bus power-profile standard

### Secondary (MEDIUM confidence)
- nwg-look GitHub — GTK settings-editor scope boundary (no Hyprland decoration control)
- GNOME Control Center / KDE System Settings / KScreen / PowerDevil / Bluedevil scope knowledge — table-stakes vs differentiator boundary
- zbus v5 readme — tokio feature, proxy/interface macros, blocking wrappers
- hyprland-rs repo — IPC socket wrapper scope
- Curated platform knowledge: NM per-profile DNS model, BlueZ ObjectManager patterns, pacman db.lck + makepkg-non-root + partial-upgrade rules, amdgpu/hwmon/NVML sysfs semantics, Wayland compositor-owns-outputs model, pkexec/polkit least-privilege patterns

### Tertiary (LOW confidence, needs live validation)
- qmetaobject-rs staleness assessment (repo state at research time — sufficient to reject, not to track)
- Qt Company commercial-term movement (re-check at packaging time; affects static-vs-dynamic linking decision)
- Exact hwmon node names / RAPL paths / NVML driver state on target hardware (enumerate live in Phase 2)

---
*Research completed: 2026-09-30*
*Ready for roadmap: yes*
