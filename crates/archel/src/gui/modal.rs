//! Shared modal session for the write-capable shell (plan 06-06).
//!
//! The three root modals (Confirm/Error/Stream) are published by
//! `ShellModel`, but the workers that drive them live on other models
//! (`PkgModel` here, later plans reuse the same surface). A QObject can
//! only be touched on its own thread (RESEARCH Pattern 1), so the
//! cross-model handoff goes through this plain-Rust session object:
//! workers mutate only `Mutex`/`AtomicBool` state here, and
//! `ShellModel::sync_session` (running on the Qt thread via a QML
//! Timer) copies it into QML properties. Property writes stay on the
//! owning thread - the streaming line feed is Timer-polled, never
//! queued from a worker (same guarantee as the 06-03 PollGuard,
//! without needing a QObject handle on the worker side).
//!
//! All user-visible wording produced here is plain ASCII (D-07:
//! French copy lives in QML or the catalog, never in `.rs` files).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

/// Hard cap on streamed lines kept in memory (T-06-30: a runaway
/// child cannot grow the modal buffer without bound).
pub const MAX_STREAM_LINES: usize = 10_000;

/// Modal kinds, mirrored by `ShellModel.modal_kind` and the three
/// root modals in Main.qml.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModalKind {
    #[default]
    None,
    Confirm,
    Error,
    Stream,
}

impl ModalKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ModalKind::None => "none",
            ModalKind::Confirm => "confirm",
            ModalKind::Error => "error",
            ModalKind::Stream => "stream",
        }
    }
}

/// How a streamed transaction ended (D-14: the stop is data, not an
/// error - the modal reports the true end state, QML renders French).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndState {
    Completed,
    Cancelled,
    Failed,
}

impl EndState {
    /// ASCII key for the `modal_result` property; QML maps each key
    /// to its French sentence.
    pub fn as_key(self) -> &'static str {
        match self {
            EndState::Completed => "completed",
            EndState::Cancelled => "cancelled",
            EndState::Failed => "failed",
        }
    }
}

/// Maps the core cancellable outcome onto the modal end state
/// (D-14): a user-requested stop is `Cancelled` even when the
/// reaped exit code is non-zero - it is never collapsed into an
/// error. Only a non-cancelled non-zero exit is `Failed`.
pub fn end_state_for(cancelled: bool, exit_code: Option<i32>) -> EndState {
    if cancelled {
        EndState::Cancelled
    } else if exit_code == Some(0) {
        EndState::Completed
    } else {
        EndState::Failed
    }
}

/// D-19 gate: the AUR build call is reachable only when the PKGBUILD
/// expander has been opened at least once AND explicit trust was
/// given. Both flags are required - neither alone suffices.
// Consumed by the PkgModel AUR gate (aur_build_allowed) in Task 3 of
// this plan.
#[allow(dead_code)]
pub fn aur_gate_allows(pkgbuild_seen: bool, trust_confirmed: bool) -> bool {
    pkgbuild_seen && trust_confirmed
}

/// Appends one streamed line, enforcing the T-06-30 cap: past the
/// cap the oldest lines fall off and a single truncation notice
/// stays pinned at the head.
pub fn push_capped(lines: &mut Vec<String>, line: &str) {
    // Invariant: after any push_capped, lines.len() <= MAX_STREAM_LINES.
    // (The previous shape removed the head notice and re-inserted it before
    // an unconditional push, which let the buffer grow by one line per push
    // once the notice was pinned.)
    lines.push(line.to_string());
    if lines.len() > MAX_STREAM_LINES {
        let excess = lines.len() - MAX_STREAM_LINES;
        // Drop the oldest lines plus the notice slot, then pin the notice
        // at index 0 so the total lands exactly on the cap.
        lines.drain(0..excess + 1);
        lines.insert(0, "[truncated: oldest lines dropped]".to_string());
    }
}

