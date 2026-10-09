---
phase: 06-gui-shell-tabs
plan: 09
type: execute
wave: 7
depends_on: [06-07, 06-08]
files_modified:
  - justfile
  - crates/archel/tests/gui_thin_view.rs
autonomous: true
requirements: [PLAT-03]
tags: [validation, harness, coverage-audit, ci]

estimate:
  tokens: 42000
  raw_tokens: 30000
  tasks: 2
  confidence: low

must_haves:
  truths:
    - "`just gui-check` runs the full GUI verification in one command: QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui, a qmllint sweep over every crates/archel/src/gui/qml/**/*.qml with zero errors, and the offscreen smoke (QT_QPA_PLATFORM=offscreen timeout 10 ./target/debug/archel settings, expecting rc=124 because the app should stay alive until the timeout kills it)"
    - "The thin-view audit test proves each of the 10 tabs exposes the same capabilities as its CLI twin: every user-facing entry point in crates/archel/src/cmds/*_cmds.rs maps to an invokable on the tab's model (curated 1:1 map in the test), and no tab implements a system mutation that isn't backed by an archel-core call"
    - "The audit runs in plain CI with no Qt runtime (#[cfg] free — pure source-deparsing/metadata assertions), so it gates the phase-goal criterion 'each tab a thin view over CLI-proven core'"
  artifacts:
    - "justfile at repo root with gui-check (and a fast gui-check-fast alias skipping qmllint)"
    - "crates/archel/tests/gui_thin_view.rs with the curated tab↔cmd map table and drift assertions"
  key_links:
    - "justfile gui-check -> cargo build -p archel --features gui (binary the smoke launches) -> qmllint-qt6 (or qt6-declarative's qmllint via which-detection) -> target/debug/archel"
    - "gui_thin_view.rs -> parses crates/archel/src/cmds/*.rs invokable-equivalent fn names vs crates/archel/src/gui/models/*.rs invokable names via the curated map"
  prohibitions:
    - "crates/archel/src/gui/** must contain no subprocess or IPC primitives (std::process, duct, hyprland IPC, zbus) — the thin-view audit enforces this and must not whitelist exceptions"
    - "The thin-view audit must not rubber-stamp: every cmds command is annotated // gui: <model> or // cli-only: <reason> and the test fails on unannotated commands"
    - "justfile recipes must be fixed literal command lines — no argument interpolation, no network fetch"
    - "This plan must not modify build.rs or the 06-01 QmlModule registration list"
---

<objective>
Land the two verification backbones the phase promised but no earlier plan
owned: the single-command GUI check (VALIDATION.md Wave-0 item) and the
PLAT-03 thin-view coverage-audit test.

Purpose: makes the phase goal machine-checkable — `just gui-check` green +
audit green is the executable definition of "modern vertical-tab GUI presenting
every proven capability" for CI and for /gsd-verify-work.
Output: justfile + gui_thin_view.rs.
</objective>

<context_read_first>
- .planning/phases/06-gui-shell-tabs/06-VALIDATION.md (Wave 0: gui-check script item)
- .planning/phases/06-gui-shell-tabs/06-RESEARCH.md (offscreen smoke convention,
  Tab↔core map lines 266+ — the map this test encodes)
- crates/archel/src/cmds/ (the 10 *_cmds.rs twins defining the capability surface)
- crates/archel/src/gui/models/ (models produced by plans 03-08)
</context_read_first>

<tasks>

<task type="auto">
  <name>Task 1: root justfile with gui-check recipe</name>
  <files>justfile</files>
  <action>
    Create justfile (none exists at repo root — verify first). Recipes:
    test (cargo test -p archel-core --no-default-features && cargo test -p archel
    --no-default-features), gui-build (QMAKE=/usr/bin/qmake6 cargo build -p
    archel --features gui), lint-qml (loop crates/archel/src/gui/qml/**/*.qml through
    the qmllint found via `which qmllint-qt6 || which qmllint`; fail on errors,
    ignore unused-import warnings with -I flag hygiene), smoke (build then
    QT_QPA_PLATFORM=offscreen timeout 10 ./target/debug/archel settings; accept
    rc 124, fail otherwise), gui-check = gui-build + lint-qml + smoke +
    cargo test -p archel --features gui. Pin QMAKE in the file itself so no
    caller forgets (06-01 build.rs requirement). If just is not installed on
    the box, still ship the justfile and mirror the recipes in a comment header
    as plain shell lines (executor checks with `command -v just`).
  </action>
  <verify>
    <automated>if command -v just >/dev/null 2>&1; then just gui-check; else QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui && QMAKE=/usr/bin/qmake6 cargo build -p archel --features gui && for f in $(find crates/archel/src/gui/qml -name '*.qml'); do qmllint-qt6 "$f" >/dev/null 2>&1 || qmllint "$f" >/dev/null || exit 1; done && { QT_QPA_PLATFORM=offscreen timeout 10 ./target/debug/archel settings; [ "$?" -eq 124 ]; }; fi</automated>
    <fails_when>any recipe step fails on the live box (compile error, qmllint error, or smoke exiting != 124 — an early crash means a tab regressed)</fails_when>
  </verify>
  <acceptance_criteria>
    From a clean checkout `just gui-check` is the only command needed to verify
    the GUI crate; documented in the justfile comment header.
  </acceptance_criteria>
  <done>All four steps green on the dev box; committed.</done>
