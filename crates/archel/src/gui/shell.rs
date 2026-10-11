//! Rust side of the shell bridge: the `ShellModel` QObject Main.qml
//! binds its frame to (D-23).
//!
//! Split from `bridge.rs` (which keeps the retired spike model) because
//! cxx-qt-build 0.10 accepts a single `cxx_qt` module per file, and a
//! single `extern "RustQt"` block accepts a single qobject for `Self`
//! inlining. Same declaration pattern as the spike bridge: shape in
//! `#[cxx_qt::bridge]`, behavior on the generated type. All user-visible
//! wording produced here is plain ASCII (D-07: French diacritics live in
//! the catalog or in QML, never in `.rs` files).

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(i32, launch_x)]
        #[qproperty(i32, launch_y)]
        #[qproperty(i32, launch_width)]
        #[qproperty(i32, launch_height)]
        #[qproperty(bool, launch_maximized)]
        #[qproperty(QString, modal_kind)]
        #[qproperty(QString, modal_question)]
        #[qproperty(QString, modal_question_top)]
        #[qproperty(i32, modal_confirm_count)]
        #[qproperty(QString, modal_lines)]
        #[qproperty(QString, modal_error)]
        #[qproperty(bool, modal_done)]
        #[qproperty(QString, modal_result)]
        #[qproperty(bool, modal_trust_needed)]
        #[qproperty(QString, modal_trust_summary)]
        #[qproperty(QString, modal_pkgbuild)]
        #[qproperty(bool, modal_pkgbuild_seen)]
        #[qproperty(bool, modal_trust_confirmed)]
        #[qproperty(bool, pairing_active)]
        #[qproperty(QString, pairing_address)]
        #[qproperty(QString, pairing_device_name)]
        #[qproperty(QString, pairing_mode)]
        #[qproperty(QString, pairing_passkey)]
        #[namespace = "archel_settings"]
        type ShellModel = super::ShellModelRust;

        #[qinvokable]
        #[cxx_name = "validateStartup"]
        fn validate_startup(self: Pin<&mut Self>, screens_json: QString);

        #[qinvokable]
        #[cxx_name = "saveGeometry"]
        fn save_geometry(
            self: Pin<&mut Self>,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            maximized: bool,
        );

        // -- Modal session surface (plan 06-06) --
        //
        // Workers never touch this QObject: they mutate only the shared
        // modal::ModalSession (Mutexes), and the QML Timer polls it
        // through syncSession on the owning thread (RESEARCH Pattern 1).
        // The command invokables below likewise only write the session;
        // visibility follows on the next poll.

        #[qinvokable]
        #[cxx_name = "syncSession"]
        fn sync_session(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "openConfirm"]
        fn open_confirm(self: Pin<&mut Self>, question: QString);

        #[qinvokable]
        #[cxx_name = "openError"]
        fn open_error(self: Pin<&mut Self>, message: QString);

        #[qinvokable]
        #[cxx_name = "closeModal"]
        fn close_modal(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "cancelStream"]
        fn cancel_stream(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "confirmAccepted"]
        fn confirm_accepted(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "confirmDeclined"]
        fn confirm_declined(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "confirmTopAccepted"]
        fn confirm_top_accepted(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "confirmTopDeclined"]
        fn confirm_top_declined(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "notePkgbuildSeen"]
        fn note_pkgbuild_seen(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "confirmTrust"]
        fn confirm_trust(self: Pin<&mut Self>, confirmed: bool);

        #[qinvokable]
        #[cxx_name = "pairingAccepted"]
        fn pairing_accepted(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "pairingDeclined"]
        fn pairing_declined(self: Pin<&mut Self>);
    }
}

use cxx_qt_lib::QString;

use super::geometry;
use super::modal::{self, ModalSession};

/// Last snapshot published to the modal QML properties. `sync_session`
/// polls the shared session, compares against this mirror, and writes a
/// full property set only when something actually changed - the same
/// set-only-on-change discipline as the 06-03 PollGuard path, so idle
/// polls emit zero change signals.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ModalSnapshot {
    kind: String,
    question: String,
    question_top: String,
    confirm_count: usize,
    lines: String,
    error: String,
    done: bool,
    result: String,
    trust_needed: bool,
    trust_summary: String,
    pkgbuild: String,
    pkgbuild_seen: bool,
    trust_confirmed: bool,
    pairing: PairingSnapshot,
}

impl ModalSnapshot {
    fn from_session(session: &ModalSession) -> Self {
        Self {
            kind: session.snapshot_kind().as_str().to_string(),
            question: session.snapshot_question(),
            question_top: session.snapshot_question_top(),
            confirm_count: session.snapshot_confirm_count(),
            lines: session.snapshot_lines_joined(),
            error: session.snapshot_error(),
            done: session.snapshot_done(),
            result: session.snapshot_result().as_key().to_string(),
            trust_needed: session.snapshot_trust_needed(),
            trust_summary: session.snapshot_trust_summary(),
            pkgbuild: session.snapshot_pkgbuild(),
            pkgbuild_seen: session.snapshot_pkgbuild_seen(),
            trust_confirmed: session.snapshot_trust_confirmed(),
            pairing: PairingSnapshot::from_session(session),
        }
    }
}

