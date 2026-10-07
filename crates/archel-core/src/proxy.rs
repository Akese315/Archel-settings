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
    fn proxy_state_json_carries_no_secret_shape() {
        // Guard for T-4-04: the serialized struct has no field that
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
}
