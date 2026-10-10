---
phase: 04-connectivity-devices
plan: 01
subsystem: network
tags: [networkmanager, zbus, tokio, nmcli, dbus, cli, dns, self-test]
status: complete

# Dependency graph
requires:
  - phase: 03-packages-cli-harness
    provides: confirm-then-execute precedent, run_capped_query spawn shape, French-table + insta CLI patterns
provides:
  - "archel net status end-to-end (D-Bus first, nmcli fallback) with French table + --json"
  - "archel net dns show/set/reset with strict IPv4 validation, preview + confirm, reactivation read-back"
  - "archel net test triple-hop raw-ms self-test (gateway, DNS, internet)"
  - "tokio async foundation in archel-core (single runtime at binary)"
  - "Wave 0 live-bus spike findings recorded in code (NM 1.58.1 member names, import dry-probes, auth-permission state)"
affects: [04-02, 04-03, 04-04, gui-phase]

# Actuals (#2632) — same chars/4 scale as the plan estimate.
actuals:
  tokens: 38000
  tasks: 3
  commits: 1

# Measured commit ledger (#3968)
plan_head_before: 56188ff9005ec41d88c74716ccfc3363fc7938d9
plan_head_after: 2583c78b7570487dd62a7e6593dd96a1388928a2

# Tech tracking
tech-stack:
  added: [tokio 1.53.2 (rt-multi-thread,time,macros), zbus 5.19.0]
  patterns: [D-Bus-first with subprocess fallback + source tag, proxy-per-object-path builder shape, terse-field nmcli parsing, plan-preview-confirm-apply for mutations, pure parsers over locale fixtures]

key-files:
  created:
    - crates/archel-core/src/network.rs
    - crates/archel/src/cmds/net_cmds.rs
  modified:
    - crates/archel-core/src/error.rs
    - crates/archel-core/src/lib.rs
    - crates/archel-core/locales/fr.json
    - crates/archel/src/cmds/mod.rs
    - crates/archel/src/main.rs
    - crates/archel/Cargo.toml
    - Cargo.lock

key-decisions:
  - "Stay zbus-direct for the whole plan (networkmanager 0.5.0 SUS verdict holds; human checkpoint RESOLVED as rejected — no wrapper spike run, no dependency added)"
  - "D-Bus-first with nmcli fallback and explicit source tag, so renderers always show provenance"
  - "Single #[tokio::main] at the binary; core exposes async fns, subprocess helpers stay sync via spawn_blocking"
  - "DNS mutations go through plan-preview-confirm-apply: read-only planning, [o/N] confirm via confirm_action (NeedsTty when piped), then modify + connection-up + effective-DNS read-back"
  - "Self-test reports raw ms only, unreachable hops are N/A/null never 0; a genuine dig-reported 0 msec (cache hit) is shown as measured"

patterns-established:
  - "NM reads: one zbus proxy per object path built from the primary-connection path outward (root -> Active -> IP4Config/Device -> AP)"
  - "Deprecated au Nameservers only fill gaps left by empty NameserverData (older daemons)"
  - "Localized nmcli words matched defensively (connected/connecté) with \\u escapes to satisfy the D-07 diacritics guard"
  - "NM writes: resolve profile client-side to UUID, build exact argv (never through shell), LC_ALL=C, wait_timeout plus kill, stderr scanned for polkit markers in EN + FR"
  - "Parsers stay label-agnostic (numbers around / and msec/ms markers) so C and FR locales parse identically"

