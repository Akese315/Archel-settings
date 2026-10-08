//! `archel appearance blur|opacity` renderers: French tables
//! (default) or JSON (`--json`).
//!
//! Bin-calls-core shape: every French string comes from
//! `archel-core::hyprland::appearance` (titles, labels, state wording,
//! confirm prompts, ok lines) — this module only lays out columns
//! (`std::fmt` widths, no table crate) and frames the JSON payloads.
//! Mutations are confirm-gated in the bin (`confirm_action`: preview
//! naming the target, `[o/N]`, `NeedsTty` when piped) and verified by
//! `getoption` read-back inside core before the ok line prints.

use archel_core::ArchelError;

/// Runs `archel appearance blur on|off`: preview, confirm, then flip
/// the live switch with snippet durability (APP-03 tracer).
pub fn run_blur(enabled: bool, json: bool) -> Result<(), ArchelError> {
    use archel_core::hyprland::appearance as core_appearance;
    run_blur_confirmed(enabled, json, core_appearance::set_blur)
}

/// Confirm-then-execute gate for the blur switch (mirrors the proxy
/// shape): the preview renders, `confirm_action` enforces the TTY +
/// `[o/N]` rule, and only a yes reaches `apply`. The injectable
/// `apply` keeps the abort path unit-coverable headless.
fn run_blur_confirmed(
    enabled: bool,
    json: bool,
    apply: impl FnOnce(bool) -> Result<archel_core::hyprland::appearance::AppearanceReport, ArchelError>,
) -> Result<(), ArchelError> {
    use archel_core::hyprland::appearance as core_appearance;
    if !json {
        print!(
            "{}",
            render_label_table(
                &core_appearance::blur_preview_title(),
                &core_appearance::blur_preview_rows(enabled)
            )
        );
    }
    archel_core::packages::confirm_action(&core_appearance::confirm_blur_prompt(enabled))?;
    let report = apply(enabled)?;
    if !report.read_back_ok {
        return Err(ArchelError::hyprland_unavailable(
            "live read-back disagrees with the applied blur value",
        ));
    }
    if json {
        println!("{}", encode_json(&report, "appearance blur")?);
    } else {
        println!("{}", core_appearance::blur_ok_line(report.blur_enabled));
    }
    Ok(())
}

/// Runs `archel appearance opacity set`: validated floats, preview,
/// confirm, live-apply with read-back (APP-04, expansion task).
pub fn run_opacity_set(
    active: Option<String>,
    inactive: Option<String>,
    json: bool,
) -> Result<(), ArchelError> {
    use archel_core::hyprland::appearance as core_appearance;
    let plan = core_appearance::plan_opacity_set(active.as_deref(), inactive.as_deref())?;
    run_opacity_plan_confirmed(&plan, json, core_appearance::apply_opacity_plan)
}

/// Runs `archel appearance opacity` (bare): live values of both
/// leaves, no confirm, no write (read-only).
pub fn run_opacity(json: bool) -> Result<(), ArchelError> {
    use archel_core::hyprland::appearance as core_appearance;
    let state = core_appearance::read_opacity()?;
    if json {
        println!("{}", encode_json(&state, "appearance opacity")?);
    } else {
        print!(
            "{}",
            render_label_table(
                &core_appearance::opacity_title(),
                &core_appearance::opacity_rows(&state)
            )
        );
    }
    Ok(())
}

/// Confirm-then-execute gate for opacity plans (mirrors the blur
/// shape above): preview, TTY + `[o/N]`, injectable apply for the
/// headless abort test.
fn run_opacity_plan_confirmed(
    plan: &archel_core::hyprland::appearance::OpacityPlan,
    json: bool,
    apply: impl FnOnce(
        &archel_core::hyprland::appearance::OpacityPlan,
    ) -> Result<archel_core::hyprland::appearance::AppearanceReport, ArchelError>,
) -> Result<(), ArchelError> {
    use archel_core::hyprland::appearance as core_appearance;
    if !json {
        print!(
            "{}",
            render_label_table(
                &core_appearance::opacity_preview_title(plan),
                &core_appearance::opacity_preview_rows(plan)
            )
        );
    }
    archel_core::packages::confirm_action(&core_appearance::confirm_opacity_prompt(plan))?;
    let report = apply(plan)?;
    if !report.read_back_ok {
        return Err(ArchelError::hyprland_unavailable(
            "live read-back disagrees with the applied opacity values",
        ));
    }
    if json {
        println!("{}", encode_json(&report, "appearance opacity")?);
    } else {
        print!(
            "{}",
            render_label_table(
                &core_appearance::opacity_report_title(),
                &core_appearance::opacity_report_rows(&report)
            )
        );
    }
    Ok(())
}

