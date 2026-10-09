//! Bridge declaration for the Input view-model.
//!
//! Thin shim on purpose (QTBUG-93443: every bridge file of the QmlModule
//! must sit in `src/gui/`): the shape lives here, the backing struct and
//! all behavior in `models::input`.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, sensitivity_text)]
        #[qproperty(QString, natural_text)]
        #[qproperty(QString, scroll_text)]
        #[qproperty(QString, binds_json)]
        #[qproperty(bool, banner_visible)]
        #[namespace = "archel_settings"]
        type InputModel = super::InputModelRust;

        #[qinvokable]
        #[cxx_name = "tick"]
        fn tick(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "dismissBanner"]
        fn dismiss_banner(self: Pin<&mut Self>);
    }

    impl cxx_qt::Threading for InputModel {}
}

// Re-exported so the bridge alias keeps its mandatory `super::T` shape
// (cxx-qt-gen rejects longer paths); the struct itself and all behavior
// live in `models::input`.
pub use super::models::input::InputModelRust;
