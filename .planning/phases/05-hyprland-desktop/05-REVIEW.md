---
phase: 05-hyprland-desktop
reviewed: 2026-10-08T11:35:00Z
depth: standard
files_reviewed: 10
files_reviewed_list:
  - crates/archel-core/src/hyprland/mod.rs
  - crates/archel-core/src/hyprland/appearance.rs
  - crates/archel-core/src/hyprland/monitors.rs
  - crates/archel-core/src/hyprland/binds.rs
  - crates/archel-core/src/hyprland/wallpaper.rs
  - crates/archel-core/src/lib.rs
  - crates/archel-core/src/error.rs
  - crates/archel-core/locales/fr.json
  - crates/archel/src/cmds/appearance_cmds.rs
  - crates/archel/src/cmds/mod.rs
findings:
  critical: 0
  warning: 9
  info: 6
  total: 15
status: issues_found
---

# Phase 05: Code Review Report

**Reviewed:** 2026-10-08T11:35:00Z
**Depth:** standard
**Files Reviewed:** 10
**Status:** issues_found

## Summary

Reviewed the Phase 05 tracer slice (commits `552d708`, `88e99b5`, `28a74d1`):
CLI `appearance blur|opacity` → core `hyprland` module → `hyprctl eval`
live-apply → `getoption -j` read-back → app-snippet persistence. The security
posture against the plan threat model is genuinely good: no shell, no
interpolated user text in eval (bools/floats are machine-rendered), exact-argv
spawn with which-gate + `LC_ALL=C` + `wait_timeout`/kill (T-05-05), eval
`error:`-prefix refusal (Pitfall 1), and a D-08 write-scope spy test fencing
all writes to app-owned paths (T-05-02). No hardcoded secrets, no injection
path, no `keyword`/`reload` spawns found.

That said, the failure-path ordering is wrong (snippet is persisted *before*
the eval leg, so a refused eval leaves durable config claiming a value the
compositor rejected), the opacity planner does real filesystem I/O against the
live home directory *before* validating its inputs, the backup scheme collides
within a one-second window, and several tests either race on process-wide env
mutation or silently pass without asserting. None of these is a ship-stopper
on its own (every one has a backup, an honest error, or a bounded blast
radius), so there are no BLOCKERs — but the nine warnings below should be
fixed before breadth plans (05-02-01 and later) copy these shapes.

Note: the plan's clippy verify leg (`-D warnings`) is red on 22 pre-existing
lints in unrelated files (logged in 05-01-01-SUMMARY.md, verified identical on
the stashed baseline) — out of this review's file scope, mentioned here only
so the gate state is not misread as green.

## Warnings

### WR-01: Snippet persisted before eval — refused eval leaves durable/live divergence

**File:** `crates/archel-core/src/hyprland/appearance.rs:138-156` and `crates/archel-core/src/hyprland/appearance.rs:332-358`
**Issue:** Both `apply_blur_with` (lines 144-148) and `apply_opacity_with`
(lines 338-347) run `write_snippet_at` + `ensure_require_line_at` *before*
`runner(eval)`. If the eval leg fails (compositor refuses the statement) or
the read-back disagrees, the function returns `Err` — but the snippet already
claims the unapplied value. On the next Hyprland reload the refused value is
loaded from the snippet, re-applying something the compositor already
rejected. The error is honest, but durable state and live state have diverged
with no rollback. The existing test `apply_propagates_runner_failures_without_silent_ok`
proves the error propagates, yet never asserts the snippet was *not* left
claiming the failed value.
**Fix:** Either apply live first and persist only on read-back agreement, or
restore the previous snippet body when `runner`/`reader` fails:
```rust
let prior = std::fs::read_to_string(snippet_path_at(base)).ok();
// ... write, eval ...
if let Err(err) = runner(&blur_eval_statement(enabled)) {
    if let Some(body) = prior { let _ = write_snippet_at(base, &body); }
    return Err(err);
}
```

### WR-02: `plan_opacity_set` reads the live home dir before validating inputs

