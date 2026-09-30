//! `archel sys cpu|ram|disk|gpu|temp|power|all` renderers: French
//! tables (default) or one JSON object per tick (`--json`), with
//! `--watch` re-render.
//!
//! Bin-calls-core shape: every number and every French string comes from
//! `archel-core::system` (snapshots, conversions, catalog labels) or
//! `archel-core::sensors` (GPU, hottest temps, power); this module only
//! lays out columns (`std::fmt` widths, no table crate), frames NDJSON
//! ticks, and owns the 500 ms watch loop. One [`Monitor`] plus one
//! [`Sensors`] handle are created outside the loop and reused across
//! ticks: ticks 2+ always carry valid diff-based CPU numbers
//! (Pitfall 1), and the NVML library stays open with no subprocess per
//! tick (Pitfall 8).
//!
//! Watch exits on Ctrl+C with default SIGINT semantics (exit 130 like
//! standard tools); no cleanup is needed — the loop holds no raw mode or
//! alternate screen. `--json` ticks print exactly one complete object
//! with no clearing and no stray lines, so the stream stays parseable
//! (diagnostics travel via `tracing` on stderr only, never here).
//!
//! Missing sensors render the honest French N/A catalog cell (D-10):
//! a `None` reading is never shown as zero.

use std::time::Duration;

use archel_core::sensors::{
    AllSnapshot, GpuSnapshot, PowerSnapshot, TempSnapshot,
};
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

/// Runs `archel sys gpu`: model plus VRAM used/total (SYS-05). The
/// NVML handle opens once outside the loop and is reused across ticks.
pub fn run_gpu(watch: bool, json: bool) -> Result<(), ArchelError> {
    let sensors = archel_core::sensors::Sensors::new();
    loop {
        let snap = sensors.gpu();
        let payload = serde_json::to_string(&snap).map_err(|err| {
            ArchelError::Internal(format!("cannot encode sys gpu JSON: {err}"))
        })?;
        emit_tick(json, watch, &payload, &render_gpu_table(&snap));
        if !watch {
            return Ok(());
        }
        std::thread::sleep(WATCH_INTERVAL);
    }
}

/// Runs `archel sys temp`: hottest temperature per category (SYS-06).
pub fn run_temp(watch: bool, json: bool) -> Result<(), ArchelError> {
    let sensors = archel_core::sensors::Sensors::new();
    loop {
        let snap = sensors.temps();
        let payload = serde_json::to_string(&snap).map_err(|err| {
            ArchelError::Internal(format!("cannot encode sys temp JSON: {err}"))
        })?;
        emit_tick(json, watch, &payload, &render_temp_table(&snap));
        if !watch {
            return Ok(());
        }
        std::thread::sleep(WATCH_INTERVAL);
    }
}

/// Runs `archel sys power`: real-time CPU and GPU watts (SYS-07/08).
pub fn run_power(watch: bool, json: bool) -> Result<(), ArchelError> {
    let sensors = archel_core::sensors::Sensors::new();
    loop {
        let snap = sensors.power();
        let payload = serde_json::to_string(&snap).map_err(|err| {
            ArchelError::Internal(format!("cannot encode sys power JSON: {err}"))
        })?;
        emit_tick(json, watch, &payload, &render_power_table(&snap));
        if !watch {
            return Ok(());
        }
        std::thread::sleep(WATCH_INTERVAL);
    }
}

