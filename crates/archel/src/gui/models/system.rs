//! System Info view-model: 1 Hz CPU/RAM sparklines, per-core expander,
//! representative temps (D-09/D-10/D-11).
//!
//! Pattern (copied by every later tab plan): `tick()` is a QML-called
//! invokable that only spawns - the blocking core reads run on the gui
//! runtime's blocking pool, and the finished payload returns exclusively
//! through `CxxQtThread.queue`, whose closure is the only code allowed to
//! set properties (RESEARCH Pattern 1). The 1 s CPU window therefore
//! never blocks the Qt loop (Pitfall 1, T-06-20).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use cxx_qt_lib::QString;

use super::super::poll::{BannerPolicy, PollGuard};

/// Rolling graph window: 60 samples at 1 Hz (D-09).
pub(crate) const SERIES_CAP: usize = 60;

/// Honest token for missing sensors (D-04): never a zero-faked number.
pub(crate) const NOT_AVAILABLE: &str = "N/A";

/// One per-core row, JSON-marshalled for the QML expander.
#[derive(Debug, Clone)]
pub(crate) struct CoreRow {
    pub id: usize,
    pub name: String,
    pub usage_pct: f32,
    pub freq_mhz: u64,
}

/// Fully formatted tick payload, built off the Qt thread so the `queue()`
/// closure only sets properties.
#[derive(Debug, Clone)]
pub(crate) struct SystemTick {
    pub cpu_avg: Option<f32>,
    pub clock_mhz: Option<u64>,
    pub cores: Vec<CoreRow>,
    pub ram_used_gib: f64,
    pub ram_total_gib: f64,
    pub ram_pct: f32,
    pub disk_gib: Option<(f64, f64)>,
    pub temp_cpu: Option<f32>,
    pub temp_gpu: Option<f32>,
    pub gpu_name: Option<String>,
    pub gpu_vram_gib: Option<(f64, f64)>,
    pub kernel: String,
    pub drivers: DriverStrings,
}

/// Driver identity strings, pre-formatted off the Qt thread (N/A for
/// every absent field, D-04).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DriverStrings {
    pub cpu_model: String,
    pub cpu_microcode: String,
    pub gpu_driver: String,
    pub cuda: String,
}

/// Driver identity never changes at runtime, and `driver_info()`
/// spawns nvidia-smi: read it once for the process, reuse forever.
static DRIVERS: std::sync::OnceLock<DriverStrings> = std::sync::OnceLock::new();

/// Collects and caches the driver strings (first caller pays the
/// nvidia-smi spawn, every later tick reuses the snapshot).
pub(crate) fn driver_strings() -> DriverStrings {
    DRIVERS
        .get_or_init(|| {
            let info = archel_core::sensors::driver_info();
            DriverStrings {
                cpu_model: info
                    .cpu_model
                    .unwrap_or_else(|| NOT_AVAILABLE.to_string()),
                cpu_microcode: info
                    .cpu_microcode
                    .unwrap_or_else(|| NOT_AVAILABLE.to_string()),
                gpu_driver: match (&info.gpu_driver, &info.gpu_driver_version) {
                    (Some(name), Some(version)) => format!("{name} {version}"),
                    (Some(name), None) => name.clone(),
                    (None, _) => NOT_AVAILABLE.to_string(),
                },
                cuda: info
                    .cuda_version
                    .unwrap_or_else(|| NOT_AVAILABLE.to_string()),
            }
        })
        .clone()
}

/// A tick either carries a full payload or reports the core snapshot
/// unavailable (kernel unreadable - the only hard failure, mirroring
/// `landing::snapshot`). Partial sensor gaps stay inside the payload as
/// `None` fields rendered N/A.
pub(crate) enum TickOutcome {
    Ready(SystemTick),
    Unavailable,
}

/// Backing state: property fields plus Qt-thread-only poll state (series,
/// guard, banner). The shared core handles are `Arc` clones handed to
/// each worker; only the `queue()` closure touches the rest.
pub struct SystemModelRust {
    pub(crate) cpu_text: QString,
    pub(crate) clock_text: QString,
    pub(crate) cpu_series: QString,
    pub(crate) ram_text: QString,
    pub(crate) ram_series: QString,
    pub(crate) disk_text: QString,
    pub(crate) temp_cpu_text: QString,
    pub(crate) temp_gpu_text: QString,
    pub(crate) gpu_text: QString,
    pub(crate) kernel_text: QString,
    pub(crate) cpu_model_text: QString,
    pub(crate) cpu_driver_text: QString,
    pub(crate) gpu_driver_text: QString,
    pub(crate) cuda_text: QString,
    pub(crate) cores_json: QString,
    pub(crate) core_count: i32,
    pub(crate) banner_visible: bool,
    monitor: Arc<Mutex<archel_core::system::Monitor>>,
    sensors: Arc<archel_core::sensors::Sensors>,
    guard: PollGuard,
    banner: BannerPolicy,
    cpu_history: VecDeque<f32>,
    ram_history: VecDeque<f32>,
}

