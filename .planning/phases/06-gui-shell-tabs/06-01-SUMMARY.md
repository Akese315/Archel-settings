---
phase: 06-gui-shell-tabs
plan: 01
subsystem: gui
tags: [cxx-qt, qml, shell, single-instance, geometry, qrc, offscreen, archel]

# Dependency graph
requires: []
provides:
  - Real `archel settings` shell (entry restructure, manual tokio runtime, Fusion style, run_settings bootstrap)
  - Main.qml 900x620 shell with 200px rail, 10 French TabButtons in locked order, StackLayout placeholders, Flickable rail
  - single_instance.rs abstract-UDS refusal + settings.already_running catalog key
  - geometry.rs XDG JSON persistence + screens-clamped place() + ShellModel qobject with validateStartup/saveGeometry invokables
  - build.rs org.archel.settings QmlModule (20 QML files + 10 SVG icons via qrc_resources)
affects: [06-03-polling-system-landing, 06-04-read-net-power-audio-bt, 06-05-read-hyprland-pkg, 06-06-modal-infra-pkg-write, 06-07-write-system-domains, 06-08-write-hyprland-domains]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
actuals:
  tokens: 17400
  tasks: 3
  commits: 4

# Tech tracking
tech-stack:
  added: [qt-build-utils (direct; already in lockfile as cxx-qt-build transitive dep, no new supply chain)]
  patterns: [one-bridge-module-per-file, undotted-cxx-namespace, qrc-resources-for-svg, behave-as-worktree-note-for-clones]

key-files:
  created:
    - crates/archel/src/gui/shell.rs
    - crates/archel/src/gui/geometry.rs
    - crates/archel/src/gui/single_instance.rs
    - crates/archel/src/gui/qml/Main.qml
    - crates/archel/src/gui/qml/tabs/LandingTab.qml
    - crates/archel/src/gui/qml/tabs/NetTab.qml
    - crates/archel/src/gui/qml/tabs/PowerTab.qml
    - crates/archel/src/gui/qml/tabs/BtTab.qml
    - crates/archel/src/gui/qml/tabs/AppearanceTab.qml
    - crates/archel/src/gui/qml/tabs/AudioTab.qml
    - crates/archel/src/gui/qml/tabs/PkgTab.qml
    - crates/archel/src/gui/qml/tabs/DisplayTab.qml
    - crates/archel/src/gui/qml/tabs/InputTab.qml
    - crates/archel/src/gui/qml/tabs/SystemTab.qml
    - crates/archel/src/gui/qml/components/ConfirmModal.qml
    - crates/archel/src/gui/qml/components/ErrorModal.qml
    - crates/archel/src/gui/qml/components/StreamModal.qml
    - crates/archel/src/gui/qml/components/CountdownModal.qml
    - crates/archel/src/gui/qml/components/PairingModal.qml
    - crates/archel/src/gui/qml/components/StatField.qml
    - crates/archel/src/gui/qml/components/PollBanner.qml
    - crates/archel/src/gui/qml/components/Sparkline.qml
    - crates/archel/src/gui/qml/components/VpnImportPopup.qml
    - crates/archel/src/gui/qml/images/home.svg
    - crates/archel/src/gui/qml/images/network.svg
    - crates/archel/src/gui/qml/images/power.svg
    - crates/archel/src/gui/qml/images/bluetooth.svg
    - crates/archel/src/gui/qml/images/appearance.svg
    - crates/archel/src/gui/qml/images/sound.svg
    - crates/archel/src/gui/qml/images/apps.svg
    - crates/archel/src/gui/qml/images/display.svg
    - crates/archel/src/gui/qml/images/input.svg
    - crates/archel/src/gui/qml/images/system.svg
  modified:
    - crates/archel/src/main.rs
    - crates/archel/src/gui/mod.rs
    - crates/archel/src/gui/bridge.rs
    - crates/archel/build.rs
    - crates/archel/Cargo.toml
    - crates/archel/src/cmds/mod.rs
    - crates/archel-core/src/i18n.rs
    - crates/archel-core/locales/fr.json
    - Cargo.lock

