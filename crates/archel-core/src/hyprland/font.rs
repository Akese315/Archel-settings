//! System font via GTK gsettings plus Hyprland snippet (APP-02, D-02).
//!
//! Two stores, one command: `gsettings org.gnome.desktop.interface
//! font-name` (GTK side, `'<family> [size]'` single-quoted value) plus
//! the `misc.font_family` leaf in the app snippet, live-applied via
//! `hyprctl eval 'hl.config({ misc = { font_family = ... } })'`
//! (D-03/D-07 wording maps to eval per the tracer deviation —
//! `hyprctl keyword` is dead on Hyprland 0.56.2). Qt (qt6ct,
//! kdeglobals) is explicitly out of scope and never touched.
//!
//! Every string reaching `eval` or snippet text routes through
//! [`crate::hyprland::lua_quote`] (T-05-07); families validate to a
//! strict charset BEFORE any spawn, read, or write (WR-02:
//! validate first, read second).
//!
//! Spawn discipline mirrors [`crate::proxy`]: which-gate, exact argv
//! never through a shell, `LC_ALL=C`, `wait_timeout` plus kill. A
//! missing `gsettings` binary maps to the typed
//! [`crate::ArchelError::HyprlandUnavailable`] (session-tool family,
//! never a panic) — glib2 ships it on every Arch desktop, so the leg
//! is effectively infallible.

use serde::Serialize;

use super::appearance::{current_snippet_state, render_snippet};
use super::{
    ensure_require_line_at, hypr_dir, lua_quote, parse_getoption_json, run_hyprctl_eval,
    write_snippet_at, OptionValue,
};
use crate::error::ArchelError;
use crate::packages::QUERY_TIMEOUT;

/// gsettings schema owning the GTK font value.
pub const FONT_SCHEMA: &str = "org.gnome.desktop.interface";
/// gsettings key owning the GTK font value.
pub const FONT_KEY: &str = "font-name";
/// Live option key for the Hyprland font leaf (verified via getoption).
pub const FONT_FAMILY_KEY: &str = "misc:font_family";
/// Longest accepted font family (sanity cap before any spawn).
pub const MAX_FAMILY_LEN: usize = 128;
/// Longest accepted `'<family> [size]'` spec (sanity cap).
pub const MAX_SPEC_LEN: usize = 160;

/// Validates one font family: non-empty, strict charset (letters,
/// digits, space, `.`, `_`, `-`). Control characters, quotes,
/// backslashes, and newlines fail typed BEFORE anything spawns,
/// reads, or writes — the value re-enters Lua for `misc:font_family`,
/// so the allowlist is the injection gate (T-05-07).
pub fn validate_font_family(raw: &str) -> Result<String, ArchelError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ArchelError::desktop_invalid("empty font family"));
    }
    if trimmed.len() > MAX_FAMILY_LEN {
        return Err(ArchelError::desktop_invalid(format!(
            "rejected overlong font family {trimmed:?}"
        )));
    }
    if !trimmed.chars().all(|c| {
        c.is_alphanumeric() || c == ' ' || c == '.' || c == '_' || c == '-'
    }) {
        return Err(ArchelError::desktop_invalid(format!(
            "rejected font family {trimmed:?}"
        )));
    }
    Ok(trimmed.to_string())
}

/// Validates one GTK font size: a finite positive float (gsettings
/// accepts fractional point sizes; the compositor side ignores it).
pub fn validate_font_size(raw: &str) -> Result<f32, ArchelError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ArchelError::desktop_invalid("empty font size"));
    }
    let value: f32 = trimmed.parse().map_err(|_| {
        ArchelError::desktop_invalid(format!("rejected font size {trimmed:?}"))
    })?;
    if !value.is_finite() || value <= 0.0 || value > 500.0 {
        return Err(ArchelError::desktop_invalid(format!(
            "font size {trimmed:?} out of range (positive, at most 500)"
        )));
    }
    Ok(value)
}

