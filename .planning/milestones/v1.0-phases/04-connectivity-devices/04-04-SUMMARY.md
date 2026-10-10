---
phase: 04-connectivity-devices
plan: 04
subsystem: audio-power
tags: [wpctl, pipewire, wireplumber, power-profiles-daemon, cpufreq, zbus, confirm-gate, insta]

# Dependency graph
requires:
  - phase: 04-03
    provides: Scan-then-pick precedent, injectable confirm gates, NeedsTty-before-spawn discipline
provides:
  - Working `archel audio list|set-output|set-input` by stable node.name with read-back
  - Working `archel power status|set` with backend provenance (PPD or cpufreq)
  - Typed AudioUnavailable/AudioDeviceNotFound errors with French catalog arms
affects: [04-gui-phase, verify-work-manual-audio-switch, verify-work-manual-governor-switch]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
actuals:
  tokens: 20000
  tasks: 3
  commits: 3

# Tech tracking
tech-stack:
  added: []
  patterns: [name-to-current-ID resolve at switch time, Audio-tree-gated wpctl parse, PPD-probe-then-cpufreq two-tier with backend tag, read-back before ok]

key-files:
  created: [crates/archel-core/src/audio.rs, crates/archel-core/src/power.rs, crates/archel/src/cmds/audio_cmds.rs, crates/archel/src/cmds/power_cmds.rs]
  modified: [crates/archel-core/src/error.rs, crates/archel-core/src/lib.rs, crates/archel-core/locales/fr.json, crates/archel/src/cmds/mod.rs]

key-decisions:
  - "Session numeric IDs travel with #[serde(skip)] — usable once at switch time, never in --json, never matched across runs (T-4-07)"
  - "powersave governor reads as power-saver; balanced and power-saver both write powersave, with the collapse note always shown on cpufreq"
  - "Governor writes without privilege map to PolkitDenied (hint family), never AuthFailed; archel-privileged stays pacman-scoped in v1 (T-4-08)"
  - "Fish completions do not emit clap possible-values, so profile words stay out of the word-list test (verbs only)"

patterns-established:
  - "Audio-tree gate: wpctl status parser tracks the Audio/Video top level so repeated Video Sinks:/Sources: headers never parse as audio"
  - "Injectable confirm/set gate (set_output_picked_with, cpufreq_set_with): abort path unit-covered headless with zero spawns"
  - "Read-back before ok: set-output/set-input and power set re-read after the switch and fail typed when the new state does not stick"

requirements-completed: [SND-01, SND-02, PWR-01]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "audio list shows stable node.names for sinks and sources with default flags, table plus --json"
    requirement: "SND-01"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/audio.rs#status_parse_reads_sinks_sources_defaults_and_skips_garbage"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/audio_cmds.rs#schema_audio_list_is_pinned"
        status: pass
      - kind: integration
        ref: "live `archel audio list --json`: 3 sinks + 2 sources with stable alsa_ names and one default each, exit 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "audio set-output/set-input switch the default sink/source by exact stable name with confirm gate and read-back"
    requirement: "SND-02"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/audio.rs#name_resolver_tracks_stable_names_across_id_renumbering"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/audio.rs#set_abort_path_spawns_nothing_without_auto_accept"
        status: pass
      - kind: integration
        ref: "live `archel audio set-output <ghost> --json` prints the typed DeviceNotFound catalog line, exit 1, nothing spawned"
        status: pass
      - kind: integration
        ref: "live piped `archel audio set-output <real-name>` refuses NeedsTty with the catalog line, exit 1, governor/graph untouched"
        status: pass
    human_judgment: false
  - id: D3
    description: "power status/set report and switch performance/balanced/power-saver with backend provenance, table plus --json"
    requirement: "PWR-01"
    verification:
      - kind: unit
        ref: "crates/archel-core/src/power.rs#governor_map_yields_performance_powersave_powersave_with_backend_note"
        status: pass
      - kind: unit
        ref: "crates/archel-core/src/power.rs#governor_writes_without_privilege_yield_the_hint_error"
        status: pass
      - kind: unit
        ref: "crates/archel/src/cmds/power_cmds.rs#schema_power_status_is_pinned"
        status: pass
      - kind: integration
        ref: "live `archel power status --json` parses: profile power-saver, backend cpufreq, collapse note present"
        status: pass
      - kind: integration
        ref: "live piped `archel power set balanced` refuses NeedsTty, exit 1, scaling_governor still powersave"
        status: pass
    human_judgment: false
  - id: D4
    description: "Audible audio switch and live governor switch against the running session"
    requirement: "SND-02"
    verification: []
    human_judgment: true
    rationale: "Both switches mutate the live session (audible route change, system-wide power state) and need a TTY confirm plus human senses; headless runs prove the gates, not the effect"

