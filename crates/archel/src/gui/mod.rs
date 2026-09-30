//! Graphical settings interface entry (per D-02).
//!
//! Every Qt type in the binary lives behind this module and the `gui`
//! feature (Pitfall 2): headless builds compile neither this module nor
//! any Qt dependency.

mod bridge;

use archel_core::ArchelError;
use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};

/// Launches the settings window (Phase 1 spike: vertical tabs plus a
/// label ticking at 1 Hz, per the D-14 success bar).
pub fn run_settings() -> Result<(), ArchelError> {
    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from(
            "qrc:/qt/qml/org/archel/spike/src/gui/qml/Spike.qml",
        ));
    }
    if let Some(app) = app.as_mut() {
        app.exec();
    }
    Ok(())
}
