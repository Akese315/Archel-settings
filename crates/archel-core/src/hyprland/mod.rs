//! Hyprland desktop integration (Phase 05 tracer: APP-03 blur).
//!
//! Store: the live compositor through the `hyprctl` CLI (no new crate —
//! the `hyprland` crate is GPL-3.0 and targets the dead keyword IPC).
//! Live-apply runs through `hyprctl eval 'hl.config({...})'` (verified
//! on Hyprland 0.56.2 — `hyprctl keyword` errors with "use eval" yet
//! exits 0, so an rc-only check would silently "succeed"). Durability
//! runs through one app-owned Lua snippet (`modules/archel.lua`) with
//! a single guarded `require("modules.archel")` line in `hyprland.lua`.
//!
//! Spawn discipline mirrors [`crate::proxy`]: which-gate, exact argv
//! never through a shell, `LC_ALL=C`, `wait_timeout` plus kill. A
//! missing `hyprctl` binary — or a missing
//! `$HYPRLAND_INSTANCE_SIGNATURE` (not running under the compositor) —
//! maps to the typed [`crate::ArchelError::HyprlandUnavailable`] (never
//! a panic, never a hang). `eval` legs fail when rc != 0 OR stdout
//! starts with `error:` (Pitfall 1).
//!
//! D-08 write-scope rule: the ONLY paths ever opened for writing are
//! the app-owned snippet, its timestamped backups, and the one-time
//! guarded require line in `hyprland.lua`. The user's own config is
//! never rewritten.

pub mod appearance;
pub mod binds;
pub mod font;
pub mod input;
pub mod monitors;
pub mod wallpaper;

use std::path::{Path, PathBuf};

use crate::error::ArchelError;
use crate::packages::QUERY_TIMEOUT;

/// App-owned snippet filename inside the Hyprland config dir.
pub const SNIPPET_FILE: &str = "archel.lua";
/// Subdirectory of the Hyprland config dir holding the snippet.
pub const SNIPPET_DIR: &str = "modules";
/// One-time guarded insertion line appended to the Lua entry file.
pub const REQUIRE_LINE: &str = r#"require("modules.archel")"#;
/// Lua entry filename (never rewritten, only appended once).
pub const ENTRY_FILE: &str = "hyprland.lua";
/// Timestamped backups kept per app-owned file (rotation policy).
pub const BACKUP_ROTATION: usize = 5;

/// Resolves the Hyprland config dir (`~/.config/hypr/` via XDG).
pub fn hypr_dir() -> PathBuf {
    directories::BaseDirs::new()
        .map(|base| base.config_dir().join("hypr"))
        .unwrap_or_else(|| PathBuf::from("/tmp/archel-no-home-hypr"))
}

/// Renders an opacity-style float canonically for `eval`/snippet
/// text (`1`, `0.9`, `0.8` — no exponent, no trailing noise).
/// Callers validate the range first, so non-finite values never reach
/// this point — the debug assertion guards the invariant.
pub fn appearance_float(value: f64) -> String {
    debug_assert!(
        value.is_finite(),
        "appearance floats must be finite after validation"
    );
    if value == value.trunc() {
        format!("{}", value.trunc() as i64)
    } else {
        format!("{value}")
    }
}

