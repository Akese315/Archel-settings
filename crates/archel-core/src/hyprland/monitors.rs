//! Per-monitor configuration with confirm-and-rollback (DSP-01..04).
//!
//! Pure plan layer first: `Monitor` deserializes from `hyprctl
//! monitors -j` (mode derived from `availableModes`/geometry strings,
//! never a stable int key), `plan_monitor_change` builds the apply plus
//! rollback `hl.monitor` eval statements, and validators reject bad
//! divisibility, overlap, and unknown modes typed-DesktopInvalid BEFORE
//! any prompt, spawn, or write. Nothing here spawns: every parse/plan
//! fn is pure over injected fixtures (D-05: multi-monitor coverage is
//! fixtures plus argv-shape only; live proof stays single-monitor).
//!
//! Spawn discipline for the thin live readers mirrors [`crate::proxy`]:
//! which-gate, exact argv never through a shell, `LC_ALL=C`,
//! `wait_timeout` plus kill (inherited from [`super::run_hyprctl`]).
//! Monitor names reaching `eval` validate against the live `monitors -j`
//! vocabulary and render through [`super::lua_quote`] (T-05-09).

use serde::{Deserialize, Serialize};

use super::{appearance_float, lua_quote};
use crate::error::ArchelError;

/// One connected monitor as reported by `hyprctl monitors -j`.
///
/// Every field defaults so a shape-skewed entry degrades to a skipped
/// row (see [`parse_monitors_json`]) instead of failing the whole read.
/// `mode` is derived (see [`current_mode`]): the JSON carries no stable
/// mode key, only `availableModes` strings plus geometry.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Monitor {
    /// Compositor output name (`HDMI-A-2`, `DP-2`). Empty means the
    /// entry is unusable and skipped at parse time.
    #[serde(default)]
    pub name: String,
    /// Human description (`AOC 24G2W1G4 ...`). Informational only.
    #[serde(default)]
    pub description: String,
    /// Current mode width in physical pixels.
    #[serde(default)]
    pub width: u32,
    /// Current mode height in physical pixels.
    #[serde(default)]
    pub height: u32,
    /// Current refresh rate in Hz (live values carry noise: 144.001).
    #[serde(rename = "refreshRate", default)]
    pub refresh_rate: f64,
    /// Top-left corner X in scaled coordinates.
    #[serde(default)]
    pub x: i32,
    /// Top-left corner Y in scaled coordinates.
    #[serde(default)]
    pub y: i32,
    /// Current scale factor.
    #[serde(default = "default_scale")]
    pub scale: f64,
    /// Disabled outputs stay listed but leave the layout.
    #[serde(default)]
    pub disabled: bool,
    /// Settable mode strings (`1920x1080@144.00Hz`). The validation
    /// vocabulary for `--mode` (T-05-09).
    #[serde(rename = "availableModes", default)]
    pub available_modes: Vec<String>,
}

/// Monitors without an explicit scale read as scale 1.
fn default_scale() -> f64 {
    1.0
}

/// Parses `hyprctl monitors -j` output into monitors. Pure over the
/// text: blank/garbage input yields an empty list, malformed entries
/// are skipped, and entries without a name are dropped. Never panics,
/// never spawns.
pub fn parse_monitors_json(out: &str) -> Vec<Monitor> {
    let values: Vec<serde_json::Value> = match serde_json::from_str(out) {
        Ok(values) => values,
        Err(_) => return Vec::new(),
    };
    values
        .into_iter()
        .filter_map(|value| serde_json::from_value::<Monitor>(value).ok())
        .filter(|monitor| !monitor.name.trim().is_empty())
        .collect()
}

/// Derives the current mode string (`1920x1080@144`) from geometry.
/// `None` when geometry is unknown (zero size or non-finite refresh).
/// Refresh noise rounds to the nearest Hz (144.001 becomes 144).
pub fn current_mode(monitor: &Monitor) -> Option<String> {
    if monitor.width == 0 || monitor.height == 0 {
        return None;
    }
    if !monitor.refresh_rate.is_finite() || monitor.refresh_rate <= 0.0 {
        return None;
    }
    Some(format!(
        "{}x{}@{}",
        monitor.width,
        monitor.height,
        monitor.refresh_rate.round() as i64
    ))
}

/// One-line human summary for previews and reports (`1920x1080@144
/// scale 1 at 2560x0`, or `disabled`). Unknown modes render as the
/// `sys.unavailable` catalog string (never a guess).
pub fn summarize_monitor(monitor: &Monitor) -> String {
    if monitor.disabled {
        return format!("{} disabled", monitor.name);
    }
    let mode = current_mode(monitor)
        .unwrap_or_else(|| rust_i18n::t!("sys.unavailable").to_string());
    format!(
        "{} scale {} at {}x{}",
        mode,
        appearance_float(monitor.scale),
        monitor.x,
        monitor.y
    )
}

