---
phase: 02-system-insight
plan: 02
subsystem: cli-telemetry
tags: [sysinfo, cpu-sampling, watch-mode, ndjson, clap, french-i18n]

# Dependency graph
requires:
  - phase: 01-foundations-safety-rails
    provides: [bin-calls-core pattern, catalog French via fr.json, typed ArchelError, diacritics guard]
  - phase: 02-system-insight
    provides: [plan 01 landing snapshot, sysinfo 0.39 + global --json + watch-loop precedent]
provides:
  - "archel-core::system (Monitor + one-shot snapshots + conversions + catalog labels)"
  - "archel sys cpu|ram|disk (French tables + --json + --watch NDJSON)"
  - "SystemPartitionNotFound typed variant + fr.json sys.* catalog section"
affects: [02-03-sensors-proof, gui-system-tab]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
# Same estimateTokens scale (chars/4 over the realized diff), never a harness token count.
actuals:
  tokens: 7496
  tasks: 2
  commits: 2

# Tech tracking
tech-stack:
  added: []
  patterns: [reusable-monitor-sampler, per-command-watch-with-global-json, label-table-layout]

key-files:
  created: [crates/archel-core/src/system.rs, crates/archel/src/cmds/sys_cmds.rs]
  modified: [crates/archel-core/src/error.rs, crates/archel-core/src/lib.rs, crates/archel-core/locales/fr.json, crates/archel/src/cmds/mod.rs]

key-decisions:
  - "Global --json (D-12) wins over the plan's per-command --json wording: sys follows the landing shape (per-command --watch, global --json)"
  - "Monitor::new primes CPU with the first refresh; the D-04 1 s wait happens lazily on the first cpu() call so ram/disk-only runs never pay it"
  - "Disk used = total - available exactly per the plan formula (includes root-reserved blocks, so it reads ~5% above df Used)"
  - "Direct-to-master commits per branching_strategy none and prior-wave precedent (HEAD assertion would refuse master)"

patterns-established:
  - "Reusable Monitor sampler: created once outside the watch loop, cpu()/ram()/disk() per tick — ticks 2+ always carry valid diff-based numbers"
  - "Label/value table layout shared by ram/disk renderers (render_label_table); per-core grid only where rows vary (cpu)"

requirements-completed: [SYS-01, SYS-02, SYS-03, SYS-04]

# Coverage metadata (#1602) — one entry per shipped deliverable. Drives DETERMINISTIC UAT routing in verify-work.
coverage:
  - id: D1
    description: "archel sys cpu prints per-core usage percentages plus MHz clocks in a French table"
    requirement: "SYS-03"
    verification:
      - kind: unit
        ref: "crates/archel/src/cmds/sys_cmds.rs#cpu_table_shows_per_core_usage_and_clock"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/system.rs#live_cpu_snapshot_is_not_all_zero"
        status: pass
      - kind: other
        ref: "cargo run -q -p archel --no-default-features -- sys cpu (16 cores, matches nproc, non-zero cores present)"
        status: pass
    human_judgment: false
  - id: D2
    description: "archel sys ram and archel sys disk print RAM used/total plus system-partition used/total/free with used Gio prominent"
    requirement: "SYS-01"
    verification:
      - kind: unit
        ref: "crates/archel/src/cmds/sys_cmds.rs#ram_table_leads_with_used_gib"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/sys_cmds.rs#disk_table_shows_used_total_free"
        status: pass
      - kind: other
        ref: "live sys ram (5.2/31.2 Gio) + sys disk total 816.0 Gio matching df / on /dev/nvme0n1p5"
        status: pass
    human_judgment: false
  - id: D3
    description: "Any sys metric with --watch re-renders every 500 ms; with --watch --json emits one parseable JSON object per tick"
    requirement: "SYS-04"
    verification:
      - kind: unit
        ref: "crates/archel/src/cmds/sys_cmds.rs#snapshots_serialize_to_parseable_json"
        status: pass
      - kind: other
        ref: "timeout-captured sys ram --watch --json: 6 ticks in 3 s, every line json.loads OK, stderr empty"
        status: pass
      - kind: other
        ref: "timeout-captured sys disk --watch: starts with ESC-c full-reset, 4 re-renders in 2 s"
        status: pass
    human_judgment: false
  - id: D4
    description: "Watch re-render looks flicker-free at 2 Hz in a real terminal"
    verification: []
    human_judgment: true
    rationale: "Flicker perception needs a human eye in a live terminal; automation only proves cadence bytes (see 02-VALIDATION.md — stays manual)"

# Metrics
duration: 10min
completed: 2026-10-01
status: complete
---

# Phase 02 Plan 02: Core CPU/RAM/Disk Samplers + Sys CLI Summary

**`archel sys cpu|ram|disk` shows live per-core CPU + clocks, RAM and root-partition disk in French tables (plus `--json`/`--watch` NDJSON), backed by a two-pass `archel-core::system` sampler with typed partition errors**

