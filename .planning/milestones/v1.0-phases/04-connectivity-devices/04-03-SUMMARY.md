---
phase: 04-connectivity-devices
plan: 03
subsystem: bluetooth
tags: [bluer, bluez, pairing, scan, d-bus, confirm-gate]

# Dependency graph
requires:
  - phase: 04-02
    provides: VPN/proxy CLI patterns (confirm_action, NeedsTty, scan-then-pick precedent)
provides:
  - Working `archel bt scan|devices|pair|connect|disconnect` over bluer 0.17 with confirm-gated pairing
  - Typed BT errors (DeviceNotFound, PairingRejected, PairingTimeout) with French catalog arms
affects: [04-gui-phase, verify-work-manual-bt-pair]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
actuals:
  tokens: 28000
  tasks: 3
  commits: 2

# Tech tracking
tech-stack:
  added: [futures-util 0.3 (direct dep for StreamExt over bluer discovery stream)]
  patterns: [scan-then-pick pick validation, injectable confirm gate for headless abort coverage, per-application confirm-only bluer agent]

key-files:
  created: []
  modified: [crates/archel-core/src/bluetooth.rs, crates/archel-core/src/error.rs, crates/archel-core/locales/fr.json, crates/archel-core/Cargo.toml, crates/archel/src/cmds/bt_cmds.rs, crates/archel/src/cmds/mod.rs, Cargo.lock]

key-decisions:
  - "pair/connect/disconnect each run the timed sweep first, then validate the pick — the just-printed list is the only valid pick source"
  - "request_default stays false: bluer routes our own pair() to our registered agent, no need to steal the desktop default"
  - "PairingTimeout via PAIR_TIMEOUT ceiling; all other bluer pair failures map to PairingRejected"
  - "passkeys_equal walks all four octets; check_passkey_shape round-trips the zero-padded render before display"

patterns-established:
  - "Scan-then-pick: resolve_pick accepts 1-based index or in-list MAC (case-insensitive, trimmed); everything else is DeviceNotFound"
  - "Injectable confirm gate (pair_picked_with): abort path unit-covered headless with zero spawns"
  - "Confirm-only agent: request_confirmation set, every other handler None (rejects), never Agent::default() alone"

requirements-completed: [BT-01, BT-02]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "bt scan timed sweep renders numbered live list (name+MAC+signal), dedupe by MAC, stable signal order, honest empty"
    requirement: "BT-01"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/bluetooth.rs#duplicate_macs_dedupe_with_strongest_signal"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/bluetooth.rs#ordering_is_stable_by_signal_at_render"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/bt_cmds.rs#scan_table_numbers_rows_with_name_mac_and_signal"
        status: pass
      - kind: integration
        ref: "live `archel bt scan --json` with hci0 owned: 3-device array with address+rssi, exit 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "pair/connect/disconnect accept picks from the scanned set only; free-form MAC rejected as DeviceNotFound"
    requirement: "BT-02"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/bluetooth.rs#pick_validator_accepts_in_list_index_and_mac"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/bluetooth.rs#pick_validator_rejects_free_form_mac_and_garbage"
        status: pass
      - kind: integration
        ref: "live `archel bt pair 00:11:22:33:44:55` prints list then DeviceNotFound catalog line"
        status: pass
    human_judgment: false
  - id: D3
    description: "pair abort path spawns nothing and returns ConfirmationAborted with no auto-accept fallback"
    requirement: "BT-02"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/bluetooth.rs#pair_abort_path_spawns_nothing_without_auto_accept"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/bluetooth.rs#pair_confirm_path_reaches_pair_step_once"
        status: pass
      - kind: other
        ref: "grep: no auto-accept constructor (no request_default:true, no all-None Agent construction in crates/)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Real-peer pairing with PIN shown and explicit [o/N] confirm against trusted hardware"
    requirement: "BT-02"
    verification: []
    human_judgment: true
    rationale: "Live scan ran against ambient devices but pairing with an unknown/random peer is unsafe and was not attempted; needs a human-owned test peer plus verify-work session"

# Metrics
duration: 45min
completed: 2026-10-07
status: complete
---

# Phase 04 Plan 03: Bluetooth Scan plus Confirm-Gated Pair Summary

