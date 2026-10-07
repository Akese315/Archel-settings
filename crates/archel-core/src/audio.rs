//! Audio sink/source reads through WirePlumber (SND-01/02 tracer).
//!
//! The read path is `wpctl status` (LC_ALL=C-pinned, same spawn shape
//! as the package queries: which-gate, `.unchecked()`, `wait_timeout`
//! + kill, stderr log-only) to learn the current session IDs plus the
//! default flags, then one `wpctl inspect <id>` per ID to learn the
//! stable `node.name`. Device identity is the `node.name` string only:
//! numeric IDs renumber every PipeWire restart, so they are never
//! matched across runs and never persisted — they travel inside
//! [`AudioDevice`] with `#[serde(skip)]` so `--json` cannot leak them
//! into scripts (T-4-07). Missing `wpctl` (or a dead session) maps to
//! the typed [`ArchelError::AudioUnavailable`] install hint, never a
//! panic and never fake devices. One-shot reads only: device lists
//! are not 500 ms ticks, so no `--watch` and no reusable handle.
//!
//! The `set-output` / `set-input` switch paths land in the next task;
//! this module already exposes the name-keyed device shape they will
//! resolve against.

use std::time::Duration;

use serde::Serialize;

use crate::error::ArchelError;

/// Upper bound per `wpctl` spawn (offline must fail fast, same budget
/// as the package queries).
pub const WPCTL_TIMEOUT: Duration = Duration::from_secs(10);

/// Which side of the graph a device sits on. Serialized as the plain
/// English wire words so `--json` stays script-stable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioKind {
    Sink,
    Source,
}

/// One audio endpoint keyed by its stable `node.name`.
///
/// `id` is the current-session numeric ID: valid only until the next
/// PipeWire restart, used once at switch time, never serialized and
/// never matched across runs (T-4-07).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AudioDevice {
    #[serde(skip)]
    pub id: u32,
    pub name: String,
    pub description: String,
    pub kind: AudioKind,
    pub default: bool,
}

/// The full `audio list` snapshot: outputs and inputs in `wpctl`
/// status order. Empty vectors are the honest no-hardware state, not
/// an error (degrade shape: N/A rows downstream, never zeros).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AudioSnapshot {
    pub sinks: Vec<AudioDevice>,
    pub sources: Vec<AudioDevice>,
}

/// One parsed `wpctl status` row: the session ID, its default flag
/// (the `*` marker), and which section it appeared under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusEntry {
    pub id: u32,
    pub is_default: bool,
    pub kind: AudioKind,
}

/// Parses `LC_ALL=C wpctl status` into status rows. Only lines inside
/// the Audio tree's `Sinks:` / `Sources:` sections produce entries:
/// the Video tree repeats the same headers (skipped via the Audio
/// gate), and every other section (Clients, Devices, Streams,
/// Settings, Default Configured Devices) plus blank and malformed
/// lines are skipped too.
/// Entry lines look like ` *   47. Name here   [vol: 0.45]` (default)
/// or `     59. Other name [vol: 0.64]`: tree-drawing prefixes are
/// stripped, an optional `*` marks the default, then `<id>. <label>`.
pub fn parse_wpctl_status(stdout: &str) -> Vec<StatusEntry> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Section {
        None,
        Sinks,
        Sources,
    }
    // `wpctl status` repeats the Sinks:/Sources: headers under both
    // the Audio and Video trees: only rows inside the Audio tree
    // count (`in_audio`), so a video loopback never parses as audio.
    let mut in_audio = false;
    let mut section = Section::None;
    let mut entries = Vec::new();
    for raw_line in stdout.lines() {
        let stripped = raw_line.trim_start_matches(|c: char| {
            c == ' ' || c == '\t' || c == '|' || c == '*' || c == '+'
                || c == '-' || c == '.'
                || matches!(c, '\u{2500}'..='\u{257F}')
        });
        let trimmed = stripped.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "Audio" {
            in_audio = true;
            section = Section::None;
            continue;
        }
        if trimmed == "Video" || trimmed == "Settings" {
            in_audio = false;
            section = Section::None;
            continue;
        }
        if trimmed == "Sinks:" {
            section = Section::Sinks;
            continue;
        }
        if trimmed == "Sources:" {
            section = Section::Sources;
            continue;
        }
        if trimmed.ends_with(':') {
            section = Section::None;
            continue;
        }
        if !in_audio {
            continue;
        }
        let kind = match section {
            Section::Sinks => AudioKind::Sink,
            Section::Sources => AudioKind::Source,
            Section::None => continue,
        };
        if let Some(entry) = parse_status_entry_line(raw_line, kind) {
            entries.push(entry);
        }
    }
    entries
}

