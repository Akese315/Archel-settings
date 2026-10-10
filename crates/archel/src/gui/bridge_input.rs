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
        #[qproperty(bool, operation_busy)]
        #[qproperty(QString, operation_kind)]
        #[qproperty(QString, keybind_error)]
        #[namespace = "archel_settings"]
        type InputModel = super::InputModelRust;

        #[qinvokable]
        #[cxx_name = "tick"]
        fn tick(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "dismissBanner"]
        fn dismiss_banner(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "setSensitivity"]
        fn set_sensitivity(self: Pin<&mut Self>, value: QString);

        #[qinvokable]
        #[cxx_name = "setAccelerate"]
        fn set_accelerate(self: Pin<&mut Self>, value: QString);

        #[qinvokable]
        #[cxx_name = "addKeybind"]
        fn add_keybind(self: Pin<&mut Self>, combo: QString, action: QString);

        #[qinvokable]
        #[cxx_name = "removeKeybind"]
        fn remove_keybind(
            self: Pin<&mut Self>,
            mods: QString,
            key: QString,
            dispatcher: QString,
            arg: QString,
        );
    }

    impl cxx_qt::Threading for InputModel {}
}

// Re-exported so the bridge alias keeps its mandatory `super::T` shape
// (cxx-qt-gen rejects longer paths); the struct itself and all behavior
// live in `models::input`.
pub use super::models::input::InputModelRust;