**File:** `crates/archel-core/src/hyprland/appearance.rs:260-277`
**Issue:** Line 264 calls `current_snippet_state(&hyprland_dir())` — real
filesystem I/O against the user's actual `~/.config/hypr/` — *before*
`validate_opacity` runs on lines 266-267. Consequences: (a) a typo'd float
(`--active abc`) performs I/O before failing, contradicting the
"rejected before anything spawns or writes" contract in spirit (a read is not
a write, but validation should still come first); (b) if the snippet is
unreadable (permissions, broken symlink), even a *valid* invocation fails with
`Internal` before validation; (c) the unit test
`opacity_plan_requires_at_least_one_leaf` touches the real home directory,
making it non-hermetic — `old_active`/`old_inactive` in tests depend on machine
state. The planner also has no injectable base, unlike the apply functions.
**Fix:** Validate first, read second — and thread the base through:
```rust
pub fn plan_opacity_set_at(
    base: &std::path::Path,
    active: Option<&str>,
    inactive: Option<&str>,
) -> Result<OpacityPlan, ArchelError> {
    let validated_active = active.map(validate_opacity).transpose()?;
    let validated_inactive = inactive.map(validate_opacity).transpose()?;
    if validated_active.is_none() && validated_inactive.is_none() {
        return Err(ArchelError::desktop_invalid(
            "opacity set needs --active and/or --inactive",
        ));
    }
    let old = current_snippet_state(base)?;
    Ok(OpacityPlan { active: validated_active, inactive: validated_inactive, .. })
}
```

### WR-03: Env-mutating tests race under the parallel harness

**File:** `crates/archel-core/src/hyprland/mod.rs:299-338`
**Issue:** `missing_binary_maps_to_typed_unavailable` (sets
`HYPRLAND_INSTANCE_SIGNATURE`) and
`unset_signature_maps_to_typed_unavailable_without_spawning` (removes it)
mutate process-wide environment with no serialisation. Rust runs tests on
multiple threads in one process: if the two tests overlap, the which-gate test
can observe the removed var (failing at the HIS gate instead of the which
gate), or the unset test can observe the fake signature and actually spawn
`hyprctl`. The SUMMARY already flags this hazard ("no interference observed")
— observed-today green is not proof; it is a flaky-test incident waiting for a
loaded CI machine.
**Fix:** Mark the module's env-touching tests `#[serial]` (via the `serial`
pattern already used elsewhere, or merge both tests into one), or eliminate
the env dependency by extracting the HIS-gate check into a pure helper:
```rust
fn session_gate() -> Result<(), ArchelError> { /* HIS check, pure over an injected var */ }
```
then unit-test the helper without touching `std::env`.

### WR-04: TTY-gated tests silently pass without asserting

**File:** `crates/archel-core/src/hyprland/mod.rs:320-324`, `crates/archel/src/cmds/appearance_cmds.rs:209-212`, `crates/archel/src/cmds/appearance_cmds.rs:278-280`
**Issue:** Three tests early-`return` when `stdin().is_terminal()`: the
out-of-session test and both confirm-abort tests. Run under an interactive
`cargo test`, they report `ok` having asserted nothing — including the two
tests that are the *only* coverage for "abort spawns no write". A developer
running the suite in a terminal gets false confidence on exactly the
safety-gate paths.
**Fix:** Invert the guard — skip only the *live-spawn* risk, or assert the
headless precondition explicitly instead of returning:
```rust
assert!(!std::io::stdin().is_terminal(), "abort-gate test requires headless stdin");
```
so an interactive run fails loudly instead of passing silently. (For the
`unset_signature` test, the pure-helper refactor from WR-03 removes the need
for the guard entirely.)

### WR-05: Backup timestamps have 1-second granularity — same-second writes collide

**File:** `crates/archel-core/src/hyprland/mod.rs:205-219`
**Issue:** `backup_before_write` names backups `<file>.lua.bak-<unix-seconds>`.
Two writes within the same second (e.g. `blur on` immediately followed by
`opacity set`, or the back-to-back writes in
`snippet_write_is_atomic_with_backup_and_rotation`) resolve to the *same*
backup path, so the second `fs::copy` silently overwrites the first backup.
The "timestamped backup set is the audit trail" (T-05-03) then holds one backup
where it should hold two, and rotation counting is skewed.
**Fix:** Include sub-second precision or a disambiguator:
```rust
let stamp = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_millis())
    .unwrap_or(0);
```
or loop-suffix `-<n>` while the candidate path exists.