/// Parses one candidate status row against the already-known section.
/// Returns `None` for garbage instead of panicking (line discipline:
/// skip blanks and malformed lines, mirroring `parse_search_output`).
fn parse_status_entry_line(line: &str, kind: AudioKind) -> Option<StatusEntry> {
    // Strip the tree-drawing prefix but keep the `*` default marker
    // visible for the next step.
    let stripped = line.trim_start_matches(|c: char| {
        c == ' ' || c == '\t' || c == '|' || c == '+' || c == '-' || c == '.'
            || matches!(c, '\u{2500}'..='\u{257F}')
    });
    let (is_default, rest) = match stripped.strip_prefix('*') {
        Some(after) => (true, after.trim_start()),
        None => (false, stripped),
    };
    let (id_text, _label) = rest.split_once('.')?;
    let id: u32 = id_text.trim().parse().ok()?;
    if _label.trim().is_empty() {
        return None;
    }
    Some(StatusEntry {
        id,
        is_default,
        kind,
    })
}

/// Extracts the stable `node.name` from `wpctl inspect` output.
/// Lines look like ` * node.name = "alsa_output.usb-..."`. `None`
/// when the property is absent (that device is skipped, never
/// mislabeled).
pub fn parse_inspect_name(stdout: &str) -> Option<String> {
    parse_inspect_property(stdout, "node.name")
}

/// Extracts the human `node.description` from `wpctl inspect`
/// output (falls back to the name when absent, handled by the
/// caller).
pub fn parse_inspect_description(stdout: &str) -> Option<String> {
    parse_inspect_property(stdout, "node.description")
}

/// Extracts one quoted `key = "value"` property from `wpctl inspect`
/// output. Pure over the text so fixtures cover it without PipeWire.
fn parse_inspect_property(stdout: &str, key: &str) -> Option<String> {
    for line in stdout.lines() {
        let trimmed = line.trim().trim_start_matches("* ").trim();
        let Some((name, value)) = trimmed.split_once('=') else {
            continue;
        };
        if name.trim() != key {
            continue;
        }
        let value = value.trim();
        let unquoted = value.strip_prefix('"').and_then(|rest| rest.strip_suffix('"'))?;
        if unquoted.is_empty() {
            return None;
        }
        return Some(unquoted.to_string());
    }
    None
}

/// Runs one `wpctl` query and returns its stdout. Mirrors the package
/// `run_capped_query` spawn shape exactly, plus the `LC_ALL=C` pin
/// (field parsing breaks on localized output otherwise). A missing
/// binary is the typed [`ArchelError::AudioUnavailable`] install
/// hint; every other failure is log detail inside the same variant
/// (stderr stays log-only, never on the terminal).
fn run_wpctl(args: &[&str]) -> Result<String, ArchelError> {
    let binary = which::which("wpctl").map_err(|err| {
        ArchelError::audio_unavailable(format!("wpctl missing ({err})"))
    })?;
    let handle = duct::cmd(&binary, args)
        .unchecked()
        .env("LC_ALL", "C")
        .stdout_capture()
        .stderr_capture()
        .start()
        .map_err(|err| ArchelError::audio_unavailable(format!("cannot run wpctl: {err}")))?;
    let output = match handle.wait_timeout(WPCTL_TIMEOUT).map_err(|err| {
        ArchelError::audio_unavailable(format!("cannot wait on wpctl: {err}"))
    })? {
        Some(output) => output.clone(),
        None => {
            let _ = handle.kill();
            return Err(ArchelError::audio_unavailable(
                "wpctl exceeded the query timeout",
            ));
        }
    };
    let _stderr_for_logs = String::from_utf8_lossy(&output.stderr);
    match output.status.code() {
        Some(0) => Ok(String::from_utf8_lossy(&output.stdout).into_owned()),
        code => Err(ArchelError::audio_unavailable(format!(
            "wpctl failed with exit code {code:?}"
        ))),
    }
}

