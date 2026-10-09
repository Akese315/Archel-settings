---
phase: 06-gui-shell-tabs
plan: 06
type: execute
wave: 5
depends_on: [06-02, 06-05]
files_modified:
  - crates/archel/src/gui/bridge.rs
  - crates/archel/src/gui/models/pkg.rs
  - crates/archel/src/gui/qml/components/ConfirmModal.qml
  - crates/archel/src/gui/qml/components/ErrorModal.qml
  - crates/archel/src/gui/qml/components/StreamModal.qml
  - crates/archel/src/gui/qml/Main.qml
  - crates/archel/src/gui/qml/tabs/PkgTab.qml
autonomous: true
requirements: [PLAT-03]
tags: [modals, write, packages, aur, privilege]

estimate:
  tokens: 98000
  raw_tokens: 70000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "Every write action routes through a modal or inline state: confirmation via ConfirmModal with the core's exact question verbatim (catalog template rendered with real values — D-12), progress via spinner + disabled controls (D-15), failure via ErrorModal with ArchelError::message() and no technical noise (D-12/D-17)"
    - "Package install/upgrade/remove stream real-time output line-by-line in StreamModal (D-13) with an Annuler button that requests a stop via the core's one-shot SIGINT path; the stream drains to EOF and the modal reports how the transaction ended (D-14)"
    - "Installing an AUR package requires reading a summary card (versions, repo vs AUR tag, dependencies, warnings); the Valider button stays disabled until the PKGBUILD expander has been opened at least once AND an explicit trust confirmation is given (D-19)"
    - "While a write is in progress the action button shows the named spinner ('Installation…' / 'Suppression…'), all action controls in the tab are disabled, and other tabs stay navigable (D-15)"
    - "Privileged operations (pkexec pacman, pkexec yay) are launched by the SAME child process as the unprivileged preview phase — never from QML (D-18); the API-side NeedsPrivilege path is what the CLI exercises, the GUI shells out through pkexec per RESEARCH Pattern 4"
  artifacts:
    - "crates/archel/src/gui/bridge.rs: ShellModalSession added to ShellModel (session kind, lines, question, error text, done flag, cancel AtomicBool) + invokables open_confirm/open_error/open_stream/close/cancel/resume"
    - "crates/archel/src/gui/models/pkg.rs: PkgModel write invokables install_start / upgrade_start / remove_start plus AurReview fields exposed for StreamModal's trust block"
    - "ConfirmModal/ErrorModal/StreamModal real implementations (bodies replaced, api block intact)"
    - "Main.qml: the three modals instantiated once at root, bound to ShellModel"
  key_links:
    - "core run_privileged_streaming_cancellable / yay_build_cancellable (shipped by 06-02) <- PkgModel spawn_blocking call -> StreamModal line feed — the ONLY path to pacman/yay; the GUI never spawns or signals a child process"
    - "StreamModal Annuler -> ShellModel.cancelStream -> shared AtomicBool observed inside core's per-line read loop -> one-shot SIGINT + drain to EOF + ended-state reported back through the modal (D-14)"
    - "core packages install_preview/remove_preview/upgrade_preview summaries -> ConfirmModal question via confirm_install/remove/upgrade helpers"
    - "core aur::review_package (AurReview { card, pkgbuild, trust, dependencies, warnings }) -> StreamModal trust block -> aur::yay_build_cancellable only after the PKGBUILD expander has been opened at least once AND trust confirmed (D-19)"
  prohibitions:
    - "The GUI crate must not spawn, signal, or terminate package subprocesses itself — stop requests flow only through the core's one-shot SIGINT + drain-to-EOF helpers (D-14)"
    - "The AUR Valider button must not become enabled while the PKGBUILD expander has never been opened, regardless of the trust checkbox state (D-19)"
    - "No StreamModal may be dismissible while a session is open except through the explicit Annuler action (D-15)"
    - "Plan 06-06 must not touch build.rs QmlModule registration (all files it fills were registered by 06-01)"
---

<objective>
Turn the tab shell into a write-capable application: the three root modals
(Confirm/Error/Stream) become real, ShellModel gains the modal-session API, and
the Packages tab gains install/upgrade/remove with streaming output and the AUR
trust gate.

