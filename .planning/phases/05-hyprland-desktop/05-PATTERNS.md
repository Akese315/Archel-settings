# Phase 5: Hyprland Desktop - Pattern Map

**Mapped:** 2026-10-07
**Files analyzed:** 12 (7 new, 5 modified)
**Analogs found:** 12 / 12 (all git-tracked; verified via `git ls-files`)

Every Phase 5 file has a strong same-repo analog. The Phase 4 idiom (subprocess discipline, plan→argv purity, confirm-then-execute, typed errors, fixture-pure tests, insta snapshots) is directly copyable — RESEARCH.md's discipline claims were verified against the live files cited below.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/archel-core/src/hyprland.rs` (or `hyprland/mod.rs`) | service (subprocess wrapper + snippet writer) | request-response, file-I/O | `crates/archel-core/src/proxy.rs` | exact (which-gate + LC_ALL + wait_timeout subprocess runner over a user-session CLI store) |
| `crates/archel-core/src/hyprland/monitors.rs` | model + plan (pure) | transform (JSON→plan→argv) | `crates/archel-core/src/proxy.rs` (`ProxyPlan` + `proxy_write_argv`) + `power.rs` (`cpufreq_set_with` confirm-inject) | role-match (plan/argv purity) |
| `crates/archel-core/src/hyprland/binds.rs` | utility (pure text render/parse) | transform | `crates/archel-core/src/audio.rs` (`parse_inspect_property` pure-over-text parsers) | role-match |
| `crates/archel-core/src/hyprland/wallpaper.rs` | service (conf render + process lifecycle) | file-I/O, batch spawn | `crates/archel-core/src/proxy.rs` (`validate_*` + argv builder) + `audio.rs` (`run_wpctl` spawn) | role-match |
| `crates/archel-core/src/error.rs` (MODIFY) | model (typed errors) | — | same file: `BluetoothUnavailable` variant (error.rs:92-99) | exact (same hint-bearing `{ detail }` family) |
| `crates/archel-core/src/lib.rs` (MODIFY) | config (module decl) | — | same file (lib.rs:11-24) | exact |
| `crates/archel-core/locales/fr.json` (MODIFY) | config (i18n catalog) | — | same file (existing `net.proxy_*`, `power.*` keys) | exact |
| `crates/archel-core/src/packages.rs` (MODIFY — optional) | utility (`confirm_action_timed`) | — | same file: `confirm_action` (packages.rs:484-496) | exact (add timed sibling, reuse `parse_confirm_answer`) |
| `crates/archel/src/cmds/appearance_cmds.rs` | controller (CLI renderer) | request-response | `crates/archel/src/cmds/net_cmds.rs` (`run_proxy_plan_confirmed`) | exact |
| `crates/archel/src/cmds/display_cmds.rs` | controller | request-response | `crates/archel/src/cmds/net_cmds.rs` + RESEARCH.md 15 s-confirm sketch | exact + one new seam |
| `crates/archel/src/cmds/input_cmds.rs` | controller | request-response | `crates/archel/src/cmds/net_cmds.rs` / `audio_cmds.rs` | exact |
| `crates/archel/src/cmds/mod.rs` (MODIFY) | config (clap enum + dispatch) | — | same file (`PowerAction`/`AudioAction` arms, mod.rs:291-324, 451-459) | exact |

## Pattern Assignments

### `crates/archel-core/src/hyprland.rs` (service, request-response + file-I/O)

**Analog:** `crates/archel-core/src/proxy.rs` (module header + runner), `crates/archel-core/src/audio.rs` (per-domain timeout const)

