//! Network inspection through NetworkManager (NET-01/02 tracer).
//!
//! This module owns the `archel net status` read path: connection state,
//! interface, SSID (Wi-Fi only), IPv4 address, gateway, effective DNS and
//! the backing NM profile. Reads are unprivileged and in-process; no write
//! path lives here (DNS/VPN writes land in later plans).
//!
//! ## Read strategy: D-Bus first, `nmcli` fallback
//!
//! [`net_status`] tries the async zbus path first and falls back to the
//! sync `nmcli -t -f` terse-field parse when D-Bus is unreachable. The
//! `source` field on [`NetStatus`] always says which path served the data
//! (`"dbus"` or `"nmcli"`), so renderers and `--json` consumers can tell
//! provenance. Every `nmcli` spawn reuses the [`crate::packages`] spawn
//! shape (which-gate, `.unchecked()`, `LC_ALL=C`, `wait_timeout` + kill,
//! stderr log-only) — `nmcli` speaks French on this machine, so terse
//! fields plus the `C` locale pin are load-bearing, never human tables.
//!
//! ## Wave 0 live-bus spike findings (2026-10-06, NM 1.58.1, recorded here
//! per plan — every ASSUMED shape below was verified against the live bus
//! before this code was written)
//!
//! - Root object `/org/freedesktop/NetworkManager`, interface
//!   `org.freedesktop.NetworkManager`: `GetDevices` and `GetAllDevices`
//!   both exist; `PrimaryConnection` property points at the default-route
//!   active connection (here
//!   `/org/freedesktop/NetworkManager/ActiveConnection/2`); global `State`
//!   property is `70` (`NM_STATE_CONNECTED_GLOBAL`).
//! - `org.freedesktop.NetworkManager.Connection.Active` properties:
//!   `Id` (s, e.g. `KLEY Rennes priv\u{e9}e`), `Uuid` (s),
//!   `Type` (s, e.g. `802-11-wireless`), `Devices` (ao, one entry),
//!   `Ip4Config` (o), `Default` (b), `State` (u). `Uuid` is the stable
//!   key for later DNS/profile work (D-01 operate-on-UUID rule).
//! - `org.freedesktop.NetworkManager.IP4Config` properties: `AddressData`
//!   (`aa{sv}`, entries carry `"address"` s + `"prefix"` u — preferred
//!   over the deprecated `Addresses`), `Gateway` (s),
//!   `NameserverData` (`aa{sv}`, entries carry `"address"` s — preferred
//!   over the deprecated `Nameservers` `au`), plus `RouteData`. IPv4
//!   integers in the deprecated `au` fields are address bytes in memory
//!   order: `20824074` decodes to `10.192.61.1` (unit-tested in
//!   [`ipv4_from_dbus_u32`]).
//! - Device object `org.freedesktop.NetworkManager.Device`: `Interface`
//!   (s, e.g. `wlan0`), `DeviceType` (u, `2` = Wi-Fi, `1` = Ethernet);
//!   `org.freedesktop.NetworkManager.Device.Wireless.ActiveAccessPoint`
//!   (o, `/` when none); `org.freedesktop.NetworkManager.AccessPoint.Ssid`
//!   (`ay` UTF-8 bytes). Wired devices have no SSID — rendered N/A.
//! - `nmcli` terse fields verified live: `connection show --active`
//!   accepts `-f NAME,UUID,TYPE,DEVICE`; `device show <if>` accepts
//!   `-f IP4.ADDRESS,IP4.GATEWAY,IP4.DNS` (values like
//!   `IP4.ADDRESS[1]:10.192.61.76/24`, `IP4.DNS[1]:10.192.61.1`);
//!   `device show <if> -f GENERAL.STATE` prints `100 (connected)`;
//!   bare `nmcli general status` prints the `STATE CONNECTIVITY …` table
//!   with `connected` in the first column.
//! - Import dry-probes (no mutation, `wg0` untouched): `nmcli connection
//!   import type wireguard file /dev/null` fails cleanly with
//!   `The name of the WireGuard config must be a valid interface name
//!   followed by ".conf"` (plugin path works, arg validation fires
//!   first); `… type openvpn file /dev/null` fails with `failed to load
//!   VPN plugin: unknown VPN plugin "org.freedesktop.NetworkManager.
//!   openvpn"` — the openvpn plugin is NOT installed, so later VPN work
//!   must map this to a typed `VpnPluginMissing` install hint.
//! - Auth probe: `nmcli device wifi rescan` succeeds (rc=0, harmless —
//!   triggers a scan, touches no profile); `GetPermissions` reports
//!   `settings.modify.system: "yes"` on this session, so the exact
//!   polkit-denial string could NOT be captured (no denial observed).
//!   The `PolkitDenied` mapping therefore stays defensive: any D-Bus
//!   access-denied/not-authorized error maps to it with the
//!   start-hyprpolkitagent hint (added with the DNS write plan, which
//!   owns the first real write).
//! - `wg0` (WireGuard `4228f697-…`) was never downed, deleted or
//!   modified: the spike ran reads plus one scan only.
//!
//! ## Threat notes (T-4-01/T-4-02, STRIDE Tampering/EoP)
//!
//! SSID, profile names and DNS strings cross from daemon output into
//! display/JSON: they are shown verbatim, never evaluated, never passed
//! to a shell (exact argv only). No write exists on this path, so no
//! profile-confusion or privilege surface is introduced here.

use std::collections::HashMap;
use std::net::Ipv4Addr;

use serde::Serialize;
use zbus::proxy;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

use crate::error::ArchelError;
use crate::packages::QUERY_TIMEOUT;

/// NM device type for Wi-Fi (`org.freedesktop.NetworkManager.Device`;
/// sibling value `1` = Ethernet — any non-Wi-Fi device skips SSID).
const DEVICE_TYPE_WIFI: u32 = 2;
/// NM global state: fully connected (`NM_STATE_CONNECTED_GLOBAL`).
const NM_STATE_CONNECTED_GLOBAL: u32 = 70;
/// NM global state: connected, site-only.
const NM_STATE_CONNECTED_SITE: u32 = 50;
/// NM global state: connected, link-local only.
const NM_STATE_CONNECTED_LOCAL: u32 = 60;
/// NM device state: activated (`NM_DEVICE_STATE_ACTIVATED`).
const NM_DEVICE_STATE_ACTIVATED: u32 = 100;

/// Connection state plus addressing of the primary NM connection.
///
/// Serialized as-is for `--json` (snake_case English fields, v1 schema):
/// `None` encodes as `null` — an unreachable hop or absent SSID is
/// honest absence, never a zero or placeholder (D-10 precedent).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NetStatus {
    /// Which read path served this snapshot: `"dbus"` or `"nmcli"`.
    pub source: String,
    /// True when the connection is established (global 70 / device 100).
    pub connected: bool,
    /// French one-line state for tables (e.g. `connected`).
    pub state_detail: String,
    /// Kernel interface name (e.g. `wlan0`).
    pub interface: Option<String>,
    /// Wi-Fi SSID; `None` on wired links or when no AP is active.
    pub ssid: Option<String>,
    /// IPv4 address with prefix (e.g. `10.192.61.76/24`).
    pub ipv4_address: Option<String>,
    /// Default gateway (e.g. `10.192.61.1`).
    pub gateway: Option<String>,
    /// Effective DNS servers in NM order.
    pub dns: Vec<String>,
    /// Active profile (connection) human name.
    pub profile_name: Option<String>,
    /// Active profile UUID — the stable key for later write plans.
    pub profile_uuid: Option<String>,
    /// NM connection type (e.g. `802-11-wireless`).
    pub connection_type: Option<String>,
}

#[proxy(
    interface = "org.freedesktop.NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
trait NetworkManager {
    /// Global connectivity state (`NM_STATE_*`, 70 = fully connected).
    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;

    /// Object path of the default-route active connection (`/` if none).
    #[zbus(property)]
    fn primary_connection(&self) -> zbus::Result<OwnedObjectPath>;
}

#[proxy(
    interface = "org.freedesktop.NetworkManager.Connection.Active",
    default_service = "org.freedesktop.NetworkManager"
)]
trait ActiveConnection {
    /// Human profile name (untrusted display data — shown verbatim).
    #[zbus(property)]
    fn id(&self) -> zbus::Result<String>;

    /// Stable profile key (used by later write plans, D-01).
    #[zbus(property)]
    fn uuid(&self) -> zbus::Result<String>;

    /// NM type string (`802-11-wireless`, `802-3-ethernet`, `wireguard`…).
    #[zbus(property, name = "Type")]
    fn connection_type(&self) -> zbus::Result<String>;

    /// Device object paths backing this connection (usually one).
    #[zbus(property)]
    fn devices(&self) -> zbus::Result<Vec<OwnedObjectPath>>;

    /// IPv4 configuration object path (`/` when none).
    #[zbus(property)]
    fn ip4_config(&self) -> zbus::Result<OwnedObjectPath>;
}

