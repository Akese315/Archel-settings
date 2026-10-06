//! Typed error taxonomy for Archel (per D-08: English identifiers).
//!
//! User-facing rendering is French (per D-05) and comes exclusively from
//! `locales/fr.json` through `t!` keys (per D-07): [`ArchelError::message`]
//! is the sole terminal renderer. The `Display` impl (via `thiserror`) stays
//! English and log-oriented — it never reaches the terminal; the binary
//! prints `message()` on stderr. Raw internal details travel in
//! `Display`/logs only, never inside catalog strings (T-1-02).

use thiserror::Error;

/// Why a privilege request failed (D-12: cancelled vs denied wording).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthFailureReason {
    /// The user dismissed the polkit dialog (pkexec exit 126).
    Cancelled,
    /// Polkit refused the authorization (pkexec exit 127).
    Denied,
}

/// Every failure the core can report. No panics on failure paths: fallible
/// core APIs return `Result<_, ArchelError>` (T-1-01 no-default-ok rule).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ArchelError {
    /// Privilege escalation failed for `action` (cancelled or denied).
    #[error("authentication failed for action '{action}': {reason:?}")]
    AuthFailed {
        action: String,
        reason: AuthFailureReason,
    },
    /// The privileged helper answered outside the versioned JSON protocol.
    #[error("privileged helper protocol error: {0}")]
    HelperProtocol(String),
    /// No mounted filesystem has `/` as its mount point, so the
    /// system-partition disk (D-08) cannot be reported. Fails typed
    /// instead of showing a non-root filesystem (Pitfall 4).
    #[error("system partition '/' not found among mounted disks")]
    SystemPartitionNotFound,
    /// A hardware sensor (GPU, RAPL domain, hwmon node) could not be
    /// read. Carries the sensor name for logs; the user only ever sees
    /// the catalog N/A string, and callers degrade to `None` fields
    /// instead of failing (D-10).
    #[error("sensor unavailable: '{sensor}'")]
    SensorUnavailable { sensor: String },
    /// An update checker binary (`checkupdates`, `yay`) is not installed.
    /// Carries the missing tool name for logs; the user only ever sees the
    /// catalog hint, and callers degrade (stale-with-age) instead of
    /// failing (D-06).
    #[error("update checker not available: '{tool}' is not installed")]
    UpdatesCheckerMissing { tool: String },
    /// No usable network state: neither the NM D-Bus service nor the
    /// `nmcli` fallback answered. Carries log-only detail; the user sees
    /// the catalog hint (check NM is running), never this payload.
    #[error("network unavailable: {detail}")]
    NetworkUnavailable { detail: String },
    /// A named package has no repo or AUR record. Carries the name for
    /// logs; the user sees the catalog French line (used by `pkg info`).
    #[error("package not found: '{name}'")]
    PackageNotFound { name: String },
    /// A PKGBUILD failed the static `bash -n` gate. Carries the name for
    /// the catalog line plus the log-only syntax detail (stderr text never
    /// reaches the terminal, T-3-11): nothing was built, nothing changed.
    #[error("PKGBUILD rejected for '{name}': {reason}")]
    PkgbuildRejected { name: String, reason: String },
    /// The `yay` binary is not installed, so AUR search and review
    /// cannot run. The user sees the catalog install hint, never a
    /// crash (PKG-05 yay-missing path).
    #[error("yay is not installed")]
    YayMissing,
    /// The user declined the D-01 confirmation prompt (or answered
    /// anything but yes): the mutation never ran, nothing changed.
    /// The user sees the catalog abort line, never a stack trace.
    #[error("mutation declined at the confirmation prompt")]
    ConfirmationAborted,
    /// A package mutation was requested without an interactive
    /// terminal on stdin (piped input, script, `--json` batch): the
    /// safety check refuses to mutate blindly. The user sees the
    /// catalog refusal, nothing executes.
    #[error("package mutation requires an interactive terminal")]
    NeedsTty,
    /// A package name failed the client-side charset gate (leading
    /// dash, shell metacharacter, empty): fast-fail before any pkexec
    /// prompt. The helper re-validates every name server-side (T-3-04).
    #[error("invalid package name: '{name}'")]
    InvalidPackageName { name: String },
    /// A single-package upgrade was requested: the partial-upgrade
    /// equivalent is refused (D-03) — only the full `pkg upgrade`
    /// exists. The user sees the catalog refusal, nothing executes.
    #[error("partial upgrade refused for '{name}'")]
    PartialUpgradeRefused { name: String },
    /// `archel settings` invoked on a build without the `gui` feature.
    #[error("graphical interface not compiled into this build")]
    GuiNotCompiled,
    /// Unclassified core failure. Carries log detail; the user only ever
    /// sees the curated catalog string, never this payload (T-1-02).
    #[error("internal error: {0}")]
    Internal(String),
}

impl ArchelError {
    /// Friendly constructor for a dismissed auth dialog, naming the
    /// rejected action (D-12).
    pub fn privilege_cancelled(action: impl Into<String>) -> Self {
        Self::AuthFailed {
            action: action.into(),
            reason: AuthFailureReason::Cancelled,
        }
    }

    /// Friendly constructor for a refused authorization, naming the
    /// rejected action (D-12).
    pub fn privilege_denied(action: impl Into<String>) -> Self {
        Self::AuthFailed {
            action: action.into(),
            reason: AuthFailureReason::Denied,
        }
    }