key-decisions:
  - "ShellModel lives in its own shell.rs bridge file: cxx-qt-build 0.10 accepts one cxx_qt module per file, one extern block per bridge, one qobject per block for Self inlining"
  - "C++ namespace is undotted (archel_settings): dotted namespaces break the cxx_qt::bridge macro parse; the QML module URI stays org.archel.settings (spike already proved they are independent)"
  - "SVG icons ride the qrc through qrc_resources (not qml_files): qmlcachegen would compile .svg entries; qt-build-utils promoted to a direct dep, already in the lockfile"
  - "No tests/gui_shell.rs file: archel is a bin-only crate so integration tests cannot import it; single_instance and geometry tests live as inline unit suites (same coverage, discovered by the same cargo test filters)"
  - "Task 3 finished inline by the orchestrator after the worker died mid-task and its continuation mis-bound to the main checkout (killed pre-commit, main reverted clean)"

patterns-established:
  - "One-bridge-module-per-file: each new QObject gets its own bridge file registered in build.rs .files(), undotted C++ namespace, QML URI from the QmlModule"
  - "Clone-not-worktree for opencode spawns: linked worktrees inherit the registered project's git identity and mis-bind tool-cwd (proven via the project sqlite table + probes); full clones bind correctly"

requirements-completed: [PLAT-03]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "archel settings opens a 900x620 Archel Réglages window with a 200px rail of 10 French TabButtons in locked order over placeholder StackLayout pages (PLAT-03)"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "cargo test -p archel --features gui (126 passed)"
        status: pass
      - kind: manual_procedural
        ref: "QT_QPA_PLATFORM=offscreen timeout 10 ./target/debug/archel settings -> rc 124, zero qrc/cannot-load errors"
        status: pass
      - kind: unit
        ref: "qmllint over all 20 registered QML files, exit 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "Second launch prints the catalog French refusal and exits 0; geometry persists to window-geometry.json and restores clamped against live screens (PLAT-03)"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "crates/archel/src/gui/single_instance.rs#single_instance_second_launch_refuses_and_slot_frees_on_drop"
        status: pass
      - kind: unit
        ref: "crates/archel/src/gui/single_instance.rs#single_instance_refusal_line_comes_from_the_catalog"
        status: pass
      - kind: unit
        ref: "crates/archel/src/gui/geometry.rs#geometry_round_trip_restores_the_saved_frame"
        status: pass
      - kind: unit
        ref: "crates/archel/src/gui/geometry.rs#geometry_off_screen_rects_are_clamped_or_recentred"
        status: pass
      - kind: manual_procedural
        ref: "live double launch offscreen: second prints 'Archel Réglages est déjà ouvert.' rc 0"
        status: pass
    human_judgment: false
  - id: D3
    description: "Qt stays behind the gui feature: headless build compiles no Qt and the full headless suite is green (PLAT-03)"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "cargo test -p archel --no-default-features (120 passed)"
        status: pass
      - kind: unit
        ref: "cargo test -p archel-core --no-default-features (269 passed)"
        status: pass
    human_judgment: false

# Metrics
duration: 150min
completed: 2026-10-09
status: complete
---

# Phase 06 Plan 01 Summary

**Real settings shell window with rail, single-instance refusal, and geometry persistence — tasks 1-2 by worker, task 3 finished inline after worker death.**

## Performance

- **Duration:** ~150min (worker ~100min for tasks 1-2 + partial 3, inline ~50min to finish task 3)
- **Started:** 2026-10-09T11:56Z
- **Completed:** 2026-10-09T14:20Z
- **Tasks:** 3 completed
- **Files modified:** 43 (26 created QML/SVG/Rust, 9 modified Rust/toml/lock)

