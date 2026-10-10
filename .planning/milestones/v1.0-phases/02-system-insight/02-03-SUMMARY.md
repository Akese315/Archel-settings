---
phase: 02-system-insight
plan: 03
subsystem: cli-telemetry
tags: [nvml, rapl, hwmon, sensors, insta, json-schema, french-i18n]

# Dependency graph
requires:
  - phase: 01-foundations-safety-rails
    provides: [bin-calls-core pattern, catalog French via fr.json, typed ArchelError, diacritics guard]
  - phase: 02-system-insight
    provides: [plan 01 landing snapshot + global --json, plan 02 Monitor sampler + sys cpu/ram/disk + watch/NDJSON loop]
provides:
  - "archel-core::sensors (GpuBackend trait, NvmlBackend + NoGpu, RAPL discovery, hottest-pick temps, AllSnapshot)"
  - "archel sys gpu|temp|power|all (French tables + --json + --watch NDJSON with honest N/A)"
  - "insta-pinned --json schemas for landing plus every sys metric"
affects: [gui-system-tab]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
# Same estimateTokens scale (chars/4 over the realized diff), never a harness token count.
actuals:
  tokens: 16040
  tasks: 2
  commits: 2

# Tech tracking
tech-stack:
  added: [nvml-wrapper 0.13, insta 1.48]
  patterns: [gpu-backend-trait-with-unavailable-fallback, rapl-discovery-by-name, hottest-pick-temps, insta-schema-pins]

key-files:
  created: [crates/archel-core/src/sensors.rs, crates/archel/src/cmds/snapshots/]
  modified: [crates/archel-core/src/error.rs, crates/archel-core/src/lib.rs, crates/archel-core/locales/fr.json, crates/archel-core/Cargo.toml, crates/archel/src/cmds/mod.rs, crates/archel/src/cmds/sys_cmds.rs, crates/archel/src/cmds/landing_cmd.rs, crates/archel/Cargo.toml]

key-decisions:
  - "Root-only RAPL energy_uj degrades to honest CPU N/A (D-10/T-2-04), not an error — SYS-07 reads watts only where the kernel permits"
  - "insta snapshots pin fixture schemas (not live values) so the T-2-05 schema-stability gate is deterministic"
  - "sys all reuses the per-metric renderers section by section so composed output cannot drift from single-metric tables"
  - "Direct-to-master commits per branching_strategy none and prior-wave precedent"

patterns-established:
  - "GpuBackend trait with probe-then-fallback: Nvml::init/device_by_index success selects NvmlBackend, any failure selects NoGpu — no panics, no subprocess"
  - "RAPL discovery by powercap name (prefer package-*, first top-level fallback, skip :0 sub-domains) with max_energy_range_uj wrap math in a pure fixture-tested fn"

requirements-completed: [SYS-05, SYS-06, SYS-07, SYS-08]

# Coverage metadata (#1602) — one entry per shipped deliverable. Drives DETERMINISTIC UAT routing in verify-work.
coverage:
  - id: D1
    description: "archel sys gpu prints model plus VRAM used/total (GTX 1070 Ti 8 Gio live)"
    requirement: "SYS-05"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/sensors.rs#live_nvml_probe_answers_on_this_machine"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/sys_cmds.rs#gpu_table_shows_model_and_vram"
        status: pass
      - kind: other
        ref: "cargo run -q -p archel --no-default-features -- sys gpu (GTX 1070 Ti, 1.3/8.0 Gio)"
        status: pass
    human_judgment: false
  - id: D2
    description: "archel sys temp prints one hottest value per category (CPU Tccd1 wins, GPU NVML)"
    requirement: "SYS-06"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/sensors.rs#live_cpu_temp_matches_hottest_k10temp"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/sys_cmds.rs#temp_table_shows_one_value_per_category"
        status: pass
      - kind: other
        ref: "cargo run -q -p archel --no-default-features -- sys temp (CPU + GPU rows)"
        status: pass
    human_judgment: false
  - id: D3
    description: "archel sys power prints real-time GPU watts plus honest CPU N/A (root-only RAPL)"
    requirement: "SYS-07"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/sensors.rs#live_gpu_power_sits_within_tdp"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/sensors.rs#live_cpu_power_is_plausible_or_honestly_absent"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/sys_cmds.rs#power_table_shows_watts_with_honest_na"
        status: pass
      - kind: other
        ref: "cargo run -q -p archel --no-default-features -- sys power (GPU 34.0 W vs nvidia-smi 33.82 W)"
        status: pass
    human_judgment: false
  - id: D4
    description: "archel sys all composes every metric into one table and one --json object"
    requirement: "SYS-08"
    verification:
      - kind: unit
        ref: "crates/archel/src/cmds/sys_cmds.rs#all_table_composes_every_section"
        status: pass
      - kind: other
        ref: "cargo run -q -p archel --no-default-features -- sys all --json | python3 json.load assert gpu+cpu"
        status: pass
    human_judgment: false
  - id: D5
    description: "Every --json schema (landing + all sys metrics) pinned by insta snapshots"
    verification:
      - kind: unit
        ref: "crates/archel/src/cmds/sys_cmds.rs#schema_sys_all_is_pinned (+6 siblings, landing)"
        status: pass
    human_judgment: false
  - id: D6
    description: "Watch re-render feels flicker-free at 2 Hz in a real terminal"
    verification: []
    human_judgment: true
    rationale: "Flicker perception needs a human eye in a live terminal; automation only proves cadence bytes (see 02-VALIDATION.md — stays manual)"
  - id: D7
    description: "Missing-checkupdates landing state renders the guided French hint"
    verification: []
    human_judgment: true
    rationale: "Depends on machine package state (pacman-contrib absent); manual per 02-VALIDATION.md"