/// One modal session. `cancel` is the exact handle handed to the
/// core cancellable helpers (`run_privileged_streaming_cancellable`,
/// `yay_build_cancellable`): `ShellModel::cancel_stream` sets it,
/// the core read loop observes it and performs its one-shot SIGINT +
/// drain-to-EOF path (D-14). The GUI never spawns, signals, or holds
/// a child pid - subprocess lifecycle is owned by core.
pub struct ModalSession {
    kind: Mutex<ModalKind>,
    question: Mutex<String>,
    lines: Mutex<Vec<String>>,
    error: Mutex<String>,
    done: Mutex<bool>,
    cancel: AtomicBool,
    trust_needed: Mutex<bool>,
    trust_summary: Mutex<String>,
    pkgbuild: Mutex<String>,
    pkgbuild_seen: Mutex<bool>,
    trust_confirmed: Mutex<bool>,
    aur_name: Mutex<String>,
    result: Mutex<EndState>,
    exit_code: Mutex<Option<i32>>,
    confirm_tx: Mutex<Option<std::sync::mpsc::Sender<bool>>>,
    // -- Bluetooth pairing session (plan 06-07, D-21) --
    //
    // Fully independent from the confirm/error/stream fields above:
    // the pairing modal has its own visibility (`pairing_active`),
    // so a worker `reset()` on the shared session (e.g. a concurrent
    // confirm from another tab) never tears down a pairing prompt.
    pairing_active: Mutex<bool>,
    pairing_address: Mutex<String>,
    pairing_name: Mutex<String>,
    pairing_mode: Mutex<String>,
    pairing_passkey: Mutex<String>,
    pairing_answer_tx: Mutex<Option<std::sync::mpsc::Sender<bool>>>,
}

impl Default for ModalSession {
    fn default() -> Self {
        Self {
            kind: Mutex::new(ModalKind::None),
            question: Mutex::new(String::new()),
            lines: Mutex::new(Vec::new()),
            error: Mutex::new(String::new()),
            done: Mutex::new(false),
            cancel: AtomicBool::new(false),
            trust_needed: Mutex::new(false),
            trust_summary: Mutex::new(String::new()),
            pkgbuild: Mutex::new(String::new()),
            pkgbuild_seen: Mutex::new(false),
            trust_confirmed: Mutex::new(false),
            aur_name: Mutex::new(String::new()),
            result: Mutex::new(EndState::Completed),
            exit_code: Mutex::new(None),
            confirm_tx: Mutex::new(None),
            pairing_active: Mutex::new(false),
            pairing_address: Mutex::new(String::new()),
            pairing_name: Mutex::new(String::new()),
            pairing_mode: Mutex::new(String::new()),
            pairing_passkey: Mutex::new(String::new()),
            pairing_answer_tx: Mutex::new(None),
        }
    }
}

// Some of this surface (request_confirm, open_stream, push_line,
// finish_stream, cancel_handle) is consumed by the PkgModel workers that
// land in Tasks 2-3 of the same plan; until then the lint would fire on
// an otherwise-tested object.
#[allow(dead_code)]
impl ModalSession {
    /// Opens a confirmation and returns the receiver the worker
    /// blocks on. QML resolves it through `confirm_accepted` /
    /// `confirm_declined`. Any previous session is reset first.
    pub fn request_confirm(&self, question: &str) -> std::sync::mpsc::Receiver<bool> {
        self.reset();
        *self.question.lock().expect("question lock") = question.to_string();
        *self.kind.lock().expect("kind lock") = ModalKind::Confirm;
        let (tx, rx) = std::sync::mpsc::channel();
        *self.confirm_tx.lock().expect("confirm lock") = Some(tx);
        rx
    }

    /// Fire-and-forget publish used by the `openConfirm` invokable
    /// (QML-driven flows with no waiting worker).
    pub fn open_confirm(&self, question: &str) {
        let _ = self.request_confirm(question);
    }

    /// Resolves a pending confirmation. No waiter (direct QML use)
    /// makes this a harmless no-op.
    pub fn resolve_confirm(&self, accepted: bool) {
        if let Some(tx) = self.confirm_tx.lock().expect("confirm lock").take() {
            let _ = tx.send(accepted);
        }
        *self.kind.lock().expect("kind lock") = ModalKind::None;
    }

    /// Opens the error modal with core-produced readable text
    /// (`ArchelError::message()` - never raw stderr).
    pub fn open_error(&self, message: &str) {
        self.reset();
        *self.error.lock().expect("error lock") = message.to_string();
        *self.kind.lock().expect("kind lock") = ModalKind::Error;
    }

    /// Opens a streaming session: lines cleared, done false, cancel
    /// latch cleared for the new run.
    pub fn open_stream(&self) {
        self.reset();
        *self.kind.lock().expect("kind lock") = ModalKind::Stream;
    }

