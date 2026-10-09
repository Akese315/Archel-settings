//! Input view-model: live mouse leaves plus the merged keybind
//! inventory (read-only; the Ajouter editor lands in plan 08).
//!
//! Same proven shape as `models::net`: `tick()` is a QML-called
//! invokable that only spawns - the sync core reads (`read_input`,
//! `binds -j` plus the snippet binds) run on the gui runtime's
//! blocking pool and the finished payload returns exclusively through
//! `CxxQtThread.queue`, whose closure is the only code allowed to set
//! properties (RESEARCH Pattern 1). Keybind rows render through the
//! core's `keybind_rows` (same rows `run_keybinds_list` prints), and
//! managed binds come from `parse_snippet_binds` over the app snippet
//! - never a GUI-side reformat (PLAT-03).

use cxx_qt_lib::QString;

use super::super::poll::{BannerPolicy, PollGuard};
use crate::gui::models::system::NOT_AVAILABLE;

/// Fully formatted tick payload, built off the Qt thread so the
/// `queue()` closure only sets properties.
pub(crate) struct AppliedInput {
    pub sensitivity_text: String,
    pub natural_text: String,
    pub scroll_text: String,
    pub binds_json: String,
    pub banner_visible: bool,
}

/// Backing state: property fields plus the poll guard and banner
/// policy. No shared core handle: every tick is an independent read.
pub struct InputModelRust {
    pub(crate) sensitivity_text: QString,
    pub(crate) natural_text: QString,
    pub(crate) scroll_text: QString,
    pub(crate) binds_json: QString,
    pub(crate) banner_visible: bool,
    guard: PollGuard,
    banner: BannerPolicy,
}

impl Default for InputModelRust {
    fn default() -> Self {
        Self {
            sensitivity_text: QString::from(""),
            natural_text: QString::from(""),
            scroll_text: QString::from(""),
            binds_json: QString::from(""),
            banner_visible: false,
            guard: PollGuard::new(),
            banner: BannerPolicy::new(),
        }
    }
}

/// Keybind rows as QML-ready JSON: `[{combo,detail}]` rendered through
/// the core's `keybind_rows` (the same `(combo, marker)` pairs the CLI
/// table prints - managed binds carry the managed marker, user binds
/// the opaque dispatcher text). No client-side reformatting.
pub(crate) fn binds_json_for(
    statuses: &[archel_core::hyprland::binds::BindStatus],
) -> String {
    let rows: Vec<serde_json::Value> =
        archel_core::hyprland::binds::keybind_rows(statuses)
            .into_iter()
            .map(|(combo, detail)| serde_json::json!({"combo": combo, "detail": detail}))
            .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string())
}

/// Merges managed snippet binds with live compositor binds into one
/// `BindStatus` inventory, mirroring `run_keybinds_list` (managed
/// first, then user). A missing snippet means no managed binds yet -
/// never an error.
pub(crate) fn merge_binds(
    user: &[archel_core::hyprland::binds::UserBind],
    managed: &[archel_core::hyprland::binds::BindSpec],
) -> Vec<archel_core::hyprland::binds::BindStatus> {
    let mut statuses: Vec<archel_core::hyprland::binds::BindStatus> =
        managed.iter().map(archel_core::hyprland::binds::BindStatus::from).collect();
    statuses.extend(
        user.iter().map(archel_core::hyprland::binds::BindStatus::from),
    );
    statuses
}

impl InputModelRust {
    /// Applies a finished worker outcome on the Qt thread. A failed
    /// `read_input` fails the whole tick (N/A everywhere plus the
    /// once-per-episode banner); a failed binds read degrades to the
    /// honest empty list (the empty-state copy covers it).
    fn apply_outcome(
        &mut self,
        input: Result<archel_core::hyprland::input::InputState, archel_core::ArchelError>,
        binds: Vec<archel_core::hyprland::binds::BindStatus>,
        binds_ok: bool,
    ) -> AppliedInput {
        match input {
            Ok(state) => {
                self.banner.on_success();
                let _ = binds_ok;
                AppliedInput {
                    sensitivity_text: format!("{:.2}", state.sensitivity),
                    natural_text: if state.natural_scroll {
                        "on".to_string()
                    } else {
                        "off".to_string()
                    },
                    scroll_text: format!("{:.2}", state.scroll_factor),
                    binds_json: binds_json_for(&binds),
                    banner_visible: false,
                }
            }
            Err(_) => {
                self.banner.on_failure();
                AppliedInput {
                    sensitivity_text: NOT_AVAILABLE.to_string(),
                    natural_text: NOT_AVAILABLE.to_string(),
                    scroll_text: NOT_AVAILABLE.to_string(),
                    binds_json: "[]".to_string(),
                    banner_visible: self.banner.wants_visible(),
                }
            }
        }
    }
}

