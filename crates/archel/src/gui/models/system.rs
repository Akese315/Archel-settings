//! System Info view-model: 1 Hz CPU/RAM sparklines, per-core expander,
//! representative temps (D-09/D-10/D-11).
//!
//! STUB (06-03-T1): backing struct plus no-op invokables so the tree
//! stays green; T2 fills the poll body and the pure helpers with tests.

use cxx_qt_lib::QString;

/// Backing state (T2: monitor handle, guard, series, banner policy).
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
    pub(crate) cores_json: QString,
    pub(crate) core_count: i32,
    pub(crate) banner_visible: bool,
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
            cores_json: QString::from(""),
            core_count: 0,
            banner_visible: false,
        }
    }
}

impl crate::gui::bridge_system::qobject::SystemModel {
    /// STUB: replaced by the spawn_blocking poll in T2.
    pub fn tick(self: core::pin::Pin<&mut Self>) {
        let _ = self;
    }

    /// STUB: replaced by the banner dismiss in T2.
    pub fn dismiss_banner(self: core::pin::Pin<&mut Self>) {
        let _ = self;
    }
}
