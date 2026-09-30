---
phase: 02-system-insight
plan: 01
subsystem: cli-telemetry
tags: [landing, sysinfo, update-count, ttl-cache, clap, french-i18n]

# Dependency graph
requires:
  - phase: 01-foundations-safety-rails
    provides: [bin-calls-core pattern, catalog French via fr.json, privileged.rs spawn/parse shape, diacritics guard, typed ArchelError]
provides:
  - "archel landing (table + --json + --refresh + --watch) backed by archel-core::{updates, landing}"
  - "15-minute update-count cache with stale-with-age degradation under ~/.cache/archel/updates.json"
  - "UpdatesCheckerMissing typed variant + guided pacman-contrib hint"
affects: [02-02-sys-metrics, 02-03-cli-proof, gui-landing-tab]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
# Same estimateTokens scale (chars/4 over the realized diff), never a harness token count.
actuals:
  tokens: 12626
  tasks: 3
  commits: 2

# Tech tracking
tech-stack:
  added: [directories 6.0, sysinfo 0.39]
  patterns: [ttl-json-cache-with-stale-age, per-half-update-degradation, injectable-recompute-for-fixture-tests]

key-files:
  created: [crates/archel-core/src/updates.rs, crates/archel-core/src/landing.rs, crates/archel/src/cmds/landing_cmd.rs]
  modified: [crates/archel-core/src/error.rs, crates/archel-core/src/lib.rs, crates/archel-core/locales/fr.json, crates/archel-core/Cargo.toml, crates/archel/Cargo.toml, crates/archel/src/cmds/mod.rs]

key-decisions:
  - "D-11 recorded APPROVED (pre-approved gate): archel sys cpu/ram/disk/gpu/temp/power/all + top-level archel landing, French tables, global --json — only landing implemented here, sys group belongs to plans 02-03"
  - "checkupdates exit 2 treated as clean zero (up-to-date, empty output) per A1; all other non-zero exits go stale"
  - "Partial refresh keeps the previous computed_at so stale numbers never look rejuvenated (D-06)"
  - "All-empty recompute results are never persisted, so transient offline failures retry instead of freezing"

patterns-established:
  - " executors note: none new beyond plan patterns (which+duct spawn, catalog French, typed errors)"

requirements-completed: [LAND-01, LAND-02, LAND-03]

# Coverage metadata (#1602) — one entry per shipped deliverable. Drives DETERMINISTIC UAT routing in verify-work.
coverage:
  - id: D1
    description: "archel landing prints one French table with app version, kernel, pending updates"
    requirement: "LAND-01"
    verification:
      - kind: unit
        ref: "crates/archel/src/cmds/landing_cmd.rs#table_shows_version_kernel_and_counts"
        status: pass
      - kind: other
        ref: "cargo run -q -p archel --no-default-features -- landing (exit 0, live, no checkupdates)"
        status: pass
    human_judgment: false
  - id: D2
    description: "archel landing --json prints one parseable JSON object (global flag, both orders)"
    requirement: "LAND-02"
    verification:
      - kind: other
        ref: "cargo run -q -p archel --no-default-features -- landing --json | python3 -c json.load (pass)"
        status: pass
    human_judgment: false
  - id: D3
    description: "archel landing --refresh forces recompute bypassing the cache"
    requirement: "LAND-03"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/updates.rs#refresh_bypasses_a_fresh_cache"
        status: pass
      - kind: other
        ref: "cargo run -q -p archel --no-default-features -- landing --refresh (exit 0, live)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Offline / missing-checker landing degrades to stale counts with age, never blank-or-crash"
    requirement: "LAND-03"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/updates.rs#expired_cache_falls_back_to_stale_with_age"
        status: pass
      - kind: other
        ref: "live expired-fixture run: '7 (4 depots + 3 AUR) - en cache, il y a 30 min', exit 0"
        status: pass
    human_judgment: false
  - id: D5
    description: "Corrupt or version-skewed cache is discarded and recomputed, never crashed on"
    requirement: "LAND-03"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/updates.rs#corrupt_cache_recomputes_without_crashing"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/updates.rs#version_skew_is_discarded_never_crashed_on"
        status: pass
    human_judgment: false
  - id: D6
    description: "D-11 CLI shape confirmed (sys group inventory + top-level landing + global --json)"
    verification: []
    human_judgment: true
    rationale: "Pre-approved by the user in chat on 2026-10-01; no automated test can prove a naming decision"

