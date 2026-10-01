//! Privileged helper: invoked via pkexec, argv-allowlisted.
//!
//! Allowlisted shapes (T-3-04/T-3-05):
//! - `["dry-run"]` prints the versioned JSON envelope and exits 0.
//! - `["pkg-upgrade"]` (exact match, zero extra args) runs a full
//!   `pacman -Syu` (the ONLY refresh path; no partial-upgrade verb exists).
//! - `["pkg-install", pkgs...]` / `["pkg-remove", pkgs...]` run
//!   `pacman -S/-R` over strictly-validated package names, one or more.
//! Anything else is rejected with exit code 2 plus machine-readable
//! error JSON, and nothing executes.
//!
//! Security rules (T-3-04): no shell, absolute `/usr/bin/pacman` path,
//! every package arg gated on charset plus leading-dash reject plus
//! non-empty, `--` inserted before the package list, ALL flags hardcoded
//! server-side (never taken from argv). The helper re-validates every
//! invocation and never trusts the CLI.

/// Protocol version spoken on stdout. The core client rejects any
/// envelope whose `v` differs (never default-ok).
pub const PROTOCOL_VERSION: u32 = 1;

/// Absolute pacman path: the helper never resolves the transaction
/// tool through `$PATH`, so a trojaned PATH entry cannot redirect a
/// root transaction (spoofing mitigation).
pub const PACMAN_PATH: &str = "/usr/bin/pacman";

/// JSON printed for the single allowlisted probe action.
fn dry_run_json() -> &'static str {
    r#"{"v":1,"action":"dry-run","ok":true}"#
}

/// JSON printed for every rejected invocation.
fn rejection_json() -> &'static str {
    r#"{"v":1,"ok":false,"error":"action-not-allowlisted"}"#
}

