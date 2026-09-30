# Phase 2: System Insight - Pattern Map

**Mapped:** 2026-09-30
**Files analyzed:** 12 (5 new core modules, 2 new CLI modules, 5 modified)
**Analogs found:** 8 / 12 (4 greenfield sensor/samplers have no in-repo analog — planner uses RESEARCH.md crate-source patterns)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/archel-core/src/system.rs` (new) | service (sampler) | request-response (snapshot fn) | `crates/archel-core/src/privileged.rs` | role-match |
| `crates/archel-core/src/sensors.rs` (new) | service (sensor glue) | transform (sysfs→typed values) | `crates/archel-core/src/privileged.rs` | role-match |
| `crates/archel-core/src/updates.rs` (new) | service (subprocess+cache) | request-response + file-I/O | `crates/archel-core/src/privileged.rs` | exact (spawn/parse) |
| `crates/archel-core/src/landing.rs` (new) | service (composer) | transform (compose→snapshot struct) | `crates/archel-core/src/lib.rs` (`version_line`) | role-match |
| `crates/archel-core/src/cache.rs` (new, or fold into `updates.rs`) | utility (TTL JSON cache) | file-I/O | `crates/archel-core/src/privileged.rs` (`HelperEnvelope` + `PROTOCOL_VERSION`) | partial |
| `crates/archel-core/src/error.rs` (modify: new variants) | model (error taxonomy) | — | itself (`crates/archel-core/src/error.rs`) | exact |
| `crates/archel-core/src/lib.rs` (modify: `pub mod` lines) | config (crate surface) | — | itself (`crates/archel-core/src/lib.rs` lines 11-13) | exact |
| `crates/archel-core/locales/fr.json` (modify: new keys) | config (catalog) | — | itself (`crates/archel-core/locales/fr.json`) | exact |
| `crates/archel-core/Cargo.toml` (modify: sysinfo/nvml-wrapper/directories) | config (deps) | — | itself (`crates/archel-core/Cargo.toml`) | exact |
| `crates/archel/src/cmds/mod.rs` (modify: `Sys` group + `Landing`) | controller (clap wiring) | request-response | itself (`crates/archel/src/cmds/mod.rs`) | exact |
| `crates/archel/src/cmds/sys_cmds.rs` (new) | controller (per-metric render + watch loop) | request-response + streaming (watch/NDJSON) | `crates/archel/src/cmds/mod.rs` | role-match |
| `crates/archel/src/cmds/landing_cmd.rs` (new, or fold into `sys_cmds.rs`) | controller (landing render) | request-response | `crates/archel/src/cmds/mod.rs` | role-match |

## Pattern Assignments

### `crates/archel-core/src/system.rs` (service, snapshot fns)

**Analog:** `crates/archel-core/src/privileged.rs`

**Imports pattern** (privileged.rs lines 12-16 — what a core module header looks like):
```rust
use std::path::PathBuf;

use serde::Deserialize;

