<!-- GSD:project-start source:PROJECT.md -->

## Project

**Archel**

A system settings suite for a personal Arch Linux (Hyprland) distro, built in Rust with a Qt6 GUI. It is split into a shared settings **API** layer that owns all system integrations, with two frontends on top: `archel-cli` (command-line access to every capability) and `archel settings` (a modern, user-friendly GUI with vertical tabs). The API is built first; the GUI and CLI are thin consumers of it.

**Core Value:** One reliable API through which every system setting (network, power, bluetooth, appearance, audio, packages, display, input, system stats) can be read and changed on Arch + Hyprland — proven via the CLI before the GUI ever draws it.

### Constraints

- **Tech stack**: Qt6 for the GUI — the user asked for Rust "if possible"; Qt-from-Rust options (Qt for Rust/Qt Rust bindings, cxx-bridge to C++ Qt, or QML-from-Rust) must be validated in research before committing
- **Platform**: Arch Linux + Hyprland (Wayland) — X11-only tools are not acceptable without verification
- **Privilege model**: package operations (pacman/yay) and some network settings require root — privilege escalation strategy (pkexec / sudo / archel daemon) must be decided early
- **Ordering**: the API must exist and be usable via `archel-cli` before the GUI phase begins
- **GUI UX**: modern, user-friendly, vertical tab layout; minimum tab set is fixed (Landing, Network, Power, Bluetooth, Appearance, Sound, Apps/Packages, Display, Input, System Info)

<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->

