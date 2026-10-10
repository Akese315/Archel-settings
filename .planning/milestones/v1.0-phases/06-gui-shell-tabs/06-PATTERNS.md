# Phase 6: GUI Shell & Tabs - Pattern Map

**Mapped:** 2026-10-09
**Files analyzed:** 28 new/modified (7 Rust bin-side, 3 core, 11 QML, 2 test suites, 5 counted per-name groups collapsed)
**Analogs found:** 20 / 28 (8 QML component files have no codebase analog — Spike.qml gives general QML conventions only; use UI-SPEC + RESEARCH patterns)

All analog paths below are git-TRACKED (verified via `git ls-files`).

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/archel/src/gui/mod.rs` (modify) | config/bootstrap | lifecycle + request-response | itself (spike `run_settings()`) + RESEARCH Pattern 2 | exact (self-extend) |
| `crates/archel/src/gui/bridge.rs` (modify) | model | event-driven (1 Hz poll, queued updates) | itself (spike `SpikeModel`) | exact for bridge mechanics |
| `crates/archel/src/gui/poll.rs` (new) | utility | streaming/polling | none in codebase → RESEARCH Pattern 1 + spike Timer | partial |
| `crates/archel/src/gui/geometry.rs` (new) | utility | file-I/O | `crates/archel-core/src/updates.rs:106-140` (cache read/atomic write) | role-match |
| `crates/archel/src/gui/single_instance.rs` (new) | utility | request-response (bind probe) | none (abstract UDS bind is new) | none |
| `crates/archel/src/main.rs` (modify) | controller/entry | request-response | itself + `crates/archel/src/cmds/mod.rs:580-597` | exact |
| `crates/archel/build.rs` (modify) | config | n/a | itself (QmlModule registration) | exact |
| `crates/archel/src/gui/qml/Main.qml` (new) | component (shell) | request-response | `crates/archel/src/gui/qml/Spike.qml` | exact |
| `crates/archel/src/gui/qml/tabs/{Landing,System,Pkg,Net,Power,Bt,Audio,Appearance,Display,Input}Tab.qml` ×10 (new) | component | request-response + 1 Hz polling | Spike.qml tab pages (lines 71-90) + per-tab `crates/archel/src/cmds/*_cmds.rs` | role-match |
| `qml/components/{ConfirmModal,ErrorModal,StreamModal,CountdownModal,PairingModal}.qml` ×5 (new) | component | event-driven (modal-as-callback) | none (no Dialog exists) → UI-SPEC copy contract + RESEARCH Pattern 3 | none |
| `qml/components/Sparkline.qml` (new) | component | streaming (60-sample VecDeque) | none (Canvas not yet used) → UI-SPEC Sparkline contract | none |
| `qml/components/{StatField,PollBanner}.qml` ×2 (new) | component | request-response (property-bound) | Spike.qml `Label { text: spike.tick_label }` binding (line 82-84) | partial |
| `crates/archel-core/src/privileged.rs` (modify) | service | streaming | itself: `run_privileged_streaming` (214-256) + `collect_streaming` (196-207) | exact (new `_cancellable` variant) |
| `crates/archel-core/src/aur.rs` (modify) | service | streaming | itself: `yay_build` (~349) | exact |
| `crates/archel-core/src/network.rs` (modify, VPN import — gap #3) | service | CRUD | itself (Phase 4 NM D-Bus paths) | role-match |
| `crates/archel/tests/gui_viewmodels.rs` (new) | test | unit | `crates/archel/src/cmds/mod.rs:768` (`mod tests`) + spike test shape | role-match |
| `crates/archel-core` privileged cancel tests (extend) | test | unit (fake child) | existing `privileged.rs` tests (`collect_streaming` is already "generic over the reader so tests feed a cursor", line 194-195) | exact |

## Pattern Assignments

### `crates/archel/src/gui/bridge.rs` — ShellModel + 10 per-tab view-models (model, event-driven)

**Analog:** itself (spike `SpikeModel`, lines 10-58) — the proven receiver/property style; RESEARCH "Deprecated" note: keep bare `Pin<&mut Self>` on invokables (01-02-SUMMARY quirk).

**Bridge declaration pattern** (bridge.rs lines 10-29) — replicate per model, same namespace `org.archel.settings`:
```rust
#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, tick_label)]
        #[qproperty(i32, tick_count)]
        #[namespace = "archel_spike"]           // → rename to "archel_settings"
        type SpikeModel = super::SpikeModelRust;

        #[qinvokable]
        #[cxx_name = "tick"]
        fn tick(self: Pin<&mut Self>);
    }
    // NEW per model: threading hook (RESEARCH Pattern 1, D-01 proof)
    impl cxx_qt::Threading for NetworkModel {}  // etc.
}
```

**Backing-struct + impl pattern** (bridge.rs lines 33-58): `pub struct XModelRust { fields }` + `impl Default` + `impl qobject::XModel { pub fn tick(mut self: core::pin::Pin<&mut Self>) {...} }`. Property access via `self.as_mut().set_*()` / `*self.as_mut().field()` accessor style.

**Naming rule** (bridge.rs lines 6-8 doc): all user-visible wording produced in `.rs` is plain ASCII — French lives in QML only (Phase 1 D-07).

**Payload marshalling** (RESEARCH Summary + Standard Stack): scalars → typed Q_PROPERTYs; lists/records → `serde_json::to_string(&core_snapshot)` into a `QString` property, `JSON.parse` in QML. Core `--json` structs are the ready-made payloads (see cmds excerpts below).

### `crates/archel/src/gui/poll.rs` + per-tab tick bodies (utility, 1 Hz polling)

**Analog:** none in codebase — copy from RESEARCH Pattern 1 / "Poll-task glue" (verified against vendored `cxx-qt-0.10.0/src/threading.rs:102-109`):

```rust
pub fn tick(self: core::pin::Pin<&mut Self>) {          // called by QML Timer at 1 Hz
    if self.in_flight() { return; }                      // Pitfall 7 guard
    self.set_in_flight(true);
    let qt = self.qt_thread();                           // Send + Sync, cloneable
    tokio::task::spawn_blocking(move || {
        let snap = collect_system_snapshot();            // sysinfo + hwmon, sync — OFF Qt thread
        let json = serde_json::to_string(&snap).unwrap();
        let _ = qt.queue(move |mut m: core::pin::Pin<&mut qobject::SystemModel>| {
            m.set_metrics_json(json.into());             // Qt thread: property-sets ONLY
            m.set_in_flight(false);
        });
    });
}
```

Async core fns (network/power/bt) use the same shape with `rt.spawn(async move { ... })` instead of `spawn_blocking`. **Rules:** invokables only spawn (never call core); `queue()` closures only set properties; sync core fns always via `spawn_blocking`; sequence-tag results; per-model in-flight flag.

**Which core fn each tab's tick calls** — copy 1:1 from `crates/archel/src/cmds/*_cmds.rs` (see Tab map below). Example, Power tab, from `crates/archel/src/cmds/power_cmds.rs:18-26`:
```rust
pub async fn run_status(json: bool) -> Result<(), ArchelError> {
    let status = archel_core::power::power_status().await?;   // ← PowerModel's poll calls this same fn
    ...
}
```
and the write side, `power_cmds.rs:31-43` (`power::parse_profile_arg` → `power::power_set(profile).await`) → the view-model's `set_profile` invokable spawns the same pair behind the D-12 confirm modal. Repeat this read for each of `landing_cmd.rs, sys_cmds.rs, pkg_cmds.rs, net_cmds.rs, bt_cmds.rs, audio_cmds.rs, power_cmds.rs, appearance_cmds.rs, display_cmds.rs, input_cmds.rs` when writing the matching model.

### `crates/archel/src/gui/mod.rs` — `run_settings()` rework (bootstrap, lifecycle)

**Analog:** itself (lines 14-26) for the QGuiApplication/engine skeleton; RESEARCH Pattern 2 for the runtime restructure:

```rust
pub fn run_settings() -> Result<(), ArchelError> {
    // NEW step 0: second-instance probe (single_instance::acquire()? → French message, exit 0)
    // NEW step 1: static RUNTIME: OnceLock<tokio::runtime::Runtime>; Runtime::new() BEFORE Qt
    let mut app = QGuiApplication::new();                  // existing (line 15)
    // NEW: QQuickStyle::set_style("Fusion") unless QT_QPA_PLATFORMTHEME set (D-24)
    let mut engine = QQmlApplicationEngine::new();         // existing (line 16)
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from(
            // CHANGED URI: qrc:/qt/qml/org/archel/settings/src/gui/qml/Main.qml
            "qrc:/qt/qml/org/archel/spike/src/gui/qml/Spike.qml",
        ));
    }
    if let Some(app) = app.as_mut() { app.exec(); }        // existing (lines 22-24)
    Ok(())
}
```
Keep the `Option`-guard style (`engine.as_mut()` / `app.as_mut()`). Own the abstract-socket listener in this scope for the whole `exec()` (RESEARCH Pitfall 3). Geometry: load before `engine.load()`, expose save via a window-closing invokable.

### `crates/archel/src/main.rs` (modify, entry controller)

**Analog:** itself (lines 14-23) + `crates/archel/src/cmds/mod.rs:580-597` (`Command::Settings` dispatch at line 592). RESEARCH Pattern 2: branch on `Command::Settings` **before** entering any `#[tokio::main]` so `app.exec()` never occupies a runtime worker. Keep the existing error voice (main.rs line 19): `eprintln!("{}", err.message())` — the GUI's error modals use the same `ArchelError::message()` renderer (`crates/archel-core/src/error.rs:282`).

### `crates/archel/build.rs` (modify, config)

**Analog:** itself (lines 9-18). Change `QmlModule::new("org.archel.spike")` → `"org.archel.settings"` and extend `.qml_file(...)` with **every** new file (`qml/Main.qml`, `qml/tabs/*.qml`, `qml/components/*.qml`) — unregistered QML fails at runtime, not compile time (RESEARCH Pitfall 2); register each `.qml` in the same commit that adds it. Keep `#[cfg(not(feature = "gui"))] fn main() {}` no-op and the `QMAKE=/usr/bin/qmake6` env pin (Pitfall 8).

### `crates/archel/src/gui/geometry.rs` (new, utility, file-I/O)

**Analog:** `crates/archel-core/src/updates.rs:106-140` (the project's established XDG-file pattern):
```rust
// updates.rs:106-111 — path via directories, graceful fallback
pub fn cache_file_path() -> PathBuf {
    directories::ProjectDirs::from("", "archel", "archel")
        .map(|dirs| dirs.cache_dir().join(CACHE_FILE_NAME))
        .unwrap_or_else(|| PathBuf::from(CACHE_FILE_NAME))
}
// updates.rs:116-123 — read: any failure (missing/unparseable) → None, caller falls back (T-2-01)
pub(crate) fn read_cache(path: &Path) -> Option<UpdateCache> { ... serde_json::from_str(&raw).ok()? ... }
// updates.rs:128-140 — write: atomic temp-file + rename, failures swallowed
```
Adapt: `config_dir().join("window-geometry.json")` (not cache_dir); `Geometry { x, y, w, h, maximized }` serde struct (RESEARCH Code Examples); validate against `QGuiApplication::screens()` before apply, 900×620 fallback (D-23, Pitfall 4).

### `crates/archel/src/gui/single_instance.rs` (new, utility)

**No analog.** Use RESEARCH Code Example verbatim (A1: verify tokio abstract-socket support at exec; fallback std+libc bind):
```rust
let listener = tokio::net::UnixListener::bind("\0archel-settings-v1")?; // EADDRINUSE => already running
// on bind error: println!("Archel Réglages est déjà ouvert."); exit 0   (D-26; copy exact string in UI-SPEC)
```
Single `bind()` attempt is the test (no probe-then-bind TOCTOU, Pitfall 3). Keep listener alive in `run_settings` scope.

### `crates/archel/src/gui/qml/Main.qml` (new, shell component)

**Analog:** `crates/archel/src/gui/qml/Spike.qml` (whole file, 99 lines) — the proven vertical-tab skeleton. Copy:
- Import block + module URI (lines 8-11, `import org.archel.settings`),
- Rail = Column(Layout) of TabButtons driving StackLayout `currentIndex` (lines 29-68) — TabBar is horizontal-only; extend to 10 buttons with icons (16×16, `spacing: 4`, item height 32, UI-SPEC rail contract),
- `StackLayout { id: stack; currentIndex: 0 }` content switcher (lines 71-90),
- 1 Hz Timer → `model.tick()` (lines 93-98; per-tab timers inside the models' scope, D-03: timers run even when hidden).
Add: `ApplicationWindow.onClosing` → geometry-save invokable; global `PairingModal` + `ErrorModal` instantiated at window level (UI-SPEC "Global modals").
Fold ad-hoc 12px margins → 16px `md` token (UI-SPEC spacing).

### `qml/tabs/*.qml` ×10 (new, view components)

**Analog:** Spike.qml tab pages (lines 77-89: page = ColumnLayout + bold Heading Label + property-bound content) + UI-SPEC per-tab contracts. Structure each tab as: Heading (1.25× bold) + `PollBanner` + `StatField` grid / ListView, all bound to its `<Tab>Model` JSON/scalar properties. Pre-first-tick `…`, post-failure `N/A` (UI-SPEC loading rows). Component vocabulary: UI-SPEC Component Inventory table (all Fusion, no new modules).

### `qml/components/` modals ×5 + Sparkline + StatField + PollBanner (new)

**No codebase analog** (no Dialog/Popup/Canvas exists yet). Build from:
- **UI-SPEC** Copywriting Contract rows (locked French labels per modal: `Conserver`/`Annuler`, `Confirmer`/`Refuser`, `Réessayer`, error title `Une erreur est survenue`),
- **RESEARCH Pattern 3** (modal-as-confirm via core DI seams — the modal buttons resolve a oneshot the tokio task awaits; GUI never re-implements confirm/rollback):
  seams verified: `apply_monitor_plan` (`crates/archel-core/src/hyprland/monitors.rs:608`), `apply_blur_with`/`apply_opacity_with`/`apply_input_with`/`apply_wallpaper_with`/`apply_font_with`, `pair_picked_with` (`crates/archel-core/src/bluetooth.rs`), `set_output_picked_with`.
- **ErrorModal** renders `ArchelError::message()` (+ cause) only — never invents strings (`crates/archel-core/src/error.rs:282`; variants `AuthFailed` :27, `ConfirmationAborted` :146, `ConfirmTimeout` :153, `NeedsPrivilege`).
- **StatField/PollBanner**: property-bound Label pattern from Spike.qml:82-84; N/A at 40% opacity, banner text `Données indisponibles pour cette section.` (D-05).

### `crates/archel-core/src/privileged.rs` (modify, service, streaming — core gap #1)

**Analog:** itself — `run_privileged_streaming` (lines 214-256) is the exact shape for the new `run_privileged_streaming_cancellable` (follow existing `*_with` convention):
```rust
// privileged.rs:196-207 — line-drain loop; the cancel check goes INSIDE this loop per line
fn collect_streaming<R: std::io::BufRead>(reader: R, on_line: &mut dyn FnMut(&str)) -> String {
    let mut transcript = String::new();
    for line in reader.lines().map_while(Result::ok) {
        on_line(&line);                       // GUI variant: also check cancel flag → SIGINT child once
        transcript.push_str(&line); transcript.push('\n');
    }
    transcript
}
```
Copy from `run_privileged_streaming` (214-256): `which::which("pkexec")` probe, argv-vector `duct::cmd(...).unchecked().env("LC_ALL","C").stderr_capture().reader()` (never `run()` — Pitfall 7 pipe deadlock), `handle.try_wait()` after EOF, `map_pkexec_exit(action_label, code)` so the modal can report **how** it ended (D-14). Add `&AtomicBool` cancel param; on set → SIGINT to pkexec child (A4), read to EOF, surface exit status. Never SIGKILL. Mirror the same cancel handle for `yay_build` (`crates/archel-core/src/aur.rs:349`). Test hook already exists: `collect_streaming` is generic over the reader "so tests feed a cursor" (line 194-195) — extend for the cancel test.

### `crates/archel-core/src/network.rs` (modify, VPN import — core gap #3)

**Analog:** itself (Phase 4 NM D-Bus paths — file-first: check for an existing VPN-add fn before writing one; if the CLI only lists/deletes, add import there — **GUI must not speak D-Bus itself**). Field collection lives in the QML popup (D-20).

### Tests

**`crates/archel/tests/gui_viewmodels.rs`** — shape from `cmds/mod.rs:768` (`#[cfg(test)] mod tests`) convention; cover RESEARCH Wave 0 gaps: in-flight skip (D-01/D-02), N-A/banner-once (D-04/D-05), BT burst guard (D-08), geometry round-trip + clamp (D-23, insta), second-bind refusal (D-26). Core cancel test extends existing `cargo test -p archel-core privileged`.

## Shared Patterns

### Cross-thread property update (THE D-01 proof — applies to every view-model)
**Source:** RESEARCH Pattern 1 [VERIFIED: vendored cxx-qt-0.10.0 `src/threading.rs:48-140`, `src/lib.rs:140-221`]
**Apply to:** all 11 QObjects + streaming modal glue. `impl cxx_qt::Threading for X {}` in bridge → `self.qt_thread()` → `qt_thread.queue(closure)` from tokio; QObjects are `!Send !Sync` — `queue()` is the only legal mutation path; guard destroyed-object cases with `is_destroyed()`.

### Tab ↔ core entry-point 1:1 (PLAT-03 coverage)
**Source:** `crates/archel/src/cmds/mod.rs:605-714` dispatch + per-domain `*_cmds.rs` files
**Apply to:** each view-model calls exactly its command's core fns (RESEARCH "Tab ↔ Core Entry-Point Map"): Landing `landing::snapshot(false)`; System `system::cpu_snapshot/ram_snapshot/disk_snapshot` + hwmon (stateful `Monitor`, one per task); Pkg `packages::list/search/info` + `updates::pending_counts(false)` + `run_privileged_streaming(_cancellable)`/`yay_build`; Net `network::net_status()`; Power `power::power_status/power_set`; Bt `bt_devices`/`bt_scan` (12 s burst only) /`pair_picked_with`; Audio `audio::audio_list`/`set_output_by_name`/`set_input_by_name`; Appearance `apply_*_with` fns; Display `apply_monitor_plan` (never `set_monitor` — Pitfall 5); Input `apply_input_with`.

### Error voice (one French voice, D-17)
**Source:** `crates/archel-core/src/error.rs:282` (`ArchelError::message()`) + `main.rs:19` usage
**Apply to:** ErrorModal, poll banners, second-instance message. GUI never authors error strings.

### French-only text split (Phase 1 D-06/D-07)
**Source:** bridge.rs:6-8 doc, main.rs:4-6 doc, Spike.qml:6-7
**Apply to:** all `.rs` plain ASCII; every user string French in `.qml`/catalog (UI-SPEC Copywriting Contract is the label source of truth).

### QmlModule registration + build pin
**Source:** `crates/archel/build.rs:9-18` + 01-02-SUMMARY
**Apply to:** every new `.qml` in the same commit as its build.rs line; `QMAKE=/usr/bin/qmake6 cargo build -p archel --features gui`; `qmllint` sweep + `QT_QPA_PLATFORM=offscreen timeout 10 archel settings` smoke (RESEARCH Code Examples).

### Atomic XDG JSON file read/write
**Source:** `crates/archel-core/src/updates.rs:106-140`
**Apply to:** `gui/geometry.rs` (config_dir variant; read → `Option` + fallback default, write → temp+rename, failures swallowed).

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `qml/components/ConfirmModal.qml` | component | event-driven | No Dialog/Popup anywhere in codebase — use UI-SPEC copy rows + RESEARCH Pattern 3 oneshot seam |
| `qml/components/ErrorModal.qml` | component | event-driven | ditto; bind `ArchelError::message()` |
| `qml/components/StreamModal.qml` | component | streaming | ditto; feed `append_line` invokable from `queue()` closures (RESEARCH streaming example) |
| `qml/components/CountdownModal.qml` | component | event-driven | ditto; drives `timed_confirm` closure of `apply_monitor_plan` |
| `qml/components/PairingModal.qml` | component | event-driven | ditto; window-level Popup backed by `pair_picked_with` callback |
| `qml/components/Sparkline.qml` | component | streaming | No Canvas/Shapes usage yet — UI-SPEC Sparkline contract (60×f64, 2px, accent/green) |
| `qml/components/PollBanner.qml` | component | request-response | dismissible banner is new UI state (only partial: Spike.qml label binding) |
| `gui/single_instance.rs` | utility | request-response | No abstract-socket/lockfile code exists — RESEARCH Code Example + A1 fallback |
| `gui/poll.rs` | utility | polling | No tokio→Qt glue exists — RESEARCH Pattern 1 verbatim |

## Metadata

**Analog search scope:** `crates/archel/src/{gui,}/**`, `crates/archel/build.rs`, `crates/archel/src/cmds/*`, `crates/archel-core/src/{privileged,aur,updates,error,network,bluetooth,hyprland}` (targeted reads/greps)
**Files scanned:** ~20 (spike trio read in full; build.rs, main.rs, updates.rs §cache, privileged.rs §streaming, power_cmds.rs read; cmds/mod.rs + core seams grepped)
**Pattern extraction date:** 2026-10-09
**Tracked-source check:** all cited analog paths confirmed via `git ls-files` (no gitignored mirrors emitted)
