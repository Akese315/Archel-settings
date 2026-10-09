//! Network view-model: live connectivity state plus the VPN list
//! (read-only; the Importer affordance stays inert until plan 07).
//!
//! Same proven shape as `models::system`: `tick()` is a QML-called
//! invokable that only spawns - the async `network::net_status()` runs
//! on the gui runtime and the sync `vpn::vpn_list()` (a quick nmcli
//! spawn) rides `spawn_blocking` beside it. The finished payload
//! returns exclusively through `CxxQtThread.queue`, whose closure is
//! the only code allowed to set properties (RESEARCH Pattern 1).
//! No D-Bus or nmcli call lives in this file: every read goes through
//! the CLI-proven core entry points (PLAT-03).

use cxx_qt_lib::QString;

use super::super::poll::{BannerPolicy, PollGuard};
use crate::gui::models::system::NOT_AVAILABLE;

/// Fully formatted tick payload, built off the Qt thread so the
/// `queue()` closure only sets properties.
pub(crate) struct AppliedNet {
    pub status_text: String,
    pub iface_text: String,
    pub ssid_text: String,
    pub ipv4_text: String,
    pub gateway_text: String,
    pub dns_text: String,
    pub profile_text: String,
    pub conns_json: String,
    pub vpn_json: String,
    pub banner_visible: bool,
}

/// Backing state: property fields plus the poll guard and banner
/// policy. No shared core handle: every tick is an independent read.
pub struct NetModelRust {
    pub(crate) status_text: QString,
    pub(crate) iface_text: QString,
    pub(crate) ssid_text: QString,
    pub(crate) ipv4_text: QString,
    pub(crate) gateway_text: QString,
    pub(crate) dns_text: QString,
    pub(crate) profile_text: QString,
    pub(crate) conns_json: QString,
    pub(crate) vpn_json: QString,
    pub(crate) banner_visible: bool,
    guard: PollGuard,
    banner: BannerPolicy,
}

impl Default for NetModelRust {
    fn default() -> Self {
        Self {
            status_text: QString::from(""),
            iface_text: QString::from(""),
            ssid_text: QString::from(""),
            ipv4_text: QString::from(""),
            gateway_text: QString::from(""),
            dns_text: QString::from(""),
            profile_text: QString::from(""),
            conns_json: QString::from(""),
            vpn_json: QString::from(""),
            banner_visible: false,
            guard: PollGuard::new(),
            banner: BannerPolicy::new(),
        }
    }
}

/// Renders an optional core field, or honest N/A (D-04).
pub(crate) fn opt_or_na(value: Option<&str>) -> String {
    match value {
        Some(text) if !text.trim().is_empty() => text.to_string(),
        _ => NOT_AVAILABLE.to_string(),
    }
}

/// Connection detail rows as QML-ready JSON: `[{"label","value"}]`.
/// Reuses the same `status_rows` the CLI table renders, so the tab and
/// `archel net status` never diverge.
pub(crate) fn conns_json_for(status: &archel_core::network::NetStatus) -> String {
    let rows: Vec<serde_json::Value> = archel_core::network::status_rows(status)
        .iter()
        .map(|(label, value)| serde_json::json!({"label": label, "value": value}))
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string())
}

/// VPN rows as QML-ready JSON: `[{name,uuid,vpn_type,active}]` - the
/// same fields `run_vpn_list` renders (name/type/active), straight from
/// the serializable `VpnConnection` (never re-derived here).
pub(crate) fn vpn_json_for(
    conns: &[archel_core::network::vpn::VpnConnection],
) -> String {
    serde_json::to_string(conns).unwrap_or_else(|_| "[]".to_string())
}

