---
phase: "06"
plan: "04"
type: execute
wave: 3
depends_on: ["06-03"]
files_modified:
  - crates/archel/src/gui/models/net.rs
  - crates/archel/src/gui/models/power.rs
  - crates/archel/src/gui/models/audio.rs
  - crates/archel/src/gui/models/bt.rs
  - crates/archel/src/gui/models/mod.rs
  - crates/archel/src/gui/mod.rs
  - crates/archel/src/gui/qml/tabs/NetTab.qml
  - crates/archel/src/gui/qml/tabs/PowerTab.qml
  - crates/archel/src/gui/qml/tabs/AudioTab.qml
  - crates/archel/src/gui/qml/tabs/BtTab.qml
autonomous: true
requirements: [PLAT-03]
estimate:
  tokens: 35000
  raw_tokens: 25000
  tasks: 2
  confidence: low
must_haves:
  truths:
    - "Network tab lists live interface/connection state from network::net_status(); Power tab shows the active profile and backend from power::power_status(); Sound tab lists sinks/sources with default markers from audio::audio_list() — each tab a thin read of the CLI-proven core entry point"
    - "Bluetooth tab lists paired/known devices via bt_devices() and runs the 12 s burst scan on Rechercher press, populating the list at burst end (D-08 — no progressive fill, RESEARCH gap #2 decision)"
    - "During the BT burst the Rechercher button is disabled with a BusyIndicator; results only replace the list when the burst completes (UI-SPEC BT-scan loading row)"
    - "Empty states render the exact UI-SPEC copywriting-contract copy with their affordance: Aucun appareil detecte + Rechercher guidance, Aucun VPN configure + Importer affordance (UI-SPEC empty rows, D-20 affordance only — the popup itself is plan 07)"
    - "Every long device/interface name elides with ElideMiddle plus full-text ToolTip (UI-SPEC long-text backstop row — verify visually with a >=60-char name)"
  artifacts:
    - crates/archel/src/gui/models/net.rs
    - crates/archel/src/gui/models/power.rs
    - crates/archel/src/gui/models/audio.rs
    - crates/archel/src/gui/models/bt.rs
    - "NetTab/PowerTab/AudioTab/BtTab .qml filled from placeholders"
  key_links:
    - "Each model's tick() -> poll.rs spawn/queue path (from 06-03) — no bespoke threading in any model"
    - "bt.scan() -> bt_scan() 12 s burst on the blocking pool with its own in-flight guard so the 1 Hz device-list tick pauses during the burst"
    - "vpn list rows <- the vpn::vpn_list() output already rendered by net_cmds run_vpn_list"
prohibitions:
  - "No D-Bus or nmcli/wpctl/bluez calls from GUI code — every read goes through an archel-core function already used by archel-cli (PLAT-03)"
  - "The BT burst scan must not be wired to the 1 Hz timer — it runs only on explicit user press (D-08)"
---

<objective>
Deliver four read-only thin tabs over the CLI-proven core: Network (net_status + VPN list), Power (profile + backend), Sound (sink/source list), Bluetooth (device list + 12 s burst scan) — each following the 06-03 model pattern.

Purpose: Success criterion 1 (navigable tabs) and criterion 4 (thin views): after this plan six of ten tabs show real data.
Output: four view-models + four populated QML tab pages.
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
@crates/archel/src/cmds/net_cmds.rs
@crates/archel/src/cmds/bt_cmds.rs
@crates/archel-core/src/bluetooth.rs
</context>

<tasks>

