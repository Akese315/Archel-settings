//! 1 Hz polling guards shared by every tab view-model (D-01/D-02, D-05).
//!
//! Two tiny state machines every model embeds:
//! - [`PollGuard`]: the Pitfall 7 in-flight flag plus the sequence tag that
//!   drops stale results (a slow tick must never overwrite a newer one).
//! - [`BannerPolicy`]: the D-05 first-failure banner rule (show once per
//!   episode, dismissible, silent afterwards, re-armed by a success).
//!
//! Both are Qt-free on purpose: the worker threads own the guards, the
//! `CxxQtThread.queue` closures only read the verdicts and set properties.
//! No French copy lives here (D-07): the banner sentence and the N/A token
//! are QML-side, this module only answers show/hide questions.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Per-model tick guard: at most one request in flight, stale answers
/// dropped by sequence tag (D-02, T-06-19).
///
/// The Qt thread calls [`try_begin`](PollGuard::try_begin) from `tick()`;
/// the worker's `queue()` closure calls [`is_newest`](PollGuard::is_newest)
/// before applying properties and always calls
/// [`finish`](PollGuard::finish) to release the flag.
#[derive(Debug, Default)]
pub struct PollGuard {
    busy: AtomicBool,
    seq: AtomicU64,
}

impl PollGuard {
    /// Idle guard, sequence at zero.
    pub fn new() -> Self {
        Self::default()
    }

    /// Claims the tick: `None` means a request is still in flight, so the
    /// caller returns early (skipped tick). `Some(tag)` is this dispatch's
    /// sequence tag, handed to the worker and checked on return.
    pub fn try_begin(&self) -> Option<u64> {
        if self
            .busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return None;
        }
        Some(self.seq.fetch_add(1, Ordering::SeqCst) + 1)
    }

    /// True when `tag` is still the newest dispatch: older answers are
    /// dropped instead of applied (stale-drop).
    pub fn is_newest(&self, tag: u64) -> bool {
        self.seq.load(Ordering::SeqCst) == tag
    }

    /// Releases the in-flight flag. Always runs, even for dropped results.
    pub fn finish(&self) {
        self.busy.store(false, Ordering::SeqCst);
    }
}

/// D-05 banner rule: the first poll failure of an episode shows the
/// dismissible banner once; later failures stay silent (fields fall back
/// to N/A) until a success re-arms the policy.
#[derive(Debug, Default)]
pub struct BannerPolicy {
    ever_shown: bool,
    dismissed: bool,
}

impl BannerPolicy {
    /// Unshown, undismissed policy.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reports a failed tick. Returns true exactly once per episode: the
    /// caller sets the banner-visible property when true.
    pub fn on_failure(&mut self) -> bool {
        if self.ever_shown || self.dismissed {
            return false;
        }
        self.ever_shown = true;
        true
    }

    /// The user dismissed the banner: stays silent until a success.
    pub fn dismiss(&mut self) {
        self.dismissed = true;
    }

    /// Reports a successful tick: hides the banner and re-arms the policy
    /// so a future failure episode shows it again.
    pub fn on_success(&mut self) {
        self.ever_shown = false;
        self.dismissed = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_skips_a_tick_while_one_is_in_flight() {
        let guard = PollGuard::new();
        assert!(guard.try_begin().is_some(), "first tick must dispatch");
        assert!(
            guard.try_begin().is_none(),
            "second tick while busy must skip"
        );
        guard.finish();
        assert!(
            guard.try_begin().is_some(),
            "tick after finish must dispatch"
        );
    }

    #[test]
    fn guard_drops_stale_sequences() {
        let guard = PollGuard::new();
        let first = guard.try_begin().expect("first tag");
        guard.finish();
        let second = guard.try_begin().expect("second tag");
        assert_ne!(first, second, "tags must advance per dispatch");
        assert!(
            !guard.is_newest(first),
            "the older answer must be dropped, never applied"
        );
        assert!(guard.is_newest(second), "the newest answer applies");
        guard.finish();
    }

    #[test]
    fn banner_shows_once_per_failure_episode() {
        let mut policy = BannerPolicy::new();
        assert!(policy.on_failure(), "first failure shows the banner");
        assert!(
            !policy.on_failure(),
            "later failures stay silent until recovery"
        );
        policy.on_success();
        assert!(
            policy.on_failure(),
            "a new episode after recovery shows it again"
        );
    }

    #[test]
    fn banner_dismiss_stays_silent_until_success() {
        let mut policy = BannerPolicy::new();
        assert!(policy.on_failure());
        policy.dismiss();
        assert!(
            !policy.on_failure(),
            "dismissed banner must not re-fire on the next failure"
        );
        policy.on_success();
        assert!(
            policy.on_failure(),
            "success re-arms the policy after a dismiss"
        );
    }
}