#[proxy(
    interface = "org.freedesktop.NetworkManager.IP4Config",
    default_service = "org.freedesktop.NetworkManager"
)]
trait Ip4Config {
    /// `[{address: s, prefix: u, …}]` — preferred address source.
    #[zbus(property)]
    fn address_data(&self) -> zbus::Result<Vec<HashMap<String, OwnedValue>>>;

    /// Default gateway (`""` when none).
    #[zbus(property)]
    fn gateway(&self) -> zbus::Result<String>;

    /// `[{address: s, …}]` — preferred DNS source.
    #[zbus(property)]
    fn nameserver_data(&self) -> zbus::Result<Vec<HashMap<String, OwnedValue>>>;

    /// Deprecated `au` fallback (kept: older NM daemons lack Data props).
    #[zbus(property)]
    fn nameservers(&self) -> zbus::Result<Vec<u32>>;
}

#[proxy(
    interface = "org.freedesktop.NetworkManager.Device",
    default_service = "org.freedesktop.NetworkManager"
)]
trait NmDevice {
    /// Kernel name (`wlan0`, `enp0s31f6`…).
    #[zbus(property)]
    fn interface(&self) -> zbus::Result<String>;

    /// `1` = Ethernet, `2` = Wi-Fi (`NM_DEVICE_TYPE_*`).
    #[zbus(property)]
    fn device_type(&self) -> zbus::Result<u32>;

    /// Device state (`100` = activated).
    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;
}

#[proxy(
    interface = "org.freedesktop.NetworkManager.Device.Wireless",
    default_service = "org.freedesktop.NetworkManager"
)]
trait WirelessDevice {
    /// Active AP path (`/` when scanning/associating).
    #[zbus(property)]
    fn active_access_point(&self) -> zbus::Result<OwnedObjectPath>;
}

#[proxy(
    interface = "org.freedesktop.NetworkManager.AccessPoint",
    default_service = "org.freedesktop.NetworkManager"
)]
trait AccessPoint {
    /// Raw SSID bytes (UTF-8 on modern APs, may be empty when hidden).
    #[zbus(property)]
    fn ssid(&self) -> zbus::Result<Vec<u8>>;
}

/// Reads the primary connection status: D-Bus first, `nmcli` fallback.
///
/// Never panics, never default-oks: both paths exhausted (no bus AND no
/// `nmcli`) surfaces [`ArchelError::NetworkUnavailable`]. Blocking
/// `nmcli` spawns stay in the sync [`net_status_nmcli`] helper and run
/// off the async executor via `spawn_blocking` (Pitfall 6 — no blocking
/// call ever sits directly on the runtime thread).
pub async fn net_status() -> Result<NetStatus, ArchelError> {
    match net_status_dbus().await {
        Ok(status) => Ok(status),
        Err(dbus_err) => {
            let fallback = tokio::task::spawn_blocking(net_status_nmcli).await.map_err(|err| {
                ArchelError::Internal(format!("net status fallback task failed: {err}"))
            })?;
            match fallback {
                Ok(status) => Ok(status),
                Err(_) => Err(dbus_err),
            }
        }
    }
}

/// D-Bus read of the primary connection (preferred path).
async fn net_status_dbus() -> Result<NetStatus, ArchelError> {
    let conn = zbus::Connection::system()
        .await
        .map_err(|err| ArchelError::network_unavailable(format!("D-Bus system bus: {err}")))?;
    let nm = NetworkManagerProxy::new(&conn)
        .await
        .map_err(|err| ArchelError::network_unavailable(format!("NetworkManager proxy: {err}")))?;
    let global_state = nm
        .state()
        .await
        .map_err(|err| ArchelError::network_unavailable(format!("NM State: {err}")))?;
    let primary_path = nm.primary_connection().await.map_err(|err| {
        ArchelError::network_unavailable(format!("NM PrimaryConnection: {err}"))
    })?;
    if is_null_path(&primary_path) {
        return Ok(NetStatus {
            source: "dbus".to_string(),
            connected: false,
            state_detail: describe_nm_state(global_state),
            interface: None,
            ssid: None,
            ipv4_address: None,
            gateway: None,
            dns: Vec::new(),
            profile_name: None,
            profile_uuid: None,
            connection_type: None,
        });
    }

    let active = ActiveConnectionProxy::builder(&conn)
        .path(primary_path)
        .map_err(|err| ArchelError::network_unavailable(format!("active path: {err}")))?
        .build()
        .await
        .map_err(|err| ArchelError::network_unavailable(format!("active proxy: {err}")))?;
    let profile_name = active.id().await.ok();
    let profile_uuid = active.uuid().await.ok();
    let connection_type = active.connection_type().await.ok();
    let device_paths = active.devices().await.unwrap_or_default();
    let ip4_path = active.ip4_config().await.ok();

    let (ipv4_address, gateway, dns) = match ip4_path {
        Some(path) if !is_null_path(&path) => read_ip4_config(&conn, &path).await,
        _ => (None, None, Vec::new()),
    };

    let (interface, ssid) = match device_paths.first() {
        Some(path) => read_device(&conn, path).await,
        None => (None, None),
    };

    Ok(NetStatus {
        source: "dbus".to_string(),
        connected: global_state >= NM_STATE_CONNECTED_SITE,
        state_detail: describe_nm_state(global_state),
        interface,
        ssid,
        ipv4_address,
        gateway,
        dns,
        profile_name,
        profile_uuid,
        connection_type,
    })
}

/// Reads address/gateway/DNS from one `IP4Config` object path.
///
/// `AddressData`/`NameserverData` win; the deprecated `au`
/// `Nameservers` only fill the gap when `NameserverData` is empty.
/// Partial property failures degrade to `None`/empty (honest N/A
/// downstream), never to a whole-status error.
async fn read_ip4_config(
    conn: &zbus::Connection,
    path: &OwnedObjectPath,
) -> (Option<String>, Option<String>, Vec<String>) {
    let Ok(builder) = Ip4ConfigProxy::builder(conn).path(path.clone()) else {
        return (None, None, Vec::new());
    };
    let proxy = match builder.build().await {
        Ok(proxy) => proxy,
        Err(_) => return (None, None, Vec::new()),
    };
    let address = proxy
        .address_data()
        .await
        .ok()
        .and_then(|rows| {
            rows.first().and_then(|row| {
                let addr = row.get("address").and_then(owned_string)?;
                let prefix = row.get("prefix").and_then(owned_u32)?;
                Some(format!("{addr}/{prefix}"))
            })
        });
    let gateway = proxy
        .gateway()
        .await
        .ok()
        .and_then(|gw| if gw.is_empty() { None } else { Some(gw) });
    let mut dns: Vec<String> = proxy
        .nameserver_data()
        .await
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.get("address").and_then(owned_string))
                .collect()
        })
        .unwrap_or_default();
    if dns.is_empty() {
        let legacy = proxy.nameservers().await.unwrap_or_default();
        dns = legacy.iter().map(|raw| ipv4_from_dbus_u32(*raw)).collect();
    }
    (address, gateway, dns)
}

/// Reads interface name plus Wi-Fi SSID from one device object path.
///
/// SSID resolution is best-effort: any hop (wireless iface, AP path,
/// SSID bytes) may be absent while associating — absence yields `None`,
/// never an error.
async fn read_device(
    conn: &zbus::Connection,
    path: &OwnedObjectPath,
) -> (Option<String>, Option<String>) {
    let Ok(builder) = NmDeviceProxy::builder(conn).path(path.clone()) else {
        return (None, None);
    };
    let dev = match builder.build().await {
        Ok(dev) => dev,
        Err(_) => return (None, None),
    };
    let interface = dev.interface().await.ok();
    let is_wifi = dev.device_type().await.ok() == Some(DEVICE_TYPE_WIFI);
    if !is_wifi {
        return (interface, None);
    }
    // SSID needs a second async hop (AccessPoint proxy); resolve it with
    // a dedicated helper so the borrow above stays simple.
    let ssid = read_ap_ssid(conn, path).await;
    (interface, ssid)
}

/// Resolves the active-AP SSID for a wireless device path.
async fn read_ap_ssid(conn: &zbus::Connection, device_path: &OwnedObjectPath) -> Option<String> {
    let Ok(wireless_builder) = WirelessDeviceProxy::builder(conn).path(device_path.clone()) else {
        return None;
    };
    let wireless = wireless_builder.build().await.ok()?;
    let ap_path = wireless.active_access_point().await.ok()?;
    if is_null_path(&ap_path) {
        return None;
    }
    let Ok(ap_builder) = AccessPointProxy::builder(conn).path(ap_path) else {
        return None;
    };
    let ap = ap_builder.build().await.ok()?;
    ssid_from_bytes(ap.ssid().await.ok().unwrap_or_default())
}

/// `/` is NM's null object path (no primary connection, no AP, no config).
fn is_null_path(path: &OwnedObjectPath) -> bool {
    path.as_str() == "/"
}

/// Extracts a string from a D-Bus variant value.
fn owned_string(value: &OwnedValue) -> Option<String> {
    String::try_from(value.clone()).ok()
}

/// Extracts a `u32` from a D-Bus variant value.
fn owned_u32(value: &OwnedValue) -> Option<u32> {
    u32::try_from(value.clone()).ok()
}