impl NetModelRust {
    /// Applies a finished worker outcome on the Qt thread. A failed
    /// `net_status` reads N/A everywhere plus the once-per-episode
    /// banner; a failed VPN list degrades to the honest empty list
    /// (the empty-state copy covers it) without nuking the tab.
    fn apply_outcome(
        &mut self,
        net: Result<archel_core::network::NetStatus, archel_core::ArchelError>,
        vpn: Result<Vec<archel_core::network::vpn::VpnConnection>, archel_core::ArchelError>,
    ) -> AppliedNet {
        match net {
            Ok(status) => {
                self.banner.on_success();
                AppliedNet {
                    status_text: status.state_detail.clone(),
                    iface_text: opt_or_na(status.interface.as_deref()),
                    ssid_text: opt_or_na(status.ssid.as_deref()),
                    ipv4_text: opt_or_na(status.ipv4_address.as_deref()),
                    gateway_text: opt_or_na(status.gateway.as_deref()),
                    dns_text: if status.dns.is_empty() {
                        NOT_AVAILABLE.to_string()
                    } else {
                        status.dns.join(", ")
                    },
                    profile_text: opt_or_na(status.profile_name.as_deref()),
                    conns_json: conns_json_for(&status),
                    vpn_json: vpn
                        .map(|conns| vpn_json_for(&conns))
                        .unwrap_or_else(|_| "[]".to_string()),
                    banner_visible: false,
                }
            }
            Err(_) => {
                self.banner.on_failure();
                AppliedNet {
                    status_text: NOT_AVAILABLE.to_string(),
                    iface_text: NOT_AVAILABLE.to_string(),
                    ssid_text: NOT_AVAILABLE.to_string(),
                    ipv4_text: NOT_AVAILABLE.to_string(),
                    gateway_text: NOT_AVAILABLE.to_string(),
                    dns_text: NOT_AVAILABLE.to_string(),
                    profile_text: NOT_AVAILABLE.to_string(),
                    conns_json: "[]".to_string(),
                    vpn_json: "[]".to_string(),
                    banner_visible: self.banner.wants_visible(),
                }
            }
        }
    }
}