# Metrics
duration: 38min
completed: 2026-09-30
status: complete
---

# Phase 02 Plan 01: Landing Snapshot Summary

**`archel landing` shows version, kernel and pending updates in one French table (plus `--json`/`--refresh`/`--watch`), backed by a 15-minute XDG cache with stale-with-age degradation**

## Performance

- **Duration:** 38 min (approx.)
- **Started:** 2026-09-30T21:20:00Z
- **Completed:** 2026-09-30T21:58:09Z
- **Tasks:** 3 (1 decision gate pre-approved + 1 tracer + 1 auto)
- **Files modified:** 10

## Accomplishments

- `archel landing` end-to-end: version (LAND-01) + kernel via sysinfo (LAND-02) + repo/AUR counts (LAND-03) in one aligned French table, exit 0 on the live machine without `checkupdates`
- Global `--json` flag (both `landing --json` and `--json landing`) emitting one parseable object; `--refresh` bypass; `--watch` 500 ms re-render (NDJSON under `--json`)
- `updates.rs`: `which`-gated `checkupdates`/`yay -Qu --aur` spawn (privileged.rs shape), 15 s timeout, line-count-only stdout, `{v:1, computed_at, repo, aur}` cache at `~/.cache/archel/updates.json` with atomic tmp+rename writes
- Cache semantics: fresh serve, expired+failure → stale-with-age, partial refresh keeps old timestamp (no rejuvenation), all-empty results never persisted, corrupt/skewed envelopes recomputed
- Missing `checkupdates` → typed `UpdatesCheckerMissing` internally, rendered as AUR-only counts plus the French `sudo pacman -S pacman-contrib` hint (02-USER-SETUP.md tracks the human step)

## Task Commits

Each task was committed atomically:

1. **Task 1: Confirm one-way CLI shape (D-11 gate)** - pre-approved by user, no commit (recorded decision only)
2. **Task 2: End-to-end archel landing tracer** - `823a588` (feat)
3. **Task 3: TTL cache semantics** - `88aee41` (feat)

## Files Created/Modified

- `crates/archel-core/src/updates.rs` - checker spawn, counting, TTL cache, stale fallback, age/hint catalog renderers (created)
- `crates/archel-core/src/landing.rs` - snapshot composer + updates_cell/checker_notes/table_labels (created)
- `crates/archel/src/cmds/landing_cmd.rs` - table layout, JSON framing, watch loop (created)
- `crates/archel-core/src/error.rs` - `UpdatesCheckerMissing` variant + catalog arm + every_variant entry (modified)
- `crates/archel-core/src/lib.rs` - `pub mod landing/updates` (modified)
- `crates/archel-core/locales/fr.json` - `landing.*`, `updates.*`, `errors.updates_checker_missing` keys (modified)
- `crates/archel-core/Cargo.toml` - `directories 6.0`, `sysinfo 0.39` deps (modified)
- `crates/archel/Cargo.toml` - `serde_json` for `--json` framing (modified)
- `crates/archel/src/cmds/mod.rs` - global `--json`, `Landing{refresh, watch}` (modified)
- `Cargo.lock` - new dependency pins (modified)

## Decisions Made

- D-11 recorded as approved per pre-approval: `archel sys cpu/ram/disk/gpu/temp/power/all` + top-level `archel landing`, French tables, global `--json`. Only landing is implemented in this plan; the `sys` group belongs to plans 02-03.
- `checkupdates` exit 2 = clean zero (up-to-date, empty stdout → zero lines); every other non-zero exit or missing code = refresh failure → stale path (A1 assumption documented in `map_update_exit`).
- Line-counting means non-blank lines; package names/versions never cross into logic (literal reading of the threat-model prohibition).
- Stale halves wrap the whole counts cell with age (`… — en cache, il y a 30 min`); a partial refresh persists merged values under the previous timestamp.
- Cache resolution takes an injectable recompute fn (`pending_counts_with`) so all TTL/stale/bypass/corrupt semantics are fixture-clock unit tests with zero live-checker dependence.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] duct 1.x removed `Expression::timeout`**
- **Found during:** Task 2 (tracer build)
- **Issue:** Plan assumed `duct` timeout builder; duct 1.1.2 only offers `start()` + `Handle::wait_timeout()`
- **Fix:** `start()` the expression, `wait_timeout(CHECK_TIMEOUT)`, `kill()` on `None`, same log-only stderr handling
- **Files modified:** crates/archel-core/src/updates.rs
- **Verification:** `cargo test -p archel-core` green; live `yay` run under timeout works
- **Committed in:** 823a588 (part of task commit)

