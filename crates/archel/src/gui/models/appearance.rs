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
/// `operation_busy`/`operation_kind` mirror the 06-06 pkg write-slot
/// shape (D-15: only the acting tab locks).
pub struct AppearanceModelRust {
    pub(crate) blur_text: QString,
    pub(crate) opacity_text: QString,
    pub(crate) wallpaper_text: QString,
    pub(crate) font_text: QString,
    pub(crate) banner_visible: bool,
    pub(crate) operation_busy: bool,
    pub(crate) operation_kind: QString,
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
            operation_busy: false,
            operation_kind: QString::from(""),
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

    /// Applies the blur switch (D-12: the worker opens the core
    /// confirm question first; the tab locks while it runs, D-15).
    pub fn set_blur(mut self: core::pin::Pin<&mut Self>, enabled: bool) {
        if !self.as_mut().begin_write_operation("blur") {
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
            let base = archel_core::hyprland::hypr_dir();
            blur_worker(
                &base,
                enabled,
                &PROD_APPEARANCE_OPS,
                super::super::modal::session(),
            );
            let _ = qt_thread.queue(|mut obj: core::pin::Pin<&mut Self>| {
                obj.as_mut().finish_write_operation();
            });
        });
    }

    /// Applies one or both opacity leaves from raw text (blank means
    /// "keep the current leaf"; both blank is a no-op, never a
    /// confirm).
    pub fn set_opacity(
        mut self: core::pin::Pin<&mut Self>,
        active: QString,
        inactive: QString,
    ) {
        let active_text = active.to_string();
        let inactive_text = inactive.to_string();
        if active_text.trim().is_empty() && inactive_text.trim().is_empty() {
            return;
        }
        if !self.as_mut().begin_write_operation("opacity") {
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
            let base = archel_core::hyprland::hypr_dir();
            let active_opt = if active_text.trim().is_empty() {
                None
            } else {
                Some(active_text.as_str())
            };
            let inactive_opt = if inactive_text.trim().is_empty() {
                None
            } else {
                Some(inactive_text.as_str())
            };
            opacity_worker(
                &base,
                active_opt,
                inactive_opt,
                &PROD_APPEARANCE_OPS,
                super::super::modal::session(),
            );
            let _ = qt_thread.queue(|mut obj: core::pin::Pin<&mut Self>| {
                obj.as_mut().finish_write_operation();
            });
        });
    }

    /// Applies a wallpaper path (blank is a no-op; the worker
    /// validates before any confirm, WR-02).
    pub fn set_wallpaper(mut self: core::pin::Pin<&mut Self>, path: QString) {
        let raw = path.to_string();
        if raw.trim().is_empty() {
            return;
        }
        if !self.as_mut().begin_write_operation("wallpaper") {
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
            let base = archel_core::hyprland::hypr_dir();
            wallpaper_worker(
                &base,
                &raw,
                &PROD_APPEARANCE_OPS,
                super::super::modal::session(),
            );
            let _ = qt_thread.queue(|mut obj: core::pin::Pin<&mut Self>| {
                obj.as_mut().finish_write_operation();
            });
        });
    }

    /// Applies a font family plus optional size (blank family is a
    /// no-op; the worker validates the composed spec first, WR-02).
    pub fn set_font(mut self: core::pin::Pin<&mut Self>, family: QString, size: QString) {
        let family_text = family.to_string();
        let size_text = size.to_string();
        if family_text.trim().is_empty() {
            return;
        }
        if !self.as_mut().begin_write_operation("font") {
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
            let base = archel_core::hyprland::hypr_dir();
            let spec = if size_text.trim().is_empty() {
                family_text.clone()
            } else {
                format!("{} {}", family_text.trim(), size_text.trim())
            };
            font_worker(
                &base,
                &spec,
                &PROD_APPEARANCE_OPS,
                super::super::modal::session(),
            );
            let _ = qt_thread.queue(|mut obj: core::pin::Pin<&mut Self>| {
                obj.as_mut().finish_write_operation();
            });
        });
    }

    /// Claims the single-operation slot (one write at a time; the
    /// second click of a double-click loses here) and mirrors the
    /// claim onto the QML-visible flags (D-15).
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

    /// Releases the operation slot and refreshes the tab so the
    /// fields reflect the change that just landed. Qt thread only
    /// (RESEARCH Pattern 1).
    fn finish_write_operation(mut self: core::pin::Pin<&mut Self>) {
        use cxx_qt::CxxQtType;
        self.as_mut().rust_mut().operation_busy = false;
        self.as_mut().set_operation_busy(false);
        self.as_mut().set_operation_kind(QString::from(""));
        self.as_mut().tick();
    }
}

