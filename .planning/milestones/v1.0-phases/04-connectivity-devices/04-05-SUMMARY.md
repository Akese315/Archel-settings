---
phase: 04-connectivity-devices
plan: 05
subsystem: network
tags: [rust, networkmanager, nmcli, vpn, wireguard, uat-gap-closure]

# Dependency graph
requires:
  - phase: 04-02
    provides: VPN list/up/down/import path (VpnConnection, vpn_down, run_vpn_down)
provides:
  - Provenance-aware vpn list (persistent flag in table + --json)
  - Ephemeral-profile warning in vpn down preview and prompt
  - Post-deactivate read-back in vpn_down via vpn_down_argv single builder
  - Headless regression tests pinning all three G-04-2 behaviors
affects: [04-UAT.md G-04-2 reconciliation, GUI network tab, verify-work]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
actuals:
  tokens: 6100
  tasks: 2
  commits: 3

# Tech tracking
tech-stack:
  added: []
  patterns: [FILENAME-derived persistence flag, post-deactivate read-back, warn-and-continue confirm preview]

key-files:
  created: []
  modified: [crates/archel-core/src/network.rs, crates/archel/src/cmds/net_cmds.rs, crates/archel-core/locales/fr.json]

key-decisions:
  - "persistent as bool on VpnConnection (not Stored|Generated|Assumed enum): binary stored/ephemeral answers the gap truth with minimum schema churn"
  - "Warn-and-continue through the existing [o/N] gate, no new refusal error type"
  - "Read-back failure degrades honestly to active:false instead of failing the deactivation"

patterns-established:
  - "Provenance flags derive from display-safe fields only — raw FILENAME paths never reach output, JSON, or snapshots (T-4-05)"
  - "Single argv builder per mutation (vpn_down_argv) — inline argv vectors are a drift defect"

requirements-completed: [NET-05]

coverage:
  - id: D1
    description: "vpn list shows persistent vs ephemeral provenance in table and --json"
    requirement: "NET-05"
    verification:
      - kind: unit
        ref: "crates/archel/src/cmds/net_cmds.rs#vpn_list_table_names_every_tunnel_and_state"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/net_cmds.rs#vpn_list_json_schema_pins_connection_fields"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/network.rs#provenance_parser_classifies_empty_dash_run_and_keyfile"
        status: pass
    human_judgment: false
  - id: D2
    description: "vpn down on ephemeral profile warns (tracking loss + vpn import hint) in preview and prompt"
    requirement: "NET-05"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/network.rs#down_preview_warns_for_ephemeral_and_stays_quiet_for_stored"
        status: pass
    human_judgment: false
  - id: D3
    description: "vpn_down deactivates via vpn_down_argv only (no delete verb) and reports post-deactivate truth"
    requirement: "NET-05"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/network.rs#down_argv_is_exactly_connection_down_uuid_with_no_delete_verb"
        status: pass
      - kind: other
        ref: "grep confirms no delete verb in the vpn down path (only test/doc mentions)"
        status: pass
    human_judgment: false

# Metrics
duration: 25min
completed: 2026-10-07
status: complete
---

# Phase 04 Plan 05: VPN Provenance + Ephemeral-Down Warning Summary

**Provenance-aware `vpn list` (stocké/éphémère in table and --json), tracking-loss warning with `vpn import` hint on ephemeral `vpn down`, and post-deactivate read-back through the single `vpn_down_argv` builder — closing UAT gap G-04-2 (NET-05)**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-10-07T00:20:00Z
- **Completed:** 2026-10-07T00:45:00Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- `archel net vpn list` queries `NAME,UUID,TYPE,FILENAME` and marks every row stored (`stocké`) or ephemeral (`éphémère`) in both the human table and `--json` (`persistent` field) — an externally-created tunnel like wg0 is never mistaken for a stored profile again
- `archel net vpn down <ephemeral>` preview and confirm prompt warn that deactivation drops NM tracking and `vpn up` cannot restore it, with the hint to make it persistent via `vpn import` first (warn-and-continue through the existing `[o/N]` gate)
- `vpn_down` routes solely through `vpn_down_argv` (`connection/down/uuid`, pinned by test) and re-queries the active set after deactivate instead of reporting hardcoded `active:false`
- All three gap behaviors pinned by headless regression tests — no `nmcli` spawn, no live wg0 touch; full workspace suite green (83 + 186 + 18 tests)