</task>

<task type="auto">
  <name>Task 2: thin-view coverage-audit test (PLAT-03 gate)</name>
  <files>crates/archel/tests/gui_thin_view.rs</files>
  <action>
    Integration test in crates/archel (no Qt needed — it reads source text, no
    runtime). Curated map: for each of the 10 tabs, a const list pairing
    cmds::<domain>_cmds user-facing run_* functions with the model invokables
    that must exist (e.g. net_cmds run_vpn_import <-> NetModel.vpn_import_start;
    pkg_cmds begin_install_session <-> PkgModel.install_start; binds path <->
    InputModel.add_keybind; monitors <-> DisplayModel.plan_change; landing <->
    LandingModel refresh; system <-> SystemModel poll). Assertions:
    (a) every mapped model invokable name appears in its gui/models/*.rs bridge
    mod (string scan of the bridge source — cxx-qt external csbridge fn naming);
    (b) no gui/models/*.rs file contains a direct system-mutation literal
    outside core calls: forbid raw Command::new(\"pacman\"|\"yay\"|\"nmcli\"|
    \"wpctl\") and Hyprland IPC verbs in gui/ (subprocess lives in core/cmds);
    (c) the map is exhaustive: every pub run_*/begin_* fn in cmds/*_cmds.rs
    annotated // gui: appears in some tab list, // cli-only: appears in none
    (annotations added in this task where missing — annotation-only edits are
    allowed in cmds/ comment lines, no behaviour change).
  </action>
  <verify>
    <automated>cargo test -p archel --no-default-features --test gui_thin_view</automated>
    <fails_when>audit red: a CLI capability lacks its GUI invokable, a forbidden subprocess/IPC literal appears in gui sources, or an unannotated pub run_* fn in cmds is discovered (drift gate)</fails_when>
  </verify>
  <acceptance_criteria>
    Test documents the map as data (const arrays) so future tabs fail CI until
    mapped — this is the thin-view prohibition owner for the whole phase.
  </acceptance_criteria>
  <done>Audit green; `just gui-check` (Task 1) includes cargo test which runs it.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| CI/dev box -> justfile recipes | commands executed on developer machine |
| test source-scanning -> repo sources | audit parses repo text (trusted, in-repo) |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-06-37 | Tampering | justfile recipes executed by devs/CI | low | mitigate | recipes are fixed literal command lines (no interpolation of args); review at commit; no network fetch in any recipe |
| T-06-SC | Tampering | npm/pip/cargo installs | high | mitigate | no new packages (just is an existing system tool, checked via command -v, not installed by the plan); T-06-SC retained |
</threat_model>

<artifacts_produced>
## Artifacts this phase produces

(New in this plan.)

- `justfile` at repo root: `test`, `gui-build`, `lint-qml`, `smoke`,
  `gui-check` recipes with the QMAKE pin baked in.
- `crates/archel/tests/gui_thin_view.rs`: curated tab↔CLI capability map,
  forbidden-subprocess-in-gui gate, and exhaustive-annotation drift check —
  the executable PLAT-03 thin-view guarantee.
- `// gui:` / `// cli-only:` annotations in `crates/archel/src/cmds/*_cmds.rs`
  (comment-only) powering the drift check.
</artifacts_produced>

<verification>
- `just gui-check` green end-to-end.
- `cargo test -p archel --no-default-features` green including the new audit.
- Offscreen smoke rc=124 with all 10 tabs registered (Landing + 9 domains navigable).
</verification>

<success_criteria>
The phase goal is machine-verifiable: build + lint + smoke + thin-view audit all
green in one command, giving /gsd-verify-work and CI the PLAT-03 evidence trail.
</success_criteria>

<output>
Create `.planning/phases/06-gui-shell-tabs/06-09-SUMMARY.md` when done
</output>