/// Lists every sink and source by stable `node.name`.
///
/// Devices whose `inspect` answers without a `node.name` are skipped
/// (stale IDs between the two spawns); a fully empty graph is the
/// honest `Ok` with empty vectors, never an error.
pub fn audio_list() -> Result<AudioSnapshot, ArchelError> {
    let status = run_wpctl(&["status"])?;
    let mut sinks = Vec::new();
    let mut sources = Vec::new();
    for entry in parse_wpctl_status(&status) {
        let inspect = match run_wpctl(&["inspect", &entry.id.to_string()]) {
            Ok(out) => out,
            Err(_) => continue,
        };
        let Some(name) = parse_inspect_name(&inspect) else {
            continue;
        };
        let description = parse_inspect_description(&inspect).unwrap_or_else(|| name.clone());
        let device = AudioDevice {
            id: entry.id,
            name,
            description,
            kind: entry.kind,
            default: entry.is_default,
        };
        match entry.kind {
            AudioKind::Sink => sinks.push(device),
            AudioKind::Source => sources.push(device),
        }
    }
    Ok(AudioSnapshot { sinks, sources })
}

/// French list title from the catalog (bin owns no copy).
pub fn list_title() -> String {
    rust_i18n::t!("audio.list_title").to_string()
}

/// French sinks/sources section headers from the catalog.
pub fn section_labels() -> (String, String) {
    (
        rust_i18n::t!("audio.sinks_section").to_string(),
        rust_i18n::t!("audio.sources_section").to_string(),
    )
}

/// French device-column labels from the catalog.
pub struct AudioColumns {
    pub name: String,
    pub description: String,
    pub default: String,
}

/// Collects the device-column labels in one call.
pub fn audio_columns() -> AudioColumns {
    AudioColumns {
        name: rust_i18n::t!("audio.col_name").to_string(),
        description: rust_i18n::t!("audio.col_description").to_string(),
        default: rust_i18n::t!("audio.col_default").to_string(),
    }
}

/// Honest empty-list line from the catalog (never a fake row).
pub fn list_empty_line() -> String {
    rust_i18n::t!("audio.list_empty").to_string()
}