impl Default for SystemModelRust {
    fn default() -> Self {
        Self {
            cpu_text: QString::from(""),
            clock_text: QString::from(""),
            cpu_series: QString::from(""),
            ram_text: QString::from(""),
            ram_series: QString::from(""),
            disk_text: QString::from(""),
            temp_cpu_text: QString::from(""),
            temp_gpu_text: QString::from(""),
            gpu_text: QString::from(""),
            kernel_text: QString::from(""),
            cpu_model_text: QString::from(""),
            cpu_driver_text: QString::from(""),
            gpu_driver_text: QString::from(""),
            cuda_text: QString::from(""),
            cores_json: QString::from(""),
            core_count: 0,
            banner_visible: false,
            monitor: Arc::new(Mutex::new(archel_core::system::Monitor::new())),
            sensors: Arc::new(archel_core::sensors::Sensors::new()),
            guard: PollGuard::new(),
            banner: BannerPolicy::new(),
            cpu_history: VecDeque::new(),
            ram_history: VecDeque::new(),
        }
    }
}

/// Pushes one sample, keeping the rolling 60-sample window (D-09).
pub(crate) fn push_capped(history: &mut VecDeque<f32>, value: f32) {
    if history.len() >= SERIES_CAP {
        history.pop_front();
    }
    history.push_back(value);
}

/// Serializes the rolling series for the Canvas sparkline.
pub(crate) fn series_json(history: &VecDeque<f32>) -> String {
    serde_json::to_string(history).unwrap_or_else(|_| "[]".to_string())
}

/// Zero-one-many row (D-10): a 1-core CPU hides the per-core expander.
/// Pre-first-tick (`0`) keeps it visible so loading still shows layout.
#[allow(dead_code)]
pub(crate) fn show_core_expander(core_count: usize) -> bool {
    core_count != 1
}

/// Mean aggregate usage, or `None` when no core reported (N/A path).
pub(crate) fn aggregate_usage(cores: &[CoreRow]) -> Option<f32> {
    if cores.is_empty() {
        return None;
    }
    Some(cores.iter().map(|core| core.usage_pct).sum::<f32>() / cores.len() as f32)
}

/// Formats an optional Celsius reading, or honest N/A (D-04/D-11).
pub(crate) fn format_opt_celsius(celsius: Option<f32>, unit: &str) -> String {
    celsius.map_or_else(
        || NOT_AVAILABLE.to_string(),
        |value| format!("{value:.1} {unit}"),
    )
}

/// Collects one tick off the Qt thread: stateful monitor plus shared
/// sensor handle, formatted into display strings here so the `queue()`
/// closure stays property-sets only.
pub(crate) fn collect_tick(
    monitor: &Mutex<archel_core::system::Monitor>,
    sensors: &archel_core::sensors::Sensors,
) -> TickOutcome {
    let kernel = match archel_core::system::kernel_version() {
        Ok(kernel) => kernel,
        Err(_) => return TickOutcome::Unavailable,
    };
    let mut monitor = monitor.lock().expect("system monitor mutex poisoned");
    let cpu = monitor.cpu();
    let ram = monitor.ram();
    let disk = monitor.disk().ok();
    drop(monitor);
    let temps = sensors.temps();
    let gpu = sensors.gpu();
    TickOutcome::Ready(SystemTick {
        cpu_avg: aggregate_usage(
            &cpu
                .cores
                .iter()
                .map(|core| CoreRow {
                    id: core.id,
                    name: core.name.clone(),
                    usage_pct: core.usage_pct,
                    freq_mhz: core.freq_mhz,
                })
                .collect::<Vec<_>>(),
        ),
        clock_mhz: cpu.cores.iter().map(|core| core.freq_mhz).max(),
        cores: cpu
            .cores
            .iter()
            .map(|core| CoreRow {
                id: core.id,
                name: core.name.clone(),
                usage_pct: core.usage_pct,
                freq_mhz: core.freq_mhz,
            })
            .collect(),
        ram_used_gib: archel_core::system::bytes_to_gib(ram.used_bytes),
        ram_total_gib: archel_core::system::bytes_to_gib(ram.total_bytes),
        ram_pct: archel_core::system::clamp_usage(
            100.0 * ram.used_bytes as f32 / ram.total_bytes.max(1) as f32,
        ),
        disk_gib: disk.map(|disk| {
            (
                archel_core::system::bytes_to_gib(disk.used_bytes),
                archel_core::system::bytes_to_gib(disk.total_bytes),
            )
        }),
        temp_cpu: temps.cpu_c,
        temp_gpu: temps.gpu_c,
        gpu_name: gpu.name,
        gpu_vram_gib: match (gpu.vram_used_bytes, gpu.vram_total_bytes) {
            (Some(used), Some(total)) => Some((
                archel_core::system::bytes_to_gib(used),
                archel_core::system::bytes_to_gib(total),
            )),
            _ => None,
        },
        kernel,
        drivers: driver_strings(),
    })
}

