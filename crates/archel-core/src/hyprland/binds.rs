//! Keybind management in the app snippet only (INP-01/02, D-06).
//!
//! Pure bind layer: modmask codec over `hyprctl binds -j` (live-verified
//! bits SHIFT=1, CTRL=4, ALT=8, SUPER=64), `hl.bind(...)` line
//! render/parse over snippet text, and line-level exact-match
//! add/edit/remove scoped to app-generated lines only. User bind
//! blocks are read-only displayed, never edited: every merge/remove
//! fn takes the app snippet body and only matches lines carrying the
//! `hl.bind(` plus `hl.dsp.` shape, so foreign lines (comments, other
//! `hl.*` calls, hand-written binds) pass through byte-identical.
//!
//! Dispatcher names come from an app-side enum ([`BindDispatcher`]);
//! free-form `exec` command args are accepted as opaque user strings
//! (Hyprland semantics — a shell command by design, run as the user
//! at user request) but MUST pass through [`super::lua_quote`] before
//! reaching `eval` or snippet text (T-05-11). Adding a bind requires
//! the confirm gate at the CLI layer. No spawn code lives here — the
//! live `binds -j` read and the `hl.bind` eval apply run in the CLI
//! from these pure pieces.
//!
//! `binds -j` entries are display-only for user binds: lua-era binds
//! expose `dispatcher: "__lua"` with an opaque numeric `arg`, so the
//! snippet file is the source of truth for managed binds and dispatcher
//! names are never string-matched against binds output (Pitfall 4).

use serde::{Deserialize, Serialize};

use super::lua_quote;
use crate::error::ArchelError;

/// Live-verified modmask bit for SHIFT (`hyprctl binds -j`).
pub const MOD_SHIFT: u32 = 1;
/// Live-verified modmask bit for CTRL.
pub const MOD_CTRL: u32 = 4;
/// Live-verified modmask bit for ALT.
pub const MOD_ALT: u32 = 8;
/// Live-verified modmask bit for SUPER.
pub const MOD_SUPER: u32 = 64;

/// Canonical modifier word order (render and normalize share it).
const MOD_WORDS: [(&str, u32); 4] = [
    ("SUPER", MOD_SUPER),
    ("CTRL", MOD_CTRL),
    ("ALT", MOD_ALT),
    ("SHIFT", MOD_SHIFT),
];

/// Dispatcher names the app can manage. `exec` carries an opaque
/// user command (lua-quoted at render); the rest carry an optional
/// plain argument (workspace numbers, …). Unknown-but-plausible
/// names validate through as [`BindDispatcher::Other`] so the
/// documented Hyprland set stays usable without a release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindDispatcher {
    Exec,
    KillActive,
    ToggleFloating,
    Workspace,
    MoveToWorkspace,
    Fullscreen,
    ToggleSplit,
    Other(String),
}

/// Documented dispatchers the app names directly (D-06 exact-match
/// vocabulary; `exec` included — its arg stays opaque).
const KNOWN_DISPATCHERS: [&str; 7] = [
    "exec",
    "killactive",
    "togglefloating",
    "workspace",
    "movetoworkspace",
    "fullscreen",
    "togglesplit",
];

impl BindDispatcher {
    /// Parses one dispatcher name: known names map to variants,
    /// anything else must be a plausible Hyprland dispatcher word
    /// (`[a-z0-9_]+`, lowercase-folded) or the plan fails typed
    /// BEFORE any prompt, spawn, or write.
    pub fn parse(raw: &str) -> Result<Self, ArchelError> {
        let trimmed = raw.trim().to_lowercase();
        if trimmed.is_empty() {
            return Err(ArchelError::desktop_invalid("empty bind dispatcher"));
        }
        match trimmed.as_str() {
            "exec" => Ok(Self::Exec),
            "killactive" => Ok(Self::KillActive),
            "togglefloating" => Ok(Self::ToggleFloating),
            "workspace" => Ok(Self::Workspace),
            "movetoworkspace" => Ok(Self::MoveToWorkspace),
            "fullscreen" => Ok(Self::Fullscreen),
            "togglesplit" => Ok(Self::ToggleSplit),
            other => {
                if other.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                {
                    Ok(Self::Other(other.to_string()))
                } else {
                    Err(ArchelError::desktop_invalid(format!(
                        "rejected bind dispatcher {raw:?}"
                    )))
                }
            }
        }
    }