/// Decodes a D-Bus `au` IPv4 integer to dotted notation.
///
/// NM stores these as address bytes in memory order (verified live:
/// `20824074` → `10.192.61.1`), hence `to_le_bytes`, not `to_be_bytes`.
pub fn ipv4_from_dbus_u32(raw: u32) -> String {
    Ipv4Addr::from(raw.to_le_bytes()).to_string()
}

/// French rendering of an NM global `State` code.
///
/// 70 (global), 60 (local) and 50 (site) all count as connected —
/// anything below is a genuine disconnect, never a guessed middle.
pub fn describe_nm_state(state: u32) -> String {
    if state >= NM_STATE_CONNECTED_SITE {
        match state {
            NM_STATE_CONNECTED_GLOBAL => rust_i18n::t!("net.state_connected").to_string(),
            NM_STATE_CONNECTED_LOCAL => {
                rust_i18n::t!("net.state_connected_local").to_string()
            }
            _ => rust_i18n::t!("net.state_connected_site").to_string(),
        }
    } else {
        rust_i18n::t!("net.state_disconnected").to_string()
    }
}

/// French rendering of an `nmcli -t -f STATE general` word.
pub fn describe_general_state(word: &str) -> String {
    match word.trim() {
        "connected" | "connect\u{e9}" | "connect\u{e9}e" => {
            rust_i18n::t!("net.state_connected").to_string()
        }
        _ => rust_i18n::t!("net.state_disconnected").to_string(),
    }
}

/// Converts raw SSID bytes to display text.
///
/// Empty (hidden AP with no broadcast name captured) is honest `None`,
/// lossy conversion keeps undecodable bytes visible instead of failing.
pub fn ssid_from_bytes(raw: Vec<u8>) -> Option<String> {
    if raw.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(&raw).into_owned())
}

/// Picks the status connection from `nmcli -t -f NAME,UUID,TYPE,DEVICE`
/// output: first non-loopback line wins.
///
/// Malformed lines (fewer than 4 fields) and blank lines are skipped;
/// loopback is never the status connection. Field order follows the
/// explicit `-f` list, never the human table.
pub fn parse_active_connections(stdout: &str) -> Option<(String, String, String, String)> {
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut fields = line.splitn(4, ':');
        let (Some(name), Some(uuid), Some(ctype), Some(device)) = (
            fields.next(),
            fields.next(),
            fields.next(),
            fields.next(),
        ) else {
            continue;
        };
        if ctype == "loopback" || device.is_empty() {
            continue;
        }
        return Some((
            name.to_string(),
            uuid.to_string(),
            ctype.to_string(),
            device.to_string(),
        ));
    }
    None
}

/// Parsed `nmcli -t -f IP4.ADDRESS,IP4.GATEWAY,IP4.DNS device show` output.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct NmcliIp4 {
    /// First `IP4.ADDRESS[n]` value (`--` means none).
    pub address: Option<String>,
    /// `IP4.GATEWAY` value (`--` means none).
    pub gateway: Option<String>,
    /// Every `IP4.DNS[n]` value in order.
    pub dns: Vec<String>,
}

/// Parses terse `IP4.*` device lines; `--` (nmcli's empty marker) maps
/// to absence, never to an empty string that renders as a value.
pub fn parse_device_ip4(stdout: &str) -> NmcliIp4 {
    let mut out = NmcliIp4::default();
    for line in stdout.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        if key.starts_with("IP4.ADDRESS") {
            if out.address.is_none() && !value.is_empty() && value != "--" {
                out.address = Some(value.to_string());
            }
        } else if key.starts_with("IP4.GATEWAY") {
            if !value.is_empty() && value != "--" {
                out.gateway = Some(value.to_string());
            }
        } else if key.starts_with("IP4.DNS") {
            if !value.is_empty() && value != "--" {
                out.dns.push(value.to_string());
            }
        }
    }
    out
}

/// Parses `GENERAL.STATE:100 (connected)` into the numeric device state.
pub fn parse_general_state(stdout: &str) -> Option<u32> {
    for line in stdout.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if key.trim() == "GENERAL.STATE" {
            return value.trim().split_whitespace().next()?.parse().ok();
        }
    }
    None
}

/// Picks the active SSID from `nmcli -t -f ACTIVE,SSID device wifi list`
/// output: the `yes:` line wins; the SSID is everything after the first
/// colon (SSIDs may legally contain `:`).
pub fn parse_wifi_ssid(stdout: &str) -> Option<String> {
    for line in stdout.lines() {
        let Some((active, ssid)) = line.split_once(':') else {
            continue;
        };
        if active.trim() == "yes" && !ssid.trim().is_empty() {
            return Some(ssid.trim().to_string());
        }
    }
    None
}

/// Maps an `nmcli` exit code: 0 is data, anything else is a query
/// failure (never default-ok — the caller degrades to the typed error).
pub fn map_nmcli_exit(tool: &str, code: Option<i32>) -> Result<(), ArchelError> {
    match code {
        Some(0) => Ok(()),
        _ => Err(ArchelError::Internal(format!(
            "{tool} query failed with exit code {code:?}"
        ))),
    }
}

