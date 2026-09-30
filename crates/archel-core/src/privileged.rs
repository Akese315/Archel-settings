//! Privileged helper client (per D-09: pkexec is the sole escalation
//! mechanism; no sudo path exists anywhere in core or the GUI).
//!
//! Single choke point for every privileged call (later phases reuse it):
//! locate the `archel-privileged` binary, spawn it with an exact argv
//! (never through a shell), capture stdout/stderr, check the versioned
//! JSON envelope (`v == 1`), and map failures to typed [`ArchelError`]
//! variants. Helper stderr is captured for logs only and never displayed
//! (Pitfall 4); raw helper detail stays in English `Display`/logs while
//! the user only sees the French catalog string (T-1-02).

use std::path::PathBuf;

use serde::Deserialize;

use crate::error::ArchelError;

/// Protocol version the helper speaks on stdout. Any envelope with a
/// different `v` is a [`ArchelError::HelperProtocol`] (never default-ok).
pub const PROTOCOL_VERSION: u32 = 1;

/// The single allowlisted action in Phase 1 (harmless dry-run probe).
pub const DRY_RUN_ACTION: &str = "dry-run";

/// Helper stdout envelope. `deny_unknown_fields` is deliberately NOT set:
/// the version gate (`v == 1`) is the compatibility check, extra fields
/// from a newer helper must not turn into a hard failure here.
#[derive(Debug, Deserialize)]
struct HelperEnvelope {
    v: u32,
    action: Option<String>,
    ok: Option<bool>,
    error: Option<String>,
}

/// Validates one helper stdout payload for `action`.
///
/// Returns the echoed action name on success. Every other shape —
/// unparseable JSON, version skew, `ok` missing/false — becomes
/// [`ArchelError::HelperProtocol`] with log detail (never user-facing).
pub fn parse_helper_stdout(action: &str, stdout: &str) -> Result<String, ArchelError> {
    let envelope: HelperEnvelope = serde_json::from_str(stdout).map_err(|err| {
        ArchelError::HelperProtocol(format!("unparseable helper output for '{action}': {err}"))
    })?;
    if envelope.v != PROTOCOL_VERSION {
        return Err(ArchelError::HelperProtocol(format!(
            "helper protocol v{} for '{action}', expected v{PROTOCOL_VERSION}",
            envelope.v
        )));
    }
    if envelope.ok != Some(true) {
        let detail = envelope.error.as_deref().unwrap_or("ok-flag-missing");
        return Err(ArchelError::HelperProtocol(format!(
            "helper refused '{action}': {detail}"
        )));
    }
    Ok(envelope.action.unwrap_or_else(|| action.to_string()))
}

/// Locates the `archel-privileged` binary next to the running `archel`
/// executable (single-binary install layout, per D-01/D-03). Absolute
/// path only — the helper is never resolved through `$PATH` at spawn
/// time, so a trojaned PATH entry cannot redirect elevation.
pub fn locate_helper() -> Result<PathBuf, ArchelError> {
    let current = std::env::current_exe().map_err(|err| {
        ArchelError::HelperProtocol(format!("cannot locate current executable: {err}"))
    })?;
    let dir = current.parent().ok_or_else(|| {
        ArchelError::HelperProtocol("current executable has no parent directory".to_string())
    })?;
    Ok(dir.join("archel-privileged"))
}

