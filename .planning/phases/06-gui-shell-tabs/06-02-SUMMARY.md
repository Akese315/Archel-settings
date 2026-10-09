---
phase: 06-gui-shell-tabs
plan: 02
subsystem: core
tags: [privileged, pkexec, yay, cancel, sigint, streaming, atomicbool, archel-core]

# Dependency graph
requires: []
provides:
  - run_privileged_streaming_cancellable (verb, packages, label, on_line, cancel) returning CancellableStreaming (transcript, cancelled, exit_code)
  - yay_build_cancellable (name, on_line, cancel) with the identical stop contract
  - Shared single cancel implementation (CancelSentry + collect_streaming_cancellable + drain_cancellable_child) used by both paths
affects: [06-01-shell-window, 06-03-polling-system-landing, 06-06-modal-infra-pkg-write, 06-07-write-system-domains]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
actuals:
  tokens: 4500
  tasks: 2
  commits: 2

# Tech tracking
tech-stack:
  added: []
  patterns: [cancel-sentry-one-shot-sigint, drain-to-eof-then-reap, stop-as-data-not-error]

key-files:
  created: []
  modified:
    - crates/archel-core/src/privileged.rs
    - crates/archel-core/src/aur.rs

key-decisions:
  - "Stop is data, not an error: a user-cancelled transaction returns Ok(CancellableStreaming { cancelled: true }) with the partial transcript and reaped exit code; only genuine failures (non-cancelled path) keep the exact map_pkexec_exit + envelope + yay non-zero errors"
  - "One shared cancel implementation (pub(crate) drain_cancellable_child + CancelSentry + collect_streaming_cancellable in privileged.rs) — the plan's 'one private fn' widened to pub(crate) so aur.rs reuses it without a new module file; no duplicated cancel logic"
  - "Signal via the kill utility subprocess (which-resolved, /bin/kill fallback), no libc/nix dependency; SIGINT exactly once via a claim-once latch shared by the drain loop and a 50 ms watcher thread, so the stop also lands while a silent child blocks the read"
  - "Spawn via std::process::Command (not duct reader) so the child pid is known to the sentry; stderr drained on a side thread into the outcome for logs-only diagnostics"

patterns-established:
  - "Cancel-sentry one-shot SIGINT: AtomicBool latch claim-once across the drain loop and a 50 ms watcher thread; kill spawn is fire-and-forget, the reap reports the true end state"
  - "Drain-to-EOF-then-reap: cancel never truncates the transcript; wait() always runs, so no zombie and no dirty pacman lock"

requirements-completed: [PLAT-03]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "Privileged transactions are cancellable: run_privileged_streaming_cancellable sends exactly one SIGINT on a set handle, drains stdout to EOF, and reports transcript + cancelled flag + exit code (PLAT-03)"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/privileged.rs#cancellable_drain_without_cancel_returns_full_transcript"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/privileged.rs#cancel_mid_stream_sigints_once_drains_to_eof_and_reports"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/privileged.rs#cancel_after_natural_end_sends_no_signal"
        status: pass
      - kind: unit
        ref: "cargo test -p archel-core --no-default-features live_sleep_cancel_reaps_the_child -- --ignored --exact (manual A4 run during execution)"
        status: pass
    human_judgment: false
  - id: D2
    description: "AUR builds are cancellable identically: yay_build_cancellable shares the one cancel implementation; yay_build signature and CLI snapshots unchanged (PLAT-03)"
    requirement: "PLAT-03"
    verification:
      - kind: unit
        ref: "cargo test -p archel-core --no-default-features aur (15 passed)"
        status: pass
      - kind: unit
        ref: "cargo test -p archel --no-default-features (120 passed, snapshots green)"
        status: pass
    human_judgment: false

# Metrics
duration: 35min
completed: 2026-10-09
status: complete
---

# Phase 06 Plan 02 Summary

**Privileged streaming and AUR builds are cancellable through one shared SIGINT-drain-report implementation, with the stop reported as data — core gap #1 closed.**

## Performance

- **Duration:** 35min
- **Started:** 2026-10-09T10:56Z
- **Completed:** 2026-10-09T11:30Z
- **Tasks:** 2 completed
- **Files modified:** 2

## Accomplishments

- `run_privileged_streaming_cancellable` in `privileged.rs`: same spawn shape as `run_privileged_streaming` (which-pkexec probe, helper argv, `LC_ALL=C`), plus an `&AtomicBool` stop channel; cancel path returns `Ok` with `cancelled == true`, non-cancelled path keeps exact `map_pkexec_exit` + envelope semantics; old function byte-identical
- `yay_build_cancellable` in `aur.rs`: identical stop contract over the yay path (same which-detection and `YayMissing` state, `yay_build` untouched for the CLI)
- Single shared implementation (`CancelSentry` + `collect_streaming_cancellable` + `drain_cancellable_child`): one-shot SIGINT via the `kill` utility, drain-to-EOF, stderr side-drain, reap with exit code; fake-child tests plus the `#[ignore]`d live A4 test (run manually during execution: silent `sleep 30` stopped at 1 s and reaped)