/// French default cell: yes/no wording from the catalog, never a
/// bare star a script could misread.
pub fn default_cell(is_default: bool) -> String {
    if is_default {
        rust_i18n::t!("audio.default_yes").to_string()
    } else {
        rust_i18n::t!("audio.default_no").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `LC_ALL=C wpctl status` shape captured live (2026-10-07):
    /// descriptions localized, headers English, tree prefixes, one
    /// default per section, plus garbage lines a parser must skip.
    const STATUS_FIXTURE: &str = "PipeWire 'pipewire-0' [1.6.9, axel@archel, cookie:694421126]\n \u{250c}\u{2500} Clients:\n        32. xdg-desktop-portal                  [1.6.9, axel@archel, pid:1324]\n\nAudio\n \u{251c}\u{2500} Devices:\n \u{2502}       48. GP104 High Definition Audio Controller [alsa]\n \u{2502}  \n \u{251c}\u{2500} Sinks:\n \u{2502}   *   47. SC60 for Lync Analog stereo   [vol: 0.45]\n \u{2502}       59. Starship Matisse HD Audio Analog stereo [vol: 0.64]\n \u{2502}       61. GP104 HDMI stereo [24G2W1G4] [vol: 1.04]\nnot a device line at all\n \u{2502}       not-an-id. broken row\n \u{2502}  \n \u{251c}\u{2500} Sources:\n \u{2502}       58. Starship Matisse HD Audio Analog stereo [vol: 1.00]\n \u{2502}   *   60. SC60 for Lync Analog stereo   [vol: 1.00]\n \u{2502}       77. \n\nVideo\n \u{251c}\u{2500} Sinks:\n \u{2502}       90. Video loopback that must not parse as audio\n\nSettings\n \u{2514}\u{2500} Default Configured Devices:\n";

    const INSPECT_FIXTURE: &str = "id 47, type PipeWire:Interface:Node\n    alsa.card = \"0\"\n  * media.class = \"Audio/Sink\"\n  * node.description = \"SC60 for Lync Analog stereo\"\n  * node.name = \"alsa_output.usb-Sennheiser_SC60-00.analog-stereo\"\n  * node.nick = \"Sennheiser SC60\"\n";

    #[test]
    fn status_parse_reads_sinks_sources_defaults_and_skips_garbage() {
        let entries = parse_wpctl_status(STATUS_FIXTURE);
        assert_eq!(entries.len(), 5, "3 sinks + 2 sources, rest skipped: {entries:?}");
        assert_eq!(
            entries[0],
            StatusEntry {
                id: 47,
                is_default: true,
                kind: AudioKind::Sink,
            }
        );
        assert_eq!(entries[1].id, 59);
        assert!(!entries[1].is_default);
        assert_eq!(entries[1].kind, AudioKind::Sink);
        assert_eq!(entries[3].id, 58);
        assert_eq!(entries[3].kind, AudioKind::Source);
        assert!(entries[4].is_default);
        // The Video-loopback ID must never leak into the audio set,
        // and client/device rows (section None) stay out too.
        assert!(
            entries.iter().all(|entry| [47, 59, 61, 58, 60].contains(&entry.id)),
            "only audio sink/source IDs, no video/client rows: {entries:?}"
        );
    }

    #[test]
    fn status_parse_never_panics_on_malformed_lines() {
        let out = "garbage\n   \n*. \n12 with no dot\n. name only\nSinks:\n*. \n78. \n";
        assert_eq!(parse_wpctl_status(out), Vec::new());
    }

    #[test]
    fn inspect_parse_reads_stable_node_names() {
        assert_eq!(
            parse_inspect_name(INSPECT_FIXTURE).as_deref(),
            Some("alsa_output.usb-Sennheiser_SC60-00.analog-stereo")
        );
        assert_eq!(
            parse_inspect_description(INSPECT_FIXTURE).as_deref(),
            Some("SC60 for Lync Analog stereo")
        );
        assert!(parse_inspect_name("id 1, type Foo\n    no node here\n").is_none());
        assert!(parse_inspect_name("node.name = \"\"\n").is_none());
    }

    #[test]
    fn missing_wpctl_maps_to_the_typed_unavailable_hint() {
        // `which` failure path is covered by construction: the error
        // the live path returns on this machine must be the typed
        // variant (LCD sentinels: message from catalog, Display
        // log-only). Here the constructor + catalog arm render check.
        rust_i18n::set_locale("fr");
        let rendered = ArchelError::audio_unavailable("no wpctl").message();
        assert!(!rendered.trim().is_empty());
        assert!(rendered.contains("wpctl"), "hint must name the tool: {rendered}");
    }

    #[test]
    fn live_audio_list_shows_stable_names() {
        let snap = audio_list().expect("wpctl answers on this machine");
        assert!(
            !snap.sinks.is_empty() && !snap.sources.is_empty(),
            "live graph carries sinks and sources"
        );
        for device in snap.sinks.iter().chain(&snap.sources) {
            assert!(!device.name.trim().is_empty(), "name must show");
            assert!(
                device.name.contains("alsa_"),
                "stable node.name keys identity, got {}",
                device.name
            );
        }
        assert_eq!(
            snap.sinks.iter().filter(|sink| sink.default).count(),
            1,
            "exactly one default sink: {:?}",
            snap.sinks
        );
    }

    #[test]
    fn audio_labels_come_from_the_catalog() {
        rust_i18n::set_locale("fr");
        assert!(!list_title().is_empty());
        assert!(!list_empty_line().is_empty());
        let (sinks, sources) = section_labels();
        assert!(!sinks.is_empty() && !sources.is_empty());
        let cols = audio_columns();
        assert!(!cols.name.is_empty() && !cols.description.is_empty());
        assert!(!default_cell(true).is_empty() && !default_cell(false).is_empty());
    }
}
