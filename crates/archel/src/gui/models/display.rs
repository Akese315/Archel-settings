//! Display view-model: live Hyprland monitor inventory (read-only;
//! the resolution/refresh ComboBoxes stay disabled until plan 08).
//!
//! Same proven shape as `models::audio`: `tick()` is a QML-called
//! invokable that only spawns - the sync `read_monitors()` (the same
//! read `display_cmds::run_monitors` calls) runs on the gui runtime's
//! blocking pool and the finished payload returns exclusively through
//! `CxxQtThread.queue`, whose closure is the only code allowed to set
//! properties (RESEARCH Pattern 1). No raw hyprctl re-parse lives
//! here (PLAT-03).

use cxx_qt_lib::QString;

use super::super::poll::{BannerPolicy, PollGuard};

/// Fully formatted tick payload, built off the Qt thread so the
/// `queue()` closure only sets properties.
pub(crate) struct AppliedDisplay {
    pub monitors_json: String,
    pub banner_visible: bool,
}

/// Backing state: property fields plus the poll guard and banner
/// policy. No shared core handle: every tick is an independent read.
/// `operation_busy`/`operation_kind` mirror the 06-06 pkg write-slot
/// shape (D-15); the countdown trio mirrors the shared
/// [`CountdownState`] onto QML through `countdown_tick` (the worker
/// only touches the shared state, never the QObject).
pub struct DisplayModelRust {
    pub(crate) monitors_json: QString,
    pub(crate) banner_visible: bool,
    pub(crate) operation_busy: bool,
    pub(crate) operation_kind: QString,
    pub(crate) countdown_visible: bool,
    pub(crate) countdown_seconds: i32,
    pub(crate) countdown_monitor: QString,
    guard: PollGuard,
    banner: BannerPolicy,
    countdown: std::sync::Arc<CountdownState>,
}

impl Default for DisplayModelRust {
    fn default() -> Self {
        Self {
            monitors_json: QString::from(""),
            banner_visible: false,
            operation_busy: false,
            operation_kind: QString::from(""),
            countdown_visible: false,
            countdown_seconds: 0,
            countdown_monitor: QString::from(""),
            guard: PollGuard::new(),
            banner: BannerPolicy::new(),
            countdown: std::sync::Arc::new(CountdownState::default()),
        }
    }
}