**Module header + imports pattern** (proxy.rs:1-24) — doc header naming requirements, spawn-discipline pledge, typed-error pledge:
```rust
//! Session-layer proxy state via gsettings (NET-04 tracer).
//! ...
//! Spawn discipline mirrors [`crate::packages`] and
//! [`crate::network`]: which-gate, exact argv never through a shell,
//! `LC_ALL=C`, `wait_timeout` plus kill. A missing `gsettings`
//! binary maps to the typed [`crate::ArchelError::NetworkUnavailable`]
//! (never a panic), mirroring the `checker_missing_hint` pattern in
//! [`crate::updates`]: the user sees the catalog hint, logs keep the
//! detail.

use serde::Serialize;

use crate::error::ArchelError;
use crate::packages::QUERY_TIMEOUT;
```
For `hyprland.rs`: replace "gsettings" with "hyprctl"; gate additionally on `$HYPRLAND_INSTANCE_SIGNATURE` (unset → `ArchelError::HyprlandUnavailable`). `QUERY_TIMEOUT` is `Duration::from_secs(15)` re-exported from packages.rs:16 — reuse it. `audio.rs` shows a per-domain const alternative (`WPCTL_TIMEOUT`, audio.rs:220 used at the call site).

**Core runner pattern** (proxy.rs:109-137 — copy nearly verbatim, rename + re-map error variant):
```rust
fn run_gsettings_query(tool: &str, args: &[&str]) -> Result<String, ArchelError> {
    let binary = which::which(tool)
        .map_err(|err| ArchelError::network_unavailable(format!("{tool} ({err})")))?;
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
    let _stderr_for_logs = String::from_utf8_lossy(&output.stderr);
    match output.status.code() {
        Some(0) => Ok(String::from_utf8_lossy(&output.stdout).into_owned()),
        code => Err(ArchelError::network_unavailable(format!(
            "{tool} query failed with exit code {code:?}"
        ))),
    }
}
```
Phase 5 additions to this shape (Pitfall 1): for `eval` calls, after rc==0 also fail when stdout starts with `error:` — check rc AND stdout. The `tool: &str` injectable-parameter trick (proxy.rs:69 `proxy_status_with_tool("gsettings")` → test at proxy.rs:751-758 asserts missing binary maps to the typed variant headless) is the established mock story — keep `run_hyprctl` parameterized or closure-injected the same way.

**Read-back pattern** (power.rs:302-313 `ppd_set` → `ppd_status_via`; audio read-back before ok): every mutating fn re-reads via `getoption -j` after apply, never returns silent ok.

**Snippet writer (D-08):** no direct analog for atomic tmp+rename in the read files, but the file-write discipline comes from power.rs:206-226 `write_governors_at`:
```rust
pub fn write_governors_at(base: &Path, governor: &str) -> Result<Vec<PathBuf>, ArchelError> {
    let mut written = Vec::new();
    for policy in policy_dirs(base)? {
        let target = policy.join("scaling_governor");
        match std::fs::write(&target, format!("{governor}\n")) {
            Ok(()) => written.push(target),
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                return Err(ArchelError::PolkitDenied {
                    action: format!("power set ({})", target.display()),
                });
            }
            Err(err) => {
                return Err(ArchelError::Internal(format!(
                    "cannot write '{}': {err}", target.display()
                )));
            }
        }
    }
    Ok(written)
}
```
Adapt: `PermissionDenied` on user-owned `~/.config/hypr/` files → `Internal`/`HyprlandUnavailable { detail }` (no privilege this phase); add the timestamped-backup + tmp+rename steps the repo does not yet have (new code — RESEARCH.md Pitfall 5 is the spec: idempotent line insertion, backup before EVERY write, rotation N=5).

---

### `crates/archel-core/src/hyprland/monitors.rs` (model + pure plan, transform)

**Analog:** `crates/archel-core/src/proxy.rs:251-471` (`ProxyPlan` + validators + `proxy_write_argv`) for the plan shape; `power.rs:232-241` for the injectable-confirm gate.

