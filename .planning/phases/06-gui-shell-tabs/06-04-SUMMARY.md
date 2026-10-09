---
phase: 06-gui-shell-tabs
plan: 04
subsystem: gui
tags: [thin-view, cxx-qt, qml, network, power, audio, bluetooth, burst-scan, archel]

# Dependency graph
requires: [06-03-polling-system-landing]
provides:
  - gui/models/net.rs (NetModel: async net_status + vpn_list, status_rows/VpnConnection JSON)
  - gui/models/power.rs (PowerModel: async power_status, wire-word profile + backend tag, 3-row read-only list)
  - gui/models/audio.rs (AudioModel: spawn_blocking audio_list, sink/source JSON with default markers)
  - gui/models/bt.rs (BtModel: async bt_devices tick + 12 s scanNow burst with in-flight guard, D-08)
  - bridge_net/power/audio/bt.rs shims + NetTab/PowerTab/AudioTab/BtTab populated QML
affects: [06-05-read-hyprland-pkg, 06-07-write-system-domains, 06-08-write-hyprland-domains]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
actuals:
  tokens: 17000
  tasks: 2
  commits: 2

# Tech tracking
tech-stack:
  added: []
  patterns: [async-tick-via-runtime-spawn, sync-subprocess-inside-async-via-spawn-blocking, burst-scan-pauses-tick]

key-files:
  created:
    - crates/archel/src/gui/models/net.rs
    - crates/archel/src/gui/models/power.rs
    - crates/archel/src/gui/models/audio.rs
    - crates/archel/src/gui/models/bt.rs
    - crates/archel/src/gui/bridge_net.rs
    - crates/archel/src/gui/bridge_power.rs
    - crates/archel/src/gui/bridge_audio.rs
    - crates/archel/src/gui/bridge_bt.rs
  modified:
    - crates/archel/src/gui/models/mod.rs
    - crates/archel/src/gui/mod.rs
    - crates/archel/build.rs
    - crates/archel/src/gui/qml/tabs/NetTab.qml
    - crates/archel/src/gui/qml/tabs/PowerTab.qml
    - crates/archel/src/gui/qml/tabs/AudioTab.qml
    - crates/archel/src/gui/qml/tabs/BtTab.qml

key-decisions:
  - "Async core fns tick via runtime.spawn(async), sync subprocess fns via spawn_blocking - net_status().await pairs with spawn_blocking(vpn_list) inside one async tick"
  - "Audio rows carry name/description/default only: core AudioDevice exposes no mute/port fields, and inventing them would be new domain logic (PLAT-03)"
  - "VPN-list failure degrades to the honest empty list without nuking the tab; only net_status failure reads N/A everywhere"
  - "Burst scan pauses the 1 Hz tick via a separate AtomicBool (not the PollGuard), so ticks skip silently without tripping the in-flight path"

requirements-completed: [PLAT-03]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "Network tab lists live interface/connection state from net_status() and the VPN list from vpn_list(); Power tab shows the active profile and backend from power_status() (PLAT-03)"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "gui::models::net::tests (4 passed: N/A + banner once, JSON row shape, vpn degrade, opt_or_na)"
        status: pass
      - kind: unit
        ref: "gui::models::power::tests (3 passed: N/A + banner once, profile/backend/row shape, live wiring)"
        status: pass
      - kind: lint
        ref: "qmllint NetTab.qml PowerTab.qml, exit 0"
        status: pass
      - kind: manual_procedural
        ref: "QT_QPA_PLATFORM=offscreen timeout 10 ./target/debug/archel settings -> rc 124, zero qml errors"
        status: pass
    human_judgment: false
  - id: D2
    description: "Sound tab lists sinks/sources with default markers from audio_list(); BT tab lists devices via bt_devices() and fills only at the 12 s burst end with button locked + spinner (D-08)"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "gui::models::audio::tests (2 passed: empty + banner once, default markers)"
        status: pass
      - kind: unit
        ref: "gui::models::bt::tests (3 passed: empty + banner once, core display strings, scan double-claim no-op)"
        status: pass
      - kind: lint
        ref: "qmllint AudioTab.qml BtTab.qml, exit 0"
        status: pass
      - kind: manual_procedural
        ref: "QT_QPA_PLATFORM=offscreen timeout 10 ./target/debug/archel settings -> rc 124, zero qml errors"
        status: pass
    human_judgment: false
  - id: D3
    description: "Empty states render the UI-SPEC copy with affordances; long names elide ElideMiddle with full-text ToolTip (PLAT-03)"
    requirement: "PLAT-03"
    verification:
      - kind: manual_procedural
        ref: "QML source review: exact 'Aucun VPN configure' and 'Aucun appareil detecte' copy + Importer/Rechercher affordances present; ElideMiddle + ToolTip in all list delegates"
        status: pass
    human_judgment: true

