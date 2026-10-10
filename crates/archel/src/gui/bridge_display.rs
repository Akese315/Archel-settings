//! Bridge declaration for the Display view-model.
//!
//! Thin shim on purpose (QTBUG-93443: every bridge file of the QmlModule
//! must sit in `src/gui/`): the shape lives here, the backing struct and
//! all behavior in `models::display`.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, monitors_json)]
        #[qproperty(bool, banner_visible)]
        #[qproperty(bool, operation_busy)]
        #[qproperty(QString, operation_kind)]
        #[qproperty(bool, countdown_visible)]
        #[qproperty(i32, countdown_seconds)]
        #[qproperty(QString, countdown_monitor)]
        #[namespace = "archel_settings"]
        type DisplayModel = super::DisplayModelRust;

        #[qinvokable]
        #[cxx_name = "tick"]
        fn tick(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "dismissBanner"]
        fn dismiss_banner(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "planChange"]
        fn plan_change(
            self: Pin<&mut Self>,
            monitor: QString,
            resolution: QString,
            refresh: QString,
            scale: QString,
        );

        #[qinvokable]
        #[cxx_name = "keepChange"]
        fn keep_change(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "revertChange"]
        fn revert_change(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "countdownTick"]
        fn countdown_tick(self: Pin<&mut Self>);
    }

    impl cxx_qt::Threading for DisplayModel {}
}

// Re-exported so the bridge alias keeps its mandatory `super::T` shape
// (cxx-qt-gen rejects longer paths); the struct itself and all behavior
// live in `models::display`.
pub use super::models::display::DisplayModelRust;
