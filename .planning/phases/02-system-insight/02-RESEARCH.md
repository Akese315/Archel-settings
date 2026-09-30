# Phase 2: System Insight - Research

**Researched:** 2026-09-30
**Domain:** Read-only system telemetry (landing facts + live metrics) in Rust — sysinfo sampling, NVML GPU, hwmon/RAPL sysfs, update-count cache, clap CLI with watch mode
**Confidence:** HIGH

## Summary

Phase 2 builds the read-only telemetry layer of `archel-core` plus its CLI proof: landing facts (app version, kernel, cached pending-update count) and live metrics (per-core CPU + clock, RAM, system-partition disk, GPU VRAM, hottest temperatures, CPU/GPU power with honest N/A). The standard stack is `sysinfo 0.39.6` for CPU/RAM/disk/frequency/temperatures, `nvml-wrapper 0.13` for everything NVIDIA, hand-rolled sysfs readers for Intel-RAPL CPU power and hwmon fallback, `checkupdates` + `yay -Qu` subprocesses (via the Phase 1 `privileged.rs` spawn/parse pattern) for the 15-minute-cached update count, and `directories 6.0` for the XDG cache path. Every crate API recommended below was verified by reading the downloaded crate source this session, and every sensor path was probed on the live Archel machine.

The live machine (AMD Ryzen 7 5800X + NVIDIA GTX 1070 Ti, driver 580.178.04, kernel 7.2.6-arch2-1) dictates the sensor strategy: CPU temperature comes from `k10temp` (`Tctl`/`Tccd1` — hottest wins), CPU power from Intel-RAPL `energy_uj` deltas (`package-0` + `core` domains exist), and GPU temperature/VRAM/power come **exclusively from NVML** (the NVIDIA card exposes no hwmon node; `/sys/class/drm/card1/device/hwmon` does not exist). NVML was proven working via `nvidia-smi` (54 °C, 42.78 W, 8192 MiB). One environment gap: `checkupdates` is **not installed** (`pacman-contrib` absent) — the plan must install it or gate LAND-03's pacman half behind its detection.

**Primary recommendation:** Build `archel-core::{system,landing,sensors}` on sysinfo + NVML + two tiny sysfs readers (RAPL energy, hwmon discovery-by-name), with a `SensorValue::Available(f64) | Unavailable(reason)` type flowing honest French N/A end-to-end; expose snapshot-by-default functions plus a `--watch` loop (500 ms re-render, NDJSON per tick under `--json`) under `archel sys <metric>` + `archel landing`, following the Phase 1 bin-calls-core/catalog-French patterns exactly.

## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Every telemetry command prints a one-shot snapshot by default and exits; `--watch` keeps it open and re-renders until Ctrl+C. — **Reversibility:** costly — scripts and later GUI polling semantics will depend on snapshot-by-default.
- **D-02:** `--watch` re-renders every 500 ms.
- **D-03:** `--watch` combined with `--json` prints one complete JSON object per tick (NDJSON-style stream), not a single final object.
- **D-04:** Snapshot CPU percentages are measured over a ~1 s sampling window (slower first response, reliable numbers).
- **D-05:** Pending-update count (pacman + AUR) is cached for 15 minutes; recomputation runs `checkupdates` + AUR query only on cache miss/expiry.
- **D-06:** When offline or the refresh fails, show the last cached value together with its age — never a bare number that looks fresh, never a hard error hiding everything else.
- **D-07:** `--refresh` flag forces an immediate recomputation, bypassing the cache.
- **D-08:** Disk reporting covers the system partition only (the one the OS runs on), shown as used / total / free with used GB prominent — not a per-mount list. (Narrows SYS-02 deliberately per user request.)
- **D-09:** Temperatures show one representative value per category (CPU, GPU, …): the hottest sensor, not an average and not the full list.
- **D-10:** Missing sensors render honest French N/A (carries forward the roadmap criterion; researcher picks per-vendor sources).
- **D-11:** Telemetry lives under an `archel sys` group with English subcommands per metric (`archel sys cpu`, `sys ram`, `sys disk`, `sys gpu`, …); the landing snapshot is a top-level `archel landing`. — **Reversibility:** one-way — renaming shipped subcommands breaks user muscle memory and scripts (same rationale as Phase 1 D-01).
- **D-12:** Human output renders as aligned French tables; `--json` is a single global flag producing machine output for every command.

