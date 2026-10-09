//! Bluetooth view-model: known-device list plus the 12 s burst scan
//! (D-08 - bursts only, never continuous discovery).
//!
//! Same proven shape as `models::net`: `tick()` is a QML-called
//! invokable that only spawns - the async `bt_devices()` runs on the
//! gui runtime and returns through `CxxQtThread.queue`. The burst scan
//! is a separate `scanNow()` invokable with its own in-flight flag
//! (T-06-23): while a burst runs the 1 Hz device tick pauses so the
//! burst gets the pool slot alone, the Rechercher button stays
//! disabled with a BusyIndicator, and the list populates only at burst
//! end - no progressive fill (RESEARCH Core Gap #2 decision). Row
//! strings reuse `bluetooth::state_label`/`signal_cell`, the same
//! helpers `bt_cmds` renders (PLAT-03).

use std::sync::atomic::{AtomicBool, Ordering};

use cxx_qt_lib::QString;

use super::super::poll::{BannerPolicy, PollGuard};

/// Fully formatted tick payload, built off the Qt thread so the
/// `queue()` closure only sets properties.
pub(crate) struct AppliedBt {
    pub devices_json: String,
    pub banner_visible: bool,
}

/// Backing state: property fields plus the poll guard, the banner
/// policy, and the burst-scan in-flight flag (separate from the 1 Hz
/// guard so a burst pauses ticks without tripping them).
pub struct BtModelRust {
    pub(crate) devices_json: QString,
    pub(crate) scanning: bool,
    pub(crate) banner_visible: bool,
    guard: PollGuard,
    banner: BannerPolicy,
    scan_busy: AtomicBool,
}

impl Default for BtModelRust {
    fn default() -> Self {
        Self {
            devices_json: QString::from(""),
            scanning: false,
            banner_visible: false,
            guard: PollGuard::new(),
            banner: BannerPolicy::new(),
            scan_busy: AtomicBool::new(false),
        }
    }
}

/// Claims the burst slot: `false` means a scan is already running, so
/// the caller returns early (double-press no-op, T-06-23). The slot is
/// released by [`release_scan`] when the burst completes.
pub(crate) fn try_begin_scan(flag: &AtomicBool) -> bool {
    flag.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
}

/// Releases the burst slot after the burst completes.
pub(crate) fn release_scan(flag: &AtomicBool) {
    flag.store(false, Ordering::SeqCst);
}