## Technology Stack

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| Rust (stable, edition 2024) | ≥ 1.85 (Arch ships latest; 1.9x current) | Language for API core, CLI, and GUI backend | Memory safety for a privileged-adjacent system tool; best D-Bus/Hyprland/pacman crate ecosystem on Linux; single language across all three crates. **Use because** every integration below has its best-maintained binding in Rust. |
| CXX-Qt (KDAB) — `cxx-qt`, `cxx-qt-lib`, `cxx-qt-build` | 0.10.0 (released 2026-08-24, ~5 weeks ago) | Qt6-from-Rust bridge for the GUI | **This is the KEY decision: CXX-Qt is the only production-viable Qt-from-Rust option today.** Backed by KDAB (the Qt consultancy), CI-tested on Linux/Win/macOS, actively released. Pattern: declare `#[qobject]` Rust types, CXX-Qt generates the C++ side, QML binds to them. You write *normal Rust* + *normal QML* — no one-to-one C++ wrapping. Verified live: book at kdab.github.io/cxx-qt, `cxx` 1.0.202 underneath (2026-09-12). |
| Qt6 (system libs) | 6.11.2-3 (Arch extra, verified on archlinux.org) | GUI toolkit the bridge targets | Arch tracks latest Qt6; CXX-Qt 0.10 targets Qt 6.x. GUI crate links system `qt6-base` + `qt6-declarative` (QML). |
| QML (Qt Quick) | Qt 6.11 | GUI markup/UX layer | **Use QML-from-Rust, not Widgets-from-Rust.** CXX-Qt's documented happy path is Rust backend objects exposed to QML. QML gives the "modern, vertical-tabs" UX cheaply; Widgets through CXX-Qt is more manual C++ surface. |
| tokio | 1.53.1 (2026-07-20) | Async runtime for the whole API core | Non-negotiable: `zbus` 5.x and `bluer` both require tokio. One runtime serves D-Bus calls, Bluetooth, and the metrics sampler. Use `rt-multi-thread` + `time` + `process` features. |
| zbus | 5.19.0 (2026-08-09) | Single D-Bus client for NetworkManager, power-profiles-daemon, WirePlumber, UPower | The standard modern Rust D-Bus stack (88M+ downloads). `zbus::proxy` macro generates typed proxies from interface names — one dependency covers four integrations instead of four wrapper crates of varying freshness. |
| clap (derive) + clap_complete | 4.6.7 / 4.6.11 (Sep 2026) | `archel-cli` argument parsing + shell completions | De-facto Rust CLI standard. Derive API keeps the CLI thin (it mirrors the API surface). `clap_complete` generates bash/fish/zsh completions for the distro. |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| hyprland (hyprland-rs) | **0.3.13 stable** (2026-08-29, GPL-3.0) — NOT 0.4.0-beta.3 | Hyprland IPC socket (monitors, workspaces, `hyprctl` dispatch, keyword set) | Display tab (resolution/scale/refresh/layout), input tab reads, appearance writes. Pin `"0.3"`; the 0.4 beta line (Sep 2025) was superseded by continued 0.3.x releases. Covers monitors + keyword dispatch; raw socket fallback for anything missing. |
| bluer | 0.17.4 (2025-06-06) | BlueZ Bluetooth (adapters, discovery, pairing, GATT) | **Official BlueZ Rust bindings**, tokio-native, streaming discovery events fit the GUI's live device list. Bluetooth tab. |
| alpm | 5.0.2 (2026-01-08) | Safe libalpm bindings (local DB reads, package queries) | Landing page update counts + Apps tab search/installed-list. Enable `checkver` feature so the build fails loudly on libalpm mismatch. **Reads only** — see privilege row. |
| sysinfo | 0.39.6 (2026-07-09) | Cross-platform CPU/RAM/disk/process sampling | System Info tab baseline: per-core CPU %, RAM, disks, frequencies. Refresh-based API maps directly to the GUI's polling loop. Does NOT do GPU/temps/power — pair with rows below. |
| nvml-wrapper | 0.13.0 (2026-08-31) | NVIDIA VRAM, temp, power draw | System Info GPU block on NVIDIA. dlopen-based — no link-time CUDA dependency, degrades gracefully on AMD/Intel. |
| pipewire (pipewire-rs) | 0.10.1 (2026-08-19) | PipeWire device/port enumeration | Sound tab device lists. For *default-sink/source switching* prefer WirePlumber's D-Bus API via zbus (no maintained `wireplumber` crate exists — 0.0.1 placeholder from 2023). `wpctl` subprocess as fallback. |
| serde + serde_json + toml | 1.0.229 / 1.0.151 / 1.1.6 | Config + IPC serialization | `hyprland.conf` block editing (line-preserving, see pitfalls), app config file, CLI `--json` output. |
| anyhow + thiserror | 1.0.104 / 2.0.21 | Error handling split | `thiserror` for typed errors in `archel-api` (per-domain error enums the GUI matches on); `anyhow` in CLI/GUI binaries. |
| tracing (+ tracing-subscriber) | 0.1.44 | Structured logging | API core emits spans per operation; CLI defaults to warn, GUI to file log. Replaces `pretty_env_logger` (stale, 2023). |
| notify | 9.0.0-rc.5 (2026-08-30) | Filesystem watching | Watch `hyprland.conf` for external edits; live-reload appearance/input tabs. |
| directories | 6.0.0 | XDG paths | Config in `~/.config/archel/`, cache for metrics history. |
| duct + which | 1.1.2 / 8.0.6 | Subprocess orchestration | Wrapping `pacman`/`yay`/`nmcli`/`wpctl`/`pkexec`: `which` for capability detection, `duct` for piped/checked commands. |
| regex | 1.13.1 | Parsing CLI outputs + conf files | `yay -Qu` output, `hyprland.conf` section matching, `sensors`-style fallbacks. |
| Hand-rolled `hwmon` module (sysfs) | n/a (~150 lines) | CPU/GPU temps + AMD power draw | Read `/sys/class/hwmon/*/temp*_input`, `power*_average`, `/sys/class/drm/card*/device/mem_info_vram_used` (amdgpu). No crate needed; sysfs is stable ABI. Intel RAPL via `/sys/class/powercap/intel-rapl`. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| rust-toolchain.toml (stable) + rustup | Pinned toolchain | `channel = "stable"`, edition 2024. Arch's `rust` package tracks stable; rustup preferred for reproducibility. |
| cxx-qt-build (build.rs in GUI crate) | Compiles Rust↔C++ bridge + moc | Requires C++ compiler + `qt6-base` + `qt6-declarative` at build time; runs moc automatically. Biggest build-complexity contributor — isolate it in the GUI crate so API/CLI build with plain cargo. |
| cargo workspace | Monorepo layout | `archel-api` (lib) + `archel-cli` (bin) + `archel-gui` (bin). GUI depends on API; CLI depends on API; API depends on nothing Qt. |
| PKGBUILD (+ AUR `-bin`/`-git` variants) | Distro packaging | `depends=(qt6-base qt6-declarative pipewire bluez networkmanager)`; `makedepends=(rust qt6-tools)`. API/CLI packaged without Qt deps via split packages. |
| insta | Snapshot testing (1.48.0) | Snapshot `hyprland.conf` transformations and CLI `--json` outputs — the two most regression-prone surfaces. |