/// Runs `archel sys all`: every metric in one table and one `--json`
/// object, composed through the two reusable handles.
pub fn run_all(watch: bool, json: bool) -> Result<(), ArchelError> {
    let mut monitor = archel_core::system::Monitor::new();
    let sensors = archel_core::sensors::Sensors::new();
    loop {
        let snap = archel_core::sensors::collect_all(&mut monitor, &sensors);
        let payload = serde_json::to_string(&snap).map_err(|err| {
            ArchelError::Internal(format!("cannot encode sys all JSON: {err}"))
        })?;
        emit_tick(json, watch, &payload, &render_all_table(&snap));
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

/// Builds the GPU table: model plus VRAM used/total with used GB
/// prominent (first numeric row). Missing hardware renders N/A rows.
pub fn render_gpu_table(snap: &GpuSnapshot) -> String {
    let labels = archel_core::sensors::gpu_table_labels();
    let rows: [(String, String); 3] = [
        (labels.name, format_opt_name(snap.name.as_deref())),
        (labels.vram_used, format_opt_gib(snap.vram_used_bytes)),
        (labels.vram_total, format_opt_gib(snap.vram_total_bytes)),
    ];
    render_label_table(&labels.title, &rows)
}

/// Builds the temperature table: one hottest value per category
/// (D-09), never an average and never a full sensor dump.
pub fn render_temp_table(snap: &TempSnapshot) -> String {
    let labels = archel_core::sensors::temp_table_labels();
    let rows: [(String, String); 2] = [
        (labels.cpu, format_opt_celsius(snap.cpu_c)),
        (labels.gpu, format_opt_celsius(snap.gpu_c)),
    ];
    render_label_table(&labels.title, &rows)
}

/// Builds the power table: real-time CPU and GPU watts.
pub fn render_power_table(snap: &PowerSnapshot) -> String {
    let labels = archel_core::sensors::power_table_labels();
    let rows: [(String, String); 2] = [
        (labels.cpu, format_opt_watts(snap.cpu_w)),
        (labels.gpu, format_opt_watts(snap.gpu_w)),
    ];
    render_label_table(&labels.title, &rows)
}

/// Builds the `sys all` table: every section in one view, reusing the
/// per-metric renderers so the composed output can never drift from
/// the single-metric tables.
pub fn render_all_table(snap: &AllSnapshot) -> String {
    let mut out = format!("{}\n", archel_core::sensors::all_table_title());
    out.push_str(&render_cpu_table(&snap.cpu));
    out.push_str(&render_ram_table(&snap.ram));
    match &snap.disk {
        Some(disk) => out.push_str(&render_disk_table(disk)),
        None => {
            let labels = archel_core::system::disk_table_labels();
            let rows: [(String, String); 1] =
                [(labels.used, archel_core::sensors::unavailable_label())];
            out.push_str(&render_label_table(&labels.title, &rows));
        }
    }
    out.push_str(&render_gpu_table(&snap.gpu));
    out.push_str(&render_temp_table(&snap.temps));
    out.push_str(&render_power_table(&snap.power));
    out
}

/// Landing-style label/value layout shared by the RAM, disk, GPU,
/// temperature and power tables.
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

/// Formats an optional byte count: gibibytes or honest N/A (D-10).
fn format_opt_gib(bytes: Option<u64>) -> String {
    bytes.map_or_else(archel_core::sensors::unavailable_label, format_gib)
}

/// Formats an optional GPU model name or honest N/A (D-10).
fn format_opt_name(name: Option<&str>) -> String {
    name.map_or_else(archel_core::sensors::unavailable_label, str::to_string)
}

/// Formats an optional temperature in degrees or honest N/A (D-10).
fn format_opt_celsius(celsius: Option<f32>) -> String {
    celsius.map_or_else(archel_core::sensors::unavailable_label, |celsius| {
        format!(
            "{celsius:.1} {}",
            archel_core::sensors::sensor_unit_labels().celsius
        )
    })
}

/// Formats an optional power draw in watts or honest N/A (D-10).
fn format_opt_watts(watts: Option<f32>) -> String {
    watts.map_or_else(archel_core::sensors::unavailable_label, |watts| {
        format!(
            "{watts:.1} {}",
            archel_core::sensors::sensor_unit_labels().watt
        )
    })
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

    fn gpu_fixture() -> GpuSnapshot {
        GpuSnapshot {
            name: Some("NVIDIA GeForce GTX 1070 Ti".to_string()),
            vram_used_bytes: Some(1308 * 1024 * 1024),
            vram_total_bytes: Some(8192 * 1024 * 1024),
            temp_c: Some(54.0),
            power_w: Some(34.3),
        }
    }

    fn gpu_absent_fixture() -> GpuSnapshot {
        GpuSnapshot {
            name: None,
            vram_used_bytes: None,
            vram_total_bytes: None,
            temp_c: None,
            power_w: None,
        }
    }

    fn temp_fixture() -> TempSnapshot {
        TempSnapshot {
            cpu_c: Some(47.8),
            gpu_c: Some(54.0),
        }
    }

    fn power_fixture() -> PowerSnapshot {
        PowerSnapshot {
            cpu_w: None,
            gpu_w: Some(34.3),
        }
    }

    fn all_fixture() -> AllSnapshot {
        AllSnapshot {
            cpu: cpu_fixture(),
            ram: ram_fixture(),
            disk: Some(disk_fixture()),
            gpu: gpu_fixture(),
            temps: temp_fixture(),
            power: power_fixture(),
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
        let gpu = serde_json::to_string(&gpu_fixture()).expect("gpu encodes");
        let temp = serde_json::to_string(&temp_fixture()).expect("temp encodes");
        let power = serde_json::to_string(&power_fixture()).expect("power encodes");
        let all = serde_json::to_string(&all_fixture()).expect("all encodes");
        for payload in [cpu, ram, disk, gpu, temp, power, all] {
            serde_json::from_str::<serde_json::Value>(&payload)
                .expect("tick payload must be one complete JSON object");
        }
    }

    #[test]
    fn gpu_table_shows_model_and_vram() {
        archel_core::i18n::set_language("fr");
        let table = render_gpu_table(&gpu_fixture());
        assert!(table.contains("1070 Ti"), "model: {table}");
        assert!(table.contains("8192") || table.contains("8.0"), "total VRAM: {table}");
    }

    #[test]
    fn temp_table_shows_one_value_per_category() {
        archel_core::i18n::set_language("fr");
        let table = render_temp_table(&temp_fixture());
        assert!(table.contains("47.8"), "hottest CPU: {table}");
        assert!(table.contains("54.0"), "GPU: {table}");
    }

    #[test]
    fn power_table_shows_watts_with_honest_na() {
        archel_core::i18n::set_language("fr");
        let table = render_power_table(&power_fixture());
        assert!(table.contains("34.3"), "GPU watts: {table}");
        assert!(
            table.contains(&archel_core::sensors::unavailable_label()),
            "unreadable CPU sensor must render N/A, never 0.0: {table}"
        );
        assert!(!table.contains("0.0 W"), "missing sensor must never show zero: {table}");
    }

    #[test]
    fn absent_gpu_renders_na_everywhere() {
        archel_core::i18n::set_language("fr");
        let table = render_gpu_table(&gpu_absent_fixture());
        assert!(
            table.matches(&archel_core::sensors::unavailable_label()).count() >= 3,
            "every GPU row must be N/A: {table}"
        );
    }

    #[test]
    fn all_table_composes_every_section() {
        archel_core::i18n::set_language("fr");
        let table = render_all_table(&all_fixture());
        for marker in ["12.5", "32.0", "800.0", "1070 Ti", "47.8", "34.3"] {
            assert!(table.contains(marker), "all must include {marker}: {table}");
        }
    }

    // insta pins: every --json schema in snake_case English keys
    // mirroring the struct fields (planner's A5 choice, v1-unstable).
    // A silent field reshape fails here before reaching scripts.

    #[test]
    fn schema_sys_cpu_is_pinned() {
        insta::assert_json_snapshot!("sys_cpu", cpu_fixture());
    }

    #[test]
    fn schema_sys_ram_is_pinned() {
        insta::assert_json_snapshot!("sys_ram", ram_fixture());
    }

    #[test]
    fn schema_sys_disk_is_pinned() {
        insta::assert_json_snapshot!("sys_disk", disk_fixture());
    }

    #[test]
    fn schema_sys_gpu_is_pinned() {
        insta::assert_json_snapshot!("sys_gpu", gpu_fixture());
    }

    #[test]
    fn schema_sys_temp_is_pinned() {
        insta::assert_json_snapshot!("sys_temp", temp_fixture());
    }

    #[test]
    fn schema_sys_power_is_pinned() {
        insta::assert_json_snapshot!("sys_power", power_fixture());
    }

    #[test]
    fn schema_sys_all_is_pinned() {
        insta::assert_json_snapshot!("sys_all", all_fixture());
    }

    #[test]
    fn padding_counts_characters_not_bytes() {
        assert_eq!(pad_right("ab", 4), "ab  ");
        assert_eq!(pad_right("abcd", 2), "abcd");
    }
}
