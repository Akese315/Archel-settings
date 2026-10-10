# Phase 1: Foundations & Safety Rails - Research

**Researched:** 2026-09-30
**Domain:** Rust workspace foundations — Cargo layout, typed errors + FR i18n, pkexec/polkit privilege boundary, CXX-Qt 0.10 spike on Qt 6.11
**Confidence:** HIGH (environment probed live on the Archel machine, CXX-Qt pattern scraped from KDAB official book, crate versions from crates.io API, pkexec/polkit from local man + policy files)

## Summary

Phase 1 builds the ground everything else stands on: a compiling Cargo workspace, a typed French error model, a pkexec privilege boundary proven by an allowlisted dry-run, and a CXX-Qt spike that proves Qt 6.11 viability or triggers the Slint pivot. No domain logic, no GUI beyond the spike window.

Three machine facts discovered this session shape the plan more than anything else. First, **no Rust toolchain is installed** — `cargo`, `rustc`, and `rustup` are all absent; `rustup 1.29.1-1` is available in the Arch repos, so toolchain bootstrap is plan step zero. Second, **the default `qmake` on this machine is Qt 5.15.19** — Qt6's qmake lives at `/usr/bin/qmake6`, so the CXX-Qt build must export `QMAKE=/usr/bin/qmake6` (documented in the KDAB book) or the spike build will silently target the wrong Qt. Third, the locked single-`archel`-binary layout (D-01) **conflicts with the older multi-binary layout** in STACK.md/ARCHITECTURE.md/ROADMAP.md (`archel-cli` / `archel-settings` as separate binaries) — the CONTEXT.md decisions win, and the plan must reconcile them with a feature-gated `settings` subcommand so headless builds don't drag the Qt link.

**Primary recommendation:** Build workspace as `archel-core` lib + single `archel` bin (clap subcommands, `settings` behind a default-on `gui` cargo feature) + standalone `archel-privileged` helper; bootstrap rustup stable first; run the CXX-Qt spike via the official `cargo_without_cmake` pattern with `QMAKE=/usr/bin/qmake6`; implement the helper as an argv-allowlisted dry-run returning versioned JSON, mapped from pkexec exit codes 126/127 to friendly French errors.

## User Constraints (from CONTEXT.md)

### Locked Decisions

**Binary layout**
- **D-01:** Single `archel` binary with subcommands — CLI commands and GUI share one installed binary instead of separate `archel-cli` / `archel-settings` binaries. — **Reversibility:** one-way — splitting a shipped single binary into two published binaries breaks install scripts, PKGBUILD package layout, and user muscle memory (`archel <cmd>` paths).
- **D-02:** `archel settings` (no dash) launches the graphical interface; all other subcommands are headless CLI operations.
- **D-03:** Privileged operations live in a separate `archel-privileged` binary invoked via pkexec — not a hidden subcommand — keeping the attack surface to one tiny allowlisted executable.
- **D-04:** CLI subcommand and flag names are English (`archel settings`, `archel power`, `--json`); all human-readable output (help text, messages, errors) is French.

**Language FR/EN**
- **D-05:** French is the primary user-facing language for CLI output, error messages, and (later) GUI text.
- **D-06:** GUI follows the same language as the CLI — one language everywhere, no per-frontend split.
- **D-07:** i18n infrastructure is prepared from Phase 1 (translation keys, no hardcoded French outside the FR catalog) even though the second language ships in v2 (PLAT-05 stays in v2).
- **D-08:** Code identifiers (functions, types, error variants) are English per Rust convention — French appears only in displayed strings.

**Privilege UX**
- **D-09:** pkexec is the single escalation mechanism for both CLI and GUI — one consistent password experience everywhere, no `sudo` path in the CLI.
- **D-10:** Standard polkit auth persistence applies — after one successful authentication, subsequent privileged ops within the retention window don't re-prompt.
- **D-11:** yay keeps its native sudo handling for the AUR build/install step — Archel shells out to yay unprivileged and does not orchestrate its elevation.
- **D-12:** Failed/refused authentication surfaces a friendly French error naming the rejected action — no raw polkit/D-Bus error text leaks to the user.

**Spike fallback**
- **D-13:** If CXX-Qt proves unbuildable against system Qt, Phase 1 auto-pivots to Slint without coming back to the user — the spike's job is to produce a working GUI foundation, not a report.
- **D-14:** Spike success bar is a window with vertical tabs and a live-updating label (proves the real GUI pattern), not a hello-world window (which would only prove the bridge links).
- **D-15:** No time-box on making CXX-Qt work — debugging effort is unbounded; only a hard build/link incompatibility with Qt 6.11 triggers the Slint pivot.