**Plan struct pattern** (proxy.rs:251-259, minus the secret-redaction Debug unless needed):
```rust
/// A validated proxy transition, planned read-only before any prompt.
pub struct ProxyPlan {
    pub mode: String,
    pub old_mode: String,
    ...
}
```
→ `MonitorPlan { pub apply: Vec<String>, pub rollback: Vec<String> }` built by a **pure** `plan_monitor_change(current: &Monitor, want: &MonitorWant)`; the current-monitor snapshot for rollback comes from a `Monitor` model deserialized with serde from `hyprctl monitors -j` (RESEARCH Pattern 3).

**Pure argv/statement builder pattern** (proxy.rs:429-471) — plan → exact command strings, unit-tested without spawning:
```rust
pub fn proxy_write_argv(plan: &ProxyPlan) -> Vec<Vec<String>> {
    let set = |key_path: &[&str], value: String| -> Vec<String> { ... };
    let mut steps = vec![set(&["mode"], gsettings_string(&plan.mode))];
    match plan.mode.as_str() { ... }
    steps
}
```
Tested golden-style at proxy.rs:790-843 (`assert!(flat.contains(needle))` per expected command line). Apply identically to `MonitorPlan.apply` / `.rollback` eval statements (`["eval", lua]` pairs).

**Validator pattern** (proxy.rs:310-353) — client-side validation before any prompt/spawn, typed error carrying the offending input:
```rust
pub fn validate_proxy_host(host: &str) -> Result<String, ArchelError> {
    let trimmed = host.trim();
    if trimmed.is_empty() {
        return Err(ArchelError::ProxyInvalid { detail: "empty proxy host".to_string() });
    }
    if trimmed.chars().any(|c| c.is_whitespace() || c.is_control() || c == '\'' || c == '"') {
        return Err(ArchelError::ProxyInvalid { detail: format!("rejected proxy host {trimmed:?}") });
    }
    Ok(trimmed.to_string())
}
```
→ scale-divisibility / overlap / mode-vocabulary validators, each returning a typed validation error (reuse the `ProxyInvalid`-style `{ detail }` shape; a `DesktopInvalid { detail }` or reuse of `Internal`-adjacent variant is planner discretion — mirror error.rs conventions).

**Machine-generated-value quoting guard** (proxy.rs:404-410) — debug_assert on the post-validation invariant; apply identically to `lua_quote`:
```rust
fn gsettings_string(value: &str) -> String {
    debug_assert!(
        !value.contains('\''),
        "gsettings values must be quote-free after validation"
    );
    format!("'{value}'")
}
```

**Injectable-confirm gate shape** (power.rs:232-241) — `confirm` runs before `writer`, so abort structurally never mutates; makes the rollback/abort path headless-testable:
```rust
pub fn cpufreq_set_with(
    base: &Path,
    profile: PowerProfile,
    confirm: impl FnOnce(&str) -> Result<(), ArchelError>,
    writer: impl FnOnce(&str) -> Result<(), ArchelError>,
) -> Result<PowerStatus, ArchelError> {
    confirm(&confirm_set_prompt(profile))?;
    writer(profile_to_governor(profile))?;
    cpufreq_status_at(base)
}
```
→ `plan_monitor_change` + `apply_monitor_plan(plan, confirm, apply_fn, rollback_fn)`; the abort-path test at power.rs:477-501 (`Cell::new(false)` writer spy, "abort must write nothing") is the template for "declined/timed-out confirm re-applies rollback and nothing else".

**15 s timed confirm:** no repo analog (`grep` confirms no `recv_timeout` / timed confirm exists) — use the RESEARCH.md sketch (thread over stdin + `mpsc::recv_timeout`), but land it beside `confirm_action` in packages.rs and reuse `parse_confirm_answer` (packages.rs:475-478):
```rust
pub fn parse_confirm_answer(input: &str) -> bool {
    let lowered = input.trim().to_lowercase();
    lowered == "o" || lowered == "ou" || lowered.starts_with("oui")
}
```
and keep the TTY gate verbatim from packages.rs:484-488 (`if !std::io::stdin().is_terminal() { return Err(ArchelError::NeedsTty); }`). Distinguish timeout from refusal with a distinct error path so the binary can always run rollback (RESEARCH Pattern 3).

