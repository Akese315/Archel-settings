//! CXX-Qt bridge build (cargo-only path, no cmake — cmake is not installed).
//!
//! Qt is discovered through `QMAKE=/usr/bin/qmake6` (Pitfall 1: the bare
//! `qmake` on PATH is Qt 5.15). The whole Qt build is gated on the `gui`
//! cargo feature so headless builds (`--no-default-features`) never touch
//! Qt headers or link Qt (Pitfall 2): without `gui`, this script is a
//! no-op and `cxx-qt-build` is not even compiled (optional build-dep).

#[cfg(feature = "gui")]
fn main() {
    use cxx_qt_build::{CxxQtBuilder, QmlModule};

    // Every .qml on disk must be registered here in the same commit it
    // appears (Pitfall 2): an unregistered file compiles fine and fails
    // only at runtime with a qrc load error. All 10 tab pages and all 9
    // shared components are pre-registered as placeholders now, so later
    // GUI plans fill content without ever touching this file.
    CxxQtBuilder::new_qml_module(
        QmlModule::new("org.archel.settings").qml_files([
            "src/gui/qml/Main.qml",
            // Rail tabs, locked French order (D-22/D-25).
            "src/gui/qml/tabs/LandingTab.qml",
            "src/gui/qml/tabs/NetTab.qml",
            "src/gui/qml/tabs/PowerTab.qml",
            "src/gui/qml/tabs/BtTab.qml",
            "src/gui/qml/tabs/AppearanceTab.qml",
            "src/gui/qml/tabs/AudioTab.qml",
            "src/gui/qml/tabs/PkgTab.qml",
            "src/gui/qml/tabs/DisplayTab.qml",
            "src/gui/qml/tabs/InputTab.qml",
            "src/gui/qml/tabs/SystemTab.qml",
            // Shared components (modals D-13/D-16/D-17/D-21, widgets).
            "src/gui/qml/components/ConfirmModal.qml",
            "src/gui/qml/components/ErrorModal.qml",
            "src/gui/qml/components/StreamModal.qml",
            "src/gui/qml/components/CountdownModal.qml",
            "src/gui/qml/components/PairingModal.qml",
            "src/gui/qml/components/StatField.qml",
            "src/gui/qml/components/PollBanner.qml",
            "src/gui/qml/components/Sparkline.qml",
            "src/gui/qml/components/VpnImportPopup.qml",
        ]),
    )
    .files(["src/gui/bridge.rs"])
    .build();
}

#[cfg(not(feature = "gui"))]
fn main() {}
