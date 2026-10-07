//! Session-layer proxy state via gsettings (NET-04 tracer).
//!
//! Store: `org.gnome.system.proxy` through the `gsettings` CLI (no new
//! crate): `mode` plus per-scheme host/port plus `ignore-hosts`, plus
//! the `autoconfig-url` PAC pointer. Reads are unprivileged and sync;
//! mode transitions land in the next task.
//!
//! D-04 scope edge (specless-probe flagged assumption, surfaced per
//! plan): this module is session-layer ONLY, independent of NM
//! profiles — it never reads or writes NM keys, never shells to
//! `nmcli`, and never touches `wg0` or any connection profile.
//!
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

/// gsettings schema owning the session proxy (GNOME store consumed by
/// Chromium/Electron/GTK; terminals need env exports — see
/// [`export_hint`]).
const PROXY_SCHEMA: &str = "org.gnome.system.proxy";
/// gsettings proxy mode: direct connection, no proxy.
pub const MODE_NONE: &str = "none";
/// gsettings proxy mode: per-scheme host/port below.
pub const MODE_MANUAL: &str = "manual";
/// gsettings proxy mode: PAC URL below.
pub const MODE_AUTO: &str = "auto";

/// Session proxy snapshot, serialized as-is for `--json`.
///
/// Secret-free by construction: authentication material is never read
/// into this struct (T-4-04) — auth usernames live only in gsettings,
/// passwords are prompted at use time and never stored, so `--json`
/// output can never leak key material.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProxyState {
    /// gsettings mode word: `none`, `manual` or `auto` (raw when exotic).
    pub mode: String,
    /// Manual-mode HTTP proxy host (`None` when unset or mode differs).
    pub http_host: Option<String>,
    /// Manual-mode HTTP proxy port.
    pub http_port: Option<u16>,
    /// Manual-mode HTTPS proxy host.
    pub https_host: Option<String>,
    /// Manual-mode HTTPS proxy port.
    pub https_port: Option<u16>,
    /// Auto-mode PAC URL.
    pub autoconfig_url: Option<String>,
    /// Hosts bypassing the proxy (gsettings `ignore-hosts`).
    pub ignore_hosts: Vec<String>,
}

/// Reads the session proxy via the `gsettings` CLI.
pub fn proxy_status() -> Result<ProxyState, ArchelError> {
    proxy_status_with_tool("gsettings")
}

/// Same as [`proxy_status`] but the binary name is injectable, so the
/// which-gate stays unit-coverable headless (mirrors the
/// `missing_binary_maps_to_typed_checker_missing` precedent).
fn proxy_status_with_tool(tool: &str) -> Result<ProxyState, ArchelError> {
    let mode = parse_gsettings_string(&run_gsettings_query(tool, &["get", PROXY_SCHEMA, "mode"])?);
    let ignore_hosts =
        parse_ignore_hosts(&run_gsettings_query(tool, &["get", PROXY_SCHEMA, "ignore-hosts"])?);
    let (http_host, http_port, https_host, https_port, autoconfig_url) = match mode.as_str() {
        MODE_MANUAL => (
            opt_string(&run_gsettings_query(tool, &["get", PROXY_SCHEMA, "http", "host"])?),
            parse_gsettings_port(&run_gsettings_query(tool, &["get", PROXY_SCHEMA, "http", "port"])?),
            opt_string(&run_gsettings_query(tool, &["get", PROXY_SCHEMA, "https", "host"])?),
            parse_gsettings_port(&run_gsettings_query(
                tool,
                &["get", PROXY_SCHEMA, "https", "port"],
            )?),
            None,
        ),
        MODE_AUTO => (
            None,
            None,
            None,
            None,
            opt_string(&run_gsettings_query(tool, &["get", PROXY_SCHEMA, "autoconfig-url"])?),
        ),
        _ => (None, None, None, None, None),
    };
    Ok(ProxyState {
        mode,
        http_host,
        http_port,
        https_host,
        https_port,
        autoconfig_url,
        ignore_hosts,
    })
}

/// Runs one `gsettings get` query and returns its stdout.
///
/// Same spawn shape as the NM queries: which-gate with the typed
/// caller-visible variant, `.unchecked()` so the exit map stays
/// reachable, `LC_ALL=C`, `wait_timeout` plus kill, stderr log-only.
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

