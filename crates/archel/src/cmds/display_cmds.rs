//! `archel display monitors|set` renderers: French tables (default)
//! or JSON (`--json`).
//!
//! Bin-calls-core shape: every French string comes from
//! `archel-core::hyprland::monitors` (titles, labels, state wording,
//! confirm prompts, ok lines) — this module only lays out columns
//! (`std::fmt` widths, no table crate) and frames the JSON payloads.
//! Mutations are confirm-gated in core (`confirm_action` pre-gate plus
//! the 15 s timed re-confirm with automatic rollback, D-04); the CLI
//! validates, plans, and projects the layout read-only first, so typos
//! fail fast with no TTY need.
//!
//! Live proof stays single-monitor (D-05): only the current values of
//! one output are ever re-applied as a no-op during automated runs;
//! multi-monitor coverage is fixtures plus argv-shape only.

use archel_core::ArchelError;

/// Runs `archel display monitors`: live inventory with current mode,
/// scale, refresh, and position (DSP-01..04 reads). Read-only: no
/// confirm, no write.
pub fn run_monitors(json: bool) -> Result<(), ArchelError> {
    use archel_core::hyprland::monitors as core_monitors;
    let monitors = core_monitors::read_monitors()?;
    if json {
        let status: Vec<core_monitors::MonitorStatus> =
            monitors.iter().map(core_monitors::MonitorStatus::from).collect();
        println!("{}", encode_json(&status, "display monitors")?);
    } else {
        print!(
            "{}",
            render_label_table(
                &core_monitors::monitors_title(),
                &core_monitors::monitor_rows(&monitors)
            )
        );
    }
    Ok(())
}

/// Runs `archel display set --output <name> [--mode ...] [--scale ...]
/// [--position ...] [--disable]`: validate, plan, project the layout,
/// preview, then confirm-gated apply with 15 s keep-or-rollback
/// (DSP-01..04, D-04).
pub fn run_display_set(
    output: String,
    mode: Option<String>,
    scale: Option<String>,
    position: Option<String>,
    disable: bool,
    json: bool,
) -> Result<(), ArchelError> {
    use archel_core::hyprland::monitors as core_monitors;
    let monitors = core_monitors::read_monitors()?;
    let want = core_monitors::MonitorWant {
        output,
        mode,
        scale,
        position,
        disabled: disable,
    };
    run_display_set_with(&monitors, &want, json, |monitors, want| {
        core_monitors::set_monitor(monitors, want)
    })
}

