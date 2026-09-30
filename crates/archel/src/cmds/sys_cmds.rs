//! `archel sys cpu|ram|disk` renderers: French tables (default) or one
//! JSON object per tick (`--json`), with `--watch` re-render.
//!
//! Bin-calls-core shape: every number and every French string comes from
//! `archel-core::system` (snapshots, conversions, catalog labels); this
//! module only lays out columns (`std::fmt` widths, no table crate),
//! frames NDJSON ticks, and owns the 500 ms watch loop. One [`Monitor`]
//! is created outside the loop and reused across ticks so ticks 2+ always
//! carry valid diff-based CPU numbers (Pitfall 1).
//!
//! Watch exits on Ctrl+C with default SIGINT semantics (exit 130 like
//! standard tools); no cleanup is needed — the loop holds no raw mode or
//! alternate screen. `--json` ticks print exactly one complete object
//! with no clearing and no stray lines, so the stream stays parseable
//! (diagnostics travel via `tracing` on stderr only, never here).

use std::time::Duration;

use archel_core::system::{
    CpuSnapshot, DiskSnapshot, RamSnapshot, bytes_to_gib, unit_labels,
};
use archel_core::ArchelError;

/// Watch re-render cadence (D-02).
const WATCH_INTERVAL: Duration = Duration::from_millis(500);

/// Runs `archel sys cpu`: per-core usage plus clock (SYS-03/04).
pub fn run_cpu(watch: bool, json: bool) -> Result<(), ArchelError> {
    let mut monitor = archel_core::system::Monitor::new();
    loop {
        let snap = monitor.cpu();
        let payload = serde_json::to_string(&snap).map_err(|err| {
            ArchelError::Internal(format!("cannot encode sys cpu JSON: {err}"))
        })?;
        emit_tick(json, watch, &payload, &render_cpu_table(&snap));
        if !watch {
            return Ok(());
        }
        std::thread::sleep(WATCH_INTERVAL);
    }
}

/// Runs `archel sys ram`: used/total memory (SYS-01).
pub fn run_ram(watch: bool, json: bool) -> Result<(), ArchelError> {
    let mut monitor = archel_core::system::Monitor::new();
    loop {
        let snap = monitor.ram();
        let payload = serde_json::to_string(&snap).map_err(|err| {
            ArchelError::Internal(format!("cannot encode sys ram JSON: {err}"))
        })?;
        emit_tick(json, watch, &payload, &render_ram_table(&snap));
        if !watch {
            return Ok(());
        }
        std::thread::sleep(WATCH_INTERVAL);
    }
}

/// Runs `archel sys disk`: system-partition used/total/free (SYS-02).
pub fn run_disk(watch: bool, json: bool) -> Result<(), ArchelError> {
    let mut monitor = archel_core::system::Monitor::new();
    loop {
        let snap = monitor.disk()?;
        let payload = serde_json::to_string(&snap).map_err(|err| {
            ArchelError::Internal(format!("cannot encode sys disk JSON: {err}"))
        })?;
        emit_tick(json, watch, &payload, &render_disk_table(&snap));
        if !watch {
            return Ok(());
        }
        std::thread::sleep(WATCH_INTERVAL);
    }
}

/// Prints one tick: NDJSON line under `--json` (no clearing), otherwise
/// the French table (full-reset clear in watch mode so the view
/// re-renders instead of scrolling).
fn emit_tick(json: bool, watch: bool, payload: &str, table: &str) {
    if json {
        println!("{payload}");
    } else {
        if watch {
            print!("\x1Bc");
        }
        print!("{table}");
    }
}

/// Builds the per-core CPU table. Labels and units come from the
/// catalog; column padding counts characters (not bytes) so accented
/// headers stay aligned.
pub fn render_cpu_table(snap: &CpuSnapshot) -> String {
    let labels = archel_core::system::cpu_table_labels();
    let units = unit_labels();
    let id_width = snap
        .cores
        .iter()
        .map(|core| core.id.to_string().len())
        .max()
        .unwrap_or(1)
        .max(labels.core.chars().count());
    let usage_width = snap
        .cores
        .iter()
        .map(|core| format_usage(core.usage_pct, &units.percent).len())
        .max()
        .unwrap_or(0)
        .max(labels.usage.chars().count());
    let mut out = format!("{}\n", labels.title);
    out.push_str(&format!(
        "{} | {} | {}\n",
        pad_right(&labels.core, id_width),
        pad_right(&labels.usage, usage_width),
        labels.frequency
    ));
    for core in &snap.cores {
        out.push_str(&format!(
            "{} | {} | {} {}\n",
            pad_right(&core.id.to_string(), id_width),
            pad_right(&format_usage(core.usage_pct, &units.percent), usage_width),
            core.freq_mhz,
            units.mhz
        ));
    }
    out
}

