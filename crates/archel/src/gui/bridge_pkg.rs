//! Bridge declaration for the Packages view-model.
//!
//! Thin shim on purpose (QTBUG-93443: every bridge file of the QmlModule
//! must sit in `src/gui/`): the shape lives here, the backing struct and
//! all behavior in `models::pkg`. The read half ships in plan 05; plan 06
//! adds write invokables to the same model file.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, installed_json)]
        #[qproperty(QString, search_json)]
        #[qproperty(QString, card_json)]
        #[qproperty(bool, search_busy)]
        #[qproperty(bool, zero_hit)]
        #[qproperty(bool, banner_visible)]
        #[qproperty(bool, operation_busy)]
        #[qproperty(QString, operation_kind)]
        #[namespace = "archel_settings"]
        type PkgModel = super::PkgModelRust;

        #[qinvokable]
        #[cxx_name = "tick"]
        fn tick(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "search"]
        fn search(self: Pin<&mut Self>, query: QString);

        #[qinvokable]
        #[cxx_name = "select"]
        fn select(self: Pin<&mut Self>, name: QString);

        #[qinvokable]
        #[cxx_name = "dismissBanner"]
        fn dismiss_banner(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "installStart"]
        fn install_start(self: Pin<&mut Self>, names_json: QString);

        #[qinvokable]
        #[cxx_name = "upgradeStart"]
        fn upgrade_start(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "removeStart"]
        fn remove_start(self: Pin<&mut Self>, names_json: QString);

        #[qinvokable]
        #[cxx_name = "aurReviewStart"]
        fn aur_review_start(self: Pin<&mut Self>, name: QString);

        #[qinvokable]
        #[cxx_name = "aurBuildStart"]
        fn aur_build_start(self: Pin<&mut Self>);
    }

    impl cxx_qt::Threading for PkgModel {}
}

// Re-exported so the bridge alias keeps its mandatory `super::T` shape
// (cxx-qt-gen rejects longer paths); the struct itself and all behavior
// live in `models::pkg`.
pub use super::models::pkg::PkgModelRust;
