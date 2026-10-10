---
phase: 02-system-insight
verified: 2026-10-10T00:00:00Z
status: passed
score: 4/4 goal-backward checks satisfied
behavior_unverified: 0
overrides_applied: 0
covered_files:
  - .planning/phases/02-system-insight/02-01-PLAN.md
  - .planning/phases/02-system-insight/02-01-SUMMARY.md
  - .planning/phases/02-system-insight/02-02-PLAN.md
  - .planning/phases/02-system-insight/02-02-SUMMARY.md
  - .planning/phases/02-system-insight/02-03-PLAN.md
  - .planning/phases/02-system-insight/02-03-SUMMARY.md
  - crates/archel-core/Cargo.toml
  - crates/archel-core/locales/fr.json
  - crates/archel-core/src/error.rs
  - crates/archel-core/src/landing.rs
  - crates/archel-core/src/lib.rs
  - crates/archel-core/src/sensors.rs
  - crates/archel-core/src/system.rs
  - crates/archel-core/src/updates.rs
  - crates/archel/src/cmds/landing_cmd.rs
  - crates/archel/src/cmds/mod.rs
  - crates/archel/src/cmds/sys_cmds.rs
covered_digest: "v2:sha256:c2b4a57b21ba7819d7eaeb5e7d1b84fbed8f518e8cf2a041c0226cd5d290f539"
re_verification:
  previous_status: passed
  previous_score: 4/4
  previous_digest: "v2:sha256:bf69cf5d7608c2ce1771054c994fc2eece2d61c17c7ea262f9be260ed29e7ba2"
  gaps_closed: []
  gaps_remaining: []
  regressions: []
---

# Phase 02: System Insight Verification Report