/// Device rows as QML-ready JSON: `[{name,address,state,signal}]` in
/// core order. Display strings come from the core helpers the CLI
/// renders - never re-derived here.
pub(crate) fn devices_json_for(devices: &[archel_core::bluetooth::BtDevice]) -> String {
    let rows: Vec<serde_json::Value> = devices
        .iter()
        .map(|device| {
            serde_json::json!({
                "name": device.name,
                "address": device.address,
                "state": archel_core::bluetooth::state_label(device),
                "signal": archel_core::bluetooth::signal_cell(device.rssi),
            })
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string())
}

impl BtModelRust {
    /// Applies a finished worker outcome on the Qt thread. `Err`
    /// (daemon down) reads the empty list plus the once-per-episode
    /// banner; the empty-state copy covers the view.
    fn apply_outcome(
        &mut self,
        outcome: Result<Vec<archel_core::bluetooth::BtDevice>, archel_core::ArchelError>,
    ) -> AppliedBt {
        match outcome {
            Ok(devices) => {
                self.banner.on_success();
                AppliedBt {
                    devices_json: devices_json_for(&devices),
                    banner_visible: false,
                }
            }
            Err(_) => {
                self.banner.on_failure();
                AppliedBt {
                    devices_json: "[]".to_string(),
                    banner_visible: self.banner.wants_visible(),
                }
            }
        }
    }
}

impl crate::gui::bridge_bt::qobject::BtModel {
    /// 1 Hz entry point: skips while a tick is in flight (D-02) and
    /// pauses entirely while a burst scan runs (D-08). Spawns the
    /// async device list, marshals back through `queue()` only.
    pub fn tick(mut self: core::pin::Pin<&mut Self>) {
        use cxx_qt::{CxxQtType, Threading};
        if self.as_mut().rust_mut().scan_busy.load(Ordering::SeqCst) {
            return;
        }
        let tag = match self.as_mut().rust_mut().guard.try_begin() {
            Some(tag) => tag,
            None => return,
        };
        let qt_thread = self.qt_thread();
        let runtime = super::super::RUNTIME
            .get()
            .expect("gui runtime is set before Qt starts");
        runtime.spawn(async move {
            let outcome = archel_core::bluetooth::bt_devices().await;
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
                        .set_devices_json(QString::from(&applied.devices_json));
                    obj.as_mut().set_banner_visible(applied.banner_visible);
                },
            );
        });
    }

    /// Starts the 12 s burst scan (D-08): user press only, never the
    /// 1 Hz timer. A press while a burst runs is a no-op (T-06-23).
    /// The list replaces only when the burst completes - no
    /// progressive fill (RESEARCH Core Gap #2 decision).
    pub fn scan_now(mut self: core::pin::Pin<&mut Self>) {
        use cxx_qt::{CxxQtType, Threading};
        if !try_begin_scan(&self.as_mut().rust_mut().scan_busy) {
            return;
        }
        self.as_mut().set_scanning(true);
        let qt_thread = self.qt_thread();
        let runtime = super::super::RUNTIME
            .get()
            .expect("gui runtime is set before Qt starts");
        runtime.spawn(async move {
            let outcome = archel_core::bluetooth::bt_scan().await;
            let _ = qt_thread.queue(
                move |mut obj: core::pin::Pin<&mut Self>| {
                    let applied = {
                        let mut rust = obj.as_mut().rust_mut();
                        release_scan(&rust.scan_busy);
                        rust.apply_outcome(outcome)
                    };
                    obj.as_mut()
                        .set_devices_json(QString::from(&applied.devices_json));
                    obj.as_mut().set_banner_visible(applied.banner_visible);
                    obj.as_mut().set_scanning(false);
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
    use archel_core::bluetooth::BtDevice;

    fn known_device() -> BtDevice {
        BtDevice {
            address: "AA:BB:CC:DD:EE:FF".to_string(),
            name: "Casque Salon".to_string(),
            rssi: Some(-42),
            paired: true,
            connected: false,
        }
    }

    fn failed_outcome() -> Result<Vec<BtDevice>, archel_core::ArchelError> {
        Err(archel_core::ArchelError::bluetooth_unavailable(
            "daemon down in test",
        ))
    }

    #[test]
    fn error_path_reads_empty_list_and_signals_banner_once() {
        archel_core::i18n::set_language("fr");
        let mut model = BtModelRust::default();
        let first = model.apply_outcome(failed_outcome());
        assert_eq!(first.devices_json, "[]");
        assert!(first.banner_visible, "first failure shows the banner");
        let second = model.apply_outcome(failed_outcome());
        assert!(
            second.banner_visible,
            "consecutive failures keep the banner without re-signalling"
        );
    }

    #[test]
    fn success_fills_rows_with_core_display_strings() {
        archel_core::i18n::set_language("fr");
        let mut model = BtModelRust::default();
        let _ = model.apply_outcome(failed_outcome());
        let applied = model.apply_outcome(Ok(vec![known_device()]));
        assert!(!applied.banner_visible, "success hides the banner");
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(&applied.devices_json).expect("devices must be JSON");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], "Casque Salon");
        assert_eq!(rows[0]["address"], "AA:BB:CC:DD:EE:FF");
        assert!(!rows[0]["state"].as_str().unwrap_or("").is_empty());
        assert_eq!(rows[0]["signal"], "-42 dBm");
    }

    #[test]
    fn scan_guard_double_claim_is_noop_and_releases() {
        let flag = AtomicBool::new(false);
        assert!(try_begin_scan(&flag), "first press claims the burst");
        assert!(!try_begin_scan(&flag), "second press is a no-op");
        assert!(!try_begin_scan(&flag), "third press stays a no-op");
        release_scan(&flag);
        assert!(try_begin_scan(&flag), "slot frees after the burst");
        release_scan(&flag);
    }
}
