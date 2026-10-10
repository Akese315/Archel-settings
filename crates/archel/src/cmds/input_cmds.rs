//! `archel input keybinds|mouse` renderers: French tables (default)
//! or JSON (`--json`).
//!
//! Bin-calls-core shape: every French string comes from
//! `archel-core::hyprland::binds` or `::input` (titles, labels,
//! confirm prompts, ok lines) — this module only lays out columns
//! (`std::fmt` widths, no table crate) and frames the JSON payloads.
//! Keybind adds are confirm-gated in the bin (the `exec` arg is an
//! opaque future shell command — T-05-11); mouse sets delegate to
//! core's confirm-plus-apply (audio precedent: preview in the bin,
//! the single `[o/N]` inside core). Bind removals live-unbind via
//! `hl.unbind` before the snippet line leaves.

use archel_core::hyprland::binds as core_binds;
use archel_core::ArchelError;

/// Runs `archel input keybinds list`: user binds from `binds -j`
/// with decoded mods plus app binds from the snippet marked as
/// managed (INP-01). Read-only: no confirm, no write.
// gui: input.tick
pub fn run_keybinds_list(json: bool) -> Result<(), ArchelError> {
    let raw = archel_core::hyprland::run_hyprctl("hyprctl", &["binds", "-j"])?;
    let user = core_binds::parse_binds_json(&raw);
    let managed = read_managed_binds()?;
    run_keybinds_list_with(&user, &managed, json)
}

/// Inventory renderer over injected binds (headless-coverable; the
/// live reader above only supplies the two lists).
fn run_keybinds_list_with(
    user: &[core_binds::UserBind],
    managed: &[core_binds::BindSpec],
    json: bool,
) -> Result<(), ArchelError> {
    let mut statuses: Vec<core_binds::BindStatus> =
        managed.iter().map(core_binds::BindStatus::from).collect();
    statuses.extend(user.iter().map(core_binds::BindStatus::from));
    if json {
        println!("{}", encode_json(&statuses, "input keybinds")?);
    } else {
        print!(
            "{}",
            render_label_table(&core_binds::keybinds_title(), &core_binds::keybind_rows(&statuses))
        );
    }
    Ok(())
}

/// Runs `archel input keybinds add --mods <words> --key <k> --action
/// <dispatcher> [--arg <opaque>]`: validate, preview, confirm, eval
/// live-apply, snippet merge (INP-02).
// gui: input.add_keybind
pub fn run_keybind_add(
    mods: String,
    key: String,
    dispatcher: String,
    arg: Option<String>,
    json: bool,
) -> Result<(), ArchelError> {
    // Read-only planning first: validation plus the live vocabulary
    // read run before any prompt, so typos fail fast with no TTY need.
    let spec = core_binds::plan_bind(&mods, &key, &dispatcher, arg.as_deref())?;
    let raw = archel_core::hyprland::run_hyprctl("hyprctl", &["binds", "-j"])?;
    let binds = core_binds::parse_binds_json(&raw);
    let hint = core_binds::collision_hint(&binds, &spec);
    run_keybind_add_with(&spec, hint.as_deref(), json, |spec| {
        let line = core_binds::render_bind_line(spec)?;
        archel_core::hyprland::run_hyprctl_eval("hyprctl", &line)?;
        let body = read_snippet_body()?;
        publish_snippet_body(&core_binds::add_bind_line(&body, spec)?)
    })
}

/// Confirm-then-execute gate for bind adds: preview (plus the
/// collision hint when the combo is taken), TTY `[o/N]`, injectable
/// apply for the headless abort test.
fn run_keybind_add_with(
    spec: &core_binds::BindSpec,
    hint: Option<&str>,
    json: bool,
    apply: impl FnOnce(&core_binds::BindSpec) -> Result<(), ArchelError>,
) -> Result<(), ArchelError> {
    if !json {
        print!(
            "{}",
            render_label_table(
                &core_binds::keybinds_title(),
                &core_binds::keybind_rows(&[core_binds::BindStatus::from(spec)])
            )
        );
        if let Some(hint) = hint {
            println!("{hint}");
        }
    }
    archel_core::packages::confirm_action(&core_binds::confirm_bind_prompt(spec))?;
    apply(spec)?;
    let ok = core_binds::bind_ok_line(spec);
    if json {
        println!("{}", keybind_report_value("keybind-add", spec, &ok, None));
    } else {
        println!("{ok}");
    }
    Ok(())
}

