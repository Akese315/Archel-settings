//! CPU / RAM / system-partition disk samplers (SYS-01..04).
//!
//! Thin ownership of the `sysinfo` sampling contract, following the
//! `privileged.rs` skeleton: fallible public fns returning
//! `Result<_, ArchelError>`, pure helpers testable on fixtures, and a
//! `#[cfg(test)]` module mixing fixture unit tests with live range
//! assertions. The binary never touches `sysinfo` directly — it calls the
//! snapshot fns (one-shot) or drives a [`Monitor`] (watch mode) and
//! renders catalog labels only.
//!
//! Sampling rules (02-RESEARCH.md Patterns 1-2, Pitfalls 1/4):
//! - CPU percentages are diff-based: refresh, wait [`CPU_SAMPLE_WINDOW`]
//!   (D-04), refresh again, then read. The first pass alone is garbage.
//! - In watch mode one [`Monitor`] is created outside the tick loop and
//!   reused, so ticks 2+ always carry valid diff-based numbers.
//! - Disk selection matches `mount_point == "/"` exactly (D-08); index 0
//!   is never assumed to be the system partition.
//! - Display ranges are enforced at the boundary (T-2-03): CPU usage is
//!   clamped to 0-100, `used` is `total.saturating_sub(available)` so it
//!   can never exceed `total`. Read failures become typed errors, never
//!   panics or fabricated numbers.

use std::path::Path;
use std::time::Duration;

use serde::Serialize;

use crate::error::ArchelError;

/// D-04 sampling window: CPU percentages are measured over ~1 s so the
/// first visible tick already carries reliable numbers.
pub const CPU_SAMPLE_WINDOW: Duration = Duration::from_secs(1);

/// Divisor turning raw byte counts into gibibytes for display.
pub const BYTES_PER_GIB: f64 = 1024.0 * 1024.0 * 1024.0;

/// Divisor turning megahertz into gigahertz for display.
pub const MHZ_PER_GHZ: f64 = 1000.0;

/// One logical core: diff-based usage plus current clock.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CpuCore {
    pub id: usize,
    pub name: String,
    pub usage_pct: f32,
    pub freq_mhz: u64,
}

/// Per-core CPU snapshot (SYS-03) plus clock (SYS-04).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CpuSnapshot {
    pub cores: Vec<CpuCore>,
}

/// Memory snapshot in raw bytes (SYS-01); the binary formats gibibytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RamSnapshot {
    pub total_bytes: u64,
    pub used_bytes: u64,
}

/// System-partition disk snapshot in raw bytes (SYS-02, D-08).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiskSnapshot {
    pub mount_point: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
}

/// Reusable sampler for watch mode. Created once outside the tick loop;
/// the CPU priming pass runs at construction (first refresh) plus the
/// first [`Monitor::cpu`] call (sleep + second refresh), so every visible
/// tick — including tick 1 — reports real numbers. Later ticks refresh
/// once per call; the inter-tick gap is the diff window.
pub struct Monitor {
    sys: sysinfo::System,
    cpu_primed: bool,
}

impl Monitor {
    /// Primes the CPU sampler with the first refresh pass. Cheap: the
    /// ~1 s D-04 wait happens lazily on the first [`Monitor::cpu`] call
    /// so RAM/disk-only consumers never pay it.
    pub fn new() -> Self {
        let mut sys = sysinfo::System::new_all();
        sys.refresh_cpu_all();
        Self {
            sys,
            cpu_primed: false,
        }
    }

    /// Reads per-core usage plus frequency. Blocks ~1 s on the first
    /// call (D-04 window); later calls refresh once and return
    /// immediately.
    pub fn cpu(&mut self) -> CpuSnapshot {
        if !self.cpu_primed {
            std::thread::sleep(CPU_SAMPLE_WINDOW);
            self.cpu_primed = true;
        }
        self.sys.refresh_cpu_all();
        CpuSnapshot {
            cores: read_cores(&self.sys),
        }
    }

    /// Reads total/used memory.
    pub fn ram(&mut self) -> RamSnapshot {
        self.sys.refresh_memory();
        RamSnapshot {
            total_bytes: self.sys.total_memory(),
            used_bytes: self.sys.used_memory(),
        }
    }

    /// Reads the system-partition (`/`) disk. Fails typed when `/` is
    /// absent (containers, exotic mounts) instead of reporting a
    /// non-root filesystem.
    pub fn disk(&mut self) -> Result<DiskSnapshot, ArchelError> {
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let root = disks
            .iter()
            .find(|disk| disk.mount_point() == Path::new("/"))
            .ok_or(ArchelError::SystemPartitionNotFound)?;
        let total = root.total_space();
        let free = root.available_space();
        Ok(DiskSnapshot {
            mount_point: "/".to_string(),
            total_bytes: total,
            used_bytes: used_from_total_available(total, free),
            free_bytes: free,
        })
    }
}