**Timed `bt scan` live list plus scan-then-pick pair/connect/disconnect behind a confirm-only bluer agent, proven live against hci0**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-10-07T (continuation session)
- **Completed:** 2026-10-07
- **Tasks:** 3 (tracer + blocking-human checkpoint + auto/TDD)
- **Files modified:** 7

## Checkpoint Outcome

**Task 2 (blocking-human): RESOLVED — "Daemon active confirmed".** Verified live this
session before implementing Task 3: `systemctl is-active bluetooth` → `active`,
`busctl tree org.bluez` → `/org/bluez/hci0` owned. Live scan evidence below was
captured against that daemon; no pairing was attempted against unknown peers.

## Accomplishments

- `archel bt scan` runs the 12 s timed BlueZ discovery sweep into a numbered French
  live list (name + MAC + signal on every row), dedupes by MAC (strongest RSSI
  wins), orders stable by signal at render, honest-empty when the air is silent
- `archel bt pair|connect|disconnect <pick>` each scan first, print the list, then
  validate the pick client-side (1-based index or in-list MAC) — free-form MACs
  fail as typed `DeviceNotFound` with the scan-then-pick catalog line
- Pairing registers a confirm-only bluer agent (`request_confirmation` surfaces the
  zero-padded code + `[o/N]`; all other handlers `None` = reject; `request_default`
  false; handle dropped after the attempt), 30 s ceiling → `PairingTimeout`
- Pair-then-connect offer reuses `confirm_action` (NeedsTty when piped); every
  mutation refuses without a TTY before anything spawns
- Typed `DeviceNotFound` / `PairingRejected` / `PairingTimeout` with French catalog
  arms; PIN compared constant-shape (`passkeys_equal` over all four octets) and
  displayed only for confirm, never logged or snapshotted
- Live evidence with hci0: `bt scan --json` returned a real 3-device array
  (address+rssi, exit 0); French table shows numbered rows with `-80 dBm` ordering
  and `N/A` (never fake 0); `bt pair 00:11:22:33:44:55` printed the list then the
  `DeviceNotFound` French line
- Full workspace suite green: 163 (core) + 72 (cli) + 18 (privileged)

## Task Commits

Each task was committed atomically:

1. **Task 1: End-to-end bt devices degrade path plus bluer agent spike** - `8db1e8d` (feat, prior session — verified intact via git log before touching anything)
2. **Task 2: Enable bluetooth.service for live proof** - checkpoint, no commit (outcome recorded above)
3. **Task 3: Scan plus confirm-gated pair, connect, disconnect** - `d6a171b` (feat)

_Note: TDD task in one atomic commit (RED confirmed via 12 compile errors, then GREEN)._

## Files Created/Modified

- `crates/archel-core/src/bluetooth.rs` - scan sweep, resolve_pick, passkey helpers, pair_picked_with gate, live pair/connect/disconnect, catalog prompt builders, 6 new tests
- `crates/archel-core/src/error.rs` - DeviceNotFound, PairingRejected, PairingTimeout variants + constructors + catalog arms + every_variant entries
- `crates/archel-core/locales/fr.json` - 7 bt.* prompts/ok-lines + 3 errors.* catalog strings
- `crates/archel-core/Cargo.toml` - futures-util 0.3 direct dep (StreamExt; official futures-rs crate, already in lockfile via bluer)
- `Cargo.lock` - futures-util promoted to direct
- `crates/archel/src/cmds/bt_cmds.rs` - run_scan/run_pair/run_connect/run_disconnect, numbered scan table, 2 render tests
- `crates/archel/src/cmds/mod.rs` - BtAction Scan/Pair/Connect/Disconnect + dispatch

## Decisions Made

- Each mutating bt command re-scans first so the validated set is always the
  just-printed list (simplest faithful reading of D-06; costs one 12 s sweep per
  pair/connect/disconnect)
- `request_default: false` — bluer docs confirm a registered agent serves the
  application's own actions, so no need to steal the desktop default agent
- Connect/disconnect bluer failures map to `Internal` (log-only detail): the plan
  scoped typed variants to pairing; transport errors keep the generic path
- `check_passkey_shape` (6-digit range + zero-padded round-trip) gates the code
  before display — gives `passkeys_equal` a genuine live call site instead of a
  test-only helper
