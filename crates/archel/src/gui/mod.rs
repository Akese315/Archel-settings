//! Graphical settings interface entry (per D-02).
//!
//! Every Qt type in the binary lives behind this module and the `gui`
//! feature (Pitfall 2): headless builds compile neither this module nor
//! any Qt dependency.

mod bridge;

use std::sync::OnceLock;

use archel_core::ArchelError;
use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QQuickStyle, QString, QUrl};

/// The GUI-owned async runtime, built in `run_settings()` BEFORE any Qt
/// object exists (per RESEARCH Pattern 2). Bridge invokables reach it via
/// `RUNTIME.get()` and `spawn` polls onto it from the Qt thread; `app.exec()`
/// itself never occupies a runtime worker.
pub(crate) static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

/// Shell window URI inside the compiled QML module. The qrc is generated
/// from build.rs's `org.archel.settings` file list — every .qml must be
/// registered there in the same commit it appears (Pitfall 2).
const MAIN_QML: &str = "qrc:/qt/qml/org/archel/settings/src/gui/qml/Main.qml";

/// Launches the settings shell window (D-22: 200px vertical rail of 10
/// tabs over a StackLayout; D-24: style follows the system hint).
pub fn run_settings() -> Result<(), ArchelError> {
    // Step 1: manual runtime before Qt (per RESEARCH Pattern 2) — the
    // entry in main.rs guarantees no runtime exists at this point.
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|err| ArchelError::Internal(format!("async runtime unavailable: {err}")))?;
    let _ = RUNTIME.set(runtime);

    // Step 2: style (D-24, A3). With a platform theme present Qt resolves
    // the style from it; otherwise fall back to Fusion. Must run before
    // any QML importing QtQuick.Controls is loaded.
    if std::env::var_os("QT_QPA_PLATFORMTHEME").is_none() {
        QQuickStyle::set_style(&QString::from("Fusion"));
    }

    // Step 3: Qt shell. The Option-guard stays (spike-proven): a refused
    // Qt start (no display, missing platform plugin) yields None rather
    // than a crash, and exec() is simply skipped.
    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from(MAIN_QML));
    }
    if let Some(app) = app.as_mut() {
        app.exec();
    }
    Ok(())
}
