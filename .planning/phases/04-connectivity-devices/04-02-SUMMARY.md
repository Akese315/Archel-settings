---
phase: 04-connectivity-devices
plan: 02
subsystem: network
tags: [proxy, gsettings, vpn, wireguard, openvpn, nmcli, rpassword, cli, secrets-redaction]
status: complete

# Dependency graph
requires:
  - phase: 04-01
    provides: net foundation (zbus-direct NM reads, nmcli fallback, confirm-then-execute precedent, spawn discipline)
provides:
  - "archel net proxy bare status one-liner + export hint over gsettings org.gnome.system.proxy"
  - "archel net proxy off/manual/auto transitions with validators, auth flags, confirm-gated writes"
  - "archel net vpn list/up/down/import with path confinement, plugin delegation, --json"
  - "rpassword no-echo secret prompting precedent (never argv/stored/logged)"
affects: [04-03, 04-04, gui-phase]

# Actuals (#2632) — same chars/4 scale as the plan estimate.
actuals:
  tokens: 24000
  tasks: 3
  commits: 3

# Measured commit ledger (#3968)
plan_head_before: 847a5f3d98c7d0f27d3d758fbf28b693ddf46f77
plan_head_after: e4870d6a33cf2b09814baec672d2dde3a6f94ee2

# Tech tracking
tech-stack:
  added: [rpassword 7.5.4 (no-echo stdin prompts for proxy/VPN secrets)]
  patterns: [gsettings CLI store access (get/set, GVariant quoting), pure argv-builder fns unit-tested without spawning, secret-free Serialize structs + redacted Debug, nmcli --ask secret delegation, canonical-path confinement]

key-files:
  created:
    - crates/archel-core/src/proxy.rs
    - crates/archel/src/cmds/snapshots/archel__cmds__net_cmds__tests__proxy_report_json_schema_pins_apply_fields_without_secrets.snap
    - crates/archel/src/cmds/snapshots/archel__cmds__net_cmds__tests__vpn_list_json_schema_pins_connection_fields.snap
    - crates/archel/src/cmds/snapshots/archel__cmds__net_cmds__tests__vpn_action_report.snap
    - crates/archel/src/cmds/snapshots/archel__cmds__net_cmds__tests__vpn_import_report.snap
  modified:
    - crates/archel-core/src/network.rs
    - crates/archel-core/src/error.rs
    - crates/archel-core/src/lib.rs
    - crates/archel-core/locales/fr.json
    - crates/archel-core/Cargo.toml
    - crates/archel/src/cmds/net_cmds.rs
    - crates/archel/src/cmds/mod.rs
    - Cargo.lock

key-decisions:
  - "Missing gsettings maps to typed NetworkUnavailable (tracer) — no new error variant for a missing binary, mirrors the checker-missing precedent"
  - "Proxy passwords travel as in-process gsettings-set argv elements (never shelled, never logged); VPN secrets stay inside nmcli --ask prompting — Archel never sees key material on the VPN path"
  - "VPN up/down apply by UUID (resolved once pre-confirm), import applies by validated path"
  - "rpassword 7.5.4 added after cargo-search verification (plan-mandated version check, matches)"

patterns-established:
  - "gsettings reads/writes: which-gate, exact argv never through a shell, LC_ALL=C, wait_timeout plus kill, stderr log-only (mirrors the nmcli shape)"
  - "GVariant quoting: validated values wrapped in single quotes; validators reject quote characters so quoting is total"
  - "ProxyPlan carries the password in memory only: no Serialize derive, manual Debug shows password_set bool"
  - "VPN terse rows split from the right (UUID/TYPE are colon-free) with escaped-colon unmasking for names"
  - "Boundary-safe prefix confinement (exact-or-slash) checked on raw AND canonicalized paths against /proc, /sys, /dev"

