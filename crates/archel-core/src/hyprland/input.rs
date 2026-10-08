//! Mouse input leaves with eval live-apply (INP-03/INP-04, D-07).
//!
//! Input-section keys in the app snippet, live-applied via
//! `hyprctl eval 'hl.config({ input = { ... } })'` (the plan
//! deviation note: D-07 says `hyprctl keyword`, but keyword is dead
//! on Hyprland 0.56.2 — eval leaf-merge verified live 2026-10-07:
//! sensitivity 0.5→0.75→0.5). Ordering follows the hardened 05-02-01
//! shape: validate first (WR-02), eval plus `getoption -j` read-back
//! first, snippet persist only on agreement (WR-01).
//!
//! Every float reaching `eval` is machine-rendered; sensitivity and
//! scroll factor validate to strict ranges BEFORE any spawn, read, or
//! write. The confirm gate lives core-side (audio precedent):
//! `set_input_options` prompts via `confirm_action` before mutating.

use serde::Serialize;

use super::{
    appearance_float, ensure_require_line_at, hypr_dir, run_hyprctl, run_hyprctl_eval,
    snippet_path_at, write_snippet_at,
};
use crate::error::ArchelError;

/// Live option key for pointer sensitivity (verified via getoption).
pub const SENSITIVITY_KEY: &str = "input:sensitivity";
/// Live option key for touchpad natural scroll (verified present).
pub const NATURAL_SCROLL_KEY: &str = "input:touchpad:natural_scroll";
/// Live option key for touchpad scroll speed (verified present).
pub const SCROLL_FACTOR_KEY: &str = "input:touchpad:scroll_factor";
/// Hyprland sensitivity range (accel-flat semantics, RESEARCH A6).
pub const SENSITIVITY_MIN: f64 = -1.0;
/// Hyprland sensitivity range (accel-flat semantics, RESEARCH A6).
pub const SENSITIVITY_MAX: f64 = 1.0;
/// Sane scroll-factor bounds (compositor default 1.0).
pub const SCROLL_FACTOR_MIN: f64 = 0.1;
/// Sane scroll-factor bounds (compositor default 1.0).
pub const SCROLL_FACTOR_MAX: f64 = 10.0;

/// Every mouse leaf the app snippet manages. Serialized as-is for
/// `--json` (bare `archel input mouse`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InputState {
    /// `input.sensitivity` leaf.
    pub sensitivity: f64,
    /// `input.touchpad.natural_scroll` leaf.
    pub natural_scroll: bool,
    /// `input.touchpad.scroll_factor` leaf.
    pub scroll_factor: f64,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            sensitivity: 0.0,
            natural_scroll: false,
            scroll_factor: 1.0,
        }
    }
}

/// Outcome of a mouse mutation: the applied leaves plus the
/// read-back confirmation. Serialized as-is for `--json`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InputReport {
    pub sensitivity: f64,
    pub natural_scroll: bool,
    pub scroll_factor: f64,
    /// Live read-back agreed with the applied values.
    pub read_back_ok: bool,
}

/// Validates one sensitivity float from CLI text: non-empty, finite,
/// within [-1, 1]. Boundaries accepted; NaN/inf/out-of-range fail
/// typed-DesktopInvalid BEFORE anything spawns, reads, or writes.
pub fn validate_sensitivity(raw: &str) -> Result<f64, ArchelError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ArchelError::desktop_invalid("empty sensitivity value"));
    }
    let value: f64 = trimmed.parse().map_err(|_| {
        ArchelError::desktop_invalid(format!("rejected sensitivity value {trimmed:?}"))
    })?;
    if !value.is_finite() || value < SENSITIVITY_MIN || value > SENSITIVITY_MAX {
        return Err(ArchelError::desktop_invalid(format!(
            "sensitivity {trimmed:?} out of range (-1.0-1.0 accepted)"
        )));
    }
    Ok(value)
}

