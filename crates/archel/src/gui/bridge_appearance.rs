//! Bridge declaration for the Appearance view-model.
//!
//! Thin shim on purpose (QTBUG-93443: every bridge file of the QmlModule
//! must sit in `src/gui/`): the shape lives here, the backing struct and
//! all behavior in `models::appearance`.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, blur_text)]
        #[qproperty(QString, opacity_text)]
        #[qproperty(QString, wallpaper_text)]
        #[qproperty(QString, font_text)]
        #[qproperty(bool, banner_visible)]
        #[qproperty(bool, operation_busy)]
        #[qproperty(QString, operation_kind)]
        #[namespace = "archel_settings"]
        type AppearanceModel = super::AppearanceModelRust;

        #[qinvokable]
        #[cxx_name = "tick"]
        fn tick(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "dismissBanner"]
        fn dismiss_banner(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "setBlur"]
        fn set_blur(self: Pin<&mut Self>, enabled: bool);

        #[qinvokable]
        #[cxx_name = "setOpacity"]
        fn set_opacity(self: Pin<&mut Self>, active: QString, inactive: QString);

        #[qinvokable]
        #[cxx_name = "setWallpaper"]
        fn set_wallpaper(self: Pin<&mut Self>, path: QString);

        #[qinvokable]
        #[cxx_name = "setFont"]
        fn set_font(self: Pin<&mut Self>, family: QString, size: QString);
    }

    impl cxx_qt::Threading for AppearanceModel {}
}

// Re-exported so the bridge alias keeps its mandatory `super::T` shape
// (cxx-qt-gen rejects longer paths); the struct itself and all behavior
// live in `models::appearance`.
pub use super::models::appearance::AppearanceModelRust;