## Task Commits

Each task was committed atomically:

1. **Task 1: Cancellable privileged streaming (SIGINT, drain, report)** - `5eb1eb0` (feat)
2. **Task 2: Cancellable yay build** - `b139b64` (feat)

## Files Created/Modified

- `crates/archel-core/src/privileged.rs` - `CancellableStreaming` outcome struct, `CancelSentry`/`collect_streaming_cancellable`/`drain_cancellable_child` shared machinery, `run_privileged_streaming_cancellable`, fake-child cancel tests (3) + ignored live test
- `crates/archel-core/src/aur.rs` - `yay_build_cancellable` reusing the shared drain; `yay_build` unchanged

## Decisions Made

- Stop-as-data: user cancel returns `Ok` with `cancelled: true` (partial transcript kept), never an error variant — the GUI modal renders the true end state from `cancelled` + `exit_code`
- `pub(crate)` (not private) for the shared drain so `aur.rs` reuses it with no new module file and no new public surface beyond the two variants + outcome struct
- Watcher thread (50 ms) alongside the per-line poll: the first live-test run proved a silent child blocks the read loop forever, so the stop must not depend on output flow
- `std::process::Command` (not duct `reader()`) for the cancellable spawns: the pid must be known to the sentry; duct's reader handle exposes no pid

## Deviations from Plan

### Auto-fixed Issues

**1. Watcher thread for silent children (correctness)**

- **Found during:** Task 1 (live A4 test run — `sleep 30` ran the full 30 s, stop never landed)
- **Issue:** The plan's per-line poll cannot fire while a silent child blocks `lines.next()`; a cancel during a quiet stretch (or a quiet child) would hang until output or natural end
- **Fix:** `CancelSentry` gained interior mutability (claim-once `AtomicBool` latch) and `drain_cancellable_child` spawns a 50 ms watcher thread under `thread::scope`; the drain-loop poll stays as the fast path for chatty output
- **Files modified:** `crates/archel-core/src/privileged.rs`
- **Verification:** live A4 test passes (stop at 1 s, reaped); full core suite green
- **Committed in:** `5eb1eb0` (part of task commit)

**2. Hard-kill token in prose (verify-gate wording)**

- **Found during:** Task 1 verification (`! grep -nE 'SIGKILL|...'` gate)
- **Issue:** A doc comment contained the literal token `SIGKILL` (in "never SIGKILL"), tripping the plan's own no-hard-kill grep gate
- **Fix:** Reworded to "never an uncatchable kill signal" (the plan's own prohibition phrasing); gate clean
- **Files modified:** `crates/archel-core/src/privileged.rs`
- **Verification:** grep gate passes; privileged tests green
- **Committed in:** `b139b64` (folded into Task 2 commit — 1-line doc reword)

**3. French diacritic in doc comment (i18n test)**

- **Found during:** Task 1 verification (full core suite)
- **Issue:** Doc comment contained "terminée" — `i18n::tests::no_hardcoded_french_diacritics_in_rust_sources` failed
- **Fix:** Reworded to diacritic-free English ("how the transaction ended")
- **Files modified:** `crates/archel-core/src/privileged.rs`
- **Verification:** full core suite 269 passed
- **Committed in:** `5eb1eb0` (amended into Task 1 commit before Task 2)

---

**Total deviations:** 3 auto-fixed (1 correctness, 2 verify-gate wording)
**Impact on plan:** All fixes required for correctness or to satisfy the plan's own gates. No scope creep; no new dependencies; public surface exactly the two variants + outcome struct.

## Issues Encountered

- `cargo clippy -p archel-core --all-features -- -D warnings` fails with 29 pre-existing lints in `updates.rs`, `sensors.rs`, `network.rs`, `hyprland/*`, `bluetooth.rs` — zero in `privileged.rs`/`aur.rs` (verified by location list). Left untouched as out of scope; the plan's clippy gate is red on the base tree independent of this plan.
- Wave-1 worker dispatch for this plan was abandoned before execution (orchestrator-worktree spawn bound tool-cwd to the main checkout; then the local model backend went down). Executed inline on the main checkout instead, per user decision. The stale worker records (`agent-p02-*`, killed pre-work, no artifacts) need `worker-complete` reconciliation after this SUMMARY lands.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 06-06 (modal infra) and 06-07 (system-domain writes) can consume `run_privileged_streaming_cancellable` / `yay_build_cancellable` with an `AtomicBool` wired to the StreamModal Annuler button; `CancellableStreaming { transcript, cancelled, exit_code }` is the full modal end-state contract
- 06-01 (shell window) still awaits execution — blocked on model-backend recovery + the worktree tool-cwd binding investigation (see STATE.md session continuity)
- Pre-existing clippy `--all-features` debt (29 lints) predates this phase and will trip the post-merge gate for later waves unless addressed or scoped

---
*Phase: 06-gui-shell-tabs*
*Completed: 2026-10-09*