/// Runs `archel input keybinds edit --mods/--key/--action (address)
/// [--new-mods/--new-key/--new-action/--new-arg]`: exact-match
/// addressed, preview old-arrow-new, confirm, eval plus snippet edit
/// (INP-02). A miss fails typed-DesktopInvalid before any prompt.
#[allow(clippy::too_many_arguments)]
// cli-only: composed in GUI via remove_keybind + add_keybind; no dedicated edit control in v1
pub fn run_keybind_edit(
    mods: String,
    key: String,
    dispatcher: String,
    new_mods: Option<String>,
    new_key: Option<String>,
    new_dispatcher: Option<String>,
    new_arg: Option<String>,
    json: bool,
) -> Result<(), ArchelError> {
    let old = core_binds::plan_bind(&mods, &key, &dispatcher, None)?;
    if new_mods.is_none() && new_key.is_none() && new_dispatcher.is_none() && new_arg.is_none() {
        return Err(ArchelError::desktop_invalid(
            "keybind edit needs at least one --new-* flag",
        ));
    }
    // A changed dispatcher drops the stale arg unless --new-arg says
    // otherwise; an unchanged dispatcher keeps it.
    let arg = if new_dispatcher.is_some() {
        new_arg.clone()
    } else {
        new_arg.clone().or(old_arg_of(&old))
    };
    let new = core_binds::plan_bind(
        new_mods.as_deref().unwrap_or(&old.mods),
        new_key.as_deref().unwrap_or(&old.key),
        new_dispatcher.as_deref().unwrap_or(old.dispatcher.name()),
        arg.as_deref(),
    )?;
    let body = read_snippet_body()?;
    run_keybind_edit_with(&old, &new, &body, json, |spec| {
        let line = core_binds::render_bind_line(spec)?;
        archel_core::hyprland::run_hyprctl_eval("hyprctl", &line)
    })
}

/// Reads the current arg for an addressed spec out of the snippet
/// (so an unchanged dispatcher keeps its arg across the edit).
fn old_arg_of(old: &core_binds::BindSpec) -> Option<String> {
    let _ = old;
    read_snippet_body()
        .ok()
        .and_then(|body| {
            core_binds::parse_snippet_binds(&body)
                .into_iter()
                .find(|existing| {
                    existing.mods == old.mods
                        && existing.key == old.key
                        && existing.dispatcher.name() == old.dispatcher.name()
                })
        })
        .and_then(|existing| existing.arg)
}

/// Confirm-then-execute gate for bind edits over an injected body
/// (headless-coverable): miss check first, then preview, prompt,
/// eval, snippet publish.
fn run_keybind_edit_with(
    old: &core_binds::BindSpec,
    new: &core_binds::BindSpec,
    body: &str,
    json: bool,
    eval: impl FnOnce(&core_binds::BindSpec) -> Result<(), ArchelError>,
) -> Result<(), ArchelError> {
    let merged = core_binds::edit_bind_line(body, old, new)?;
    if !json {
        print!(
            "{}",
            render_label_table(
                &core_binds::keybinds_title(),
                &core_binds::edit_preview_rows(old, new)
            )
        );
    }
    archel_core::packages::confirm_action(&core_binds::confirm_bind_prompt(new))?;
    eval(new)?;
    publish_snippet_body(&merged)?;
    let ok = core_binds::bind_ok_line(new);
    if json {
        println!("{}", keybind_report_value("keybind-edit", new, &ok, None));
    } else {
        println!("{ok}");
    }
    Ok(())
}

/// Runs `archel input keybinds remove --mods/--key/--action`:
/// exact-match addressed, confirm, `hl.unbind` live leg plus snippet
/// removal (INP-02). A refused unbind fails typed with the snippet
/// untouched. A miss fails typed-DesktopInvalid before any prompt.
// gui: input.remove_keybind
pub fn run_keybind_remove(
    mods: String,
    key: String,
    dispatcher: String,
    json: bool,
) -> Result<(), ArchelError> {
    let target = core_binds::plan_bind(&mods, &key, &dispatcher, None)?;
    let body = read_snippet_body()?;
    run_keybind_remove_with(&target, &body, json, |spec| {
        let stmt = core_binds::unbind_eval_statement(spec)?;
        archel_core::hyprland::run_hyprctl_eval("hyprctl", &stmt)
    })
}

