# Architecture Research: Archel (Arch + Hyprland Settings Suite in Rust)

**Domain:** Linux system settings suite — shared Rust API + CLI + Qt6 GUI
**Researched:** 2026-09-30
**Confidence:** MEDIUM (Qt/D-Bus claims verified against official docs — CXX-Qt repo+book, zbus v5 crates.io; tokio/Qt-bridge, polkit, and hwmon details from curated knowledge, LOW–MEDIUM)

## Standard Architecture

Settings suites on Linux converge on one of two shapes. GNOME Settings / KDE System Settings use **unprivileged GUI + privileged system services over D-Bus** (NetworkManager, power-profiles-daemon, BlueZ all own their state; the GUI is a thin D-Bus client). Small distro tools (e.g. Manjaro Settings Manager) use a **monolithic GUI that shells out to pkexec helpers**. Archel sits between: it needs a real programmatic API (per PROJECT.md, the API is the product, CLI and GUI are consumers), but it does **not** need a resident root daemon on day one.

**Recommendation: single library crate (`archel-core`) consumed by both binaries, no custom daemon in v1.** A private D-Bus/IPC daemon is over-engineering for a single-machine settings app whose "1 Hz+ live stats" are consumed in-process by the GUI. IPC (D-Bus service, unix socket, gRPC) only pays off when two *processes* must share live state — here the GUI can link `archel-core` directly and sample in-process at any rate with zero serialization cost. The privileged operations (pacman/yay, some NM writes) are handled by a **one-shot pkexec helper binary**, not a resident daemon (see Privilege Boundary below).

### System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│  FRONTENDS (thin; no system calls except via archel-core)        │
│  ┌──────────────────┐        ┌──────────────────────────────┐   │
│  │   archel-cli     │        │   archel settings (Qt6 GUI)  │   │
│  │ clap subcommands │        │ QML views + Rust QObject     │   │
│  │ per domain       │        │ models (cxx-qt bridge)       │   │
│  └────────┬─────────┘        └──────────────┬───────────────┘   │
│           │  function calls / tokio channels │                   │
├───────────┴────────────────────────────────┴───────────────────┤
│  archel-core (LIBRARY — owns ALL system integration)             │
│  ┌──────────┐ ┌──────────┐ ┌────────┐ ┌───────┐ ┌────────────┐  │
│  │ network  │ │bluetooth │ │ audio  │ │ power │ │  packages  │  │
│  │ (zbus→NM)│ │(zbus→BlueZ)│ │(pipewire)│ │(ppd D-Bus)│ │(helper+alpm)|│
│  ├──────────┤ ├──────────┤ ├────────┤ ├───────┤ ├────────────┤  │
│  │ display  │ │  input   │ │appear. │ │ stats │ │  hyprland  │  │
│  │(hyprland │ │(hyprland │ │(hyprl. │ │sampler│ │ conf edit  │  │
│  │  IPC)    │ │ conf)    │ │ conf) │ │thread │ │ (managed   │  │
│  └──────────┘ └──────────┘ └────────┘ └───────┘ │  blocks)   │  │
│                                                 └────────────┘  │
├─────────────────────────────────────────────────────────────────┤
│  SYSTEM SERVICES (already running — never reimplemented)         │
│  NetworkManager │ BlueZ │ power-profiles-daemon │ PipeWire/WP   │
│  Hyprland IPC socket │ pacman/yay │ sysfs/hwmon │ NVML │ polkit │
└─────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Typical Implementation |
|-----------|---------------|------------------------|
| `archel-core` lib | All read/write logic per domain; plain Rust structs + `async` fns; zero Qt dependency | Cargo lib crate, tokio-based, `zbus` (tokio feature), `hyprland` crate, `sysinfo` |
| `archel-cli` | Clap CLI mapping 1:1 to `archel-core` API; doubles as API test harness | Thin binary crate, `tokio::main` |
| `archel settings` GUI | QML views per tab + Rust `QObject` view-models bridging to `archel-core`; Qt event loop owns UI thread only | Binary crate, `cxx-qt` + `cxx-qt-lib`, QML files |
| `archel-privileged` helper | One-shot root actions (pacman install/remove/upgrade); invoked via pkexec, argv-validated, exits immediately | Small binary crate, installed setuid-free, polkit `.policy` file |
| stats sampler | Background 1 Hz sampling → ring buffers → channels; owned by `archel-core`, consumed by CLI (poll) and GUI (model updates) | `tokio::task::spawn` + `tokio::sync::watch`/`broadcast` |
| hyprland conf editor | Line-preserving read/modify/write of `hyprland.conf` inside `# MANAGED BY ARCHEL` blocks only | Custom text-surgery module, never a full parse-and-rewrite |