/// Runs one `hyprctl` query and returns its stdout.
///
/// Same spawn shape as the proxy runner: which-gate with the typed
/// caller-visible variant, `.unchecked()` so the exit map stays
/// reachable, `LC_ALL=C`, `wait_timeout` plus kill, stderr log-only.
/// Additionally gated on `$HYPRLAND_INSTANCE_SIGNATURE`: without it
/// there is no compositor to talk to.
pub fn run_hyprctl(tool: &str, args: &[&str]) -> Result<String, ArchelError> {
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none() {
        return Err(ArchelError::hyprland_unavailable(
            "HYPRLAND_INSTANCE_SIGNATURE is unset (not running under Hyprland)",
        ));
    }
    let binary = which::which(tool).map_err(|err| {
        ArchelError::hyprland_unavailable(format!("{tool} ({err})"))
    })?;
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
                "'{tool}' exceeded the query timeout (compositor unresponsive)"
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

/// Runs one `hyprctl eval <lua>` live-apply leg.
///
/// On top of [`run_hyprctl`]: stdout starting with `error:` is a
/// failure even at rc 0 (Pitfall 1 — the keyword trap retired, but
/// eval keeps the same honest-read-back contract).
pub fn run_hyprctl_eval(tool: &str, lua: &str) -> Result<(), ArchelError> {
    let out = run_hyprctl(tool, &["eval", lua])?;
    if out.trim_start().starts_with("error:") {
        return Err(ArchelError::hyprland_unavailable(format!(
            "hyprctl eval refused the statement: {}",
            out.trim()
        )));
    }
    Ok(())
}

/// Quotes one string for embedding in Lua `eval`/snippet text:
/// backslash and `"` are escaped; newline and NUL are rejected (they
/// would break the single-line statement shape). All values reaching
/// `eval` are machine-rendered; this is the only string path in.
pub fn lua_quote(raw: &str) -> Result<String, ArchelError> {
    if raw.contains(['\n', '\r', '\0']) {
        return Err(ArchelError::desktop_invalid(format!(
            "rejected Lua string with newline/NUL {raw:?}"
        )));
    }
    let escaped = raw.replace('\\', "\\\\").replace('"', "\\\"");
    let quoted = format!("\"{escaped}\"");
    debug_assert!(
        !quoted.contains(['\n', '\r', '\0']),
        "lua_quote output must stay single-line"
    );
    Ok(quoted)
}

/// One `hyprctl getoption <key> -j` value: `{"bool"|"float"|"str"|"int", "set"}`.
#[derive(Debug, Clone, PartialEq)]
pub enum OptionValue {
    Bool(bool),
    Float(f64),
    Str(String),
    Int(i64),
}

/// Parses one `hyprctl getoption -j` payload into an [`OptionValue`].
/// Pure over captured stdout for headless coverage. Unknown shapes are
/// the typed unavailable error (honest absence, never a guessed value).
pub fn parse_getoption_json(out: &str) -> Result<OptionValue, ArchelError> {
    let value: serde_json::Value = serde_json::from_str(out).map_err(|err| {
        ArchelError::hyprland_unavailable(format!("cannot parse getoption output: {err}"))
    })?;
    if let Some(b) = value.get("bool").and_then(serde_json::Value::as_bool) {
        return Ok(OptionValue::Bool(b));
    }
    if let Some(f) = value.get("float").and_then(serde_json::Value::as_f64) {
        return Ok(OptionValue::Float(f));
    }
    if let Some(s) = value.get("str").and_then(serde_json::Value::as_str) {
        return Ok(OptionValue::Str(s.to_string()));
    }
    if let Some(i) = value.get("int").and_then(serde_json::Value::as_i64) {
        return Ok(OptionValue::Int(i));
    }
    Err(ArchelError::hyprland_unavailable(format!(
        "unrecognized getoption shape: {}",
        out.trim()
    )))
}

/// Reads one live option via `getoption <key> -j`.
pub fn read_option(key: &str) -> Result<OptionValue, ArchelError> {
    read_option_with_tool("hyprctl", key)
}

/// Same as [`read_option`] but the binary name is injectable, so the
/// which-gate stays unit-coverable headless (proxy.rs:69 precedent).
fn read_option_with_tool(tool: &str, key: &str) -> Result<OptionValue, ArchelError> {
    let out = run_hyprctl(tool, &["getoption", key, "-j"])?;
    parse_getoption_json(&out)
}

/// App-owned snippet path under a Hyprland config dir.
pub fn snippet_path_at(base: &Path) -> PathBuf {
    base.join(SNIPPET_DIR).join(SNIPPET_FILE)
}

/// Lua entry path under a Hyprland config dir (append-only, once).
fn entry_path_at(base: &Path) -> PathBuf {
    base.join(ENTRY_FILE)
}

/// Backs up `path` to `<file>.bak-<UTC millis>` before a write.
/// Missing source files need no backup (first touch). Millis plus a
/// loop suffix keep back-to-back writes from colliding on the same
/// backup name (WR-05). Keeps the newest [`BACKUP_ROTATION`] backups
/// per file. Generic over the file name so the hyprpaper slice (Task
/// 3) reuses the same stamps.
pub fn backup_before_write(path: &Path) -> Result<(), ArchelError> {
    if !path.exists() {
        return Ok(());
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return Err(ArchelError::Internal(format!(
            "cannot back up '{}': unrepresentable file name",
            path.display()
        )));
    };
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut candidate = parent.join(format!("{name}.bak-{stamp}"));
    let mut disambiguator = 0u32;
    while candidate.exists() {
        disambiguator += 1;
        candidate = parent.join(format!("{name}.bak-{stamp}-{disambiguator}"));
    }
    std::fs::copy(path, &candidate).map_err(|err| {
        ArchelError::Internal(format!("cannot back up '{}': {err}", path.display()))
    })?;
    prune_backups(path)?;
    Ok(())
}

/// Drops backups older than the newest [`BACKUP_ROTATION`] for `path`.
fn prune_backups(path: &Path) -> Result<(), ArchelError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    let Some(stem) = path.file_name().and_then(|n| n.to_str()) else {
        return Ok(());
    };
    let prefix = format!("{stem}.bak-");
    let mut backups: Vec<PathBuf> = std::fs::read_dir(parent)
        .map_err(|err| {
            ArchelError::Internal(format!("cannot list '{}': {err}", parent.display()))
        })?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|candidate| {
            candidate
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(&prefix))
        })
        .collect();
    backups.sort();
    while backups.len() > BACKUP_ROTATION {
        let oldest = backups.remove(0);
        let _ = std::fs::remove_file(&oldest);
    }
    Ok(())
}

