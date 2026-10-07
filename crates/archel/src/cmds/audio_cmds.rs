//! `archel audio list` renderer: French tables (default) or JSON
//! (`--json`).
//!
//! Bin-calls-core shape: every French string comes from
//! `archel-core::audio` (title, section headers, column labels,
//! default cells, honest-empty line) — this module only lays out
//! columns (`std::fmt` widths, no table crate) and frames the JSON
//! payload. A dead PipeWire session maps to the typed
//! `AudioUnavailable` catalog error, never a panic, never fake
//! devices. `set-output` / `set-input` land in the next task.

use archel_core::audio::{AudioDevice, AudioSnapshot};
use archel_core::ArchelError;

/// Runs `archel audio list`: sinks then sources by stable node.name.
pub fn run_list(json: bool) -> Result<(), ArchelError> {
    let snap = archel_core::audio::audio_list()?;
    if json {
        println!("{}", encode_json(&snap, "audio list")?);
    } else {
        print!("{}", render_list_table(&snap));
    }
    Ok(())
}

fn encode_json(payload: &impl serde::Serialize, what: &str) -> Result<String, ArchelError> {
    serde_json::to_string(payload)
        .map_err(|err| ArchelError::Internal(format!("cannot encode {what} JSON: {err}")))
}

/// Right-pads a label on char count (mirrors the `bt_cmds` helper —
/// byte widths would misalign accented French labels).
fn pad_right(label: &str, width: usize) -> String {
    let mut out = label.to_string();
    while out.chars().count() < width {
        out.push(' ');
    }
    out
}

/// French table for the whole snapshot: one section per direction.
/// Empty input renders the honest-empty line, never a fake row.
fn render_list_table(snap: &AudioSnapshot) -> String {
    if snap.sinks.is_empty() && snap.sources.is_empty() {
        return format!("{}\n", archel_core::audio::list_empty_line());
    }
    let mut out = format!("{}\n", archel_core::audio::list_title());
    let (sinks_label, sources_label) = archel_core::audio::section_labels();
    out.push_str(&render_section(&sinks_label, &snap.sinks));
    out.push_str(&render_section(&sources_label, &snap.sources));
    out
}

/// French table for one direction. An empty direction prints its
/// header alone (the other direction still shows real devices).
fn render_section(title: &str, devices: &[AudioDevice]) -> String {
    let cols = archel_core::audio::audio_columns();
    let name_width = devices
        .iter()
        .map(|row| row.name.chars().count())
        .chain(std::iter::once(cols.name.chars().count()))
        .max()
        .unwrap_or(0);
    let desc_width = devices
        .iter()
        .map(|row| row.description.chars().count())
        .chain(std::iter::once(cols.description.chars().count()))
        .max()
        .unwrap_or(0);
    let mut out = format!("{title}\n");
    out.push_str(&format!(
        "{}  {}  {}\n",
        pad_right(&cols.name, name_width),
        pad_right(&cols.description, desc_width),
        cols.default
    ));
    for row in devices {
        out.push_str(&format!(
            "{}  {}  {}\n",
            pad_right(&row.name, name_width),
            pad_right(&row.description, desc_width),
            archel_core::audio::default_cell(row.default)
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use archel_core::audio::AudioKind;

    fn sample_sink() -> AudioDevice {
        AudioDevice {
            id: 47,
            name: "alsa_output.usb-Sennheiser_SC60-00.analog-stereo".to_string(),
            description: "SC60 for Lync Analog stereo".to_string(),
            kind: AudioKind::Sink,
            default: true,
        }
    }

    fn sample_source() -> AudioDevice {
        AudioDevice {
            id: 60,
            name: "alsa_input.usb-Sennheiser_SC60-00.analog-stereo".to_string(),
            description: "SC60 for Lync Analog stereo".to_string(),
            kind: AudioKind::Source,
            default: true,
        }
    }

    fn sample_snapshot() -> AudioSnapshot {
        AudioSnapshot {
            sinks: vec![sample_sink()],
            sources: vec![sample_source()],
        }
    }

    #[test]
    fn list_table_shows_stable_names_with_sections() {
        archel_core::i18n::set_language("fr");
        let table = render_list_table(&sample_snapshot());
        assert!(
            table.contains("alsa_output.usb-Sennheiser_SC60-00.analog-stereo"),
            "table must show the stable sink name: {table:?}"
        );
        assert!(
            table.contains("alsa_input.usb-Sennheiser_SC60-00.analog-stereo"),
            "table must show the stable source name: {table:?}"
        );
        assert!(!table.contains(" 47"), "numeric IDs never render: {table:?}");
    }

    #[test]
    fn list_empty_table_is_honest_without_fake_rows() {
        archel_core::i18n::set_language("fr");
        let table = render_list_table(&AudioSnapshot {
            sinks: Vec::new(),
            sources: Vec::new(),
        });
        assert!(!table.contains("alsa_"), "empty table must hold no device: {table:?}");
        assert!(
            table.contains(&archel_core::audio::list_empty_line()),
            "empty table must print the honest line"
        );
    }

    #[test]
    fn list_json_pins_snapshot_fields_without_session_ids() {
        let value = serde_json::to_value(sample_snapshot()).expect("serializes");
        for section in ["sinks", "sources"] {
            let row = &value[section][0];
            for field in ["name", "description", "kind", "default"] {
                assert!(
                    row.get(field).is_some(),
                    "audio list --json must pin `{section}[0].{field}`"
                );
            }
            assert!(
                row.get("id").is_none(),
                "session IDs must never leak into --json (T-4-07)"
            );
        }
        assert_eq!(value["sinks"][0]["kind"], "sink");
        assert_eq!(value["sources"][0]["kind"], "source");
    }

    // insta pin: the --json schema in snake_case English keys
    // mirroring the struct fields. A silent field reshape fails here
    // before reaching scripts.
    #[test]
    fn schema_audio_list_is_pinned() {
        insta::assert_json_snapshot!("audio_list", sample_snapshot());
    }
}