<task type="auto" id="06-04-T1">
  <name>Task 1: NetModel + PowerModel (+ their tabs)</name>
  <read_first>crates/archel-core/src/network.rs:241 (net_status) and the vpn submodule vpn_list:2272, crates/archel-core/src/power.rs:334 (power_status) and backend_tag/profile types, crates/archel/src/cmds/net_cmds.rs:17 (run_status) and power_cmds.rs:18 (run_status) — the CLI twins, crates/archel/src/gui/models/system.rs (06-03 model pattern), 06-UI-SPEC.md (empty-state rows for VPN, GroupBox rows)</read_first>
  <files>crates/archel/src/gui/models/net.rs, crates/archel/src/gui/models/power.rs, crates/archel/src/gui/models/mod.rs, crates/archel/src/gui/mod.rs, crates/archel/src/gui/qml/tabs/NetTab.qml, crates/archel/src/gui/qml/tabs/PowerTab.qml</files>
  <action>NetModel: tick() -> spawn the async network::net_status() on the runtime handle (it is already async — spawn, do not block_on) via the 06-03 poll helper; queue() sets hostname/connected/iface props and a serde_json QString of connection rows plus VPN rows from vpn::vpn_list() (name/type/active — the same fields run_vpn_list renders). NetTab.qml: GroupBox sections (UI-SPEC spacing md/lg), connection detail StatFields, VPN ListView of rows with ElideMiddle + ToolTip on names and the exact 'Aucun VPN configure' empty copy + Importer Button whose onClicked is a placeholder comment for plan 07 (affordance only). PowerModel: tick() -> power::power_status() (async — same spawn path); queue() sets profile + backend_tag props. PowerTab.qml: profile RadioDelegate list rendered read-only at this stage (writes land in plan 07), backend note line, ellipsis-until-first-tick rule via StatField. Register both models in models/mod.rs + gui/mod.rs (append lines only). Inline tests: Err -> N/A props + banner signal, vpn row JSON shape.</action>
  <verify>
    <automated>QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui 'net_model::' && QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui 'power_model::'</automated>
    <fails_when>either model's error-path or JSON-shape tests fail; N/A not emitted on Err</fails_when>
    <automated>qmllint crates/archel/src/gui/qml/tabs/NetTab.qml crates/archel/src/gui/qml/tabs/PowerTab.qml && QT_QPA_PLATFORM=offscreen timeout 10 ./target/debug/archel settings &lt;/dev/null >/tmp/archel-np.log 2>&1; rc=$?; test $rc -eq 124 && ! grep -qiE 'qml:|is not loaded' /tmp/archel-np.log</automated>
    <fails_when>qmllint errors or the offscreen smoke dies/logs binding errors for these tabs</fails_when>
  </verify>
  <acceptance_criteria>
  - Network tab shows live connectivity state and the VPN list with the correct empty copy + Importer affordance present but inert (plan 07 wires it)
  - Power tab reflects the active PPD/cpufreq profile read-only; thin view — zero logic beyond core calls (PLAT-03)
  </acceptance_criteria>
  <done>Network and Power tabs live from core reads.</done>
</task>

