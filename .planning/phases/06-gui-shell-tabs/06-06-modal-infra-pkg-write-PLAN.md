---
phase: 06-gui-shell-tabs
plan: 06
type: execute
wave: 5
depends_on: [06-02, 06-05]
files_modified:
  - crates/archel/src/gui/bridge.rs
  - crates/archel/src/gui/models/packages.rs
  - crates/archel/qml/components/ConfirmModal.qml
  - crates/archel/qml/components/ErrorModal.qml
  - crates/archel/qml/components/StreamModal.qml
  - crates/archel/qml/Main.qml
  - crates/archel/qml/tabs/PackagesTab.qml
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
    - "Package install/upgrade/remove stream real-time output line-by-line in StreamModal (D-13/D-14) with an Annuler button that terminates the child process"
    - "Installing an AUR package requires reading a summary card (versions, repo vs AUR tag, dependencies, warnings) and expanding the raw PKGBUILD before an explicit trust confirmation (D-19)"
    - "While a write is in progress the action button shows the named spinner ('Installation…' / 'Suppression…'), all action controls in the tab are disabled, and other tabs stay navigable (D-15)"
    - "Privileged operations (pkexec pacman, pkexec yay) are launched by the SAME child process as the unprivileged preview phase — never from QML (D-18); the API-side NeedsPrivilege path is what the CLI exercises, the GUI shells out through pkexec per RESEARCH Pattern 4"
  artifacts:
    - "crates/archel/src/gui/bridge.rs: ShellModalSession added to ShellModel (session kind, lines, question, error text, done flag, cancel AtomicBool) + invokables open_confirm/open_error/open_stream/close/cancel/resume"
    - "crates/archel/src/gui/models/packages.rs: PkgModel write invokables install_start / upgrade_start / remove_start plus AurReview fields exposed for StreamModal's trust block"
    - "ConfirmModal/ErrorModal/StreamModal real implementations (bodies replaced, api block intact)"
    - "Main.qml: the three modals instantiated once at root, bound to ShellModel"
  key_links:
    - "cmds/pkg_cmds.rs:154 begin_upgrade_session / :203 begin_install_session / :237 begin_remove_session -> PkgModel spawn_blocking -> StreamModal line feed — the ONLY path to pacman/yay"
    - "StreamModal Annuler -> ShellModel.cancel -> AtomicBool observed by the streaming loop -> child killed (SIGTERM then SIGKILL)"
    - "core packages install_preview/remove_preview/upgrade_preview summaries -> ConfirmModal question via confirm_install/remove/upgrade helpers"
    - "core aur::review_package (AurReview { card, pkgbuild, trust, dependencies, warnings }) -> StreamModal trust block -> aur::yay_build only after trust confirmed"
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
  Pitfall 3 AUR trust gate — "never install an AUR package without showing the
  PKGBUILD review"; pkexec note; AurReview section)
- crates/archel/src/cmds/pkg_cmds.rs (begin_install_session/begin_remove_session/
  begin_upgrade_session + stream_session + aur_install_session — the exact twins)
- crates/archel-core/src/packages.rs (install_preview:398, remove_preview:434,
  upgrade_preview:461, validate_package_names:360) and crates/archel-core/src/aur.rs
  (AurReview:103, review_package:309, yay_build:349, trust_labels, validate_pkgbuild_source)
- crates/archel/src/gui/bridge.rs (ShellModel as written by 06-01/06-03)
- crates/archel/qml/Main.qml (root instantiation site)
</context_read_first>

<tasks>

