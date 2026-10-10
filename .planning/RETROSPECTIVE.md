# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v1.0 — Archel Settings Suite

**Shipped:** 2026-10-11
**Phases:** 6 | **Plans:** 28 | **Sessions:** ~10 (interactive + autonomous close-out)

### What Was Built
- `archel-core` settings API: 17 domain modules over zbus (NM/PPD/UPower), bluer, hyprland-rs, alpm, pipewire/wpctl, hwmon/RAPL/NVML sensors — 278 core tests green offline
- `archel` CLI: full API surface in French tables + pinned `--json` (landing, sys, pkg, net, bt, audio, power, appearance, display, input) with shell completions
- pkexec allowlisted helper (`archel-privileged`, v1 JSON protocol, D-12 exit mapping) — Archel never runs as root
- `archel settings` GUI: CXX-Qt 0.10/QML vertical-tab shell, 10 live tabs, confirm-token mutations, 1 Hz/500 ms tickers, `just gui-check` gate
- 43/43 requirements shipped and verified; line-preserving hyprland.conf snippet writes with watch-driven reload

### What Worked
- API-first ordering paid off: GUI plans were thin consumers with zero backend rework; CLI doubled as the live verification harness for every verifier pass
- Wave-parallel plan execution (parallelization on) kept 28 plans moving in 11 days
- Goal-backward verifier caught real defects pre-close: G-04-2 (VPN read-back), GAP-1 (dead AUR Valider reference), thin-view audit
- Confirm-token mutation pattern gave every GUI write a uniform preview→confirm→apply→read-back shape
- Insta snapshots pinned `--json` schemas early — the most regression-prone surface never drifted

### What Was Inefficient
- Verification staleness churn: shared core files (error.rs, lib.rs, updates.rs) repeatedly invalidated earlier phases' digests, forcing 3 re-verification rounds at close (phases 01–04 + a phase-02 repeat after `bf52ba1`)
- Phase-06 worker death mid-plan required inline rescue of task 3
- Verifier wrote a `v3`-frame digest that mismatched the status scanner's `v2` frame — a hand-offs contract gap between tool invocations
- camelCase/snake_case QML binding mismatch recurred as 5 separate code-review findings instead of being decided once up front
- 13 live-session UAT items stayed unconfirmed (headless environment) — deferred with the acknowledged-deferral path

### Patterns Established
- Canonicalization chain for close: UAT rows final → `phase uat-passed --uat-only` → sanctioned `frontmatter.set --field status` → `phase complete`
- `audit-open acknowledge` CLI writer as the ONLY suppression path; STATE.md `## Deferred Items` rows as disclosure (sanitized via `sanitizeForDisplay`)
- Digests always copied verbatim from `verification fingerprint` output; recompute in the status scanner's frame before trusting a report
- Deferred-item ledgers in CLI-writable bullet form (`## Deferred Items` + `  status:` continuation lines), never GFM tables

### Key Lessons
1. Commit-touching a shared core file late silently staleness-bombs every phase that covers it — batch core-wide fixes before re-verification waves, or re-verify last-mile phases immediately after.
2. Digest-producing tool invocations must run in the consumer's canonicalization frame; verify with `verification status` right after the writer returns, not at close time.
3. Decide the QML↔Rust property-naming convention in the spike gate and codify it in qmllint policy; convention ambiguity becomes repeated findings, not one.
4. Headless environments should front-load "live item" identification at plan time so UAT deferral is a planned disposition, not a close-time scramble.
5. `milestone complete --confirm`'s preservation warnings (e.g. `current_phase preserved-over-disagreeing-derived`) are worth reading before accepting the rewritten STATE.md.

### Cost Observations
- Model mix: `model_profile: adaptive` (runtime-resolved mix; exact ratios not tracked by the harness)
- Notable: close-out consumed ~2 sessions of gates (audit, re-verification, acknowledgments) — ~15% of total milestone effort; front-loading audit hygiene during execution would cut this materially

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Sessions | Phases | Key Change |
|-----------|----------|--------|------------|
| v1.0 | ~10 | 6 | First milestone: API-first ordering, wave-parallel execution, verifier + code-review gates |

### Cumulative Quality

| Milestone | Tests | Coverage | Zero-Dep Additions |
|-----------|-------|----------|-------------------|
| v1.0 | 403 (278 core + 125 bin) | All 43 reqs have code + SUMMARY + UAT/VERIFICATION evidence | insta-pinned JSON schemas; hwmon sysfs module hand-rolled |

### Top Lessons (Verified Across Milestones)

1. API-first ordering (CLI proves everything before GUI draws anything) eliminated GUI-backend rework — reconfirm in v1.1.
2. Late shared-file edits invalidate earlier verification — schedule core-wide fixes before verification waves.