/// Runs `archel appearance font` (bare): both stores (GTK gsettings
/// plus live Hyprland leaf), no confirm, no write (read-only).
pub fn run_font(json: bool) -> Result<(), ArchelError> {
    use archel_core::hyprland::font as core_font;
    let status = core_font::read_font()?;
    if json {
        println!("{}", encode_json(&status, "appearance font")?);
    } else {
        print!(
            "{}",
            render_label_table(&core_font::font_title(), &core_font::font_rows(&status))
        );
    }
    Ok(())
}

/// Runs `archel appearance font set '<family> [size]'`: validated
/// spec, preview, confirm, two-store apply with read-back (APP-02).
pub fn run_font_set(value: String, json: bool) -> Result<(), ArchelError> {
    use archel_core::hyprland::font as core_font;
    // Read-only planning first: validation plus old-snippet read run
    // before any prompt, so typos fail fast with no TTY need.
    let plan = core_font::plan_font_set_at(
        &archel_core::hyprland::hypr_dir(),
        &value,
    )?;
    run_font_plan_confirmed(&plan, json, |plan| {
        core_font::apply_font_with(
            &archel_core::hyprland::hypr_dir(),
            plan,
            |lua| archel_core::hyprland::run_hyprctl_eval("hyprctl", lua),
            core_font::read_live_family,
            core_font::write_gtk_font,
        )
    })
}

/// Confirm-then-execute gate for font plans (mirrors the blur shape):
/// preview, TTY + `[o/N]`, injectable apply for the headless abort
/// test.
fn run_font_plan_confirmed(
    plan: &archel_core::hyprland::font::FontPlan,
    json: bool,
    apply: impl FnOnce(
        &archel_core::hyprland::font::FontPlan,
    ) -> Result<archel_core::hyprland::font::FontReport, ArchelError>,
) -> Result<(), ArchelError> {
    use archel_core::hyprland::font as core_font;
    if !json {
        print!(
            "{}",
            render_label_table(
                &core_font::font_preview_title(),
                &core_font::font_preview_rows(plan)
            )
        );
    }
    archel_core::packages::confirm_action(&core_font::confirm_font_prompt(plan))?;
    let report = apply(plan)?;
    if !report.read_back_ok {
        return Err(ArchelError::hyprland_unavailable(
            "live read-back disagrees with the applied font family",
        ));
    }
    if json {
        println!("{}", encode_json(&report, "appearance font")?);
    } else {
        println!("{}", core_font::font_ok_line(&report));
    }
    Ok(())
}

/// Runs `archel appearance wallpaper` (bare): the managed slice path,
/// no confirm, no write (read-only).
pub fn run_wallpaper(json: bool) -> Result<(), ArchelError> {
    use archel_core::hyprland::wallpaper as core_wallpaper;
    let status = core_wallpaper::read_wallpaper_at(&archel_core::hyprland::hypr_dir());
    if json {
        println!("{}", encode_json(&status, "appearance wallpaper")?);
    } else if status.path.is_none() {
        println!("{}", core_wallpaper::wallpaper_empty_line());
    } else {
        print!(
            "{}",
            render_label_table(
                &core_wallpaper::wallpaper_title(),
                &core_wallpaper::wallpaper_rows(&status)
            )
        );
    }
    Ok(())
}

/// Runs `archel appearance wallpaper set <path>`: validated path,
/// preview, confirm, slice rewrite plus daemon restart (APP-01).
pub fn run_wallpaper_set(path: String, json: bool) -> Result<(), ArchelError> {
    use archel_core::hyprland::wallpaper as core_wallpaper;
    // Read-only planning first: validation runs before any prompt, so
    // bad paths fail fast with no TTY need.
    let plan = core_wallpaper::plan_wallpaper_set(&path)?;
    run_wallpaper_plan_confirmed(&plan, json, |plan| {
        core_wallpaper::apply_wallpaper_with(
            &archel_core::hyprland::hypr_dir(),
            plan,
            core_wallpaper::kill_hyprpaper,
            core_wallpaper::respawn_hyprpaper,
        )
    })
}

