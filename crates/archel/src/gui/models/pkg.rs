//! Packages view-model: installed list, live search and package card
//! (read half; plan 06 adds the install/remove/upgrade invokables to
//! this same file).
//!
//! Same proven shape as the other tab models: `tick()` is a QML-called
//! invokable that only spawns - the heavier `list_installed()` (alpm
//! localdb walk) runs on the gui runtime's blocking pool with a
//! sequence tag so a slow list never overwrites a newer one, and the
//! finished payload returns exclusively through `CxxQtThread.queue`,
//! whose closure is the only code allowed to set properties (RESEARCH
//! Pattern 1). Search and card selection follow the same spawn/queue
//! shape with per-query sequence tags (typing fast drops stale hits,
//! T-06-26). Every read goes through the CLI-proven core entry points
//! (`list_installed`, `search` with its core-side `sort_hits`,
//! `package_card` - the `run_list`/`run_search`/`run_info` twins);
//! no GUI-side sorting or field invention (PLAT-03).

use std::sync::atomic::AtomicBool;

use cxx_qt_lib::QString;

use super::super::poll::{BannerPolicy, PollGuard};
use crate::gui::modal::{self, ModalSession};
use crate::gui::models::system::NOT_AVAILABLE;

/// Fully formatted list payload, built off the Qt thread so the
/// `queue()` closure only sets properties.
pub(crate) struct AppliedPkgList {
    pub installed_json: String,
    pub banner_visible: bool,
}

/// Fully formatted search payload with its dispatch tag, so the
/// `queue()` closure can drop stale hits (T-06-26).
pub(crate) struct AppliedSearch {
    pub tag: u64,
    pub search_json: String,
    pub zero_hit: bool,
    pub busy: bool,
}

/// Fully formatted card payload with its dispatch tag.
pub(crate) struct AppliedCard {
    pub tag: u64,
    pub card_json: String,
}

/// Injection seam for the write path: the exact core entry points the
/// workers call, as plain function pointers. Production passes the
/// real core functions ([`PROD_WRITE_OPS`]); unit tests pass fakes so
/// the confirm/cancel choreography is proven without spawning pkexec,
/// pacman or yay (the subprocess contract itself is 06-02's tested
/// territory — the GUI never holds a child, a pid or a signal).
#[derive(Clone, Copy)]
#[allow(clippy::type_complexity)]
pub(crate) struct PkgWriteOps {
    pub install_preview:
        fn(&[String]) -> Result<archel_core::packages::InstallPreview, archel_core::ArchelError>,
    pub remove_preview:
        fn(&[String]) -> Result<archel_core::packages::RemovePreview, archel_core::ArchelError>,
    pub upgrade_preview: fn() -> Result<Vec<String>, archel_core::ArchelError>,
    pub run_streaming: fn(
        &str,
        &[String],
        &str,
        &mut dyn FnMut(&str),
        &AtomicBool,
    ) -> Result<archel_core::privileged::CancellableStreaming, archel_core::ArchelError>,
}

/// The production wiring: every field is the CLI-proven core function,
/// so the GUI and the CLI share one implementation of preview, prompt
/// and transaction (thin-view contract; PLAT-03).
pub(crate) const PROD_WRITE_OPS: PkgWriteOps = PkgWriteOps {
    install_preview: archel_core::packages::install_preview,
    remove_preview: archel_core::packages::remove_preview,
    upgrade_preview: archel_core::packages::upgrade_preview,
    run_streaming: archel_core::privileged::run_privileged_streaming_cancellable,
};

/// Injection seam for the AUR path: the exact core entry points the
/// review/build workers call, same fn-pointer shape as
/// [`PkgWriteOps`]. Production wires the CLI-proven aur module
/// ([`PROD_AUR_OPS`]); unit tests pass fakes so the D-19 gate is
/// proven without any network fetch or yay child.
#[derive(Clone, Copy)]
#[allow(clippy::type_complexity)]
pub(crate) struct PkgAurOps {
    pub validate_names: fn(&[String]) -> Result<(), archel_core::ArchelError>,
    pub review: fn(&str) -> Result<archel_core::aur::AurReview, archel_core::ArchelError>,
    pub validate_source:
        fn(&str, &str) -> Result<Vec<String>, archel_core::ArchelError>,
    pub build: fn(
        &str,
        &mut dyn FnMut(&str),
        &AtomicBool,
    ) -> Result<archel_core::privileged::CancellableStreaming, archel_core::ArchelError>,
}

/// Production AUR wiring: every field is the CLI-proven core function
/// (thin-view contract; PLAT-03). The GUI never fetches, validates or
/// spawns anything itself - yay is reached only through
/// `yay_build_cancellable`, the 06-02 cancellable variant.
pub(crate) const PROD_AUR_OPS: PkgAurOps = PkgAurOps {
    validate_names: archel_core::packages::validate_package_names,
    review: archel_core::aur::review_package,
    validate_source: archel_core::aur::validate_pkgbuild_source,
    build: archel_core::aur::yay_build_cancellable,
};

/// Builds the confirmation question: the core's own `confirm_*` prompt
/// verbatim, with the preview section appended under the core's own
/// label (no GUI-invented French, no reconstructed prompt).
fn question_with(prompt: &str, label: &str, list: &[String]) -> String {
    if list.is_empty() {
        return prompt.to_string();
    }
    format!("{prompt}\n\n{label}: {}", list.join(", "))
}

/// Blocks the worker on the modal's one-shot answer. `None`-shaped
/// results (session dropped mid-wait) default to No — the same
/// default-No semantics as the CLI's `parse_confirm_answer`.
fn await_confirm(sess: &ModalSession, question: &str) -> bool {
    let rx = sess.request_confirm(question);
    rx.recv().unwrap_or(false)
}

/// Streams one privileged transaction through core's cancellable
/// helper: every stdout line feeds the modal buffer, and the returned
/// `{ cancelled, exit_code }` pair decides the D-14 end state — a
/// user-cancelled stop is reported as data, never collapsed into an
/// error. A core error (`NeedsPrivilege` shapes included) surfaces as
/// its readable `message()`, never raw stderr.
fn run_streamed(
    sess: &ModalSession,
    ops: &PkgWriteOps,
    verb: &str,
    packages: &[String],
    action_label: &str,
) {
    sess.open_stream();
    let outcome = (ops.run_streaming)(
        verb,
        packages,
        action_label,
        &mut |line| sess.push_line(line),
        sess.cancel_handle(),
    );
    match outcome {
        Ok(streamed) => sess.finish_stream(streamed.cancelled, streamed.exit_code),
        Err(err) => sess.open_error(&err.message()),
    }
}

