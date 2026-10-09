---
phase: 06-gui-shell-tabs
plan: 07
type: execute
wave: 6
depends_on: [06-04, 06-06]
files_modified:
  - crates/archel/src/gui/models/power.rs
  - crates/archel/src/gui/models/audio.rs
  - crates/archel/src/gui/models/network.rs
  - crates/archel/src/gui/models/bluetooth.rs
  - crates/archel/qml/tabs/PowerTab.qml
  - crates/archel/qml/tabs/AudioTab.qml
  - crates/archel/qml/tabs/NetworkTab.qml
  - crates/archel/qml/tabs/BluetoothTab.qml
  - crates/archel/qml/components/PairingModal.qml
  - crates/archel/qml/components/VpnImportPopup.qml
  - crates/archel/qml/Main.qml
autonomous: true
requirements: [PLAT-03]
tags: [write, power, audio, dns, bluetooth, vpn, privilege]

estimate:
  tokens: 84000
  raw_tokens: 60000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "Power profile change shows the core's confirm_set_prompt question in ConfirmModal; on confirm power_set applies and the poll reflects the new profile (spinner on the segment while in flight, D-15)"
    - "Audio default sink/source switches via a picker list; the selection writes through set_output_by_name / set_input_by_name and the PollGuard refresh confirms it"
    - "DNS servers are edited as a list, validated by nm_dns::validate_dns_servers, previewed via plan_dns_set (question verbatim), then apply_dns_plan — invalid input shows the French validation message inline"
    - "Bluetooth pairing opens the global PairingModal (instantiated once at Main.qml root, D-21): passkey display with format_passkey + confirmation on both sides for ConfirmOnly/DisplayYesNo via passkeys_equal; connect/disconnect buttons per device with busy state"
    - "VPN import opens VpnImportPopup (D-20) with FileDialog; if Qt file dialog is unavailable in the environment a typed-path fallback TextBox is offered; core vpn_import runs via spawn_blocking and its validation errors (validate_vpn_import_path messages) render in ErrorModal"
    - "Privilege-required paths surface NeedsPrivilege-style errors through ErrorModal; pkexec needs a polkit agent (hyprpolkit) on the live box — precondition, not plan work (D-18)"
  artifacts:
    - "Write invokables added to the 06-04 models: PowerModel.set_profile, AudioModel.set_default_output/set_default_input, NetModel.dns_validate/dns_apply/vpn_import_start, BtModel.pair/unpair/connect/disconnect/set_adapter_on"
    - "PairingModal and VpnImportPopup real implementations (api blocks from 06-02 untouched) instantiated at Main.qml root"
  key_links:
    - "PowerModel.set_profile -> core power::confirm_set_prompt question -> ShellModel.openConfirm -> power::power_set via spawn_blocking"
    - "NetModel.dns_apply -> nm_dns::validate_dns_servers -> plan_dns_set -> apply_dns_plan (NEVER edit nm-connection-editor config directly)"
    - "BtModel.pair -> bluetooth::pair_picked_with with PassKey/YesNo callbacks -> ShellModel pairing-session properties -> PairingModal (the ONLY pairing UI)"
    - "NetModel.vpn_import_start -> network::vpn_import (nmcli --ask child, stdin Stdio::null() per assumption A6) -> results banner/ErrorModal"
---

<objective>
Give the four system-domain tabs their write actions — power profile, audio
defaults, DNS, Bluetooth pairing/connect, VPN import — all routed through the
06-06 modal infrastructure and the core's CLI-proven entry points.

Purpose: completes the "thin view over CLI-proven core" criterion for the
D-Bus/subprocess domains and exercises Confirm/Error/Stream modals beyond
Packages.
Output: write-capable Power/Audio/Network/Bluetooth tabs.
</objective>

<context_read_first>
- .planning/phases/06-gui-shell-tabs/06-CONTEXT.md (D-12, D-15, D-18, D-20, D-21)
- .planning/phases/06-gui-shell-tabs/06-RESEARCH.md (Tab↔core map lines 266+,
  Pitfall on pair_picked_with callbacks, nm_dns plan/apply pattern)
- crates/archel/src/cmds/{power_cmds,audio_cmds,net_cmds,bt_cmds}.rs (CLI twins)
- crates/archel-core/src/{power.rs, audio.rs, network.rs, bluetooth.rs} entry
  points listed in key_links