---

### `crates/archel-core/src/hyprland/binds.rs` (utility, transform)

**Analog:** `crates/archel-core/src/audio.rs:144-201` (pure text parsers over captured output, garbage→`None` never panic), `proxy.rs:175-191` (`parse_ignore_hosts` line-splitting parser over fixture strings).

**Pure parser pattern** (audio.rs:184-201):
```rust
/// Extracts one quoted `key = "value"` property from `wpctl inspect`
/// output. Pure over the text so fixtures cover it without PipeWire.
fn parse_inspect_property(stdout: &str, key: &str) -> Option<String> {
    for line in stdout.lines() {
        let trimmed = line.trim().trim_start_matches("* ").trim();
        let Some((name, value)) = trimmed.split_once('=') else { continue; };
        ...
    }
    None
}
```
→ snippet `hl.bind(...)` line parse + render, modmask codec over `binds -j` serde structs. Line-discipline comment at audio.rs:142-143 ("skip blanks and malformed lines, mirroring `parse_search_output`") applies verbatim.

**Exact-match dedup:** string-level line filtering over the app-owned snippet only (D-06) — precedent for `contains`/line-equality idempotence: RESEARCH.md "Plain string ops" row; no repo analog needed beyond proxy.rs test-style `assert_eq!` on rendered text (proxy.rs:790-843).

---

### `crates/archel-core/src/hyprland/wallpaper.rs` (service, file-I/O + batch spawn)

**Analog:** `crates/archel-core/src/proxy.rs` (validators + pure argv builder + idempotent-store-write flow) and `audio.rs:209-238` (`run_wpctl` for the `pkill`/`setsid` spawn legs).

- Path validation: copy the proxy.rs validator shape (`validate_pawpaper_path` → typed error naming the rejected path) — closest live sibling is the VPN import path gate: `ArchelError::VpnPathInvalid { path }` (error.rs:71-76: "missing, not a file, wrong extension, or under /proc, /sys, /dev ... nothing was spawned"). Add canonicalize + extension + reject `"`/`\`/newline/NUL per Security Domain.
- Conf-slice render: pure `render_hyprpaper_slice(...) -> String` tested golden-style (proxy.rs:790 precedent).
- Restart sequence: `run_*` spawn per audio.rs:209-238 shape for `pkill -x hyprpaper`, then `duct::cmd("setsid", &["hyprpaper", ...])` through the same runner (RESEARCH Pattern 4's "setsid argv shim to avoid a libc dep"). HYPRLAND_INSTANCE_SIGNATURE / XDG_RUNTIME_DIR env passthrough replaces the `.env("LC_ALL", "C")` line (keep LC_ALL too; both env() calls chain).
- Idempotent `source =` insertion: no repo analog — RESEARCH Pitfall 5 is the spec (grep line equality before append, tmp+rename, backup first).

---

### `crates/archel-core/src/error.rs` (MODIFY — add `HyprlandUnavailable`)

**Analog in-file:** `BluetoothUnavailable` family (error.rs:92-99) — exact shape to copy:
```rust
    /// The BlueZ daemon (`bluetooth.service`) answered no bt command:
    /// ... Carries log-only detail; the user sees the catalog
    /// enable hint, never this payload. ...
    #[error("bluetooth unavailable: {detail}")]
    BluetoothUnavailable { detail: String },
```
→ add `#[error("hyprland unavailable: {detail}")] HyprlandUnavailable { detail: String }` plus:
- friendly constructor (error.rs:199-205 style): `pub fn hyprland_unavailable(detail: impl Into<String>) -> Self { Self::HyprlandUnavailable { detail: detail.into() } }`
- `message()` arm (error.rs:292-294 style): `Self::HyprlandUnavailable { .. } => rust_i18n::t!("errors.hyprland_unavailable").to_string(),`
- register the variant in `every_variant()` (error.rs:337+) — the test at error.rs:412-422 (`every_variant_renders_non_empty_french`) then enforces the fr.json key automatically.
- If a distinct confirm-timeout error is added (RESEARCH Pattern 3), follow the `ConfirmationAborted` (error.rs:142-146) unit-variant + catalog-message pattern; `NeedsTty` (error.rs:147-152) is reused unchanged.

