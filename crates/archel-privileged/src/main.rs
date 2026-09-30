//! Privileged helper (per D-03): invoked via pkexec, argv-allowlisted.
//!
//! Only the exact argv `["dry-run"]` is honored: it prints the versioned
//! JSON envelope `{"v":1,...}` to stdout and exits 0. Anything else is
//! rejected with exit code 2 plus machine-readable error JSON.
//!
//! Security rules (T-1-03): no shell, no prefix matching, absolute paths
//! only on the caller side, writes nothing (pkexec runs with the target
//! home and a minimal env, so no `$HOME` assumption is made here).

/// Protocol version spoken on stdout. The core client rejects any
/// envelope whose `v` differs (never default-ok).
pub const PROTOCOL_VERSION: u32 = 1;

/// JSON printed for the single allowlisted action.
fn dry_run_json() -> &'static str {
    r#"{"v":1,"action":"dry-run","ok":true}"#
}

/// JSON printed for every rejected invocation.
fn rejection_json() -> &'static str {
    r#"{"v":1,"ok":false,"error":"action-not-allowlisted"}"#
}

/// Pure allowlist decision: the exit code plus the stdout payload.
/// Kept pure so the matrix tests run without root and without spawning.
fn handle_args(args: &[String]) -> (i32, &'static str) {
    if args.len() == 1 && args[0] == "dry-run" {
        (0, dry_run_json())
    } else {
        (2, rejection_json())
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (code, payload) = handle_args(&args);
    println!("{payload}");
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn dry_run_returns_exit_zero_with_v1_json() {
        let (code, payload) = handle_args(&args(&["dry-run"]));
        assert_eq!(code, 0);
        assert!(payload.contains("\"v\":1"), "envelope must be v1: {payload}");
        assert!(payload.contains("\"action\":\"dry-run\""));
        assert!(payload.contains("\"ok\":true"));
    }

    #[test]
    fn smuggled_extra_arg_is_rejected_with_exit_two() {
        let (code, payload) = handle_args(&args(&["dry-run", "extra"]));
        assert_eq!(code, 2);
        assert!(payload.contains("\"v\":1"));
        assert!(payload.contains("\"ok\":false"));
        assert!(payload.contains("action-not-allowlisted"));
    }

    #[test]
    fn unknown_action_is_rejected_with_exit_two() {
        let (code, payload) = handle_args(&args(&["install", "foo"]));
        assert_eq!(code, 2);
        assert!(payload.contains("\"ok\":false"));
    }

    #[test]
    fn empty_argv_is_rejected_with_exit_two() {
        let (code, _) = handle_args(&args(&[]));
        assert_eq!(code, 2);
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
}