- .planning/phases/06-gui-shell-tabs/06-06-SUMMARY.md (modal API created by the
  prior plan in this phase)
</context_read_first>

<tasks>

<task type="auto">
  <name>Task 1: Power + audio writes</name>
  <files>crates/archel/src/gui/models/power.rs, crates/archel/src/gui/models/audio.rs, crates/archel/qml/tabs/PowerTab.qml, crates/archel/qml/tabs/AudioTab.qml</files>
  <action>
    PowerModel.set_profile(profile): build question from core
    power::confirm_set_prompt, openConfirm on ShellModel, on yes
    spawn_blocking power::power_set(backend_tag, profile) — power_set already
    parses/validates via parse_profile_arg; BusyIndicator on the segmented
    control + controls disabled while in flight (D-15); on done force a
    PollGuard refresh so the new profile reads back from PPD.
    AudioModel: picker ComboBoxes over audio_list devices;
    set_default_output(name)/set_default_input(name) via the core by-name
    setters; unknown-name errors from core render inline under the picker;
    device rows show ports read by 06-04 (read-side wiring unchanged).
  </action>
  <verify>
    <automated>cargo test -p archel-core --no-default-features power && cargo test -p archel-core --no-default-features audio && cargo test -p archel --no-default-features</automated>
    <fails_when>write unit tests red (question verbatim from core helper; invalid profile/name rejected before any child/IPC call)</fails_when>
  </verify>
  <acceptance_criteria>
    Unit tests cover: confirm-decline short-circuits before power_set;
    by-name audio setters surface core error strings unmodified.
  </acceptance_criteria>
  <done>Both tabs write through the proven core path with spinner + confirm + error states.</done>
</task>

<task type="auto">
  <name>Task 2: DNS edit/apply + VPN import (VpnImportPopup)</name>
  <files>crates/archel/src/gui/models/network.rs, crates/archel/qml/tabs/NetworkTab.qml, crates/archel/qml/components/VpnImportPopup.qml, crates/archel/qml/Main.qml</files>
  <action>
    NetModel DNS: dnsServers string list property + add/remove/edit rows;
    dns_validate runs nm_dns::validate_dns_servers live (French inline errors);
    dns_apply -> plan_dns_set -> confirm question verbatim -> apply_dns_plan via
    spawn_blocking; spinner on the apply button; ErrorModal on apply failure.
    NetModel.vpn_import_start(path): pre-check with validate_vpn_import_path +
    infer_vpn_type (both core); then network::vpn_import via spawn_blocking —
    the nmcli --ask child must run with stdin Stdio::null() (assumption A6:
    verify at execution; if the core fn inherits the GUI's stdin, add a stdin
    null parameter or a thin wrapper in network.rs keeping the CLI twin's
    behaviour for net_cmds run_vpn_import — net_cmds.rs:279).
    VpnImportPopup (Popup at Main.qml root, opened by the VPN-section
    "Importer…" button): FileDialog for .ovpn/.conf picks; typed-path fallback
    TextBox + "Utiliser ce chemin" button when the platform file dialog fails to
    open (A2 fallback); import result (type inferred + connection name) shown as
    success banner, failures in ErrorModal. UI Considerations empty-vpn,
    long-text-names (connection list elide + ToolTip) owned here.
  </action>
  <verify>
    <automated>cargo test -p archel-core --no-default-features network && cargo test -p archel --no-default-features</automated>
    <fails_when>DNS/VPN unit tests red (invalid DNS rejected pre-plan; import path validated before vpn_import; question from plan_dns_set verbatim)</fails_when>
  </verify>
  <acceptance_criteria>
    Unit tests: validate_dns_servers gate; plan/apply sequence; VPN path
    validation rejects traversal/nonexistent paths with core message verbatim.
  </acceptance_criteria>
  <done>DNS round-trip + VPN import with both picker paths (dialog + typed fallback) work.</done>
</task>

