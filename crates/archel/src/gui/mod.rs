//! Graphical settings interface entry (per D-02).
//!
//! Every Qt type in the binary lives behind this module and the `gui`
//! feature (Pitfall 2): headless builds compile neither this module nor
//! any Qt dependency.

mod bridge;
mod geometry;
mod shell;
mod single_instance;

use std::sync::OnceLock;

use archel_core::ArchelError;
use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QQuickStyle, QString, QUrl};

/// The GUI-owned async runtime, built in `run_settings()` BEFORE any Qt
/// object exists (per RESEARCH Pattern 2). Bridge invokables reach it via
/// `RUNTIME.get()` and `spawn` polls onto it from the Qt thread; `app.exec()`
/// itself never occupies a runtime worker.
pub(crate) static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

/// Shell window URI inside the compiled QML module. The qrc is generated
/// from build.rs's `org.archel.settings` file list  -  every .qml must be
/// registered there in the same commit it appears (Pitfall 2).
const MAIN_QML: &str = "qrc:/qt/qml/org/archel/settings/src/gui/qml/Main.qml";

/// Launches the settings shell window (D-22: 200px vertical rail of 10
/// tabs over a StackLayout; D-24: style follows the system hint).
pub fn run_settings() -> Result<(), ArchelError> {
    // Step 1: manual runtime before Qt (per RESEARCH Pattern 2)  -  the
    // entry in main.rs guarantees no runtime exists at this point.
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|err| ArchelError::Internal(format!("async runtime unavailable: {err}")))?;
    let _ = RUNTIME.set(runtime);
    // Entering the context lets the synchronous bind below (and later the
    // bridge invokables' spawns) register with the runtime that owns I/O.
    let _runtime_guard = RUNTIME.get().expect("runtime set above").enter();

    // Step 2: language + single-instance refusal (D-26), both BEFORE any Qt
    // object exists  -  the refused launch prints the catalog line and exits
    // 0 without ever loading QML. The listener is owned here for the whole
    // exec() scope; dropping it is what frees the abstract name.
    archel_core::i18n::set_language("fr");
    let Some(_instance) = single_instance::acquire()? else {
        println!("{}", archel_core::i18n::settings_already_running());
        return Ok(());
    };

    // Step 3: style (D-24, A3). With a platform theme present Qt resolves
    // the style from it; otherwise fall back to Fusion. Must run before
    // any QML importing QtQuick.Controls is loaded.
    if std::env::var_os("QT_QPA_PLATFORMTHEME").is_none() {
        QQuickStyle::set_style(&QString::from("Fusion"));
    }

    // Step 4: Qt shell. The saved frame is loaded (any failure => None,
    // D-23) and stashed BEFORE engine.load() so ShellModel::validate_startup
    // can place the window against the live screens as QML instantiates it.
    // The Option-guards below stay (spike-proven): a refused Qt start (no
    // display, missing platform plugin) yields None rather than a crash,
    // and exec() is simply skipped.
    let _ = geometry::STARTUP.set(geometry::load(&geometry::path()));
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
