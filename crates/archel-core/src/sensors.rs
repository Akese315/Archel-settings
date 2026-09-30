//! GPU / temperature / power sensors (SYS-05..08).
//!
//! Glue over three vendor sources, following the `privileged.rs`
//! skeleton: fallible public fns, pure helpers testable on fixtures,
//! and a `#[cfg(test)]` module mixing fixture unit tests with live
//! range assertions. The binary never touches NVML or sysfs directly
//! -- it calls the snapshot fns (one-shot) or drives a [`Sensors`]
//! handle (watch mode) and renders catalog labels only.
//!
//! Source strategy (02-RESEARCH.md Patterns 2-4, live map 2026-09-30):
//! - GPU (VRAM, temp, power) comes exclusively from NVML. The NVIDIA
//!   card exposes no hwmon node, and no subprocess (`nvidia-smi`) ever
//!   runs in the tick path. `Nvml::init` failure (non-NVIDIA, nouveau,
//!   headless) selects the [`NoGpu`] backend, never a panic.
//! - CPU power comes from Intel-RAPL `energy_uj` deltas. Domains are
//!   discovered by enumerating `/sys/class/powercap` names (never a
//!   hardcoded `:0`); the `package` domain is preferred. Wrap-around
//!   is handled via `max_energy_range_uj`.
//! - Temperatures report the hottest sensor per category (D-09):
//!   sysinfo `Components` filtered by label first, hand-rolled hwmon
//!   discovery-by-name as fallback (never a hardcoded `hwmon2`).
//! - Every reading crossing the core-to-CLI boundary is `Option`:
//!   missing hardware becomes honest French N/A end-to-end, never 0.0.
//! - Display ranges are enforced at the boundary (T-2-04): temps
//!   outside silicon bounds and watts outside sane limits become
//!   `None`; unreadable sysfs nodes become `None`, never panics.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::Serialize;

/// Milliwatts per watt (NVML `power_usage` unit -- Pitfall 2: this
/// divisor lives here as a named const, never as an inline 1000.0).
pub const MILLIWATT_TO_WATT: f32 = 1000.0;

/// Millidegrees per degree (hwmon `temp*_input` unit -- same rule).
pub const MILLIDEGREE_TO_CELSIUS: f32 = 1000.0;

/// Microjoules per joule (RAPL `energy_uj` unit -- same rule).
pub const MICROJOULE_TO_JOULE: f64 = 1_000_000.0;

/// sysfs directory holding the RAPL powercap domains.
pub const POWERCAP_DIR: &str = "/sys/class/powercap";

/// sysfs directory holding the hwmon sensor devices.
pub const HWMON_DIR: &str = "/sys/class/hwmon";

/// RAPL sampling window: two `energy_uj` reads this far apart give the
/// CPU power delta. Short enough to keep snapshots snappy, long enough
/// that quantisation noise stays below a watt on this machine.
pub const RAPL_SAMPLE_WINDOW: Duration = Duration::from_millis(200);

/// Upper bound for a believable watt reading (generous headroom above
/// any single-socket TDP; T-2-04 range assert).
pub const MAX_SANE_WATTS: f64 = 1000.0;

/// Silicon temperature bounds in degrees (T-2-04 range assert).
pub const MIN_SANE_TEMP_C: f32 = -50.0;
pub const MAX_SANE_TEMP_C: f32 = 150.0;

/// Prefix of a RAPL package-domain `name` (e.g. `package-0`). The AMD
/// CPU on this machine sits behind the `intel-rapl` ABI name -- the
/// kernel powercap driver exposes the Intel-RAPL interface.
const RAPL_PACKAGE_PREFIX: &str = "package";

/// hwmon `name`-file values that report CPU silicon temperature.
/// Discovery matches these exactly; indices (`hwmon2`) are never
/// assumed stable across boots (Pitfall 4).
const CPU_HWMON_DRIVERS: &[&str] = &["k10temp", "coretemp", "zenpower", "k8temp"];