    /// Canonical dispatcher word (renders into `hl.dsp.<name>`).
    pub fn name(&self) -> &str {
        match self {
            Self::Exec => "exec",
            Self::KillActive => "killactive",
            Self::ToggleFloating => "togglefloating",
            Self::Workspace => "workspace",
            Self::MoveToWorkspace => "movetoworkspace",
            Self::Fullscreen => "fullscreen",
            Self::ToggleSplit => "togglesplit",
            Self::Other(name) => name,
        }
    }

    /// True for the one dispatcher whose arg is an opaque shell
    /// command (confirm-gated at the CLI layer, T-05-11).
    pub fn is_exec(&self) -> bool {
        matches!(self, Self::Exec)
    }
}

/// One managed bind: normalized (mods, key, dispatcher, arg) tuple.
/// Addressed by exact-match on (mods, key, dispatcher) per D-06.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindSpec {
    /// Canonical mods (`SUPER + CTRL`, empty when modeless).
    pub mods: String,
    /// Key token (`V`, `Return`, `1`, …).
    pub key: String,
    /// App-side dispatcher.
    pub dispatcher: BindDispatcher,
    /// Opaque arg (`exec` command, workspace number, …).
    pub arg: Option<String>,
}

/// One user bind from `hyprctl binds -j`: key plus modmask plus
/// submap for display (INP-01). The dispatcher field stays opaque
/// (lua-era binds report `__lua`) and is never matched on.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct UserBind {
    /// Key token as reported by the compositor.
    #[serde(default)]
    pub key: String,
    /// Modifier bitmask (SHIFT=1, CTRL=4, ALT=8, SUPER=64).
    #[serde(default)]
    pub modmask: u32,
    /// Active submap name (empty at toplevel).
    #[serde(default)]
    pub submap: String,
    /// Opaque dispatcher text (`__lua` for lua-era binds).
    #[serde(default)]
    pub dispatcher: String,
}

/// Validates one modifier word list into a modmask. Words fold case;
/// only SUPER/CTRL/ALT/SHIFT pass — anything else fails typed BEFORE
/// any prompt, spawn, or write.
pub fn mods_to_modmask(words: &[&str]) -> Result<u32, ArchelError> {
    let mut mask = 0u32;
    for word in words {
        let upper = word.trim().to_uppercase();
        let bit = MOD_WORDS
            .iter()
            .find(|(name, _)| *name == upper)
            .map(|(_, bit)| *bit)
            .ok_or_else(|| {
                ArchelError::desktop_invalid(format!("rejected bind modifier {word:?}"))
            })?;
        mask |= bit;
    }
    Ok(mask)
}

/// Parses one `SUPER + CTRL` mods string into a modmask (empty or
/// blank means modeless, mask 0). Pure for headless coverage.
pub fn parse_mods(raw: &str) -> Result<u32, ArchelError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(0);
    }
    let words: Vec<&str> = trimmed.split('+').map(str::trim).filter(|w| !w.is_empty()).collect();
    mods_to_modmask(&words)
}

/// Decodes a modmask into canonical modifier words (canonical order:
/// SUPER, CTRL, ALT, SHIFT). Bits outside the verified set are
/// dropped for display — the vocabulary hint only needs the known
/// words, never a guess.
pub fn modmask_to_mods(mask: u32) -> Vec<String> {
    MOD_WORDS
        .iter()
        .filter(|(_, bit)| mask & bit != 0)
        .map(|(name, _)| name.to_string())
        .collect()
}

/// Renders canonical mods text from a modmask (`SUPER + CTRL`,
/// empty when modeless). Roundtrips with [`parse_mods`].
pub fn render_mods(mask: u32) -> String {
    modmask_to_mods(mask).join(" + ")
}

