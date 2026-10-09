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
    use qt_build_utils::{QResource, QResourceFile, QResources};

    // Rail icons ship beside Main.qml inside the same qrc. They go through
    // an explicit qrc resource (not qml_files): qmlcachegen compiles every
    // qml_files entry and would choke on .svg. Aliases keep the names
    // QML's relative "images/<name>.svg" sources resolve to.
    let manifest = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("set by cargo"));
    let images = ["home", "network", "power", "bluetooth", "appearance", "sound", "apps",
                  "display", "input", "system"]
        .into_iter()
        .fold(
            QResource::new().prefix("/qt/qml/org/archel/settings/src/gui/qml/images"),
            |res, name| {
                res.file(
                    QResourceFile::new(manifest.join(format!("src/gui/qml/images/{name}.svg")))
                        .alias(format!("{name}.svg")),
                )
            },
        );

    // Every .qml on disk must be registered here in the same commit it
    // appears (Pitfall 2): an unregistered file compiles fine and fails
    // only at runtime with a qrc load error. All 10 tab pages and all 9
    // shared components are pre-registered as placeholders now, so later
    // GUI plans fill content without ever touching this file.
    //
    // The SVG resource rides the same qrc through qrc_resources (not a
    // .qrc file on disk): at runtime the icons resolve beside Main.qml.
    let resources = QResources::new().resource(images);
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
    .files([
        "src/gui/shell.rs",
        "src/gui/bridge_appearance.rs",
        "src/gui/bridge_display.rs",
        "src/gui/bridge_input.rs",
        "src/gui/bridge_system.rs",
        "src/gui/bridge_landing.rs",
        "src/gui/bridge_net.rs",
        "src/gui/bridge_power.rs",
        "src/gui/bridge_audio.rs",
        "src/gui/bridge_bt.rs",
    ])
    .qrc_resources(resources)
    .build();
}

#[cfg(not(feature = "gui"))]
fn main() {}
