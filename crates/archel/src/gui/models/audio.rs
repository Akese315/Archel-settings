//! Sound view-model: sink/source lists with default markers
//! (read-only; switches land in plan 07).
//!
//! Same proven shape as `models::landing`: `tick()` is a QML-called
//! invokable that only spawns - the sync `audio::audio_list()` (wpctl
//! subprocesses) runs on the gui runtime's blocking pool and the
//! finished snapshot returns exclusively through `CxxQtThread.queue`,
//! whose closure is the only code allowed to set properties
//! (RESEARCH Pattern 1). Rows reuse `AudioSnapshot`'s own flags
//! (name/description/default) rather than re-deriving anything: core
//! exposes no mute or port fields, so the tab shows exactly what the
//! CLI twin (`run_list`) knows (PLAT-03).

use cxx_qt_lib::QString;

use super::super::poll::{BannerPolicy, PollGuard};
use crate::gui::modal::{self, ModalSession};

/// Fully formatted tick payload, built off the Qt thread so the
/// `queue()` closure only sets properties.
pub(crate) struct AppliedAudio {
    pub sinks_json: String,
    pub sources_json: String,
    pub banner_visible: bool,
}

/// Backing state: property fields plus the poll guard and banner
/// policy. No shared core handle: every tick is an independent read.
pub struct AudioModelRust {
    pub(crate) sinks_json: QString,
    pub(crate) sources_json: QString,
    pub(crate) banner_visible: bool,
    pub(crate) operation_busy: bool,
    pub(crate) output_error_text: QString,
    pub(crate) input_error_text: QString,
    guard: PollGuard,
    banner: BannerPolicy,
}

impl Default for AudioModelRust {
    fn default() -> Self {
        Self {
            sinks_json: QString::from(""),
            sources_json: QString::from(""),
            banner_visible: false,
            operation_busy: false,
            output_error_text: QString::from(""),
            input_error_text: QString::from(""),
            guard: PollGuard::new(),
            banner: BannerPolicy::new(),
        }
    }
}

/// One device list as QML-ready JSON: `[{name,description,default}]`,
/// in `wpctl` status order. Reuses the snapshot's own flags; an empty
/// vector is the honest no-hardware state (`Ok` from core), never an
/// error.
pub(crate) fn devices_json_for(devices: &[archel_core::audio::AudioDevice]) -> String {
    let rows: Vec<serde_json::Value> = devices
        .iter()
        .map(|device| {
            serde_json::json!({
                "name": device.name,
                "description": device.description,
                "default": device.default,
            })
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string())
}

impl AudioModelRust {
    /// Applies a finished worker outcome on the Qt thread. `Err`
    /// (wpctl missing/failing) reads empty lists plus the
    /// once-per-episode banner; the empty-state copy covers the view.
    fn apply_outcome(
        &mut self,
        outcome: Result<archel_core::audio::AudioSnapshot, archel_core::ArchelError>,
    ) -> AppliedAudio {
        match outcome {
            Ok(snap) => {
                self.banner.on_success();
                AppliedAudio {
                    sinks_json: devices_json_for(&snap.sinks),
                    sources_json: devices_json_for(&snap.sources),
                    banner_visible: false,
                }
            }
            Err(_) => {
                self.banner.on_failure();
                AppliedAudio {
                    sinks_json: "[]".to_string(),
                    sources_json: "[]".to_string(),
                    banner_visible: self.banner.wants_visible(),
                }
            }
        }
    }
}

impl crate::gui::bridge_audio::qobject::AudioModel {
    /// 1 Hz entry point: skips while a tick is in flight (D-02),
    /// spawns the blocking list, marshals back through `queue()` only.
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
            let outcome = archel_core::audio::audio_list();
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
                        .set_sinks_json(QString::from(&applied.sinks_json));
                    obj.as_mut()
                        .set_sources_json(QString::from(&applied.sources_json));
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

    /// Switches the default sink (D-12): the picker value resolves
    /// against a fresh `audio_list` first, so an unknown name is
    /// rejected with core's own error string BEFORE any confirmation
    /// is shown; only a yes reaches `wpctl set-default`. Failures
    /// render inline under the picker (the tab's own error copy), and
    /// the PollGuard refresh confirms the new default on success.
    pub fn set_default_output(mut self: core::pin::Pin<&mut Self>, name: QString) {
        let text = name.to_string();
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
            let outcome = output_worker(&text, &PROD_AUDIO_WRITE_OPS, modal::session());
            let _ = qt_thread.queue(move |mut obj: core::pin::Pin<&mut Self>| {
                obj.as_mut().finish_output_operation(outcome);
            });
        });
    }

    /// Switches the default source: same resolve-confirm-switch-
    /// read-back shape over the sources.
    pub fn set_default_input(mut self: core::pin::Pin<&mut Self>, name: QString) {
        let text = name.to_string();
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
            let outcome = input_worker(&text, &PROD_AUDIO_WRITE_OPS, modal::session());
            let _ = qt_thread.queue(move |mut obj: core::pin::Pin<&mut Self>| {
                obj.as_mut().finish_input_operation(outcome);
            });
        });
    }

    /// Claims the single-operation slot (one default switch at a
    /// time) and mirrors the claim onto the QML-visible flag so both
    /// pickers disable themselves while a switch is in flight (D-15).
    fn begin_write_operation(mut self: core::pin::Pin<&mut Self>) -> bool {
        use cxx_qt::CxxQtType;
        if self.as_mut().rust_mut().operation_busy {
            return false;
        }
        self.as_mut().rust_mut().operation_busy = true;
        self.as_mut().set_operation_busy(true);
        true
    }

    /// Publishes a finished sink switch: the inline error (empty on
    /// success), the released slot, and a refresh tick so the default
    /// markers read back from the graph. Qt thread only (RESEARCH
    /// Pattern 1).
    fn finish_output_operation(mut self: core::pin::Pin<&mut Self>, outcome: Option<String>) {
        use cxx_qt::CxxQtType;
        self.as_mut().rust_mut().operation_busy = false;
        self.as_mut().set_operation_busy(false);
        self.as_mut()
            .set_output_error_text(QString::from(&outcome.unwrap_or_default()));
        self.as_mut().tick();
    }

    /// Publishes a finished source switch: same shape as the sink.
    fn finish_input_operation(mut self: core::pin::Pin<&mut Self>, outcome: Option<String>) {
        use cxx_qt::CxxQtType;
        self.as_mut().rust_mut().operation_busy = false;
        self.as_mut().set_operation_busy(false);
        self.as_mut()
            .set_input_error_text(QString::from(&outcome.unwrap_or_default()));
        self.as_mut().tick();
    }
}