/// Validates one key token: non-empty, no whitespace/control/quotes
/// (the token re-enters Lua inside the bind string, so the charset
/// gate runs BEFORE any prompt, spawn, or write).
pub fn validate_key(raw: &str) -> Result<String, ArchelError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ArchelError::desktop_invalid("empty bind key"));
    }
    if trimmed.chars().any(|c| {
        c.is_whitespace()
            || c.is_control()
            || c == '"'
            || c == '\''
            || c == '\\'
            || c == ','
            || c == '('
            || c == ')'
    }) {
        return Err(ArchelError::desktop_invalid(format!(
            "rejected bind key {raw:?}"
        )));
    }
    Ok(trimmed.to_string())
}

/// Builds a validated [`BindSpec`] from raw CLI strings. Validates
/// FIRST (mods, key, dispatcher — WR-02), reads nothing: typos fail
/// fast with no TTY need. The `exec` arg is required for `exec`
/// binds and accepted opaque (lua-quoted at render, T-05-11).
pub fn plan_bind(
    mods: &str,
    key: &str,
    dispatcher: &str,
    arg: Option<&str>,
) -> Result<BindSpec, ArchelError> {
    let mask = parse_mods(mods)?;
    let key = validate_key(key)?;
    let dispatcher = BindDispatcher::parse(dispatcher)?;
    let arg = match arg.map(str::trim) {
        Some(text) if !text.is_empty() => Some(text.to_string()),
        _ => None,
    };
    if dispatcher.is_exec() && arg.is_none() {
        return Err(ArchelError::desktop_invalid(
            "exec binds need --arg (the command to run)",
        ));
    }
    Ok(BindSpec {
        mods: render_mods(mask),
        key,
        dispatcher,
        arg,
    })
}

/// Parses `hyprctl binds -j` output into user binds. Pure over the
/// text: blank/garbage input yields an empty list, malformed entries
/// are skipped, entries without a key are dropped. Never panics,
/// never spawns.
pub fn parse_binds_json(out: &str) -> Vec<UserBind> {
    let values: Vec<serde_json::Value> = match serde_json::from_str(out) {
        Ok(values) => values,
        Err(_) => return Vec::new(),
    };
    values
        .into_iter()
        .filter_map(|value| serde_json::from_value::<UserBind>(value).ok())
        .filter(|bind| !bind.key.trim().is_empty())
        .collect()
}

/// Human-readable user-bind inventory for INP-01 display. Serialized
/// as-is for `--json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BindStatus {
    pub key: String,
    pub mods: Vec<String>,
    pub submap: String,
    /// True for lines parsed from the app snippet (managed);
    /// false for compositor-reported user binds.
    pub managed: bool,
    /// Dispatcher word for managed binds; opaque compositor text
    /// (`__lua`) for user binds — never matched on.
    pub dispatcher: String,
}

impl From<&UserBind> for BindStatus {
    fn from(bind: &UserBind) -> Self {
        Self {
            key: bind.key.clone(),
            mods: modmask_to_mods(bind.modmask),
            submap: bind.submap.clone(),
            managed: false,
            dispatcher: bind.dispatcher.clone(),
        }
    }
}

impl From<&BindSpec> for BindStatus {
    fn from(spec: &BindSpec) -> Self {
        Self {
            key: spec.key.clone(),
            mods: if spec.mods.is_empty() {
                Vec::new()
            } else {
                spec.mods.split(" + ").map(str::to_string).collect()
            },
            submap: String::new(),
            managed: true,
            dispatcher: spec.dispatcher.name().to_string(),
        }
    }
}