## Task Commits

Each task was committed atomically:

1. **Task 1: Provenance in vpn list end-to-end (FILENAME query to output)** - `a29ddf5` (feat)
2. **Task 2: Ephemeral down warning plus honest read-back and builder routing** - `3fb5336` (feat)

**Plan metadata:** `docs commit` (docs: complete plan, includes this SUMMARY)

## Files Created/Modified

- `crates/archel-core/src/network.rs` - `VpnConnection.persistent` + `is_persistent_filename`, 4-field `parse_vpn_row` (escaped-colon right-split kept), FILENAME queries, provenance row, `vpn_ephemeral_warning` / `down_preview_warning_rows` / `confirm_vpn_down_prompt`, `vpn_down` read-back via `vpn_down_argv`, `vpn_provenance_tests`
- `crates/archel/src/cmds/net_cmds.rs` - `run_vpn_down` warns via extra preview rows + warned prompt, `run_vpn_action_confirmed` carries `extra_preview_rows`, fixtures/schema/table tests pin `persistent`
- `crates/archel-core/locales/fr.json` - `label_provenance`, `vpn_stored`, `vpn_ephemeral`, `label_warning`, `vpn_ephemeral_warning` (French, catalog-owned)
- `crates/archel/src/cmds/snapshots/...vpn_list_json_schema...snap` - refreshed: only the new `persistent` flag, secret-free

## Decisions Made

- `persistent: bool` on `VpnConnection` rather than a Stored|Generated|Assumed enum: the gap truth is binary (re-activatable or not) and a bool keeps the `--json` schema churn to one field.
- Warn-and-continue through the existing `[o/N]` gate instead of a new refusal error type: the operator stays in control, and the plan explicitly allowed the gate to carry the warning.
- `parse_active_uuid` tolerates both 4-field and legacy 3-field `--active` rows (separator count decides the UUID position) so old fixtures and skewed daemons still join.
- Read-back failure degrades honestly to `active: false` rather than failing the deactivation report.

## Deviations from Plan

None - plan executed exactly as written. (TDD RED demonstrated honestly: the three Task 2 tests were added first and failed compilation on the missing `down_preview_warning_rows` / `confirm_vpn_down_prompt` / `vpn_ephemeral_warning` imports before the GREEN implementation.)

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: information-disclosure-reviewed | crates/archel-core/src/network.rs | FILENAME path string crosses into display/JSON — mitigated per T-4-05: only the derived `persistent` flag is surfaced, raw path never leaves the parser; snapshot reviewed secret-free |
| threat_flag: dos-reviewed | crates/archel/src/cmds/net_cmds.rs | `vpn down` on ephemeral profile — mitigated per T-4-06: preview + prompt warning with import hint, post-deactivate read-back, zero delete verb in the down path |

## Known Stubs

None.

## Issues Encountered

None - the only failing test during execution was the expected insta snapshot diff for the new `persistent` field (accepted after verifying the diff contained only the new flag, no secrets).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- G-04-2 behaviors are code-complete and regression-pinned; the orchestrator reconciles `04-UAT.md` (re-run of live UAT test 2 against a real ephemeral profile still needs a human with a tunnel to sacrifice — automation cannot down a live tunnel).
- GUI network tab (later phase) can consume `VpnConnection.persistent` directly for the same provenance marking.

---
*Phase: 04-connectivity-devices*
*Completed: 2026-10-07*

## Self-Check: PASSED

- `crates/archel-core/src/network.rs` FOUND; `crates/archel/src/cmds/net_cmds.rs` FOUND; `crates/archel-core/locales/fr.json` FOUND; `04-05-SUMMARY.md` FOUND
- Commits `a29ddf5`, `3fb5336` FOUND in `git log`
- `cargo test -p archel-core --no-default-features network::vpn` green (6 passed); `cargo test --workspace --no-default-features` green (83 + 186 + 18 passed)
- No `delete` verb in the vpn down path; no secrets in output/JSON/logs/snapshots; no live wg0 mutation in tests
