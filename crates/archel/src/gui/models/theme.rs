//! Theme view-model: derivation of the 12 QPalette token sets (2 modes
//! x 6 accents) plus theme.json persistence (spec 2026-10-11).
//!
//! Pure derivation core (no Qt types) so contrast and completeness are
//! unit-testable headless; QColor conversion and the ThemeModel
//! qobject live at the bottom of the file.

use std::path::{Path, PathBuf};

use cxx_qt_lib::{QColor, QString};
use serde::{Deserialize, Serialize};

/// Save file name, beside window-geometry.json in ~/.config/archel/.
pub(crate) const FILE_NAME: &str = "theme.json";
/// Startup default: dark mode (spec direction C).
pub(crate) const DEFAULT_DARK: bool = true;
/// Startup default accent.
pub(crate) const DEFAULT_ACCENT: &str = "blue";

/// RGBA token, 0-255 per channel. Pure data: no Qt types in the
/// derivation core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Token {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Token {
    /// Fully opaque color from 8-bit RGB.
    pub(crate) const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Same hue, custom alpha (tinted surface over the base).
    pub(crate) const fn with_alpha(self, a: u8) -> Self {
        Self { r: self.r, g: self.g, b: self.b, a }
    }
}

/// One accent in its four roles: fill on dark surfaces, accent-clair
/// text on dark, fill tone for light mode, and the darker light-mode
/// text tone (latte hues darkened for AA contrast on white).
pub(crate) struct AccentDef {
    pub key: &'static str,
    /// Full accent color on dark surfaces (e.g. button fill, gauges).
    pub dark_full: Token,
    /// Accent-clair: text/label tone on dark surfaces.
    pub dark_text: Token,
    /// Fill color on light surfaces (latte hue).
    pub light_full: Token,
    /// Text tone on light surfaces: darker than light_full so values
    /// stay readable (>= 4.5:1 on the light window).
    pub light_text: Token,
}

/// The six catppuccin accents the user picked (accent-color.html mockup).
pub(crate) const ACCENTS: [AccentDef; 6] = [
    AccentDef { key: "blue",  dark_full: Token::rgb(137,180,250), dark_text: Token::rgb(188,208,250), light_full: Token::rgb(30,102,245),  light_text: Token::rgb(23,68,179) },
    AccentDef { key: "mauve", dark_full: Token::rgb(203,166,247), dark_text: Token::rgb(216,194,250), light_full: Token::rgb(136,57,239),  light_text: Token::rgb(107,41,184) },
    AccentDef { key: "green", dark_full: Token::rgb(166,227,161), dark_text: Token::rgb(198,233,195), light_full: Token::rgb(64,160,43),   light_text: Token::rgb(47,125,31) },
    AccentDef { key: "teal",  dark_full: Token::rgb(148,226,213), dark_text: Token::rgb(185,236,226), light_full: Token::rgb(23,146,153),  light_text: Token::rgb(15,111,116) },
    AccentDef { key: "pink",  dark_full: Token::rgb(245,194,231), dark_text: Token::rgb(243,211,236), light_full: Token::rgb(234,118,203), light_text: Token::rgb(176,54,140) },
    AccentDef { key: "peach", dark_full: Token::rgb(250,179,135), dark_text: Token::rgb(251,203,176), light_full: Token::rgb(254,100,11),   light_text: Token::rgb(180,72,20) },
];

/// Every accent key, in the swatch order of the Appearance tab.
pub(crate) const ACCENT_KEYS: [&str; 6] = ["blue", "mauve", "green", "teal", "pink", "peach"];

/// Resolves an accent key; unknown keys fall back to the default so a
/// hand-edited theme.json can never poison the palette.
pub(crate) fn accent(key: &str) -> &'static AccentDef {
    ACCENTS
        .iter()
        .find(|a| a.key == key)
        .unwrap_or(&ACCENTS[0])
}

/// The 11 palette roles the ThemeModel exposes to QML (mapped onto
/// QPalette roles; see spec section 1 table).
#[derive(Debug, Clone, Copy)]
pub(crate) struct ThemePalette {
    pub window: Token,
    pub base: Token,
    pub alternate_base: Token,
    pub mid: Token,
    pub button: Token,
    pub text: Token,
    pub bright_text: Token,
    pub placeholder_text: Token,
    pub accent: Token,
    pub highlight: Token,
    pub highlighted_text: Token,
}