/// Renders one exact `hl.bind(...)` snippet line for a spec. Pure
/// (golden-testable without spawning): mods/key route through the
/// validators, every string reaching Lua routes through
/// [`lua_quote`] (T-05-11).
pub fn render_bind_line(spec: &BindSpec) -> Result<String, ArchelError> {
    let combo = if spec.mods.is_empty() {
        spec.key.clone()
    } else {
        format!("{} + {}", spec.mods, spec.key)
    };
    let call = match spec.arg.as_deref() {
        Some(arg) => format!("hl.dsp.{}({})", spec.dispatcher.name(), lua_quote(arg)?),
        None => format!("hl.dsp.{}()", spec.dispatcher.name()),
    };
    Ok(format!("hl.bind({}, {})", lua_quote(&combo)?, call))
}

/// Parses one app-generated `hl.bind("MODS + KEY", hl.dsp.name("arg"))`
/// line back into a [`BindSpec`]. Pure over the text: blanks,
/// comments, foreign `hl.*` lines, and malformed binds yield `None`
/// (audio.rs:144-201 discipline — skip, never panic). Only the exact
/// app shape matches, so user-block lines never parse.
pub fn parse_bind_line(line: &str) -> Option<BindSpec> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("hl.bind(")?.strip_suffix(')')?;
    let (combo_raw, call_raw) = rest.split_once(',')?;
    let combo = unquote(combo_raw.trim())?;
    let (mods, key) = split_combo(&combo)?;
    let call = call_raw.trim().strip_prefix("hl.dsp.")?;
    let (name, arg_raw) = call.split_once('(')?;
    let dispatcher = BindDispatcher::parse(name.trim()).ok()?;
    let arg = {
        let inner = arg_raw.trim().strip_suffix(')')?.trim();
        if inner.is_empty() {
            None
        } else {
            Some(unquote(inner)?)
        }
    };
    let mask = parse_mods(&mods).ok()?;
    let key = validate_key(&key).ok()?;
    Some(BindSpec {
        mods: render_mods(mask),
        key,
        dispatcher,
        arg,
    })
}

/// Strips one layer of Lua double-quote wrapping (no escape
/// processing — rendered args never contain raw quotes thanks to
/// [`lua_quote`]). `None` for non-quoted text.
fn unquote(raw: &str) -> Option<String> {
    raw.strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .map(str::to_string)
}

/// Splits one `MODS + KEY` combo: the last `+`-separated token is
/// the key, the rest are mods. A bare key (no `+`) is modeless.
fn split_combo(combo: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = combo.split('+').map(str::trim).collect();
    if parts.iter().any(|part| part.is_empty()) {
        return None;
    }
    let key = (*parts.last()?).to_string();
    let mods = parts[..parts.len() - 1].join(" + ");
    Some((mods, key))
}

/// Key for exact-match addressing (D-06): normalized (mods, key,
/// dispatcher). The arg is payload, never address.
fn bind_address(spec: &BindSpec) -> (String, String, String) {
    (
        spec.mods.clone(),
        spec.key.clone(),
        spec.dispatcher.name().to_string(),
    )
}

/// Human-readable bind summary (`SUPER + T -> exec "kitty"`).
pub fn summarize_bind(spec: &BindSpec) -> String {
    let combo = if spec.mods.is_empty() {
        spec.key.clone()
    } else {
        format!("{} + {}", spec.mods, spec.key)
    };
    match spec.arg.as_deref() {
        Some(arg) => format!("{combo} -> {} {arg:?}", spec.dispatcher.name()),
        None => format!("{combo} -> {}", spec.dispatcher.name()),
    }
}

/// Adds one bind line to a snippet body, idempotently: an existing
/// app line with the same (mods, key, dispatcher) address is
/// replaced in place (same bind twice yields one line); otherwise
/// the rendered line appends. Sibling lines (other binds, `hl.*`
/// calls, comments) pass through byte-identical.
pub fn add_bind_line(body: &str, spec: &BindSpec) -> Result<String, ArchelError> {
    let rendered = render_bind_line(spec)?;
    let address = bind_address(spec);
    let mut replaced = false;
    let mut out = String::new();
    for body_line in body.lines() {
        match parse_bind_line(body_line) {
            Some(existing) if bind_address(&existing) == address && !replaced => {
                out.push_str(&rendered);
                out.push('\n');
                replaced = true;
            }
            _ => {
                out.push_str(body_line);
                out.push('\n');
            }
        }
    }
    if !replaced {
        out.push_str(&rendered);
        out.push('\n');
    }
    Ok(out)
}