<task type="auto">
  <name>Task 3: Bluetooth pair/connect/disconnect + global PairingModal</name>
  <files>crates/archel/src/gui/models/bluetooth.rs, crates/archel/qml/tabs/BluetoothTab.qml, crates/archel/qml/components/PairingModal.qml, crates/archel/qml/Main.qml</files>
  <action>
    BtModel.pair(address): call bluetooth::pair_picked_with with callbacks that
    drive the ShellModel pairing session (pairingActive, pairingAddress,
    pairingDeviceName, pairingMode "passkey"/"yesno", pairingPasskey via
    format_passkey) — PairingModal (Main.qml root, D-21) shows device name +
    passkey large monospace, or yes/no pair for ConfirmOnly/DisplayYesNo;
    user answer returns through the callback (passkeys_equal for cross-check);
    pairingProgress "Appairement…" while awaiting. unpair/connect/disconnect
    invokables over the core fns, each with a per-row busy state; adapter
    toggle via BtModel.set_adapter_on (bt adapter set-powered through bluer
    as cmds/bt_cmds does — CLI twin). Device rows get Pair/Connect/Disconnect
    buttons gated by paired/connected flags from 06-04 scan population. UI
    Considerations empty-bt, loading-bt-scan (Scan en cours… + cancel, owned by
    06-04 burst — verify here), long-text-names (device name elide + ToolTip).
  </action>
  <verify>
    <automated>cargo test -p archel-core --no-default-features bluetooth && cargo test -p archel --no-default-features</automated>
    <fails_when>pairing callback tests red (passkey mode awaits user confirm; decline path unPAIRs/cancels; no second pairing dialog constructed anywhere — grep PairingModal instantiation count == 1 in Main.qml)</fails_when>
  </verify>
  <acceptance_criteria>
    Unit test with a fake callback harness: passkey displayed matches
    format_passkey output; yes/no answer propagated; PairingModal is the single
    instantiation.
  </acceptance_criteria>
  <done>Full BT lifecycle: adapter on -> scan burst -> pair with modal -> connect -> disconnect.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| GUI -> D-Bus (NM/PPD/WirePlumber) | system-service mutations from user input |
| GUI -> pkexec/nmcli child | privileged or interactive subprocess |
| Bluetooth radio -> pairing callback | remote device supplies passkey comparison flow |
| user file path -> nmcli import | arbitrary path read by imported-file validation |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-06-31 | Tampering | DNS apply | medium | mitigate | input gated by nm_dns::validate_dns_servers; plan/apply uses core plan structure, no free-form config writes |
| T-06-32 | Information Disclosure | VPN import path handling | medium | mitigate | core validate_vpn_import_path before nmcli (path traversal/nonexistent rejected); nmcli --ask with stdin null so no credential prompt hangs the GUI (A6) |
| T-06-33 | Spoofing | Bluetooth pairing | high | mitigate | D-21 single PairingModal; passkey rendered from format_passkey; confirmation compared with passkeys_equal; no silent auto-accept |
| T-06-SC | Tampering | npm/pip/cargo installs | high | mitigate | no new packages; package legitimacy gate N/A; T-06-SC retained |
</threat_model>

<artifacts_produced>
## Artifacts this phase produces

(New in this plan.)

- Write invokables: `PowerModel.set_profile`, `AudioModel.set_default_output` /
  `set_default_input`, `NetModel.dns_validate` / `dns_apply` /
  `vpn_import_start`, `BtModel.pair` / `unpair` / `connect` / `disconnect` /
  `set_adapter_on`.
- `PairingModal` real implementation + single root instantiation (D-21);
  `VpnImportPopup` with FileDialog + typed-path fallback (D-20/A2).
- ShellModel pairing-session properties (pairingActive/Address/DeviceName/
  Mode/Passkey/Progress) fed by `pair_picked_with` callbacks.
- UI Considerations coverage: empty-vpn, empty-bt, loading-bt-scan,
  long-text-names.
</artifacts_produced>

<verification>
- `cargo test -p archel --no-default-features` green; clippy clean; offscreen smoke rc=124.
- Manual UAT at wave end (not blocking): profile switch with confirm; DNS
  1.1.1.1 apply + revert; BT pair to a phone; VPN .ovpn import.
</verification>

<success_criteria>
Power/Audio/Network/Bluetooth tabs perform every CLI-proven write through the
shared modal infrastructure with proper busy/disabled/error states.
</success_criteria>

<output>
Create `.planning/phases/06-gui-shell-tabs/06-07-SUMMARY.md` when done
</output>
