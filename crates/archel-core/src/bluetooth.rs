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

use futures_util::StreamExt;
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

/// Ceiling for one pairing handshake (`bt pair`, BT-02): past this
/// the attempt is abandoned as [`ArchelError::PairingTimeout`] —
/// a silent peer must never hang the CLI.
pub const PAIR_TIMEOUT: Duration = Duration::from_secs(30);

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

/// Resolves a `pair/connect/disconnect` pick against the just-printed
/// scanned set (D-06 scan-then-pick, T-4-05): a 1-based list index or
/// an in-list MAC (case-insensitive, trimmed) resolves; a free-form
/// MAC never seen in the scan — or any other input, including a bare
/// device name — fails as [`ArchelError::DeviceNotFound`]. There is
/// no path from a raw MAC string to a mutation without membership
/// in the scanned set.
pub fn resolve_pick(
    scanned: &[BtDevice],
    pick: &str,
) -> Result<BtDevice, ArchelError> {
    let trimmed = pick.trim();
    if let Ok(index) = trimmed.parse::<usize>()
        && index >= 1
        && index <= scanned.len()
    {
        return Ok(scanned[index - 1].clone());
    }
    let upper = trimmed.to_uppercase();
    if let Some(device) = scanned
        .iter()
        .find(|row| row.address.to_uppercase() == upper)
    {
        return Ok(device.clone());
    }
    Err(ArchelError::device_not_found(pick))
}

/// Renders a BlueZ passkey for user confirmation: always 6 digits,
/// zero-padded (the 2.1+harmony note in the bluer spike — a bare
/// `42` would mismatch the peer's `000042`). Display-only: the
/// value never enters logs or snapshots (T-4-06).
pub fn format_passkey(passkey: u32) -> String {
    format!("{passkey:06}")
}

/// Constant-shape passkey compare (T-4-06): every call walks all
/// four octets and accumulates the difference, so the comparison
/// shape never depends on where two codes differ. Used to verify
/// the displayed code round-trips exactly before it is shown.
pub fn passkeys_equal(expected: u32, observed: u32) -> bool {
    let mut diff = 0u32;
    for shift in [0, 8, 16, 24] {
        diff |= ((expected >> shift) & 0xFF) ^ ((observed >> shift) & 0xFF);
    }
    diff == 0
}

/// Shape gate for a BlueZ-supplied code: 6-digit range AND an exact
/// format round-trip (catches truncation/padding bugs before the
/// code reaches the user — showing `42` for `000042` would fail
/// the peer compare). Rejects malformed codes as `false` so the
/// live agent handler can decline instead of prompting.
pub fn check_passkey_shape(passkey: u32) -> bool {
    passkey <= 999_999
        && format_passkey(passkey).parse::<u32>().is_ok_and(|back| passkeys_equal(passkey, back))
}

/// Validates the pick against the scanned set, then runs the
/// injectable confirm gate BEFORE the pair step: a declined (or
/// non-TTY) confirm returns `ConfirmationAborted`/`NeedsTty` with
/// the pair closure never invoked — the abort path is unit-covered
/// headless, spawns nothing, and has no auto-accept fallback
/// (T-4-05/T-4-06). Live callers pass [`crate::packages::confirm_action`]
/// as `confirm`; tests inject literals.
pub async fn pair_picked_with<F, P, Fut>(
    scanned: &[BtDevice],
    pick: &str,
    confirm: F,
    pair: P,
) -> Result<BtDevice, ArchelError>
where
    F: FnOnce(&str) -> Result<(), ArchelError>,
    P: FnOnce(BtDevice) -> Fut,
    Fut: std::future::Future<Output = Result<(), ArchelError>>,
{
    let device = resolve_pick(scanned, pick)?;
    confirm(&pair_confirm_prompt(&device))?;
    pair(device.clone()).await?;
    Ok(device)
}