impl Default for Monitor {
    fn default() -> Self {
        Self::new()
    }
}

/// One-shot per-core CPU snapshot with the D-04 window: refresh, sleep
/// ~1 s, refresh, read.
pub fn cpu_snapshot() -> Result<CpuSnapshot, ArchelError> {
    Ok(Monitor::new().cpu())
}

/// One-shot memory snapshot.
pub fn ram_snapshot() -> RamSnapshot {
    Monitor::new().ram()
}

/// One-shot system-partition disk snapshot.
pub fn disk_snapshot() -> Result<DiskSnapshot, ArchelError> {
    Monitor::new().disk()
}

/// Running kernel version, reused by the landing snapshot (LAND-02).
pub fn kernel_version() -> Result<String, ArchelError> {
    sysinfo::System::kernel_version().ok_or_else(|| {
        ArchelError::Internal("kernel version unavailable from sysinfo".to_string())
    })
}

/// Collects per-core readings from an already-refreshed `System`.
/// Pure over the sysinfo state; range-enforced (T-2-03).
fn read_cores(sys: &sysinfo::System) -> Vec<CpuCore> {
    sys.cpus()
        .iter()
        .enumerate()
        .map(|(id, cpu)| CpuCore {
            id,
            name: cpu.name().to_owned(),
            usage_pct: clamp_usage(cpu.cpu_usage()),
            freq_mhz: cpu.frequency(),
        })
        .collect()
}

/// Clamps a raw CPU percentage into the display range (T-2-03).
/// sysinfo diffs can overshoot marginally on hotplug or timer skew;
/// the table never shows out-of-range values.
pub fn clamp_usage(pct: f32) -> f32 {
    pct.clamp(0.0, 100.0)
}

/// Derives used bytes so `used <= total` always holds (T-2-03), even if
/// the kernel reports a transient `available > total`.
pub fn used_from_total_available(total: u64, available: u64) -> u64 {
    total.saturating_sub(available)
}

/// Converts raw bytes to gibibytes for French display (`Gio`).
pub fn bytes_to_gib(bytes: u64) -> f64 {
    bytes as f64 / BYTES_PER_GIB
}

/// Converts megahertz to gigahertz for French display (`GHz`).
pub fn mhz_to_ghz(mhz: u64) -> f64 {
    mhz as f64 / MHZ_PER_GHZ
}

/// Index of the `/` mount in `mount_points`, or `None`. Matches exactly
/// (D-08); the system partition is never assumed to sit at index 0.
pub fn find_root_index(mount_points: &[&Path]) -> Option<usize> {
    mount_points.iter().position(|path| *path == Path::new("/"))
}

/// French CPU table labels from the catalog (bin owns no copy).
pub struct CpuTableLabels {
    pub title: String,
    pub core: String,
    pub usage: String,
    pub frequency: String,
}

/// Collects the CPU table labels in one call.
pub fn cpu_table_labels() -> CpuTableLabels {
    CpuTableLabels {
        title: rust_i18n::t!("sys.cpu_title").to_string(),
        core: rust_i18n::t!("sys.cpu_core").to_string(),
        usage: rust_i18n::t!("sys.cpu_usage").to_string(),
        frequency: rust_i18n::t!("sys.cpu_frequency").to_string(),
    }
}

/// French RAM table labels from the catalog.
pub struct RamTableLabels {
    pub title: String,
    pub used: String,
    pub total: String,
}

/// Collects the RAM table labels in one call.
pub fn ram_table_labels() -> RamTableLabels {
    RamTableLabels {
        title: rust_i18n::t!("sys.ram_title").to_string(),
        used: rust_i18n::t!("sys.ram_used").to_string(),
        total: rust_i18n::t!("sys.ram_total").to_string(),
    }
}

/// French disk table labels from the catalog.
pub struct DiskTableLabels {
    pub title: String,
    pub used: String,
    pub total: String,
    pub free: String,
}

/// Collects the disk table labels in one call.
pub fn disk_table_labels() -> DiskTableLabels {
    DiskTableLabels {
        title: rust_i18n::t!("sys.disk_title").to_string(),
        used: rust_i18n::t!("sys.disk_used").to_string(),
        total: rust_i18n::t!("sys.disk_total").to_string(),
        free: rust_i18n::t!("sys.disk_free").to_string(),
    }
}