    /// Worker-side line feed (Mutex only - safe off the Qt thread).
    pub fn push_line(&self, line: &str) {
        push_capped(&mut self.lines.lock().expect("lines lock"), line);
    }

    /// Marks the stream finished with its D-14 end state.
    pub fn finish_stream(&self, cancelled: bool, exit_code: Option<i32>) {
        *self.result.lock().expect("result lock") = end_state_for(cancelled, exit_code);
        *self.exit_code.lock().expect("exit lock") = exit_code;
        *self.done.lock().expect("done lock") = true;
    }

    /// Closes whatever modal is open (Fermer buttons, post-resolve).
    pub fn close(&self) {
        *self.kind.lock().expect("kind lock") = ModalKind::None;
    }

    /// Full reset between sessions. `pkgbuild_seen` is deliberately
    /// NOT cleared here: the D-19 latch is one-way per package
    /// review and is cleared only when a new review starts
    /// (`begin_aur_review`), so a mid-session modal reopen cannot
    /// reopen the trust hole.
    fn reset(&self) {
        *self.question.lock().expect("question lock") = String::new();
        self.lines.lock().expect("lines lock").clear();
        *self.error.lock().expect("error lock") = String::new();
        *self.done.lock().expect("done lock") = false;
        self.cancel.store(false, Ordering::SeqCst);
        *self.trust_needed.lock().expect("trust lock") = false;
        *self.trust_summary.lock().expect("trust lock") = String::new();
        *self.pkgbuild.lock().expect("pkgbuild lock") = String::new();
        *self.trust_confirmed.lock().expect("trust lock") = false;
        *self.aur_name.lock().expect("aur lock") = String::new();
        *self.result.lock().expect("result lock") = EndState::Completed;
        *self.exit_code.lock().expect("exit lock") = None;
        *self.confirm_tx.lock().expect("confirm lock") = None;
    }

    /// Starts a new AUR review: the only place that clears the
    /// one-way `pkgbuild_seen` latch (a new package needs a new
    /// viewing).
    pub fn begin_aur_review(&self, name: &str, summary_json: &str, pkgbuild: &str) {
        self.reset();
        *self.pkgbuild_seen.lock().expect("seen lock") = false;
        *self.aur_name.lock().expect("aur lock") = name.to_string();
        *self.trust_summary.lock().expect("trust lock") = summary_json.to_string();
        *self.pkgbuild.lock().expect("pkgbuild lock") = pkgbuild.to_string();
        *self.trust_needed.lock().expect("trust lock") = true;
        *self.kind.lock().expect("kind lock") = ModalKind::Stream;
    }

    /// One-way D-19 latch: once the PKGBUILD expander has been
    /// opened it stays open - collapsing the expander never resets
    /// it (there is no `false` path by design).
    pub fn note_pkgbuild_seen(&self) {
        *self.pkgbuild_seen.lock().expect("seen lock") = true;
    }

    pub fn set_trust_confirmed(&self, confirmed: bool) {
        *self.trust_confirmed.lock().expect("trust lock") = confirmed;
    }

    /// Annuler on the StreamModal routes here. During the AUR review
    /// stage (trust block showing, no child running) the true "cancel"
    /// is dismissing the review - and the trust checkbox is cleared as
    /// a D-19 fail-safe, so a discarded review can never leave both
    /// gate flags set for a later stray build call. During a live
    /// stream it is only the flag the core read loop observes (D-14):
    /// the GUI never signals a child itself.
    pub fn dismiss_or_cancel(&self) {
        if self.snapshot_trust_needed() && !self.snapshot_done() {
            self.set_trust_confirmed(false);
            self.close();
        } else {
            self.request_cancel();
        }
    }

    /// Shared cancel handle for the core cancellable helpers.
    pub fn cancel_handle(&self) -> &AtomicBool {
        &self.cancel
    }

    /// Annuler path: sets the flag the core read loop observes.
    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    // -- Bluetooth pairing session (plan 06-07, D-21) --
    //
    // The global PairingModal (Main.qml root, the ONLY pairing UI)
    // renders this state. Workers drive it from the blocking pool;
    // `ShellModel::sync_session` mirrors it onto QML properties; the
    // modal's Confirmer/Refuser resolve the pending answer through
    // `resolve_pairing`. Never auto-accept: no waiter resolves to
    // `true` - only an explicit Confirmer does.

