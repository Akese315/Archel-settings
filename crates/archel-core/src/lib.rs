//! Archel shared core library.
//!
//! Every system setting (network, power, bluetooth, appearance, audio,
//! packages, display, input, system stats) is exposed through this crate as a
//! programmatic interface. The `archel` binary and the future GUI are thin
//! consumers — all integrations live here.
//!
//! Phase 1 exposes only the foundation seams (version, error taxonomy, i18n);
//! domain modules land with their owning phases.

/// Returns the running `archel-core` library version.
///
/// The `archel version` command calls this to prove the bin -> core link:
/// the printed core version always matches the linked library, never a
/// hardcoded string in the binary.
pub fn library_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