use crate::error::ArchelError;
```

**Core pattern — fallible public fns returning `Result<_, ArchelError>`, pure parse helpers kept testable without hardware** (privileged.rs lines 36-58):
```rust
/// Validates one helper stdout payload for `action`.
///
/// Returns the echoed action name on success. Every other shape —
/// unparseable JSON, version skew, `ok` missing/false — becomes
/// [`ArchelError::HelperProtocol`] with log detail (never user-facing).
pub fn parse_helper_stdout(action: &str, stdout: &str) -> Result<String, ArchelError> {
    let envelope: HelperEnvelope = serde_json::from_str(stdout).map_err(|err| {
        ArchelError::HelperProtocol(format!("unparseable helper output for '{action}': {err}"))
    })?;
```

**Copy this shape:** `pub fn cpu_snapshot() -> Result<Vec<CpuCore>, ArchelError>`, `ram_snapshot()`, `disk_snapshot()`, etc. — public snapshot fns returning `Result`, with pure unit-convertible helpers (`watts_from_milliwatts`, `pick_hottest`, `root_partition`) extracted as pure fns testable on fixtures. Module doc comment (`//!`) states the contract first, exactly like privileged.rs lines 1-10.

**Unit-test pattern** (privileged.rs lines 157-176 — table of pure cases, `expect_err` with message):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_v1_ok_envelope_returns_the_action() {
        let out = parse_helper_stdout(
            "dry-run",
            r#"{"v":1,"action":"dry-run","ok":true}"#,
        )
        .expect("valid envelope must parse");
        assert_eq!(out, "dry-run");
    }
```

**No in-repo analog for the sysinfo calls themselves** — planner must use RESEARCH.md Code Examples (two-pass CPU sampling, `Disks::new_with_refreshed_list` + `mount_point() == "/"`, `Components` hottest pick), which were verified against the downloaded 0.39.6 sources.

---

### `crates/archel-core/src/sensors.rs` (service, sysfs + NVML glue)

**Analog:** `crates/archel-core/src/privileged.rs`

**What to copy:** same module skeleton as `system.rs` above (imports → consts → public fallible fns → `#[cfg(test)] mod tests`). Two specifics:

**Const-gated protocol values, not magic literals** (privileged.rs lines 18-23):
```rust
/// Protocol version the helper speaks on stdout. Any envelope with a
/// different `v` is a [`ArchelError::HelperProtocol`] (never default-ok).
pub const PROTOCOL_VERSION: u32 = 1;
```
→ `pub const RAPL_TTL...` style: `CACHE_TTL_SECS`, sysfs dir consts (`/sys/class/powercap`, `/sys/class/hwmon`), unit divisors (`MILLIWATT_TO_WATT: f32 = 1000.0`, `MILLIDEGREE_TO_C: f32 = 1000.0`) as named consts with doc comments.

**Discovery-by-enumeration, never hardcoded indices** — no literal analog exists, but the *negative* precedent is `locate_helper` (privileged.rs lines 64-72): resolve paths at runtime from the environment, fail typed when absent:
```rust
pub fn locate_helper() -> Result<PathBuf, ArchelError> {
    let current = std::env::current_exe().map_err(|err| {
        ArchelError::HelperProtocol(format!("cannot locate current executable: {err}"))
    })?;
```

**No in-repo analog for NVML/hwmon/RAPL I/O** — planner uses RESEARCH.md Patterns 2-4 (GpuBackend trait, `Nvml::init().and_then(device_by_index(0))` → N/A backend on `Err`, RAPL `energy_uj` delta with `max_energy_range_uj` wrap handling, hwmon discovery by `name` file). Missing sensor → `SensorValue::Unavailable` / typed error, never zero (mirrors the never-default-ok rule in `parse_helper_stdout`).

---

### `crates/archel-core/src/updates.rs` (service, subprocess + stale cache)

**Analog:** `crates/archel-core/src/privileged.rs` — **exact match, copy line-for-line shapes below.**

**Subprocess pattern — `which` gate + `duct` + stdout/stderr capture, stderr log-only** (privileged.rs lines 124-142):
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
```

**Copy for:** `checkupdates` and `yay -Qu` recomputation — `which::which("checkupdates")` gate → typed missing-checker error, `duct::cmd` with absolute path, `.stdout_capture().stderr_capture()`, stderr kept log-only, count `stdout.lines()` (never eval). Add a timeout (10–15 s) on top; on ANY failure return last cache + age (D-06), never a hard error.

**Exit-code mapping as a pure fn** (privileged.rs lines 101-118):
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
→ same treatment for `checkupdates`/`yay` offline exits: pure `map_update_exit(tool, code)` fn, unit-tested with mocked codes, unknown codes → stale-with-age path.

**Versioned JSON envelope precedent** (privileged.rs lines 25-34):
```rust
#[derive(Debug, Deserialize)]
struct HelperEnvelope {
    v: u32,
    action: Option<String>,
    ok: Option<bool>,
    error: Option<String>,
}
```
→ `UpdateCache { v: u32, computed_at: u64, repo: u32, aur: u32 }` with `#[derive(Serialize, Deserialize)]`, `v == 1` gate, corrupt/parse failure → discard + recompute, never crash.

---

### `crates/archel-core/src/landing.rs` (service, snapshot composer)

**Analog:** `crates/archel-core/src/lib.rs` lines 24-38 (`version_line`)

**Composer pattern — core builds the French-ready string from the catalog, bin only prints** (lib.rs lines 28-38):
```rust
/// French `archel version` line, rendered from the catalog (D-05/D-07).
/// Takes no input that could surprise it: unexpected core data surfaces as
/// a typed error upstream, never as a default-ok line.
pub fn version_line() -> String {
    rust_i18n::t!(
        "app.version_line",
        bin = env!("CARGO_PKG_VERSION"),
        core = library_version()
    )
    .to_string()
}
```

**Copy this shape:** `pub fn snapshot() -> Result<LandingSnapshot, ArchelError>` composing `library_version()` + `System::kernel_version()` + cached update count, plus `pub fn render_*` helpers that take the struct and return catalog-rendered `String`s via `rust_i18n::t!("landing.…")`. Table column layout lives in the bin; core returns structs + catalog keys only (RESEARCH.md Anti-Patterns).

---

### `crates/archel-core/src/cache.rs` (utility, TTL JSON file — or fold into `updates.rs`)

**Analog:** `crates/archel-core/src/privileged.rs` (`HelperEnvelope` + `PROTOCOL_VERSION`, lines 18-34) — partial match; no generic cache helper exists in-repo.

**What to copy:** version-gated envelope struct + const (`PROTOCOL_VERSION` pattern above), atomic write (tmp + rename), mtime-based TTL check (`now - computed_at < 900`). XDG path via `directories::ProjectDirs::cache_dir()` (new dep, no analog — RESEARCH.md Pattern 5). Planner decides split vs fold; if folded, the patterns still land in `updates.rs`.

---

### `crates/archel-core/src/error.rs` (modify — add Phase 2 variants)

**Analog:** itself — extend in place, following the existing three-part shape.

**Variant declaration** (error.rs lines 24-41):
```rust
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ArchelError {
    /// Privilege escalation failed for `action` (cancelled or denied).
    #[error("authentication failed for action '{action}': {reason:?}")]
    AuthFailed {
        action: String,
        reason: AuthFailureReason,
    },
```

**New variants follow the same mold:** English `#[error("…")]` log string (never user-facing), doc comment naming the decision, `PartialEq, Eq` preserved for test assertions. Candidate set (planner finalizes): `SensorUnavailable { sensor: String }`, `SystemPartitionNotFound`, `UpdatesCheckerMissing { tool: String }`, `CacheUnreadable(String)` — each carrying log detail only.

**Rendering arm** (error.rs lines 64-78 — the ONLY terminal path):
```rust
pub fn message(&self) -> String {
    match self {
        Self::AuthFailed {
            action,
            reason: AuthFailureReason::Cancelled,
        } => rust_i18n::t!("errors.auth_cancelled", action = action).to_string(),
        ...
        Self::Internal(_) => rust_i18n::t!("errors.internal").to_string(),
    }
}
```
→ one new `t!("errors.…")` arm per variant; raw payloads stay in `Display`/logs (T-1-02 rule in the file header, lines 1-8).

**Test extension** (error.rs lines 85-93 — exhaustive variant list):
```rust
fn every_variant() -> Vec<ArchelError> {
    vec![
        ArchelError::privilege_cancelled("package install"),
        ...
    ]
}
```
→ append every new variant to `every_variant()` so `every_variant_renders_non_empty_french` covers them from the first commit.

---

### `crates/archel-core/src/lib.rs` (modify — expose new modules)

**Analog:** itself, lines 11-13:
```rust
pub mod error;
pub mod i18n;
pub mod privileged;
```
→ append `pub mod cache; pub mod landing; pub mod sensors; pub mod system; pub mod updates;` (alphabetical, matching existing order). Keep the `rust_i18n::i18n!("locales", fallback = "fr");` backend declaration untouched at line 17.

---

### `crates/archel-core/locales/fr.json` (modify — all new French strings)

**Analog:** itself (full file, 15 lines):
```json
{
  "app": {
    "version_line": "Archel version %{bin} — moteur archel-core %{core}"
  },
  "privileged": {
    "dry_run_ok": "Module privilégié vérifié : réponse JSON v1 validée pour « %{action} »."
  },
  "errors": {
    "auth_cancelled": "Authentification annulée — « %{action} » n'a pas été exécutée.",
    "auth_denied": "Authentification refusée — « %{action} » n'a pas été autorisée.",
    "helper_protocol": "Le module privilégié a répondu de façon inattendue. Réessayez.",
    "gui_not_compiled": "Archel settings : interface graphique non disponible dans cette version.",
    "internal": "Une erreur interne est survenue. Réessayez."
  }
}
```
**Conventions to copy:** `%{var}` interpolation, `« … »` quoting of dynamic values, `—` separators, no raw polkit/errno text. New sections: `sys.*` (table headers, units Go/MHz/°C/W), `landing.*`, `updates.*` (age strings, missing-checker hint with install command), `errors.*` (one key per new variant). Every header/unit/label is a key from the first commit — the diacritics guard (`i18n.rs` test `no_hardcoded_french_diacritics_in_rust_sources`, lines 28-55) scans ALL `.rs` files repo-wide and will fail the build otherwise.

---

### `crates/archel/src/cmds/mod.rs` (modify — `Sys` group + `Landing`)

**Analog:** itself (full file, 67 lines) — the clap-derive + bin-calls-core skeleton:

**CLI definition** (mod.rs lines 10-34):
```rust
#[derive(Debug, Parser)]
#[command(name = "archel", version, about = "Archel system settings")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print version information
    Version,
    ...
    /// Probe the privileged helper (pkexec boundary)
    Privileged {
        #[command(subcommand)]
        action: PrivilegedAction,
    },
}
```
→ add `Sys { #[command(subcommand)] metric: SysMetric }` group + top-level `Landing { #[arg(long)] refresh: bool, #[arg(long)] watch: bool, #[arg(long)] json: bool }`. Doc comments stay English (D-04); `--json` is a single global flag (D-12) — planner decides `ArgAction::SetTrue` global vs per-command.

**Dispatch** (mod.rs lines 41-67):
```rust
pub fn run() -> Result<(), ArchelError> {
    // The catalog backend defaults to the fallback locale; pin French so
    // every rendering below is deterministic (D-05/D-07).
    archel_core::i18n::set_language("fr");
    let cli = Cli::parse();
    match cli.command {
        Command::Version => {
            println!("{}", archel_core::version_line());
            Ok(())
        }
```

**Error propagation to `main`** (`crates/archel/src/main.rs` lines 14-22 — the terminal renderer, never duplicated):
```rust
fn main() -> ExitCode {
    match cmds::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{}", err.message());
            ExitCode::FAILURE
        }
    }
}
```

---

### `crates/archel/src/cmds/sys_cmds.rs` + `landing_cmd.rs` (new controllers)

**Analog:** `crates/archel/src/cmds/mod.rs` dispatch arms (lines 46-66) — the bin-calls-core-prints shape:
```rust
PrivilegedAction::DryRun => {
    let echoed = archel_core::privileged::dry_run_direct()?;
    println!("{}", archel_core::privileged::dry_run_success_line(&echoed));
    Ok(())
}
```
→ every metric handler is `let snap = archel_core::system::cpu_snapshot()?; println!("{}", render_table(&snap)); Ok(())` — `?` propagates typed errors to `main`, which renders French on stderr. **No logic, no French literals, no sysfs/NVML/subprocess in the bin** (RESEARCH.md Architecture Patterns + Anti-Patterns).

**Table rendering:** `std::fmt` width specifiers (`{:<12}`) inline — no `comfy-table`/`tabled` dep (RESEARCH.md Don't Hand-Roll). All headers via `rust_i18n::t!` keys.

**Watch loop:** no in-repo analog (greenfield) — planner uses RESEARCH.md Pattern 6 (`loop { snapshot; render; sleep(500ms) }`, `\x1Bc` re-render for human mode, one `println!(json)` per tick for NDJSON, no `crossterm`). NVML handle + `System` instance created once outside the loop, reused across ticks.

---

## Shared Patterns

### Bin-calls-core / catalog-French (applies to: all `cmds/*.rs`, all new core modules)
**Sources:** `crates/archel/src/cmds/mod.rs` lines 41-50; `crates/archel-core/src/lib.rs` lines 28-38; `crates/archel-core/locales/fr.json`
- Bin: `set_language("fr")` once in `run()`, call `archel_core::…`, `println!` the returned `String`. `?` on every fallible call; `main` renders `err.message()` on stderr with `ExitCode::FAILURE`.
- Core: public fns return `Result<T, ArchelError>` or catalog-rendered `String` via `rust_i18n::t!`. No `println!`/`eprint!` in core (payload printing is presentation).
- French strings exist ONLY in `fr.json` (`%{var}` interpolation); the `no_hardcoded_french_diacritics_in_rust_sources` guard fails the build on violations — run `cargo test -p archel-core` after each renderer commit.

### Typed errors, never-default-ok (applies to: `error.rs`, `system.rs`, `sensors.rs`, `updates.rs`, `cache.rs`)
**Source:** `crates/archel-core/src/error.rs` lines 21-41 + `privileged.rs` lines 41-58
- New failure modes become `ArchelError` variants with English `#[error]` log strings; user sees only `message()` catalog French.
- Parsers/validators reject unknown shapes (`version skew`, `ok missing/false`, corrupt cache, missing sysfs node) as typed errors — never silent defaults, never panics, never fake zeros for missing sensors (`Option`/`Unavailable` across the core→CLI boundary).

### Subprocess via which+duct, stderr log-only (applies to: `updates.rs` only)
**Source:** `crates/archel-core/src/privileged.rs` lines 124-142
```rust
let pkexec = which::which("pkexec").map_err(|err| {
    ArchelError::HelperProtocol(format!("pkexec not available: {err}"))
})?;
let output = duct::cmd(&pkexec, [&helper, &PathBuf::from(action)])
    .stdout_capture()
    .stderr_capture()
    .run()
```
- Absolute binary path from `which`, argv array (never shell), both streams captured, stderr bound to `_…_for_logs` and never displayed. `checkupdates`/`yay` run as the invoking user — no pkexec, no sudo, no `-Sy` in Phase 2.

### Test gates (applies to: every new module)
**Sources:** `privileged.rs` tests (lines 157-271), `error.rs` tests (lines 81-164), `archel-privileged/src/main.rs` tests (lines 42-90)
- Pure fns unit-tested with mocked inputs (exit codes, JSON payloads, argv vectors) — see `map_pkexec_exit` matrix test and `handle_args` allowlist tests as the copy-paste template.
- Per-module `#[cfg(test)] mod tests` with `expect("…")` / `expect_err("…")` messages.
- Gates: `cargo test -p archel-core --no-default-features` per commit; `cargo test --workspace --no-default-features` per wave (headless; no Qt).

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `system.rs` sysinfo call shapes | sampler | snapshot | sysinfo 0.39 API (`refresh_cpu_all`, `Disks::new_with_refreshed_list`, `Components`) is new to this repo — use RESEARCH.md Code Examples (verified against 0.39.6 sources, not training memory) |
| `sensors.rs` NVML + RAPL + hwmon I/O | sensor glue | transform | No GPU/sysfs code exists in-repo — use RESEARCH.md Patterns 2-4 + live-machine sensor map (k10temp hottest, RAPL `package-0`, NVML-only GPU) |
| `sys_cmds.rs` watch loop + NDJSON | controller | streaming | No loop/streaming code exists in-repo — use RESEARCH.md Pattern 6 (`\x1Bc`, 500 ms, one JSON line per tick) |
| `cache.rs` TTL/refresh semantics | utility | file-I/O | Envelope shape has an analog (`HelperEnvelope`); TTL/stale-with-age/refresh-bypass logic does not — use RESEARCH.md Pattern 5 |

## Metadata

**Analog search scope:** `crates/` (all 10 `.rs` files) + git-tracked check (`git ls-files` — all analog paths above are tracked; no mirror paths emitted)
**Files scanned:** 10 source files, read in full: `archel-core/{lib,error,i18n,privileged}.rs`, `archel/{main,cmds/mod}.rs`, `archel-privileged/main.rs`, `fr.json`, both `Cargo.toml`s
**Pattern extraction date:** 2026-09-30
