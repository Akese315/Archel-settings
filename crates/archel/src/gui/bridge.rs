//! Rust side of the spike bridge: the `SpikeModel` QObject exposed to QML.
//!
//! Pattern (per the CXX-Qt book `cargo_without_cmake` example): declare the
//! QObject shape in `#[cxx_qt::bridge]`, implement behavior on the generated
//! type. `tick_label` is a Q_PROPERTY with NOTIFY codegen, so the QML label
//! bound to it re-renders every time `tick()` sets it. All user-visible
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
        #[qproperty(QString, tick_label)]
        #[qproperty(i32, tick_count)]
        #[namespace = "archel_spike"]
        type SpikeModel = super::SpikeModelRust;

        #[qinvokable]
        #[cxx_name = "tick"]
        fn tick(self: Pin<&mut Self>);
    }
}

use cxx_qt_lib::QString;

/// Backing state: the label shown in the spike window plus the tick
/// counter it is derived from.
pub struct SpikeModelRust {
    tick_label: QString,
    tick_count: i32,
}

impl Default for SpikeModelRust {
    fn default() -> Self {
        Self {
            tick_label: QString::from("Battements : 0"),
            tick_count: 0,
        }
    }
}

impl qobject::SpikeModel {
    /// Advances the counter and publishes the new label. Called from QML
    /// at 1 Hz; the NOTIFY signal on `tick_label` re-renders the binding.
    pub fn tick(mut self: core::pin::Pin<&mut Self>) {
        let next = *self.as_mut().tick_count() + 1;
        self.as_mut().set_tick_count(next);
        self.as_mut()
            .set_tick_label(QString::from(&format!("Battements : {next}")));
    }
}
