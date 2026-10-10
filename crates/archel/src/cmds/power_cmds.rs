//! `archel power status|set` renderers: French tables (default) or
//! JSON (`--json`).
//!
//! Bin-calls-core shape: every French string comes from
//! `archel-core::power` (title, row labels, note, confirm prompt,
//! ok line) — this module only lays out rows and frames the JSON
//! payloads. `status` always shows the backend tag so provenance
//! survives into scripts; `set` is confirm-gated in core
//! (`confirm_action`: preview naming the target profile, `[o/N]`,
//! `NeedsTty` when piped). This `power` group switches profiles;
//! `sys power` stays the read-only watts readout, with distinct
//! catalog titles so the two never collide.

use archel_core::power::PowerStatus;
use archel_core::ArchelError;

/// Runs `archel power status`: active profile plus backend tag.
// gui: power.tick
pub async fn run_status(json: bool) -> Result<(), ArchelError> {
    let status = archel_core::power::power_status().await?;
    if json {
        println!("{}", encode_json(&status, "power status")?);
    } else {
        print!("{}", render_status_table(&status));
    }
    Ok(())
}

/// Runs `archel power set <profile>`: confirm-gated switch on
/// whichever backend serves this machine. `--json` emits the
/// single-object mutation report shape.
// gui: power.set_profile
pub async fn run_set(profile_arg: &str, json: bool) -> Result<(), ArchelError> {
    let profile = archel_core::power::parse_profile_arg(profile_arg).ok_or_else(|| {
        ArchelError::Internal(format!("clap allowed an unknown power profile: '{profile_arg}'"))
    })?;
    let status = archel_core::power::power_set(profile).await?;
    let ok = archel_core::power::set_ok_line(status.profile);
    if json {
        println!("{}", mutation_report_value(profile_arg, &status, &ok));
    } else {
        println!("{ok}");
    }
    Ok(())
}

fn encode_json(payload: &impl serde::Serialize, what: &str) -> Result<String, ArchelError> {
    serde_json::to_string(payload)
        .map_err(|err| ArchelError::Internal(format!("cannot encode {what} JSON: {err}")))
}

/// Builds the single-object mutation report: the CLI profile word,
/// the resulting status (profile plus backend plus note), and the
/// catalog ok line as the transcript. One object, always parseable.
fn mutation_report_value(
    profile_arg: &str,
    status: &PowerStatus,
    ok_line: &str,
) -> serde_json::Value {
    serde_json::json!({
        "action": "set",
        "profile": profile_arg,
        "ok": true,
        "status": status,
        "transcript": [ok_line],
        "note": serde_json::Value::Null,
    })
}

/// Right-pads a label on char count (mirrors the `audio_cmds`
/// helper — byte widths would misalign accented French labels).
fn pad_right(label: &str, width: usize) -> String {
    let mut out = label.to_string();
    while out.chars().count() < width {
        out.push(' ');
    }
    out
}

/// French table for one power status: profile, backend tag, and the
/// collapse note when the backend carries one.
fn render_status_table(status: &PowerStatus) -> String {
    let labels = archel_core::power::power_labels();
    let profile_name = archel_core::power::profile_arg_name(status.profile);
    let backend = archel_core::power::backend_tag(status.backend);
    let mut rows: Vec<(String, String)> = vec![
        (labels.profile, profile_name.to_string()),
        (labels.backend, backend.to_string()),
    ];
    if let Some(note) = &status.note {
        rows.push((labels.note, note.clone()));
    }
    let width = rows
        .iter()
        .map(|(label, _)| label.chars().count())
        .max()
        .unwrap_or(0);
    let mut out = format!("{}\n", archel_core::power::status_title());
    for (label, value) in &rows {
        out.push_str(&format!("{} : {value}\n", pad_right(label, width)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use archel_core::power::{PowerBackend, PowerProfile};

    fn cpufreq_status() -> PowerStatus {
        PowerStatus {
            profile: PowerProfile::PowerSaver,
            backend: PowerBackend::Cpufreq,
            note: Some("collapse note".to_string()),
        }
    }

    #[test]
    fn status_table_shows_profile_backend_and_note() {
        archel_core::i18n::set_language("fr");
        let table = render_status_table(&cpufreq_status());
        assert!(table.contains("power-saver"), "profile: {table:?}");
        assert!(table.contains("cpufreq"), "backend tag: {table:?}");
        assert!(table.contains("collapse note"), "note: {table:?}");
    }

    #[test]
    fn status_table_without_note_shows_two_rows() {
        archel_core::i18n::set_language("fr");
        let table = render_status_table(&PowerStatus {
            profile: PowerProfile::Balanced,
            backend: PowerBackend::Ppd,
            note: None,
        });
        assert!(table.contains("balanced"), "profile: {table:?}");
        assert!(table.contains("power-profiles-daemon"), "backend tag: {table:?}");
        assert_eq!(table.lines().count(), 3, "title plus two rows: {table:?}");
    }

    #[test]
    fn set_report_carries_profile_status_and_transcript() {
        archel_core::i18n::set_language("fr");
        let report = mutation_report_value("balanced", &cpufreq_status(), "ok line");
        assert_eq!(report["action"], "set");
        assert_eq!(report["profile"], "balanced");
        assert_eq!(report["ok"], true);
        assert_eq!(report["status"]["backend"], "cpufreq");
        assert_eq!(report["transcript"][0], "ok line");
        assert!(report["note"].is_null());
    }

    // insta pins: the --json schemas in snake_case English keys. A
    // silent field reshape fails here before reaching scripts.
    #[test]
    fn schema_power_status_is_pinned() {
        insta::assert_json_snapshot!("power_status", cpufreq_status());
    }

    #[test]
    fn schema_power_set_is_pinned() {
        archel_core::i18n::set_language("fr");
        let report = mutation_report_value("balanced", &cpufreq_status(), "ok line");
        insta::assert_json_snapshot!("power_set", report);
    }
}