## Accomplishments

- Entry rework: sync `main()` branches `Command::Settings` into `gui::run_settings()` before any tokio runtime; runtime owned by a gui `OnceLock`; spike retired (Spike.qml deleted, spike module unregistered)
- Main.qml shell: 900x620 (min 720x480), 10 icon+label TabButtons in locked French order driving a StackLayout of placeholder tabs, rail in its own Flickable, all 20 QML files + 10 SVGs registered in build.rs
- Single-instance refusal: one abstract-UDS `bind()` on `\0archel-settings-v1`, refusal line from the `settings.already_running` catalog key, listener owned by the run_settings scope; live double-launch verified (rc 0)
- Geometry: XDG JSON load/save (atomic temp+rename, failures swallowed), `place()` clamps against live screens with centered fallback, ShellModel publishes launch frame via `validateStartup`/`saveGeometry` invokables wired to `onCompleted`/`onClosing`

## Task Commits

Each task was committed atomically (first two in the worker clone, merged as 1193ef2):

1. **Task 1: Entry rework + shell window + QML skeleton** - `21c955b` (feat)
2. **Task 2: Second-instance refusal** - `7d4aeca` (feat)
3. **Task 3: Geometry persistence + ShellModel + rail icons** - `3741ef6` (feat)

**Merge:** `1193ef2` (merge wave-1 clone branch into master)

## Files Created/Modified

- `crates/archel/src/gui/shell.rs` - ShellModel bridge (split file, see deviations)
- `crates/archel/src/gui/geometry.rs` - Geometry/ScreenRect/load/save/place + inline tests
- `crates/archel/src/gui/single_instance.rs` - abstract-UDS acquire() + inline tests
- `crates/archel/src/gui/qml/Main.qml` - shell window, rail, StackLayout, geometry bindings
- 10 tab placeholders + 9 component placeholders + 10 vendored MIT SVGs
- `build.rs`/`Cargo.toml`/`main.rs`/`mod.rs`/`bridge.rs`/`cmds/mod.rs`/`i18n.rs`/`fr.json`/`Cargo.lock` - wiring, registration, catalog key

## Decisions Made

- One bridge module per file, undotted C++ namespace (toolchain-forced; verified against cxx-qt-gen 0.10 sources, not guessed)
- `qt-build-utils` as a direct dependency: already in Cargo.lock as a cxx-qt-build transitive dep, so no new supply-chain review surface
- Inline unit tests instead of `tests/gui_shell.rs`: bin-only crate cannot host integration tests; same filters (`single_instance`, `geometry`) discover them
- Task 3 taken over inline: worker died mid-task (process death), continuation spawn mis-bound to the main checkout and was killed pre-commit; main reverted byte-clean

## Deviations from Plan

### Auto-fixed Issues

**1. ShellModel split into shell.rs (toolchain constraint)**

- **Found during:** Task 3 (build failure after adding the second QObject)
- **Issue:** cxx-qt-build 0.10 accepts one `cxx_qt` module per file; one `extern RustQt` block per bridge; one qobject per block for `Self` inlining. Two qobjects in one block reject `Self`; two blocks are rejected by the cxx parser. Verified in cxx-qt-gen sources (self_inlining.rs, types.rs).
- **Fix:** New `gui/shell.rs` bridge file with the ShellModel module, registered in build.rs `.files()` and `mod.rs`
- **Files modified:** `crates/archel/src/gui/shell.rs` (new), `bridge.rs`, `mod.rs`, `build.rs`
- **Verification:** `cargo build -p archel --features gui` green
- **Committed in:** `3741ef6` (part of task commit)

**2. Undotted C++ namespace (macro parse failure)**