## Recommended Project Structure

```
archel/                          # Cargo workspace
├── Cargo.toml                   # workspace members, shared deps
├── crates/
│   ├── archel-core/             # THE API — no Qt, no CLI code
│   │   ├── src/
│   │   │   ├── lib.rs           # public re-exports per domain
│   │   │   ├── error.rs         # single ArchelError enum (thiserror)
│   │   │   ├── network.rs       # zbus proxies → NetworkManager
│   │   │   ├── bluetooth.rs     # zbus proxies → BlueZ
│   │   │   ├── audio.rs         # PipeWire/WirePlumber
│   │   │   ├── power.rs         # power-profiles-daemon D-Bus
│   │   │   ├── packages.rs      # query via alpm/libalpm; mutate via helper
│   │   │   ├── display.rs       # hyprland-rs IPC (monitors, live apply)
│   │   │   ├── input.rs         # keybinds/mouse → conf editor + keyword
│   │   │   ├── appearance.rs    # blur/opacity/font/wallpaper → conf + hyprpaper
│   │   │   ├── stats.rs         # sampler: sysinfo + hwmon + NVML
│   │   │   ├── hyprland_conf.rs # managed-block text editor (shared by above)
│   │   │   └── privileged.rs    # client side: spawn pkexec helper, parse result
│   │   └── tests/               # per-domain integration tests (CLI uses these too)
│   ├── archel-cli/
│   │   └── src/main.rs          # clap tree mirroring core modules
│   ├── archel-settings/         # Qt6 GUI
│   │   ├── src/
│   │   │   ├── main.rs          # Qt event loop entry, engine setup
│   │   │   └── models/          # cxx-qt QObject view-models per tab
│   │   ├── qml/
│   │   │   ├── Main.qml         # shell: vertical tabs
│   │   │   └── tabs/            # Landing, Network, Power, ... one file per tab
│   │   └── build.rs             # cxx-qt-build (moc/rcc via qt-build-utils)
│   └── archel-privileged/
│       └── src/main.rs          # argv allowlist → pacman/libalpm action → exit
├── policy/
│   └── org.archel.privileged.policy  # polkit action file
└── packaging/
    └── PKGBUILD / install layout
```

### Structure Rationale

- **`archel-core` has no Qt dependency:** keeps the API testable headless, lets `archel-cli` prove every capability before QML exists (explicit PROJECT.md ordering: API → CLI proof → GUI). Qt types must never leak into core signatures — view-models translate.
- **One module per settings domain, mirroring GUI tabs 1:1:** tab-per-module means GUI work parallelizes cleanly once the API surface per domain is frozen.
- **`hyprland_conf.rs` shared, not duplicated:** appearance, display-fallback, and input all edit `hyprland.conf`; a single managed-block editor prevents three divergent parsers.
- **Separate `archel-privileged` binary (not a core feature flag):** privilege boundary must be visible in the build graph — the helper compiles to its own minimal binary with no GUI/network code, minimizing audit surface.

## Architectural Patterns

### Pattern 1: Library-first, daemon-never (v1)

**What:** `archel-core` is a plain library; both frontends link it in-process. Live stats flow through in-memory channels, not IPC.
**When to use:** single-machine app where all consumers can link the same crate. **This is Archel v1.**
**Trade-offs:** (+) zero IPC latency/serialization — 1 Hz+ stat graphs are trivial; (+) no daemon lifecycle, socket activation, or version-skew bugs; (−) GUI crash takes down sampling (acceptable — sampler restarts with GUI); (−) a future remote/phone UI would need a daemon then, not now.