/// Replaces the app line at `old`'s exact address with `new`.
/// A miss fails typed-DesktopInvalid BEFORE any prompt, spawn, or
/// write — user-block lines never match (they never parse).
pub fn edit_bind_line(body: &str, old: &BindSpec, new: &BindSpec) -> Result<String, ArchelError> {
    let rendered = render_bind_line(new)?;
    let address = bind_address(old);
    let mut replaced = false;
    let mut out = String::new();
    for body_line in body.lines() {
        match parse_bind_line(body_line) {
            Some(existing) if bind_address(&existing) == address && !replaced => {
                out.push_str(&rendered);
                out.push('\n');
                replaced = true;
            }
            _ => {
                out.push_str(body_line);
                out.push('\n');
            }
        }
    }
    if !replaced {
        return Err(ArchelError::desktop_invalid(format!(
            "no managed bind at {} (nothing edited)",
            summarize_bind(old)
        )));
    }
    Ok(out)
}

/// Removes the app line at `target`'s exact address. A miss fails
/// typed-DesktopInvalid; sibling lines survive byte-identical.
pub fn remove_bind_line(body: &str, target: &BindSpec) -> Result<String, ArchelError> {
    let address = bind_address(target);
    let mut removed = false;
    let mut out = String::new();
    for body_line in body.lines() {
        match parse_bind_line(body_line) {
            Some(existing) if bind_address(&existing) == address && !removed => {
                removed = true;
            }
            _ => {
                out.push_str(body_line);
                out.push('\n');
            }
        }
    }
    if !removed {
        return Err(ArchelError::desktop_invalid(format!(
            "no managed bind at {} (nothing removed)",
            summarize_bind(target)
        )));
    }
    Ok(out)
}

/// Parses every managed bind out of a snippet body (foreign lines
/// skipped, never an error).
pub fn parse_snippet_binds(body: &str) -> Vec<BindSpec> {
    body.lines().filter_map(parse_bind_line).collect()
}

/// Collision hint: when `spec`'s (mods, key) already exists in the
/// `binds -j` vocabulary, name it so the preview warns about the
/// duplicate. `None` means no collision. Display-only — user binds
/// are never edited (D-06).
pub fn collision_hint(binds: &[UserBind], spec: &BindSpec) -> Option<String> {
    let mask = parse_mods(&spec.mods).unwrap_or(0);
    binds
        .iter()
        .find(|bind| bind.modmask == mask && bind.key == spec.key)
        .map(|bind| {
            let mods = render_mods(bind.modmask);
            if mods.is_empty() {
                format!("key {:?} is already bound (compositor inventory)", bind.key)
            } else {
                format!(
                    "combo {} + {} is already bound (compositor inventory)",
                    mods, bind.key
                )
            }
        })
}

/// French preview title (shown before `[o/N]`).
pub fn keybinds_title() -> String {
    rust_i18n::t!("input.keybinds_title").to_string()
}

/// French rows for the bind inventory table: managed binds carry the
/// managed marker, user binds the opaque dispatcher text.
pub fn keybind_rows(statuses: &[BindStatus]) -> Vec<(String, String)> {
    statuses
        .iter()
        .map(|status| {
            let combo = if status.mods.is_empty() {
                status.key.clone()
            } else {
                format!("{} + {}", status.mods.join(" + "), status.key)
            };
            let marker = if status.managed {
                rust_i18n::t!("input.managed_marker").to_string()
            } else {
                status.dispatcher.clone()
            };
            (combo, marker)
        })
        .collect()
}

/// French confirmation prompt naming the planned bind.
pub fn confirm_bind_prompt(spec: &BindSpec) -> String {
    rust_i18n::t!("input.confirm_bind", change = summarize_bind(spec)).to_string()
}