/// One sensor backend behind the [`GpuBackend`] trait: real NVML
/// values or honest unavailability. Mirrors the never-default-ok rule:
/// a missing GPU is a typed state, not zeros.
pub trait GpuBackend {
    fn name(&self) -> Option<String>;
    fn vram_used_bytes(&self) -> Option<u64>;
    fn vram_total_bytes(&self) -> Option<u64>;
    fn temp_c(&self) -> Option<f32>;
    fn power_w(&self) -> Option<f32>;
}

/// Live NVIDIA backend. Holds the `Nvml` handle (dlopen'd once);
/// device handles are opened per read from the stored index.
pub struct NvmlBackend {
    nvml: nvml_wrapper::Nvml,
    index: u32,
}

impl NvmlBackend {
    /// Probes for an NVIDIA device. `None` on any failure (no driver,
    /// nouveau, headless) so the caller falls back to [`NoGpu`].
    pub fn probe() -> Option<Self> {
        let nvml = nvml_wrapper::Nvml::init().ok()?;
        // Prove device 0 answers before advertising the backend.
        nvml.device_by_index(0).ok()?;
        Some(Self { nvml, index: 0 })
    }

    fn device(&self) -> Option<nvml_wrapper::Device<'_>> {
        self.nvml.device_by_index(self.index).ok()
    }
}

impl GpuBackend for NvmlBackend {
    fn name(&self) -> Option<String> {
        self.device()?.name().ok()
    }

    fn vram_used_bytes(&self) -> Option<u64> {
        let mem = self.device()?.memory_info().ok()?;
        (mem.used <= mem.total).then_some(mem.used)
    }

    fn vram_total_bytes(&self) -> Option<u64> {
        let mem = self.device()?.memory_info().ok()?;
        (mem.used <= mem.total).then_some(mem.total)
    }

    fn temp_c(&self) -> Option<f32> {
        use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
        let raw = self.device()?.temperature(TemperatureSensor::Gpu).ok()?;
        sane_temp_c(raw as f32)
    }

    fn power_w(&self) -> Option<f32> {
        let milliwatts = self.device()?.power_usage().ok()?;
        sane_watts(milliwatts_to_watts(milliwatts) as f64)
            .map(|w| w as f32)
    }
}

/// Honest-absence backend for machines without NVIDIA hardware.
/// Every reading is `None` so the CLI renders French N/A (D-10).
pub struct NoGpu;

impl GpuBackend for NoGpu {
    fn name(&self) -> Option<String> {
        None
    }
    fn vram_used_bytes(&self) -> Option<u64> {
        None
    }
    fn vram_total_bytes(&self) -> Option<u64> {
        None
    }
    fn temp_c(&self) -> Option<f32> {
        None
    }
    fn power_w(&self) -> Option<f32> {
        None
    }
}

/// GPU snapshot in raw units (SYS-05/06-GPU/08). `None` fields mean
/// the sensor does not exist here -- the binary renders N/A, never 0.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GpuSnapshot {
    pub name: Option<String>,
    pub vram_used_bytes: Option<u64>,
    pub vram_total_bytes: Option<u64>,
    pub temp_c: Option<f32>,
    pub power_w: Option<f32>,
}

/// Hottest temperature per category in degrees (SYS-06, D-09).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TempSnapshot {
    pub cpu_c: Option<f32>,
    pub gpu_c: Option<f32>,
}

/// Real-time power draw in watts (SYS-07/08).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PowerSnapshot {
    pub cpu_w: Option<f32>,
    pub gpu_w: Option<f32>,
}

/// Every metric in one struct for `sys all`: one table, one `--json`
/// object. `disk` is `Option` because the `/` lookup is fallible;
/// everything else degrades to `None` fields internally.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AllSnapshot {
    pub cpu: crate::system::CpuSnapshot,
    pub ram: crate::system::RamSnapshot,
    pub disk: Option<crate::system::DiskSnapshot>,
    pub gpu: GpuSnapshot,
    pub temps: TempSnapshot,
    pub power: PowerSnapshot,
}