## Installation

# System deps (Arch)

# Workspace

# Cargo.toml (workspace root)

# [workspace] members = ["archel-api", "archel-cli", "archel-gui"]

# Core (archel-api)

# CLI (archel-cli)

# GUI (archel-gui) — the ONLY crate that pays the Qt tax

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| CXX-Qt 0.10 + QML | Slint 1.18.1 (pure Rust, very active — updated 2026-09-21) | **Escape hatch only.** If CXX-Qt's C++/moc build burden blocks the GUI phase, Slint is the best pure-Rust GUI. But it violates the Qt6 requirement and restyles everything — decide in GUI phase, not now. |
| CXX-Qt + QML | qmetaobject-rs (QML bindings) | Never for new code in 2026 — last release 0.2.10 (Nov 2023), effectively unmaintained. |
| zbus direct proxies | `networkmanager` crate 0.5.0 (2025-06-26) | If its safe wrapper covers all NM needs (devices, connections, VPN) it saves proxy boilerplate. Spike it in the Network phase; default stays zbus-direct so one D-Bus stack serves NM+PPD+WP. |
| `archel-api` as in-process lib | Separate `archel-daemon` over zbus/Unix socket | Only if v2 needs background watchers (tray, auto-update checks) or cross-user state. v1: library keeps auth/lifecycle trivial — D-Bus to system services is already IPC. |
| `pacman`/`yay` subprocess for mutations | Reimplement transactions via alpm | Never — alpm transaction + signature + hook handling duplicates pacman; subprocess inherits pacman.conf, keyring, and progress semantics for free. alpm stays read-side. |
| Custom hwmon sysfs reader | `systemstat` 0.2.7 / heim | `systemstat` (updated 2026-06) is a fallback for basic stats, but it lacks GPU/power/hwmon depth. heim is dead (2020). Custom sysfs module wins for temps/power/VRAM. |
| tracing | log + env_logger | tracing's spans justify it the moment CLI and GUI consume the same async API core. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| qmetaobject-rs | Unmaintained since 2023; predates Qt 6.5+ QML features; no edition-2024 support | CXX-Qt 0.10 |
| `wireplumber` crate | 0.0.1 placeholder (2023), 1.6k downloads — empty binding | WirePlumber D-Bus via zbus; `wpctl` subprocess fallback |
| `dbus` crate (0.9.x) | Legacy blocking API, superseded by zbus's async design; mixing runtimes with bluer/zbus causes pain | zbus 5.x everywhere |
| heim / nvtop-rs | Dead (2020 / 2023); bitrotted against current kernels | sysinfo + hwmon sysfs + nvml-wrapper |
| libadwaita/GTK4 | Directly contradicts the Qt6 requirement; second toolkit doubles theming work on Hyprland | Qt6/QML via CXX-Qt |
| X11-only helpers (xrandr, xinput) | No effect under Hyprland/Wayland | hyprland crate IPC + `hyprland.conf` for input; wlr-output-management for future external-display parity |
| `pretty_env_logger` | Stale (2023) | tracing-subscriber |
| serde_yaml 0.9 | Deprecated upstream | serde_json / toml |
| tonic/tarpc daemon IPC in v1 | Solves a problem v1 doesn't have (single-user local app); adds auth, versioning, lifecycle work before any setting works | In-process `archel-api` lib; revisit for v2 daemon |
| yay wrappers as libs | No maintained AUR-helper library; AUR RPC + makepkg reimplementation is a security surface | `yay` subprocess (+ `which` detection, graceful "yay not installed" state) |