/// French ok line naming the applied bind.
pub fn bind_ok_line(spec: &BindSpec) -> String {
    rust_i18n::t!("input.bind_ok", change = summarize_bind(spec)).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed live-shaped `hyprctl binds -j` capture: one lua-era
    /// opaque entry plus one plain entry (fixtures live in-module —
    /// the repo keeps no tests/ dir).
    const BINDS_FIXTURE: &str = r#"[
        {"key": "V", "modmask": 65, "submap": "", "dispatcher": "__lua", "arg": "12"},
        {"key": "Return", "modmask": 64, "submap": "", "dispatcher": "__lua", "arg": "13"},
        {"key": "1", "modmask": 64, "submap": "", "dispatcher": "workspace", "arg": "1"}
    ]"#;

    #[test]
    fn modmask_codec_roundtrips_the_verified_bits() {
        // Live-verified single bits first.
        assert_eq!(mods_to_modmask(&["SHIFT"]).expect("shift maps"), 1);
        assert_eq!(mods_to_modmask(&["CTRL"]).expect("ctrl maps"), 4);
        assert_eq!(mods_to_modmask(&["ALT"]).expect("alt maps"), 8);
        assert_eq!(mods_to_modmask(&["SUPER"]).expect("super maps"), 64);
        // Plan-mandated roundtrips: CTRL+ALT to 12, SUPER to 64,
        // SUPER+SHIFT to 65.
        assert_eq!(parse_mods("CTRL + ALT").expect("ctrl+alt parses"), 12);
        assert_eq!(parse_mods("SUPER").expect("super parses"), 64);
        assert_eq!(parse_mods("SUPER + SHIFT").expect("super+shift parses"), 65);
        assert_eq!(render_mods(12), "CTRL + ALT");
        assert_eq!(render_mods(64), "SUPER");
        assert_eq!(render_mods(65), "SUPER + SHIFT");
        assert_eq!(render_mods(0), "");
        // Case folds, order normalizes to canonical.
        assert_eq!(parse_mods("shift + super").expect("folds"), 65);
        assert_eq!(render_mods(parse_mods("ALT + CTRL").expect("parses")), "CTRL + ALT");
        // Unknown words fail typed, never silently drop; empty
        // is modeless (mask 0).
        assert_eq!(parse_mods("").expect("empty is modeless"), 0);
        for bad in ["META", "WIN", "SUPERB"] {
            let err = parse_mods(bad).expect_err("unknown mod must fail");
            assert!(matches!(err, ArchelError::DesktopInvalid { .. }), "{err:?}");
        }
    }

    #[test]
    fn binds_fixture_parses_key_modmask_and_submap() {
        let binds = parse_binds_json(BINDS_FIXTURE);
        assert_eq!(binds.len(), 3, "all fixture binds parse: {binds:?}");
        assert_eq!(binds[0].key, "V");
        assert_eq!(binds[0].modmask, 65);
        assert_eq!(modmask_to_mods(binds[0].modmask), vec!["SUPER", "SHIFT"]);
        // Dispatcher text stays opaque — display only, never matched.
        assert_eq!(binds[0].dispatcher, "__lua");
        // Garbage degrades to empty, never a panic.
        assert!(parse_binds_json("").is_empty());
        assert!(parse_binds_json("not json").is_empty());
        assert!(parse_binds_json(r#"[{"modmask": 64}]"#).is_empty());
    }

    #[test]
    fn bind_line_renders_and_parses_back() {
        let spec = plan_bind("SUPER", "T", "exec", Some("kitty")).expect("plans");
        let line = render_bind_line(&spec).expect("renders");
        assert_eq!(line, r#"hl.bind("SUPER + T", hl.dsp.exec("kitty"))"#);
        let back = parse_bind_line(&line).expect("parses back");
        assert_eq!(back, spec, "render/parse round-trips");
        // Modeless binds render bare-keyed.
        let bare = plan_bind("", "F1", "killactive", None).expect("modeless plans");
        let bare_line = render_bind_line(&bare).expect("renders");
        assert_eq!(bare_line, r#"hl.bind("F1", hl.dsp.killactive())"#);
        assert_eq!(parse_bind_line(&bare_line).expect("parses"), bare);
        // Foreign lines never parse: comments, other hl calls,
        // malformed binds, user-shaped lines without hl.dsp.
        for foreign in [
            "",
            "-- a comment",
            "hl.config({ input = { sensitivity = 0.5 } })",
            "hl.monitor({ output = \"HDMI-A-2\" })",
            "hl.bind(\"SUPER + T\")",
            "bind = SUPER, T, exec, kitty",
            "hl.bind(SUPER + T, hl.dsp.exec(\"kitty\"))",
        ] {
            assert!(
                parse_bind_line(foreign).is_none(),
                "foreign line must not parse: {foreign:?}"
            );
        }
    }

    #[test]
    fn exec_args_pass_through_lua_quote() {
        // Hostile arg content reaches the snippet only quoted —
        // the T-05-11 injection gate in the render path.
        let spec = plan_bind("SUPER", "T", "exec", Some(r#"a"b\c"#)).expect("plans");
        let line = render_bind_line(&spec).expect("renders");
        assert!(
            line.contains(r#""a\"b\\c""#),
            "arg must lua-quote through render: {line:?}"
        );
        // Newline args never reach eval or snippet text.
        let bad = plan_bind("SUPER", "T", "exec", Some("has\nnewline"));
        assert!(bad.is_ok(), "planning accepts the opaque string");
        assert!(
            render_bind_line(&bad.expect("plans")).is_err(),
            "render refuses line-breaking args"
        );
        // Exec without an arg fails at plan time, before any prompt.
        let err = plan_bind("SUPER", "T", "exec", None).expect_err("exec needs --arg");
        assert!(matches!(err, ArchelError::DesktopInvalid { .. }), "{err:?}");
    }

    #[test]
    fn add_is_idempotent_and_never_touches_siblings() {
        let first = plan_bind("SUPER", "T", "exec", Some("kitty")).expect("plans");
        let sibling = plan_bind("SUPER", "Return", "exec", Some("foot")).expect("plans");
        let body = format!(
            "-- archel managed block\n{}\n{}\n",
            render_bind_line(&sibling).expect("renders"),
            "hl.config({ decoration = { blur = { enabled = true } } })"
        );
        let once = add_bind_line(&body, &first).expect("adds");
        assert_eq!(
            once.lines().filter(|l| parse_bind_line(l).is_some()).count(),
            2,
            "sibling plus new line: {once:?}"
        );
        let twice = add_bind_line(&once, &first).expect("re-adds");
        assert_eq!(
            twice.lines().filter(|l| parse_bind_line(l).is_some()).count(),
            2,
            "same bind twice yields one line: {twice:?}"
        );
        assert!(
            twice.contains("blur = { enabled = true }"),
            "sibling leaves survive: {twice:?}"
        );
    }

    #[test]
    fn edit_and_remove_exact_match_with_typed_misses() {
        let old = plan_bind("SUPER", "T", "exec", Some("kitty")).expect("plans");
        let new = plan_bind("SUPER", "T", "exec", Some("foot")).expect("plans");
        let sibling = plan_bind("SUPER + SHIFT", "T", "exec", Some("kitty")).expect("plans");
        let body = format!(
            "{}\n{}\n",
            render_bind_line(&old).expect("renders"),
            render_bind_line(&sibling).expect("renders")
        );
        let edited = edit_bind_line(&body, &old, &new).expect("edits");
        assert!(edited.contains("\"foot\""), "new arg lands: {edited:?}");
        assert!(
            edited.contains("SUPER + SHIFT"),
            "near-miss sibling untouched: {edited:?}"
        );
        assert_eq!(
            edited.lines().filter(|l| parse_bind_line(l).is_some()).count(),
            2,
            "edit replaces, never duplicates: {edited:?}"
        );
        let removed = remove_bind_line(&edited, &new).expect("removes");
        assert!(
            !removed.contains("\"foot\""),
            "removed line leaves: {removed:?}"
        );
        assert!(
            removed.contains("SUPER + SHIFT"),
            "sibling survives removal: {removed:?}"
        );
        // Misses fail typed before any prompt, spawn, or write.
        let ghost = plan_bind("ALT", "F4", "killactive", None).expect("plans");
        for result in [edit_bind_line(&removed, &ghost, &new), remove_bind_line(&removed, &ghost)] {
            let err = result.expect_err("miss must fail");
            assert!(matches!(err, ArchelError::DesktopInvalid { .. }), "{err:?}");
        }
    }

    #[test]
    fn user_block_sentinel_lines_never_match() {
        // A body mixing app lines with foreign user-shaped content:
        // only the app line is addressable, the rest pass through.
        let app = plan_bind("SUPER", "T", "exec", Some("kitty")).expect("plans");
        let body = format!(
            "-- user binds below (never managed)\n\
             bind = SUPER, T, exec, kitty\n\
             {}\n",
            render_bind_line(&app).expect("renders")
        );
        // Same visible combo in user syntax is NOT the app address:
        // removing the app bind leaves the user line alone.
        let removed = remove_bind_line(&body, &app).expect("removes the app line");
        assert!(
            removed.contains("bind = SUPER, T, exec, kitty"),
            "user-block line survives: {removed:?}"
        );
        assert!(
            !removed.contains("hl.bind("),
            "only the app line leaves: {removed:?}"
        );
    }

    #[test]
    fn collision_hint_names_binds_j_vocabulary() {
        let binds = parse_binds_json(BINDS_FIXTURE);
        // SUPER+Return collides with the fixture inventory.
        let taken = plan_bind("SUPER", "Return", "exec", Some("kitty")).expect("plans");
        let hint = collision_hint(&binds, &taken).expect("hint fires");
        assert!(hint.contains("Return"), "hint names the key: {hint:?}");
        // SUPER+T is free in the fixture vocabulary.
        let free = plan_bind("SUPER", "T", "exec", Some("kitty")).expect("plans");
        assert!(
            collision_hint(&binds, &free).is_none(),
            "free combos stay silent"
        );
    }

    #[test]
    fn dispatcher_vocabulary_accepts_the_documented_set() {
        for known in [
            "exec",
            "killactive",
            "togglefloating",
            "workspace",
            "movetoworkspace",
            "fullscreen",
            "togglesplit",
        ] {
            assert!(BindDispatcher::parse(known).is_ok(), "{known:?} parses");
        }
        // Pass-through for plausible Hyprland words, typed reject
        // for shell-shaped names.
        assert!(matches!(
            BindDispatcher::parse("cyclenext"),
            Ok(BindDispatcher::Other(_))
        ));
        for bad in ["", "exec; rm -rf ~", "kill active", "EXEC\"("] {
            assert!(
                BindDispatcher::parse(bad).is_err(),
                "dispatcher {bad:?} must fail"
            );
        }
        assert!(KNOWN_DISPATCHERS.contains(&"exec"));
    }

    #[test]
    fn key_validator_rejects_lua_breaking_tokens() {
        for bad in ["", "   ", "a b", "a\"b", "a\\b", "a\nb"] {
            assert!(validate_key(bad).is_err(), "key {bad:?} must fail");
        }
        for good in ["V", "Return", "F12", "1", "KP_1", "minus"] {
            assert!(validate_key(good).is_ok(), "key {good:?} passes");
        }
    }

    #[test]
    fn labels_come_from_the_catalog() {
        crate::i18n::set_language("fr");
        assert!(!keybinds_title().is_empty());
        let spec = plan_bind("SUPER", "T", "exec", Some("kitty")).expect("plans");
        assert!(!confirm_bind_prompt(&spec).is_empty());
        assert!(!bind_ok_line(&spec).is_empty());
        let statuses: Vec<BindStatus> =
            parse_binds_json(BINDS_FIXTURE).iter().map(BindStatus::from).collect();
        assert_eq!(keybind_rows(&statuses).len(), 3);
    }
}
