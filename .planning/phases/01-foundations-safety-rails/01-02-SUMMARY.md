---
phase: 01-foundations-safety-rails
plan: 02
subsystem: infra
tags: [rust, pkexec, polkit, cxx-qt, qml, qt6, privilege-boundary]

# Dependency graph
requires:
  - phase: 01-01
    provides: "Compiling headless workspace, typed ArchelError with French catalog, bin-to-core wiring"
provides:
  - "Allowlisted archel-privileged helper speaking versioned v1 JSON (dry-run only)"
  - "archel-core privileged client: direct spawn + pkexec path with 126/127 French mapping"
  - "Least-privilege polkit policy draft (auth_admin_keep, never yes)"
  - "CXX-Qt 0.10 spike window on Qt 6.11: vertical tabs + 1 Hz live label, headless stays Qt-free"
affects: [privilege-boundary, packages-phase, gui-phase, all-domain-phases]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
actuals:
  tokens: 7800
  tasks: 3
  commits: 4

# Tech tracking
tech-stack:
  added: [serde-1.0-derive, serde_json-1.0, duct-1.1, which-8.0, cxx-1, cxx-qt-0.10, cxx-qt-lib-0.10-qt_gui-qt_qml, cxx-qt-build-0.10-link_qt_object_files]
  patterns: [exact-match-argv-allowlist, versioned-json-envelope, single-privileged-client, feature-gated-qt, cargo-only-cxxqt-build]

key-files:
  created: [crates/archel-core/src/privileged.rs, crates/archel/src/gui/bridge.rs, crates/archel/src/gui/mod.rs, crates/archel/src/gui/qml/Spike.qml, crates/archel/build.rs, policy/org.archel.privileged.policy]
  modified: [crates/archel-privileged/src/main.rs, crates/archel-core/src/lib.rs, crates/archel-core/Cargo.toml, crates/archel-core/locales/fr.json, crates/archel/Cargo.toml, crates/archel/src/main.rs, crates/archel/src/cmds/mod.rs, Cargo.lock, .gitignore]

key-decisions:
  - "Zero-dep helper (hand-written fixed JSON strings) instead of serde_json in archel-privileged — smallest possible root attack surface"
  - "cxx-qt-lib features qt_gui+qt_qml only (not qt_full) — minimal Qt link surface for the spike"
  - "cxx-qt-build as optional build-dep + cfg-gated build.rs — headless builds never compile Qt tooling"
  - "Two qproperties (tick_label + tick_count) — counter state lives in Qt properties via generated accessors, no unsafe Pin projection"
  - "CLI privileged dry-run stays on the direct-spawn path; pkexec path exposed for Phase 3 callers (pub, unit-tested)"

patterns-established:
  - "Helper speaks fixed v1 JSON envelopes; core rejects everything else as HelperProtocol (never default-ok)"
  - "pkexec exit mapping: 0 -> parse, 126 -> cancelled, 127 -> denied, other -> HelperProtocol with raw code for logs"
  - "All Qt types stop at crates/archel/src/gui/ behind the default-on gui feature"

requirements-completed: [PLAT-04]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "Allowlisted helper dry-run returns v1 JSON exit 0; smuggled/empty/unknown argv exit 2 with error JSON"
    requirement: "PLAT-04"
    verification:
      - kind: unit
        ref: "crates/archel-privileged/src/main.rs#allowlist matrix (5 tests)"
        status: pass
      - kind: other
        ref: "cargo run -q -p archel --no-default-features -- privileged dry-run prints French success line, exit 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "pkexec 126/127 map to distinct friendly French AuthFailed errors naming the action; stderr log-only"
    requirement: "PLAT-04"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/privileged.rs#pkexec exit-code matrix (6 tests)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Polkit policy draft: dry-run action, auth_admin_keep triple, never yes"
    requirement: "PLAT-04"
    verification:
      - kind: other
        ref: "XML validated via minidom; grep: 3x auth_admin_keep, 0x yes-rules; shape mirrors hostname1.policy"
        status: pass
    human_judgment: false
  - id: D4
    description: "Spike window builds against Qt 6.11 (QMAKE=qmake6), boots with zero QML errors, headless stays Qt-free"
    requirement: "PLAT-04"
    verification:
      - kind: other
        ref: "QMAKE=/usr/bin/qmake6 cargo build -p archel (0 qt5 paths); offscreen settings run alive 6-8s, no QML errors"
        status: pass
    human_judgment: true
    rationale: "Offscreen run proves QML loads and the event loop ticks, but visual adequacy of vertical tabs + 1 Hz label needs a human eye on the live Hyprland session"