- **Found during:** Task 3 (8x `error[cxx]: unexpected token` on the new bridge)
- **Issue:** `#[namespace = "org.archel.settings"]` breaks the `cxx_qt::bridge` macro parse; bisected to the dots (undotted parses). QML module URI is independent (spike precedent).
- **Fix:** `#[namespace = "archel_settings"]`; QML side unchanged (`import org.archel.settings`)
- **Files modified:** `crates/archel/src/gui/shell.rs`
- **Verification:** build green; offscreen smoke alive with zero qrc errors (proves QML registration resolves)
- **Committed in:** `3741ef6` (part of task commit)

**3. Missing qinvokable declarations + QML imports/screens source (runtime defects)**

- **Found during:** Task 3 review (worker WIP)
- **Issue:** `validate_startup`/`save_geometry` had no `#[qinvokable]` bridge entries (QML calls would fail at runtime); Main.qml lacked `import QtQuick.Window` + `import org.archel.settings` and read the nonexistent `window.screens`
- **Fix:** Added both invokable declarations (cxx_name camelCase), both imports, `Qt.application.screens`
- **Files modified:** `shell.rs`, `Main.qml`
- **Verification:** offscreen smoke + qmllint clean
- **Committed in:** `3741ef6` (part of task commit)

**4. SVG resource never attached (dead code + missing icons)**

- **Found during:** Task 3 review (build warnings: unused `QResources`, unused `images`)
- **Issue:** Worker built the `QResource` but never passed it to the builder — icons would 404 at runtime
- **Fix:** `.qrc_resources(QResources::new().resource(images))`; `qml_files` untouched (.svg must stay out of qmlcachegen)
- **Files modified:** `crates/archel/build.rs`
- **Verification:** build warnings gone (only Qt system-header warning remains); smoke log has no image-load errors
- **Committed in:** `3741ef6` (part of task commit)

**5. Em-dashes in new .rs files (ASCII gate)**

- **Found during:** Task 3 verification (grep gate over new sources)
- **Issue:** Worker prose used em-dashes in comments across 4 files
- **Fix:** Replaced with `-`
- **Files modified:** `geometry.rs`, `shell.rs`, `mod.rs`, `bridge.rs`
- **Verification:** ASCII gate clean (.qml French strings correctly remain)
- **Committed in:** `3741ef6` (part of task commit)

---

**Total deviations:** 5 auto-fixed (2 toolchain constraints, 2 runtime defects, 1 gate wording)
**Impact on plan:** All required for a compiling, smoke-green shell. One new file beyond `files_modified` (shell.rs, forced). One artifact path changed (inline tests instead of tests/gui_shell.rs, forced by bin-only crate). No scope creep.

## Issues Encountered

- Worker process died mid-task-3 after ~100 min (process death, likely transport kill); no SUMMARY, 2 commits + uncommitted WIP preserved in the clone. Continuation spawn mis-bound tool-cwd to the main checkout (edited 3 main files, killed pre-commit, reverted clean). Root cause identified: opencode binds sessions by registered-project git identity (sqlite `project` table) — linked worktrees collide, full clones bind correctly. Future spawns: use clones, and verify binding from the first tool call before leaving them alone.
- Model backend (thor.sealix.eu) flapped all session (Transport socket-closed); probes succeed ~1 in 3. Worker progress survived blips (retries), but spawn/death classification is noisy.
- Full core suite showed 2 flaky live-network failures once post-merge (packages live AUR tests); green on immediate rerun (269 passed). Not plan-related.
- `cargo clippy -p archel-core --all-features -D warnings` remains red on 29 pre-existing lints outside this plan (carried from 06-02).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Wave 2 (06-03 polling + System/Landing tabs) is unblocked: shell chrome, ShellModel pattern, and registered QML skeletons are in place; 06-03 copies the model pattern established here
- SpikeModel stays compiling in bridge.rs until plan 03 removes it (plan-stated)
- Spawning guidance for the rest of the phase: full-clone checkouts (not linked worktrees), verify first-tool-call binding, expect backend flaps

---
*Phase: 06-gui-shell-tabs*
*Completed: 2026-10-09*