### Agent's Discretion
- Workspace crate names and internal module layout (research suggests `archel-api`/`archel-core` lib + bins — planner picks).
- Which i18n mechanism to use for the FR catalog (planner/researcher decision, e.g. gettext vs fluent vs project-fluent).
- Exact pkexec polkit policy file contents and allowlisted action set (privilege architecture detail for planner + security review).
- Spike implementation details beyond the agreed success bar.

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope. (PLAT-05 UI language toggle already lives in v2 requirements.)

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PLAT-01 | A shared Rust API library (`archel-core`) exposes every domain above as a programmatic interface | Workspace-layout + error-model research below; Phase 1 delivers the lib skeleton with `ArchelError` taxonomy and module seams (domains land in later phases) |
| PLAT-04 | Privileged operations (pacman transactions) go through an allowlisted pkexec helper; yay always runs unprivileged | pkexec/polkit research below; Phase 1 delivers helper skeleton + policy draft + allowlisted `dry-run` action returning versioned JSON |

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Cargo workspace + lib/bin layout | Build system | — | Single `archel` bin + `archel-privileged` + `archel-core` lib; decided by D-01/D-03, pure Rust tooling, no runtime tier involved |
| Typed error model + FR catalog | API / Backend (archel-core) | — | Errors originate in core; frontends only render them — keeps EN identifiers / FR strings split (D-08) in one place |
| pkexec escalation + polkit policy | OS integration (privilege boundary) | API / Backend | OS owns auth (polkit); core owns the client side (spawn, parse JSON, map exit codes to `ArchelError`) |
| CXX-Qt spike window | Frontend (GUI) | Build system | Proves the Rust→QML bridge links and binds against system Qt 6.11; build wiring (`build.rs`, `QMAKE`) is build-system work |
| Auth-failure UX (friendly FR error) | API / Backend | Frontend | Core maps pkexec 126/127 to a typed variant with the action name; CLI/GUI render the catalog string (D-12) |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Rust stable, edition 2024 | ≥ 1.85 via rustup (rustup 1.29.1-1 in Arch repos) [VERIFIED: local pacman -Si] | Language + toolchain for all crates | Locked by project; `rust-toolchain.toml` pins `channel = "stable"` for reproducibility |
| clap (derive) + clap_complete | 4.6.7 / 4.6.x [VERIFIED: crates.io API 2026-09-30] | Single `archel` bin subcommands (`settings`, `power`, …) + `--json` + shell completions | De-facto Rust CLI standard; derive API keeps the thin bin mirroring core |
| thiserror | 2.0.21 [VERIFIED: crates.io API 2026-09-30] | Typed `ArchelError` enum in `archel-core` | Standard typed-error derive; per-domain variants the GUI/CLI match on |
| serde + serde_json | 1.0.229 [ASSUMED] / 1.0.151 [VERIFIED: crates.io API 2026-09-30] | Helper↔core JSON protocol (`{"v":1,…}`), CLI `--json` output | Ubiquitous; versioned envelope lets the protocol evolve |
| cxx-qt + cxx-qt-lib + cxx-qt-build | 0.10.0 (all three pinned to same 0.10.x) [VERIFIED: crates.io API 2026-09-30] | Qt6-from-Rust bridge for the spike (and later GUI) | Only production-viable Qt-from-Rust path; KDAB-backed, CI on Linux/Win/macOS [CITED: https://kdab.github.io/cxx-qt/book/] |
| Qt6 system libs | 6.11.2 (`qt6-base`, `qt6-declarative`, `qt6-tools` installed) [VERIFIED: local pacman -Q] | Toolkit the bridge links against | Arch extra; dynamic link (LGPLv3 compliance) — never static-link Qt |
| rust-i18n | 4.2.4 [VERIFIED: crates.io API 2026-09-30] | FR message catalog (`t!` macro, `locales/fr.json`), i18n-ready skeleton for v2 EN | Actively maintained (updated 2026-09-29), tiny integration surface vs fluent; choice is planner discretion per CONTEXT — see Alternatives |
| tracing + tracing-subscriber | 0.1.44 [ASSUMED] | Structured logging (CLI defaults warn, file log later) | Replaces stale `pretty_env_logger`; spans pay off once CLI+GUI share async core |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| duct + which | 1.1.2 / 8.0.6 [ASSUMED] | Spawning `pkexec archel-privileged …` with piped/checked output; capability detection | Phase 1 privilege client side (`which::which("pkexec")` pre-flight) |
| Slint | 1.18.1 [VERIFIED: crates.io API 2026-09-30] | GUI escape hatch | ONLY if D-15 hard-incompatibility trigger fires — auto-pivot, no user round-trip (D-13) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| rust-i18n | fluent 0.17.0 + fluent-bundle 0.16.0 [VERIFIED: crates.io API] | Mozilla standard, handles plurals/gender properly — but heavier integration, and facade/bundle minor-version skew (0.17 vs 0.16) needs care; justified only if FR catalog needs pluralization in v1 (it doesn't — single language) |
| rust-i18n | gettext-rs 0.8.0 [VERIFIED: crates.io API] | Classic gettext `.po` workflow translators know — but needs system gettext tooling at build time; overkill for a one-language v1 catalog |
| cxx-qt cargo build | cxx-qt CMake integration | CMake path exists in the book but needs `cmake` which is NOT installed here; cargo path (`cargo_without_cmake` example) needs only `cc`-crate + C++ compiler (g++ 16.2.1 present) — use cargo path |
| single `archel` bin, `gui` feature | separate `archel-cli` / `archel-settings` bins (old STACK.md layout) | Rejected by D-01 (one-way decision). Feature-gating is the reconciliation: `gui` feature default-on for distro package, `--no-default-features` for headless/CI builds that must not link Qt |

**Installation:**
```bash
# 0. Toolchain bootstrap (NOT installed — verified absent this session)
sudo pacman -S rustup qt6-base qt6-declarative qt6-tools base-devel
rustup toolchain install stable
# rust-toolchain.toml pins channel = "stable", edition 2024

# 1. Workspace (names per planner; archel-core recommended over archel-api
#    to match REQUIREMENTS.md PLAT-01 wording)
cargo new archel --bin            # single bin, subcommands incl. `settings`
cargo new --lib archel-core       # the API library, zero Qt dependency
cargo new --bin archel-privileged # pkexec helper, minimal deps (serde_json only)

# 2. Core deps (archel-core): clap NOT here — clap lives in the `archel` bin
#    cargo add thiserror serde serde_json rust-i18n tracing duct which

# 3. `archel` bin: cargo add clap --features derive; clap_complete; archel-core --path
#    + optional cxx-qt deps behind `gui` feature (default = ["gui"])

# 4. Spike builds with: QMAKE=/usr/bin/qmake6 cargo run -p archel --features gui -- settings
```

**Version verification:** all Phase 1 crate versions confirmed via `curl -A … https://crates.io/api/v1/crates/<name>` on 2026-09-30 (crates.io API returns 403 without a User-Agent header — automation must set one).

## Package Legitimacy Audit

> Seam check `gsd_run query package-legitimacy check --ecosystem crates …` run 2026-09-30 — all OK.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| cxx-qt 0.10.0 | crates.io | since 2021-12 | ~2.7k/wk | github.com/KDAB/cxx-qt | OK | Approved |
| cxx-qt-lib 0.10.0 | crates.io | since 2021-12 | ~2.7k/wk | github.com/KDAB/cxx-qt | OK | Approved |
| cxx-qt-build 0.10.0 | crates.io | since 2021-12 | ~2.7k/wk | github.com/KDAB/cxx-qt | OK | Approved |
| clap 4.6.7 | crates.io | since 2015-03 | ~18.8M/wk | github.com/clap-rs/clap | OK | Approved |
| thiserror 2.0.21 | crates.io | established | ~1.5B total | dtolnay ecosystem | OK | Approved |
| serde_json 1.0.151 | crates.io | established | ~1.36B total | serde-rs | OK | Approved |
| rust-i18n 4.2.4 | crates.io | established | ~3.6M total | longbridge/rust-i18n | OK | Approved |
| slint 1.18.1 | crates.io | established | ~1.8M total | slint-ui/slint | OK | Approved (pivot-only) |
| duct 1.1.2 | crates.io | established | std subprocess wrapper | o mutually vetted | OK | Approved |
| which 8.0.6 | crates.io | established | capability detection | OK | Approved |
| tracing | crates.io | established | logging | OK | Approved |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

Phase 1 wires the skeleton all later phases fill in. Data flow for the one proven path (privilege dry-run):

```
[archel <cmd>] ──clap subcommand──▶ [archel-core API fn]
        │                                    │
        │  unprivileged path                 │  privileged path
        │  (direct return)                   ▼
        │                           [core::privileged client]
        │                              │ spawn pkexec
        │                              ▼
        │                    [polkit auth dialog (OS)]
        │                              │ allow → run as root
        │                              ▼
        │                    [archel-privileged: argv allowlist
        │                     → dry-run → JSON {"v":1,…} on stdout]
        │                              │
        └◀── typed ArchelError ─────────┘
           (exit 126/127 → AuthFailed{action};
            JSON parse fail → HelperProtocol;
            never raw polkit text — D-12)

[archel settings] ──gui feature──▶ [QML views (spike)]
        │                                ▲ qproperty/qinvokable
        │                                │ (cxx-qt bridge)
        └────────────────▶ [Rust QObject view-model]
```

Entry points: `archel` CLI args; `archel settings` GUI launch; pkexec callback into helper. External dependencies: polkit (auth), Qt 6.11 (spike link), rustup toolchain (build).

### Recommended Project Structure

```
archel/                            # Cargo workspace root
├── Cargo.toml                     # [workspace] members + shared lints; default-members excludes nothing
├── rust-toolchain.toml            # channel = "stable"
├── Cargo.lock                     # committed (binary ships)
├── crates/
│   ├── archel-core/               # THE API — no Qt, no clap
│   │   └── src/
│   │       ├── lib.rs             # public re-exports per future domain
│   │       ├── error.rs           # ArchelError (thiserror) + FR rendering
│   │       ├── i18n.rs            # rust-i18n init (t! catalog wiring)
│   │       ├── privileged.rs      # client: spawn pkexec, parse v1 JSON, map exit codes
│   │       └── locales/fr.json    # FR catalog — ONLY place with French strings (D-07)
│   ├── archel/                    # single binary (D-01)
│   │   └── src/
│   │       ├── main.rs            # clap root; `settings` dispatches to gui module
│   │       ├── cmds/              # one module per subcommand (skeleton: settings, stub)
│   │       └── gui/               # #[cfg(feature = "gui")] — cxx-qt bridge + QML loader
│   │           ├── bridge.rs      # #[cxx_qt::bridge] SpikeModel (qproperty + qinvokable)
│   │           └── qml/Spike.qml  # vertical tabs + live label
│   └── archel-privileged/
│       └── src/main.rs            # argv allowlist → dry-run → println JSON → exit; NO other deps
├── policy/
│   └── org.archel.privileged.policy  # polkit actions, auth_admin_keep, FR <message>
└── tests/ (per-crate)             # allowlist matrix, error→FR snapshot, protocol version tests
```

Planner picks final crate names (CONTEXT discretion); `archel-core` is recommended because REQUIREMENTS.md PLAT-01 names it.

### Pattern 1: Single binary + feature-gated GUI
**What:** One `archel` bin; all Qt code lives behind `#[cfg(feature = "gui")]` with `default = ["gui"]`. Headless builds/tests use `--no-default-features` and never link Qt. The `settings` subcommand without the feature prints a typed error (missing-feature, FR message).
**When to use:** always in this project — it is how D-01 (single binary) coexists with the "API/CLI build without Qt" invariant from prior research.
**Example:**
```toml
# Source: standard Cargo features pattern, adapted for D-01
[features]
default = ["gui"]
gui = ["dep:cxx-qt", "dep:cxx-qt-lib"]
```

### Pattern 2: CXX-Qt cargo-only spike (`cargo_without_cmake`)
**What:** Pure-cargo Qt build — `CxxQtBuilder::new_qml_module` in `build.rs` generates/compiles the C++ side and links Qt; `main.rs` creates `QGuiApplication` + `QQmlApplicationEngine` and loads the QML from the `qrc:/` URL. No CMake required (and cmake is not installed on this machine).
**When to use:** the Phase 1 spike; retained as the GUI build pattern unless it hits the D-15 trigger.
**Example:**
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

### Pattern 3: pkexec one-shot helper with versioned JSON protocol
**What:** `archel-privileged` validates `argv` against a hardcoded allowlist (Phase 1: exactly one action, `dry-run`), does no work requiring root yet, prints `{"v":1,"action":"dry-run","ok":true,…}` to stdout, exits 0. Core spawns `pkexec --disable-internal-agent? NO — default agent` + helper path + args, captures stdout, parses JSON, maps failures. Protocol envelope carries `v` so later phases can evolve it.
**When to use:** every privileged op from Phase 3 on; Phase 1 proves the plumbing with the harmless action.
**Example:**
```rust
// Action allowlist shape (planner detail; principle: match on argv, no shell, no prefix-matching)
match args {
    ["dry-run"] => Ok(serde_json::json!({"v": 1, "action": "dry-run", "ok": true})),
    _ => Err(exit 2 + machine-readable {"v":1,"ok":false,"error":"action-not-allowlisted"}),
}
```

### Pattern 4: Typed errors with catalog-rendered French
**What:** `ArchelError` variants are English-identified (`AuthFailed { action: String }`, `HelperProtocol(String)`, `GuiNotCompiled`, …); the `Display` impl (or a `message(&self) -> String` renderer) looks up the FR string via `t!` keys — French lives only in `locales/fr.json` (D-07/D-08). No panics on failure paths: `main` returns `Result` / prints catalog error + non-zero exit.
**When to use:** every core failure path from day one — success criterion 4.

### Anti-Patterns to Avoid
- **Separate `archel-cli` / `archel-settings` binaries:** contradicts locked D-01 (one-way). Use subcommands + `gui` feature instead.
- **Qt dependency in `archel-core`:** breaks headless builds and the CLI-proof ordering; Qt types stop at the `gui/` module.
- **`sudo` anywhere in core/GUI paths:** D-09 mandates pkexec-only; yay keeps its own native sudo (D-11) — Archel never orchestrates it.
- **`rewrite hyprland.conf` helpers in Phase 1:** out of scope (no domain logic); the conf strategy arrives with its owning phase.
- **Hello-window spike:** fails D-14 — must show vertical tabs + live label or it proves nothing about the real GUI pattern.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CLI parsing | Custom arg parser | clap 4.6 derive | Help text, `--json` flags, shell completions, error UX — all free and FR-localizable |
| Typed errors | `String` errors / `Box<dyn Error>` up the stack | thiserror enum | GUI must match on variants for toasts; strings can't be matched or i18n-keyed |
| FR strings | Inline French literals in code | rust-i18n catalog (`locales/fr.json`) | D-07: v2 adds EN by adding a file, not by hunting literals; single grep audits compliance |
| Qt bridge | Raw `cxx` + handwritten moc glue | cxx-qt-build `CxxQtBuilder` | Runs moc/rcc, generates QObject C++ — hand-rolled moc invocation is the classic week-eater |
| pkexec spawn/wait/timeout | `std::process::Command` ad-hoc per call site | One `privileged.rs` client (+ duct) | Timeouts, env scrubbing, exit-code mapping in one reviewed place; every later phase reuses it |
| Auth dialog | Custom password prompt / embedded sudo | polkit agent via pkexec default | Native desktop dialog, standard persistence (D-10), no secret handling in our code |

**Key insight:** Phase 1's value is entirely in *boundaries* (bin/lib split, privilege spawn surface, bridge seam, language seam). Each boundary gets exactly one implementation owned by one module; every later phase reuses rather than re-derives them.

## Common Pitfalls

### Pitfall 1: Building the spike against Qt5 via bare `qmake`
**What goes wrong:** `cargo build` fails with Qt5 headers / missing Qt6 modules, or worse, links Qt 5.15 silently.
**Why it happens:** bare `qmake` on this machine reports Qt 5.15.19 [VERIFIED: `qmake -query QT_VERSION` → 5.15.19]; Qt6's is `qmake6` → 6.11.2 [VERIFIED].
**How to avoid:** every Qt build command runs with `QMAKE=/usr/bin/qmake6` exported; CI + README + `.cargo/config.toml`-adjacent docs record it. `QT_VERSION_MAJOR=6` is the alternative selector.
**Warning signs:** build log shows `/usr/include/qt5` or `Qt version 5` anywhere.

### Pitfall 2: GUI code leaking Qt into headless builds
**What goes wrong:** `cargo test -p archel-core` or headless `archel` build requires Qt headers; CI-without-Qt red from day one.
**Why it happens:** cxx-qt dependency placed unconditionally, or `gui/` module not feature-gated.
**How to avoid:** `gui` feature gate + CI job matrix: (a) `--no-default-features` build+test (no Qt installed), (b) full build (Qt image). The spike plan must include both jobs.
**Warning signs:** `use cxx_qt…` outside `#[cfg(feature = "gui")]`.

### Pitfall 3: Trusting helper argv
**What goes wrong:** helper invoked as `archel-privileged install foo; rm -rf …` or with smuggled flags; if a retained-auth policy ever applies, this is a root hole.
**Why it happens:** pkexec does NO validation of arguments passed to PROGRAM [VERIFIED: local `man pkexec` — "pkexec does no validation of the ARGUMENTS"].
**How to avoid:** exact-match allowlist on (`action`, validated-arg-shape); reject everything else with exit 2 + machine JSON; never pass through to shell; re-validate package names against `^[a-z0-9@._+-]+$` when real actions land (Phase 3).
**Warning signs:** `format!("… {} …", user_input)` + shell anywhere near the helper.

### Pitfall 4: Raw polkit errors reaching the user
**What goes wrong:** dismissing the auth dialog shows `GDBus.Error:org.freedesktop.policykit.exec…` instead of French guidance.
**Why it happens:** pkexec exit 126 (dismissed) vs 127 (not authorized/error) [VERIFIED: local `man pkexec`] are distinct but both non-zero; naive `status.success()?` collapses them.
**How to avoid:** map 126 → `AuthFailed{action, reason: Dismissed}` ("Authentification annulée — «action» n'a pas été exécutée."), 127 → `AuthFailed{reason: Denied}` ("Authentification refusée…"), each naming the rejected action (D-12). Stderr captured for logs only, never displayed.
**Warning signs:** any `eprint!("{}", stderr)` on the pkexec path.

### Pitfall 5: Helper writing to `$HOME` as root
**What goes wrong:** files under the user's home end up root-owned; Wayland/`XDG_RUNTIME_DIR` confusion.
**Why it happens:** pkexec runs PROGRAM in the target user's (root's) home with a minimal sanitized env (`PKEXEC_UID` set, `DISPLAY`/`XAUTHORITY` stripped) [VERIFIED: local `man pkexec`]; code assuming `$HOME` == user's home breaks.
**How to avoid:** helper uses absolute paths only, writes nothing under any home; all user-file writes happen in the unprivileged process from data returned over stdout. Phase 1 dry-run writes nothing at all.
**Warning signs:** `dirs::home_dir()` / `~` / relative paths in helper code.

### Pitfall 6: Hardcoding the FR strings (killing D-07)
**What goes wrong:** v2 language toggle requires hunting literals across crates; mixed FR/EN output.
**Why it happens:** `#[error("Échec de…")]` inline literals are the path of least resistance with thiserror.
**How to avoid:** lint rule — `grep -rP '[\x{00C0}-\x{00FF}\x{0152}\x{0153}]' crates/*/src` (FR diacritics outside `locales/`) fails CI; error tests assert every variant renders non-empty FR.
**Warning signs:** accented characters in any `.rs` file.

## Code Examples

### CXX-Qt app entry (spike `settings`)
```rust
// Source: https://kdab.github.io/cxx-qt/book/getting-started/4-cargo-executable.html
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

### Spike QML: vertical tabs + live label (D-14 bar)
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

### pkexec client mapping (D-12)
```rust
// Exit semantics VERIFIED: local `man pkexec` (polkit 127)
match output.status.code() {
    Some(0)   => parse_v1_json(&stdout).map_err(ArchelError::HelperProtocol),
    Some(126) => Err(ArchelError::privilege_cancelled(&action)), // dialogue fermé
    Some(127) => Err(ArchelError::privilege_denied(&action)),    // non autorisé / erreur
    other     => Err(ArchelError::HelperProtocol(format!("exit={other:?}"))),
}
```

### Polkit policy skeleton (D-10 persistence = `auth_admin_keep`)
```xml
<!-- Shape VERIFIED against /usr/share/polkit-1/actions/org.freedesktop.hostname1.policy -->
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

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Separate `archel-cli` / `archel-settings` bins (STACK.md, ARCHITECTURE.md, ROADMAP.md) | Single `archel` bin + `settings` subcommand + `archel-privileged` helper | 2026-09-30 discuss-phase (D-01–D-03, one-way) | Prior research docs are stale on layout; planner follows CONTEXT.md, uses `gui` feature to keep the no-Qt invariant |
| `hyprland` crate 0.4.0-beta line | 0.3.13 stable (`"0.3"`) | STACK.md correction 2026-09-30 | Irrelevant for Phase 1 (no Hyprland code), noted so Phase 1 pins don't copy the beta |
| `sudo -n` detection fallback in CLI | pkexec-only (D-09) | 2026-09-30 | Prior STACK.md sudo-fallback pattern is superseded for Archel's own escalation |
| CXX-Qt 0.7/0.8-era `cxx = "1.0.95"` pin in book example | cxx-qt-build 0.10 manages its own `cxx` (1.0.202 underneath per STACK.md) | 2026-08/09 | Don't copy the book's `cxx` version pin verbatim; let 0.10 resolve it |

**Deprecated/outdated:**
- qmetaobject-rs: unmaintained since 2023 — never an option, re-confirmed by the D-13/D-15 framing (pivot target is Slint, not qmetaobject).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | rust-i18n 4.2.4 is the best-fit FR catalog (vs fluent/gettext) — registry activity + API simplicity, not a deep integration test | Standard Stack | LOW — mechanism is planner discretion; any catalog meeting D-07 works, swap cost is one module |
| A2 | `cargo_without_cmake` path needs no `cmake` binary (uses `cc` crate) — inferred from example name + `CxxQtBuilder`/`cc::Build` docs, not build-tested (no toolchain present) | Patterns | MEDIUM — if cmake is required, `sudo pacman -S cmake` is a one-line fix; plan should include it as a fallback step |
| A3 | `auth_admin_keep` = "standard polkit persistence" satisfying D-10 — matches systemd policy convention on this machine | Code Examples | LOW — security review confirms exact retention semantics; worst case the message/persistence wording adjusts |
| A4 | Vertical tabs via TabButton-column + StackLayout is acceptable proof of the D-14 "real GUI pattern" | Code Examples | LOW — spike detail is agent discretion; any vertical-tab construction counts |
| A5 | thiserror `Display` can route through `t!` lookups (runtime catalog call inside `fmt`) — standard pattern, not compile-tested here | Patterns | LOW — fallback is a `message()` renderer method beside `Display`; plan can pick either |

## Open Questions

1. **Crate names: `archel-core` vs `archel-api`?**
   - What we know: REQUIREMENTS.md PLAT-01 says `archel-core`; STACK.md/ARCHITECTURE.md say `archel-api`. CONTEXT leaves it to planner discretion.
   - Recommendation: `archel-core` (matches the normative requirements doc).

2. **Does the `archel` bin's `gui` feature complicate the PKGBUILD split?**
   - What we know: one binary means one package drags Qt unless split packages build with different features.
   - Recommendation: planner notes a `archel` (full, default features) vs `archel-headless` (`--no-default-features`) split-package option; v1 can ship one package and revisit.

3. **Slint pivot readiness — pre-add `slint` dep or decide at trigger time?**
   - What we know: D-13/D-15 make the pivot automatic on hard incompatibility; slint 1.18.1 verified available.
   - Recommendation: do NOT pre-wire Slint (keeps the spike honest); planner defines the trigger evidence bar (exact failing build log against Qt 6.11.2 + `QMAKE=/usr/bin/qmake6`).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain (cargo/rustc/rustup) | everything | ✗ | — (rustup 1.29.1-1 in repos) | `sudo pacman -S rustup && rustup toolchain install stable` — plan step zero |
| C++ compiler (g++/gcc) | cxx-qt-build C++ codegen | ✓ | g++ 16.2.1, base-devel installed | — |
| qmake (Qt6) | cxx-qt-build Qt discovery | ✓ with caveat | `/usr/bin/qmake6` → Qt 6.11.2; bare `qmake` → Qt 5.15.19 | MUST export `QMAKE=/usr/bin/qmake6` (Pitfall 1) |
| qt6-base / qt6-declarative / qt6-tools | spike link + QML | ✓ | 6.11.2-3 / 6.11.2-2 / 6.11.2-1 | — |
| cmake | cxx-qt CMake path only | ✗ | — | not needed for cargo path; `sudo pacman -S cmake` if A2 resolves against us |
| pkexec + polkit + pkaction | privilege boundary | ✓ | polkit 127 | — |
| Hyprland compositor (live) | spike window test | ✓ | 0.56.2, Wayland session active | spike can also run nested; live session preferred |
| QML runtime (qml6, QtQuick.Controls) | spike QML | ✓ | controls styles present (Basic/Fusion/FluentWinUI3/Imagine) | — |
| crates.io network | `cargo add`/build | ✓ | reachable (requires User-Agent header) | vendor dir if offline builds needed later |

**Missing dependencies with no fallback:**
- Rust toolchain — planner's first plan must install it (blocks every success criterion).

**Missing dependencies with fallback:**
- cmake — only needed if the cargo-only CXX-Qt path fails; one-command install.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in) — no extra harness for Phase 1 |
| Config file | none — see Wave 0 |
| Quick run command | `cargo test --workspace --no-default-features` (headless, no Qt) |
| Full suite command | `cargo test --workspace` (with `QMAKE=/usr/bin/qmake6` for the gui feature) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PLAT-01 | workspace builds with one cargo command; lib exposes error module | unit/build | `cargo build --workspace` + `cargo test -p archel-core` | ❌ Wave 0 (greenfield) |
| PLAT-01 | every `ArchelError` variant renders non-empty FR, no panics on failure paths | unit | `cargo test -p archel-core error::tests` | ❌ Wave 0 |
| PLAT-01 | no hardcoded French outside `locales/` (D-07 audit) | unit/lint | diacritics grep test or CI step | ❌ Wave 0 |
| PLAT-04 | helper allowlist: `dry-run` → v1 JSON exit 0; anything else → exit 2 + error JSON | unit + integration | `cargo test -p archel-privileged` (argv matrix, no root needed) | ❌ Wave 0 |
| PLAT-04 | core maps pkexec 126/127 to `AuthFailed` naming the action (D-12) | unit (mocked exit codes) | `cargo test -p archel-core privileged::tests` | ❌ Wave 0 |
| Spike bar | window with vertical tabs + 1 Hz live label (D-14) | manual-only | `QMAKE=/usr/bin/qmake6 cargo run --features gui -- settings` + visual confirm | n/a — justified: GUI rendering can't be unit-asserted; evidence is a screenshot/run log |