/// `[{name,summary,mode,scale,
/// position,disabled,available_modes}]`. The mode comes from the
/// core's `current_mode` derivation and the summary from
/// `summarize_monitor` - never re-derived here. The option list is
/// the core-exposed `available_modes` vocabulary the plan 08
/// ComboBoxes will offer.
pub(crate) fn monitors_json_for(monitors: &[archel_core::hyprland::monitors::Monitor]) -> String {
    use archel_core::hyprland::monitors as core_monitors;
    let rows: Vec<serde_json::Value> = monitors
        .iter()
        .map(|monitor| {
            serde_json::json!({
                "name": monitor.name,
                "summary": core_monitors::summarize_monitor(monitor),
                "mode": core_monitors::current_mode(monitor),
                "scale": monitor.scale,
                "position": format!("{}x{}", monitor.x, monitor.y),
                "disabled": monitor.disabled,
                "available_modes": monitor.available_modes,
            })
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string())
}

impl DisplayModelRust {
    /// Applies a finished worker outcome on the Qt thread. `Err` (no
    /// compositor) reads the honest empty list plus the
    /// once-per-episode banner; the empty state covers the view.
    fn apply_outcome(
        &mut self,
        outcome: Result<
            Vec<archel_core::hyprland::monitors::Monitor>,
            archel_core::ArchelError,
        >,
    ) -> AppliedDisplay {
        match outcome {
            Ok(monitors) => {
                self.banner.on_success();
                AppliedDisplay {
                    monitors_json: monitors_json_for(&monitors),
                    banner_visible: false,
                }
            }
            Err(_) => {
                self.banner.on_failure();
                AppliedDisplay {
                    monitors_json: "[]".to_string(),
                    banner_visible: self.banner.wants_visible(),
                }
            }
        }
    }
}

impl crate::gui::bridge_display::qobject::DisplayModel {
    /// 1 Hz entry point: skips while a tick is in flight (D-02),
    /// spawns the blocking monitors read, marshals back through
    /// `queue()` only.
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
            let outcome = archel_core::hyprland::monitors::read_monitors();
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
                        .set_monitors_json(QString::from(&applied.monitors_json));
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

    /// Plans and applies one monitor change (D-16): the worker
    /// validates plus plans read-only first, confirms through the
    /// global ConfirmModal, then applies through
    /// `apply_monitor_plan` with the countdown as the timed confirm.
    /// Empty resolution/refresh keeps the current mode; empty scale
    /// keeps the current scale; all three empty is a no-op.
    pub fn plan_change(
        mut self: core::pin::Pin<&mut Self>,
        monitor: QString,
        resolution: QString,
        refresh: QString,
        scale: QString,
    ) {
        use cxx_qt::CxxQtType;
        let monitor_text = monitor.to_string();
        let resolution_text = resolution.to_string();
        let refresh_text = refresh.to_string();
        let scale_text = scale.to_string();
        if monitor_text.trim().is_empty()
            || (resolution_text.trim().is_empty()
                && refresh_text.trim().is_empty()
                && scale_text.trim().is_empty())
        {
            return;
        }
        if !self.as_mut().begin_write_operation("display") {
            return;
        }
        let countdown = self.as_mut().rust_mut().countdown.clone();
        let qt_thread = {
            use cxx_qt::Threading;
            self.qt_thread()
        };
        let runtime = super::super::RUNTIME
            .get()
            .expect("gui runtime is set before Qt starts");
        runtime.spawn_blocking(move || {
            let base = archel_core::hyprland::hypr_dir();
            display_set_worker(
                &base,
                &monitor_text,
                &resolution_text,
                &refresh_text,
                &scale_text,
                &PROD_DISPLAY_OPS,
                &countdown,
                super::super::modal::session(),
                COUNTDOWN_SECS,
            );
            countdown.hide();
            let _ = qt_thread.queue(|mut obj: core::pin::Pin<&mut Self>| {
                obj.as_mut().finish_write_operation();
            });
        });
    }

    /// Conserver (D-16, default): keeps the applied mode, stopping
    /// the auto-revert. No-op without a pending countdown.
    pub fn keep_change(mut self: core::pin::Pin<&mut Self>) {
        use cxx_qt::CxxQtType;
        self.as_mut().rust_mut().countdown.resolve(true);
    }

    /// Annuler (D-16): reverts immediately instead of waiting for
    /// the timeout. No-op without a pending countdown.
    pub fn revert_change(mut self: core::pin::Pin<&mut Self>) {
        use cxx_qt::CxxQtType;
        self.as_mut().rust_mut().countdown.resolve(false);
    }

    /// Mirrors the shared countdown state onto the QML-visible
    /// properties and advances the displayed second. Polled by the
    /// tab's 1 s Timer (same discipline as `sync_session`).
    pub fn countdown_tick(mut self: core::pin::Pin<&mut Self>) {
        use cxx_qt::CxxQtType;
        let (visible, seconds, monitor) = self.as_mut().rust_mut().countdown.display();
        self.as_mut().set_countdown_visible(visible);
        self.as_mut().set_countdown_seconds(seconds);
        self.as_mut()
            .set_countdown_monitor(QString::from(&monitor));
    }

    /// Claims the single-operation slot (D-15).
    fn begin_write_operation(mut self: core::pin::Pin<&mut Self>, kind: &str) -> bool {
        use cxx_qt::CxxQtType;
        if self.as_mut().rust_mut().operation_busy {
            return false;
        }
        self.as_mut().rust_mut().operation_busy = true;
        self.as_mut().set_operation_kind(QString::from(kind));
        self.as_mut().set_operation_busy(true);
        true
    }

    /// Releases the operation slot, hides any countdown residue and
    /// refreshes the inventory (Qt thread only, RESEARCH Pattern 1).
    fn finish_write_operation(mut self: core::pin::Pin<&mut Self>) {
        use cxx_qt::CxxQtType;
        self.as_mut().rust_mut().operation_busy = false;
        self.as_mut().set_operation_busy(false);
        self.as_mut().set_operation_kind(QString::from(""));
        self.as_mut().set_countdown_visible(false);
        self.as_mut().tick();
    }
}

// -- Write path (plan 06-08, Task 3) -----------------------------------
//
// One worker drives the whole D-16 flow through the CLI-proven
// `plan_monitor_change` -> `apply_monitor_plan` chain (RESEARCH
// Pitfall 5: no other Hyprland IPC verb is reachable from here).
// The GUI never owns the revert timer: `apply_monitor_plan` rolls
// back on timeout/decline, the worker only displays the countdown
// through the shared state below.

use std::path::Path;
use std::sync::{mpsc, Mutex};
use std::time::Duration;

use crate::gui::modal::ModalSession;

/// D-16 countdown length, in seconds.
pub(crate) const COUNTDOWN_SECS: u64 = 15;

/// Thread-safe countdown handoff between the blocking worker (which
/// opens/resolves it) and the Qt thread (which mirrors it through
/// `countdown_tick`). Plain Rust so both sides stay Send-safe; the
/// QObject properties are only ever written on the Qt thread.
pub(crate) struct CountdownState {
    inner: Mutex<CountdownInner>,
}

struct CountdownInner {
    visible: bool,
    monitor: String,
    seconds_left: i32,
    total: i32,
    tx: Option<mpsc::Sender<bool>>,
}

impl Default for CountdownState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(CountdownInner {
                visible: false,
                monitor: String::new(),
                seconds_left: 0,
                total: COUNTDOWN_SECS as i32,
                tx: None,
            }),
        }
    }
}

