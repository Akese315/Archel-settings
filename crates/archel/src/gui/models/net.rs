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

use std::path::{Path, PathBuf};

use cxx_qt_lib::QString;

use super::super::poll::{BannerPolicy, PollGuard};
use crate::gui::modal::{self, ModalSession};
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
    pub(crate) dns_busy: bool,
    pub(crate) vpn_busy: bool,
    pub(crate) vpn_result_text: QString,
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
            dns_busy: false,
            vpn_busy: false,
            vpn_result_text: QString::from(""),
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

    /// Dismisses the VPN import success banner.
    pub fn dismiss_vpn_result(mut self: core::pin::Pin<&mut Self>) {
        use cxx_qt::CxxQtType;
        self.as_mut()
            .set_vpn_result_text(QString::from(""));
    }

    /// Live DNS validation for the editor rows: runs core
    /// `validate_dns_servers` synchronously (pure, no spawn) and
    /// returns the French error for inline display, or the empty
    /// string when the list is valid (T-06-31 gate, same call the
    /// apply worker re-checks before planning).
    pub fn dns_validate(self: core::pin::Pin<&mut Self>, servers_json: QString) -> QString {
        let servers = parse_servers_json(&servers_json.to_string());
        match archel_core::network::dns::validate_dns_servers(&servers) {
            Ok(_) => QString::from(""),
            Err(err) => QString::from(&err.message()),
        }
    }

    /// Applies the edited DNS list (D-12): validate, plan against the
    /// active profile, confirm with the plan's own question verbatim,
    /// then apply via `spawn_blocking` with a spinner on the apply
    /// button (D-15). Failures surface in the error modal (D-17);
    /// success refreshes the tab so the DNS row reads back.
    pub fn dns_apply(mut self: core::pin::Pin<&mut Self>, servers_json: QString) {        use cxx_qt::CxxQtType;
        if self.as_mut().rust_mut().dns_busy {
            return;
        }
        self.as_mut().rust_mut().dns_busy = true;
        self.as_mut().set_dns_busy(true);
        let servers = parse_servers_json(&servers_json.to_string());
        let qt_thread = {
            use cxx_qt::Threading;
            self.qt_thread()
        };
        let runtime = super::super::RUNTIME
            .get()
            .expect("gui runtime is set before Qt starts");
        runtime.spawn_blocking(move || {
            dns_worker(&servers, &PROD_NET_DNS_OPS, modal::session());
            let _ = qt_thread.queue(|mut obj: core::pin::Pin<&mut Self>| {
                use cxx_qt::CxxQtType;
                obj.as_mut().rust_mut().dns_busy = false;
                obj.as_mut().set_dns_busy(false);
                obj.as_mut().tick();
            });
        });
    }

    /// Returns the active profile to automatic DNS (D-09, plan 07-04):
    /// no servers to validate, so the reset worker plans straight
    /// against the active profile (`None`), confirms with the core
    /// reset question verbatim (D-10), then applies through the same
    /// `apply_dns_plan` leg the set path uses (same privilege gate,
    /// no new escalation path). Busy-guard plus spinner plus refresh
    /// mirror `dns_apply` exactly.
    pub fn dns_reset(mut self: core::pin::Pin<&mut Self>) {
        use cxx_qt::CxxQtType;
        if self.as_mut().rust_mut().dns_busy {
            return;
        }
        self.as_mut().rust_mut().dns_busy = true;
        self.as_mut().set_dns_busy(true);
        let qt_thread = {
            use cxx_qt::Threading;
            self.qt_thread()
        };
        let runtime = super::super::RUNTIME
            .get()
            .expect("gui runtime is set before Qt starts");
        runtime.spawn_blocking(move || {
            dns_reset_worker(&PROD_NET_DNS_OPS, modal::session());
            let _ = qt_thread.queue(|mut obj: core::pin::Pin<&mut Self>| {
                use cxx_qt::CxxQtType;
                obj.as_mut().rust_mut().dns_busy = false;
                obj.as_mut().set_dns_busy(false);
                obj.as_mut().tick();
            });
        });
    }

    /// Starts a VPN import for one picked or typed path (D-20): the
    /// core path validation and plugin inference run first (traversal
    /// and nonexistent paths are rejected with core's own message
    /// before any confirm), then the core import confirm question,
    /// then `vpn_import_stdin_null` on the blocking pool - the nmcli
    /// `--ask` child runs with stdin detached so no secret prompt can
    /// hang the GUI (T-06-32). The inferred type plus the reported
    /// connection name publish as a success banner; failures go to
    /// the error modal.
    pub fn vpn_import_start(mut self: core::pin::Pin<&mut Self>, path: QString) {
        use cxx_qt::CxxQtType;
        if self.as_mut().rust_mut().vpn_busy {
            return;
        }
        self.as_mut().rust_mut().vpn_busy = true;
        self.as_mut().set_vpn_busy(true);
        let text = path.to_string();
        let qt_thread = {
            use cxx_qt::Threading;
            self.qt_thread()
        };
        let runtime = super::super::RUNTIME
            .get()
            .expect("gui runtime is set before Qt starts");
        runtime.spawn_blocking(move || {
            let outcome = vpn_worker(&text, &PROD_NET_VPN_OPS, modal::session());
            let _ = qt_thread.queue(move |mut obj: core::pin::Pin<&mut Self>| {
                use cxx_qt::CxxQtType;
                obj.as_mut().rust_mut().vpn_busy = false;
                obj.as_mut().set_vpn_busy(false);
                if let Some(line) = outcome {
                    obj.as_mut()
                        .set_vpn_result_text(QString::from(&line));
                }
                obj.as_mut().tick();
            });
        });
    }
}