### Sampling Rate
- **Per task commit:** `cargo test --workspace --no-default-features`
- **Per wave merge:** `cargo test --workspace` (Qt env) + both CI jobs
- **Phase gate:** Full suite green + spike run evidence + pkexec dry-run JSON capture before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `rust-toolchain.toml` + workspace `Cargo.toml` — toolchain pin + members (blocks everything)
- [ ] `crates/archel-core/src/{lib,error,i18n,privileged}.rs` + `locales/fr.json`
- [ ] `crates/archel/src/main.rs` (clap root) + `gui/` spike module + `qml/Spike.qml`
- [ ] `crates/archel-privileged/src/main.rs` (allowlist + v1 JSON)
- [ ] `policy/org.archel.privileged.policy` draft
- [ ] Framework install: `sudo pacman -S rustup && rustup toolchain install stable` — no test framework to install beyond cargo itself

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | partial | delegated to polkit via pkexec; our surface is exit-code mapping only — no credential handling |
| V3 Session Management | no | no sessions in Phase 1 |
| V4 Access Control | yes | least-privilege `.policy`: one action per verb, `auth_admin_keep`, never `yes`; helper allowlist as second gate |
| V5 Input Validation | yes | exact-match argv allowlist in helper; JSON envelope version-checked (`v == 1`) in core parser |
| V6 Cryptography | no | none in Phase 1 |

