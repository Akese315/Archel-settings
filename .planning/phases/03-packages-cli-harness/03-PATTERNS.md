# Phase 3: Packages & CLI Harness - Pattern Map

**Mapped:** 2026-10-01
**Files analyzed:** 11 (4 new, 7 modified/extended)
**Analogs found:** 11 / 11

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/archel-core/src/packages.rs` (NEW) | service | request-response (subprocess query) + CRUD (alpm reads) | `crates/archel-core/src/updates.rs` | role-match |
| `crates/archel-core/src/aur.rs` (NEW, or inside packages.rs) | service | request-response + file-I/O (yay -G fetch, PKGBUILD read) | `crates/archel-core/src/updates.rs` | role-match |
| `crates/archel-core/src/privileged.rs` (EXTEND) | service | request-response (pkexec spawn/parse) | self (`crates/archel-core/src/privileged.rs`) | exact |
| `crates/archel-privileged/src/main.rs` (EXTEND) | service (privileged) | request-response (argv allowlist) | self (`crates/archel-privileged/src/main.rs`) | exact |
| `crates/archel-core/src/error.rs` (EXTEND) | model (error taxonomy) | transform (typed error + catalog render) | self (`crates/archel-core/src/error.rs`) | exact |
| `crates/archel/src/cmds/pkg_cmds.rs` (NEW) | controller (CLI group) | request-response | `crates/archel/src/cmds/sys_cmds.rs` | exact |
| `crates/archel/src/cmds/mod.rs` (EXTEND: wire `pkg` group) | route (clap dispatch) | request-response | self (`crates/archel/src/cmds/mod.rs`) | exact |
| `crates/archel/src/cmds/landing_cmd.rs` (reference only, not modified) | controller | request-response | — (read as secondary analog for confirm/streaming contrast) | n/a |
| `crates/archel-core/locales/fr.json` (EXTEND) | config (i18n catalog) | transform | self (`crates/archel-core/locales/fr.json`) | exact |
| `policy/org.archel.privileged.policy` (EXTEND ×3 verbs) | config (polkit) | n/a | self (`policy/org.archel.privileged.policy`) | exact |
| `crates/archel-core/Cargo.toml` + `crates/archel/Cargo.toml` (EXTEND: alpm, clap_complete) | config (deps) | n/a | self (existing dep lines) | exact |

## Pattern Assignments

### `crates/archel-core/src/packages.rs` (NEW — service, request-response + CRUD)

**Analog:** `crates/archel-core/src/updates.rs` (which-gated duct spawn + timeout + stderr-log-only + typed errors)

**Imports pattern** (updates.rs lines 17-22):
```rust
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::ArchelError;
```

**Core spawn pattern** (updates.rs lines 175-200 — copy shape, adapt for LC_ALL=C + new args):
```rust
fn run_checker(tool: &str, args: &[&str]) -> Result<String, ArchelError> {
    let binary = which::which(tool).map_err(|err| ArchelError::UpdatesCheckerMissing {
        tool: format!("{tool} ({err})"),
    })?;
    let handle = duct::cmd(&binary, args)
        .stdout_capture()
        .stderr_capture()
        .start()
        .map_err(|err| ArchelError::Internal(format!("cannot run '{tool}': {err}")))?;
    let output = match handle.wait_timeout(CHECK_TIMEOUT).map_err(|err| {
        ArchelError::Internal(format!("cannot wait on '{tool}': {err}"))
    })? {
        Some(output) => output.clone(),
        None => {
            let _ = handle.kill();
            return Err(ArchelError::Internal(format!(
                "'{tool}' exceeded the refresh timeout"
            )));
        }
    };
    // Log-only capture of checker stderr (kept for diagnostics; only the
    // numeric count below ever reaches the user).
    let _stderr_for_logs = String::from_utf8_lossy(&output.stderr);
    map_update_exit(tool, output.status.code())?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
```

**Adaptations for packages.rs (from RESEARCH.md Patterns 2-3):**
- Every parse-bound spawn adds `.env("LC_ALL", "C")` before `.stdout_capture()` — RESEARCH Pattern 3 (pacman/yay localize fields; updates.rs line-counting is locale-proof and needs no change, but `-Si`/`-Ss` field parsing breaks without it).
- Live-mutation streaming does NOT use `.run()` with both captures (deadlock, Pitfall 7) — use `.reader()` + `BufRead::lines()` tee per RESEARCH code example; see streaming excerpt under `privileged.rs` below.
- Dep preview: `pacman -S --print <pkgs>` / `pacman -R --print <pkgs>` unprivileged; upgrade preview is `pacman -Qu` + per-package `-Si` (never `-Syu --print`, needs root — Pitfall 4); AUR dep preview is `yay -Si` Depends/MakeDeps (never `yay -S --print` — Pitfall 5).
- `PackageInfo` struct follows `LandingSnapshot` shape (`crates/archel-core/src/landing.rs` lines 23-28): `#[derive(Debug, Clone, PartialEq, Eq, Serialize)]`, English snake_case field names = the v1 `--json` schema, serialized directly.
- Table labels follow `landing.rs` lines 49-64: a `TableLabels` struct + `table_labels()` collecting `rust_i18n::t!(...)` strings so the bin owns no French copy.
- alpm read shape (RESEARCH-verified, no in-repo analog — use verbatim):
```rust
use alpm::Alpm;

let alpm = Alpm::new("/", "/var/lib/pacman")?;
let local = alpm.localdb();
let mut pkgs: Vec<_> = local.pkgs().iter().collect();
pkgs.sort_by(|a, b| b.version().partial_cmp(a.version()).unwrap_or(std::cmp::Ordering::Equal));
```
- Deps to add in `crates/archel-core/Cargo.toml` (append to existing `[dependencies]`, cf. lines 9-17): `alpm = { version = "5.0.2", features = ["checkver"] }`.

**Error handling pattern** (updates.rs lines 202-212):
```rust
fn refresh_half(tool: &str, args: &[&str]) -> UpdateCount {
    match run_checker(tool, args) {
        Ok(stdout) => UpdateCount::Fresh(count_package_lines(&stdout)),
        Err(ArchelError::UpdatesCheckerMissing { .. }) => {
            UpdateCount::Unavailable(UnavailableReason::CheckerMissing)
        }
        Err(_) => UpdateCount::Unavailable(UnavailableReason::RefreshFailed),
    }
}
```
Copy: missing binary → typed `YayMissing`-style variant (never crash); other failures → typed/log-detail variant; stderr is `let _stderr_for_logs`, never displayed.

**Testing pattern** (updates.rs lines 339-353, pure parse fns + fixture strings):
```rust
#[test]
fn counts_one_non_blank_line_per_package() {
    let out = "firefox 1.0-1 -> 1.1-1\nvim 9.0-1 -> 9.1-1 [1d]\n";
    assert_eq!(count_package_lines(out), 2);
}
```
Copy: pure parse/format/sort fns unit-tested on `LC_ALL=C`-shaped fixture strings; validator matrix tests for package-name charset (see helper section).

---

### `crates/archel-core/src/aur.rs` (NEW — service, request-response + file-I/O; may live inside packages.rs)

**Analog:** `crates/archel-core/src/updates.rs` (same spawn/parse/degrade shape; yay-as-user precedent)

**Core pattern — yay spawn as invoking user, no privilege** (updates.rs lines 247-249 show the exact AUR arg shape to reuse):
```rust
refresh(YAY_TOOL, &["-Qu", "--aur"])
```
New code adds: `yay -Ss/-Si` (metadata, `LC_ALL=C`-pinned), `yay -G <pkg>` into a `mktemp`-style temp dir for D-05 review, `bash -n PKGBUILD` static gate, then `yay -S` post-confirm as the user with native sudo handling (D-06 — Archel never orchestrates elevation).

**Capability-detection pattern** (updates.rs lines 230-239):
```rust
pub fn checker_missing_hint(tool: &str) -> String {
    if tool == CHECKUPDATES_TOOL {
        rust_i18n::t!("updates.checker_missing_checkupdates").to_string()
    } else {
        rust_i18n::t!("updates.checker_missing_generic", tool = tool).to_string()
    }
}
```
Copy: yay-missing → typed error + French install hint via new `fr.json` keys (planner adds `YayMissing`-equivalent variant or reuses pattern with new key).

**Catalog-label pattern** (updates.rs lines 216-228 — age_label precedent for review-card fields):
```rust
pub fn age_label(age_secs: u64) -> String {
    if age_secs < 90 {
        rust_i18n::t!("updates.age_moments").to_string()
    } else if age_secs < 5400 {
        rust_i18n::t!(
            "updates.age_minutes",
            minutes = age_secs.max(60) / 60
        )
        .to_string()
    } else {
        rust_i18n::t!("updates.age_hours", hours = age_secs / 3600).to_string()
    }
}
```
Copy: review-card composer (`sizes + source + installed-vs-candidate versions`, D-04) builds all strings via `rust_i18n::t!` in core; bin only displays. Guard: `crates/archel-core/src/i18n.rs` lines 27-55 — no hardcoded French diacritics in `.rs`, diacritics-audit test fails the build otherwise.

---

### `crates/archel-core/src/privileged.rs` (EXTEND — streaming client fns)

**Analog:** self, lines 124-148 (`run_action_via_pkexec` — pkexec spawn + 126/127 map + envelope parse + stderr-log-only).

**Existing pkexec pattern to extend** (lines 124-142):
```rust
pub fn run_action_via_pkexec(action: &str) -> Result<String, ArchelError> {
    let pkexec = which::which("pkexec").map_err(|err| {
        ArchelError::HelperProtocol(format!("pkexec not available: {err}"))
    })?;
    let helper = locate_helper()?;
    let output = duct::cmd(&pkexec, [&helper, &PathBuf::from(action)])
        .stdout_capture()
        .stderr_capture()
        .run()
        .map_err(|err| {
            ArchelError::HelperProtocol(format!("cannot spawn pkexec for '{action}': {err}"))
        })?;
    // Log-only capture of helper stderr (kept for diagnostics; the
    // user-facing error below never embeds it).
    let _stderr_for_logs = String::from_utf8_lossy(&output.stderr);
    map_pkexec_exit(action, output.status.code())?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_helper_stdout(action, &stdout)
}
```

**Exit-code matrix to reuse verbatim** (lines 109-118):
```rust
pub fn map_pkexec_exit(action: &str, code: Option<i32>) -> Result<(), ArchelError> {
    match code {
        Some(0) => Ok(()),
        Some(126) => Err(ArchelError::privilege_cancelled(action)),
        Some(127) => Err(ArchelError::privilege_denied(action)),
        other => Err(ArchelError::HelperProtocol(format!(
            "pkexec exited with code {other:?} for '{action}'"
        ))),
    }
}
```
New `pkg_install/pkg_remove/pkg_upgrade` fns reuse `locate_helper` (lines 64-72, absolute path next to `archel` binary — never `$PATH`) + `map_pkexec_exit` + `parse_helper_stdout` (lines 41-58, `v == 1` gate, never default-ok). New code passes validated package args through (helper re-validates; CLI never trusted).

**Streaming adaptation (D-02)** — `.run()` with both captures deadlocks on long `pacman -Syu` output (Pitfall 7); new streaming fns use the RESEARCH-verified reader shape (no in-repo analog — `updates.rs` uses `start()` + `wait_timeout()`, which has no reader; use verbatim):
```rust
use std::io::{BufRead, BufReader};

let handle = duct::cmd(&pkexec, [&helper, &action_arg])
    .env("LC_ALL", "C")
    .stderr_capture()
    .reader()?;
let mut transcript = String::new();
for line in BufReader::new(&handle).lines().map_while(Result::ok) {
    println!("{line}");
    transcript.push_str(&line);
    transcript.push('\n');
}
```

**Absolute-path rule** (privileged.rs lines 60-72): helper located via `current_exe().parent().join("archel-privileged")`; helper itself execs `/usr/bin/pacman` absolute (PATH-trojan mitigation, RESEARCH Security Domain).

---

### `crates/archel-privileged/src/main.rs` (EXTEND — pkg-install/pkg-remove/pkg-upgrade verbs)

**Analog:** self, lines 27-33 (pure `handle_args` allowlist + exit-code contract) and lines 42-90 (matrix tests).

**Existing allowlist pattern** (lines 25-33):
```rust
/// Pure allowlist decision: the exit code plus the stdout payload.
/// Kept pure so the matrix tests run without root and without spawning.
fn handle_args(args: &[String]) -> (i32, &'static str) {
    if args.len() == 1 && args[0] == "dry-run" {
        (0, dry_run_json())
    } else {
        (2, rejection_json())
    }
}
```

**Extension shape (RESEARCH Pattern 1 — verb + strictly-validated-args, NOT exact-match):**
- `pkg-upgrade` stays exact-match: `args == ["pkg-upgrade"]` → `pacman -Syu --noconfirm` (zero open-ended args).
- `pkg-install <pkgs…>` / `pkg-remove <pkgs…>`: first arg is the verb, rest are package names; EVERY package must match `^[a-zA-Z0-9@._+\-]+$`, must not start with `-`, must be non-empty; helper inserts `--` between hardcoded flags and packages; ALL flags (`-S --noconfirm` / `-Rns --noconfirm` or planner's A8 choice / `-Syu --noconfirm`) hardcoded server-side, never from argv. No shell ever.
- Keep `handle_args` pure (returns exit code + payload) so the validator matrix runs without root — extend the existing tests below.

**Existing test-matrix pattern to extend** (lines 59-89):
```rust
#[test]
fn smuggled_extra_arg_is_rejected_with_exit_two() {
    let (code, payload) = handle_args(&args(&["dry-run", "extra"]));
    assert_eq!(code, 2);
    assert!(payload.contains("\"v\":1"));
    assert!(payload.contains("\"ok\":false"));
    assert!(payload.contains("action-not-allowlisted"));
}

#[test]
fn prefix_match_does_not_pass_the_allowlist() {
    // "dry-runner" shares a prefix with "dry-run" but must be rejected:
    // prefix matching near a root process is a privilege hole (T-1-03).
    let (code, _) = handle_args(&args(&["dry-runner"]));
    assert_eq!(code, 2);
    let (code, _) = handle_args(&args(&["dry-"]));
    assert_eq!(code, 2);
}
```
New tests add injection cases: `-foo`, `--overwrite`, `foo;bar`, empty string, `pkg-install` with zero packages — all must yield exit 2 + `action-not-allowlisted`.

**Envelope contract** (lines 11-23): `PROTOCOL_VERSION: u32 = 1`, stdout JSON `{"v":1,…}`, rejection exit 2 — new verbs keep the same envelope so `parse_helper_stdout` needs no change.

---

### `crates/archel-core/src/error.rs` (EXTEND — new variants)

**Analog:** self, lines 24-58 (8-variant taxonomy) + lines 81-104 (`message()` catalog dispatch).

**Variant pattern to extend** (lines 45-57):
```rust
/// An update checker binary (`checkupdates`, `yay`) is not installed.
/// Carries the missing tool name for logs; the user only ever sees the
/// catalog hint, and callers degrade (stale-with-age) instead of
/// failing (D-06).
#[error("update checker not available: '{tool}' is not installed")]
UpdatesCheckerMissing { tool: String },
```

**Render pattern to extend** (lines 98-100):
```rust
Self::UpdatesCheckerMissing { tool } => {
    rust_i18n::t!("errors.updates_checker_missing", tool = tool).to_string()
}
```
RESEARCH Open Question 2 proposes: `PackageNotFound{…}`, `YayMissing`, `NeedsTty`, `ConfirmationAborted`, `PkgbuildRejected{…}` (planner may merge). Each follows the same shape: English `#[error(...)]` Display for logs + `message()` arm rendering a `fr.json` `errors.*` key; `every_variant_renders_non_empty_french` test (lines 128-138) must include every new variant.

---

### `crates/archel/src/cmds/pkg_cmds.rs` (NEW — controller, request-response)

**Analog:** `crates/archel/src/cmds/sys_cmds.rs` (bin-calls-core: numbers + French from core, bin only lays out + frames JSON + owns loops).

**Module-docstring contract to copy** (sys_cmds.rs lines 1-22 — adapt names):
```rust
//! `archel sys cpu|ram|disk|gpu|temp|power|all` renderers: French
//! tables (default) or one JSON object per tick (`--json`), with
//! `--watch` re-render.
```
pkg_cmds.rs equivalent: `archel pkg search|list|install|remove|upgrade|info` renderers — French tables/cards (default) or one JSON object (`--json`); NO `--watch` (mutations never loop); confirmation is TTY-gated with default-No.

**Per-command fn shape to copy** (sys_cmds.rs lines 37-51):
```rust
pub fn run_cpu(watch: bool, json: bool) -> Result<(), ArchelError> {
    let mut monitor = archel_core::system::Monitor::new();
    loop {
        let snap = monitor.cpu();
        let payload = serde_json::to_string(&snap).map_err(|err| {
            ArchelError::Internal(format!("cannot encode sys cpu JSON: {err}"))
        })?;
        emit_tick(json, watch, &payload, &render_cpu_table(&snap));
        if !watch {
            return Ok(());
        }
        std::thread::sleep(WATCH_INTERVAL);
    }
}
```
Copy: `run_search/run_list/run_install/…(…, json: bool) -> Result<(), ArchelError>` — snapshot/compose in core, `serde_json::to_string` with `ArchelError::Internal("cannot encode …")` on failure, branch table vs JSON.

**Emit shape to copy** (sys_cmds.rs lines 155-164):
```rust
fn emit_tick(json: bool, watch: bool, payload: &str, table: &str) {
    if json {
        println!("{payload}");
    } else {
        if watch {
            print!("\x1Bc");
        }
        print!("{table}");
    }
}
```
pkg variant drops `watch` (single-shot emit: `if json { println!(payload) } else { print!(table) }`).

**Table shape to copy** (sys_cmds.rs lines 282-295 + 337-341):
```rust
fn render_label_table(title: &str, rows: &[(String, String)]) -> String {
    let width = rows
        .iter()
        .map(|(label, _)| label.chars().count())
        .max()
        .unwrap_or(0);
    let mut out = format!("{title}\n");
    for (label, value) in rows {
        out.push_str(&format!("{} : {value}\n", pad_right(label, width)));
    }
    out
}

fn pad_right(s: &str, width: usize) -> String {
    let missing = width.saturating_sub(s.chars().count());
    format!("{s}{}", " ".repeat(missing))
}
```
Copy verbatim for the pre-install card (D-04: sizes + source + installed-vs-candidate versions) and search/list tables. Char-count padding (not bytes) keeps accented headers aligned — covered by `padding_counts_characters_not_bytes` test (sys_cmds.rs lines 567-571).

**Confirmation shape (no in-repo analog — RESEARCH Pattern 4, hand-rolled, use verbatim):**
```rust
// eprint! prompt + stdin().read_line(); empty/EOF/non-`o`-prefix →
// Err(ArchelError::ConfirmationAborted); non-TTY stdin → Err(ArchelError::NeedsTty).
// Accept `o`/`O`/`oui…` prefix (FR). Never use dialoguer/inquire.
```

**Completions shape (no in-repo analog — RESEARCH-verified `aot::` path, use verbatim):**
```rust
use clap_complete::aot::{Shell, generate};
use std::io;

let mut cmd = Cli::command();
generate(shell, &mut cmd, "archel", &mut io::stdout());
```
Dep to add in `crates/archel/Cargo.toml` (next to `clap`, cf. line 18): `clap_complete = "4.6"`. NOTE: root re-exports are deprecated in 4.6.11 — always the `aot::` path.

**Testing pattern to copy** (sys_cmds.rs lines 343-479: fixtures + table asserts + JSON round-trip + insta pins):
- Fixture constructors (`cpu_fixture`/`ram_fixture` pattern, lines 348-365) for `PackageInfo`/card snapshots.
- Table-content asserts (`table.contains("12.5")` pattern, lines 428-435).
- `snapshots_serialize_to_parseable_json` (lines 466-479): every payload must parse as one complete JSON object.
- insta pins per schema (lines 532-565, e.g. `insta::assert_json_snapshot!("sys_cpu", cpu_fixture());`) — new pins: card, search, list. Existing snapshots live in `crates/archel/src/cmds/snapshots/` (8 files: landing + sys_cpu/ram/disk/gpu/temp/power/all).

---

### `crates/archel/src/cmds/mod.rs` (EXTEND — route)

**Analog:** self, lines 13-48 (clap derive, global `--json`) + lines 56-102 (`SysMetric` group) + lines 134-146 (dispatch).

**Clap group pattern to extend** (lines 43-48 + 56-58):
```rust
/// Live CPU, memory and system-partition disk insight (D-11)
Sys {
    #[command(subcommand)]
    metric: SysMetric,
},
```
New: `Pkg { #[command(subcommand)] action: PkgAction }` alongside `Sys`/`Landing`; `mod pkg_cmds;` next to `mod sys_cmds;` (lines 10-11); EN subcommand names (D-04), FR output only via core catalog (D-05/D-07).

**Dispatch pattern to extend** (lines 137-145):
```rust
Command::Sys { metric } => match metric {
    SysMetric::Cpu { watch } => sys_cmds::run_cpu(watch, cli.json),
    SysMetric::Ram { watch } => sys_cmds::run_ram(watch, cli.json),
    ...
},
```
New: `Command::Pkg { action } => match action { PkgAction::Search{…} => pkg_cmds::run_search(…, cli.json), … }`. Global `json` flag (lines 16-18, `#[arg(long, global = true)]`) flows through unchanged — D-10 requires every new subcommand drivable with `--json`.

**Error contract** (lines 104-109 + main.rs lines 14-22): every failure returns typed `ArchelError`; `main` renders `err.message()` on stderr with non-zero exit. No panics, no silent ok.

---

### `crates/archel-core/locales/fr.json` (EXTEND — catalog)

**Analog:** self — `sys` block (lines 8-37), `updates` hints (lines 48-55), `errors` block (lines 56-65).

**Key shapes to mirror:**
```json
"sys": {
  "cpu_title": "Processeur",
  "unavailable": "N/A",
  "unit_gib": "Gio"
},
"updates": {
  "checker_missing_checkupdates": "« checkupdates » est introuvable : lancez « sudo pacman -S pacman-contrib » …",
  "checker_missing_generic": "« %{tool} » est introuvable : installez le paquet correspondant …"
},
"errors": {
  "auth_cancelled": "Authentification annulée — « %{action} » n'a pas été exécutée.",
  "updates_checker_missing": "Compteur indisponible : « %{tool} » est introuvable. …"
}
```
New keys: `pkg.*` (card labels, search/list headers, confirm prompt `[o/N]`, D-03 partial-upgrade refusal, DB-age footnote), `errors.package_not_found/yay_missing/needs_tty/confirmation_aborted/pkgbuild_rejected` (planner's variant set). Interpolation uses `%{name}`. All French copy lives here — the diacritics audit (`i18n.rs` test) forbids accented literals in `.rs`.

---

### `policy/org.archel.privileged.policy` (EXTEND — 3 action blocks)

**Analog:** self, lines 19-27 (dry-run block).

**Block to mirror ×3** (lines 19-27):
```xml
<action id="org.archel.privileged.dry-run">
        <description>Tester le module privilegie Archel</description>
        <message>L'authentification est requise pour tester le module privilegie Archel.</message>
        <defaults>
                <allow_any>auth_admin_keep</allow_any>
                <allow_inactive>auth_admin_keep</allow_inactive>
                <allow_active>auth_admin_keep</allow_active>
        </defaults>
</action>
```
New ids (RESEARCH OQ-3 recommendation): `org.archel.privileged.pkg-install`, `org.archel.privileged.pkg-remove`, `org.archel.privileged.pkg-upgrade` — same `auth_admin_keep` triple, never `yes` (T-1-04 least-privilege comment, lines 8-13, stays).

---

## Shared Patterns

### Privilege boundary
**Source:** `crates/archel-core/src/privileged.rs` (locate/spawn/parse/126-127) + `crates/archel-privileged/src/main.rs` (allowlist) + `policy/org.archel.privileged.policy` (action ids)
**Apply to:** `packages.rs` mutation paths, `privileged.rs` new fns, helper new verbs, policy new blocks
- Reads stay unprivileged in-process; mutations go through `archel-privileged` via pkexec; helper re-validates every invocation (never trusts the CLI); absolute binary paths both sides.

### Subprocess (unprivileged reads)
**Source:** `crates/archel-core/src/updates.rs` lines 175-200 (`which` gate → `duct` + captures + `wait_timeout` → stderr-log-only → exit-map → stdout string)
**Apply to:** all `packages.rs`/`aur.rs` pacman/yay query spawns
- Add `.env("LC_ALL", "C")` on every parse-bound spawn (new vs updates.rs); long/streaming children use `.reader()` tee, never `.run()` with both captures.

### i18n / error rendering
**Source:** `crates/archel-core/src/error.rs` (`ArchelError::message()` sole terminal renderer) + `crates/archel-core/locales/fr.json` (all FR copy) + `crates/archel-core/src/i18n.rs` (diacritics guard)
**Apply to:** every new error variant, card/table label, confirm prompt, refusal message
- EN identifiers in code, FR via `rust_i18n::t!` keys; `Display` stays English log-only; `main.rs` prints `message()` on stderr, exit non-zero.

### CLI shape (bin-calls-core)
**Source:** `crates/archel/src/cmds/sys_cmds.rs` + `crates/archel/src/cmds/landing_cmd.rs` (core composes snapshots + labels; bin lays out + frames JSON) + `crates/archel/src/cmds/mod.rs` (EN names, global `--json`)
**Apply to:** `pkg_cmds.rs` + `mod.rs` wiring + `completions` subcommand
- Core owns data + French strings; bin owns layout/JSON framing; `--json` emits exactly one complete object (insta-pinned in `crates/archel/src/cmds/snapshots/`); `completions` uses `clap_complete::aot::generate()` to stdout for bash+zsh+fish.

### Testing
**Source:** `crates/archel-core/src/updates.rs` (pure-fn fixture tests, TTL/cache envelope tests), `crates/archel-privileged/src/main.rs` (allowlist matrix), `crates/archel/src/cmds/sys_cmds.rs` (fixture tables + JSON round-trip + insta pins), `crates/archel-core/src/error.rs` (every-variant-renders test), `crates/archel-core/src/i18n.rs` (diacritics audit)
**Apply to:** all new code
- Per-task: `cargo test -p archel-core -p archel -p archel-privileged --no-default-features`; per-wave: + `rg '"-Sy"'` negative gate (no bare `-Sy` literal outside the refusal test) + diacritics audit.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| (none — all covered) | — | — | alpm localdb reads, `LC_ALL=C` field parsing, TTY-gated confirm, `clap_complete::aot` generation, and `reader()` streaming have no in-repo precedent; planner must use the RESEARCH.md Code Examples verbatim for those five shapes (all sourced from vendored crate sources + live probes cited in 03-RESEARCH.md §§ Code Examples / Sources). |

## Metadata

**Analog search scope:** `crates/` (all 17 `.rs` files), `policy/`, `crates/*/Cargo.toml`, `crates/archel/src/cmds/snapshots/`, `crates/archel-core/locales/fr.json`
**Files scanned:** 17 Rust sources + policy + 3 Cargo.tomls + catalog + 8 snapshots
**Pattern extraction date:** 2026-10-01
**Tracked-source gate:** all 13 cited paths verified via `git ls-files` (non-empty output) — no mirror/untracked paths emitted.