/// Runs one allowlisted `action` by spawning the helper directly
/// (no pkexec). This is the Phase 1 end-to-end proof path; the pkexec
/// invocation path (with the 126/127 mapping) is layered on in task 2.
pub fn run_action_direct(action: &str) -> Result<String, ArchelError> {
    let helper = locate_helper()?;
    let output = duct::cmd(&helper, [action])
        .stdout_capture()
        .stderr_capture()
        .run()
        .map_err(|err| {
            ArchelError::HelperProtocol(format!("cannot spawn helper for '{action}': {err}"))
        })?;
    if !output.status.success() {
        return Err(ArchelError::HelperProtocol(format!(
            "helper exited with status {} for '{action}'",
            output.status
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_helper_stdout(action, &stdout)
}

/// Phase 1 probe: `dry-run` through the direct-spawn path.
pub fn dry_run_direct() -> Result<String, ArchelError> {
    run_action_direct(DRY_RUN_ACTION)
}

/// Maps a pkexec exit code to the next step (per D-12, Pitfall 4).
///
/// Pure so the 0/126/127/other matrix is unit-tested with mocked codes:
/// - `0` → `Ok(())`: caller proceeds to parse the helper v1 JSON.
/// - `126` → dismissed dialog → friendly `AuthFailed{Cancelled}` naming
///   the action (never raw polkit text).
/// - `127` → refused authorization → friendly `AuthFailed{Denied}`.
/// - anything else → `HelperProtocol` with the raw code for logs only.
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

/// Runs one allowlisted `action` through the real escalation path:
/// `pkexec <absolute-helper-path> <action>` with the default polkit
/// agent. Helper stderr is captured for logs only and never displayed
/// (Pitfall 4: no `GDBus.Error...` text ever reaches the user).
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

/// Phase 1 probe through the real escalation path (prompts the desktop
/// polkit agent; dismissing maps to the friendly cancelled error).
pub fn dry_run_via_pkexec() -> Result<String, ArchelError> {
    run_action_via_pkexec(DRY_RUN_ACTION)
}

/// French success line for a validated dry-run, rendered from the catalog
/// (D-05/D-07). Lives here — not in the binary — so `t!` resolves against
/// this crate's catalog backend, exactly like [`crate::version_line`].
pub fn dry_run_success_line(echoed_action: &str) -> String {
    rust_i18n::t!("privileged.dry_run_ok", action = echoed_action).to_string()
}

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

    #[test]
    fn version_skew_never_defaults_to_ok() {
        let err = parse_helper_stdout("dry-run", r#"{"v":2,"action":"dry-run","ok":true}"#)
            .expect_err("v != 1 must be rejected");
        assert!(matches!(err, ArchelError::HelperProtocol(_)));
    }

    #[test]
    fn unparseable_stdout_maps_to_helper_protocol() {
        for garbage in ["", "not json", "GDBus.Error:org.freedesktop.policykit.exec"] {
            let err = parse_helper_stdout("dry-run", garbage)
                .expect_err("garbage must never parse as ok");
            assert!(
                matches!(err, ArchelError::HelperProtocol(_)),
                "wrong variant for {garbage:?}"
            );
        }
    }

    #[test]
    fn refused_envelope_maps_to_helper_protocol() {
        let err = parse_helper_stdout(
            "dry-run",
            r#"{"v":1,"ok":false,"error":"action-not-allowlisted"}"#,
        )
        .expect_err("ok:false must be rejected");
        assert!(matches!(err, ArchelError::HelperProtocol(_)));
    }

    #[test]
    fn missing_ok_flag_never_defaults_to_ok() {
        let err = parse_helper_stdout("dry-run", r#"{"v":1,"action":"dry-run"}"#)
            .expect_err("missing ok flag must be rejected");
        assert!(matches!(err, ArchelError::HelperProtocol(_)));
    }

    #[test]
    fn pkexec_zero_means_parse_next() {
        assert!(map_pkexec_exit("dry-run", Some(0)).is_ok());
    }

    #[test]
    fn pkexec_126_maps_to_cancelled_naming_the_action() {
        let err = map_pkexec_exit("package install", Some(126))
            .expect_err("126 must be the dismissed-dialog path");
        assert_eq!(
            err,
            ArchelError::privilege_cancelled("package install"),
            "126 must map to AuthFailed/Cancelled"
        );
        let rendered = err.message();
        assert!(
            rendered.contains("package install"),
            "cancelled message must name the action: {rendered}"
        );
    }

    #[test]
    fn pkexec_127_maps_to_denied_naming_the_action() {
        let err = map_pkexec_exit("package install", Some(127))
            .expect_err("127 must be the refused-auth path");
        assert_eq!(
            err,
            ArchelError::privilege_denied("package install"),
            "127 must map to AuthFailed/Denied"
        );
        let rendered = err.message();
        assert!(
            rendered.contains("package install"),
            "denied message must name the action: {rendered}"
        );
    }

    #[test]
    fn cancelled_vs_denied_render_distinct_french() {
        rust_i18n::set_locale("fr");
        let cancelled = map_pkexec_exit("package install", Some(126))
            .expect_err("126")
            .message();
        let denied = map_pkexec_exit("package install", Some(127))
            .expect_err("127")
            .message();
        assert_ne!(cancelled, denied, "D-12 requires distinct wording");
        assert!(
            !cancelled.contains("GDBus") && !denied.contains("GDBus"),
            "raw polkit text must never leak into catalog strings"
        );
    }

    #[test]
    fn pkexec_other_codes_map_to_helper_protocol() {
        for code in [Some(1), Some(2), Some(127 + 1), None] {
            let err = map_pkexec_exit("dry-run", code)
                .expect_err("unexpected code must not pass");
            assert!(
                matches!(err, ArchelError::HelperProtocol(_)),
                "wrong variant for code {code:?}"
            );
        }
    }
}