/// Confirm-then-execute gate for wallpaper plans (mirrors the font
/// shape): preview, TTY + `[o/N]`, injectable lifecycle for the
/// headless abort test.
fn run_wallpaper_plan_confirmed(
    plan: &archel_core::hyprland::wallpaper::WallpaperPlan,
    json: bool,
    apply: impl FnOnce(
        &archel_core::hyprland::wallpaper::WallpaperPlan,
    ) -> Result<archel_core::hyprland::wallpaper::WallpaperReport, ArchelError>,
) -> Result<(), ArchelError> {
    use archel_core::hyprland::wallpaper as core_wallpaper;
    if !json {
        print!(
            "{}",
            render_label_table(
                &core_wallpaper::wallpaper_preview_title(),
                &core_wallpaper::wallpaper_preview_rows(plan)
            )
        );
    }
    archel_core::packages::confirm_action(&core_wallpaper::confirm_wallpaper_prompt(plan))?;
    let report = apply(plan)?;
    if !report.restarted {
        return Err(ArchelError::hyprland_unavailable(
            "the wallpaper daemon did not restart after the conf write",
        ));
    }
    if json {
        println!("{}", encode_json(&report, "appearance wallpaper")?);
    } else {
        println!("{}", core_wallpaper::wallpaper_ok_line(&report));
    }
    Ok(())
}

/// Encodes one `--json` payload (single object per line, NDJSON-safe).
fn encode_json(payload: &impl serde::Serialize, what: &str) -> Result<String, ArchelError> {
    serde_json::to_string(payload)
        .map_err(|err| ArchelError::Internal(format!("cannot encode {what} JSON: {err}")))
}

/// Right-pads a label on char count (mirrors the `net_cmds` helper —
/// byte widths would misalign accented French labels).
fn pad_right(label: &str, width: usize) -> String {
    let mut out = label.to_string();
    while out.chars().count() < width {
        out.push(' ');
    }
    out
}