/// Confirm-then-execute gate for bind removals over an injected body
/// (headless-coverable): miss check first, then preview, prompt,
/// unbind eval, snippet publish.
fn run_keybind_remove_with(
    target: &core_binds::BindSpec,
    body: &str,
    json: bool,
    eval: impl FnOnce(&core_binds::BindSpec) -> Result<(), ArchelError>,
) -> Result<(), ArchelError> {
    let merged = core_binds::remove_bind_line(body, target)?;
    if !json {
        print!(
            "{}",
            render_label_table(
                &core_binds::keybinds_title(),
                &core_binds::keybind_rows(&[core_binds::BindStatus::from(target)])
            )
        );
    }
    archel_core::packages::confirm_action(&core_binds::confirm_bind_remove_prompt(target))?;
    eval(target)?;
    publish_snippet_body(&merged)?;
    let ok = core_binds::bind_removed_line(target);
    if json {
        println!("{}", keybind_report_value("keybind-remove", target, &ok, None));
    } else {
        println!("{ok}");
    }
    Ok(())
}

/// Runs `archel input mouse` (bare): all three live leaves,
/// read-only (INP-03/04 reads).
// gui: input.tick
pub fn run_mouse(json: bool) -> Result<(), ArchelError> {
    use archel_core::hyprland::input as core_input;
    let state = core_input::read_input()?;
    if json {
        println!("{}", encode_json(&state, "input mouse")?);
    } else {
        print!(
            "{}",
            render_label_table(&core_input::mouse_title(), &core_input::mouse_rows(&state))
        );
    }
    Ok(())
}

/// Runs `archel input mouse set [--sensitivity <f>]
/// [--natural-scroll <on|off>] [--scroll-factor <f>]`: validated
/// leaves, preview, core confirm-plus-apply with read-back
/// (INP-03/04).
// gui: input.set_sensitivity,input.set_accelerate
pub fn run_mouse_set(
    sensitivity: Option<String>,
    natural_scroll: Option<String>,
    scroll_factor: Option<String>,
    json: bool,
) -> Result<(), ArchelError> {
    use archel_core::hyprland::input as core_input;
    // Read-only planning first: validation runs before any prompt,
    // so typos fail fast with no TTY need.
    let plan = core_input::plan_input_set_at(
        &archel_core::hyprland::hypr_dir(),
        sensitivity.as_deref(),
        natural_scroll.as_deref(),
        scroll_factor.as_deref(),
    )?;
    run_mouse_set_with(&plan, json, |plan| {
        core_input::apply_input_with(
            &archel_core::hyprland::hypr_dir(),
            plan,
            archel_core::packages::confirm_action,
            |lua| archel_core::hyprland::run_hyprctl_eval("hyprctl", lua),
            core_input::read_input,
        )
    })
}

/// Preview-then-delegate gate for mouse plans (mirrors the display
/// shape): the FR preview renders here, the single `[o/N]` lives in
/// the delegated apply. The injectable `apply` keeps the fail-fast
/// paths unit-coverable headless.
fn run_mouse_set_with(
    plan: &archel_core::hyprland::input::InputPlan,
    json: bool,
    apply: impl FnOnce(
        &archel_core::hyprland::input::InputPlan,
    ) -> Result<archel_core::hyprland::input::InputReport, ArchelError>,
) -> Result<(), ArchelError> {
    use archel_core::hyprland::input as core_input;
    if !json {
        print!(
            "{}",
            render_label_table(
                &core_input::mouse_preview_title(),
                &core_input::mouse_preview_rows(plan)
            )
        );
    }
    let report = apply(plan)?;
    if !report.read_back_ok {
        return Err(ArchelError::hyprland_unavailable(
            "live read-back disagrees with the applied mouse values",
        ));
    }
    if json {
        println!("{}", mouse_report_value(&report));
    } else {
        println!("{}", core_input::mouse_ok_line(&report));
    }
    Ok(())
}

/// Reads the app snippet body, defaulting to empty on first run
/// (never touches user files — only the app-owned snippet).
fn read_snippet_body() -> Result<String, ArchelError> {
    match std::fs::read_to_string(archel_core::hyprland::snippet_path_at(
        &archel_core::hyprland::hypr_dir(),
    )) {
        Ok(text) => Ok(text),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(err) => Err(ArchelError::Internal(format!(
            "cannot read snippet: {err}"
        ))),
    }
}

/// Reads the managed binds out of the app snippet (missing snippet
/// means no managed binds yet — never an error).
fn read_managed_binds() -> Result<Vec<core_binds::BindSpec>, ArchelError> {
    Ok(core_binds::parse_snippet_binds(&read_snippet_body()?))
}