/// Owned display strings computed on the Qt thread: the `queue()`
/// closure feeds each one through its generated setter so NOTIFY fires
/// and QML re-renders. Empty means pre-first-tick (QML shows the
/// ellipsis); N/A means a failed or missing source (D-04).
pub(crate) struct AppliedTick {
    pub cpu_text: String,
    pub clock_text: String,
    pub cpu_series: String,
    pub ram_text: String,
    pub ram_series: String,
    pub disk_text: String,
    pub temp_cpu_text: String,
    pub temp_gpu_text: String,
    pub gpu_text: String,
    pub kernel_text: String,
    pub cpu_model_text: String,
    pub cpu_driver_text: String,
    pub gpu_driver_text: String,
    pub cuda_text: String,
    pub cores_json: String,
    pub core_count: i32,
    pub banner_visible: bool,
}

impl SystemModelRust {
    /// Applies a finished worker outcome on the Qt thread: rolls the
    /// series, runs the banner policy, and returns owned strings. The
    /// caller (the `queue()` closure) pushes each string through its
    /// generated property setter - direct field writes would skip NOTIFY.
    fn apply_outcome(&mut self, outcome: TickOutcome) -> AppliedTick {
        match outcome {
            TickOutcome::Ready(tick) => {
                self.banner.on_success();
                let gib = archel_core::system::unit_labels().gib;
                let mhz = archel_core::system::unit_labels().mhz;
                let percent = archel_core::system::unit_labels().percent;
                let celsius = archel_core::sensors::sensor_unit_labels().celsius;
                let cpu_text = match tick.cpu_avg {
                    Some(avg) => {
                        push_capped(&mut self.cpu_history, avg);
                        format!("{avg:.1} {percent}")
                    }
                    None => NOT_AVAILABLE.to_string(),
                };
                let clock_text = tick.clock_mhz.map_or_else(
                    || NOT_AVAILABLE.to_string(),
                    |clock| format!("{clock} {mhz}"),
                );
                let ram_text = format!(
                    "{:.1} / {:.1} {gib}",
                    tick.ram_used_gib, tick.ram_total_gib
                );
                push_capped(&mut self.ram_history, tick.ram_pct);
                let disk_text = tick.disk_gib.map_or_else(
                    || NOT_AVAILABLE.to_string(),
                    |(used, total)| format!("{used:.1} / {total:.1} {gib}"),
                );
                let gpu_text = match (&tick.gpu_name, tick.gpu_vram_gib) {
                    (Some(name), Some((used, total))) => {
                        format!("{name} : {used:.1} / {total:.1} {gib}")
                    }
                    (Some(name), None) => name.clone(),
                    (None, Some((used, total))) => {
                        format!("{used:.1} / {total:.1} {gib}")
                    }
                    (None, None) => NOT_AVAILABLE.to_string(),
                };
                let rows: Vec<serde_json::Value> = tick
                    .cores
                    .iter()
                    .map(|core| {
                        serde_json::json!({
                            "id": core.id,
                            "name": core.name,
                            "usage": core.usage_pct,
                            "freq": core.freq_mhz,
                        })
                    })
                    .collect();
                AppliedTick {
                    cpu_text,
                    clock_text,
                    cpu_series: series_json(&self.cpu_history),
                    ram_text,
                    ram_series: series_json(&self.ram_history),
                    disk_text,
                    temp_cpu_text: format_opt_celsius(tick.temp_cpu, &celsius),
                    temp_gpu_text: format_opt_celsius(tick.temp_gpu, &celsius),
                    gpu_text,
                    kernel_text: tick.kernel,
                    cpu_model_text: tick.drivers.cpu_model,
                    cpu_driver_text: tick.drivers.cpu_microcode,
                    gpu_driver_text: tick.drivers.gpu_driver,
                    cuda_text: tick.drivers.cuda,
                    cores_json: serde_json::to_string(&rows)
                        .unwrap_or_else(|_| "[]".to_string()),
                    core_count: tick.cores.len() as i32,
                    banner_visible: false,
                }
            }
            TickOutcome::Unavailable => {
                self.banner.on_failure();
                let na = NOT_AVAILABLE.to_string();
                AppliedTick {
                    cpu_text: na.clone(),
                    clock_text: na.clone(),
                    cpu_series: series_json(&self.cpu_history),
                    ram_text: na.clone(),
                    ram_series: series_json(&self.ram_history),
                    disk_text: na.clone(),
                    temp_cpu_text: na.clone(),
                    temp_gpu_text: na.clone(),
                    gpu_text: na.clone(),
                    kernel_text: na.clone(),
                    cpu_model_text: na.clone(),
                    cpu_driver_text: na.clone(),
                    gpu_driver_text: na.clone(),
                    cuda_text: na,
                    cores_json: "[]".to_string(),
                    core_count: 0,
                    banner_visible: self.banner.wants_visible(),
                }
            }
        }
    }
}