/// Generic French label table (title + padded `label : value` rows).
fn render_label_table(title: &str, rows: &[(String, String)]) -> String {
    let width = rows
        .iter()
        .map(|(label, _)| label.chars().count())
        .max()
        .unwrap_or(0);
    let mut out = format!("{title}\n");
    for (label, value) in rows {
        out.push_str(&format!("{} : {value}\n", pad_right(label, width)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blur_preview_names_target_state() {
        use archel_core::hyprland::appearance as core_appearance;
        archel_core::i18n::set_language("fr");
        let preview = render_label_table(
            &core_appearance::blur_preview_title(),
            &core_appearance::blur_preview_rows(false),
        );
        assert!(
            preview.contains(&core_appearance::blur_preview_title()),
            "preview keeps its title"
        );
        // Diacritic-free matchers (D-07 guard: no French diacritics in .rs —
        // same precedent as error.rs "graphique"). "sactiv" matches only the
        // off-state value, never the on-state one.
        assert!(
            preview.contains("sactiv"),
            "preview names the target state: {preview:?}"
        );
        let prompt = core_appearance::confirm_blur_prompt(true);
        assert!(
            prompt.contains("activ") && !prompt.contains("sactiv"),
            "confirm names the target: {prompt:?}"
        );
    }

    #[test]
    fn blur_report_json_schema_pins_apply_fields() {
        use archel_core::hyprland::appearance::AppearanceReport;
        let report = AppearanceReport {
            blur_enabled: true,
            active_opacity: 1.0,
            inactive_opacity: 1.0,
            read_back_ok: true,
        };
        let value: serde_json::Value =
            serde_json::to_value(&report).expect("AppearanceReport serializes");
        for field in ["blur_enabled", "active_opacity", "inactive_opacity", "read_back_ok"] {
            assert!(value.get(field).is_some(), "report must carry `{field}`");
        }
        insta::assert_json_snapshot!("appearance_blur", value);
    }

    #[test]
    fn blur_confirm_abort_spawns_no_write() {
        // Same injectable-closure abort gate as DNS/proxy: headless
        // stdin is not a TTY, so the gate refuses before the closure —
        // the abort path structurally spawns no `hyprctl eval`. The
        // headless precondition is asserted, never a silent return
        // (WR-04): an interactive run fails loudly instead of passing
        // without checking the safety gate.
        use std::io::IsTerminal;
        assert!(
            !std::io::stdin().is_terminal(),
            "abort-gate test requires headless stdin"
        );
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let writes = Arc::new(AtomicUsize::new(0));
        let probe = Arc::clone(&writes);
        let err = run_blur_confirmed(true, false, move |_| {
            probe.fetch_add(1, Ordering::SeqCst);
            Ok(archel_core::hyprland::appearance::AppearanceReport {
                blur_enabled: true,
                active_opacity: 1.0,
                inactive_opacity: 1.0,
                read_back_ok: true,
            })
        })
        .expect_err("headless confirm must refuse");
        assert!(
            matches!(err, ArchelError::NeedsTty),
            "abort path needs the typed refusal, never a write: {err:?}"
        );
        assert_eq!(
            writes.load(Ordering::SeqCst),
            0,
            "the abort path spawns no hyprctl write"
        );
    }

    #[test]
    fn opacity_preview_names_both_leaves() {
        use archel_core::hyprland::appearance as core_appearance;
        archel_core::i18n::set_language("fr");
        let plan = core_appearance::OpacityPlan {
            active: Some(0.9),
            inactive: Some(0.8),
            old_active: 1.0,
            old_inactive: 1.0,
        };
        let preview = render_label_table(
            &core_appearance::opacity_preview_title(&plan),
            &core_appearance::opacity_preview_rows(&plan),
        );
        for needle in ["0.9", "0.8"] {
            assert!(preview.contains(needle), "preview must show `{needle}`");
        }
    }

    #[test]
    fn opacity_report_json_schema_pins_apply_fields() {
        use archel_core::hyprland::appearance::AppearanceReport;
        let report = AppearanceReport {
            blur_enabled: true,
            active_opacity: 0.9,
            inactive_opacity: 0.8,
            read_back_ok: true,
        };
        let value: serde_json::Value =
            serde_json::to_value(&report).expect("AppearanceReport serializes");
        for field in ["blur_enabled", "active_opacity", "inactive_opacity", "read_back_ok"] {
            assert!(value.get(field).is_some(), "report must carry `{field}`");
        }
        insta::assert_json_snapshot!("appearance_opacity", value);
    }

    #[test]
    fn opacity_confirm_abort_spawns_no_write() {
        // Same gate as blur: headless stdin refuses before the closure.
        // Headless precondition asserted, never silently returned (WR-04).
        use std::io::IsTerminal;
        assert!(
            !std::io::stdin().is_terminal(),
            "abort-gate test requires headless stdin"
        );
        use archel_core::hyprland::appearance as core_appearance;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let writes = Arc::new(AtomicUsize::new(0));
        let probe = Arc::clone(&writes);
        let plan = core_appearance::OpacityPlan {
            active: Some(0.9),
            inactive: None,
            old_active: 1.0,
            old_inactive: 1.0,
        };
        let err = run_opacity_plan_confirmed(&plan, false, move |_| {
            probe.fetch_add(1, Ordering::SeqCst);
            Ok(core_appearance::AppearanceReport {
                blur_enabled: true,
                active_opacity: 0.9,
                inactive_opacity: 1.0,
                read_back_ok: true,
            })
        })
        .expect_err("headless confirm must refuse");
        assert!(
            matches!(err, ArchelError::NeedsTty),
            "abort path needs the typed refusal, never a write: {err:?}"
        );
        assert_eq!(
            writes.load(Ordering::SeqCst),
            0,
            "the abort path spawns no hyprctl write"
        );
    }

    #[test]
    fn font_preview_names_family_and_size() {
        use archel_core::hyprland::font as core_font;
        archel_core::i18n::set_language("fr");
        let plan = core_font::FontPlan {
            family: "Adwaita Sans".to_string(),
            size: Some(11.0),
            old_family: None,
        };
        let preview = render_label_table(
            &core_font::font_preview_title(),
            &core_font::font_preview_rows(&plan),
        );
        for needle in ["Adwaita Sans", "11"] {
            assert!(preview.contains(needle), "preview must show `{needle}`");
        }
        let prompt = core_font::confirm_font_prompt(&plan);
        assert!(
            prompt.contains("Adwaita Sans 11"),
            "confirm names the GTK value: {prompt:?}"
        );
    }

    #[test]
    fn font_report_json_schema_pins_apply_fields() {
        use archel_core::hyprland::font::FontReport;
        let report = FontReport {
            family: "Adwaita Sans".to_string(),
            size: Some(11.0),
            gtk_value: "Adwaita Sans 11".to_string(),
            read_back_ok: true,
        };
        let value: serde_json::Value =
            serde_json::to_value(&report).expect("FontReport serializes");
        for field in ["family", "size", "gtk_value", "read_back_ok"] {
            assert!(value.get(field).is_some(), "report must carry `{field}`");
        }
        insta::assert_json_snapshot!("appearance_font", value);
    }

    #[test]
    fn font_status_json_schema_pins_read_fields() {
        use archel_core::hyprland::font::FontStatus;
        let status = FontStatus {
            gtk_family: "Adwaita Sans".to_string(),
            gtk_size: Some(11.0),
            hyprland_family: "Adwaita Sans".to_string(),
        };
        let value: serde_json::Value =
            serde_json::to_value(&status).expect("FontStatus serializes");
        for field in ["gtk_family", "gtk_size", "hyprland_family"] {
            assert!(value.get(field).is_some(), "status must carry `{field}`");
        }
        insta::assert_json_snapshot!("appearance_font_status", value);
    }

    #[test]
    fn font_confirm_abort_spawns_no_write() {
        // Same gate as blur/opacity: headless stdin refuses before the
        // closure, so neither the `hyprctl eval` nor the `gsettings set`
        // leg runs. Headless precondition asserted (WR-04).
        use std::io::IsTerminal;
        assert!(
            !std::io::stdin().is_terminal(),
            "abort-gate test requires headless stdin"
        );
        use archel_core::hyprland::font as core_font;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let writes = Arc::new(AtomicUsize::new(0));
        let probe = Arc::clone(&writes);
        let plan = core_font::FontPlan {
            family: "Adwaita Sans".to_string(),
            size: Some(11.0),
            old_family: None,
        };
        let err = run_font_plan_confirmed(&plan, false, move |_| {
            probe.fetch_add(1, Ordering::SeqCst);
            Ok(core_font::FontReport {
                family: "Adwaita Sans".to_string(),
                size: Some(11.0),
                gtk_value: "Adwaita Sans 11".to_string(),
                read_back_ok: true,
            })
        })
        .expect_err("headless confirm must refuse");
        assert!(
            matches!(err, ArchelError::NeedsTty),
            "abort path needs the typed refusal, never a write: {err:?}"
        );
        assert_eq!(
            writes.load(Ordering::SeqCst),
            0,
            "the abort path spawns no hyprctl write"
        );
    }

    #[test]
    fn wallpaper_preview_names_candidate_path() {
        use archel_core::hyprland::wallpaper as core_wallpaper;
        archel_core::i18n::set_language("fr");
        let plan = core_wallpaper::WallpaperPlan {
            path: std::path::PathBuf::from("/home/user/wall.png"),
        };
        let preview = render_label_table(
            &core_wallpaper::wallpaper_preview_title(),
            &core_wallpaper::wallpaper_preview_rows(&plan),
        );
        assert!(
            preview.contains("/home/user/wall.png"),
            "preview must show the candidate: {preview:?}"
        );
        let prompt = core_wallpaper::confirm_wallpaper_prompt(&plan);
        assert!(
            prompt.contains("/home/user/wall.png"),
            "confirm names the candidate: {prompt:?}"
        );
    }

    #[test]
    fn wallpaper_report_json_schema_pins_apply_fields() {
        use archel_core::hyprland::wallpaper::WallpaperReport;
        let report = WallpaperReport {
            path: "/home/user/wall.png".to_string(),
            restarted: true,
        };
        let value: serde_json::Value =
            serde_json::to_value(&report).expect("WallpaperReport serializes");
        for field in ["path", "restarted"] {
            assert!(value.get(field).is_some(), "report must carry `{field}`");
        }
        insta::assert_json_snapshot!("appearance_wallpaper", value);
    }

    #[test]
    fn wallpaper_status_json_schema_pins_read_fields() {
        use archel_core::hyprland::wallpaper::WallpaperStatus;
        let status = WallpaperStatus {
            path: Some("/home/user/wall.png".to_string()),
        };
        let value: serde_json::Value =
            serde_json::to_value(&status).expect("WallpaperStatus serializes");
        assert!(value.get("path").is_some(), "status must carry `path`");
        insta::assert_json_snapshot!("appearance_wallpaper_status", value);
    }

    #[test]
    fn wallpaper_confirm_abort_spawns_no_write() {
        // Same gate as font: headless stdin refuses before the closure,
        // so neither the slice write nor the daemon restart runs.
        // Headless precondition asserted (WR-04).
        use std::io::IsTerminal;
        assert!(
            !std::io::stdin().is_terminal(),
            "abort-gate test requires headless stdin"
        );
        use archel_core::hyprland::wallpaper as core_wallpaper;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let writes = Arc::new(AtomicUsize::new(0));
        let probe = Arc::clone(&writes);
        let plan = core_wallpaper::WallpaperPlan {
            path: std::path::PathBuf::from("/home/user/wall.png"),
        };
        let err = run_wallpaper_plan_confirmed(&plan, false, move |_| {
            probe.fetch_add(1, Ordering::SeqCst);
            Ok(core_wallpaper::WallpaperReport {
                path: "/home/user/wall.png".to_string(),
                restarted: true,
            })
        })
        .expect_err("headless confirm must refuse");
        assert!(
            matches!(err, ArchelError::NeedsTty),
            "abort path needs the typed refusal, never a write: {err:?}"
        );
        assert_eq!(
            writes.load(Ordering::SeqCst),
            0,
            "the abort path restarts no daemon"
        );
    }
}