/// Runs one parse-bound `nmcli` query and returns its stdout.
///
/// Mirrors the [`crate::packages`] spawn shape exactly: which-gate with
/// the caller-visible typed missing variant, `.unchecked()` so the
/// exit-code map below stays reachable, `LC_ALL=C` (nmcli localizes
/// human output — terse fields stay stable either way), `wait_timeout`
/// + kill, stderr log-only.
fn run_nm_query(tool: &str, args: &[&str]) -> Result<String, ArchelError> {
    let binary = which::which(tool)
        .map_err(|err| ArchelError::network_unavailable(format!("{tool} ({err})")))?;
    let handle = duct::cmd(&binary, args)
        .unchecked()
        .env("LC_ALL", "C")
        .stdout_capture()
        .stderr_capture()
        .start()
        .map_err(|err| ArchelError::Internal(format!("cannot run '{tool}': {err}")))?;
    let output = match handle.wait_timeout(QUERY_TIMEOUT).map_err(|err| {
        ArchelError::Internal(format!("cannot wait on '{tool}': {err}"))
    })? {
        Some(output) => output.clone(),
        None => {
            let _ = handle.kill();
            return Err(ArchelError::Internal(format!(
                "'{tool}' exceeded the query timeout"
            )));
        }
    };
    // Log-only capture of query stderr (kept for diagnostics; the user
    // only ever sees catalog French, never this raw text).
    let _stderr_for_logs = String::from_utf8_lossy(&output.stderr);
    map_nmcli_exit(tool, output.status.code())?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Sync `nmcli` fallback for [`net_status`] (runs off-executor via
/// `spawn_blocking`; see the Pitfall 6 note on [`net_status`]).
///
/// Resolution order: active connections → IP4 device fields → general
/// state → Wi-Fi SSID (wireless profiles only). Any hop may be absent;
/// only a hard spawn failure (missing binary, timeout, non-zero exit)
/// errors — and then only so the caller can surface the original D-Bus
/// error instead.
pub fn net_status_nmcli() -> Result<NetStatus, ArchelError> {
    let conns = run_nm_query("nmcli", &["-t", "-f", "NAME,UUID,TYPE,DEVICE", "connection", "show", "--active"])?;
    let Some((name, uuid, ctype, device)) = parse_active_connections(&conns) else {
        return Err(ArchelError::network_unavailable(
            "no active non-loopback connection".to_string(),
        ));
    };
    let ip4 = parse_device_ip4(&run_nm_query(
        "nmcli",
        &["-t", "-f", "IP4.ADDRESS,IP4.GATEWAY,IP4.DNS", "device", "show", &device],
    )?);
    let general = run_nm_query("nmcli", &["-t", "-f", "STATE", "general"])?;
    let general_word = general.trim().to_string();
    let connected = general_word == "connected" || general_word == "connect\u{e9}";
    let device_state = run_nm_query(
        "nmcli",
        &["-t", "-f", "GENERAL.STATE", "device", "show", &device],
    )
    .ok()
    .and_then(|out| parse_general_state(&out));
    let connected = connected || device_state == Some(NM_DEVICE_STATE_ACTIVATED);
    let ssid = if ctype == "802-11-wireless" {
        run_nm_query(
            "nmcli",
            &["-t", "-f", "ACTIVE,SSID", "device", "wifi", "list", "ifname", &device],
        )
        .ok()
        .and_then(|out| parse_wifi_ssid(&out))
    } else {
        None
    };
    Ok(NetStatus {
        source: "nmcli".to_string(),
        state_detail: describe_general_state(&general_word),
        connected,
        interface: Some(device),
        ssid,
        ipv4_address: ip4.address,
        gateway: ip4.gateway,
        dns: ip4.dns,
        profile_name: Some(name),
        profile_uuid: Some(uuid),
        connection_type: Some(ctype),
    })
}

/// French window title for `archel net status`, from the catalog so the
/// binary owns no French copy (D-05/D-07).
pub fn status_title() -> String {
    rust_i18n::t!("net.status_title").to_string()
}

/// French `(label, value)` rows for the status table.
///
/// Absent values render the shared `sys.unavailable` N/A cell (honest
/// absence, never zero-fill); DNS joins with spaces or N/A when empty.
pub fn status_rows(status: &NetStatus) -> Vec<(String, String)> {
    let na = || rust_i18n::t!("sys.unavailable").to_string();
    let dns = if status.dns.is_empty() {
        na()
    } else {
        status.dns.join(" ")
    };
    let profile = match (&status.profile_name, &status.profile_uuid) {
        (Some(name), Some(uuid)) => format!("{name} ({uuid})"),
        (Some(name), None) => name.clone(),
        (None, Some(uuid)) => uuid.clone(),
        (None, None) => na(),
    };
    vec![
        (rust_i18n::t!("net.label_state").to_string(), status.state_detail.clone()),
        (
            rust_i18n::t!("net.label_interface").to_string(),
            status.interface.clone().unwrap_or_else(na),
        ),
        (
            rust_i18n::t!("net.label_ssid").to_string(),
            status.ssid.clone().unwrap_or_else(na),
        ),
        (
            rust_i18n::t!("net.label_ip").to_string(),
            status.ipv4_address.clone().unwrap_or_else(na),
        ),
        (
            rust_i18n::t!("net.label_gateway").to_string(),
            status.gateway.clone().unwrap_or_else(na),
        ),
        (rust_i18n::t!("net.label_dns").to_string(), dns),
        (rust_i18n::t!("net.label_profile").to_string(), profile),
    ]
}

/// Runs one mutating `nmcli` write and returns its stdout.
///
/// Same spawn discipline as [`run_nm_query`], but a non-zero exit maps
/// through [`map_nm_write_error`] so polkit refusals surface the typed
/// [`ArchelError::PolkitDenied`] agent hint (T-4-02) instead of a bare
/// internal error. Never routes through `archel-privileged`, never runs
/// anything as root — the caller's NM polkit is the only auth path.
fn run_nm_write(tool: &str, args: &[&str], action: &str) -> Result<String, ArchelError> {
    let binary = which::which(tool)
        .map_err(|err| ArchelError::network_unavailable(format!("{tool} ({err})")))?;
    let handle = duct::cmd(&binary, args)
        .unchecked()
        .env("LC_ALL", "C")
        .stdout_capture()
        .stderr_capture()
        .start()
        .map_err(|err| ArchelError::Internal(format!("cannot run '{tool}': {err}")))?;
    let output = match handle.wait_timeout(QUERY_TIMEOUT).map_err(|err| {
        ArchelError::Internal(format!("cannot wait on '{tool}': {err}"))
    })? {
        Some(output) => output.clone(),
        None => {
            let _ = handle.kill();
            return Err(ArchelError::Internal(format!(
                "'{tool}' exceeded the query timeout"
            )));
        }
    };
    // Stderr is evidence here, not log-only: the polkit-denial markers
    // live in it (nmcli prints the D-Bus error text on failure).
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    match output.status.code() {
        Some(0) => Ok(String::from_utf8_lossy(&output.stdout).into_owned()),
        code => Err(map_nm_write_error(action, tool, code, &stderr)),
    }
}

/// Maps an `nmcli` write failure: polkit/authorization refusals become
/// [`ArchelError::PolkitDenied`], everything else stays `Internal`.
///
/// Pure over the captured stderr so the mapping is unit-coverable
/// without touching the live daemon. Markers cover the `LC_ALL=C`
/// English shapes plus the French shapes (defense in depth — a locale
/// leak must never turn a denial into a bare internal error).
pub fn map_nm_write_error(
    action: &str,
    tool: &str,
    code: Option<i32>,
    stderr: &str,
) -> ArchelError {
    const DENIAL_MARKERS: &[&str] = &[
        "notauthorized",
        "not authorized",
        "accessdenied",
        "access denied",
        "insufficient privileg",
        "permission denied",
        "authentification requise",
        "non autoris",
    ];
    let haystack = stderr.to_lowercase();
    if DENIAL_MARKERS.iter().any(|mark| haystack.contains(mark)) {
        ArchelError::PolkitDenied {
            action: action.to_string(),
        }
    } else {
        ArchelError::Internal(format!(
            "{tool} write failed with exit code {code:?}: {stderr_trimmed}",
            stderr_trimmed = stderr.trim()
        ))
    }
}

/// DNS per-profile reads and writes (NET-03, D-01/D-02/D-03).
///
/// Writes go through `nmcli connection modify` with the exact replace
/// (D-01) and reset (D-03) sequences from the research code examples,
/// followed by `connection up` reactivation (Pitfall 5: an update
/// without reapply changes nothing) and an effective-DNS read-back.
/// Every write operates on a profile UUID resolved client-side — the
/// default is the active profile's UUID, `--profile` requires an exact
/// name/UUID match (T-4-01). Only `ipv4.*` keys are ever touched (D-02).
pub mod dns {
    use serde::Serialize;

    use crate::error::ArchelError;

    /// A validated, resolved DNS mutation awaiting confirmation.
    ///
    /// Built by [`plan_dns_set`]/[`plan_dns_reset`] (read-only spawns
    /// only), previewed to the operator, then executed by
    /// [`apply_dns_plan`] after [`crate::packages::confirm_action`].
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct DnsPlan {
        /// Human profile name for the preview line.
        pub profile_name: String,
        /// Profile UUID every `nmcli` argv addresses (never the name).
        pub profile_uuid: String,
        /// Currently configured servers (empty = DHCP).
        pub old_servers: Vec<String>,
        /// `true` for a D-03 reset back to DHCP.
        pub reset: bool,
        /// Validated replacement servers (empty when `reset`).
        pub new_servers: Vec<String>,
    }

    /// What a DNS mutation changed, plus the post-reapply truth.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    pub struct DnsReport {
        pub profile_name: String,
        pub profile_uuid: String,
        pub old_servers: Vec<String>,
        pub new_servers: Vec<String>,
        /// Effective DNS read back after reactivation (`None` when the
        /// read-back itself failed — reported honestly, never guessed).
        pub effective_dns: Option<Vec<String>>,
        /// Whether `connection up` reactivation succeeded.
        pub reactivated: bool,
    }

    /// Configured plus effective DNS for `net dns show`.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    pub struct DnsShow {
        pub profile_name: String,
        pub profile_uuid: String,
        pub configured_servers: Vec<String>,
        /// `ipv4.ignore-auto-dns` — true means custom DNS wins over DHCP.
        pub ignore_auto_dns: bool,
        pub effective_dns: Vec<String>,
    }

    /// Strict IPv4 validator (D-02): dotted quads only.
    ///
    /// Rejects IPv6 (`::1`), hostnames, empty strings, wrong part
    /// counts, non-digit octets, out-of-range values (`999.1.1.1`) and
    /// ambiguous leading-zero octets (`01.2.3.4` — octal confusion,
    /// rejected before any spawn). Runs before any confirmation or
    /// spawn: invalid input never reaches NM.
    pub fn validate_dns_servers(servers: &[String]) -> Result<Vec<String>, ArchelError> {
        if servers.is_empty() {
            return Err(ArchelError::DnsInvalid {
                server: String::new(),
            });
        }
        for server in servers {
            if !is_valid_ipv4(server) {
                return Err(ArchelError::DnsInvalid {
                    server: server.clone(),
                });
            }
        }
        Ok(servers.to_vec())
    }

    /// One dotted quad: exactly 4 dot-separated all-digit octets, no
    /// leading zeros (except a lone `0`), each 0-255.
    fn is_valid_ipv4(server: &str) -> bool {
        let parts: Vec<&str> = server.split('.').collect();
        if parts.len() != 4 {
            return false;
        }
        for part in parts {
            if part.is_empty() || part.len() > 3 || !part.bytes().all(|b| b.is_ascii_digit()) {
                return false;
            }
            if part.len() > 1 && part.starts_with('0') {
                return false;
            }
            if part.parse::<u32>().is_ok_and(|n| n > 255) {
                return false;
            }
        }
        true
    }

    /// D-01 replace argv: `connection modify <uuid> ipv4.dns "<a> <b>"
    /// `ipv4.ignore-auto-dns yes`. Joined with spaces per nmcli list
    /// syntax; callers pass pre-validated servers only.
    pub fn dns_modify_argv(profile_uuid: &str, servers: &[String]) -> Vec<String> {
        vec![
            "connection".to_string(),
            "modify".to_string(),
            profile_uuid.to_string(),
            "ipv4.dns".to_string(),
            servers.join(" "),
            "ipv4.ignore-auto-dns".to_string(),
            "yes".to_string(),
        ]
    }

    /// D-03 reset argv: clears `ipv4.dns` and restores DHCP (`no`).
    pub fn dns_reset_argv(profile_uuid: &str) -> Vec<String> {
        vec![
            "connection".to_string(),
            "modify".to_string(),
            profile_uuid.to_string(),
            "ipv4.dns".to_string(),
            String::new(),
            "ipv4.ignore-auto-dns".to_string(),
            "no".to_string(),
        ]
    }

    /// Reactivation argv (Pitfall 5): applies the profile change now.
    pub fn dns_up_argv(profile_uuid: &str) -> Vec<String> {
        vec![
            "connection".to_string(),
            "up".to_string(),
            profile_uuid.to_string(),
        ]
    }

    /// Parses `nmcli -t -f NAME,UUID connection show` into (name, uuid)
    /// rows. Malformed lines are skipped, never trusted.
    pub fn parse_profiles(stdout: &str) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for line in stdout.lines() {
            let Some((name, uuid)) = line.split_once(':') else {
                continue;
            };
            let (name, uuid) = (name.trim(), uuid.trim());
            if name.is_empty() || uuid.is_empty() {
                continue;
            }
            out.push((name.to_string(), uuid.to_string()));
        }
        out
    }

    /// Resolves a `--profile` query to exactly one (name, uuid) pair.
    ///
    /// UUID match wins (globally unique); otherwise the name must match
    /// exactly one profile — zero or ambiguous matches are
    /// [`ArchelError::ProfileNotFound`], never a guess (T-4-01).
    pub fn resolve_profile_uuid(
        profiles: &[(String, String)],
        query: &str,
    ) -> Result<(String, String), ArchelError> {
        if let Some(found) = profiles.iter().find(|(_, uuid)| uuid == query) {
            return Ok(found.clone());
        }
        let mut named = profiles.iter().filter(|(name, _)| name == query);
        match (named.next(), named.next()) {
            (Some(found), None) => Ok(found.clone()),
            _ => Err(ArchelError::ProfileNotFound {
                query: query.to_string(),
            }),
        }
    }

    /// Parses `nmcli -t -f ipv4.dns,ipv4.ignore-auto-dns connection
    /// show <uuid>`: comma/space-separated servers (`--`/empty = DHCP)
    /// plus the ignore-auto flag.
    pub fn parse_profile_dns(stdout: &str) -> (Vec<String>, bool) {
        let mut servers = Vec::new();
        let mut ignore_auto = false;
        for line in stdout.lines() {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            match key.trim() {
                "ipv4.dns" => {
                    for token in value.split([',', ' ', '\t']) {
                        let token = token.trim();
                        if !token.is_empty() && token != "--" {
                            servers.push(token.to_string());
                        }
                    }
                }
                "ipv4.ignore-auto-dns" => {
                    ignore_auto = value.trim() == "yes";
                }
                _ => {}
            }
        }
        (servers, ignore_auto)
    }

    /// Reads the configured DNS of one profile UUID (read-only spawn).
    pub fn read_profile_dns(profile_uuid: &str) -> Result<(Vec<String>, bool), ArchelError> {
        let out = super::run_nm_query(
            "nmcli",
            &[
                "-t",
                "-f",
                "ipv4.dns,ipv4.ignore-auto-dns",
                "connection",
                "show",
                profile_uuid,
            ],
        )?;
        Ok(parse_profile_dns(&out))
    }

    /// Resolves the target profile: active connection by default (UUID
    /// from the D-Bus-shaped active parse), exact `--profile` match
    /// otherwise. Returns (name, uuid).
    pub fn resolve_target_profile(profile: Option<&str>) -> Result<(String, String), ArchelError> {
        match profile {
            Some(query) => {
                let out = super::run_nm_query(
                    "nmcli",
                    &["-t", "-f", "NAME,UUID", "connection", "show"],
                )?;
                resolve_profile_uuid(&parse_profiles(&out), query)
            }
            None => {
                let out = super::run_nm_query(
                    "nmcli",
                    &[
                        "-t",
                        "-f",
                        "NAME,UUID,TYPE,DEVICE",
                        "connection",
                        "show",
                        "--active",
                    ],
                )?;
                super::parse_active_connections(&out)
                    .map(|(name, uuid, _, _)| (name, uuid))
                    .ok_or_else(|| {
                        ArchelError::network_unavailable(
                            "no active non-loopback connection".to_string(),
                        )
                    })
            }
        }
    }

    /// Plans a D-01 replace: validate, resolve, read old config. Only
    /// read-only spawns run here — the mutation waits for confirmation.
    pub fn plan_dns_set(
        profile: Option<&str>,
        servers: &[String],
    ) -> Result<DnsPlan, ArchelError> {
        let new_servers = validate_dns_servers(servers)?;
        let (profile_name, profile_uuid) = resolve_target_profile(profile)?;
        let (old_servers, _) = read_profile_dns(&profile_uuid)?;
        Ok(DnsPlan {
            profile_name,
            profile_uuid,
            old_servers,
            reset: false,
            new_servers,
        })
    }

    /// Plans a D-03 reset back to DHCP (same read-only shape as set).
    pub fn plan_dns_reset(profile: Option<&str>) -> Result<DnsPlan, ArchelError> {
        let (profile_name, profile_uuid) = resolve_target_profile(profile)?;
        let (old_servers, _) = read_profile_dns(&profile_uuid)?;
        Ok(DnsPlan {
            profile_name,
            profile_uuid,
            old_servers,
            reset: true,
            new_servers: Vec::new(),
        })
    }

    /// Executes a confirmed plan: modify, reactivate, read back
    /// effective DNS. Write failures map to [`ArchelError::PolkitDenied`]
    /// with the agent hint (T-4-02); a failed read-back degrades the
    /// report field to `None` instead of failing the whole mutation.
    pub fn apply_dns_plan(plan: &DnsPlan) -> Result<DnsReport, ArchelError> {
        let action = format!("set DNS on profile '{}'", plan.profile_name);
        let modify = if plan.reset {
            dns_reset_argv(&plan.profile_uuid)
        } else {
            dns_modify_argv(&plan.profile_uuid, &plan.new_servers)
        };
        let modify_refs: Vec<&str> = modify.iter().map(String::as_str).collect();
        super::run_nm_write("nmcli", &modify_refs, &action)?;
        let up = dns_up_argv(&plan.profile_uuid);
        let up_refs: Vec<&str> = up.iter().map(String::as_str).collect();
        super::run_nm_write("nmcli", &up_refs, &action)?;
        let effective_dns = super::net_status_nmcli().ok().map(|status| status.dns);
        Ok(DnsReport {
            profile_name: plan.profile_name.clone(),
            profile_uuid: plan.profile_uuid.clone(),
            old_servers: plan.old_servers.clone(),
            new_servers: plan.new_servers.clone(),
            effective_dns,
            reactivated: true,
        })
    }