---

### `crates/archel-core/src/lib.rs` (MODIFY)

**Analog in-file** (lib.rs:11-24): add `pub mod hyprland;` alphabetically alongside `pub mod ...;` declarations. Nothing else — `ArchelError` re-export (line 26) and `rust_i18n::i18n!("locales", fallback = "fr")` (line 28) already cover the new module.

---

### `crates/archel-core/locales/fr.json` (MODIFY)

**Analog in-file:** existing `net.proxy_*` and `power.*` keys. Phase 5 needs (naming pattern `domain.key`, EN command names / FR display): `errors.hyprland_unavailable`, `appearance.*` / `display.*` / `input.*` titles, preview labels, confirm prompts, ok lines — every string the cmds renderers would otherwise own. Key collection convention: label-struct functions in core (see Shared Patterns → i18n).

---

### `crates/archel/src/cmds/appearance_cmds.rs` · `display_cmds.rs` · `input_cmds.rs` (controllers)

**Analog:** `crates/archel/src/cmds/net_cmds.rs` — the plan→preview→confirm→apply gate, lines 39-82:
```rust
/// Runs `archel net dns set`: validate, preview, confirm, then replace
/// the profile DNS and reactivate (NET-03, D-01/D-02).
pub fn run_dns_set(
    profile: Option<String>,
    servers: Vec<String>,
    json: bool,
) -> Result<(), ArchelError> {
    // Read-only planning first: validation + resolution + old-config
    // read run before any prompt, so typos fail fast with no TTY need.
    let plan = archel_core::network::dns::plan_dns_set(profile.as_deref(), &servers)?;
    run_dns_plan_confirmed(&plan, json, archel_core::network::dns::apply_dns_plan)
}

fn run_dns_plan_confirmed(
    plan: &archel_core::network::dns::DnsPlan,
    json: bool,
    apply: impl FnOnce(&archel_core::network::dns::DnsPlan) -> Result<DnsReport, ArchelError>,
) -> Result<(), ArchelError> {
    if !json {
        print!("{}", render_dns_plan_preview(plan));
    }
    archel_core::packages::confirm_action(
        &archel_core::network::dns::confirm_dns_prompt(plan),
    )?;
    let report = apply(plan)?;
    if json {
        println!("{}", encode_json(&report, "net dns")?);
    } else {
        print!("{}", render_dns_report_table(&report));
    }
    Ok(())
}
```
The proxy gate (net_cmds.rs:166-192) shows the label-table variant (`render_label_table(&core_proxy::proxy_preview_title(plan), &core_proxy::proxy_preview_rows(plan))` → `confirm_action(confirm_*_prompt)` → `apply` → encode/report). Per-domain copies of these helpers are the established convention (`encode_json` duplicated verbatim in net_cmds.rs:355, audio_cmds.rs:67, power_cmds.rs:45, bt_cmds.rs:117 — do NOT refactor into shared, follow the duplication).

- `display_cmds.rs` extends the gate with the 15 s re-confirm + rollback leg: `confirm_action_timed` (RESEARCH sketch) between apply and keep/rollback decision.
- Simple mutation (opacity/font/sensitivity set) may use the even-thinner audio shape (audio_cmds.rs:31-52): core fn does confirm+apply+read-back, bin prints ok line or `mutation_report_value` JSON (`json!({"action","device","ok":true,"transcript":[ok_line],"note":null})`, audio_cmds.rs:57-65 — rename fields per domain, EN keys).
- Table rendering: `std::fmt` widths only, no table crate; core owns ALL French copy (module-header doctrine, net_cmds.rs:1-9, audio_cmds.rs:1-12).
- `render_label_table` to copy: net_cmds.rs:376 (also pkg_cmds.rs:478, sys_cmds.rs:284).

