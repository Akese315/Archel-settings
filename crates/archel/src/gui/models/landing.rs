//! Landing view-model: cached update counts plus version/kernel
//! snapshot (D-07).
//!
//! STUB (06-03-T1): backing struct plus no-op invokables so the tree
//! stays green; T3 fills the poll body and the tests.

use cxx_qt_lib::QString;

/// Backing state (T3: guard, banner policy).
pub struct LandingModelRust {
    pub(crate) version_text: QString,
    pub(crate) kernel_text: QString,
    pub(crate) updates_text: QString,
    pub(crate) notes_text: QString,
    pub(crate) banner_visible: bool,
}

impl Default for LandingModelRust {
    fn default() -> Self {
        Self {
            version_text: QString::from(""),
            kernel_text: QString::from(""),
            updates_text: QString::from(""),
            notes_text: QString::from(""),
            banner_visible: false,
        }
    }
}

impl crate::gui::bridge_landing::qobject::LandingModel {
    /// STUB: replaced by the spawn_blocking poll in T3.
    pub fn tick(self: core::pin::Pin<&mut Self>) {
        let _ = self;
    }

    /// STUB: replaced by the banner dismiss in T3.
    pub fn dismiss_banner(self: core::pin::Pin<&mut Self>) {
        let _ = self;
    }
}