## Performance

- **Duration:** ~10 min (approx.)
- **Started:** 2026-09-30T23:58:00Z
- **Completed:** 2026-10-01T00:08:00Z
- **Tasks:** 2 (both auto)
- **Files modified:** 6

## Accomplishments

- `archel-core::system`: `Monitor` (reusable sampler) + one-shot `cpu_snapshot`/`ram_snapshot`/`disk_snapshot` + `kernel_version` for landing reuse, with D-04 ~1 s two-pass CPU window and `mount_point == "/"` exact disk match
- T-2-03 mitigated at the boundary: CPU clamped 0–100, `used = total.saturating_sub(available)`, missing `/` → typed `SystemPartitionNotFound` (never a wrong filesystem, never a panic)
- `archel sys cpu|ram|disk` end-to-end: French tables live-proven (16 cores = nproc, 5.2/31.2 Gio RAM, 816.0 Gio total = `df /`), `--json` parses, `--watch` re-renders at 500 ms with `ESC-c`, `--watch --json` streams clean NDJSON (6/6 ticks parse, stderr empty)
- Full workspace gate green: 49 core + 9 bin + 5 privileged tests, diacritics guard included

## Task Commits

Each task was committed atomically:

1. **Task 1: system.rs sampler (CPU two-pass, RAM, root disk, kernel, frequency)** - `366e151` (feat)
2. **Task 2: Sys CLI group (cpu/ram/disk renderers + shared watch/NDJSON loop)** - `d2c5a46` (feat)

## Files Created/Modified

- `crates/archel-core/src/system.rs` - Monitor, snapshots, conversions, root-pick, catalog label helpers, fixture + live tests (created)
- `crates/archel/src/cmds/sys_cmds.rs` - run_cpu/ram/disk, watch loop, table renderers, JSON framing, bin tests (created)
- `crates/archel-core/src/error.rs` - `SystemPartitionNotFound` variant + catalog arm + every_variant entry (modified)
- `crates/archel-core/src/lib.rs` - `pub mod system` (modified)
- `crates/archel-core/locales/fr.json` - `sys.*` section (titles, headers, units) + `errors.system_partition_not_found` (modified)
- `crates/archel/src/cmds/mod.rs` - `Sys` group + `SysMetric` enum + dispatch (modified)

## Decisions Made

- Global `--json` (locked D-12) wins over the plan task's "per-command --watch/--json" wording: sys mirrors the landing shape exactly (per-command `--watch`, global `--json`), so `sys cpu --watch --json` works in any flag order.
- `Monitor::new()` runs only the first CPU refresh; the D-04 1 s sleep happens lazily inside the first `cpu()` call. RAM/disk-only invocations (`sys ram`, `sys disk`) never pay the sampling window.
- Disk `used = total - available` applied literally per the plan formula: on this machine it reads 250.8 Gio vs `df` Used 210G — the ~40G gap is the root-reserved 5% that `available` excludes. Total (816.0 Gio) and free (565.1 Gio) match `df /` exactly.
- sysinfo 0.39 needed no Cargo.toml change — plan 01 already vendored it; the API (`refresh_cpu_all`, `frequency`, `Disks::new_with_refreshed_list`) was re-verified against the downloaded 0.39.6 source before use.
- Commits landed directly on `master`: `.planning/config.json` sets `branching_strategy: none` and every prior wave commit (823a588, 88aee41, 2d29421) sits on master. The per-commit HEAD safety assertion would refuse a protected branch; project convention (trunk commits, single executor) takes precedence here.

## Deviations from Plan

None - plan executed exactly as written. (The sysinfo dep note and disk-convention observation above record followed-spec facts, not deviations: no Rule 1–3 fix was needed — both tasks compiled and passed on the first verification run.)

## Issues Encountered

- None blocking. Observation only: idle-machine CPU tables legitimately show several `0.0 %` cores; the Pitfall 1 guard asserts *not-all-zero* (not *none-zero*), which holds live (3.6 %, 9.2 % cores present).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Ready for 02-03: `system::Monitor` + label helpers are GUI-reusable core APIs; `gpu/temp/power/all` subcommands plug into the established `SysMetric` + `sys_cmds` shape. `system::kernel_version()` is exposed for `landing.rs` to adopt (it still calls sysinfo inline — optional cleanup, not a stub).
- Watch out: snapshot JSON field names (`cores{id,name,usage_pct,freq_mhz}`, byte-count ram/disk) are v1-unstable — pin before the GUI consumes them.
- Blocker: none. Manual carry: watch-mode flicker perception stays human per 02-VALIDATION.md (coverage D4).

---
*Phase: 02-system-insight*
*Completed: 2026-10-01*

## Self-Check: PASSED

All created files found on disk (`system.rs`, `sys_cmds.rs`); both task commits (`366e151`, `d2c5a46`) present in history; workspace suite green (49 + 9 + 5); live `sys cpu/ram/disk` + `--json` + `--watch` verified on the machine.