impl crate::gui::bridge_input::qobject::InputModel {
    /// 1 Hz entry point: skips while a tick is in flight (D-02),
    /// spawns the blocking input + binds reads, marshals back through
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
            use archel_core::hyprland::binds as core_binds;
            let input = archel_core::hyprland::input::read_input();
            let (binds, binds_ok) = match archel_core::hyprland::run_hyprctl(
                "hyprctl",
                &["binds", "-j"],
            ) {
                Ok(raw) => {
                    let user = core_binds::parse_binds_json(&raw);
                    let base = archel_core::hyprland::hypr_dir();
                    let body = std::fs::read_to_string(
                        archel_core::hyprland::snippet_path_at(&base),
                    )
                    .unwrap_or_default();
                    let managed = core_binds::parse_snippet_binds(&body);
                    (super::input::merge_binds(&user, &managed), true)
                }
                Err(_) => (Vec::new(), false),
            };
            let _ = qt_thread.queue(
                move |mut obj: core::pin::Pin<&mut Self>| {
                    let applied = {
                        let mut rust = obj.as_mut().rust_mut();
                        if !rust.guard.is_newest(tag) {
                            rust.guard.finish();
                            return;
                        }
                        rust.guard.finish();
                        rust.apply_outcome(input, binds, binds_ok)
                    };
                    obj.as_mut()
                        .set_sensitivity_text(QString::from(&applied.sensitivity_text));
                    obj.as_mut()
                        .set_natural_text(QString::from(&applied.natural_text));
                    obj.as_mut()
                        .set_scroll_text(QString::from(&applied.scroll_text));
                    obj.as_mut()
                        .set_binds_json(QString::from(&applied.binds_json));
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

    fn ready_input() -> archel_core::hyprland::input::InputState {
        archel_core::hyprland::input::InputState {
            sensitivity: 0.5,
            natural_scroll: true,
            scroll_factor: 1.5,
        }
    }

    fn failed_input(
    ) -> Result<archel_core::hyprland::input::InputState, archel_core::ArchelError> {
        Err(archel_core::ArchelError::hyprland_unavailable(
            "no compositor in test",
        ))
    }

    #[test]
    fn error_path_reads_na_and_signals_banner_once() {
        let mut model = InputModelRust::default();
        let first = model.apply_outcome(failed_input(), Vec::new(), false);
        for text in [
            &first.sensitivity_text,
            &first.natural_text,
            &first.scroll_text,
        ] {
            assert_eq!(text, NOT_AVAILABLE, "failed tick must read N/A");
        }
        assert_eq!(first.binds_json, "[]");
        assert!(first.banner_visible, "first failure shows the banner");
        let second = model.apply_outcome(failed_input(), Vec::new(), false);
        assert!(
            second.banner_visible,
            "consecutive failures keep the banner without re-signalling"
        );
    }

    #[test]
    fn binds_json_carries_core_rendered_rows_verbatim() {
        use archel_core::hyprland::binds as core_binds;
        let user = core_binds::parse_binds_json(
            r#"[{"key":"T","modmask":64,"submap":"","dispatcher":"__lua"}]"#,
        );
        assert_eq!(user.len(), 1, "fixture must parse one user bind");
        let statuses = merge_binds(&user, &[]);
        let json = binds_json_for(&statuses);
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(&json).expect("binds must be JSON");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["combo"], "SUPER + T");
        assert_eq!(
            rows[0]["detail"], "__lua",
            "user binds carry the opaque dispatcher text"
        );
    }

    #[test]
    fn success_fills_mouse_fields_and_empty_binds_degrade() {
        let mut model = InputModelRust::default();
        let _ = model.apply_outcome(failed_input(), Vec::new(), false);
        let applied = model.apply_outcome(Ok(ready_input()), Vec::new(), false);
        assert!(!applied.banner_visible, "success hides the banner");
        assert_eq!(applied.sensitivity_text, "0.50");
        assert_eq!(applied.natural_text, "on");
        assert_eq!(applied.scroll_text, "1.50");
        assert_eq!(
            applied.binds_json, "[]",
            "no binds is the honest empty state, not a banner trip"
        );
    }
}
