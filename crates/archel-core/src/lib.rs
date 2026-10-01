//! Archel shared core library.
//!
//! Every system setting (network, power, bluetooth, appearance, audio,
//! packages, display, input, system stats) is exposed through this crate as a
//! programmatic interface. The `archel` binary and the future GUI are thin
//! consumers — all integrations live here.
//!
//! Phase 1 exposes only the foundation seams (version, error taxonomy, i18n);
//! domain modules land with their owning phases.

pub mod aur;
pub mod error;
pub mod i18n;
pub mod landing;
pub mod packages;
pub mod privileged;
pub mod sensors;
pub mod system;
pub mod updates;

pub use error::{ArchelError, AuthFailureReason};

rust_i18n::i18n!("locales", fallback = "fr");

/// Returns the running `archel-core` library version.
///
/// The `archel version` command calls this to prove the bin -> core link:
/// the printed core version always matches the linked library, never a
/// hardcoded string in the binary.
pub fn library_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// French `archel version` line, rendered from the catalog (D-05/D-07).
/// Takes no input that could surprise it: unexpected core data surfaces as
/// a typed error upstream, never as a default-ok line.
pub fn version_line() -> String {
    rust_i18n::t!(
        "app.version_line",
        bin = env!("CARGO_PKG_VERSION"),
        core = library_version()
    )
    .to_string()
}