    /// Opens the pairing session for one device. `mode` is the BlueZ
    /// association model the core agent reports (`yesno` for the
    /// numeric-comparison path - the code shows plus an explicit
    /// match question). Any previous pairing answer is dropped.
    pub fn begin_pairing(&self, address: &str, name: &str, mode: &str) {
        *self.pairing_address.lock().expect("pairing lock") = address.to_string();
        *self.pairing_name.lock().expect("pairing lock") = name.to_string();
        *self.pairing_mode.lock().expect("pairing lock") = mode.to_string();
        *self.pairing_passkey.lock().expect("pairing lock") = String::new();
        *self.pairing_answer_tx.lock().expect("pairing lock") = None;
        *self.pairing_active.lock().expect("pairing lock") = true;
    }

    /// Publishes a BlueZ code and returns the receiver the agent
    /// callback blocks on. The code shown is always the core
    /// `format_passkey` output - this function stores exactly what it
    /// is given, and the only caller is the GUI pair worker fed by
    /// the core agent (thin-view contract).
    pub fn pairing_request_code(&self, code: &str) -> std::sync::mpsc::Receiver<bool> {
        *self.pairing_passkey.lock().expect("pairing lock") = code.to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        *self.pairing_answer_tx.lock().expect("pairing lock") = Some(tx);
        rx
    }

    /// Resolves a pending pairing answer. Confirmer releases the
    /// waiter with `true`; Refuser with `false`. No waiter (direct
    /// QML use) makes this a harmless no-op that never accepts.
    pub fn resolve_pairing(&self, accepted: bool) {
        if let Some(tx) = self.pairing_answer_tx.lock().expect("pairing lock").take() {
            let _ = tx.send(accepted);
        }
        if !accepted {
            *self.pairing_active.lock().expect("pairing lock") = false;
        }
    }

    /// Closes the pairing session after the handshake ends (paired,
    /// rejected, timed out or declined). Clears the code and drops
    /// any unanswered waiter (a late Confirmer after the close is a
    /// no-op, never a stray accept).
    pub fn end_pairing(&self) {
        *self.pairing_answer_tx.lock().expect("pairing lock") = None;
        *self.pairing_passkey.lock().expect("pairing lock") = String::new();
        *self.pairing_active.lock().expect("pairing lock") = false;
    }

    // -- Qt-thread snapshot getters (used by sync_session) --

    pub fn snapshot_kind(&self) -> ModalKind {
        *self.kind.lock().expect("kind lock")
    }
    pub fn snapshot_question(&self) -> String {
        self.question.lock().expect("question lock").clone()
    }
    pub fn snapshot_lines_joined(&self) -> String {
        self.lines.lock().expect("lines lock").join("\n")
    }
    pub fn snapshot_line_count(&self) -> usize {
        self.lines.lock().expect("lines lock").len()
    }
    pub fn snapshot_error(&self) -> String {
        self.error.lock().expect("error lock").clone()
    }
    pub fn snapshot_done(&self) -> bool {
        *self.done.lock().expect("done lock")
    }
    pub fn snapshot_trust_needed(&self) -> bool {
        *self.trust_needed.lock().expect("trust lock")
    }
    pub fn snapshot_trust_summary(&self) -> String {
        self.trust_summary.lock().expect("trust lock").clone()
    }
    pub fn snapshot_pkgbuild(&self) -> String {
        self.pkgbuild.lock().expect("pkgbuild lock").clone()
    }
    pub fn snapshot_pkgbuild_seen(&self) -> bool {
        *self.pkgbuild_seen.lock().expect("seen lock")
    }
    pub fn snapshot_trust_confirmed(&self) -> bool {
        *self.trust_confirmed.lock().expect("trust lock")
    }
    pub fn snapshot_aur_name(&self) -> String {
        self.aur_name.lock().expect("aur lock").clone()
    }
    pub fn snapshot_result(&self) -> EndState {
        *self.result.lock().expect("result lock")
    }
    pub fn snapshot_exit_code(&self) -> Option<i32> {
        *self.exit_code.lock().expect("exit lock")
    }

    // -- Pairing snapshot getters (used by sync_session) --

