//! Power view-model: active profile plus backend tag (read-only at
//! this stage - writes land in plan 07).
//!
//! Same proven shape as `models::landing`: `tick()` is a QML-called
//! invokable that only spawns - the async `power::power_status()` runs
//! on the gui runtime and the finished payload returns exclusively
//! through `CxxQtThread.queue`, whose closure is the only code allowed
//! to set properties (RESEARCH Pattern 1). Zero logic beyond the core
//! calls (PLAT-03): the profile list is rendered from the three PPD
//! wire words with the active flag, never re-derived.

use cxx_qt_lib::QString;

use super::super::poll::{BannerPolicy, PollGuard};
use crate::gui::modal::{self, ModalSession};
use crate::gui::models::system::NOT_AVAILABLE;

/// Fully formatted tick payload, built off the Qt thread so the
/// `queue()` closure only sets properties.
pub(crate) struct AppliedPower {
    pub profile_text: String,
    pub backend_text: String,
    pub note_text: String,
    pub profiles_json: String,
    pub banner_visible: bool,
}

/// Backing state: property fields plus the poll guard and banner
/// policy. No shared core handle: every tick is an independent read.
pub struct PowerModelRust {
    pub(crate) profile_text: QString,
    pub(crate) backend_text: QString,
    pub(crate) note_text: QString,
    pub(crate) profiles_json: QString,
    pub(crate) banner_visible: bool,
    pub(crate) operation_busy: bool,
    guard: PollGuard,
    banner: BannerPolicy,
}

impl Default for PowerModelRust {
    fn default() -> Self {
        Self {
            profile_text: QString::from(""),
            backend_text: QString::from(""),
            note_text: QString::from(""),
            profiles_json: QString::from(""),
            banner_visible: false,
            operation_busy: false,
            guard: PollGuard::new(),
            banner: BannerPolicy::new(),
        }
    }
}

/// The three PPD profiles as QML-ready JSON: `[{id, active}]`, built
/// from the core `PowerProfile` values with their wire words - the
/// tab highlights the active row and stays read-only.
pub(crate) fn profiles_json_for(active: archel_core::power::PowerProfile) -> String {
    use archel_core::power::{PowerProfile, profile_arg_name};
    let rows: Vec<serde_json::Value> = [
        PowerProfile::Performance,
        PowerProfile::Balanced,
        PowerProfile::PowerSaver,
    ]
    .iter()
    .map(|profile| {
        serde_json::json!({
            "id": profile_arg_name(*profile),
            "active": *profile == active,
        })
    })
    .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string())
}

impl PowerModelRust {
    /// Applies a finished worker outcome on the Qt thread. `Err`
    /// reads N/A everywhere plus the once-per-episode banner; `Ok`
    /// renders the wire words straight from core helpers.
    fn apply_outcome(
        &mut self,
        outcome: Result<archel_core::power::PowerStatus, archel_core::ArchelError>,
    ) -> AppliedPower {
        match outcome {
            Ok(status) => {
                self.banner.on_success();
                AppliedPower {
                    profile_text: archel_core::power::profile_arg_name(status.profile)
                        .to_string(),
                    backend_text: archel_core::power::backend_tag(status.backend).to_string(),
                    note_text: status.note.unwrap_or_default(),
                    profiles_json: profiles_json_for(status.profile),
                    banner_visible: false,
                }
            }
            Err(_) => {
                self.banner.on_failure();
                AppliedPower {
                    profile_text: NOT_AVAILABLE.to_string(),
                    backend_text: NOT_AVAILABLE.to_string(),
                    note_text: String::new(),
                    profiles_json: "[]".to_string(),
                    banner_visible: self.banner.wants_visible(),
                }
            }
        }
    }
}

