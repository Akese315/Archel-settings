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

/// Helper verbs for the package mutations (mirror the allowlist in
/// `archel-privileged`; the client never invents flags here).
pub const PKG_INSTALL_VERB: &str = "pkg-install";
/// Helper verb removing packages through the allowlist.
pub const PKG_REMOVE_VERB: &str = "pkg-remove";
/// Helper verb running the full-system upgrade through the allowlist.
pub const PKG_UPGRADE_VERB: &str = "pkg-upgrade";

/// Builds the exact helper argv for a mutation verb: the verb first,
/// then the package list verbatim. No flags are ever added client-side
/// (T-3-04: every flag is hardcoded server-side in the helper).
fn pkg_helper_argv(verb: &str, packages: &[String]) -> Vec<String> {
    let mut argv = Vec::with_capacity(packages.len() + 1);
    argv.push(verb.to_string());
    argv.extend(packages.iter().cloned());
    argv
}

/// French action label naming an install in the 126/127 dialogs, from
/// the catalog so the binary owns no French copy.
pub fn pkg_install_action_label() -> String {
    rust_i18n::t!("pkg.action_install").to_string()
}

/// French action label naming a removal in the 126/127 dialogs.
pub fn pkg_remove_action_label() -> String {
    rust_i18n::t!("pkg.action_remove").to_string()
}

/// French action label naming the full upgrade in the 126/127 dialogs.
pub fn pkg_upgrade_action_label() -> String {
    rust_i18n::t!("pkg.action_upgrade").to_string()
}

/// Last-line envelope extraction: the helper prints the v1 envelope as
/// the FINAL stdout line after the live pacman transcript, so the
/// client parses only that line and `parse_helper_stdout` needs no
/// format change. Pure, so the streaming contract is unit-tested
/// without pkexec. `None` on an empty transcript (never default-ok).
fn stream_envelope_line(transcript: &str) -> Option<&str> {
    transcript.lines().last()
}

/// Drains a live stdout pipe line by line: every line reaches `on_line`
/// the moment it arrives (the bin prints it for D-02) AND accumulates
/// into the returned transcript for `--json`. Generic over the reader
/// so tests feed a cursor instead of spawning pkexec.
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

/// Runs one mutation verb through pkexec with live streaming (D-02).
/// Uses `reader()` (never `run()` with both captures on this path, per
/// Pitfall 7: the pipe drains while pacman runs, so a long `-Syu`
/// cannot deadlock on a full pipe buffer). Returns the full transcript
/// for `--json` framing; stderr stays captured for logs only.
pub fn run_privileged_streaming(
    verb: &str,
    packages: &[String],
    action_label: &str,
    on_line: &mut dyn FnMut(&str),
) -> Result<String, ArchelError> {
    let pkexec = which::which("pkexec").map_err(|err| {
        ArchelError::HelperProtocol(format!("pkexec not available: {err}"))
    })?;
    let helper = locate_helper()?;
    let mut argv: Vec<PathBuf> = vec![helper];
    argv.extend(pkg_helper_argv(verb, packages).iter().map(PathBuf::from));
    // `.unchecked()` is load-bearing: without it duct turns the final
    // read into an IO error on any non-zero exit, and the pkexec-code
    // map below becomes unreachable (same shape as `packages.rs`).
    let handle = duct::cmd(&pkexec, argv)
        .unchecked()
        .env("LC_ALL", "C")
        .stderr_capture()
        .reader()
        .map_err(|err| {
            ArchelError::HelperProtocol(format!("cannot spawn pkexec for '{verb}': {err}"))
        })?;
    let transcript = collect_streaming(std::io::BufReader::new(&handle), on_line);
    // After EOF the child is reaped, so `try_wait` always yields the
    // status here; `None` (or a missing code) is a protocol failure,
    // never a silent ok.
    let waited = handle.try_wait().map_err(|err| {
        ArchelError::HelperProtocol(format!("cannot wait on pkexec for '{verb}': {err}"))
    })?;
    // Log-only capture of helper stderr (kept for diagnostics; the
    // user-facing error below never embeds it).
    let _stderr_for_logs = waited.map(|output| {
        String::from_utf8_lossy(&output.stderr).into_owned()
    });
    let code = waited.and_then(|output| output.status.code());
    map_pkexec_exit(action_label, code)?;
    let envelope = stream_envelope_line(&transcript).ok_or_else(|| {
        ArchelError::HelperProtocol(format!("helper produced no output for '{verb}'"))
    })?;
    parse_helper_stdout(verb, envelope)?;
    Ok(transcript)
}

/// Installs packages through the helper with live output streaming.
/// The helper re-validates every name; only the confirmed transaction
/// reaches this path (D-01 gates in the bin).
pub fn pkg_install(
    packages: &[String],
    on_line: &mut dyn FnMut(&str),
) -> Result<String, ArchelError> {
    let label = pkg_install_action_label();
    run_privileged_streaming(PKG_INSTALL_VERB, packages, &label, on_line)
}

/// Removes packages through the helper with live output streaming.
pub fn pkg_remove(
    packages: &[String],
    on_line: &mut dyn FnMut(&str),
) -> Result<String, ArchelError> {
    let label = pkg_remove_action_label();
    run_privileged_streaming(PKG_REMOVE_VERB, packages, &label, on_line)
}