impl CountdownState {
    /// Opens the countdown for one output and returns the receiver
    /// the timed confirm blocks on. Drops any previous pending
    /// decision (its waiter disconnects, which reads as a decline).
    fn open(&self, monitor: &str, total: u64) -> mpsc::Receiver<bool> {
        let (tx, rx) = mpsc::channel();
        let mut inner = self.inner.lock().expect("countdown lock");
        inner.visible = true;
        inner.monitor = monitor.to_string();
        inner.seconds_left = total as i32;
        inner.total = total as i32;
        inner.tx = Some(tx);
        rx
    }

    /// Qt-thread mirror: advances the displayed second (floored at
    /// zero; the worker's `recv_timeout` owns the real expiry) and
    /// returns (visible, seconds, monitor).
    fn display(&self) -> (bool, i32, String) {
        let mut inner = self.inner.lock().expect("countdown lock");
        if inner.visible && inner.seconds_left > 0 {
            inner.seconds_left -= 1;
        }
        (
            inner.visible,
            inner.seconds_left,
            inner.monitor.clone(),
        )
    }

    /// Resolves a pending countdown (Conserver/Annuler). Returns
    /// false when nothing was pending. Always hides.
    fn resolve(&self, kept: bool) -> bool {
        let mut inner = self.inner.lock().expect("countdown lock");
        let pending = inner.tx.take().is_some_and(|tx| tx.send(kept).is_ok());
        inner.visible = false;
        pending
    }

    /// Hides without resolving (worker teardown safety net).
    fn hide(&self) {
        let mut inner = self.inner.lock().expect("countdown lock");
        inner.tx.take();
        inner.visible = false;
    }

    #[cfg(test)]
    fn snapshot(&self) -> (bool, i32, String) {
        let inner = self.inner.lock().expect("countdown lock");
        (inner.visible, inner.seconds_left, inner.monitor.clone())
    }
}