/// Parses one `'<family> [size]'` spec: a trailing token that parses
/// as a valid size (with at least one family token before it) splits
/// off as the size; otherwise the whole spec is the family. Both
/// halves validate before return — hostile input fails here, never
/// at a spawn.
pub fn parse_font_spec(raw: &str) -> Result<(String, Option<f32>), ArchelError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ArchelError::desktop_invalid("empty font value"));
    }
    if trimmed.len() > MAX_SPEC_LEN {
        return Err(ArchelError::desktop_invalid(format!(
            "rejected overlong font value {trimmed:?}"
        )));
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(ArchelError::desktop_invalid(format!(
            "rejected font value with control characters {trimmed:?}"
        )));
    }
    let mut tokens: Vec<&str> = trimmed.split_whitespace().collect();
    // A trailing numeric token always claims to be the size — even when
    // out of range (failing loudly beats silently adopting "Sans 0" as
    // a family name). A non-numeric tail keeps the whole spec a family.
    let size = if tokens.len() > 1 && tokens[tokens.len() - 1].parse::<f32>().is_ok() {
        let size = validate_font_size(tokens[tokens.len() - 1])?;
        tokens.pop();
        Some(size)
    } else {
        None
    };
    let family = validate_font_family(&tokens.join(" "))?;
    Ok((family, size))
}

/// Renders one GTK `font-name` value from a family plus optional size.
pub fn gtk_font_value(family: &str, size: Option<f32>) -> String {
    match size {
        Some(points) => format!("{family} {}", super::appearance_float(f64::from(points))),
        None => family.to_string(),
    }
}

/// Strips one layer of gsettings single-quote wrapping (`'Adwaita
/// Sans 11'` -> `Adwaita Sans 11`). Pure over captured stdout.
pub fn unquote_gsettings_string(out: &str) -> String {
    let trimmed = out.trim();
    trimmed
        .strip_prefix('\'')
        .and_then(|inner| inner.strip_suffix('\''))
        .unwrap_or(trimmed)
        .to_string()
}

/// Renders the exact `hl.config` Lua statement for a font family.
/// The family routes through [`lua_quote`] — the golden test below
/// pins that production path (IN-02).
pub fn font_eval_statement(family: &str) -> Result<String, ArchelError> {
    Ok(format!(
        "hl.config({{ misc = {{ font_family = {} }} }})",
        lua_quote(family)?
    ))
}

/// Parses a `getoption misc:font_family -j` payload into the live
/// family string. Pure for headless coverage.
pub fn parse_font_family_option(out: &str) -> Result<String, ArchelError> {
    match parse_getoption_json(out)? {
        OptionValue::Str(value) => Ok(value),
        other => Err(ArchelError::hyprland_unavailable(format!(
            "unexpected font option shape: {other:?}"
        ))),
    }
}

/// Runs one `gsettings` invocation and returns its stdout. Same spawn
/// shape as the proxy runner: which-gate with the typed
/// session-unavailable variant, `.unchecked()` so the exit map stays
/// reachable, `LC_ALL=C`, `wait_timeout` plus kill, stderr log-only.
fn run_gsettings(tool: &str, args: &[&str]) -> Result<String, ArchelError> {
    let binary = which::which(tool)
        .map_err(|err| ArchelError::hyprland_unavailable(format!("{tool} ({err})")))?;
    let handle = duct::cmd(&binary, args)
        .unchecked()
        .env("LC_ALL", "C")
        .stdout_capture()
        .stderr_capture()
        .start()
        .map_err(|err| ArchelError::Internal(format!("cannot run '{tool}': {err}")))?;
    let output = match handle.wait_timeout(QUERY_TIMEOUT).map_err(|err| {
        ArchelError::Internal(format!("cannot wait on '{tool}': {err}"))
    })? {
        Some(output) => output.clone(),
        None => {
            let _ = handle.kill();
            return Err(ArchelError::hyprland_unavailable(format!(
                "'{tool}' exceeded the query timeout (session bus unresponsive)"
            )));
        }
    };
    let _stderr_for_logs = String::from_utf8_lossy(&output.stderr);
    match output.status.code() {
        Some(0) => Ok(String::from_utf8_lossy(&output.stdout).into_owned()),
        code => Err(ArchelError::hyprland_unavailable(format!(
            "{tool} query failed with exit code {code:?}"
        ))),
    }
}

/// Reads the GTK font store: `(family, size)` from `gsettings get`.
pub fn read_gtk_font() -> Result<(String, Option<f32>), ArchelError> {
    read_gtk_font_with_tool("gsettings")
}