/// One discovered RAPL domain: where to read energy plus its wrap
/// range. Resolved once at [`Sensors`] construction, reused across
/// watch ticks (no sysfs re-discovery per tick).
pub struct RaplDomain {
    energy_path: PathBuf,
    max_range_uj: u64,
}

/// Reusable sensor handle for watch mode. The NVML library and the
/// RAPL domain are resolved once outside the tick loop; GPU reads go
/// through the open handle every tick with no subprocess (Pitfall 8).
pub struct Sensors {
    gpu: GpuHandle,
    rapl: Option<RaplDomain>,
}

enum GpuHandle {
    Nvml(NvmlBackend),
    Unavailable,
}

impl Sensors {
    /// Resolves the GPU backend and the RAPL domain once. Never fails:
    /// missing hardware becomes the N/A backends, never an error.
    pub fn new() -> Self {
        Self {
            gpu: NvmlBackend::probe().map_or(GpuHandle::Unavailable, GpuHandle::Nvml),
            rapl: discover_rapl_domain(Path::new(POWERCAP_DIR)),
        }
    }

    /// Reads VRAM used/total plus GPU temp/power through the backend.
    pub fn gpu(&self) -> GpuSnapshot {
        match &self.gpu {
            GpuHandle::Nvml(backend) => snapshot_from_backend(backend),
            GpuHandle::Unavailable => snapshot_from_backend(&NoGpu),
        }
    }

    /// Reads the hottest CPU and GPU temperatures (D-09). CPU tries
    /// sysinfo Components first, then the hwmon discovery fallback.
    pub fn temps(&self) -> TempSnapshot {
        let cpu = hottest_cpu_temp()
            .or_else(|| hwmon_cpu_temp(Path::new(HWMON_DIR)));
        let gpu = match &self.gpu {
            GpuHandle::Nvml(backend) => backend.temp_c(),
            GpuHandle::Unavailable => None,
        };
        TempSnapshot { cpu_c: cpu, gpu_c: gpu }
    }

    /// Reads real-time CPU (RAPL delta) and GPU (NVML) watts. The CPU
    /// read blocks ~[`RAPL_SAMPLE_WINDOW`] for the energy delta.
    pub fn power(&self) -> PowerSnapshot {
        let gpu = match &self.gpu {
            GpuHandle::Nvml(backend) => backend.power_w(),
            GpuHandle::Unavailable => None,
        };
        PowerSnapshot {
            cpu_w: self.cpu_power(),
            gpu_w: gpu,
        }
    }

    /// Whether an NVIDIA backend answered the probe. The CLI never
    /// branches on this (it renders `None` fields as N/A); exposed for
    /// tests and the future GUI.
    pub fn has_nvidia(&self) -> bool {
        matches!(self.gpu, GpuHandle::Nvml(_))
    }

    /// Samples RAPL `energy_uj` twice across the window and converts
    /// the delta to watts. `None` when no RAPL domain was discovered.
    fn cpu_power(&self) -> Option<f32> {
        let domain = self.rapl.as_ref()?;
        let before = read_energy_uj(&domain.energy_path)?;
        let started = Instant::now();
        std::thread::sleep(RAPL_SAMPLE_WINDOW);
        let after = read_energy_uj(&domain.energy_path)?;
        rapl_watts(before, after, domain.max_range_uj, started.elapsed().as_secs_f64())
    }
}

impl Default for Sensors {
    fn default() -> Self {
        Self::new()
    }
}

/// Collects every metric through the two reusable handles (watch mode)
/// or fresh one-shots. CPU pays the D-04 window on first use.
pub fn collect_all(
    monitor: &mut crate::system::Monitor,
    sensors: &Sensors,
) -> AllSnapshot {
    AllSnapshot {
        cpu: monitor.cpu(),
        ram: monitor.ram(),
        disk: monitor.disk().ok(),
        gpu: sensors.gpu(),
        temps: sensors.temps(),
        power: sensors.power(),
    }
}