/// Display unit labels from the catalog (`Gio`, `MHz`, `%`).
pub struct UnitLabels {
    pub gib: String,
    pub mhz: String,
    pub percent: String,
}

/// Collects the unit labels in one call.
pub fn unit_labels() -> UnitLabels {
    UnitLabels {
        gib: rust_i18n::t!("sys.unit_gib").to_string(),
        mhz: rust_i18n::t!("sys.unit_mhz").to_string(),
        percent: rust_i18n::t!("sys.unit_percent").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_usage_keeps_valid_values_and_bounds_outliers() {
        assert_eq!(clamp_usage(42.5), 42.5);
        assert_eq!(clamp_usage(0.0), 0.0);
        assert_eq!(clamp_usage(100.0), 100.0);
        assert_eq!(clamp_usage(-3.0), 0.0);
        assert_eq!(clamp_usage(101.5), 100.0);
    }

    #[test]
    fn used_bytes_never_exceeds_total() {
        assert_eq!(used_from_total_available(100, 30), 70);
        assert_eq!(used_from_total_available(100, 100), 0);
        // Transient kernel skew: saturates instead of underflowing.
        assert_eq!(used_from_total_available(100, 130), 0);
    }

    #[test]
    fn byte_and_frequency_conversions_match_binary_units() {
        assert!((bytes_to_gib(1024 * 1024 * 1024) - 1.0).abs() < f64::EPSILON);
        assert!((mhz_to_ghz(4200) - 4.2).abs() < 1e-9);
    }

    #[test]
    fn root_pick_matches_slash_exactly_never_index_zero() {
        let boot = Path::new("/boot/efi");
        let root = Path::new("/");
        let home = Path::new("/home");
        assert_eq!(find_root_index(&[boot, root, home]), Some(1));
        assert_eq!(find_root_index(&[boot, home]), None);
        assert_eq!(find_root_index(&[]), None);
        // A prefix-looking path is not the system partition.
        assert_eq!(find_root_index(&[Path::new("/rooted")]), None);
    }

    #[test]
    fn live_cpu_snapshot_has_sane_per_core_numbers() {
        let snap = cpu_snapshot().expect("CPU snapshot must succeed");
        assert!(!snap.cores.is_empty(), "at least one core expected");
        for core in &snap.cores {
            assert!(
                (0.0..=100.0).contains(&core.usage_pct),
                "core {} out of range: {}",
                core.id,
                core.usage_pct
            );
            assert!(
                core.freq_mhz > 0,
                "core {} must report a clock speed",
                core.id
            );
        }
    }

    #[test]
    fn live_core_count_matches_parallelism() {
        let snap = cpu_snapshot().expect("CPU snapshot must succeed");
        let expected = std::thread::available_parallelism()
            .expect("parallelism must be readable")
            .get();
        assert_eq!(
            snap.cores.len(),
            expected,
            "sysinfo core count must match the machine"
        );
    }

    #[test]
    fn live_cpu_snapshot_is_not_all_zero() {
        // Pitfall 1 guard: a single refresh pass yields all-zero cores.
        // The two-pass window must produce at least one non-zero core on
        // a live machine (background activity always exists).
        let snap = cpu_snapshot().expect("CPU snapshot must succeed");
        assert!(
            snap.cores.iter().any(|core| core.usage_pct > 0.0),
            "two-pass snapshot must not report all-zero cores"
        );
    }

    #[test]
    fn live_ram_values_are_sane() {
        let snap = ram_snapshot();
        assert!(snap.total_bytes > 0, "RAM total must be positive");
        assert!(
            snap.used_bytes <= snap.total_bytes,
            "RAM used must not exceed total"
        );
    }

    #[test]
    fn live_disk_reports_the_root_partition() {
        let snap = disk_snapshot().expect("root disk must be readable");
        assert_eq!(snap.mount_point, "/");
        assert!(snap.total_bytes > 0, "disk total must be positive");
        assert!(
            snap.used_bytes <= snap.total_bytes,
            "disk used must not exceed total"
        );
    }

    #[test]
    fn live_kernel_version_is_non_empty() {
        let kernel = kernel_version().expect("kernel must be readable");
        assert!(!kernel.trim().is_empty());
    }

    #[test]
    fn table_and_unit_labels_come_from_the_catalog() {
        rust_i18n::set_locale("fr");
        assert!(!cpu_table_labels().title.trim().is_empty());
        assert!(!ram_table_labels().title.trim().is_empty());
        assert!(!disk_table_labels().title.trim().is_empty());
        let units = unit_labels();
        assert!(!units.gib.trim().is_empty());
        assert!(!units.mhz.trim().is_empty());
        assert!(!units.percent.trim().is_empty());
    }
}