requirements-completed: [NET-04, NET-05]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "bare archel net proxy prints one-line French state plus shell export hint; --json emits the secret-free ProxyState schema"
    requirement: "NET-04"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/proxy.rs#status_line_is_single_line_with_endpoint, status_line_flattens_exotic_newlines, export_hint_is_non_empty_french_guidance, parsers_cover_gsettings_output_shapes, missing_binary_maps_to_typed_unavailable"
        status: pass
      - kind: e2e
        ref: "./target/debug/archel net proxy (live: one-line state + hint, rc=0); net proxy --json | python3 -m json.tool (parses, rc=0)"
        status: pass
    human_judgment: false
  - id: D2
    description: "net proxy off/manual/auto transitions with host/port/PAC validation, exceptions + auth flags, confirm-gated writes, secret-free reports"
    requirement: "NET-04"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/proxy.rs#argv_builder_maps_modes_to_exact_gsettings_sequences, host_port_and_pac_validators_reject_garbage, secret_redaction_keeps_passwords_out_of_debug_json_and_reports"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/net_cmds.rs#proxy_preview_names_mode_and_endpoint, proxy_report_json_schema_pins_apply_fields_without_secrets (insta snapshot), proxy_confirm_abort_spawns_no_write"
        status: pass
      - kind: e2e
        ref: "piped manual shows preview then NeedsTty refusal with no mutation; port 0 and malformed PAC fail pre-spawn with ProxyInvalid; gsettings mode still 'none' after all probes"
        status: pass
    human_judgment: false
  - id: D3
    description: "net vpn list/up/down/import with path confinement, plugin delegation, typed plugin-missing path, honest-empty list, secret-free --json"
    requirement: "NET-05"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/network.rs#vpn_import_tests::import_validator_accepts_configs_and_rejects_everything_else, import_errors_map_plugin_missing_polkit_and_generic_shapes, empty_vpn_list_serializes_to_honest_empty_array"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/net_cmds.rs#vpn_empty_table_is_honest_without_fake_rows, vpn_list_table_names_every_tunnel_and_state, vpn_list_json_schema_pins_connection_fields (insta), vpn_reports_json_pin_action_and_import_fields (2 insta), vpn_action_confirm_abort_spawns_no_write"
        status: pass
      - kind: e2e
        ref: "./target/debug/archel net vpn list (live: real wg0 wireguard actif, rc=0); list --json parses; piped up refuses with no mutation; /proc + missing + wrong-ext imports refuse pre-spawn with VpnPathInvalid"
        status: pass
    human_judgment: false
  - id: D4
    description: "live proxy mode cycling against the real gsettings store (mutates session state)"
    requirement: "NET-04"
    verification: []
    human_judgment: true
    rationale: "Automation proves planning, validation and refusal; actually flipping the session proxy mutates live user state and needs a human to run and revert"
  - id: D5
    description: "live VPN connect plus config-file import against a real tunnel (needs plugin + tunnel)"
    requirement: "NET-05"
    verification: []
    human_judgment: true
    rationale: "Up/down/import mutate live tunnels and need the openvpn plugin plus a real config; automation covers everything up to the spawn"

# Metrics
duration: ~55min
completed: 2026-10-07
---

# Phase 04 Plan 02: Proxy Modes + VPN Management Summary

**Session-layer proxy (`off`/`manual`/`auto`) over the gsettings store plus multi-protocol VPN `list`/`up`/`down`/`import` on the NM path — pure argv builders, client-side validators, confirm-gated writes, secrets never in argv/logs/snapshots/`--json`**

## Performance

- **Duration:** ~55 min (2026-10-07)
- **Tasks:** 3 of 3 (tracer + 2 TDD tasks, each committed atomically)
- **Files modified:** 13 (2 new source modules sections, 1 new core module, 5 new snapshots)

## Accomplishments

