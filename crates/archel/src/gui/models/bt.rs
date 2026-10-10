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
use crate::gui::modal::{self, ModalSession};

/// Tag carried in `busy_address` while the adapter toggle runs (no
/// device row owns that spinner).
pub(crate) const ADAPTER_BUSY_TAG: &str = "adapter";

/// Fully formatted tick payload, built off the Qt thread so the
/// `queue()` closure only sets properties.
pub(crate) struct AppliedBt {
    pub devices_json: String,
    pub adapter_on: bool,
    pub banner_visible: bool,
}

/// Backing state: property fields plus the poll guard, the banner
/// policy, the last known device set (pair/connect/disconnect picks
/// resolve against it - scan-then-pick, never a free-form MAC), and
/// the burst-scan in-flight flag (separate from the 1 Hz guard so a
/// burst pauses ticks without tripping them).
pub struct BtModelRust {
    pub(crate) devices_json: QString,
    pub(crate) scanning: bool,
    pub(crate) banner_visible: bool,
    pub(crate) adapter_on: bool,
    pub(crate) operation_busy: bool,
    pub(crate) busy_address: QString,
    pub(crate) last_devices: Vec<archel_core::bluetooth::BtDevice>,
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
            adapter_on: false,
            operation_busy: false,
            busy_address: QString::from(""),
            last_devices: Vec::new(),
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

/// Device rows as QML-ready JSON: `[{name,address,state,signal,
/// paired,connected}]` in core order. Display strings come from the
/// core helpers the CLI renders - never re-derived here. The flags
/// gate the Pair/Connect/Disconnect buttons per row.
pub(crate) fn devices_json_for(devices: &[archel_core::bluetooth::BtDevice]) -> String {
    let rows: Vec<serde_json::Value> = devices
        .iter()
        .map(|device| {
            serde_json::json!({
                "name": device.name,
                "address": device.address,
                "state": archel_core::bluetooth::state_label(device),
                "signal": archel_core::bluetooth::signal_cell(device.rssi),
                "paired": device.paired,
                "connected": device.connected,
            })
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string())
}

impl BtModelRust {
    /// Applies a finished worker outcome on the Qt thread. `Err`
    /// (daemon down) reads the empty list plus the once-per-episode
    /// banner; the empty-state copy covers the view. The known-device
    /// set refreshes on every success (later picks resolve against
    /// it); the adapter flag keeps its previous value when the read
    /// itself failed (honest, never guessed).
    fn apply_outcome(
        &mut self,
        outcome: Result<Vec<archel_core::bluetooth::BtDevice>, archel_core::ArchelError>,
        adapter: Option<bool>,
    ) -> AppliedBt {
        match outcome {
            Ok(devices) => {
                self.banner.on_success();
                self.last_devices = devices.clone();
                if let Some(on) = adapter {
                    self.adapter_on = on;
                }
                AppliedBt {
                    devices_json: devices_json_for(&devices),
                    adapter_on: self.adapter_on,
                    banner_visible: false,
                }
            }
            Err(_) => {
                self.banner.on_failure();
                AppliedBt {
                    devices_json: "[]".to_string(),
                    adapter_on: self.adapter_on,
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
            let adapter = archel_core::bluetooth::adapter_powered().await.ok();
            let _ = qt_thread.queue(
                move |mut obj: core::pin::Pin<&mut Self>| {
                    let applied = {
                        let mut rust = obj.as_mut().rust_mut();
                        if !rust.guard.is_newest(tag) {
                            rust.guard.finish();
                            return;
                        }
                        rust.guard.finish();
                        rust.apply_outcome(outcome, adapter)
                    };
                    obj.as_mut()
                        .set_devices_json(QString::from(&applied.devices_json));
                    obj.as_mut().set_adapter_on(applied.adapter_on);
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
                        rust.apply_outcome(outcome, None)
                    };
                    obj.as_mut()
                        .set_devices_json(QString::from(&applied.devices_json));
                    obj.as_mut().set_adapter_on(applied.adapter_on);
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

    /// Pairs a known device (D-12 + D-21): the pick resolves against
    /// the last known set first (unknown picks never reach a
    /// confirm); the intent confirm uses the core pair prompt; the
    /// BlueZ code then surfaces in the global PairingModal with
    /// Confirmer/Refuser - never auto-accept. A declined code removes
    /// any half-bond best-effort and reports nothing (decline is a
    /// first-class answer). Each device row carries its own spinner
    /// through `busy_address` while the tab locks (D-15).
    pub fn pair(mut self: core::pin::Pin<&mut Self>, address: QString) {
        // The pair worker needs the process-global session (its code
        // callback is 'static for the core agent slot), so it skips
        // the slot-local handoff the other workers share.
        self.as_mut().start_device_operation(
            &address.to_string(),
            |devices, pick, ops, _sess| {
                pair_worker(devices, pick, ops, modal::session())
            },
        );
    }

    /// Removes a bond after the core unpair confirm.
    pub fn unpair(mut self: core::pin::Pin<&mut Self>, address: QString) {
        self.as_mut().start_device_operation(&address.to_string(), |devices, pick, ops, sess| {
            unpair_worker(devices, pick, ops, sess)
        });
    }

    /// Connects a paired device after the core connect confirm.
    pub fn connect(mut self: core::pin::Pin<&mut Self>, address: QString) {
        self.as_mut().start_device_operation(&address.to_string(), |devices, pick, ops, sess| {
            connect_worker(devices, pick, ops, sess)
        });
    }

    /// Disconnects a connected device after the core disconnect confirm.
    pub fn disconnect(mut self: core::pin::Pin<&mut Self>, address: QString) {
        self.as_mut().start_device_operation(&address.to_string(), |devices, pick, ops, sess| {
            disconnect_worker(devices, pick, ops, sess)
        });
    }

    /// Toggles the adapter radio after the core adapter confirm; the
    /// read-back state is the proof. The spinner sits on the switch
    /// row (`ADAPTER_BUSY_TAG`) while the tab locks (D-15).
    pub fn set_adapter_powered(mut self: core::pin::Pin<&mut Self>, on: bool) {
        use cxx_qt::CxxQtType;
        if self.as_mut().rust_mut().operation_busy {
            return;
        }
        self.as_mut().rust_mut().operation_busy = true;
        self.as_mut().rust_mut().busy_address = QString::from(ADAPTER_BUSY_TAG);
        self.as_mut().set_operation_busy(true);
        self.as_mut()
            .set_busy_address(QString::from(ADAPTER_BUSY_TAG));
        let qt_thread = {
            use cxx_qt::Threading;
            self.qt_thread()
        };
        let runtime = super::super::RUNTIME
            .get()
            .expect("gui runtime is set before Qt starts");
        runtime.spawn_blocking(move || {
            adapter_worker(on, &PROD_BT_WRITE_OPS, modal::session());
            let _ = qt_thread.queue(|mut obj: core::pin::Pin<&mut Self>| {
                obj.as_mut().finish_operation();
            });
        });
    }

    /// Claims the single-operation slot for one device address and
    /// mirrors the claim onto the QML-visible flags so the tab
    /// disables itself and the acting row shows its spinner (D-15).
    /// The pick resolves worker-side against a clone of the last
    /// known set (scan-then-pick, never a free-form MAC).
    fn start_device_operation(
        mut self: core::pin::Pin<&mut Self>,
        address: &str,
        worker: fn(
            &[archel_core::bluetooth::BtDevice],
            &str,
            &BtWriteOps,
            &'static ModalSession,
        ),
    ) {
        use cxx_qt::CxxQtType;
        if self.as_mut().rust_mut().operation_busy {
            return;
        }
        self.as_mut().rust_mut().operation_busy = true;
        self.as_mut().rust_mut().busy_address = QString::from(address);
        self.as_mut().set_operation_busy(true);
        self.as_mut()
            .set_busy_address(QString::from(address));
        let devices = self.as_mut().rust_mut().last_devices.clone();
        let pick = address.to_string();
        let qt_thread = {
            use cxx_qt::Threading;
            self.qt_thread()
        };
        let runtime = super::super::RUNTIME
            .get()
            .expect("gui runtime is set before Qt starts");
        runtime.spawn_blocking(move || {
            worker(&devices, &pick, &PROD_BT_WRITE_OPS, modal::session());
            let _ = qt_thread.queue(|mut obj: core::pin::Pin<&mut Self>| {
                obj.as_mut().finish_operation();
            });
        });
    }

    /// Releases the operation slot, clears the row spinner and
    /// refreshes the list so the row flags read back from the daemon.
    /// Called from the `queue()` closure only (property writes stay
    /// on the Qt thread, RESEARCH Pattern 1).
    fn finish_operation(mut self: core::pin::Pin<&mut Self>) {
        use cxx_qt::CxxQtType;
        self.as_mut().rust_mut().operation_busy = false;
        self.as_mut().rust_mut().busy_address = QString::from("");
        self.as_mut().set_operation_busy(false);
        self.as_mut().set_busy_address(QString::from(""));
        self.as_mut().tick();
    }
}

/// Answer callback for one pairing code: a plain blocking bool over
/// the pairing session. Boxed so the production adapter can hand
/// ownership to the core agent (whose callback slot is `'static`).
pub(crate) type PairAnswer = dyn Fn(String) -> bool + Send + Sync;

/// Injection seam for the write path: the exact core entry points
/// the workers call, as plain function pointers. Production passes
/// the real core functions ([`PROD_BT_WRITE_OPS`]); unit tests pass
/// fakes so the resolve-confirm-pair/switch choreography is proven
/// without touching BlueZ (the D-Bus contract is the core's tested
/// territory - the GUI never holds a child, a pid or a signal).
pub(crate) struct BtWriteOps {
    pub pair: fn(
        &archel_core::bluetooth::BtDevice,
        Box<PairAnswer>,
    ) -> Result<(), archel_core::ArchelError>,
    pub unpair:
        fn(&archel_core::bluetooth::BtDevice) -> Result<(), archel_core::ArchelError>,
    pub connect:
        fn(&archel_core::bluetooth::BtDevice) -> Result<(), archel_core::ArchelError>,
    pub disconnect:
        fn(&archel_core::bluetooth::BtDevice) -> Result<(), archel_core::ArchelError>,
    pub set_adapter: fn(bool) -> Result<bool, archel_core::ArchelError>,
}

/// The production wiring: every field is the CLI-proven core
/// function (thin-view contract; PLAT-03). The async transport calls
/// run through the current runtime handle - the workers always run
/// on the blocking pool, where that is allowed.
pub(crate) const PROD_BT_WRITE_OPS: BtWriteOps = BtWriteOps {
    pair: |device, answer| {
        tokio::runtime::Handle::current().block_on(
            archel_core::bluetooth::pair_device_with(device, move |code| {
                let result = answer(code);
                async move { result }
            }),
        )
    },
    unpair: |device| {
        tokio::runtime::Handle::current()
            .block_on(archel_core::bluetooth::unpair_device(device))
    },
    connect: |device| {
        tokio::runtime::Handle::current()
            .block_on(archel_core::bluetooth::connect_device(device))
    },
    disconnect: |device| {
        tokio::runtime::Handle::current()
            .block_on(archel_core::bluetooth::disconnect_device(device))
    },
    set_adapter: |on| {
        tokio::runtime::Handle::current()
            .block_on(archel_core::bluetooth::set_adapter_powered(on))
    },
};

/// Resolves a row pick against the last known set. An unknown pick
/// opens the honest typed error - it never reaches a confirm.
fn resolve_target(
    devices: &[archel_core::bluetooth::BtDevice],
    pick: &str,
    sess: &ModalSession,
) -> Option<archel_core::bluetooth::BtDevice> {
    match archel_core::bluetooth::resolve_pick(devices, pick) {
        Ok(device) => Some(device),
        Err(err) => {
            sess.open_error(&err.message());
            None
        }
    }
}

/// Blocks the worker on the modal's one-shot answer. `None`-shaped
/// results (session dropped mid-wait) default to No - the same
/// default-No semantics as the CLI's `parse_confirm_answer`.
fn await_confirm(sess: &ModalSession, question: &str) -> bool {
    let rx = sess.request_confirm(question);
    rx.recv().unwrap_or(false)
}

/// Pair worker body (runs on the blocking pool): resolve, intent
/// confirm with the core pair prompt, then the BlueZ handshake with
/// the code callback driving the global PairingModal. A declined
/// code removes any half-bond best-effort and reports nothing;
/// transport failures surface as core's readable `message()` in the
/// error modal (D-17), privilege shapes included.
///
/// The session is `&'static` (in production the process-global
/// session): the code callback is boxed for the core agent slot,
/// which is `'static`, so the callback must own everything it
/// touches - a short-lived borrow cannot cross that boundary.
pub(crate) fn pair_worker(
    devices: &[archel_core::bluetooth::BtDevice],
    pick: &str,
    ops: &BtWriteOps,
    sess: &'static ModalSession,
) {
    let Some(device) = resolve_target(devices, pick, sess) else {
        return;
    };
    if !await_confirm(sess, &archel_core::bluetooth::pair_confirm_prompt(&device)) {
        return;
    }
    sess.begin_pairing(&device.address, &device.name, "yesno");
    let declined = std::sync::Arc::new(AtomicBool::new(false));
    let declined_cb = declined.clone();
    let result = (ops.pair)(
        &device,
        Box::new(move |code| {
            let rx = sess.pairing_request_code(&code);
            let answer = rx
                .recv_timeout(archel_core::bluetooth::PAIR_TIMEOUT)
                .unwrap_or(false);
            if !answer {
                declined_cb.store(true, Ordering::SeqCst);
            }
            answer
        }),
    );
    sess.end_pairing();
    if declined.load(Ordering::SeqCst) {
        let _ = (ops.unpair)(&device);
        return;
    }
    if let Err(err) = result {
        sess.open_error(&err.message());
    }
}

/// Unpair worker body: resolve, confirm with the core unpair prompt,
/// remove the bond; failures surface in the error modal.
pub(crate) fn unpair_worker(
    devices: &[archel_core::bluetooth::BtDevice],
    pick: &str,
    ops: &BtWriteOps,
    sess: &ModalSession,
) {
    let Some(device) = resolve_target(devices, pick, sess) else {
        return;
    };
    if !await_confirm(
        sess,
        &archel_core::bluetooth::unpair_confirm_prompt(&device),
    ) {
        return;
    }
    if let Err(err) = (ops.unpair)(&device) {
        sess.open_error(&err.message());
    }
}

/// Connect worker body: same resolve-confirm-act shape over the core
/// connect prompt and transport call.
pub(crate) fn connect_worker(
    devices: &[archel_core::bluetooth::BtDevice],
    pick: &str,
    ops: &BtWriteOps,
    sess: &ModalSession,
) {
    let Some(device) = resolve_target(devices, pick, sess) else {
        return;
    };
    if !await_confirm(
        sess,
        &archel_core::bluetooth::connect_confirm_prompt(&device),
    ) {
        return;
    }
    if let Err(err) = (ops.connect)(&device) {
        sess.open_error(&err.message());
    }
}

/// Disconnect worker body: same shape over the core disconnect
/// prompt and transport call.
pub(crate) fn disconnect_worker(
    devices: &[archel_core::bluetooth::BtDevice],
    pick: &str,
    ops: &BtWriteOps,
    sess: &ModalSession,
) {
    let Some(device) = resolve_target(devices, pick, sess) else {
        return;
    };
    if !await_confirm(
        sess,
        &archel_core::bluetooth::disconnect_confirm_prompt(&device),
    ) {
        return;
    }
    if let Err(err) = (ops.disconnect)(&device) {
        sess.open_error(&err.message());
    }
}

/// Adapter-toggle worker body (runs on the blocking pool): confirm
/// with the core adapter prompt, then set; the read-back state is
/// the proof (a mismatch surfaces as the core internal message).
/// Failures surface in the error modal.
pub(crate) fn adapter_worker(on: bool, ops: &BtWriteOps, sess: &ModalSession) {
    if !await_confirm(sess, &archel_core::bluetooth::adapter_confirm_prompt(on)) {
        return;
    }
    if let Err(err) = (ops.set_adapter)(on) {
        sess.open_error(&err.message());
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
        let first = model.apply_outcome(failed_outcome(), None);
        assert_eq!(first.devices_json, "[]");
        assert!(first.banner_visible, "first failure shows the banner");
        let second = model.apply_outcome(failed_outcome(), None);
        assert!(
            second.banner_visible,
            "consecutive failures keep the banner without re-signalling"
        );
    }

    #[test]
    fn success_fills_rows_with_core_display_strings() {
        archel_core::i18n::set_language("fr");
        let mut model = BtModelRust::default();
        let _ = model.apply_outcome(failed_outcome(), None);
        let applied = model.apply_outcome(Ok(vec![known_device()]), Some(true));
        assert!(!applied.banner_visible, "success hides the banner");
        assert!(applied.adapter_on, "the adapter read publishes");
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(&applied.devices_json).expect("devices must be JSON");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], "Casque Salon");
        assert_eq!(rows[0]["address"], "AA:BB:CC:DD:EE:FF");
        assert!(!rows[0]["state"].as_str().unwrap_or("").is_empty());
        assert_eq!(rows[0]["signal"], "-42 dBm");
        assert_eq!(rows[0]["paired"], true);
        assert_eq!(rows[0]["connected"], false);
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

    // -- Write-path choreography (plan 06-07, Task 3) --
    //
    // The fakes below replace only the `BtWriteOps` seams; the
    // workers themselves run for real against a local ModalSession,
    // so resolve-before-confirm, default-No, the verbatim questions,
    // the pairing code round-trip and the decline-unpairs rule are
    // all proven without touching BlueZ.

    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::time::{Duration, Instant};

    use crate::gui::modal::{ModalKind, ModalSession};

    fn wait_until(what: &str, cond: impl Fn() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if cond() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("condition never held within 5s: {what}");
    }

    fn known_devices() -> Vec<BtDevice> {
        vec![known_device()]
    }

    fn transport_refused(
        device: &BtDevice,
    ) -> Result<(), archel_core::ArchelError> {
        Err(archel_core::ArchelError::Internal(format!(
            "transport must never run here ({})",
            device.address
        )))
    }

    /// Fake BlueZ handshake: shows the canonical code like the real
    /// agent would, propagates the modal answer, and maps a decline
    /// to the core pairing-rejected shape.
    fn fake_handshake(
        device: &BtDevice,
        answer: Box<PairAnswer>,
    ) -> Result<(), archel_core::ArchelError> {
        let code = archel_core::bluetooth::format_passkey(42);
        if answer(code) {
            Ok(())
        } else {
            Err(archel_core::ArchelError::pairing_rejected(device.address.clone()))
        }
    }

    #[test]
    fn pair_accept_shows_the_canonical_code_and_propagates_yes() {
        use std::sync::Mutex;
        static UNPAIR_CALLS: AtomicUsize = AtomicUsize::new(0);
        static SHOWN: Mutex<String> = Mutex::new(String::new());
        fn fake_pair(
            device: &BtDevice,
            answer: Box<PairAnswer>,
        ) -> Result<(), archel_core::ArchelError> {
            let code = archel_core::bluetooth::format_passkey(42);
            *SHOWN.lock().expect("shown lock") = code.clone();
            if answer(code) {
                Ok(())
            } else {
                Err(archel_core::ArchelError::pairing_rejected(
                    device.address.clone(),
                ))
            }
        }
        fn unpair_must_not_run(
            device: &BtDevice,
        ) -> Result<(), archel_core::ArchelError> {
            UNPAIR_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            transport_refused(device)
        }
        let ops = BtWriteOps {
            pair: fake_pair,
            unpair: unpair_must_not_run,
            connect: transport_refused,
            disconnect: transport_refused,
            set_adapter: |_| {
                panic!("adapter toggle must never run during a pair");
            },
        };
        let sess: &'static ModalSession = Box::leak(Box::new(ModalSession::default()));
        std::thread::scope(|s| {
            s.spawn(|| pair_worker(&known_devices(), "AA:BB:CC:DD:EE:FF", &ops, sess));
            wait_until("pair intent opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            sess.resolve_confirm(true);
            wait_until("pairing modal opens", || sess.snapshot_pairing_active());
            assert_eq!(
                sess.snapshot_pairing_passkey(),
                archel_core::bluetooth::format_passkey(42),
                "the displayed code is the core-formatted passkey"
            );
            assert_eq!(sess.snapshot_pairing_passkey(), "000042");
            assert_eq!(sess.snapshot_pairing_mode(), "yesno");
            assert_eq!(
                sess.snapshot_pairing_address(),
                "AA:BB:CC:DD:EE:FF"
            );
            sess.resolve_pairing(true);
        });
        assert_eq!(
            SHOWN.lock().expect("shown lock").as_str(),
            "000042",
            "the yes answer propagated through the agent callback"
        );
        assert_eq!(UNPAIR_CALLS.load(AtomicOrdering::SeqCst), 0);
        assert_eq!(sess.snapshot_kind(), ModalKind::None);
        assert!(!sess.snapshot_pairing_active());
    }

    #[test]
    fn pair_decline_at_the_code_unpairs_and_reports_nothing() {
        use std::sync::Mutex;
        static UNPAIR_CALLS: AtomicUsize = AtomicUsize::new(0);
        static UNPAIRED: Mutex<String> = Mutex::new(String::new());
        fn fake_pair(
            device: &BtDevice,
            answer: Box<PairAnswer>,
        ) -> Result<(), archel_core::ArchelError> {
            fake_handshake(device, answer)
        }
        fn unpair_records(
            device: &BtDevice,
        ) -> Result<(), archel_core::ArchelError> {
            UNPAIR_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            *UNPAIRED.lock().expect("unpaired lock") = device.address.clone();
            Ok(())
        }
        let ops = BtWriteOps {
            pair: fake_pair,
            unpair: unpair_records,
            connect: transport_refused,
            disconnect: transport_refused,
            set_adapter: |_| {
                panic!("adapter toggle must never run during a pair");
            },
        };
        let sess: &'static ModalSession = Box::leak(Box::new(ModalSession::default()));
        std::thread::scope(|s| {
            s.spawn(|| pair_worker(&known_devices(), "AA:BB:CC:DD:EE:FF", &ops, sess));
            wait_until("pair intent opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            sess.resolve_confirm(true);
            wait_until("pairing modal opens", || sess.snapshot_pairing_active());
            sess.resolve_pairing(false);
        });
        assert_eq!(UNPAIR_CALLS.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(
            UNPAIRED.lock().expect("unpaired lock").as_str(),
            "AA:BB:CC:DD:EE:FF",
            "the declined device loses any half-bond"
        );
        assert_eq!(
            sess.snapshot_kind(),
            ModalKind::None,
            "a decline is silent, never an error"
        );
    }

    #[test]
    fn unknown_pick_is_rejected_before_any_confirm() {
        static PAIR_CALLS: AtomicUsize = AtomicUsize::new(0);
        fn pair_must_not_run(
            _device: &BtDevice,
            _answer: Box<PairAnswer>,
        ) -> Result<(), archel_core::ArchelError> {
            PAIR_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(())
        }
        let ops = BtWriteOps {
            pair: pair_must_not_run,
            unpair: transport_refused,
            connect: transport_refused,
            disconnect: transport_refused,
            set_adapter: |_| Ok(true),
        };
        let sess: &'static ModalSession = Box::leak(Box::new(ModalSession::default()));
        pair_worker(&known_devices(), "11:22:33:44:55:66", &ops, sess);
        assert_eq!(sess.snapshot_kind(), ModalKind::Error);
        assert_eq!(
            sess.snapshot_error(),
            archel_core::ArchelError::device_not_found("11:22:33:44:55:66").message()
        );
        assert_eq!(PAIR_CALLS.load(AtomicOrdering::SeqCst), 0);
        assert!(!sess.snapshot_pairing_active());
    }

    #[test]
    fn connect_decline_never_reaches_the_transport() {
        static CONNECT_CALLS: AtomicUsize = AtomicUsize::new(0);
        fn connect_must_not_run(
            _device: &BtDevice,
        ) -> Result<(), archel_core::ArchelError> {
            CONNECT_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(())
        }
        let ops = BtWriteOps {
            pair: |_, _| Ok(()),
            unpair: transport_refused,
            connect: connect_must_not_run,
            disconnect: transport_refused,
            set_adapter: |_| Ok(true),
        };
        let sess = ModalSession::default();
        std::thread::scope(|s| {
            s.spawn(|| {
                connect_worker(&known_devices(), "AA:BB:CC:DD:EE:FF", &ops, &sess)
            });
            wait_until("connect confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            assert_eq!(
                sess.snapshot_question(),
                archel_core::bluetooth::connect_confirm_prompt(&known_device())
            );
            sess.resolve_confirm(false);
        });
        assert_eq!(CONNECT_CALLS.load(AtomicOrdering::SeqCst), 0);
    }

    #[test]
    fn adapter_accept_applies_and_reads_back() {
        use std::sync::Mutex;
        static SET_CALLS: Mutex<Vec<bool>> = Mutex::new(Vec::new());
        fn set_records(on: bool) -> Result<bool, archel_core::ArchelError> {
            SET_CALLS.lock().expect("set lock").push(on);
            Ok(on)
        }
        let ops = BtWriteOps {
            pair: |_, _| Ok(()),
            unpair: transport_refused,
            connect: transport_refused,
            disconnect: transport_refused,
            set_adapter: set_records,
        };
        let sess = ModalSession::default();
        std::thread::scope(|s| {
            s.spawn(|| adapter_worker(true, &ops, &sess));
            wait_until("adapter confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            assert_eq!(
                sess.snapshot_question(),
                archel_core::bluetooth::adapter_confirm_prompt(true)
            );
            sess.resolve_confirm(true);
            wait_until("adapter set runs", || {
                !SET_CALLS.lock().expect("set lock").is_empty()
            });
        });
        assert_eq!(*SET_CALLS.lock().expect("set lock"), vec![true]);
    }
}
