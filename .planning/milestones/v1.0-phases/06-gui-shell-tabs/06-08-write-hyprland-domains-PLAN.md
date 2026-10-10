---
phase: 06-gui-shell-tabs
plan: 08
type: execute
wave: 6
depends_on: [06-05, 06-06]
files_modified:
  - crates/archel/src/gui/models/appearance.rs
  - crates/archel/src/gui/models/input.rs
  - crates/archel/src/gui/models/display.rs
  - crates/archel/src/gui/qml/tabs/AppearanceTab.qml
  - crates/archel/src/gui/qml/tabs/InputTab.qml
  - crates/archel/src/gui/qml/tabs/DisplayTab.qml
  - crates/archel/src/gui/qml/components/CountdownModal.qml
autonomous: true
requirements: [PLAT-03]
tags: [write, hyprland, appearance, input, display, countdown]

estimate:
  tokens: 84000
  raw_tokens: 60000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "Appearance writes apply blur, opacity, wallpaper and font through the core apply_*_with seams (validate -> Hyprland IPC -> line-preserving conf write), each behind the CLI-proven question in ConfirmModal, with per-control spinner while in flight (D-15)"
    - "Input writes apply sensitivity/accelerate via apply_input_with and add/remove keybinds via plan_bind/add_bind_line/remove_bind_line; removal is destructive-toned (red) and always confirmed with the core confirm_bind_remove_prompt question"
    - "Display changes are planned with plan_monitor_change, previewed in ConfirmModal, then applied via apply_monitor_plan which owns the 15 s countdown-to-revert (D-16); CountdownModal shows the live countdown with Conserver (default) / Annuler buttons per D-16 and is instantiated inside DisplayTab.qml only"
    - "Long values (keybind combos, conf option values, monitor names) elide with ToolTip full value (UI Considerations long-text-keybind owned here)"
  artifacts:
    - "Write invokables: AppearanceModel.set_blur/set_opacity/set_wallpaper/set_font, InputModel.set_sensitivity/set_accelerate/add_keybind/remove_keybind, DisplayModel.plan_change/keep_change/revert_change"
    - "CountdownModal real implementation (06-01 placeholder body replaced; its api block defined by this plan following the 06-06 modal convention) instantiated in DisplayTab.qml"
  key_links:
    - "AppearanceModel writes -> hyprland::appearance::apply_blur_with / validate_opacity+apply path / wallpaper::apply_wallpaper_with / font::apply_font_with (DI-with-hyprctl seams so unit tests run headless)"
    - "InputModel.add_keybind -> binds::plan_bind -> collision_hint -> confirm_bind_prompt question -> add_bind_line; remove -> confirm_bind_remove_prompt -> remove_bind_line"
    - "DisplayModel.plan_change -> monitors::plan_monitor_change -> ConfirmModal -> monitors::apply_monitor_plan (15 s timer inside core) -> CountdownModal countdown + keep -> keep-monitor keyword ack"
  prohibitions:
    - "No GUI code path may call Hyprland set_monitor / dispatch directly — display mutation flows only through plan_monitor_change -> apply_monitor_plan (RESEARCH Pitfall 5)"
    - "No hyprland.conf write may bypass the core line editor (apply_*_with / apply_input_with) — the GUI never edits conf text itself"
    - "CountdownModal may not be instantiated anywhere except inside DisplayTab.qml (instantiation-count grep == 1)"
    - "This plan must not touch build.rs QmlModule registration (all files it fills were registered by 06-01)"
---

<objective>
Complete write coverage: the three Hyprland-integrated tabs (Appearance, Input,
Display) mutate configuration through the CLI-proven core seams, including the
watch-the-screen countdown revert flow.

Purpose: last of the write plans; after this every tab is a thin writable view
over archel-core, satisfying the phase goal's "each tab thin view over
CLI-proven core" criterion.
Output: write-capable Appearance/Input/Display tabs + CountdownModal.
</objective>

<context_read_first>
- .planning/phases/06-gui-shell-tabs/06-CONTEXT.md (D-12, D-15, D-16)
- .planning/phases/06-gui-shell-tabs/06-RESEARCH.md (Pitfall 5 never bypass
  plan_monitor_change; hyprland conf line-preserving editing; Tab↔core map)
- crates/archel/src/cmds/{appearance_cmds,input_cmds,display_cmds}.rs (CLI twins)
- crates/archel-core/src/hyprland/{appearance,wallpaper,font,input,monitors,binds}.rs
  (apply seams + confirm/collision helpers listed in key_links)
- .planning/phases/06-gui-shell-tabs/06-06-SUMMARY.md (modal API)
</context_read_first>

<tasks>

<task type="auto">
  <name>Task 1: Appearance writes (blur / opacity / wallpaper / font)</name>
  <files>crates/archel/src/gui/models/appearance.rs, crates/archel/src/gui/qml/tabs/AppearanceTab.qml</files>
  <action>
    Four write invokables on AppearanceModel (06-05 read model), each:
    validate (opacity via validate_opacity, wallpaper path via the read path in
    wallpaper.rs) -> ConfirmModal question from the core prompt helper ->
    spawn_blocking apply_*_with on the Hyprland connection held by the model
    (same DI seam the cmds twins use so tests inject a fake hyprctl).
    Blur: Switch + sliders for passes/size driven by read_blur values.
    Opacity: slider (window/insert values) with % labels.
    Wallpaper: path TextBox + FileDialog (typed-path fallback like VpnImportPopup),
    preview Image when the path exists.
    Font: family + size fields driven by font read values.
    Spinner + disabled controls per D-15; failure -> ErrorModal
    (ArchelError::message()).
  </action>
  <verify>
    <automated>cargo test -p archel-core --no-default-features hyprland && cargo test -p archel --no-default-features</automated>
    <fails_when>any apply unit test red (invalid opacity rejected pre-IPC; conf written only after IPC success; decline skips both)</fails_when>
  </verify>
  <acceptance_criteria>
    Snapshot-style tests on the conf transformation for each of the four writes
    (insta per STACK guidance where the core helper exposes the rendered conf).
  </acceptance_criteria>
  <done>Four appearance writes round-trip against fake-hyprctl tests; committed.</done>
