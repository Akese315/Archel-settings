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
pub struct DisplayModelRust {
    pub(crate) monitors_json: QString,
    pub(crate) banner_visible: bool,
    guard: PollGuard,
    banner: BannerPolicy,
}

impl Default for DisplayModelRust {
    fn default() -> Self {
        Self {
            monitors_json: QString::from(""),
            banner_visible: false,
            guard: PollGuard::new(),
            banner: BannerPolicy::new(),
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
}