/// Same as [`read_gtk_font`] but the binary name is injectable, so the
/// which-gate stays unit-coverable headless (proxy.rs:69 precedent).
fn read_gtk_font_with_tool(tool: &str) -> Result<(String, Option<f32>), ArchelError> {
    let out = run_gsettings(tool, &["get", FONT_SCHEMA, FONT_KEY])?;
    parse_font_spec(&unquote_gsettings_string(&out))
}

/// Writes the GTK font store and verifies with a read-back: the
/// re-read value must equal the written one, else typed unavailable
/// (honest confirmation on the GTK leg, mirroring getoption).
pub fn write_gtk_font(value: &str) -> Result<(), ArchelError> {
    let out = run_gsettings("gsettings", &["set", FONT_SCHEMA, FONT_KEY, value])?;
    let _ = out;
    let back = run_gsettings("gsettings", &["get", FONT_SCHEMA, FONT_KEY])?;
    if unquote_gsettings_string(&back) != value {
        return Err(ArchelError::hyprland_unavailable(
            "GTK font read-back disagrees with the applied value",
        ));
    }
    Ok(())
}

/// Reads the live Hyprland font leaf (honest read-back leg).
pub fn read_live_family() -> Result<String, ArchelError> {
    let out = super::run_hyprctl("hyprctl", &["getoption", FONT_FAMILY_KEY, "-j"])?;
    parse_font_family_option(&out)
}

/// Both font stores as one readout: GTK `(family, size)` plus the
/// live Hyprland family. Serialized as-is for `--json`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FontStatus {
    pub gtk_family: String,
    pub gtk_size: Option<f32>,
    pub hyprland_family: String,
}

/// Reads both font stores (bare `archel appearance font`): GTK via
/// gsettings, Hyprland via getoption. Read-only: no confirm, no
/// write, no snippet touch.
pub fn read_font() -> Result<FontStatus, ArchelError> {
    let (gtk_family, gtk_size) = read_gtk_font()?;
    let hyprland_family = read_live_family()?;
    Ok(FontStatus {
        gtk_family,
        gtk_size,
        hyprland_family,
    })
}

/// A validated font transition: family plus optional size, planned
/// read-only before any prompt. The old snippet family feeds the
/// preview; GTK history is read live at display time.
#[derive(Debug, Clone, PartialEq)]
pub struct FontPlan {
    pub family: String,
    pub size: Option<f32>,
    pub old_family: Option<String>,
}

/// Plans a font transition from one `'<family> [size]'` spec.
/// Validates FIRST (WR-02), reads the snippet second through the
/// injectable `base` so tests never touch the live home dir.
pub fn plan_font_set_at(
    base: &std::path::Path,
    spec: &str,
) -> Result<FontPlan, ArchelError> {
    let (family, size) = parse_font_spec(spec)?;
    let old = current_snippet_state(base)?;
    Ok(FontPlan {
        family,
        size,
        old_family: old.font_family,
    })
}

/// Outcome of a font mutation: the applied family plus both
/// confirmations. Serialized as-is for `--json`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FontReport {
    pub family: String,
    pub size: Option<f32>,
    pub gtk_value: String,
    pub read_back_ok: bool,
}

/// Applies a font plan: `eval` live leg plus `getoption` read-back
/// FIRST, GTK store write with its own read-back second, snippet
/// persist only when both legs agree (Task-1 WR-01 ordering extended
/// to the two-store font path). Any refusal returns `Err` with the
/// prior snippet untouched. Injectable legs keep the orchestration
/// unit-coverable headless.
pub fn apply_font_with(
    base: &std::path::Path,
    plan: &FontPlan,
    runner: impl FnOnce(&str) -> Result<(), ArchelError>,
    reader: impl FnOnce() -> Result<String, ArchelError>,
    gtk: impl FnOnce(&str) -> Result<(), ArchelError>,
) -> Result<FontReport, ArchelError> {
    runner(&font_eval_statement(&plan.family)?)?;
    let live = reader()?;
    if live != plan.family {
        return Err(ArchelError::hyprland_unavailable(
            "live read-back disagrees with the applied font family",
        ));
    }
    let value = gtk_font_value(&plan.family, plan.size);
    gtk(&value)?;
    let mut state = current_snippet_state(base)?;
    state.font_family = Some(plan.family.clone());
    write_snippet_at(base, &render_snippet(&state))?;
    ensure_require_line_at(base)?;
    Ok(FontReport {
        family: plan.family.clone(),
        size: plan.size,
        gtk_value: value,
        read_back_ok: true,
    })
}