# Metrics
duration: 45min
completed: 2026-10-09
status: complete
commits: 2
plan_head_before: 7e666a32631fd4557ce5c7178a427fce25e12ce8
plan_head_after: a6f7d5f1acd8c9d8a2c11607001376b71461470a
---

# Phase 06 Plan 04 Summary

**Four read-only thin tabs over the CLI-proven core: Network, Power, Sound, Bluetooth with the D-08 burst scan.**

## Performance

- **Duration:** ~45min
- **Tasks:** 2 completed
- **Files modified:** 15 (8 created Rust, 7 modified Rust/QML/build)
- **Tests added:** 12 (4 net + 3 power + 2 audio + 3 bt)

## Accomplishments

- `NetModel`: `tick()` spawns one async task doing `network::net_status().await` plus `spawn_blocking(vpn::vpn_list)`; detail rows reuse `status_rows` (same rows the CLI table renders), VPN rows serialize `VpnConnection` directly (name/uuid/type/active). NetTab: Connexion GroupBox of StatFields, VPN ListView with ElideMiddle + ToolTip, exact `Aucun VPN configure` empty copy + inert `Importer` button (plan 07 wires it, D-20).
- `PowerModel`: `tick()` spawns async `power::power_status()`; renders PPD wire words (`profile_arg_name`, `backend_tag`) plus a 3-row `[{id, active}]` read-only RadioButton list. PowerTab: Profil + Pilote GroupBoxes, ellipsis-until-first-tick via StatField.
- `AudioModel`: `tick()` runs `spawn_blocking(audio::audio_list())`; sink/source JSON reuses `AudioSnapshot` flags (name/description/default). AudioTab: Lecture/Enregistrement GroupBoxes with default-marker icons, ElideMiddle + ToolTip.
- `BtModel`: `tick()` spawns async `bt_devices()` but returns early while `scan_busy` is set; `scanNow()` claims an `AtomicBool` slot (double-press no-op), runs the 12 s `bt_scan()` burst, and replaces the list only at burst end (D-08, no progressive fill). Row strings reuse `state_label`/`signal_cell`. BtTab: device ListView, Rechercher disabled + BusyIndicator while scanning, exact `Aucun appareil detecte` empty copy with Rechercher guidance.
- Wiring: four `bridge_<tab>.rs` shims (proven split), `models/mod.rs` + `gui/mod.rs` + `build.rs` appends only. No new dependencies (T-06-SC holds); no D-Bus/nmcli/wpctl/bluez calls in GUI code (prohibition holds).

## Task Commits

Each task was committed atomically:

1. **Task 1: NetModel + PowerModel (+ their tabs)** - `15f20e5` (feat)
2. **Task 2: AudioModel + BtModel (+ their tabs, burst scan D-08)** - `a6f7d5f` (feat)

## Files Created/Modified

- `crates/archel/src/gui/models/{net,power,audio,bt}.rs` - view-model logic + inline tests
- `crates/archel/src/gui/bridge_{net,power,audio,bt}.rs` - bridge shims + Threading + re-exports
- `crates/archel/src/gui/models/mod.rs`, `gui/mod.rs`, `build.rs` - registration appends
- `crates/archel/src/gui/qml/tabs/{Net,Power,Audio,Bt}Tab.qml` - populated tab pages