impl crate::gui::bridge_power::qobject::PowerModel {
    /// 1 Hz entry point: skips while a tick is in flight (D-02),
    /// spawns the async status, marshals back through `queue()` only.
    pub fn tick(mut self: core::pin::Pin<&mut Self>) {
        use cxx_qt::{CxxQtType, Threading};
        let tag = match self.as_mut().rust_mut().guard.try_begin() {
            Some(tag) => tag,
            None => return,
        };
        let qt_thread = self.qt_thread();
        let runtime = super::super::RUNTIME
            .get()
            .expect("gui runtime is set before Qt starts");
        runtime.spawn(async move {
            let outcome = archel_core::power::power_status().await;
            let _ = qt_thread.queue(
                move |mut obj: core::pin::Pin<&mut Self>| {
                    let applied = {
                        let mut rust = obj.as_mut().rust_mut();
                        if !rust.guard.is_newest(tag) {
                            rust.guard.finish();
                            return;
                        }
                        rust.guard.finish();
                        rust.apply_outcome(outcome)
                    };
                    obj.as_mut()
                        .set_profile_text(QString::from(&applied.profile_text));
                    obj.as_mut()
                        .set_backend_text(QString::from(&applied.backend_text));
                    obj.as_mut()
                        .set_note_text(QString::from(&applied.note_text));
                    obj.as_mut()
                        .set_profiles_json(QString::from(&applied.profiles_json));
                    obj.as_mut().set_banner_visible(applied.banner_visible);
                },
            );
        });
    }

    /// Hides the D-05 banner after the user dismisses it.
    pub fn dismiss_banner(mut self: core::pin::Pin<&mut Self>) {
        use cxx_qt::CxxQtType;
        self.as_mut().rust_mut().banner.dismiss();
        self.as_mut().set_banner_visible(false);
    }

    /// Switches the power profile (D-12): the question is the core's
    /// own `confirm_set_prompt` verbatim; only a yes reaches the
    /// backend switch, which runs on the blocking pool (D-Bus + a
    /// possible cpufreq sysfs write). While in flight the segmented
    /// control shows a spinner and stays disabled (D-15); on done a
    /// PollGuard refresh reads the new profile back from PPD.
    pub fn set_profile(mut self: core::pin::Pin<&mut Self>, profile: QString) {
        let text = profile.to_string();
        if !self.as_mut().begin_write_operation() {
            return;
        }
        let qt_thread = {
            use cxx_qt::Threading;
            self.qt_thread()
        };
        let runtime = super::super::RUNTIME
            .get()
            .expect("gui runtime is set before Qt starts");
        runtime.spawn_blocking(move || {
            power_worker(&text, &PROD_POWER_WRITE_OPS, modal::session());
            let _ = qt_thread.queue(|mut obj: core::pin::Pin<&mut Self>| {
                obj.as_mut().finish_write_operation();
            });
        });
    }

    /// Claims the single-operation slot (one profile switch at a time;
    /// the second click of a double-click loses here) and mirrors the
    /// claim onto the QML-visible flag so the segmented control can
    /// disable itself and show the spinner (D-15).
    fn begin_write_operation(mut self: core::pin::Pin<&mut Self>) -> bool {
        use cxx_qt::CxxQtType;
        if self.as_mut().rust_mut().operation_busy {
            return false;
        }
        self.as_mut().rust_mut().operation_busy = true;
        self.as_mut().set_operation_busy(true);
        true
    }

    /// Releases the operation slot and refreshes the status so the
    /// tab reflects the switch that just ended. Called from the
    /// `queue()` closure only (property writes stay on the Qt thread,
    /// RESEARCH Pattern 1).
    fn finish_write_operation(mut self: core::pin::Pin<&mut Self>) {
        use cxx_qt::CxxQtType;
        self.as_mut().rust_mut().operation_busy = false;
        self.as_mut().set_operation_busy(false);
        self.as_mut().tick();
    }
}

/// Injection seam for the write path: the exact core entry points
/// the worker calls, as plain function pointers. Production passes
/// the real core functions ([`PROD_POWER_WRITE_OPS`]); unit tests
/// pass fakes so the confirm choreography is proven without touching
/// D-Bus or sysfs (the subprocess/IPC contract is the core's tested
/// territory - the GUI never holds a child, a pid or a signal).
pub(crate) struct PowerWriteOps {
    pub parse: fn(&str) -> Option<archel_core::power::PowerProfile>,
    pub prompt: fn(archel_core::power::PowerProfile) -> String,
    pub apply:
        fn(archel_core::power::PowerProfile) -> Result<archel_core::power::PowerStatus, archel_core::ArchelError>,
}

