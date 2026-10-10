---
phase: 06-gui-shell-tabs
plan: 07
subsystem: gui-settings
status: complete
tags: [write, power, audio, dns, bluetooth, vpn, privilege, pairing, cxx-qt, qml]
requires:
  - phase: 06-04
    provides: read-side tab models and tab QML shells this plan adds writes to
  - phase: 06-06
    provides: ModalSession, ShellModel modal surface, Confirm/Error/Stream modals in Main.qml
provides:
  - Write invokables on Power/Audio/Net/Bt models through fn-pointer seams to CLI-proven core entry points
  - ShellModel pairing-session surface plus the single global PairingModal (D-21)
  - VpnImportPopup with FileDialog plus typed-path fallback (D-20)
  - Core confirm-free twins: power_set_direct, audio set_*_by_name_with, vpn_import_stdin_null, bluetooth pair_device_with/unpair/adapter fns
affects:
  - 06-08/06-09 (hyprland writes, thin-view check — same worker/seam/modal conventions)
  - gui-check (offscreen smoke covers the new tabs and modals)
tech-stack:
  added: []
  patterns:
    - "fn-pointer injection seams per domain (PowerWriteOps/AudioWriteOps/NetDnsOps/NetVpnOps/BtWriteOps) with PROD_* wiring; unit tests pass fakes, workers run on the blocking pool"
    - "request_confirm/recv modal handshake reused for every write; pairing answers use a dedicated pairing session independent of modal_kind"
    - "property-alias model exposure (netTab.net) instead of cross-component id references"
requirements-completed: [PLAT-03]
coverage:
  - id: D1
    description: "Power profile switch with core-verbatim confirm, spinner, read-back refresh"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "crates/archel/src/gui/models/power.rs#decline_short_circuits_before_any_backend_call"
        status: pass
      - kind: unit
        ref: "crates/archel/src/gui/models/power.rs#accept_reaches_the_backend_and_surfaces_its_error_verbatim"
        status: pass
      - kind: unit
        ref: "crates/archel/src/gui/models/power.rs#unknown_profile_is_rejected_before_any_confirm_or_backend"
        status: pass
    human_judgment: false
  - id: D2
    description: "Audio default sink/source pickers with inline core errors and read-back proof"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "crates/archel/src/gui/models/audio.rs#accept_switches_and_proves_the_read_back"
        status: pass
      - kind: unit
        ref: "crates/archel/src/gui/models/audio.rs#unknown_sink_name_is_rejected_before_any_confirm"
        status: pass
      - kind: unit
        ref: "crates/archel/src/gui/models/audio.rs#wpctl_failure_surfaces_the_core_string_unmodified"
        status: pass
    human_judgment: false
  - id: D3
    description: "DNS edit with live core validation, plan/apply round-trip, VPN import with stdin-detached nmcli"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "crates/archel/src/gui/models/net.rs#dns_decline_asks_the_plan_question_verbatim_and_never_applies"
        status: pass
      - kind: unit
        ref: "crates/archel/src/gui/models/net.rs#vpn_accept_imports_and_reports_type_plus_name"
        status: pass
      - kind: unit
        ref: "crates/archel/src/gui/models/net.rs#vpn_bad_path_is_rejected_with_the_core_message_before_any_spawn"
        status: pass
    human_judgment: false
  - id: D4
    description: "Bluetooth pair/connect/disconnect/unpair plus adapter toggle through the single global PairingModal"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "crates/archel/src/gui/models/bt.rs#pair_accept_shows_the_canonical_code_and_propagates_yes"
        status: pass
      - kind: unit
        ref: "crates/archel/src/gui/models/bt.rs#pair_decline_at_the_code_unpairs_and_reports_nothing"
        status: pass
      - kind: unit
        ref: "crates/archel/src/gui/models/bt.rs#adapter_accept_applies_and_reads_back"
        status: pass
    human_judgment: false
  - id: D5
    description: "Live-hardware end-to-end (profile switch with confirm, DNS apply, BT phone pair, .ovpn import)"
    requirement: "PLAT-03"
    verification: []
    human_judgment: true
    rationale: "Needs the live Archel box with PPD, NM, a phone and an .ovpn file — headless CI cannot exercise radios, buses or pkexec."
actuals:
  tokens: 37000
  tasks: 3
  commits: 3
metrics:
  duration: ~2h
  completed: 2026-10-10
  tasks: 3
  files: 19
  tests-added: 21
  tests-total-gui: 236
