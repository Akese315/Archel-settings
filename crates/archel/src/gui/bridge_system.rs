//! Bridge declaration for the System Info view-model (D-09/D-10/D-11).
//!
//! Thin shim on purpose (QTBUG-93443: every bridge file of the QmlModule
//! must sit in `src/gui/`): the shape lives here, the backing struct and
//! all behavior in `models::system`. Later tab plans copy this split.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    // auto_cxx_name: generated Q_PROPERTY/notify names surface camelCase in QML (explicit cxx_name invokables unaffected).
    #[auto_cxx_name]
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, cpu_text)]
        #[qproperty(QString, clock_text)]
        #[qproperty(QString, cpu_series)]
        #[qproperty(QString, ram_text)]
        #[qproperty(QString, ram_series)]
        #[qproperty(QString, disk_text)]
        #[qproperty(QString, temp_cpu_text)]
        #[qproperty(QString, temp_gpu_text)]
        #[qproperty(QString, gpu_text)]
        #[qproperty(QString, kernel_text)]
        #[qproperty(QString, cores_json)]
        #[qproperty(i32, core_count)]
        #[qproperty(bool, banner_visible)]
        #[namespace = "archel_settings"]
        type SystemModel = super::SystemModelRust;

        #[qinvokable]
        #[cxx_name = "tick"]
        fn tick(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "dismissBanner"]
        fn dismiss_banner(self: Pin<&mut Self>);
    }

    impl cxx_qt::Threading for SystemModel {}
}

// Re-exported so the bridge alias keeps its mandatory `super::T` shape
// (cxx-qt-gen rejects longer paths); the struct itself and all behavior
// live in `models::system`.
pub use super::models::system::SystemModelRust;