- Bare `archel net proxy` reads `org.gnome.system.proxy` through the `gsettings` CLI (mode, per-scheme host/port, autoconfig-url, ignore-hosts) and prints a guaranteed-one-line French state plus the Hyprland-terminal export hint; `--json` emits the secret-free `ProxyState` schema
- `net proxy off`, `net proxy manual <host> <port>` (`--exceptions`, `--user`, `--password` no-echo flag) and `net proxy auto <pac-url>`: validated client-side (`ProxyInvalid` before any prompt/spawn), planned read-only, previewed, confirm-gated (`[o/N]`, `NeedsTty` when piped), applied as exact `gsettings set` sequences
- `net vpn list` joins all-profiles plus active-set terse output into honest VPN rows (empty store renders the empty line + `[]`, never fake rows); `up`/`down` resolve exact name/UUID once (ambiguity refused) and apply by UUID; `import` confines the path (exists, is-file, `.ovpn`/`.conf`, no `/proc`/`/sys`/`/dev` even via symlinks) then delegates to `nmcli --ask connection import` — `.ovpn`/`.conf` never parsed in Rust, secrets never touch Archel
- Typed errors `ProxyInvalid`, `VpnPathInvalid`, `VpnPluginMissing` (install hint), `VpnImportFailed` with French catalog renderings and `every_variant` coverage; polkit denials still map to `PolkitDenied` on both new write paths
- Full workspace suite green headless (`67 + 152 + 18`), diacritics guard clean, snapshot grep confirms zero secret material

## Task Commits

Each task was committed atomically:

1. **Task 1: End-to-end proxy status one-liner through gsettings** - `291f399` (feat)
2. **Task 2: Proxy mode transitions with optional auth** - `8522a90` (feat)
3. **Task 3: VPN list, up/down, and config-file import** - `e4870d6` (feat)

## Files Created/Modified

- `crates/archel-core/src/proxy.rs` - gsettings status reads, pure parsers, validators, plan/argv/apply, secret-free reports, 10 unit tests (tasks 1+2)
- `crates/archel-core/src/network.rs` - `pub mod vpn`: parsers, resolver, argv builders, path confinement, import error map, French row helpers, 3 unit tests (task 3)
- `crates/archel/src/cmds/net_cmds.rs` - `run_proxy*` + `run_vpn_*` runners with injectable-closure confirm gates, list tables, 9 CLI tests + 4 insta snapshots (tasks 1+2+3)
- `crates/archel/src/cmds/mod.rs` - `Proxy` (optional-subcommand) + `Vpn` groups, dispatch, completions words (tasks 1+2+3)
- `crates/archel-core/src/error.rs` - `ProxyInvalid`, `VpnPathInvalid`, `VpnPluginMissing`, `VpnImportFailed` variants + constructors path + catalog arms + `every_variant` entries (tasks 2+3)
- `crates/archel-core/locales/fr.json` - `net.proxy_*`, `net.vpn_*`, `net.label_*`, `net.confirm_*` keys + 4 `errors.*` keys (tasks 1+2+3)
- `crates/archel-core/src/lib.rs` - `pub mod proxy` (task 1)
- `crates/archel-core/Cargo.toml`, `Cargo.lock` - `rpassword 7.5.4` (task 2, version verified via `cargo search` per plan)

## Decisions Made

- Missing `gsettings` maps to the existing typed `NetworkUnavailable` (tracer) — no new variant for a missing binary, mirrors the `checker_missing_hint` precedent from `updates.rs`
- Proxy passwords travel as in-process `gsettings set` argv elements (exact argv, never shelled, never logged — stderr stays log-only and never echoes argv); VPN secrets stay inside `nmcli --ask` prompting so Archel never sees key material on that path
- VPN up/down resolve once pre-confirm and apply by UUID (no post-confirm re-resolution that could hit ambiguity); import applies by validated path
- `rpassword 7.5.4` added after `cargo search` verification (plan-mandated version check, exact match); prompt failures map to typed `NeedsTty`
- `VpnImportFailed` carries only the file name (not raw plugin stderr) — stderr shapes stay in `Display`/logs per T-1-02, same rule as `DnsInvalid` echoing only user input

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical] Proxy catalog keys landed in the tracer instead of task 2**
- **Found during:** Task 1 (tracer)
- **Issue:** Plan scoped `fr.json` to task 2, but the tracer's own done criteria (French one-line state + export hint from catalog keys, D-05/D-07) require catalog strings on the first path
- **Fix:** Added the `net.proxy_*` status keys (`proxy_label`, mode words, `proxy_export_hint`) with the tracer; task 2 added only transition/confirm keys
- **Files modified:** `crates/archel-core/locales/fr.json`
- **Verification:** Tracer tests render French through the catalog; full suite green
- **Committed in:** `291f399` (part of task commit)

