//! Appearance view-model: blur switch, opacity leaves, wallpaper path
//! and font stores (read-only; writes land in plan 08).
//!
//! Same proven shape as `models::audio`: `tick()` is a QML-called
//! invokable that only spawns - the sync core reads (`read_blur`,
//! `current_snippet_state`, `read_wallpaper_at`, `read_font`) run on
//! the gui runtime's blocking pool and the finished payload returns
//! exclusively through `CxxQtThread.queue`, whose closure is the only
//! code allowed to set properties (RESEARCH Pattern 1). Every read
//! goes through the CLI-proven core entry points (PLAT-03); no
//! hyprland.conf parsing or hyprctl dispatch lives here.

use cxx_qt_lib::QString;

use super::super::poll::{BannerPolicy, PollGuard};
use crate::gui::models::system::NOT_AVAILABLE;

/// Fully formatted tick payload, built off the Qt thread so the
/// `queue()` closure only sets properties.
pub(crate) struct AppliedAppearance {
    pub blur_text: String,
    pub opacity_text: String,
    pub wallpaper_text: String,
    pub font_text: String,
    pub banner_visible: bool,
}

/// Backing state: property fields plus the poll guard and banner
/// policy. No shared core handle: every tick is an independent read.
pub struct AppearanceModelRust {
    pub(crate) blur_text: QString,
    pub(crate) opacity_text: QString,
    pub(crate) wallpaper_text: QString,
    pub(crate) font_text: QString,
    pub(crate) banner_visible: bool,
    guard: PollGuard,
    banner: BannerPolicy,
}

impl Default for AppearanceModelRust {
    fn default() -> Self {
        Self {
            blur_text: QString::from(""),
            opacity_text: QString::from(""),
            wallpaper_text: QString::from(""),
            font_text: QString::from(""),
            banner_visible: false,
            guard: PollGuard::new(),
            banner: BannerPolicy::new(),
        }
    }
}

impl AppearanceModelRust {
    /// Applies a finished worker outcome on the Qt thread. A failed
    /// live blur read fails the whole tick (N/A everywhere plus the
    /// once-per-episode banner); font and wallpaper failures degrade
    /// to per-field N/A without nuking the tab.
    fn apply_outcome(
        &mut self,
        blur: Result<bool, archel_core::ArchelError>,
        snippet: Result<
            archel_core::hyprland::appearance::AppearanceState,
            archel_core::ArchelError,
        >,
        wallpaper: archel_core::hyprland::wallpaper::WallpaperStatus,
        font: Result<archel_core::hyprland::font::FontStatus, archel_core::ArchelError>,
    ) -> AppliedAppearance {
        match (blur, snippet) {
            (Ok(enabled), Ok(state)) => {
                self.banner.on_success();
                let on_off = if enabled { "on" } else { "off" };
                AppliedAppearance {
                    blur_text: on_off.to_string(),
                    opacity_text: format!(
                        "{:.2} / {:.2}",
                        state.active_opacity, state.inactive_opacity
                    ),
                    wallpaper_text: wallpaper.path.unwrap_or_else(|| NOT_AVAILABLE.to_string()),
                    font_text: font
                        .map(|status| match status.gtk_size {
                            Some(size) => format!("{} {:.1}", status.gtk_family, size),
                            None => status.gtk_family,
                        })
                        .unwrap_or_else(|_| NOT_AVAILABLE.to_string()),
                    banner_visible: false,
                }
            }
            _ => {
                self.banner.on_failure();
                AppliedAppearance {
                    blur_text: NOT_AVAILABLE.to_string(),
                    opacity_text: NOT_AVAILABLE.to_string(),
                    wallpaper_text: NOT_AVAILABLE.to_string(),
                    font_text: NOT_AVAILABLE.to_string(),
                    banner_visible: self.banner.wants_visible(),
                }
            }
        }
    }
}