    pub fn snapshot_pairing_active(&self) -> bool {
        *self.pairing_active.lock().expect("pairing lock")
    }
    pub fn snapshot_pairing_address(&self) -> String {
        self.pairing_address.lock().expect("pairing lock").clone()
    }
    pub fn snapshot_pairing_name(&self) -> String {
        self.pairing_name.lock().expect("pairing lock").clone()
    }
    pub fn snapshot_pairing_mode(&self) -> String {
        self.pairing_mode.lock().expect("pairing lock").clone()
    }
    pub fn snapshot_pairing_passkey(&self) -> String {
        self.pairing_passkey.lock().expect("pairing lock").clone()
    }
}

static SESSION: OnceLock<ModalSession> = OnceLock::new();

/// The process-wide modal session shared by ShellModel (Qt thread
/// publisher) and the tab-model workers (Mutex-only producers).
pub fn session() -> &'static ModalSession {
    SESSION.get_or_init(ModalSession::default)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> ModalSession {
        ModalSession::default()
    }

    #[test]
    fn stream_round_trip_open_lines_close() {
        let session = fresh();
        session.open_stream();
        assert_eq!(session.snapshot_kind(), ModalKind::Stream);
        assert!(!session.snapshot_done());
        session.push_line(":: installing foo...");
        session.push_line(":: done");
        assert_eq!(session.snapshot_line_count(), 2);
        assert!(session.snapshot_lines_joined().contains("installing foo"));
        session.finish_stream(false, Some(0));
        assert!(session.snapshot_done());
        assert_eq!(session.snapshot_result(), EndState::Completed);
        session.close();
        assert_eq!(session.snapshot_kind(), ModalKind::None);
    }

    #[test]
    fn cancel_flag_is_observable_by_the_core_loop() {
        let session = fresh();
        session.open_stream();
        assert!(!session.is_cancelled());
        session.request_cancel();
        assert!(session.is_cancelled());
        session.finish_stream(true, None);
        assert_eq!(session.snapshot_result(), EndState::Cancelled);
    }

    #[test]
    fn confirm_round_trip_resolves_the_waiter() {
        let session = fresh();
        let rx = session.request_confirm("Installer foo ?");
        assert_eq!(session.snapshot_kind(), ModalKind::Confirm);
        assert_eq!(session.snapshot_question(), "Installer foo ?");
        session.resolve_confirm(true);
        assert!(rx.recv().expect("waiter resolves"));
        assert_eq!(session.snapshot_kind(), ModalKind::None);
    }

    #[test]
    fn confirm_decline_resolves_false() {
        let session = fresh();
        let rx = session.request_confirm("Supprimer bar ?");
        session.resolve_confirm(false);
        assert!(!rx.recv().expect("waiter resolves"));
    }

    #[test]
    fn error_session_carries_readable_text() {
        let session = fresh();
        session.open_error("Impossible d obtenir les privileges.");
        assert_eq!(session.snapshot_kind(), ModalKind::Error);
        assert!(session.snapshot_error().contains("privileges"));
    }

    #[test]
    fn stream_cap_pins_a_truncation_notice() {
        let mut lines = Vec::new();
        for i in 0..(MAX_STREAM_LINES + 5) {
            push_capped(&mut lines, &format!("line {i}"));
        }
        assert_eq!(
            lines.len(),
            MAX_STREAM_LINES,
            "the cap is exact, not approximate"
        );
        assert!(
            lines[0].starts_with("[truncated"),
            "oldest lines fall off behind a notice"
        );
    }

    #[test]
    fn end_state_never_collapses_cancel_into_error() {
        assert_eq!(end_state_for(true, Some(1)), EndState::Cancelled);
        assert_eq!(end_state_for(true, None), EndState::Cancelled);
        assert_eq!(end_state_for(false, Some(0)), EndState::Completed);
        assert_eq!(end_state_for(false, Some(1)), EndState::Failed);
        assert_eq!(end_state_for(false, None), EndState::Failed);
    }

    #[test]
    fn aur_gate_needs_both_flags() {
        assert!(!aur_gate_allows(false, false));
        assert!(!aur_gate_allows(false, true), "trust alone never opens D-19");
        assert!(!aur_gate_allows(true, false), "viewing alone never opens D-19");
        assert!(aur_gate_allows(true, true));
    }

    #[test]
    fn pkgbuild_seen_is_a_one_way_latch() {
        let session = fresh();
        session.begin_aur_review("foo", "{}", "pkgname=foo");
        assert!(!session.snapshot_pkgbuild_seen());
        session.note_pkgbuild_seen();
        assert!(session.snapshot_pkgbuild_seen());
        // A mid-session reopen must not clear it: reset() deliberately
        // skips the latch, so it survives without re-noting.
        session.open_stream();
        assert!(session.snapshot_pkgbuild_seen());
        // Only a NEW review for another package clears it.
        session.begin_aur_review("bar", "{}", "pkgname=bar");
        assert!(!session.snapshot_pkgbuild_seen());
        assert_eq!(session.snapshot_aur_name(), "bar");
    }

    #[test]
    fn trust_block_fields_survive_into_the_session() {
        let session = fresh();
        session.begin_aur_review("foo", "{\"version\":\"1.0\"}", "pkgname=foo");
        assert!(session.snapshot_trust_needed());
        assert!(session.snapshot_trust_summary().contains("1.0"));
        assert_eq!(session.snapshot_pkgbuild(), "pkgname=foo");
        session.set_trust_confirmed(true);
        assert!(session.snapshot_trust_confirmed());
        assert!(!aur_gate_allows(
            session.snapshot_pkgbuild_seen(),
            session.snapshot_trust_confirmed()
        ));
        session.note_pkgbuild_seen();
        assert!(aur_gate_allows(
            session.snapshot_pkgbuild_seen(),
            session.snapshot_trust_confirmed()
        ));
    }

    #[test]
    fn annuler_during_the_review_dismisses_and_clears_trust() {
        let session = fresh();
        session.begin_aur_review("foo", "{}", "pkgname=foo");
        session.note_pkgbuild_seen();
        session.set_trust_confirmed(true);
        session.dismiss_or_cancel();
        assert_eq!(session.snapshot_kind(), ModalKind::None);
        assert!(
            !session.snapshot_trust_confirmed(),
            "a discarded review must not leave the trust flag armed (D-19 fail-safe)"
        );
        // The one-way viewing latch survives the dismissal: a stray
        // build call still needs the trust that was just cleared.
        assert!(session.snapshot_pkgbuild_seen());
        assert!(!session.is_cancelled());
    }

    #[test]
    fn annuler_during_a_live_stream_only_sets_the_flag() {
        let session = fresh();
        session.open_stream();
        session.dismiss_or_cancel();
        assert!(session.is_cancelled());
        assert_eq!(
            session.snapshot_kind(),
            ModalKind::Stream,
            "the running stream stays open until the worker reports the end state"
        );
    }

    #[test]
    fn pairing_session_carries_device_code_and_mode() {
        let session = fresh();
        assert!(!session.snapshot_pairing_active());
        session.begin_pairing("AA:BB:CC:DD:EE:FF", "Casque Salon", "yesno");
        assert!(session.snapshot_pairing_active());
        assert_eq!(session.snapshot_pairing_address(), "AA:BB:CC:DD:EE:FF");
        assert_eq!(session.snapshot_pairing_name(), "Casque Salon");
        assert_eq!(session.snapshot_pairing_mode(), "yesno");
        assert_eq!(session.snapshot_pairing_passkey(), "");
        let rx = session.pairing_request_code("000042");
        assert_eq!(session.snapshot_pairing_passkey(), "000042");
        session.resolve_pairing(true);
        assert!(rx.recv().expect("waiter resolves"));
        session.end_pairing();
        assert!(!session.snapshot_pairing_active());
        assert_eq!(session.snapshot_pairing_passkey(), "");
    }

    #[test]
    fn pairing_decline_never_accepts_and_late_answers_are_noops() {
        let session = fresh();
        session.begin_pairing("AA:BB:CC:DD:EE:FF", "Casque Salon", "yesno");
        let rx = session.pairing_request_code("123456");
        session.resolve_pairing(false);
        assert!(!rx.recv().expect("waiter resolves"));
        assert!(!session.snapshot_pairing_active());
        // A Confirmer arriving after the close has no waiter to
        // release and changes nothing (never a stray accept).
        session.resolve_pairing(true);
        assert!(!session.snapshot_pairing_active());
    }
}