### the agent's Discretion
- Exact per-metric subcommand list beyond cpu/ram/disk/gpu, cache file location under XDG paths, sampling implementation details, per-vendor sensor source selection (sysinfo/hwmon/NVML/RAPL), table column layout, JSON schema field names — researcher and planner decide following Phase 1 patterns (typed errors, catalog French, no panics).

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope (SYS-09 history/export already roadmap-deferred).

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| LAND-01 | App version on home page | `archel-core::library_version()` exists (Phase 1); landing snapshot composes it — no research needed |
| LAND-02 | Running kernel version on home page | `sysinfo::System::kernel_version()` [VERIFIED: sysinfo 0.39.6 source] |
| LAND-03 | Pending-update count (pacman repo + yay/AUR) | `checkupdates` + `yay -Qu` subprocesses via `privileged.rs` spawn/parse pattern; 15-min JSON cache under XDG cache dir; `checkupdates` missing on live machine — plan must handle |
| SYS-01 | RAM usage (used/total) | `sys.refresh_memory()` + `total_memory()`/`used_memory()` [VERIFIED: sysinfo 0.39.6 source] |
| SYS-02 | Disk usage (narrowed: system partition only, used/total/free) | `Disks::new_with_refreshed_list()` + match `mount_point() == "/"`; `total_space()`/`available_space()` [VERIFIED: sysinfo 0.39.6 source] |
| SYS-03 | Live per-core CPU usage | `refresh_cpu_all()` twice with ~1 s gap (D-04); `cpus()` → `cpu_usage()` per core [VERIFIED: sysinfo 0.39.6 source] |
| SYS-04 | Current CPU clock (GHz) | `Cpu::frequency()` in MHz [VERIFIED: sysinfo 0.39.6 source]; corroborated by `/sys/.../scaling_cur_freq` on live machine |
| SYS-05 | GPU VRAM used/total | NVML `device.memory_info()` → `MemoryInfo{used,total}` [VERIFIED: nvml-wrapper 0.13.0 source]; 8192 MiB confirmed on live GPU |
| SYS-06 | CPU + GPU temperature | CPU: hottest of `k10temp` via sysinfo `Components` or hwmon reader; GPU: `device.temperature(TemperatureSensor::Gpu)` [VERIFIED: nvml-wrapper 0.13.0 source] |
| SYS-07 | Real-time CPU power (W) | Intel-RAPL `/sys/class/powercap/intel-rapl:0/energy_uj` delta ÷ time [VERIFIED: live machine probe — `package-0` + `core` domains present with live `energy_uj`] |
| SYS-08 | Real-time GPU power (W) | NVML `device.power_usage()` in milliwatts ÷ 1000 [VERIFIED: nvml-wrapper 0.13.0 source]; unit (mW) cross-checked against `nvidia-smi` 42.78 W on live machine |

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Telemetry sampling (CPU/RAM/disk/temps/power) | API / Backend (`archel-core`) | — | PROJECT.md core value: the API owns every integration; CLI and future GUI are thin consumers. Sampling logic must live in core so the GUI's 1 Hz poller reuses it without duplication |
| Update-count cache file | API / Backend (`archel-core`) | — | Cache read/write is core logic; CLI only renders. Keeps the future GUI landing tab on identical freshness semantics |
| Snapshot vs watch rendering, tables, `--json` | CLI (`archel` bin) | — | Phase 1 pattern: bin renders, core computes. Watch loop + NDJSON framing are presentation, not telemetry |
| French strings | `archel-core` catalog (`locales/fr.json`) | — | D-07 single-catalog invariant from Phase 1; bin owns no copy |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `sysinfo` | 0.39.6 [VERIFIED: `cargo search` + legitimacy gate OK, 3.8M/wk] | CPU per-core % + frequency, RAM, disks, kernel version, temperature components | Best-maintained cross-platform sampler; refresh-based API maps directly onto snapshot (refresh→sleep→refresh) and future GUI polling. Default features (`component,disk,system`) cover everything Phase 2 needs from it |
| `nvml-wrapper` | 0.13.0 [VERIFIED: `cargo search` + legitimacy gate OK, 137k/wk] | NVIDIA VRAM, temp, power, utilization | Official-style safe NVML binding, dlopen-based — `Nvml::init()` fails cleanly on non-NVIDIA machines so the binary runs everywhere and reports honest N/A. **Mandatory on this machine**: the GTX 1070 Ti exposes no hwmon node, NVML is the only GPU source |
| `directories` | 6.0.0 [VERIFIED: `cargo search` + legitimacy gate OK, 1M/wk] | XDG cache dir for the update-count cache | Tiny standard crate for `ProjectDirs::cache_dir()`; avoids hand-rolled `$XDG_CACHE_HOME` fallback logic |
| `serde` + `serde_json` | 1.0.229 / 1.0.151 (already in `archel-core`) | Cache file envelope + CLI `--json` output | Already vendored in Phase 1; reuse, no new dep |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| Hand-rolled `sensors` module (sysfs) | n/a (~120 lines) | Intel-RAPL CPU power via `energy_uj` deltas; hwmon hottest-sensor discovery by `name` file | Always: RAPL has no maintained crate; hwmon index (`hwmon0` vs `hwmon2`) is unstable across boots so discovery-by-name (`k10temp`, `coretemp`, `amdgpu`, …) is required (PITFALLS.md Pitfall 4) |
| `which` + `duct` | 8.0.6 / 1.1.2 (already in `archel-core`) | `checkupdates` / `yay` capability detection + timeout-guarded subprocess | LAND-03 recomputation path; exact Phase 1 `privileged.rs` pattern (absolute path, stderr-for-logs, exit-code mapping) |
| `tokio` (mini) | 1.53.1 [ASSUMED — STACK.md pin, not re-verified this session] | `--watch` tick timer + async sleep for the 1 s CPU sampling window | Only `time` + `rt` features; Phase 2 needs no D-Bus, no multithreaded runtime. If planner prefers zero new deps, `std::thread::sleep` in a blocking watch loop is equally acceptable — researcher leaves the choice to the planner |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hand-rolled hwmon reader | `sysinfo::Components` only | Components already reads hwmon temps (label/temperature API verified) — use it as the **primary** temp source; hand-rolled reader stays as fallback for power nodes (`power*_average`) which sysinfo does not expose, and for label control (hottest-per-category needs raw labels) |
| `checkupdates` subprocess | `alpm` crate sync-DB version comparison | Rejected: computing pending updates from libalpm requires a sync-DB refresh (`-Sy`), which risks the partial-upgrade state PITFALLS.md forbids. `checkupdates` is the Arch-blessed read-only path (downloads fresh DBs to a temp dir, never touches the system DB) |
| NVML for GPU temp | `nvidia-smi` subprocess per tick | Rejected per PITFALLS.md Pitfall 8 (no subprocess per tick); NVML handle stays open across watch ticks |