/// Injection seam for the write path: the exact core entry points
/// the workers call, as plain function pointers. Production passes
/// the real core functions ([`PROD_AUDIO_WRITE_OPS`]); unit tests
/// pass fakes so the resolve-confirm-read-back choreography is
/// proven without spawning wpctl (the subprocess contract is the
/// core's tested territory - the GUI never holds a child).
pub(crate) struct AudioWriteOps {
    pub list: fn() -> Result<archel_core::audio::AudioSnapshot, archel_core::ArchelError>,
    pub set_default: fn(u32) -> Result<(), archel_core::ArchelError>,
}

/// The production wiring: every field is the CLI-proven core
/// function (thin-view contract; PLAT-03).
pub(crate) const PROD_AUDIO_WRITE_OPS: AudioWriteOps = AudioWriteOps {
    list: archel_core::audio::audio_list,
    set_default: archel_core::audio::set_default,
};

/// Sink-switch worker body (runs on the blocking pool). Returns
/// `None` on success or decline, `Some(core message)` on failure -
/// the caller publishes the text inline under the picker, never in
/// the error modal. Unknown names never reach a confirm; a switch
/// that does not stick reads back as the core-shaped internal error.
pub(crate) fn output_worker(
    name: &str,
    ops: &AudioWriteOps,
    sess: &ModalSession,
) -> Option<String> {
    switch_worker(
        name,
        ops,
        sess,
        archel_core::audio::resolve_sink_id,
        |snapshot, id| {
            snapshot
                .sinks
                .iter()
                .find(|sink| sink.id == id)
                .map(|sink| archel_core::audio::confirm_output_prompt(sink))
        },
        |fresh, target| {
            fresh
                .sinks
                .iter()
                .any(|sink| sink.name == *target && sink.default)
        },
    )
}

/// Source-switch worker body: same shape over the sources.
pub(crate) fn input_worker(
    name: &str,
    ops: &AudioWriteOps,
    sess: &ModalSession,
) -> Option<String> {
    switch_worker(
        name,
        ops,
        sess,
        archel_core::audio::resolve_source_id,
        |snapshot, id| {
            snapshot
                .sources
                .iter()
                .find(|source| source.id == id)
                .map(|source| archel_core::audio::confirm_input_prompt(source))
        },
        |fresh, target| {
            fresh
                .sources
                .iter()
                .any(|source| source.name == *target && source.default)
        },
    )
}