- `resolve_pick` trims surrounding whitespace (paste-friendly) — membership check
  still applies post-trim, so the threat property is unchanged

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added futures-util direct dependency for stream combinators**
- **Found during:** Task 3 (live scan sweep needs `Stream::next`)
- **Issue:** `discover_devices` returns `impl Stream`; polling needs `StreamExt`, and
  neither tokio nor the existing dep set provides it
- **Fix:** Pinned `futures-util = "0.3"` (official futures-rs crate, already vendored
  in the lockfile via bluer — no new supply-chain surface)
- **Files modified:** crates/archel-core/Cargo.toml, Cargo.lock
- **Verification:** `cargo test --workspace` green; no new crate versions introduced
- **Committed in:** d6a171b (part of task commit)

**2. [Rule 1 - Bug] Fixed duplicated module header in bt_cmds.rs mid-task**
- **Found during:** Task 3 (workspace build failed: E0753/E0252)
- **Issue:** The run_scan/run_pair edit inserted a second `//!` header + duplicate
  imports/fns instead of extending the file
- **Fix:** Merged into a single module header, one import block, one of each fn
- **Files modified:** crates/archel/src/cmds/bt_cmds.rs
- **Verification:** Full workspace suite green after fix
- **Committed in:** d6a171b (part of task commit)

**3. [Rule 1 - Bug] Fixed FnOnce shape mismatch for the pair step**
- **Found during:** Task 3 (E0631: `pair_device(&BtDevice)` vs `FnOnce(BtDevice)`)
- **Issue:** Live `pair_device` borrows; the injectable gate owns the picked device
- **Fix:** Adapter closure `|device: BtDevice| async move { pair_device(&device).await }`
  at the single live call site; gate keeps the own-and-pass shape tests cover
- **Files modified:** crates/archel/src/cmds/bt_cmds.rs
- **Verification:** `cargo test --workspace` green
- **Committed in:** d6a171b (part of task commit)

---

**Total deviations:** 3 auto-fixed (1 blocking-dep, 2 correctness/bugs)
**Impact on plan:** All required for a compiling, tested slice. No scope creep; no
architectural change (no new tables/services; agent shape matches the Task 1 spike).

## Issues Encountered

- Live scan aliases render dash-separated (`5B-4F-…`) — that is BlueZ-supplied alias
  data, passed through honestly, not a formatting bug
- `bt pair <bad-pick>` exit code was masked by the `| tail` evidence pipe; the
  rejection line itself is the proof, and `DeviceNotFound` is covered by unit + the
  printed catalog line

## Threat Flags

None beyond the plan's register — T-4-05 mitigated (no auto-accept constructor:
only one `Agent` construction site and it sets `request_confirmation`;
scan-then-pick enforced in `resolve_pick`, the sole path to all three mutations),
T-4-06 mitigated (constant-shape compare, code in prompt string only, error
payloads carry name+MAC labels, never the code).

## Known Stubs

None. Live-peer pairing (D4) is queued as manual verify-work, not a stub — the
headless slice is complete and covered.

## User Setup Required

None - no external service configuration required (checkpoint already had the user
enable bluetooth.service; hci0 confirmed owned).

## Next Phase Readiness

- BT-01/BT-02 headless slice complete; GUI phase can bind `bt_scan`/`resolve_pick`/
  pair-status rows to the Bluetooth tab live list
- Manual verify-work: pair + connect + disconnect against a human-owned peer
  (exercise the `[o/N]` code confirm and the pair-then-connect offer)

## Self-Check: PASSED

- `bt_scan`, `resolve_pick`, `pair_picked_with`, `pair_device`, `connect_device`,
  `disconnect_device` present in `crates/archel-core/src/bluetooth.rs` — FOUND
- `DeviceNotFound`, `PairingRejected`, `PairingTimeout` in `error.rs` with catalog
  arms — FOUND (`cargo test -p archel-core error` via workspace run green)
- Commits `8db1e8d` + `d6a171b` in `git log` — FOUND
- Workspace suite: 163 + 72 + 18 passed, 0 failed — GREEN
- `archel bt devices --json` / `bt scan --json` parse — VERIFIED live

---
*Phase: 04-connectivity-devices*
*Completed: 2026-10-07*