/// Human-readable live inventory. Serialized as-is for `--json`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MonitorStatus {
    pub name: String,
    pub mode: Option<String>,
    pub scale: f64,
    pub position: String,
    pub disabled: bool,
}

impl From<&Monitor> for MonitorStatus {
    fn from(monitor: &Monitor) -> Self {
        Self {
            name: monitor.name.clone(),
            mode: current_mode(monitor),
            scale: monitor.scale,
            position: format!("{}x{}", monitor.x, monitor.y),
            disabled: monitor.disabled,
        }
    }
}

/// Reads the live monitor inventory (read-only: no confirm, no write).
pub fn read_monitors() -> Result<Vec<Monitor>, ArchelError> {
    read_monitors_with_tool("hyprctl")
}

/// Same as [`read_monitors`] but the binary name is injectable, so the
/// which-gate stays unit-coverable headless (proxy.rs:69 precedent).
fn read_monitors_with_tool(tool: &str) -> Result<Vec<Monitor>, ArchelError> {
    let out = super::run_hyprctl(tool, &["monitors", "-j"])?;
    Ok(parse_monitors_json(&out))
}

/// Special mode words accepted without vocabulary lookup (wiki-cited
/// in RESEARCH: compositor-resolved aliases, not resolution strings).
const SPECIAL_MODES: [&str; 3] = ["preferred", "highrr", "maxwidth"];

/// Special position words accepted without coordinate parsing.
const SPECIAL_POSITIONS: [&str; 5] = ["auto", "auto-right", "auto-left", "auto-up", "auto-down"];

/// Strips one trailing `Hz` plus whitespace (`1920x1080@144.00Hz`
/// becomes `1920x1080@144.00`). Pure for headless coverage.
fn normalize_mode(raw: &str) -> String {
    raw.trim().strip_suffix("Hz").unwrap_or(raw.trim()).trim().to_string()
}

/// Validates a `--mode` value against the monitor vocabulary BEFORE
/// any prompt or spawn. Accepts special words plus exact or
/// prefix-truncated vocabulary hits (`1920x1080@144` matches the
/// advertised `1920x1080@144.00Hz`). Returns the trimmed want string.
/// Rejects typed-DesktopInvalid naming the value (T-05-09).
pub fn validate_mode(current: &Monitor, raw: &str) -> Result<String, ArchelError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ArchelError::desktop_invalid("empty monitor mode"));
    }
    if SPECIAL_MODES.contains(&trimmed.to_lowercase().as_str()) {
        return Ok(trimmed.to_string());
    }
    let vocab: Vec<String> = current
        .available_modes
        .iter()
        .map(|mode| normalize_mode(mode))
        .collect();
    let hit = vocab
        .iter()
        .any(|known| known == trimmed || known.starts_with(&format!("{trimmed}.")));
    if !hit {
        return Err(ArchelError::desktop_invalid(format!(
            "rejected monitor mode {trimmed:?} for output {:?} (not in availableModes)",
            current.name
        )));
    }
    Ok(trimmed.to_string())
}

/// Parses one `WxH` dimension pair (`1920x1080`). `None` for special
/// mode words or malformed text (callers fall back to the current
/// geometry for layout math).
pub fn parse_mode_dims(mode: &str) -> Option<(u32, u32)> {
    let (w, rest) = mode.split_once('x')?;
    let h = rest.split('@').next()?;
    let (width, height) = (w.parse::<u32>().ok()?, h.parse::<u32>().ok()?);
    if width == 0 || height == 0 {
        return None;
    }
    Some((width, height))
}

/// Validates a `--scale` value BEFORE any prompt or spawn: finite,
/// positive, and dividing the resolution cleanly (1.4 at 1920x1080
/// fails: scaled pixels must land on whole pixels). Rejects
/// typed-DesktopInvalid naming the value, including NaN/inf (T-05-09).
pub fn validate_scale(current: &Monitor, raw: &str) -> Result<f64, ArchelError> {
    let trimmed = raw.trim();
    let scale: f64 = trimmed.parse().map_err(|_| {
        ArchelError::desktop_invalid(format!("rejected monitor scale {trimmed:?}"))
    })?;
    if !scale.is_finite() || scale <= 0.0 {
        return Err(ArchelError::desktop_invalid(format!(
            "rejected monitor scale {trimmed:?} (must be a finite positive factor)"
        )));
    }
    if current.width > 0 && current.height > 0 {
        for (pixels, label) in [(current.width, "width"), (current.height, "height")] {
            let scaled = f64::from(pixels) / scale;
            if (scaled - scaled.round()).abs() > 1e-6 {
                return Err(ArchelError::desktop_invalid(format!(
                    "rejected monitor scale {trimmed:?} for output {:?}: {} {} does not divide cleanly",
                    current.name, pixels, label
                )));
            }
        }
    }
    Ok(scale)
}