# Metrics
duration: 7min
completed: 2026-09-30
status: complete
---

# Phase 02 Plan 03: GPU/Sensor/Power Layer + Remaining Sys Renderers Summary

**`archel sys gpu|temp|power|all` shows live VRAM, hottest temps and real-time watts with honest French N/A (root-gated RAPL), backed by `archel-core::sensors` (NVML + RAPL + hwmon discovery-by-name), with insta-pinned `--json` schemas**

## Performance

- **Duration:** ~7 min (approx.)
- **Started:** 2026-09-30T22:09:32Z
- **Completed:** 2026-09-30T22:16:50Z
- **Tasks:** 2 (both auto)
- **Files modified:** 18 (incl. 8 new snapshot files)

## Accomplishments

- `archel-core::sensors`: `GpuBackend` trait with `NvmlBackend` (VRAM used/total, temp, milliwatts-to-watts) + `NoGpu` honest-absence backend; RAPL package-domain discovery by powercap name with `max_energy_range_uj` wrap math; hottest-per-category temps via sysinfo Components with hwmon discovery-by-name fallback; every reading `Option`-typed end-to-end
- T-2-04 mitigated at the boundary: named unit consts (never inline 1000.0), silicon range asserts (temps, watts), unreadable sysfs nodes become `None`, never panics
- T-2-05 mitigated: 8 insta snapshots pin the landing + every sys metric `--json` schema (snake_case English keys, v1-unstable)
- `archel sys gpu|temp|power|all` end-to-end: French tables live-proven (GTX 1070 Ti 1.3/8.0 Gio, CPU 48–57 °C hottest, GPU 54.0 °C = nvidia-smi, GPU 34 W ≈ nvidia-smi 33.82 W, CPU power honest N/A), `--json` parses, `--watch --json` streams clean NDJSON (6/6 ticks parse, stderr empty)
- Full workspace gate green: 66 core + 22 bin + 5 privileged tests, diacritics guard included

## Task Commits

Each task was committed atomically:

1. **Task 1: sensors.rs (GpuBackend, NVML, RAPL, hottest temps)** - `5d2c940` (feat)
2. **Task 2: gpu/temp/power/all renderers + insta JSON snapshots** - `cae4f6d` (feat)

## Files Created/Modified

- `crates/archel-core/src/sensors.rs` - GpuBackend/NvmlBackend/NoGpu, RAPL discovery + delta math, hottest-pick, AllSnapshot, catalog label helpers, fixture + live tests (created)
- `crates/archel/src/cmds/snapshots/` - 8 insta `.snap` pins (landing + cpu/ram/disk/gpu/temp/power/all) (created)
- `crates/archel/src/cmds/sys_cmds.rs` - run_gpu/temp/power/all, table renderers, N/A formatters, bin tests + snapshot pins (modified)
- `crates/archel/src/cmds/mod.rs` - Gpu/Temp/Power/All subcommands + dispatch (modified)
- `crates/archel/src/cmds/landing_cmd.rs` - landing schema pin test (modified)
- `crates/archel-core/src/error.rs` - `SensorUnavailable` variant + catalog arm + every_variant entry (modified)
- `crates/archel-core/src/lib.rs` - `pub mod sensors` (modified)
- `crates/archel-core/locales/fr.json` - `sys.gpu_*`, `sys.temp_*`, `sys.power_*`, `sys.all_title`, `sys.unavailable`, `sys.unit_celsius/watt`, `errors.sensor_unavailable` (modified)
- `crates/archel-core/Cargo.toml` - `nvml-wrapper 0.13` (modified)
- `crates/archel/Cargo.toml` - `insta 1.48` with `json` feature as dev-dep (modified)
- `Cargo.lock` - new dependency pins (modified)

