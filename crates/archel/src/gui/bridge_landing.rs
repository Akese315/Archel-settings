//! Bridge declaration for the Landing view-model (D-07).
//!
//! Thin shim on purpose (QTBUG-93443: every bridge file of the QmlModule
//! must sit in `src/gui/`): the shape lives here, the backing struct and
//! all behavior in `models::landing`. Later tab plans copy this split.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, version_text)]
        #[qproperty(QString, kernel_text)]
        #[qproperty(QString, updates_text)]
        #[qproperty(QString, notes_text)]
        #[qproperty(bool, banner_visible)]
        #[namespace = "archel_settings"]
        type LandingModel = super::LandingModelRust;

        #[qinvokable]
        #[cxx_name = "tick"]
        fn tick(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "dismissBanner"]
        fn dismiss_banner(self: Pin<&mut Self>);
    }

    impl cxx_qt::Threading for LandingModel {}
}

// Re-exported so the bridge alias keeps its mandatory `super::T` shape
// (cxx-qt-gen rejects longer paths); the struct itself and all behavior
// live in `models::landing`.
pub use super::models::landing::LandingModelRust;