Purpose: delivers success criterion 3 (every write action shows spinner +
disabled state + readable error) for the riskiest domain (root-adjacent package
mutations) and the D-12/D-13/D-14/D-15/D-18/D-19 contract.
Output: working write flows for Apps/Packages; modal infra every later plan reuses.
<!-- key_links: plan 06-07 instantiates PairingModal + VpnImportPopup at the same
     Main.qml root; plan 06-08 instantiates CountdownModal inside DisplayTab. -->
</objective>

<context_read_first>
- .planning/phases/06-gui-shell-tabs/06-CONTEXT.md (D-12, D-13, D-14, D-15, D-18, D-19)
- .planning/phases/06-gui-shell-tabs/06-RESEARCH.md (Pattern 4 subprocess+streaming;
  Pitfall 6 — never hard-kill a transaction on Annuler (D-14 SIGINT + drain +
  report); AUR trust gate per D-19 + Tab↔core map line 274 — "never install an
  AUR package without showing the PKGBUILD review"; pkexec note; AurReview section)
- .planning/phases/06-gui-shell-tabs/06-02-SUMMARY.md + crates/archel-core/src/privileged.rs
  (run_privileged_streaming_cancellable — the cancellable core helpers this plan calls)
- crates/archel/src/cmds/pkg_cmds.rs (begin_install_session/begin_remove_session/
  begin_upgrade_session + stream_session + aur_install_session — the CLI twins to
  compare against; the GUI calls the 06-02 core helpers, not these directly)
- crates/archel-core/src/packages.rs (install_preview:398, remove_preview:434,
  upgrade_preview:461, validate_package_names:360) and crates/archel-core/src/aur.rs
  (AurReview:103, review_package:309, yay_build:349, trust_labels, validate_pkgbuild_source)
- crates/archel/src/gui/bridge.rs (ShellModel as written by 06-01/06-03)
- crates/archel/src/gui/qml/Main.qml (root instantiation site)
</context_read_first>

<tasks>

<task type="auto">
  <name>Task 1: ShellModel modal-session API + real Confirm/Error/Stream bodies</name>
  <files>crates/archel/src/gui/bridge.rs, crates/archel/src/gui/qml/components/ConfirmModal.qml, crates/archel/src/gui/qml/components/ErrorModal.qml, crates/archel/src/gui/qml/components/StreamModal.qml, crates/archel/src/gui/qml/Main.qml</files>
  <action>
    ShellModel gains a modal session (UI-SPEC Component #1/#2/#3 state):
    properties modalKind ("none"/"confirm"/"error"/"stream"), modalQuestion,
    modalLines (QStringList-backed for streaming), modalError, modalDone,
    modalTrustNeeded + modalTrustSummary (AUR block, used by Task 3), plus
    invokables openConfirm(question) -> oneshot resolution (QML resolves via
    confirmAccepted()/confirmDeclined() invokables), openError(message),
    openStream(lines-source handle), closeModal, cancelStream (sets a
    std::sync::atomic::AtomicBool shared with the core cancellable helpers
    (06-02), which observe it inside their read loop). Session state lives
    in Mutex-protected ShellData; property writes stay on the owning thread —
    streaming line feeds are routed queue-only like PollGuard (06-03 pattern; the
    shared QueueOnlyWrite helper may be extracted to gui/queue.rs if cleaner).

    Flesh the three placeholder modals (placeholders registered by 06-01; their
    api blocks are defined by this task and stay frozen for 06-07/08):
    ConfirmModal — CardDialog, title "Archel", question text (wrapMode), Fermer
    (secondary) + Confirmer (primary) buttons wired to the oneshot.
    ErrorModal — CardDialog, red-accent title "Une erreur est survenue",
    modalError text (readable French, word-wrapped, selectable), unique Fermer
    primary button.
    StreamModal — CardDialog title "Opération en cours"; ScrollView/ListView of
    modalLines monospace 13px on surface-variant; Annuler button -> cancelStream;
    when modalDone the footer button becomes Fermer; while !modalDone Annuler is
    the only enabled button and closing via Esc is blocked (D-15: interruption
    goes through the explicit Annuler action, not disappearance).
    Auto-scroll to bottom on new lines.

    Main.qml instantiates ConfirmModal{}, ErrorModal{}, StreamModal{} once at
    root (z above OverlayBanner), each visible == ShellModel.modalKind match.
    UI Considerations rows loading-write-buttons (spinner + disabled during an
    operation) and error-write (named error, Fermer) — backstops verified at
    wave end via the 06-09 gui-check harness conventions.
  </action>
  <verify>
    <automated>QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui && QMAKE=/usr/bin/qmake6 cargo clippy -p archel --features gui --all-targets -- -D warnings</automated>
    <fails_when>pattern compile fails, ShellModalSession API incomplete, or a modal's api block (defined here) diverges from what 06-07/08 depend on</fails_when>
  </verify>
  <acceptance_criteria>
    ShellModel exposes the modal session API (unit test: opening a stream session
    appends lines and cancel sets the flag). The three modals render from real
    templates; Main.qml instantiates them; offscreen smoke still rc=124.
  </acceptance_criteria>
  <done>Modal round-trip (open -> lines/resolve -> close) unit-tested; `cargo test -p archel --no-default-features` green.</done>
</task>

<task type="auto">
  <name>Task 2: PkgModel write invokables + PkgTab write UI</name>
  <files>crates/archel/src/gui/models/pkg.rs, crates/archel/src/gui/qml/tabs/PkgTab.qml</files>
  <action>
    PkgModel write methods, each: validate -> core preview -> core confirm_*
    question -> ShellModel.openConfirm -> on yes spawn_blocking the core
    cancellable streaming helper run_privileged_streaming_cancellable shipped by
    06-02 (crates/archel-core/src/privileged.rs — verb/packages/action_label +
    on_line callback + &AtomicBool cancel handle) -> the on_line callback feeds
    modalLines -> on return set modalDone and report how the transaction ended
    from the returned { transcript, cancelled, ended_abnormally, exit_code }
    envelope (D-14 — a user-cancelled stop is NOT collapsed into an error),
    refresh the installed list, push a success/error inline banner. The GUI
    never builds a Command, never holds a child pid, never signals a process —
    subprocess lifecycle is owned by core (thin-view contract; RESEARCH
    responsibility map). install_start(names) accepts the batch selection from
    the checkbox column; upgrade_start targets all pending; remove_start
    removes the selected set.
    PkgTab write UI: selected-row toolbar — "Installer" / "Mettre à jour
    ({n})" / "Supprimer" primary Buttons (disabled + BusyIndicator label while a
    session is open, D-15), selection via CheckBox column on ListView rows.
    NeedsPrivilege-shaped failures surface through ErrorModal with
    ArchelError::message() produced by the core's map_pkexec_exit (pkexec codes
    0/1/126/127 semantics stay in core — "Impossible d'obtenir les privilèges
    d'administration." for auth denied/agent missing; never leak stderr verbatim
    into the modal; raw transcript lines stay in modalLines for the curious).
  </action>
  <verify>
    <automated>cargo test -p archel-core --no-default-features packages && cargo test -p archel --no-default-features</automated>
    <fails_when>any write unit test red (question uses core helper verbatim, cancel flag honored, batch names validated pre-session)</fails_when>
  </verify>
  <acceptance_criteria>
    Unit tests: install/remove/upgrade build the question from the core
    confirm_* helper output (not reconstructed French); cancel mid-stream
    asserts the 06-02 contract via its fake-child harness (cancel sets the
    flag; child receives SIGINT once; stream drains to EOF; the modal reports
    how it ended — D-14), never a hard-kill; invalid names rejected pre-confirm.
  </acceptance_criteria>
  <done>repo install/upgrade/remove end-to-end via CLI twin command comparison; AUR deferred to Task 3.</done>