## Decisions Made

- Root-only RAPL degrades to honest CPU N/A rather than failing: both `energy_uj` nodes are mode 400 root-only on this machine with no hwmon power fallback, so SYS-07 yields watts only where the kernel permits reads. The delta math is fixture-proven; the live test branches on readability (T-2-04 "unreadable nodes become typed N/A").
- insta pins fixture schemas, not live values — deterministic pins that fail on silent reshapes without flakiness from sampling noise.
- `sys all` reuses per-metric renderers section-by-section (plus `collect_all` for the JSON object) so composed output cannot drift from single-metric tables.
- `RaplDomain` made `pub` (private fields) to satisfy the `private_interfaces` lint on the public `discover_rapl_domain` — construction stays internal via `Sensors::new`.
- Commits landed directly on `master`: `.planning/config.json` sets `branching_strategy: none` and every prior wave commit sits on master (same precedent as 02-02).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] insta needs the `json` feature for `assert_json_snapshot!`**
- **Found during:** Task 2 (first snapshot build)
- **Issue:** Plain `insta = "1.48"` does not export `assert_json_snapshot!`; compilation failed on all 8 pins
- **Fix:** `insta = { version = "1.48", features = ["json"] }` in `crates/archel/Cargo.toml`
- **Files modified:** crates/archel/Cargo.toml, Cargo.lock
- **Verification:** `cargo test -p archel --no-default-features schema` 8/8 green
- **Committed in:** cae4f6d (part of task commit)

**2. [Rule 1 - Bug] `private_interfaces` lint on `discover_rapl_domain`**
- **Found during:** Task 1 (full core suite)
- **Issue:** Public fn returned the private `RaplDomain` type — compiler warning, would fail deny-warnings CI
- **Fix:** Made `RaplDomain` a `pub struct` with private fields
- **Files modified:** crates/archel-core/src/sensors.rs
- **Verification:** warning-free `cargo test -p archel-core` green
- **Committed in:** 5d2c940 (part of task commit)

**3. [Rule 1 - Bug] Live temp-agreement test flaked under parallel suite load**
- **Found during:** Task 1 (full core suite — passed solo, failed once in-suite)
- **Issue:** Components-vs-hwmon 5 °C tolerance too tight: the suite itself heats the 5800X between the two back-to-back reads
- **Fix:** Tolerance widened to 10 °C with a comment recording the thermal-slew cause; silicon-bounds assertion unchanged
- **Files modified:** crates/archel-core/src/sensors.rs
- **Verification:** full core suite green twice consecutively (66/66)
- **Committed in:** 5d2c940 (part of task commit)

---

**Total deviations:** 3 auto-fixed (all Rule 1 bugs)
**Impact on plan:** All fixes required for compilation, lint-cleanliness, or test stability. No scope creep; sensor truths and snapshot pins ship exactly as planned.

## Issues Encountered

- RAPL `energy_uj` unreadable unprivileged (mode 400 root): by design this becomes honest CPU N/A per D-10 — surfaced here because the plan's done-criteria wording assumed live CPU watts. GPU watts, both temps, and VRAM all read live; CPU power math is fixture-proven and will light up under kernels/users with readable RAPL.
- Live CPU temp varies run to run (48–57 °C across invocations): expected — compile/test load moves the hottest k10temp node; the D-09 hottest-pick assertion (not a fixed value) is the stable contract.

## User Setup Required

None - no external service configuration required. (Unchanged: `pacman-contrib` install for LAND-03 stays tracked in 02-USER-SETUP.md from plan 01.)

## Next Phase Readiness

- Ready for verify-work: all 11 phase requirement IDs placed (LAND-01..03 in 01, SYS-01..04 in 02, SYS-05..08 here); every sys metric plus landing renders French tables and parseable `--json`, schemas pinned.
- Watch out: `--json` schemas are documented v1-unstable — the GUI phase consumes them as-is until it declares stability.
- Manual carry (per 02-VALIDATION.md, not blocking): 500 ms watch re-render feel (coverage D6), missing-checkupdates graceful state (coverage D7).
- Blocker: none.

---
*Phase: 02-system-insight*
*Completed: 2026-09-30*

## Self-Check: PASSED

All created files found on disk (`sensors.rs`, `sys_cmds.rs`, 8 snapshots); both task commits (`5d2c940`, `cae4f6d`) present in history; workspace suite green (66 + 22 + 5); live `sys gpu/temp/power/all` + `--json` + `--watch` NDJSON verified against nvidia-smi/hwmon; no stubs; no unlisted threat surface (NVML dlopen + sysfs reads are the plan's T-2-04/T-2-05 scope).
