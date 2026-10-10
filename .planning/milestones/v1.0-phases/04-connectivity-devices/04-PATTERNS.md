# Phase 4: Connectivity & Devices - Pattern Map

**Mapped:** 2026-10-06
**Files analyzed:** 14 (5 new core modules, 4 new CLI groups, 5 modified files)
**Analogs found:** 12 / 14 (2 partial — no async/D-Bus code exists yet)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/archel-core/src/network.rs` (new) | service | request-response (zbus) + CRUD (nmcli/ping/dig subprocess) | `crates/archel-core/src/packages.rs` (`run_capped_query`, lines 160-211) + `updates.rs` (`run_checker`, lines 175-200) | role-match |
| `crates/archel-core/src/proxy.rs` (new) | service | CRUD (gsettings subprocess) | `crates/archel-core/src/updates.rs` (`run_checker`, lines 175-200) | role-match |
| `crates/archel-core/src/bluetooth.rs` (new) | service | event-driven (bluer discovery stream) + request-response | `crates/archel-core/src/system.rs` (sampler ownership) + `privileged.rs` (`collect_streaming`, lines 196-207) | partial |
| `crates/archel-core/src/audio.rs` (new) | service | CRUD + transform (wpctl spawn → parse → name→ID resolve) | `crates/archel-core/src/packages.rs` (`parse_search_output` + `run_capped_query`) | role-match |
| `crates/archel-core/src/power.rs` (new) | service | CRUD (PPD D-Bus probe → sysfs fallback) + transform | `crates/archel-core/src/system.rs` + `sensors.rs` (sysfs hwmon reads) | role-match |
| `crates/archel/src/cmds/net_cmds.rs` (new) | command group | request-response | `crates/archel/src/cmds/sys_cmds.rs` + `pkg_cmds.rs` | exact |
| `crates/archel/src/cmds/bt_cmds.rs` (new) | command group | request-response + event-driven (scan stream) | `crates/archel/src/cmds/pkg_cmds.rs` (`run_search` + `run_aur` confirm flow) | exact |
| `crates/archel/src/cmds/audio_cmds.rs` (new) | command group | request-response | `crates/archel/src/cmds/sys_cmds.rs` | exact |
| `crates/archel/src/cmds/power_cmds.rs` (new) | command group | request-response | `crates/archel/src/cmds/sys_cmds.rs` (`run_power`, lines 118-132) | exact |
| `crates/archel-core/src/lib.rs` (mod) | config (module registry) | — | self, lines 11-19 | exact |
| `crates/archel-core/src/error.rs` (mod) | error taxonomy | — | self, lines 24-93 + `message()` 116-156 | exact |
| `crates/archel-core/locales/fr.json` (mod) | i18n catalog | — | self (`sys`, `pkg` sections) | exact |
| `crates/archel/src/cmds/mod.rs` (mod) | route (clap tree) | — | self, lines 11-62 + 160-220 | exact |
| `crates/archel-core/Cargo.toml` (mod) | config (deps) | — | self + Phase 3 `alpm` pin line | exact |

All analog paths verified git-tracked via `git ls-files` 2026-10-06.

## Pattern Assignments

### `crates/archel-core/src/network.rs` (service, request-response + CRUD)

**Analog:** `crates/archel-core/src/packages.rs` (`run_capped_query`, lines 166-199) + `crates/archel-core/src/updates.rs` (`run_checker`, lines 175-200; `map_update_exit`, lines 160-168)

**Imports pattern** (packages.rs lines 8-12; network.rs adds tokio/zbus — first async in core):
```rust
use std::time::Duration;
use serde::Serialize;
use crate::error::ArchelError;
```

**Core spawn pattern** — copy `run_capped_query` verbatim shape for every `nmcli`/`ping`/`dig` call (packages.rs lines 166-199):
```rust
fn run_capped_query(
    tool: &str,
    args: &[&str],
    missing: impl FnOnce(String) -> ArchelError,
) -> Result<String, ArchelError> {
    let binary =
        which::which(tool).map_err(|err| missing(format!("{tool} ({err})")))?;
    // `.unchecked()` is load-bearing: without it duct turns every
    // non-zero exit (including pacman's exit-1 no-match) into an IO
    // error here, and the exit-code map below becomes unreachable.
    let handle = duct::cmd(&binary, args)
        .unchecked()
        .env("LC_ALL", "C")
        .stdout_capture()
        .stderr_capture()
        .start()
        .map_err(|err| ArchelError::Internal(format!("cannot run '{tool}': {err}")))?;
    let output = match handle.wait_timeout(QUERY_TIMEOUT).map_err(|err| {
        ArchelError::Internal(format!("cannot wait on '{tool}': {err}"))
    })? {
        Some(output) => output.clone(),
        None => {
            let _ = handle.kill();
            return Err(ArchelError::Internal(format!(
                "'{tool}' exceeded the query timeout"
            )));
        }
    };
    // Log-only capture of query stderr (kept for diagnostics; the user
    // only ever sees catalog French, never this raw text).
    let _stderr_for_logs = String::from_utf8_lossy(&output.stderr);
    map_query_exit(tool, output.status.code())?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
```
Rules inherited: `which` gate first with caller-chosen typed missing variant; `.unchecked()` + `.env("LC_ALL", "C")` on every parse-bound spawn (nmcli speaks French on this machine — Pitfall 4); `wait_timeout` + `kill` on timeout (use `QUERY_TIMEOUT`-style const, 15 s; self-tests want faster per-hop budgets — see RESEARCH rulings); stderr into `_stderr_for_logs` only.

**Exit-code map pattern** — copy `map_update_exit` matrix shape (updates.rs lines 160-168):
```rust
pub fn map_update_exit(tool: &str, code: Option<i32>) -> Result<(), ArchelError> {
    match (tool, code) {
        (_, Some(0)) => Ok(()),
        ("checkupdates", Some(2)) => Ok(()),
        _ => Err(ArchelError::Internal(format!(
            "{tool} refresh failed with exit code {code:?}"
        ))),
    }
}
```
Network analogue: `ping`/`dig`/`nmcli` each get a `map_*_exit` pure fn unit-tested with mocked codes (exit 0/1/2/None matrix), never default-ok.

**Parse pattern** — copy `parse_search_output` line discipline (packages.rs lines 138+): split lines, skip blanks/indented/malformed, read only needed fields, normalize source tokens (`aur` → `AUR` precedent for e.g. NM state strings). Pure fns over `&str` fixtures incl. FR-locale samples.

**Async/D-Bus note:** no zbus/tokio code exists in the repo — the `#[proxy]` macro shape and `#[tokio::main]` placement come from RESEARCH.md Code Examples, not from an analog. Keep subprocess helpers sync; expose `async fn`s for D-Bus paths; single `#[tokio::main]` at the binary (Pitfall 6); never `block_on` inside core.

---

### `crates/archel-core/src/proxy.rs` (service, CRUD)

**Analog:** `crates/archel-core/src/updates.rs` (`run_checker`, lines 175-200; `checker_missing_hint`, lines 233-239)

Same `run_capped_query` spawn shape for `gsettings get/set` (LC_ALL=C still required — gsettings output localizes). Mode transitions (`none`/`manual`/`auto`) map to a pure argv-builder fn unit-tested without spawning (mirror `pkg_helper_argv` purity in privileged.rs lines 161-165 — verb first, values verbatim, never through a shell). Status one-liner + `export` hint string built from catalog keys, mirroring `checker_missing_hint` (updates.rs lines 233-239):
```rust
pub fn checker_missing_hint(tool: &str) -> String {
    if tool == CHECKUPDATES_TOOL {
        rust_i18n::t!("updates.checker_missing_checkupdates").to_string()
    } else {
        rust_i18n::t!("updates.checker_missing_generic", tool = tool).to_string()
    }
}
```
Password (`--user/--password`) never in argv snapshots or `--json` (see Shared Patterns / Secrets).

---

### `crates/archel-core/src/bluetooth.rs` (service, event-driven)

**Analog:** `crates/archel-core/src/system.rs` (sampler ownership header, lines 1-28) + `crates/archel-core/src/privileged.rs` (`collect_streaming`, lines 196-207)

**Module doc pattern** — copy the ownership header style (system.rs lines 1-21): what the module owns, the sampling/streaming contract, what the binary must never touch directly (here: binary never touches bluer `Session`/`Adapter` — it calls scan/pair/connect fns).

**Streaming pattern** — discovery events accumulate through the `collect_streaming` generic-reader shape (privileged.rs lines 196-207):
```rust
fn collect_streaming<R: std::io::BufRead>(
    reader: R,
    on_line: &mut dyn FnMut(&str),
) -> String {
    let mut transcript = String::new();
    for line in reader.lines().map_while(Result::ok) {
        on_line(&line);
        transcript.push_str(&line);
        transcript.push('\n');
    }
    transcript
}
```
Adapt to bluer's async event stream: callback per discovered device into a live list (scan-then-pick, D-06), generic over the event source so tests feed mock events without hardware.

**Pairing-confirm pattern** — the PIN-confirm callback reuses `confirm_action` (packages.rs lines 484-496, see Shared Patterns): surface code, ask `[o/N]`, never auto-accept (D-05). Daemon-absent path is the typed `BluetoothUnavailable` with enable-hint, mirroring `UpdatesCheckerMissing` degrade shape (updates.rs lines 204-212 — `refresh_half` maps typed missing to a degrade value, never a crash).

**Risk flag:** bluer 0.17 `Session`/`Adapter`/agent-callback API shape has NO repo analog — Wave 0 spike against bluer docs first (RESEARCH A2, highest-risk surface).

---

### `crates/archel-core/src/audio.rs` (service, CRUD + transform)

**Analog:** `crates/archel-core/src/packages.rs` (`run_capped_query` lines 166-199 + `parse_search_output` line discipline + `PackageCard` composer)

Closest role+flow match in the repo: `wpctl status` parse ≡ `pacman -Ss` parse (locale-pinned spawn, header-line field extraction, skip garbage); `wpctl inspect <id>` name map ≡ `package_card` composer (one record assembled from a detail query); `set-default` ≡ mutation verb. Name→ID resolution is a pure fn over parsed fixtures (IDs renumber — never match by ID across runs). Missing `wpctl` is a typed error with install hint (YayMissing precedent, aur.rs lines 117-120). Unit tests with multi-locale `wpctl status` fixtures + insta snapshot on the device-list JSON (sys_cmds `schema_sys_*` precedent).

---

### `crates/archel-core/src/power.rs` (service, CRUD + transform)

**Analog:** `crates/archel-core/src/system.rs` (sampler module, lines 1-60: consts + Serialize snapshots) + `sensors.rs` (sysfs hwmon reads, N/A degrade)

Snapshot struct pattern (system.rs lines 40-60):
```rust
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CpuCore {
    pub id: usize,
    pub name: String,
    pub usage_pct: f32,
    pub freq_mhz: u64,
}
```
Power analogue: `{ profile: String, backend: "power-profiles-daemon" | "cpufreq", note: Option<String> }` — Serialize directly = the `--json` schema (bin-calls-core invariant). Governor map (`performance→performance`, `balanced→powersave`, `power-saver→powersave`) is a pure fn unit-tested on fixtures. Sysfs reads (`scaling_available_governors`, `scaling_governor`) copy the sensors.rs hwmon discipline: read file, parse, failure → `SensorUnavailable`-style typed error / `None` field with honest N/A, never zero-fill (D-10 precedent). Root-owned writes → typed error-with-hint (`PolkitDenied`-family, NOT `AuthFailed` which is helper-scoped) — no helper extension in v1.

---

### `crates/archel/src/cmds/net_cmds.rs` (command group, request-response)

**Analog:** `crates/archel/src/cmds/sys_cmds.rs` (run_* + render_* + emit) and `crates/archel/src/cmds/pkg_cmds.rs` (`run_install` confirm-then-stream flow, lines 111-137)

**Run-fn pattern** — copy `run_cpu` tick shape for reads (sys_cmds.rs lines 37-51):
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
Net analogue: `status`/`dns show`/`proxy` bare/`vpn list`/`test` = one-shot (no `--watch` — network state is not a 500 ms tick; drop the loop, keep `if json { println!(payload) } else { print!(table) }`).

**Mutation pattern** — copy `run_install` preview → confirm → stream (pkg_cmds.rs lines 111-137): DNS set/reset and VPN up/down show old→new preview, call `confirm_action`, then execute; `--json` accumulates silently and emits one `mutation_report_value` object (pkg_cmds.rs lines 433-447):
```rust
fn mutation_report_value(
    action: &str,
    packages: &[String],
    transcript: String,
    note: Option<String>,
) -> serde_json::Value {
    let lines: Vec<String> = transcript.lines().map(str::to_string).collect();
    serde_json::json!({
        "action": action,
        "packages": packages,
        "ok": true,
        "transcript": lines,
        "note": note,
    })
}
```

**Render pattern** — copy `render_label_table` + `pad_right` char-count padding (sys_cmds.rs lines 284-295, 338-341):
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
```
Labels/units from core catalog fns only — the cmds file owns zero French copy. Insta-pin every `--json` schema (`insta::assert_json_snapshot!`, sys_cmds.rs lines 532-564).

---

### `crates/archel/src/cmds/bt_cmds.rs` (command group, request-response + streaming)

**Analog:** `crates/archel/src/cmds/pkg_cmds.rs` (`run_search` lines 30-41 + `run_aur` review→confirm→build lines 282-312 + `run_aur_confirmed` injectable-closure gate lines 319-326)

`bt scan` ≡ `run_search` (live list, query-echo title, honest empty table). `bt pair` ≡ `run_aur` (review device row → TTY confirm → connect closure), copying the injectable-build-closure gate so the abort path is unit-covered headless with a spawn probe:
```rust
fn run_aur_confirmed(
    name: &str,
    json: bool,
    build: impl FnOnce(&mut dyn FnMut(&str)) -> Result<String, ArchelError>,
) -> Result<(), ArchelError> {
    archel_core::packages::confirm_action(&archel_core::aur::confirm_aur_prompt(name))?;
    run_aur_stream(name, json, build)
}
```
Scan-then-pick (D-06): connect takes a picked index/MAC from the just-printed live list, never a free-form MAC arg — validate the pick against the scanned set client-side (mirror `validate_package_names` fast-fail gate before any spawn).

---

### `crates/archel/src/cmds/audio_cmds.rs` (command group, request-response)

**Analog:** `crates/archel/src/cmds/sys_cmds.rs` (`run_gpu`/`render_gpu_table` + N/A discipline, lines 87-100, 229-237)

`audio list` ≡ `run_gpu` one-shot; device table shows stable node names with N/A rows for absent hardware (`format_opt_name`/`format_opt_gib`, sys_cmds.rs lines 307-315). `set-output`/`set-input <name>` ≡ mutation report shape (no pkexec streaming — `wpctl set-default` runs as the user; still confirm-gated per phase rule, TTY first).

---

### `crates/archel/src/cmds/power_cmds.rs` (command group, request-response)

**Analog:** `crates/archel/src/cmds/sys_cmds.rs` (`run_power`, lines 118-132)

Copy `run_power` exactly (handle-reuse loop collapses to one-shot; `power set` is not a watch tick). **Name-collision warning for planner:** `sys power` (watts readout) already exists — `power set/status` must not register a confusing duplicate; keep `sys power` read-only watts and `power` as the profile switcher, with distinct catalog titles.

---

### Modified files

**`lib.rs`** — add `pub mod audio; pub mod bluetooth; pub mod network; pub mod power; pub mod proxy;` to the decl block (lines 11-19), alphabetical, one line per module. No other change.

**`error.rs`** — new variants follow the existing shape (lines 24-93): struct variants carry log-only payloads (`{ plugin }`, `{ name }`), `#[error("…english…")]` stays English/log-only, `message()` gains one catalog arm per variant (lines 116-156), `every_variant()` test (lines 163-194) gains each new variant so the non-empty-French test covers them. RESEARCH §Architecture lists the exact set: `NetworkUnavailable`, `ProfileNotFound`, `DnsInvalid`, `VpnPluginMissing { plugin }`, `VpnImportFailed`, `BluetoothUnavailable`, `DeviceNotFound`, `PairingRejected`/`PairingTimeout`, `AudioDeviceNotFound`, `PowerBackendMissing`, `PolkitDenied { action }` (NOT `AuthFailed` — that enum is helper-scoped). `NeedsTty` + `ConfirmationAborted` are reused unchanged.

**`fr.json`** — new `net`/`bt`/`audio`/`power` sections mirroring the `sys`/`pkg` key style (`<domain>.<label>`, `%{var}` interpolation, `[o/N]` confirm prompts, `N/A` via existing `sys.unavailable`). Binary and core both resolve through `t!` — never literals at call sites.

**`cmds/mod.rs`** — new `Net {…}` / `Bt {…}` / `Audio {…}` / `Power {…}` top-level groups beside `Sys`/`Pkg` (lines 46-54 pattern), EN subcommand/flag names, `run()` match arms returning `Result<(), ArchelError>` (lines 193-210), completions test word-list extended with the new verbs (lines 253-274) so clap-tree drift fails the test.

**`archel-core/Cargo.toml`** — append `tokio` (features `rt-multi-thread,time,macros`), `zbus`, `bluer`, `rpassword` beside the Phase 3 `alpm` pin line (which carries the `checkver` comment precedent — keep version-source comments on new pins). Verify `rpassword` exact version via `cargo search` at plan time (RESEARCH A5).

---

## Shared Patterns

### Error rendering (applies to: all new files)
**Source:** `crates/archel/src/main.rs` lines 14-22 + `error.rs` lines 114-156
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
Core returns `Result<_, ArchelError>` on every fallible path (never panic, never default-ok); `message()` is the sole terminal renderer (French catalog); `Display` stays English/log-only with raw detail (T-1-02). No `unwrap`/`expect` on failure paths; edge inputs render without panicking (error.rs test lines 253-264).

### Confirmation gate (applies to: every write — DNS set/reset, proxy set, VPN up/down/import, BT pair/connect, audio switch, power set)
**Source:** `crates/archel-core/src/packages.rs` lines 484-496 + prompt builders 499-511
```rust
pub fn confirm_action(prompt: &str) -> Result<(), ArchelError> {
    use std::io::{IsTerminal, Write};
    if !std::io::stdin().is_terminal() {
        return Err(ArchelError::NeedsTty);
    }
    eprint!("{prompt} ");
    let _ = std::io::stderr().flush();
    let mut line = String::new();
    match std::io::stdin().read_line(&mut line) {
        Ok(_) if parse_confirm_answer(&line) => Ok(()),
        _ => Err(ArchelError::ConfirmationAborted),
    }
}
```
TTY check before prompt (piped/batch `--json` refuses with `NeedsTty`); prompt on stderr with `[o/N]`, only `o`/`oui`-prefix accepts (test line 809); prompt strings name every affected target and come from the catalog (`confirm_install_prompt` pattern — `packages.join(" ")` interpolation). Abort path spawns nothing (injectable-closure test precedent, pkg_cmds.rs lines 848-875).

### Input validation gate (applies to: DNS addrs, MAC picks, proxy host/port, import paths, profile names)
**Source:** `crates/archel-core/src/packages.rs` lines 360-384
```rust
pub fn validate_package_names(packages: &[String]) -> Result<(), ArchelError> {
    if packages.is_empty() {
        return Err(ArchelError::InvalidPackageName {
            name: String::new(),
        });
    }
    for name in packages {
        if !is_valid_package_name(name) {
            return Err(ArchelError::InvalidPackageName { name: name.clone() });
        }
    }
    Ok(())
}
```
Client-side strict gate failing fast to a typed variant before any spawn/prompt; server/helper re-validates (never trusted). Phase 4 analogues: IPv4-only DNS (reject v6/hostnames), strict `XX:…` MAC, proxy host/port range, import path confined (exists, is-file, `.ovpn`/`.conf`, no `/proc|/sys|/dev`). Untrusted display data (SSID, device names) shown verbatim, never evaled.

### Spawn discipline (applies to: all `nmcli`/`gsettings`/`wpctl`/`ping`/`dig` subprocesses)
**Source:** `crates/archel-core/src/packages.rs` lines 176-182 + `privileged.rs` lines 229-236
`which::which` gate → typed missing variant; `duct` with absolute path, exact argv (never shell); `.unchecked()` + `.env("LC_ALL", "C")` on every parse-bound spawn; `stdout_capture` + `stderr_capture`; `wait_timeout` + `kill`; stderr log-only. Streaming variant (VPN import transcript, BT scan): `.reader()` + drain-while-running (privileged.rs `run_privileged_streaming`, never `run()` with both captures — Pitfall 7 deadlock).

### JSON + snapshots (applies to: every new command)
**Source:** `crates/archel/src/cmds/sys_cmds.rs` lines 42-45 (encode), 532-564 (insta pins)
Structs derive `Serialize` with snake_case English fields = the v1 schema; encode errors map to `ArchelError::Internal`; one complete object per `println!` (NDJSON-safe); `insta::assert_json_snapshot!` per schema; secrets redacted from fixtures and `Display` (RESEARCH anti-patterns).

### i18n (applies to: every user-visible string)
**Source:** `crates/archel-core/src/lib.rs` line 23 + `i18n.rs` + `fr.json`
`rust_i18n::i18n!("locales", fallback = "fr")`; binary pins `set_language("fr")` at `run()` entry (mod.rs lines 166-168); core owns `*_labels()`/`*_title()` accessors so cmds files contain no French; tests call `set_language("fr")` before rendering assertions.

## No Analog Found

| File / Concern | Role | Data Flow | Reason |
|---|---|---|---|
| zbus `#[proxy]` NM/PPD proxies + `Connection.Update` write shape | service | request-response (D-Bus) | No D-Bus code exists in the repo yet — use RESEARCH.md "Code Examples" NM proxy sketch + Wave 0 live-bus spike (member names + auth-error strings ASSUMED, A1) |
| bluer 0.17 `Session`/`Adapter`/agent-callback shape + `#[tokio::main]` placement | service | event-driven | First async dependency in core — RESEARCH Pitfall 6 rules apply (sync subprocess helpers, `async fn`s in core, one runtime at binary); spike agent signature before CLI wiring (A2) |
| `networkmanager` 0.5.0 wrapper crate | — | — | SUS legitimacy verdict (630/wk) — planner inserts `checkpoint:human-verify`; default stays zbus-direct |

## Metadata

**Analog search scope:** `crates/archel-core/src/` (lib, error, privileged, updates, system, sensors, packages, aur) + `crates/archel/src/` (main, cmds/mod, sys_cmds, pkg_cmds) + `locales/fr.json` + workspace Cargo manifests
**Files scanned:** ~16 source files (all git-tracked; full reads of lib/error/updates/privileged/mod/sys_cmds/pkg_cmds/main; targeted reads of packages/aur/system; grep for spawn/confirm/validate sites)
**Strong matches stopped at:** 5 (privileged spawn/parse, error/i18n catalog, cmds clap wiring, system.rs samplers, packages.rs LC_ALL=C parsing + card composer) per early-stopping rule; aur.rs fetch/validate gates and pkg_cmds confirm/redirect flows added as mutation-flow analogs
**Pattern extraction date:** 2026-10-06