---

# Phase 06 Plan 07: Write System Domains Summary

Power profile switching, audio default pickers, DNS edit/apply, VPN import and the full Bluetooth lifecycle — every write routed through the 06-06 modal infrastructure and confirm-injected twins of the CLI-proven core entry points, with per-tab busy/disabled/error states.

## Performance

- **Duration:** ~2h
- **Tasks:** 3
- **Files modified:** 19
- **Tests added:** 21 (3 power + 4 audio + 5 net + 5 bt + 2 modal-pairing + 2 extended read-path asserts)

## Accomplishments

- PowerTab writes: `set_profile` with the core `confirm_set_prompt` verbatim, spinner + disabled segment while in flight (D-15), PollGuard refresh reading the new profile back from PPD.
- AudioTab writes: sink/source ComboBox pickers; unknown names rejected pre-confirm with the core error string inline; switch proven by read-back; both pickers lock during a switch.
- NetTab writes: DNS row editor with live `validate_dns_servers` French inline errors, `plan_dns_set` → verbatim `confirm_dns_prompt` → `apply_dns_plan` (never direct config/nmcli writes, T-06-31); VPN import with path validation + plugin inference pre-confirm and a stdin-detached `nmcli --ask` child (T-06-32).
- BtTab writes: pair/unpair/connect/disconnect gated by paired/connected flags, per-row spinners, adapter radio toggle with read-back proof; pairing code shown once in the global PairingModal (D-21, single Main-root instantiation).
- Core gained five thin confirm-free twins; every CLI twin delegates unchanged (`power_set`, `set_output/input_by_name`, `vpn_import`, `pair_device`).

## Task Commits

Each task was committed atomically (see Deviations for the Task 1 attribution note):

1. **Task 1: Power + audio writes** - `8a8b8b8` (feat, shared with 06-08 — see deviation 1)
2. **Task 2: DNS edit/apply + VPN import** - `8d41e7d` (feat 06-07)
3. **Task 3: Bluetooth pair/connect/disconnect + global PairingModal** - `53dd31e` (feat 06-07)

## Files Created/Modified

- `crates/archel-core/src/power.rs` - `power_set_direct` (confirm-free backend switch; `power_set` = confirm + direct)
- `crates/archel-core/src/audio.rs` - `set_output/input_by_name_with` (confirm-injected; existing fns delegate)
- `crates/archel-core/src/network.rs` - `run_vpn_import` stdin-null flag + `vpn_import_stdin_null` (CLI `vpn_import` unchanged)
- `crates/archel-core/src/bluetooth.rs` - `pair_device_with` seam (`pair_device` delegates), `unpair_device`, `adapter_powered`, `set_adapter_powered`, unpair/adapter prompts + ok lines
- `crates/archel-core/locales/fr.json` - 7 new `bt.*` keys (unpair/adapter confirms, ok lines, state words)
- `crates/archel/src/gui/modal.rs` - pairing session (`begin/publish/resolve/end` + snapshots), independent of modal_kind
- `crates/archel/src/gui/shell.rs` - pairing qproperties, sync mirroring, `pairingAccepted`/`pairingDeclined`
- `crates/archel/src/gui/models/power.rs` - `PowerWriteOps` seam, `power_worker`, `set_profile`, operation slot
- `crates/archel/src/gui/models/audio.rs` - `AudioWriteOps` seam, output/input workers with read-back, inline error props
- `crates/archel/src/gui/models/net.rs` - `NetDnsOps`/`NetVpnOps` seams, dns/vpn workers, `dnsValidate`/`dnsApply`/`vpnImportStart`
- `crates/archel/src/gui/models/bt.rs` - `BtWriteOps` seam, pair/unpair/connect/disconnect/adapter workers, paired/connected row flags, adapter read in tick
- `crates/archel/src/gui/bridge_power.rs`, `bridge_audio.rs`, `bridge_net.rs`, `bridge_bt.rs` - new qproperties + invokables (no build.rs touches)
- `crates/archel/src/gui/qml/tabs/PowerTab.qml` - clickable segment + spinner
- `crates/archel/src/gui/qml/tabs/AudioTab.qml` - ComboBox pickers + inline errors + spinners
- `crates/archel/src/gui/qml/tabs/NetTab.qml` - DNS editor + VPN section with result banner + `property alias net` + `importRequested`
- `crates/archel/src/gui/qml/tabs/BtTab.qml` - adapter switch + per-row Pair/Connect/Disconnect/Supprimer + spinners + scan state
- `crates/archel/src/gui/qml/components/PairingModal.qml` - real D-21 body (code large monospace, Confirmer gated on code presence)
- `crates/archel/src/gui/qml/components/VpnImportPopup.qml` - real D-20 body (FileDialog + typed-path fallback)
- `crates/archel/src/gui/qml/Main.qml` - single PairingModal + VpnImportPopup at root, NetTab wiring