</task>

<task type="auto">
  <name>Task 3: AUR trust gate (summary card + PKGBUILD expander + explicit trust)</name>
  <files>crates/archel/src/gui/models/pkg.rs, crates/archel/src/gui/qml/tabs/PkgTab.qml, crates/archel/src/gui/qml/components/StreamModal.qml</files>
  <action>
    For a name not in the installed/repo list: aur::review_package(name) via
    spawn_blocking -> AurReview. StreamModal gains the trust block (visible when
    modalTrustNeeded): summary card rendered from card + dependencies + warnings
    (version + repo/AUR tag + dependency list + warning rows — the UI-SPEC
    empty-pkg/error-aur-copy elements), TextExpand "Voir le PKGBUILD" revealing
    the raw pkgbuild text monospace (text already in memory). The expander's
    onExpandedChanged latches a **one-way** `pkgbuildSeen` bool on the session
    (once true it never resets — collapsing it would reopen the D-19 hole), and
    that flag is carried down to the yay build call so the gate survives a
    mid-session modal reopen. trust_labels render the three trust levels, and a
    CheckBox "Je confirme la source" adds a second explicit-consent layer. The
    primary Button is labelled "Valider" per D-19 and stays disabled until
    `pkgbuildSeen && trustConfirmed` — the expander-opened condition is the
    D-19 gate itself, not the checkbox (D-19 preserves Phase 3 D-05's
    full-display boundary in summary-first form). Only then does
    aur::yay_build_cancellable (the 06-02 cancellable variant, not the plain
    yay_build) run with streaming + Annuler (same session machinery as Task 2).
    If yay is absent the whole flow is inert (UI Already states per UI-SPEC).
    Pre-check aur::validate_pkgbuild_source on the fetched source; reject via
    ErrorModal on failure. UI Consideration error-aur-copy backstop owned here.
  </action>
  <verify>
    <automated>cargo test -p archel-core --no-default-features aur && cargo test -p archel --no-default-features</automated>
    <fails_when>AUR unit tests red: Valider enabled while pkgbuildSeen is false (the primary D-19 assertion), any path to yay_build_cancellable without BOTH pkgbuildSeen and trustConfirmed, pkgbuildSeen resettable after one open, or validate_pkgbuild_source missing on the build path</fails_when>
  </verify>
  <acceptance_criteria>
    Unit tests: review_package output reaches the trust block fields;
    yay_build_cancellable invocation is unreachable while `pkgbuildSeen` is
    false even with trust checked, and unreachable while trust is unchecked even
    with the expander opened (two separate flag-checked seam assertions);
    pkgbuildSeen stays true once set; a session reopened mid-flow still reports
    pkgbuildSeen at the build call.
  </acceptance_criteria>
  <done>AUR install flow: search -> card + PKGBUILD expander -> explicit trust -> streamed yay build; committed.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| GUI -> pkexec child | privileged package transaction launched from GUI process |
