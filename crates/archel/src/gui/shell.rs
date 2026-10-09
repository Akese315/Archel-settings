//! Rust side of the shell bridge: the `ShellModel` QObject Main.qml
//! binds its frame to (D-23).
//!
//! Split from `bridge.rs` (which keeps the retired spike model) because
//! cxx-qt-build 0.10 accepts a single `cxx_qt` module per file, and a
//! single `extern "RustQt"` block accepts a single qobject for `Self`
//! inlining. Same declaration pattern as the spike bridge: shape in
//! `#[cxx_qt::bridge]`, behavior on the generated type. All user-visible
//! wording produced here is plain ASCII (D-07: French diacritics live in
//! the catalog or in QML, never in `.rs` files).

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(i32, launch_x)]
        #[qproperty(i32, launch_y)]
        #[qproperty(i32, launch_width)]
        #[qproperty(i32, launch_height)]
        #[qproperty(bool, launch_maximized)]
        #[namespace = "archel_settings"]
        type ShellModel = super::ShellModelRust;

        #[qinvokable]
        #[cxx_name = "validateStartup"]
        fn validate_startup(self: Pin<&mut Self>, screens_json: QString);

        #[qinvokable]
        #[cxx_name = "saveGeometry"]
        fn save_geometry(
            self: Pin<&mut Self>,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            maximized: bool,
        );
    }
}

use cxx_qt_lib::QString;

use super::geometry;

/// Backing state: the validated frame Main.qml launches with. The defaults
/// match the QML defaults so the window is correct even if
/// `validate_startup` never runs.
pub struct ShellModelRust {
    launch_x: i32,
    launch_y: i32,
    launch_width: i32,
    launch_height: i32,
    launch_maximized: bool,
}

impl Default for ShellModelRust {
    fn default() -> Self {
        Self {
            launch_x: 0,
            launch_y: 0,
            launch_width: geometry::DEFAULT_WIDTH,
            launch_height: geometry::DEFAULT_HEIGHT,
            launch_maximized: false,
        }
    }
}

impl qobject::ShellModel {
    /// Reads the frame stashed by `run_settings()`, validates it against
    /// the screens QML reported (`Qt.application.screens` marshalled as
    /// JSON - cxx-qt-lib 0.10 has no QScreen), and publishes the
    /// placement. A garbage screens string degrades to "no screens":
    /// centered default.
    pub fn validate_startup(mut self: core::pin::Pin<&mut Self>, screens_json: QString) {
        let screens: Vec<geometry::ScreenRect> =
            serde_json::from_str(&screens_json.to_string()).unwrap_or_default();
        let placed = geometry::place(geometry::STARTUP.get().copied().flatten(), &screens);
        self.as_mut().set_launch_x(placed.x);
        self.as_mut().set_launch_y(placed.y);
        self.as_mut().set_launch_width(placed.width);
        self.as_mut().set_launch_height(placed.height);
        self.as_mut().set_launch_maximized(placed.maximized);
    }

    /// Called from `ApplicationWindow.onClosing` with the live window
    /// props; persistence failures are swallowed inside `geometry::save`.
    pub fn save_geometry(
        self: core::pin::Pin<&mut Self>,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        maximized: bool,
    ) {
        geometry::save(
            &geometry::path(),
            geometry::Geometry {
                x,
                y,
                width,
                height,
                maximized,
            },
        );
    }
}