/// Parses the DNS editor payload QML sends (`JSON.stringify` of the
/// row fields). Entries are trimmed and blanks dropped; anything
/// unparsable is an honest empty list - the worker's own validation
/// refuses it before any plan exists.
pub(crate) fn parse_servers_json(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw)
        .unwrap_or_default()
        .into_iter()
        .map(|server| server.trim().to_string())
        .filter(|server| !server.is_empty())
        .collect()
}

/// Injection seam for the DNS path: the exact core entry points the
/// worker calls, as plain function pointers. Production passes the
/// real core functions ([`PROD_NET_DNS_OPS`]); unit tests pass fakes
/// so the validate-plan-confirm-apply sequence is proven without
/// touching NM (the D-Bus/nmcli contract is the core's tested
/// territory - the GUI never spawns or edits config itself,
/// T-06-31).
pub(crate) struct NetDnsOps {
    pub validate: fn(&[String]) -> Result<Vec<String>, archel_core::ArchelError>,
    pub plan: fn(
        Option<&str>,
        &[String],
    ) -> Result<archel_core::network::dns::DnsPlan, archel_core::ArchelError>,
    pub plan_reset:
        fn(Option<&str>) -> Result<archel_core::network::dns::DnsPlan, archel_core::ArchelError>,
    pub prompt: fn(&archel_core::network::dns::DnsPlan) -> String,
    pub apply: fn(
        &archel_core::network::dns::DnsPlan,
    ) -> Result<archel_core::network::dns::DnsReport, archel_core::ArchelError>,
}

/// The production wiring: validate, plan and apply are the
/// CLI-proven `net dns set` pieces (`run_dns_set` calls the same
/// three in the same order); the plan targets the active profile
/// (`None`), exactly as the CLI does without `--profile`. The reset
/// entry is the CLI-proven `net dns reset` leg (`run_dns_reset`).
pub(crate) const PROD_NET_DNS_OPS: NetDnsOps = NetDnsOps {
    validate: archel_core::network::dns::validate_dns_servers,
    plan: archel_core::network::dns::plan_dns_set,
    plan_reset: archel_core::network::dns::plan_dns_reset,
    prompt: archel_core::network::dns::confirm_dns_prompt,
    apply: archel_core::network::dns::apply_dns_plan,
};

/// DNS apply worker body (runs on the blocking pool): the validated
/// servers plan against the active profile; the question is the
/// plan's own confirm prompt verbatim; only a yes reaches the apply;
/// every failure (validation, plan, apply - privilege shapes
/// included) surfaces as core's readable `message()` in the error
/// modal (D-17).
pub(crate) fn dns_worker(
    servers: &[String],
    ops: &NetDnsOps,
    sess: &ModalSession,
) {
    let validated = match (ops.validate)(servers) {
        Ok(validated) => validated,
        Err(err) => {
            sess.open_error(&err.message());
            return;
        }
    };
    let plan = match (ops.plan)(None, &validated) {
        Ok(plan) => plan,
        Err(err) => {
            sess.open_error(&err.message());
            return;
        }
    };
    let rx = sess.request_confirm(&(ops.prompt)(&plan));
    if !rx.recv().unwrap_or(false) {
        return;
    }
    if let Err(err) = (ops.apply)(&plan) {
        sess.open_error(&err.message());
    }
}