```rust
// archel-core: sampler exposes watch channels; GUI polls/copies at frame rate
pub struct StatsHandle {
    pub cpu_per_core: watch::Receiver<Vec<f32>>,
    pub temps: watch::Receiver<Temps>,
    pub power: watch::Receiver<PowerDraw>,
}
impl StatsSampler {
    pub fn spawn(interval: Duration) -> StatsHandle { /* tokio::task::spawn(...) */ }
}
```

### Pattern 2: CXX-Qt bridge — QML owns pixels, Rust owns state

**What:** Each GUI tab gets a Rust `QObject` view-model declared with `cxx-qt` macros; the CXX bridge generates the C++ side; QML binds to properties/signals only. No business logic in QML/JS, no Qt types in `archel-core`. Verified: CXX-Qt (KDAB) is the maintained path — safe interop via `cxx`, macro-declared QObjects, codegen'd C++ representation, CI-tested on Linux/Win/macOS. `qmetaobject-rs` (Woboq) is cargo-only and convenient but Qt5-era with low maintenance — do not choose it for a new Qt6 app.
**When to use:** any Rust + Qt6 GUI. Build with `cxx-qt-build` in `build.rs` (handles moc/rcc via `qt-build-utils`); requires Qt6 dev packages at build time.
**Trade-offs:** (+) memory-safe QObject threading model; (+) normal Rust + normal QML idioms instead of 1:1 C++ bindings; (−) build complexity (C++ toolchain + Qt6 headers required); (−) smaller talent pool than e.g. Tauri/Electron — acceptable for a personal distro tool.

```rust
// archel-settings view-model sketch (cxx-qt style)
#[cxx_qt::bridge(namespace = "archel")]
mod ffi {
    #[cxx_qt::qobject]
    #[derive(Default)]
    pub struct NetworkModel {
        #[qproperty] pub connected: bool,
        #[qproperty] pub ip_address: QString,
    }
    impl qobject::NetworkModel {
        #[qinvokable] pub fn refresh(&mut self) { /* fetch from archel-core */ }
    }
}
```

### Pattern 3: Tokio-on-background-thread + queued delivery to Qt

**What:** The Qt GUI thread **never awaits**. A dedicated tokio `Runtime` lives on a worker thread; results cross into Qt via `QueuedConnection` signal emission (or an `mpsc` → Qt-timer pump). Verified: `zbus` v5 has a first-class `tokio` cargo feature with no hidden threads — so the whole `archel-core` async stack can run on that same background runtime. `zbus` also ships blocking wrappers if a call site genuinely needs sync.
**When to use:** every GUI-initiated core call and every sampler tick.
**Trade-offs:** (+) UI can never be wedged by a slow D-Bus call; (+) matches CXX-Qt's safe cross-thread story; (−) all view-model mutations from worker threads must be queued, never direct — enforce by convention + code review.

```
[QML action] → [QObject slot, GUI thread] → (mpsc send) →
[tokio worker: archel-core async fn] → (result) →
[queued signal / QMetaObject::invokeMethod] → [QObject property update, GUI thread] → [QML re-renders]
```

### Pattern 4: pkexec one-shot helper + polkit policy (no resident root daemon)

**What:** `archel-privileged` is a tiny argv-allowlisted binary (`install <pkgs>`, `remove <pkgs>`, `upgrade`). `archel-core::privileged` spawns `pkexec archel-privileged …`, captures JSON result on stdout. A `/usr/share/polkit-1/actions/org.archel.privileged.policy` file gives friendly auth dialogs and lets the hotspot actions carry distinct message strings. NetworkManager writes need **no** custom helper — NM's own D-Bus endpoints enforce polkit (`org.freedesktop.NetworkManager.settings.modify`) and prompt via the existing agent.
**When to use:** pacman/yay mutations only. Reads (`-Q`, `-Qu`, `-Ss`) run unprivileged in-process. yay/AUR builds must run **as the user, never as root** — the helper installs built packages but the build itself stays unprivileged (makepkg refuses root anyway).
**Trade-offs:** (+) no always-on root process, tiny audit surface, GNOME-authenticator UX via polkit; (−) one auth prompt per operation burst — mitigate by batching package lists into a single helper invocation; (−) resident-daemon upgrade path exists later if prompts become annoying, but do not build it now.