| AUR network -> local build | untrusted PKGBUILD fetched and later executed by makepkg |
| child output -> GUI | subprocess stdout/stderr rendered in StreamModal |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-06-28 | Tampering | install/upgrade/remove child spawn | high | mitigate | fixed argv arrays via the core preview/session builders, validate_package_names gate, no shell interpolation, pkexec only (T-06-SC applies) |
| T-06-29 | Elevation | AUR build executing untrusted PKGBUILD | high | mitigate | D-19 gate: aur::review_package + validate_pkgbuild_source + raw PKGBUILD shown + Valider disabled until the expander has been opened (pkgbuildSeen) AND trust checkbox checked; yay_build_cancellable unreachable otherwise (unit-tested both ways) |
| T-06-30 | Denial of Service | runaway streamed child output | low | accept | line count capped at 10k in modalLines with truncation notice; Annuler always available; manual UAT |
| T-06-SC | Tampering | npm/pip/cargo installs | high | mitigate | no new packages introduced; package-legitimacy gate N/A; blocking human checkpoint retained for any install discovered at execution |
</threat_model>

<artifacts_produced>
## Artifacts this phase produces

(New in this plan; additions to files created by earlier plans.)

- `ShellModel` modal session API — `openConfirm` (oneshot), `openError`,
  `openStream` + line feed + `cancelStream` (AtomicBool) — the single modal
  surface 06-07 and 06-08 reuse for every domain write.
- `ConfirmModal` / `ErrorModal` / `StreamModal` real implementations (templates;
  api blocks defined here, frozen for 06-07/08) + root instantiation in `Main.qml`.
- `PkgModel` write methods: `install_start` / `upgrade_start` / `remove_start` +
  AUR trust flow (`aur_review_start`, trust-confirmed `aur_build_start`).
- StreamModal trust block (summary card, PKGBUILD expander, trust checkbox) —
  D-19 gate UI.
- UI Considerations coverage: loading-write-buttons, error-write, error-aur-copy.
</artifacts_produced>

<verification>
- `cargo test -p archel --no-default-features` green; clippy clean both feature sets.
- Offscreen smoke rc=124.
- Manual (wave-end UAT, not blocking): install a tiny repo package — spinner +
  disabled buttons during, streaming lines appear, success banner; Annuler mid-
  download makes pacman stop via the core SIGINT path and the modal reports how
  it ended; a wrong-name remove opens ErrorModal with readable French.
</verification>

<success_criteria>
The phase goal's write-action criterion is delivered for Packages, and the modal
infrastructure for all remaining write plans exists and is reusable.
</success_criteria>

<output>
Create `.planning/phases/06-gui-shell-tabs/06-06-SUMMARY.md` when done
</output>