/// DNS reset worker body (runs on the blocking pool, plan 07-04):
/// mirrors `dns_worker` with validation skipped (a reset takes no
/// servers). Plans DHCP restore against the active profile (`None`),
/// confirms with the core reset question verbatim, and only a yes
/// reaches the apply leg - which branches on `plan.reset` to clear
/// `ipv4.dns` and restore DHCP. A decline writes nothing and shows
/// nothing; every failure surfaces as core's readable `message()`
/// in the error modal (D-17).
pub(crate) fn dns_reset_worker(ops: &NetDnsOps, sess: &ModalSession) {
    let plan = match (ops.plan_reset)(None) {
        Ok(plan) => plan,
        Err(err) => {
            sess.open_error(&err.message());
            return;
        }
    };
    let rx = sess.request_confirm(&(ops.prompt)(&plan));
    if !rx.recv().unwrap_or(false) {
        return;
    }
    if let Err(err) = (ops.apply)(&plan) {
        sess.open_error(&err.message());
    }
}

/// Injection seam for the VPN import path: the exact core entry
/// points the worker calls. Production ([`PROD_NET_VPN_OPS`]) runs
/// the stdin-detached import; unit tests pass fakes so the
/// validate-infer-confirm-import sequence is proven without spawning
/// nmcli (T-06-32).
pub(crate) struct NetVpnOps {
    pub validate_path: fn(&str) -> Result<PathBuf, archel_core::ArchelError>,
    pub infer: fn(&Path) -> Option<&'static str>,
    pub prompt: fn(&str, &str) -> String,
    pub import: fn(&str) -> Result<archel_core::network::vpn::VpnImportReport, archel_core::ArchelError>,
}

/// The production wiring: path validation, plugin inference and the
/// import question are the CLI-proven `net vpn import` pieces; the
/// import itself is the stdin-detached twin of the CLI's call.
pub(crate) const PROD_NET_VPN_OPS: NetVpnOps = NetVpnOps {
    validate_path: archel_core::network::vpn::validate_vpn_import_path,
    infer: archel_core::network::vpn::infer_vpn_type,
    prompt: archel_core::network::vpn::confirm_vpn_import_prompt,
    import: archel_core::network::vpn::vpn_import_stdin_null,
};

/// VPN import worker body (runs on the blocking pool). Returns
/// `Some(success line)` on success for the banner, `None` on decline
/// or failure (failures open the error modal with core's message
/// verbatim - traversal and nonexistent paths never reach a confirm).
pub(crate) fn vpn_worker(path: &str, ops: &NetVpnOps, sess: &ModalSession) -> Option<String> {
    let canonical = match (ops.validate_path)(path) {
        Ok(canonical) => canonical,
        Err(err) => {
            sess.open_error(&err.message());
            return None;
        }
    };
    let vpn_type = match (ops.infer)(&canonical) {
        Some(vpn_type) => vpn_type,
        None => {
            sess.open_error(
                &archel_core::ArchelError::VpnPathInvalid {
                    path: path.to_string(),
                }
                .message(),
            );
            return None;
        }
    };
    let rx = sess.request_confirm(&(ops.prompt)(path, vpn_type));
    if !rx.recv().unwrap_or(false) {
        return None;
    }
    match (ops.import)(path) {
        Ok(report) => Some(vpn_success_line(&report)),
        Err(err) => {
            sess.open_error(&err.message());
            None
        }
    }
}

