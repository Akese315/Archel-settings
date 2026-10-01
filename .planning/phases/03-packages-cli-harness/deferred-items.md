# Phase 03 Deferred Items

Out-of-scope discoveries logged by plan executors (per scope-boundary rule).
The orchestrator routes these to future plans.

| Date | Found During | Item | Status |
|------|--------------|------|--------|
| 2026-10-01 | 03-01 task 3 | `updates.rs::run_checker` spawns without duct `.unchecked()`, so `checkupdates` exit 2 (up-to-date system) surfaces as `Internal` via `wait_timeout` instead of reaching `map_update_exit`'s explicit `("checkupdates", Some(2)) => Ok` arm. Proven with a minimal duct probe (`unchecked=false` errors on `exit 2`). Effect: on a fully updated machine with `checkupdates` installed, the repo half degrades to stale/unavailable instead of `Fresh(0)`. Fix is one word (`.unchecked()` on the spawn) but `updates.rs` is Phase 2-owned, so it is deferred, not fixed here. `packages.rs::run_capped_query` already carries `.unchecked()` plus a comment. | open |