/// Joins DNS servers for display, or the shared DHCP placeholder when
/// the profile takes DNS from DHCP (honest, never blank).
pub fn display_servers(servers: &[String]) -> String {
    if servers.is_empty() {
        super::dns_dhcp_label()
    } else {
        servers.join(" ")
    }
}

/// French title plus `(label, value)` rows for `net dns show`.
pub fn show_title() -> String {
    rust_i18n::t!("net.dns_show_title").to_string()
}

/// French rows for a [`DnsShow`] snapshot.
pub fn show_rows(show: &DnsShow) -> Vec<(String, String)> {
    let yes_no = |flag: bool| {
        rust_i18n::t!(if flag { "net.value_yes" } else { "net.value_no" }).to_string()
    };
    vec![
        (
            rust_i18n::t!("net.label_profile").to_string(),
            format!("{} ({})", show.profile_name, show.profile_uuid),
        ),
        (
            rust_i18n::t!("net.label_configured").to_string(),
            display_servers(&show.configured_servers),
        ),
        (
            rust_i18n::t!("net.label_ignore_auto").to_string(),
            yes_no(show.ignore_auto_dns),
        ),
        (
            rust_i18n::t!("net.label_effective").to_string(),
            display_servers(&show.effective_dns),
        ),
    ]
}

/// French title for a [`DnsPlan`] preview (set vs reset).
pub fn plan_preview_title(plan: &DnsPlan) -> String {
    rust_i18n::t!(if plan.reset {
        "net.dns_reset_title"
    } else {
        "net.dns_set_title"
    })
    .to_string()
}

/// French old-to-new preview rows for a [`DnsPlan`].
pub fn plan_preview_rows(plan: &DnsPlan) -> Vec<(String, String)> {
    let new_value = if plan.reset {
        super::dns_dhcp_label()
    } else {
        plan.new_servers.join(" ")
    };
    vec![
        (
            rust_i18n::t!("net.label_profile").to_string(),
            format!("{} ({})", plan.profile_name, plan.profile_uuid),
        ),
        (
            rust_i18n::t!("net.label_old").to_string(),
            display_servers(&plan.old_servers),
        ),
        (rust_i18n::t!("net.label_new").to_string(), new_value),
    ]
}

/// French title for a [`DnsReport`].
pub fn report_title() -> String {
    rust_i18n::t!("net.dns_set_title").to_string()
}

