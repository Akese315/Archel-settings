//! Bluetooth devices over BlueZ via bluer 0.17 (BT-01/BT-02).
//!
//! Every bt command probes the `org.bluez` bus name FIRST (cheap
//! `NameHasOwner` on the system bus, timeout-capped): absence maps to
//! typed [`crate::ArchelError::BluetoothUnavailable`] with the catalog
//! enable hint — never a panic, never a hang, never a daemon
//! auto-start (service management is out of scope by design; the
//! UpdatesCheckerMissing degrade shape is the precedent, D-06).
//!
//! ## bluer 0.17.4 spike findings (highest-risk surface, no repo analog)
//!
//! Resolved against the vendored `bluer-0.17.4` sources
//! (`~/.cargo/registry/.../bluer-0.17.4/src/`), not training memory:
//!
//! - `Session::new().await` dials the SYSTEM bus synchronously inside
//!   `spawn_blocking` (`connection::new_system_sync`); it fails when
//!   bluetoothd is down, which is why the cheap bus-name probe runs
//!   first — a `Session` is only ever built on a present daemon.
//! - Default adapter: `session.default_adapter().await -> Result<Adapter>`
//!   (or `session.adapter("hci0")` for an explicit name). Known devices:
//!   `adapter.device_addresses().await -> Vec<Address>`, then
//!   `adapter.device(addr) -> Result<Device>` for per-device props.
//! - Discovery is a stream: `adapter.discover_devices().await ->
//!   Result<impl Stream<Item = AdapterEvent>>` with
//!   `AdapterEvent::DeviceAdded(Address)` / `DeviceRemoved(Address)` /
//!   `PropertyChanged(_)`. Per-device rows come from the `Device`
//!   handle: `device.alias().await` (falls back to the remote name),
//!   `device.rssi().await -> Result<Option<i16>>`,
//!   `device.is_paired().await`, `device.is_connected().await`.
//! - `bluer::Address` is `pub struct Address(pub [u8; 6])` rendering as
//!   `XX:XX:XX:XX:XX:XX` via `Display` — the MAC string shown in every
//!   row (rogue-spoof mitigation T-4-05: name + MAC + signal always).
//! - Agent registration: `session.register_agent(agent).await ->
//!   Result<AgentHandle>`; dropping the handle unregisters. `Agent` is
//!   a struct of `Option<...Fn>` callback boxes plus
//!   `request_default: bool`. Any handler left `None` REJECTS that
//!   request (`ReqError::Rejected`); all-`None` advertises
//!   `NoInputNoOutput`, which ACCEPTS everything — so Archel must never
//!   construct a default `Agent` (no auto-accept constructor may exist,
//!   T-4-05). The pairing surface sets `request_confirmation`
//!   (`RequestConfirmation { adapter, device, passkey: u32 }`) so BlueZ
//!   advertises `DisplayYesNo`, and the callback blocks on the CLI
//!   `[o/N]` prompt: `Ok(())` confirms, `Err(ReqError::Rejected)`
//!   declines. `display_passkey` (`DisplayPasskey { passkey: u32 (always
//!   6-digit, zero-pad at render), entered: u16, cancel:
//!   oneshot::Receiver<()> }`) and `display_pin_code` (`DisplayPinCode
//!   { pincode: String (always 6-digit, zero-padded per the 2.1+harmony
//!   note) }`) surface the code for user confirm only — compared
//!   constant-shape, displayed never logged (T-4-06).
//! - Pairing itself: `device.pair().await`; dropping the future cancels
//!   via `CancelPairing`. Connect: `device.connect().await` (or
//!   `adapter.connect_device(addr, addr_type)`); disconnect:
//!   `device.disconnect().await`.
//!
//! Reference D-05/D-06: French rendering lives in the catalog only;
//! failures degrade to typed errors, never default-ok rows.

use std::time::Duration;

use serde::Serialize;

use crate::error::ArchelError;

/// BlueZ well-known bus name on the system bus (D-06: the cheap probe
/// every bt command runs before touching bluer).
pub const BLUEZ_BUS_NAME: &str = "org.bluez";

/// Ceiling for the daemon probe: `bt` commands must fail honestly
/// well under the 15 s hang budget with the daemon down.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// Ceiling for one timed discovery sweep (`bt scan`, BT-01).
pub const SCAN_TIMEOUT: Duration = Duration::from_secs(12);

/// One Bluetooth device row: name plus MAC plus signal are ALWAYS all
/// shown together (T-4-05 rogue-spoof mitigation — a name alone never
/// identifies a pairing peer).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BtDevice {
    /// MAC address (`XX:XX:XX:XX:XX:XX`, from `bluer::Address`).
    pub address: String,
    /// Human alias (falls back to the remote name, then the MAC).
    pub name: String,
    /// Last-seen signal in dBm (`None` when BlueZ reports no RSSI).
    pub rssi: Option<i16>,
    /// Already paired (bonded) with this host.
    pub paired: bool,
    /// Currently connected.
    pub connected: bool,
}