**Installation:**
```bash
cargo add sysinfo@0.39 nvml-wrapper@0.13 directories@6.0 -p archel-core
# tokio (if chosen for watch): cargo add tokio --features time,rt -p archel-core
# system (Arch): pacman-contrib provides checkupdates — NOT installed on live machine (see Environment Availability)
```

**Version verification:** `cargo search` run 2026-09-30: `sysinfo = "0.39.6"`, `nvml-wrapper = "0.13.0"`, `directories = "6.0.0"` — all current, matching STACK.md pins.

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| sysinfo | crates | 11 yrs (2015) | 3.8M/wk | github.com/GuillaumeGomez/sysinfo | OK | Approved |
| nvml-wrapper | crates | 9 yrs (2017) | 137k/wk | github.com/Cldfire/nvml-wrapper | OK | Approved |
| directories | crates | 9 yrs (2017) | 1M/wk | github.com/soc/directories-rs | OK | Approved |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

*Legitimacy gate run via `gsd_run query package-legitimacy check --ecosystem crates` 2026-09-30 — all three `OK`, no postinstall scripts (Rust crates have none). Crate APIs additionally confirmed by reading the downloaded 0.39.6 / 0.13.0 sources this session.*

## Architecture Patterns

### System Architecture Diagram

```
[archel sys cpu|ram|disk|gpu|temp|power|all | archel landing]   ← entry: clap English subcommands + global --json + --watch/--refresh flags
        │
        ▼ (function calls; bin NEVER touches sysfs/NVML/subprocess directly)
┌────────────────────────────── archel-core ──────────────────────────────────┐
│  system.rs (snapshot fns)      landing.rs (compose)      cache.rs (15-min)   │
│   ├─ sysinfo System (CPU/RAM/disk/freq/kernel/Components)                   │
│   ├─ sensors.rs (GpuBackend trait: NvmlBackend │ HwmonBackend │ None)        │
│   │    ├─ NVIDIA: Nvml::init → device.memory_info/temperature/power_usage   │
│   │    ├─ CPU power: RAPL energy_uj delta / dt → W                          │
│   │    └─ temps: hottest Component per category → Option<f32>               │
│   └─ updates.rs (checkupdates + yay -Qu → counts; stale-value-with-age)     │
│  error.rs (new typed variants) + locales/fr.json (all new French strings)   │
└──────────────────────────────────────────────────────────────────────────────┘
        │ plain Rust structs; SensorValue::Available(f64) | Unavailable(reason)
        ▼
[CLI render: French tables (default) │ one JSON object per snapshot (snapshot --json) │ NDJSON per 500 ms tick (watch --json)]
        │
        ▼ (data sources — read-only, unprivileged, no pkexec in Phase 2)
[/proc+sysfs via sysinfo] [libnvidia-ml.so dlopen] [/sys/class/hwmon + /sys/class/powercap] [checkupdates | yay -Qu]
```

A reader traces the primary use case: `archel sys cpu` → `system::cpu_snapshot()` (refresh, sleep ~1 s per D-04, refresh) → struct → French table on stdout. `archel landing` → `landing::snapshot()` composing version + kernel + cached update count (stale-with-age per D-06) → one table.

### Recommended Project Structure
```
crates/archel-core/src/
├── system.rs        # CPU/RAM/disk/frequency/kernel snapshots (sysinfo owner lives here)
├── sensors.rs       # GpuBackend trait + NvmlBackend + hwmon/RAPL sysfs readers + hottest-pick
├── updates.rs       # Pending-update count: checkupdates + yay, 15-min JSON cache, stale-with-age
├── landing.rs       # Landing snapshot composing version + kernel + update-count
├── cache.rs         # Generic TTL JSON cache helper (mtime-based, atomic write) — or fold into updates.rs (planner decides)
├── error.rs         # New variants: SensorUnavailable{…}? / Cache… — planner decides granularity
└── lib.rs           # pub mod system/sensors/updates/landing (+cache)
crates/archel/src/cmds/
├── mod.rs           # Sys group + Landing wiring, global --json
├── sys_cmds.rs      # per-metric subcommands + --watch loop + table/JSON renderers (planner decides file split)
└── landing_cmd.rs   # archel landing + --refresh (or fold into sys_cmds.rs)
```

### Pattern 1: Two-pass CPU sampling (D-04)
**What:** `System::new_all()` (or `new()` + explicit refresh list) → `refresh_cpu_all()` → sleep ~1 s → `refresh_cpu_all()` → read `cpus()[i].cpu_usage()`. First-pass values are always 0/diff-based garbage — sysinfo docs state "the result will be inaccurate at the first call, you need to call this method at least twice" [VERIFIED: sysinfo 0.39.6 `src/common/system.rs` docstring].
**When to use:** every snapshot path that reports CPU %. In `--watch`, the loop's own 500 ms cadence provides the second pass after the first tick — planner decides whether tick 1 shows `N/A (échantillonnage…)` or blocks 1 s before tick 1 (researcher recommends: block once at startup so every visible tick has real numbers, matching htop's mental model from CONTEXT).
**Example:**
```rust
// Source: sysinfo 0.39.6 src/common/system.rs (crate source read 2026-09-30)
use sysinfo::{System, MINIMUM_CPU_UPDATE_INTERVAL};
let mut sys = System::new_all();
std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL); // 200 ms minimum; D-04 mandates ~1 s
sys.refresh_cpu_usage();
```