requirements-completed: [NET-01, NET-02, NET-03, NET-06]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "archel net status prints connection state, interface, SSID, IP, gateway, DNS and profile in one French table"
    requirement: "NET-01"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/network.rs#status_rows_render_na_for_absent_fields"
        status: pass
      - kind: e2e
        ref: "./target/debug/archel net status (live NM 1.58.1, source=dbus, rc=0)"
        status: pass
    human_judgment: false
  - id: D2
    description: "archel net status --json emits the pinned v1 schema and parses"
    requirement: "NET-02"
    verification:
      - kind: unit
        ref: "crates/archel/src/cmds/net_cmds.rs#status_json_schema_pins_snapshot_fields (insta snapshot)"
        status: pass
      - kind: e2e
        ref: "./target/debug/archel net status --json | python3 -m json.tool (parses, rc=0)"
        status: pass
    human_judgment: false
  - id: D3
    description: "networkmanager wrapper crate decision (spike the wrapper vs stay zbus-direct)"
    requirement: ""
    verification:
      - kind: human
        ref: "blocking-human checkpoint resolved on resume: user_response = rejected (stay zbus-direct)"
        status: pass
    human_judgment: true
    rationale: "Blocking-human gate by plan design — default on no-answer is stay zbus-direct; the human explicitly rejected the wrapper, so no spike was run and no dependency was added"
  - id: D4
    description: "archel net dns set replaces profile DNS (D-01/D-02) and net dns reset restores DHCP (D-03), with preview, confirm, reactivation and effective-DNS read-back"
    requirement: "NET-03"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/network.rs#validator_accepts_plain_ipv4, validator_rejects_v6_hostnames_and_bad_quads, modify_argv_emits_replace_sequence_on_ipv4_keys_only, reset_argv_clears_dns_and_restores_dhcp, profile_resolution_prefers_uuid_and_rejects_ambiguity"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/net_cmds.rs#dns_preview_names_profile_and_old_to_new, dns_confirm_abort_spawns_no_write (injectable-closure abort gate)"
        status: pass
      - kind: e2e
        ref: "./target/debug/archel net dns show --json (live: configured=[], ignore_auto_dns=false, effective=[10.192.61.1], rc=0); piped set shows preview then NeedsTty refusal with no mutation; invalid input fails pre-spawn with DnsInvalid"
        status: pass
    human_judgment: false
  - id: D5
    description: "archel net test reports raw per-hop ms for gateway, DNS and internet hops (D-09/D-10), unreachable renders N/A never 0, no verdict strings"
    requirement: "NET-06"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/network.rs selftest#ping_avg_extracts_the_second_field_in_both_locales, ping_avg_ignores_packet_lines_and_loss_output, dig_ms_parses_msec_and_ms_units (C + FR fixtures)"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/net_cmds.rs#test_table_shows_raw_ms_and_na_without_verdicts, test_json_schema_pins_hop_fields (dead hop encodes null)"
        status: pass
      - kind: e2e
        ref: "./target/debug/archel net test --json (live triple: gateway + DNS + internet hops with ms figures, rc=0)"
        status: pass
    human_judgment: false
  - id: D6
    description: "net dns report and net test --json schemas are insta-pinned (mutation fields, hop fields)"
    requirement: "NET-03, NET-06"
    verification:
      - kind: unit
        ref: "archel__cmds__net_cmds__tests__dns_report_json_schema_pins_mutation_fields.snap, archel__cmds__net_cmds__tests__json_schema_pins_hop_fields.snap"
        status: pass
    human_judgment: false

# Metrics
duration: ~40min tracer + ~50min DNS/test completion
completed: 2026-10-07
---

# Phase 04 Plan 01: Net Status Tracer + DNS/Test Summary

**`archel net status|dns|test` complete end-to-end over zbus-direct NM access with nmcli fallback — French tables + pinned `--json` schemas, strict IPv4 DNS validation, preview-confirm-apply mutations with reactivation read-back, verdict-free triple self-test — COMPLETE (wrapper checkpoint rejected, zbus-direct confirmed)**

## Performance

- **Duration:** ~40 min tracer + ~50 min DNS/test completion (2026-10-06 → 2026-10-07)
- **Completed:** 2026-10-07
- **Tasks:** 3 of 3 (tracer committed; blocking-human gate resolved as rejected; DNS/test committed)
- **Files modified:** 11 + 2 new snapshots (tracer) and 9 + 2 new snapshots (DNS/test task)

## Accomplishments