/// Install worker body (runs on the blocking pool): preview first so
/// invalid/unknown names are rejected with the honest typed error
/// BEFORE any confirmation is shown; AUR targets are refused here with
/// core's hint (the AUR flow is its own reviewed path); the question
/// is the core prompt verbatim; only a yes reaches the helper.
pub(crate) fn install_worker(names: &[String], ops: &PkgWriteOps, sess: &ModalSession) {
    let preview = match (ops.install_preview)(names) {
        Ok(preview) => preview,
        Err(err) => {
            sess.open_error(&err.message());
            return;
        }
    };
    let aur_names: Vec<String> = preview
        .cards
        .iter()
        .filter(|card| archel_core::aur::is_aur_card(card))
        .map(|card| card.name.clone())
        .collect();
    if !aur_names.is_empty() {
        sess.open_error(&archel_core::aur::is_aur_hint_list(&aur_names));
        return;
    }
    let question = question_with(
        &archel_core::packages::confirm_install_prompt(names),
        &archel_core::packages::preview_dependencies_label(),
        &preview.dependencies,
    );
    if !await_confirm(sess, &question) {
        return;
    }
    run_streamed(
        sess,
        ops,
        archel_core::privileged::PKG_INSTALL_VERB,
        names,
        &archel_core::privileged::pkg_install_action_label(),
    );
}

/// Remove worker body: same shape as install — the removal closure is
/// previewed before the question, unknown names never reach a confirm.
pub(crate) fn remove_worker(names: &[String], ops: &PkgWriteOps, sess: &ModalSession) {
    let preview = match (ops.remove_preview)(names) {
        Ok(preview) => preview,
        Err(err) => {
            sess.open_error(&err.message());
            return;
        }
    };
    let question = question_with(
        &archel_core::packages::confirm_remove_prompt(names),
        &archel_core::packages::preview_removals_label(),
        &preview.removals,
    );
    if !await_confirm(sess, &question) {
        return;
    }
    run_streamed(
        sess,
        ops,
        archel_core::privileged::PKG_REMOVE_VERB,
        names,
        &archel_core::privileged::pkg_remove_action_label(),
    );
}

/// Upgrade worker body: an empty pending preview is the honest
/// up-to-date line (a confirm carries the message; nothing spawns —
/// this path never runs `--print`, Pitfall 4). Otherwise the core
/// upgrade prompt plus the pending list gates the full-system
/// transaction.
pub(crate) fn upgrade_worker(ops: &PkgWriteOps, sess: &ModalSession) {
    let pending = match (ops.upgrade_preview)() {
        Ok(pending) => pending,
        Err(err) => {
            sess.open_error(&err.message());
            return;
        }
    };
    if pending.is_empty() {
        sess.open_confirm(&archel_core::packages::up_to_date_line());
        return;
    }
    let question = question_with(
        &archel_core::packages::confirm_upgrade_prompt(),
        &archel_core::packages::preview_pending_label(),
        &pending,
    );
    if !await_confirm(sess, &question) {
        return;
    }
    run_streamed(
        sess,
        ops,
        archel_core::privileged::PKG_UPGRADE_VERB,
        &[],
        &archel_core::privileged::pkg_upgrade_action_label(),
    );
}

/// AUR review worker (blocking pool): the strict name gate first
/// (nothing fetches for an invalid name), then the core's complete
/// review - card, PKGBUILD fetch, `bash -n` validation, trust signals
/// and warnings all happen core-side, exactly as the CLI twin does.
/// A repo-sourced name is redirected with core's own hint (builds
/// nothing); yay absence surfaces as the typed YayMissing error, so
/// the flow is inert without yay (UI-SPEC error-aur-copy backstop).
/// Only a successful review opens the trust block.
pub(crate) fn aur_review_worker(name: &str, ops: &PkgAurOps, sess: &ModalSession) {
    if let Err(err) = (ops.validate_names)(&[name.to_string()]) {
        sess.open_error(&err.message());
        return;
    }
    let review = match (ops.review)(name) {
        Ok(review) => review,
        Err(err) => {
            sess.open_error(&err.message());
            return;
        }
    };
    if !archel_core::aur::is_aur_card(&review.card) {
        sess.open_error(&archel_core::aur::not_aur_hint(name));
        return;
    }
    sess.begin_aur_review(name, &aur_summary_json(name, &review), &review.pkgbuild);
}

/// AUR build worker: the D-19 / T-06-29 gate lives HERE, never in a
/// QML binding - both flags are snapshotted before `open_stream`
/// (whose reset deliberately preserves the one-way `pkgbuild_seen`
/// latch but clears `trust_confirmed`). `validate_pkgbuild_source`
/// re-checks the exact text that was shown before anything runs, and
/// only then does the cancellable yay build stream through the same
/// session machinery as the repo writes (Annuler always available).
pub(crate) fn aur_build_worker(sess: &ModalSession, ops: &PkgAurOps) {
    let name = sess.snapshot_aur_name();
    if name.is_empty() {
        return;
    }
    let seen = sess.snapshot_pkgbuild_seen();
    let trusted = sess.snapshot_trust_confirmed();
    if !modal::aur_gate_allows(seen, trusted) {
        return;
    }
    if let Err(err) = (ops.validate_source)(&name, &sess.snapshot_pkgbuild()) {
        sess.open_error(&err.message());
        return;
    }
    sess.open_stream();
    let outcome = (ops.build)(&name, &mut |line| sess.push_line(line), sess.cancel_handle());
    match outcome {
        Ok(streamed) => {
            if !streamed.cancelled {
                sess.push_line(&archel_core::aur::aur_ok_line(&name));
            }
            sess.finish_stream(streamed.cancelled, streamed.exit_code);
        }
        Err(err) => sess.open_error(&err.message()),
    }
}

/// Summary payload for the StreamModal trust block: the D-04 card
/// fields, the four trust signals (honest nulls, never zero-filled)
/// with their core catalog labels, and the dependency/warning lists
/// with theirs. French comes from the core catalog at runtime - this
/// file stays plain ASCII (D-07); QML only renders fields.
pub(crate) fn aur_summary_json(name: &str, review: &archel_core::aur::AurReview) -> String {
    let labels = archel_core::aur::trust_labels();
    serde_json::json!({
        "name": name,
        "version": review.card.candidate_version,
        "source": review.card.source,
        "maintainerLabel": labels.maintainer,
        "votesLabel": labels.votes,
        "popularityLabel": labels.popularity,
        "outdatedLabel": labels.out_of_date,
        "dependenciesLabel": archel_core::aur::aur_dependencies_label(),
        "warningsLabel": archel_core::aur::warnings_title(),
        "maintainer": review.trust.maintainer,
        "votes": review.trust.votes,
        "popularity": review.trust.popularity,
        "outOfDate": review.trust.out_of_date,
        "dependencies": review.dependencies,
        "warnings": review.warnings,
    })
    .to_string()
}

/// Parses the batch-selection payload QML sends (`JSON.stringify` of
/// the checked rows). Anything unparsable is an honest empty batch —
/// the invokable refuses it without touching the session.
pub(crate) fn parse_names_json(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw)
        .unwrap_or_default()
        .into_iter()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect()
}