<task type="auto" id="06-04-T2">
  <name>Task 2: AudioModel + BtModel (+ their tabs, burst scan D-08)</name>
  <read_first>crates/archel-core/src/audio.rs:245 (audio_list, AudioSnapshot shape), crates/archel/src/cmds/audio_cmds.rs:18 (run_list twin), crates/archel-core/src/bluetooth.rs (bt_devices:215, bt_scan:383 — the 12 s burst, state_label/signal_cell display helpers), crates/archel/src/cmds/bt_cmds.rs:19-45 (run_devices/run_scan twins), 06-UI-SPEC.md (BT empty row, BT-scan loading row, long-text backstop row), 06-RESEARCH.md (Core Gap #2 decision: populate at burst end)</read_first>
  <files>crates/archel/src/gui/models/audio.rs, crates/archel/src/gui/models/bt.rs, crates/archel/src/gui/models/mod.rs, crates/archel/src/gui/mod.rs, crates/archel/src/gui/qml/tabs/AudioTab.qml, crates/archel/src/gui/qml/tabs/BtTab.qml</files>
  <action>AudioModel: tick() -> spawn_blocking audio::audio_list(); queue() sets JSON props for sinks and sources (name, port, muted, is-default) reusing AudioSnapshot's own flags rather than re-deriving anything. AudioTab.qml: two GroupBox sections (Lecture/Enregistrement per UI-SPEC copywriting rows) of ItemDelegates with a default marker icon; ElideMiddle + ToolTip on device names (backstop row). BtModel: tick() -> async bt_devices() via spawn (same path as net); separate scanNow() invokable guarded by its own in-flight flag that spawns the 12 s bluetooth::bt_scan() burst on the blocking pool, pausing the device-list tick (skip while scanning), and populates the JSON device list only at burst end (D-08, gap #2 decision — no progressive fill); reuse bluetooth::state_label/signal_cell strings for the row columns like bt_cmds does. BtTab.qml: device ListView (name ElideMiddle + ToolTip, state, signal), Rechercher Button disabled + BusyIndicator while scanning (loading row), 'Aucun appareil detecte' empty copy with Rechercher guidance. Register both; inline tests: scan guard double-press no-op, empty-list copy key, Err -> N/A.</action>
  <verify>
    <automated>QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui 'audio_model::' && QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui 'bt_model::'</automated>
    <fails_when>audio/bt model tests fail (scan re-entrancy, list populated mid-burst, missing empty-state key)</fails_when>
    <automated>qmllint crates/archel/src/gui/qml/tabs/AudioTab.qml crates/archel/src/gui/qml/tabs/BtTab.qml && QT_QPA_PLATFORM=offscreen timeout 10 ./target/debug/archel settings &lt;/dev/null >/tmp/archel-ab.log 2>&1; rc=$?; test $rc -eq 124 && ! grep -qiE 'qml:|is not loaded' /tmp/archel-ab.log</automated>
    <fails_when>qmllint errors or offscreen smoke fails with these tabs registered</fails_when>
  </verify>
  <acceptance_criteria>
  - Sound tab lists sinks/sources with default markers, one tick behind reality
  - BT list fills at burst end only; button locked with spinner during the 12 s scan (D-08)
  - Both empty-state copies render with their affordances (UI-SPEC rows)
  </acceptance_criteria>
  <done>Sound and Bluetooth read tabs live; six data tabs done after plan 05.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| NM/WirePlumber/BlueZ data -> QML lists | Device/interface names from system services render as text |
| user press -> 12 s blocking scan | Long-running scan occupies a blocking-pool thread |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-06-22 | Spoofing | device/interface name strings in ListView delegates | low | accept | Rendered via Text with ElideMiddle only — no rich text, no markup interpretation; names originate from the user's own paired devices |
| T-06-23 | Denial of Service | overlapping Rechercher bursts spawning many 12 s scans | medium | mitigate | Per-model scan in-flight flag (button disabled + tested double-press no-op); scan pauses the 1 Hz tick so the burst gets the pool slot alone |
| T-06-24 | Tampering | GUI bypassing core to talk D-Bus directly | medium | mitigate | Prohibition above; plan 09's thin-view audit asserts every model call maps to a cmds-proven core fn; no zbus dependency added to the GUI crate |
| T-06-SC | Tampering | crate installs | high | mitigate | No new packages (RESEARCH audit: none) |
</threat_model>

<verification>
- `cargo test -p archel-core --no-default-features && cargo test -p archel --no-default-features && QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui` + qmllint + offscreen smoke (VALIDATION.md)
</verification>

<success_criteria>
Four more tabs render live core data with honest loading/empty states; all reads trace 1:1 to a CLI entry point.
</success_criteria>

## Artifacts this phase produces (this plan)

- NetModel, PowerModel, AudioModel, BtModel (per-file bridge-mod pattern), populated NetTab/PowerTab/AudioTab/BtTab, burst-scan guard reused by plan 07's pairing flows.

<output>
When all tasks pass, create `.planning/phases/06-gui-shell-tabs/06-04-SUMMARY.md`.
</output>