/// Timed discovery sweep (`bt scan`, BT-01): probes the daemon
/// first, then collects `DeviceAdded` sightings for [`SCAN_TIMEOUT`]
/// into the merged live list (dedupe by MAC, stable signal order).
/// Empty air yields an empty vec — the honest-empty table.
pub async fn bt_scan() -> Result<Vec<BtDevice>, ArchelError> {
    ensure_daemon(daemon_present().await)?;
    let session = bluer::Session::new().await.map_err(|err| {
        ArchelError::bluetooth_unavailable(format!("bluer session: {err}"))
    })?;
    let adapter = session.default_adapter().await.map_err(|err| {
        ArchelError::bluetooth_unavailable(format!("no default adapter: {err}"))
    })?;
    let mut stream = adapter.discover_devices().await.map_err(|err| {
        ArchelError::bluetooth_unavailable(format!("discovery start: {err}"))
    })?;
    let deadline = tokio::time::Instant::now() + SCAN_TIMEOUT;
    let mut sightings = Vec::new();
    loop {
        let remaining =
            deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, stream.next()).await {
            Ok(Some(bluer::AdapterEvent::DeviceAdded(address))) => {
                let mac = address.to_string();
                let (name, rssi) = match adapter.device(address) {
                    Ok(device) => (
                        device
                            .alias()
                            .await
                            .ok()
                            .filter(|alias| !alias.is_empty()),
                        device.rssi().await.ok().flatten(),
                    ),
                    Err(_) => (None, None),
                };
                sightings.push(ScannedEvent::new(mac, name, rssi));
            }
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => break,
        }
    }
    Ok(merge_scan_events(sightings))
}

/// Pairs an already scan-picked device (BT-02): registers a
/// confirm-only agent — `request_confirmation` surfaces the BlueZ
/// code with an explicit `[o/N]` prompt and maps decline/no-TTY to
/// `Rejected`; every other handler stays `None` so those requests
/// reject (never a default-`Agent` auto-accept constructor, T-4-05).
/// The agent is per-application (bluer routes our own `pair()` to
/// it), `request_default` stays `false`, and the handle drops on
/// return. Past [`PAIR_TIMEOUT`] the attempt is abandoned as
/// `PairingTimeout`; any other bluer failure is `PairingRejected`.
/// The code is displayed for confirm only — never logged (T-4-06).
pub async fn pair_device(device: &BtDevice) -> Result<(), ArchelError> {
    let name = device.name.clone();
    let address = device.address.clone();
    pair_device_with(device, move |code| {
        let prompt = pair_code_prompt(&name, &address, &code);
        async move {
            tokio::task::spawn_blocking(move || crate::packages::confirm_action(&prompt))
                .await
                .is_ok_and(|answer| answer.is_ok())
        }
    })
    .await
}

/// Confirm-injected pairing: the same confirm-only agent body with
/// the caller's code gate. `on_code` receives the zero-padded
/// display code ([`format_passkey`]) and resolves `true` (accept) or
/// `false` (decline); anything but an explicit accept rejects, and a
/// callback that outlives [`PAIR_TIMEOUT`] rejects too. The GUI
/// passes a closure that publishes the code to its global pairing
/// modal and awaits the Confirmer/Refuser answer; the CLI passes the
/// terminal prompt (see [`pair_device`]). `on_code` runs at most
/// once per pairing - a second BlueZ request rejects.
pub async fn pair_device_with<F, Fut>(device: &BtDevice, on_code: F) -> Result<(), ArchelError>
where
    F: FnOnce(String) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = bool> + Send,
{
    ensure_daemon(daemon_present().await)?;
    let label = format!("{} ({})", device.name, device.address);
    let address: bluer::Address = device
        .address
        .parse()
        .map_err(|_| ArchelError::device_not_found(device.address.clone()))?;
    let session = bluer::Session::new().await.map_err(|err| {
        ArchelError::bluetooth_unavailable(format!("bluer session: {err}"))
    })?;
    let adapter = session.default_adapter().await.map_err(|err| {
        ArchelError::bluetooth_unavailable(format!("no default adapter: {err}"))
    })?;
    let peer = adapter
        .device(address)
        .map_err(|_| ArchelError::device_not_found(device.address.clone()))?;
    let slot = std::sync::Arc::new(std::sync::Mutex::new(Some(on_code)));
    let agent = bluer::agent::Agent {
        request_confirmation: Some(Box::new(move |req| {
            let slot = slot.clone();
            Box::pin(async move {
                if !check_passkey_shape(req.passkey) {
                    return Err(bluer::agent::ReqError::Rejected);
                }
                // Display for confirm only: the code travels to the
                // caller, never to logs or error payloads.
                let code = format_passkey(req.passkey);
                let callback = slot.lock().ok().and_then(|mut guard| guard.take());
                let accepted = match callback {
                    Some(callback) => {
                        tokio::time::timeout(PAIR_TIMEOUT, callback(code))
                            .await
                            .unwrap_or(false)
                    }
                    None => false,
                };
                if accepted {
                    Ok(())
                } else {
                    Err(bluer::agent::ReqError::Rejected)
                }
            })
                as std::pin::Pin<
                    Box<dyn std::future::Future<Output = bluer::agent::ReqResult<()>> + Send>,
                >
        })),
        ..bluer::agent::Agent::default()
    };
    let _agent = session.register_agent(agent).await.map_err(|err| {
        ArchelError::pairing_rejected(format!("{label}: agent: {err}"))
    })?;
    match tokio::time::timeout(PAIR_TIMEOUT, peer.pair()).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(err)) => Err(ArchelError::pairing_rejected(format!("{label}: {err}"))),
        Err(_) => Err(ArchelError::pairing_timeout(label)),
    }
}