/// Pure allowlist decision: either answer directly (print payload, exit
/// with the code) or spawn pacman with the exact argv built here.
/// Kept pure so the matrix tests run without root and without spawning.
enum Decision {
    /// Print the payload on stdout, exit with the code.
    Respond(i32, &'static str),
    /// Spawn `/usr/bin/pacman` with these exact arguments (flags are
    /// hardcoded at the construction site, never taken from argv).
    RunPacman {
        action: &'static str,
        argv: Vec<String>,
    },
}

/// A package name is accepted only when it is non-empty, does not start
/// with `-` (flag-injection reject), and uses the strict charset
/// (no shell metacharacters, no path separators, no whitespace).
fn is_valid_package_name(name: &str) -> bool {
    if name.is_empty() || name.starts_with('-') {
        return false;
    }
    name.chars().all(|c| {
        c.is_ascii_alphanumeric() || matches!(c, '@' | '.' | '_' | '+' | '-')
    })
}

/// Pure allowlist entry point. `dry-run` and `pkg-upgrade` are exact
/// matches; `pkg-install` / `pkg-remove` take one or more validated
/// package names. Every other shape is a rejection (exit 2, v1
/// envelope) so the core client needs no new parsing.
fn handle_args(args: &[String]) -> Decision {
    if args.len() == 1 && args[0] == "dry-run" {
        return Decision::Respond(0, dry_run_json());
    }
    // Mutation verbs are validated here (RED stub: rejected until the
    // GREEN step wires the pacman argv mapping).
    let _ = is_valid_package_name;
    Decision::Respond(2, rejection_json())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match handle_args(&args) {
        Decision::Respond(code, payload) => {
            println!("{payload}");
            std::process::exit(code);
        }
        Decision::RunPacman { action: _, argv: _ } => {
            // GREEN step: spawn pacman with inherited stdio, print the
            // v1 envelope last, exit with the transaction status.
            std::process::exit(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    fn code_of(decision: Decision) -> (i32, Option<&'static str>) {
        match decision {
            Decision::Respond(code, payload) => (code, Some(payload)),
            Decision::RunPacman { .. } => (0, None),
        }
    }

    #[test]
    fn dry_run_returns_exit_zero_with_v1_json() {
        let (code, payload) = code_of(handle_args(&args(&["dry-run"])));
        let payload = payload.expect("dry-run answers directly");
        assert_eq!(code, 0);
        assert!(payload.contains("\"v\":1"), "envelope must be v1: {payload}");
        assert!(payload.contains("\"action\":\"dry-run\""));
        assert!(payload.contains("\"ok\":true"));
    }

    #[test]
    fn smuggled_extra_arg_is_rejected_with_exit_two() {
        let (code, payload) = code_of(handle_args(&args(&["dry-run", "extra"])));
        let payload = payload.expect("rejections answer directly");
        assert_eq!(code, 2);
        assert!(payload.contains("\"v\":1"));
        assert!(payload.contains("\"ok\":false"));
        assert!(payload.contains("action-not-allowlisted"));
    }

    #[test]
    fn unknown_action_is_rejected_with_exit_two() {
        let (code, payload) = code_of(handle_args(&args(&["install", "foo"])));
        assert_eq!(code, 2);
        assert!(payload.expect("rejections answer directly").contains("\"ok\":false"));
    }

    #[test]
    fn empty_argv_is_rejected_with_exit_two() {
        let (code, _) = code_of(handle_args(&args(&[])));
        assert_eq!(code, 2);
    }

    #[test]
    fn prefix_match_does_not_pass_the_allowlist() {
        // "dry-runner" shares a prefix with "dry-run" but must be rejected:
        // prefix matching near a root process is a privilege hole (T-1-03).
        let (code, _) = code_of(handle_args(&args(&["dry-runner"])));
        assert_eq!(code, 2);
        let (code, _) = code_of(handle_args(&args(&["dry-"])));
        assert_eq!(code, 2);
    }

    // --- Mutation verb matrix (T-3-04): pkg-upgrade exact-match,
    // --- pkg-install / pkg-remove with strictly-validated args.

    #[test]
    fn pkg_upgrade_with_zero_extra_args_is_accepted() {
        let (code, payload) = code_of(handle_args(&args(&["pkg-upgrade"])));
        assert_eq!(code, 0, "exact-match upgrade must be accepted");
        assert!(payload.is_none(), "upgrade spawns pacman, it does not answer directly");
    }

    #[test]
    fn pkg_upgrade_with_extra_args_is_rejected() {
        for extra in [
            &["pkg-upgrade", "firefox"][..],
            &["pkg-upgrade", "--noconfirm"][..],
        ] {
            let (code, payload) = code_of(handle_args(&args(&extra)));
            assert_eq!(code, 2, "upgrade takes no args: {extra:?}");
            assert!(payload.expect("rejections answer directly").contains("\"v\":1"));
        }
    }

    #[test]
    fn pkg_install_with_one_package_is_accepted() {
        let (code, payload) = code_of(handle_args(&args(&["pkg-install", "firefox"])));
        assert_eq!(code, 0, "valid install must be accepted");
        assert!(payload.is_none(), "install spawns pacman, it does not answer directly");
    }

    #[test]
    fn pkg_install_with_zero_packages_is_rejected() {
        let (code, payload) = code_of(handle_args(&args(&["pkg-install"])));
        assert_eq!(code, 2, "zero-package install must be rejected");
        assert!(payload.expect("rejections answer directly").contains("\"v\":1"));
    }

    #[test]
    fn crafted_package_names_are_rejected_with_exit_two() {
        for injection in [
            "-foo",
            "--overwrite",
            "-Syu",
            "foo;bar",
            "foo|bar",
            "$(reboot)",
            "foo/bar",
            "foo bar",
            "",
        ] {
            let (code, payload) =
                code_of(handle_args(&args(&["pkg-install", injection])));
            assert_eq!(code, 2, "injection must be rejected: {injection:?}");
            assert!(
                payload.expect("rejections answer directly").contains("\"v\":1"),
                "rejection keeps the v1 envelope: {injection:?}"
            );
        }
    }

    #[test]
    fn smuggled_extra_flag_after_a_valid_package_is_rejected() {
        for extra in [
            &["pkg-install", "firefox", "--overwrite"][..],
            &["pkg-install", "firefox", "-foo"][..],
            &["pkg-remove", "firefox", "--noconfirm"][..],
        ] {
            let (code, _) = code_of(handle_args(&args(&extra)));
            assert_eq!(code, 2, "smuggled flag must be rejected: {extra:?}");
        }
    }

    #[test]
    fn pkg_remove_follows_the_same_validator_as_install() {
        let (code, _) = code_of(handle_args(&args(&["pkg-remove", "firefox"])));
        assert_eq!(code, 0, "valid remove must be accepted");
        let (code, _) = code_of(handle_args(&args(&["pkg-remove"])));
        assert_eq!(code, 2, "zero-package remove must be rejected");
        let (code, _) = code_of(handle_args(&args(&["pkg-remove", "--overwrite"])));
        assert_eq!(code, 2, "flag-shaped remove arg must be rejected");
    }

    #[test]
    fn dry_run_prefix_variants_stay_rejected() {
        for probe in [
            &["dry-runner"][..],
            &["pkg-installer", "firefox"][..],
            &["pkg-upgrade-foo"][..],
            &["pkg"][..],
        ] {
            let (code, _) = code_of(handle_args(&args(&probe)));
            assert_eq!(code, 2, "prefix variant must be rejected: {probe:?}");
        }
    }
}