## Decisions Made

- Worker pattern everywhere (pkg.rs precedent): invokables only spawn; workers do validate → core-verbatim confirm → apply on the blocking pool; async core fns reached via `Handle::current().block_on` in PROD seams.
- Audio failures render inline under the picker (plan-mandated); everything else failures → ErrorModal with `message()` verbatim (D-17).
- Pairing session independent of `ModalKind`: `begin_pairing` never calls `reset()`, so a concurrent confirm from another tab cannot tear down a pairing prompt; `end_pairing` always runs (timeout-bounded `block_on`).
- Declined pairing code → best-effort `unpair` (no half-bond) + silence; decline is a first-class answer, never an error.
- `pair_worker` takes `&'static ModalSession` (the agent slot is `'static`; production passes the global session, tests `Box::leak`); all other workers keep plain borrows.
- Adapter toggle reuses the single-operation slot with `busy_address = "adapter"` (`ADAPTER_BUSY_TAG`) instead of a second flag.
- VpnImportPopup closes on submit so the tab shows the spinner and the result banner; QML reaches the model through `property alias net`, not cross-component ids.

## Deviations from Plan

### Auto-fixed Issues

**1. [Environment — parallel-wave commit collision] Task 1 changes committed under 06-08's message**
- **Found during:** Task 1 commit
- **Issue:** A sibling wave agent (`feat(06-08)`, same shared working directory, no worktree isolation) ran a `commit -a`-style commit at 12:21 that swept up this plan's staged Task 1 files under its own message (`8a8b8b8` contains both appearance and power/audio changes).
- **Fix:** Verified every Task 1 file and test survived intact in `8a8b8b8` (content greps + full verify re-run green); recorded the shared hash as the Task 1 commit instead of fabricating an empty commit. Tasks 2–3 committed atomically in single add+commit commands with pre-commit `git status` checks.
- **Verification:** `cargo test -p archel-core power/audio` + `cargo test -p archel --no-default-features` + gui-feature model tests all green on the committed tree.
- **Committed in:** `8a8b8b8` (shared)

**2. [Rule 2 - Missing critical] Core had no confirm-free/GUI-safe write entry points**
- **Found during:** Tasks 1–3 (core reconnaissance)
- **Issue:** `power_set`, `set_output/input_by_name` call terminal `confirm_action` internally (NeedsTty/refuse under GUI); `run_vpn_import` inherits stdin (`--ask` could block on an unseen terminal); `pair_picked_with` has no code-callback slot and `pair_device` prompts on the terminal; no unpair/adapter core fns exist despite the plan's key_links assuming them.
- **Fix:** Added `power_set_direct`, `set_output/input_by_name_with`, `vpn_import_stdin_null` (stdin flag, CLI path unchanged), `pair_device_with` + `pair_device` delegation, `unpair_device`, `adapter_powered`, `set_adapter_powered`, plus 7 `bt.*` catalog keys. All CLI twins delegate with identical behavior.
- **Files modified:** `crates/archel-core/src/{power,audio,network,bluetooth}.rs`, `locales/fr.json`
- **Verification:** `cargo test -p archel-core --no-default-features {power,audio,network,bluetooth}` green; CLI snapshot tests unaffected.
- **Committed in:** `8a8b8b8` (power/audio core), `8d41e7d` (vpn core), `53dd31e` (bt core + catalog)

**3. [Rule 2 - Missing critical] BtDevice rows lacked paired/connected flags; tick had no adapter read**
- **Found during:** Task 3
- **Issue:** `devices_json_for` emitted only name/address/state/signal, so per-row Pair/Connect/Disconnect gating was impossible; no adapter-state property existed for the toggle.
- **Fix:** Added `paired`/`connected` to the JSON rows and an `adapter_on` property fed by best-effort `adapter_powered()` in the tick (keeps previous on error).
- **Files modified:** `crates/archel/src/gui/models/bt.rs`, `bridge_bt.rs`
- **Verification:** row-shape asserts in `success_fills_rows_with_core_display_strings`; gui suite green.
- **Committed in:** `53dd31e`