## Stack Patterns by Variant

- Enable `nvml-wrapper` for VRAM/temp/power; gate behind `which`-style NVML init probe so the binary runs clean on AMD/Intel.
- No NVML — read `mem_info_vram_used` + `hwmon/power1_average` sysfs nodes directly (amdgpu exposes both, stable ABI).
- CLI/GUI shell out through `pkexec` (desktop-polkit path, works from GUI) with `sudo -n` detection in CLI. No setuid helper, no embedded password prompts in v1. The API returns a typed `NeedsPrivilege(cmdline)` error so frontends render the escalation prompt uniformly.
- `archel-api` + `archel-cli` still build and test green (no Qt dependency). CI splits into `cargo test -p archel-api -p archel-cli` (fast, no Qt) and `-p archel-gui` (Qt image). This is the payoff of isolating CXX-Qt in one crate.
- Pin `qt6-base`/`qt6-declarative` versions in PKGBUILD; upstream KDAB tracks Qt within weeks. Fallback decision point: Slint (see alternatives).

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| cxx-qt 0.10.0 | Qt 6.x (Arch 6.11.2-3 verified) + cxx 1.0.202 + Qt 6 moc | All three cxx-qt crates must pin the same 0.10.x; cxx version is managed by cxx-qt-build. |
| zbus 5.19 + bluer 0.17.4 | tokio 1.x (1.53 verified) | bluer is tokio-only; do not introduce async-std anywhere. |
| alpm 5.0.2 | pacman 7.x libalpm (Arch current) + `checkver` feature | Targets latest libalpm; rebuild on pacman major bumps. |
| hyprland 0.3.13 | Hyprland ≥ 0.4x IPC socket format | Socket lives at `$XDG_RUNTIME_DIR/hypr/$SIG/.socket.sock`; protocol is version-skew tolerant for get/set-keyword paths. |
| pipewire 0.10.1 | libpipewire 1.x (Arch current) | Links system libpipewire; needs `pipewire` in PKGBUILD depends. |
| nvml-wrapper 0.13 | NVIDIA drivers 555+ (dlopen at runtime) | Optional dependency in practice — `dlopen` failure = "no NVIDIA" state, not a crash. |
| clap 4.6 + clap_complete 4.6 | Keep minor versions in lockstep | clap_complete tracks clap's minor releases. |

## Sources

- crates.io registry API (live, 2026-09-30) — exact newest_version + updated_at for: cxx-qt/cxx-qt-lib/cxx-qt-build 0.10.0, cxx 1.0.202, qmetaobject 0.2.10 (2023, stale), hyprland 0.4.0-beta.3, zbus 5.19.0, bluer 0.17.4, clap 4.6.7, sysinfo 0.39.6, alpm 5.0.2, tokio 1.53.1, serde 1.0.229, pipewire 0.10.1, wireplumber 0.0.1 (dead), nvml-wrapper 0.13.0, slint 1.18.1, networkmanager 0.5.0, heim/nvtop/pretty_env_logger (dead/stale) — HIGH confidence
- docs.rs (live scrape) — hyprland latest stable is 0.3.13 (2026-08-29, GPL-3.0); bluer is "official BlueZ bindings, tokio-based"; alpm.rs targets latest libalpm with `checkver`/`generate` features — HIGH confidence
- KDAB CXX-Qt book (live scrape) — bridge-via-CXX design, CI on Linux/Win/macOS, QML-first integration — HIGH confidence
- archlinux.org packages (live scrape) — qt6-base 6.11.2-3 in extra — HIGH confidence
- D-Bus service names (NetworkManager, PPD, WirePlumber), pkexec patterns, sysfs hwmon paths, yay `-Qu` behavior — training knowledge, MEDIUM confidence, must be verified against the live Archel machine in the API phase

<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->

## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->

## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->

## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->

## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->

<!-- GSD:profile-start -->

## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