// -- Write path (plan 06-08, Task 1) -----------------------------------
//
// Four workers, one per appearance store, all through the CLI-proven
// `apply_*_with` seams (validate -> core-verbatim confirm question ->
// live IPC -> line-preserving conf write). The GUI never edits conf
// text itself (plan prohibition 2); tests inject fake ops so no
// hyprctl, gsettings or hyprpaper child ever spawns headless.

use std::path::Path;

use crate::gui::modal::ModalSession;

/// Injection seam for the appearance writes: the exact core apply
/// legs as plain function pointers. Production passes
/// [`PROD_APPEARANCE_OPS`]; unit tests pass fakes.
pub(crate) struct AppearanceOps {
    pub apply_blur: fn(
        &Path,
        bool,
    ) -> Result<archel_core::hyprland::appearance::AppearanceReport, archel_core::ArchelError>,
    pub apply_opacity: fn(
        &Path,
        &archel_core::hyprland::appearance::OpacityPlan,
    ) -> Result<archel_core::hyprland::appearance::AppearanceReport, archel_core::ArchelError>,
    pub apply_wallpaper: fn(
        &Path,
        &archel_core::hyprland::wallpaper::WallpaperPlan,
    ) -> Result<archel_core::hyprland::wallpaper::WallpaperReport, archel_core::ArchelError>,
    pub apply_font: fn(
        &Path,
        &archel_core::hyprland::font::FontPlan,
    ) -> Result<archel_core::hyprland::font::FontReport, archel_core::ArchelError>,
}

/// Production legs: each wraps the matching `apply_*_with` with the
/// real hyprctl/gsettings/hyprpaper legs (the same legs the CLI
/// twins pass), parameterized by `base` so the workers stay
/// hermetic-capable.
fn prod_apply_blur(
    base: &Path,
    enabled: bool,
) -> Result<archel_core::hyprland::appearance::AppearanceReport, archel_core::ArchelError> {
    use archel_core::hyprland::{self, appearance as core_appearance};
    core_appearance::apply_blur_with(
        base,
        enabled,
        |lua| hyprland::run_hyprctl_eval("hyprctl", lua),
        || {
            hyprland::read_option(core_appearance::BLUR_KEY).and_then(|value| match value {
                hyprland::OptionValue::Bool(on) => Ok(on),
                other => Err(archel_core::ArchelError::hyprland_unavailable(format!(
                    "unexpected blur option shape: {other:?}"
                ))),
            })
        },
    )
}

fn prod_apply_opacity(
    base: &Path,
    plan: &archel_core::hyprland::appearance::OpacityPlan,
) -> Result<archel_core::hyprland::appearance::AppearanceReport, archel_core::ArchelError> {
    use archel_core::hyprland::{self, appearance as core_appearance};
    core_appearance::apply_opacity_with(
        base,
        plan,
        |lua| hyprland::run_hyprctl_eval("hyprctl", lua),
        || {
            let state = core_appearance::read_opacity()?;
            Ok((state.active_opacity, state.inactive_opacity))
        },
    )
}

fn prod_apply_wallpaper(
    base: &Path,
    plan: &archel_core::hyprland::wallpaper::WallpaperPlan,
) -> Result<archel_core::hyprland::wallpaper::WallpaperReport, archel_core::ArchelError> {
    use archel_core::hyprland::wallpaper as core_wallpaper;
    core_wallpaper::apply_wallpaper_with(
        base,
        plan,
        core_wallpaper::kill_hyprpaper,
        core_wallpaper::respawn_hyprpaper,
    )
}

fn prod_apply_font(
    base: &Path,
    plan: &archel_core::hyprland::font::FontPlan,
) -> Result<archel_core::hyprland::font::FontReport, archel_core::ArchelError> {
    use archel_core::hyprland::{self, font as core_font};
    core_font::apply_font_with(
        base,
        plan,
        |lua| hyprland::run_hyprctl_eval("hyprctl", lua),
        core_font::read_live_family,
        core_font::write_gtk_font,
    )
}