/// Production leg: snippet under `~/.config/hypr/`, real hyprctl plus
/// real gsettings.
pub fn set_font(spec: &str) -> Result<FontReport, ArchelError> {
    let base = hypr_dir();
    let plan = plan_font_set_at(&base, spec)?;
    apply_font_with(
        &base,
        &plan,
        |lua| run_hyprctl_eval("hyprctl", lua),
        read_live_family,
        write_gtk_font,
    )
}

/// French title for the bare-font readout table.
pub fn font_title() -> String {
    rust_i18n::t!("appearance.font_title").to_string()
}

/// French rows for the bare-font readout table (both stores).
pub fn font_rows(status: &FontStatus) -> Vec<(String, String)> {
    let na = || rust_i18n::t!("sys.unavailable").to_string();
    vec![
        (
            rust_i18n::t!("appearance.font_label_gtk").to_string(),
            status.gtk_family.clone(),
        ),
        (
            rust_i18n::t!("appearance.font_label_size").to_string(),
            status
                .gtk_size
                .map(|size| super::appearance_float(f64::from(size)))
                .unwrap_or_else(na),
        ),
        (
            rust_i18n::t!("appearance.font_label_live").to_string(),
            status.hyprland_family.clone(),
        ),
    ]
}

/// French preview title (shown before `[o/N]`).
pub fn font_preview_title() -> String {
    rust_i18n::t!("appearance.font_preview_title").to_string()
}

/// French `(label, value)` preview rows: new family plus size, and
/// the previous snippet family when one was set.
pub fn font_preview_rows(plan: &FontPlan) -> Vec<(String, String)> {
    let na = || rust_i18n::t!("sys.unavailable").to_string();
    vec![
        (
            rust_i18n::t!("appearance.label_family").to_string(),
            plan.family.clone(),
        ),
        (
            rust_i18n::t!("appearance.label_size").to_string(),
            plan.size
                .map(|size| super::appearance_float(f64::from(size)))
                .unwrap_or_else(na),
        ),
        (
            rust_i18n::t!("appearance.font_label_previous").to_string(),
            plan.old_family.clone().unwrap_or_else(na),
        ),
    ]
}

/// French confirmation prompt naming the target family plus size.
pub fn confirm_font_prompt(plan: &FontPlan) -> String {
    rust_i18n::t!(
        "appearance.confirm_font",
        value = gtk_font_value(&plan.family, plan.size)
    )
    .to_string()
}

/// French title for the post-apply report table.
pub fn font_report_title() -> String {
    rust_i18n::t!("appearance.font_report_title").to_string()
}

/// French rows for the post-apply report table.
pub fn font_report_rows(report: &FontReport) -> Vec<(String, String)> {
    vec![
        (
            rust_i18n::t!("appearance.label_family").to_string(),
            report.family.clone(),
        ),
        (
            rust_i18n::t!("appearance.font_label_gtk").to_string(),
            report.gtk_value.clone(),
        ),
    ]
}

/// French ok line naming the applied family plus size.
pub fn font_ok_line(report: &FontReport) -> String {
    rust_i18n::t!("appearance.font_ok", value = report.gtk_value.clone()).to_string()
}

#[cfg(test)]
mod tests {
    use super::super::appearance::AppearanceState;
    use super::super::{snippet_path_at, ENTRY_FILE};
    use super::*;

    fn scratch_base(tag: &str) -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!("archel-font-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).expect("scratch base must create");
        std::fs::write(base.join(ENTRY_FILE), "-- entry\n").expect("entry seeds");
        base
    }

    #[test]
    fn family_validator_rejects_everything_but_the_allowlist() {
        for good in ["Adwaita Sans", "Cantarell 11", "Noto Sans Mono", "a.b_c-d 12"] {
            // The size suffix is not part of the family: validate the
            // family half the way parse_font_spec splits it.
            let (family, _) = parse_font_spec(good).expect("allowlisted spec parses");
            assert!(!family.is_empty(), "family survives: {good:?}");
        }
        for hostile in [
            "",
            "   ",
            "x\"}) evil( --",
            "a\\b",
            "fam\nily",
            "fam\tily",
            "semi;colon",
            "dollar$var",
            "back`tick",
            "paren(x)",
            "slash/x",
            "colon:x",
        ] {
            assert!(
                validate_font_family(hostile).is_err(),
                "family {hostile:?} must fail validation pre-spawn"
            );
        }
        let err = validate_font_family("x\"}) evil( --").expect_err("quote-break fails");
        assert!(matches!(err, ArchelError::DesktopInvalid { .. }), "{err:?}");
    }