/// Validates a `--position` value BEFORE any prompt or spawn: either a
/// `WxH` coordinate pair (negative components allowed for leftward
/// stacking) or a special word (`auto`, `auto-right`, ...). Positions
/// live in scaled coordinate space. Rejects typed-DesktopInvalid
/// naming the value.
pub fn validate_position(raw: &str) -> Result<String, ArchelError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ArchelError::desktop_invalid("empty monitor position"));
    }
    if SPECIAL_POSITIONS.contains(&trimmed.to_lowercase().as_str()) {
        return Ok(trimmed.to_lowercase());
    }
    let (x, y) = trimmed.split_once('x').ok_or_else(|| {
        ArchelError::desktop_invalid(format!("rejected monitor position {trimmed:?}"))
    })?;
    if x.parse::<i32>().is_err() || y.parse::<i32>().is_err() {
        return Err(ArchelError::desktop_invalid(format!(
            "rejected monitor position {trimmed:?} (want WxH or auto...)"
        )));
    }
    Ok(trimmed.to_string())
}

/// Validates a monitor output name against the live inventory
/// vocabulary (T-05-09: names cross into `eval` Lua, so typos fail
/// here — before any prompt — never at the compositor).
pub fn validate_output(monitors: &[Monitor], raw: &str) -> Result<String, ArchelError> {
    let trimmed = raw.trim();
    if monitors.iter().any(|monitor| monitor.name == trimmed) {
        return Ok(trimmed.to_string());
    }
    Err(ArchelError::desktop_invalid(format!(
        "rejected monitor output {trimmed:?} (no such connected monitor)"
    )))
}

/// Wanted change for one output: raw CLI strings validated inside
/// [`plan_monitor_change`] (validate-first per WR-02, so typos fail
/// with no TTY need).
#[derive(Debug, Clone, PartialEq)]
pub struct MonitorWant {
    /// Compositor output name (must match a connected monitor).
    pub output: String,
    /// Raw `--mode` (`1920x1080@144`); `None` keeps the current mode.
    pub mode: Option<String>,
    /// Raw `--scale` (`1.5`); `None` keeps the current scale.
    pub scale: Option<String>,
    /// Raw `--position` (`2560x0`, `auto-right`); `None` keeps it.
    pub position: Option<String>,
    /// `--disable`: remove the output from the layout.
    pub disabled: bool,
}

/// A validated monitor transition: `apply` eval legs in layout order,
/// `rollback` re-applying the pre-change snapshot verbatim (reverse
/// order for multi-monitor sets — the Vec order carries that
/// contract). Both legs are `hl.monitor` Lua statements; the CLI pairs
/// each with `eval` via [`eval_argv`].
#[derive(Debug, Clone, PartialEq)]
pub struct MonitorPlan {
    /// Output this plan mutates.
    pub output: String,
    /// Resolved mode the apply leg carries (specials stay literal;
    /// layout projection resolves them to current geometry).
    pub mode: String,
    /// Resolved scale the apply leg carries.
    pub scale: f64,
    /// Resolved position the apply leg carries.
    pub position: String,
    /// Whether this plan disables the output.
    pub disabled: bool,
    /// Old-arrow-new preview strings for the FR table.
    pub old_summary: String,
    /// Old-arrow-new preview strings for the FR table.
    pub new_summary: String,
    /// `hl.monitor({...})` statements applying the change.
    pub apply: Vec<String>,
    /// `hl.monitor({...})` statements restoring the snapshot.
    pub rollback: Vec<String>,
    /// Snippet line persisted only when the change is kept (D-08).
    pub snippet_line: String,
}

/// Renders one exact `hl.config`-style `hl.monitor` Lua statement for
/// an enabled output. Machine-rendered values only: names, modes, and
/// positions route through [`lua_quote`] after vocabulary validation
/// (T-05-09); scale renders canonically. Pure for golden tests.
pub fn monitor_eval_statement(
    output: &str,
    mode: &str,
    position: &str,
    scale: f64,
) -> Result<String, ArchelError> {
    Ok(format!(
        "hl.monitor({{ output = {}, mode = {}, position = {}, scale = {} }})",
        lua_quote(output)?,
        lua_quote(mode)?,
        lua_quote(position)?,
        appearance_float(scale)
    ))
}