impl crate::gui::bridge_appearance::qobject::AppearanceModel {
    /// 1 Hz entry point: skips while a tick is in flight (D-02),
    /// spawns the blocking reads, marshals back through `queue()`
    /// only.
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
            let blur = archel_core::hyprland::appearance::read_blur();
            let base = archel_core::hyprland::hypr_dir();
            let snippet = archel_core::hyprland::appearance::current_snippet_state(&base);
            let wallpaper = archel_core::hyprland::wallpaper::read_wallpaper_at(&base);
            let font = archel_core::hyprland::font::read_font();
            let _ = qt_thread.queue(
                move |mut obj: core::pin::Pin<&mut Self>| {
                    let applied = {
                        let mut rust = obj.as_mut().rust_mut();
                        if !rust.guard.is_newest(tag) {
                            rust.guard.finish();
                            return;
                        }
                        rust.guard.finish();
                        rust.apply_outcome(blur, snippet, wallpaper, font)
                    };
                    obj.as_mut()
                        .set_blur_text(QString::from(&applied.blur_text));
                    obj.as_mut()
                        .set_opacity_text(QString::from(&applied.opacity_text));
                    obj.as_mut()
                        .set_wallpaper_text(QString::from(&applied.wallpaper_text));
                    obj.as_mut()
                        .set_font_text(QString::from(&applied.font_text));
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

    fn ready_snippet() -> archel_core::hyprland::appearance::AppearanceState {
        archel_core::hyprland::appearance::AppearanceState {
            blur_enabled: true,
            active_opacity: 0.95,
            inactive_opacity: 0.85,
            font_family: None,
        }
    }

    fn ready_font() -> archel_core::hyprland::font::FontStatus {
        archel_core::hyprland::font::FontStatus {
            gtk_family: "Cantarell".to_string(),
            gtk_size: Some(11.0),
            hyprland_family: "Cantarell".to_string(),
        }
    }

    fn empty_wallpaper() -> archel_core::hyprland::wallpaper::WallpaperStatus {
        archel_core::hyprland::wallpaper::WallpaperStatus { path: None }
    }

    fn failed_blur() -> Result<bool, archel_core::ArchelError> {
        Err(archel_core::ArchelError::hyprland_unavailable(
            "no compositor in test",
        ))
    }

    #[test]
    fn error_path_reads_na_and_signals_banner_once() {
        let mut model = AppearanceModelRust::default();
        let first = model.apply_outcome(
            failed_blur(),
            Ok(ready_snippet()),
            empty_wallpaper(),
            Ok(ready_font()),
        );
        for text in [
            &first.blur_text,
            &first.opacity_text,
            &first.wallpaper_text,
            &first.font_text,
        ] {
            assert_eq!(text, NOT_AVAILABLE, "failed tick must read N/A");
        }
        assert!(first.banner_visible, "first failure shows the banner");
        let second = model.apply_outcome(
            failed_blur(),
            Ok(ready_snippet()),
            empty_wallpaper(),
            Ok(ready_font()),
        );
        assert!(
            second.banner_visible,
            "consecutive failures keep the banner without re-signalling"
        );
    }

    #[test]
    fn success_fills_fields_from_core_reads() {
        let mut model = AppearanceModelRust::default();
        let _ = model.apply_outcome(
            failed_blur(),
            Ok(ready_snippet()),
            empty_wallpaper(),
            Ok(ready_font()),
        );
        let applied = model.apply_outcome(
            Ok(true),
            Ok(ready_snippet()),
            archel_core::hyprland::wallpaper::WallpaperStatus {
                path: Some("/home/axel/wall.png".to_string()),
            },
            Ok(ready_font()),
        );
        assert!(!applied.banner_visible, "success hides the banner");
        assert_eq!(applied.blur_text, "on");
        assert_eq!(applied.opacity_text, "0.95 / 0.85");
        assert_eq!(applied.wallpaper_text, "/home/axel/wall.png");
        assert_eq!(applied.font_text, "Cantarell 11.0");
    }

    #[test]
    fn font_and_wallpaper_gaps_degrade_per_field() {
        let mut model = AppearanceModelRust::default();
        let font_err: Result<
            archel_core::hyprland::font::FontStatus,
            archel_core::ArchelError,
        > = Err(archel_core::ArchelError::hyprland_unavailable(
            "gsettings missing in test",
        ));
        let applied =
            model.apply_outcome(Ok(false), Ok(ready_snippet()), empty_wallpaper(), font_err);
        assert_eq!(applied.blur_text, "off", "blur stays live");
        assert_eq!(
            applied.wallpaper_text, NOT_AVAILABLE,
            "missing slice reads N/A"
        );
        assert_eq!(
            applied.font_text, NOT_AVAILABLE,
            "font failure degrades per field"
        );
        assert!(
            !applied.banner_visible,
            "per-field gaps must not trip the banner"
        );
    }
}