### Pattern 2: GpuBackend trait with Option-per-sensor (PITFALLS.md Pitfall 4)
**What:** `trait GpuBackend { fn vram(&self) -> (Option<u64>, Option<u64>); fn temp_c(&self) -> Option<f32>; fn power_w(&self) -> Option<f32>; }` with `NvmlBackend` (real values), and a `NoGpu`/`Unavailable(reason)` backend. Detection: `Nvml::init().and_then(|n| n.device_by_index(0))` — failure (non-NVIDIA, nouveau, headless) selects the N/A backend, never a panic. `Nvml::init()` + `device_by_index(u32)` + `device_count()` signatures [VERIFIED: nvml-wrapper 0.13.0 source].
**When to use:** SYS-05/06/08. Unit conversion at the boundary: `power_usage()` returns **milliwatts** → ÷ 1000.0 (cross-checked: live `nvidia-smi` reads 42.78 W; plan must assert `power_usage()/1000 ≈ nvidia-smi` on the live machine at implementation time). `temperature(TemperatureSensor::Gpu)` returns °C `u32`; enum path `nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu` [VERIFIED: nvml-wrapper 0.13.0 source + `examples/basic_usage.rs`].
**Example:**
```rust
// Source: nvml-wrapper 0.13.0 src/device.rs + examples/basic_usage.rs (crate source read 2026-09-30)
use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
let nvml = nvml_wrapper::Nvml::init()?;              // Err on non-NVIDIA → N/A backend
let device = nvml.device_by_index(0)?;
let mem = device.memory_info()?;                     // MemoryInfo { used, total, free, …u64 bytes }
let temp_c = device.temperature(TemperatureSensor::Gpu)?; // u32 °C
let power_w = device.power_usage()? as f32 / 1000.0; // u32 milliwatts → W
```