/// Renders the `hl.monitor` statement disabling one output. Pure for
/// golden tests.
pub fn monitor_disable_statement(output: &str) -> Result<String, ArchelError> {
    Ok(format!(
        "hl.monitor({{ output = {}, disabled = true }})",
        lua_quote(output)?
    ))
}

/// Pairs one Lua statement with its exact `hyprctl` argv
/// (`["eval", lua]`). Golden-testable without spawning.
pub fn eval_argv(lua: &str) -> Vec<String> {
    vec!["eval".to_string(), lua.to_string()]
}

/// Plans one monitor transition from the live snapshot plus raw CLI
/// wants. Validates FIRST (mode vocabulary, scale divisibility,
/// position shape — WR-02), builds the plan second. The rollback leg
/// re-applies the snapshot verbatim, so a reject/timeout restores the
/// exact prior state (D-04). A want equal to the snapshot plans a
/// single no-op re-apply leg (the D-05 live-proof shape).
pub fn plan_monitor_change(
    current: &Monitor,
    want: &MonitorWant,
) -> Result<MonitorPlan, ArchelError> {
    if want.output.trim() != current.name {
        return Err(ArchelError::desktop_invalid(format!(
            "rejected monitor output {:?} (plan snapshot is output {:?})",
            want.output, current.name
        )));
    }
    if want.disabled {
        let apply = monitor_disable_statement(&current.name)?;
        let rollback = verbatim_statement(current)?;
        return Ok(MonitorPlan {
            output: current.name.clone(),
            mode: current_mode(current).unwrap_or_default(),
            scale: current.scale,
            position: format!("{}x{}", current.x, current.y),
            disabled: true,
            old_summary: summarize_monitor(current),
            new_summary: format!("{} disabled", current.name),
            snippet_line: apply.clone(),
            apply: vec![apply],
            rollback: vec![rollback],
        });
    }
    let mode = match want.mode.as_deref() {
        Some(raw) => validate_mode(current, raw)?,
        None => current_mode(current).ok_or_else(|| {
            ArchelError::desktop_invalid(format!(
                "output {:?} has no derivable current mode (specify --mode)",
                current.name
            ))
        })?,
    };
    let scale = match want.scale.as_deref() {
        Some(raw) => validate_scale(current, raw)?,
        None => current.scale,
    };
    let position = match want.position.as_deref() {
        Some(raw) => validate_position(raw)?,
        None => format!("{}x{}", current.x, current.y),
    };
    let apply = monitor_eval_statement(&current.name, &mode, &position, scale)?;
    let rollback = verbatim_statement(current)?;
    let scale_text = appearance_float(scale);
    Ok(MonitorPlan {
        output: current.name.clone(),
        mode: mode.clone(),
        scale,
        position: position.clone(),
        disabled: false,
        old_summary: summarize_monitor(current),
        new_summary: format!("{mode} scale {scale_text} at {position}"),
        snippet_line: apply.clone(),
        apply: vec![apply],
        rollback: vec![rollback],
    })
}

/// Re-applies a snapshot verbatim (the rollback leg): current mode,
/// position, and scale exactly as read, or the disable statement when
/// the snapshot itself was disabled.
fn verbatim_statement(current: &Monitor) -> Result<String, ArchelError> {
    if current.disabled {
        return monitor_disable_statement(&current.name);
    }
    let mode = current_mode(current).ok_or_else(|| {
        ArchelError::desktop_invalid(format!(
            "output {:?} has no derivable current mode",
            current.name
        ))
    })?;
    monitor_eval_statement(
        &current.name,
        &mode,
        &format!("{}x{}", current.x, current.y),
        current.scale,
    )
}

/// Scaled-space rectangle for layout math: `(x, y, w, h)`. `None` for
/// disabled outputs or unknown geometry (they leave the layout and
/// cannot overlap anything).
pub fn monitor_rect(monitor: &Monitor) -> Option<(f64, f64, f64, f64)> {
    if monitor.disabled || monitor.width == 0 || monitor.height == 0 {
        return None;
    }
    if !monitor.scale.is_finite() || monitor.scale <= 0.0 {
        return None;
    }
    Some((
        f64::from(monitor.x),
        f64::from(monitor.y),
        f64::from(monitor.width) / monitor.scale,
        f64::from(monitor.height) / monitor.scale,
    ))
}

