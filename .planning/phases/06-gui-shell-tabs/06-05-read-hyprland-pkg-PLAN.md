---
phase: "06"
plan: "05"
type: execute
wave: 4
depends_on: ["06-03", "06-04"]
files_modified:
  - crates/archel/src/gui/models/appearance.rs
  - crates/archel/src/gui/models/display.rs
  - crates/archel/src/gui/models/input.rs
  - crates/archel/src/gui/models/pkg.rs
  - crates/archel/src/gui/models/mod.rs
  - crates/archel/src/gui/mod.rs
  - crates/archel/src/gui/qml/tabs/AppearanceTab.qml
  - crates/archel/src/gui/qml/tabs/DisplayTab.qml
  - crates/archel/src/gui/qml/tabs/InputTab.qml
  - crates/archel/src/gui/qml/tabs/PkgTab.qml
autonomous: true
requirements: [PLAT-03]
estimate:
  tokens: 35000
  raw_tokens: 25000
  tasks: 2
  confidence: low
must_haves:
  truths:
    - "Appearance tab reads blur/opacity/wallpaper/font through the same core reads archel appearance uses (read_blur, current_snippet_state, read_wallpaper_at, font read) — controls reflect reality on every tick"
    - "Display tab lists Hyprland monitors via the same path as archel display run_monitors; Input tab lists input settings via hyprland::input::read_input and custom binds via the binds parse helpers — the same rows input_cmds keybinds_list renders"
    - "Packages tab lists installed packages, live search results, and a package card through packages::list_installed/search/package_card (the run_list/run_search/run_info twins)"
    - "Package search shows the exact 'Aucun paquet trouve' refinement copy and the keybind list its 'Aucun raccourci personnalise' + Ajouter affordance copy (UI-SPEC empty rows; Ajouter wired in plan 08)"
    - "All four tabs use the 06-03 poll path with hidden-tab Timer stopped (D-03) and ellipsis-until-first-tick loading"
  artifacts:
    - crates/archel/src/gui/models/appearance.rs
    - crates/archel/src/gui/models/display.rs
    - crates/archel/src/gui/models/input.rs
    - crates/archel/src/gui/models/pkg.rs
    - "AppearanceTab/DisplayTab/InputTab/PkgTab .qml filled from placeholders"
  key_links:
    - "PkgModel.search(query) -> packages::search(query) on the blocking pool with per-query sequence tag (typing must not flash stale hits)"
    - "InputModel binds <- binds::parse_binds_json/parse_snippet_binds + keybind_rows, identical to input_cmds.rs:20 run_keybinds_list"
    - "DisplayModel monitors <- the same monitors read display_cmds.rs:22 calls (never a raw hyprctl JSON re-parse in GUI code)"
prohibitions:
  - "No hyprland.conf parsing or hyprctl dispatch in GUI code — every appearance/display/input read and (later) write goes through the archel-core hyprland module (PLAT-03)"
  - "The Ajouter button on the keybind empty state is an affordance only at this stage — editor wiring is plan 08"
---

<objective>
Deliver the remaining four read tabs — Appearance, Display, Input, Packages — completing navigable coverage of all ten tabs over CLI-proven core entry points (success criteria 1 and 4).

Purpose: last read-half of the phase; also builds PkgModel whose write side plan 06 extends.
Output: four more view-models + four populated QML pages; every tab now a live thin view.
</objective>

<execution_context>
@/home/axel/.config/opencode/gsd-core/workflows/execute-plan.md
@/home/axel/.config/opencode/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/phases/06-gui-shell-tabs/06-CONTEXT.md
@.planning/phases/06-gui-shell-tabs/06-RESEARCH.md
@.planning/phases/06-gui-shell-tabs/06-PATTERNS.md
@.planning/phases/06-gui-shell-tabs/06-UI-SPEC.md
@.planning/phases/06-gui-shell-tabs/06-03-SUMMARY.md
@crates/archel/src/cmds/pkg_cmds.rs
@crates/archel/src/cmds/input_cmds.rs
@crates/archel-core/src/hyprland/appearance.rs
</context>