### Pattern 5: Managed-block surgery for hyprland.conf; D-Bus for everything with a daemon

**What:** Two different write strategies, chosen per target:
- **Has a daemon (NetworkManager, BlueZ, power-profiles-daemon, PipeWire):** write exclusively through its D-Bus/IPC API via `zbus` proxies. Never edit its config files (`/etc/NetworkManager/…`) by hand — file writes bypass daemon state, need root, and race with the daemon.
- **Is a flat file (hyprland.conf):** no stable machine schema exists, so do line-preserving text surgery confined to `# >>> ARCHEL MANAGED >>> … # <<< ARCHEL` blocks; everything outside is the user's and is never touched. Live-apply via `hyprland-rs` `keyword` IPC first (instant feedback), then persist the same value into the managed block. Verified: `hyprland-rs` wraps the Hyprland IPC socket (monitors, workspaces, keywords) — use it for reads/live-apply, not for conf parsing.
**Trade-offs:** (+) user comments/formatting survive (a full parse-and-rewrite would destroy them — the #1 config-editor pitfall); (+) live-apply + persist keeps compositor and file consistent; (−) drift if the user hand-edits inside a managed block — mitigate by re-reading the block before each write and warning on mismatch.

## Data Flow

### Request Flow (read path — e.g. Landing tab)

```
[QML view] ─binds→ [Rust QObject view-model (GUI thread)]
    ↑ property change (queued)              │ refresh() slot
    │                                       ↓ (mpsc)
                              [tokio worker: archel-core fns]
                                ├─ packages: alpm query (unprivileged)
                                ├─ system: uname, kernel version
                                └─ stats: latest watch-channel snapshot
```

### Request Flow (privileged write path — e.g. install package)

```
[QML confirm] → [view-model] → [core::privileged::install(pkgs)]
    → [pkexec archel-privileged install …] → [polkit auth dialog]
    → [helper: libalpm transaction, JSON to stdout, exit]
    → [core parses result] → [queued signal] → [QML success/error toast]
```

### State Management

- **Ephemeral UI state** (selected tab, dialog open): QML-local properties.
- **System state**: never cached authoritatively in the GUI. View-models hold last-known snapshots refreshed on tab-focus + explicit refresh + relevant D-Bus signal (NM `StateChanged`, BlueZ `PropertiesChanged` via `zbus` signal streams).
- **Live stats**: sampler-owned `watch` channels at 1 Hz (CPU per-core ring of ~60 samples for graphs, temps, GHz, VRAM via NVML, power via hwmon/RAPL). QML graph views bind to a `QAbstractListModel` (or `QVariantList` property) that the view-model refreshes from the channel on a 1 s `QTimer` — decoupled from sampling so a slow frame never drops samples.
- **Config intent** (desired blur/opacity/keybinds): lives in `hyprland.conf` managed blocks; view-models re-read after write to confirm.

### Key Data Flows

1. **Stats stream (GUI):** `sysfs/hwmon/NVML → sampler thread (1 Hz) → watch channels → view-model (QTimer 1 s) → QML graphs.** Highest-frequency path; entirely in-process, no IPC.
2. **CLI probe:** `archel-cli stats --once → core sampler single-shot → stdout JSON/table.** Same code path as GUI minus the loop.
3. **Network write:** `QML → core::network (zbus NM proxy) → NM daemon → polkit (NM's own) → kernel.** No files, no helper.
4. **Package mutate:** `QML → core::privileged → pkexec helper → libalpm → stdout JSON → GUI.** Only path that crosses the privilege boundary.
5. **Hyprland tue:** `QML → core::appearance/display/input → hyprland-rs keyword (live) + managed-block persist (durable).** Live-first, persist-second.

## Scaling Considerations

Single-machine personal tool — "scale" means distro-install count and maintainer burden, not users-per-second.

| Scale | Architecture Adjustments |
|-------|--------------------------|
| v1, 1 machine (axel) | Library + CLI + GUI + pkexec helper as specified. No daemon. |
| Shared with friends / AUR package | Same shape; hardening matters: helper argv allowlist fuzzing, polkit policy review, `archel-core` error taxonomy stable for CLI scripting. |
| If a remote/mobile companion ever appears | Add `archel-daemon` exposing `archel-core` over D-Bus (`zbus #[interface]`) on the session bus; GUI/CLI gain a transport flag. Stats then flow as D-Bus signals (throttle to 1 Hz, fixed-point to keep marshalling cheap). Do not pre-build this. |

### Scaling Priorities

1. **First bottleneck:** QML model updates for per-core graphs if core count × sample rate grows — cap ring buffers (60–120 points), update models at 1 Hz max, decimate in Rust before crossing the bridge.
2. **Second bottleneck:** `zbus` signal storms (BlueZ discovery) — subscribe narrowly per tab (only while tab visible), unsubscribe on tab switch.

## Anti-Patterns

### Anti-Pattern 1: Resident root daemon from day one

**What people do:** build a `archel-daemon` running as root with a custom D-Bus API for everything privileged.
**Why it's wrong:** always-on root attack surface, daemon lifecycle/socket-activation debugging, API version skew between daemon and GUI — all for operations that happen a few times a day (package installs).
**Do this instead:** pkexec one-shot helper; graduate to a daemon only with evidence (e.g. unattended-upgrade scheduling needs).

### Anti-Pattern 2: Parsing + rewriting hyprland.conf wholesale

**What people do:** deserialize the conf into structs, modify, serialize back.
**Why it's wrong:** destroys comments, blank lines, ordering, and user sections the parser doesn't understand — instant user fury on a hand-tuned tiling config.
**Do this instead:** managed-block surgery; unknown/outside content is byte-preserved.

### Anti-Pattern 3: Shelling out to nmcli/pacman text output

**What people do:** `Command::new("nmcli")` + regex; `pacman -Qu` string scraping in core logic.
**Why it's wrong:** brittle across versions/locales, no change notifications, spawns processes at 1 Hz for stats.
**Do this instead:** `zbus` proxies for NM/BlueZ/ppd (typed, signal-capable); `libalpm` bindings for pacman queries; subprocess only where no library exists (yay query wrapper with stable `--quiet` flags, and the pkexec helper boundary itself).

### Anti-Pattern 4: Blocking the Qt thread on async work

**What people do:** `block_on` inside a slot, or synchronous D-Bus calls on the GUI thread.
**Why it's wrong:** every NM hiccup freezes the whole settings window; deadlock risk with `zbus` internal executor.
**Do this instead:** Pattern 3 — tokio worker thread, queued delivery, optimistic UI with error toasts.

### Anti-Pattern 5: Business logic in QML/JS

**What people do:** parsing, validation, and system calls in `.qml`/JS for speed.
**Why it's wrong:** untestable headless, duplicates `archel-cli` behavior, JS has no access to `archel-core` types.
**Do this instead:** QML binds properties and calls invokables; all logic lives in Rust (core or view-model).

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| NetworkManager | `zbus` async proxies (`org.freedesktop.NetworkManager`); signals for state | Writes use NM's own polkit; DNS per-connection via `Settings.Connection.Update`; proxy/VPN via NM profiles — no file writes |
| BlueZ | `zbus` proxies (`org.bluez.Adapter1/Device1`); discovery session per tab-visit | Pairing needs agent registration — implement minimal `Agent1` in core; wayland has no impact |
| power-profiles-daemon | `zbus` proxy (`net.hadess.PowerProfiles`); `ActiveProfile` property + `ProfilesChanged` | Fallback: `powerprofilesctl` only if daemon absent; TLP conflicts — detect and warn, don't fight it |
| PipeWire/WirePlumber | `libpipewire-rs` or `pw-cli`/`wpctl` JSON for device enum; `DefaultNodes` for select | Read-heavy; default-sink switch is the main write — via WirePlumber API, not config files |
| Hyprland IPC | `hyprland-rs` crate over `$HYPRLAND_INSTANCE_SIGNATURE` socket | Live apply (`keyword`), monitors (`monitors`); guard every call on compositor presence for CLI-over-SSH use |
| pacman | `libalpm` (via `alpm` crate) for queries; helper binary for transactions | Reads unprivileged; writes only via helper |
| yay/AUR | unprivileged `yay -Qu --aur` query wrapper; build as user, install via helper | Never run makepkg/yay-build as root |
| sysfs/hwmon/RAPL, NVML | direct sysfs reads + `nvml-wrapper` for NVIDIA; `sysinfo` crate for CPU/RAM/disk | Vendor-dependent (AMD vs NVIDIA) — feature-detect at runtime, degrade gracefully |
| polkit | `.policy` XML for helper actions; `pkexec` invocation from core | Action IDs namespaced `org.archel.privileged.*`; one action per verb for granular messages |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| CLI ↔ core | direct function calls, `tokio::main` | CLI is the API conformance harness; every GUI-visible behavior must be CLI-reachable first |
| QML ↔ view-models | properties/signals/invokables over `cxx-qt` bridge | Qt types stop at view-models; core signatures stay Qt-free |
| view-models ↔ core | `mpsc` request + queued result delivery via tokio worker thread | GUI thread never awaits; see Pattern 3 |
| core ↔ helper | `pkexec` spawn, argv allowlist, JSON over stdout, exit codes | Only boundary crossing privilege levels; keep protocol versioned (`{"v":1,…}`) |
| core modules ↔ hyprland_conf | internal function calls (read_block/write_block/verify) | Single editor prevents divergent parsers |
| sampler ↔ consumers | `tokio::sync::watch` channels + snapshot fns | CLI polls once; GUI subscribes at 1 Hz |

## Build Order (dependencies between components)

```
 1  error.rs + hyprland_conf.rs (foundations; everything Hyprland touches needs the editor)
 2  stats sampler ─────────────────────── (independent; no deps; early win, feeds Landing tab)
 3  packages QUERY (libalpm read-only) ── (independent; feeds Landing update-count)
 4  display READ + input READ + appearance READ (hyprland-rs IPC + conf-block reads)
 5  network READ + bluetooth READ + audio READ + power READ (zbus proxies; read-only first)
 6  archel-cli over all of the above ──── (API proof gate: CLI must work before any write path)
 7  privileged helper + polkit policy + packages MUTATE (first write path; needs CLI harness)
 8  network/Bluetooth/audio/power WRITES (daemon-mediated; each needs its read path + CLI proof)
 9  hyprland conf WRITES (appearance/display/input live-apply + persist)
10  archel-settings shell (vertical tabs, Landing + System Info first — read-only tabs)
11  remaining GUI tabs (one per proven domain module)
```

Notes: stats (#2) and package queries (#3) are dependency-free — start them first for momentum and to feed the Landing tab. Every write path (#7–#9) depends on its read path plus CLI proof (#6) — this is the "API first, proven via CLI" gate from PROJECT.md. The GUI shell (#10) depends on the full read API being CLI-proven; individual tabs land as their domain's write paths complete. The privileged helper (#7) is the only component needing root-aware code review and should get a dedicated security pass before packaging.

## Sources

- CXX-Qt repo (github.com/KDAB/cxx-qt) + CXX-Qt Book intro (kdab.github.io/cxx-qt/book) — bridge concept, QObject macros/codegen, CI platforms [MEDIUM, scraped]
- qmetaobject-rs repo (github.com/woboq/qmetaobject-rs) — derive-based QML registration, cargo-only, Qt ≥ 5.8 [MEDIUM, scraped]
- zbus crates.io v5.19.0 readme — proxy/interface macros, tokio feature (no hidden threads), blocking wrappers [MEDIUM, scraped]
- hyprland-rs repo (github.com/hyprland-community/hyprland-rs) — unofficial Hyprland IPC wrapper, 0.4 in development [MEDIUM, scraped]
- Curated Linux-settings knowledge: NM/BlueZ/ppd D-Bus ownership, pkexec-vs-daemon trade-offs, managed-block conf editing, hwmon/NVML sampling (LOW–MEDIUM, needs phase-level verification during implementation)

---
*Architecture research for: Archel — Rust settings API + CLI + Qt6 GUI on Arch/Hyprland*
*Researched: 2026-09-30*
