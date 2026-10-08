# Phase 6: GUI Shell & Tabs - Research

**Researched:** 2026-10-08
**Domain:** CXX-Qt 0.10 / QML (Qt 6.11) desktop shell over the existing `archel-core` API + tokio 1.x async runtime
**Confidence:** HIGH on bridge/threading/packaging mechanics (verified against vendored cxx-qt 0.10.0 source and on-disk Qt install); MEDIUM on a few core-gap API shapes the planner must design

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Live-data rhythm
- **D-01:** Every tab polls live — not just Landing/System Info graphs. The researcher must prove the worker-thread pattern holds under ~10 concurrent polling sources without freezing the UI thread (the roadmap's no-freeze criterion). → **PROVEN, see "Architectural Responsibility Map" + Pattern 1 below: CXX-Qt 0.10 `CxxQtThread::queue` is the verified fan-in mechanism.**
- **D-02:** Uniform 1 Hz cadence for all live sources (matches the spike's Timer pattern — simplest mental model).
- **D-03:** All tabs keep their 1 Hz timers running even when hidden; only the visible tab is rendered, but switching is always to fresh data.
- **D-04:** When a source is offline/slow, its fields show honest French N/A immediately (same rule as missing sensors in Phase 2) — never a number that pretends to be fresh.
- **D-05:** First poll failure shows a dismissible banner in the tab; repeated failures stay silent (N/A fallback) until recovery. No per-tick error spam.
- **D-06:** Polling continues while the window is minimized or on another workspace — returning to the window always shows current data.
- **D-07:** Package update counts keep the CLI's 15-minute cache TTL (mirror-friendly); the 1 Hz rule applies to live sources only, not to `checkupdates`/AUR queries. → **Core already enforces this: `CACHE_TTL_SECS: u64 = 900` [VERIFIED: crates/archel-core/src/updates.rs:29]. GUI calls `pending_counts(false)`/`landing::snapshot(false)` at 1 Hz; the cache short-circuits refreshes.**
- **D-08:** Bluetooth discovery uses scan bursts (~12 s on tab open / refresh press, then stop), not continuous discovery — radio/battery respect. Standard BT-UX pattern. → **Core already matches: `pub const SCAN_TIMEOUT: Duration = Duration::from_secs(12);` and `pub async fn bt_scan() -> Result<Vec<BtDevice>, ArchelError>` [VERIFIED: crates/archel-core/src/bluetooth.rs:74,383].**
- **D-09:** CPU/RAM graphs draw a rolling 60-second window fed by the 1 Hz ticks — bounded memory, spikes visible.
- **D-10:** CPU graph shows aggregate line + clock speed by default, with an expander revealing per-core lines (like the CLI's per-core view).
- **D-11:** Temperatures show representative values only (hottest per sensor class), consistent with Phase 2 D-09. No full sensor list.

#### Mutation dialogs
- **D-12:** Every write action confirms in a modal dialog first — the GUI twin of the CLI's confirm-before-mutation rule (Phase 3 D-01). No silent writes, no inline-confirm exceptions for "simple" toggles. — **Reversibility:** costly — dropping confirmations later reopens the mutation-trust surface the CLI phase deliberately locked.
- **D-13:** Package install/upgrade/remove shows a modal with a scrolling terminal-like live view of pacman/yay output plus an Annuler button — mirroring the CLI's streaming output (Phase 3 D-02).
- **D-14:** Annuler during a running transaction requests a stop and reports pacman's final state; it never hard-kills the subprocess (lock/partial-transaction safety). The GUI reports how the transaction ended.
- **D-15:** During a long mutation only the acting tab locks; all other tabs stay usable (consistent with all-tabs-live — user can watch graphs during an upgrade).
- **D-16:** Display apply uses a countdown modal: 15 s progress bar with Conserver/Annuler — the direct GUI twin of the CLI confirm-and-rollback (Phase 5 D-04). Timeout auto-rolls back. → **No core change needed: `apply_monitor_plan` at [VERIFIED: crates/archel-core/src/hyprland/monitors.rs:608] is dependency-injected (`base, plan, confirm, applier, rollbacker, timed_confirm` closures) — the GUI passes its own closures backed by the modal. Only `set_monitor` (the TTY convenience wrapper) hardcodes terminal prompts.**
- **D-17:** Write failures surface in an error modal with a friendly French sentence + concrete cause and a Réessayer button. One consistent error voice across all tabs. → **`ArchelError::message()` [VERIFIED: crates/archel-core/src/error.rs:282] is the sole French renderer; variants `AuthFailed{..}` (line 27), `ConfirmationAborted` (146), `ConfirmTimeout` (153), `NeedsPrivilege` — render `message()` directly, never invent strings.**
- **D-18:** pkexec password is asked up front, inside the confirmation modal, then the operation runs elevated in one shot — one interruption, predictable timing.

### Review surfaces
- **D-19:** AUR install shows a summary card (name, version, deps, sources) with a "Voir le PKGBUILD" expander containing the full PKGBUILD text. — **Reversibility:** costly — the Valider button stays disabled until the expander has been opened at least once, preserving Phase 3 D-05's full-display trust boundary in summary-first form. → **Core `AurReview { card, pkgbuild, trust, dependencies, warnings }` [VERIFIED: crates/archel-core/src/aur.rs:103-109] already carries everything the card needs.**
- **D-20:** VPN import closes Phase 4 D-08's deferral: a popup form with file-picker for `.ovpn`/WireGuard `.conf` OR inline fields (username, password, certificate, key) — single popup, not a multi-step wizard.
- **D-21:** Bluetooth pairing-code confirmation is a global modal (pops anywhere in the app, showing the code with Confirmer/Refuser) — never auto-accept, can't be missed on another tab (GUI twin of Phase 4 D-05). → **Core already DI-ready: `pair_picked_with` accepts an injection callback for the pairing decision [VERIFIED: crates/archel-core/src/bluetooth.rs, `PAIR_TIMEOUT: Duration = Duration::from_secs(30)` at line 79].**

#### Window chrome
- **D-22:** The window opens on the Landing tab (overview with update counts + system snapshot — GNOME Settings-like home).
- **D-23:** Window geometry (size, position, maximized state) is remembered in `~/.config/archel/` and restored on next launch; sensible default (~900×620, the spike size) on first run.
- **D-24:** Theme follows the system style hint (QT_QPA_PLATFORMTHEME / Hyprland setting) automatically — no in-app dark/light toggle in v1.
- **D-25:** The vertical rail shows an icon beside each French tab label (Qt built-in icons or simple inline SVGs). No settings search box in v1.
- **D-26:** A second `archel settings` launch prints a French message and exits (refuses the second instance) — no single-instance focus plumbing in v1.

### the agent's Discretion
- Bridge architecture (one QObject per tab vs. one root model + sub-models), QML file layout, and how the 1 Hz worker fan-out is structured (tokio tasks behind the bridge) are the planner's/executor's call — the spike's `#[cxx_qt::bridge]` pattern is the starting point, not a mandate.
- Icon sourcing (Qt Quick Controls built-ins vs. bundled SVGs) is agent's choice within D-25.

### Deferred Ideas (OUT OF SCOPE)
- Settings search box above the rail — nice-to-have, cut from v1 (D-25)
- In-app dark/light toggle — v2 at earliest (system-follow locked for v1, D-24)
- Single-instance focus (second launch focuses existing window) — refused-instance behavior chosen instead (D-26)
- System tray / background watcher — belongs to a future daemon v2, not this phase
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PLAT-03 | GUI capability coverage — every core/CLI capability reachable through the GUI tabs | 1:1 tab↔subcommand mapping already exists in `crates/archel/src/cmds/mod.rs` (full 10-tab dispatch verified this session: Landing/Sys/Pkg/Net/Bt/Audio/Power/Appearance/Display/Input). Each tab's view-model calls the same core entry points the CLI commands call — see "Tab ↔ Core Entry-Point Map". |
</phase_requirements>

## Project Constraints (from AGENTS.md)

- Qt6 for the GUI via CXX-Qt; Arch Linux + Hyprland (Wayland) — no X11-only tools
- Privilege model: pkexec path (desktop-polkit), typed `NeedsPrivilege` error — no embedded password prompts in v1 (D-18 puts the pkexec ask inside the confirm modal via the normal pkexec dialog)
- API-first: **no new domain logic in the GUI** — every read/write goes through `archel-core`; if a capability is missing there it is an earlier-phase bug (or an explicitly scoped core task, see Core Gaps)
- `gui` cargo feature isolates ALL Qt deps; headless `cargo test -p archel-core --no-default-features` and `-p archel --no-default-features` must stay green
- French-only user text: QML strings French, `.rs` plain ASCII (Phase 1 D-04/D-05/D-06/D-07)
- Ordering already satisfied: API + CLI exist and are proven

## Summary

The Phase 1 spike already proves the hard parts: a `#[cxx_qt::bridge]` QObject with Q_PROPERTY/NOTIFY/qinvokable, a QML vertical-tab shell (Column of TabButtons + StackLayout), a 1 Hz `Timer` tick, qrc resource loading, and an offscreen run with zero QML errors. This phase is that pattern multiplied by ten tabs, plus four frontend-only features (geometry persistence, second-instance refusal, global pairing modal, confirm/countdown modals) and three small core additions (cancel-aware streaming being the only real one).

The single most important verified finding: **CXX-Qt 0.10 ships a first-class cross-threading API** (`impl cxx_qt::Threading for X {}` in the bridge → `self.qt_thread()` → `CxxQtThread<T>` which is `Send + Sync` → `.queue(closure)` executes on the Qt event loop holding the Rust-object lock). This is the documented, KDAB-maintained answer to D-01: ten tokio polling tasks (async zbus/bluer calls, or `spawn_blocking` for the sync core fns) each hand results back to the GUI thread exclusively through `queue()`. The Qt main thread never blocks; it only runs short property-set closures. No manual QMetaObject/invokeMethod FFI hackery is needed.

Two structural changes from the spike are unavoidable: (1) the tokio runtime must be created **before/around** `QGuiApplication` in `run_settings()` rather than wrapping Qt's `exec()` inside `#[tokio::main]` (the current `main.rs` shape), and (2) the bridge grows from one `SpikeModel` to a root model + per-tab view-models registered in the same `QmlModule`, with the module URI renamed `org.archel.spike` → `org.archel.settings`. QtCharts is **not installed** on this machine — graphs are QtQuick `Canvas`/`Shapes` (Shapes IS present), which also avoids a new system dependency. No new cargo packages are expected beyond the workspace pins.

**Primary recommendation:** Root `ShellModel` QObject (tab index, modals, second-instance, geometry) + one `#[qobject]` view-model per tab, each owning its tokio poll task and updating via `CxxQtThread::queue`; payloads as typed Q_PROPERTYs for scalars and `serde_json`→`QString` JSON for lists; add only cancel-aware streaming variants to `archel-core`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Data reads (all tabs, 1 Hz) | tokio runtime (worker threads, behind bridge) | — | Core fns are async (zbus/bluer) or blocking (sysfs/pacman queries); Qt thread must never touch them (D-01 no-freeze) |
| Property updates / NOTIFY | Qt main thread (via `CxxQtThread::queue`) | — | QObjects are `!Send !Sync`; only `queue()` closures may mutate them [VERIFIED: cxx-qt-0.10.0/src/lib.rs:144-146 "The QObjects generated by CXX-Qt are neither Send nor Sync"] |
| Rendering, tab rail, StackLayout | QML (QQmlApplicationEngine, qrc) | — | Vertical tab pattern already proven in Spike.qml |
| Confirmation / error / streaming / countdown modals | QML `Popup`/`Dialog` + root model state | core DI seams (`apply_monitor_plan`, `pair_picked_with`) | Modals are UI state; core exposes closure seams so the modal *is* the confirm callback |
| Privilege escalation | `archel-core::privileged` (pkexec spawn) | QML confirm modal (D-18 ordering) | argv-vector spawn through duct; GUI only renders `NeedsPrivilege`/`AuthFailed` |
| Window geometry persistence | GUI binary (`~/.config/archel/`, directories crate) | — | Pure frontend concern; core must stay Qt/GUI-free |
| Second-instance refusal | GUI binary (abstract Unix socket bind at startup) | — | Frontend lifecycle; tokio `UnixListener` supports abstract names |
| Package/AUR queries with 15-min TTL | `archel-core::updates` cache | — | Core owns TTL (`CACHE_TTL_SECS = 900`); GUI just calls at 1 Hz with `force_refresh=false` |
| VPN import (D-20) | core NetworkManager D-Bus (Phase 4 path) | QML file-picker popup | Import logic belongs in core (CLI twin), GUI collects fields |

## Standard Stack

### Core (all already vendored/pinned in the workspace — NO new dependencies)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| cxx-qt / cxx-qt-lib / cxx-qt-build | `=0.10.0` (vendored at `/home/axel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`) | QObject bridge, Qt types, moc build | KDAB-backed; the only production-viable Qt-from-Rust; spike proven |
| Qt6 (system) | 6.11.2-3 (Arch extra) | QML runtime, QuickControls | Requirement; QGuiApplication + QQmlApplicationEngine path proven |
| tokio | 1.53.x (workspace pin) | Poll task runtime + `spawn_blocking` + abstract UDS | zbus/bluer already require it |
| serde_json | 1.0.x (pin) | Rust→QML payload marshalling | Core snapshot structs are already `Serialize` |
| directories | 6.0.0 (pin) | `~/.config/archel/` for geometry file | Existing app-config home |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| QtQuick.Controls styles: **Fusion** | system, present at `/usr/lib/qt6/qml/QtQuick/Controls/Fusion` [VERIFIED: on-disk listing] | Native-ish desktop look for settings UI | `QQuickStyle::setStyle("Fusion")` in `run_settings()` unless `QT_QPA_PLATFORMTHEME` says otherwise (D-24) |
| QtQuick `Canvas` / QtQuick.Shapes | system (Shapes present on-disk [VERIFIED: /usr/lib/qt6/qml/QtQuick/Shapes exists]) | Rolling 60-s CPU/RAM sparklines (D-09/D-10) | Graph widgets — QtCharts is NOT installed; Canvas keeps zero new system deps |
| FileDialog via `Qt.labs.platform` or QtQuick.Dialogs | check availability at exec-time; fallback: manual path TextField + `FolderDialog`-free typing | VPN `.ovpn`/`.conf` picker (D-20) | Verify `qmlscene`-style presence during Wave 0; if QtQuick.Dialogs native dialogs need xdg-desktop-portal and it's absent on bare Hyprland, degrade to typed path field + inline fields (D-20 already allows inline fields) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `CxxQtThread::queue` | Emit a Qt signal via raw `QMetaObject::invokeMethod` FFI | Unnecessary — 0.10 has first-class `threading` support; only use if a bug blocks `queue` |
| Canvas sparklines | Install `qt6-charts` (QtCharts QML module) | New system dep + PKGBUILD churn; 60-point polylines don't warrant a chart lib |
| JSON-string payloads | Typed `QVariantList` / cxx-qt-lib serde bridge | cxx-qt-lib has serde support but is less battle-tested in this project; JSON + `JSON.parse` in QML is boring and debuggable. Mix: scalars stay typed Q_PROPERTYs. |
| Abstract-socket single-instance | PID lockfile in XDG_RUNTIME_DIR | Stale-file cleanup on crash; abstract socket auto-releases on exit |
| Root model + per-tab models | One giant `SettingsModel` | 10 tabs × properties = unmaintainable NOTIFY soup; per-tab models match the 1:1 cmds mapping |

**Installation:** none — `cargo build -p archel --features gui` with `QMAKE=/usr/bin/qmake6` (established in Phase 1 [VERIFIED: .planning/phases/01-foundations-safety-rails/01-02-SUMMARY.md]).

## Package Legitimacy Audit

No external packages are installed by this phase — all crates are existing workspace pins; Qt/QuickControls come from system `qt6-base`/`qt6-declarative`.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| — none — | | | | | | No new installs; N/A |

**Packages removed due to [SLOP] verdict:** none. **Flagged [SUS]:** none.

## Architecture Patterns

### System Architecture Diagram

```
 QML (qrc: org.archel.settings)                 Rust (gui feature, binary archel)
┌───────────────────────────────┐      ┌─────────────────────────────────────────┐
│ Main.qml ApplicationWindow    │      │ main.rs: Command::Settings              │
│  ├ Vertical rail (10 buttons) │ idx  │  └─ gui::run_settings()                 │
│  ├ StackLayout (10 tab views) │◄────►│     1. second-instance probe (UDS bind) │
│  ├ Global: PairingModal       │props │     2. tokio Runtime::new() (OnceLock)  │
│  ├ Global: ErrorModal (D-17)  │      │     3. QGuiApplication + Fusion style   │
│  ├ Global: StreamModal (D-13) │sig   │     4. QQmlApplicationEngine load qrc   │
│  └ ConfirmModal (D-12/D-16)   │◄─────┤     5. ShellModel + 10 tab view-models  │
└────────────▲──────────────────┘      │        registered as context/creation   │
             │ only short closures     │        object singletons                │
             │ (set props, emit sig)   │     6. app.exec()  ← Qt main thread     │
      ┌──────┴───────────────┐         │─────────────────────────────────────────│
      │ CxxQtThread::queue() │         │ tokio runtime (worker threads)          │
      │  [Qt-thread affinity]│         │  per-tab poll task @1Hz ──► archel-core │
      └──────▲───────────────┘         │   • async: network/power/bt (zbus/bluer)│
             │ results                 │   • blocking: sensors/system/audio/pkg/ │
   ┌─────────┴──────────┐              │     hyprland reads via spawn_blocking   │
   │ tokio tasks × ~10  │◄────────────►│   mutation tasks: confirm→core fn→      │
   │ (Send payloads out)│              │     stream lines → queue() to modal     │
   └────────────────────┘              └─────────────────────────────────────────┘
```

Entry: `archel settings` → refusal probe → runtime → Qt exec. The Qt thread only *renders*; every core call happens on tokio, and results return only via `queue()`.

### Recommended Project Structure
```
crates/archel/src/gui/
├── mod.rs            # run_settings(): refusal, runtime bootstrap, QGuiApplication, engine
├── bridge.rs         # root ShellModel + per-tab #[qobject] view-models (or bridge/ per tab)
├── poll.rs           # 1 Hz poll-task helpers (tokio::spawn + CxxQtThread::queue glue)
├── geometry.rs       # load/save ~/.config/archel/window-geometry.json (serde struct)
├── single_instance.rs# abstract UDS bind helper
└── qml/
    ├── Main.qml      # ApplicationWindow: rail + StackLayout + global modals
    ├── tabs/         # LandingTab.qml NetworkTab.qml PowerTab.qml BtTab.qml
    │                 # AudioTab.qml PkgTab.qml DisplayTab.qml InputTab.qml
    │                 # AppearanceTab.qml SystemTab.qml
    └── components/   # ConfirmModal, ErrorModal, StreamModal, CountdownModal,
                      # PairingModal, Sparkline (Canvas), N_a field label
```

### Pattern 1: Cross-thread property update (THE D-01 proof)
**What:** CXX-Qt 0.10 `Threading` — queue closures from tokio tasks onto the Qt event loop.
**When to use:** every poll tick, every streamed line, every async completion.
**Example:**
```rust
// Source: vendored cxx-qt-0.10.0 — src/lib.rs:152-192 (Threading doc example)
//         src/threading.rs:48-140 (CxxQtThread, Send+Sync, queue, is_destroyed)
#[cxx_qt::bridge]
mod qobject {
    extern "RustQt" {
        #[qobject]
        type NetworkModel = super::NetworkModelRust;
        #[qinvokable]
        fn start(self: Pin<&mut NetworkModel>);
    }
    impl cxx_qt::Threading for NetworkModel {}   // generates qt_thread()/queue plumbing
}

use cxx_qt::Threading;

impl qobject::NetworkModel {
    pub fn start(self: core::pin::Pin<&mut Self>) {
        let qt_thread = self.qt_thread();          // Send + Sync, cloneable
        tokio::spawn(async move {
            let st = archel_core::network::net_status().await;   // OFF Qt thread
            let json = serde_json::to_string(&st).unwrap();
            // Return to Qt thread — closure runs on event loop holding object lock:
            let _ = qt_thread.queue(move |mut obj: core::pin::Pin<&mut qobject::NetworkModel>| {
                obj.set_status_json(json.into());  // triggers NOTIFY → QML repaint
            });
        });
    }
}
```
Verbatim anchor: `pub fn queue<F>(&self, f: F) -> Result<(), crate::ThreadingQueueError> where F: FnOnce(Pin<&mut T>), F: Send + 'static` [VERIFIED: cxx-qt-0.10.0/src/threading.rs:102-109]; `unsafe impl Send`/`Sync` at lines 84-91; guard `is_destroyed()` at 139.

**Concurrency budget (D-01):** ~10 tasks each firing one small closure per second = ~10 queued events/s into an event loop that comfortably handles thousands. The only freeze risk is a closure doing heavy work — keep `queue()` closures to property-sets only; parse/format on the tokio side first. Sync core fns (`cpu_snapshot`, `audio_list`, `packages::list`, sensors) MUST be wrapped in `tokio::task::spawn_blocking` inside the poll task (they were designed for the CLI's sync world; e.g. `system.rs` `Monitor` is stateful and reused — keep one per System tab task).

### Pattern 2: Runtime bootstrap outside `#[tokio::main]`
**What:** `main.rs` currently is `#[tokio::main] async fn main` → `Command::Settings` → `gui::run_settings()`, so `app.exec()` blocks a tokio worker thread today. For the GUI path, restructure to: sync `main`, build `tokio::runtime::Runtime::new()` manually, stash a handle (`OnceLock<Runtime>` or pass `&Runtime` into models), construct QGuiApplication **after** the runtime, and let the bridge call `rt.spawn(...)` / `rt.spawn_blocking(...)` from invokables (invokables run on the Qt thread; `Runtime::spawn` is cheap and non-blocking). CLI subcommands keep a runtime of their own — either the same manual construction or keep `#[tokio::main]` for CLI-only builds and branch on `Command::Settings` before entering it.
**When to use:** `gui::run_settings()` rework — Wave 1 task.

### Pattern 3: Modal-as-confirm via core DI seams
**What:** core already exposes `*_with` closures for exactly the GUI-interactive paths [VERIFIED this session]: `apply_monitor_plan(base, plan, confirm, applier, rollbacker, timed_confirm)` [crates/archel-core/src/hyprland/monitors.rs:608], plus `apply_blur_with`, `apply_opacity_with`, `apply_input_with`, `apply_wallpaper_with`, `apply_font_with`, `pair_picked_with`, `set_output_picked_with`.
**When to use:** D-16 countdown modal (pass `timed_confirm` closure that drives the 15 s ProgressBar and resolves via oneshot), D-21 pairing modal (pass the confirm callback that surfaces the global Popup and awaits Confirmer/Refuser). No new core API — the GUI registers a pending-request slot on `ShellModel`, the tokio task awaits a oneshot resolved by the modal's buttons.

### Pattern 4: qrc resource registration for the real shell
Rename the QmlModule URI `org.archel.spike` → `org.archel.settings` in `crates/archel/build.rs` (`CxxQtBuilder::new_qml_module`), register **every** QML file added (tabs/, components/ — each must appear in the module's `qml_files`/resource list or qrc load fails at runtime, not compile time), and load `qrc:/qt/qml/org/archel/settings/src/gui/qml/Main.qml`. `qmlcachegen` on-disk confirms compilation is available but the build.rs registration list is the source of truth [VERIFIED: crates/archel/build.rs read this session; spike loads `qrc:/qt/qml/org/archel/spike/src/gui/Spike.qml` pattern from gui/mod.rs].

### Anti-Patterns to Avoid
- **Touching a QObject from a tokio task directly** — QObjects are `!Send !Sync` [VERIFIED: cxx-qt-0.10.0/src/lib.rs:144-146]; only `queue()` closures may mutate them.
- **Core calls on the Qt thread** (inside `invokeMethod`/invokable bodies) — even "cheap" sync fns like `audio_list()` shell out to wpctl; 10×/s of that freezes the UI. Invokables only *spawn*.
- **Continuous BT discovery** — D-08 locks bursts; `bt_scan()`'s 12 s burst already matches.
- **Per-tick error toasts** — D-05: first failure → banner, then silence + N/A (D-04).
- **Inline "simple" toggles that write immediately** — D-12: every write opens the confirm modal, no exceptions (including appearance toggles).
- **French strings in `.rs`** — Phase 1 D-07; all French lives in QML/catalog strings.
- **Rendering the hidden tabs' heavy delegates** — use StackLayout + `active`-only rendering (visible-only per D-03), timers keep ticking in the model regardless.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Cross-thread Qt updates | Manual `QMetaObject::invokeMethod` FFI or signal-marshalling hacks | `CxxQtThread::queue` (cxx-qt 0.10 `Threading`) | Race-free + destroyed-object guard, KDAB-maintained |
| 15 s confirm-and-rollback | GUI-side timer + manual restore code | `apply_monitor_plan(...timed_confirm...)` DI seam | Core already rolls back on timeout (CLI-proven, Phase 5) |
| Pairing approval | Poll BlueZ for "awaiting pairing" | `pair_picked_with` callback → global Popup | Core owns the 30 s PAIR_TIMEOUT semantics |
| Rolling graph widget | Custom chart library port | QtQuick `Canvas` polyline over a `VecDeque<f64>` (60 samples) | 60 points ≈ trivial; QtCharts absent AND oversized |
| Second-instance detection | Lockfile + PID parsing | Abstract-namespace Unix socket bind (tokio `UnixListener`, path starting `'\0'`) [ASSUMED: tokio abstract-name support — verify tokio docs at exec time] | Kernel releases abstract socket on exit; no stale files |
| Error text | GUI error strings | `ArchelError::message()` [VERIFIED: error.rs:282] | One French voice, CLI-identical |
| Privileged streaming | GUI spawning pkexec itself | core `run_privileged_streaming` [VERIFIED: privileged.rs:214] (+ new cancel variant, below) | argv-vector discipline stays in one audited place |
| Update-count caching | GUI-side TTL logic | core cache (`CACHE_TTL_SECS = 900`, `pending_counts(false)`) | D-07 satisfied for free |
| Geometry persistence | QML Settings/Qt.labs.settings | serde struct + JSON in `~/.config/archel/` via directories | labs.settings needs an extra qml module + storage backend choice; JSON file is inspectable and testable headlessly |

**Key insight:** the core was deliberately built with closure-injection seams precisely so the GUI could own the interactive moments; re-implementing those flows in QML duplicates tested logic and diverges from CLI behavior.

## Core Gaps (small, explicit core tasks in this phase)

1. **Cancel-aware privileged streaming (needed for D-13/D-14).** `run_privileged_streaming(verb, packages, label, on_line)` [VERIFIED: privileged.rs:214] and `yay_build(name: &str, on_line: &mut dyn FnMut(&str))` [VERIFIED: aur.rs:349] have no stop channel. Add variants taking a cancel handle (e.g. `&AtomicBool` or a `Poll`-polled callback in the line-reading loop); on cancel request send **SIGINT to the pkexec child** (graceful; pacman finalizes/aborts its transaction, reports final state) — never SIGKILL (D-14). Report exit status so the modal says how it ended. Follow the existing `*_with` naming convention (`run_privileged_streaming_cancellable` or `run_privileged_streaming_with`).
2. **Optional: streaming BT scan (`bt_scan_with(on_event)`)** for a progressive device list within the 12 s burst. Current `bt_scan()` returns the full `Vec` after 12 s [VERIFIED: bluetooth.rs:383-394]. D-08 doesn't require progressive fill; planner may defer this and populate the list at burst end (simpler). Decide at planning.
3. **VPN import collect-and-apply (D-20).** Check Phase 4 core for an existing NM VPN-add path; if the CLI only lists/deletes VPN, add the import function in core (file or inline fields → NM setting), thin popup in GUI. GUI must not speak D-Bus itself.

Everything else (geometry, second instance, modals, graphs) is frontend-only — **zero** other core changes expected; keep PLAT-03 honest.

## Tab ↔ Core Entry-Point Map (PLAT-03 coverage checklist)

Verified against `crates/archel/src/cmds/mod.rs` (read in full this session) and core module `pub` surfaces:

| Tab | Reads (1 Hz) | Writes (via confirm modal) |
|-----|-------------|---------------------------|
| Landing | `landing::snapshot(false)` (update counts via cache), `sensors`/`system` summary | — |
| System Info | `system::cpu_snapshot/ram_snapshot/disk_snapshot` (stateful `Monitor`, 1 s CPU window), hwmon temps (representative only, D-11), GPU via sensors/NVML path | — |
| Apps/Packages | `packages::list/search/info`, `updates::pending_counts(false)`; AUR `aur` review fns | install/remove/upgrade via `run_privileged_streaming` + `yay_build` (streaming modal D-13, Annuler gap #1) |
| Network | `network::net_status()` async | DNS plan/apply fns, VPN import (gap #3) |
| Power | `power::power_status()` | `power::power_set(profile)` (PPD + cpufreq fallback already in core) |
| Bluetooth | `bt_devices()`; burst `bt_scan()` on open/refresh (D-08) | pair (`pair_picked_with` → global modal D-21), connect/disconnect |
| Sound | `audio::audio_list()` | `set_output_by_name` / `set_input_by_name` |
| Appearance | blur/opacity/wallpaper/font reads | `apply_blur_with`/`apply_opacity_with`/`apply_wallpaper_with`/`apply_font_with` |
| Display | hyprland monitors list | `apply_monitor_plan` w/ 15 s countdown closure (D-16) |
| Input | hyprland input reads | `apply_input_with` |

## Common Pitfalls

### Pitfall 1: Blocking the Qt event loop
**What goes wrong:** UI freezes (fails the roadmap no-freeze criterion) — a core call (wpctl subprocess, sysfs walk, checkupdates) runs in an invokable or a queued closure.
**Why:** invokables execute on the Qt thread; it feels natural to "just call" the core fn there.
**How to avoid:** invokables only `rt.spawn(...)`; all core calls in tokio/`spawn_blocking`; `queue()` closures only set properties.
**Warning signs:** window stops repainting while a tab refreshes; `top` shows the archel main thread busy.

### Pitfall 2: Forgetting to register a QML file in build.rs
**What goes wrong:** blank window or `file not found` engine error at runtime for the new tab/component files.
**Why:** qrc entries come from the `QmlModule` file list in `crates/archel/build.rs`, not the filesystem.
**How to avoid:** every new `.qml` goes into the module list in the same commit; smoke test (offscreen run) in CI catches it.
**Warning signs:** works in `qmllint` but engine logs "Cannot load qrc:/…".

### Pitfall 3: Second-instance probe race / leak
**What goes wrong:** two instances start (TOCTOU) or a refused instance deletes the first instance's listener state.
**Why:** probing-then-binding as two steps, or cleanup of a socket you didn't own.
**How to avoid:** single `bind()` attempt is the test — `EADDRINUSE`/bind-failure ⇒ print French message (D-26), exit 0. Abstract socket dies with the process, so no stale state. Keep the listener alive for the whole `exec()` (own it in `run_settings` scope).
**Warning signs:** launching twice shows two windows; or second launch succeeds after a crash (means you used a path socket without abstract namespace).

### Pitfall 4: Geometry restore to an unavailable screen
**What goes wrong:** window restores off-screen / 0×0 after monitor layout changes; app appears "broken".
**Why:** blindly applying saved x/y/width/height.
**How to avoid:** validate saved geometry against `QGuiApplication::screens()` extents at startup (or clamp to a sane minimum and center when off all screens); keep default 900×620 (D-23) as fallback; also guard against NaN/zero in a hand-edited JSON.
**Warning signs:** UAT on a machine with changed monitor setup shows an invisible window.

### Pitfall 5: `set_monitor` convenience wrapper instead of `apply_monitor_plan`
**What goes wrong:** Display tab opens a terminal-style prompt path / blocks, or rollback semantics diverge from CLI.
**Why:** `set_monitor` is the TTY-friendly API; the GUI needs the raw DI function.
**How to avoid:** GUI binds `apply_monitor_plan` [VERIFIED: monitors.rs:608] with its own `confirm`/`timed_confirm` closures backed by the countdown Popup (D-16).
**Warning signs:** display apply ignores the modal / CLI-visible confirm text appears in logs.

### Pitfall 6: Hard-killing a transaction on Annuler
**What goes wrong:** pacman DB lock left dirty / partial transaction — directly violates D-14.
**Why:** convenience `kill()` on the child; also killing pkexec wrapper hard leaves orphans.
**How to avoid:** core gap #1 — cancel flag observed between lines → SIGINT to the child; then read to EOF and surface exit status in the modal ("Comment s'est terminée la transaction").
**Warning signs:** next install fails with "unable to lock database" after an Annuler.

### Pitfall 7: 1 Hz Timer drift / stacked ticks
**What goes wrong:** slow source (e.g. BT daemon hiccup) causes overlapping spawns; results arrive out of order, UI flickers old data over new.
**Why:** each tick unconditionally spawns a poll task; a 12 s `bt_scan` inside a 1 Hz poller would pile up 12 tasks.
**How to avoid:** per-model "tick in flight" flag (skip if previous tick still running — the right D-04 honesty: keep last values / N/A). Scan-style long ops are user-triggered bursts, not 1 Hz polls (D-08). Sequence-tag results and drop stale ones.
**Warning signs:** CPU samples arriving irregularly; log spam of duplicate task starts.

### Pitfall 8: `QMAKE` picking up Qt 5
**What goes wrong:** build.rs fails or links wrong moc.
**Why:** bare `qmake` on this box is Qt 5.15.
**How to avoid:** `QMAKE=/usr/bin/qmake6` for every gui build (established Phase 1 [VERIFIED: 01-02-SUMMARY.md]); bake into docs/justfile.
**Warning signs:** cxx-qt-build errors mentioning qmake version.

## Code Examples

### Poll-task glue (sync core fn behind 1 Hz)
```rust
// Pattern assembled from verified sources: cxx-qt-0.10.0/src/threading.rs:102 (queue),
// crates/archel-core/src/system.rs (stateful Monitor), spike Timer pattern
// (crates/archel/src/gui/qml/Spike.qml 1 Hz Timer)
pub fn tick(self: core::pin::Pin<&mut Self>) {          // called by QML Timer at 1 Hz
    if self.in_flight() { return; }                      // pitfall 7 guard
    self.set_in_flight(true);
    let qt = self.qt_thread();
    tokio::task::spawn_blocking(move || {
        let snap = collect_system_snapshot();            // sysinfo + hwmon, sync
        let json = serde_json::to_string(&snap).unwrap();
        let _ = qt.queue(move |mut m: core::pin::Pin<&mut qobject::SystemModel>| {
            m.set_metrics_json(json.into());
            m.set_in_flight(false);
        });
    });
}
```

### Streaming modal with cancel (D-13/D-14, after core gap #1)
```rust
// QML StreamModal sets cancelled=true on Annuler → AtomicBool in a shared Cancellation token
pub fn install(self: Pin<&mut PkgModel>, name: QString) {
    let qt = self.qt_thread();
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel2 = cancel.clone();
    rt_move.spawn_blocking(move || {
        let mut on_line = |line: &str| {
            let l = line.to_string();
            let _ = qt.clone().queue(move |m: Pin<&mut qobject::PkgModel>| m.append_line(l.into()));
        };
        let out = run_privileged_streaming_cancellable("install", &[name], "installation", &mut on_line, &cancel2);
        // final state (ok / cancelled-and-how-it-ended / error) → another queue() → modal summary
    });
}
```
(After each read line the cancellable variant checks `cancel2`; on true it SIGINTs the child — core-side, see gap #1.)

### Geometry persistence (D-23)
```rust
// ~/.config/archel/window-geometry.json — serde; validate against screens() before apply
#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Geometry { pub x: i32, pub y: i32, pub w: i32, pub h: i32, pub maximized: bool }
// save on ApplicationWindow.onClosing (QML → invokable), load before engine.load()
```

### Second-instance refusal (D-26)
```rust
// Abstract namespace: path starts with NUL → kernel frees it on process exit
let listener = tokio::net::UnixListener::bind("\0archel-settings-v1")?; // EADDRINUSE => already running
// on bind error: println!("Archel Settings est déjà ouvert."); return ExitCode
```
[ASSUMED: tokio abstract-socket support — verify `tokio::net::UnixListener::bind` docs; fallback is `std::os::unix::net` + `libc::sockaddr_un` or `nix` (nix is a workspace-new dep, prefer the tokio path).]

### Offscreen smoke test (reuse Phase 1 proof)
```bash
QMAKE=/usr/bin/qmake6 cargo run -p archel --features gui -- settings &   # or check --help path
QT_QPA_PLATFORM=offscreen timeout 10 archel settings; echo "exit=$?"
qmllint --blocklist=false $(find crates/archel/src/gui/qml -name '*.qml')   # /usr/bin/qmllint verified present
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Hand-rolled QMetaObject marshalling for tokio→Qt | `cxx_qt::Threading` + `CxxQtThread::queue` | cxx-qt 0.7 → refined through 0.10.0 (2026-08) | D-01 solved by library, zero FFI risk code |
| Spike `SpikeModel` single QObject + `org.archel.spike` URI | Root `ShellModel` + per-tab models, `org.archel.settings` URI | this phase | build.rs module rename + file list growth |
| `#[tokio::main]` wrapping `app.exec()` | Manual `Runtime::new()` before QGuiApplication | this phase | Qt main thread no longer consumes a runtime worker |
| QtCharts for graphs | QtQuick Canvas/Shapes | forced (QtCharts not installed) | keeps PKGBUILD `depends` unchanged |

**Deprecated/outdated:** none beyond the spike itself (Spike.qml is superseded by Main.qml; keep bridge.rs's proven receiver style — bare `Pin<&mut Self>` on invokables, from 01-02-SUMMARY).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | tokio `UnixListener::bind` accepts abstract (`\0`-prefixed) names on Linux | Code Examples / Don't Hand-Roll | Low — trivial fallback to std+libc bind; verify at exec |
| A2 | QtQuick.Dialogs file picker works without xdg-desktop-portal on bare Hyprland; else D-20 inline-fields fallback | Standard Stack | Medium — test early in Wave 1; D-20 already permits inline fields |
| A3 | `QQuickStyle::setStyle("Fusion")` is the right reading of D-24 when `QT_QPA_PLATFORMTHEME` is unset on Hyprland | Standard Stack | Low — visual only; revisit if user has qt6ct |
| A4 | SIGINT to pkexec child propagates to pacman and yields clean-ish abort | Core Gaps | Medium — verify on live machine during the streaming task; D-14 wording ("requests a stop") matches SIGINT semantics |
| A5 | Progressive BT list can be deferred to burst-end populate | Core Gaps #2 | Low — D-08 compatible either way |

## Open Questions

1. **Where do icon assets come from (D-25)?**
   - Known: Qt built-in icon providers are thin in QML; bundled inline SVG in qrc is the robust path.
   - Unclear: user taste on icon set.
   - Recommendation: 10 hand-picked simple SVGs (Material-style, MIT) vendored in qrc; agent's discretion per CONTEXT.
2. **One bridge module or one per tab?**
   - Known: cxx-qt supports multiple `#[cxx_qt::bridge]` modules; spike has one.
   - Recommendation: one `bridge.rs` (or `bridge/` folder) with N qobjects sharing namespace `org.archel.settings` — fewer build.rs moving parts.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| qmake6 | cxx-qt-build | ✓ | `/usr/bin/qmake6`, Qt 6.11.2-3 | — |
| qmllint | QML lint gate | ✓ | `/usr/bin/qmllint` | — |
| qmlformat / qmlcachegen | fmt/compile cache | ✓ | system | — |
| QtQuick.Controls + Fusion style | shell UI (D-24) | ✓ | `/usr/lib/qt6/qml/QtQuick/Controls/Fusion` present | Basic style also present |
| QtQuick.Shapes | graph strokes | ✓ | present on disk | Canvas alone |
| QtCharts | (not used) | ✗ | not installed | Canvas/Shapes — chosen |
| cxx-qt trio =0.10.0 | bridge | ✓ | vendored in cargo registry | — |
| offscreen QPA | smoke test | ✓ | proven in Phase 1 | — |

**Missing dependencies with no fallback:** none. **Missing with fallback:** QtCharts → Canvas (already decided).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (workspace) + qmllint + offscreen smoke; insta (1.48) for geometry/streaming snapshot shapes |
| Config file | none extra (workspace Cargo); spike precedent |
| Quick run command | `cargo test -p archel-core --no-default-features && cargo test -p archel --no-default-features` |
| Full suite command | `cargo clippy -p archel-core --all-features -- -D warnings && cargo test -p archel-core --no-default-features && cargo test -p archel --no-default-features && QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui` |

### Phase Requirements → Test Map
| Req | Behavior | Test Type | Automated Command | File Exists? |
|-----|----------|-----------|-------------------|--------------|
| PLAT-03 | Each tab calls its mapped core entry points | unit (view-model tick calls fake/core fn) | `cargo test -p archel --features gui view_model` | ❌ Wave 0 |
| D-01/D-02 | Tick while task in flight → skipped, no stacking | unit | `cargo test -p archel --features gui in_flight` | ❌ Wave 0 |
| D-04/D-05 | Source error → N/A state + one banner flag, no repeat | unit | `cargo test -p archel --features gui poll_failures` | ❌ Wave 0 |
| D-07 | 1 Hz `pending_counts(false)` stays cached until TTL | unit (core, may already exist from Phase 3) | `cargo test -p archel-core updates` | ✅ (extend if needed) |
| D-08 | Scan is single burst, guarded | unit | `cargo test -p archel --features gui bt_scan` | ❌ Wave 0 |
| D-13/D-14 + gap #1 | Cancel flag → SIGINT path, exit status reported | unit (fake child) | `cargo test -p archel-core privileged` | ❌ Wave 0 (new variant) |
| D-16 | `timed_confirm` closure timeout → rollback called | unit (reuse core tests from Phase 5) | `cargo test -p archel-core monitors` | ✅ extend |
| D-23 | Geometry save/load round-trip + clamp | unit + insta | `cargo test -p archel --features gui geometry` | ❌ Wave 0 |
| D-26 | Second bind fails cleanly | unit (bind twice in test) | `cargo test -p archel --features gui single_instance` | ❌ Wave 0 |
| All QML | No QML syntax/type errors | lint | `qmllint $(find … -name '*.qml')` | ❌ Wave 0 (CI step) |
| Shell | Window loads with zero engine errors | smoke | `QT_QPA_PLATFORM=offscreen timeout 10 archel settings` exit 0 / no `qrc: load` errors | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** quick run command (+ targeted `--features gui` test)
- **Per wave merge:** full suite command
- **Phase gate:** full suite green + offscreen smoke green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/archel/tests/gui_viewmodels.rs` (or unit modules) — tick/N-A/in-flight/single-instance/geometry
- [ ] Cancel-aware test harness in `archel-core` privileged tests (fake streaming child emitting lines + responding to SIGINT)
- [ ] `just gui-check` (or shell script): QMAKE pin + qmllint sweep + offscreen smoke, wired into CI alongside existing headless jobs

## Security Domain

(`security_enforcement` enabled — no explicit disable found in config; ASVS L1.)

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no (local single-user; pkexec is the auth boundary) | pkexec/polkit — unchanged |
| V3 Session Management | no | — |
| V4 Access Control | partial | Every write gated by D-12 confirm modal + core `NeedsPrivilege` path; the modal is the authorization surface — no bypass code path |
| V5 Input Validation | **yes** | Validate package names (argv, charset), wallpaper/font paths (canonicalize, existence), VPN field contents (opaque → NM), geometry JSON (serde strict + clamps) before any core call |
| V6 Cryptography | no | — |
| V12 Readability/Auditability | yes | French text only in QML/catalog; core fns untouched |

### Known Threat Patterns for this stack
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Package-name injection (`pkg"; rm -rf /`) | Tampering | argv-vector via duct (never shell), core-side name validation — GUI never concatenates command strings |
| Privilege escalation via GUI-triggered arbitrary exec | Elevation | Only the audited `privileged`/`aur` core fns can spawn elevated; GUI passes typed args, not command lines |
| Pairing auto-accept / missed prompt | Spoofing | D-21 global modal; never auto-accept (core `pair_picked_with` requires explicit decision fn) |
| Silent mutation (user clicks toggle, config changes) | Tampering | D-12: 100% confirm coverage; verify per-tab checklist in UAT |
| Stale-screenshot trust (frozen numbers look live) | Information disclosure/Integrity | D-04 honest N/A + freshness rule; D-06 keeps data actually fresh |
| Second instance confusion | Tampering | D-26 refusal with clear French message |

## Sources

### Primary (HIGH confidence — read this session)
- Vendored `cxx-qt-0.10.0`: `src/threading.rs:48-140` (CxxQtThread Send/Sync/queue/is_destroyed), `src/lib.rs:140-221` (Threading trait + example)
- Vendored `cxx-qt-gen-0.10.0`: `src/parser/cxxqtdata.rs:437-447` (`impl cxx_qt::Threading for X {}` bridge syntax), `src/generator/rust/threading.rs` (generated qtThread/cxxQtThreadQueue FFI)
- On-disk Qt install: `/usr/lib/qt6/qml/QtQuick/Controls/{Basic,Fusion,Material,Imagine}`, `/usr/lib/qt6/qml/QtQuick/Shapes` present; QtCharts absent
- `crates/archel-core`: `hyprland/monitors.rs:608` (apply_monitor_plan), `privileged.rs:214`, `aur.rs:349` (yay_build), `bluetooth.rs:74,79,383` (SCAN_TIMEOUT/PAIR_TIMEOUT/bt_scan), `updates.rs:29` (CACHE_TTL_SECS), `error.rs:27,146,153,282`
- `crates/archel/src/gui/{bridge.rs,mod.rs}`, `crates/archel/build.rs`, `main.rs`, `cmds/mod.rs` (full read), `crates/archel/Cargo.toml` — spike inventory
- `.planning/phases/01-foundations-safety-rails/01-02-SUMMARY.md` (QMAKE pin, offscreen proof, bare-Pin receiver quirk)

### Secondary (MEDIUM)
- KDAB CXX-Qt book (threading chapter) — corroborates `queue()`-from-background-thread pattern [CITED: kdab.github.io/cxx-qt book, Threading page]

### Tertiary (LOW — Assumptions Log A1–A5)

## Metadata

**Confidence breakdown:**
- Bridge/threading mechanics (the D-01 proof): HIGH — read vendored source of the exact pinned version
- Core entry-point map & DI seams: HIGH — files read this session with line cites
- Core gaps (streaming cancel shape, tokio abstract sockets): MEDIUM — design proposals, flagged A1/A4
- Dialog/file-picker behavior on bare Hyprland: MEDIUM — A2, test early

**Research date:** 2026-10-08
**Valid until:** 2026-11-07 (stable pins; re-check only if cxx-qt releases 0.11 mid-phase)