- `archel net status` reads the primary NM connection live (state, interface, SSID, IP/prefix, gateway, effective DNS, profile name + UUID) via zbus D-Bus proxies, falling back to `nmcli -t -f` terse parsing when the bus is unreachable; the `source` field tags provenance
- First async in core: tokio 1.53.2 + zbus 5.19.0 pinned with version-source comments; single `#[tokio::main]` at the binary; subprocess helpers stay sync and run off-executor via `spawn_blocking`
- Wave 0 live-bus spike executed and recorded in `network.rs` module docs: `GetDevices`/primary-connection/`IP4Config` member names, AddressData/NameserverData preferred over deprecated `au` fields (u32 decode `20824074` -> `10.192.61.1` unit-tested), WireGuard/OpenVPN import dry-probes (openvpn plugin missing -> future `VpnPluginMissing`), `GetPermissions` shows `settings.modify.system: yes` so no denial string was observable, `wg0` never touched
- `archel net dns show/set/reset`: strict IPv4 validator (accepts `1.1.1.1`/`9.9.9.9`, rejects `::1`, hostnames, `999.1.1.1`, leading-zero octets) runs before any spawn; D-01 replace argv (`ipv4.dns` + `ipv4.ignore-auto-dns yes`) and D-03 reset argv (empty + `no`) touch only `ipv4.*` keys; active-profile UUID by default, `--profile` exact name/UUID match with ambiguity refused (T-4-01); old-to-new preview + `confirm_action` `[o/N]` (NeedsTty when piped); `connection up` reactivation with effective-DNS read-back (failed read-back degrades to `None`, never guessed)
- `archel net test`: D-09 triple (gateway `ping -c 3 -W 2`, `dig +time=2 +tries=1 archlinux.org`, internet `ping 1.1.1.1`) with label-agnostic pure parsers over C + FR fixtures; raw ms only, unreachable hops render N/A / JSON `null`, never `0 ms`; no verdict/diagnosis strings anywhere on the path
- Typed errors `ProfileNotFound`, `DnsInvalid`, `PolkitDenied` (English Display, French catalog arms, `every_variant` entries); polkit-denial markers cover EN + FR stderr shapes with the hyprpolkitagent hint; writes never route through `archel-privileged` (T-4-02)
- Full workspace suite green headless (`cargo test --workspace --no-default-features`: 59 + 139 + 18), diacritics guard clean, no warnings, no runtime `unwrap` on failure paths

## Task Commits

Each task was committed atomically:

1. **Task 1: End-to-end net status tracer plus live-bus spike and async foundation** - `56188ff` (feat)
2. **Task 2: Gate the networkmanager wrapper crate decision** - resolved as **rejected (stay zbus-direct)**; no code, no commit (checkpoint-only task)
3. **Task 3: DNS set/reset plus triple self-test with validators** - `2583c78` (feat)

## Files Created/Modified

- `crates/archel-core/src/network.rs` - NM status reads (D-Bus + nmcli), pure parsers, French labels/rows, 15 unit tests (task 1) + `dns` module (validator, argv builders, profile resolution, plan/apply, show/report rows, 8 tests) + `selftest` module (ping/dig parsers, hop runner, 4 tests) + top-level `run_nm_write`/`map_nm_write_error` + polkit-mapping test (task 3)
- `crates/archel/src/cmds/net_cmds.rs` - `net status` group (table + --json), render helper, 2 tests + insta snapshot (task 1) + `dns show/set/reset`, `test` runners, confirm-then-execute gate with injectable apply, 4 tests + 2 insta snapshots (task 3)
- `crates/archel/src/cmds/snapshots/...status_json_schema_pins_snapshot_fields.snap` - pinned v1 `--json` schema (task 1)
- `crates/archel/src/cmds/snapshots/...dns_report_json_schema_pins_mutation_fields.snap` - pinned DNS report schema, no ipv6 keys (task 3)
- `crates/archel/src/cmds/snapshots/...json_schema_pins_hop_fields.snap` - pinned triple-hop schema, dead hop null (task 3)
- `crates/archel-core/src/error.rs` - `NetworkUnavailable { detail }` variant (task 1) + `ProfileNotFound`/`DnsInvalid`/`PolkitDenied` variants + constructors + catalog arms + `every_variant` entries (task 3)
- `crates/archel-core/locales/fr.json` - `net.*` section (title, labels, state wordings) + `errors.network_unavailable` (task 1) + DNS/test keys (titles, labels, confirms, hop names) + `errors.profile_not_found/dns_invalid/polkit_denied` (task 3)
- `crates/archel-core/src/lib.rs` - `pub mod network` (task 1)
- `crates/archel/src/cmds/mod.rs` - `Net` group + `Status` action + completions words; `run()` is now async (task 1) + `Dns` show/set/reset and `Test` actions (task 3)
- `crates/archel/src/main.rs` - single `#[tokio::main]` (task 1)
- `crates/archel-core/Cargo.toml`, `crates/archel/Cargo.toml`, `Cargo.lock` - tokio/zbus pins (binary: runtime + macros only); `serde` added to the `archel` bin for the `Serialize` encode bound (task 3)