/// The production wiring (thin-view contract; PLAT-03).
pub(crate) const PROD_APPEARANCE_OPS: AppearanceOps = AppearanceOps {
    apply_blur: prod_apply_blur,
    apply_opacity: prod_apply_opacity,
    apply_wallpaper: prod_apply_wallpaper,
    apply_font: prod_apply_font,
};

/// Blocks the worker on the modal's one-shot answer. A dropped
/// session defaults to No (CLI `parse_confirm_answer` semantics).
fn await_confirm(sess: &ModalSession, question: &str) -> bool {
    let rx = sess.request_confirm(question);
    rx.recv().unwrap_or(false)
}

/// Blur worker: the core question verbatim, then the injected apply.
/// A decline touches neither IPC nor conf.
pub(crate) fn blur_worker(
    base: &Path,
    enabled: bool,
    ops: &AppearanceOps,
    sess: &ModalSession,
) {
    use archel_core::hyprland::appearance as core_appearance;
    if !await_confirm(sess, &core_appearance::confirm_blur_prompt(enabled)) {
        return;
    }
    if let Err(err) = (ops.apply_blur)(base, enabled) {
        sess.open_error(&err.message());
    }
}

/// Opacity worker: validate-plus-plan first (WR-02: typos fail
/// before any confirm), then the core question, then apply.
pub(crate) fn opacity_worker(
    base: &Path,
    active: Option<&str>,
    inactive: Option<&str>,
    ops: &AppearanceOps,
    sess: &ModalSession,
) {
    use archel_core::hyprland::appearance as core_appearance;
    let plan = match core_appearance::plan_opacity_set_at(base, active, inactive) {
        Ok(plan) => plan,
        Err(err) => {
            sess.open_error(&err.message());
            return;
        }
    };
    if !await_confirm(sess, &core_appearance::confirm_opacity_prompt(&plan)) {
        return;
    }
    if let Err(err) = (ops.apply_opacity)(base, &plan) {
        sess.open_error(&err.message());
    }
}

/// Wallpaper worker: path validation first (WR-02), then confirm,
/// then the slice rewrite plus daemon restart.
pub(crate) fn wallpaper_worker(
    base: &Path,
    raw: &str,
    ops: &AppearanceOps,
    sess: &ModalSession,
) {
    use archel_core::hyprland::wallpaper as core_wallpaper;
    let plan = match core_wallpaper::plan_wallpaper_set(raw) {
        Ok(plan) => plan,
        Err(err) => {
            sess.open_error(&err.message());
            return;
        }
    };
    if !await_confirm(sess, &core_wallpaper::confirm_wallpaper_prompt(&plan)) {
        return;
    }
    if let Err(err) = (ops.apply_wallpaper)(base, &plan) {
        sess.open_error(&err.message());
    }
}