/// Publishes a merged snippet body (backup plus atomic) and ensures
/// the guarded require line (D-08).
fn publish_snippet_body(body: &str) -> Result<(), ArchelError> {
    let base = archel_core::hyprland::hypr_dir();
    archel_core::hyprland::write_snippet_at(&base, body)?;
    archel_core::hyprland::ensure_require_line_at(&base)?;
    Ok(())
}

/// Builds the single-object keybind mutation report: the action
/// name, the bind summary, and the catalog ok line as the transcript.
/// Mirrors the audio mutation shape (one object, always parseable).
fn keybind_report_value(
    action: &str,
    spec: &core_binds::BindSpec,
    ok_line: &str,
    note: Option<&str>,
) -> serde_json::Value {
    serde_json::json!({
        "action": action,
        "bind": core_binds::summarize_bind(spec),
        "managed": true,
        "ok": true,
        "transcript": [ok_line],
        "note": note.map_or(serde_json::Value::Null, |text| {
            serde_json::Value::String(text.to_string())
        }),
    })
}

/// Builds the single-object mouse mutation report with per-domain EN
/// keys (audio `mutation_report_value` precedent, mouse-owned shape).
fn mouse_report_value(report: &archel_core::hyprland::input::InputReport) -> serde_json::Value {
    serde_json::json!({
        "action": "mouse-set",
        "sensitivity": report.sensitivity,
        "natural_scroll": report.natural_scroll,
        "scroll_factor": report.scroll_factor,
        "ok": true,
        "transcript": [archel_core::hyprland::input::mouse_ok_line(report)],
        "note": serde_json::Value::Null,
    })
}

/// Encodes one `--json` payload (single object per line, NDJSON-safe).
fn encode_json(payload: &impl serde::Serialize, what: &str) -> Result<String, ArchelError> {
    serde_json::to_string(payload)
        .map_err(|err| ArchelError::Internal(format!("cannot encode {what} JSON: {err}")))
}

/// Right-pads a label on char count (mirrors the `net_cmds` helper —
/// byte widths would misalign accented French labels).
fn pad_right(label: &str, width: usize) -> String {
    let mut out = label.to_string();
    while out.chars().count() < width {
        out.push(' ');
    }
    out
}