/// Derives the token set for one mode/accent combination. Direction C
/// (spec): near-black base with white-alpha surfaces in dark mode,
/// latte-style light surfaces otherwise.
///
/// Selection contrast: `highlighted_text` over the composited
/// `highlight` tint is consciously accepted at ~3.9-4.4:1 in light
/// mode (selected rows also carry the filled accent indicator), so
/// no composite-aware test is asserted.
pub(crate) fn palette(dark: bool, key: &str) -> ThemePalette {
    let acc = accent(key);
    if dark {
        let white = Token::rgb(255, 255, 255);
        let text = Token::rgb(238, 240, 246);
        ThemePalette {
            window: Token::rgb(16, 16, 24),
            base: white.with_alpha(13),
            alternate_base: white.with_alpha(26),
            mid: white.with_alpha(18),
            button: white.with_alpha(15),
            text,
            bright_text: acc.dark_text,
            placeholder_text: Token::rgb(139, 143, 163),
            accent: acc.dark_full,
            highlight: acc.dark_full.with_alpha(41),
            highlighted_text: acc.dark_text,
        }
    } else {
        let black = Token::rgb(0, 0, 0);
        let text = Token::rgb(28, 29, 34);
        ThemePalette {
            window: Token::rgb(242, 243, 247),
            base: Token::rgb(255, 255, 255),
            alternate_base: Token::rgb(232, 234, 242),
            mid: black.with_alpha(20),
            button: Token::rgb(255, 255, 255),
            text,
            bright_text: acc.light_text,
            placeholder_text: Token::rgb(108, 111, 124),
            accent: acc.light_full,
            highlight: acc.light_full.with_alpha(46),
            highlighted_text: acc.light_text,
        }
    }
}

/// WCAG relative luminance (0..1) of an opaque token.
fn luminance(t: Token) -> f64 {
    let lin = |v: u8| {
        let c = v as f64 / 255.0;
        if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * lin(t.r) + 0.7152 * lin(t.g) + 0.0722 * lin(t.b)
}

/// WCAG contrast ratio (1..21) between two opaque tokens.
pub(crate) fn contrast(a: Token, b: Token) -> f64 {
    let la = luminance(a);
    let lb = luminance(b);
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

// ── theme.json persistence (mirrors geometry.rs) ──────────────────

/// Saved theme. Plain serde struct; unknown accent keys are sanitized
/// on load, never on write.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemePrefs {
    pub dark: bool,
    pub accent: String,
}

impl Default for ThemePrefs {
    fn default() -> Self {
        Self { dark: DEFAULT_DARK, accent: DEFAULT_ACCENT.to_string() }
    }
}

/// Where the save file lives (~/.config/archel/theme.json).
pub fn path() -> PathBuf {
    directories::ProjectDirs::from("", "archel", "archel")
        .map(|dirs| dirs.config_dir().join(FILE_NAME))
        .unwrap_or_else(|| PathBuf::from(FILE_NAME))
}

/// Reads the saved theme. Missing file, unreadable bytes or unparseable
/// JSON all return `None`: the caller falls back to the default
/// (dark + blue), exactly like geometry.rs. A parseable theme with an
/// unknown accent key also reads as "no save" (sanitize drops it).
pub fn load(path: &Path) -> Option<ThemePrefs> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok().and_then(ThemePrefs::sanitize)
}

/// Persists the theme atomically (temp file + rename); failures are
/// swallowed — losing the remember is cosmetic (geometry.rs pattern).
pub fn save(path: &Path, prefs: &ThemePrefs) {
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    let Ok(payload) = serde_json::to_string(prefs) else { return };
    let tmp = path.with_extension("tmp");
    if std::fs::write(&tmp, &payload).is_err() {
        return;
    }
    let _ = std::fs::rename(&tmp, path);
}

impl ThemePrefs {
    /// Drops unknown accent keys (hand-edited file) to the default.
    pub fn sanitize(self) -> Option<Self> {
        if ACCENT_KEYS.contains(&self.accent.as_str()) {
            Some(self)
        } else {
            None
        }
    }
}

// ── ThemeModel (CXX-Qt qobject backing) ───────────────────────────

