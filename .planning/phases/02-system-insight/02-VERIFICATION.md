---
phase: 02-system-insight
verified: 2026-10-01T00:00:00Z
status: passed
score: 4/4 must-haves verified
covered_files:
  - .planning/phases/02-system-insight/02-01-PLAN.md
  - .planning/phases/02-system-insight/02-02-PLAN.md
  - .planning/phases/02-system-insight/02-03-PLAN.md
  - .planning/phases/02-system-insight/02-01-SUMMARY.md
  - .planning/phases/02-system-insight/02-02-SUMMARY.md
  - .planning/phases/02-system-insight/02-03-SUMMARY.md
  - crates/archel-core/src/updates.rs
  - crates/archel-core/src/landing.rs
  - crates/archel-core/src/system.rs
  - crates/archel-core/src/sensors.rs
  - crates/archel/src/cmds/landing_cmd.rs
  - crates/archel/src/cmds/sys_cmds.rs
covered_digest: "v2:sha256:b46b2be6c454134fc03c3fd9fccfc7b8af6cec3c49735b3e7aeccd178d8ff996"
behavior_unverified: 0
overrides_applied: 0
---

# Phase 02: System Insight Verification Report

**Phase Goal:** Users can see at a glance what their machine is doing — landing facts plus live system telemetry
**Verified:** 2026-10-01 (verifier live runs on the Archel machine)
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (from ROADMAP.md Phase 2 Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User can see app version, running kernel version, and pending update count (pacman + AUR) in one landing snapshot | ✓ VERIFIED | `archel landing` prints `Version: 0.1.0`, `Noyau: 7.2.6-arch2-1`, AUR counts + guided pacman-contrib hint, exit 0 (verifier live run). `landing::snapshot()` composes `library_version + kernel_version + pending_counts` in `crates/archel-core/src/landing.rs`. Missing-checkupdates degrades to AUR-only + French hint, never blank/crash. |
| 2 | User can see RAM usage, disk usage per mount, and live per-core CPU graph with current clock speed | ✓ VERIFIED (note) | `sys cpu` shows 16 per-core % + MHz rows live; `sys ram` shows 5.2/31.2 Gio; `sys disk` shows system-partition used/total/free (816.0 Gio total = `df /`). NOTE: disk is system-partition-only per approved D-08, not all mounts — see observation below. "Graph" is satisfied at CLI-proven level by the per-core table + 500 ms `--watch` re-render; GUI graphs belong to Phase 6. |
| 3 | User can see GPU VRAM used/total, CPU/GPU temperatures, and real-time CPU/GPU power draw — with honest N/A where hardware has no sensor | ✓ VERIFIED (note) | `sys gpu` shows GTX 1070 Ti 1.3/8.0 Gio live via NVML; `sys temp` shows CPU 44.9 °C (hottest k10temp pick) + GPU 54.0 °C (= nvidia-smi); `sys power` shows GPU 36.5 W live + honest CPU N/A. NOTE: CPU watts N/A is permission-gated (RAPL `energy_uj` mode 400 root-only), not missing hardware — fixture tests prove the delta math; watts appear where the kernel permits reads. Honest N/A per D-10, never 0.0. |
| 4 | User can fetch every value above headlessly via `archel-cli` (JSON output), proving the read API before any GUI exists | ✓ VERIFIED | `landing --json` parses (keys app_version/kernel_version/repo/aur); `sys all --json` parses with all six top-level keys (cpu/disk/gpu/power/ram/temps); 8 insta snapshots pin landing + every sys metric schema; `--watch --json` streams 6/6 parseable NDJSON ticks with no payload pollution (verifier live runs). |

**Score:** 4/4 truths verified (0 present-but-behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/archel-core/src/updates.rs` | Checker spawn + TTL cache + stale-with-age | ✓ VERIFIED | Exists, substantive (which-gated duct spawn, 15 s timeout, `{v:1, computed_at, repo, aur}` atomic cache, fixture-clock unit tests for stale/refresh-bypass/corrupt) |
| `crates/archel-core/src/landing.rs` | Snapshot composer | ✓ VERIFIED | Exists, substantive, composes version + kernel + counts; `table_labels()` keeps French out of the bin |
| `crates/archel-core/src/system.rs` | Monitor + cpu/ram/disk snapshots | ✓ VERIFIED | Exists, substantive (two-pass CPU, `mount_point == /` exact match, `SystemPartitionNotFound` typed variant, live-range + fixture tests) |
| `crates/archel-core/src/sensors.rs` | GpuBackend + RAPL + hottest temps | ✓ VERIFIED | Exists, substantive (`NvmlBackend`/`NoGpu` probe-fallback, RAPL discovery-by-name with wrap math, hottest-pick, named unit consts, Option-typed end-to-end) |
| `crates/archel/src/cmds/landing_cmd.rs` | Landing table + JSON + watch | ✓ VERIFIED | Exists, substantive, renders exclusively from core + fr.json catalog (zero French literals in bin per grep) |
| `crates/archel/src/cmds/sys_cmds.rs` | cpu/ram/disk/gpu/temp/power/all renderers | ✓ VERIFIED | Exists, substantive (per-metric renderers reused section-by-section by `all`, watch loop, NDJSON framing) |
| `crates/archel/src/cmds/snapshots/` (8× .snap) | Pinned --json schemas | ✓ VERIFIED | All 8 present (landing + cpu/ram/disk/gpu/temp/power/all), suite green |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `landing_cmd` | `landing::snapshot` | bin-calls-core, renders only | WIRED | 7 core call sites in landing_cmd.rs; no sysinfo/sysfs in bin |
| `sys_cmds` | `system::Monitor` + `sensors` | bin-calls-core, renders only | WIRED | 29 core call sites in sys_cmds.rs |
| `updates` cache | `landing` snapshot | TTL envelope + stale-with-age | WIRED | Expired+failure returns stale counts with age; partial refresh keeps old timestamp; corrupt/skew recomputes (unit-tested) |
| `Monitor` | watch loop | one instance reused across ticks | WIRED | Ticks 2+ carry valid diff-based CPU (Pitfall 1 guarded by not-all-zero live test) |
| NVML handle | watch ticks | probed once via `Sensors::new`, reused | WIRED | `NvmlBackend::probe()` at construction; no subprocess in tick path (only doc-comment mentions of nvidia-smi) |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| landing table | version/kernel/counts | `library_version` + sysinfo kernel + checkupdates/yay line counts | ✓ live (AUR 3, kernel 7.2.6) | ✓ FLOWING |
| sys cpu | per-core % + MHz | sysinfo two-pass refresh | ✓ live (16 cores, non-zero present) | ✓ FLOWING |
| sys ram/disk | used/total | sysinfo memory + Disks root match | ✓ live (matches `df /` total/free) | ✓ FLOWING |
| sys gpu/temp/power | VRAM/temps/watts | NVML dlopen + Components/hwmon + RAPL | ✓ live (VRAM, both temps, GPU watts); CPU watts honest N/A | ✓ FLOWING |

### Behavioral Spot-Checks (verifier live runs, this machine)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| landing table, exit 0 without checkupdates | `cargo run -q -p archel --no-default-features -- landing` | French table + contrib hint, exit 0 | ✓ PASS |
| landing --json parses | `... -- landing --json \| python3 -c json.load` | `{"app_version":"0.1.0","kernel_version":...,"repo":{"Unavailable":"CheckerMissing"},"aur":{"Fresh":3}}` | ✓ PASS |
| landing --refresh recomputes | `... -- landing --refresh` | exit 0, fresh table | ✓ PASS |
| sys cpu/ram/disk/gpu/temp/power tables | `... -- sys {cpu,ram,disk,gpu,temp,power}` | All render live French tables (see evidence) | ✓ PASS |
| sys all --json has all keys | `... -- sys all --json \| python3 assert` | keys `['cpu','disk','gpu','power','ram','temps']` | ✓ PASS |
| NDJSON watch stream | `timeout 5 ... sys ram --watch --json` | 6 ticks, all `json.loads` OK | ✓ PASS |
| Full workspace suite | `cargo test --workspace --no-default-features` | 66 core + 22 bin + 5 privileged, 0 failed | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| LAND-01 | 02-01 | App version on home page | ✓ SATISFIED | landing table `Version: 0.1.0` |
| LAND-02 | 02-01 | Kernel version on home page | ✓ SATISFIED | landing table `Noyau: 7.2.6-arch2-1` |
| LAND-03 | 02-01 | Pending update count (repo + AUR) | ✓ SATISFIED | AUR count live; repo degrades to guided hint without pacman-contrib (user setup tracked in 02-USER-SETUP.md) |
| SYS-01 | 02-02 | RAM used/total | ✓ SATISFIED | 5.2/31.2 Gio, used prominent |
| SYS-02 | 02-02 | Disk usage per mount | ✓ SATISFIED (scoped) | System partition only per D-08 — see observation |
| SYS-03 | 02-02 | Live per-core CPU | ✓ SATISFIED | 16 cores, not-all-zero live test |
| SYS-04 | 02-02 | CPU clock speed | ✓ SATISFIED | Per-core MHz column |
| SYS-05 | 02-03 | GPU VRAM used/total | ✓ SATISFIED | 1.3/8.0 Gio via NVML |
| SYS-06 | 02-03 | CPU + GPU temperature | ✓ SATISFIED | Hottest-pick both categories |
| SYS-07 | 02-03 | Real-time CPU power (W) | ✓ SATISFIED (honest N/A) | Fixture-proven math; live N/A root-gated RAPL |
| SYS-08 | 02-03 | Real-time GPU power (W) | ✓ SATISFIED | 36.5 W ≈ nvidia-smi |

No orphaned requirements: all 11 Phase 2 IDs (LAND-01..03, SYS-01..08) are claimed across the three plans.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | None | — | No TBD/FIXME/TODO/HACK/placeholder/empty-return matches in any of the 6 implementation files; no French literals in the bin; no hardcoded `hwmonN`/`card0`/`:0` outside comments/tests; no per-tick subprocess |

### Deferred Human-Checks — Explicit Disposition

1. **Missing-checkupdates graceful state** (02-VALIDATION.md manual item, coverage D7 in 02-03-SUMMARY): **CLOSED by verifier live run.** This machine lacks pacman-contrib, so the verifier directly observed the degraded state: full table + `« checkupdates » est introuvable : lancez « sudo pacman -S pacman-contrib »`, exit 0, `--json` emits `{"Unavailable":"CheckerMissing"}`. No human check remains; the `sudo pacman -S pacman-contrib` install itself stays tracked in 02-USER-SETUP.md.
2. **Watch re-render flicker feel at 2 Hz** (02-VALIDATION.md manual item, coverage D4/D6): **NON-BLOCKING, carried as polish.** Automation proves everything automatable: `ESC-c` full-reset clear, 4 re-renders/2 s cadence, 6/6 parseable NDJSON ticks, clean Ctrl+C semantics. What remains is purely perceptual (does 2 Hz full-reset flicker annoy a human eye in a real terminal) — it cannot fail the phase goal (CLI-proven telemetry API) and both plans declare it non-blocking. Recommended: fold into Phase 6 GUI manual UAT or first-use eyeballing; no `human_needed` gate on Phase 2.

### Observations (non-blocking, informational)

- **SC2 wording vs D-08:** the roadmap SC says "disk usage per mount" while the approved D-08 scopes `sys disk` to the system partition (`mount_point == /` exactly). This was a deliberate, documented design decision (never report a wrong filesystem), consistently reflected in all three plans' must-haves. Recorded here so Phase 6 can decide whether the System tab needs multi-mount; not a Phase 2 gap.
- **Disk `used` vs `df`:** `used = total − available` literally per plan reads ~5% above `df` Used (root-reserved blocks). Total and free match `df /` exactly. Documented in 02-02-SUMMARY; correct per spec.

### Gaps Summary

None. All four roadmap success criteria hold on the live machine, all 11 requirement IDs are satisfied (two with explicitly approved scoping/honesty caveats), the full headless workspace suite is green (66 + 22 + 5), all 8 JSON schemas are insta-pinned, and both deferred manual checks are dispositioned above. No GUI exists yet, satisfying the API-before-GUI ordering gate. Ready for Phase 3.

---
_Verified: 2026-10-01_
_Verifier: the agent (gsd-verifier)_