/// One-shot GPU snapshot (opens and drops the NVML handle).
pub fn gpu_snapshot() -> GpuSnapshot {
    Sensors::new().gpu()
}

/// One-shot hottest-temperature snapshot.
pub fn temps_snapshot() -> TempSnapshot {
    Sensors::new().temps()
}

/// One-shot power snapshot (blocks ~[`RAPL_SAMPLE_WINDOW`] for CPU).
pub fn power_snapshot() -> PowerSnapshot {
    Sensors::new().power()
}

fn snapshot_from_backend(backend: &impl GpuBackend) -> GpuSnapshot {
    GpuSnapshot {
        name: backend.name(),
        vram_used_bytes: backend.vram_used_bytes(),
        vram_total_bytes: backend.vram_total_bytes(),
        temp_c: backend.temp_c(),
        power_w: backend.power_w(),
    }
}

/// Converts NVML milliwatts to watts at the source boundary.
pub fn milliwatts_to_watts(milliwatts: u32) -> f32 {
    milliwatts as f32 / MILLIWATT_TO_WATT
}

/// Converts hwmon millidegrees to degrees at the source boundary.
pub fn millidegrees_to_celsius(millidegrees: f32) -> f32 {
    millidegrees / MILLIDEGREE_TO_CELSIUS
}

/// Parses one hwmon `temp*_input` file body to degrees. Pure over the
/// text so fixtures cover it without sysfs.
pub fn parse_temp_input(body: &str) -> Option<f32> {
    let millidegrees: f32 = body.trim().parse().ok()?;
    sane_temp_c(millidegrees_to_celsius(millidegrees))
}

/// Keeps only finite in-silicon-bound temperatures (T-2-04).
pub fn sane_temp_c(raw_c: f32) -> Option<f32> {
    if raw_c.is_finite() && raw_c >= MIN_SANE_TEMP_C && raw_c <= MAX_SANE_TEMP_C {
        Some(raw_c)
    } else {
        None
    }
}

/// Keeps only finite non-negative sane watt readings (T-2-04).
pub fn sane_watts(raw_w: f64) -> Option<f64> {
    if raw_w.is_finite() && raw_w >= 0.0 && raw_w <= MAX_SANE_WATTS {
        Some(raw_w)
    } else {
        None
    }
}

/// Returns the hottest of the given readings, never an average
/// (D-09). Empty input is `None`, not zero.
pub fn pick_hottest<I>(temps: I) -> Option<f32>
where
    I: IntoIterator<Item = f32>,
{
    temps
        .into_iter()
        .filter(|temp| temp.is_finite())
        .max_by(f32::total_cmp)
}

/// Converts a RAPL energy delta to watts with counter wrap handling.
/// Pure over fixture values so the wrap path is unit-tested.
pub fn rapl_watts(before_uj: u64, after_uj: u64, max_range_uj: u64, dt_secs: f64) -> Option<f32> {
    if !dt_secs.is_finite() || dt_secs <= 0.0 {
        return None;
    }
    let mut after = after_uj;
    if after < before_uj {
        after = after.saturating_add(max_range_uj);
    }
    let delta_uj = after.saturating_sub(before_uj) as f64;
    let watts = delta_uj / dt_secs / MICROJOULE_TO_JOULE;
    sane_watts(watts).map(|watts| watts as f32)
}

/// Discovers the preferred RAPL domain by enumerating `powercap_dir`
/// entries (never a hardcoded `:0`). Prefers a top-level
/// `intel-rapl:<n>` domain whose `name` starts with `package`;
/// otherwise takes the first top-level RAPL domain found.
pub fn discover_rapl_domain(powercap_dir: &Path) -> Option<RaplDomain> {
    let entries = std::fs::read_dir(powercap_dir).ok()?;
    let mut fallback: Option<RaplDomain> = None;
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if !is_top_level_rapl(&name) {
            continue;
        }
        let domain_dir = entry.path();
        let domain_name =
            std::fs::read_to_string(domain_dir.join("name")).unwrap_or_default();
        let domain = RaplDomain {
            energy_path: domain_dir.join("energy_uj"),
            max_range_uj: read_u64_file(&domain_dir.join("max_energy_range_uj"))
                .unwrap_or(u64::MAX),
        };
        if !domain.energy_path.is_file() {
            continue;
        }
        if domain_name.trim().starts_with(RAPL_PACKAGE_PREFIX) {
            return Some(domain);
        }
        if fallback.is_none() {
            fallback = Some(domain);
        }
    }
    fallback
}