/// The production wiring: parse and prompt are the CLI-proven core
/// helpers; apply is the confirm-free backend switch over the same
/// dispatch [`archel_core::power::power_set`] uses (thin-view
/// contract; PLAT-03). Blocking is fine - the worker runs on the
/// blocking pool.
pub(crate) const PROD_POWER_WRITE_OPS: PowerWriteOps = PowerWriteOps {
    parse: archel_core::power::parse_profile_arg,
    prompt: archel_core::power::confirm_set_prompt,
    apply: |profile| {
        tokio::runtime::Handle::current()
            .block_on(archel_core::power::power_set_direct(profile))
    },
};

/// Profile-switch worker body (runs on the blocking pool): the word
/// is parsed first so an unknown profile is rejected with the honest
/// typed error BEFORE any confirmation is shown; the question is the
/// core prompt verbatim; only a yes reaches the backend switch; a
/// backend failure surfaces as core's readable `message()` in the
/// error modal (D-17), privilege shapes included.
pub(crate) fn power_worker(
    profile_arg: &str,
    ops: &PowerWriteOps,
    sess: &ModalSession,
) {
    let profile = match (ops.parse)(profile_arg) {
        Some(profile) => profile,
        None => {
            sess.open_error(
                &archel_core::ArchelError::Internal(format!(
                    "unknown power profile: '{profile_arg}'"
                ))
                .message(),
            );
            return;
        }
    };
    let rx = sess.request_confirm(&(ops.prompt)(profile));
    if !rx.recv().unwrap_or(false) {
        return;
    }
    if let Err(err) = (ops.apply)(profile) {
        sess.open_error(&err.message());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archel_core::power::{PowerBackend, PowerProfile, PowerStatus};

    fn ready_status() -> PowerStatus {
        PowerStatus {
            profile: PowerProfile::Balanced,
            backend: PowerBackend::Ppd,
            note: None,
        }
    }

    fn failed_outcome() -> Result<PowerStatus, archel_core::ArchelError> {
        Err(archel_core::ArchelError::Internal(
            "PPD bus down in test".to_string(),
        ))
    }

    #[test]
    fn error_path_reads_na_and_signals_banner_once() {
        let mut model = PowerModelRust::default();
        let first = model.apply_outcome(failed_outcome());
        assert_eq!(first.profile_text, NOT_AVAILABLE);
        assert_eq!(first.backend_text, NOT_AVAILABLE);
        assert_eq!(first.profiles_json, "[]");
        assert!(first.banner_visible, "first failure shows the banner");
        let second = model.apply_outcome(failed_outcome());
        assert!(
            second.banner_visible,
            "consecutive failures keep the banner without re-signalling"
        );
    }

    #[test]
    fn success_fills_profile_backend_and_row_shape() {
        let mut model = PowerModelRust::default();
        let _ = model.apply_outcome(failed_outcome());
        let applied = model.apply_outcome(Ok(ready_status()));
        assert!(!applied.banner_visible, "success hides the banner");
        assert_eq!(applied.profile_text, "balanced");
        assert_eq!(applied.backend_text, "power-profiles-daemon");
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(&applied.profiles_json).expect("profiles must be JSON");
        assert_eq!(rows.len(), 3, "all three PPD profiles render");
        let active: Vec<&serde_json::Value> =
            rows.iter().filter(|row| row["active"] == true).collect();
        assert_eq!(active.len(), 1, "exactly one row is active");
        assert_eq!(active[0]["id"], "balanced");
    }

    #[test]
    fn live_status_wires_without_panic() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime");
        let outcome = runtime.block_on(archel_core::power::power_status());
        let mut model = PowerModelRust::default();
        let applied = model.apply_outcome(outcome);
        assert!(
            !applied.profile_text.is_empty() && !applied.backend_text.is_empty(),
            "live status must fill or N/A, never blank: {:?} / {:?}",
            applied.profile_text,
            applied.backend_text
        );
    }

    // -- Write-path choreography (plan 06-07, Task 1) --
    //
    // The fakes below replace only the three `PowerWriteOps` seams;
    // the worker itself runs for real against a local ModalSession, so
    // parse-before-confirm, default-No and the verbatim question are
    // all proven without touching D-Bus or sysfs.

    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::time::{Duration, Instant};

    use crate::gui::modal::ModalKind;

    fn wait_until(what: &str, cond: impl Fn() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if cond() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("condition never held within 5s: {what}");
    }

    fn ok_status() -> PowerStatus {
        ready_status()
    }

    #[test]
    fn decline_short_circuits_before_any_backend_call() {
        static APPLY_CALLS: AtomicUsize = AtomicUsize::new(0);
        fn apply_must_not_run(
            _profile: PowerProfile,
        ) -> Result<PowerStatus, archel_core::ArchelError> {
            APPLY_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            Err(archel_core::ArchelError::Internal(
                "backend must never run after a decline".to_string(),
            ))
        }
        let ops = PowerWriteOps {
            parse: archel_core::power::parse_profile_arg,
            prompt: archel_core::power::confirm_set_prompt,
            apply: apply_must_not_run,
        };
        let sess = ModalSession::default();
        std::thread::scope(|s| {
            s.spawn(|| power_worker("performance", &ops, &sess));
            wait_until("profile confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            assert_eq!(
                sess.snapshot_question(),
                archel_core::power::confirm_set_prompt(PowerProfile::Performance),
                "the question is the core prompt verbatim"
            );
            sess.resolve_confirm(false);
        });
        assert_eq!(
            APPLY_CALLS.load(AtomicOrdering::SeqCst),
            0,
            "a decline must never reach the backend switch"
        );
        assert_eq!(sess.snapshot_kind(), ModalKind::None);
    }

    #[test]
    fn accept_reaches_the_backend_and_surfaces_its_error_verbatim() {
        static APPLY_CALLS: AtomicUsize = AtomicUsize::new(0);
        fn apply_fails(
            _profile: PowerProfile,
        ) -> Result<PowerStatus, archel_core::ArchelError> {
            APPLY_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            Err(archel_core::ArchelError::Internal(
                "PPD set ActiveProfile: backend down in test".to_string(),
            ))
        }
        let ops = PowerWriteOps {
            parse: archel_core::power::parse_profile_arg,
            prompt: archel_core::power::confirm_set_prompt,
            apply: apply_fails,
        };
        let sess = ModalSession::default();
        std::thread::scope(|s| {
            s.spawn(|| power_worker("balanced", &ops, &sess));
            wait_until("profile confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            sess.resolve_confirm(true);
            wait_until("backend error opens", || {
                sess.snapshot_kind() == ModalKind::Error
            });
        });
        assert_eq!(APPLY_CALLS.load(AtomicOrdering::SeqCst), 1);
        assert!(
            sess.snapshot_error()
                == archel_core::ArchelError::Internal(
                    "PPD set ActiveProfile: backend down in test".to_string()
                )
                .message(),
            "the error modal carries core's readable message"
        );
    }

    #[test]
    fn unknown_profile_is_rejected_before_any_confirm_or_backend() {
        static APPLY_CALLS: AtomicUsize = AtomicUsize::new(0);
        fn apply_must_not_run(
            _profile: PowerProfile,
        ) -> Result<PowerStatus, archel_core::ArchelError> {
            APPLY_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(ok_status())
        }
        let ops = PowerWriteOps {
            parse: archel_core::power::parse_profile_arg,
            prompt: archel_core::power::confirm_set_prompt,
            apply: apply_must_not_run,
        };
        let sess = ModalSession::default();
        power_worker("turbo-ultra", &ops, &sess);
        assert_eq!(sess.snapshot_kind(), ModalKind::Error);
        assert_eq!(APPLY_CALLS.load(AtomicOrdering::SeqCst), 0);
    }
}