<tasks>

<task type="auto" id="06-05-T1">
  <name>Task 1: AppearanceModel + DisplayModel + InputModel (+ tabs)</name>
  <read_first>crates/archel-core/src/hyprland/appearance.rs (read_blur:205, current_snippet_state:94), wallpaper.rs (read_wallpaper_at:297), font.rs (its read fn beside apply_font_with:293), input.rs (read_input:324, InputState fields), monitors.rs (the read used by display_cmds run_monitors), binds.rs (parse_binds_json:304, parse_snippet_binds:540, keybind_rows:584), crates/archel/src/cmds/{appearance_cmds.rs,input_cmds.rs:20,display_cmds.rs:22} (CLI twins), 06-UI-SPEC.md (keybind empty row, ComboBox rows, Slider row)</read_first>
  <files>crates/archel/src/gui/models/{appearance,display,input}.rs, crates/archel/src/gui/models/mod.rs, crates/archel/src/gui/mod.rs, crates/archel/src/gui/qml/tabs/{AppearanceTab,DisplayTab,InputTab}.qml</files>
  <action>Three models on the 06-03 pattern (spawn_blocking + queue; these core reads are sync). AppearanceModel: tick() -> read_blur + current_snippet_state + read_wallpaper_at + the font read; props: blur on/off, opacity double, wallpaper path, font family/size; controls in AppearanceTab.qml (Switch, Slider at 0..1 step 0.01, path Label, font fields) reflect props each tick but perform NO writes yet (plan 08 owns writes — keep invokable slots absent, do not stub them). DisplayModel: tick() -> the monitors read display_cmds.rs:22 uses; JSON prop of per-monitor rows (name, resolution, refresh, scale); DisplayTab.qml: monitor GroupBox rows with read-only ComboBox placeholders for resolution/refresh (model populated from the row's option lists the core read exposes, disabled until plan 08). InputModel: tick() -> read_input + the binds list (parse_binds_json + parse_snippet_binds, rendered via keybind_rows like run_keybinds_list); props: input JSON (mouse speed etc.) + binds JSON; InputTab.qml: Slider/ComboBox placeholders reflecting values read-only, binds ListView (render_mods + summarize_bind strings as rows — no client-side reformatting), empty-state copy 'Aucun raccourci personnalise' + Ajouter Button (affordance only). ElideMiddle + ToolTip on long bind summaries. Register all three; inline tests per model: Err -> N/A + banner, binds JSON shape.</action>
  <verify>
    <automated>QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui 'appearance_model::' && QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui 'display_model::' && QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui 'input_model::'</automated>
    <fails_when>any of the three model test modules fails; N/A/banner contract broken</fails_when>
    <automated>qmllint crates/archel/src/gui/qml/tabs/AppearanceTab.qml crates/archel/src/gui/qml/tabs/DisplayTab.qml crates/archel/src/gui/qml/tabs/InputTab.qml</automated>
    <fails_when>qmllint reports errors on any of the three tabs</fails_when>
  </verify>
  <acceptance_criteria>
  - All three tabs render live Hyprland state through core reads only; controls disabled/pending until plan 08
  - Keybind rows use core's render_mods/summarize_bind strings verbatim (no GUI reformatting)
  - Empty-state copy present with inert Ajouter affordance
  </acceptance_criteria>
  <done>Hyprland-domain reads done; only writes remain for these three tabs.</done>
</task>

<task type="auto" id="06-05-T2">
  <name>Task 2: PkgModel reads + Packages tab (list/search/card)</name>
  <read_first>crates/archel-core/src/packages.rs (list_installed:37, search:87 incl. sort_hits, package_card:246, PackageCard/PackageInfo fields, card_labels), crates/archel/src/cmds/pkg_cmds.rs:14-110 (run_list/run_search/run_info twins incl. render_search_table/render_info_table row choices), 06-UI-SPEC.md (package empty row, ListView/ItemDelegate row, long-text backstop)</read_first>
  <files>crates/archel/src/gui/models/pkg.rs, crates/archel/src/gui/models/mod.rs, crates/archel/src/gui/mod.rs, crates/archel/src/gui/qml/tabs/PkgTab.qml</files>
  <action>PkgModel (this plan creates the read half; plan 06 adds write invokables to this same file): tick() -> spawn_blocking list_installed() (heavier — sequence tag so a slow list never overwrites a newer one); search(query) invokable -> spawn_blocking packages::search with per-query sequence tag (typing fast => stale hits dropped); select(name) -> spawn_blocking package_card. Props: installed JSON (name/version/size/description rows as render_list_table chooses), search JSON (sorted via core sort_hits — no GUI sorting), card JSON (fields per card_labels order), busy flag for the search field. PkgTab.qml: TextField search (bound to search() on accepted/debounced text), installed ListView with ElideMiddle + ToolTip names, 'Aucun paquet trouve' refinement copy when a search yields zero, detail Pane rendering the card labels/values via StatField pairs. Install/Remove/Ma a jour buttons are NOT added here (plan 06). Register; inline tests: sequence drop on rapid search, zero-hit flag, card Err -> N/A.</action>
  <verify>
    <automated>QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui 'pkg_model::'</automated>
    <fails_when>pkg model tests fail (stale search hits win, zero-hit copy key missing)</fails_when>
    <automated>qmllint crates/archel/src/gui/qml/tabs/PkgTab.qml && QT_QPA_PLATFORM=offscreen timeout 10 ./target/debug/archel settings &lt;/dev/null >/tmp/archel-pkg.log 2>&1; rc=$?; test $rc -eq 124 && ! grep -qiE 'qml:|is not loaded' /tmp/archel-pkg.log</automated>
    <fails_when>qmllint errors or offscreen smoke dies/logs binding errors</fails_when>
  </verify>
  <acceptance_criteria>
  - Installed list, live search, and package card all render from core reads; rapid typing never flashes stale hits
  - Zero-hit refinement copy renders; card follows core's label set (no GUI-side field invention)
  - All ten tabs now navigate with real content or awaiting-write placeholders
  </acceptance_criteria>
  <done>Packages reads live; PLAT-03 read surface complete for every tab.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| pacman/alpm package metadata -> QML lists | Package names/descriptions from the mirror DB render as text |
| hyprland.conf-derived values -> controls | Config strings flow into displayed fields |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-06-25 | Tampering | package search results (repo-controlled strings) rendered in delegates | low | accept | Plain Text + ElideMiddle, no rich text; metadata already rendered by the CLI; user cannot install from this plan (writes are plan 06 with confirm + AUR gate) |
| T-06-26 | Spoofing | stale search hits misleading the user into picking an outdated row | medium | mitigate | Per-query sequence tag (tested); selection operates on the row object rendered from the accepted tag, not index math |
| T-06-27 | Information Disclosure | installed-package inventory exposed to QML | low | accept | Same data the CLI already prints locally; single-user desktop, no network exposure |
| T-06-SC | Tampering | crate installs | high | mitigate | No new packages (RESEARCH audit: none) |
</threat_model>

<verification>
- `cargo test -p archel-core --no-default-features && cargo test -p archel --no-default-features && QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui` + qmllint + offscreen smoke (VALIDATION.md)
</verification>

<success_criteria>
Ten navigable tabs with live reads; success criterion 1 satisfied for the read half; every entry point traces to a cmds twin.
</success_criteria>

## Artifacts this phase produces (this plan)

- AppearanceModel, DisplayModel, InputModel, PkgModel (read half), populated AppearanceTab/DisplayTab/InputTab/PkgTab; keybind rows and package card driven purely by core label helpers.

<output>
When all tasks pass, create `.planning/phases/06-gui-shell-tabs/06-05-SUMMARY.md`.
</output>