# Metrics
duration: 60min
completed: 2026-10-07
status: complete
---

# Phase 04 Plan 04: Audio plus Power Summary

**Name-keyed audio switching (`wpctl` parse with Video-tree gate, read-back) plus two-tier power profiles (PPD probe, cpufreq fallback, backend tag on every output), all CLI-proven with pinned --json**

## Performance

- **Duration:** ~60 min
- **Started:** 2026-10-07T (continuation session)
- **Completed:** 2026-10-07
- **Tasks:** 3 (tracer + 2 auto/TDD)
- **Files modified:** 12

## Accomplishments

- `archel audio list` parses `LC_ALL=C wpctl status` (Audio-tree Sinks/Sources only — the repeated Video headers are gated out) plus per-ID `wpctl inspect` for stable `node.name`; French sink/source sections plus `--json` with session IDs excluded by `#[serde(skip)]` (T-4-07)
- `archel audio set-output/set-input <name>` resolve the exact name to the current session ID at switch time (renumber-proof, stale IDs and duplicates fail as typed `AudioDeviceNotFound`), confirm-gate, `wpctl set-default` as the user, then read back the graph before the ok line
- `archel power status|set` probe `net.hadess.PowerProfiles` first and use PPD when present; otherwise the cpufreq fallback (performance/powersave governors, balanced collapses to powersave with the catalog note always shown); every output carries the `power-profiles-daemon`/`cpufreq` backend tag
- Governor writes without privilege yield typed `PolkitDenied` hint (T-4-08); `archel-privileged` untouched; `sys power` (watts) and `power` (profiles) keep distinct catalog titles
- Full workspace suite green: 83 (cli) + 183 (core) + 18 (privileged); diacritics guard clean; completions cover net/bt/audio/power verbs

## Task Commits

Each task was committed atomically:

1. **Task 1: End-to-end audio list through wpctl parsing** - `eaf504f` (feat)
2. **Task 2: Audio set-output and set-input by stable name** - `935e019` (feat)
3. **Task 3: Power status and profile switching with backend provenance** - `1341c66` (feat)

_Note: TDD tasks in one atomic commit each (RED confirmed via todo!/missing-fn failures, then GREEN)._

## Files Created/Modified

- `crates/archel-core/src/audio.rs` - wpctl status/inspect parsers, name-keyed snapshot, resolve + confirm-gated set with read-back, catalog label fns, 11 tests
- `crates/archel-core/src/power.rs` - PPD probe/proxy, cpufreq read/write with backend tag, injectable set gate, catalog label fns, 9 tests
- `crates/archel-core/src/lib.rs` - `pub mod audio; pub mod power;`
- `crates/archel-core/src/error.rs` - `AudioUnavailable` + `AudioDeviceNotFound` variants, constructors, catalog arms, every_variant entries
- `crates/archel-core/locales/fr.json` - audio.* (titles, columns, prompts, ok lines), power.* (titles, labels, note, prompts), errors.audio_* arms
- `crates/archel/src/cmds/audio_cmds.rs` - run_list/run_set_output/run_set_input, section tables, mutation-report JSON, 6 tests + 2 insta pins
- `crates/archel/src/cmds/power_cmds.rs` - run_status/run_set, profile/backend/note table, mutation-report JSON, 5 tests + 2 insta pins
- `crates/archel/src/cmds/mod.rs` - Audio + Power groups, dispatch, completions word-list (audio, power, set-output, set-input)

## Decisions Made

- Session numeric IDs travel inside `AudioDevice` with `#[serde(skip)]`: resolvable once at switch time, invisible to `--json` consumers, never matched across restarts (T-4-07)
- `powersave` governor reads as `power-saver`; both `balanced` and `power-saver` write `powersave`, with the collapse note always rendered on the cpufreq backend so the asymmetry is never silent
- Unwritable governor files map to `PolkitDenied` (hint family), never `AuthFailed` (helper-scoped); PPD read failure with a present bus name falls back to cpufreq rather than failing
- Fish completions do not emit clap possible-values, so the profile words stay out of the word-list test — verbs only, per the plan gate

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed `?`-on-split_once bailing the whole inspect parse on the first `=`-less line**
- **Found during:** Task 1 (inspect unit test returned None on the header line)
- **Issue:** `parse_inspect_property` used `split_once('=')?`, so `id 47, type ...` (no `=`) exited the function instead of skipping the line
- **Fix:** `let Some(..) = .. else { continue };`
- **Files modified:** crates/archel-core/src/audio.rs
- **Verification:** `inspect_parse_reads_stable_node_names` green
- **Committed in:** eaf504f (part of task commit)

