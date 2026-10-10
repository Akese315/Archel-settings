---
phase: 02-system-insight
verified: 2026-10-11T00:00:00Z
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
covered_digest: "v2:sha256:50ab3f6f0e7f04619d17737ca38e40ea3f0aac1d00d4ef70d362afb8683ce667"
re_verification:
  previous_status: passed
  previous_score: 4/4
  previous_digest: "v2:sha256:c2b4a57b21ba7819d7eaeb5e7d1b84fbed8f518e8cf2a041c0226cd5d290f539"
  gaps_closed: []
  gaps_remaining: []
  regressions: []
---

# Phase 02: System Insight Verification Report

**Phase Goal:** Users can see at a glance what their machine is doing — landing facts plus live system telemetry
**Verified:** 2026-10-11 (v1.0 milestone-close re-verification — stale only because post-report commit `bf52ba1` modified covered file `crates/archel-core/src/updates.rs`; plans NOT re-executed, goal re-verified against current codebase)
**Status:** passed
**Re-verification:** Yes — previous report (2026-10-10, digest `c2b4a57b…`) went stale solely due to commit `bf52ba1` (2026-10-10 22:38), a 30-line addition to `updates.rs`: `.unchecked()` on the `run_checker` duct spawn (closes the phase-03 deferred finding where `checkupdates` exit 2 on a fully-updated system errored at `wait_timeout` instead of reaching `map_update_exit`'s Ok arm) plus regression test `exit_two_spawn_reaches_exit_mapping`. No other covered file changed since the prior digest (`45dad26`/`53dd31e` at 12:32–12:40 local predate the prior verifier run — its own spot-check already counted the 125-test bin suite including the thin-view audit). All four success criteria re-proven against current code with fresh offline test runs and live headless CLI executions. No regressions.

## Goal Achievement

### Observable Truths (from ROADMAP.md Phase 2 Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User can see app version, running kernel version, and pending update count (pacman + AUR) in one landing snapshot | ✓ VERIFIED | Fresh live run on this machine (binary rebuilt at 00:12 to include `bf52ba1`): `archel landing --json` → keys `['app_version','aur','kernel_version','repo']`, values `app: 0.1.0`, `kernel: 7.2.6-arch2-1`, `repo: {Fresh: 196}`, `aur: {Fresh: 4}`, exit 0. `landing::snapshot()` composes `library_version + kernel_version + updates::pending_counts` (`landing.rs` L35-41). The `bf52ba1` fix verified directly in code: `.unchecked()` at `updates.rs` L187 with load-bearing comment, `map_update_exit` retains `("checkupdates", Some(2)) => Ok(())` (L163), stdout interpreted by `.lines()` count only (L150-152). Named regression test `exit_two_spawn_reaches_exit_mapping` run in isolation this pass: 1 passed. TTL cache + stale-with-age paths intact (`ttl_valid`, `refresh_or_cached`, partial-refresh timestamp preservation). |
| 2 | User can see RAM usage, disk usage per mount, and live per-core CPU graph with current clock speed | ✓ VERIFIED | Fresh live run: `sys all --json` parses with keys `['cpu','disk','gpu','power','ram','temps']`; `sys ram --watch --json` streamed 10/10 parseable NDJSON ticks (500 ms loop). Live core tests green in this run: `live_cpu_snapshot_is_not_all_zero`, `live_cpu_snapshot_has_sane_per_core_numbers`, `live_core_count_matches_parallelism`, `live_ram_values_are_sane`, plus disk root-partition test. `system.rs` unchanged since prior pass: two-pass CPU (`refresh_cpu_all` L87/L102 + delay + re-refresh), `freq_mhz` per core (L176), exact root-mount match with typed `SystemPartitionNotFound` (L125). "Graph" satisfied at CLI-proven level (per-core table + watch re-render); GUI graphs are Phase 6 scope. Disk scoped to system partition per approved D-08 (see observations). |
| 3 | User can see GPU VRAM used/total, CPU/GPU temperatures, and real-time CPU/GPU power draw — with honest N/A where hardware has no sensor | ✓ VERIFIED | Fresh live run: `sys gpu` → GTX 1070 Ti, VRAM 1.7/8.0 Gio via NVML; `sys temp` → CPU 45.6 °C + GPU 54.0 °C (hottest-pick per category); `sys power` → GPU 34.8 W live + CPU `N/A` (honest, never 0.0 — RAPL `energy_uj` root-gated on this machine; delta math fixture-proven). `sensors.rs` unchanged since prior pass: `NvmlBackend`/`NoGpu` probe fallback (L85/L14), RAPL discovery-by-name (`POWERCAP_DIR` L44, `RAPL_PACKAGE_PREFIX` L65), `CPU_HWMON_DRIVERS` name-match (L70) — no `hwmon2`/`card0` production literals. |
| 4 | User can fetch every value above headlessly via `archel-cli` (JSON output), proving the read API before any GUI exists | ✓ VERIFIED | Fresh headless runs on this machine: `landing --json` and `sys all --json` both parse with full key sets; NDJSON watch 10/10 ticks `json.loads` OK. All 8 insta schema snapshots green in this run (`schema_sys_all_is_pinned` and siblings in the 125-pass bin suite). Bin renders exclusively from core: `mod.rs` routes `Command::Landing`/`Command::Sys` to `landing_cmd`/`sys_cmds` (L614-617); grep confirms zero `Command::new`/`duct::` in `sys_cmds.rs` (count 0); `Monitor::new()` created outside each watch loop and reused across ticks (L40/L57/L74), `Sensors::new()` probed once per command (L92). |

**Score:** 4/4 goal-backward checks satisfied (0 present-but-behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/archel-core/src/updates.rs` | Checker spawn + TTL cache + stale-with-age | ✓ VERIFIED | 614 lines (584 + `bf52ba1`'s 30); `.unchecked()` spawn (L187, load-bearing comment), `CACHE_TTL_SECS=900`, versioned `{v:1, computed_at, repo, aur}` envelope, `ttl_valid()` pure fn, `Stale{count, age_secs}` variant, corrupt/version-skew discard, line-count-only stdout interpretation; new regression test `exit_two_spawn_reaches_exit_mapping` green in isolation and in the 278-test suite |
| `crates/archel-core/src/landing.rs` | Snapshot composer | ✓ VERIFIED | Unchanged since prior pass; composes version + kernel + counts (L35-41); `table_labels()` keeps French out of the bin; degrade tests for `CheckerMissing`/`RefreshFailed` green |
| `crates/archel-core/src/system.rs` | Monitor + cpu/ram/disk snapshots | ✓ VERIFIED | Unchanged since prior pass; two-pass CPU, lazy ~1 s first cpu() window, exact root-mount match, `SystemPartitionNotFound`; live tests green this run |
| `crates/archel-core/src/sensors.rs` | GpuBackend + RAPL + hottest temps | ✓ VERIFIED | Unchanged since prior pass; NVML/NoGpu fallback, powercap discovery by name, hottest-pick, Option-typed end-to-end; live cross-check confirmed this run (VRAM/temps/GPU watts) |
| `crates/archel/src/cmds/landing_cmd.rs` | Landing table + JSON + refresh/watch | ✓ VERIFIED | Renders only from `landing::snapshot`/`table_labels`/catalog; insta snapshot green; live `landing --json` exit 0 |
| `crates/archel/src/cmds/sys_cmds.rs` | cpu/ram/disk/gpu/temp/power renderers | ✓ VERIFIED | `Monitor` created OUTSIDE each watch loop and reused; NDJSON framing (10/10 ticks parse); N/A via `unavailable_label()`; zero subprocess calls |
| `crates/archel/src/cmds/snapshots/` (8× phase-2 .snap) | Pinned --json schemas | ✓ VERIFIED | `schema_sys_all_is_pinned` and siblings green in this run |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `mod.rs` CLI dispatch | `landing_cmd` / `sys_cmds` | `Command::Landing`/`Command::Sys` match arms | WIRED | `mod.rs` L614-617 route all metrics |
| `landing_cmd` | `landing::snapshot` | bin-calls-core, renders only | WIRED | Labels from `landing::table_labels()`; live JSON proves chain |
| `sys_cmds` | `system::Monitor` + `sensors` | bin-calls-core, renders only | WIRED | Monitor + `Sensors::new()` per command; zero `Command::new`/`duct::` in bin |
| `updates` cache | `landing` snapshot | TTL envelope + stale-with-age | WIRED | `pending_counts(force_refresh)`; expired+failure → `Stale{count,age_secs}`; post-fix spawn reaches `map_update_exit` Ok arm for exit 2 |
| `Monitor` | watch loop | one instance created outside loop, reused | WIRED | `run_cpu/ram/disk` create Monitor before `loop { }`; 10/10 NDJSON ticks parse live |
| NVML handle | watch ticks | probed once via `Sensors::new`, reused | WIRED | No subprocess in any tick path |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| landing table/JSON | version/kernel/repo/aur | `library_version` + sysinfo kernel + checkupdates/yay line counts via post-`bf52ba1` unchecked spawn | ✓ live (`repo: Fresh 196`, `aur: Fresh 4`, kernel 7.2.6-arch2-1) | ✓ FLOWING |
| sys cpu | per-core % + MHz | sysinfo two-pass refresh | ✓ live (not-all-zero test passes on real hardware) | ✓ FLOWING |
| sys ram/disk | used/total | sysinfo memory + root-mount disk match | ✓ live (JSON keys present, root-partition test green) | ✓ FLOWING |
| sys gpu/temp/power | VRAM/temps/watts | NVML dlopen + Components/hwmon discovery + RAPL | ✓ live (VRAM 1.7/8.0 Gio, CPU 45.6 °C + GPU 54.0 °C, GPU 34.8 W; CPU watts honest N/A on root-gated RAPL) | ✓ FLOWING |

### Behavioral Spot-Checks (fresh verifier live runs, this machine, 2026-10-11)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Core suite headless | `cargo test --offline -p archel-core --no-default-features` | 278 passed, 0 failed, 1 ignored | ✓ PASS |
| Bin suite headless (incl. 8 insta snapshots + thin-view audit) | `cargo test --offline -p archel --no-default-features` | 125 passed, 0 failed | ✓ PASS |
| bf52ba1 regression test in isolation | `cargo test --offline -p archel-core … exit_two_spawn_reaches_exit_mapping` | 1 passed, 0 failed | ✓ PASS |
| landing --json parses (post-fix binary, rebuilt at 00:12) | `archel landing --json \| python3 json.load` | keys `['app_version','aur','kernel_version','repo']`, exit 0 | ✓ PASS |
| sys all --json parses | `archel sys all --json \| python3 json.load` | keys `['cpu','disk','gpu','power','ram','temps']`, exit 0 | ✓ PASS |
| NDJSON watch stream | `timeout 5 archel sys ram --watch --json` | 10 ticks, 10/10 `json.loads` OK | ✓ PASS |
| GPU/temp/power live values | `archel sys gpu` / `sys temp` / `sys power` | VRAM 1.7/8.0 Gio; 45.6 °C + 54.0 °C; GPU 34.8 W + CPU N/A | ✓ PASS |

### Prohibitions (flagged-unverified, judgment tier — autonomous LLM-judge verdicts with mirror evidence)

| Prohibition | Judge verdict | Mirror evidence |
|-------------|---------------|-----------------|
| landing must not exit non-zero / hide snapshot on refresh failure | holds | live exit 0; `RefreshFailed`/`CheckerMissing` degrade paths unit-tested |
| checker stdout interpreted by line-counting only | holds | `.lines()` only (L150-152); no shell, no eval of package names |
| CPU snapshot must not report all-zero cores from one pass | holds | two-pass refresh; `live_cpu_snapshot_is_not_all_zero` green this run |
| disk must not report a non-root filesystem | holds | exact root-mount match; live root-partition test green |
| GPU telemetry must not shell out per tick | holds | zero `Command::new`/`duct::` in sys_cmds; NVML handle reused |
| sensor paths must not be hardcoded (`hwmon2`/`card0`/`:0`) | holds | production code enumerates by name; hits only in doc comments + tmp-dir test fixtures |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| LAND-01 | 02-01 | App version on home page | ✓ SATISFIED | live JSON `app_version: 0.1.0` |
| LAND-02 | 02-01 | Kernel version on home page | ✓ SATISFIED | live JSON `kernel_version: 7.2.6-arch2-1` |
| LAND-03 | 02-01 | Pending update count (repo + AUR) | ✓ SATISFIED | live `repo: Fresh 196` + `aur: Fresh 4`; `bf52ba1` additionally hardens the fully-updated (exit-2) case to `Fresh(0)` instead of degraded, pinned by regression test |
| SYS-01 | 02-02 | RAM used/total | ✓ SATISFIED | live test + JSON key green |
| SYS-02 | 02-02 | Disk usage per mount | ✓ SATISFIED (scoped) | System partition per approved D-08 — see observations |
| SYS-03 | 02-02 | Live per-core CPU | ✓ SATISFIED | not-all-zero + sane-numbers live tests green |
| SYS-04 | 02-02 | CPU clock speed | ✓ SATISFIED | `freq_mhz` per core, rendered + snapshot-pinned |
| SYS-05 | 02-03 | GPU VRAM used/total | ✓ SATISFIED | live 1.7/8.0 Gio via NVML |
| SYS-06 | 02-03 | CPU + GPU temperature | ✓ SATISFIED | live 45.6 °C / 54.0 °C hottest-pick |
| SYS-07 | 02-03 | Real-time CPU power (W) | ✓ SATISFIED (honest N/A) | fixture-proven delta math; live N/A is root-gated RAPL, D-10 honest-N/A design |
| SYS-08 | 02-03 | Real-time GPU power (W) | ✓ SATISFIED | live 34.8 W via NVML |

No orphaned requirements: all 11 Phase 2 IDs (LAND-01..03, SYS-01..08) are claimed across the three plans. 02-UAT.md: status complete, all tests pass.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | None | — | Debt-marker sweep (`TBD|FIXME|XXX`) across all 11 changed implementation files + fr.json + Cargo.toml: exit 1 (zero matches). No per-tick subprocess, no hardcoded sensor paths, no stub returns in tick/render paths. `bf52ba1` adds only an intentional fix + a real regression test — no markers introduced. |

### Observations (non-blocking, informational)

- **`bf52ba1` scope confirmed:** `git show --stat` = 1 file, +30 lines, `updates.rs` only. Commit-message claims match the diff exactly (`.unchecked()` on `run_checker` spawn mirroring `packages.rs::run_capped_query`, Ok-arm mapping, regression test). The fix strictly improves LAND-03: a fully-updated machine now shows `Fresh(0)` instead of degrading to Unavailable/stale.
- **Covered-file churn audit:** besides `bf52ba1`, only `45dad26` (thin-view audit test, landing_cmd.rs/sys_cmds.rs) and `53dd31e` (fr.json bluetooth keys) touched covered files on 2026-10-10, both at ~12:32–12:40 local — before the prior verifier's own runs, whose reported tallies (125 bin tests incl. thin-view audit) already reflect them. Not regressions; new digest pins current content regardless.
- **Binary staleness caught and corrected during this pass:** the pre-existing debug binary predated the `updates.rs` edit (mtime check); it was rebuilt before live CLI checks so all landing runs exercised post-`bf52ba1` code.
- **SC2 wording vs D-08:** roadmap says "disk usage per mount"; approved D-08 scopes `sys disk` to the system partition (exact root-mount match) so a wrong filesystem is never reported. Deliberate, documented, consistent across all three plans; Phase 6 may revisit multi-mount. Not a Phase 2 gap.
- **CPU watts N/A is permission-gated, not missing hardware:** RAPL `energy_uj` is root-only on this machine; the delta math is fixture-proven and watts appear wherever the kernel permits reads. The truth as written ("or honest N/A where no sensor exists") is satisfied by the honest-N/A design (D-10).
- **Watch flicker feel at 2 Hz:** purely perceptual polish (all prior reports dispositioned it non-blocking); automation proves 10/10 parseable NDJSON ticks. Fold into Phase 6 first-use eyeballing.

### Gaps Summary

None. All four roadmap success criteria re-proven against the current codebase with fresh headless offline runs (278 + 125 tests green, named regression test green, 7/7 live CLI spot-checks on a freshly rebuilt binary), all 11 requirement IDs satisfied (two with explicitly approved scoping/honesty caveats), all 8 JSON schemas insta-pinned and green, no debt markers, no regressions. The post-report commit `bf52ba1` verified as an intentional, tested improvement to the LAND-03 update-count path. Fresh digest `v3:sha256:3d772d74…` (tool-computed via `verification fingerprint`) replaces stale `v2:sha256:c2b4a57b…`.

---
_Verified: 2026-10-11T00:00:00Z_
_Verifier: the agent (gsd-verifier)_