/// Validates one scroll-factor float from CLI text: non-empty,
/// finite, within [0.1, 10]. Boundaries accepted; NaN/inf and
/// non-positive factors fail typed BEFORE any spawn, read, or write.
pub fn validate_scroll_factor(raw: &str) -> Result<f64, ArchelError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ArchelError::desktop_invalid("empty scroll factor value"));
    }
    let value: f64 = trimmed.parse().map_err(|_| {
        ArchelError::desktop_invalid(format!("rejected scroll factor value {trimmed:?}"))
    })?;
    if !value.is_finite() || value < SCROLL_FACTOR_MIN || value > SCROLL_FACTOR_MAX {
        return Err(ArchelError::desktop_invalid(format!(
            "scroll factor {trimmed:?} out of range (0.1-10.0 accepted)"
        )));
    }
    Ok(value)
}

/// Parses one natural-scroll word: on/off/true/false (case folds).
/// Anything else fails typed-DesktopInvalid naming the value.
pub fn parse_natural_scroll(raw: &str) -> Result<bool, ArchelError> {
    match raw.trim().to_lowercase().as_str() {
        "on" | "true" => Ok(true),
        "off" | "false" => Ok(false),
        _ => Err(ArchelError::desktop_invalid(format!(
            "rejected natural scroll value {raw:?} (on/off accepted)"
        ))),
    }
}

/// A validated mouse transition: at least one leaf set. Planned
/// read-only before any prompt, so typos fail fast with no TTY need.
#[derive(Debug, Clone, PartialEq)]
pub struct InputPlan {
    pub sensitivity: Option<f64>,
    pub natural_scroll: Option<bool>,
    pub scroll_factor: Option<f64>,
    pub old_sensitivity: f64,
    pub old_natural_scroll: bool,
    pub old_scroll_factor: f64,
}

/// Plans a mouse transition from CLI text. Validates FIRST (WR-02),
/// reads the snippet second through the injectable `base` so tests
/// never touch the live home dir. All three `None` (bare `mouse`
/// read, which never reaches here) fails typed.
pub fn plan_input_set_at(
    base: &std::path::Path,
    sensitivity: Option<&str>,
    natural_scroll: Option<&str>,
    scroll_factor: Option<&str>,
) -> Result<InputPlan, ArchelError> {
    let validated_sensitivity = sensitivity.map(validate_sensitivity).transpose()?;
    let validated_natural = natural_scroll.map(parse_natural_scroll).transpose()?;
    let validated_factor = scroll_factor.map(validate_scroll_factor).transpose()?;
    if validated_sensitivity.is_none() && validated_natural.is_none() && validated_factor.is_none()
    {
        return Err(ArchelError::desktop_invalid(
            "mouse set needs --sensitivity, --natural-scroll, and/or --scroll-factor",
        ));
    }
    let old = current_snippet_input(base)?;
    Ok(InputPlan {
        sensitivity: validated_sensitivity,
        natural_scroll: validated_natural,
        scroll_factor: validated_factor,
        old_sensitivity: old.sensitivity,
        old_natural_scroll: old.natural_scroll,
        old_scroll_factor: old.scroll_factor,
    })
}

/// Plans against the live config dir (production leg; the `_at`
/// sibling above is the hermetic, test-covered shape).
pub fn plan_input_set(
    sensitivity: Option<&str>,
    natural_scroll: Option<&str>,
    scroll_factor: Option<&str>,
) -> Result<InputPlan, ArchelError> {
    plan_input_set_at(&hypr_dir(), sensitivity, natural_scroll, scroll_factor)
}