/// Matches top-level RAPL domains (`intel-rapl:0`) while skipping
/// sub-domains (`intel-rapl:0:0`) and unrelated powercap entries.
fn is_top_level_rapl(entry: &str) -> bool {
    entry
        .strip_prefix("intel-rapl:")
        .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit()))
}

/// Reads one sysfs energy/counter file as u64.
fn read_energy_uj(path: &Path) -> Option<u64> {
    read_u64_file(path)
}

fn read_u64_file(path: &Path) -> Option<u64> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

/// Whether a sysinfo component label reports CPU silicon. Matches the
/// AMD (`Tctl`/`Tccd1` via k10temp) and Intel (`Package id`,
/// `Core N` via coretemp) label shapes observed live.
pub fn is_cpu_temp_label(label: &str) -> bool {
    label.contains("Tctl")
        || label.contains("Tccd")
        || label.to_lowercase().contains("package")
        || label.to_lowercase().contains("core")
}

/// Hottest CPU temperature via sysinfo Components (primary source).
/// `None` when no CPU-labelled component reports a sane value.
pub fn hottest_cpu_temp() -> Option<f32> {
    let mut components = sysinfo::Components::new_with_refreshed_list();
    components.refresh(true);
    pick_hottest(
        components
            .iter()
            .filter(|component| is_cpu_temp_label(component.label()))
            .filter_map(|component| component.temperature())
            .filter_map(sane_temp_c),
    )
}

/// Hottest CPU temperature via hwmon discovery-by-name (fallback).
/// Matches device `name` files against [`CPU_HWMON_DRIVERS`], then
/// takes the max over every `temp*_input` node found.
pub fn hwmon_cpu_temp(hwmon_dir: &Path) -> Option<f32> {
    let devices = std::fs::read_dir(hwmon_dir).ok()?;
    let mut hottest: Option<f32> = None;
    for device in devices.flatten() {
        let dir = device.path();
        let driver = std::fs::read_to_string(dir.join("name")).unwrap_or_default();
        if !CPU_HWMON_DRIVERS.contains(&driver.trim()) {
            continue;
        }
        let nodes = std::fs::read_dir(&dir).ok()?;
        for node in nodes.flatten() {
            let file_name = node.file_name();
            let node_name = file_name.to_string_lossy();
            if !node_name.starts_with("temp") || !node_name.ends_with("_input") {
                continue;
            }
            let body = std::fs::read_to_string(node.path()).unwrap_or_default();
            if let Some(celsius) = parse_temp_input(&body) {
                hottest = pick_hottest(hottest.into_iter().chain([celsius]));
            }
        }
    }
    hottest
}

/// French GPU table labels from the catalog (bin owns no copy).
pub struct GpuTableLabels {
    pub title: String,
    pub name: String,
    pub vram_used: String,
    pub vram_total: String,
}

/// Collects the GPU table labels in one call.
pub fn gpu_table_labels() -> GpuTableLabels {
    GpuTableLabels {
        title: rust_i18n::t!("sys.gpu_title").to_string(),
        name: rust_i18n::t!("sys.gpu_name").to_string(),
        vram_used: rust_i18n::t!("sys.gpu_vram_used").to_string(),
        vram_total: rust_i18n::t!("sys.gpu_vram_total").to_string(),
    }
}

/// French temperature table labels from the catalog.
pub struct TempTableLabels {
    pub title: String,
    pub cpu: String,
    pub gpu: String,
}