/// Font worker: spec validation first (WR-02), then confirm, then
/// the two-store apply with read-back.
pub(crate) fn font_worker(
    base: &Path,
    spec: &str,
    ops: &AppearanceOps,
    sess: &ModalSession,
) {
    use archel_core::hyprland::font as core_font;
    let plan = match core_font::plan_font_set_at(base, spec) {
        Ok(plan) => plan,
        Err(err) => {
            sess.open_error(&err.message());
            return;
        }
    };
    if !await_confirm(sess, &core_font::confirm_font_prompt(&plan)) {
        return;
    }
    if let Err(err) = (ops.apply_font)(base, &plan) {
        sess.open_error(&err.message());
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

    // -- Write-path choreography (plan 06-08, Task 1) --
    //
    // The fakes below replace only the four `AppearanceOps` seams;
    // the workers run for real against a local ModalSession, so
    // validate-before-confirm, default-No and the verbatim core
    // questions are all proven without spawning hyprctl, gsettings
    // or hyprpaper.

    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
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
        let base = std::env::temp_dir().join(format!("archel-appearance-gui-{tag}-{unique}"));
        std::fs::create_dir_all(&base).expect("test base");
        base
    }

    fn ok_report() -> archel_core::hyprland::appearance::AppearanceReport {
        archel_core::hyprland::appearance::AppearanceReport {
            blur_enabled: true,
            active_opacity: 0.95,
            inactive_opacity: 0.85,
            read_back_ok: true,
        }
    }

    fn ok_wallpaper_report() -> archel_core::hyprland::wallpaper::WallpaperReport {
        archel_core::hyprland::wallpaper::WallpaperReport {
            path: "/tmp/wall.png".to_string(),
            restarted: true,
        }
    }

    fn ok_font_report() -> archel_core::hyprland::font::FontReport {
        archel_core::hyprland::font::FontReport {
            family: "Cantarell".to_string(),
            size: Some(11.0),
            gtk_value: "Cantarell 11".to_string(),
            read_back_ok: true,
        }
    }

    fn succeeding_ops() -> AppearanceOps {
        AppearanceOps {
            apply_blur: |_, _| Ok(ok_report()),
            apply_opacity: |_, _| Ok(ok_report()),
            apply_wallpaper: |_, _| Ok(ok_wallpaper_report()),
            apply_font: |_, _| Ok(ok_font_report()),
        }
    }

    #[test]
    fn blur_decline_asks_the_core_question_and_touches_nothing() {
        static APPLY_CALLS: AtomicUsize = AtomicUsize::new(0);
        let ops = AppearanceOps {
            apply_blur: |_, _| {
                APPLY_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
                Ok(ok_report())
            },
            ..succeeding_ops()
        };
        let sess = ModalSession::default();
        let base = test_base("blur-decline");
        std::thread::scope(|s| {
            s.spawn(|| blur_worker(&base, true, &ops, &sess));
            wait_until("blur confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            assert_eq!(
                sess.snapshot_question(),
                archel_core::hyprland::appearance::confirm_blur_prompt(true),
                "the question is the core prompt verbatim"
            );
            sess.resolve_confirm(false);
        });
        assert_eq!(
            APPLY_CALLS.load(AtomicOrdering::SeqCst),
            0,
            "decline skips both IPC and the conf write"
        );
        assert_eq!(sess.snapshot_kind(), ModalKind::None);
    }

    #[test]
    fn blur_accept_reaches_the_apply_leg() {
        static APPLY_CALLS: AtomicUsize = AtomicUsize::new(0);
        let ops = AppearanceOps {
            apply_blur: |_, enabled| {
                APPLY_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
                assert!(enabled, "the switch value reaches the leg");
                Ok(ok_report())
            },
            ..succeeding_ops()
        };
        let sess = ModalSession::default();
        let base = test_base("blur-accept");
        std::thread::scope(|s| {
            s.spawn(|| blur_worker(&base, true, &ops, &sess));
            wait_until("blur confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            sess.resolve_confirm(true);
            wait_until("worker settles", || {
                APPLY_CALLS.load(AtomicOrdering::SeqCst) == 1
            });
        });
        assert_eq!(sess.snapshot_kind(), ModalKind::None);
    }

    #[test]
    fn blur_apply_failure_surfaces_the_core_message() {
        let ops = AppearanceOps {
            apply_blur: |_, _| {
                Err(archel_core::ArchelError::hyprland_unavailable(
                    "live read-back disagrees",
                ))
            },
            ..succeeding_ops()
        };
        let sess = ModalSession::default();
        let base = test_base("blur-fail");
        std::thread::scope(|s| {
            s.spawn(|| blur_worker(&base, false, &ops, &sess));
            wait_until("blur confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            sess.resolve_confirm(true);
            wait_until("error opens", || sess.snapshot_kind() == ModalKind::Error);
        });
        assert_eq!(
            sess.snapshot_error(),
            archel_core::ArchelError::hyprland_unavailable("live read-back disagrees")
                .message(),
            "ErrorModal carries the core French message verbatim"
        );
    }

    #[test]
    fn invalid_opacity_is_rejected_before_any_confirm() {
        static APPLY_CALLS: AtomicUsize = AtomicUsize::new(0);
        let ops = AppearanceOps {
            apply_opacity: |_, _| {
                APPLY_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
                Ok(ok_report())
            },
            ..succeeding_ops()
        };
        let sess = ModalSession::default();
        let base = test_base("opacity-invalid");
        opacity_worker(&base, Some("abc"), None, &ops, &sess);
        assert_eq!(
            sess.snapshot_kind(),
            ModalKind::Error,
            "a typo never reaches a confirm"
        );
        assert_eq!(APPLY_CALLS.load(AtomicOrdering::SeqCst), 0);
    }

    #[test]
    fn opacity_decline_writes_no_conf() {
        static APPLY_CALLS: AtomicUsize = AtomicUsize::new(0);
        let ops = AppearanceOps {
            apply_opacity: |_, _| {
                APPLY_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
                Ok(ok_report())
            },
            ..succeeding_ops()
        };
        let sess = ModalSession::default();
        let base = test_base("opacity-decline");
        std::thread::scope(|s| {
            s.spawn(|| opacity_worker(&base, Some("0.9"), Some("0.8"), &ops, &sess));
            wait_until("opacity confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            sess.resolve_confirm(false);
        });
        assert_eq!(APPLY_CALLS.load(AtomicOrdering::SeqCst), 0);
        assert_eq!(sess.snapshot_kind(), ModalKind::None);
    }

    #[test]
    fn opacity_conf_names_the_planned_leaves() {
        use archel_core::hyprland::appearance as core_appearance;
        let base = test_base("opacity-conf");
        let plan = core_appearance::plan_opacity_set_at(&base, Some("0.9"), None)
            .expect("plan validates");
        let rendered = core_appearance::render_snippet(&archel_core::hyprland::appearance::AppearanceState {
            blur_enabled: true,
            active_opacity: plan.active.unwrap_or(plan.old_active),
            inactive_opacity: plan.inactive.unwrap_or(plan.old_inactive),
            font_family: None,
        });
        assert!(
            rendered.contains("active_opacity = 0.9"),
            "the conf transformation carries the planned leaf: {rendered}"
        );
        assert!(
            rendered.contains("inactive_opacity = 1"),
            "the untouched leaf round-trips the default: {rendered}"
        );
    }

    #[test]
    fn wallpaper_invalid_path_fails_before_confirm() {
        static APPLY_CALLS: AtomicUsize = AtomicUsize::new(0);
        let ops = AppearanceOps {
            apply_wallpaper: |_, _| {
                APPLY_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
                Ok(ok_wallpaper_report())
            },
            ..succeeding_ops()
        };
        let sess = ModalSession::default();
        let base = test_base("wallpaper-invalid");
        wallpaper_worker(&base, "/no/such/file.xyz", &ops, &sess);
        assert_eq!(sess.snapshot_kind(), ModalKind::Error);
        assert_eq!(APPLY_CALLS.load(AtomicOrdering::SeqCst), 0);
    }

    #[test]
    fn wallpaper_slice_renders_the_planned_path() {
        use archel_core::hyprland::wallpaper as core_wallpaper;
        let path = std::path::Path::new("/home/axel/wall.png");
        let rendered = core_wallpaper::render_hyprpaper_slice(path);
        assert!(
            rendered.contains("/home/axel/wall.png"),
            "the slice carries the planned path: {rendered}"
        );
    }

    #[test]
    fn font_invalid_spec_fails_before_confirm() {
        static APPLY_CALLS: AtomicUsize = AtomicUsize::new(0);
        let ops = AppearanceOps {
            apply_font: |_, _| {
                APPLY_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
                Ok(ok_font_report())
            },
            ..succeeding_ops()
        };
        let sess = ModalSession::default();
        let base = test_base("font-invalid");
        font_worker(&base, "", &ops, &sess);
        assert_eq!(sess.snapshot_kind(), ModalKind::Error);
        assert_eq!(APPLY_CALLS.load(AtomicOrdering::SeqCst), 0);
    }

    #[test]
    fn font_conf_transformation_keeps_sibling_leaves() {
        use archel_core::hyprland::appearance as core_appearance;
        use archel_core::hyprland::font as core_font;
        let base = test_base("font-conf");
        let plan =
            core_font::plan_font_set_at(&base, "Cantarell 11").expect("spec validates");
        assert_eq!(plan.family, "Cantarell");
        let rendered = core_appearance::render_snippet(&core_appearance::AppearanceState {
            blur_enabled: true,
            active_opacity: 1.0,
            inactive_opacity: 1.0,
            font_family: Some(plan.family.clone()),
        });
        assert!(
            rendered.contains("font_family = \"Cantarell\""),
            "the conf transformation carries the planned family: {rendered}"
        );
        assert!(
            rendered.contains("blur = { enabled = true }"),
            "sibling leaves survive the font write: {rendered}"
        );
    }
}
