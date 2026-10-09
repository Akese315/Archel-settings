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
    guard: PollGuard,
    banner: BannerPolicy,
}

impl Default for AudioModelRust {
    fn default() -> Self {
        Self {
            sinks_json: QString::from(""),
            sources_json: QString::from(""),
            banner_visible: false,
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
}