</task>

<task type="auto">
  <name>Task 2: Input writes (sensitivity / accel / keybind add + remove)</name>
  <files>crates/archel/src/gui/models/input.rs, crates/archel/src/gui/qml/tabs/InputTab.qml</files>
  <action>
    InputModel: set_sensitivity/set_accelerate via input::apply_input_with
    (pointer settings read by 06-05). Keybind list: add row = key combo
    TextBox + action TextBox -> binds::plan_bind -> collision_hint shown inline
    as warning before confirm -> confirm_bind_prompt question in ConfirmModal ->
    add_bind_line. Remove: trash Button per row, destructive style (window
    Button destructive variant red), confirm_bind_remove_prompt question.
    Rows elide long combos/actions with ToolTip (long-text-keybind backstop
    owned here). Invalid combo -> French validation message inline.
  </action>
  <verify>
    <automated>cargo test -p archel-core --no-default-features binds && cargo test -p archel --no-default-features</automated>
    <fails_when>bind add/remove tests red (collision warning surfaced before confirm; remove always confirmed; no conf write on decline)</fails_when>
  </verify>
  <acceptance_criteria>
    Tests: plan_bind output drives row fields; duplicate bind surfaces
    collision_hint; removal path calls confirm helper before remove_bind_line.
  </acceptance_criteria>
  <done>Keybind CRUD + pointer settings write through core with confirmation gates.</done>
</task>

<task type="auto">
  <name>Task 3: Display plan->apply->countdown (CountdownModal)</name>
  <files>crates/archel/src/gui/models/display.rs, crates/archel/src/gui/qml/tabs/DisplayTab.qml, crates/archel/src/gui/qml/components/CountdownModal.qml</files>
  <action>
    DisplayModel.plan_change(monitor, resolution, refresh, scale):
    monitors::plan_monitor_change -> preview summary (current -> proposed) in
    ConfirmModal -> on confirm spawn_blocking monitors::apply_monitor_plan
    through the model's injected Hyprland seam. apply_monitor_plan owns the
    15 s auto-revert timer (D-16) — GUI only displays it: CountdownModal shows
    monitor name + the UI-SPEC copy "Conserver ces paramètres ? Annulation
    automatique dans {s} s." ticking each second, primary Button "Conserver"
    (default, D-16) calling keep -> the core keep ack, secondary Button
    "Annuler" forcing immediate revert. Instantiated
    inside DisplayTab.qml (its only host — instantiation-count grep == 1).
    NEVER call set_monitor/dispatch directly (Pitfall 5) — grep display.rs
    model for forbidden IPC verbs as part of verify.
  </action>
  <verify>
    <automated>cargo test -p archel --no-default-features && ! grep -nE '"(dispatch|set_monitor)"' crates/archel/src/gui/models/display.rs</automated>
    <fails_when>countdown tests red (keep stops revert, timeout reverts) or a direct IPC verb appears in the display model</fails_when>
  </verify>
  <acceptance_criteria>
    Unit test with fake clock/hyprctl: plan -> apply -> keep prevents revert;
    expiry triggers revert call; CountdownModal bound to the tick property.
  </acceptance_criteria>
  <done>D-16 flow proven headless and live-checked manually at wave-end UAT.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| GUI -> Hyprland IPC socket | config mutation keywords to the live compositor |
| GUI -> hyprland.conf on disk | line-preserving file edits watched by notify |
| keybind/action user input -> conf | unvalidated strings reaching add_bind_line |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-06-34 | Denial of Service | display change locking user out | high | mitigate | D-16 auto-revert lives in core apply_monitor_plan (15 s), GUI cannot disable it; CountdownModal always offers Annuler maintenant |
| T-06-35 | Tampering | keybind add with crafted action strings | medium | mitigate | binds::plan_bind validation before add_bind_line; collision_hint review; conf written only through the core line editor (no free-form file writes from GUI) |
| T-06-36 | Repudiation | silent conf edits racing external editors | low | accept | notify watcher (core) reloads external edits; confirm question quotes exact change; manual UAT |
| T-06-SC | Tampering | npm/pip/cargo installs | high | mitigate | no new packages; legitimacy gate N/A; T-06-SC retained |
</threat_model>

<artifacts_produced>
## Artifacts this phase produces

(New in this plan.)

- Write invokables: `AppearanceModel.set_blur` / `set_opacity` / `set_wallpaper`
  / `set_font`; `InputModel.set_sensitivity` / `set_accelerate` /
  `add_keybind` / `remove_keybind`; `DisplayModel.plan_change` /
  `keep_change` / `revert_change`.
- `CountdownModal` real implementation + sole instantiation in `DisplayTab.qml`.
- UI Considerations coverage: long-text-keybind.
</artifacts_produced>

<verification>
- `cargo test -p archel --no-default-features` green; clippy clean; offscreen smoke rc=124.
- Manual UAT at wave end: wallpaper change, keybind add/remove, resolution
  change letting the countdown expire (auto-revert) and once with Conserver.
</verification>

<success_criteria>
All ten tabs are fully writable thin views over archel-core; no GUI-side
bypass of a core seam.
</success_criteria>

<output>
Create `.planning/phases/06-gui-shell-tabs/06-08-SUMMARY.md` when done
</output>