/// Saved theme, loaded once in `run_settings()` before `engine.load()`
/// so the model is correct the moment QML instantiates it (geometry.rs
/// STARTUP pattern).
pub(crate) static STARTUP: std::sync::OnceLock<Option<ThemePrefs>> =
    std::sync::OnceLock::new();

/// Converts a pure token into a QColor (f32 components, QColor's API).
fn to_qcolor(t: Token) -> QColor {
    QColor::from_rgba_f(
        t.r as f32 / 255.0,
        t.g as f32 / 255.0,
        t.b as f32 / 255.0,
        t.a as f32 / 255.0,
    )
}

/// Backing struct: current state plus the 11 derived palette colors.
/// All mutation flows through `apply_prefs()`; the generated setters
/// emit the notify signals that make every QML palette binding
/// re-evaluate.
pub struct ThemeModelRust {
    pub(crate) dark_mode: bool,
    pub(crate) accent_key: QString,
    pub(crate) color_window: QColor,
    pub(crate) color_base: QColor,
    pub(crate) color_alternate_base: QColor,
    pub(crate) color_mid: QColor,
    pub(crate) color_button: QColor,
    pub(crate) color_text: QColor,
    pub(crate) color_bright_text: QColor,
    pub(crate) color_placeholder_text: QColor,
    pub(crate) color_accent: QColor,
    pub(crate) color_highlight: QColor,
    pub(crate) color_highlighted_text: QColor,
}

impl Default for ThemeModelRust {
    fn default() -> Self {
        let prefs = STARTUP.get().and_then(|p| p.clone()).unwrap_or_default();
        Self::from_prefs(prefs.dark, &prefs.accent)
    }
}

impl ThemeModelRust {
    fn from_prefs(dark: bool, key: &str) -> Self {
        let p = palette(dark, key);
        Self {
            dark_mode: dark,
            accent_key: QString::from(key),
            color_window: to_qcolor(p.window),
            color_base: to_qcolor(p.base),
            color_alternate_base: to_qcolor(p.alternate_base),
            color_mid: to_qcolor(p.mid),
            color_button: to_qcolor(p.button),
            color_text: to_qcolor(p.text),
            color_bright_text: to_qcolor(p.bright_text),
            color_placeholder_text: to_qcolor(p.placeholder_text),
            color_accent: to_qcolor(p.accent),
            color_highlight: to_qcolor(p.highlight),
            color_highlighted_text: to_qcolor(p.highlighted_text),
        }
    }
}

impl crate::gui::bridge_theme::qobject::ThemeModel {
    /// Applies the mode switch: recompute + notify + persist.
    /// Runs entirely on the Qt thread (pure property work, no spawn).
    /// Property reads go through `rust_mut()` — the proven models
    /// pattern (net.rs `dns_busy`) — writes through the generated
    /// setters that emit the notify signals.
    pub fn set_dark(mut self: core::pin::Pin<&mut Self>, dark: bool) {
        use cxx_qt::CxxQtType;
        if self.as_mut().rust_mut().dark_mode == dark {
            return;
        }
        let key = self.as_mut().rust_mut().accent_key.to_string();
        apply_prefs(self.as_mut(), dark, key);
    }

    /// Applies the accent switch: recompute + notify + persist.
    /// Unknown keys are refused: the load path rejects them, so the
    /// write path must too (a persisted raw key would reset the whole
    /// prefs — dark flag included — on the next load's sanitize pass).
    pub fn set_accent(mut self: core::pin::Pin<&mut Self>, key: QString) {
        use cxx_qt::CxxQtType;
        let key = key.to_string();
        if !ACCENT_KEYS.contains(&key.as_str()) {
            return;
        }
        let dark = self.as_mut().rust_mut().dark_mode;
        apply_prefs(self.as_mut(), dark, key);
    }
}