**Insta snapshot pattern** (power_cmds.rs:150-161):
```rust
    // insta pins: the --json schemas in snake_case English keys. A
    ...
        insta::assert_json_snapshot!("power_status", cpufreq_status());
    ...
        insta::assert_json_snapshot!("power_set", report);
```
→ one `assert_json_snapshot!` per new `--json` surface (`appearance_wallpaper`, `display_monitors`, `input_keybinds`, …) with in-module fixture builders; snapshot files land in `crates/archel/src/cmds/snapshots/` (naming `archel__cmds__<module>__tests__<name>.snap`).

---

### `crates/archel/src/cmds/mod.rs` (MODIFY)

**Analog in-file:** `PowerAction` (mod.rs:310-324) + `AudioAction` (mod.rs:291-309) for enum shape; dispatch arms at mod.rs:451-459.

**Enum addition pattern** (mod.rs:314-324):
```rust
#[derive(Debug, Subcommand)]
enum PowerAction {
    /// Active profile plus backend provenance (read-only)
    Status,
    /// Switch the power profile (confirm-gated)
    Set {
        /// Target profile: performance, balanced or power-saver
        #[arg(value_parser = ["performance", "balanced", "power-saver"])]
        profile: String,
    },
}
```
→ add `enum AppearanceAction`, `DisplayAction`, `InputAction` + `Command::{Appearance, Display, Input} { #[command(subcommand)] action: ... }` variants (doc-comment each with requirement IDs, EN names), `mod appearance_cmds; mod display_cmds; mod input_cmds;` at mod.rs:11-17, and dispatch arms in `run()` following mod.rs:456-459.
**Mandatory:** extend the completions coverage test word list (mod.rs:498-547 `completions_cover_the_full_command_tree`) with every new subcommand word — drift fails there by design.

## Shared Patterns

### 1. Subprocess spawn discipline (which-gate + LC_ALL=C + wait_timeout + kill)
**Source:** `crates/archel-core/src/proxy.rs:109-137` (and `audio.rs:209-238`)
**Apply to:** every hyprctl/hyprpaper/pkill/gsettings call site in `hyprland*` modules. Copy verbatim, only the typed error variant changes per binary (`HyprlandUnavailable` for hyprctl/hyprpaper; gsettings font leg may reuse the `NetworkUnavailable`-mapped `run_gsettings_query` shape or re-point it at `HyprlandUnavailable` — planner picks one, but the missing-binary test proxy.rs:751-758 must be cloned).
**Phase 5 twist:** eval verbs fail on stdout prefix `error:` even at rc==0 (RESEARCH Pitfall 1) — extend the exit map.

### 2. Plan → validate → preview → confirm → apply → read-back
**Sources:** validators `proxy.rs:310-353`; pure argv `proxy.rs:429-471`; gate `net_cmds.rs:64-82` / `:166-192`; injectable-confirm `power.rs:232-241`; read-back `power.rs:302-313`; `confirm_action` `packages.rs:484-496`; `QUERY_TIMEOUT` `packages.rs:16`.
**Apply to:** all Phase 5 mutations (font set, blur/opacity set, monitor change, keybind add/edit/remove, sensitivity/scrolling set). Validation and old-state reads ALWAYS precede the prompt (net_cmds.rs:46-48 comment). Aborts map to `ConfirmationAborted` / `NeedsTty` — reuse, never re-implement.

### 3. Typed errors, catalog-only rendering, no-default-ok
**Source:** `crates/archel-core/src/error.rs` (variant + constructor + `message()` arm + `every_variant()` registration)
**Apply to:** `HyprlandUnavailable { detail }` and any new validation error. Display impl stays English/log-oriented; `message()` is the sole terminal string (error.rs:1-8 header).