**2. [Rule 1 - Bug] Scoped wpctl status parsing to the Audio tree**
- **Found during:** Task 1 (Video `Sinks:` header re-entered the section and parsed a video loopback as audio)
- **Issue:** The Video tree repeats `Sinks:`/`Sources:` headers; a section-only parser attributes those rows to audio
- **Fix:** `in_audio` gate set by the `Audio`/`Video`/`Settings` top-level lines; entries only inside the Audio tree
- **Files modified:** crates/archel-core/src/audio.rs
- **Verification:** video-loopback ID assertion in the status unit test; live list shows exactly the 3+2 audio endpoints
- **Committed in:** eaf504f (part of task commit)

**3. [Rule 1 - Bug] Added box-drawing chars to the tree-prefix strip set**
- **Found during:** Task 1 (zero entries parsed: `├─`/`│`/`─` are U+2500-U+257F, not ASCII `|-`)
- **Issue:** `trim_start_matches` covered ASCII tree chars only, so no header or row was ever recognized
- **Fix:** Strip `\u{2500}..=\u{257F}` alongside the ASCII set in both the header detector and the row parser
- **Files modified:** crates/archel-core/src/audio.rs
- **Verification:** all 6 Task-1 audio unit tests green plus live `audio list`
- **Committed in:** eaf504f (part of task commit)

**4. [Rule 2 - Missing Critical] Covered the T-4-08 privilege path with a read-only governor test**
- **Found during:** Task 3 (the PolkitDenied mapping on EACCES had no live coverage — headless runs stop at NeedsTty first)
- **Issue:** Untested mapping risks a silent-escalation regression on the exact path the threat register constrains
- **Fix:** `governor_writes_without_privilege_yield_the_hint_error` uses a chmod-444 fixture governor (skipped when running as root) and asserts `PolkitDenied` plus a non-empty catalog rendering
- **Files modified:** crates/archel-core/src/power.rs
- **Verification:** new test green as non-root
- **Committed in:** 1341c66 (part of task commit)

---

**Total deviations:** 4 auto-fixed (3 correctness/bugs, 1 missing-critical coverage)
**Impact on plan:** All required for a correct, threat-register-faithful slice. No scope creep; no new dependencies; no architectural change.

## Issues Encountered

- `powerprofilesctl` is not installed and `net.hadess.PowerProfiles` has no bus owner on this machine, so the PPD tier is implemented and unit-covered but live-proven only via the cpufreq tier — PPD activation against a PPD-bearing session is queued as manual verify-work alongside the governor switch
- `wpctl status` localizes descriptions (Stéréo) but keeps English headers under `LC_ALL=C`; fixtures stay ASCII-only because the diacritics guard scans all `.rs` files (multibyte opacity is covered with CJK text instead)

## Threat Flags

None beyond the plan's register — T-4-07 mitigated (exact-name resolve at switch time, IDs `#[serde(skip)]`, duplicates fail with the candidate list), T-4-08 mitigated (confirm gate, `NeedsTty` before spawn, EACCES maps to `PolkitDenied` hint, helper untouched). No new cargo dependencies, so no package-legitimacy surface.

## Known Stubs

None. Live audible switch (D4-audio) and live governor switch (D4-power) are queued as manual verify-work, not stubs — the headless slices are complete and covered.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- SND-01/02 + PWR-01 headless slices complete; GUI phase can bind the audio device list (stable names), the set-output/set-input mutations, and the power status (profile + backend provenance) to the Sound/Power tabs
- Manual verify-work: audible `set-output`/`set-input` switch with TTY confirm, and `power set` governor switch (needs privilege — expect the `PolkitDenied` hint line as non-root)
- Phase 4 CLI proof gate satisfied: every Phase 4 write path (DNS, proxy, VPN, BT pair/connect, audio switch, power switch) is drivable via `archel` with `--json`

## Self-Check: PASSED

- `crates/archel-core/src/audio.rs` + `power.rs` present with resolvers, gates, and read-back — FOUND
- `crates/archel/src/cmds/audio_cmds.rs` + `power_cmds.rs` present with tables, mutation reports, insta pins — FOUND
- Commits `eaf504f` + `935e019` + `1341c66` in `git log` — FOUND
- Workspace suite: 83 + 183 + 18 passed, 0 failed — GREEN
- `archel audio list --json` (3 sinks + 2 sources) / `archel power status --json` (profile+backend+note) parse — VERIFIED live
- `sys power` vs `power` catalog titles distinct (`Puissance` vs `Alimentation — profil`) — VERIFIED in fr.json

---
*Phase: 04-connectivity-devices*
*Completed: 2026-10-07*
