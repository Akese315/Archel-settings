//! Bridge declaration for the Network view-model.
//!
//! Thin shim on purpose (QTBUG-93443: every bridge file of the QmlModule
//! must sit in `src/gui/`): the shape lives here, the backing struct and
//! all behavior in `models::net`.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, status_text)]
        #[qproperty(QString, iface_text)]
        #[qproperty(QString, ssid_text)]
        #[qproperty(QString, ipv4_text)]
        #[qproperty(QString, gateway_text)]
        #[qproperty(QString, dns_text)]
        #[qproperty(QString, profile_text)]
        #[qproperty(QString, conns_json)]
        #[qproperty(QString, vpn_json)]
        #[qproperty(bool, banner_visible)]
        #[qproperty(bool, dns_busy)]
        #[qproperty(bool, vpn_busy)]
        #[qproperty(QString, vpn_result_text)]
        #[namespace = "archel_settings"]
        type NetModel = super::NetModelRust;

        #[qinvokable]
        #[cxx_name = "tick"]
        fn tick(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "dismissBanner"]
        fn dismiss_banner(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "dismissVpnResult"]
        fn dismiss_vpn_result(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "dnsValidate"]
        fn dns_validate(self: Pin<&mut Self>, servers_json: QString) -> QString;

        #[qinvokable]
        #[cxx_name = "dnsApply"]
        fn dns_apply(self: Pin<&mut Self>, servers_json: QString);

        #[qinvokable]
        #[cxx_name = "dnsReset"]
        fn dns_reset(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "vpnImportStart"]
        fn vpn_import_start(self: Pin<&mut Self>, path: QString);
    }

    impl cxx_qt::Threading for NetModel {}
}

// Re-exported so the bridge alias keeps its mandatory `super::T` shape
// (cxx-qt-gen rejects longer paths); the struct itself and all behavior
// live in `models::net`.
pub use super::models::net::NetModelRust;