/// Renders the exact `hl.config` Lua statement for an input plan.
/// Pure (golden-testable without spawning): machine-rendered floats
/// and bools only, never interpolated user text.
pub fn input_eval_statement(plan: &InputPlan) -> String {
    let mut input_leaves = Vec::new();
    if let Some(sensitivity) = plan.sensitivity {
        input_leaves.push(format!("sensitivity = {}", appearance_float(sensitivity)));
    }
    let mut touchpad_leaves = Vec::new();
    if let Some(natural_scroll) = plan.natural_scroll {
        touchpad_leaves.push(format!("natural_scroll = {natural_scroll}"));
    }
    if let Some(scroll_factor) = plan.scroll_factor {
        touchpad_leaves.push(format!("scroll_factor = {}", appearance_float(scroll_factor)));
    }
    if !touchpad_leaves.is_empty() {
        input_leaves.push(format!("touchpad = {{ {} }}", touchpad_leaves.join(", ")));
    }
    format!("hl.config({{ input = {{ {} }} }})", input_leaves.join(", "))
}

/// Renders one managed `hl.config` input line for a state: only the
/// input table, so it composes with the appearance `hl.config` line
/// (leaf-merge at load) and with `hl.monitor`/`hl.bind` lines.
pub fn render_input_line(state: &InputState) -> String {
    format!(
        "hl.config({{ input = {{ sensitivity = {}, touchpad = {{ natural_scroll = {}, scroll_factor = {} }} }} }})",
        appearance_float(state.sensitivity),
        state.natural_scroll,
        appearance_float(state.scroll_factor),
    )
}