/// Rejects overlapping monitor rectangles unless intentionally
/// stacked (identical origins). Pure over snapshots: the CLI projects
/// the planned monitor into the live set and runs this BEFORE any
/// prompt (DSP-04). Typed-DesktopInvalid names both outputs.
pub fn validate_layout(monitors: &[Monitor]) -> Result<(), ArchelError> {
    let rects: Vec<(&Monitor, (f64, f64, f64, f64))> = monitors
        .iter()
        .filter_map(|monitor| monitor_rect(monitor).map(|rect| (monitor, rect)))
        .collect();
    for (i, (a, rect_a)) in rects.iter().enumerate() {
        let (ax, ay, aw, ah) = *rect_a;
        for (b, rect_b) in rects.iter().skip(i + 1) {
            let (bx, by, bw, bh) = *rect_b;
            let overlaps = ax < bx + bw && bx < ax + aw && ay < by + bh && by < ay + ah;
            let stacked = ax == bx && ay == by;
            if overlaps && !stacked {
                return Err(ArchelError::desktop_invalid(format!(
                    "rejected monitor layout: {:?} at {}x{} overlaps {:?} at {}x{}",
                    a.name, a.x, a.y, b.name, b.x, b.y
                )));
            }
        }
    }
    Ok(())
}

/// Projects a planned change into a snapshot set for [`validate_layout`]:
/// clones the live monitors, swaps in the planned geometry (mode dims,
/// scale, position; specials keep the current origin), and drops the
/// output when disabled. Returns the projected set (validation itself
/// stays in `validate_layout`, so tests assert each layer alone).
pub fn project_layout(
    monitors: &[Monitor],
    output: &str,
    mode: &str,
    scale: f64,
    position: &str,
    disabled: bool,
) -> Vec<Monitor> {
    monitors
        .iter()
        .map(|monitor| {
            if monitor.name != output {
                return monitor.clone();
            }
            let mut next = monitor.clone();
            if disabled {
                next.disabled = true;
                return next;
            }
            next.disabled = false;
            if let Some((w, h)) = parse_mode_dims(mode) {
                next.width = w;
                next.height = h;
            }
            next.scale = scale;
            if let Some((x, rest)) = position.split_once('x') {
                if let (Ok(px), Ok(py)) = (x.parse::<i32>(), rest.parse::<i32>()) {
                    next.x = px;
                    next.y = py;
                }
            }
            next
        })
        .collect()
}

/// Outcome of a display mutation: the kept (or rolled-back) config
/// plus how the change settled. Serialized as-is for `--json`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MonitorReport {
    pub output: String,
    pub mode: String,
    pub scale: f64,
    pub position: String,
    /// True when the 15 s re-confirm kept the change; false when a
    /// reject or timeout rolled it back (D-04).
    pub kept: bool,
    /// Live read-back agreed with the expected end state.
    pub read_back_ok: bool,
}