# Metrics
duration: 12min
completed: 2026-09-30
status: complete
---

# Phase 01 Plan 02: Privilege Boundary + CXX-Qt Spike Summary

**Allowlisted pkexec helper with versioned v1 JSON, D-12 exit-code mapping, least-privilege policy draft, and a CXX-Qt 0.10 spike window (vertical tabs + 1 Hz live label) proven against system Qt 6.11**

## Performance

- **Duration:** 12 min
- **Started:** 2026-09-30T19:36:05Z
- **Completed:** 2026-09-30T19:47:45Z
- **Tasks:** 3 (1 tracer + 2 auto)
- **Files modified:** 15

## Accomplishments

- End-to-end tracer: `archel privileged dry-run` spawns the helper directly, validates the v1 envelope, and prints `Module privilegie verifie : reponse JSON v1 validee pour dry-run` (exit 0); smuggled args exit 2 with machine JSON
- pkexec client path with the verified exit map (0/126/127/other) and French cancelled-vs-denied strings naming the action; helper stderr captured for logs only
- `policy/org.archel.privileged.policy` draft mirrors the live `hostname1.policy` shape: one dry-run action, `auth_admin_keep` on all three defaults, no `yes` anywhere
- Spike window links and boots on Qt 6.11.2 via the cargo-only CXX-Qt path: QML module `org.archel.spike`, `SpikeModel` bridge (`tick_label` NOTIFY + `tick()`), vertical TabButton rail + StackLayout, offscreen run holds the event loop with zero QML errors; headless build never touches Qt

## Task Commits

Each task was committed atomically:

1. **Task 1: End-to-end helper dry-run tracer** - `d0c85c6` (feat)
2. **Task 2: pkexec exit-code mapping + polkit policy draft** - `265624c` (feat)
3. **Task 3: CXX-Qt spike window** - `eecad6c` (feat) + `ce07b08` (chore: ignore Qt-generated `.qmlls.ini`)

## Files Created/Modified

- `crates/archel-privileged/src/main.rs` - exact-match `["dry-run"]` allowlist, fixed v1 JSON, exit 0/2, zero dependencies + 5 matrix tests
- `crates/archel-core/src/privileged.rs` - single privileged client: envelope parser, helper locator (absolute, exe-relative), direct spawn, `map_pkexec_exit`, pkexec spawn, catalog success line + 11 tests
- `crates/archel-core/src/lib.rs` - `pub mod privileged` re-export seam
- `crates/archel-core/Cargo.toml` - serde (derive), serde_json, duct, which (all at research-pinned versions)
- `crates/archel-core/locales/fr.json` - `privileged.dry_run_ok` French success string (auth strings pre-existed from 01-01)
- `crates/archel/src/cmds/mod.rs` - `privileged dry-run` subcommand; `settings` dispatches to `gui::run_settings()` behind the feature
- `crates/archel/src/main.rs` - `#[cfg(feature = "gui")] mod gui`
- `crates/archel/Cargo.toml` - optional cxx/cxx-qt/cxx-qt-lib (qt_gui+qt_qml) + optional cxx-qt-build (link_qt_object_files), all joined by the `gui` feature
- `crates/archel/build.rs` - cfg-gated `CxxQtBuilder::new_qml_module`, no-op without `gui`
- `crates/archel/src/gui/bridge.rs` - `SpikeModel` QObject: `tick_label`/`tick_count` Q_PROPERTYs + `tick()` invokable
- `crates/archel/src/gui/mod.rs` - `QGuiApplication` + `QQmlApplicationEngine` entry loading the `qrc:/` spike URL
- `crates/archel/src/gui/qml/Spike.qml` - vertical tabs (TabButton column + StackLayout), French labels, 1 Hz Timer
- `policy/org.archel.privileged.policy` - least-privilege draft (T-1-04)
- `.gitignore` - ignore Qt-regenerated `.qmlls.ini`