impl crate::gui::bridge_net::qobject::NetModel {
    /// 1 Hz entry point: skips while a tick is in flight (D-02),
    /// spawns the async status plus the VPN list, marshals back
    /// through `queue()` only.
    pub fn tick(mut self: core::pin::Pin<&mut Self>) {
        use cxx_qt::{CxxQtType, Threading};
        let tag = match self.as_mut().rust_mut().guard.try_begin() {
            Some(tag) => tag,
            None => return,
        };
        let qt_thread = self.qt_thread();
        let runtime = super::super::RUNTIME
            .get()
            .expect("gui runtime is set before Qt starts");
        runtime.spawn(async move {
            let net = archel_core::network::net_status().await;
            let vpn = tokio::task::spawn_blocking(archel_core::network::vpn::vpn_list)
                .await
                .unwrap_or_else(|err| {
                    Err(archel_core::ArchelError::Internal(format!(
                        "vpn list task failed: {err}"
                    )))
                });
            let _ = qt_thread.queue(
                move |mut obj: core::pin::Pin<&mut Self>| {
                    let applied = {
                        let mut rust = obj.as_mut().rust_mut();
                        if !rust.guard.is_newest(tag) {
                            rust.guard.finish();
                            return;
                        }
                        rust.guard.finish();
                        rust.apply_outcome(net, vpn)
                    };
                    obj.as_mut()
                        .set_status_text(QString::from(&applied.status_text));
                    obj.as_mut()
                        .set_iface_text(QString::from(&applied.iface_text));
                    obj.as_mut()
                        .set_ssid_text(QString::from(&applied.ssid_text));
                    obj.as_mut()
                        .set_ipv4_text(QString::from(&applied.ipv4_text));
                    obj.as_mut()
                        .set_gateway_text(QString::from(&applied.gateway_text));
                    obj.as_mut()
                        .set_dns_text(QString::from(&applied.dns_text));
                    obj.as_mut()
                        .set_profile_text(QString::from(&applied.profile_text));
                    obj.as_mut()
                        .set_conns_json(QString::from(&applied.conns_json));
                    obj.as_mut()
                        .set_vpn_json(QString::from(&applied.vpn_json));
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

    fn ready_status() -> archel_core::network::NetStatus {
        archel_core::network::NetStatus {
            source: "test".to_string(),
            connected: true,
            state_detail: "connecte".to_string(),
            interface: Some("wlan0".to_string()),
            ssid: Some("home-wifi".to_string()),
            ipv4_address: Some("10.0.0.5/24".to_string()),
            gateway: Some("10.0.0.1".to_string()),
            dns: vec!["10.0.0.1".to_string()],
            profile_name: Some("home".to_string()),
            profile_uuid: Some("uuid-1".to_string()),
            connection_type: Some("802-11-wireless".to_string()),
        }
    }

    fn failed_net() -> Result<archel_core::network::NetStatus, archel_core::ArchelError> {
        Err(archel_core::ArchelError::network_unavailable(
            "no bus in test",
        ))
    }

    #[test]
    fn error_path_reads_na_and_signals_banner_once() {
        let mut model = NetModelRust::default();
        let first = model.apply_outcome(
            failed_net(),
            Ok(Vec::new()),
        );
        for text in [
            &first.status_text,
            &first.iface_text,
            &first.ssid_text,
            &first.ipv4_text,
            &first.gateway_text,
            &first.dns_text,
            &first.profile_text,
        ] {
            assert_eq!(text, NOT_AVAILABLE, "failed tick must read N/A");
        }
        assert_eq!(first.vpn_json, "[]");
        assert!(first.banner_visible, "first failure shows the banner");
        let second = model.apply_outcome(failed_net(), Ok(Vec::new()));
        assert!(
            second.banner_visible,
            "consecutive failures keep the banner without re-signalling"
        );
    }

    #[test]
    fn success_fills_fields_and_vpn_row_shape() {
        let mut model = NetModelRust::default();
        let _ = model.apply_outcome(failed_net(), Ok(Vec::new()));
        let vpn = archel_core::network::vpn::VpnConnection {
            name: "bureau".to_string(),
            uuid: "uuid-vpn-1".to_string(),
            vpn_type: "wireguard".to_string(),
            active: true,
            persistent: true,
        };
        let applied = model.apply_outcome(Ok(ready_status()), Ok(vec![vpn]));
        assert!(!applied.banner_visible, "success hides the banner");
        assert_eq!(applied.iface_text, "wlan0");
        assert_eq!(applied.ssid_text, "home-wifi");
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(&applied.vpn_json).expect("vpn list must be JSON");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], "bureau");
        assert_eq!(rows[0]["vpn_type"], "wireguard");
        assert_eq!(rows[0]["active"], true);
        assert!(rows[0].get("uuid").is_some(), "uuid key must survive");
        let conns: Vec<serde_json::Value> =
            serde_json::from_str(&applied.conns_json).expect("rows must be JSON");
        assert!(!conns.is_empty(), "detail rows must mirror status_rows");
        assert!(conns[0].get("label").is_some());
        assert!(conns[0].get("value").is_some());
    }

    #[test]
    fn vpn_failure_degrades_to_empty_list_without_nuking_tab() {
        let mut model = NetModelRust::default();
        let vpn_err: Result<
            Vec<archel_core::network::vpn::VpnConnection>,
            archel_core::ArchelError,
        > = Err(archel_core::ArchelError::network_unavailable(
            "nmcli missing",
        ));
        let applied = model.apply_outcome(Ok(ready_status()), vpn_err);
        assert_eq!(applied.iface_text, "wlan0", "net fields stay live");
        assert_eq!(applied.vpn_json, "[]", "vpn degrades to empty list");
    }

    #[test]
    fn blank_options_read_na_never_empty() {
        assert_eq!(opt_or_na(None), NOT_AVAILABLE);
        assert_eq!(opt_or_na(Some("")), NOT_AVAILABLE);
        assert_eq!(opt_or_na(Some("  ")), NOT_AVAILABLE);
        assert_eq!(opt_or_na(Some("wlan0")), "wlan0");
    }
}
