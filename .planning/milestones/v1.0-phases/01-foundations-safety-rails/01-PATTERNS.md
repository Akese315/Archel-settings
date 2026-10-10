# Phase 1: Foundations & Safety Rails - Pattern Map

**Mapped:** 2026-09-30
**Files analyzed:** 14 new files (greenfield — zero existing source files)
**Analogs found:** 0 / 14 (no codebase analogs exist; all patterns sourced from 01-RESEARCH.md)

## Greenfield Declaration

**Analog search scope:** repo root (`Cargo.toml`, `**/*.rs`, `**/*.qml`, `**/build.rs`, `policy/*`) + `git ls-files` verification.
**Files scanned:** 23 git-tracked files — all are docs/planning (`.planning/**`, `AGENTS.md`). Zero `.rs` files, zero `.toml` files, zero `.qml` files on disk.
**Tracked-source gate (#3645):** no analog paths emitted because no git-tracked source files exist. There are no gitignored mirror candidates either (no `.gsd/`, no `capabilities/` dirs).

**Consequence for planner:** every file below is greenfield. The "Analog" column points at the normative reference pattern from `01-RESEARCH.md` (which itself cites primary sources: KDAB CXX-Qt book, local machine probes, `man pkexec`, crates.io API) instead of a codebase file. Copy the excerpts verbatim as the starting skeleton.

## File Classification

| New File | Role | Data Flow | Closest Analog | Match Quality |
|----------|------|-----------|----------------|---------------|
| `Cargo.toml` (workspace root) | config | build | none — RESEARCH.md "Installation" + Pattern 1 | no-analog (RESEARCH.md pattern) |
| `rust-toolchain.toml` | config | build | none — RESEARCH.md Standard Stack row 1 | no-analog (RESEARCH.md pattern) |
| `crates/archel-core/src/lib.rs` | model (lib entry) | request-response (in-process API) | none — RESEARCH.md Recommended Project Structure | no-analog (RESEARCH.md pattern) |
| `crates/archel-core/src/error.rs` | model (error taxonomy) | transform (variant → FR string) | none — RESEARCH.md Pattern 4 | no-analog (RESEARCH.md pattern) |
| `crates/archel-core/src/i18n.rs` | utility (catalog wiring) | transform (key → FR string) | none — RESEARCH.md Pattern 4 + Don't Hand-Roll row 3 | no-analog (RESEARCH.md pattern) |
| `crates/archel-core/src/privileged.rs` | service (privilege client) | request-response (spawn pkexec → parse JSON) | none — RESEARCH.md Pattern 3 + pkexec client mapping | no-analog (RESEARCH.md pattern) |
| `crates/archel-core/locales/fr.json` | config (i18n catalog) | transform (static lookup) | none — RESEARCH.md Pattern 4 (D-07/D-08) | no-analog (RESEARCH.md pattern) |
| `crates/archel/src/main.rs` | controller (clap root) | request-response (argv → dispatch) | none — RESEARCH.md Don't Hand-Roll row 1 + Pattern 1 | no-analog (RESEARCH.md pattern) |
| `crates/archel/src/cmds/` (settings + stub modules) | controller (subcommands) | request-response | none — RESEARCH.md Architecture Diagram entry points | no-analog (RESEARCH.md pattern) |
| `crates/archel/build.rs` | config (bridge codegen) | build | none — RESEARCH.md Pattern 2 (`cargo_without_cmake`) | no-analog (RESEARCH.md pattern) |
| `crates/archel/src/gui/bridge.rs` | provider (QObject bridge) | event-driven (QML ↔ Rust NOTIFY/invokable) | none — RESEARCH.md Pattern 2 bridge excerpt | no-analog (RESEARCH.md pattern) |
| `crates/archel/src/gui/qml/Spike.qml` | component (view) | event-driven (Timer → tick → NOTIFY re-render) | none — RESEARCH.md Spike QML excerpt (D-14) | no-analog (RESEARCH.md pattern) |
| `crates/archel-privileged/src/main.rs` | service (pkexec helper) | request-response (argv → stdout JSON) | none — RESEARCH.md Pattern 3 allowlist excerpt | no-analog (RESEARCH.md pattern) |
| `policy/org.archel.privileged.policy` | config (polkit policy) | auth (OS-owned) | none — RESEARCH.md Polkit policy skeleton | no-analog (RESEARCH.md pattern) |

## Pattern Assignments

### `Cargo.toml` + `rust-toolchain.toml` (config, build)

**Analog:** none in codebase. **Source:** `01-RESEARCH.md` § Standard Stack + Pattern 1 + Installation.

**Workspace layout pattern** (RESEARCH.md lines 103-107, 209-214):
```toml
# Workspace root: single archel bin + archel-core lib + archel-privileged helper
[workspace]
members = ["crates/archel", "crates/archel-core", "crates/archel-privileged"]
resolver = "2"
```
```toml
# crates/archel/Cargo.toml — feature-gated GUI (D-01 reconciliation)
[features]
default = ["gui"]
gui = ["dep:cxx-qt", "dep:cxx-qt-lib"]
```
```toml
# rust-toolchain.toml
[toolchain]
channel = "stable"
```
```toml
# Edition + version pins (RESEARCH.md Standard Stack, registry-verified 2026-09-30)
# edition = "2024"; cxx-qt/cx-qt-lib/cxx-qt-build = "=0.10.0" (all three same 0.10.x)
# clap = "4.6" (derive feature); thiserror = "2.0"; serde_json = "1.0"
# rust-i18n = "4.2.4"; duct = "1.1"; which = "8.0"
```

**Rules for planner:** `archel-core` and `archel-privileged` must have zero Qt dependency (Pitfall 2). `cxx` version is managed by `cxx-qt-build` — do NOT copy any `cxx` pin from the KDAB book example (State of the Art). `Cargo.lock` is committed (binary ships).

---

### `crates/archel-core/src/error.rs` (model, transform)

**Analog:** none in codebase. **Source:** `01-RESEARCH.md` Pattern 4 (lines 264-266) + Pitfall 6 + pkexec client mapping (lines 371-380).

**Core pattern — English variants, FR rendered via catalog (D-07/D-08):**
```rust
// Pattern 4: variants are English-identified; Display (or message()) looks up
// the FR string via t! keys — French lives ONLY in locales/fr.json.
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArchelError {
    #[error(transparent)] // Display routes through catalog lookup, NOT inline French
    AuthFailed { action: String, reason: AuthDismissal },
    HelperProtocol(String),
    GuiNotCompiled,
    // per-domain variants added by later phases; GUI matches on variants for toasts
}
```

**Auth-failure mapping pattern (D-12, VERIFIED against local `man pkexec`):**
```rust
match output.status.code() {
    Some(0)   => parse_v1_json(&stdout).map_err(ArchelError::HelperProtocol),
    Some(126) => Err(ArchelError::privilege_cancelled(&action)), // dialogue fermé
    Some(127) => Err(ArchelError::privilege_denied(&action)),    // non autorisé / erreur
    other     => Err(ArchelError::HelperProtocol(format!("exit={other:?}"))),
}
// Stderr captured for logs only, NEVER displayed (Pitfall 4).
// Friendly FR strings naming the rejected action, e.g.:
// "Authentification annulée — «action» n'a pas été exécutée."
```

**Error-handling rules:** no panics on failure paths — `main` returns `Result` / prints catalog error + non-zero exit. `t!` inside `fmt::Display` is assumed (A5) with fallback to a `message(&self) -> String` renderer. Tests: every variant renders non-empty FR; diacritics grep (`grep -rP '[\x{00C0}-\x{00FF}\x{0152}\x{0153}]' crates/*/src`) fails CI if French leaks outside `locales/` (Pitfall 6).

---

### `crates/archel-core/src/i18n.rs` + `locales/fr.json` (utility + config, transform)

**Analog:** none in codebase. **Source:** `01-RESEARCH.md` Standard Stack (`rust-i18n 4.2.4`) + Don't Hand-Roll row 3 + D-07.

**Catalog wiring pattern:**
```rust
// rust-i18n 4.2.4 (planner discretion vs fluent/gettext per CONTEXT; rust-i18n
// recommended: actively maintained, tiny surface, single-language v1 needs no plurals).
use rust_i18n::t;
rust_i18n::i18n!("locales"); // catalog root beside src
// Usage: t!("error.auth_cancelled", action = action) — keys, never literals.
```
```json
// locales/fr.json — the ONLY place with French strings (D-07)
{ "error": { "auth_cancelled": "Authentification annulée — « %{action} » n'a pas été exécutée." } }
```

**Rules:** code identifiers English (D-08); FR appears only in displayed strings via keys. v2 adds EN by adding a file, not by hunting literals.

---

### `crates/archel-core/src/privileged.rs` (service, request-response)

**Analog:** none in codebase. **Source:** `01-RESEARCH.md` Pattern 3 (lines 252-262) + Don't Hand-Roll row 5 + Pitfalls 3/4/5.

**Core pattern — one client module owns spawn/parse/map:**
```rust
// One privileged.rs client (+ duct): timeouts, env scrubbing, exit-code mapping
// in one reviewed place; every later phase reuses it. Never ad-hoc Command per call site.
use duct::cmd;
use which::which;

pub fn run_privileged(action: &str, args: &[&str]) -> Result<serde_json::Value, ArchelError> {
    let pkexec = which("pkexec").map_err(|_| ArchelError::HelperProtocol("pkexec introuvable".into()))?;
    let helper = std::env::current_exe_helper_path(); // absolute path only (Pitfall 5)
    let output = cmd(pkexec, ["archel-privileged", action].iter().chain(args))
        .stderr_capture().stdout_capture().run()?;
    // then exit-code match per error.rs pattern; envelope version-checked: v == 1
}
```

**Rules:** envelope carries `v` (protocol evolution); JSON parse failure → `HelperProtocol`, never default-ok. Helper writes nothing under any home; all user-file writes happen unprivileged from stdout data (Pitfall 5).

---

### `crates/archel-privileged/src/main.rs` (service, request-response)

**Analog:** none in codebase. **Source:** `01-RESEARCH.md` Pattern 3 (lines 252-262) + Pitfall 3.

**Core pattern — exact-match argv allowlist, versioned JSON, minimal deps:**
```rust
// Minimal deps: serde_json ONLY. Absolute paths only. No shell. No prefix-matching.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["dry-run"] => println!("{}", serde_json::json!({"v": 1, "action": "dry-run", "ok": true})),
        _ => {
            println!("{}", serde_json::json!({"v":1,"ok":false,"error":"action-not-allowlisted"}));
            std::process::exit(2);
        }
    }
}
```

**Rules:** pkexec does NO argv validation (`man pkexec` VERIFIED) — the helper is the second gate. Real actions (Phase 3+) re-validate package names against `^[a-z0-9@._+-]+$`. Phase 1 dry-run writes nothing, needs nothing root. Tests: allowlist matrix (`dry-run` → v1 JSON exit 0; anything else → exit 2 + error JSON), no root needed.

---

### `crates/archel/src/main.rs` + `src/cmds/` (controller, request-response)

**Analog:** none in codebase. **Source:** `01-RESEARCH.md` Architecture Diagram (lines 147-164) + Don't Hand-Roll row 1 + Pattern 1.

**Core pattern — clap derive root, thin dispatch to core:**
```rust
// clap 4.6 derive: help text, --json flags, shell completions (clap_complete) free.
// Subcommand + flag names English; all human-readable output French (D-04).
#[derive(Parser)]
#[command(name = "archel")]
enum Cli { Settings(SettingsArgs), /* …stubs… */ }

fn main() -> anyhow::Result<()> {
    // `settings` without `gui` feature → typed GuiNotCompiled FR error (Pattern 1).
    // Unprivileged path: direct core return. Privileged path: core::privileged client.
}
```

**Rules:** clap lives in the `archel` bin, NEVER in `archel-core`. `settings` dispatches to `gui/` module behind `#[cfg(feature = "gui")]`. Headless builds/tests use `--no-default-features` and must never link Qt.

---

### `crates/archel/build.rs` + `src/gui/bridge.rs` (config + provider, build / event-driven)

**Analog:** none in codebase. **Source:** `01-RESEARCH.md` Pattern 2 (lines 216-250) + Code Examples app entry (lines 328-340). Origin: KDAB CXX-Qt book `cargo_without_cmake` example.

**Build pattern (no CMake — cmake is NOT installed on this machine):**
```rust
// Source: https://kdab.github.io/cxx-qt/book/getting-started/4-cargo-executable.html
use cxx_qt_build::{CxxQtBuilder, QmlModule};
fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("org.archel.spike").qml_file("qml/Spike.qml"),
    )
    .files(["src/gui/bridge.rs"])
    .build();
}
```

**Bridge pattern (Rust QObject view-model, QML binds to it):**
```rust
// Source: https://kdab.github.io/cxx-qt/book/getting-started/2-our-first-cxx-qt-module.html
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
        #[namespace = "archel_spike"]
        type SpikeModel = super::SpikeModelRust;
        #[qinvokable]
        #[cxx_name = "tick"]
        fn tick(self: Pin<&mut Self>);   // QML Timer calls tick() 1 Hz; NOTIFY re-renders label
    }
}
```

**App entry pattern:**
```rust
use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};
fn main() {
    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/org/archel/spike/qml/Spike.qml"));
    }
    if let Some(app) = app.as_mut() { app.exec(); }
}
```

**Rules:** every Qt build runs with `QMAKE=/usr/bin/qmake6` exported (Pitfall 1 — bare `qmake` is Qt 5.15.19 on this machine). `use cxx_qt…` ONLY inside `#[cfg(feature = "gui")]`. Qt types stop at `gui/` — never in core (Anti-Patterns).

---

### `crates/archel/src/gui/qml/Spike.qml` (component, event-driven)

**Analog:** none in codebase. **Source:** `01-RESEARCH.md` Spike QML excerpt (lines 342-369). Success bar D-14.

**Core pattern — vertical tabs prove the real GUI pattern, live label proves the bridge binds:**
```qml
// Pattern: QtQuick.Controls TabBar is horizontal-only — vertical tabs =
// Column of TabButtons + StackLayout. Live label via QML Timer → Rust invokable.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.archel.spike 1.0
ApplicationWindow {
    visible: true; width: 800; height: 600; title: "Archel — spike"
    SpikeModel { id: spike }
    RowLayout {
        anchors.fill: parent
        ColumnLayout {  // vertical tabs
            TabButton { text: "Réseau"; checkable: true; autoExclusive: true; checked: true
                        onClicked: stack.currentIndex = 0 }
            TabButton { text: "Audio"; checkable: true; autoExclusive: true
                        onClicked: stack.currentIndex = 1 }
        }
        StackLayout {
            id: stack; currentIndex: 0
            Label { text: spike.tick_label }   // live-updating via NOTIFY
            Label { text: "onglet 2 (spike)" }
        }
    }
    Timer { interval: 1000; running: true; repeat: true; onTriggered: spike.tick() }
}
```

**Rules:** a hello-window fails D-14. Spike QML is static (no untrusted strings — Security Domain). Manual-only verification: screenshot/run log.

---

### `policy/org.archel.privileged.policy` (config, auth)

**Analog:** none in codebase. **Source:** `01-RESEARCH.md` Polkit policy skeleton (lines 382-397). Shape VERIFIED against `/usr/share/polkit-1/actions/org.freedesktop.hostname1.policy`.

```xml
<policyconfig>
  <vendor>Archel</vendor>
  <action id="org.archel.privileged.dry-run">
    <description>Archel : test du module privilégié</description>
    <message>L'authentification est requise pour « test du module privilégié ».</message>
    <defaults>
      <allow_any>auth_admin_keep</allow_any>
      <allow_inactive>auth_admin_keep</allow_inactive>
      <allow_active>auth_admin_keep</allow_active>
    </defaults>
  </action>
</policyconfig>
```

**Rules:** `auth_admin_keep` = standard polkit persistence (D-10, assumption A3 — security review confirms). Never `yes`; one action per verb; dedicated security review before packaging (ASVS V4).

---

## Shared Patterns

### Error Handling (applies to: all `archel-core` + both binaries)
**Source:** `01-RESEARCH.md` Pattern 4 + Validation Architecture (no codebase source exists).
- `thiserror` 2.0.21 typed enum in core; `anyhow` in binaries.
- `Display`/`message()` renders FR via `t!` catalog keys; no inline French literals (Pitfall 6 lint).
- No panics on failure paths; `main` returns `Result` + non-zero exit.
- Excerpts: see `error.rs` assignment above.

### Privilege Boundary (applies to: `privileged.rs` + `archel-privileged/main.rs` + policy file)
**Source:** `01-RESEARCH.md` Pattern 3 + Pitfalls 3/4/5 (no codebase source exists).
- Exact-match argv allowlist, no shell, no prefix-matching; exit 2 + machine JSON on reject.
- Exit-code map 0 → parse v1 JSON / 126 → cancelled / 127 → denied, action named in FR (D-12).
- pkexec-only (D-09); yay keeps native sudo, never orchestrated (D-11). Helper: absolute paths, writes nothing.
- Excerpts: see `privileged.rs` / `archel-privileged` / policy assignments above.

### Logging (applies to: all crates)
**Source:** `01-RESEARCH.md` Standard Stack (`tracing` 0.1.44 + `tracing-subscriber`).
- `tracing` spans per core operation; CLI defaults to warn, GUI to file log later. Never `pretty_env_logger` (stale).

### Feature-Gate / Build Discipline (applies to: `archel` bin, `build.rs`, CI)
**Source:** `01-RESEARCH.md` Pattern 1 + Pitfalls 1/2.
- `gui = ["dep:cxx-qt", "dep:cxx-qt-lib"]`, `default = ["gui"]`; `#[cfg(feature = "gui")]` on all Qt code.
- `QMAKE=/usr/bin/qmake6` on every Qt build; `QT_VERSION_MAJOR=6` alternative.
- CI matrix: (a) `--no-default-features` build+test without Qt, (b) full build in Qt image. Per-commit: `cargo test --workspace --no-default-features`; per-wave: full suite.

### i18n Discipline (applies to: all user-facing output)
**Source:** `01-RESEARCH.md` Pattern 4 + Pitfall 6 (D-04/D-05/D-07/D-08).
- English identifiers/commands/flags; French display strings via `locales/fr.json` keys only.
- Commands `archel settings`, `archel power`, `--json` stay English; help/errors/messages French.

## No Analog Found

All 14 files — greenfield repository, zero tracked source files. Planner uses RESEARCH.md excerpts above (not abstract guidance; concrete copy-paste skeletons with line references into 01-RESEARCH.md).

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `Cargo.toml` | config | build | no workspace exists yet |
| `rust-toolchain.toml` | config | build | no toolchain pin exists yet |
| `crates/archel-core/src/lib.rs` | model | request-response | no lib crate exists yet |
| `crates/archel-core/src/error.rs` | model | transform | no error type exists yet |
| `crates/archel-core/src/i18n.rs` | utility | transform | no catalog wiring exists yet |
| `crates/archel-core/locales/fr.json` | config | transform | no catalog exists yet |
| `crates/archel-core/src/privileged.rs` | service | request-response | no privilege client exists yet |
| `crates/archel/src/main.rs` | controller | request-response | no binary exists yet |
| `crates/archel/src/cmds/` | controller | request-response | no subcommands exist yet |
| `crates/archel/build.rs` | config | build | no bridge build exists yet |
| `crates/archel/src/gui/bridge.rs` | provider | event-driven | no QObject bridge exists yet |
| `crates/archel/src/gui/qml/Spike.qml` | component | event-driven | no QML exists yet |
| `crates/archel-privileged/src/main.rs` | service | request-response | no helper exists yet |
| `policy/org.archel.privileged.policy` | config | auth | no policy file exists yet |

**Explicit non-files (planner must NOT create):** separate `archel-cli` / `archel-settings` binaries (contradicts D-01); `sudo` paths (contradicts D-09); Qt deps in `archel-core` (Pitfall 2); Slint wiring (pivot-only, decided at D-15 trigger); `hyprland.conf` helpers (out of scope, Phase 5+).

## Metadata

**Analog search scope:** repo root globs (`**/*.rs`, `**/*.toml`, `**/*.qml`, `policy/*`) + `git ls-files` (23 tracked files, all docs).
**Files scanned:** 23 tracked / 0 source.
**Pattern extraction date:** 2026-09-30
**Primary reference:** `.planning/phases/01-foundations-safety-rails/01-RESEARCH.md` (HIGH confidence; machine-verified Qt5-qmake trap, pkexec 126/127, registry versions same-day).
