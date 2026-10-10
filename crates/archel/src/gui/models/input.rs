//! Input view-model: live mouse leaves plus the merged keybind
//! inventory (read-only; the Ajouter editor lands in plan 08).
//!
//! Same proven shape as `models::net`: `tick()` is a QML-called
//! invokable that only spawns - the sync core reads (`read_input`,
//! `binds -j` plus the snippet binds) run on the gui runtime's
//! blocking pool and the finished payload returns exclusively through
//! `CxxQtThread.queue`, whose closure is the only code allowed to set
//! properties (RESEARCH Pattern 1). Keybind rows render through the
//! core's `keybind_rows` (same rows `run_keybinds_list` prints), and
//! managed binds come from `parse_snippet_binds` over the app snippet
//! - never a GUI-side reformat (PLAT-03).

use cxx_qt_lib::QString;

use super::super::poll::{BannerPolicy, PollGuard};
use crate::gui::models::system::NOT_AVAILABLE;

/// Fully formatted tick payload, built off the Qt thread so the
/// `queue()` closure only sets properties.
pub(crate) struct AppliedInput {
    pub sensitivity_text: String,
    pub natural_text: String,
    pub scroll_text: String,
    pub binds_json: String,
    pub banner_visible: bool,
}

/// Backing state: property fields plus the poll guard and banner
/// policy. No shared core handle: every tick is an independent read.
/// `operation_busy`/`operation_kind` mirror the 06-06 pkg write-slot
/// shape (D-15); `keybind_error` carries inline French validation
/// for the keybind editor (never a modal for typos).
pub struct InputModelRust {
    pub(crate) sensitivity_text: QString,
    pub(crate) natural_text: QString,
    pub(crate) scroll_text: QString,
    pub(crate) binds_json: QString,
    pub(crate) banner_visible: bool,
    pub(crate) operation_busy: bool,
    pub(crate) operation_kind: QString,
    pub(crate) keybind_error: QString,
    guard: PollGuard,
    banner: BannerPolicy,
}

impl Default for InputModelRust {
    fn default() -> Self {
        Self {
            sensitivity_text: QString::from(""),
            natural_text: QString::from(""),
            scroll_text: QString::from(""),
            binds_json: QString::from(""),
            banner_visible: false,
            operation_busy: false,
            operation_kind: QString::from(""),
            keybind_error: QString::from(""),
            guard: PollGuard::new(),
            banner: BannerPolicy::new(),
        }
    }
}

