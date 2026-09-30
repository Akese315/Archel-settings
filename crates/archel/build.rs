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

    CxxQtBuilder::new_qml_module(
        QmlModule::new("org.archel.spike").qml_file("src/gui/qml/Spike.qml"),
    )
    .files(["src/gui/bridge.rs"])
    .build();
}

#[cfg(not(feature = "gui"))]
fn main() {}