### 4. i18n: core owns all French copy via label structs + prompt fns
**Source:** `crates/archel-core/src/power.rs:361-391` (`status_title`, `struct PowerLabels`, `power_labels()`, `confirm_set_prompt`, `set_ok_line`), `proxy.rs:559-630` (`*_preview_title/rows`, `confirm_*_prompt`, `*_report_title/rows`)
**Apply to:** every new cmds file — the binary owns zero French strings; collect labels in one core call (`DesktopLabels { ... }` style), keys in `crates/archel-core/locales/fr.json`, `rust_i18n::t!("domain.key", arg = ...)` with interpolation as in power.rs:385.

### 5. Test strategy: fixture-pure parses, injected binaries, abort-writes-nothing, headless gate
**Sources:**
- Pure-over-text parsers: `audio.rs:144-201`, `proxy.rs:141-191`
- Injected tool name for which-gate coverage: `proxy.rs:66-71` + test `proxy.rs:751-758`
- "Never spawn" fixture tests: network.rs vpn provenance tests (~network.rs:1887-1905) — `#[cfg(test)] mod … { // Headless fixtures only — never spawn nmcli ... }`, inline fixture consts (repo has no tests/ dir — keep that convention)
- Abort-path spies: `power.rs:477-501` (`Cell` writer spy) and `net_cmds.rs` injectable-apply gate
- Golden command-sequence assertions: `proxy.rs:790-843`
- JSON schema pins: `power_cmds.rs:150-161`, `audio_cmds.rs:210-241` (insta)
- Error catalog completeness: `error.rs:412-422` (`every_variant_renders_non_empty_french`)
**Commands:** per-task `cargo test -p archel-core --no-default-features hyprland`; per-wave `cargo test --workspace --no-default-features` (RESEARCH Validation Architecture — the `--no-default-features` headless gate must stay green).

### 6. Struct-serialization convention
**Source:** `power.rs:47-75`, `proxy.rs:43-59`
`#[derive(Debug, Clone, PartialEq, Eq, Serialize)]` on every `--json`-reachable struct; `#[serde(rename = "...")]` wire words on enums; field names = the v1 JSON schema (packages.rs:21-22 comment: snake_case EN keys, pinned by insta snapshots).

## No Analog Found

| File | Role | Data Flow | Reason → use RESEARCH.md instead |
|------|------|-----------|--------------------------------|
| `confirm_action_timed` (packages.rs addition) | utility | request-response | No timed-confirm exists in repo (verified by grep); RESEARCH.md "15 s timed confirm sketch" is the spec — reuse `parse_confirm_answer` + `NeedsTty` gate |
| Snippet tmp+rename atomic write + timestamped backup/rotation | utility | file-I/O | Repo writes are plain `std::fs::write` (power.rs:210); RESEARCH Pitfall 5 defines the requirement (idempotent insert, backup-before-every-write, rotation N=5) |
| Detached hyprpaper respawn | service | batch | Repo has no daemon-lifecycle spawn; RESEARCH Pattern 4 (`setsid hyprpaper` argv shim through duct, stdio null, env passthrough) |
| `lua_quote` escape helper | utility | transform | No Lua escaping precedent; RESEARCH Pattern 2 + Security Domain (allowlist charset `[A-Za-z0-9 ._-]` + reject `"`/`\`/newline/NUL), sibling of proxy.rs:404 debug_assert discipline |

## Metadata

**Analog search scope:** `crates/archel-core/src/` (all 15 modules read or grepped), `crates/archel/src/cmds/` (mod.rs, net_cmds.rs, audio_cmds.rs read; power_cmds.rs, sys_cmds.rs, pkg_cmds.rs, bt_cmds.rs, landing_cmd.rs grepped), `crates/archel-core/locales/fr.json`, `crates/archel/src/cmds/snapshots/`
**Files scanned:** ~25
**Tracked-source gate:** all named analog paths confirmed git-tracked (`git ls-files`); no gitignored mirrors referenced
**Pattern extraction date:** 2026-10-07