/// Replaces the managed `hl.config({ input` line in a snippet body,
/// or appends it when absent. Pure over the text: sibling leaves
/// (blur, opacity, font, monitors, binds) pass through
/// byte-identical.
pub fn upsert_input_line(body: &str, line: &str) -> String {
    let mut replaced = false;
    let mut out = String::new();
    for body_line in body.lines() {
        if !replaced && body_line.contains("hl.config(") && body_line.contains("input") {
            out.push_str(line);
            out.push('\n');
            replaced = true;
        } else {
            out.push_str(body_line);
            out.push('\n');
        }
    }
    if !replaced {
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Parses rendered input leaves back out of a snippet body (pure
/// over the text so fixtures cover it without the filesystem).
/// Scoped to lines mentioning `input` so sibling floats (opacity,
/// scale) never leak in.
fn parse_snippet_input(text: &str) -> InputState {
    let mut state = InputState::default();
    for line in text.lines() {
        if !line.contains("input") {
            continue;
        }
        if let Some(rest) = line.find("sensitivity = ").map(|i| &line[i + "sensitivity = ".len()..])
        {
            let num: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                .collect();
            if let Ok(value) = num.parse::<f64>() {
                if value.is_finite() {
                    state.sensitivity = value;
                }
            }
        }
        if line.contains("natural_scroll = true") {
            state.natural_scroll = true;
        } else if line.contains("natural_scroll = false") {
            state.natural_scroll = false;
        }
        if let Some(rest) = line
            .find("scroll_factor = ")
            .map(|i| &line[i + "scroll_factor = ".len()..])
        {
            let num: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                .collect();
            if let Ok(value) = num.parse::<f64>() {
                if value.is_finite() && value > 0.0 {
                    state.scroll_factor = value;
                }
            }
        }
    }
    state
}

/// Reads the current snippet input state, falling back to defaults
/// when the snippet does not exist yet (first run). Never touches
/// user files — only the app-owned snippet is parsed.
pub fn current_snippet_input(base: &std::path::Path) -> Result<InputState, ArchelError> {
    match std::fs::read_to_string(snippet_path_at(base)) {
        Ok(text) => Ok(parse_snippet_input(&text)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(InputState::default()),
        Err(err) => Err(ArchelError::Internal(format!(
            "cannot read snippet: {err}"
        ))),
    }
}

/// Parses a `getoption input:sensitivity -j` payload into the live
/// float. Pure for headless coverage.
pub fn parse_sensitivity_option(out: &str) -> Result<f64, ArchelError> {
    match super::parse_getoption_json(out)? {
        super::OptionValue::Float(value) => Ok(value),
        other => Err(ArchelError::hyprland_unavailable(format!(
            "unexpected sensitivity option shape: {other:?}"
        ))),
    }
}

/// Parses a `getoption input:touchpad:natural_scroll -j` payload
/// into the live bool. Pure for headless coverage.
pub fn parse_natural_scroll_option(out: &str) -> Result<bool, ArchelError> {
    match super::parse_getoption_json(out)? {
        super::OptionValue::Bool(value) => Ok(value),
        other => Err(ArchelError::hyprland_unavailable(format!(
            "unexpected natural scroll option shape: {other:?}"
        ))),
    }
}

/// Parses a `getoption input:touchpad:scroll_factor -j` payload into
/// the live float. Pure for headless coverage.
pub fn parse_scroll_factor_option(out: &str) -> Result<f64, ArchelError> {
    match super::parse_getoption_json(out)? {
        super::OptionValue::Float(value) => Ok(value),
        other => Err(ArchelError::hyprland_unavailable(format!(
            "unexpected scroll factor option shape: {other:?}"
        ))),
    }
}

/// Reads all three live mouse leaves (honest read-back for bare
/// `mouse` and for the post-apply agreement check).
pub fn read_input() -> Result<InputState, ArchelError> {
    let out = run_hyprctl("hyprctl", &["getoption", SENSITIVITY_KEY, "-j"])?;
    let sensitivity = parse_sensitivity_option(&out)?;
    let out = run_hyprctl("hyprctl", &["getoption", NATURAL_SCROLL_KEY, "-j"])?;
    let natural_scroll = parse_natural_scroll_option(&out)?;
    let out = run_hyprctl("hyprctl", &["getoption", SCROLL_FACTOR_KEY, "-j"])?;
    let scroll_factor = parse_scroll_factor_option(&out)?;
    Ok(InputState {
        sensitivity,
        natural_scroll,
        scroll_factor,
    })
}

/// Applies an input plan: confirm FIRST (abort structurally never
/// mutates — audio precedent), `eval` live leg plus `getoption`
/// read-back of the planned leaves second, snippet upsert only on
/// read-back agreement (WR-01). A refused eval, a disagreeing
/// read-back, or a declined prompt returns `Err` with the prior
/// snippet untouched. Injectable legs keep the orchestration
/// unit-coverable headless.
pub fn apply_input_with(
    base: &std::path::Path,
    plan: &InputPlan,
    confirm: impl FnOnce(&str) -> Result<(), ArchelError>,
    runner: impl FnOnce(&str) -> Result<(), ArchelError>,
    reader: impl FnOnce() -> Result<InputState, ArchelError>,
) -> Result<InputReport, ArchelError> {
    if plan.sensitivity.is_none() && plan.natural_scroll.is_none() && plan.scroll_factor.is_none()
    {
        return Err(ArchelError::desktop_invalid(
            "mouse set needs --sensitivity, --natural-scroll, and/or --scroll-factor",
        ));
    }
    confirm(&confirm_input_prompt(plan))?;
    runner(&input_eval_statement(plan))?;
    let live = reader()?;
    let prior = current_snippet_input(base)?;
    let want_sensitivity = plan.sensitivity.unwrap_or(prior.sensitivity);
    let want_natural = plan.natural_scroll.unwrap_or(prior.natural_scroll);
    let want_factor = plan.scroll_factor.unwrap_or(prior.scroll_factor);
    if (live.sensitivity - want_sensitivity).abs() >= 1e-9
        || live.natural_scroll != want_natural
        || (live.scroll_factor - want_factor).abs() >= 1e-9
    {
        return Err(ArchelError::hyprland_unavailable(
            "live read-back disagrees with the applied mouse values",
        ));
    }
    let next = InputState {
        sensitivity: want_sensitivity,
        natural_scroll: want_natural,
        scroll_factor: want_factor,
    };
    let body = match std::fs::read_to_string(snippet_path_at(base)) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => {
            return Err(ArchelError::Internal(format!(
                "cannot read snippet: {err}"
            )));
        }
    };
    write_snippet_at(base, &upsert_input_line(&body, &render_input_line(&next)))?;
    ensure_require_line_at(base)?;
    Ok(InputReport {
        sensitivity: want_sensitivity,
        natural_scroll: want_natural,
        scroll_factor: want_factor,
        read_back_ok: true,
    })
}

/// Production leg: snippet under `~/.config/hypr/`, real hyprctl,
/// real confirm prompt.
pub fn set_input_options(
    sensitivity: Option<&str>,
    natural_scroll: Option<&str>,
    scroll_factor: Option<&str>,
) -> Result<InputReport, ArchelError> {
    let base = hypr_dir();
    let plan = plan_input_set_at(&base, sensitivity, natural_scroll, scroll_factor)?;
    apply_input_with(
        &base,
        &plan,
        |prompt| crate::packages::confirm_action(prompt),
        |lua| run_hyprctl_eval("hyprctl", lua),
        read_input,
    )
}

/// French title for the bare-mouse readout table.
pub fn mouse_title() -> String {
    rust_i18n::t!("input.mouse_title").to_string()
}

/// French rows for the bare-mouse readout table.
pub fn mouse_rows(state: &InputState) -> Vec<(String, String)> {
    vec![
        (
            rust_i18n::t!("input.label_sensitivity").to_string(),
            appearance_float(state.sensitivity),
        ),
        (
            rust_i18n::t!("input.label_natural_scroll").to_string(),
            on_off_word(state.natural_scroll),
        ),
        (
            rust_i18n::t!("input.label_scroll_factor").to_string(),
            appearance_float(state.scroll_factor),
        ),
    ]
}

/// French preview title (shown before `[o/N]`).
pub fn mouse_preview_title() -> String {
    rust_i18n::t!("input.mouse_preview_title").to_string()
}

/// French `(label, value)` preview rows: old arrow new per leaf.
pub fn mouse_preview_rows(plan: &InputPlan) -> Vec<(String, String)> {
    let na = || rust_i18n::t!("sys.unavailable").to_string();
    vec![
        (
            rust_i18n::t!("input.label_sensitivity").to_string(),
            plan.sensitivity
                .map(|new| {
                    format!(
                        "{} -> {}",
                        appearance_float(plan.old_sensitivity),
                        appearance_float(new)
                    )
                })
                .unwrap_or_else(na),
        ),
        (
            rust_i18n::t!("input.label_natural_scroll").to_string(),
            plan.natural_scroll
                .map(|new| {
                    format!(
                        "{} -> {}",
                        on_off_word(plan.old_natural_scroll),
                        on_off_word(new)
                    )
                })
                .unwrap_or_else(na),
        ),
        (
            rust_i18n::t!("input.label_scroll_factor").to_string(),
            plan.scroll_factor
                .map(|new| {
                    format!(
                        "{} -> {}",
                        appearance_float(plan.old_scroll_factor),
                        appearance_float(new)
                    )
                })
                .unwrap_or_else(na),
        ),
    ]
}

/// French confirmation prompt naming the planned leaves.
pub fn confirm_input_prompt(plan: &InputPlan) -> String {
    let mut changes = Vec::new();
    if let Some(sensitivity) = plan.sensitivity {
        changes.push(format!(
            "{}={}",
            rust_i18n::t!("input.label_sensitivity"),
            appearance_float(sensitivity)
        ));
    }
    if let Some(natural_scroll) = plan.natural_scroll {
        changes.push(format!(
            "{}={}",
            rust_i18n::t!("input.label_natural_scroll"),
            on_off_word(natural_scroll)
        ));
    }
    if let Some(scroll_factor) = plan.scroll_factor {
        changes.push(format!(
            "{}={}",
            rust_i18n::t!("input.label_scroll_factor"),
            appearance_float(scroll_factor)
        ));
    }
    rust_i18n::t!("input.confirm_mouse", detail = changes.join(", ")).to_string()
}

/// French title for the post-apply report table.
pub fn mouse_report_title() -> String {
    rust_i18n::t!("input.mouse_report_title").to_string()
}

/// French rows for the post-apply report table.
pub fn mouse_report_rows(report: &InputReport) -> Vec<(String, String)> {
    mouse_rows(&InputState {
        sensitivity: report.sensitivity,
        natural_scroll: report.natural_scroll,
        scroll_factor: report.scroll_factor,
    })
}

/// French ok line naming the applied leaves.
pub fn mouse_ok_line(report: &InputReport) -> String {
    rust_i18n::t!(
        "input.mouse_ok",
        sensitivity = appearance_float(report.sensitivity),
        scroll_factor = appearance_float(report.scroll_factor)
    )
    .to_string()
}

/// French wording for a bool leaf (raw bools never render).
fn on_off_word(on: bool) -> String {
    if on {
        rust_i18n::t!("input.value_on").to_string()
    } else {
        rust_i18n::t!("input.value_off").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::ENTRY_FILE;

    /// Seeds the Lua entry file in a scratch base: apply legs
    /// refuse a missing entry (WR-06), so every orchestration test
    /// starts from a present-but-untouched entry.
    fn seed_entry(base: &std::path::Path) {
        std::fs::write(base.join(ENTRY_FILE), "-- entry\n").expect("entry seeds");
    }

    fn scratch_base(tag: &str) -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!("archel-input-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).expect("scratch base must create");
        base
    }

    fn full_plan() -> InputPlan {
        InputPlan {
            sensitivity: Some(0.75),
            natural_scroll: Some(true),
            scroll_factor: Some(1.5),
            old_sensitivity: 0.5,
            old_natural_scroll: false,
            old_scroll_factor: 1.0,
        }
    }

    fn live_state(plan: &InputPlan, base: &std::path::Path) -> InputState {
        let prior = current_snippet_input(base).unwrap_or_default();
        InputState {
            sensitivity: plan.sensitivity.unwrap_or(prior.sensitivity),
            natural_scroll: plan.natural_scroll.unwrap_or(prior.natural_scroll),
            scroll_factor: plan.scroll_factor.unwrap_or(prior.scroll_factor),
        }
    }

    #[test]
    fn sensitivity_validator_rejects_nan_inf_and_out_of_range() {
        for bad in [
            "",
            "   ",
            "NaN",
            "nan",
            "inf",
            "-inf",
            "-1.000001",
            "1.000001",
            "2.0",
            "abc",
        ] {
            assert!(
                validate_sensitivity(bad).is_err(),
                "sensitivity {bad:?} must fail validation"
            );
        }
        for (raw, want) in [
            ("-1", -1.0),
            ("-1.0", -1.0),
            ("0", 0.0),
            ("0.5", 0.5),
            (" 0.75 ", 0.75),
            ("1", 1.0),
            ("1.0", 1.0),
        ] {
            assert_eq!(
                validate_sensitivity(raw).expect("boundary input validates"),
                want,
                "sensitivity {raw:?} must validate"
            );
        }
        let err = validate_sensitivity("1.5").expect_err("out of range must fail");
        assert!(matches!(err, ArchelError::DesktopInvalid { .. }), "{err:?}");
    }

    #[test]
    fn scroll_factor_validator_rejects_non_positive_and_wild() {
        for bad in ["", "   ", "NaN", "inf", "0", "-1.0", "0.05", "10.000001", "100", "fast"] {
            assert!(
                validate_scroll_factor(bad).is_err(),
                "scroll factor {bad:?} must fail validation"
            );
        }
        for (raw, want) in [("0.1", 0.1), ("1", 1.0), ("1.0", 1.0), (" 1.5 ", 1.5), ("10", 10.0)]
        {
            assert_eq!(
                validate_scroll_factor(raw).expect("boundary input validates"),
                want,
                "scroll factor {raw:?} must validate"
            );
        }
    }

    #[test]
    fn natural_scroll_words_parse_with_typed_rejects() {
        assert_eq!(parse_natural_scroll("on").expect("on"), true);
        assert_eq!(parse_natural_scroll("OFF").expect("folds"), false);
        assert_eq!(parse_natural_scroll(" true ").expect("trims"), true);
        assert_eq!(parse_natural_scroll("false").expect("false"), false);
        for bad in ["", "yes", "no", "1", "0", "enabled"] {
            let err = parse_natural_scroll(bad).expect_err("unknown word must fail");
            assert!(matches!(err, ArchelError::DesktopInvalid { .. }), "{err:?}");
        }
    }

    #[test]
    fn eval_golden_pins_exact_statements() {
        assert_eq!(
            input_eval_statement(&full_plan()),
            "hl.config({ input = { sensitivity = 0.75, touchpad = { natural_scroll = true, scroll_factor = 1.5 } } })"
        );
        let single = InputPlan {
            sensitivity: Some(0.5),
            natural_scroll: None,
            scroll_factor: None,
            old_sensitivity: 0.0,
            old_natural_scroll: false,
            old_scroll_factor: 1.0,
        };
        assert_eq!(
            input_eval_statement(&single),
            "hl.config({ input = { sensitivity = 0.5 } })"
        );
        for stmt in [input_eval_statement(&full_plan()), input_eval_statement(&single)] {
            assert!(
                !stmt.contains("keyword") && !stmt.contains("reload"),
                "banned verbs must never shape apply argv: {stmt:?}"
            );
        }
    }

    #[test]
    fn snippet_line_round_trips_all_three_leaves() {
        let state = InputState {
            sensitivity: 0.75,
            natural_scroll: true,
            scroll_factor: 1.5,
        };
        let line = render_input_line(&state);
        let back = parse_snippet_input(&line);
        assert_eq!(back, state, "render/parse round-trips: {line:?}");
    }

    #[test]
    fn upsert_preserves_sibling_leaves_byte_identical() {
        let sibling = "hl.config({ decoration = { blur = { enabled = true } } })";
        let body = format!("-- archel managed block\n{sibling}\n");
        let next = upsert_input_line(&body, &render_input_line(&InputState::default()));
        assert!(next.contains(sibling), "sibling line survives: {next:?}");
        assert!(next.contains("sensitivity = 0"), "input line appends: {next:?}");
        let changed = InputState {
            sensitivity: 0.75,
            natural_scroll: true,
            scroll_factor: 1.5,
        };
        let twice = upsert_input_line(&next, &render_input_line(&changed));
        assert_eq!(
            twice.lines().filter(|l| l.contains("sensitivity")).count(),
            1,
            "re-upsert replaces, never duplicates: {twice:?}"
        );
        assert!(twice.contains(sibling), "sibling still survives: {twice:?}");
        assert_eq!(parse_snippet_input(&twice), changed);
    }

    #[test]
    fn plan_validates_first_and_reads_second() {
        // Hermetic: the injectable base keeps the planner off the live
        // home dir (WR-02) — garbage fails even on a missing base.
        let base = scratch_base("plan");
        let err = plan_input_set_at(&base, None, None, None).expect_err("empty plan must fail");
        assert!(matches!(err, ArchelError::DesktopInvalid { .. }), "{err:?}");
        assert!(plan_input_set_at(&base, Some("0.75"), None, None).is_ok());
        assert!(plan_input_set_at(&base, None, Some("on"), None).is_ok());
        assert!(plan_input_set_at(&base, None, None, Some("1.5")).is_ok());
        assert!(plan_input_set_at(&base, Some("abc"), None, None).is_err());
        assert!(plan_input_set_at(&base, None, Some("maybe"), None).is_err());
        let missing = base.join("no-such-dir");
        assert!(plan_input_set_at(&missing, Some("abc"), None, None).is_err());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn apply_evals_first_and_persists_only_on_agreement() {
        use std::cell::RefCell;
        use std::rc::Rc;
        let base = scratch_base("apply");
        seed_entry(&base);
        let plan = full_plan();
        let ran = Rc::new(RefCell::new(String::new()));
        let probe = Rc::clone(&ran);
        let report = apply_input_with(
            &base,
            &plan,
            |_| Ok(()),
            |lua| {
                *probe.borrow_mut() = lua.to_string();
                Ok(())
            },
            || Ok(live_state(&plan, &base)),
        )
        .expect("injected legs must carry the apply");
        assert!(report.read_back_ok, "matching read-back confirms");
        assert_eq!(ran.borrow().as_str(), &input_eval_statement(&plan));
        let snippet = std::fs::read_to_string(snippet_path_at(&base)).expect("snippet reads");
        assert!(snippet.contains("sensitivity = 0.75"), "snippet persists: {snippet:?}");
        // Divergent read-back fails typed with the prior body alone.
        let before = snippet.clone();
        let divergent = InputState {
            sensitivity: 0.0,
            natural_scroll: false,
            scroll_factor: 1.0,
        };
        let err = apply_input_with(&base, &plan, |_| Ok(()), |_| Ok(()), || Ok(divergent))
            .expect_err("divergent read-back must fail, never silent-ok");
        assert!(
            matches!(err, ArchelError::HyprlandUnavailable { .. }),
            "disagreement refuses typed: {err:?}"
        );
        assert_eq!(
            std::fs::read_to_string(snippet_path_at(&base)).expect("snippet re-reads"),
            before,
            "a refused read-back leaves the prior snippet untouched"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn abort_writes_nothing_and_skips_both_legs() {
        use std::cell::Cell;
        let base = scratch_base("abort");
        seed_entry(&base);
        let plan = full_plan();
        let applied = Cell::new(false);
        let err = apply_input_with(
            &base,
            &plan,
            |_| Err(ArchelError::ConfirmationAborted),
            |_| {
                applied.set(true);
                Ok(())
            },
            || panic!("reader must never run after a pre-apply abort"),
        )
        .expect_err("declined confirm aborts");
        assert!(matches!(err, ArchelError::ConfirmationAborted));
        assert!(!applied.get(), "abort applies nothing");
        assert!(
            !snippet_path_at(&base).exists(),
            "abort writes no snippet"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn getoption_fixtures_parse_per_leaf() {
        assert_eq!(
            parse_sensitivity_option(r#"{"float": 0.5, "set": true}"#).expect("parses"),
            0.5
        );
        assert_eq!(
            parse_natural_scroll_option(r#"{"bool": true, "set": true}"#).expect("parses"),
            true
        );
        assert_eq!(
            parse_scroll_factor_option(r#"{"float": 1.0, "set": false}"#).expect("parses"),
            1.0
        );
        assert!(parse_sensitivity_option(r#"{"bool": true, "set": true}"#).is_err());
        assert!(parse_natural_scroll_option(r#"{"float": 1.0, "set": true}"#).is_err());
    }

    #[test]
    fn labels_come_from_the_catalog() {
        crate::i18n::set_language("fr");
        let plan = full_plan();
        assert!(!mouse_title().is_empty());
        assert!(!mouse_preview_title().is_empty());
        assert_eq!(mouse_preview_rows(&plan).len(), 3);
        assert!(!confirm_input_prompt(&plan).is_empty());
        assert!(!mouse_report_title().is_empty());
        let report = InputReport {
            sensitivity: 0.75,
            natural_scroll: true,
            scroll_factor: 1.5,
            read_back_ok: true,
        };
        assert!(!mouse_ok_line(&report).is_empty());
    }
}