## Decisions Made

- Async tick shape (`runtime.spawn(async)`) for net/power/bt vs `spawn_blocking` for audio/vpn-list — matches each core fn's own sync/async nature, one pattern per call shape
- Audio rows omit mute/port: core has no such fields; thin-view rule forbids inventing them
- VPN failure degrades locally (empty list) instead of failing the whole Net tick
- Burst guard is a dedicated `AtomicBool`, not the `PollGuard` — pausing ticks must not look like in-flight ticks

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Correctness] Audio rows carry name/description/default, not name/port/muted/is-default**
- **Found during:** Task 2 (core read before writing the model)
- **Issue:** The plan's `(name, port, muted, is-default)` shape assumed fields `AudioDevice` does not have — only `name/description/kind/default` exist, and mute state would need new `wpctl` parsing (new domain logic, forbidden by PLAT-03)
- **Fix:** Rows serialize exactly what `AudioSnapshot` carries; default marker renders from the snapshot's own `default` flag
- **Files modified:** `crates/archel/src/gui/models/audio.rs`
- **Commit:** `a6f7d5f`

**2. [Rule 3 - Blocking] Plan verify filters adjusted to real module paths**
- **Found during:** Task 1 verification (`cargo test` takes one TESTNAME filter; `net_model::` matches nothing)
- **Issue:** Tests live at `gui::models::net::tests`, so the plan's `net_model::`/`power_model::` filters match zero tests (same adjustment 06-03 documented for system/landing)
- **Fix:** Ran `models::net`, `models::power`, `models::audio`, `models::bt` filters — all green
- **Commit:** n/a (verification only)

**3. [Rule 2 - Correctness] NetModel has no hostname prop**
- **Found during:** Task 1 (core read)
- **Issue:** The plan's `hostname/connected/iface props` assumed a hostname field `NetStatus` does not carry; hostname comes from the System/Landing surface, not `net_status()`
- **Fix:** Status/iface/ssid/ipv4/gateway/dns/profile props plus the full `status_rows` JSON — everything `net_status()` actually knows
- **Files modified:** `crates/archel/src/gui/models/net.rs`
- **Commit:** `15f20e5`

---

**Total deviations:** 3 auto-fixed (2 correctness, 1 verify-filter). No scope creep; no new dependencies; every read traces 1:1 to a CLI entry point (`run_status`, `run_vpn_list`, `run_status` power, `run_list` audio, `run_devices`/`run_scan` bt).

## Issues Encountered

- None beyond the deviations above. Full verification green: core 269 / headless 120 / gui 151 (139 + 12 new), qmllint sweep clean, offscreen smoke rc 124 with zero QML errors.
- The `>=60-char name` ElideMiddle visual check (UI-SPEC backstop row) is code-reviewed (ElideMiddle + ToolTip in every list delegate) but not visually rendered — headless environment has no display; flagged for UAT.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Six of ten tabs now show live data (Landing, System, Net, Power, Audio, BT); plan 05 adds the Hyprland/package read tabs on the same recipe
- Plan 07 consumes: the inert `Importer` affordance (NetTab `onClicked: {}` placeholder), the read-only profile RadioButtons, and the `scan_busy` slot pattern for pairing flows
- T-06-23 mitigation (scan in-flight flag) is implemented and unit-tested; T-06-24 holds (no zbus in the GUI crate — verify with `grep -r zbus crates/archel/src/gui` in plan 09's audit)

## Self-Check: PASSED

- All 15 files exist on disk; commits `15f20e5`, `a6f7d5f` verified in `git log`
- Full verification green at commit time (core 269 / headless 120 / gui 151, qmllint sweep, offscreen smoke rc 124 with zero QML errors)
- No `.planning/` or `prompt.md` files staged in task commits; deletions check clean on both commits

---
*Phase: 06-gui-shell-tabs*
*Completed: 2026-10-09*
