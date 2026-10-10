//! Bridge declaration for the Power view-model (read-only).
//!
//! Thin shim on purpose (QTBUG-93443: every bridge file of the QmlModule
//! must sit in `src/gui/`): the shape lives here, the backing struct and
//! all behavior in `models::power`.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, profile_text)]
        #[qproperty(QString, backend_text)]
        #[qproperty(QString, note_text)]
        #[qproperty(QString, profiles_json)]
        #[qproperty(bool, banner_visible)]
        #[qproperty(bool, operation_busy)]
        #[namespace = "archel_settings"]
        type PowerModel = super::PowerModelRust;

        #[qinvokable]
        #[cxx_name = "tick"]
        fn tick(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "dismissBanner"]
        fn dismiss_banner(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "setProfile"]
        fn set_profile(self: Pin<&mut Self>, profile: QString);
    }

    impl cxx_qt::Threading for PowerModel {}
}

// Re-exported so the bridge alias keeps its mandatory `super::T` shape
// (cxx-qt-gen rejects longer paths); the struct itself and all behavior
// live in `models::power`.
pub use super::models::power::PowerModelRust;