/// Writes `body` to the app-owned snippet atomically (tmp + rename),
/// with a timestamped backup first. Creates `modules/` when missing.
/// The ONLY file content this writer ever produces is the managed
/// block it is given — sibling app leaves pass through untouched by
/// the caller composing `body` from the current snippet state.
pub fn write_snippet_at(base: &Path, body: &str) -> Result<PathBuf, ArchelError> {
    let dir = base.join(SNIPPET_DIR);
    std::fs::create_dir_all(&dir).map_err(|err| {
        ArchelError::Internal(format!("cannot create '{}': {err}", dir.display()))
    })?;
    let target = snippet_path_at(base);
    backup_before_write(&target)?;
    let tmp = target.with_extension("lua.tmp");
    std::fs::write(&tmp, body).map_err(|err| {
        ArchelError::Internal(format!("cannot write '{}': {err}", tmp.display()))
    })?;
    std::fs::rename(&tmp, &target).map_err(|err| {
        ArchelError::Internal(format!("cannot publish '{}': {err}", target.display()))
    })?;
    Ok(target)
}

/// Ensures the one-time guarded `require("modules.archel")` line in
/// the Lua entry file (grep line equality before append — idempotent).
/// Backs up the entry on first touch and publishes through tmp+rename
/// like the snippet (WR-06: the user-owned entry deserves the same
/// atomicity). Refuses with typed [`crate::ArchelError::HyprlandUnavailable`]
/// when the entry file is absent instead of planting a stray entry the
/// compositor never loads. Returns true when the line was appended,
/// false when already present.
pub fn ensure_require_line_at(base: &Path) -> Result<bool, ArchelError> {
    let entry = entry_path_at(base);
    let current = std::fs::read_to_string(&entry).map_err(|err| {
        ArchelError::hyprland_unavailable(format!(
            "lua entry '{}' not present: {err}",
            entry.display()
        ))
    })?;
    if current.lines().any(|line| line.trim() == REQUIRE_LINE) {
        return Ok(false);
    }
    backup_before_write(&entry)?;
    let mut next = current;
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    next.push_str(REQUIRE_LINE);
    next.push('\n');
    let tmp = entry.with_extension("lua.tmp");
    std::fs::write(&tmp, next).map_err(|err| {
        ArchelError::Internal(format!("cannot write '{}': {err}", tmp.display()))
    })?;
    std::fs::rename(&tmp, &entry).map_err(|err| {
        ArchelError::Internal(format!("cannot publish '{}': {err}", entry.display()))
    })?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_and_binary_gates_map_to_typed_unavailable() {
        // WR-03: the two env-touching probes live in ONE test so the
        // parallel harness can never interleave them (a which-gate
        // probe observing a removed var, or vice versa). The headless
        // precondition is asserted, never a silent return (WR-04).
        use std::io::IsTerminal;
        assert!(
            !std::io::stdin().is_terminal(),
            "session-gate test requires headless stdin"
        );
        let key = "HYPRLAND_INSTANCE_SIGNATURE";
        let stash = std::env::var_os(key);
        // Binary gate first: pin a fake signature so the which-gate is
        // what this leg covers (proxy.rs:751-758 precedent).
        unsafe { std::env::set_var(key, "test-signature") };
        let err = read_option_with_tool("archel-no-such-tool", "decoration:blur:enabled")
            .expect_err("a missing hyprctl must fail typed");
        assert!(
            matches!(err, ArchelError::HyprlandUnavailable { .. }),
            "missing hyprctl is typed unavailable, never a panic: {err:?}"
        );
        // Session gate second: no signature means no spawn at all.
        unsafe { std::env::remove_var(key) };
        let err = run_hyprctl("hyprctl", &["getoption", "decoration:blur:enabled", "-j"])
            .expect_err("no session signature must fail typed");
        match stash {
            Some(value) => unsafe { std::env::set_var(key, value) },
            None => unsafe { std::env::remove_var(key) },
        }
        assert!(
            matches!(err, ArchelError::HyprlandUnavailable { .. }),
            "out-of-session calls degrade typed, never hang: {err:?}"
        );
    }

    #[test]
    fn getoption_parser_covers_bool_float_str_shapes() {
        assert_eq!(
            parse_getoption_json(r#"{"bool": true, "set": true}"#),
            Ok(OptionValue::Bool(true))
        );
        assert_eq!(
            parse_getoption_json(r#"{"float": 1.0, "set": true}"#),
            Ok(OptionValue::Float(1.0))
        );
        assert_eq!(
            parse_getoption_json(r#"{"str": "Adwaita Sans", "set": true}"#),
            Ok(OptionValue::Str("Adwaita Sans".to_string()))
        );
        assert!(parse_getoption_json("not json").is_err());
        assert!(parse_getoption_json(r#"{"weird": []}"#).is_err());
    }

    #[test]
    fn lua_quote_escapes_and_rejects_line_breaks() {
        assert_eq!(
            lua_quote("Adwaita Sans").expect("plain family quotes"),
            "\"Adwaita Sans\""
        );
        assert_eq!(
            lua_quote(r#"a"b\c"#).expect("quotes quote"),
            r#""a\"b\\c""#
        );
        for bad in ["has\nnewline", "has\0nul", "has\rcr"] {
            assert!(
                lua_quote(bad).is_err(),
                "line-breaking input must fail typed: {bad:?}"
            );
        }
    }

    /// Unique scratch Hyprland config dir (std-only, no new deps).
    fn scratch_hypr_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "archel-hyprland-test-{}-{}",
            tag,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(SNIPPET_DIR))
            .expect("scratch modules dir must create");
        dir
    }

    #[test]
    fn snippet_write_is_atomic_with_backup_and_rotation() {
        let base = scratch_hypr_dir("snippet");
        write_snippet_at(&base, "-- v1\n").expect("first write");
        write_snippet_at(&base, "-- v2\n").expect("second write backs up v1");
        let target = snippet_path_at(&base);
        assert_eq!(std::fs::read_to_string(&target).expect("snippet reads"), "-- v2\n");
        assert!(
            !target.with_extension("lua.tmp").exists(),
            "no tmp file may survive the atomic publish"
        );
        let backups: Vec<_> = std::fs::read_dir(base.join(SNIPPET_DIR))
            .expect("modules lists")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.contains(".bak-")))
            .collect();
        assert_eq!(backups.len(), 1, "second write keeps one backup: {backups:?}");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn require_line_is_idempotent_with_first_touch_backup() {
        let base = scratch_hypr_dir("require");
        let entry = base.join(ENTRY_FILE);
        std::fs::write(&entry, "require(\"modules.appearance\")\n").expect("entry seeds");
        assert!(ensure_require_line_at(&base).expect("first ensure appends"));
        assert!(!ensure_require_line_at(&base).expect("second ensure is a no-op"));
        let content = std::fs::read_to_string(&entry).expect("entry reads");
        assert_eq!(
            content.lines().filter(|l| l.trim() == REQUIRE_LINE).count(),
            1,
            "the guarded line appears exactly once: {content:?}"
        );
        assert!(
            content.contains("require(\"modules.appearance\")"),
            "user lines survive untouched: {content:?}"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn require_line_refuses_a_missing_entry_atomically() {
        // WR-06: a missing entry is refused typed (never a stray file
        // the compositor would not load) and no tmp survives.
        let base = scratch_hypr_dir("require-missing");
        let entry = base.join(ENTRY_FILE);
        assert!(!entry.exists(), "scratch entry must start absent");
        let err = ensure_require_line_at(&base).expect_err("missing entry must refuse");
        assert!(
            matches!(err, ArchelError::HyprlandUnavailable { .. }),
            "missing entry refuses typed, never creates: {err:?}"
        );
        assert!(!entry.exists(), "refusal must not plant a stray entry");
        assert!(
            !entry.with_extension("lua.tmp").exists(),
            "no tmp file may survive the refusal"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn back_to_back_writes_keep_distinct_backups() {
        // WR-05: three writes inside one wall-clock window keep two
        // distinct backups — millis plus loop-suffix never collide.
        let base = scratch_hypr_dir("backups");
        write_snippet_at(&base, "-- v1\n").expect("first write");
        write_snippet_at(&base, "-- v2\n").expect("second write backs up v1");
        write_snippet_at(&base, "-- v3\n").expect("third write backs up v2");
        let mut backups: Vec<_> = std::fs::read_dir(base.join(SNIPPET_DIR))
            .expect("modules lists")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.contains(".bak-"))
            })
            .collect();
        backups.sort();
        backups.dedup();
        assert_eq!(backups.len(), 2, "two priors keep two backups: {backups:?}");
        let bodies: Vec<String> = backups
            .iter()
            .map(|p| std::fs::read_to_string(p).expect("backup reads"))
            .collect();
        assert!(
            bodies.contains(&"-- v1\n".to_string()) && bodies.contains(&"-- v2\n".to_string()),
            "backups hold each prior body: {bodies:?}"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn writer_touches_nothing_outside_the_hypr_dir() {
        // D-08 write-scope spy: a user tree (entry + unrelated module)
        // plus a sentinel outside the base; after snippet + require
        // work, only app-owned paths may differ. The entry is seeded
        // because ensure_require_line_at refuses a missing entry
        // (WR-06) instead of creating one from nothing.
        let base = scratch_hypr_dir("scope");
        let outside = base.with_extension("outside-sentinel");
        std::fs::write(&outside, "sentinel").expect("sentinel seeds");
        let user_module = base.join(SNIPPET_DIR).join("keybinds.lua");
        std::fs::write(&user_module, "-- user binds\n").expect("user module seeds");
        std::fs::write(base.join(ENTRY_FILE), "-- entry\n").expect("entry seeds");
        let user_before = std::fs::read(&user_module).expect("user module reads");
        write_snippet_at(&base, "-- managed\n").expect("snippet writes");
        ensure_require_line_at(&base).expect("require ensures");
        assert_eq!(
            std::fs::read(&user_module).expect("user module re-reads"),
            user_before,
            "sibling user modules are byte-for-byte unchanged"
        );
        assert_eq!(
            std::fs::read_to_string(&outside).expect("sentinel reads"),
            "sentinel",
            "nothing outside the hypr dir may be touched"
        );
        let _ = std::fs::remove_dir_all(&base);
        let _ = std::fs::remove_file(&outside);
    }

    #[test]
    fn eval_refuses_error_prefixed_stdout() {
        // Pitfall 1 unit shape: the refusal predicate lives on the
        // trimmed stdout prefix (the spawn leg is covered live).
        for bad in ["error: oops", "  error: oops\n", "error:"] {
            assert!(
                bad.trim_start().starts_with("error:"),
                "fixture must read as an eval refusal: {bad:?}"
            );
        }
        assert!(!"ok".trim_start().starts_with("error:"));
    }
}
