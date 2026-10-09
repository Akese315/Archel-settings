//! Window geometry persistence (D-23, T-06-09).
//!
//! Pure, Qt-free logic so it is unit-testable without a display: the
//! saved rect lives as JSON at
//! `directories::ProjectDirs::from("", "archel", "archel").config_dir()
//! .join("window-geometry.json")`. Reading mirrors `updates.rs::read_cache`
//! (any failure => `None`, the caller falls back), writing mirrors
//! `updates.rs::write_cache` (atomic temp file + rename, failures
//! swallowed). Validation against the live screens happens in
//! [`place`]: a hand-corrupted or off-screen rect is clamped back, and a
//! fully-off-screen rect is replaced by the centered default (Pitfall 4) -
//! the window can never launch invisible.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Width of the fallback window (matches Main.qml defaults, D-23).
pub const DEFAULT_WIDTH: i32 = 900;
/// Height of the fallback window (matches Main.qml defaults, D-23).
pub const DEFAULT_HEIGHT: i32 = 620;

const FILE_NAME: &str = "window-geometry.json";

/// The startup rect, loaded once in `run_settings()` before `engine.load()`
/// and read back by `ShellModel::validate_startup` when QML instantiates it.
pub(crate) static STARTUP: std::sync::OnceLock<Option<Geometry>> =
    std::sync::OnceLock::new();

/// One monitor's rect in the virtual desktop coordinate space. QML passes
/// these as JSON (from `window.screens`) because cxx-qt-lib 0.10 exposes no
/// QScreen wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl ScreenRect {
    fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0
    }
}

/// Saved window frame. Plain serde struct: any field outside i32 range or
/// any non-integer (NaN, null...) fails parsing and counts as "no save".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Geometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub maximized: bool,
}

impl Geometry {
    fn has_usable_size(&self) -> bool {
        self.width > 0 && self.height > 0
    }
}

/// Where the save file lives (`~/.config/archel/window-geometry.json`).
pub fn path() -> PathBuf {
    directories::ProjectDirs::from("", "archel", "archel")
        .map(|dirs| dirs.config_dir().join(FILE_NAME))
        .unwrap_or_else(|| PathBuf::from(FILE_NAME))
}

/// Reads the saved rect. Missing file, unreadable bytes, unparseable JSON
/// or a degenerate size all return `None`: the caller falls back to the
/// centered default instead of crashing (T-06-09).
pub fn load(path: &Path) -> Option<Geometry> {
    let raw = std::fs::read_to_string(path).ok()?;
    let saved: Geometry = serde_json::from_str(&raw).ok()?;
    saved.has_usable_size().then_some(saved)
}

/// Persists the frame atomically (temp file + rename). Write failures are
/// swallowed exactly like `updates.rs::write_cache`: losing the remember
/// is cosmetic, crashing on exit is not. A degenerate size is refused so
/// an odd QML state can never poison the file.
pub fn save(path: &Path, geometry: Geometry) {
    if !geometry.has_usable_size() {
        return;
    }
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    let Ok(payload) = serde_json::to_string(&geometry) else {
        return;
    };
    let tmp = path.with_extension("tmp");
    if std::fs::write(&tmp, &payload).is_err() {
        return;
    }
    let _ = std::fs::rename(&tmp, path);
}

/// Virtual-desktop bounds over the valid screens, `None` when no screen
/// reports a usable rect. Saturating math: hand-edited i32 extremes must
/// not panic the loader.
fn virtual_bounds(screens: &[ScreenRect]) -> Option<ScreenRect> {
    let valid: Vec<ScreenRect> = screens.iter().copied().filter(ScreenRect::is_valid).collect();
    if valid.is_empty() {
        return None;
    }
    let left = valid.iter().map(|s| s.x).min().unwrap_or(0);
    let top = valid.iter().map(|s| s.y).min().unwrap_or(0);
    let right = valid
        .iter()
        .map(|s| s.x.saturating_add(s.width))
        .max()
        .unwrap_or(0);
    let bottom = valid
        .iter()
        .map(|s| s.y.saturating_add(s.height))
        .max()
        .unwrap_or(0);
    Some(ScreenRect {
        x: left,
        y: top,
        width: right.saturating_sub(left),
        height: bottom.saturating_sub(top),
    })
}

/// Centers a `DEFAULT_WIDTH` x `DEFAULT_HEIGHT` window (shrunk to the
/// bounds when they are smaller) inside `bounds`, keeping `maximized`.
fn centered(bounds: Option<ScreenRect>, maximized: bool) -> Geometry {
    let width = DEFAULT_WIDTH.min(bounds.map_or(DEFAULT_WIDTH, |b| b.width.max(1)));
    let height = DEFAULT_HEIGHT.min(bounds.map_or(DEFAULT_HEIGHT, |b| b.height.max(1)));
    match bounds {
        Some(b) => Geometry {
            x: b.x + (b.width - width) / 2,
            y: b.y + (b.height - height) / 2,
            width,
            height,
            maximized,
        },
        None => Geometry {
            x: 0,
            y: 0,
            width,
            height,
            maximized,
        },
    }
}