/// One mockable discovery sighting: the streaming layer feeds these
/// (live BlueZ events map to this shape; tests feed literals without
/// hardware). Mirrors the `collect_streaming` generic-reader idea from
/// `privileged.rs`: pure input, pure output, no spawn inside.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedEvent {
    /// MAC address of the sighted device.
    pub address: String,
    /// Advertised alias, when BlueZ has resolved one yet.
    pub name: Option<String>,
    /// Signal in dBm, when BlueZ reports one yet.
    pub rssi: Option<i16>,
}

impl ScannedEvent {
    /// Builds a sighting with every field present (tests + live map).
    pub fn new(
        address: impl Into<String>,
        name: Option<String>,
        rssi: Option<i16>,
    ) -> Self {
        Self {
            address: address.into(),
            name,
            rssi,
        }
    }
}

/// Merges raw sightings into render-ready rows: duplicate MACs dedupe
/// (strongest RSSI wins; a sighting with a name beats a nameless one),
/// then stable-sorts by signal descending (`None` RSSI last) so the
/// live list in `bt scan` orders deterministically at render.
/// Empty input yields an empty vec — the honest-empty table, never a
/// fake row (BT-01 adjacency/empty/ordering edges).
pub fn merge_scan_events(
    events: impl IntoIterator<Item = ScannedEvent>,
) -> Vec<BtDevice> {
    let mut merged: Vec<BtDevice> = Vec::new();
    for event in events {
        match merged.iter_mut().find(|row| row.address == event.address) {
            Some(row) => {
                if let Some(name) = event.name {
                    if row.name.is_empty() || row.name == row.address {
                        row.name = name;
                    }
                }
                match (row.rssi, event.rssi) {
                    (None, Some(rssi)) => row.rssi = Some(rssi),
                    (Some(current), Some(rssi)) if rssi > current => {
                        row.rssi = Some(rssi)
                    }
                    _ => {}
                }
            }
            None => merged.push(BtDevice {
                address: event.address.clone(),
                name: event.name.unwrap_or_else(|| event.address.clone()),
                rssi: event.rssi,
                paired: false,
                connected: false,
            }),
        }
    }
    // Stable sort: equal-signal rows keep first-seen order; unknown
    // signal (`None`) sorts last because `None < Some` in both
    // directions of the reversed compare.
    merged.sort_by(|a, b| b.rssi.cmp(&a.rssi));
    merged
}

/// Pure daemon gate for tests and the live probe alike: `true` passes
/// through, `false` becomes the typed unavailable error with the
/// log-only detail (the user sees the catalog enable hint, T-4-SC:
/// core never starts the service, it only hints).
pub fn ensure_daemon(present: bool) -> Result<(), ArchelError> {
    if present {
        Ok(())
    } else {
        Err(ArchelError::bluetooth_unavailable(format!(
            "bus name {BLUEZ_BUS_NAME} has no owner"
        )))
    }
}

/// Cheap daemon probe: `NameHasOwner org.bluez` on the system bus,
/// timeout-capped so a down daemon degrades in seconds, never hangs.
/// `false` on ANY failure (no bus, no name, timeout) — callers map it
/// through [`ensure_daemon`].
pub async fn daemon_present() -> bool {
    let probe = async {
        let conn = zbus::Connection::system().await.ok()?;
        let proxy = zbus::Proxy::new(
            &conn,
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
        )
        .await
        .ok()?;
        let owned: bool = proxy
            .call("NameHasOwner", &(BLUEZ_BUS_NAME,))
            .await
            .ok()?;
        Some(owned)
    };
    tokio::time::timeout(PROBE_TIMEOUT, probe)
        .await
        .ok()
        .flatten()
        .unwrap_or(false)
}

/// Lists known Bluetooth devices (BT-01 read path behind `bt devices`).
/// Probes the daemon first: down maps to `BluetoothUnavailable`, never
/// a bluer connect hang. Per-device prop failures degrade the row
/// (name falls back to the MAC, RSSI to `None`), never the whole list.
pub async fn bt_devices() -> Result<Vec<BtDevice>, ArchelError> {
    ensure_daemon(daemon_present().await)?;
    let session = bluer::Session::new().await.map_err(|err| {
        ArchelError::bluetooth_unavailable(format!("bluer session: {err}"))
    })?;
    let adapter = session.default_adapter().await.map_err(|err| {
        ArchelError::bluetooth_unavailable(format!("no default adapter: {err}"))
    })?;
    let addresses = adapter.device_addresses().await.map_err(|err| {
        ArchelError::bluetooth_unavailable(format!("device enumeration: {err}"))
    })?;
    let mut rows = Vec::with_capacity(addresses.len());
    for address in addresses {
        let mac = address.to_string();
        let row = match adapter.device(address) {
            Ok(device) => BtDevice {
                address: mac.clone(),
                name: device
                    .alias()
                    .await
                    .ok()
                    .filter(|alias| !alias.is_empty())
                    .unwrap_or_else(|| mac.clone()),
                rssi: device.rssi().await.ok().flatten(),
                paired: device.is_paired().await.unwrap_or(false),
                connected: device.is_connected().await.unwrap_or(false),
            },
            Err(_) => BtDevice {
                address: mac.clone(),
                name: mac.clone(),
                rssi: None,
                paired: false,
                connected: false,
            },
        };
        rows.push(row);
    }
    Ok(rows)
}