/// Pairing half of the mirrored session: compared and published
/// together with the modal half, so one `sync_session` poll carries
/// both (the PairingModal never sets its own visibility).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct PairingSnapshot {
    active: bool,
    address: String,
    device_name: String,
    mode: String,
    passkey: String,
}

impl PairingSnapshot {
    fn from_session(session: &ModalSession) -> Self {
        Self {
            active: session.snapshot_pairing_active(),
            address: session.snapshot_pairing_address(),
            device_name: session.snapshot_pairing_name(),
            mode: session.snapshot_pairing_mode(),
            passkey: session.snapshot_pairing_passkey(),
        }
    }
}

/// Backing state: the validated frame Main.qml launches with. The defaults
/// match the QML defaults so the window is correct even if
/// `validate_startup` never runs.
pub struct ShellModelRust {
    launch_x: i32,
    launch_y: i32,
    launch_width: i32,
    launch_height: i32,
    launch_maximized: bool,
    // Modal property fields (names must match the qproperty declarations).
    modal_kind: QString,
    modal_question: QString,
    modal_question_top: QString,
    modal_confirm_count: i32,
    modal_lines: QString,
    modal_error: QString,
    modal_done: bool,
    modal_result: QString,
    modal_trust_needed: bool,
    modal_trust_summary: QString,
    modal_pkgbuild: QString,
    modal_pkgbuild_seen: bool,
    modal_trust_confirmed: bool,
    pairing_active: bool,
    pairing_address: QString,
    pairing_device_name: QString,
    pairing_mode: QString,
    pairing_passkey: QString,
    /// Change-detector for `sync_session` (never read from QML).
    mirror: ModalSnapshot,
}

impl Default for ShellModelRust {
    fn default() -> Self {
        Self {
            launch_x: 0,
            launch_y: 0,
            launch_width: geometry::DEFAULT_WIDTH,
            launch_height: geometry::DEFAULT_HEIGHT,
            launch_maximized: false,
            modal_kind: QString::from(""),
            modal_question: QString::from(""),
            modal_question_top: QString::from(""),
            modal_confirm_count: 0,
            modal_lines: QString::from(""),
            modal_error: QString::from(""),
            modal_done: false,
            modal_result: QString::from(""),
            modal_trust_needed: false,
            modal_trust_summary: QString::from(""),
            modal_pkgbuild: QString::from(""),
            modal_pkgbuild_seen: false,
            modal_trust_confirmed: false,
            pairing_active: false,
            pairing_address: QString::from(""),
            pairing_device_name: QString::from(""),
            pairing_mode: QString::from(""),
            pairing_passkey: QString::from(""),
            mirror: ModalSnapshot::default(),
        }
    }
}

impl qobject::ShellModel {
    /// Reads the frame stashed by `run_settings()`, validates it against
    /// the screens QML reported (`Qt.application.screens` marshalled as
    /// JSON - cxx-qt-lib 0.10 has no QScreen), and publishes the
    /// placement. A garbage screens string degrades to "no screens":
    /// centered default.
    pub fn validate_startup(mut self: core::pin::Pin<&mut Self>, screens_json: QString) {
        let screens: Vec<geometry::ScreenRect> =
            serde_json::from_str(&screens_json.to_string()).unwrap_or_default();
        let placed = geometry::place(geometry::STARTUP.get().copied().flatten(), &screens);
        self.as_mut().set_launch_x(placed.x);
        self.as_mut().set_launch_y(placed.y);
        self.as_mut().set_launch_width(placed.width);
        self.as_mut().set_launch_height(placed.height);
        self.as_mut().set_launch_maximized(placed.maximized);
    }

    /// Called from `ApplicationWindow.onClosing` with the live window
    /// props; persistence failures are swallowed inside `geometry::save`.
    pub fn save_geometry(
        self: core::pin::Pin<&mut Self>,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        maximized: bool,
    ) {
        geometry::save(
            &geometry::path(),
            geometry::Geometry {
                x,
                y,
                width,
                height,
                maximized,
            },
        );
    }

