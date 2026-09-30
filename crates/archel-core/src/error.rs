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