/// French rows for a [`DnsReport`]: old, new, effective read-back.
/// A failed read-back renders N/A, never a guess.
pub fn report_rows(report: &DnsReport) -> Vec<(String, String)> {
    let effective = report
        .effective_dns
        .as_deref()
        .map(display_servers)
        .unwrap_or_else(|| rust_i18n::t!("sys.unavailable").to_string());
    vec![
        (
            rust_i18n::t!("net.label_profile").to_string(),
            format!("{} ({})", report.profile_name, report.profile_uuid),
        ),
        (
            rust_i18n::t!("net.label_old").to_string(),
            display_servers(&report.old_servers),
        ),
        (
            rust_i18n::t!("net.label_new").to_string(),
            display_servers(&report.new_servers),
        ),
        (
            rust_i18n::t!("net.label_effective_after").to_string(),
            effective,
        ),
    ]
}
    /// Confirmation prompt naming the profile plus old-to-new preview
    /// (T-4-01: the operator sees exactly what changes before `[o/N]`).
    pub fn confirm_dns_prompt(plan: &DnsPlan) -> String {
        fn servers_or_dhcp(servers: &[String]) -> String {
            if servers.is_empty() {
                crate::network::dns_dhcp_label()
            } else {
                servers.join(" ")
            }
        }
        if plan.reset {
            rust_i18n::t!(
                "net.confirm_dns_reset",
                profile = plan.profile_name.clone()
            )
            .to_string()
        } else {
            rust_i18n::t!(
                "net.confirm_dns_set",
                profile = plan.profile_name.clone(),
                servers = servers_or_dhcp(&plan.new_servers)
            )
            .to_string()
        }
    }

    /// Reads configured plus effective DNS for `net dns show`.
    pub async fn dns_show(profile: Option<&str>) -> Result<DnsShow, ArchelError> {
        let (profile_name, profile_uuid) = resolve_target_profile(profile)?;
        let (configured_servers, ignore_auto_dns) = read_profile_dns(&profile_uuid)?;
        let effective_dns = super::net_status().await.map(|status| status.dns)?;
        Ok(DnsShow {
            profile_name,
            profile_uuid,
            configured_servers,
            ignore_auto_dns,
            effective_dns,
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn validator_accepts_plain_ipv4() {
            let servers = vec!["1.1.1.1".to_string(), "9.9.9.9".to_string()];
            assert_eq!(
                validate_dns_servers(&servers).expect("plain quads pass"),
                servers
            );
        }

        #[test]
        fn validator_rejects_v6_hostnames_and_bad_quads() {
            for bad in [
                "::1",
                "fe80::1",
                "example.com",
                "dns.example",
                "999.1.1.1",
                "1.1.1.256",
                "1.1.1",
                "1.1.1.1.1",
                "01.2.3.4",
                "1.02.3.4",
                "",
                "1.1.1.-1",
                "1.1.1.a",
            ] {
                let err = validate_dns_servers(&[bad.to_string()])
                    .expect_err(format!("{bad} must fail").as_str());
                assert!(
                    matches!(err, ArchelError::DnsInvalid { .. }),
                    "{bad} needs the typed variant, got {err:?}"
                );
            }
            assert!(
                validate_dns_servers(&[]).is_err(),
                "empty server list must fail before any spawn"
            );
        }

        #[test]
        fn validator_accepts_lone_zero_octets() {
            // A lone `0` is unambiguous (only multi-digit leading zeros
            // are octal-confusable).
            assert!(validate_dns_servers(&["10.0.0.1".to_string()]).is_ok());
            assert!(validate_dns_servers(&["0.0.0.0".to_string()]).is_ok());
        }

        #[test]
        fn modify_argv_emits_replace_sequence_on_ipv4_keys_only() {
            let argv = dns_modify_argv(
                "cbb5b16c-02d9-4ac7-a99c-d02b909cfb69",
                &["1.1.1.1".to_string(), "9.9.9.9".to_string()],
            );
            assert_eq!(
                argv,
                vec![
                    "connection",
                    "modify",
                    "cbb5b16c-02d9-4ac7-a99c-d02b909cfb69",
                    "ipv4.dns",
                    "1.1.1.1 9.9.9.9",
                    "ipv4.ignore-auto-dns",
                    "yes",
                ]
            );
            let joined = argv.join(" ");
            assert!(
                !joined.contains("ipv6"),
                "D-02: DNS changes never touch IPv6 keys: {joined}"
            );
        }

        #[test]
        fn reset_argv_clears_dns_and_restores_dhcp() {
            let argv = dns_reset_argv("cbb5b16c-02d9-4ac7-a99c-d02b909cfb69");
            assert_eq!(
                argv,
                vec![
                    "connection",
                    "modify",
                    "cbb5b16c-02d9-4ac7-a99c-d02b909cfb69",
                    "ipv4.dns",
                    "",
                    "ipv4.ignore-auto-dns",
                    "no",
                ]
            );
            assert!(
                !argv.join(" ").contains("ipv6"),
                "D-02: reset touches only IPv4 keys"
            );
            assert_eq!(
                dns_up_argv("cbb5b16c-02d9-4ac7-a99c-d02b909cfb69"),
                vec![
                    "connection",
                    "up",
                    "cbb5b16c-02d9-4ac7-a99c-d02b909cfb69",
                ]
            );
        }

        #[test]
        fn profile_resolution_prefers_uuid_and_rejects_ambiguity() {
            let profiles = vec![
                ("home".to_string(), "uuid-1".to_string()),
                ("home".to_string(), "uuid-2".to_string()),
                ("work".to_string(), "uuid-3".to_string()),
            ];
            assert_eq!(
                resolve_profile_uuid(&profiles, "uuid-2")
                    .expect("uuid match wins"),
                ("home".to_string(), "uuid-2".to_string())
            );
            assert_eq!(
                resolve_profile_uuid(&profiles, "work").expect("unique name resolves"),
                ("work".to_string(), "uuid-3".to_string())
            );
            assert!(
                matches!(
                    resolve_profile_uuid(&profiles, "home"),
                    Err(ArchelError::ProfileNotFound { .. })
                ),
                "duplicate names are ambiguous — never guess"
            );
            assert!(
                matches!(
                    resolve_profile_uuid(&profiles, "ghost"),
                    Err(ArchelError::ProfileNotFound { .. })
                ),
                "unknown names fail typed"
            );
        }

        #[test]
        fn profile_dns_parses_live_read_only_fixture() {
            // Captured read-only from the live machine (DHCP state).
            let (servers, ignore) = parse_profile_dns(
                "ipv4.dns:\nipv4.ignore-auto-dns:no\nipv4.method:auto\n",
            );
            assert!(servers.is_empty());
            assert!(!ignore);
            let (servers, ignore) =
                parse_profile_dns("ipv4.dns:1.1.1.1,9.9.9.9\nipv4.ignore-auto-dns:yes\n");
            assert_eq!(servers, vec!["1.1.1.1".to_string(), "9.9.9.9".to_string()]);
            assert!(ignore);
            let (servers, _) = parse_profile_dns("ipv4.dns:--\nipv4.ignore-auto-dns:no\n");
            assert!(servers.is_empty(), "nmcli dashes mean absence");
        }

        #[test]
        fn profiles_parse_skips_garbage() {
            let parsed = parse_profiles("a:uuid-1\nbroken-line\n:empty\n");
            assert_eq!(parsed, vec![("a".to_string(), "uuid-1".to_string())]);
        }
    }
}

/// Triple self-test: gateway ping, DNS resolution, internet reachability
/// (NET-06, D-09/D-10).
///
/// Raw millisecond figures only — no verdict, diagnosis or health
/// strings anywhere on this path (plan prohibition). Unreachable hops
/// surface as absent (`None` → N/A/`null`), never as `0 ms` (the
/// no-fake-zero rule). Parsers are label-agnostic on purpose: they
/// extract numbers around `/` and `msec`/`ms` markers instead of
/// matching C/French labels, so both locales parse.
pub mod selftest {
    use std::time::Duration;

    use serde::Serialize;

    use crate::error::ArchelError;

    /// Per-hop execution budget (`ping -c 3 -W 2` worst case plus
    /// spawn overhead; the whole triple stays under ~1 min).
    pub const HOP_TIMEOUT: Duration = Duration::from_secs(20);

    /// Host resolved for the DNS hop (NM connectivity-check family).
    pub const DNS_PROBE_HOST: &str = "archlinux.org";
    /// Address pinged for the internet hop (numeric — no DNS involved).
    pub const INTERNET_PROBE_IP: &str = "1.1.1.1";

    /// One hop: French name, probed target, average ms when reached.
    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct HopResult {
        pub hop: String,
        pub target: String,
        pub ms: Option<f64>,
    }

    /// French title for the triple report.
    pub fn test_title() -> String {
        rust_i18n::t!("net.test_title").to_string()
    }

    /// Formats one hop latency: raw ms with two decimals (sub-ms LAN
    /// RTTs need the second digit), or honest N/A — never 0 ms.
    pub fn format_hop_ms(ms: Option<f64>) -> String {
        ms.map(|value| {
            format!(
                "{value:.2} {}",
                rust_i18n::t!("net.unit_ms").to_string()
            )
        })
        .unwrap_or_else(|| rust_i18n::t!("sys.unavailable").to_string())
    }

    /// French `(hop (target), latency)` rows for a [`NetTest`].
    /// Verdict-free by construction (D-10): only names, targets and
    /// figures cross into display/JSON.
    pub fn test_rows(test: &NetTest) -> Vec<(String, String)> {
        test.hops
            .iter()
            .map(|hop| {
                (
                    format!("{} ({})", hop.hop, hop.target),
                    format_hop_ms(hop.ms),
                )
            })
            .collect()
    }

    /// The three hops in D-09 order: gateway, DNS, internet.
    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct NetTest {
        pub hops: Vec<HopResult>,
    }

    /// Extracts the average RTT from ping summary output.
    ///
    /// Takes the last `=` line whose right side holds 3+ slash-separated
    /// numbers and returns the second (avg). Per-packet `ttl=` lines
    /// never qualify (single field, no slashes). French decimal commas
    /// parse (`0,052` → `0.052`); the unit suffix parses by leading
    /// number. `None` on loss/timeout output — honest absence.
    pub fn parse_ping_avg(stdout: &str) -> Option<f64> {
        let mut avg = None;
        for line in stdout.lines() {
            let Some((_, right)) = line.split_once('=') else {
                continue;
            };
            let fields: Vec<Option<f64>> =
                right.split('/').map(|f| leading_float(f.trim())).collect();
            if fields.iter().filter(|f| f.is_some()).count() >= 3 {
                avg = fields[1];
            }
        }
        avg
    }

    /// Leading numeric prefix of a field (`0.052 ms` → `0.052`,
    /// `0,052` → `0.052`). Stops at the first non-numeric char.
    fn leading_float(field: &str) -> Option<f64> {
        let mut end = 0;
        let mut comma_seen = false;
        for (idx, ch) in field.char_indices() {
            if ch.is_ascii_digit() {
                end = idx + ch.len_utf8();
            } else if (ch == '.' || ch == ',') && !comma_seen {
                comma_seen = true;
                end = idx + ch.len_utf8();
            } else {
                break;
            }
        }
        if end == 0 {
            return None;
        }
        field[..end].replace(',', ".").parse().ok()
    }

    /// Extracts dig's self-reported query time in ms.
    ///
    /// Reads the first `;;`-comment number preceding `msec` (C locale)
    /// or `ms` (French locale). `None` on timeout output (`connection
    /// timed out; no servers could be reached` carries no figure).
    pub fn parse_dig_ms(stdout: &str) -> Option<u64> {
        for line in stdout.lines() {
            let line = line.trim();
            if !line.starts_with(";;") {
                continue;
            }
            for unit in ["msec", "ms"] {
                if let Some((left, _)) = line.split_once(unit) {
                    if let Some(num) = left.split_whitespace().last() {
                        if let Ok(ms) = num.parse::<u64>() {
                            return Some(ms);
                        }
                    }
                }
            }
        }
        None
    }

    /// Runs one hop probe and returns stdout on ANY exit code.
    ///
    /// `ping` exits 1 on 100% loss and `dig` exits 0 on timeout — the
    /// exit code carries no signal here, only the parsed figures do.
    /// Errors (missing binary, spawn failure, timeout) so the caller
    /// can degrade that hop to `None`.
    fn run_hop_output(tool: &str, args: &[&str]) -> Result<String, ArchelError> {
        let binary = which::which(tool)
            .map_err(|err| ArchelError::Internal(format!("{tool} ({err})")))?;
        let handle = duct::cmd(&binary, args)
            .unchecked()
            .env("LC_ALL", "C")
            .stdout_capture()
            .stderr_capture()
            .start()
            .map_err(|err| ArchelError::Internal(format!("cannot run '{tool}': {err}")))?;
        let output = match handle.wait_timeout(HOP_TIMEOUT).map_err(|err| {
            ArchelError::Internal(format!("cannot wait on '{tool}': {err}"))
        })? {
            Some(output) => output,
            None => {
                let _ = handle.kill();
                return Err(ArchelError::Internal(format!(
                    "'{tool}' exceeded the hop timeout"
                )));
            }
        };
        let _stderr_for_logs = String::from_utf8_lossy(&output.stderr);
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    /// Runs the D-09 triple. Gateways come from the synchronous NM
    /// read; a missing gateway (or missing probe tool) degrades that
    /// hop to `None`, never to zero. Only a fully unreadable network
    /// stack errors ([`ArchelError::NetworkUnavailable`]).
    pub fn run_net_test() -> Result<NetTest, ArchelError> {
        let status = super::net_status_nmcli()?;
        let gateway_target = status.gateway.clone().unwrap_or_default();
        let gateway_ms = if gateway_target.is_empty() {
            None
        } else {
            run_hop_output("ping", &["-c", "3", "-W", "2", &gateway_target])
                .ok()
                .and_then(|out| parse_ping_avg(&out))
        };
        let dns_ms = run_hop_output(
            "dig",
            &["+time=2", "+tries=1", DNS_PROBE_HOST],
        )
        .ok()
        .and_then(|out| parse_dig_ms(&out))
        .map(|ms| ms as f64);
        let internet_ms = run_hop_output("ping", &["-c", "3", "-W", "2", INTERNET_PROBE_IP])
            .ok()
            .and_then(|out| parse_ping_avg(&out));
        Ok(NetTest {
            hops: vec![
                HopResult {
                    hop: rust_i18n::t!("net.hop_gateway").to_string(),
                    target: gateway_target,
                    ms: gateway_ms,
                },
                HopResult {
                    hop: rust_i18n::t!("net.hop_dns").to_string(),
                    target: DNS_PROBE_HOST.to_string(),
                    ms: dns_ms,
                },
                HopResult {
                    hop: rust_i18n::t!("net.hop_internet").to_string(),
                    target: INTERNET_PROBE_IP.to_string(),
                    ms: internet_ms,
                },
            ],
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// C-locale ping summary fixture.
        const PING_C: &str = "PING 10.192.61.1 (10.192.61.1) 56(84) bytes of data.\n64 bytes from 10.192.61.1: icmp_seq=1 ttl=64 time=0.312 ms\n64 bytes from 10.192.61.1: icmp_seq=2 ttl=64 time=0.298 ms\n64 bytes from 10.192.61.1: icmp_seq=3 ttl=64 time=0.341 ms\n\n--- 10.192.61.1 ping statistics ---\n3 packets transmitted, 3 received, 0% packet loss, time 2003ms\nrtt min/avg/max/mdev = 0.298/0.317/0.341/0.018 ms\n";

        /// French-locale ping summary fixture (decimal commas, no
        /// per-packet lines shown — the parser must not need them).
        const PING_FR: &str = "PING 10.192.61.1 (10.192.61.1) 56(84) octets de donn\u{e9}es.\n\n--- 10.192.61.1 statistiques ping ---\n3 paquets transmis, 3 re\u{e7}us, 0% de perte, temps 2003ms\naller-retour min/moy/max/mdev = 0,298/0,317/0,341/0,018 ms\n";

        /// Unreachable-host output: loss line, no figures anywhere.
        const PING_DOWN: &str = "PING 192.0.2.1 (192.0.2.1) 56(84) bytes of data.\n\n--- 192.0.2.1 ping statistics ---\n3 packets transmitted, 0 received, 100% packet loss, time 2040ms\n";

        /// C-locale dig tail fixture.
        const DIG_C: &str = ";; Query time: 12 msec\n;; SERVER: 10.192.61.1#53(10.192.61.1) (UDP)\n;; WHEN: Tue Oct 06 14:00:00 CEST 2026\n;; MSG SIZE  rcvd: 59\n";

        /// French-locale dig tail fixture.
        const DIG_FR: &str = ";; Temps de la requ\u{ea}te : 13 ms\n;; SERVEUR : 10.192.61.1#53(10.192.61.1) (UDP)\n";

        /// Dig timeout output: no figure to extract.
        const DIG_TIMEOUT: &str = ";; connection timed out; no servers could be reached\n";

        #[test]
        fn ping_avg_extracts_the_second_field_in_both_locales() {
            assert_eq!(parse_ping_avg(PING_C), Some(0.317));
            assert_eq!(parse_ping_avg(PING_FR), Some(0.317));
        }

        #[test]
        fn ping_avg_ignores_packet_lines_and_loss_output() {
            // Per-packet `ttl=` lines must never feed the average, and
            // total loss yields absence — never 0 ms.
            assert_eq!(parse_ping_avg(PING_DOWN), None);
            assert_eq!(parse_ping_avg(""), None);
            assert_ne!(parse_ping_avg(PING_DOWN), Some(0.0));
        }

        #[test]
        fn dig_ms_parses_msec_and_ms_units() {
            assert_eq!(parse_dig_ms(DIG_C), Some(12));
            assert_eq!(parse_dig_ms(DIG_FR), Some(13));
            assert_eq!(parse_dig_ms(DIG_TIMEOUT), None);
            assert_eq!(parse_dig_ms(""), None);
        }

        #[test]
        fn leading_float_handles_units_and_commas() {
            assert_eq!(leading_float("0.317 ms"), Some(0.317));
            assert_eq!(leading_float("0,317"), Some(0.317));
            assert_eq!(leading_float("ms"), None);
            assert_eq!(leading_float(""), None);
        }
    }
}

/// French DHCP placeholder for DNS previews (empty configured list).
pub(crate) fn dns_dhcp_label() -> String {
    rust_i18n::t!("net.value_dhcp").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Live-shaped `connection show --active` fixture (captured 2026-10-06;
    /// loopback and the `wg0` tunnel must never win over the Wi-Fi link).
    const ACTIVE_FIXTURE: &str = "KLEY Rennes priv\u{e9}e:cbb5b16c-02d9-4ac7-a99c-d02b909cfb69:802-11-wireless:wlan0\nlo:f1eac978-9bbb-4d2f-a7b7-65cdff4a9fb1:loopback:lo\nwg0:4228f697-8c46-482d-a53e-ce15f5b04ec0:wireguard:wg0\n";

    /// Live-shaped `device show wlan0` IP4 fixture.
    const DEVICE_IP4_FIXTURE: &str = "IP4.ADDRESS[1]:10.192.61.76/24\nIP4.GATEWAY:10.192.61.1\nIP4.DNS[1]:10.192.61.1\n";

    /// Live-shaped `device wifi list` fixture (C locale).
    const WIFI_LIST_FIXTURE: &str = "no:KLEY Rennes\nno:KLEY Rennes priv\u{e9}e\nno:eduroam\nno:Livebox-8456\nno:KLEY Rennes invit\u{e9}s\nno:SFR_2A50\nno:Freebox-8BE8A0\nno:freebox\n";

    /// Same list with the active network marked (NM marks one `yes:` row).
    const WIFI_LIST_ACTIVE_FIXTURE: &str = "no:KLEY Rennes\nno:KLEY Rennes priv\u{e9}e\nyes:KLEY Rennes priv\u{e9}e\nno:eduroam\n";

    /// SSID containing a colon — the split must keep the whole SSID.
    const WIFI_LIST_COLON_SSID: &str = "no:other\nyes:corp:lobby:guest\n";

    #[test]
    fn active_connection_pick_skips_loopback_and_prefers_first_link() {
        let picked = parse_active_connections(ACTIVE_FIXTURE).expect("must pick a link");
        assert_eq!(picked.0, "KLEY Rennes priv\u{e9}e");
        assert_eq!(picked.1, "cbb5b16c-02d9-4ac7-a99c-d02b909cfb69");
        assert_eq!(picked.2, "802-11-wireless");
        assert_eq!(picked.3, "wlan0");
    }

    #[test]
    fn active_connection_pick_returns_none_without_links() {
        assert_eq!(parse_active_connections(""), None);
        assert_eq!(
            parse_active_connections("lo:f1eac978-9bbb-4d2f-a7b7-65cdff4a9fb1:loopback:lo\n"),
            None
        );
        assert_eq!(parse_active_connections("garbage-line\n"), None);
    }

    #[test]
    fn device_ip4_parses_live_fields() {
        let parsed = parse_device_ip4(DEVICE_IP4_FIXTURE);
        assert_eq!(parsed.address.as_deref(), Some("10.192.61.76/24"));
        assert_eq!(parsed.gateway.as_deref(), Some("10.192.61.1"));
        assert_eq!(parsed.dns, vec!["10.192.61.1".to_string()]);
    }

    #[test]
    fn device_ip4_treats_dashes_as_absence() {
        let parsed = parse_device_ip4("IP4.ADDRESS[1]:--\nIP4.GATEWAY:--\nIP4.DNS[1]:--\n");
        assert_eq!(parsed, NmcliIp4::default());
    }

    #[test]
    fn device_ip4_collects_multiple_dns() {
        let parsed = parse_device_ip4("IP4.DNS[1]:1.1.1.1\nIP4.DNS[2]:9.9.9.9\n");
        assert_eq!(parsed.dns, vec!["1.1.1.1".to_string(), "9.9.9.9".to_string()]);
    }

    #[test]
    fn general_state_parses_numeric_code() {
        assert_eq!(
            parse_general_state("GENERAL.STATE:100 (connected)\n"),
            Some(100)
        );
        assert_eq!(parse_general_state("GENERAL.STATE:30 (disconnected)\n"), Some(30));
        assert_eq!(parse_general_state(""), None);
    }

    #[test]
    fn wifi_ssid_picks_the_yes_row() {
        assert_eq!(parse_wifi_ssid(WIFI_LIST_FIXTURE), None);
        assert_eq!(
            parse_wifi_ssid(WIFI_LIST_ACTIVE_FIXTURE).as_deref(),
            Some("KLEY Rennes priv\u{e9}e")
        );
    }

    #[test]
    fn wifi_ssid_keeps_colons_inside_the_ssid() {
        assert_eq!(
            parse_wifi_ssid(WIFI_LIST_COLON_SSID).as_deref(),
            Some("corp:lobby:guest")
        );
    }

    #[test]
    fn dbus_u32_decodes_to_dotted_quad() {
        // Live value for 10.192.61.1 (address bytes in memory order).
        assert_eq!(ipv4_from_dbus_u32(20824074), "10.192.61.1");
        assert_eq!(ipv4_from_dbus_u32(0x0100007f), "127.0.0.1");
    }

    #[test]
    fn nm_state_codes_render_french_without_panic() {
        rust_i18n::set_locale("fr");
        let connected = describe_nm_state(70);
        assert!(!connected.trim().is_empty());
        assert_ne!(connected, describe_nm_state(30));
        // Site/local count as connected (same family, distinct wording).
        assert_ne!(describe_nm_state(50), describe_nm_state(30));
        assert_ne!(describe_nm_state(60), describe_nm_state(30));
        // Unknown codes never panic and never claim connected wording.
        let _ = describe_nm_state(u32::MAX);
    }

    #[test]
    fn ssid_bytes_map_to_display_or_absence() {
        assert_eq!(
            ssid_from_bytes("KLEY Rennes priv\u{e9}e".as_bytes().to_vec()).as_deref(),
            Some("KLEY Rennes priv\u{e9}e")
        );
        assert_eq!(ssid_from_bytes(Vec::new()), None);
    }

    #[test]
    fn nmcli_exit_map_accepts_zero_only() {
        assert!(map_nmcli_exit("nmcli", Some(0)).is_ok());
        assert!(map_nmcli_exit("nmcli", Some(1)).is_err());
        assert!(map_nmcli_exit("nmcli", None).is_err());
    }

    #[test]
    fn status_rows_render_na_for_absent_fields() {
        rust_i18n::set_locale("fr");
        let status = NetStatus {
            source: "dbus".to_string(),
            connected: false,
            state_detail: describe_nm_state(10),
            interface: None,
            ssid: None,
            ipv4_address: None,
            gateway: None,
            dns: Vec::new(),
            profile_name: None,
            profile_uuid: None,
            connection_type: None,
        };
        let rows = status_rows(&status);
        assert_eq!(rows.len(), 7);
        for (label, value) in &rows {
            assert!(!label.trim().is_empty(), "label must not be empty");
            assert!(!value.trim().is_empty(), "value must not be empty");
            assert_ne!(value, "0", "absent fields render N/A, never zero");
        }
    }

    #[test]
    fn status_snapshot_serializes_to_json() {
        let status = NetStatus {
            source: "nmcli".to_string(),
            connected: true,
            state_detail: "connect\u{e9}".to_string(),
            interface: Some("wlan0".to_string()),
            ssid: Some("KLEY Rennes priv\u{e9}e".to_string()),
            ipv4_address: Some("10.192.61.76/24".to_string()),
            gateway: Some("10.192.61.1".to_string()),
            dns: vec!["10.192.61.1".to_string()],
            profile_name: Some("KLEY Rennes priv\u{e9}e".to_string()),
            profile_uuid: Some("cbb5b16c-02d9-4ac7-a99c-d02b909cfb69".to_string()),
            connection_type: Some("802-11-wireless".to_string()),
        };
        let value = serde_json::to_value(&status).expect("NetStatus serializes");
        assert_eq!(value["interface"], "wlan0");
        assert_eq!(value["dns"][0], "10.192.61.1");
    }

    #[test]
    fn wifi_device_type_matches_nm_spec() {
        // Only Wi-Fi devices (type 2) resolve an SSID; every other
        // device type (1 = Ethernet, …) skips the AP lookup.
        assert_eq!(DEVICE_TYPE_WIFI, 2);
    }

    #[test]
    fn write_denials_map_to_polkit_with_agent_hint() {
        // T-4-02: EN (LC_ALL=C) plus FR denial shapes become the typed
        // refusal; anything else stays an internal write failure.
        for stderr in [
            "Error: Not authorized to control networking.",
            "Error: access denied: insufficient privileges.",
            "Erreur : authentification requise pour modifier la connexion.",
            "Erreur : acc\u{e8}s non autoris\u{e9}.",
        ] {
            let err = map_nm_write_error("set DNS", "nmcli", Some(1), stderr);
            assert!(
                matches!(err, ArchelError::PolkitDenied { .. }),
                "denial must map typed, got {err:?} for {stderr:?}"
            );
        }
        let err = map_nm_write_error("set DNS", "nmcli", Some(1), "Error: unknown connection.");
        assert!(
            matches!(err, ArchelError::Internal(_)),
            "non-auth failures stay internal: {err:?}"
        );
    }
}