/// Shared write path: setters emit the generated notifies, the prefs
/// file is rewritten best-effort (write failure is cosmetic, log-free
/// like geometry.rs).
fn apply_prefs(mut obj: core::pin::Pin<&mut crate::gui::bridge_theme::qobject::ThemeModel>, dark: bool, key: String) {
    let fresh = ThemeModelRust::from_prefs(dark, &key);
    obj.as_mut().set_dark_mode(dark);
    obj.as_mut().set_accent_key(QString::from(key.as_str()));
    obj.as_mut().set_color_window(fresh.color_window);
    obj.as_mut().set_color_base(fresh.color_base);
    obj.as_mut().set_color_alternate_base(fresh.color_alternate_base);
    obj.as_mut().set_color_mid(fresh.color_mid);
    obj.as_mut().set_color_button(fresh.color_button);
    obj.as_mut().set_color_text(fresh.color_text);
    obj.as_mut().set_color_bright_text(fresh.color_bright_text);
    obj.as_mut().set_color_placeholder_text(fresh.color_placeholder_text);
    obj.as_mut().set_color_accent(fresh.color_accent);
    obj.as_mut().set_color_highlight(fresh.color_highlight);
    obj.as_mut().set_color_highlighted_text(fresh.color_highlighted_text);
    let prefs = ThemePrefs { dark, accent: key };
    save(&path(), &prefs);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// window/text stay opaque in every combination; the tints carry
    /// the alpha.
    #[test]
    fn window_and_text_are_opaque_in_all_twelve_combinations() {
        for dark in [true, false] {
            for key in ACCENT_KEYS {
                let p = palette(dark, key);
                assert_eq!(p.window.a, 255, "window must be opaque ({key})");
                assert_eq!(p.text.a, 255, "text must be opaque ({key})");
            }
        }
    }

    /// Main text reaches AAA (>= 7:1) on the window background in both
    /// modes — the two constants that never move with the accent.
    #[test]
    fn main_text_reaches_aaa_contrast_in_both_modes() {
        for dark in [true, false] {
            for key in ACCENT_KEYS {
                let p = palette(dark, key);
                let ratio = contrast(p.text, p.window);
                assert!(ratio >= 7.0, "text/window only {ratio}:1 ({dark}/{key})");
            }
        }
    }

    /// brightText (accent-clair values) stays AA (>= 4.5:1) for every
    /// accent in both modes.
    #[test]
    fn bright_text_reaches_aa_contrast_for_every_accent() {
        for dark in [true, false] {
            for key in ACCENT_KEYS {
                let p = palette(dark, key);
                let ratio = contrast(p.bright_text, p.window);
                assert!(ratio >= 4.5, "brightText/window only {ratio}:1 ({dark}/{key})");
            }
        }
    }

    /// Unknown accent keys can never poison the palette: blue comes back.
    #[test]
    fn unknown_accent_falls_back_to_blue() {
        assert_eq!(palette(true, "nope").accent, palette(true, "blue").accent);
        assert_eq!(palette(false, "nope").accent, palette(false, "blue").accent);
    }

    /// The tinted surfaces are strictly lighter than their base tint in
    /// dark mode (hover must read above rest).
    #[test]
    fn dark_hover_surface_is_stronger_than_rest_surface() {
        let p = palette(true, "blue");
        assert!(p.alternate_base.a > p.base.a);
        assert!(p.highlight.a > 0);
    }

    /// theme.json round trip, mirroring the geometry.rs tests.
    #[test]
    fn theme_prefs_round_trip() {
        let dir = std::env::temp_dir().join(format!(
            "archel-theme-rt-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let file = dir.join(FILE_NAME);
        let prefs = ThemePrefs { dark: false, accent: "mauve".to_string() };
        save(&file, &prefs);
        assert_eq!(load(&file), Some(prefs.clone()));
        assert!(!dir.join("theme.tmp").exists(), "no stray .tmp");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Corrupt file or unknown accent reads as "no save" -> defaults.
    #[test]
    fn theme_prefs_corrupt_or_unknown_accent_falls_back() {
        let dir = std::env::temp_dir().join(format!(
            "archel-theme-bad-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let file = dir.join(FILE_NAME);

        std::fs::write(&file, "not json {{").unwrap();
        assert_eq!(load(&file), None);

        std::fs::write(&file, r#"{"dark":true,"accent":"chartreuse"}"#).unwrap();
        assert_eq!(load(&file), None, "unknown accent must not load");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Defaults are the spec ones: dark + blue.
    #[test]
    fn defaults_are_dark_blue() {
        let d = ThemePrefs::default();
        assert!(d.dark);
        assert_eq!(d.accent, "blue");
    }
}