/// Replaces the `hl.monitor(...)` line for `output` in a snippet body,
/// or appends it when absent. Pure over the text: matched by the
/// `hl.monitor(` marker plus the output name, so sibling leaves
/// (blur, opacity, font) pass through byte-identical.
pub fn upsert_monitor_line(body: &str, output: &str, line: &str) -> String {
    let mut replaced = false;
    let mut out = String::new();
    for body_line in body.lines() {
        if !replaced && body_line.contains("hl.monitor(") && body_line.contains(output) {
            out.push_str(line);
            out.push('\n');
            replaced = true;
        } else {
            out.push_str(body_line);
            out.push('\n');
        }
    }
    if !replaced {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// French pre-apply confirmation prompt naming the output change.
pub fn confirm_monitor_prompt(plan: &MonitorPlan) -> String {
    rust_i18n::t!(
        "display.confirm_set",
        output = plan.output.clone(),
        change = plan.new_summary.clone()
    )
    .to_string()
}

/// French 15 s keep-or-rollback prompt (names the timeout so the
/// lock-screen caveat is visible at the decision point).
pub fn confirm_monitor_keep_prompt(plan: &MonitorPlan) -> String {
    rust_i18n::t!(
        "display.confirm_keep",
        output = plan.output.clone(),
        change = plan.new_summary.clone()
    )
    .to_string()
}

/// Builds the kept-change report from a plan.
fn report_for(plan: &MonitorPlan, kept: bool) -> MonitorReport {
    MonitorReport {
        output: plan.output.clone(),
        mode: plan.new_summary.clone(),
        scale: plan.scale,
        position: plan.position.clone(),
        kept,
        read_back_ok: true,
    }
}

/// French title for the monitor inventory table.
pub fn monitors_title() -> String {
    rust_i18n::t!("display.monitors_title").to_string()
}

/// French rows for the monitor inventory table.
pub fn monitor_rows(monitors: &[Monitor]) -> Vec<(String, String)> {
    monitors
        .iter()
        .map(|monitor| (monitor.name.clone(), summarize_monitor(monitor)))
        .collect()
}

/// French preview title (shown before `[o/N]`).
pub fn monitor_preview_title() -> String {
    rust_i18n::t!("display.set_preview_title").to_string()
}

/// French `(label, value)` preview rows: old arrow new.
pub fn monitor_preview_rows(plan: &MonitorPlan) -> Vec<(String, String)> {
    vec![(
        plan.output.clone(),
        format!("{} -> {}", plan.old_summary, plan.new_summary),
    )]
}

/// French title for the post-apply report table.
pub fn monitor_report_title() -> String {
    rust_i18n::t!("display.set_report_title").to_string()
}

/// French rows for the post-apply report table.
pub fn monitor_report_rows(report: &MonitorReport) -> Vec<(String, String)> {
    vec![(
        report.output.clone(),
        format!("{} ({})", report.mode, kept_word(report.kept)),
    )]
}

/// French kept/rolled-back wording for reports.
pub fn kept_word(kept: bool) -> String {
    if kept {
        rust_i18n::t!("display.kept").to_string()
    } else {
        rust_i18n::t!("display.rolled_back").to_string()
    }
}

/// French ok line naming the kept output change.
pub fn monitor_ok_line(report: &MonitorReport) -> String {
    rust_i18n::t!(
        "display.set_ok",
        output = report.output.clone(),
        change = report.mode.clone()
    )
    .to_string()
}

/// French rollback line naming the restored output.
pub fn monitor_rollback_line(output: &str) -> String {
    rust_i18n::t!("display.set_rolled_back", output = output).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed live-shaped fixture: DP-2 4K@60 at 0x0 scale 1.5 plus
    /// HDMI-A-2 1080p@144 at 2560x0 scale 1 (RESEARCH Pitfall 7: the
    /// scaled widths line up edge to edge, 3840/1.5 = 2560).
    const TWO_MONITOR_FIXTURE: &str = r#"[
        {"id": 1, "name": "DP-2", "description": "BNQ BenQ PD2700U",
         "width": 3840, "height": 2160, "refreshRate": 59.997,
         "x": 0, "y": 0, "scale": 1.5, "disabled": false,
         "availableModes": ["3840x2160@60.00Hz", "3840x2160@59.94Hz", "1920x1080@60.00Hz"]},
        {"id": 0, "name": "HDMI-A-2", "description": "AOC 24G2W1G4",
         "width": 1920, "height": 1080, "refreshRate": 144.00101,
         "x": 2560, "y": 0, "scale": 1.0, "disabled": false,
         "availableModes": ["1920x1080@60.00Hz", "1920x1080@144.00Hz", "1280x720@59.94Hz"]}
    ]"#;

    fn fixture_monitors() -> Vec<Monitor> {
        parse_monitors_json(TWO_MONITOR_FIXTURE)
    }

    fn hdmi<'a>(monitors: &'a [Monitor]) -> &'a Monitor {
        monitors.iter().find(|m| m.name == "HDMI-A-2").expect("fixture has HDMI-A-2")
    }

    #[test]
    fn fixture_parse_reads_both_monitors_with_derived_modes() {
        let monitors = fixture_monitors();
        assert_eq!(monitors.len(), 2, "both fixture monitors parse: {monitors:?}");
        let dp = &monitors[0];
        assert_eq!(dp.width, 3840);
        assert_eq!(dp.scale, 1.5);
        assert_eq!(current_mode(dp).as_deref(), Some("3840x2160@60"));
        let hdmi = hdmi(&monitors);
        assert_eq!(current_mode(hdmi).as_deref(), Some("1920x1080@144"));
        assert_eq!(hdmi.x, 2560);
    }

    #[test]
    fn parser_skips_blanks_malformed_and_nameless_without_panicking() {
        assert!(parse_monitors_json("").is_empty());
        assert!(parse_monitors_json("   \n  ").is_empty());
        assert!(parse_monitors_json("not json at all {{{").is_empty());
        assert!(parse_monitors_json(r#"{"name": "solo"}"#).is_empty());
        let mixed = r#"[{"name": "HDMI-A-2", "width": 1920}, {"width": 800}, "junk", 42]"#;
        let monitors = parse_monitors_json(mixed);
        assert_eq!(monitors.len(), 1, "only the named object survives: {monitors:?}");
        assert_eq!(monitors[0].name, "HDMI-A-2");
    }

    #[test]
    fn mode_validator_accepts_vocabulary_prefixes_and_specials() {
        let monitors = fixture_monitors();
        let hdmi = hdmi(&monitors);
        assert_eq!(
            validate_mode(hdmi, "1920x1080@144").expect("prefix matches 144.00Hz"),
            "1920x1080@144"
        );
        assert_eq!(
            validate_mode(hdmi, "1920x1080@144.00").expect("exact normalizes"),
            "1920x1080@144.00"
        );
        for special in ["preferred", "highrr", "maxwidth"] {
            assert!(validate_mode(hdmi, special).is_ok(), "special {special:?} passes");
        }
        for bad in ["", "1920x1080@145", "3840x2160@60", "1920x1080", "highres-fake"] {
            let err = validate_mode(hdmi, bad).expect_err("unknown mode must fail");
            assert!(
                matches!(err, ArchelError::DesktopInvalid { .. }),
                "wrong variant for {bad:?}: {err:?}"
            );
        }
    }

    #[test]
    fn scale_validator_rejects_bad_divisibility_nans_and_garbage() {
        let monitors = fixture_monitors();
        let hdmi = hdmi(&monitors);
        assert_eq!(validate_scale(hdmi, "1").expect("unity scales"), 1.0);
        assert_eq!(validate_scale(hdmi, "1.5").expect("clean divisor scales"), 1.5);
        for bad in ["1.4", "0", "-1", "NaN", "inf", "abc", ""] {
            let err = validate_scale(hdmi, bad).expect_err("bad scale must fail");
            assert!(
                matches!(err, ArchelError::DesktopInvalid { .. }),
                "wrong variant for {bad:?}: {err:?}"
            );
        }
        let dp = &monitors[0];
        assert!(validate_scale(dp, "1.4").is_err(), "1.4 fails at 4K too");
        assert_eq!(validate_scale(dp, "2").expect("2 divides 4K"), 2.0);
    }

    #[test]
    fn position_validator_accepts_coordinates_and_specials() {
        assert_eq!(validate_position("2560x0").expect("coords parse"), "2560x0");
        assert_eq!(validate_position("-1920x0").expect("negative parses"), "-1920x0");
        assert_eq!(validate_position("auto-right").expect("special parses"), "auto-right");
        assert_eq!(validate_position("AUTO").expect("case folds"), "auto");
        for bad in ["", "2560", "2560x", "x0", "2560,0", "far-right"] {
            assert!(validate_position(bad).is_err(), "position {bad:?} must fail");
        }
    }

    #[test]
    fn output_validator_names_unknown_monitors_pre_spawn() {
        let monitors = fixture_monitors();
        assert_eq!(
            validate_output(&monitors, "HDMI-A-2").expect("known output passes"),
            "HDMI-A-2"
        );
        let err = validate_output(&monitors, "VGA-9").expect_err("typo fails");
        assert!(matches!(err, ArchelError::DesktopInvalid { .. }), "{err:?}");
    }

    #[test]
    fn apply_and_rollback_goldens_pin_exact_eval_pairs() {
        let monitors = fixture_monitors();
        let hdmi = hdmi(&monitors);
        let plan = plan_monitor_change(
            hdmi,
            &MonitorWant {
                output: "HDMI-A-2".to_string(),
                mode: Some("1280x720@59".to_string()),
                scale: Some("1".to_string()),
                position: Some("2560x0".to_string()),
                disabled: false,
            },
        )
        .expect("valid want plans");
        assert_eq!(
            plan.apply,
            vec![
                "hl.monitor({ output = \"HDMI-A-2\", mode = \"1280x720@59\", position = \"2560x0\", scale = 1 })"
                    .to_string()
            ]
        );
        assert_eq!(
            plan.rollback,
            vec![
                "hl.monitor({ output = \"HDMI-A-2\", mode = \"1920x1080@144\", position = \"2560x0\", scale = 1 })"
                    .to_string()
            ]
        );
        assert_eq!(
            eval_argv(&plan.apply[0]),
            vec![
                "eval".to_string(),
                "hl.monitor({ output = \"HDMI-A-2\", mode = \"1280x720@59\", position = \"2560x0\", scale = 1 })"
                    .to_string()
            ]
        );
        let flat = plan.apply.join("\n") + "\n" + &plan.rollback.join("\n");
        assert!(!flat.contains("keyword"), "eval-only, never keyword");
        assert!(!flat.contains("reload"), "eval-only, never reload");
    }

    #[test]
    fn noop_want_plans_a_single_reapply_leg() {
        let monitors = fixture_monitors();
        let hdmi = hdmi(&monitors);
        let plan = plan_monitor_change(
            hdmi,
            &MonitorWant {
                output: "HDMI-A-2".to_string(),
                mode: None,
                scale: None,
                position: None,
                disabled: false,
            },
        )
        .expect("no-op want plans");
        assert_eq!(plan.apply, plan.rollback, "no-op re-applies verbatim");
        assert_eq!(plan.apply.len(), 1);
    }

    #[test]
    fn disable_plans_a_disable_leg_with_verbatim_rollback() {
        let monitors = fixture_monitors();
        let hdmi = hdmi(&monitors);
        let plan = plan_monitor_change(
            hdmi,
            &MonitorWant {
                output: "HDMI-A-2".to_string(),
                mode: None,
                scale: None,
                position: None,
                disabled: true,
            },
        )
        .expect("disable plans");
        assert_eq!(
            plan.apply,
            vec!["hl.monitor({ output = \"HDMI-A-2\", disabled = true })".to_string()]
        );
        assert!(plan.rollback[0].contains("1920x1080@144"), "rollback restores the mode");
    }

    #[test]
    fn two_monitor_fixture_proves_layout_without_live_hardware() {
        let monitors = fixture_monitors();
        validate_layout(&monitors).expect("edge-to-edge scaled layout passes");
        let mut shifted = monitors.clone();
        shifted[1].x = 100;
        let err = validate_layout(&shifted).expect_err("100px shift overlaps DP-2");
        assert!(matches!(err, ArchelError::DesktopInvalid { .. }), "{err:?}");
        let mut stacked = monitors.clone();
        stacked[1].x = 0;
        stacked[1].y = 0;
        validate_layout(&stacked).expect("identical origins are intentional stacking");
        let mut disabled = monitors.clone();
        disabled[1].disabled = true;
        disabled[1].x = 100;
        validate_layout(&disabled).expect("disabled outputs leave the layout");
    }

    #[test]
    fn projected_layout_catches_a_planned_overlap_pre_prompt() {
        let monitors = fixture_monitors();
        let ok = project_layout(&monitors, "HDMI-A-2", "1920x1080@144", 1.0, "2560x0", false);
        validate_layout(&ok).expect("unchanged projection passes");
        let bad = project_layout(&monitors, "HDMI-A-2", "1920x1080@144", 1.0, "100x0", false);
        assert!(validate_layout(&bad).is_err(), "planned shift into DP-2 fails");
        let gone = project_layout(&monitors, "HDMI-A-2", "1920x1080@144", 1.0, "100x0", true);
        validate_layout(&gone).expect("disabling clears the overlap");
    }

    #[test]
    fn snippet_upsert_replaces_only_the_matching_output_line() {
        let body = "-- archel managed block\n\
            hl.config({\n\
            \x20   decoration = { blur = { enabled = true } },\n\
            })\n\
            hl.monitor({ output = \"DP-2\", mode = \"3840x2160@60\", position = \"0x0\", scale = 1.5 })\n";
        let next = upsert_monitor_line(
            body,
            "HDMI-A-2",
            "hl.monitor({ output = \"HDMI-A-2\", mode = \"1920x1080@144\", position = \"2560x0\", scale = 1 })",
        );
        assert!(next.contains("blur = { enabled = true }"), "sibling leaves survive");
        assert!(next.contains("output = \"DP-2\""), "other outputs survive");
        assert!(next.contains("output = \"HDMI-A-2\""), "new line appends");
        let replaced = upsert_monitor_line(
            &next,
            "DP-2",
            "hl.monitor({ output = \"DP-2\", mode = \"1920x1080@60\", position = \"0x0\", scale = 1 })",
        );
        assert_eq!(
            replaced.lines().filter(|l| l.contains("output = \"DP-2\"")).count(),
            1,
            "replacing never duplicates the output line"
        );
        assert!(replaced.contains("1920x1080@60"), "replacement carries the new mode");
    }

    #[test]
    fn missing_binary_maps_to_typed_unavailable_without_spawning() {
        let err = read_monitors_with_tool("archel-no-such-tool").expect_err("missing binary fails");
        assert!(
            matches!(err, ArchelError::HyprlandUnavailable { .. }),
            "missing hyprctl is typed unavailable: {err:?}"
        );
    }

    #[test]
    fn labels_come_from_the_catalog() {
        crate::i18n::set_language("fr");
        let monitors = fixture_monitors();
        assert!(!monitors_title().is_empty());
        assert_eq!(monitor_rows(&monitors).len(), 2);
        let plan = plan_monitor_change(
            hdmi(&monitors),
            &MonitorWant {
                output: "HDMI-A-2".to_string(),
                mode: None,
                scale: None,
                position: None,
                disabled: false,
            },
        )
        .expect("plan builds");
        assert!(!confirm_monitor_prompt(&plan).is_empty());
        assert!(!confirm_monitor_keep_prompt(&plan).is_empty());
        assert!(!monitor_preview_title().is_empty());
        assert_eq!(monitor_preview_rows(&plan).len(), 1);
        let report = report_for(&plan, true);
        assert!(!monitor_report_title().is_empty());
        assert!(!monitor_ok_line(&report).is_empty());
        assert!(!monitor_rollback_line("HDMI-A-2").is_empty());
    }

}