**4. [Rule 3 - Blocking] `Box<PairAnswer>` demands a `'static` closure**
- **Found during:** Task 3 (compile)
- **Issue:** The pair worker's answer closure borrows the session and the decline flag, but the boxed callback for the core agent slot must own everything.
- **Fix:** `pair_worker` takes `&'static ModalSession` (documented why), decline flag behind `Arc`; the `pair` invokable passes the global `modal::session()`; tests `Box::leak` local sessions.
- **Files modified:** `crates/archel/src/gui/models/bt.rs`
- **Verification:** gui-feature `bt` tests green (13 passed).
- **Committed in:** `53dd31e`

---

**Total deviations:** 4 (1 environmental, 2 missing-critical, 1 blocking)
**Impact on plan:** All additions are thin seams/wrappers around CLI-proven core functions — no new domain logic in the GUI, no subprocess/privilege code outside core, no build.rs registration touches. No scope creep.

## Issues Encountered

- **Sibling uncommitted breakage blocked gui-feature test runs twice:** a concurrent `display.rs` edit (06-09 wave, same directory) failed compilation mid-plan; left untouched per scope boundary — retried after the sibling fixed it. Also one transient concurrent-cargo failure (stale C++ errors that vanished on rerun). Logged pattern, not fixed.
- **QML id scoping defect found (not fixed, out of scope):** `Main.qml:357 onAurValidated: pkg.aurBuildStart()` references the component-local `pkg` id inside `PkgTab.qml` — latent ReferenceError on AUR Valider click. Logged to `deferred-items.md` for the owning (06-06) or check (06-09) plan. This plan's own wiring uses `property alias` and is unaffected.
- `gsd_run` CLI unavailable in this environment — WINDOWS ledger append skipped (best-effort); defect captured in `deferred-items.md`.

## Authentication Gates

None — all privilege paths delegate to core's pkexec/sudo detection; `NeedsPrivilege`/`PolkitDenied` shapes render through ErrorModal (D-17/D-18). No auth was needed to run this plan's gates.

## Known Stubs

None. Every write path is wired end-to-end through `PROD_*` ops to core functions proven by the CLI; tests fake only the seam, never the production wiring. The PairingModal `······` placeholder is the honest waiting state (code not yet published), not a stub.

## Threat Flags

None new. Plan dispositions verified present: DNS gated by `validate_dns_servers` with plan/apply structure intact (T-06-31); VPN import validates pre-confirm and runs stdin-detached (T-06-32); single PairingModal with `format_passkey` display and no auto-accept path (T-06-33); no new packages (T-06-SC). GUI crate spawns no subprocess (grepped: no `Command`/`duct`/`std::process` in gui write paths).

## Verification

| Gate | Result |
|------|--------|
| `cargo test -p archel-core --no-default-features power/audio/network/bluetooth` | 11 + 11 + 34 + 11 passed |
| `cargo test -p archel --no-default-features` | 120 passed |
| `cargo test -p archel --features gui` | 236 passed, 0 failed |
| `cargo clippy -p archel-core / -p archel --no-default-features` | clean |
| `cargo build -p archel --features gui` | green |
| `QT_QPA_PLATFORM=offscreen timeout 10 archel settings` | rc=124 (ran to timeout), 0 property/qrc errors |
| `qmllint` on all touched QML | clean |
| `grep PairingModal Main.qml` instantiation count | 1 |
| Manual UAT (profile switch, DNS apply, BT pair, .ovpn import) | deferred to wave end per plan (not blocking) |

## Self-Check: PASSED

- All files listed above exist on disk with the committed content (`grep` verified per task).
- Commits `8a8b8b8` (shared, contains Task 1), `8d41e7d`, `53dd31e` present in `git log`.
- No `build.rs` modifications in any of the three commits.
- No subprocess code in GUI write paths.

## Next Phase Readiness

- Power/Audio/Network/Bluetooth tabs are fully write-capable thin views; the modal + seam + worker conventions are ready for 06-08/06-09 to mirror.
- Watch item: shared-directory parallel waves can sweep staged files into foreign commits — atomic add+commit and pre-commit status checks mitigated it here.
- Open (owned elsewhere): AUR Valider QML scoping defect in `deferred-items.md`; camelCase binding debt and flaky `live_*` core tests already logged by 06-06.

---
*Phase: 06-gui-shell-tabs*
*Completed: 2026-10-10*
