//! Bridge declaration for the Bluetooth view-model (D-08 bursts).
//!
//! Thin shim on purpose (QTBUG-93443: every bridge file of the QmlModule
//! must sit in `src/gui/`): the shape lives here, the backing struct and
//! all behavior in `models::bt`.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, devices_json)]
        #[qproperty(bool, scanning)]
        #[qproperty(bool, banner_visible)]
        #[qproperty(bool, adapter_on)]
        #[qproperty(bool, operation_busy)]
        #[qproperty(QString, busy_address)]
        #[namespace = "archel_settings"]
        type BtModel = super::BtModelRust;

        #[qinvokable]
        #[cxx_name = "tick"]
        fn tick(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "scanNow"]
        fn scan_now(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "dismissBanner"]
        fn dismiss_banner(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "pair"]
        fn pair(self: Pin<&mut Self>, address: QString);

        #[qinvokable]
        #[cxx_name = "unpair"]
        fn unpair(self: Pin<&mut Self>, address: QString);

        #[qinvokable]
        #[cxx_name = "connect"]
        fn connect(self: Pin<&mut Self>, address: QString);

        #[qinvokable]
        #[cxx_name = "disconnect"]
        fn disconnect(self: Pin<&mut Self>, address: QString);

        #[qinvokable]
        #[cxx_name = "setAdapterOn"]
        fn set_adapter_powered(self: Pin<&mut Self>, on: bool);
    }

    impl cxx_qt::Threading for BtModel {}
}

// Re-exported so the bridge alias keeps its mandatory `super::T` shape
// (cxx-qt-gen rejects longer paths); the struct itself and all behavior
// live in `models::bt`.
pub use super::models::bt::BtModelRust;