/// Keybind rows as QML-ready JSON: `[{combo,detail}]` rendered through
/// the core's `keybind_rows` (the same `(combo, marker)` pairs the CLI
/// table prints - managed binds carry the managed marker, user binds
/// the opaque dispatcher text). No client-side reformatting.
/// Managed rows additionally carry their exact-match address
/// (`mods`/`key`/`dispatcher`/`arg`) so the per-row trash button can
/// address `remove_keybind` without re-deriving anything (T-06-35:
/// user binds never carry an address and show no trash button).
pub(crate) fn binds_json_for(
    statuses: &[archel_core::hyprland::binds::BindStatus],
    managed: &[archel_core::hyprland::binds::BindSpec],
) -> String {
    use archel_core::hyprland::binds as core_binds;
    let pairs = core_binds::keybind_rows(statuses);
    let rows: Vec<serde_json::Value> = pairs
        .into_iter()
        .zip(statuses.iter())
        .map(|((combo, detail), status)| {
            let mut row =
                serde_json::json!({"combo": combo, "detail": detail, "managed": status.managed});
            if status.managed {
                let mods = status.mods.join(" + ");
                row["mods"] = serde_json::Value::String(mods.clone());
                row["key"] = serde_json::Value::String(status.key.clone());
                row["dispatcher"] = serde_json::Value::String(status.dispatcher.clone());
                let arg = managed
                    .iter()
                    .find(|spec| {
                        spec.mods == mods
                            && spec.key == status.key
                            && spec.dispatcher.name() == status.dispatcher
                    })
                    .and_then(|spec| spec.arg.clone())
                    .unwrap_or_default();
                row["arg"] = serde_json::Value::String(arg);
            }
            row
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string())
}

/// Merges managed snippet binds with live compositor binds into one
/// `BindStatus` inventory, mirroring `run_keybinds_list` (managed
/// first, then user). A missing snippet means no managed binds yet -
/// never an error.
pub(crate) fn merge_binds(
    user: &[archel_core::hyprland::binds::UserBind],
    managed: &[archel_core::hyprland::binds::BindSpec],
) -> Vec<archel_core::hyprland::binds::BindStatus> {
    let mut statuses: Vec<archel_core::hyprland::binds::BindStatus> =
        managed.iter().map(archel_core::hyprland::binds::BindStatus::from).collect();
    statuses.extend(
        user.iter().map(archel_core::hyprland::binds::BindStatus::from),
    );
    statuses
}

impl InputModelRust {
    /// Applies a finished worker outcome on the Qt thread. A failed
    /// `read_input` fails the whole tick (N/A everywhere plus the
    /// once-per-episode banner); a failed binds read degrades to the
    /// honest empty list (the empty-state copy covers it).
    fn apply_outcome(
        &mut self,
        input: Result<archel_core::hyprland::input::InputState, archel_core::ArchelError>,
        binds: Vec<archel_core::hyprland::binds::BindStatus>,
        managed: &[archel_core::hyprland::binds::BindSpec],
        binds_ok: bool,
    ) -> AppliedInput {
        match input {
            Ok(state) => {
                self.banner.on_success();
                let _ = binds_ok;
                AppliedInput {
                    sensitivity_text: format!("{:.2}", state.sensitivity),
                    natural_text: if state.natural_scroll {
                        "on".to_string()
                    } else {
                        "off".to_string()
                    },
                    scroll_text: format!("{:.2}", state.scroll_factor),
                    binds_json: binds_json_for(&binds, managed),
                    banner_visible: false,
                }
            }
            Err(_) => {
                self.banner.on_failure();
                AppliedInput {
                    sensitivity_text: NOT_AVAILABLE.to_string(),
                    natural_text: NOT_AVAILABLE.to_string(),
                    scroll_text: NOT_AVAILABLE.to_string(),
                    binds_json: "[]".to_string(),
                    banner_visible: self.banner.wants_visible(),
                }
            }
        }
    }
}

impl crate::gui::bridge_input::qobject::InputModel {
    /// 1 Hz entry point: skips while a tick is in flight (D-02),
    /// spawns the blocking input + binds reads, marshals back through
    /// `queue()` only.
    pub fn tick(mut self: core::pin::Pin<&mut Self>) {
        use cxx_qt::{CxxQtType, Threading};
        let tag = match self.as_mut().rust_mut().guard.try_begin() {
            Some(tag) => tag,
            None => return,
        };
        let qt_thread = self.qt_thread();
        let runtime = super::super::RUNTIME
            .get()
            .expect("gui runtime is set before Qt starts");
        runtime.spawn_blocking(move || {
            use archel_core::hyprland::binds as core_binds;
            let input = archel_core::hyprland::input::read_input();
            let (binds, managed, binds_ok) = match archel_core::hyprland::run_hyprctl(
                "hyprctl",
                &["binds", "-j"],
            ) {
                Ok(raw) => {
                    let user = core_binds::parse_binds_json(&raw);
                    let base = archel_core::hyprland::hypr_dir();
                    let body = std::fs::read_to_string(
                        archel_core::hyprland::snippet_path_at(&base),
                    )
                    .unwrap_or_default();
                    let managed = core_binds::parse_snippet_binds(&body);
                    (super::input::merge_binds(&user, &managed), managed, true)
                }
                Err(_) => (Vec::new(), Vec::new(), false),
            };
            let _ = qt_thread.queue(
                move |mut obj: core::pin::Pin<&mut Self>| {
                    let applied = {
                        let mut rust = obj.as_mut().rust_mut();
                        if !rust.guard.is_newest(tag) {
                            rust.guard.finish();
                            return;
                        }
                        rust.guard.finish();
                        rust.apply_outcome(input, binds, &managed, binds_ok)
                    };
                    obj.as_mut()
                        .set_sensitivity_text(QString::from(&applied.sensitivity_text));
                    obj.as_mut()
                        .set_natural_text(QString::from(&applied.natural_text));
                    obj.as_mut()
                        .set_scroll_text(QString::from(&applied.scroll_text));
                    obj.as_mut()
                        .set_binds_json(QString::from(&applied.binds_json));
                    obj.as_mut().set_banner_visible(applied.banner_visible);
                },
            );
        });
    }

    /// Hides the D-05 banner after the user dismisses it.
    pub fn dismiss_banner(mut self: core::pin::Pin<&mut Self>) {
        use cxx_qt::CxxQtType;
        self.as_mut().rust_mut().banner.dismiss();
        self.as_mut().set_banner_visible(false);
    }

    /// Applies a sensitivity value from raw text (the worker
    /// validates; typos surface in the ErrorModal, never a spawn).
    pub fn set_sensitivity(mut self: core::pin::Pin<&mut Self>, value: QString) {
        let raw = value.to_string();
        if !self.as_mut().begin_write_operation("sensitivity") {
            return;
        }
        let qt_thread = {
            use cxx_qt::Threading;
            self.qt_thread()
        };
        let runtime = super::super::RUNTIME
            .get()
            .expect("gui runtime is set before Qt starts");
        runtime.spawn_blocking(move || {
            let base = archel_core::hyprland::hypr_dir();
            sensitivity_worker(&base, &raw, &PROD_INPUT_OPS, super::super::modal::session());
            let _ = qt_thread.queue(|mut obj: core::pin::Pin<&mut Self>| {
                obj.as_mut().finish_write_operation();
            });
        });
    }

    /// Applies the pointer acceleration (scroll-factor) leaf from raw
    /// text (same worker shape as sensitivity).
    pub fn set_accelerate(mut self: core::pin::Pin<&mut Self>, value: QString) {
        let raw = value.to_string();
        if !self.as_mut().begin_write_operation("accelerate") {
            return;
        }
        let qt_thread = {
            use cxx_qt::Threading;
            self.qt_thread()
        };
        let runtime = super::super::RUNTIME
            .get()
            .expect("gui runtime is set before Qt starts");
        runtime.spawn_blocking(move || {
            let base = archel_core::hyprland::hypr_dir();
            accelerate_worker(&base, &raw, &PROD_INPUT_OPS, super::super::modal::session());
            let _ = qt_thread.queue(|mut obj: core::pin::Pin<&mut Self>| {
                obj.as_mut().finish_write_operation();
            });
        });
    }

    /// Adds one keybind from the editor fields. Validation runs
    /// synchronously here (pure core fns, no I/O): typos render
    /// inline in `keybind_error` and never reach a modal or a
    /// worker. Only a valid spec spawns the confirm-gated worker.
    pub fn add_keybind(mut self: core::pin::Pin<&mut Self>, combo: QString, action: QString) {
        let combo_text = combo.to_string();
        let action_text = action.to_string();
        match plan_keybind_from_fields(&combo_text, &action_text) {
            Ok(_) => {}
            Err(err) => {
                self.as_mut()
                    .set_keybind_error(QString::from(&err.message()));
                return;
            }
        }
        self.as_mut().set_keybind_error(QString::from(""));
        if !self.as_mut().begin_write_operation("keybind-add") {
            return;
        }
        let qt_thread = {
            use cxx_qt::Threading;
            self.qt_thread()
        };
        let runtime = super::super::RUNTIME
            .get()
            .expect("gui runtime is set before Qt starts");
        runtime.spawn_blocking(move || {
            let base = archel_core::hyprland::hypr_dir();
            add_keybind_worker(
                &base,
                &combo_text,
                &action_text,
                &PROD_INPUT_OPS,
                super::super::modal::session(),
            );
            let _ = qt_thread.queue(|mut obj: core::pin::Pin<&mut Self>| {
                obj.as_mut().finish_write_operation();
            });
        });
    }

    /// Removes the managed bind at the row's exact address (the row
    /// carries its own address fields; user binds show no trash
    /// button). A miss fails typed before any confirm.
    pub fn remove_keybind(
        mut self: core::pin::Pin<&mut Self>,
        mods: QString,
        key: QString,
        dispatcher: QString,
        arg: QString,
    ) {
        let mods_text = mods.to_string();
        let key_text = key.to_string();
        let dispatcher_text = dispatcher.to_string();
        let arg_text = arg.to_string();
        let arg_opt = if arg_text.trim().is_empty() {
            None
        } else {
            Some(arg_text.as_str())
        };
        if archel_core::hyprland::binds::plan_bind(
            &mods_text,
            &key_text,
            &dispatcher_text,
            arg_opt,
        )
        .is_err()
        {
            self.as_mut().set_keybind_error(QString::from(
                &archel_core::ArchelError::desktop_invalid(format!(
                    "rejected bind address {mods_text:?} + {key_text:?}"
                ))
                .message(),
            ));
            return;
        }
        self.as_mut().set_keybind_error(QString::from(""));
        if !self.as_mut().begin_write_operation("keybind-remove") {
            return;
        }
        let qt_thread = {
            use cxx_qt::Threading;
            self.qt_thread()
        };
        let runtime = super::super::RUNTIME
            .get()
            .expect("gui runtime is set before Qt starts");
        runtime.spawn_blocking(move || {
            let base = archel_core::hyprland::hypr_dir();
            let arg_owned = if arg_text.trim().is_empty() {
                None
            } else {
                Some(arg_text.clone())
            };
            remove_keybind_worker(
                &base,
                &mods_text,
                &key_text,
                &dispatcher_text,
                arg_owned.as_deref(),
                &PROD_INPUT_OPS,
                super::super::modal::session(),
            );
            let _ = qt_thread.queue(|mut obj: core::pin::Pin<&mut Self>| {
                obj.as_mut().finish_write_operation();
            });
        });
    }

    /// Claims the single-operation slot (D-15).
    fn begin_write_operation(mut self: core::pin::Pin<&mut Self>, kind: &str) -> bool {
        use cxx_qt::CxxQtType;
        if self.as_mut().rust_mut().operation_busy {
            return false;
        }
        self.as_mut().rust_mut().operation_busy = true;
        self.as_mut().set_operation_kind(QString::from(kind));
        self.as_mut().set_operation_busy(true);
        true
    }

    /// Releases the operation slot and refreshes the tab (Qt thread
    /// only, RESEARCH Pattern 1).
    fn finish_write_operation(mut self: core::pin::Pin<&mut Self>) {
        use cxx_qt::CxxQtType;
        self.as_mut().rust_mut().operation_busy = false;
        self.as_mut().set_operation_busy(false);
        self.as_mut().set_operation_kind(QString::from(""));
        self.as_mut().tick();
    }
}

// -- Write path (plan 06-08, Task 2) -----------------------------------
//
// Pointer workers go through `apply_input_with` (validate ->
// core-verbatim question -> live IPC -> line-preserving conf write);
// keybind workers through `plan_bind` (+ `collision_hint`) ->
// confirm -> live eval -> the core line editor. The GUI never edits
// conf text itself and never spawns (thin-view contract; T-06-35).

use std::path::Path;

use crate::gui::modal::ModalSession;

/// Injection seam for the input writes. Production passes
/// [`PROD_INPUT_OPS`]; unit tests pass fakes so no hyprctl child or
/// conf write ever happens headless.
pub(crate) struct InputOps {
    pub apply_input: fn(
        &Path,
        &archel_core::hyprland::input::InputPlan,
    ) -> Result<archel_core::hyprland::input::InputReport, archel_core::ArchelError>,
    pub read_live_binds: fn() -> Vec<archel_core::hyprland::binds::UserBind>,
    pub eval_bind_line: fn(&str) -> Result<(), archel_core::ArchelError>,
    pub eval_unbind:
        fn(&archel_core::hyprland::binds::BindSpec) -> Result<(), archel_core::ArchelError>,
    pub read_body: fn(&Path) -> Result<String, archel_core::ArchelError>,
    pub write_body: fn(&Path, &str) -> Result<(), archel_core::ArchelError>,
}

fn prod_apply_input(
    base: &Path,
    plan: &archel_core::hyprland::input::InputPlan,
) -> Result<archel_core::hyprland::input::InputReport, archel_core::ArchelError> {
    use archel_core::hyprland::{self, input as core_input};
    // Already confirmed through the modal: the core confirm leg is a
    // no-op here so no second TTY prompt can appear.
    core_input::apply_input_with(
        base,
        plan,
        |_| Ok(()),
        |lua| hyprland::run_hyprctl_eval("hyprctl", lua),
        core_input::read_input,
    )
}

fn prod_read_live_binds() -> Vec<archel_core::hyprland::binds::UserBind> {
    use archel_core::hyprland::{self, binds as core_binds};
    hyprland::run_hyprctl("hyprctl", &["binds", "-j"])
        .map(|raw| core_binds::parse_binds_json(&raw))
        .unwrap_or_default()
}

fn prod_eval_bind_line(line: &str) -> Result<(), archel_core::ArchelError> {
    archel_core::hyprland::run_hyprctl_eval("hyprctl", line)
}

fn prod_eval_unbind(
    spec: &archel_core::hyprland::binds::BindSpec,
) -> Result<(), archel_core::ArchelError> {
    use archel_core::hyprland::{self, binds as core_binds};
    let stmt = core_binds::unbind_eval_statement(spec)?;
    hyprland::run_hyprctl_eval("hyprctl", &stmt)
}

fn prod_read_body(base: &Path) -> Result<String, archel_core::ArchelError> {
    match std::fs::read_to_string(archel_core::hyprland::snippet_path_at(base)) {
        Ok(text) => Ok(text),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(err) => Err(archel_core::ArchelError::Internal(format!(
            "cannot read snippet: {err}"
        ))),
    }
}

fn prod_write_body(base: &Path, body: &str) -> Result<(), archel_core::ArchelError> {
    archel_core::hyprland::write_snippet_at(base, body)?;
    archel_core::hyprland::ensure_require_line_at(base)?;
    Ok(())
}

/// The production wiring (thin-view contract; PLAT-03).
pub(crate) const PROD_INPUT_OPS: InputOps = InputOps {
    apply_input: prod_apply_input,
    read_live_binds: prod_read_live_binds,
    eval_bind_line: prod_eval_bind_line,
    eval_unbind: prod_eval_unbind,
    read_body: prod_read_body,
    write_body: prod_write_body,
};

/// Blocks the worker on the modal's one-shot answer (default No).
fn await_confirm(sess: &ModalSession, question: &str) -> bool {
    let rx = sess.request_confirm(question);
    rx.recv().unwrap_or(false)
}

/// Sensitivity worker: validate-plus-plan first (WR-02), then the
/// core question verbatim, then the injected apply.
pub(crate) fn sensitivity_worker(
    base: &Path,
    raw: &str,
    ops: &InputOps,
    sess: &ModalSession,
) {
    use archel_core::hyprland::input as core_input;
    let plan = match core_input::plan_input_set_at(base, Some(raw), None, None) {
        Ok(plan) => plan,
        Err(err) => {
            sess.open_error(&err.message());
            return;
        }
    };
    if !await_confirm(sess, &core_input::confirm_input_prompt(&plan)) {
        return;
    }
    if let Err(err) = (ops.apply_input)(base, &plan) {
        sess.open_error(&err.message());
    }
}

/// Acceleration (scroll-factor) worker: same shape, the third mouse
/// leaf instead of the first.
pub(crate) fn accelerate_worker(
    base: &Path,
    raw: &str,
    ops: &InputOps,
    sess: &ModalSession,
) {
    use archel_core::hyprland::input as core_input;
    let plan = match core_input::plan_input_set_at(base, None, None, Some(raw)) {
        Ok(plan) => plan,
        Err(err) => {
            sess.open_error(&err.message());
            return;
        }
    };
    if !await_confirm(sess, &core_input::confirm_input_prompt(&plan)) {
        return;
    }
    if let Err(err) = (ops.apply_input)(base, &plan) {
        sess.open_error(&err.message());
    }
}

/// Splits the editor combo (`SUPER + T`, `ALT + F4`, bare `F1`) into
/// canonical (mods, key). Pure for headless coverage; typos fail
/// typed before any prompt, spawn, or write.
pub(crate) fn split_combo(raw: &str) -> Result<(String, String), archel_core::ArchelError> {
    use archel_core::hyprland::binds as core_binds;
    let parts: Vec<&str> = raw
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.is_empty() {
        return Err(archel_core::ArchelError::desktop_invalid(
            "empty bind combination",
        ));
    }
    if parts.len() == 1 && raw.contains('+') {
        return Err(archel_core::ArchelError::desktop_invalid(format!(
            "rejected bind combination {raw:?} (modifier without a key)"
        )));
    }
    let key = parts[parts.len() - 1].to_string();
    let mods = if parts.len() == 1 {
        String::new()
    } else {
        let mask = core_binds::parse_mods(&parts[..parts.len() - 1].join(" + "))?;
        core_binds::render_mods(mask)
    };
    core_binds::validate_key(&key)?;
    Ok((mods, key))
}

/// Splits the editor action (`exec kitty`, `killactive`,
/// `workspace 3`) into (dispatcher, arg). The arg stays opaque
/// (lua-quoted at render, T-05-11).
pub(crate) fn split_action(raw: &str) -> Result<(String, Option<String>), archel_core::ArchelError> {
    let mut words = raw.split_whitespace();
    match words.next() {
        None => Err(archel_core::ArchelError::desktop_invalid(
            "empty bind action",
        )),
        Some(dispatcher) => {
            let rest: Vec<&str> = words.collect();
            let arg = if rest.is_empty() {
                None
            } else {
                Some(rest.join(" "))
            };
            Ok((dispatcher.to_string(), arg))
        }
    }
}

/// Plans one bind from the two editor fields (validate FIRST,
/// WR-02). Shared by the invokable pre-check and the worker so both
/// agree on what "valid" means.
pub(crate) fn plan_keybind_from_fields(
    combo: &str,
    action: &str,
) -> Result<archel_core::hyprland::binds::BindSpec, archel_core::ArchelError> {
    use archel_core::hyprland::binds as core_binds;
    let (mods, key) = split_combo(combo)?;
    let (dispatcher, arg) = split_action(action)?;
    core_binds::plan_bind(&mods, &key, &dispatcher, arg.as_deref())
}

/// Keybind-add worker (INP-02): validate, read the live vocabulary,
/// surface `collision_hint` ahead of the confirm question, then live
/// eval plus the snippet merge. A decline writes nothing.
pub(crate) fn add_keybind_worker(
    base: &Path,
    combo: &str,
    action: &str,
    ops: &InputOps,
    sess: &ModalSession,
) {
    use archel_core::hyprland::binds as core_binds;
    let spec = match plan_keybind_from_fields(combo, action) {
        Ok(spec) => spec,
        Err(err) => {
            sess.open_error(&err.message());
            return;
        }
    };
    let live = (ops.read_live_binds)();
    let mut question = String::new();
    if let Some(hint) = core_binds::collision_hint(&live, &spec) {
        question.push_str(&hint);
        question.push_str("\n\n");
    }
    question.push_str(&core_binds::confirm_bind_prompt(&spec));
    if !await_confirm(sess, &question) {
        return;
    }
    let line = match core_binds::render_bind_line(&spec) {
        Ok(line) => line,
        Err(err) => {
            sess.open_error(&err.message());
            return;
        }
    };
    if let Err(err) = (ops.eval_bind_line)(&line) {
        sess.open_error(&err.message());
        return;
    }
    let body = match (ops.read_body)(base) {
        Ok(body) => body,
        Err(err) => {
            sess.open_error(&err.message());
            return;
        }
    };
    let merged = match core_binds::add_bind_line(&body, &spec) {
        Ok(merged) => merged,
        Err(err) => {
            sess.open_error(&err.message());
            return;
        }
    };
    if let Err(err) = (ops.write_body)(base, &merged) {
        sess.open_error(&err.message());
    }
}

/// Keybind-remove worker (INP-02): exact-match miss check first
/// (typed, before any confirm), then the destructive-toned confirm
/// question, then a post-confirm re-read (external edits during the
/// confirm window must not be clobbered), then live unbind plus the
/// snippet removal.
pub(crate) fn remove_keybind_worker(
    base: &Path,
    mods: &str,
    key: &str,
    dispatcher: &str,
    arg: Option<&str>,
    ops: &InputOps,
    sess: &ModalSession,
) {
    use archel_core::hyprland::binds as core_binds;
    let target = match core_binds::plan_bind(mods, key, dispatcher, arg) {
        Ok(spec) => spec,
        Err(err) => {
            sess.open_error(&err.message());
            return;
        }
    };
    // Pre-confirm miss check (typed, before any confirm): a nonexistent
    // bind never reaches the destructive prompt.
    let pre = match (ops.read_body)(base) {
        Ok(body) => body,
        Err(err) => {
            sess.open_error(&err.message());
            return;
        }
    };
    if let Err(err) = core_binds::remove_bind_line(&pre, &target) {
        sess.open_error(&err.message());
        return;
    }
    if !await_confirm(sess, &core_binds::confirm_bind_remove_prompt(&target)) {
        return;
    }
    // Re-read AFTER the (unbounded) confirm window: external
    // hyprland.conf edits made while the user thinks must not be
    // clobbered by a stale pre-confirm snapshot (CR-02). A target that
    // vanished meanwhile re-fails typed instead of writing.
    let body = match (ops.read_body)(base) {
        Ok(body) => body,
        Err(err) => {
            sess.open_error(&err.message());
            return;
        }
    };
    let merged = match core_binds::remove_bind_line(&body, &target) {
        Ok(merged) => merged,
        Err(err) => {
            sess.open_error(&err.message());
            return;
        }
    };
    if let Err(err) = (ops.eval_unbind)(&target) {
        sess.open_error(&err.message());
        return;
    }
    if let Err(err) = (ops.write_body)(base, &merged) {
        sess.open_error(&err.message());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_input() -> archel_core::hyprland::input::InputState {
        archel_core::hyprland::input::InputState {
            sensitivity: 0.5,
            natural_scroll: true,
            scroll_factor: 1.5,
        }
    }

    fn failed_input(
    ) -> Result<archel_core::hyprland::input::InputState, archel_core::ArchelError> {
        Err(archel_core::ArchelError::hyprland_unavailable(
            "no compositor in test",
        ))
    }

    #[test]
    fn error_path_reads_na_and_signals_banner_once() {
        let mut model = InputModelRust::default();
        let first = model.apply_outcome(failed_input(), Vec::new(), &[], false);
        for text in [
            &first.sensitivity_text,
            &first.natural_text,
            &first.scroll_text,
        ] {
            assert_eq!(text, NOT_AVAILABLE, "failed tick must read N/A");
        }
        assert_eq!(first.binds_json, "[]");
        assert!(first.banner_visible, "first failure shows the banner");
        let second = model.apply_outcome(failed_input(), Vec::new(), &[], false);
        assert!(
            second.banner_visible,
            "consecutive failures keep the banner without re-signalling"
        );
    }

    #[test]
    fn binds_json_carries_core_rendered_rows_verbatim() {
        use archel_core::hyprland::binds as core_binds;
        let user = core_binds::parse_binds_json(
            r#"[{"key":"T","modmask":64,"submap":"","dispatcher":"__lua"}]"#,
        );
        assert_eq!(user.len(), 1, "fixture must parse one user bind");
        let statuses = merge_binds(&user, &[]);
        let json = binds_json_for(&statuses, &[]);
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(&json).expect("binds must be JSON");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["combo"], "SUPER + T");
        assert_eq!(
            rows[0]["detail"], "__lua",
            "user binds carry the opaque dispatcher text"
        );
    }

    #[test]
    fn success_fills_mouse_fields_and_empty_binds_degrade() {
        let mut model = InputModelRust::default();
        let _ = model.apply_outcome(failed_input(), Vec::new(), &[], false);
        let applied = model.apply_outcome(Ok(ready_input()), Vec::new(), &[], false);
        assert!(!applied.banner_visible, "success hides the banner");
        assert_eq!(applied.sensitivity_text, "0.50");
        assert_eq!(applied.natural_text, "on");
        assert_eq!(applied.scroll_text, "1.50");
        assert_eq!(
            applied.binds_json, "[]",
            "no binds is the honest empty state, not a banner trip"
        );
    }

    #[test]
    fn managed_rows_carry_their_removal_address() {
        use archel_core::hyprland::binds as core_binds;
        let spec =
            core_binds::plan_bind("SUPER", "T", "exec", Some("kitty")).expect("spec plans");
        let statuses = merge_binds(&[], std::slice::from_ref(&spec));
        let json = binds_json_for(&statuses, std::slice::from_ref(&spec));
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(&json).expect("binds must be JSON");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["managed"], true);
        assert_eq!(rows[0]["mods"], "SUPER");
        assert_eq!(rows[0]["key"], "T");
        assert_eq!(rows[0]["dispatcher"], "exec");
        assert_eq!(rows[0]["arg"], "kitty");
    }

    // -- Write-path choreography (plan 06-08, Task 2) --
    //
    // The fakes below replace only the six `InputOps` seams; the
    // workers run for real against a local ModalSession, so
    // validate-before-confirm, the collision warning, the
    // always-confirmed removal and the miss-before-confirm order are
    // all proven without spawning hyprctl or touching conf.

    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use crate::gui::modal::{ModalKind, ModalSession};

    /// Polls `cond` until it holds or 5 s elapse (worker rendezvous).
    fn wait_until(what: &str, cond: impl Fn() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if cond() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("condition never held within 5s: {what}");
    }

    fn test_base(tag: &str) -> std::path::PathBuf {
        // Nanos stamp keeps parallel test runs apart without any
        // process-handle API (gui/ stays subprocess-free, 06-09 audit).
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let base = std::env::temp_dir().join(format!("archel-input-gui-{tag}-{unique}"));
        std::fs::create_dir_all(&base).expect("test base");
        base
    }

    fn ok_input_report() -> archel_core::hyprland::input::InputReport {
        archel_core::hyprland::input::InputReport {
            sensitivity: 0.5,
            natural_scroll: true,
            scroll_factor: 1.5,
            read_back_ok: true,
        }
    }

    fn succeeding_input_ops() -> InputOps {
        InputOps {
            apply_input: |_, _| Ok(ok_input_report()),
            read_live_binds: Vec::new,
            eval_bind_line: |_| Ok(()),
            eval_unbind: |_| Ok(()),
            read_body: |_| Ok(String::new()),
            write_body: |_, _| Ok(()),
        }
    }

    #[test]
    fn combo_and_action_splitters_reject_typos_typed() {
        assert!(split_combo("").is_err(), "empty combo fails");
        assert!(split_combo("SUPER + ").is_err(), "dangling mods fail");
        assert!(split_combo("NOPE + T").is_err(), "unknown word fails");
        assert!(split_action("").is_err(), "empty action fails");
        let (mods, key) = split_combo("SUPER + CTRL + T").expect("combo splits");
        assert_eq!(mods, "SUPER + CTRL");
        assert_eq!(key, "T");
        let (dispatcher, arg) = split_action("exec kitty --hold").expect("action splits");
        assert_eq!(dispatcher, "exec");
        assert_eq!(arg.as_deref(), Some("kitty --hold"));
        assert!(
            !plan_keybind_from_fields("SUPER + T", "exec kitty")
                .expect("valid fields plan")
                .mods
                .is_empty()
        );
        assert!(
            plan_keybind_from_fields("T", "exec").is_err(),
            "exec without a command fails before any prompt"
        );
    }

    #[test]
    fn sensitivity_decline_asks_the_core_question_and_applies_nothing() {
        use archel_core::hyprland::input as core_input;
        static APPLY_CALLS: AtomicUsize = AtomicUsize::new(0);
        let ops = InputOps {
            apply_input: |_, _| {
                APPLY_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
                Ok(ok_input_report())
            },
            ..succeeding_input_ops()
        };
        let sess = ModalSession::default();
        let base = test_base("sensitivity-decline");
        std::thread::scope(|s| {
            s.spawn(|| sensitivity_worker(&base, "0.75", &ops, &sess));
            wait_until("sensitivity confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            let plan = core_input::plan_input_set_at(&base, Some("0.75"), None, None)
                .expect("plan validates");
            assert_eq!(
                sess.snapshot_question(),
                core_input::confirm_input_prompt(&plan),
                "the question is the core prompt verbatim"
            );
            sess.resolve_confirm(false);
        });
        assert_eq!(APPLY_CALLS.load(AtomicOrdering::SeqCst), 0);
        assert_eq!(sess.snapshot_kind(), ModalKind::None);
    }

    #[test]
    fn sensitivity_typo_is_rejected_before_any_confirm() {
        static APPLY_CALLS: AtomicUsize = AtomicUsize::new(0);
        let ops = InputOps {
            apply_input: |_, _| {
                APPLY_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
                Ok(ok_input_report())
            },
            ..succeeding_input_ops()
        };
        let sess = ModalSession::default();
        let base = test_base("sensitivity-typo");
        sensitivity_worker(&base, "abc", &ops, &sess);
        assert_eq!(sess.snapshot_kind(), ModalKind::Error);
        assert_eq!(APPLY_CALLS.load(AtomicOrdering::SeqCst), 0);
    }

    #[test]
    fn accelerate_accept_applies_the_scroll_factor_leaf() {
        static LAST: Mutex<Option<f64>> = Mutex::new(None);
        let ops = InputOps {
            apply_input: |_, plan| {
                *LAST.lock().expect("last lock") = plan.scroll_factor;
                Ok(ok_input_report())
            },
            ..succeeding_input_ops()
        };
        let sess = ModalSession::default();
        let base = test_base("accelerate-accept");
        std::thread::scope(|s| {
            s.spawn(|| accelerate_worker(&base, "2.0", &ops, &sess));
            wait_until("accelerate confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            sess.resolve_confirm(true);
            wait_until("apply runs", || LAST.lock().expect("last lock").is_some());
        });
        assert_eq!(*LAST.lock().expect("last lock"), Some(2.0));
    }

    #[test]
    fn add_clean_combo_confirms_the_core_prompt_then_writes() {
        use archel_core::hyprland::binds as core_binds;
        static EVAL_CALLS: AtomicUsize = AtomicUsize::new(0);
        static WRITTEN: Mutex<String> = Mutex::new(String::new());
        let ops = InputOps {
            eval_bind_line: |_| {
                EVAL_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
                Ok(())
            },
            write_body: |_, body| {
                *WRITTEN.lock().expect("written lock") = body.to_string();
                Ok(())
            },
            ..succeeding_input_ops()
        };
        let sess = ModalSession::default();
        let base = test_base("add-clean");
        std::thread::scope(|s| {
            s.spawn(|| add_keybind_worker(&base, "SUPER + T", "exec kitty", &ops, &sess));
            wait_until("add confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            let spec = core_binds::plan_bind("SUPER", "T", "exec", Some("kitty"))
                .expect("spec plans");
            assert_eq!(
                sess.snapshot_question(),
                core_binds::confirm_bind_prompt(&spec),
                "no collision means the bare core prompt"
            );
            sess.resolve_confirm(true);
            wait_until("write lands", || !WRITTEN.lock().expect("written lock").is_empty());
        });
        assert_eq!(EVAL_CALLS.load(AtomicOrdering::SeqCst), 1);
        let written = WRITTEN.lock().expect("written lock").clone();
        assert!(
            core_binds::parse_snippet_binds(&written).len() == 1,
            "the merged body carries the new bind: {written}"
        );
    }

    #[test]
    fn add_colliding_combo_warns_before_confirm_and_decline_writes_nothing() {
        use archel_core::hyprland::binds as core_binds;
        static EVAL_CALLS: AtomicUsize = AtomicUsize::new(0);
        static WRITE_CALLS: AtomicUsize = AtomicUsize::new(0);
        let ops = InputOps {
            read_live_binds: || {
                core_binds::parse_binds_json(
                    r#"[{"key":"T","modmask":64,"submap":"","dispatcher":"__lua"}]"#,
                )
            },
            eval_bind_line: |_| {
                EVAL_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
                Ok(())
            },
            write_body: |_, _| {
                WRITE_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
                Ok(())
            },
            ..succeeding_input_ops()
        };
        let sess = ModalSession::default();
        let base = test_base("add-collision");
        std::thread::scope(|s| {
            s.spawn(|| add_keybind_worker(&base, "SUPER + T", "exec kitty", &ops, &sess));
            wait_until("add confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            let spec = core_binds::plan_bind("SUPER", "T", "exec", Some("kitty"))
                .expect("spec plans");
            let question = sess.snapshot_question();
            assert!(
                question.contains(&core_binds::confirm_bind_prompt(&spec)),
                "the core prompt still gates the write"
            );
            assert!(
                question.contains("SUPER + T"),
                "the collision warning names the taken combo: {question}"
            );
            sess.resolve_confirm(false);
        });
        assert_eq!(EVAL_CALLS.load(AtomicOrdering::SeqCst), 0);
        assert_eq!(WRITE_CALLS.load(AtomicOrdering::SeqCst), 0);
    }

    #[test]
    fn remove_miss_fails_before_any_confirm() {
        use archel_core::hyprland::binds as core_binds;
        static EVAL_CALLS: AtomicUsize = AtomicUsize::new(0);
        let ops = InputOps {
            eval_unbind: |_| {
                EVAL_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
                Ok(())
            },
            ..succeeding_input_ops()
        };
        let sess = ModalSession::default();
        let base = test_base("remove-miss");
        remove_keybind_worker(&base, "ALT", "F4", "killactive", None, &ops, &sess);
        assert_eq!(
            sess.snapshot_kind(),
            ModalKind::Error,
            "a miss never reaches a confirm"
        );
        assert_eq!(EVAL_CALLS.load(AtomicOrdering::SeqCst), 0);
        let _ = core_binds::confirm_bind_remove_prompt;
    }

    #[test]
    fn remove_accept_unbinds_live_then_removes_the_line() {
        use archel_core::hyprland::binds as core_binds;
        static EVAL_CALLS: AtomicUsize = AtomicUsize::new(0);
        static WRITTEN: Mutex<String> = Mutex::new(String::new());
        let target = core_binds::plan_bind("SUPER", "T", "exec", Some("kitty")).expect("plans");
        let fixture = core_binds::add_bind_line("", &target).expect("fixture body");
        static FIXTURE_BODY: Mutex<String> = Mutex::new(String::new());
        *FIXTURE_BODY.lock().expect("fixture lock") = fixture;
        fn fixture_body(_: &std::path::Path) -> Result<String, archel_core::ArchelError> {
            Ok(FIXTURE_BODY.lock().expect("fixture lock").clone())
        }
        let ops = InputOps {
            read_body: fixture_body,
            eval_unbind: |_| {
                EVAL_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
                Ok(())
            },
            write_body: |_, body| {
                *WRITTEN.lock().expect("written lock") = body.to_string();
                Ok(())
            },
            ..succeeding_input_ops()
        };
        let sess = ModalSession::default();
        let base = test_base("remove-accept");
        std::thread::scope(|s| {
            s.spawn(|| {
                remove_keybind_worker(
                    &base,
                    "SUPER",
                    "T",
                    "exec",
                    Some("kitty"),
                    &ops,
                    &sess,
                )
            });
            wait_until("remove confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            assert_eq!(
                sess.snapshot_question(),
                core_binds::confirm_bind_remove_prompt(&target),
                "removal is always confirmed with the core question"
            );
            sess.resolve_confirm(true);
            wait_until("eval runs", || EVAL_CALLS.load(AtomicOrdering::SeqCst) == 1);
        });
        let written = WRITTEN.lock().expect("written lock").clone();
        assert!(
            core_binds::parse_snippet_binds(&written).is_empty(),
            "the line is gone, siblings untouched: {written}"
        );
    }

    #[test]
    fn remove_external_edit_during_confirm_is_not_clobbered() {
        use archel_core::hyprland::binds as core_binds;
        static READS: AtomicUsize = AtomicUsize::new(0);
        static WRITES: AtomicUsize = AtomicUsize::new(0);
        let target = core_binds::plan_bind("SUPER", "T", "exec", Some("kitty")).expect("plans");
        let fixture = core_binds::add_bind_line("", &target).expect("fixture body");
        static FIXTURE_BODY: Mutex<String> = Mutex::new(String::new());
        *FIXTURE_BODY.lock().expect("fixture lock") = fixture;
        fn vanishing_body(_: &std::path::Path) -> Result<String, archel_core::ArchelError> {
            // First read (pre-confirm miss check) sees the bind; the
            // post-confirm re-read finds it gone — an external edit won
            // the race while the user was thinking.
            if READS.fetch_add(1, AtomicOrdering::SeqCst) == 0 {
                Ok(FIXTURE_BODY.lock().expect("fixture lock").clone())
            } else {
                Ok(String::new())
            }
        }
        let ops = InputOps {
            read_body: vanishing_body,
            write_body: |_, _| {
                WRITES.fetch_add(1, AtomicOrdering::SeqCst);
                Ok(())
            },
            ..succeeding_input_ops()
        };
        let sess = ModalSession::default();
        let base = test_base("remove-drift");
        std::thread::scope(|s| {
            s.spawn(|| {
                remove_keybind_worker(
                    &base,
                    "SUPER",
                    "T",
                    "exec",
                    Some("kitty"),
                    &ops,
                    &sess,
                )
            });
            wait_until("remove confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            sess.resolve_confirm(true);
            wait_until("drift error opens", || {
                sess.snapshot_kind() == ModalKind::Error
            });
        });
        assert_eq!(
            WRITES.load(AtomicOrdering::SeqCst),
            0,
            "the stale pre-confirm snapshot is never written"
        );
    }
}