<task type="auto">
  <name>Task 1: ShellModel modal-session API + real Confirm/Error/Stream bodies</name>
  <files>crates/archel/src/gui/bridge.rs, crates/archel/qml/components/ConfirmModal.qml, crates/archel/qml/components/ErrorModal.qml, crates/archel/qml/components/StreamModal.qml, crates/archel/qml/Main.qml</files>
  <action>
    ShellModel gains a modal session (UI-SPEC Component #1/#2/#3 state):
    properties modalKind ("none"/"confirm"/"error"/"stream"), modalQuestion,
    modalLines (QStringList-backed for streaming), modalError, modalDone,
    modalTrustNeeded + modalTrustSummary (AUR block, used by Task 3), plus
    invokables openConfirm(question) -> oneshot resolution (QML resolves via
    confirmAccepted()/confirmDeclined() invokables), openError(message),
    openStream(lines-source handle), closeModal, cancelStream (sets a
    std::sync::atomic::AtomicBool the streaming loop polls). Session state lives
    in Mutex-protected ShellData; property writes stay on the owning thread —
    streaming line feeds are routed queue-only like PollGuard (06-03 pattern; the
    shared QueueOnlyWrite helper may be extracted to gui/queue.rs if cleaner).

    Flesh the three placeholder modals (api blocks from 06-02 untouched):
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
    <fails_when>pattern compile fails, ShellModalSession API incomplete, or any modal's api block from 06-02 was altered</fails_when>
  </verify>
  <acceptance_criteria>
    ShellModel exposes the modal session API (unit test: opening a stream session
    appends lines and cancel sets the flag). The three modals render from real
    templates; Main.qml instantiates them; offscreen smoke still rc=124.
  </acceptance_criteria>
  <done>Modal round-trip (open -> lines/resolve -> close) unit-tested; `cargo test -p archel --no-default-features` green.</done>
</task>

<task type="auto">
  <name>Task 2: PkgModel write invokables + PackagesTab write UI</name>
  <files>crates/archel/src/gui/models/packages.rs, crates/archel/qml/tabs/PackagesTab.qml</files>
  <action>
    PkgModel write methods, each: validate -> core preview -> core confirm_*
    question -> ShellModel.openConfirm -> on yes spawn_blocking the streaming
    session (reuse begin_install_session/begin_remove_session/begin_upgrade_session
    + stream_session from pkg_cmds — import from cmds::pkg_cmds, they are pub) ->
    feed modalLines line-by-line while polling the cancel flag -> on finish set
    modalDone, refresh the installed list, push a success/error inline banner.
    install_start(names) accepts the batch set (D-14); upgrade_start targets all
    pending; remove_start removes the selected set. All three use
    tokio::task::spawn_blocking (blocking child I/O). Child stdio: piped stdout
    + stderr merged into lines, Stdio::null() stdin, kill on cancel (child id
    kept in the session, SIGTERM 2 s then SIGKILL).
    PackagesTab write UI: selected-row toolbar — "Installer" / "Mettre à jour
    ({n})" / "Supprimer" primary Buttons (disabled + BusyIndicator label while a
    session is open, D-15), selection via CheckBox column on ListView rows.
    NeedsPrivilege-shaped failures surface through ErrorModal with
    ArchelError::message() — "Impossible d'obtenir les privilèges
    d'administration." when pkexec exits 126/127 (auth denied/agent missing —
    detect exit code, never leak stderr verbatim into the modal; raw stderr stays
    in modalLines for the curious).
  </action>
  <verify>
    <automated>cargo test -p archel-core --no-default-features packages && cargo test -p archel --no-default-features</automated>
    <fails_when>any write unit test red (question uses core helper verbatim, cancel flag honored, batch names validated pre-session)</fails_when>
  </verify>
  <acceptance_criteria>
    Unit tests: install/remove/upgrade build the question from the core
    confirm_* helper output (not reconstructed French); cancel mid-stream
    terminates the child (test uses /bin/sleep); invalid names rejected pre-confirm.
  </acceptance_criteria>
  <done>repo install/upgrade/remove end-to-end via CLI twin command comparison; AUR deferred to Task 3.</done>
</task>

<task type="auto">
  <name>Task 3: AUR trust gate (summary card + PKGBUILD expander + explicit trust)</name>
  <files>crates/archel/src/gui/models/packages.rs, crates/archel/qml/tabs/PackagesTab.qml, crates/archel/qml/components/StreamModal.qml</files>
  <action>
    For a name not in the installed/repo list: aur::review_package(name) via
    spawn_blocking -> AurReview. StreamModal gains the trust block (visible when
    modalTrustNeeded): summary card rendered from card + dependencies + warnings
    (version + repo/AUR tag + dependency list + warning rows — the UI-SPEC
    empty-pkg/error-aur-copy elements), TextExpand "Voir le PKGBUILD" revealing
    the raw pkgbuild text monospace (Qt6::Core FileDialog-free, text already in
    memory), trust_labels for the three trust levels, CheckBox "Je confirme la
    source" gating the Confirmer button (stays disabled until checked — D-19).
    Only then aur::yay_build runs with streaming + Annuler (same session
    machinery as Task 2). If yay is absent the whole flow is inert (UI Already
    states per UI-SPEC). Pre-check aur::validate_pkgbuild_source on the fetched
    source; reject via ErrorModal on failure. UI Consideration error-aur-copy
    backstop owned here.
  </action>
  <verify>
    <automated>cargo test -p archel-core --no-default-features aur && cargo test -p archel --no-default-features</automated>
    <fails_when>AUR unit tests red (trust gate enforced before yay_build, no path to yay_build without trust checked, validate_pkgbuild_source on the build path)</fails_when>
  </verify>
  <acceptance_criteria>
    Unit test: review_package output reaches the trust block fields; yay_build
    invocation is unreachable while trust unconfirmed (flag-checked seam, test asserts).
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
| T-06-29 | Elevation | AUR build executing untrusted PKGBUILD | high | mitigate | D-19 gate: aur::review_package + validate_pkgbuild_source + raw PKGBUILD shown + explicit trust CheckBox before yay_build; no path bypasses it (unit-tested) |
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
  api blocks unchanged) + root instantiation in `Main.qml`.
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
  download kills pacman; a wrong-name remove opens ErrorModal with readable French.
</verification>

<success_criteria>
The phase goal's write-action criterion is delivered for Packages, and the modal
infrastructure for all remaining write plans exists and is reusable.
</success_criteria>

<output>
Create `.planning/phases/06-gui-shell-tabs/06-06-SUMMARY.md` when done
</output>