/// Unquotes one `gsettings get` string value (`'none'` -> `none`).
/// Pure over captured stdout, so locale-shaped output stays testable.
pub fn parse_gsettings_string(out: &str) -> String {
    let trimmed = out.trim();
    trimmed
        .strip_prefix('\'')
        .and_then(|inner| inner.strip_suffix('\''))
        .unwrap_or(trimmed)
        .to_string()
}

/// Parses one `gsettings get` port value (`8080` or `'8080'`) into a
/// port. Zero, garbage and out-of-range values degrade to `None`
/// (honest absence, never a guessed port).
pub fn parse_gsettings_port(out: &str) -> Option<u16> {
    let raw = parse_gsettings_string(out);
    match raw.parse::<u16>() {
        Ok(port) if port != 0 => Some(port),
        _ => None,
    }
}

/// Empty gsettings strings (`''`) degrade to `None` so tables render
/// the shared N/A cell instead of an empty quote pair.
fn opt_string(out: &str) -> Option<String> {
    let value = parse_gsettings_string(out);
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// Parses the gsettings `ignore-hosts` strv
/// (`['localhost', '127.0.0.0/8', ...]`; `[]` or `@as []` when empty).
/// Pure over captured stdout for headless coverage.
pub fn parse_ignore_hosts(out: &str) -> Vec<String> {
    let trimmed = out.trim();
    if trimmed == "[]" || trimmed.starts_with("@as") {
        return Vec::new();
    }
    trimmed
        .split('\'')
        .enumerate()
        .filter_map(|(index, part)| {
            if index % 2 == 1 && !part.is_empty() {
                Some(part.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// One-line French proxy state for bare `archel net proxy`
/// (NET-04): mode word plus the active endpoint detail. Guaranteed
/// single-line — embedded newlines from exotic store values are
/// flattened, so the done-criteria line count holds by construction.
pub fn status_line(state: &ProxyState) -> String {
    let label = rust_i18n::t!("net.proxy_label").to_string();
    let mode_word = match state.mode.as_str() {
        MODE_MANUAL => rust_i18n::t!("net.proxy_mode_manual").to_string(),
        MODE_AUTO => rust_i18n::t!("net.proxy_mode_auto").to_string(),
        MODE_NONE => rust_i18n::t!("net.proxy_mode_none").to_string(),
        other => other.to_string(),
    };
    let detail = proxy_endpoint_detail(state);
    let line = match detail {
        Some(detail) => format!("{label} : {mode_word} ({detail})"),
        None => format!("{label} : {mode_word}"),
    };
    line.replace(['\n', '\r'], " ")
}

/// Active endpoint detail for [`status_line`]: manual mode prefers the
/// HTTP endpoint, auto mode shows the PAC URL, `none` shows nothing.
fn proxy_endpoint_detail(state: &ProxyState) -> Option<String> {
    match state.mode.as_str() {
        MODE_MANUAL => {
            let (host, port) = match (&state.http_host, state.http_port) {
                (Some(host), Some(port)) => (host.clone(), port),
                _ => match (&state.https_host, state.https_port) {
                    (Some(host), Some(port)) => (host.clone(), port),
                    _ => return state
                        .https_host
                        .clone()
                        .or_else(|| state.http_host.clone()),
                },
            };
            Some(format!("{host}:{port}"))
        }
        MODE_AUTO => state.autoconfig_url.clone(),
        _ => None,
    }
}

/// Shell export hint for Hyprland terminals (NET-04): gsettings is
/// consumed by Chromium/Electron/GTK, but terminals and console tools
/// need `http_proxy`/`https_proxy` in the environment. Static catalog
/// text — it never echoes store values, so no secret can leak through
/// it (T-4-04).
pub fn export_hint() -> String {
    rust_i18n::t!("net.proxy_export_hint").to_string()
}

// --- Mode transitions (NET-04): off/manual/auto with optional auth.
//
// Writes are confirm-gated by the caller (preview plus `[o/N]` via
// `confirm_action`, `NeedsTty` when piped — same phase rule as DNS).
// Passwords arrive here already prompted (see [`prompt_proxy_password`]);
// this module never reads argv for secrets and never stores them.

/// A validated proxy transition, planned read-only before any prompt.
///
/// Carries the password in memory ONLY between the no-echo prompt and
/// the `gsettings set` spawn: no `Serialize` derive (it can never
/// reach `--json`), and the manual `Debug` below redacts the secret
/// so logs and test failures stay clean (T-4-04).
pub struct ProxyPlan {
    /// Target gsettings mode word (`none`, `manual`, `auto`).
    pub mode: String,
    /// Mode observed before the transition (for the preview table).
    pub old_mode: String,
    /// Manual-mode host (both schemes share it).
    pub host: Option<String>,
    /// Manual-mode port (both schemes share it).
    pub port: Option<u16>,
    /// Auto-mode PAC URL.
    pub pac_url: Option<String>,
    /// Bypass list applied as gsettings `ignore-hosts`.
    pub exceptions: Vec<String>,
    /// Optional proxy auth username (gsettings auth keys).
    pub user: Option<String>,
    /// Optional proxy auth password: in-memory only, never serialized.
    pub password: Option<String>,
}

impl std::fmt::Debug for ProxyPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxyPlan")
            .field("mode", &self.mode)
            .field("old_mode", &self.old_mode)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("pac_url", &self.pac_url)
            .field("exceptions", &self.exceptions)
            .field("user", &self.user)
            .field("password_set", &self.password.is_some())
            .finish()
    }
}

impl ProxyPlan {
    /// Plans the `off` transition (used by the argv unit test without
    /// touching the store).
    pub fn off(old_mode: &str) -> Self {
        Self {
            mode: MODE_NONE.to_string(),
            old_mode: old_mode.to_string(),
            host: None,
            port: None,
            pac_url: None,
            exceptions: Vec::new(),
            user: None,
            password: None,
        }
    }
}

/// Validates a manual proxy host: non-empty, no whitespace or quote
/// characters (quotes would break the gsettings GVariant quoting).
pub fn validate_proxy_host(host: &str) -> Result<String, ArchelError> {
    let trimmed = host.trim();
    if trimmed.is_empty() {
        return Err(ArchelError::ProxyInvalid {
            detail: "empty proxy host".to_string(),
        });
    }
    if trimmed
        .chars()
        .any(|c| c.is_whitespace() || c.is_control() || c == '\'' || c == '"')
    {
        return Err(ArchelError::ProxyInvalid {
            detail: format!("rejected proxy host {trimmed:?}"),
        });
    }
    Ok(trimmed.to_string())
}

/// Validates a proxy port: 0 is never a usable proxy port.
pub fn validate_proxy_port(port: u16) -> Result<u16, ArchelError> {
    if port == 0 {
        return Err(ArchelError::ProxyInvalid {
            detail: "proxy port 0 is out of range (1-65535)".to_string(),
        });
    }
    Ok(port)
}

/// Validates an auto-mode PAC URL: `http(s)` scheme, non-empty
/// remainder, no whitespace. Anything else fails before any prompt.
pub fn validate_pac_url(url: &str) -> Result<String, ArchelError> {
    let trimmed = url.trim();
    let rest = trimmed
        .strip_prefix("http://")
        .or_else(|| trimmed.strip_prefix("https://"));
    match rest {
        Some(rest) if !rest.is_empty() && !rest.chars().any(|c| c.is_whitespace() || c == '\'') => {
            Ok(trimmed.to_string())
        }
        _ => Err(ArchelError::ProxyInvalid {
            detail: format!("malformed PAC URL {trimmed:?} (http(s)://host/path expected)"),
        }),
    }
}

/// Splits the `--exceptions` comma list, trimming and dropping empties.
/// Pure and total — an absent flag plans no bypass list.
pub fn parse_exceptions(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

/// Plans the `manual` transition: validates host/port client-side
/// before any prompt or spawn (nothing mutates on validation error).
pub fn plan_proxy_manual(
    old_mode: &str,
    host: &str,
    port: u16,
    exceptions: &str,
    user: Option<&str>,
    password: Option<String>,
) -> Result<ProxyPlan, ArchelError> {
    Ok(ProxyPlan {
        mode: MODE_MANUAL.to_string(),
        old_mode: old_mode.to_string(),
        host: Some(validate_proxy_host(host)?),
        port: Some(validate_proxy_port(port)?),
        pac_url: None,
        exceptions: parse_exceptions(exceptions),
        user: user.map(str::to_string),
        password,
    })
}

/// Plans the `auto` transition: validates the PAC URL client-side.
pub fn plan_proxy_auto(old_mode: &str, pac_url: &str) -> Result<ProxyPlan, ArchelError> {
    Ok(ProxyPlan {
        mode: MODE_AUTO.to_string(),
        old_mode: old_mode.to_string(),
        host: None,
        port: None,
        pac_url: Some(validate_pac_url(pac_url)?),
        exceptions: Vec::new(),
        user: None,
        password: None,
    })
}

/// Quotes one value for `gsettings set` (GVariant strings need single
/// quotes). Callers validate first, so no quote characters survive to
/// this point — the debug assertion guards the invariant.
fn gsettings_string(value: &str) -> String {
    debug_assert!(
        !value.contains('\''),
        "gsettings values must be quote-free after validation"
    );
    format!("'{value}'")
}

/// Formats a bypass list for `gsettings set` (`['a', 'b']` strv).
fn gsettings_strv(values: &[String]) -> String {
    let items = values
        .iter()
        .map(|item| gsettings_string(item))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{items}]")
}

/// Builds the exact `gsettings set` sequences for a [`ProxyPlan`].
///
/// Pure over the plan (pkg_helper_argv purity precedent): every mode
/// maps to `mode` plus its scheme keys, so the transition is
/// unit-coverable without spawning. Auth keys land only when a
/// username was provided; the password travels as an in-process argv
/// element (never through a shell, never logged, never snapshotted).
pub fn proxy_write_argv(plan: &ProxyPlan) -> Vec<Vec<String>> {
    let set = |key_path: &[&str], value: String| -> Vec<String> {
        let mut argv = vec!["gsettings".to_string(), "set".to_string(), PROXY_SCHEMA.to_string()];
        argv.extend(key_path.iter().map(|key| (*key).to_string()));
        argv.push(value);
        argv
    };
    let mut steps = vec![set(&["mode"], gsettings_string(&plan.mode))];
    match plan.mode.as_str() {
        MODE_MANUAL => {
            if let (Some(host), Some(port)) = (plan.host.as_deref(), plan.port) {
                for scheme in ["http", "https"] {
                    steps.push(set(&[scheme, "host"], gsettings_string(host)));
                    steps.push(set(&[scheme, "port"], port.to_string()));
                }
            }
            if !plan.exceptions.is_empty() {
                steps.push(set(&["ignore-hosts"], gsettings_strv(&plan.exceptions)));
            }
            if let Some(user) = plan.user.as_deref() {
                for scheme in ["http", "https"] {
                    steps.push(set(
                        &[scheme, "authentication-user"],
                        gsettings_string(user),
                    ));
                    if let Some(password) = plan.password.as_deref() {
                        steps.push(set(
                            &[scheme, "authentication-password"],
                            gsettings_string(password),
                        ));
                    }
                }
            }
        }
        MODE_AUTO => {
            if let Some(pac_url) = plan.pac_url.as_deref() {
                steps.push(set(&["autoconfig-url"], gsettings_string(pac_url)));
            }
        }
        _ => {}
    }
    steps
}

/// Applies one [`ProxyPlan`] through the `gsettings` CLI and reports
/// the secret-free outcome for tables and `--json`.
pub fn apply_proxy_plan(plan: &ProxyPlan) -> Result<ProxyApplyReport, ArchelError> {
    for argv in proxy_write_argv(plan) {
        run_gsettings_write(&argv, &format!("set proxy {}", plan.mode))?;
    }
    Ok(ProxyApplyReport::from_plan(plan))
}

/// Runs one mutating `gsettings set` argv (exact args, never shelled).
fn run_gsettings_write(argv: &[String], action: &str) -> Result<(), ArchelError> {
    let binary = which::which(&argv[0])
        .map_err(|err| ArchelError::network_unavailable(format!("{} ({err})", argv[0])))?;
    let handle = duct::cmd(&binary, &argv[1..])
        .unchecked()
        .env("LC_ALL", "C")
        .stdout_capture()
        .stderr_capture()
        .start()
        .map_err(|err| ArchelError::Internal(format!("cannot run gsettings: {err}")))?;
    let output = match handle.wait_timeout(QUERY_TIMEOUT).map_err(|err| {
        ArchelError::Internal(format!("cannot wait on gsettings: {err}"))
    })? {
        Some(output) => output.clone(),
        None => {
            let _ = handle.kill();
            return Err(ArchelError::Internal("gsettings exceeded the query timeout".to_string()));
        }
    };
    // Stderr stays log-only: auth-adjacent failures must never echo
    // argv (which may carry the password element) to the terminal.
    let _stderr_for_logs = String::from_utf8_lossy(&output.stderr);
    match output.status.code() {
        Some(0) => Ok(()),
        code => Err(ArchelError::Internal(format!(
            "cannot {action} (gsettings exit {code:?})"
        ))),
    }
}

/// Reads the proxy password via a no-echo stdin prompt at use time
/// (researcher ruling: gsettings auth keys trivially supported).
/// Never a CLI arg, never stored — the caller moves it straight into
/// the plan. A prompt failure without a terminal maps to the typed
/// `NeedsTty` refusal (same phase rule as mutations).
pub fn prompt_proxy_password() -> Result<String, ArchelError> {
    let prompt = rust_i18n::t!("net.proxy_password_prompt").to_string();
    rpassword::prompt_password(prompt).map_err(|_| ArchelError::NeedsTty)
}

/// Secret-free outcome of [`apply_proxy_plan`]: the only shape that
/// may reach tables and `--json` (T-4-04).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProxyApplyReport {
    /// Target mode word that was applied.
    pub mode: String,
    /// Human endpoint detail (`host:port`, PAC URL, or `None` for off).
    pub endpoint: Option<String>,
    /// Bypass list that was applied.
    pub exceptions: Vec<String>,
    /// Whether an auth username was configured (never the secret).
    pub auth_configured: bool,
}

impl ProxyApplyReport {
    /// Derives the report from the plan, dropping the password.
    pub fn from_plan(plan: &ProxyPlan) -> Self {
        let endpoint = match plan.mode.as_str() {
            MODE_MANUAL => match (plan.host.as_deref(), plan.port) {
                (Some(host), Some(port)) => Some(format!("{host}:{port}")),
                (Some(host), None) => Some(host.to_string()),
                _ => None,
            },
            MODE_AUTO => plan.pac_url.clone(),
            _ => None,
        };
        Self {
            mode: plan.mode.clone(),
            endpoint,
            exceptions: plan.exceptions.clone(),
            auth_configured: plan.user.is_some(),
        }
    }
}

/// French preview title for the confirm gate (shown before `[o/N]`).
pub fn proxy_preview_title(plan: &ProxyPlan) -> String {
    let _ = plan;
    rust_i18n::t!("net.proxy_preview_title").to_string()
}

/// French `(label, value)` preview rows: old mode plus new endpoint.
pub fn proxy_preview_rows(plan: &ProxyPlan) -> Vec<(String, String)> {
    let report = ProxyApplyReport::from_plan(plan);
    let na = || rust_i18n::t!("sys.unavailable").to_string();
    vec![
        (
            rust_i18n::t!("net.label_old").to_string(),
            mode_word(&plan.old_mode),
        ),
        (
            rust_i18n::t!("net.label_new").to_string(),
            mode_word(&plan.mode),
        ),
        (
            rust_i18n::t!("net.label_server").to_string(),
            report.endpoint.unwrap_or_else(na),
        ),
        (
            rust_i18n::t!("net.label_exceptions").to_string(),
            if report.exceptions.is_empty() {
                na()
            } else {
                report.exceptions.join(" ")
            },
        ),
    ]
}

/// French confirmation prompt naming the target mode and endpoint.
pub fn confirm_proxy_prompt(plan: &ProxyPlan) -> String {
    let report = ProxyApplyReport::from_plan(plan);
    let na = || rust_i18n::t!("sys.unavailable").to_string();
    rust_i18n::t!(
        "net.confirm_proxy",
        mode = mode_word(&plan.mode),
        endpoint = report.endpoint.unwrap_or_else(na)
    )
    .to_string()
}

/// French title for the post-apply report table.
pub fn proxy_report_title() -> String {
    rust_i18n::t!("net.proxy_report_title").to_string()
}

/// French rows for the post-apply report table.
pub fn proxy_report_rows(report: &ProxyApplyReport) -> Vec<(String, String)> {
    let na = || rust_i18n::t!("sys.unavailable").to_string();
    vec![
        (
            rust_i18n::t!("net.label_mode").to_string(),
            mode_word(&report.mode),
        ),
        (
            rust_i18n::t!("net.label_server").to_string(),
            report.endpoint.clone().unwrap_or_else(na),
        ),
        (
            rust_i18n::t!("net.label_exceptions").to_string(),
            if report.exceptions.is_empty() {
                na()
            } else {
                report.exceptions.join(" ")
            },
        ),
    ]
}

/// French wording for a gsettings mode word (raw fallback for exotic
/// modes — honest display, never a default-ok label).
fn mode_word(mode: &str) -> String {
    match mode {
        MODE_MANUAL => rust_i18n::t!("net.proxy_mode_manual").to_string(),
        MODE_AUTO => rust_i18n::t!("net.proxy_mode_auto").to_string(),
        MODE_NONE => rust_i18n::t!("net.proxy_mode_none").to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manual_state() -> ProxyState {
        ProxyState {
            mode: MODE_MANUAL.to_string(),
            http_host: Some("proxy.example".to_string()),
            http_port: Some(8080),
            https_host: None,
            https_port: None,
            autoconfig_url: None,
            ignore_hosts: vec!["localhost".to_string()],
        }
    }

    #[test]
    fn status_line_is_single_line_with_endpoint() {
        crate::i18n::set_language("fr");
        let line = status_line(&manual_state());
        assert!(
            !line.contains(['\n', '\r']),
            "the status line must stay one line: {line:?}"
        );
        assert!(
            line.contains("proxy.example:8080"),
            "manual detail names the endpoint: {line:?}"
        );
    }

    #[test]
    fn status_line_flattens_exotic_newlines() {
        crate::i18n::set_language("fr");
        let mut state = manual_state();
        state.http_host = Some("evil\ninjection".to_string());
        let line = status_line(&state);
        assert!(
            !line.contains(['\n', '\r']),
            "exotic store values never break the one-liner: {line:?}"
        );
    }

    #[test]
    fn auto_line_shows_pac_url_and_none_shows_mode_only() {
        crate::i18n::set_language("fr");
        let auto = ProxyState {
            mode: MODE_AUTO.to_string(),
            http_host: None,
            http_port: None,
            https_host: None,
            https_port: None,
            autoconfig_url: Some("http://pac.example/proxy.pac".to_string()),
            ignore_hosts: Vec::new(),
        };
        let line = status_line(&auto);
        assert!(
            line.contains("http://pac.example/proxy.pac"),
            "auto detail shows the PAC URL: {line:?}"
        );
        let off = ProxyState {
            mode: MODE_NONE.to_string(),
            http_host: None,
            http_port: None,
            https_host: None,
            https_port: None,
            autoconfig_url: None,
            ignore_hosts: Vec::new(),
        };
        let line = status_line(&off);
        assert!(
            !line.contains(['\n', '\r']),
            "off state stays one line: {line:?}"
        );
        assert!(
            !line.contains('('),
            "off state carries no endpoint detail: {line:?}"
        );
    }

    #[test]
    fn export_hint_is_non_empty_french_guidance() {
        crate::i18n::set_language("fr");
        let hint = export_hint();
        assert!(!hint.trim().is_empty(), "the export hint must exist");
        assert!(
            hint.contains("http_proxy"),
            "the hint must name the shell variable: {hint:?}"
        );
    }

    #[test]
    fn parsers_cover_gsettings_output_shapes() {
        assert_eq!(parse_gsettings_string("'none'\n"), "none");
        assert_eq!(parse_gsettings_string("plain"), "plain");
        assert_eq!(parse_gsettings_port("8080\n"), Some(8080));
        assert_eq!(parse_gsettings_port("'3128'"), Some(3128));
        assert_eq!(parse_gsettings_port("0"), None);
        assert_eq!(parse_gsettings_port("99999"), None);
        assert_eq!(parse_gsettings_port("garbage"), None);
        assert_eq!(
            parse_ignore_hosts("['localhost', '127.0.0.0/8']\n"),
            vec!["localhost".to_string(), "127.0.0.0/8".to_string()]
        );
        assert!(parse_ignore_hosts("[]\n").is_empty());
        assert!(parse_ignore_hosts("@as []\n").is_empty());
    }

    #[test]
    fn missing_binary_maps_to_typed_unavailable() {
        let err = proxy_status_with_tool("archel-no-such-tool")
            .expect_err("a missing gsettings must fail typed");
        assert!(
            matches!(err, ArchelError::NetworkUnavailable { .. }),
            "missing gsettings is typed unavailable, never a panic: {err:?}"
        );
    }

    #[test]
    fn proxy_state_json_carries_no_secret_shape() {        // Guard for T-4-04: the serialized struct has no field that
        // could carry key material — auth never enters this struct.
        let value =
            serde_json::to_value(&manual_state()).expect("ProxyState serializes");
        for field in [
            "mode",
            "http_host",
            "http_port",
            "https_host",
            "https_port",
            "autoconfig_url",
            "ignore_hosts",
        ] {
            assert!(value.get(field).is_some(), "state carries `{field}`");
        }
        let joined = serde_json::to_string(&value).expect("state encodes");
        for marker in ["password", "secret", "authentication"] {
            assert!(
                !joined.to_lowercase().contains(marker),
                "no secret-shaped field `{marker}` in proxy JSON"
            );
        }
    }

    // --- Task 2 (TDD RED): mode-transition behavior, written before
    // the implementation. These fail to compile until the plan/argv
    // API below them exists.

    #[test]
    fn argv_builder_maps_modes_to_exact_gsettings_sequences() {
        let off = ProxyPlan::off("manual");
        assert_eq!(
            proxy_write_argv(&off),
            vec![vec![
                "gsettings".to_string(),
                "set".to_string(),
                "org.gnome.system.proxy".to_string(),
                "mode".to_string(),
                "'none'".to_string(),
            ]]
        );
        let manual = plan_proxy_manual(
            "none",
            "proxy.example",
            8080,
            "localhost, 127.0.0.0/8",
            Some("alice"),
            Some("s3cret".to_string()),
        )
        .expect("valid manual input plans");
        let argv = proxy_write_argv(&manual);
        let flat = argv
            .iter()
            .map(|step| step.join(" "))
            .collect::<Vec<_>>()
            .join("\n");
        for needle in [
            "set org.gnome.system.proxy mode 'manual'",
            "set org.gnome.system.proxy http host 'proxy.example'",
            "set org.gnome.system.proxy http port 8080",
            "set org.gnome.system.proxy https host 'proxy.example'",
            "set org.gnome.system.proxy https port 8080",
            "ignore-hosts ['localhost', '127.0.0.0/8']",
            "http authentication-user 'alice'",
            "https authentication-user 'alice'",
        ] {
            assert!(flat.contains(needle), "manual argv must carry `{needle}`");
        }
        let auto = plan_proxy_auto("none", "http://pac.example/proxy.pac")
            .expect("valid PAC URL plans");
        let argv = proxy_write_argv(&auto);
        let flat = argv
            .iter()
            .map(|step| step.join(" "))
            .collect::<Vec<_>>()
            .join("\n");
        for needle in [
            "set org.gnome.system.proxy mode 'auto'",
            "set org.gnome.system.proxy autoconfig-url 'http://pac.example/proxy.pac'",
        ] {
            assert!(flat.contains(needle), "auto argv must carry `{needle}`");
        }
    }

    #[test]
    fn host_port_and_pac_validators_reject_garbage() {
        for bad_host in ["", "   ", "has space.example", "quote'.example"] {
            assert!(
                validate_proxy_host(bad_host).is_err(),
                "host `{bad_host:?}` must fail validation"
            );
        }
        assert!(validate_proxy_port(0).is_err(), "port 0 must fail");
        assert!(validate_proxy_port(8080).is_ok());
        for bad_pac in ["", "notaurl", "ftp://x/y.pac", "http://", "http://has space/x"] {
            assert!(
                validate_pac_url(bad_pac).is_err(),
                "PAC URL `{bad_pac:?}` must fail validation"
            );
        }
        assert!(validate_pac_url("https://pac.example/p.pac").is_ok());
    }

    #[test]
    fn secret_redaction_keeps_passwords_out_of_debug_json_and_reports() {
        let plan = plan_proxy_manual(
            "none",
            "proxy.example",
            8080,
            "",
            Some("alice"),
            Some("s3cret-pw".to_string()),
        )
        .expect("valid manual input plans");
        let debug = format!("{plan:?}");
        assert!(
            !debug.contains("s3cret-pw"),
            "Debug must never echo the password: {debug:?}"
        );
        let report = ProxyApplyReport::from_plan(&plan);
        let joined =
            serde_json::to_string(&report).expect("apply report serializes");
        assert!(
            !joined.contains("s3cret-pw"),
            "apply report JSON must never carry the password"
        );
        for marker in ["password", "secret"] {
            assert!(
                !joined.to_lowercase().contains(marker),
                "no secret-shaped field `{marker}` in report JSON"
            );
        }
    }
}