### WR-06: `ensure_require_line_at` writes the user-owned entry non-atomically (and creates it from nothing)

**File:** `crates/archel-core/src/hyprland/mod.rs:276-293`
**Issue:** Unlike `write_snippet_at` (tmp + rename), the entry file
(`hyprland.lua` — user-owned, the one file D-08 cares most about) is written
with a direct truncating `fs::write` (line 289). A crash/kill mid-write leaves
a truncated entry. Additionally, `read_to_string(...).unwrap_or_default()`
(line 278) means a missing entry is silently *created* containing only the
require line — on a machine using `hyprland.conf` instead of the Lua layout,
this plants a stray `hyprland.lua` the compositor never loads (durability
illusion: the user believes the value persists across reload, it does not).
**Fix:** Reuse the tmp+rename publish path for the entry write, and refuse
instead of creating when the entry is absent:
```rust
let current = std::fs::read_to_string(&entry).map_err(|err| {
    ArchelError::hyprland_unavailable(format!("lua entry '{}' not present: {err}", entry.display()))
})?;
```

### WR-07: Hyprctl timeout maps to `Internal`, not `HyprlandUnavailable`

**File:** `crates/archel-core/src/hyprland/mod.rs:91-101`
**Issue:** Every other `run_hyprctl` failure (unset signature, missing binary,
non-zero exit) maps to the typed `HyprlandUnavailable` with its helpful French
catalog hint — but the T-05-05 timeout leg (frozen compositor, the exact case
the threat model calls out) returns `ArchelError::Internal`, rendering the
generic "une erreur interne est survenue" catalog string. The user gets the
*least* helpful message in precisely the most confusing situation (compositor
wedged), and log triage loses the typed variant.
**Fix:**
```rust
None => {
    let _ = handle.kill();
    return Err(ArchelError::hyprland_unavailable(
        format!("'{tool}' exceeded the query timeout (compositor unresponsive)"),
    ));
}
```

### WR-08: `read_opacity` couples the blur read — one leaf failure fails the whole readout

**File:** `crates/archel-core/src/hyprland/appearance.rs:308-327`
**Issue:** Bare `archel appearance opacity` is documented read-only for the two
opacity leaves, but line 321 also reads `BLUR_KEY` and maps *any* non-bool
shape to `Err`. A compositor that renames/moves the blur key (or a transient
parse quirk on that one leg) makes the opacity readout fail even though both
opacity legs succeeded. Three sequential spawns, one fused failure domain.
**Fix:** Degrade the blur leaf instead of failing — bare reads should be
honest, never brittle:
```rust
let blur = read_option(BLUR_KEY).ok().map(|v| matches!(v, OptionValue::Bool(true))).unwrap_or(true);
```
or drop the blur leaf from the opacity readout entirely (it is not displayed
by `opacity_rows` anyway).

### WR-09: Opacity preview documents "old → new" but never shows old values

**File:** `crates/archel-core/src/hyprland/appearance.rs:399-416`
**Issue:** The doc comment on `opacity_preview_rows` promises "old → new per
leaf", and `OpacityPlan` carries `old_active`/`old_inactive` (populated from
the live snippet at real cost — see WR-02), but the rows render only the *new*
values, with the catalog `sys.unavailable` N/A cell for unset leaves. The
`old_*` fields are written and never read anywhere (confirmed by grep: only
construction sites + tests reference them). The confirm gate therefore shows
less than the plan paid to know, and the doc/behavior mismatch will mislead
the next wave. Related inconsistency: the confirm prompt (lines 419-430) uses a
hardcoded `"—"` placeholder for unset leaves while the preview uses the
`sys.unavailable` catalog string.
**Fix:** Render old → new in the preview rows (using the already-paid-for
fields) and share one placeholder:
```rust
plan.active.map(|new| format!("{} -> {}", super::appearance_float(plan.old_active), super::appearance_float(new)))
    .unwrap_or_else(na)
```

## Info

### IN-01: Dead catalog keys `appearance.blur_title` and `appearance.opacity_ok`