/// French section title for the devices table, from the catalog.
pub fn devices_title() -> String {
    rust_i18n::t!("bt.devices_title").to_string()
}

/// French honest-empty line for a daemon-up but device-free store.
pub fn devices_empty_line() -> String {
    rust_i18n::t!("bt.devices_empty").to_string()
}

/// French column labels (name, MAC, signal, state), from the catalog.
pub fn device_columns() -> (String, String, String, String) {
    (
        rust_i18n::t!("bt.label_name").to_string(),
        rust_i18n::t!("bt.label_mac").to_string(),
        rust_i18n::t!("bt.label_signal").to_string(),
        rust_i18n::t!("bt.label_state").to_string(),
    )
}

/// French state cell for a device row: connected wins over paired
/// over known. Lives in core so the binary owns no French copy
/// (D-07) and the `rust_i18n` backend resolves here.
pub fn state_label(device: &BtDevice) -> String {
    if device.connected {
        rust_i18n::t!("bt.state_connected").to_string()
    } else if device.paired {
        rust_i18n::t!("bt.state_paired").to_string()
    } else {
        rust_i18n::t!("bt.state_known").to_string()
    }
}
/// reports no RSSI (never a fake 0 dBm).
/// Human signal cell: `"-42 dBm"` or the catalog N/A when BlueZ
/// reports no RSSI (never a fake 0 dBm).
pub fn signal_cell(rssi: Option<i16>) -> String {
    match rssi {
        Some(dbm) => format!("{dbm} dBm"),
        None => rust_i18n::t!("bt.signal_unknown").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_absent_maps_to_typed_unavailable() {
        let err =
            ensure_daemon(false).expect_err("absent daemon must fail typed");
        assert!(
            matches!(err, ArchelError::BluetoothUnavailable { .. }),
            "wrong variant: {err:?}"
        );
    }

    #[test]
    fn daemon_present_passes_through() {
        assert!(ensure_daemon(true).is_ok());
    }

    #[test]
    fn empty_stream_yields_honest_empty() {
        assert!(merge_scan_events([]).is_empty());
    }

    #[test]
    fn duplicate_macs_dedupe_with_strongest_signal() {
        let rows = merge_scan_events([
            ScannedEvent::new("AA:BB:CC:DD:EE:FF", Some("Mouse".to_string()), Some(-70)),
            ScannedEvent::new("AA:BB:CC:DD:EE:FF", Some("Mouse".to_string()), Some(-50)),
            ScannedEvent::new("11:22:33:44:55:66", None, None),
        ]);
        assert_eq!(rows.len(), 2, "duplicate MAC must collapse: {rows:?}");
        let mouse = rows
            .iter()
            .find(|row| row.address == "AA:BB:CC:DD:EE:FF")
            .expect("mouse row must survive");
        assert_eq!(mouse.rssi, Some(-50));
        let nameless = rows
            .iter()
            .find(|row| row.address == "11:22:33:44:55:66")
            .expect("nameless row must survive");
        assert_eq!(nameless.name, "11:22:33:44:55:66");
        assert_eq!(nameless.rssi, None);
    }

    #[test]
    fn ordering_is_stable_by_signal_at_render() {
        let rows = merge_scan_events([
            ScannedEvent::new("AA:AA:AA:AA:AA:AA", Some("Weak".to_string()), Some(-80)),
            ScannedEvent::new("BB:BB:BB:BB:BB:BB", Some("NoSignal".to_string()), None),
            ScannedEvent::new("CC:CC:CC:CC:CC:CC", Some("Strong".to_string()), Some(-40)),
        ]);
        let order: Vec<&str> =
            rows.iter().map(|row| row.address.as_str()).collect();
        assert_eq!(
            order,
            vec!["CC:CC:CC:CC:CC:CC", "AA:AA:AA:AA:AA:AA", "BB:BB:BB:BB:BB:BB"],
            "strongest first, unknown signal last: {order:?}"
        );
    }

    #[test]
    fn signal_cell_never_fakes_zero() {
        rust_i18n::set_locale("fr");
        assert_eq!(signal_cell(Some(-42)), "-42 dBm");
        let unknown = signal_cell(None);
        assert!(
            !unknown.contains('0'),
            "unknown signal must not render a zero: {unknown}"
        );
    }
}
