# Phase 2: System Insight - Context

**Gathered:** 2026-09-30
**Status:** Ready for planning

## Phase Boundary

Phase 2 delivers the read-only system telemetry layer: an `archel-core` API exposing landing facts (app version, kernel, pending update count) plus live metrics (CPU per-core + clock, RAM, system-partition disk, GPU VRAM, temperatures, CPU/GPU power with honest N/A), proven headlessly through English CLI subcommands with French output and `--json`. No GUI work in this phase (GUI tabs land in a later phase); the CLI proof gate from the project core value applies.

## Implementation Decisions

### Refresh behavior (telemetry)
- **D-01:** Every telemetry command prints a one-shot snapshot by default and exits; `--watch` keeps it open and re-renders until Ctrl+C. — **Reversibility:** costly — scripts and later GUI polling semantics will depend on snapshot-by-default.
- **D-02:** `--watch` re-renders every 500 ms.
- **D-03:** `--watch` combined with `--json` prints one complete JSON object per tick (NDJSON-style stream), not a single final object.
- **D-04:** Snapshot CPU percentages are measured over a ~1 s sampling window (slower first response, reliable numbers).

### Update-count freshness (LAND-03)
- **D-05:** Pending-update count (pacman + AUR) is cached for 15 minutes; recomputation runs `checkupdates` + AUR query only on cache miss/expiry.
- **D-06:** When offline or the refresh fails, show the last cached value together with its age — never a bare number that looks fresh, never a hard error hiding everything else.
- **D-07:** `--refresh` flag forces an immediate recomputation, bypassing the cache.

### Sensor scope (disk + temperatures)
- **D-08:** Disk reporting covers the system partition only (the one the OS runs on), shown as used / total / free with used GB prominent — not a per-mount list. (Narrows SYS-02 deliberately per user request.)
- **D-09:** Temperatures show one representative value per category (CPU, GPU, …): the hottest sensor, not an average and not the full list.
- **D-10:** Missing sensors render honest French N/A (carries forward the roadmap criterion; researcher picks per-vendor sources).

### CLI shape
- **D-11:** Telemetry lives under an `archel sys` group with English subcommands per metric (`archel sys cpu`, `sys ram`, `sys disk`, `sys gpu`, …); the landing snapshot is a top-level `archel landing`. — **Reversibility:** one-way — renaming shipped subcommands breaks user muscle memory and scripts (same rationale as Phase 1 D-01).
- **D-12:** Human output renders as aligned French tables; `--json` is a single global flag producing machine output for every command.

### the agent's Discretion
- Exact per-metric subcommand list beyond cpu/ram/disk/gpu, cache file location under XDG paths, sampling implementation details, per-vendor sensor source selection (sysinfo/hwmon/NVML/RAPL), table column layout, JSON schema field names — researcher and planner decide following Phase 1 patterns (typed errors, catalog French, no panics).

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

No external specs — requirements fully captured in decisions above. Phase sources:
- `.planning/REQUIREMENTS.md` — LAND-01..03, SYS-01..08 (SYS-09 explicitly out of scope)
- `.planning/ROADMAP.md` — Phase 2 goal + 4 success criteria
- `.planning/phases/01-foundations-safety-rails/01-CONTEXT.md` — locked D-01..D-15 (single binary, FR output/EN commands, pkexec-only, i18n keys)
- `.planning/phases/01-foundations-safety-rails/01-01-SUMMARY.md` — established patterns (bin→core calls, `message()` French rendering, diacritics audit)
- `.planning/phases/01-foundations-safety-rails/01-02-SUMMARY.md` — privileged client pattern (relevant if any metric ever needs elevation; none should in Phase 2)

## Existing Code Insights

### Reusable Assets
- `crates/archel-core/src/error.rs` (`ArchelError`) — new metric failures map to typed variants, French via `message()`
- `crates/archel-core/src/i18n.rs` + `crates/archel-core/locales/fr.json` — every new user-visible string needs a catalog key (diacritics audit enforced)
- `crates/archel/src/cmds/mod.rs` — where `sys` group + `landing` subcommands plug in (clap derive, English names)
- `crates/archel-core/src/privileged.rs` — spawn/parse pattern if a metric needs a subprocess (e.g. `checkupdates`)

### Established Patterns
- Bin calls into `archel-core` and renders French; no logic or French literals in the bin
- `gui` cargo feature gate — Phase 2 adds no Qt code; everything builds headless
- Unit tests per module + `cargo test --workspace --no-default-features` as the quick gate

### Integration Points
- New `archel-core` modules (e.g. `system.rs`, `landing.rs`) exposed through `lib.rs`, consumed by `archel/src/cmds/`
- Update-count cache file under XDG cache dir (researcher picks exact path)

## Specific Ideas

- User's words: "les giga utilisés sur la partition qu'utilise ce system" — disk = system partition, GB used prominent
- User runs commands from a plain terminal; watch mode compared to `htop` as the mental model
- clap's raw English error (`error: unrecognized subcommand`) noticed during discussion — francization of CLI framework errors is a future-phase concern, not Phase 2

## Deferred Ideas

None — discussion stayed within phase scope (SYS-09 history/export already roadmap-deferred).

---

*Phase: 2-System Insight*
*Context gathered: 2026-09-30*