/// Injection seam for the display write: the live legs behind
/// `apply_monitor_plan` plus the monitor inventory read. Production
/// passes [`PROD_DISPLAY_OPS`]; tests pass fakes so no compositor
/// IPC happens headless.
pub(crate) struct DisplayOps {
    pub read_monitors:
        fn() -> Result<Vec<archel_core::hyprland::monitors::Monitor>, archel_core::ArchelError>,
    pub applier: fn(&str) -> Result<(), archel_core::ArchelError>,
    pub rollbacker: fn(&str) -> Result<(), archel_core::ArchelError>,
}

fn prod_read_monitors(
) -> Result<Vec<archel_core::hyprland::monitors::Monitor>, archel_core::ArchelError> {
    archel_core::hyprland::monitors::read_monitors()
}

fn prod_apply_leg(lua: &str) -> Result<(), archel_core::ArchelError> {
    archel_core::hyprland::run_hyprctl_eval("hyprctl", lua)
}

/// The production wiring (thin-view contract; PLAT-03). Both legs
/// are the plain eval path - rollback re-applies the snapshot
/// verbatim through the same IPC the CLI twin uses.
pub(crate) const PROD_DISPLAY_OPS: DisplayOps = DisplayOps {
    read_monitors: prod_read_monitors,
    applier: prod_apply_leg,
    rollbacker: prod_apply_leg,
};

/// Composes the `--mode` value from the tab fields: blank
/// resolution keeps the current mode; blank refresh keeps the
/// resolution as-is (specials and bare modes pass through to
/// `validate_mode` untouched).
pub(crate) fn compose_mode(resolution: &str, refresh: &str) -> Option<String> {
    let resolution = resolution.trim();
    let refresh = refresh.trim();
    if resolution.is_empty() {
        return None;
    }
    if refresh.is_empty() {
        return Some(resolution.to_string());
    }
    Some(format!("{resolution}@{refresh}"))
}

/// Blocks the worker on the modal's one-shot answer (default No).
fn await_confirm(sess: &ModalSession, question: &str) -> bool {
    let rx = sess.request_confirm(question);
    rx.recv().unwrap_or(false)
}