## Decisions Made

- **Zero-dep helper:** fixed JSON string literals instead of serde_json in the root-executed binary — nothing to deserialize, nothing to misconfigure; core side still parses strictly with serde_json.
- **Minimal Qt features (`qt_gui` + `qt_qml`, not `qt_full`):** verified against the extracted `cxx-qt-lib 0.10.0` manifest; `qt_quickcontrols` needs no Rust bindings (pure-QML usage).
- **Optional build-dependency + cfg-gated `build.rs`:** `cargo build -p archel --no-default-features` compiles without `cxx-qt-build` present at all (Pitfall 2 proof, not just discipline).
- **Bare `Pin<&mut Self>` receiver (book-exact):** the bridge macro parser rejects the fully-qualified `core::pin::Pin` path — surface syntax must match the KDAB example verbatim.
- **Counter as second Q_PROPERTY:** avoids unsafe `Pin` field projection; both properties update via generated accessors with `as_mut()` reborrows.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Workspace build required before CLI verify**
- **Found during:** Task 1 (tracer verify)
- **Issue:** `cargo run -p archel` alone does not build the sibling `archel-privileged` binary, so `locate_helper()` found a stale plan-01 stub (exit 2, no JSON) and the CLI printed a protocol error
- **Fix:** `cargo build --workspace --no-default-features` before the CLI run (plan-verify commands already include per-crate `cargo test`, which also materializes the helper; documented ordering, no code change)
- **Files modified:** none
- **Verification:** CLI prints the French success line with exit 0 after the workspace build
- **Committed in:** n/a (workflow ordering; reflected in `d0c85c6` verify)

**2. [Rule 2 - Missing Critical] Catalog success line lives in core, not the binary**
- **Found during:** Task 1 (bin implementation)
- **Issue:** `rust_i18n::t!` in the `archel` bin cannot resolve keys — the `i18n!` backend is declared in `archel-core`; a bin-side lookup would silently miss and break the D-07 single-catalog invariant
- **Fix:** `privileged::dry_run_success_line()` renders the key inside core (same pattern as `version_line()`); the binary only prints the returned string
- **Files modified:** crates/archel-core/src/privileged.rs, crates/archel/src/cmds/mod.rs
- **Verification:** French line renders with the echoed action; diacritics guard still green
- **Committed in:** `d0c85c6` (part of task commit)

**3. [Rule 1 - Bug] Bridge `tick` receiver must be bare `Pin`, not a qualified path**
- **Found during:** Task 3 (first Qt build — cxx-qt codegen stage)
- **Issue:** `fn tick(self: core::pin::Pin<&mut Self>)` fails codegen with `Expected a non mutable T reference! Use either &T or Pin<&mut T>` — the macro matches surface syntax, not semantics
- **Fix:** bare `self: Pin<&mut Self>` exactly as the KDAB book; the `use core::pin::Pin` import proved unused by rustc and was removed
- **Files modified:** crates/archel/src/gui/bridge.rs
- **Verification:** `QMAKE=/usr/bin/qmake6 cargo build -p archel` links Qt 6.11 clean
- **Committed in:** `eecad6c` (part of task commit)