**2. [Rule 1 - Bug] Binary used `rust_i18n::t!` with no catalog backend**
- **Found during:** Task 2 (tracer build)
- **Issue:** `landing_cmd.rs` called `rust_i18n::t!` directly, but the backend is declared in `archel-core`; also violates the bin-owns-no-French invariant
- **Fix:** Added `landing::table_labels()` core helper; bin only lays out the returned strings
- **Files modified:** crates/archel-core/src/landing.rs, crates/archel/src/cmds/landing_cmd.rs
- **Verification:** bin compiles headless; diacritics guard passes
- **Committed in:** 823a588 (part of task commit)

**3. [Rule 1 - Bug] Alignment test compared byte indices**
- **Found during:** Task 2 (bin tests)
- **Issue:** `table_rows_share_one_label_column` used `str::find` (byte index); the accented label is one char but two bytes, so a correctly aligned table failed
- **Fix:** Compare `chars().position(|c| c == ':')` (character index)
- **Files modified:** crates/archel/src/cmds/landing_cmd.rs
- **Verification:** `cargo test -p archel --no-default-features` 3/3 green
- **Committed in:** 823a588 (part of task commit)

**4. [Rule 3 - Blocking] `serde_json` missing from the `archel` bin**
- **Found during:** Task 2 (tracer build)
- **Issue:** `--json` framing needs `serde_json` in the bin crate, which only had `archel-core` + `clap`
- **Fix:** Added `serde_json = "1.0"` to `crates/archel/Cargo.toml` (already vendored, RESEARCH-approved for CLI `--json`)
- **Files modified:** crates/archel/Cargo.toml, Cargo.lock
- **Verification:** `--json` output parses via `python3 -c json.load`
- **Committed in:** 823a588 (part of task commit)

**5. [Rule 3 - Blocking] `cargo` not on non-interactive PATH**
- **Found during:** Task 2 (first build)
- **Issue:** User-local rustup (`~/.cargo/bin`) is not on the agent shell PATH
- **Fix:** Exported `PATH="$HOME/.cargo/bin:$PATH"` for all cargo invocations (toolchain 1.98.1 confirmed)
- **Files modified:** none
- **Verification:** `cargo --version` → 1.98.1
- **Committed in:** n/a (environment only)

---

**Total deviations:** 5 auto-fixed (3 bugs, 2 blocking)
**Impact on plan:** All fixes required for compilation, correctness, or plan invariants. No scope creep; `sys` group and sensors stay in plans 02-03.

## Issues Encountered

- `checkupdates` absent on this machine (pacman-contrib not installed) — covered by design: the MissingChecker path was live-proven (AUR-only table + hint, exit 0). Human install tracked in 02-USER-SETUP.md.
- `sysinfo`/`directories`/`serde_json` needed registry downloads at build time — network was available, `Cargo.lock` pins them.

## User Setup Required

**External step requires manual action.** See [02-USER-SETUP.md](./02-USER-SETUP.md) for:
- Installing `pacman-contrib` via `sudo pacman -S pacman-contrib` (human password step)
- Verification command (`landing --refresh` showing the full repo+AUR breakdown)

## Next Phase Readiness

- Ready for 02-02/02-03: `landing::snapshot` + `updates::pending_counts` are GUI-reusable core APIs; the `sys` clap group + `system.rs`/`sensors.rs` samplers plug into the established bin-calls-core shape.
- Watch out: JSON schema field names (`Fresh`/`Stale`/`Unavailable` external tagging) are v1-unstable — pin before the GUI consumes them.
- Blocker: none. Concern: `checkupdates` exit-2-means-clean is an A1 assumption that cannot be live-verified until the user installs pacman-contrib — re-probe then.

---
*Phase: 02-system-insight*
*Completed: 2026-09-30*

## Self-Check: PASSED

All created files found on disk; both task commits (`823a588`, `88aee41`) present in history.