/// Removes an existing bond (BT-03): daemon probe first, then the
/// adapter's `RemoveDevice`. Used after a declined pairing code to
/// leave no half-bond behind, and by the GUI's Supprimer action.
/// Bluer failures surface as `Internal` with log-only detail (pairing
/// has the dedicated typed variants; unpair is a transport call).
pub async fn unpair_device(device: &BtDevice) -> Result<(), ArchelError> {
    ensure_daemon(daemon_present().await)?;
    let address: bluer::Address = device
        .address
        .parse()
        .map_err(|_| ArchelError::device_not_found(device.address.clone()))?;
    let session = bluer::Session::new().await.map_err(|err| {
        ArchelError::bluetooth_unavailable(format!("bluer session: {err}"))
    })?;
    let adapter = session.default_adapter().await.map_err(|err| {
        ArchelError::bluetooth_unavailable(format!("no default adapter: {err}"))
    })?;
    adapter.remove_device(address).await.map_err(|err| {
        ArchelError::Internal(format!(
            "bluetooth unpair {} ({}): {err}",
            device.name, device.address
        ))
    })
}

/// Reads the default adapter's `Powered` property (BT-04): the GUI
/// toggle's read side. Daemon and adapter probes first, like every
/// other transport call.
pub async fn adapter_powered() -> Result<bool, ArchelError> {
    ensure_daemon(daemon_present().await)?;
    let session = bluer::Session::new().await.map_err(|err| {
        ArchelError::bluetooth_unavailable(format!("bluer session: {err}"))
    })?;
    let adapter = session.default_adapter().await.map_err(|err| {
        ArchelError::bluetooth_unavailable(format!("no default adapter: {err}"))
    })?;
    adapter.is_powered().await.map_err(|err| {
        ArchelError::Internal(format!("bluetooth adapter powered read: {err}"))
    })
}

/// Sets the default adapter's `Powered` property (BT-04) and reads it
/// back: the returned state is the proof, never a silent ok.
pub async fn set_adapter_powered(on: bool) -> Result<bool, ArchelError> {
    ensure_daemon(daemon_present().await)?;
    let session = bluer::Session::new().await.map_err(|err| {
        ArchelError::bluetooth_unavailable(format!("bluer session: {err}"))
    })?;
    let adapter = session.default_adapter().await.map_err(|err| {
        ArchelError::bluetooth_unavailable(format!("no default adapter: {err}"))
    })?;
    adapter.set_powered(on).await.map_err(|err| {
        ArchelError::Internal(format!("bluetooth adapter set powered: {err}"))
    })?;
    adapter.is_powered().await.map_err(|err| {
        ArchelError::Internal(format!("bluetooth adapter powered read-back: {err}"))
    })
}

/// Connects an already scan-picked device (BT-02): daemon probe
/// first, then the bluer connect. Bluer failures surface as
/// `Internal` with log-only detail (pairing has the dedicated
/// typed variants; connect is a transport call).
pub async fn connect_device(device: &BtDevice) -> Result<(), ArchelError> {
    ensure_daemon(daemon_present().await)?;
    let address: bluer::Address = device
        .address
        .parse()
        .map_err(|_| ArchelError::device_not_found(device.address.clone()))?;
    let session = bluer::Session::new().await.map_err(|err| {
        ArchelError::bluetooth_unavailable(format!("bluer session: {err}"))
    })?;
    let adapter = session.default_adapter().await.map_err(|err| {
        ArchelError::bluetooth_unavailable(format!("no default adapter: {err}"))
    })?;
    let peer = adapter
        .device(address)
        .map_err(|_| ArchelError::device_not_found(device.address.clone()))?;
    peer.connect().await.map_err(|err| {
        ArchelError::Internal(format!(
            "bluetooth connect {} ({}): {err}",
            device.name, device.address
        ))
    })
}