**Phase Goal:** Users can see at a glance what their machine is doing — landing facts plus live system telemetry
**Verified:** 2026-10-10 (re-verification — prior report's covered files legitimately changed during Phases 05/06 and code-review fixes; plans NOT re-executed, goal re-verified against current codebase)
**Status:** passed
**Re-verification:** Yes — previous report (2026-09-30, digest `bf69cf5d…`) went stale because shared core files (`error.rs`, `lib.rs`, `mod.rs`, `fr.json`, `Cargo.toml`, `sys_cmds.rs`) were legitimately modified by later-phase work. This pass re-proved every success criterion against the **current** code with fresh automated runs and live headless CLI executions on the Archel machine. No regressions found: all four truths still hold.

## Goal Achievement

### Observable Truths (from ROADMAP.md Phase 2 Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User can see app version, running kernel version, and pending update count (pacman + AUR) in one landing snapshot | ✓ VERIFIED | Fresh live run: `archel landing --json` → keys `['app_version','aur','kernel_version','repo']`, values `app: 0.1.0`, `kernel: 7.2.6-arch2-1`, `repo: {Fresh: 194}`, `aur: {Fresh: 4}`, exit 0. `landing::snapshot()` composes `library_version + kernel_version + updates::pending_counts` (`crates/archel-core/src/landing.rs` L34-41). pacman-contrib is now installed (`/usr/bin/checkupdates`), so the full repo path is live; the missing-checker degrade path remains unit-tested (`landing.rs` `CheckerMissing` tests). `--refresh` exit 0. |
| 2 | User can see RAM usage, disk usage per mount, and live per-core CPU graph with current clock speed | ✓ VERIFIED | Fresh live run: `sys all --json` parses with keys `['cpu','disk','gpu','power','ram','temps']`; `sys ram --watch --json` streamed 8/8 parseable NDJSON ticks (500 ms loop). Live core tests green in this run: `live_cpu_snapshot_is_not_all_zero`, `live_cpu_snapshot_has_sane_per_core_numbers`, `live_core_count_matches_parallelism`, `live_ram_values_are_sane`, `live_disk_reports_the_root_partition`. `system.rs` keeps two-pass CPU (`refresh_cpu_all` + delay + re-refresh), `freq_mhz` per core, exact `mount_point == "/"` match with typed `SystemPartitionNotFound`. "Graph" satisfied at CLI-proven level (per-core table + watch re-render); GUI graphs are Phase 6 scope. Disk scoped to system partition per approved D-08 (see observations). |
| 3 | User can see GPU VRAM used/total, CPU/GPU temperatures, and real-time CPU/GPU power draw — with honest N/A where hardware has no sensor | ✓ VERIFIED | Fresh live run: `sys gpu` → GTX 1070 Ti, VRAM 1.7/8.0 Gio via NVML; `sys temp` → CPU 40.8 °C + GPU 54.0 °C (hottest-pick per category); `sys power` → GPU 36.7 W live + CPU `N/A` (honest, never 0.0 — RAPL `energy_uj` root-gated on this machine; delta math fixture-proven, `live` anchor comments pin ~34 W cross-check). `sensors.rs` intact: `NvmlBackend`/`NoGpu` probe fallback, RAPL discovery-by-name (`POWERCAP_DIR`, `RAPL_PACKAGE_PREFIX`), `CPU_HWMON_DRIVERS` name-match (no `hwmon2`/`card0` production literals — grep confirms matches only in doc comments and tmp-dir test fixtures). |
| 4 | User can fetch every value above headlessly via `archel-cli` (JSON output), proving the read API before any GUI exists | ✓ VERIFIED | Fresh live runs (headless, this machine): `landing --json` and `sys all --json` both parse with full key sets; NDJSON watch 8/8 ticks `json.loads` OK. All 8 insta schema snapshots present and green in this run (`snapshots/`: `landing`, `sys_cpu`, `sys_ram`, `sys_disk`, `sys_gpu`, `sys_temp`, `sys_power`, `sys_all`). Bin renders exclusively from core (`mod.rs` routes `Command::Landing`/`Sys` to `landing_cmd`/`sys_cmds`; no sysinfo/sysfs/Command::new/duct in `sys_cmds.rs` tick path). |

**Score:** 4/4 goal-backward checks satisfied (0 present-but-behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/archel-core/src/updates.rs` | Checker spawn + TTL cache + stale-with-age | ✓ VERIFIED | 584 lines; `CACHE_TTL_SECS=900`, versioned `{v:1, computed_at, repo, aur}` envelope, `ttl_valid()` pure fn, `Stale{count, age_secs}` variant, corrupt/version-skew discard, line-count-only stdout interpretation (`.lines()`, no shell/eval) |
| `crates/archel-core/src/landing.rs` | Snapshot composer | ✓ VERIFIED | 194 lines; composes version + kernel + counts; `table_labels()` keeps French out of the bin; degrade tests for `CheckerMissing`/`RefreshFailed` |
| `crates/archel-core/src/system.rs` | Monitor + cpu/ram/disk snapshots | ✓ VERIFIED | 399 lines; two-pass CPU, lazy ~1 s first cpu() window, `mount_point == "/"` exact, `SystemPartitionNotFound`; 6 live tests green this run |
| `crates/archel-core/src/sensors.rs` | GpuBackend + RAPL + hottest temps | ✓ VERIFIED | 800 lines; NVML/NoGpu fallback, powercap discovery by name, hottest-pick, Option-typed end-to-end, live cross-check anchors |
| `crates/archel/src/cmds/landing_cmd.rs` | Landing table + JSON + refresh/watch | ✓ VERIFIED | 131 lines; renders only from `landing::snapshot`/`table_labels`/catalog; insta snapshot green |
| `crates/archel/src/cmds/sys_cmds.rs` | cpu/ram/disk/gpu/temp/power/all renderers | ✓ VERIFIED | 579 lines; `Monitor` created OUTSIDE each watch loop and reused across ticks; NDJSON framing; N/A via `unavailable_label()` |
| `crates/archel/src/cmds/snapshots/` (8× phase-2 .snap) | Pinned --json schemas | ✓ VERIFIED | All 8 present; `schema_sys_all_is_pinned` and siblings green in this run |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `mod.rs` CLI dispatch | `landing_cmd` / `sys_cmds` | `Command::Landing`/`Command::Sys` match arms | WIRED | `mod.rs` L614-621 route all metrics |
| `landing_cmd` | `landing::snapshot` | bin-calls-core, renders only | WIRED | L25 core call; labels from `landing::table_labels()` |
| `sys_cmds` | `system::Monitor` + `sensors` | bin-calls-core, renders only | WIRED | Monitor + `Sensors::new()` per command; no sysinfo/sysfs/Command::new in bin |
| `updates` cache | `landing` snapshot | TTL envelope + stale-with-age | WIRED | `pending_counts(force_refresh)`; expired+failure → `Stale{count,age_secs}`; partial refresh keeps old timestamp (unit-tested) |
| `Monitor` | watch loop | one instance created outside loop, reused | WIRED | `run_cpu/ram/disk` create Monitor before `loop { }`; 8/8 NDJSON ticks parse live |
| NVML handle | watch ticks | probed once via `Sensors::new`, reused | WIRED | No subprocess in any tick path (grep: zero `Command::new`/`duct` in sys_cmds.rs) |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| landing table/JSON | version/kernel/repo/aur | `library_version` + sysinfo kernel + checkupdates/yay line counts | ✓ live (`repo: Fresh 194`, `aur: Fresh 4`, kernel 7.2.6-arch2-1) | ✓ FLOWING |
| sys cpu | per-core % + MHz | sysinfo two-pass refresh | ✓ live (not-all-zero test passes on real hardware) | ✓ FLOWING |
| sys ram/disk | used/total | sysinfo memory + root-mount disk match | ✓ live (JSON keys present, `live_disk_reports_the_root_partition` green) | ✓ FLOWING |
| sys gpu/temp/power | VRAM/temps/watts | NVML dlopen + Components/hwmon discovery + RAPL | ✓ live (VRAM 1.7/8.0 Gio, CPU 40.8 °C + GPU 54.0 °C, GPU 36.7 W; CPU watts honest N/A on root-gated RAPL) | ✓ FLOWING |

### Behavioral Spot-Checks (fresh verifier live runs, this machine, 2026-10-10)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Core suite headless | `cargo test --offline -p archel-core --no-default-features` | 277 passed, 0 failed, 1 ignored | ✓ PASS |
| Bin suite headless (incl. 8 insta snapshots + thin-view audit) | `cargo test --offline -p archel --no-default-features` | 125 passed, 0 failed | ✓ PASS |
| landing --json parses | `archel landing --json \| python3 json.load` | keys `['app_version','aur','kernel_version','repo']` | ✓ PASS |
| sys all --json parses | `archel sys all --json \| python3 json.load` | keys `['cpu','disk','gpu','power','ram','temps']` | ✓ PASS |
| NDJSON watch stream | `timeout 5 archel sys ram --watch --json` | 8 ticks, 8/8 `json.loads` OK | ✓ PASS |
| landing table + refresh exit 0 | `archel landing` / `archel landing --refresh` | exit 0 both | ✓ PASS |
| GPU/temp/power live values | `archel sys gpu` / `sys temp` / `sys power` | VRAM 1.7/8.0 Gio; 40.8 °C + 54.0 °C; GPU 36.7 W + CPU N/A | ✓ PASS |

### Prohibitions (flagged-unverified, judgment tier — autonomous LLM-judge verdicts with mirror evidence)

| Prohibition | Judge verdict | Mirror evidence |
|-------------|---------------|-----------------|
| landing must not exit non-zero / hide snapshot on refresh failure | holds | live exit 0; `RefreshFailed`/`CheckerMissing` degrade paths unit-tested |
| checker stdout interpreted by line-counting only | holds | `.lines()` only; no shell, no eval of package names |
| CPU snapshot must not report all-zero cores from one pass | holds | two-pass refresh; `live_cpu_snapshot_is_not_all_zero` green this run |
| disk must not report a non-root filesystem | holds | exact `mount_point == "/"` match; `live_disk_reports_the_root_partition` green |
| GPU telemetry must not shell out per tick | holds | zero `Command::new`/`duct` in sys_cmds tick path; NVML handle reused |
| sensor paths must not be hardcoded (`hwmon2`/`card0`/`:0`) | holds | production code enumerates by name; grep hits only in doc comments + tmp-dir test fixtures |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| LAND-01 | 02-01 | App version on home page | ✓ SATISFIED | live JSON `app_version: 0.1.0` |
| LAND-02 | 02-01 | Kernel version on home page | ✓ SATISFIED | live JSON `kernel_version: 7.2.6-arch2-1` |
| LAND-03 | 02-01 | Pending update count (repo + AUR) | ✓ SATISFIED | live `repo: Fresh 194` + `aur: Fresh 4` (checkupdates now installed — earlier user_setup item satisfied); degrade path unit-tested |
| SYS-01 | 02-02 | RAM used/total | ✓ SATISFIED | live test + JSON key green |
| SYS-02 | 02-02 | Disk usage per mount | ✓ SATISFIED (scoped) | System partition per approved D-08 — see observations |
| SYS-03 | 02-02 | Live per-core CPU | ✓ SATISFIED | not-all-zero + sane-numbers live tests green |
| SYS-04 | 02-02 | CPU clock speed | ✓ SATISFIED | `freq_mhz` per core, rendered + snapshot-pinned |
| SYS-05 | 02-03 | GPU VRAM used/total | ✓ SATISFIED | live 1.7/8.0 Gio via NVML |
| SYS-06 | 02-03 | CPU + GPU temperature | ✓ SATISFIED | live 40.8 °C / 54.0 °C hottest-pick |
| SYS-07 | 02-03 | Real-time CPU power (W) | ✓ SATISFIED (honest N/A) | fixture-proven delta math; live N/A is root-gated RAPL, D-10 honest-N/A design |
| SYS-08 | 02-03 | Real-time GPU power (W) | ✓ SATISFIED | live 36.7 W via NVML |

No orphaned requirements: all 11 Phase 2 IDs (LAND-01..03, SYS-01..08) are claimed across the three plans.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | None | — | Debt-marker sweep (`TBD|FIXME|XXX`) across all 11 changed implementation files + fr.json + Cargo.toml: exit 1 (zero matches). No per-tick subprocess, no hardcoded sensor paths, no stub returns in tick/render paths. |

### Observations (non-blocking, informational)

- **SC2 wording vs D-08:** roadmap says "disk usage per mount"; approved D-08 scopes `sys disk` to the system partition (`mount_point == /` exactly) so a wrong filesystem is never reported. Deliberate, documented, consistent across all three plans; Phase 6 may revisit multi-mount. Not a Phase 2 gap.
- **CPU watts N/A is permission-gated, not missing hardware:** RAPL `energy_uj` is root-only on this machine; the delta math is fixture-proven and watts appear wherever the kernel permits reads. The truth as written ("or honest N/A where no sensor exists") is satisfied by the honest-N/A design (D-10).
- **State drift since prior verification:** pacman-contrib was installed meanwhile (`checkupdates` now at `/usr/bin/checkupdates`), upgrading the repo-count path from degraded-with-hint to fully live. The USER-SETUP tracked install is effectively done.
- **Watch flicker feel at 2 Hz:** purely perceptual polish (all prior reports dispositioned it non-blocking); automation proves 8/8 parseable NDJSON ticks. Fold into Phase 6 first-use eyeballing.

### Gaps Summary

None. All four roadmap success criteria re-proven against the current codebase with fresh headless runs (277 + 125 tests green, 8/8 live CLI spot-checks), all 11 requirement IDs satisfied (two with explicitly approved scoping/honesty caveats), all 8 JSON schemas insta-pinned and green, no debt markers, no regressions from later-phase edits to shared files. Fresh digest `c2b4a57b…` replaces stale `bf69cf5d…`.

---
_Verified: 2026-10-10_
_Verifier: the agent (gsd-verifier)_