/// Runs the full-system upgrade through the helper with live output
/// streaming. The ONLY refresh path (D-03): no partial-upgrade verb
/// exists anywhere in the helper or this client.
pub fn pkg_upgrade(on_line: &mut dyn FnMut(&str)) -> Result<String, ArchelError> {
    let label = pkg_upgrade_action_label();
    run_privileged_streaming(PKG_UPGRADE_VERB, &[], &label, on_line)
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

    #[test]
    fn pkg_argv_is_verb_first_with_packages_verbatim() {
        assert_eq!(
            pkg_helper_argv(PKG_INSTALL_VERB, &["firefox".to_string()]),
            vec!["pkg-install".to_string(), "firefox".to_string()]
        );
        assert_eq!(
            pkg_helper_argv(PKG_REMOVE_VERB, &["a".to_string(), "b".to_string()]),
            vec![
                "pkg-remove".to_string(),
                "a".to_string(),
                "b".to_string()
            ]
        );
        assert_eq!(
            pkg_helper_argv(PKG_UPGRADE_VERB, &[]),
            vec!["pkg-upgrade".to_string()],
            "upgrade carries no package args (exact-match verb)"
        );
    }

    #[test]
    fn pkg_argv_never_carries_client_side_flags() {
        for verb in [PKG_INSTALL_VERB, PKG_REMOVE_VERB, PKG_UPGRADE_VERB] {
            let argv = pkg_helper_argv(verb, &["firefox".to_string()]);
            assert!(
                argv.iter().all(|a| !a.starts_with('-')),
                "flags are server-side only, client sends verb plus names: {argv:?}"
            );
        }
    }

    #[test]
    fn envelope_is_the_last_transcript_line() {
        let transcript = ":: downloading firefox-156.0.1\n:: installing\n{\"v\":1,\"action\":\"pkg-install\",\"ok\":true}\n";
        assert_eq!(
            stream_envelope_line(transcript),
            Some("{\"v\":1,\"action\":\"pkg-install\",\"ok\":true}"),
            "only the final line parses as the envelope"
        );
        assert_eq!(stream_envelope_line(""), None, "empty transcript never defaults to ok");
    }

    #[test]
    fn pkg_envelope_parses_through_the_unchanged_parser() {
        let action = parse_helper_stdout(
            PKG_INSTALL_VERB,
            r#"{"v":1,"action":"pkg-install","ok":true}"#,
        )
        .expect("pkg envelope keeps the v1 shape");
        assert_eq!(action, "pkg-install");
        let err = parse_helper_stdout(
            PKG_REMOVE_VERB,
            r#"{"v":1,"action":"pkg-remove","ok":false,"error":"pacman-exit-1"}"#,
        )
        .expect_err("failed transactions never parse as ok");
        assert!(matches!(err, ArchelError::HelperProtocol(_)));
    }

    #[test]
    fn collector_tees_live_lines_and_accumulates_the_transcript() {
        use std::io::Cursor;
        let fixture = "line one\nline two\n";
        let mut seen = Vec::new();
        let transcript =
            collect_streaming(Cursor::new(fixture), &mut |line: &str| {
                seen.push(line.to_string())
            });
        assert_eq!(seen, vec!["line one".to_string(), "line two".to_string()]);
        assert_eq!(transcript, fixture, "transcript holds exactly the live lines");
    }

    #[test]
    fn reader_pattern_drains_and_reports_the_exit_code() {
        use std::io::BufReader;
        // Proves the duct usage the privileged path relies on:
        // unchecked reader drains live, try_wait reports the code after.
        let handle = duct::cmd("echo", ["hello"])
            .unchecked()
            .reader()
            .expect("echo must spawn on this machine");
        let mut seen = Vec::new();
        let transcript =
            collect_streaming(BufReader::new(&handle), &mut |line: &str| {
                seen.push(line.to_string())
            });
        let code = handle
            .try_wait()
            .expect("wait must succeed after EOF")
            .and_then(|output| output.status.code());
        assert_eq!(seen, vec!["hello".to_string()]);
        assert!(transcript.contains("hello"));
        assert_eq!(code, Some(0));
    }

    #[test]
    fn pkg_cancel_and_deny_name_the_french_action() {
        rust_i18n::set_locale("fr");
        for (code, label_fn) in [
            (Some(126), pkg_install_action_label as fn() -> String),
            (Some(127), pkg_remove_action_label as fn() -> String),
            (Some(126), pkg_upgrade_action_label as fn() -> String),
        ] {
            let label = label_fn();
            let rendered = map_pkexec_exit(&label, code)
                .expect_err("126/127 must fail")
                .message();
            assert!(
                rendered.contains(&label),
                "126/127 path must name the French action: {rendered}"
            );
        }
        rust_i18n::set_locale("fr");
        let cancelled = map_pkexec_exit(&pkg_install_action_label(), Some(126))
            .expect_err("126")
            .message();
        let denied = map_pkexec_exit(&pkg_install_action_label(), Some(127))
            .expect_err("127")
            .message();
        assert_ne!(cancelled, denied, "cancelled vs denied stay distinct per action");
    }
}