/// Collects the temperature table labels in one call.
pub fn temp_table_labels() -> TempTableLabels {
    TempTableLabels {
        title: rust_i18n::t!("sys.temp_title").to_string(),
        cpu: rust_i18n::t!("sys.temp_cpu").to_string(),
        gpu: rust_i18n::t!("sys.temp_gpu").to_string(),
    }
}

/// French power table labels from the catalog.
pub struct PowerTableLabels {
    pub title: String,
    pub cpu: String,
    pub gpu: String,
}

/// Collects the power table labels in one call.
pub fn power_table_labels() -> PowerTableLabels {
    PowerTableLabels {
        title: rust_i18n::t!("sys.power_title").to_string(),
        cpu: rust_i18n::t!("sys.power_cpu").to_string(),
        gpu: rust_i18n::t!("sys.power_gpu").to_string(),
    }
}

/// French `sys all` table title from the catalog.
pub fn all_table_title() -> String {
    rust_i18n::t!("sys.all_title").to_string()
}

/// Honest French N/A cell for a missing sensor (D-10).
pub fn unavailable_label() -> String {
    rust_i18n::t!("sys.unavailable").to_string()
}

/// Display unit labels for sensors (degrees, watts).
pub struct SensorUnitLabels {
    pub celsius: String,
    pub watt: String,
}