/// Validates the saved frame against the live screens and returns the
/// rect to launch with (Pitfall 4):
/// - no save, no usable screen, or degenerate size -> centered default;
/// - rect overlapping the virtual bounds -> same size clamped back inside;
/// - rect fully off every screen -> centered default (never invisible),
///   with the saved `maximized` flag preserved in every branch.
pub fn place(saved: Option<Geometry>, screens: &[ScreenRect]) -> Geometry {
    let bounds = virtual_bounds(screens);
    let Some(saved) = saved.filter(Geometry::has_usable_size) else {
        return centered(bounds, false);
    };
    let Some(b) = bounds else {
        return centered(None, saved.maximized);
    };

    let right = saved.x.saturating_add(saved.width);
    let bottom = saved.y.saturating_add(saved.height);
    let overlaps = saved.x < b.x.saturating_add(b.width)
        && right > b.x
        && saved.y < b.y.saturating_add(b.height)
        && bottom > b.y;
    if !overlaps {
        return centered(Some(b), saved.maximized);
    }

    let width = saved.width.min(b.width);
    let height = saved.height.min(b.height);
    Geometry {
        x: saved.x.clamp(b.x, b.x.saturating_add(b.width - width)),
        y: saved.y.clamp(b.y, b.y.saturating_add(b.height - height)),
        width,
        height,
        maximized: saved.maximized,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Fresh scratch dir per test (no dev-deps: hand-rolled like the other
    /// inline suites), cleaned up by the test itself.
    fn scratch_dir(label: &str) -> PathBuf {
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "archel-geometry-{label}-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    fn screens() -> Vec<ScreenRect> {
        vec![ScreenRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }]
    }

    #[test]
    fn geometry_round_trip_restores_the_saved_frame() {
        let dir = scratch_dir("roundtrip");
        let file = dir.join(FILE_NAME);
        let saved = Geometry {
            x: 137,
            y: 84,
            width: 1024,
            height: 700,
            maximized: true,
        };
        save(&file, saved);
        assert_eq!(load(&file), Some(saved), "closed frame must come back");
        assert!(
            !dir.join("window-geometry.tmp").exists(),
            "atomic write must leave no stray .tmp"
        );
        // And placement of the saved rect inside the screens keeps it verbatim.
        assert_eq!(place(Some(saved), &screens()), saved);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn geometry_corrupt_or_zeroed_json_falls_back_to_the_centered_default() {
        let dir = scratch_dir("corrupt");
        let file = dir.join(FILE_NAME);

        std::fs::write(&file, "not json at all {{").unwrap();
        assert_eq!(load(&file), None, "unparseable file reads as no save");

        // i32 fields make any NaN/Infinity/null parse fail -> no save.
        std::fs::write(&file, r#"{"x":NaN,"y":0,"width":900,"height":620,"maximized":false}"#)
            .unwrap();
        assert_eq!(load(&file), None, "NaN must not reach placement");

        // A parsed-but-degenerate size is rejected by the load validator...
        std::fs::write(&file, r#"{"x":10,"y":10,"width":0,"height":620,"maximized":false}"#)
            .unwrap();
        assert_eq!(load(&file), None, "zero width reads as no save");
        // ...and a poisoned file can never be written back.
        save(&file, Geometry::default());
        assert_eq!(load(&file), None, "degenerate saves are refused");

        // All three cases land on the sane 900x620 centered default.
        assert_eq!(
            place(load(&file), &screens()),
            Geometry {
                x: 510,
                y: 230,
                width: 900,
                height: 620,
                maximized: false
            }
        );
        assert!(
            !dir.join("window-geometry.tmp").exists(),
            "atomic write must leave no stray .tmp"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn geometry_off_screen_rects_are_clamped_or_recentred() {
        // Second monitor to the right: virtual bounds 0..3840 x 0..1080.
        let dual = vec![
            ScreenRect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            ScreenRect {
                x: 1920,
                y: 0,
                width: 1920,
                height: 1080,
            },
        ];

        // Partially off (hangs 400px over the right edge) -> clamped back.
        let hanging = Geometry {
            x: 3500,
            y: 100,
            width: 900,
            height: 620,
            maximized: false,
        };
        let fixed = place(Some(hanging), &dual);
        assert_eq!(fixed.x, 3840 - 900, "must sit flush inside the bounds");
        assert_eq!(fixed.y, 100, "in-range axis stays put");

        // Fully off every screen (orphaned laptop-dock position) -> centered
        // default, maximized flag preserved (Pitfall 4, never invisible).
        let gone = Geometry {
            x: 9000,
            y: 9000,
            width: 900,
            height: 620,
            maximized: true,
        };
        assert_eq!(
            place(Some(gone), &dual),
            Geometry {
                x: (3840 - 900) / 2,
                y: (1080 - 620) / 2,
                width: 900,
                height: 620,
                maximized: true
            }
        );

        // No usable screens at all -> deterministic origin default.
        assert_eq!(
            place(None, &[]),
            Geometry {
                x: 0,
                y: 0,
                width: DEFAULT_WIDTH,
                height: DEFAULT_HEIGHT,
                maximized: false
            }
        );
    }
}
