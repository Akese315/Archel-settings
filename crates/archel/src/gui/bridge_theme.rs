//! Bridge declaration for the Theme view-model (spec 2026-10-11).
//!
//! Thin shim on purpose (QTBUG-93443): the shape lives here, the
//! derivation core and persistence in `models::theme`.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qcolor.h");
        type QColor = cxx_qt_lib::QColor;
    }

    #[auto_cxx_name]
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, dark_mode)]
        #[qproperty(QString, accent_key)]
        #[qproperty(QColor, color_window)]
        #[qproperty(QColor, color_base)]
        #[qproperty(QColor, color_alternate_base)]
        #[qproperty(QColor, color_mid)]
        #[qproperty(QColor, color_button)]
        #[qproperty(QColor, color_text)]
        #[qproperty(QColor, color_bright_text)]
        #[qproperty(QColor, color_placeholder_text)]
        #[qproperty(QColor, color_accent)]
        #[qproperty(QColor, color_highlight)]
        #[qproperty(QColor, color_highlighted_text)]
        #[namespace = "archel_settings"]
        type ThemeModel = super::ThemeModelRust;

        #[qinvokable]
        #[cxx_name = "setDark"]
        fn set_dark(self: Pin<&mut Self>, dark: bool);

        #[qinvokable]
        #[cxx_name = "setAccent"]
        fn set_accent(self: Pin<&mut Self>, key: QString);
    }

    impl cxx_qt::Threading for ThemeModel {}
}

pub use super::models::theme::ThemeModelRust;