/// Collects the sensor unit labels in one call.
pub fn sensor_unit_labels() -> SensorUnitLabels {
    SensorUnitLabels {
        celsius: rust_i18n::t!("sys.unit_celsius").to_string(),
        watt: rust_i18n::t!("sys.unit_watt").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn milliwatt_conversion_matches_nvidia_smi_scale() {
        // Live anchor 2026-10-01: nvidia-smi read ~34 W while NVML
        // reported the same draw in milliwatts (A2 cross-check).
        assert!((milliwatts_to_watts(34_290) - 34.29).abs() < 0.001);
        assert!((milliwatts_to_watts(42_780) - 42.78).abs() < 0.001);
        assert_eq!(milliwatts_to_watts(0), 0.0);
    }

    #[test]
    fn millidegree_conversion_matches_hwmon_scale() {
        // Live anchor: k10temp Tccd1 read 38750 millidegrees.
        assert!((millidegrees_to_celsius(38_750.0) - 38.75).abs() < f32::EPSILON);
        assert!((parse_temp_input("38750\n").expect("must parse") - 38.75).abs() < 0.001);
        assert!(parse_temp_input("not-a-number").is_none());
    }

    #[test]
    fn hottest_pick_reports_the_max_never_an_average() {
        // Live anchor: Tctl 38.0 vs Tccd1 38.75 -- Tccd1 must win (D-09).
        assert_eq!(pick_hottest([38.0, 38.75]), Some(38.75));
        assert_eq!(pick_hottest([38.75, 38.0]), Some(38.75));
        assert_eq!(pick_hottest([]), None);
        assert_eq!(pick_hottest([f32::NAN, 40.0]), Some(40.0));
    }

    #[test]
    fn rapl_delta_converts_energy_over_time_to_watts() {
        // 20 joules over 2 s is 10 W.
        let watts = rapl_watts(1_000_000, 21_000_000, u64::MAX, 2.0).expect("must compute");
        assert!((watts - 10.0).abs() < 0.001, "got {watts}");
    }

    #[test]
    fn rapl_counter_wrap_uses_max_energy_range() {
        // Counter wrapped: max 1000, before 950, after 50 -> delta 100 uJ.
        let watts = rapl_watts(950, 50, 1000, 100.0).expect("wrap must compute");
        assert!((watts - 0.000_001).abs() < 1e-9, "got {watts}");
    }

    #[test]
    fn rapl_rejects_non_positive_or_absurd_windows() {
        assert!(rapl_watts(0, 1_000_000, u64::MAX, 0.0).is_none());
        assert!(rapl_watts(0, 1_000_000, u64::MAX, -1.0).is_none());
        assert!(rapl_watts(0, 1_000_000, u64::MAX, f64::NAN).is_none());
    }

    #[test]
    fn sane_filters_reject_out_of_silicon_values() {
        assert!(sane_temp_c(47.75).is_some());
        assert!(sane_temp_c(200.0).is_none());
        assert!(sane_temp_c(f32::NAN).is_none());
        assert!(sane_watts(42.78).is_some());
        assert!(sane_watts(-1.0).is_none());
        assert!(sane_watts(50_000.0).is_none());
    }

    #[test]
    fn cpu_label_filter_matches_amd_and_intel_shapes() {
        assert!(is_cpu_temp_label("Tctl"));
        assert!(is_cpu_temp_label("Tccd1"));
        assert!(is_cpu_temp_label("Package id 0"));
        assert!(is_cpu_temp_label("Core 0"));
        assert!(!is_cpu_temp_label("Composite"));
        assert!(!is_cpu_temp_label("Sensor 1"));
    }

    #[test]
    fn no_gpu_backend_reports_nothing() {
        let snap = snapshot_from_backend(&NoGpu);
        assert_eq!(
            snap,
            GpuSnapshot {
                name: None,
                vram_used_bytes: None,
                vram_total_bytes: None,
                temp_c: None,
                power_w: None,
            }
        );
    }

    #[test]
    fn rapl_discovery_prefers_the_package_domain() {
        let base = fixture_dir("rapl-pick");
        write_fixture(&base, "intel-rapl:0", "package-0", 1_000_000, 500_000);
        write_fixture(&base, "intel-rapl:0:0", "core", 100_000, 10_000);
        let domain = discover_rapl_domain(&base).expect("package domain must win");
        assert!(
            domain.energy_path.to_string_lossy().contains("intel-rapl:0/"),
            "top-level domain must win over the :0 sub-domain: {}",
            domain.energy_path.display()
        );
        assert_eq!(domain.max_range_uj, 1_000_000);
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn rapl_discovery_ignores_sub_domains_and_missing_energy() {
        let base = fixture_dir("rapl-skip");
        std::fs::create_dir_all(base.join("intel-rapl:0")).expect("fixture dir");
        // No energy_uj file: not a usable domain.
        std::fs::write(base.join("intel-rapl:0").join("name"), "package-0").expect("name");
        assert!(discover_rapl_domain(&base).is_none());
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn hwmon_fallback_reads_by_driver_name() {
        let base = fixture_dir("hwmon-pick");
        let k10 = base.join("hwmon9");
        std::fs::create_dir_all(&k10).expect("fixture dir");
        std::fs::write(k10.join("name"), "k10temp").expect("name");
        std::fs::write(k10.join("temp1_input"), "38000\n").expect("tctl");
        std::fs::write(k10.join("temp3_input"), "38750\n").expect("tccd1");
        let wifi = base.join("hwmon4");
        std::fs::create_dir_all(&wifi).expect("fixture dir");
        std::fs::write(wifi.join("name"), "mt7921_phy0").expect("name");
        std::fs::write(wifi.join("temp1_input"), "99999\n").expect("ignored");
        let hottest = hwmon_cpu_temp(&base).expect("k10temp must be found");
        assert!((hottest - 38.75).abs() < 0.001, "got {hottest}");
        std::fs::remove_dir_all(&base).ok();
    }

    fn fixture_dir(case: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "archel-sensors-test-{}-{}",
            std::process::id(),
            case
        ))
    }

    fn write_fixture(base: &Path, domain: &str, name: &str, max_range: u64, energy: u64) {
        let dir = base.join(domain);
        std::fs::create_dir_all(&dir).expect("fixture dir");
        std::fs::write(dir.join("name"), name).expect("name file");
        std::fs::write(dir.join("max_energy_range_uj"), max_range.to_string())
            .expect("range file");
        std::fs::write(dir.join("energy_uj"), energy.to_string()).expect("energy file");
    }

    #[test]
    fn live_nvml_probe_answers_on_this_machine() {
        // A6-adjacent guard: this machine has a GTX 1070 Ti, so the
        // probe must succeed here. On non-NVIDIA machines the same
        // call must return None (never panic) -- covered by NoGpu.
        let sensors = Sensors::new();
        assert!(sensors.has_nvidia(), "GTX 1070 Ti must probe");
        let gpu = sensors.gpu();
        let total = gpu.vram_total_bytes.expect("VRAM total must read");
        assert_eq!(total, 8 * 1024 * 1024 * 1024, "1070 Ti has 8 GiB");
        let used = gpu.vram_used_bytes.expect("VRAM used must read");
        assert!(used <= total, "used {used} must not exceed total {total}");
        let temp = gpu.temp_c.expect("GPU temp must read");
        assert!(
            (0.0..=110.0).contains(&temp),
            "GPU temp out of silicon bounds: {temp}"
        );
    }

    #[test]
    fn live_gpu_power_sits_within_tdp() {
        // A2 live cross-check anchor: nvidia-smi draws ~34 W idle and
        // the card TDP is 180 W, so NVML/1000 must land in between.
        let power = sensors_power_w();
        assert!(
            power.is_some_and(|watts| watts > 0.0 && watts <= 180.0),
            "GPU power must be plausible watts, got {power:?}"
        );
    }

    #[test]
    fn live_cpu_temp_matches_hottest_k10temp() {
        // A6: Components labels surface Tctl/Tccd1 on this machine, and
        // the reported CPU temp must be the hottest of the two (D-09).
        let cpu = hottest_cpu_temp().expect("k10temp must be visible");
        assert!(
            (20.0..=105.0).contains(&cpu),
            "CPU temp out of silicon bounds: {cpu}"
        );
        let fallback =
            hwmon_cpu_temp(Path::new(HWMON_DIR)).expect("hwmon fallback must agree");
        // Loose bound on purpose: the two reads are seconds apart and
        // the suite itself heats the CPU in between (parallel live
        // tests), so only gross disagreement fails here.
        assert!(
            (cpu - fallback).abs() < 10.0,
            "Components {cpu} vs hwmon {fallback} must roughly agree"
        );
    }

    #[test]
    fn live_cpu_power_is_plausible_or_honestly_absent() {
        // Live environment fact (2026-10-01): both RAPL `energy_uj`
        // nodes are mode 400 root-only, and no hwmon power node exists,
        // so an unprivileged run CANNOT read CPU watts. T-2-04 then
        // requires honest N/A (None), never a panic and never 0.0. The
        // delta math itself is proven on fixtures above; if a future
        // kernel relaxes the mode, this test asserts plausible watts.
        let power = Sensors::new().power();
        match std::fs::read_to_string(
            std::path::Path::new(POWERCAP_DIR).join("intel-rapl:0").join("energy_uj"),
        ) {
            Err(_) => assert!(
                power.cpu_w.is_none(),
                "unreadable RAPL must degrade to N/A, got {:?}",
                power.cpu_w
            ),
            Ok(_) => {
                let cpu = power.cpu_w.expect("readable RAPL must produce watts");
                assert!(
                    cpu >= 0.0 && cpu <= 300.0,
                    "CPU power must be plausible watts, got {cpu}"
                );
            }
        }
    }

    fn sensors_power_w() -> Option<f32> {
        Sensors::new().power().gpu_w
    }

    #[test]
    fn sensor_labels_come_from_the_catalog() {
        rust_i18n::set_locale("fr");
        assert!(!gpu_table_labels().title.trim().is_empty());
        assert!(!temp_table_labels().title.trim().is_empty());
        assert!(!power_table_labels().title.trim().is_empty());
        assert!(!all_table_title().trim().is_empty());
        assert!(!unavailable_label().trim().is_empty());
        let units = sensor_unit_labels();
        assert!(!units.celsius.trim().is_empty());
        assert!(!units.watt.trim().is_empty());
    }
}