### Pattern 3: RAPL energy-delta CPU power
**What:** Read `/sys/class/powercap/intel-rapl:0/energy_uj` twice, `power_W = (e2 - e1) µJ / dt_s / 1e6`, wrapping-aware via `max_energy_range_uj` (`if e2 < e1 { e2 += max_range }`). Live machine: `package-0` + `package-0:0/core` domains present, `energy_uj` live [VERIFIED: live machine probe 2026-09-30]. Prefer the `package-0` domain (whole-CPU). Missing path → `Unavailable("capteur indisponible")`, never zero.
**When to use:** SYS-07. Note the irony (AMD CPU under an `intel-rapl` sysfs name — the kernel's `amd_rapl`/powercap driver exposes the Intel-RAPL ABI); the plan must discover domains by enumerating `/sys/class/powercap/*` names, not hardcoding `:0`.

### Pattern 4: Hottest-sensor pick (D-09) with discovery-by-name
**What:** Enumerate `/sys/class/hwmon/hwmon*/{name,temp*_input,temp*_label}` (or sysinfo `Components::new_with_refreshed_list()` → `label()`/`temperature()` [VERIFIED: sysinfo 0.39.6 source]) and take the max per category. Live map [VERIFIED: live machine probe]: `hwmon2/k10temp` = CPU (`Tctl` 41.25 °C, `Tccd1` 47.75 °C → report 47.75 °C hottest); `hwmon0/hwmon1/nvme` = disks (ignore for CPU/GPU categories); `hwmon3/asus` = no temp nodes (skip); `hwmon4/mt7921` = wifi (ignore). GPU hottest = NVML (no GPU hwmon exists). `Component::temperature()` returns `Option<f32>` in °C [VERIFIED: sysinfo 0.39.6 source] — `None` entries are skipped, not averaged.
**When to use:** SYS-06. Never hardcode `hwmon2` — discover by `name` file (`k10temp`, `coretemp`, `zenpower`, `amdgpu`, `nvidia`, `iwlwifi`…).

### Pattern 5: TTL cache with stale-value-with-age (D-05/D-06/D-07)
**What:** JSON envelope `{ "v": 1, "computed_at": <unix>, "repo": <n>, "aur": <n> }` at `ProjectDirs::cache_dir()/updates.json` (XDG: `~/.cache/archel/…`); valid if `now - computed_at < 900 s`. On recompute failure: return last cached + `age` (D-06); on total absence (no cache ever): typed error, and landing still renders everything else. `--refresh` skips the read path. Atomic write (temp + rename) per Phase 1 conventions. Count sources: `checkupdates | wc -l` (repo) + `yay -Qu --aur | wc -l` (AUR) [ASSUMED — standard Arch tool behavior; verify flags on live machine at implementation]. Each subprocess gets a timeout (10–15 s) and missing-binary detection via `which` (graceful "non installé" state, not a crash).
**When to use:** LAND-03. No privilege needed: `checkupdates` and `yay -Qu` run as the user — Phase 2 touches neither pkexec nor the helper.

### Pattern 6: Watch loop with NDJSON (D-02/D-03)
**What:** `loop { render(snapshot()); sleep(500ms) }` until Ctrl+C. Human mode: clear screen + reprint (`\x1Bc` full-reset is simplest and flicker-acceptable at 2 Hz; `crossterm` only if the planner wants cursor-addressed re-render — researcher recommends plain `\x1Bc`, zero new deps). `--json` mode: `println!(one JSONObject per tick)` with no clearing (NDJSON stream). Ctrl+C: default SIGINT kill is acceptable for v1 (no cleanup needed — no raw mode, no alternate screen); document that watch exits 130 on SIGINT like standard tools [ASSUMED — Unix convention].
**When to use:** every `sys` subcommand + `landing` under `--watch`.

### Anti-Patterns to Avoid
- **Subprocess per watch tick:** never `nvidia-smi`/`sensors`/`free` in the loop — NVML handle and `System` instance are created once, reused across ticks (PITFALLS.md Pitfall 8).
- **Hardcoded sensor paths:** no `hwmon2`, no `card0`, no `:0` literals — enumerate and match by `name`/driver (PITFALLS.md Pitfall 4).
- **Fake zeros for missing sensors:** every metric crossing the core→CLI boundary is `Option`/typed-N/A; the GUI phase will reuse these types (UX Pitfalls table).
- **French literals or table layout in core:** core returns structs + catalog keys; column widths and alignment live in the bin (Phase 1 Established Patterns).
- **`checkupdates` as root or with `-Sy`:** `checkupdates` is user-only and never touches the system DB — running any refresh with sudo invites the partial-upgrade/lock states PITFALLS.md forbids.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CPU/RAM/disk/frequency sampling | `/proc/stat` + `/proc/meminfo` parsers | `sysinfo` 0.39.6 | Diff-based CPU math, hotplug CPU lists, mount filtering, and platform quirks are already handled and tested upstream |
| NVIDIA telemetry | `nvidia-smi` XML scraping or NVML FFI by hand | `nvml-wrapper` 0.13.0 | Safe wrapper over the dlopen'd C library; version skew and struct layouts handled upstream |
| XDG cache path | `$HOME/.cache` string concat | `directories` 6.0 | Correct `$XDG_CACHE_HOME` fallback + macOS/Windows paths for free; 1M/wk standard |
| Update availability | libalpm sync-DB diffing | `checkupdates` + `yay -Qu` subprocesses | Inherits pacman.conf/mirrorlist semantics; never risks `-Sy`-without-`-u` partial upgrades |
| French table alignment | Custom column-width engine | `std::fmt` width specifiers (`{:<12}`) inline in the bin | 5 tables, fixed columns — a `comfy-table`/`tabled` dep is unjustified weight for v1 (revisit in GUI phase if CLI tables grow) |
| Config/cache serialization | Custom text format for the cache file | `serde_json` (already vendored) | Versioned envelope (`v` field) follows the Phase 1 helper-JSON precedent |

**Key insight:** Phase 2 is a composition phase, not an invention phase — sysinfo owns `/proc`, NVML owns the GPU, `checkupdates` owns the sync DBs, and the only hand-written code is the ~120-line RAPL/hwmon sysfs glue plus the cache envelope and renderers. Every line of custom parsing is a future vendor-breakage liability.

## Common Pitfalls

### Pitfall 1: First CPU reading is always garbage
**What goes wrong:** snapshot shows 0% on every core because only one refresh ran.
**Why it happens:** sysinfo CPU % is diff-based; the docs warn the first call is inaccurate and two calls ≥200 ms apart are required [VERIFIED: sysinfo 0.39.6 source].
**How to avoid:** D-04's ~1 s window IS the fix — refresh → sleep 1 s → refresh → read. In watch mode, reuse the same `System` across ticks so ticks 2+ are always valid.
**Warning signs:** all-zero CPU table in manual test.

### Pitfall 2: Milliwatt / millidegree unit slips
**What goes wrong:** GPU power displayed as 42 780 W; temperature as 54 000 °C.
**Why it happens:** NVML `power_usage()` is milliwatts; hwmon `temp*_input` is millidegrees; sysinfo `Component::temperature()` is already °C — mixing sources mixes units.
**How to avoid:** convert at the source boundary (`/1000.0`, `/1000.0`) with unit tests on fixture values; plan includes a live cross-check (`power_usage()/1000 ≈ nvidia-smi` power.draw, `temperature() ≈ nvidia-smi` temp) as an implementation-time assertion.
**Warning signs:** values off by exactly 10³ or 10⁶.

### Pitfall 3: checkupdates missing or hanging offline
**What goes wrong:** LAND-03 hard-errors on machines without `pacman-contrib` (like this one) or blocks 60 s+ on dead network.
**Why it happens:** `checkupdates` is not in base `pacman` — it ships in `pacman-contrib`, absent on the live machine [VERIFIED: live probe + `pacman -Si pacman-contrib` → 1.13.1-1 available in extra].
**How to avoid:** `which("checkupdates")` gate → typed `UpdatesCheckerMissing` French state; 10–15 s timeout on both subprocesses; D-06 stale-with-age fallback so landing never blanks.
**Warning signs:** `landing` exits non-zero on a fresh install.

### Pitfall 4: Disk reports the wrong filesystem
**What goes wrong:** "system partition" shows `/boot/efi` or a snap mount instead of `/`.
**Why it happens:** `Disks` lists every mount including pseudo-filesystems; picking index 0 is wrong.
**How to avoid:** select by `mount_point() == Path::new("/")` exactly (D-08: the OS partition); `used = total - available` with `total_space()`/`available_space()` [VERIFIED: sysinfo 0.39.6 source]. Live root: `/dev/nvme0n1p5`, 876 GB total [VERIFIED: live probe].
**Warning signs:** used GB doesn't match `df /`.

### Pitfall 5: Watch mode floods or flickers
**What goes wrong:** NDJSON ticks interleave with progress noise, or human re-render scrolls instead of refreshing.
**Why it happens:** stray `eprintln!` progress lines inside the tick path; printing without clearing.
**How to avoid:** tick path prints exactly one payload (table-with-clear or JSON line) and nothing else; all diagnostics go through `tracing` (stderr, warn-default) outside the payload.
**Warning signs:** `archel sys cpu --watch --json | jq` fails to parse.

### Pitfall 6: Diacritics-guard violation wave
**What goes wrong:** dozens of new French table headers trip the Phase 1 diacritics audit (`no_hardcoded_french_diacritics_in_rust_sources`).
**Why it happens:** headers like "Utilisé", "Fréquence", "Température" written as Rust literals.
**How to avoid:** every header/unit/label is a `fr.json` key from the first commit; run `cargo test -p archel-core` (which owns the guard) after adding each renderer. Headers needed (non-exhaustive, planner finalizes): sections cpu/ram/disque/gpu/température/puissance, units (Go, MHz/GHz, °C, W), N/A states (`indisponible`, `capteur absent`), cache age (`il y a X min`), refresh states.
**Warning signs:** CI red on the i18n test after a render commit.

## Code Examples

### Per-core CPU + frequency snapshot (D-04 compliant)
```rust
// Source: sysinfo 0.39.6 src/common/system.rs (crate source read 2026-09-30)
use sysinfo::System;

pub struct CpuCore { pub id: usize, pub name: String, pub usage_pct: f32, pub freq_mhz: u64 }

pub fn cpu_snapshot() -> Vec<CpuCore> {
    let mut sys = System::new_all();
    sys.refresh_cpu_all();
    std::thread::sleep(std::time::Duration::from_secs(1)); // D-04 ~1 s window
    sys.refresh_cpu_all(); // + refresh_cpu_frequency included in refresh_cpu_all (CpuRefreshKind::everything)
    sys.cpus().iter().enumerate().map(|(i, c)| CpuCore {
        id: i, name: c.name().to_owned(), usage_pct: c.cpu_usage(), freq_mhz: c.frequency(),
    }).collect()
}
```

### System-partition disk (D-08)
```rust
// Source: sysinfo 0.39.6 src/common/disk.rs (crate source read 2026-09-30)
use sysinfo::Disks;
use std::path::Path;

let disks = Disks::new_with_refreshed_list();
let root = disks.iter().find(|d| d.mount_point() == Path::new("/"))
    .ok_or(ArchelError::SystemPartitionNotFound)?;
let total = root.total_space();
let free = root.available_space();
let used = total.saturating_sub(free);
```

### Hottest temperature per category (D-09)
```rust
// Source: sysinfo 0.39.6 src/common/component.rs (crate source read 2026-09-30)
use sysinfo::Components;

let mut comps = Components::new_with_refreshed_list();
comps.refresh(true);
let hottest_cpu: Option<f32> = comps.iter()
    .filter(|c| c.label().contains("Tctl") || c.label().contains("Tccd")
         || c.label().to_lowercase().contains("package") || c.label().to_lowercase().contains("core"))
    .filter_map(|c| c.temperature()) // Option<f32> °C — None skipped, never averaged
    .max_by(f32::total_cmp);
// Live expectation on this machine: Tccd1 47.75 °C wins over Tctl 41.25 °C [VERIFIED: live probe]
```

### Landing update-count cache envelope (D-05/D-06/D-07)
```rust
// Pattern: Phase 1 privileged.rs spawn/parse + v1 JSON envelope precedent (ARCHITECTURE.md Pattern 4)
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct UpdateCache { v: u32, computed_at: u64, repo: u32, aur: u32 }
const CACHE_TTL_SECS: u64 = 900; // D-05: 15 minutes

// Recompute (user context, never pkexec): `checkupdates | wc -l`, `yay -Qu --aur | wc -l`
// each under which()-gate + 15 s timeout; on failure return last cache + age (D-06).
// `--refresh` bypasses the mtime check (D-07). Atomic write: tmp + rename.
```

### Watch loop with NDJSON (D-02/D-03)
```rust
// Pattern: bin-side presentation loop (Phase 1: bin renders, core computes)
loop {
    let snap = archel_core::system::cpu_snapshot(); // System reused across ticks in real code
    if json { println!("{}", serde_json::to_string(&snap)?); } // D-03: one object per tick, no clear
    else { print!("\x1Bc"); print!("{}", render_french_table(&snap)); } // D-02: 500 ms re-render
    std::thread::sleep(std::time::Duration::from_millis(500));
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `sysinfo` 0.3x `refresh_all()` + `get_disks()` | 0.39 `refresh_cpu_all()` + `Disks::new_with_refreshed_list()` + `CpuRefreshKind` granularity | 0.30+ era (2024–2025) | Old snippets from training data do not compile — use the 0.39 API verified above, not remembered 0.2x/0.3x shapes |
| `nvidia-smi` scraping for GPU stats | `nvml-wrapper` persistent handle | Long-standing best practice (PITFALLS.md Pitfall 8) | Per-tick subprocess is an observer-overhead bug; handle-once is the only acceptable watch-mode path |
| Global `/etc/resolv.conf` / single-disk-index assumptions | Enumerate + match (`mount == "/"`, hwmon by `name`) | Stable Linux practice | Hardcoded indices break across boots and machines — enumerate everywhere |

**Deprecated/outdated:**
- `heim`, `systemstat` for stats: dead or shallower than sysinfo + sysfs — do not introduce (STACK.md What NOT to Use)
- `wireplumber` crate, `dbus` crate: irrelevant to Phase 2 (no audio/D-Bus work in this phase)
- `comfy-table`/`tabled` for v1 tables: unjustified weight for 5 fixed-column tables — `std::fmt` widths suffice

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `checkupdates` outputs one line per pending repo update and `yay -Qu --aur` one per AUR update, both exit 0 with empty output when fully updated | Pattern 5, LAND-03 | Counts wrong (off-by-header or error-path lines counted); mitigation: implementation-time probe on live machine comparing against `pacman -Qu` counts, filter non-package lines |
| A2 | `power_usage()` returns milliwatts (÷1000 → W) | Pattern 2, SYS-08 | Power off by 10³; mitigation: live cross-check vs `nvidia-smi --query-gpu=power.draw` is a plan verification step |
| A3 | `tokio` mini (`time,rt`) vs plain `std::thread::sleep` for watch/sampling waits | Standard Stack | Negligible — both satisfy D-02/D-04; planner picks, no user impact |
| A4 | Exact `sys` subcommand list beyond cpu/ram/disk/gpu (temp? power? all? freq folded into cpu?) | CLI shape | Low — D-11 locks the `sys` group + `landing` top-level, subcommand inventory is agent's-discretion; planner proposes, verify-work confirms UX |
| A5 | JSON schema field names + French table column layout | Code Examples | Low — internal choice, but reversibility-costly once scripted; planner should pick snake_case English keys mirroring struct fields and note the schema is v1-unstable |
| A6 | `Components` labels on this machine surface k10temp as `Tctl`/`Tccd1` (sysinfo reads the same hwmon `*_label` files probed live) | Pattern 4, SYS-06 | Low — fallback is the hand-rolled hwmon reader over identical files; implementation test asserts hottest == max of the two live values |

## Open Questions

1. **Should `pacman-contrib` (provides `checkupdates`) become a documented dependency / install step in Phase 2?**
   - What we know: absent on live machine; available as `extra/pacman-contrib 1.13.1-1`; LAND-03's repo half cannot work without it.
   - What's unclear: whether the user wants a `sudo pacman -S pacman-contrib` prerequisite note vs a plan task that installs it (sudo needs a password — Phase 1 established user-local workarounds).
   - Recommendation: planner adds graceful `UpdatesCheckerMissing` state (no hard error) + surfaces the one-line install hint in French; installation itself is a human step, not an executor step.
2. **Does `yay -Qu` behave offline the same way `checkupdates` does (non-zero exit, empty stdout)?**
   - What we know: both are network-dependent; D-06 requires stale-with-age on any failure.
   - What's unclear: exact exit codes/structured errors per tool when offline.
   - Recommendation: treat ANY non-zero exit or timeout from either tool as "refresh failed → stale-with-age", and pin exact codes in implementation tests on the live machine (including an offline simulation).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `libnvidia-ml.so` (driver 580) | SYS-05/06/08 (GPU) | ✓ | 580.178.04 | N/A backend with French reason (non-NVIDIA machines) |
| Intel-RAPL powercap (`package-0`, `core`) | SYS-07 (CPU power) | ✓ | live `energy_uj` | Honest N/A |
| k10temp hwmon (`Tctl`, `Tccd1`) | SYS-06 (CPU temp) | ✓ | 41–48 °C live | sysinfo Components → hwmon reader → N/A |
| `nvidia-smi` | Verification cross-check only | ✓ | 580.178.04 | — (not a runtime dep) |
| `checkupdates` (pacman-contrib) | LAND-03 (repo count) | ✗ | — (1.13.1-1 in extra) | `UpdatesCheckerMissing` state + install hint; AUR half still works |
| `yay` | LAND-03 (AUR count) | ✓ | present | AUR count N/A with reason if ever absent |
| Root fs `/` on `/dev/nvme0n1p5` | SYS-02 | ✓ | 876 GB total | — |
| Rust stable 1.98.1 + cargo | Build | ✓ | 1.98.1 (user-local rustup) | — |
| `sysinfo` rust-version floor | Build | ✓ | needs ≥1.95, have 1.98.1 | — |

**Missing dependencies with no fallback:**
- None blocking — every sensor has an honest-N/A path per D-10, and LAND-03 degrades per D-06.

**Missing dependencies with fallback:**
- `checkupdates` — fallback is the missing-checker state + `~/.cache` stale value; human installs `pacman-contrib` when convenient.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in) + `insta` 1.48.0 for snapshot tests [ASSUMED version — STACK.md pin, re-verify at plan time] |
| Config file | none — see Wave 0 |
| Quick run command | `cargo test -p archel-core --no-default-features` |
| Full suite command | `cargo test --workspace --no-default-features` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| LAND-01 | version string in landing snapshot | unit | `cargo test -p archel-core landing --no-default-features` | ❌ Wave 0 |
| LAND-02 | kernel version non-empty | unit | same suite | ❌ Wave 0 |
| LAND-03 | cache TTL/stale/refresh semantics | unit (fixture clock + fake cache file) | same suite | ❌ Wave 0 |
| SYS-01/02/04 | RAM/disk/freq values sane (total>0, used≤total) | unit (live, range-asserted) | same suite | ❌ Wave 0 |
| SYS-03 | per-core count == nproc, 0≤%≤100 after 1 s window | unit (live, slower ~1 s) | same suite | ❌ Wave 0 |
| SYS-05/06/08 | NVML values sane (used≤total, 0<°C<110, W within TDP) or typed N/A | unit (live NVIDIA assertions) | same suite | ❌ Wave 0 |
| SYS-06/07 | RAPL delta ≥0, wrap-handling unit test on fixtures | unit (fixture) + live | same suite | ❌ Wave 0 |
| CLI shape | `sys`/`landing` tables + `--json` parse + NDJSON 2-tick | integration/snapshot (`insta` on `--json`) | `cargo test -p archel --no-default-features` | ❌ Wave 0 |
| i18n guard | no hardcoded French diacritics in new `.rs` | unit (existing audit test covers repo-wide) | existing `i18n.rs` test | ✅ exists |

### Sampling Rate
- **Per task commit:** `cargo test -p archel-core --no-default-features`
- **Per wave merge:** `cargo test --workspace --no-default-features`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/archel-core/src/system.rs`, `sensors.rs`, `updates.rs`, `landing.rs` (+ `cache.rs` if split) — new modules with unit tests per row above
- [ ] CLI snapshot tests for `--json` schemas (`insta`) — first use of insta in this repo; add `insta` dev-dep at plan time
- [ ] Live-assertion bounds (TDP for 1070 Ti = 180 W [ASSUMED — verify via `nvidia-smi -q -d POWER` at implementation]; CPU temp < 105 °C silicon max for 5800X [ASSUMED])

## Security Domain

Phase 2 is read-only and unprivileged: no pkexec, no helper changes, no policy changes, no secrets, no network writes. Telemetry reads (`/proc`, sysfs, NVML dlopen, `checkupdates`/`yay -Qu` as user) expose no privileged surface.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — (no auth in Phase 2) |
| V3 Session Management | no | — |
| V4 Access Control | no | — (all sources world-readable; subprocesses run as the invoking user) |
| V5 Input Validation | yes | Cache file re-parse is defensive: `v` version gate + serde strictness; corrupt cache → recompute, never crash (mirrors Phase 1 HelperProtocol never-default-ok) |
| V6 Cryptography | no | — |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Corrupt/tampered cache file in `~/.cache` | Tampering | Version-gated envelope; parse failure → discard + recompute; atomic writes |
| `yay`/`checkupdates` output injection into counts | Tampering | Count lines, never eval; numeric-only parse; timeouts bound hangs |
| Symlinked `/sys` or `/proc` oddities in containers | Spoofing | Read failures → typed N/A, never panic; range-assert before display |

## Sources

### Primary (HIGH confidence)
- nvml-wrapper 0.13.0 crate source (downloaded + read 2026-09-30): `Nvml::init/device_by_index/device_count`, `Device::memory_info/temperature/power_usage/utilization_rates`, `TemperatureSensor::Gpu` path, `MemoryInfo{used,total}` fields, `examples/basic_usage.rs`
- sysinfo 0.39.6 crate source (downloaded + read 2026-09-30): `refresh_cpu_all/refresh_cpu_usage/refresh_cpu_frequency`, two-call requirement + `MINIMUM_CPU_UPDATE_INTERVAL`, `cpus/cpu_usage/frequency`, `refresh_memory/total_memory/used_memory/kernel_version`, `Disks::new_with_refreshed_list/mount_point/total_space/available_space`, `Components::new_with_refreshed_list/label/temperature→Option<f32>`
- Live machine probes (shell, 2026-09-30): hwmon0–4 names + k10temp values, RAPL `package-0`/`core` + live `energy_uj`, drm `card1` NVIDIA uevent + absent hwmon, `nvidia-smi` (54 °C/42.78 W/8192 MiB/driver 580), `checkupdates` absent + `pacman-contrib 1.13.1-1` in extra, root `/dev/nvme0n1p5` 876 GB, cpufreq node, kernel 7.2.6-arch2-1, Ryzen 7 5800X
- `cargo search` registry (2026-09-30): sysinfo 0.39.6, nvml-wrapper 0.13.0, directories 6.0.0 current
- `gsd_run query package-legitimacy` (2026-09-30): sysinfo/nvml-wrapper/directories all `OK`
- Phase 1 artifacts: 01-01/01-02 SUMMARies, 01-PATTERNS.md, live `cmds/mod.rs` + `archel-core/Cargo.toml` (bin→core, catalog, `gui` feature, privileged spawn pattern)

### Secondary (MEDIUM confidence)
- STACK.md / ARCHITECTURE.md / PITFALLS.md / SUMMARY.md (project research 2026-09-30): GPU backend trait, no-subprocess-per-tick, discovery-by-name, TTL-cache performance trap, `checkupdates` read-only semantics

### Tertiary (LOW confidence)
- `checkupdates`/`yay -Qu` exact output shape + offline exit codes [ASSUMED — A1, live probe at implementation]
- NVML `power_usage` milliwatt unit [ASSUMED — A2, live cross-check at implementation]
- `insta` 1.48.0 pin [ASSUMED — from STACK.md, re-verify at plan time]

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - all three new crates legitimacy-OK + version-current + APIs read from source; sysinfo/NVML pins corroborate STACK.md
- Architecture: HIGH - Phase 1 patterns directly extended (bin→core, catalog, spawn/parse); sensor strategy grounded in live-machine sysfs/NVML probes, not docs
- Pitfalls: HIGH - unit-slip and first-reading traps verified against crate docstrings; environment gap (`checkupdates` absent) discovered by probing, not assumed

**Research date:** 2026-09-30
**Valid until:** 2026-10-30 (stable domain; re-probe hwmon/RAPL paths if kernel ≥7.3 lands, re-check `pacman-contrib` presence at plan time)
