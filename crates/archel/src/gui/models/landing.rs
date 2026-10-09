//! Landing view-model: cached update counts plus version/kernel
//! snapshot (D-07).
//!
//! Same proven shape as `models::system`: `tick()` only spawns - the
//! cache-backed `landing::snapshot(false)` runs on the gui runtime's
//! blocking pool (it may shell out to checkers on a cold cache) and the
//! finished snapshot returns exclusively through `CxxQtThread.queue`.
//! `force_refresh` stays false at 1 Hz: the core 15-minute TTL owns
//! refresh timing, the GUI never forces one.

use cxx_qt_lib::QString;

use super::super::poll::{BannerPolicy, PollGuard};
use crate::gui::models::system::NOT_AVAILABLE;

/// Fully formatted tick payload, built on the Qt thread from the worker's
/// snapshot so the `queue()` closure stays property-sets only.
pub(crate) struct AppliedLanding {
    pub version_text: String,
    pub kernel_text: String,
    pub updates_text: String,
    pub notes_text: String,
    pub banner_visible: bool,
}

/// Backing state: property fields plus the poll guard and banner policy.
/// No shared core handle needed: every tick is an independent
/// cache-backed snapshot.
pub struct LandingModelRust {
    pub(crate) version_text: QString,
    pub(crate) kernel_text: QString,
    pub(crate) updates_text: QString,
    pub(crate) notes_text: QString,
    pub(crate) banner_visible: bool,
    guard: PollGuard,
    banner: BannerPolicy,
}

impl Default for LandingModelRust {
    fn default() -> Self {
        Self {
            version_text: QString::from(""),
            kernel_text: QString::from(""),
            updates_text: QString::from(""),
            notes_text: QString::from(""),
            banner_visible: false,
            guard: PollGuard::new(),
            banner: BannerPolicy::new(),
        }
    }
}

impl LandingModelRust {
    /// Applies a finished worker snapshot on the Qt thread. `Err` (kernel
    /// unreadable - the only hard failure) reads N/A everywhere plus the
    /// once-per-episode banner; update halves degrade inside core and
    /// never fail this path.
    fn apply_outcome(
        &mut self,
        outcome: Result<archel_core::landing::LandingSnapshot, archel_core::ArchelError>,
    ) -> AppliedLanding {
        match outcome {
            Ok(snap) => {
                self.banner.on_success();
                AppliedLanding {
                    version_text: snap.app_version.clone(),
                    kernel_text: snap.kernel_version.clone(),
                    updates_text: archel_core::landing::updates_cell(&snap),
                    notes_text: archel_core::landing::checker_notes(&snap).join("\n"),
                    banner_visible: false,
                }
            }
            Err(_) => {
                self.banner.on_failure();
                AppliedLanding {
                    version_text: NOT_AVAILABLE.to_string(),
                    kernel_text: NOT_AVAILABLE.to_string(),
                    updates_text: NOT_AVAILABLE.to_string(),
                    notes_text: String::new(),
                    banner_visible: self.banner.wants_visible(),
                }
            }
        }
    }
}

impl crate::gui::bridge_landing::qobject::LandingModel {
    /// 1 Hz entry point: skips while a tick is in flight (D-02), spawns
    /// the cache-backed snapshot, marshals back through `queue()` only.
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
        runtime.spawn_blocking(move || {
            let outcome = archel_core::landing::snapshot(false);
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
                        .set_version_text(QString::from(&applied.version_text));
                    obj.as_mut()
                        .set_kernel_text(QString::from(&applied.kernel_text));
                    obj.as_mut()
                        .set_updates_text(QString::from(&applied.updates_text));
                    obj.as_mut()
                        .set_notes_text(QString::from(&applied.notes_text));
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use archel_core::updates::{UnavailableReason, UpdateCount};

    fn ready_snapshot() -> archel_core::landing::LandingSnapshot {
        archel_core::landing::LandingSnapshot {
            app_version: "0.1.0".to_string(),
            kernel_version: "6.18-test".to_string(),
            repo: UpdateCount::Fresh(3),
            aur: UpdateCount::Fresh(2),
        }
    }

    fn failed_outcome() -> Result<archel_core::landing::LandingSnapshot, archel_core::ArchelError>
    {
        Err(archel_core::ArchelError::Internal(
            "kernel version unavailable from sysinfo".to_string(),
        ))
    }

    #[test]
    fn error_path_reads_na_and_signals_banner_once() {
        archel_core::i18n::set_language("fr");
        let mut model = LandingModelRust::default();
        let first = model.apply_outcome(failed_outcome());
        assert_eq!(first.updates_text, NOT_AVAILABLE);
        assert_eq!(first.version_text, NOT_AVAILABLE);
        assert!(first.banner_visible, "first failure shows the banner");
        let second = model.apply_outcome(failed_outcome());
        assert!(
            second.banner_visible,
            "consecutive failures keep the banner without re-signalling"
        );
        assert!(
            !second.updates_text.contains('0'),
            "error path must never fake zero counts"
        );
    }

    #[test]
    fn success_populates_counts_and_hides_banner() {
        archel_core::i18n::set_language("fr");
        let mut model = LandingModelRust::default();
        let _ = model.apply_outcome(failed_outcome());
        let applied = model.apply_outcome(Ok(ready_snapshot()));
        assert!(!applied.banner_visible, "success hides the banner");
        assert!(
            applied.updates_text.contains('5'),
            "total must show: {}",
            applied.updates_text
        );
        assert_eq!(applied.version_text, "0.1.0");
        assert_eq!(applied.kernel_text, "6.18-test");
    }

    #[test]
    fn live_snapshot_uses_the_cached_path() {
        archel_core::i18n::set_language("fr");
        let snap =
            archel_core::landing::snapshot(false).expect("landing renders on this machine");
        assert!(!snap.app_version.trim().is_empty());
        assert!(!snap.kernel_version.trim().is_empty());
        assert!(!archel_core::landing::updates_cell(&snap).is_empty());
    }
}