## Decisions Made

- Stay zbus-direct for the tracer; the `networkmanager` 0.5.0 wrapper stays out (SUS legitimacy verdict from research) pending the human checkpoint below (task 1)
- **Checkpoint resolution: human REJECTED the wrapper spike — stay zbus-direct for the whole plan. No spike was run, no `networkmanager` dependency was added (T-4-SC honored).**
- `rpassword` 7.5.4 version verified via `cargo search` at plan time but NOT added (VPN/proxy-auth prompt belongs to a later plan)
- `NetworkUnavailable` added in the tracer although the plan scoped `error.rs` to the DNS task — minor anticipation (deviation 3)
- No `--watch` on `net status` (connection state is not a 500 ms tick); first non-loopback active connection wins in the nmcli fallback (D-Bus primary is exact)
- DNS mutations go through plan-preview-confirm-apply: read-only planning (validate → resolve → read old), preview, `[o/N]` confirm, then modify + `connection up` + effective-DNS read-back (task 3, mirrors `run_aur` precedent)
- A genuine dig-reported `0 msec` (local stub cache hit) renders as measured `0.00 ms` — the no-fake-zero rule targets unreachable hops, which still render N/A/null (observation, not a bug)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] French diacritics in Rust sources broke the D-07 guard test**
- **Found during:** Task 1 (tracer)
- **Issue:** Live SSID fixture (`KLEY Rennes privée`) and localized-word matching (`connecté`) tripped `i18n::tests::no_hardcoded_french_diacritics_in_rust_sources`
- **Fix:** All non-ASCII literals rewritten as `\u{...}` escapes (runtime-identical); prose reworded to ASCII
- **Files modified:** `crates/archel-core/src/network.rs`, `crates/archel/src/cmds/net_cmds.rs`
- **Verification:** Full core suite green (126 passed at the time)
- **Committed in:** `56188ff` (part of task commit)

**2. [Rule 3 - Blocking] zbus `Builder::path()` returns `Result`, not `Builder`**
- **Found during:** Task 1 (tracer)
- **Issue:** Proxy-builder chains for dynamic object paths did not compile under zbus 5.19.0
- **Fix:** `let-else` on the builder step at all four dynamic-path sites (root proxy uses `::new` with defaults)
- **Files modified:** `crates/archel-core/src/network.rs`
- **Verification:** Compiles warning-free; live D-Bus read succeeds
- **Committed in:** `56188ff` (part of task commit)

**3. [Rule 2 - Missing critical] `NetworkUnavailable` added in the tracer instead of the DNS task**
- **Found during:** Task 1 (tracer)
- **Issue:** Plan scoped `error.rs` to task 3, but the tracer's own done criteria demand a typed (non-`Internal`) error on NM-absent machines
- **Fix:** Added the single variant + constructor + catalog key + `every_variant` entry now; remaining variants (`DnsInvalid`, `PolkitDenied`, `ProfileNotFound`) landed with task 3
- **Files modified:** `crates/archel-core/src/error.rs`, `crates/archel-core/locales/fr.json`
- **Verification:** `every_variant_renders_non_empty_french` passes
- **Committed in:** `56188ff` (part of task commit)