**File:** `crates/archel-core/locales/fr.json:203,217`
**Issue:** `blur_title` and `opacity_ok` have zero Rust callers (verified by
grep — the blur path uses `blur_preview_title`/`blur_report_title`/`blur_ok`,
the opacity path renders a report table and never an ok line). Dead keys rot:
a future translator edits them believing they render, and catalog-completeness
review gets noisier. Either wire them up or delete them.
**Fix:** Delete both keys, or use `opacity_ok` in the opacity apply path for
symmetry with `blur_ok_line`.

### IN-02: `lua_quote` has no production callers — T-05-01 mitigation is test-only

**File:** `crates/archel-core/src/hyprland/mod.rs:131-144`
**Issue:** `lua_quote` (the T-05-01 Lua-injection defense) is called only by
its own unit test; every current eval/snippet value is machine-rendered
bool/float, so there is no live hole *today*. But the mitigation the threat
model relies on for the string-valued waves (font family in 05-02-01,
wallpaper paths) is currently an unexercised helper, not a proven gate in a
call path. Easy to forget to actually call it when the string leaves land.
**Fix:** No code change now — but 05-02-01 must add a golden test proving the
font/wallpaper eval path routes through `lua_quote` (rejection case included),
not just that `lua_quote` itself works.

### IN-03: `opacity_preview_title` takes a plan it ignores

**File:** `crates/archel-core/src/hyprland/appearance.rs:394-397`
**Issue:** Signature is `opacity_preview_title(plan: &OpacityPlan)` but the body
is `let _ = plan;` plus a catalog lookup. The dead parameter apes a future
need and forces every caller (and test) to fabricate a plan to ask for a
static title — compare `blur_preview_title()`, which correctly takes nothing.
**Fix:** `pub fn opacity_preview_title() -> String` and update the two call
sites (`appearance.rs` is the definer; caller is `appearance_cmds.rs:102`).

### IN-04: `hypr_dir()` silently falls back to `/tmp` when no home resolves

**File:** `crates/archel-core/src/hyprland/mod.rs:46-50`
**Issue:** If `BaseDirs::new()` fails, the writer target becomes
`/tmp/archel-no-home-hypr` — snippet + require line go to a scratch dir with
no warning, and the user is told the value applied (read-back passes) while
nothing persists under the real config tree. Bounded (HOME-less sessions are
rare) but silent.
**Fix:** Return `Result<PathBuf, ArchelError>` and fail typed
(`HyprlandUnavailable`) instead of inventing a config dir.

### IN-05: Empty opacity plan renders a degenerate eval statement

**File:** `crates/archel-core/src/hyprland/appearance.rs:282-294`
**Issue:** With `active: None, inactive: None`, `opacity_eval_statement`
renders `hl.config({ decoration = {  } })` — a no-op eval that would still
rewrite the snippet and touch the compositor. Unreachable via the CLI
(`plan_opacity_set` rejects the empty plan), but `apply_opacity_with` does not
re-check, so any future direct caller (GUI phase, tests) can drive it.
**Fix:** Early-return a typed `DesktopInvalid` in `apply_opacity_with` when
both leaves are `None` (defense in depth at the orchestration layer, not just
the planner).

### IN-06: `parse_snippet_state` matches substrings, silently defaults on corrupt input

**File:** `crates/archel-core/src/hyprland/appearance.rs:104-121`
**Issue:** Blur is detected by `text.contains("enabled = false")` *anywhere*
— including inside a comment a user hand-added to the managed block. The float
scan takes `[0-9.]` prefixes, so a hand-edited `-0.5` parses as `0.5` (sign
dropped) and `0.9 garbage` parses as `0.9`. A wholly corrupt snippet returns
defaults with `Ok`, and the next apply overwrites the file — destroying
whatever was there without ever reporting the corruption. Blast radius is
bounded (app-owned file only, values re-asserted on next apply), hence Info,
but an anchored line-level parse with an `Err` on unparseable managed blocks
would fail honest instead of guessing.
**Fix:** Parse line-anchored (`decoration = { blur ...` line grammar) and
surface `DesktopInvalid`/`Internal` on shape mismatch rather than defaulting.

---

_Reviewed: 2026-10-08T11:35:00Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