/// Builds the RAM label/value table with used GB prominent (first row).
pub fn render_ram_table(snap: &RamSnapshot) -> String {
    let labels = archel_core::system::ram_table_labels();
    let rows: [(String, String); 2] = [
        (labels.used, format_gib(snap.used_bytes)),
        (labels.total, format_gib(snap.total_bytes)),
    ];
    render_label_table(&labels.title, &rows)
}

/// Builds the system-partition disk table: used, total, free with used
/// GB prominent (first row).
pub fn render_disk_table(snap: &DiskSnapshot) -> String {
    let labels = archel_core::system::disk_table_labels();
    let rows: [(String, String); 3] = [
        (labels.used, format_gib(snap.used_bytes)),
        (labels.total, format_gib(snap.total_bytes)),
        (labels.free, format_gib(snap.free_bytes)),
    ];
    render_label_table(&labels.title, &rows)
}

/// Landing-style label/value layout shared by the RAM and disk tables.
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

/// Formats one usage percentage with its catalog unit.
fn format_usage(pct: f32, percent_unit: &str) -> String {
    format!("{pct:.1} {percent_unit}")
}

/// Formats raw bytes as gibibytes with the catalog unit.
fn format_gib(bytes: u64) -> String {
    format!("{:.1} {}", bytes_to_gib(bytes), unit_labels().gib)
}

/// Right-pads `s` with spaces to `width` characters.
fn pad_right(s: &str, width: usize) -> String {
    let missing = width.saturating_sub(s.chars().count());
    format!("{s}{}", " ".repeat(missing))
}

#[cfg(test)]
mod tests {
    use super::*;
    use archel_core::system::CpuCore;

    fn cpu_fixture() -> CpuSnapshot {
        CpuSnapshot {
            cores: vec![
                CpuCore {
                    id: 0,
                    name: "cpu0".to_string(),
                    usage_pct: 12.5,
                    freq_mhz: 4200,
                },
                CpuCore {
                    id: 1,
                    name: "cpu1".to_string(),
                    usage_pct: 3.0,
                    freq_mhz: 3800,
                },
            ],
        }
    }

    fn ram_fixture() -> RamSnapshot {
        RamSnapshot {
            total_bytes: 32 * 1024 * 1024 * 1024,
            used_bytes: 12 * 1024 * 1024 * 1024,
        }
    }

    fn disk_fixture() -> DiskSnapshot {
        DiskSnapshot {
            mount_point: "/".to_string(),
            total_bytes: 800 * 1024 * 1024 * 1024,
            used_bytes: 120 * 1024 * 1024 * 1024,
            free_bytes: 680 * 1024 * 1024 * 1024,
        }
    }

    #[test]
    fn cpu_table_shows_per_core_usage_and_clock() {
        archel_core::i18n::set_language("fr");
        let table = render_cpu_table(&cpu_fixture());
        assert!(table.contains("12.5"), "core 0 usage: {table}");
        assert!(table.contains("4200"), "core 0 clock: {table}");
        assert!(table.contains("3800"), "core 1 clock: {table}");
    }

    #[test]
    fn ram_table_leads_with_used_gib() {
        archel_core::i18n::set_language("fr");
        let table = render_ram_table(&ram_fixture());
        assert!(table.contains("12.0"), "used gib: {table}");
        assert!(table.contains("32.0"), "total gib: {table}");
        let used_pos = table.find("12.0").expect("used must show");
        let total_pos = table.find("32.0").expect("total must show");
        assert!(used_pos < total_pos, "used GB stays prominent: {table}");
    }

    #[test]
    fn disk_table_shows_used_total_free() {
        archel_core::i18n::set_language("fr");
        let table = render_disk_table(&disk_fixture());
        assert!(table.contains("120.0"), "used: {table}");
        assert!(table.contains("800.0"), "total: {table}");
        assert!(table.contains("680.0"), "free: {table}");
    }

    #[test]
    fn disk_table_leads_with_used_gib() {
        archel_core::i18n::set_language("fr");
        let table = render_disk_table(&disk_fixture());
        let used_pos = table.find("120.0").expect("used must show");
        let total_pos = table.find("800.0").expect("total must show");
        assert!(used_pos < total_pos, "used GB stays prominent: {table}");
    }

    #[test]
    fn snapshots_serialize_to_parseable_json() {
        let cpu = serde_json::to_string(&cpu_fixture()).expect("cpu encodes");
        let ram = serde_json::to_string(&ram_fixture()).expect("ram encodes");
        let disk = serde_json::to_string(&disk_fixture()).expect("disk encodes");
        for payload in [cpu, ram, disk] {
            serde_json::from_str::<serde_json::Value>(&payload)
                .expect("tick payload must be one complete JSON object");
        }
    }

    #[test]
    fn padding_counts_characters_not_bytes() {
        assert_eq!(pad_right("ab", 4), "ab  ");
        assert_eq!(pad_right("abcd", 2), "abcd");
    }
}