**4. [Rule 2 - Missing critical] `map_nm_write_error` had no unit coverage for the T-4-02 polkit mapping**
- **Found during:** Task 3 completion audit (continuation)
- **Issue:** The EN/FR denial-marker mapping was implemented but untested — a locale-shaped denial could regress to a bare `Internal` error silently
- **Fix:** Added `write_denials_map_to_polkit_with_agent_hint` covering EN (`Not authorized`, `access denied`) and FR (`authentification requise`, `accès non autorisé`) shapes plus the non-denial passthrough
- **Files modified:** `crates/archel-core/src/network.rs`
- **Verification:** New test passes; full workspace suite green
- **Committed in:** `2583c78` (part of task commit)

---

**Total deviations:** 4 auto-fixed (1 bug, 1 blocking, 2 missing-critical)
**Impact on plan:** All required for correctness/compilation/coverage of the plan scope. No scope creep — no VPN/proxy logic added; task 3 reused the substantial partial work found uncommitted in the tree (validator, argv builders, parsers, CLI actions, snapshots all verified and kept; only the polkit-mapping test was added).

## Issues Encountered

- Transient `index.crates.io` DNS failure during `cargo add` (first attempt, task 1); retry succeeded and `cargo fetch` completed — no code impact
- New insta snapshots needed one accept cycle each (first-run `.snap.new`); content reviewed (full schemas, no secrets, no ipv6 keys, dead hop null) before accepting
- Live `net test` second run showed `0.00 ms` for the DNS hop: dig self-reported `Query time: 0 msec` (local stub cache hit), not parser failure — rendered as measured per the D-10 raw-figures contract
- Continuation started with uncommitted partial Task 3 work in the tree (core + CLI + 2 snaps, all green); audited file-by-file against the plan's `<behavior>` items, kept everything, added only the missing polkit-mapping test

## User Setup Required

None - no external service configuration required.

## Checkpoint Resolved (was blocking-human — execution completed)

**Gate:** the `networkmanager` wrapper crate decision (plan task 2, `gate="blocking-human"`).

- **What was built:** zbus-direct NM implementation path (default per research legitimacy audit: `networkmanager` 0.5.0 is SUS at ~630 downloads/week, repo `zibebe/networkmanager-rs`)
- **Human decision:** **rejected (stay zbus-direct)** — received on resume; recorded here per plan instructions
- **Consequence:** No wrapper spike was run, no `networkmanager` dependency was added. Task 3 was implemented on the zbus-direct + nmcli-fallback path. T-4-SC honored.

## Threat Flags

None beyond the plan's threat model — no new surface introduced:
- DNS/profile names cross into `nmcli` only via exact `Vec<String>` argv (never shelled), UUID-resolved client-side, validated before any spawn (T-4-01 mitigated)
- Write denials map to `PolkitDenied` with the agent hint; the privileged helper is never involved (T-4-02 mitigated)
- No new crate was installed (T-4-SC: wrapper rejected, `serde` in the bin was already a workspace dependency)

## Next Phase Readiness

- Plan 01 is shippable: status/DNS/test proven headlessly with unit + snapshot coverage plus live D-Bus runs
- Live DNS round-trip (actual `set` + `reset` against a scratch profile) and the polkit-denied hint path remain manual verify-work items (mutate live profile, need polkit agent interaction)
- Plans 04-02/03/04 build on the `dns`/`selftest` modules, the `run_nm_write` shape, and the plan-preview-confirm-apply precedent established here

## Self-Check: PASSED

- All created files exist (`network.rs`, `net_cmds.rs`, 3 snapshots, SUMMARY)
- Commits `56188ff` (task 1) and `2583c78` (task 3) exist, no deletions in either, no untracked task files left behind
- Live proof re-runnable: `./target/debug/archel net status --json`, `net dns show --json`, `net test --json` (all rc=0 at completion time)
- Workspace suite green: 59 + 139 + 18, diacritics guard clean

---
*Phase: 04-connectivity-devices*
*Completed: 2026-10-07*