/// Shared resolve-confirm-switch-read-back body. `resolve` maps the
/// picker name to the session id (pure core fn); `prompt` builds the
/// confirm question for that id; `stuck` proves the read-back shows
/// the new default. Every core error surfaces through its readable
/// `message()`, unmodified.
fn switch_worker(
    name: &str,
    ops: &AudioWriteOps,
    sess: &ModalSession,
    resolve: fn(
        &archel_core::audio::AudioSnapshot,
        &str,
    ) -> Result<u32, archel_core::ArchelError>,
    prompt: impl FnOnce(&archel_core::audio::AudioSnapshot, u32) -> Option<String>,
    stuck: impl FnOnce(&archel_core::audio::AudioSnapshot, &str) -> bool,
) -> Option<String> {
    let snapshot = match (ops.list)() {
        Ok(snapshot) => snapshot,
        Err(err) => return Some(err.message()),
    };
    let id = match resolve(&snapshot, name) {
        Ok(id) => id,
        Err(err) => return Some(err.message()),
    };
    let question = match prompt(&snapshot, id) {
        Some(question) => question,
        None => {
            return Some(
                archel_core::ArchelError::Internal(format!(
                    "resolved audio id {id} has no device row"
                ))
                .message(),
            );
        }
    };
    let rx = sess.request_confirm(&question);
    if !rx.recv().unwrap_or(false) {
        return None;
    }
    if let Err(err) = (ops.set_default)(id) {
        return Some(err.message());
    }
    match (ops.list)() {
        Ok(fresh) if stuck(&fresh, name) => None,
        Ok(_) => Some(
            archel_core::ArchelError::Internal(format!(
                "wpctl set-default accepted '{name}' but read-back shows no new default"
            ))
            .message(),
        ),
        Err(err) => Some(err.message()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archel_core::audio::{AudioDevice, AudioKind, AudioSnapshot};

    fn ready_snapshot() -> AudioSnapshot {
        AudioSnapshot {
            sinks: vec![
                AudioDevice {
                    id: 47,
                    name: "alsa_output.usb-sc60".to_string(),
                    description: "SC60 for Lync Analog stereo".to_string(),
                    kind: AudioKind::Sink,
                    default: true,
                },
                AudioDevice {
                    id: 59,
                    name: "alsa_output.pci-hdmi".to_string(),
                    description: "GP104 HDMI stereo".to_string(),
                    kind: AudioKind::Sink,
                    default: false,
                },
            ],
            sources: vec![AudioDevice {
                id: 60,
                name: "alsa_input.usb-sc60".to_string(),
                description: "SC60 for Lync Analog stereo".to_string(),
                kind: AudioKind::Source,
                default: true,
            }],
        }
    }

    fn failed_outcome() -> Result<AudioSnapshot, archel_core::ArchelError> {
        Err(archel_core::ArchelError::audio_unavailable(
            "wpctl missing in test",
        ))
    }

    #[test]
    fn error_path_reads_empty_lists_and_signals_banner_once() {
        let mut model = AudioModelRust::default();
        let first = model.apply_outcome(failed_outcome());
        assert_eq!(first.sinks_json, "[]");
        assert_eq!(first.sources_json, "[]");
        assert!(first.banner_visible, "first failure shows the banner");
        let second = model.apply_outcome(failed_outcome());
        assert!(
            second.banner_visible,
            "consecutive failures keep the banner without re-signalling"
        );
    }

    #[test]
    fn success_fills_rows_with_default_markers() {
        let mut model = AudioModelRust::default();
        let _ = model.apply_outcome(failed_outcome());
        let applied = model.apply_outcome(Ok(ready_snapshot()));
        assert!(!applied.banner_visible, "success hides the banner");
        let sinks: Vec<serde_json::Value> =
            serde_json::from_str(&applied.sinks_json).expect("sinks must be JSON");
        assert_eq!(sinks.len(), 2, "both sinks render");
        assert_eq!(sinks[0]["default"], true, "default marker survives");
        assert_eq!(sinks[1]["default"], false);
        assert_eq!(sinks[0]["name"], "alsa_output.usb-sc60");
        assert!(sinks[0].get("description").is_some());
        let sources: Vec<serde_json::Value> =
            serde_json::from_str(&applied.sources_json).expect("sources must be JSON");
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0]["default"], true);
    }

    // -- Write-path choreography (plan 06-07, Task 1) --
    //
    // The fakes below replace only the two `AudioWriteOps` seams; the
    // workers themselves run for real against a local ModalSession, so
    // resolve-before-confirm, default-No, the verbatim question and
    // the read-back proof are all proven without spawning wpctl.

    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::time::{Duration, Instant};

    use crate::gui::modal::{ModalKind, ModalSession};

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

    #[test]
    fn unknown_sink_name_is_rejected_before_any_confirm() {
        use archel_core::audio::AudioSnapshot;
        static SET_CALLS: AtomicUsize = AtomicUsize::new(0);
        fn fake_list() -> Result<AudioSnapshot, archel_core::ArchelError> {
            Ok(ready_snapshot())
        }
        fn set_must_not_run(_id: u32) -> Result<(), archel_core::ArchelError> {
            SET_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(())
        }
        let ops = AudioWriteOps {
            list: fake_list,
            set_default: set_must_not_run,
        };
        let sess = ModalSession::default();
        let outcome = output_worker("no-such-sink", &ops, &sess);
        let expected = archel_core::ArchelError::audio_device_not_found("no-such-sink").message();
        assert_eq!(
            outcome.as_deref(),
            Some(expected.as_str()),
            "the core error string surfaces unmodified"
        );
        assert_eq!(
            sess.snapshot_kind(),
            ModalKind::None,
            "no confirm is ever shown for an unknown name"
        );
        assert_eq!(SET_CALLS.load(AtomicOrdering::SeqCst), 0);
    }

    #[test]
    fn decline_short_circuits_before_any_wpctl_call() {
        use archel_core::audio::AudioSnapshot;
        static SET_CALLS: AtomicUsize = AtomicUsize::new(0);
        fn fake_list() -> Result<AudioSnapshot, archel_core::ArchelError> {
            Ok(ready_snapshot())
        }
        fn set_must_not_run(_id: u32) -> Result<(), archel_core::ArchelError> {
            SET_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(())
        }
        let ops = AudioWriteOps {
            list: fake_list,
            set_default: set_must_not_run,
        };
        let sess = ModalSession::default();
        let outcome = std::thread::scope(|s| {
            let handle = s.spawn(|| output_worker("alsa_output.pci-hdmi", &ops, &sess));
            wait_until("sink confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            sess.resolve_confirm(false);
            handle.join().expect("worker joins")
        });
        assert_eq!(outcome, None, "a decline is silent, not an error");
        assert_eq!(SET_CALLS.load(AtomicOrdering::SeqCst), 0);
    }

    #[test]
    fn accept_switches_and_proves_the_read_back() {
        use archel_core::audio::AudioSnapshot;
        static SET_ID: Mutex<Option<u32>> = Mutex::new(None);
        fn fake_list() -> Result<AudioSnapshot, archel_core::ArchelError> {
            let picked = *SET_ID.lock().expect("set lock");
            let mut snap = ready_snapshot();
            for sink in &mut snap.sinks {
                sink.default = Some(sink.id) == picked;
            }
            Ok(snap)
        }
        fn fake_set(id: u32) -> Result<(), archel_core::ArchelError> {
            *SET_ID.lock().expect("set lock") = Some(id);
            Ok(())
        }
        *SET_ID.lock().expect("set lock") = None;
        let ops = AudioWriteOps {
            list: fake_list,
            set_default: fake_set,
        };
        let sess = ModalSession::default();
        let outcome = std::thread::scope(|s| {
            let handle = s.spawn(|| output_worker("alsa_output.pci-hdmi", &ops, &sess));
            wait_until("sink confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            let expected = archel_core::audio::confirm_output_prompt(&ready_snapshot().sinks[1]);
            assert_eq!(sess.snapshot_question(), expected);
            sess.resolve_confirm(true);
            handle.join().expect("worker joins")
        });
        assert_eq!(outcome, None, "a stuck switch reports no error");
        assert_eq!(
            *SET_ID.lock().expect("set lock"),
            Some(59),
            "the resolved session id reaches wpctl"
        );
    }

    #[test]
    fn wpctl_failure_surfaces_the_core_string_unmodified() {
        use archel_core::audio::AudioSnapshot;
        fn fake_list() -> Result<AudioSnapshot, archel_core::ArchelError> {
            Ok(ready_snapshot())
        }
        fn failing_set(_id: u32) -> Result<(), archel_core::ArchelError> {
            Err(archel_core::ArchelError::Internal(
                "wpctl set-default failed in test".to_string(),
            ))
        }
        let ops = AudioWriteOps {
            list: fake_list,
            set_default: failing_set,
        };
        let sess = ModalSession::default();
        let outcome = std::thread::scope(|s| {
            let handle = s.spawn(|| input_worker("alsa_input.usb-sc60", &ops, &sess));
            wait_until("source confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            sess.resolve_confirm(true);
            handle.join().expect("worker joins")
        });
        let expected = archel_core::ArchelError::Internal(
            "wpctl set-default failed in test".to_string(),
        )
        .message();
        assert_eq!(outcome.as_deref(), Some(expected.as_str()));
    }
}