/// Disconnects an already scan-picked device (BT-02): same probe
/// and lookup shape as [`connect_device`].
pub async fn disconnect_device(device: &BtDevice) -> Result<(), ArchelError> {
    ensure_daemon(daemon_present().await)?;
    let address: bluer::Address = device
        .address
        .parse()
        .map_err(|_| ArchelError::device_not_found(device.address.clone()))?;
    let session = bluer::Session::new().await.map_err(|err| {
        ArchelError::bluetooth_unavailable(format!("bluer session: {err}"))
    })?;
    let adapter = session.default_adapter().await.map_err(|err| {
        ArchelError::bluetooth_unavailable(format!("no default adapter: {err}"))
    })?;
    let peer = adapter
        .device(address)
        .map_err(|_| ArchelError::device_not_found(device.address.clone()))?;
    peer.disconnect().await.map_err(|err| {
        ArchelError::Internal(format!(
            "bluetooth disconnect {} ({}): {err}",
            device.name, device.address
        ))
    })
}

/// French section title for the live scan table, from the catalog.
pub fn scan_title() -> String {
    rust_i18n::t!("bt.scan_title").to_string()
}

/// French honest-empty line for a scan that saw no air traffic.
pub fn scan_empty_line() -> String {
    rust_i18n::t!("bt.scan_empty").to_string()
}

/// French pair-intent prompt naming name plus MAC (`[o/N]`),
/// from the catalog (D-05/D-07).
pub fn pair_confirm_prompt(device: &BtDevice) -> String {
    rust_i18n::t!(
        "bt.pair_confirm",
        name = device.name,
        address = device.address
    )
    .to_string()
}

/// French code-confirm prompt: the BlueZ code plus the peer name
/// plus MAC travel together so the user confirms the RIGHT device
/// (T-4-05 rogue-spoof mitigation). The code is display-only.
pub fn pair_code_prompt(name: &str, address: &str, code: &str) -> String {
    rust_i18n::t!("bt.pair_code", name = name, address = address, code = code)
        .to_string()
}

/// French connect-intent prompt, from the catalog.
pub fn connect_confirm_prompt(device: &BtDevice) -> String {
    rust_i18n::t!(
        "bt.connect_confirm",
        name = device.name,
        address = device.address
    )
    .to_string()
}

/// French disconnect-intent prompt, from the catalog.
pub fn disconnect_confirm_prompt(device: &BtDevice) -> String {
    rust_i18n::t!(
        "bt.disconnect_confirm",
        name = device.name,
        address = device.address
    )
    .to_string()
}

/// French paired-ok line naming the device, from the catalog.
pub fn paired_ok_line(device: &BtDevice) -> String {
    rust_i18n::t!("bt.paired_ok", name = device.name).to_string()
}

/// French connected-ok line naming the device, from the catalog.
pub fn connected_ok_line(device: &BtDevice) -> String {
    rust_i18n::t!("bt.connected_ok", name = device.name).to_string()
}

/// French disconnected-ok line naming the device, from the catalog.
pub fn disconnected_ok_line(device: &BtDevice) -> String {
    rust_i18n::t!("bt.disconnected_ok", name = device.name).to_string()
}

/// French unpair-intent prompt naming name plus MAC (`[o/N]`), from
/// the catalog (D-05/D-07).
pub fn unpair_confirm_prompt(device: &BtDevice) -> String {
    rust_i18n::t!(
        "bt.unpair_confirm",
        name = device.name,
        address = device.address
    )
    .to_string()
}

/// French adapter-toggle prompt (`[o/N]`), from the catalog.
pub fn adapter_confirm_prompt(on: bool) -> String {
    if on {
        rust_i18n::t!("bt.adapter_on_confirm").to_string()
    } else {
        rust_i18n::t!("bt.adapter_off_confirm").to_string()
    }
}