    #[test]
    fn spec_parser_splits_optional_trailing_size() {
        assert_eq!(
            parse_font_spec("Adwaita Sans 11").expect("spec parses"),
            ("Adwaita Sans".to_string(), Some(11.0))
        );
        assert_eq!(
            parse_font_spec("Adwaita Sans").expect("bare family parses"),
            ("Adwaita Sans".to_string(), None)
        );
        assert_eq!(
            parse_font_spec("Noto Sans Mono 10.5").expect("fractional size parses"),
            ("Noto Sans Mono".to_string(), Some(10.5))
        );
        // A lone number is a family-shaped token, not a size: it stays
        // the family (validated, harmless) rather than guessing.
        assert_eq!(
            parse_font_spec("123").expect("numeric token parses"),
            ("123".to_string(), None)
        );
        for bad in ["", "   ", "Adwaita Sans 0", "Adwaita Sans -3", "Adwaita Sans NaN"] {
            assert!(
                parse_font_spec(bad).is_err(),
                "spec {bad:?} must fail validation"
            );
        }
    }

    #[test]
    fn gsettings_unquote_strips_one_quote_layer() {
        assert_eq!(unquote_gsettings_string("'Adwaita Sans 11'"), "Adwaita Sans 11");
        assert_eq!(unquote_gsettings_string("Adwaita Sans"), "Adwaita Sans");
        assert_eq!(unquote_gsettings_string("  'x'  \n"), "x");
    }