**2. [Rule 1 - Bug] insta custom snapshot names take the name-first position**
- **Found during:** Task 3 (VPN snapshots)
- **Issue:** Two snapshots in one test used `assert_json_snapshot!(value, @"name")` (inline-snapshot syntax) then `(value, "name")` — neither compiles under insta 1.48
- **Fix:** `assert_json_snapshot!("vpn_action_report", value)` name-first form; reviewed both `.snap.new` files (full schemas, no secrets) before accepting, plus the auto-named list snapshot
- **Files modified:** `crates/archel/src/cmds/net_cmds.rs`, 3 new `.snap` files
- **Verification:** Full workspace suite green after accept cycle
- **Committed in:** `e4870d6` (part of task commit)

**3. [Rule 1 - Bug] Borrow of a temporary argv Vec in `vpn_up`**
- **Found during:** Task 3 (E0716 compile error)
- **Issue:** `vpn_up_argv(...).iter().collect()` borrowed a dropped temporary
- **Fix:** Bound the `Vec<String>` to a local before building `&[&str]`
- **Files modified:** `crates/archel-core/src/network.rs`
- **Verification:** Compiles warning-free; unit + e2e green
- **Committed in:** `e4870d6` (part of task commit)

---

**Total deviations:** 3 auto-fixed (2 bugs, 1 missing-critical)
**Impact on plan:** All required for correctness/compilation of the plan scope. No scope creep — no GUI work, no daemon, no `.ovpn`/`.conf` parsing, no NM-key access from proxy code.

## Issues Encountered

- TDD RED evidence for tasks 2 and 3 captured as compile failures (`ProxyPlan`/validators missing, `super::vpn` unresolved) before GREEN implementations — recorded in tool output, resolved within the same task commits
- One stale insta `.pending-snap` artifact from the wrong-syntax snapshot attempts removed (untracked test artifact of this task, not a repo file)
- Two accidental whitespace-only edits (removed blank lines) left in test modules — cosmetic, fmt-clean, kept to avoid churn
- Live `net vpn list` shows the real `wg0` (wireguard, active) read-only; `wg0` was never downed, modified, or imported over — reads plus refused mutations only

## Auth Gates

None — no external auth, login, or token step occurred. (`rpassword` is a library dependency verified via `cargo search`, not an auth gate; the T-4-SC package-legitimacy concern does not apply: the plan explicitly mandated this crate at the verified version.)

## Threat Flags

None beyond the plan's threat model — no new surface introduced:

- Import paths cross into nmcli plugin execution only after confinement (exists, is-file, extension allowlist, raw + canonical prefix gate) with display-before-activate confirm; file contents never cross into Archel (T-4-03 mitigated)
- Proxy passwords: no-echo prompt at use time, in-memory plan only, redacted `Debug`, secret-free `Serialize`; VPN secrets never enter Archel at all (`--ask` delegation); snapshot grep confirms zero secret material (T-4-04 mitigated)
- One new dependency (`rpassword` 7.5.4): version verified via `cargo search` per plan, well-known crate, no ASSUMED/SUS status (T-4-SC honored)

## Next Phase Readiness

- Plan 02 is shippable: proxy + VPN proven headlessly with unit + snapshot coverage plus live reads and refusal-path e2e
- Live proxy mode cycling (actual `off`/`manual`/`auto` against the session store) and live VPN connect + import remain manual verify-work items (mutate live session/tunnel state)
- Plans 04-03/04 build on the `proxy`/`vpn` modules, the confirm-gated write shape, and the `--ask` secret-delegation precedent established here

## Self-Check: PASSED

- All created files exist (`proxy.rs`, 4 snapshots listed above, SUMMARY)
- Commits `291f399`, `8522a90`, `e4870d6` exist with no deletions, no untracked task files left behind (stale `.pending-snap` removed)
- Live proof re-runnable: `./target/debug/archel net proxy --json`, `net vpn list --json` (both parse, rc=0 at completion time)
- Workspace suite green: 67 + 152 + 18, diacritics guard clean

---
*Phase: 04-connectivity-devices*
*Completed: 2026-10-07*