    /// Friendly constructor for an unreachable network stack, naming
    /// the log-only detail (no bus, no `nmcli`, no active profile).
    pub fn network_unavailable(detail: impl Into<String>) -> Self {
        Self::NetworkUnavailable {
            detail: detail.into(),
        }
    }

    /// French user-facing rendering from the catalog — the ONLY string that
    /// may reach the terminal for this error (D-05/D-07/D-12).
    pub fn message(&self) -> String {
        match self {
            Self::AuthFailed {
                action,
                reason: AuthFailureReason::Cancelled,
            } => rust_i18n::t!("errors.auth_cancelled", action = action).to_string(),
            Self::AuthFailed {
                action,
                reason: AuthFailureReason::Denied,
            } => rust_i18n::t!("errors.auth_denied", action = action).to_string(),
            Self::HelperProtocol(_) => rust_i18n::t!("errors.helper_protocol").to_string(),
            Self::SystemPartitionNotFound => {
                rust_i18n::t!("errors.system_partition_not_found").to_string()
            }
            Self::SensorUnavailable { .. } => {
                rust_i18n::t!("errors.sensor_unavailable").to_string()
            }
            Self::UpdatesCheckerMissing { tool } => {
                rust_i18n::t!("errors.updates_checker_missing", tool = tool).to_string()
            }
            Self::NetworkUnavailable { .. } => {
                rust_i18n::t!("errors.network_unavailable").to_string()
            }
            Self::PackageNotFound { name } => {
                rust_i18n::t!("errors.package_not_found", name = name).to_string()
            }
            Self::PkgbuildRejected { name, .. } => {
                rust_i18n::t!("errors.pkgbuild_rejected", name = name).to_string()
            }
            Self::YayMissing => rust_i18n::t!("errors.yay_missing").to_string(),
            Self::ConfirmationAborted => {
                rust_i18n::t!("errors.confirmation_aborted").to_string()
            }
            Self::NeedsTty => rust_i18n::t!("errors.needs_tty").to_string(),
            Self::InvalidPackageName { name } => {
                rust_i18n::t!("errors.invalid_package_name", name = name).to_string()
            }
            Self::PartialUpgradeRefused { name } => {
                rust_i18n::t!("errors.partial_upgrade_refused", name = name).to_string()
            }
            Self::GuiNotCompiled => rust_i18n::t!("errors.gui_not_compiled").to_string(),
            Self::Internal(_) => rust_i18n::t!("errors.internal").to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every_variant() -> Vec<ArchelError> {
        vec![
            ArchelError::privilege_cancelled("package install"),
            ArchelError::privilege_denied("package install"),
            ArchelError::HelperProtocol("exit=3".to_string()),
            ArchelError::SystemPartitionNotFound,
            ArchelError::SensorUnavailable {
                sensor: "gpu".to_string(),
            },
            ArchelError::UpdatesCheckerMissing {
                tool: "checkupdates".to_string(),
            },
            ArchelError::NetworkUnavailable {
                detail: "no bus".to_string(),
            },
            ArchelError::PackageNotFound {
                name: "firefox".to_string(),
            },
            ArchelError::PkgbuildRejected {
                name: "some-aur-pkg".to_string(),
                reason: "syntax error near unexpected token".to_string(),
            },
            ArchelError::YayMissing,
            ArchelError::ConfirmationAborted,
            ArchelError::NeedsTty,
            ArchelError::InvalidPackageName {
                name: "-foo".to_string(),
            },
            ArchelError::PartialUpgradeRefused {
                name: "firefox".to_string(),
            },
            ArchelError::GuiNotCompiled,
            ArchelError::Internal("boom".to_string()),
        ]
    }

    #[test]
    fn every_variant_renders_non_empty_french() {
        rust_i18n::set_locale("fr");
        for err in every_variant() {
            let rendered = err.message();
            assert!(
                !rendered.trim().is_empty(),
                "empty French rendering for {err:?}"
            );
        }
    }

    #[test]
    fn gui_error_comes_from_catalog() {
        rust_i18n::set_locale("fr");
        assert!(
            ArchelError::GuiNotCompiled
                .message()
                .contains("graphique"),
            "GuiNotCompiled must render the catalog French string"
        );
    }

    #[test]
    fn auth_errors_name_the_rejected_action() {
        rust_i18n::set_locale("fr");
        let action = "package install";
        let cancelled = ArchelError::privilege_cancelled(action).message();
        let denied = ArchelError::privilege_denied(action).message();
        assert!(
            cancelled.contains(action),
            "cancelled error must name the action: {cancelled}"
        );
        assert!(
            denied.contains(action),
            "denied error must name the action: {denied}"
        );
        assert_ne!(
            cancelled, denied,
            "cancelled vs denied wording must differ (D-12)"
        );
    }

    #[test]
    fn failure_path_returns_err_instead_of_panicking() {
        fn fallible(fail: bool) -> Result<(), ArchelError> {
            if fail {
                Err(ArchelError::Internal("boom".to_string()))
            } else {
                Ok(())
            }
        }
        assert!(fallible(true).is_err());
        assert!(fallible(false).is_ok());
    }

    #[test]
    fn rendering_never_panics_on_edge_inputs() {
        rust_i18n::set_locale("fr");
        let edge_cases = vec![
            ArchelError::privilege_cancelled(""),
            ArchelError::privilege_denied(""),
            ArchelError::HelperProtocol(String::new()),
            ArchelError::Internal(String::new()),
        ];
        for err in edge_cases {
            let _ = err.message();
        }
    }
}
