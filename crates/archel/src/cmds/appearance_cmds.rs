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
        assert!(
            preview.contains("désactivé"),
            "preview names the target state: {preview:?}"
        );
        let prompt = core_appearance::confirm_blur_prompt(true);
        assert!(prompt.contains("activé"), "confirm names the target: {prompt:?}");
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
        // the abort path structurally spawns no `hyprctl eval`.
        use std::io::IsTerminal;
        if std::io::stdin().is_terminal() {
            return;
        }
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
}