/// French unpaired-ok line naming the device, from the catalog.
pub fn unpaired_ok_line(device: &BtDevice) -> String {
    rust_i18n::t!("bt.unpaired_ok", name = device.name).to_string()
}

/// French adapter state word (on/off), from the catalog.
pub fn adapter_state_word(on: bool) -> String {
    if on {
        rust_i18n::t!("bt.adapter_on_word").to_string()
    } else {
        rust_i18n::t!("bt.adapter_off_word").to_string()
    }
}

/// French adapter-ok line carrying the read-back state, from the catalog.
pub fn adapter_ok_line(on: bool) -> String {
    rust_i18n::t!("bt.adapter_ok", state = adapter_state_word(on)).to_string()
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

    fn scanned_pair() -> Vec<BtDevice> {
        vec![
            BtDevice {
                address: "AA:BB:CC:DD:EE:FF".to_string(),
                name: "Mouse".to_string(),
                rssi: Some(-50),
                paired: false,
                connected: false,
            },
            BtDevice {
                address: "11:22:33:44:55:66".to_string(),
                name: "Keyboard".to_string(),
                rssi: Some(-70),
                paired: true,
                connected: false,
            },
        ]
    }

    #[test]
    fn pick_validator_accepts_in_list_index_and_mac() {
        let scanned = scanned_pair();
        let by_index = resolve_pick(&scanned, "1")
            .expect("in-list 1-based index must resolve");
        assert_eq!(by_index.address, "AA:BB:CC:DD:EE:FF");
        let by_mac = resolve_pick(&scanned, "11:22:33:44:55:66")
            .expect("in-list MAC must resolve");
        assert_eq!(by_mac.name, "Keyboard");
        let by_mac_lower = resolve_pick(&scanned, "aa:bb:cc:dd:ee:ff")
            .expect("MAC match must be case-insensitive");
        assert_eq!(by_mac_lower.name, "Mouse");
        let padded = resolve_pick(&scanned, "  AA:BB:CC:DD:EE:FF ")
            .expect("surrounding whitespace must not break an in-list MAC");
        assert_eq!(padded.name, "Mouse");
    }

    #[test]
    fn pick_validator_rejects_free_form_mac_and_garbage() {
        let scanned = scanned_pair();
        for pick in ["00:11:22:33:44:55", "3", "0", "", "mouse", "not-a-mac"] {
            let err = resolve_pick(&scanned, pick)
                .expect_err(&format!("pick {pick:?} must be rejected"));
            assert!(
                matches!(err, ArchelError::DeviceNotFound { .. }),
                "wrong variant for {pick:?}: {err:?}"
            );
        }
    }

    #[test]
    fn passkey_renders_zero_padded_and_compares_constant_shape() {
        assert_eq!(format_passkey(42), "000042");
        assert_eq!(format_passkey(999999), "999999");
        assert!(passkeys_equal(123456, 123456));
        assert!(!passkeys_equal(123456, 123457));
        assert!(!passkeys_equal(0, 999999));
    }

    #[tokio::test]
    async fn pair_abort_path_spawns_nothing_without_auto_accept() {
        let scanned = scanned_pair();
        let mut paired = false;
        let err = pair_picked_with(
            &scanned,
            "1",
            |_| Err(ArchelError::ConfirmationAborted),
            |_| {
                paired = true;
                async { Ok(()) }
            },
        )
        .await
        .expect_err("declined confirm must abort the pair");
        assert!(
            matches!(err, ArchelError::ConfirmationAborted),
            "wrong variant: {err:?}"
        );
        assert!(!paired, "abort path must never reach the pair step");
    }

    #[tokio::test]
    async fn pair_confirm_path_reaches_pair_step_once() {
        let scanned = scanned_pair();
        let mut calls = 0;
        let picked = pair_picked_with(
            &scanned,
            "AA:BB:CC:DD:EE:FF",
            |_| Ok(()),
            |device: BtDevice| {
                calls += 1;
                assert_eq!(device.address, "AA:BB:CC:DD:EE:FF");
                async { Ok(()) }
            },
        )
        .await
        .expect("confirmed pick must pair");
        assert_eq!(picked.address, "AA:BB:CC:DD:EE:FF");
        assert_eq!(calls, 1);
    }
}