/// Confirm-then-execute gate for monitor plans: read-only planning
/// (lookup, validation, layout projection) runs BEFORE any prompt, so
/// typos and overlaps fail fast with no TTY need; only a valid plan
/// reaches `apply` (production: `set_monitor`, which pre-confirms,
/// applies, then runs the 15 s timed re-confirm). The injectable
/// `apply` keeps the fail-fast paths unit-coverable headless.
fn run_display_set_with(
    monitors: &[archel_core::hyprland::monitors::Monitor],
    want: &archel_core::hyprland::monitors::MonitorWant,
    json: bool,
    apply: impl FnOnce(
        &[archel_core::hyprland::monitors::Monitor],
        &archel_core::hyprland::monitors::MonitorWant,
    ) -> Result<archel_core::hyprland::monitors::MonitorReport, ArchelError>,
) -> Result<(), ArchelError> {
    use archel_core::hyprland::monitors as core_monitors;
    let current = monitors
        .iter()
        .find(|monitor| monitor.name == want.output.trim())
        .ok_or_else(|| {
            ArchelError::desktop_invalid(format!(
                "rejected monitor output {:?} (no such connected monitor)",
                want.output
            ))
        })?;
    let plan = core_monitors::plan_monitor_change(current, want)?;
    let projected = core_monitors::project_layout(
        monitors,
        &plan.output,
        &plan.mode,
        plan.scale,
        &plan.position,
        plan.disabled,
    );
    core_monitors::validate_layout(&projected)?;
    if !json {
        print!(
            "{}",
            render_label_table(
                &core_monitors::monitor_preview_title(),
                &core_monitors::monitor_preview_rows(&plan)
            )
        );
    }
    let report = match apply(monitors, want) {
        Ok(report) => report,
        Err(err) => {
            // The rollback legs already ran inside core (D-04); name
            // the restored output so the refusal/timeout reads as a
            // completed rollback, not a crash.
            if matches!(
                err,
                ArchelError::ConfirmationAborted | ArchelError::ConfirmTimeout
            ) && !json
            {
                println!("{}", core_monitors::monitor_rollback_line(&want.output));
            }
            return Err(err);
        }
    };
    if !report.read_back_ok {
        return Err(ArchelError::hyprland_unavailable(
            "live read-back disagrees with the applied monitor values",
        ));
    }
    if json {
        println!("{}", encode_json(&report, "display set")?);
    } else {
        println!("{}", core_monitors::monitor_ok_line(&report));
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
    use archel_core::hyprland::monitors as core_monitors;

    /// Trimmed live-shaped fixture: DP-2 4K@60 at 0x0 scale 1.5 plus
    /// HDMI-A-2 1080p@144 at 2560x0 scale 1 (fixtures only, D-05 —
    /// this module never spawns).
    const TWO_MONITOR_FIXTURE: &str = r#"[
        {"id": 1, "name": "DP-2", "description": "BNQ BenQ PD2700U",
         "width": 3840, "height": 2160, "refreshRate": 59.997,
         "x": 0, "y": 0, "scale": 1.5, "disabled": false,
         "availableModes": ["3840x2160@60.00Hz", "3840x2160@59.94Hz", "1920x1080@60.00Hz"]},
        {"id": 0, "name": "HDMI-A-2", "description": "AOC 24G2W1G4",
         "width": 1920, "height": 1080, "refreshRate": 144.00101,
         "x": 2560, "y": 0, "scale": 1.0, "disabled": false,
         "availableModes": ["1920x1080@60.00Hz", "1920x1080@144.00Hz", "1280x720@59.94Hz"]}
    ]"#;

    fn fixture_monitors() -> Vec<core_monitors::Monitor> {
        core_monitors::parse_monitors_json(TWO_MONITOR_FIXTURE)
    }

    fn want(output: &str) -> core_monitors::MonitorWant {
        core_monitors::MonitorWant {
            output: output.to_string(),
            mode: None,
            scale: None,
            position: None,
            disabled: false,
        }
    }

    #[test]
    fn monitors_preview_names_both_outputs() {
        archel_core::i18n::set_language("fr");
        let monitors = fixture_monitors();
        let preview = render_label_table(
            &core_monitors::monitors_title(),
            &core_monitors::monitor_rows(&monitors),
        );
        for needle in ["DP-2", "HDMI-A-2", "3840x2160@60", "1920x1080@144"] {
            assert!(preview.contains(needle), "inventory must show `{needle}`");
        }
    }

    #[test]
    fn monitors_json_schema_pins_read_fields() {
        let monitors = fixture_monitors();
        let status: Vec<core_monitors::MonitorStatus> =
            monitors.iter().map(core_monitors::MonitorStatus::from).collect();
        let value: serde_json::Value =
            serde_json::to_value(&status).expect("MonitorStatus serializes");
        for field in ["name", "mode", "scale", "position", "disabled"] {
            assert!(
                value[0].get(field).is_some(),
                "monitor status must carry `{field}`"
            );
        }
        insta::assert_json_snapshot!("display_monitors", value);
    }

    #[test]
    fn set_preview_shows_old_arrow_new() {
        archel_core::i18n::set_language("fr");
        let monitors = fixture_monitors();
        let current = monitors
            .iter()
            .find(|monitor| monitor.name == "HDMI-A-2")
            .expect("fixture has HDMI-A-2");
        let plan = core_monitors::plan_monitor_change(
            current,
            &core_monitors::MonitorWant {
                output: "HDMI-A-2".to_string(),
                mode: Some("1280x720@59".to_string()),
                scale: None,
                position: None,
                disabled: false,
            },
        )
        .expect("plan builds");
        let preview = render_label_table(
            &core_monitors::monitor_preview_title(),
            &core_monitors::monitor_preview_rows(&plan),
        );
        assert!(preview.contains("1920x1080@144"), "preview shows the old mode");
        assert!(preview.contains("1280x720@59"), "preview shows the new mode");
        assert!(preview.contains("->"), "preview reads old-arrow-new");
    }

    #[test]
    fn set_report_json_schemas_pin_keep_and_rollback() {
        let kept = core_monitors::MonitorReport {
            output: "HDMI-A-2".to_string(),
            mode: "1280x720@59 scale 1 at 2560x0".to_string(),
            scale: 1.0,
            position: "2560x0".to_string(),
            kept: true,
            read_back_ok: true,
        };
        let value: serde_json::Value =
            serde_json::to_value(&kept).expect("MonitorReport serializes");
        for field in ["output", "mode", "scale", "position", "kept", "read_back_ok"] {
            assert!(value.get(field).is_some(), "report must carry `{field}`");
        }
        insta::assert_json_snapshot!("display_set_keep", value);
        let rolled_back = core_monitors::MonitorReport {
            output: "HDMI-A-2".to_string(),
            mode: "1920x1080@144 scale 1 at 2560x0".to_string(),
            scale: 1.0,
            position: "2560x0".to_string(),
            kept: false,
            read_back_ok: true,
        };
        insta::assert_json_snapshot!(
            "display_set_rollback",
            serde_json::to_value(&rolled_back).expect("rollback report serializes")
        );
    }

    #[test]
    fn set_unknown_output_fails_before_any_prompt_or_spawn() {
        // Validate-first (WR-02): the output lookup runs before any
        // prompt, so a typo fails with no TTY need and the apply leg
        // structurally never runs.
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let monitors = fixture_monitors();
        let calls = Arc::new(AtomicUsize::new(0));
        let probe = Arc::clone(&calls);
        let err = run_display_set_with(&monitors, &want("VGA-9"), false, move |_, _| {
            probe.fetch_add(1, Ordering::SeqCst);
            panic!("unknown output must never reach apply");
        })
        .expect_err("typo fails fast");
        assert!(
            matches!(err, ArchelError::DesktopInvalid { .. }),
            "typo is typed DesktopInvalid: {err:?}"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0, "apply never runs on a typo");
    }

    #[test]
    fn set_bad_mode_fails_before_any_prompt_or_spawn() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let monitors = fixture_monitors();
        let calls = Arc::new(AtomicUsize::new(0));
        let probe = Arc::clone(&calls);
        let mut bad = want("HDMI-A-2");
        bad.mode = Some("1920x1080@145".to_string());
        let err = run_display_set_with(&monitors, &bad, false, move |_, _| {
            probe.fetch_add(1, Ordering::SeqCst);
            panic!("unknown mode must never reach apply");
        })
        .expect_err("unknown mode fails fast");
        assert!(matches!(err, ArchelError::DesktopInvalid { .. }), "{err:?}");
        assert_eq!(calls.load(Ordering::SeqCst), 0, "apply never runs on a bad mode");
    }

    #[test]
    fn set_overlapping_layout_fails_before_any_prompt_or_spawn() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let monitors = fixture_monitors();
        let calls = Arc::new(AtomicUsize::new(0));
        let probe = Arc::clone(&calls);
        let mut overlap = want("HDMI-A-2");
        overlap.position = Some("100x0".to_string());
        let err = run_display_set_with(&monitors, &overlap, false, move |_, _| {
            probe.fetch_add(1, Ordering::SeqCst);
            panic!("overlapping layout must never reach apply");
        })
        .expect_err("overlap fails fast");
        assert!(matches!(err, ArchelError::DesktopInvalid { .. }), "{err:?}");
        assert_eq!(calls.load(Ordering::SeqCst), 0, "apply never runs on overlap");
    }

    #[test]
    fn set_headless_refusal_propagates_typed_needs_tty() {
        // The CLI itself never spawns (lookup, validation, projection,
        // and preview are pure over the already-read inventory); the
        // mutation gate lives in the delegated apply, which refuses
        // headless before mutating — mirroring set_monitor's
        // confirm-first order. The headless precondition is asserted,
        // never a silent return (WR-04).
        use std::io::IsTerminal;
        assert!(
            !std::io::stdin().is_terminal(),
            "abort-gate test requires headless stdin"
        );
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let monitors = fixture_monitors();
        let calls = Arc::new(AtomicUsize::new(0));
        let probe = Arc::clone(&calls);
        let err = run_display_set_with(&monitors, &want("HDMI-A-2"), false, move |_, _| {
            probe.fetch_add(1, Ordering::SeqCst);
            // Same order as set_monitor: pre-confirm first, any
            // mutation (eval, snippet) only after a yes.
            archel_core::packages::confirm_action("display abort probe [o/N]")?;
            unreachable!("headless confirm refuses before any mutation");
        })
        .expect_err("headless confirm must refuse");
        assert!(
            matches!(err, ArchelError::NeedsTty),
            "abort path needs the typed refusal, never a write: {err:?}"
        );
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "the delegated apply runs its gate and refuses before mutating"
        );
    }

    #[test]
    fn set_refusal_reports_the_rollback_line() {
        // On reject/timeout the rollback legs already ran inside core;
        // the CLI names the restored output (FR) instead of failing
        // silently. The headless precondition is asserted (WR-04).
        use std::io::IsTerminal;
        assert!(
            !std::io::stdin().is_terminal(),
            "rollback-report test requires headless stdin"
        );
        archel_core::i18n::set_language("fr");
        assert!(!core_monitors::monitor_rollback_line("HDMI-A-2").is_empty());
        assert!(core_monitors::monitor_rollback_line("HDMI-A-2").contains("HDMI-A-2"));
    }

    #[test]
    fn set_path_touches_nothing_outside_app_owned_files() {
        // D-08 write-scope spy for the display persist path: a user tree
        // (entry plus an unrelated module) plus a sentinel outside the
        // base; after the snippet upsert plus require work, only
        // app-owned paths may differ.
        let base = std::env::temp_dir().join(format!(
            "archel-display-scope-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("modules")).expect("modules creates");
        let outside = base.with_extension("outside-sentinel");
        std::fs::write(&outside, "sentinel").expect("sentinel seeds");
        let user_module = base.join("modules").join("keybinds.lua");
        std::fs::write(&user_module, "-- user binds\n").expect("user module seeds");
        std::fs::write(base.join("hyprland.lua"), "-- entry\n").expect("entry seeds");
        let user_before = std::fs::read(&user_module).expect("user module reads");
        let monitors = fixture_monitors();
        let current = monitors
            .iter()
            .find(|monitor| monitor.name == "HDMI-A-2")
            .expect("fixture has HDMI-A-2");
        let plan = core_monitors::plan_monitor_change(current, &want("HDMI-A-2"))
            .expect("no-op plan builds");
        core_monitors::persist_monitor_plan(&base, &plan).expect("persist writes");
        assert_eq!(
            std::fs::read(&user_module).expect("user module re-reads"),
            user_before,
            "sibling user modules are byte-for-byte unchanged"
        );
        assert_eq!(
            std::fs::read_to_string(&outside).expect("sentinel reads"),
            "sentinel",
            "nothing outside the hypr dir may be touched"
        );
        let _ = std::fs::remove_dir_all(&base);
        let _ = std::fs::remove_file(&outside);
    }
}