/// Display set worker: read-only planning (lookup, validation,
/// layout projection) runs BEFORE any prompt so typos and overlaps
/// fail fast with no modal; then the modal pre-confirm; then
/// `apply_monitor_plan` with the countdown as the timed confirm.
/// A decline or a timeout rolls back inside core (D-16) and the tab
/// simply refreshes; other failures surface in the ErrorModal.
#[allow(clippy::too_many_arguments)]
pub(crate) fn display_set_worker(
    base: &Path,
    output: &str,
    resolution: &str,
    refresh: &str,
    scale: &str,
    ops: &DisplayOps,
    countdown: &CountdownState,
    sess: &ModalSession,
    countdown_secs: u64,
) {
    use archel_core::hyprland::monitors as core_monitors;
    let monitors = match (ops.read_monitors)() {
        Ok(monitors) => monitors,
        Err(err) => {
            sess.open_error(&err.message());
            return;
        }
    };
    let current = match monitors.iter().find(|m| m.name == output.trim()) {
        Some(current) => current,
        None => {
            sess.open_error(
                &archel_core::ArchelError::desktop_invalid(format!(
                    "rejected monitor output {output:?} (no such connected monitor)"
                ))
                .message(),
            );
            return;
        }
    };
    let mode = compose_mode(resolution, refresh);
    let scale_opt = if scale.trim().is_empty() {
        None
    } else {
        Some(scale.trim())
    };
    let want = core_monitors::MonitorWant {
        output: current.name.clone(),
        mode,
        scale: scale_opt.map(str::to_string),
        position: None,
        disabled: false,
    };
    let plan = match core_monitors::plan_monitor_change(current, &want) {
        Ok(plan) => plan,
        Err(err) => {
            sess.open_error(&err.message());
            return;
        }
    };
    let projected = core_monitors::project_layout(
        &monitors,
        &plan.output,
        &plan.mode,
        plan.scale,
        &plan.position,
        plan.disabled,
    );
    if let Err(err) = core_monitors::validate_layout(&projected) {
        sess.open_error(&err.message());
        return;
    }
    let pre_prompt = core_monitors::confirm_monitor_prompt(&plan);
    let outcome = core_monitors::apply_monitor_plan(
        base,
        &plan,
        |prompt| {
            debug_assert_eq!(prompt, pre_prompt.as_str());
            if await_confirm(sess, prompt) {
                Ok(())
            } else {
                Err(archel_core::ArchelError::ConfirmationAborted)
            }
        },
        |leg| (ops.applier)(leg),
        |leg| (ops.rollbacker)(leg),
        |_| {
            let rx = countdown.open(&plan.output, countdown_secs);
            match rx.recv_timeout(Duration::from_secs(countdown_secs)) {
                Ok(true) => {
                    countdown.hide();
                    Ok(true)
                }
                Ok(false) => {
                    countdown.hide();
                    Err(archel_core::ArchelError::ConfirmationAborted)
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    countdown.hide();
                    Err(archel_core::ArchelError::ConfirmTimeout)
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    countdown.hide();
                    Err(archel_core::ArchelError::ConfirmationAborted)
                }
            }
        },
    );
    match outcome {
        Ok(_) => {}
        Err(archel_core::ArchelError::ConfirmationAborted)
        | Err(archel_core::ArchelError::ConfirmTimeout) => {}
        Err(err) => sess.open_error(&err.message()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_monitor() -> archel_core::hyprland::monitors::Monitor {
        archel_core::hyprland::monitors::Monitor {
            name: "DP-2".to_string(),
            description: "AOC 24G2W1G4".to_string(),
            width: 1920,
            height: 1080,
            refresh_rate: 144.001,
            x: 0,
            y: 0,
            scale: 1.0,
            disabled: false,
            available_modes: vec![
                "1920x1080@144.00Hz".to_string(),
                "1920x1080@60.00Hz".to_string(),
            ],
        }
    }

    fn failed_outcome() -> Result<
        Vec<archel_core::hyprland::monitors::Monitor>,
        archel_core::ArchelError,
    > {
        Err(archel_core::ArchelError::hyprland_unavailable(
            "no compositor in test",
        ))
    }

    #[test]
    fn error_path_reads_empty_list_and_signals_banner_once() {
        let mut model = DisplayModelRust::default();
        let first = model.apply_outcome(failed_outcome());
        assert_eq!(first.monitors_json, "[]");
        assert!(first.banner_visible, "first failure shows the banner");
        let second = model.apply_outcome(failed_outcome());
        assert!(
            second.banner_visible,
            "consecutive failures keep the banner without re-signalling"
        );
    }

    #[test]
    fn success_fills_rows_with_core_derived_mode_and_options() {
        let mut model = DisplayModelRust::default();
        let _ = model.apply_outcome(failed_outcome());
        let applied = model.apply_outcome(Ok(vec![ready_monitor()]));
        assert!(!applied.banner_visible, "success hides the banner");
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(&applied.monitors_json).expect("monitors must be JSON");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], "DP-2");
        assert_eq!(rows[0]["mode"], "1920x1080@144");
        assert_eq!(rows[0]["scale"], 1.0);
        assert_eq!(rows[0]["disabled"], false);
        let modes = rows[0]["available_modes"]
            .as_array()
            .expect("option list must survive");
        assert_eq!(modes.len(), 2, "ComboBox options come from the core read");
        assert!(rows[0].get("summary").is_some());
    }

    // -- Write path choreography (plan 06-08, Task 3) --
    //
    // The fakes below replace only the three `DisplayOps` seams; the
    // worker runs for real against a local ModalSession plus a local
    // CountdownState, so pre-confirm-before-apply, keep-prevents-
    // revert and expiry-triggers-revert are all proven headless with
    // a fake clock (short countdown windows, real channels).

    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use crate::gui::modal::{ModalKind, ModalSession};

    /// Polls `cond` until it holds or 5 s elapse (worker rendezvous).
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

    fn test_base(tag: &str) -> std::path::PathBuf {
        // Nanos stamp keeps parallel test runs apart without any
        // process-handle API (gui/ stays subprocess-free, 06-09 audit).
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let base = std::env::temp_dir().join(format!("archel-display-gui-{tag}-{unique}"));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).expect("test base");
        base
    }

    fn fake_monitors() -> Vec<archel_core::hyprland::monitors::Monitor> {
        vec![ready_monitor()]
    }

    fn fake_monitors_ok(
    ) -> Result<Vec<archel_core::hyprland::monitors::Monitor>, archel_core::ArchelError> {
        Ok(fake_monitors())
    }

    /// Per-test fake legs over function-local counters. Each test
    /// declares its own statics (no cross-test sharing: the harness
    /// runs tests on parallel threads).
    macro_rules! fake_ops {
        ($apply:ident, $rollback:ident, $legs:ident) => {{
            fn fake_apply(leg: &str) -> Result<(), archel_core::ArchelError> {
                $apply.fetch_add(1, AtomicOrdering::SeqCst);
                $legs.lock().expect("legs lock").push(leg.to_string());
                Ok(())
            }
            fn fake_rollback(_: &str) -> Result<(), archel_core::ArchelError> {
                $rollback.fetch_add(1, AtomicOrdering::SeqCst);
                Ok(())
            }
            DisplayOps {
                read_monitors: fake_monitors_ok,
                applier: fake_apply,
                rollbacker: fake_rollback,
            }
        }};
    }

    #[test]
    fn mode_compose_keeps_current_when_fields_are_blank() {
        assert_eq!(compose_mode("", ""), None);
        assert_eq!(
            compose_mode("1920x1080", ""),
            Some("1920x1080".to_string())
        );
        assert_eq!(
            compose_mode("1920x1080", "144"),
            Some("1920x1080@144".to_string())
        );
    }

    #[test]
    fn decline_at_pre_confirm_applies_nothing_and_shows_no_countdown() {
        static APPLY_CALLS: AtomicUsize = AtomicUsize::new(0);
        static ROLLBACK_CALLS: AtomicUsize = AtomicUsize::new(0);
        static LEGS: Mutex<Vec<String>> = Mutex::new(Vec::new());
        let ops = fake_ops!(APPLY_CALLS, ROLLBACK_CALLS, LEGS);
        let sess = ModalSession::default();
        let countdown = CountdownState::default();
        let base = test_base("decline");
        std::thread::scope(|s| {
            s.spawn(|| {
                display_set_worker(
                    &base, "DP-2", "1920x1080", "60", "1", &ops, &countdown, &sess, 15,
                )
            });
            wait_until("pre-confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            sess.resolve_confirm(false);
            s.spawn(|| {
                std::thread::sleep(Duration::from_millis(300));
            });
        });
        assert_eq!(APPLY_CALLS.load(AtomicOrdering::SeqCst), 0);
        assert_eq!(ROLLBACK_CALLS.load(AtomicOrdering::SeqCst), 0);
        assert!(
            !countdown.snapshot().0,
            "no apply means no countdown ever opens"
        );
        assert_eq!(sess.snapshot_kind(), ModalKind::None);
    }

    #[test]
    fn keep_stops_the_revert_and_persists_the_snippet() {
        static APPLY_CALLS: AtomicUsize = AtomicUsize::new(0);
        static ROLLBACK_CALLS: AtomicUsize = AtomicUsize::new(0);
        static LEGS: Mutex<Vec<String>> = Mutex::new(Vec::new());
        LEGS.lock().expect("legs lock").clear();
        let ops = fake_ops!(APPLY_CALLS, ROLLBACK_CALLS, LEGS);
        let sess = ModalSession::default();
        let countdown = CountdownState::default();
        let base = test_base("keep");
        std::thread::scope(|s| {
            s.spawn(|| {
                display_set_worker(
                    &base, "DP-2", "1920x1080", "60", "1", &ops, &countdown, &sess, 15,
                )
            });
            wait_until("pre-confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            sess.resolve_confirm(true);
            wait_until("countdown opens", || countdown.snapshot().0);
            assert_eq!(countdown.snapshot().2, "DP-2");
            assert!(countdown.resolve(true), "Conserver resolves the pending keep");
        });
        assert_eq!(APPLY_CALLS.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(
            ROLLBACK_CALLS.load(AtomicOrdering::SeqCst),
            0,
            "keep prevents the revert"
        );
        let snippet = std::fs::read_to_string(
            archel_core::hyprland::snippet_path_at(&base),
        )
        .expect("kept change persists the snippet");
        assert!(
            snippet.contains("DP-2"),
            "the persisted line names the kept output: {snippet}"
        );
        assert!(
            !countdown.snapshot().0,
            "the countdown closes once the keep lands"
        );
    }

    #[test]
    fn expiry_triggers_the_rollback_with_no_snippet_write() {
        static APPLY_CALLS: AtomicUsize = AtomicUsize::new(0);
        static ROLLBACK_CALLS: AtomicUsize = AtomicUsize::new(0);
        static LEGS: Mutex<Vec<String>> = Mutex::new(Vec::new());
        let ops = fake_ops!(APPLY_CALLS, ROLLBACK_CALLS, LEGS);
        let sess = ModalSession::default();
        let countdown = CountdownState::default();
        let base = test_base("expiry");
        std::thread::scope(|s| {
            s.spawn(|| {
                // Zero-second window: the timed confirm expires
                // immediately (fake clock), exactly like a lapsed 15 s
                // window on the live box.
                display_set_worker(
                    &base, "DP-2", "1920x1080", "60", "1", &ops, &countdown, &sess, 0,
                )
            });
            wait_until("pre-confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            sess.resolve_confirm(true);
            wait_until("rollback runs", || {
                ROLLBACK_CALLS.load(AtomicOrdering::SeqCst) == 1
            });
        });
        assert_eq!(APPLY_CALLS.load(AtomicOrdering::SeqCst), 1);
        assert!(
            std::fs::read_to_string(archel_core::hyprland::snippet_path_at(&base)).is_err(),
            "an expired change persists nothing"
        );
        assert_eq!(sess.snapshot_kind(), ModalKind::None);
    }

    #[test]
    fn unknown_output_and_bad_mode_fail_before_any_confirm() {
        static APPLY_CALLS: AtomicUsize = AtomicUsize::new(0);
        static ROLLBACK_CALLS: AtomicUsize = AtomicUsize::new(0);
        static LEGS: Mutex<Vec<String>> = Mutex::new(Vec::new());
        let ops = fake_ops!(APPLY_CALLS, ROLLBACK_CALLS, LEGS);
        let sess = ModalSession::default();
        let countdown = CountdownState::default();
        let base = test_base("invalid");
        display_set_worker(
            &base, "NOPE-1", "1920x1080", "", "1", &ops, &countdown, &sess, 15,
        );
        assert_eq!(sess.snapshot_kind(), ModalKind::Error);
        let sess = ModalSession::default();
        display_set_worker(
            &base, "DP-2", "999x999", "", "1", &ops, &countdown, &sess, 15,
        );
        assert_eq!(
            sess.snapshot_kind(),
            ModalKind::Error,
            "an off-vocabulary mode never reaches a confirm"
        );
        assert_eq!(APPLY_CALLS.load(AtomicOrdering::SeqCst), 0);
        assert_eq!(ROLLBACK_CALLS.load(AtomicOrdering::SeqCst), 0);
    }

    #[test]
    fn countdown_ticks_down_and_resolves_once() {
        let countdown = CountdownState::default();
        let rx = countdown.open("DP-2", 15);
        assert!(countdown.snapshot().0);
        assert_eq!(countdown.snapshot().1, 15);
        let (_, seconds, _) = countdown.display();
        assert_eq!(seconds, 14, "each mirror tick advances one second");
        assert!(countdown.resolve(true));
        assert!(rx.recv().expect("keep resolves true"));
        assert!(!countdown.snapshot().0);
        assert!(
            !countdown.resolve(false),
            "a second resolve is a no-op, never a double send"
        );
    }
}