/// Generic French label table (title + padded `label : value` rows).
fn render_label_table(title: &str, rows: &[(String, String)]) -> String {
    let width = rows
        .iter()
        .map(|(label, _)| label.chars().count())
        .max()
        .unwrap_or(0);
    let mut out = format!("{title}\n");
    for (label, value) in rows {
        out.push_str(&format!("{} : {value}\n", pad_right(label, width)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use archel_core::hyprland::input as core_input;

    /// Trimmed live-shaped `hyprctl binds -j` capture (fixtures live
    /// in-module — the repo keeps no tests/ dir; this module never
    /// spawns).
    const BINDS_FIXTURE: &str = r#"[
        {"key": "V", "modmask": 65, "submap": "", "dispatcher": "__lua", "arg": "12"},
        {"key": "Return", "modmask": 64, "submap": "", "dispatcher": "__lua", "arg": "13"}
    ]"#;

    fn fixture_binds() -> Vec<core_binds::UserBind> {
        core_binds::parse_binds_json(BINDS_FIXTURE)
    }

    fn sample_spec() -> core_binds::BindSpec {
        core_binds::plan_bind("SUPER", "T", "exec", Some("kitty")).expect("spec plans")
    }

    fn render_list(
        user: &[core_binds::UserBind],
        managed: &[core_binds::BindSpec],
    ) -> String {
        let mut statuses: Vec<core_binds::BindStatus> =
            managed.iter().map(core_binds::BindStatus::from).collect();
        statuses.extend(user.iter().map(core_binds::BindStatus::from));
        render_label_table(&core_binds::keybinds_title(), &core_binds::keybind_rows(&statuses))
    }

    #[test]
    fn list_table_shows_user_binds_plus_managed_marks() {
        archel_core::i18n::set_language("fr");
        let table = render_list(&fixture_binds(), &[sample_spec()]);
        for needle in ["SUPER + T", "SUPER + SHIFT + V", "SUPER + Return", "__lua"] {
            assert!(table.contains(needle), "inventory must show `{needle}`");
        }
    }

    #[test]
    fn list_json_schema_pins_managed_flags() {
        let mut statuses: Vec<core_binds::BindStatus> =
            [sample_spec()].iter().map(core_binds::BindStatus::from).collect();
        statuses.extend(fixture_binds().iter().map(core_binds::BindStatus::from));
        let value: serde_json::Value =
            serde_json::to_value(&statuses).expect("BindStatus serializes");
        for field in ["key", "mods", "submap", "managed", "dispatcher"] {
            assert!(
                value[0].get(field).is_some(),
                "keybind status must carry `{field}`"
            );
        }
        assert_eq!(value[0]["managed"], true);
        assert_eq!(value[1]["managed"], false);
        insta::assert_json_snapshot!("input_keybinds", value);
    }

    #[test]
    fn add_report_json_schema_is_pinned() {
        archel_core::i18n::set_language("fr");
        let spec = sample_spec();
        let report = keybind_report_value(
            "keybind-add",
            &spec,
            &core_binds::bind_ok_line(&spec),
            None,
        );
        assert_eq!(report["action"], "keybind-add");
        assert_eq!(report["managed"], true);
        assert!(report["note"].is_null());
        insta::assert_json_snapshot!("input_keybind_add", report);
    }

    #[test]
    fn remove_report_pins_action_bind_and_ok() {
        archel_core::i18n::set_language("fr");
        let spec = sample_spec();
        let report = keybind_report_value(
            "keybind-remove",
            &spec,
            &core_binds::bind_removed_line(&spec),
            None,
        );
        assert_eq!(report["action"], "keybind-remove");
        assert!(report["note"].is_null(), "live unbind needs no reload note");
        insta::assert_json_snapshot!("input_keybind_remove", report);
    }

    #[test]
    fn add_typo_fails_before_any_prompt_or_spawn() {
        // Validate-first: planning runs before any prompt, so a bad
        // dispatcher fails with no TTY need and the apply leg
        // structurally never runs.
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let calls = Arc::new(AtomicUsize::new(0));
        let probe = Arc::clone(&calls);
        let spec = core_binds::plan_bind("SUPER", "T", "exec; evil", None);
        assert!(spec.is_err(), "hostile dispatcher fails at plan time");
        let spec = sample_spec();
        let err = (|| -> Result<(), ArchelError> {
            // A valid spec that never reaches apply: simulate the
            // headless refusal at the confirm gate.
            use std::io::IsTerminal;
            assert!(
                !std::io::stdin().is_terminal(),
                "abort-gate test requires headless stdin"
            );
            archel_core::packages::confirm_action("input abort probe [o/N]")?;
            probe.fetch_add(1, Ordering::SeqCst);
            let _ = spec;
            unreachable!("headless confirm refuses before any mutation");
        })()
        .expect_err("headless confirm must refuse");
        assert!(
            matches!(err, ArchelError::NeedsTty),
            "abort path needs the typed refusal, never a write: {err:?}"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0, "apply never runs headless");
    }

    #[test]
    fn add_headless_refusal_writes_nothing() {
        // The with-gate refuses at the confirm prompt before the
        // injectable apply runs (abort-spy shape).
        use std::io::IsTerminal;
        assert!(
            !std::io::stdin().is_terminal(),
            "abort-gate test requires headless stdin"
        );
        use std::cell::Cell;
        let applied = Cell::new(false);
        let err = run_keybind_add_with(&sample_spec(), None, false, |_| {
            applied.set(true);
            Ok(())
        })
        .expect_err("headless confirm must refuse");
        assert!(matches!(err, ArchelError::NeedsTty), "{err:?}");
        assert!(!applied.get(), "abort applies nothing");
    }

    #[test]
    fn edit_miss_fails_before_any_prompt_or_spawn() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let calls = Arc::new(AtomicUsize::new(0));
        let probe = Arc::clone(&calls);
        let ghost = core_binds::plan_bind("ALT", "F4", "killactive", None).expect("plans");
        let new = core_binds::plan_bind("ALT", "F4", "exec", Some("x")).expect("plans");
        let err = run_keybind_edit_with(&ghost, &new, "-- empty body\n", false, move |_| {
            probe.fetch_add(1, Ordering::SeqCst);
            panic!("a miss must never reach eval");
        })
        .expect_err("miss fails fast");
        assert!(
            matches!(err, ArchelError::DesktopInvalid { .. }),
            "miss is typed DesktopInvalid: {err:?}"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0, "eval never runs on a miss");
    }

    #[test]
    fn remove_miss_fails_before_any_prompt_or_spawn() {
        let ghost = core_binds::plan_bind("ALT", "F4", "killactive", None).expect("plans");
        let err = run_keybind_remove_with(&ghost, "-- empty body\n", false, |_| {
            panic!("a miss must never reach eval")
        })
        .expect_err("miss fails fast");
        assert!(matches!(err, ArchelError::DesktopInvalid { .. }), "{err:?}");
    }

    #[test]
    fn edit_validation_rejects_empty_new_flags() {
        let err = run_keybind_edit(
            "SUPER".to_string(),
            "T".to_string(),
            "exec".to_string(),
            None,
            None,
            None,
            None,
            false,
        );
        // Fails before any session read: either the empty-edit guard
        // or the session gate — both typed, never a prompt.
        assert!(err.is_err(), "flagless edit must fail");
    }

    #[test]
    fn mouse_preview_names_old_arrow_new() {
        archel_core::i18n::set_language("fr");
        let base = std::env::temp_dir().join(format!("archel-input-cli-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).expect("scratch base must create");
        let plan = core_input::plan_input_set_at(&base, Some("0.75"), Some("on"), None)
            .expect("plan builds");
        let preview = render_label_table(
            &core_input::mouse_preview_title(),
            &core_input::mouse_preview_rows(&plan),
        );
        assert!(preview.contains("0 -> 0.75"), "preview shows old-arrow-new");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn mouse_set_failure_propagates_typed_without_silent_ok() {
        // A valid plan whose delegated apply refuses typed (headless
        // NeedsTty in production) propagates the error — the with-gate
        // never prints a report on failure.
        let base = std::env::temp_dir().join(format!("archel-input-typo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).expect("scratch base must create");
        let err = core_input::plan_input_set_at(&base, Some("abc"), None, None)
            .expect_err("typo fails fast at the planner");
        assert!(matches!(err, ArchelError::DesktopInvalid { .. }), "{err:?}");
        let plan = core_input::plan_input_set_at(&base, Some("0.75"), None, None)
            .expect("valid plan builds");
        let err = run_mouse_set_with(&plan, false, |_| Err(ArchelError::NeedsTty))
            .expect_err("refused apply propagates");
        assert!(matches!(err, ArchelError::NeedsTty), "{err:?}");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn mouse_json_schemas_pin_state_and_report() {
        let state = core_input::InputState {
            sensitivity: 0.5,
            natural_scroll: true,
            scroll_factor: 1.0,
        };
        let value: serde_json::Value =
            serde_json::to_value(&state).expect("InputState serializes");
        for field in ["sensitivity", "natural_scroll", "scroll_factor"] {
            assert!(value.get(field).is_some(), "mouse state must carry `{field}`");
        }
        insta::assert_json_snapshot!("input_mouse", value);
        archel_core::i18n::set_language("fr");
        let report = core_input::InputReport {
            sensitivity: 0.75,
            natural_scroll: true,
            scroll_factor: 1.5,
            read_back_ok: true,
        };
        insta::assert_json_snapshot!("input_mouse_set", mouse_report_value(&report));
    }

    #[test]
    fn set_path_touches_nothing_outside_app_owned_files() {
        // D-08 write-scope spy for the input persist path: a user tree
        // (entry plus a fake user keybinds module) plus a sentinel
        // outside the base; after a snippet merge, only app-owned
        // paths may differ.
        let base = std::env::temp_dir().join(format!(
            "archel-input-scope-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("modules")).expect("modules creates");
        let outside = base.with_extension("outside-sentinel");
        std::fs::write(&outside, "sentinel").expect("sentinel seeds");
        let user_module = base.join("modules").join("keybinds.lua");
        std::fs::write(&user_module, "-- user binds\nbind = SUPER, T, exec, kitty\n")
            .expect("user module seeds");
        std::fs::write(base.join("hyprland.lua"), "-- entry\n").expect("entry seeds");
        let user_before = std::fs::read(&user_module).expect("user module reads");
        let spec = sample_spec();
        let body = String::new();
        let merged = core_binds::add_bind_line(&body, &spec).expect("merges");
        archel_core::hyprland::write_snippet_at(&base, &merged).expect("snippet writes");
        archel_core::hyprland::ensure_require_line_at(&base).expect("require ensures");
        assert_eq!(
            std::fs::read(&user_module).expect("user module re-reads"),
            user_before,
            "fake user keybinds are byte-for-byte unchanged"
        );
        assert_eq!(
            std::fs::read_to_string(&outside).expect("sentinel reads"),
            "sentinel",
            "nothing outside the hypr dir may be touched"
        );
        let _ = std::fs::remove_dir_all(&base);
        let _ = std::fs::remove_file(&outside);
    }
}