    /// Polls the shared modal session and republishes its snapshot to the
    /// QML properties (Main.qml drives this from a 120 ms Timer). This is
    /// the only producer of the modal properties and it always runs on the
    /// owning Qt thread; worker lines arrive via Mutex, never via queued
    /// property writes (RESEARCH Pattern 1).
    pub fn sync_session(mut self: core::pin::Pin<&mut Self>) {
        use cxx_qt::CxxQtType;
        let next = ModalSnapshot::from_session(modal::session());
        let changed = {
            let mut mirror = self.as_mut().rust_mut();
            if mirror.mirror == next {
                false
            } else {
                mirror.mirror = next.clone();
                true
            }
        };
        if !changed {
            return;
        }
        self.as_mut().set_modal_kind(QString::from(&next.kind));
        self.as_mut()
            .set_modal_question(QString::from(&next.question));
        self.as_mut()
            .set_modal_question_top(QString::from(&next.question_top));
        self.as_mut()
            .set_modal_confirm_count(next.confirm_count as i32);
        self.as_mut()
            .set_modal_lines(QString::from(&next.lines));
        self.as_mut()
            .set_modal_error(QString::from(&next.error));
        self.as_mut().set_modal_done(next.done);
        self.as_mut()
            .set_modal_result(QString::from(&next.result));
        self.as_mut()
            .set_modal_trust_needed(next.trust_needed);
        self.as_mut()
            .set_modal_trust_summary(QString::from(&next.trust_summary));
        self.as_mut()
            .set_modal_pkgbuild(QString::from(&next.pkgbuild));
        self.as_mut()
            .set_modal_pkgbuild_seen(next.pkgbuild_seen);
        self.as_mut()
            .set_modal_trust_confirmed(next.trust_confirmed);
        self.as_mut().set_pairing_active(next.pairing.active);
        self.as_mut()
            .set_pairing_address(QString::from(&next.pairing.address));
        self.as_mut()
            .set_pairing_device_name(QString::from(&next.pairing.device_name));
        self.as_mut()
            .set_pairing_mode(QString::from(&next.pairing.mode));
        self.as_mut()
            .set_pairing_passkey(QString::from(&next.pairing.passkey));
    }

    // -- Command invokables: session writers only --
    //
    // Each mutates the shared session and returns; the visible property
    // state follows on the next syncSession poll, which keeps every
    // property write on the owning thread and every command idempotent
    // against a concurrent worker.

    /// Fire-and-forget confirm for QML-driven flows. Worker-driven
    /// confirms go through `ModalSession::request_confirm` directly.
    pub fn open_confirm(self: core::pin::Pin<&mut Self>, question: QString) {
        modal::session().open_confirm(&question.to_string());
    }

    /// Publishes a core-produced readable error (`ArchelError::message()`,
    /// never raw stderr) in the error modal.
    pub fn open_error(self: core::pin::Pin<&mut Self>, message: QString) {
        modal::session().open_error(&message.to_string());
    }

    /// Fermer on any open modal.
    pub fn close_modal(self: core::pin::Pin<&mut Self>) {
        modal::session().close();
    }

    /// Annuler: dismisses an AUR review (clearing its trust, D-19
    /// fail-safe) or, for a live stream, sets the flag the core read
    /// loop observes (D-14). The GUI never signals a child itself.
    pub fn cancel_stream(self: core::pin::Pin<&mut Self>) {
        modal::session().dismiss_or_cancel();
    }

    /// Confirmer: releases a waiting worker with `true`.
    pub fn confirm_accepted(self: core::pin::Pin<&mut Self>) {
        modal::session().resolve_confirm(true);
    }

    /// Fermer on the confirm modal: releases a waiting worker with
    /// `false` (decline is a first-class answer, not an error).
    pub fn confirm_declined(self: core::pin::Pin<&mut Self>) {
        modal::session().resolve_confirm(false);
    }

    /// Confirmer on the stacked top modal: releases the newest waiter
    /// with `true`. Resolution is top-only (D-08), so the base-modal
    /// `confirm_accepted` and this variant share the same session
    /// path; the separate call site documents which modal resolved.
    pub fn confirm_top_accepted(self: core::pin::Pin<&mut Self>) {
        modal::session().resolve_confirm(true);
    }

    /// Fermer on the stacked top modal: releases the newest waiter
    /// with `false`, leaving older pending confirms stacked (D-08).
    pub fn confirm_top_declined(self: core::pin::Pin<&mut Self>) {
        modal::session().resolve_confirm(false);
    }

    /// One-way D-19 latch: called when the PKGBUILD expander is first
    /// opened; collapsing it never un-notes the viewing.
    pub fn note_pkgbuild_seen(self: core::pin::Pin<&mut Self>) {
        modal::session().note_pkgbuild_seen();
    }

    /// Trust checkbox state for the AUR review block.
    pub fn confirm_trust(self: core::pin::Pin<&mut Self>, confirmed: bool) {
        modal::session().set_trust_confirmed(confirmed);
    }

    /// PairingModal Confirmer: releases a waiting agent callback with
    /// `true` (the ONLY path that accepts a pairing - never
    /// auto-accept, D-21).
    pub fn pairing_accepted(self: core::pin::Pin<&mut Self>) {
        modal::session().resolve_pairing(true);
    }

    /// PairingModal Refuser: releases a waiting agent callback with
    /// `false` (decline is a first-class answer - the worker then
    /// removes any half-bond and reports nothing).
    pub fn pairing_declined(self: core::pin::Pin<&mut Self>) {
        modal::session().resolve_pairing(false);
    }
}