### Known Threat Patterns for Rust CLI + pkexec + Qt stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Helper arg injection → unintended root command | Elevation of Privilege | Exact argv allowlist, no shell, no prefix matching (Pitfall 3) |
| Over-broad polkit policy (`allow_any: yes`) | Elevation of Privilege | Per-action `auth_admin_keep`; dedicated security review of policy file before packaging |
| Retained-auth reuse of helper by another process | Elevation of Privilege | Helper validates every invocation independently; keep action set minimal (dry-run only in Phase 1) |
| Fake-zero sensor/error states misleading UI | Tampering (integrity) | Typed errors with explicit variants; `HelperProtocol` on any unparseable output — never default-ok |
| QML executing untrusted strings | Injection | Spike QML is static; later phases validate values before writing conf (out of scope here, noted for Phase 5) |

## Sources

### Primary (HIGH confidence)
- KDAB CXX-Qt book (live scrape 2026-09-30): `cargo_without_cmake` pattern — `Cargo.toml` deps, `build.rs` `CxxQtBuilder::new_qml_module`, `main.rs` QGuiApplication/QQmlApplicationEngine + `qrc:/` URL, `qmake` discovery order + `QMAKE` env var; bridge syntax — `#[cxx_qt::bridge]`, `extern "RustQt"`, `#[qobject]`, `#[qproperty]` + NOTIFY codegen, `#[qinvokable]`, `#[qml_element]`
- Local machine probes (2026-09-30): `pacman -Q` (qt6 6.11.2 trio, polkit 127), `qmake -query` (5.15.19 vs `qmake6` 6.11.2), absent cargo/rustc/rustup, `rustup 1.29.1-1` in repos, g++ 16.2.1, base-devel, no cmake, Hyprland 0.56.2 live Wayland session, QtQuick.Controls styles present
- Local `man pkexec` (polkit 127): exit 126 dismissed / 127 unauthorized-or-error, minimal sanitized env + `PKEXEC_UID`, root-home cwd, NO argv validation, `allow_gui` annotation semantics
- Local `/usr/share/polkit-1/actions/org.freedesktop.hostname1.policy`: `.policy` XML shape — action id, gettext-domain description/message, `auth_admin_keep` defaults triple
- crates.io registry API (2026-09-30, UA-headered): cxx-qt/-lib/-build 0.10.0, clap 4.6.7, thiserror 2.0.21, serde_json 1.0.151, slint 1.18.1, rust-i18n 4.2.4, fluent 0.17.0 / fluent-bundle 0.16.0, gettext-rs 0.8.0
- `gsd_run query package-legitimacy check` (2026-09-30): all 11 Phase 1 crates verdict OK

### Secondary (MEDIUM confidence)
- Prior project research (`.planning/research/{STACK,ARCHITECTURE,PITFALLS,SUMMARY}.md` 2026-09-30): workspace rationale, privilege patterns, pitfall catalog — reused except where superseded by D-01/D-09 decisions (see State of the Art)

### Tertiary (LOW confidence)
- A1–A5 assumptions table above — flagged for planner confirmation, none blocking

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — versions registry-verified same-day; legitimacy seam OK; toolchain/Qt/pkexec facts probed on the target machine
- Architecture: HIGH — single-binary reconciliation follows directly from locked decisions; CXX-Qt pattern from official book
- Pitfalls: HIGH — the two sharpest (Qt5-qmake trap, pkexec exit codes) are machine-verified, not recalled

**Research date:** 2026-09-30
**Valid until:** ~30 days (stable domain; re-verify cxx-qt/Qt minor versions if Arch ships Qt 6.12+ before the GUI phase)