impl crate::gui::bridge_system::qobject::SystemModel {
    /// 1 Hz entry point: skips while a tick is in flight (D-02), spawns
    /// the blocking core reads, marshals back through `queue()` only.
    pub fn tick(mut self: core::pin::Pin<&mut Self>) {
        use cxx_qt::{CxxQtType, Threading};
        let tag = match self.as_mut().rust_mut().guard.try_begin() {
            Some(tag) => tag,
            None => return,
        };
        let qt_thread = self.qt_thread();
        let monitor = self.as_mut().rust_mut().monitor.clone();
        let sensors = self.as_mut().rust_mut().sensors.clone();
        let runtime = super::super::RUNTIME
            .get()
            .expect("gui runtime is set before Qt starts");
        runtime.spawn_blocking(move || {
            let outcome = collect_tick(&monitor, &sensors);
            let _ = qt_thread.queue(
                move |mut obj: core::pin::Pin<&mut Self>| {
                    let applied = {
                        let mut rust = obj.as_mut().rust_mut();
                        if !rust.guard.is_newest(tag) {
                            rust.guard.finish();
                            return;
                        }
                        rust.guard.finish();
                        rust.apply_outcome(outcome)
                    };
                    obj.as_mut()
                        .set_cpu_text(QString::from(&applied.cpu_text));
                    obj.as_mut()
                        .set_clock_text(QString::from(&applied.clock_text));
                    obj.as_mut()
                        .set_cpu_series(QString::from(&applied.cpu_series));
                    obj.as_mut()
                        .set_ram_text(QString::from(&applied.ram_text));
                    obj.as_mut()
                        .set_ram_series(QString::from(&applied.ram_series));
                    obj.as_mut()
                        .set_disk_text(QString::from(&applied.disk_text));
                    obj.as_mut()
                        .set_temp_cpu_text(QString::from(&applied.temp_cpu_text));
                    obj.as_mut()
                        .set_temp_gpu_text(QString::from(&applied.temp_gpu_text));
                    obj.as_mut()
                        .set_gpu_text(QString::from(&applied.gpu_text));
                    obj.as_mut()
                        .set_kernel_text(QString::from(&applied.kernel_text));
                    obj.as_mut()
                        .set_cpu_model_text(QString::from(&applied.cpu_model_text));
                    obj.as_mut()
                        .set_cpu_driver_text(QString::from(&applied.cpu_driver_text));
                    obj.as_mut()
                        .set_gpu_driver_text(QString::from(&applied.gpu_driver_text));
                    obj.as_mut()
                        .set_cuda_text(QString::from(&applied.cuda_text));
                    obj.as_mut()
                        .set_cores_json(QString::from(&applied.cores_json));
                    obj.as_mut().set_core_count(applied.core_count);
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_fixture(cores: usize) -> TickOutcome {
        TickOutcome::Ready(SystemTick {
            cpu_avg: Some(12.5),
            clock_mhz: Some(4200),
            cores: (0..cores)
                .map(|id| CoreRow {
                    id,
                    name: format!("cpu{id}"),
                    usage_pct: 10.0,
                    freq_mhz: 4200,
                })
                .collect(),
            ram_used_gib: 12.0,
            ram_total_gib: 32.0,
            ram_pct: 37.5,
            disk_gib: Some((120.0, 800.0)),
            temp_cpu: Some(47.8),
            temp_gpu: None,
            gpu_name: None,
            gpu_vram_gib: None,
            kernel: "6.18-test".to_string(),
            drivers: DriverStrings {
                cpu_model: "AMD Ryzen 7 5800X 8-Core Processor".to_string(),
                cpu_microcode: "0xa201211".to_string(),
                gpu_driver: "nvidia 580.178.04".to_string(),
                cuda: "13.0".to_string(),
            },
        })
    }

    #[test]
    fn series_window_caps_at_sixty_samples() {
        let mut history = VecDeque::new();
        for sample in 0..80 {
            push_capped(&mut history, sample as f32);
        }
        assert_eq!(history.len(), SERIES_CAP, "rolling window must stay bounded");
        assert_eq!(
            history.front().copied(),
            Some(20.0),
            "oldest samples drop first"
        );
        let parsed: Vec<f32> =
            serde_json::from_str(&series_json(&history)).expect("series must be JSON");
        assert_eq!(parsed.len(), SERIES_CAP);
    }

    #[test]
    fn unavailable_tick_emits_na_never_zero() {
        let mut model = SystemModelRust::default();
        let applied = model.apply_outcome(TickOutcome::Unavailable);
        for text in [
            &applied.cpu_text,
            &applied.clock_text,
            &applied.ram_text,
            &applied.disk_text,
            &applied.temp_cpu_text,
            &applied.gpu_text,
            &applied.kernel_text,
        ] {
            assert_eq!(text, NOT_AVAILABLE, "failed tick must read N/A");
            assert!(!text.contains('0'), "N/A must never be zero-faked");
        }
        assert!(applied.banner_visible, "first failure shows the banner");
        let second = model.apply_outcome(TickOutcome::Unavailable);
        assert!(
            second.banner_visible,
            "consecutive failures keep the banner, never re-signal"
        );
    }

    #[test]
    fn success_hides_banner_and_fills_values() {
        let mut model = SystemModelRust::default();
        let _ = model.apply_outcome(TickOutcome::Unavailable);
        let applied = model.apply_outcome(ready_fixture(4));
        assert!(!applied.banner_visible, "success hides the banner");
        assert!(applied.cpu_text.contains("12.5"), "cpu: {}", applied.cpu_text);
        assert!(applied.ram_text.contains("12.0"), "ram: {}", applied.ram_text);
        assert_eq!(
            applied.temp_gpu_text, NOT_AVAILABLE,
            "missing GPU sensor reads N/A, never 0"
        );
        assert_eq!(applied.core_count, 4);
        assert!(show_core_expander(4), "multi-core shows the expander");
        assert!(!show_core_expander(1), "1-core hides the expander");
        assert!(show_core_expander(0), "pre-first-tick keeps layout");
        // Driver strings pass through verbatim, N/A for none (D-04).
        assert_eq!(applied.cpu_model_text, "AMD Ryzen 7 5800X 8-Core Processor");
        assert_eq!(applied.cpu_driver_text, "0xa201211");
        assert_eq!(applied.gpu_driver_text, "nvidia 580.178.04");
        assert_eq!(applied.cuda_text, "13.0");
    }

    #[test]
    fn aggregate_usage_needs_at_least_one_core() {
        assert!(aggregate_usage(&[]).is_none());
        let cores = vec![
            CoreRow { id: 0, name: "cpu0".to_string(), usage_pct: 10.0, freq_mhz: 4200 },
            CoreRow { id: 1, name: "cpu1".to_string(), usage_pct: 30.0, freq_mhz: 4200 },
        ];
        assert_eq!(aggregate_usage(&cores), Some(20.0));
    }

    #[test]
    fn live_collect_tick_returns_a_real_payload() {
        let monitor = Mutex::new(archel_core::system::Monitor::new());
        let sensors = archel_core::sensors::Sensors::new();
        match collect_tick(&monitor, &sensors) {
            TickOutcome::Ready(tick) => {
                assert!(!tick.cores.is_empty(), "live CPU must list cores");
                assert!(!tick.kernel.trim().is_empty());
            }
            TickOutcome::Unavailable => {
                panic!("kernel must be readable on this machine");
            }
        }
    }
}