/// Success banner line for an imported tunnel: ASCII only (D-07) -
/// QML renders the surrounding French sentence from its own copy.
/// Carries the inferred plugin plus the connection name nmcli
/// reported, so the user can find the new tunnel in the list.
pub(crate) fn vpn_success_line(report: &archel_core::network::vpn::VpnImportReport) -> String {
    match &report.connection_name {
        Some(name) => format!("{} ({})", name, report.vpn_type),
        None => format!("import ok ({})", report.vpn_type),
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

    // -- Write-path choreography (plan 06-07, Task 2) --
    //
    // The fakes below replace only the `NetDnsOps`/`NetVpnOps` seams;
    // the workers themselves run for real against a local
    // ModalSession, so the validate-plan-confirm-apply and
    // validate-infer-confirm-import sequences are proven without
    // touching NM or spawning nmcli.

    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use archel_core::network::dns::{DnsPlan, DnsReport};
    use archel_core::network::vpn::VpnImportReport;

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

    fn plan_fixture() -> DnsPlan {
        DnsPlan {
            profile_name: "home".to_string(),
            profile_uuid: "uuid-1".to_string(),
            old_servers: Vec::new(),
            reset: false,
            new_servers: vec!["1.1.1.1".to_string()],
        }
    }

    fn report_fixture() -> DnsReport {
        DnsReport {
            profile_name: "home".to_string(),
            profile_uuid: "uuid-1".to_string(),
            old_servers: Vec::new(),
            new_servers: vec!["1.1.1.1".to_string()],
            effective_dns: Some(vec!["1.1.1.1".to_string()]),
            reactivated: true,
        }
    }

    #[test]
    fn invalid_dns_is_rejected_before_any_plan_or_apply() {
        static PLAN_CALLS: AtomicUsize = AtomicUsize::new(0);
        static APPLY_CALLS: AtomicUsize = AtomicUsize::new(0);
        fn validate_rejects(servers: &[String]) -> Result<Vec<String>, archel_core::ArchelError> {
            assert_eq!(servers, &["999.1.1.1".to_string()]);
            Err(archel_core::ArchelError::DnsInvalid {
                server: "999.1.1.1".to_string(),
            })
        }
        fn plan_must_not_run(
            _profile: Option<&str>,
            _servers: &[String],
        ) -> Result<DnsPlan, archel_core::ArchelError> {
            PLAN_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(plan_fixture())
        }
        fn apply_must_not_run(_plan: &DnsPlan) -> Result<DnsReport, archel_core::ArchelError> {
            APPLY_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(report_fixture())
        }
        let ops = NetDnsOps {
            validate: validate_rejects,
            plan: plan_must_not_run,
            plan_reset: archel_core::network::dns::plan_dns_reset,
            prompt: archel_core::network::dns::confirm_dns_prompt,
            apply: apply_must_not_run,
        };
        let sess = ModalSession::default();
        dns_worker(&["999.1.1.1".to_string()], &ops, &sess);
        assert_eq!(sess.snapshot_kind(), ModalKind::Error);
        assert_eq!(
            sess.snapshot_error(),
            archel_core::ArchelError::DnsInvalid {
                server: "999.1.1.1".to_string(),
            }
            .message()
        );
        assert_eq!(PLAN_CALLS.load(AtomicOrdering::SeqCst), 0);
        assert_eq!(APPLY_CALLS.load(AtomicOrdering::SeqCst), 0);
    }

    #[test]
    fn dns_decline_asks_the_plan_question_verbatim_and_never_applies() {
        static APPLY_CALLS: AtomicUsize = AtomicUsize::new(0);
        fn validate_echo(servers: &[String]) -> Result<Vec<String>, archel_core::ArchelError> {
            Ok(servers.to_vec())
        }
        fn plan_fixture_fn(
            _profile: Option<&str>,
            _servers: &[String],
        ) -> Result<DnsPlan, archel_core::ArchelError> {
            Ok(plan_fixture())
        }
        fn apply_must_not_run(_plan: &DnsPlan) -> Result<DnsReport, archel_core::ArchelError> {
            APPLY_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(report_fixture())
        }
        let ops = NetDnsOps {
            validate: validate_echo,
            plan: plan_fixture_fn,
            plan_reset: archel_core::network::dns::plan_dns_reset,
            prompt: archel_core::network::dns::confirm_dns_prompt,
            apply: apply_must_not_run,
        };
        let sess = ModalSession::default();
        std::thread::scope(|s| {
            s.spawn(|| dns_worker(&["1.1.1.1".to_string()], &ops, &sess));
            wait_until("dns confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            assert_eq!(
                sess.snapshot_question(),
                archel_core::network::dns::confirm_dns_prompt(&plan_fixture()),
                "the question comes from the plan verbatim"
            );
            sess.resolve_confirm(false);
        });
        assert_eq!(APPLY_CALLS.load(AtomicOrdering::SeqCst), 0);
        assert_eq!(sess.snapshot_kind(), ModalKind::None);
    }

    #[test]
    fn dns_accept_applies_the_planned_servers() {
        static APPLIED: Mutex<Vec<String>> = Mutex::new(Vec::new());
        fn validate_echo(servers: &[String]) -> Result<Vec<String>, archel_core::ArchelError> {
            Ok(servers.to_vec())
        }
        fn plan_fixture_fn(
            _profile: Option<&str>,
            _servers: &[String],
        ) -> Result<DnsPlan, archel_core::ArchelError> {
            Ok(plan_fixture())
        }
        fn apply_records(plan: &DnsPlan) -> Result<DnsReport, archel_core::ArchelError> {
            *APPLIED.lock().expect("applied lock") = plan.new_servers.clone();
            Ok(report_fixture())
        }
        let ops = NetDnsOps {
            validate: validate_echo,
            plan: plan_fixture_fn,
            plan_reset: archel_core::network::dns::plan_dns_reset,
            prompt: archel_core::network::dns::confirm_dns_prompt,
            apply: apply_records,
        };
        let sess = ModalSession::default();
        std::thread::scope(|s| {
            s.spawn(|| dns_worker(&["1.1.1.1".to_string()], &ops, &sess));
            wait_until("dns confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            sess.resolve_confirm(true);
            wait_until("dns apply runs", || {
                !APPLIED.lock().expect("applied lock").is_empty()
            });
        });
        assert_eq!(
            *APPLIED.lock().expect("applied lock"),
            vec!["1.1.1.1".to_string()],
            "the planned servers reach the apply leg"
        );
    }

    fn reset_plan_fixture() -> DnsPlan {
        DnsPlan {
            profile_name: "home".to_string(),
            profile_uuid: "uuid-1".to_string(),
            old_servers: vec!["1.1.1.1".to_string()],
            reset: true,
            new_servers: Vec::new(),
        }
    }

    #[test]
    fn dns_reset_accept_applies_the_dhcp_restore() {
        static APPLIED_RESET: Mutex<bool> = Mutex::new(false);
        fn plan_reset_fixture(
            _profile: Option<&str>,
        ) -> Result<DnsPlan, archel_core::ArchelError> {
            Ok(reset_plan_fixture())
        }
        fn apply_records_reset(plan: &DnsPlan) -> Result<DnsReport, archel_core::ArchelError> {
            assert!(plan.reset, "the reset plan must carry the reset flag");
            *APPLIED_RESET.lock().expect("applied lock") = true;
            Ok(report_fixture())
        }
        fn plan_must_not_run(
            _profile: Option<&str>,
            _servers: &[String],
        ) -> Result<DnsPlan, archel_core::ArchelError> {
            panic!("reset takes no servers: the set leg must not run")
        }
        fn validate_must_not_run(
            _servers: &[String],
        ) -> Result<Vec<String>, archel_core::ArchelError> {
            panic!("reset takes no servers: validation must not run")
        }
        let ops = NetDnsOps {
            validate: validate_must_not_run,
            plan: plan_must_not_run,
            plan_reset: plan_reset_fixture,
            prompt: archel_core::network::dns::confirm_dns_prompt,
            apply: apply_records_reset,
        };
        let sess = ModalSession::default();
        std::thread::scope(|s| {
            s.spawn(|| dns_reset_worker(&ops, &sess));
            wait_until("dns reset confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            assert_eq!(
                sess.snapshot_question(),
                archel_core::network::dns::confirm_dns_prompt(&reset_plan_fixture()),
                "the question comes from the reset plan verbatim"
            );
            sess.resolve_confirm(true);
            wait_until("dns reset apply runs", || {
                *APPLIED_RESET.lock().expect("applied lock")
            });
        });
        assert_eq!(sess.snapshot_kind(), ModalKind::None);
    }

    #[test]
    fn dns_reset_decline_writes_nothing() {
        static APPLY_CALLS: AtomicUsize = AtomicUsize::new(0);
        fn plan_reset_fixture(
            _profile: Option<&str>,
        ) -> Result<DnsPlan, archel_core::ArchelError> {
            Ok(reset_plan_fixture())
        }
        fn apply_must_not_run(_plan: &DnsPlan) -> Result<DnsReport, archel_core::ArchelError> {
            APPLY_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(report_fixture())
        }
        fn plan_must_not_run(
            _profile: Option<&str>,
            _servers: &[String],
        ) -> Result<DnsPlan, archel_core::ArchelError> {
            panic!("reset takes no servers: the set leg must not run")
        }
        fn validate_must_not_run(
            _servers: &[String],
        ) -> Result<Vec<String>, archel_core::ArchelError> {
            panic!("reset takes no servers: validation must not run")
        }
        let ops = NetDnsOps {
            validate: validate_must_not_run,
            plan: plan_must_not_run,
            plan_reset: plan_reset_fixture,
            prompt: archel_core::network::dns::confirm_dns_prompt,
            apply: apply_must_not_run,
        };
        let sess = ModalSession::default();
        std::thread::scope(|s| {
            s.spawn(|| dns_reset_worker(&ops, &sess));
            wait_until("dns reset confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            sess.resolve_confirm(false);
        });
        assert_eq!(APPLY_CALLS.load(AtomicOrdering::SeqCst), 0);
        assert_eq!(sess.snapshot_kind(), ModalKind::None);
    }

    #[test]
    fn vpn_bad_path_is_rejected_with_the_core_message_before_any_spawn() {
        static IMPORT_CALLS: AtomicUsize = AtomicUsize::new(0);
        fn import_must_not_run(
            _path: &str,
        ) -> Result<VpnImportReport, archel_core::ArchelError> {
            IMPORT_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(VpnImportReport {
                file: String::new(),
                vpn_type: String::new(),
                connection_name: None,
            })
        }
        let ops = NetVpnOps {
            validate_path: archel_core::network::vpn::validate_vpn_import_path,
            infer: archel_core::network::vpn::infer_vpn_type,
            prompt: archel_core::network::vpn::confirm_vpn_import_prompt,
            import: import_must_not_run,
        };
        let sess = ModalSession::default();
        let outcome = vpn_worker("/tmp/does-not-exist-archel.ovpn", &ops, &sess);
        assert_eq!(outcome, None);
        assert_eq!(sess.snapshot_kind(), ModalKind::Error);
        assert_eq!(
            sess.snapshot_error(),
            archel_core::ArchelError::VpnPathInvalid {
                path: "/tmp/does-not-exist-archel.ovpn".to_string(),
            }
            .message(),
            "traversal/nonexistent paths surface the core message verbatim"
        );
        assert_eq!(IMPORT_CALLS.load(AtomicOrdering::SeqCst), 0);
    }

    #[test]
    fn vpn_accept_imports_and_reports_type_plus_name() {
        static IMPORTED: Mutex<String> = Mutex::new(String::new());
        fn validate_ok(path: &str) -> Result<PathBuf, archel_core::ArchelError> {
            Ok(PathBuf::from(path))
        }
        fn infer_openvpn(_path: &Path) -> Option<&'static str> {
            Some("openvpn")
        }
        fn import_records(path: &str) -> Result<VpnImportReport, archel_core::ArchelError> {
            *IMPORTED.lock().expect("imported lock") = path.to_string();
            Ok(VpnImportReport {
                file: path.to_string(),
                vpn_type: "openvpn".to_string(),
                connection_name: Some("bureau".to_string()),
            })
        }
        let ops = NetVpnOps {
            validate_path: validate_ok,
            infer: infer_openvpn,
            prompt: archel_core::network::vpn::confirm_vpn_import_prompt,
            import: import_records,
        };
        let sess = ModalSession::default();
        let outcome = std::thread::scope(|s| {
            let handle = s.spawn(|| vpn_worker("/tmp/bureau.ovpn", &ops, &sess));
            wait_until("vpn confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            assert_eq!(
                sess.snapshot_question(),
                archel_core::network::vpn::confirm_vpn_import_prompt(
                    "/tmp/bureau.ovpn",
                    "openvpn"
                )
            );
            sess.resolve_confirm(true);
            handle.join().expect("worker joins")
        });
        assert_eq!(outcome.as_deref(), Some("bureau (openvpn)"));
        assert_eq!(
            IMPORTED.lock().expect("imported lock").as_str(),
            "/tmp/bureau.ovpn"
        );
    }
}