/// Backing state: property fields plus the poll guard, the banner
/// policy and two plain sequence counters. The counters are only
/// touched on the Qt thread (`search()`/`select()` dispatch there and
/// the `queue()` closures run there), so no atomics are needed for
/// them; the guard covers the 1 Hz list tick.
pub struct PkgModelRust {
    pub(crate) installed_json: QString,
    pub(crate) search_json: QString,
    pub(crate) card_json: QString,
    pub(crate) search_busy: bool,
    pub(crate) zero_hit: bool,
    pub(crate) banner_visible: bool,
    pub(crate) operation_busy: bool,
    pub(crate) operation_kind: QString,
    guard: PollGuard,
    banner: BannerPolicy,
    search_seq: u64,
    card_seq: u64,
}

impl Default for PkgModelRust {
    fn default() -> Self {
        Self {
            installed_json: QString::from(""),
            search_json: QString::from(""),
            card_json: QString::from(""),
            search_busy: false,
            zero_hit: false,
            banner_visible: false,
            operation_busy: false,
            operation_kind: QString::from(""),
            guard: PollGuard::new(),
            banner: BannerPolicy::new(),
            search_seq: 0,
            card_seq: 0,
        }
    }
}

/// Installed rows as QML-ready JSON: `[{name,version,size}]` in
/// name-ascending order (the core read's own order) - `size` is the
/// alpm installed size in bytes; QML renders it and drives the size
/// filters off it.
pub(crate) fn installed_json_for(pkgs: &[archel_core::packages::InstalledPkg]) -> String {
    let rows: Vec<serde_json::Value> = pkgs
        .iter()
        .map(|pkg| {
            serde_json::json!({
                "name": pkg.name,
                "version": pkg.version,
                "size": pkg.size_bytes,
            })
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string())
}

/// Search hits as QML-ready JSON: `[{name,version,source}]` in the
/// core's `sort_hits` order (newest first, repo before AUR) - never
/// re-sorted here.
pub(crate) fn search_json_for(hits: &[archel_core::packages::PackageInfo]) -> String {
    let rows: Vec<serde_json::Value> = hits
        .iter()
        .map(|hit| {
            serde_json::json!({
                "name": hit.name,
                "version": hit.version,
                "source": hit.source.as_deref().unwrap_or("-"),
            })
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string())
}

/// Package card as QML-ready JSON: the core `PackageCard` fields
/// verbatim (name, candidate/installed versions, source, sizes), in
/// the `card_labels` field set - no GUI-side field invention.
pub(crate) fn card_json_for(card: &archel_core::packages::PackageCard) -> String {
    serde_json::to_string(card).unwrap_or_else(|_| "{}".to_string())
}

/// Honest not-found card: every value reads N/A, never a zero-filled
/// card (mirrors the CLI's typed not-found error).
pub(crate) fn na_card_json(name: &str) -> String {
    serde_json::json!({
        "name": name,
        "candidate_version": NOT_AVAILABLE,
        "installed_version": NOT_AVAILABLE,
        "source": NOT_AVAILABLE,
        "download_size": NOT_AVAILABLE,
        "installed_size": NOT_AVAILABLE,
    })
    .to_string()
}

impl PkgModelRust {
    /// Applies a finished list outcome on the Qt thread. `Err` (alpm
    /// localdb unreadable) reads the honest empty list plus the
    /// once-per-episode banner.
    fn apply_list(
        &mut self,
        outcome: Result<Vec<archel_core::packages::InstalledPkg>, archel_core::ArchelError>,
    ) -> AppliedPkgList {
        match outcome {
            Ok(pkgs) => {
                self.banner.on_success();
                AppliedPkgList {
                    installed_json: installed_json_for(&pkgs),
                    banner_visible: false,
                }
            }
            Err(_) => {
                self.banner.on_failure();
                AppliedPkgList {
                    installed_json: "[]".to_string(),
                    banner_visible: self.banner.wants_visible(),
                }
            }
        }
    }

    /// Applies a finished search outcome. Only the newest tag applies
    /// (T-06-26 stale-drop); `zero_hit` arms the refinement copy only
    /// for a completed non-blank query with no hits.
    fn apply_search(
        &mut self,
        tag: u64,
        query_blank: bool,
        hits: Vec<archel_core::packages::PackageInfo>,
    ) -> Option<AppliedSearch> {
        if self.search_seq != tag {
            return None;
        }
        let zero_hit = !query_blank && hits.is_empty();
        Some(AppliedSearch {
            tag,
            search_json: search_json_for(&hits),
            zero_hit,
            busy: false,
        })
    }

    /// Applies a finished card outcome. Only the newest tag applies;
    /// `Err` (unknown name) reads the N/A card, never a zero-filled one.
    fn apply_card(
        &mut self,
        tag: u64,
        name: String,
        outcome: Result<archel_core::packages::PackageCard, archel_core::ArchelError>,
    ) -> Option<AppliedCard> {
        if self.card_seq != tag {
            return None;
        }
        Some(AppliedCard {
            tag,
            card_json: match outcome {
                Ok(card) => card_json_for(&card),
                Err(_) => na_card_json(&name),
            },
        })
    }
}

impl crate::gui::bridge_pkg::qobject::PkgModel {
    /// 1 Hz entry point: skips while a tick is in flight (D-02),
    /// spawns the heavier installed-list read, marshals back through
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
            let outcome = archel_core::packages::list_installed_with_sizes();
            let _ = qt_thread.queue(
                move |mut obj: core::pin::Pin<&mut Self>| {
                    let applied = {
                        let mut rust = obj.as_mut().rust_mut();
                        if !rust.guard.is_newest(tag) {
                            rust.guard.finish();
                            return;
                        }
                        rust.guard.finish();
                        rust.apply_list(outcome)
                    };
                    obj.as_mut()
                        .set_installed_json(QString::from(&applied.installed_json));
                    obj.as_mut().set_banner_visible(applied.banner_visible);
                },
            );
        });
    }

    /// Live search: spawns `packages::search` on the blocking pool
    /// with a per-query sequence tag - rapid typing drops stale hits
    /// (T-06-26). A blank query clears the search back to the
    /// installed list without spawning.
    pub fn search(mut self: core::pin::Pin<&mut Self>, query: QString) {
        use cxx_qt::{CxxQtType, Threading};
        let text = query.to_string();
        if text.trim().is_empty() {
            let mut rust = self.as_mut().rust_mut();
            rust.search_seq = rust.search_seq.wrapping_add(1);
            self.as_mut().set_search_json(QString::from(""));
            self.as_mut().set_zero_hit(false);
            self.as_mut().set_search_busy(false);
            return;
        }
        let tag = {
            let mut rust = self.as_mut().rust_mut();
            rust.search_seq = rust.search_seq.wrapping_add(1);
            rust.search_seq
        };
        self.as_mut().set_search_busy(true);
        let qt_thread = self.qt_thread();
        let runtime = super::super::RUNTIME
            .get()
            .expect("gui runtime is set before Qt starts");
        runtime.spawn_blocking(move || {
            let query_blank = text.trim().is_empty();
            let hits = archel_core::packages::search(&text).unwrap_or_default();
            let _ = qt_thread.queue(
                move |mut obj: core::pin::Pin<&mut Self>| {
                    let applied = {
                        let mut rust = obj.as_mut().rust_mut();
                        match rust.apply_search(tag, query_blank, hits) {
                            Some(applied) => applied,
                            None => return,
                        }
                    };
                    let _ = applied.tag;
                    obj.as_mut()
                        .set_search_json(QString::from(&applied.search_json));
                    obj.as_mut().set_zero_hit(applied.zero_hit);
                    obj.as_mut().set_search_busy(applied.busy);
                },
            );
        });
    }

    /// Card selection: spawns `package_card` on the blocking pool with
    /// a per-selection sequence tag; an unknown name reads the N/A
    /// card.
    pub fn select(mut self: core::pin::Pin<&mut Self>, name: QString) {
        use cxx_qt::{CxxQtType, Threading};
        let text = name.to_string();
        let tag = {
            let mut rust = self.as_mut().rust_mut();
            rust.card_seq = rust.card_seq.wrapping_add(1);
            rust.card_seq
        };
        let qt_thread = self.qt_thread();
        let runtime = super::super::RUNTIME
            .get()
            .expect("gui runtime is set before Qt starts");
        runtime.spawn_blocking(move || {
            let outcome = archel_core::packages::package_card(&text);
            let _ = qt_thread.queue(
                move |mut obj: core::pin::Pin<&mut Self>| {
                    let applied = {
                        let mut rust = obj.as_mut().rust_mut();
                        match rust.apply_card(tag, text.clone(), outcome) {
                            Some(applied) => applied,
                            None => return,
                        }
                    };
                    let _ = applied.tag;
                    obj.as_mut()
                        .set_card_json(QString::from(&applied.card_json));
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

    /// Opens an install session for the checked rows. Empty or
    /// unparsable batches are refused here, before the session is
    /// touched; invalid names are then rejected worker-side by the
    /// core preview (pre-confirm, D-01).
    pub fn install_start(mut self: core::pin::Pin<&mut Self>, names_json: QString) {
        let names = parse_names_json(&names_json.to_string());
        if names.is_empty() {
            return;
        }
        if !self.as_mut().begin_write_operation("install") {
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
            install_worker(&names, &PROD_WRITE_OPS, modal::session());
            let _ = qt_thread.queue(|mut obj: core::pin::Pin<&mut Self>| {
                obj.as_mut().finish_write_operation();
            });
        });
    }

    /// Opens the full-system upgrade session (D-03: one refresh path,
    /// targeting all pending; no partial-upgrade verb exists).
    pub fn upgrade_start(mut self: core::pin::Pin<&mut Self>) {
        if !self.as_mut().begin_write_operation("upgrade") {
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
            upgrade_worker(&PROD_WRITE_OPS, modal::session());
            let _ = qt_thread.queue(|mut obj: core::pin::Pin<&mut Self>| {
                obj.as_mut().finish_write_operation();
            });
        });
    }

    /// Opens a removal session for the checked rows (same refusal and
    /// preview-before-confirm shape as install).
    pub fn remove_start(mut self: core::pin::Pin<&mut Self>, names_json: QString) {
        let names = parse_names_json(&names_json.to_string());
        if names.is_empty() {
            return;
        }
        if !self.as_mut().begin_write_operation("remove") {
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
            remove_worker(&names, &PROD_WRITE_OPS, modal::session());
            let _ = qt_thread.queue(|mut obj: core::pin::Pin<&mut Self>| {
                obj.as_mut().finish_write_operation();
            });
        });
    }

    /// Starts the AUR review for one selected name (D-19): the core
    /// fetches, validates and assembles the review off the Qt thread,
    /// then the StreamModal trust block opens from the session. With
    /// yay absent the review fails as the typed YayMissing error -
    /// the whole AUR flow stays inert, stated honestly.
    pub fn aur_review_start(mut self: core::pin::Pin<&mut Self>, name: QString) {
        let name = name.to_string().trim().to_string();
        if name.is_empty() {
            return;
        }
        if !self.as_mut().begin_write_operation("aur") {
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
            aur_review_worker(&name, &PROD_AUR_OPS, modal::session());
            let _ = qt_thread.queue(|mut obj: core::pin::Pin<&mut Self>| {
                obj.as_mut().finish_write_operation();
            });
        });
    }

    /// Second stage of the AUR flow, reached only from the modal's
    /// Valider button. No pre-check here on purpose: the authoritative
    /// D-19 gate is the snapshot inside `aur_build_worker`, so no
    /// caller - QML binding bug included - reaches yay without both
    /// flags. Shares the single-operation slot with the repo writes.
    pub fn aur_build_start(mut self: core::pin::Pin<&mut Self>) {
        if !self.as_mut().begin_write_operation("aur-build") {
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
            aur_build_worker(modal::session(), &PROD_AUR_OPS);
            let _ = qt_thread.queue(|mut obj: core::pin::Pin<&mut Self>| {
                obj.as_mut().finish_write_operation();
            });
        });
    }

    /// Claims the single-operation slot (one write session at a time;
    /// the second click of a double-click loses here) and mirrors the
    /// claim onto the QML-visible flags so the toolbar can disable
    /// itself and show the busy label (D-15).
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

    /// Releases the operation slot and refreshes the installed list
    /// so the table reflects the transaction that just ended. Called
    /// from the `queue()` closure only (property writes stay on the
    /// Qt thread, RESEARCH Pattern 1).
    fn finish_write_operation(mut self: core::pin::Pin<&mut Self>) {
        use cxx_qt::CxxQtType;
        self.as_mut().rust_mut().operation_busy = false;
        self.as_mut().set_operation_busy(false);
        self.as_mut().set_operation_kind(QString::from(""));
        self.as_mut().tick();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archel_core::packages::{PackageCard, PackageInfo};

    fn installed_fixture() -> Vec<archel_core::packages::InstalledPkg> {
        vec![
            archel_core::packages::InstalledPkg {
                name: "linux".to_string(),
                version: "6.16.1-1".to_string(),
                size_bytes: 180_000_000,
            },
            archel_core::packages::InstalledPkg {
                name: "neovim".to_string(),
                version: "0.11.3-1".to_string(),
                size_bytes: 6_000_000,
            },
        ]
    }

    fn search_fixture() -> Vec<PackageInfo> {
        vec![PackageInfo {
            name: "helix".to_string(),
            version: "25.07-1".to_string(),
            source: Some("extra".to_string()),
        }]
    }

    fn card_fixture() -> PackageCard {
        PackageCard {
            name: "helix".to_string(),
            candidate_version: "25.07-1".to_string(),
            installed_version: None,
            source: "extra".to_string(),
            download_size: Some("8.1 MiB".to_string()),
            installed_size: Some("30.4 MiB".to_string()),
        }
    }

    #[test]
    fn stale_search_hits_are_dropped_never_applied() {
        let mut model = PkgModelRust {
            search_seq: 2,
            ..Default::default()
        };
        let stale = model.apply_search(1, false, search_fixture());
        assert!(stale.is_none(), "an overtaken tag must not apply");
        let fresh = model
            .apply_search(2, false, search_fixture())
            .expect("newest tag applies");
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(&fresh.search_json).expect("search must be JSON");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], "helix");
        assert_eq!(rows[0]["source"], "extra");
        assert!(!fresh.busy, "the accepted hit clears the busy flag");
    }

    #[test]
    fn zero_hit_arms_the_refinement_copy_key() {
        let mut model = PkgModelRust {
            search_seq: 1,
            ..Default::default()
        };
        let applied = model
            .apply_search(1, false, Vec::new())
            .expect("newest tag applies");
        assert!(applied.zero_hit, "zero hits arm the refinement copy");
        assert_eq!(applied.search_json, "[]");
        model.search_seq = 2;
        let hits = model
            .apply_search(2, false, search_fixture())
            .expect("newest tag applies");
        assert!(!hits.zero_hit, "hits clear the refinement copy");
    }

    #[test]
    fn card_error_reads_na_never_zero_filled() {
        let mut model = PkgModelRust {
            card_seq: 1,
            ..Default::default()
        };
        let err: Result<PackageCard, archel_core::ArchelError> =
            Err(archel_core::ArchelError::PackageNotFound {
                name: "nope".to_string(),
            });
        let applied = model
            .apply_card(1, "nope".to_string(), err)
            .expect("newest tag applies");
        let card: serde_json::Value =
            serde_json::from_str(&applied.card_json).expect("card must be JSON");
        assert_eq!(card["name"], "nope", "the requested name survives");
        for key in [
            "candidate_version",
            "installed_version",
            "source",
            "download_size",
            "installed_size",
        ] {
            assert_eq!(
                card[key], NOT_AVAILABLE,
                "unknown package reads N/A for {key}"
            );
        }
        model.card_seq = 2;
        let stale = model.apply_card(1, "nope".to_string(), Ok(card_fixture()));
        assert!(stale.is_none(), "overtaken card selections never apply");
    }

    #[test]
    fn list_success_keeps_core_order_and_failure_reads_empty() {
        let mut model = PkgModelRust::default();
        let applied = model.apply_list(Ok(installed_fixture()));
        assert!(!applied.banner_visible, "success hides the banner");
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(&applied.installed_json).expect("list must be JSON");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], "linux");
        assert!(rows[0].get("version").is_some());
        assert_eq!(rows[0]["size"], 180_000_000, "size column must survive");
        let err: Result<
            Vec<archel_core::packages::InstalledPkg>,
            archel_core::ArchelError,
        > = Err(archel_core::ArchelError::Internal(
            "alpm unavailable in test".to_string(),
        ));
        let failed = model.apply_list(err);
        assert_eq!(failed.installed_json, "[]");
        assert!(failed.banner_visible, "first failure shows the banner");
    }

    #[test]
    fn card_success_follows_the_core_label_set() {
        let json = card_json_for(&card_fixture());
        let card: serde_json::Value =
            serde_json::from_str(&json).expect("card must be JSON");
        for key in [
            "name",
            "candidate_version",
            "installed_version",
            "source",
            "download_size",
            "installed_size",
        ] {
            assert!(card.get(key).is_some(), "card keeps the core field {key}");
        }
        assert_eq!(card["candidate_version"], "25.07-1");
    }

    // -- Write-path choreography (plan 06-06, Task 2) --
    //
    // The fakes below replace only the four `PkgWriteOps` seams; the
    // workers themselves run for real against a local ModalSession, so
    // preview-before-confirm, default-No, the AUR refusal, the D-14
    // end states and the verb/package plumbing are all proven without
    // ever spawning pkexec, pacman or yay.

    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use archel_core::packages::{InstallPreview, RemovePreview};
    use archel_core::privileged::CancellableStreaming;

    use crate::gui::modal::{EndState, ModalKind};

    /// Polls `cond` until it holds or 5 s elapse. The workers run on
    /// real threads; this is the rendezvous, not a race lottery.
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

    fn names(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn ok_install_preview(names: &[String]) -> Result<InstallPreview, archel_core::ArchelError> {
        Ok(InstallPreview {
            cards: names
                .iter()
                .map(|n| PackageCard {
                    name: n.clone(),
                    candidate_version: "1.0-1".to_string(),
                    installed_version: None,
                    source: "extra".to_string(),
                    download_size: None,
                    installed_size: None,
                })
                .collect(),
            dependencies: vec![
                format!("{base}-libs", base = names[0]),
                "glibc".to_string(),
            ],
        })
    }

    fn aur_install_preview(names: &[String]) -> Result<InstallPreview, archel_core::ArchelError> {
        Ok(InstallPreview {
            cards: names
                .iter()
                .map(|n| PackageCard {
                    name: n.clone(),
                    candidate_version: "1.0-1".to_string(),
                    installed_version: None,
                    source: "aur".to_string(),
                    download_size: None,
                    installed_size: None,
                })
                .collect(),
            dependencies: Vec::new(),
        })
    }

    fn ok_remove_preview(names: &[String]) -> Result<RemovePreview, archel_core::ArchelError> {
        Ok(RemovePreview {
            installed: names.iter().map(|n| (n.clone(), "1.0-1".to_string())).collect(),
            removals: names.to_vec(),
        })
    }

    fn package_not_found(names: &[String]) -> Result<InstallPreview, archel_core::ArchelError> {
        Err(archel_core::ArchelError::PackageNotFound {
            name: names[0].clone(),
        })
    }

    fn streaming_succeeded(
        _verb: &str,
        _packages: &[String],
        _label: &str,
        on_line: &mut dyn FnMut(&str),
        _cancel: &AtomicBool,
    ) -> Result<CancellableStreaming, archel_core::ArchelError> {
        on_line(":: processing package foo-1.0-1-x86_64");
        on_line(":: running post transaction hooks");
        Ok(CancellableStreaming {
            transcript: String::new(),
            cancelled: false,
            exit_code: Some(0),
        })
    }

    #[test]
    fn install_decline_previews_before_confirm_and_never_streams() {
        static STREAM_CALLS: AtomicUsize = AtomicUsize::new(0);
        fn stream_must_not_run(
            _verb: &str,
            _packages: &[String],
            _label: &str,
            _on_line: &mut dyn FnMut(&str),
            _cancel: &AtomicBool,
        ) -> Result<CancellableStreaming, archel_core::ArchelError> {
            STREAM_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            Err(archel_core::ArchelError::Internal(
                "helper must never run after a decline".to_string(),
            ))
        }
        let ops = PkgWriteOps {
            install_preview: ok_install_preview,
            remove_preview: ok_remove_preview,
            upgrade_preview: || Ok(Vec::new()),
            run_streaming: stream_must_not_run,
        };
        let sess = ModalSession::default();
        let targets = names(&["foo"]);
        std::thread::scope(|s| {
            s.spawn(|| install_worker(&targets, &ops, &sess));
            wait_until("install confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            // The question is the core prompt verbatim, dependencies
            // appended under the core's own label (no invented copy).
            let expected = format!(
                "{}\n\n{}: {}",
                archel_core::packages::confirm_install_prompt(&targets),
                archel_core::packages::preview_dependencies_label(),
                "foo-libs, glibc"
            );
            assert_eq!(sess.snapshot_question(), expected);
            sess.resolve_confirm(false);
        });
        assert_eq!(
            STREAM_CALLS.load(AtomicOrdering::SeqCst),
            0,
            "decline must never reach the helper"
        );
        assert_eq!(sess.snapshot_kind(), ModalKind::None);
    }

    #[test]
    fn install_accept_streams_with_the_core_verb_and_packages() {
        static LAST: Mutex<(String, Vec<String>)> = Mutex::new((String::new(), Vec::new()));
        fn record_and_succeed(
            verb: &str,
            packages: &[String],
            _label: &str,
            on_line: &mut dyn FnMut(&str),
            cancel: &AtomicBool,
        ) -> Result<CancellableStreaming, archel_core::ArchelError> {
            *LAST.lock().expect("last lock") = (verb.to_string(), packages.to_vec());
            streaming_succeeded(verb, packages, _label, on_line, cancel)
        }
        let ops = PkgWriteOps {
            install_preview: ok_install_preview,
            remove_preview: ok_remove_preview,
            upgrade_preview: || Ok(Vec::new()),
            run_streaming: record_and_succeed,
        };
        let sess = ModalSession::default();
        let targets = names(&["foo", "bar"]);
        std::thread::scope(|s| {
            s.spawn(|| install_worker(&targets, &ops, &sess));
            wait_until("install confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            sess.resolve_confirm(true);
            wait_until("stream finishes", || sess.snapshot_done());
        });
        let (verb, packages) = LAST.lock().expect("last lock").clone();
        assert_eq!(verb, archel_core::privileged::PKG_INSTALL_VERB);
        assert_eq!(packages, targets, "the batch reaches core verbatim");
        assert_eq!(sess.snapshot_kind(), ModalKind::Stream);
        assert!(sess.snapshot_lines_joined().contains("processing package foo"));
        assert_eq!(sess.snapshot_result(), EndState::Completed);
        assert_eq!(sess.snapshot_exit_code(), Some(0));
    }

    #[test]
    fn invalid_name_is_rejected_before_any_confirm() {
        static STREAM_CALLS: AtomicUsize = AtomicUsize::new(0);
        fn stream_must_not_run(
            _verb: &str,
            _packages: &[String],
            _label: &str,
            _on_line: &mut dyn FnMut(&str),
            _cancel: &AtomicBool,
        ) -> Result<CancellableStreaming, archel_core::ArchelError> {
            STREAM_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(CancellableStreaming {
                transcript: String::new(),
                cancelled: false,
                exit_code: Some(0),
            })
        }
        let ops = PkgWriteOps {
            install_preview: package_not_found,
            remove_preview: ok_remove_preview,
            upgrade_preview: || Ok(Vec::new()),
            run_streaming: stream_must_not_run,
        };
        let sess = ModalSession::default();
        install_worker(&names(&["nope!"]), &ops, &sess);
        assert_eq!(sess.snapshot_kind(), ModalKind::Error);
        assert_eq!(
            sess.snapshot_error(),
            archel_core::ArchelError::PackageNotFound {
                name: "nope!".to_string()
            }
            .message(),
            "the error modal carries core's readable message"
        );
        assert_eq!(STREAM_CALLS.load(AtomicOrdering::SeqCst), 0);
    }

    #[test]
    fn aur_card_is_refused_with_the_core_hint_never_a_confirm() {
        let ops = PkgWriteOps {
            install_preview: aur_install_preview,
            remove_preview: ok_remove_preview,
            upgrade_preview: || Ok(Vec::new()),
            run_streaming: |_, _, _, _, _| Err(archel_core::ArchelError::Internal(
                "must not stream".to_string(),
            )),
        };
        let sess = ModalSession::default();
        let targets = names(&["cool-aur-thing"]);
        install_worker(&targets, &ops, &sess);
        assert_eq!(
            sess.snapshot_kind(),
            ModalKind::Error,
            "AUR goes to the error modal, not a confirm"
        );
        assert_eq!(
            sess.snapshot_error(),
            archel_core::aur::is_aur_hint_list(&targets),
            "core's own AUR hint is reused verbatim"
        );
    }

    #[test]
    fn remove_decline_never_streams() {
        static STREAM_CALLS: AtomicUsize = AtomicUsize::new(0);
        fn stream_must_not_run(
            _verb: &str,
            _packages: &[String],
            _label: &str,
            _on_line: &mut dyn FnMut(&str),
            _cancel: &AtomicBool,
        ) -> Result<CancellableStreaming, archel_core::ArchelError> {
            STREAM_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(CancellableStreaming {
                transcript: String::new(),
                cancelled: false,
                exit_code: Some(0),
            })
        }
        let ops = PkgWriteOps {
            install_preview: ok_install_preview,
            remove_preview: ok_remove_preview,
            upgrade_preview: || Ok(Vec::new()),
            run_streaming: stream_must_not_run,
        };
        let sess = ModalSession::default();
        let targets = names(&["oldpkg"]);
        std::thread::scope(|s| {
            s.spawn(|| remove_worker(&targets, &ops, &sess));
            wait_until("remove confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            let expected = format!(
                "{}\n\n{}: {}",
                archel_core::packages::confirm_remove_prompt(&targets),
                archel_core::packages::preview_removals_label(),
                "oldpkg"
            );
            assert_eq!(sess.snapshot_question(), expected);
            sess.resolve_confirm(false);
        });
        assert_eq!(STREAM_CALLS.load(AtomicOrdering::SeqCst), 0);
    }

    #[test]
    fn upgrade_with_empty_pending_reports_up_to_date_without_streaming() {
        static STREAM_CALLS: AtomicUsize = AtomicUsize::new(0);
        fn stream_must_not_run(
            _verb: &str,
            _packages: &[String],
            _label: &str,
            _on_line: &mut dyn FnMut(&str),
            _cancel: &AtomicBool,
        ) -> Result<CancellableStreaming, archel_core::ArchelError> {
            STREAM_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(CancellableStreaming {
                transcript: String::new(),
                cancelled: false,
                exit_code: Some(0),
            })
        }
        let ops = PkgWriteOps {
            install_preview: ok_install_preview,
            remove_preview: ok_remove_preview,
            upgrade_preview: || Ok(Vec::new()),
            run_streaming: stream_must_not_run,
        };
        let sess = ModalSession::default();
        upgrade_worker(&ops, &sess);
        assert_eq!(sess.snapshot_kind(), ModalKind::Confirm);
        assert_eq!(
            sess.snapshot_question(),
            archel_core::packages::up_to_date_line(),
            "empty pending is the honest core up-to-date line, nothing spawns"
        );
        assert_eq!(STREAM_CALLS.load(AtomicOrdering::SeqCst), 0);
    }

    #[test]
    fn upgrade_pending_streams_the_full_system_verb_with_no_packages() {
        static LAST: Mutex<(String, Vec<String>)> = Mutex::new((String::new(), Vec::new()));
        fn record_and_succeed(
            verb: &str,
            packages: &[String],
            _label: &str,
            on_line: &mut dyn FnMut(&str),
            cancel: &AtomicBool,
        ) -> Result<CancellableStreaming, archel_core::ArchelError> {
            *LAST.lock().expect("last lock") = (verb.to_string(), packages.to_vec());
            streaming_succeeded(verb, packages, _label, on_line, cancel)
        }
        let ops = PkgWriteOps {
            install_preview: ok_install_preview,
            remove_preview: ok_remove_preview,
            upgrade_preview: || Ok(vec!["helix 25.04 -> 25.07-1".to_string()]),
            run_streaming: record_and_succeed,
        };
        let sess = ModalSession::default();
        std::thread::scope(|s| {
            s.spawn(|| upgrade_worker(&ops, &sess));
            wait_until("upgrade confirm opens", || {
                sess.snapshot_kind() == ModalKind::Confirm
            });
            let expected = format!(
                "{}\n\n{}: {}",
                archel_core::packages::confirm_upgrade_prompt(),
                archel_core::packages::preview_pending_label(),
                "helix 25.04 -> 25.07-1"
            );
            assert_eq!(sess.snapshot_question(), expected);
            sess.resolve_confirm(true);
            wait_until("upgrade stream finishes", || sess.snapshot_done());
        });
        let (verb, packages) = LAST.lock().expect("last lock").clone();
        assert_eq!(verb, archel_core::privileged::PKG_UPGRADE_VERB);
        assert!(packages.is_empty(), "the upgrade verb carries no target list");
        assert_eq!(sess.snapshot_result(), EndState::Completed);
    }

    #[test]
    fn cancel_mid_stream_is_reported_cancelled_not_error() {
        fn fake_cancelled_stream(
            verb: &str,
            packages: &[String],
            _label: &str,
            on_line: &mut dyn FnMut(&str),
            cancel: &AtomicBool,
        ) -> Result<CancellableStreaming, archel_core::ArchelError> {
            on_line(":: downloading foo.pkg.tar.zst");
            // Behave like the core read loop: watch the shared flag.
            let deadline = Instant::now() + Duration::from_secs(5);
            while !cancel.load(AtomicOrdering::SeqCst) && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
            on_line(":: interrupted");
            let _ = verb;
            let _ = packages;
            Ok(CancellableStreaming {
                transcript: String::new(),
                cancelled: true,
                exit_code: None,
            })
        }
        let ops = PkgWriteOps {
            install_preview: ok_install_preview,
            remove_preview: ok_remove_preview,
            upgrade_preview: || Ok(Vec::new()),
            run_streaming: fake_cancelled_stream,
        };
        let sess = ModalSession::default();
        let targets = names(&["foo"]);
        std::thread::scope(|s| {
            s.spawn(|| install_worker(&targets, &ops, &sess));
            // The install path always confirms first - release the
            // waiter, otherwise the worker never reaches the stream.
            wait_until("confirm opens", || sess.snapshot_kind() == ModalKind::Confirm);
            sess.resolve_confirm(true);
            wait_until("stream opens", || {
                sess.snapshot_kind() == ModalKind::Stream && !sess.snapshot_done()
            });
            sess.request_cancel();
            wait_until("cancelled end state", || sess.snapshot_done());
        });
        // D-14: the stop is data - the modal stays a Stream, never an Error.
        assert_eq!(sess.snapshot_kind(), ModalKind::Stream);
        assert_eq!(sess.snapshot_result(), EndState::Cancelled);
        assert!(sess.snapshot_lines_joined().contains("interrupted"));
    }

    #[test]
    fn parse_names_json_trims_and_refuses_garbage() {
        assert_eq!(parse_names_json(r#"[" foo ","","bar" ]"#), names(&["foo", "bar"]));
        assert!(parse_names_json("not json").is_empty());
        assert!(parse_names_json("[]").is_empty());
        assert!(parse_names_json(r#""a string""#).is_empty());
        assert!(parse_names_json("[1, 2]").is_empty(), "non-string arrays are refused");
    }

    // -- AUR trust gate (D-19 / T-06-29) -------------------------------

    fn aur_review_fixture(name: &str, source: &str) -> archel_core::aur::AurReview {
        archel_core::aur::AurReview {
            card: PackageCard {
                name: name.to_string(),
                candidate_version: "1.2-3".to_string(),
                installed_version: None,
                source: source.to_string(),
                download_size: None,
                installed_size: None,
            },
            pkgbuild: format!("pkgname={name}\npkgver=1.2\n"),
            trust: archel_core::aur::TrustSignals {
                maintainer: Some("A Trusted Person".to_string()),
                votes: Some("42".to_string()),
                popularity: Some("1.37".to_string()),
                out_of_date: None,
            },
            dependencies: names(&["glibc", "openssl"]),
            warnings: names(&["flagged out of date"]),
        }
    }

    fn ok_review(name: &str) -> Result<archel_core::aur::AurReview, archel_core::ArchelError> {
        Ok(aur_review_fixture(name, "aur"))
    }

    fn ok_source(_name: &str, _text: &str) -> Result<Vec<String>, archel_core::ArchelError> {
        Ok(Vec::new())
    }

    #[allow(clippy::type_complexity)]
    fn aur_ops(
        validate_names: fn(&[String]) -> Result<(), archel_core::ArchelError>,
        review: fn(&str) -> Result<archel_core::aur::AurReview, archel_core::ArchelError>,
        validate_source: fn(
            &str,
            &str,
        ) -> Result<Vec<String>, archel_core::ArchelError>,
        build: fn(
            &str,
            &mut dyn FnMut(&str),
            &AtomicBool,
        ) -> Result<CancellableStreaming, archel_core::ArchelError>,
    ) -> PkgAurOps {
        PkgAurOps {
            validate_names,
            review,
            validate_source,
            build,
        }
    }

    fn session_in_review(name: &str) -> ModalSession {
        let sess = ModalSession::default();
        sess.begin_aur_review(name, "{}", &format!("pkgname={name}\npkgver=1.2\n"));
        sess
    }

    #[test]
    fn aur_gate_blocks_the_build_when_trust_is_checked_but_pkgbuild_unviewed() {
        static BUILD_CALLS: AtomicUsize = AtomicUsize::new(0);
        fn sneaky_build(
            _name: &str,
            _on_line: &mut dyn FnMut(&str),
            _cancel: &AtomicBool,
        ) -> Result<CancellableStreaming, archel_core::ArchelError> {
            BUILD_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(CancellableStreaming {
                transcript: String::new(),
                cancelled: false,
                exit_code: Some(0),
            })
        }
        let ops = aur_ops(|_| Ok(()), ok_review, ok_source, sneaky_build);
        let sess = session_in_review("cool-aur-thing");
        sess.set_trust_confirmed(true); // checkbox ticked without ever opening the PKGBUILD
        aur_build_worker(&sess, &ops);
        assert_eq!(BUILD_CALLS.load(AtomicOrdering::SeqCst), 0, "seen=false must block");
        assert!(!sess.snapshot_done());
    }

    #[test]
    fn aur_gate_blocks_the_build_when_pkgbuild_was_viewed_without_trust() {
        static BUILD_CALLS: AtomicUsize = AtomicUsize::new(0);
        fn sneaky_build(
            _name: &str,
            _on_line: &mut dyn FnMut(&str),
            _cancel: &AtomicBool,
        ) -> Result<CancellableStreaming, archel_core::ArchelError> {
            BUILD_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(CancellableStreaming {
                transcript: String::new(),
                cancelled: false,
                exit_code: Some(0),
            })
        }
        let ops = aur_ops(|_| Ok(()), ok_review, ok_source, sneaky_build);
        let sess = session_in_review("cool-aur-thing");
        sess.note_pkgbuild_seen(); // expander opened, trust checkbox left off
        aur_build_worker(&sess, &ops);
        assert_eq!(BUILD_CALLS.load(AtomicOrdering::SeqCst), 0, "trusted=false must block");
        assert!(!sess.snapshot_done());
    }

    #[test]
    fn aur_gate_opens_when_both_flags_are_set_before_the_run() {
        static BUILD_CALLS: AtomicUsize = AtomicUsize::new(0);
        fn real_build(
            _name: &str,
            on_line: &mut dyn FnMut(&str),
            _cancel: &AtomicBool,
        ) -> Result<CancellableStreaming, archel_core::ArchelError> {
            BUILD_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            on_line("==> Making package: cool-aur-thing 1.2-3");
            Ok(CancellableStreaming {
                transcript: "==> Making package: cool-aur-thing 1.2-3".to_string(),
                cancelled: false,
                exit_code: Some(0),
            })
        }
        let ops = aur_ops(|_| Ok(()), ok_review, ok_source, real_build);
        let sess = session_in_review("cool-aur-thing");
        sess.note_pkgbuild_seen();
        sess.set_trust_confirmed(true);
        // Both flags are armed BEFORE the worker runs: `open_stream`
        // inside it clears trust_confirmed, so a worker that read the
        // flag after the reset would block here. Passing proves the
        // snapshot-before-reset read that D-19 requires.
        aur_build_worker(&sess, &ops);
        assert_eq!(BUILD_CALLS.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(sess.snapshot_kind(), ModalKind::Stream);
        assert!(sess.snapshot_done());
        assert_eq!(sess.snapshot_result(), EndState::Completed);
        let lines = sess.snapshot_lines_joined();
        assert!(lines.contains("Making package: cool-aur-thing"));
        assert!(
            lines.contains(&archel_core::aur::aur_ok_line("cool-aur-thing")),
            "success appends core's ok line"
        );
    }

    #[test]
    fn aur_build_revalidates_the_shown_pkgbuild_before_running() {
        static BUILD_CALLS: AtomicUsize = AtomicUsize::new(0);
        fn sneaky_build(
            _name: &str,
            _on_line: &mut dyn FnMut(&str),
            _cancel: &AtomicBool,
        ) -> Result<CancellableStreaming, archel_core::ArchelError> {
            BUILD_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(CancellableStreaming {
                transcript: String::new(),
                cancelled: false,
                exit_code: Some(0),
            })
        }
        fn broken_source(
            _name: &str,
            _text: &str,
        ) -> Result<Vec<String>, archel_core::ArchelError> {
            Err(archel_core::ArchelError::Internal(
                "PKGBUILD rejected by bash -n".to_string(),
            ))
        }
        let ops = aur_ops(|_| Ok(()), ok_review, broken_source, sneaky_build);
        let sess = session_in_review("cool-aur-thing");
        sess.note_pkgbuild_seen();
        sess.set_trust_confirmed(true);
        aur_build_worker(&sess, &ops);
        assert_eq!(BUILD_CALLS.load(AtomicOrdering::SeqCst), 0);
        assert_eq!(sess.snapshot_kind(), ModalKind::Error);
        assert_eq!(
            sess.snapshot_error(),
            archel_core::ArchelError::Internal("PKGBUILD rejected by bash -n".to_string())
                .message()
        );
    }

    #[test]
    fn aur_review_publishes_summary_pkgbuild_and_trust_state() {
        let ops = aur_ops(|_| Ok(()), ok_review, ok_source, |_, _, _| {
            Err(archel_core::ArchelError::Internal(
                "must not build during review".to_string(),
            ))
        });
        let sess = ModalSession::default();
        aur_review_worker("cool-aur-thing", &ops, &sess);
        assert_eq!(sess.snapshot_kind(), ModalKind::Stream);
        assert!(sess.snapshot_trust_needed());
        assert_eq!(sess.snapshot_aur_name(), "cool-aur-thing");
        assert!(
            !sess.snapshot_pkgbuild_seen(),
            "the expander starts unopened: both latch flags are false at review time"
        );
        assert!(!sess.snapshot_trust_confirmed());
        assert!(sess.snapshot_pkgbuild().contains("pkgname=cool-aur-thing"));
        let summary = sess.snapshot_trust_summary();
        let labels = archel_core::aur::trust_labels();
        assert!(summary.contains("cool-aur-thing"));
        assert!(summary.contains("1.2-3"));
        assert!(summary.contains(&labels.maintainer));
        assert!(summary.contains(&labels.votes));
        assert!(summary.contains("A Trusted Person"));
        assert!(summary.contains("42"));
    }

    #[test]
    fn aur_review_redirects_repo_cards_with_core_hint() {
        fn repo_review(
            name: &str,
        ) -> Result<archel_core::aur::AurReview, archel_core::ArchelError> {
            Ok(aur_review_fixture(name, "extra"))
        }
        let ops = aur_ops(|_| Ok(()), repo_review, ok_source, |_, _, _| {
            Err(archel_core::ArchelError::Internal(
                "must not build for a repo card".to_string(),
            ))
        });
        let sess = ModalSession::default();
        aur_review_worker("firefox", &ops, &sess);
        assert_eq!(sess.snapshot_kind(), ModalKind::Error);
        assert_eq!(
            sess.snapshot_error(),
            archel_core::aur::not_aur_hint("firefox"),
            "core's own hint is reused verbatim, no trust block opens"
        );
        assert!(!sess.snapshot_trust_needed());
    }

    #[test]
    fn aur_review_refuses_invalid_names_before_fetching() {
        static REVIEW_CALLS: AtomicUsize = AtomicUsize::new(0);
        fn review_must_not_run(
            name: &str,
        ) -> Result<archel_core::aur::AurReview, archel_core::ArchelError> {
            REVIEW_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(aur_review_fixture(name, "aur"))
        }
        fn validate_rejects(names: &[String]) -> Result<(), archel_core::ArchelError> {
            Err(archel_core::ArchelError::PackageNotFound {
                name: names[0].clone(),
            })
        }
        let ops = aur_ops(validate_rejects, review_must_not_run, ok_source, |_, _, _| {
            Err(archel_core::ArchelError::Internal(
                "must not build".to_string(),
            ))
        });
        let sess = ModalSession::default();
        aur_review_worker("nope!", &ops, &sess);
        assert_eq!(sess.snapshot_kind(), ModalKind::Error);
        assert_eq!(REVIEW_CALLS.load(AtomicOrdering::SeqCst), 0, "nothing fetched");
    }
}
