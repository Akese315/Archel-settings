//! Per-tab view-models: one bridge module per file (06-RESEARCH Open Q2
//! resolution), so each tab plan edits a disjoint file and wave plans
//! never collide. Later tab plans only append their own `pub mod` line
//! here; registration on the QML side is automatic through
//! `#[qml_element]` plus the build.rs `.files()` list.

pub mod appearance;
pub mod audio;
pub mod bt;
pub mod display;
pub mod input;
pub mod landing;
pub mod net;
pub mod power;
pub mod system;