    #[test]
    fn font_eval_routes_through_lua_quote_with_hostile_rejection() {
        // IN-02 golden: the production statement is composed from the
        // lua_quote helper output — no parallel quoting path exists.
        let quoted = lua_quote("Adwaita Sans").expect("family quotes");
        assert_eq!(
            font_eval_statement("Adwaita Sans").expect("statement renders"),
            format!("hl.config({{ misc = {{ font_family = {quoted} }} }})")
        );
        for hostile in ["x\"}) evil( --", "a\\b", "fam\nily", "semi;colon"] {
            assert!(
                parse_font_spec(hostile).is_err(),
                "hostile spec {hostile:?} fails validation before any spawn"
            );
        }
        // End-to-end gate: a hostile spec cannot even build a plan, so
        // no runner closure could ever observe it.
        let base = scratch_base("hostile");
        let err = plan_font_set_at(&base, "x\"}) evil( --").expect_err("hostile plan fails");
        assert!(matches!(err, ArchelError::DesktopInvalid { .. }), "{err:?}");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn planner_validates_before_reading_and_stays_hermetic() {
        // Garbage fails on a missing base: validation runs before any
        // read, and no test touches the live home dir (WR-02).
        let missing = std::env::temp_dir().join("archel-font-test-no-such-dir");
        assert!(plan_font_set_at(&missing, "x\"}) evil( --").is_err());
        let base = scratch_base("plan");
        let plan = plan_font_set_at(&base, "Adwaita Sans 11").expect("plan builds");
        assert_eq!(plan.family, "Adwaita Sans");
        assert_eq!(plan.size, Some(11.0));
        assert_eq!(plan.old_family, None, "absent snippet means no previous family");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn apply_persists_only_when_both_legs_agree() {
        use std::cell::RefCell;
        use std::rc::Rc;
        let base = scratch_base("apply");
        // Sibling leaves pre-seeded: the font upsert must preserve them.
        let seeded = AppearanceState {
            blur_enabled: false,
            active_opacity: 0.9,
            inactive_opacity: 0.8,
            font_family: None,
        };
        super::super::write_snippet_at(&base, &render_snippet(&seeded)).expect("seed writes");
        let plan = plan_font_set_at(&base, "Adwaita Sans 11").expect("plan builds");
        let gtk_seen = Rc::new(RefCell::new(String::new()));
        let gtk_probe = Rc::clone(&gtk_seen);
        let report = apply_font_with(
            &base,
            &plan,
            |lua| {
                assert!(
                    lua.contains(&lua_quote("Adwaita Sans").expect("probe quotes")),
                    "eval leg carries the quoted family: {lua:?}"
                );
                Ok(())
            },
            || Ok("Adwaita Sans".to_string()),
            |value| {
                *gtk_probe.borrow_mut() = value.to_string();
                Ok(())
            },
        )
        .expect("agreeing legs carry the apply");
        assert!(report.read_back_ok);
        assert_eq!(gtk_seen.borrow().as_str(), "Adwaita Sans 11");
        let snippet = std::fs::read_to_string(snippet_path_at(&base)).expect("snippet reads");
        for needle in [
            "enabled = false",
            "active_opacity = 0.9",
            "font_family = \"Adwaita Sans\"",
        ] {
            assert!(snippet.contains(needle), "snippet keeps `{needle}`: {snippet:?}");
        }
        // Refused eval: snippet gains no font leaf.
        let before = snippet.clone();
        let err = apply_font_with(
            &base,
            &plan,
            |_| Err(ArchelError::hyprland_unavailable("error: boom")),
            || Ok("Adwaita Sans".to_string()),
            |_| Ok(()),
        )
        .expect_err("refused eval fails");
        assert!(matches!(err, ArchelError::HyprlandUnavailable { .. }));
        assert_eq!(
            std::fs::read_to_string(snippet_path_at(&base)).expect("snippet re-reads"),
            before,
            "refused eval leaves the snippet untouched"
        );
        // Disagreeing read-back: same guarantee.
        let err = apply_font_with(
            &base,
            &plan,
            |_| Ok(()),
            || Ok("Sans".to_string()),
            |_| Ok(()),
        )
        .expect_err("divergent read-back fails");
        assert!(matches!(err, ArchelError::HyprlandUnavailable { .. }));
        assert_eq!(
            std::fs::read_to_string(snippet_path_at(&base)).expect("snippet re-reads"),
            before,
            "divergent read-back leaves the snippet untouched"
        );
        // GTK refusal after hyprland agreement: still no persist.
        let err = apply_font_with(
            &base,
            &plan,
            |_| Ok(()),
            || Ok("Adwaita Sans".to_string()),
            |_| Err(ArchelError::hyprland_unavailable("gsettings down")),
        )
        .expect_err("GTK refusal fails");
        assert!(matches!(err, ArchelError::HyprlandUnavailable { .. }));
        assert_eq!(
            std::fs::read_to_string(snippet_path_at(&base)).expect("snippet re-reads"),
            before,
            "GTK refusal leaves the snippet untouched"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn snippet_font_leaf_round_trips() {
        let state = AppearanceState {
            blur_enabled: true,
            active_opacity: 1.0,
            inactive_opacity: 1.0,
            font_family: Some("Adwaita Sans".to_string()),
        };
        let body = render_snippet(&state);
        assert!(body.contains("misc = { font_family = \"Adwaita Sans\" }"), "{body:?}");
        let base = scratch_base("roundtrip");
        std::fs::create_dir_all(base.join(super::super::SNIPPET_DIR))
            .expect("modules dir creates");
        std::fs::write(snippet_path_at(&base), &body).expect("snippet seeds");
        let back = current_snippet_state(&base).expect("snippet parses");
        assert_eq!(back.font_family, Some("Adwaita Sans".to_string()));
        assert!(back.blur_enabled);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn labels_come_from_the_catalog() {
        crate::i18n::set_language("fr");
        let plan = FontPlan {
            family: "Adwaita Sans".to_string(),
            size: Some(11.0),
            old_family: None,
        };
        assert!(!font_title().is_empty());
        assert!(!font_preview_title().is_empty());
        assert!(!confirm_font_prompt(&plan).is_empty());
        assert!(!font_report_title().is_empty());
        let report = FontReport {
            family: "Adwaita Sans".to_string(),
            size: Some(11.0),
            gtk_value: "Adwaita Sans 11".to_string(),
            read_back_ok: true,
        };
        assert!(!font_ok_line(&report).is_empty());
    }

    #[test]
    fn missing_gsettings_maps_to_typed_unavailable() {
        let err = read_gtk_font_with_tool("archel-no-such-tool").expect_err("missing binary fails");
        assert!(
            matches!(err, ArchelError::HyprlandUnavailable { .. }),
            "missing gsettings is typed unavailable: {err:?}"
        );
    }
}