**4. [Rule 2 - Missing Critical] `.qmlls.ini` build output ignored**
- **Found during:** Task 3 (post-commit untracked-file check)
- **Issue:** the Qt build regenerates `crates/archel/.qmlls.ini` (QML language-server config) as an untracked file
- **Fix:** `**/.qmlls.ini` added to `.gitignore` (generated, never hand-edited)
- **Files modified:** .gitignore
- **Verification:** `git status` clean of build output
- **Committed in:** `ce07b08` (chore, part of task 3)

---

**Total deviations:** 4 auto-fixed (1 blocking, 1 bug, 2 missing-critical)
**Impact on plan:** All required for correctness or plan-truth compliance. No scope creep — no domain logic, no Slint pre-wiring, no extra policy actions.

## Issues Encountered

- **pkexec interactive evidence unavailable headless:** `pkexec ./target/debug/archel-privileged dry-run` exits 127 with `Error creating textual authentication agent ... /dev/tty: No such device` — no polkit agent exists in this shell. That 127 lands exactly on the unit-covered Denied path, but the desktop-dialog visual confirm stays a human-check for verify-work (the plan's flagged-unverified PLAT-04 probe).
- **Spike visual confirm is human-side:** offscreen run proves QML loads and the loop ticks with zero errors, but "vertical tabs look right + label updates at 1 Hz" needs eyes on the live Hyprland session (same flagged probe, recorded as coverage D4 `human_judgment: true`).
- **No Slint pivot:** the Qt 6.11 build succeeded on the cargo-only path — the D-13/D-15 hard-incompatibility trigger never fired, so no pivot evidence was recorded (per plan: do NOT pre-wire Slint).
- **Cosmetic warnings only in the Qt build:** Qt 6.11 `qchar.h` SFINAE warning under GCC 16 (upstream header, also present in KDAB's own builds) and the gold-linker deprecation note; zero Qt5 paths in the verbose log.

## Threat Flags

None beyond the plan's threat model. Implemented as specified: exact-match allowlist with reject-all exit 2 + matrix tests (T-1-03); policy is one action, `auth_admin_keep` triple, no `yes` (T-1-04); helper re-validates every invocation and Phase 1 exposes only the harmless dry-run (T-1-05); malformed helper output never becomes ok-state (version gate + `ok` flag checks, tested).

## Known Stubs

None. `run_action_via_pkexec` / `dry_run_via_pkexec` are public, unit-tested, and awaiting their Phase 3 callers — wired API surface, not placeholder output.

## User Setup Required

None - no external service configuration required. (Environment note: Qt spike builds require `QMAKE=/usr/bin/qmake6` in the environment — bare `qmake` is Qt 5.15.19. Rust toolchain remains user-local at `~/.cargo/bin`.)

## Next Phase Readiness

- Ready for Phase 2 (system insight): privilege spawn surface, exit-code mapping, and policy shape are proven; the GUI bridge seam (Rust QObject <-> QML NOTIFY bindings) is proven and reusable for every later tab
- Watch item: Phase 3 wires real privileged actions through `run_action_via_pkexec` + new policy action ids (one per verb, same least-privilege shape)
- Watch item: verify-work should run the two human-checks on the live machine — `pkexec` desktop dialog (cancel + deny paths) and the spike window visual (tabs + ticking label)
- Note: `target/debug/archel` currently on disk is the GUI build (87 MB); `cargo build -p archel --no-default-features` restores the headless binary — both verified green

## Self-Check: PASSED

All 6 created files FOUND on disk; commits `d0c85c6`, `265624c`, `eecad6c`, `ce07b08` verified in git history; headless suite (16 + 5 tests) and full Qt suite green; no unintended deletions (`git diff --diff-filter=D` limited to none in plan range — verified via `--stat`: 723 insertions, 10 deletions, all deletions are the stub/scaffold lines this plan intentionally replaced).

---
*Phase: 01-foundations-safety-rails*
*Completed: 2026-09-30*
