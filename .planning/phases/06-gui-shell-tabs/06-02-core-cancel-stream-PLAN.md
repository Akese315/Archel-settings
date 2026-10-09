---
phase: "06"
plan: "02"
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/archel-core/src/privileged.rs
  - crates/archel-core/src/aur.rs
autonomous: true
requirements: [PLAT-03]
estimate:
  tokens: 35000
  raw_tokens: 25000
  tasks: 2
  confidence: low
must_haves:
  truths:
    - "A privileged transaction can be asked to stop: setting the cancel handle sends SIGINT to the pkexec child once, the pipe is drained to EOF, and the caller learns how the transaction ended (exit status) — the GUI twin of the CLI stop path (D-13/D-14)"
    - "`yay_build` gains the same cancel handle so AUR builds are cancellable identically"
    - "Existing non-cancellable wrappers keep their exact signatures — the CLI stays bit-compatible and its snapshots stay green"
  artifacts:
    - "run_privileged_streaming_cancellable in crates/archel-core/src/privileged.rs"
    - "yay_build_cancellable in crates/archel-core/src/aur.rs"
    - "cancel-aware fake-child tests in the existing privileged test module"
  key_links:
    - "cancel flag (AtomicBool) checked INSIDE the per-line read loop -> one-shot SIGINT -> drain to EOF -> try_wait -> map_pkexec_exit reporting"
prohibitions:
  - "Cancellation must never send an uncatchable kill signal to pacman/yay or the pkexec wrapper (D-14 — lock/partial-transaction safety); the child is only ever asked politely via SIGINT and always drained to EOF"
---

<objective>
Close RESEARCH core gap #1: give the privileged streaming paths a stop channel so the GUI's StreamModal Annuler button can request a graceful stop and report the final state — the only core change this phase needs for mutations.

Purpose: D-13/D-14 are impossible without a cancel-aware core; the GUI must not manage subprocesses itself.
Output: `run_privileged_streaming_cancellable` + `yay_build_cancellable` with tested SIGINT semantics.
</objective>

<execution_context>
@/home/axel/.config/opencode/gsd-core/workflows/execute-plan.md
@/home/axel/.config/opencode/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/phases/06-gui-shell-tabs/06-CONTEXT.md
@.planning/phases/06-gui-shell-tabs/06-RESEARCH.md
@.planning/phases/06-gui-shell-tabs/06-PATTERNS.md
@crates/archel-core/src/privileged.rs
@crates/archel-core/src/aur.rs
</context>

<tasks>

<task type="auto" id="06-02-T1">
  <name>Task 1: Cancellable privileged streaming (SIGINT, drain, report)</name>
  <read_first>crates/archel-core/src/privileged.rs (collect_streaming:196-207, run_privileged_streaming:214-256, map_pkexec_exit, pkg_helper_argv, the existing test module feeding a cursor to collect_streaming), 06-RESEARCH.md (Core Gaps #1, Pitfall 6, A4), 06-PATTERNS.md (privileged.rs section)</read_first>
  <files>crates/archel-core/src/privileged.rs</files>
  <action>Add `pub fn run_privileged_streaming_cancellable(verb, packages, action_label, on_line, cancel: &AtomicBool) -> Result<CancellableStreaming, ArchelError>` following the existing `*_with`/cancellable naming convention. Mirror run_privileged_streaming exactly (which pkexec probe, locate_helper, argv vector via pkg_helper_argv, duct `.unchecked().env("LC_ALL","C").stderr_capture().reader()` — never `run()`, Pitfall 7) except: the child must be spawned so its pid is known (std::process::Command with piped stdout, or duct's child API) and the per-line loop checks `cancel` between lines; on first set, send SIGINT to the pkexec child exactly ONCE via the `kill` utility subprocess (`kill -INT <pid>` through which-detected /bin/kill — no libc/nix dependency, no new packages), then keep reading to EOF and reap with try_wait. Return a struct `{ transcript, cancelled: bool, ended_abnormally: bool, exit_code: Option<i32> }` (or reuse the existing envelope types) so the caller reports HOW it ended (D-14) — do not collapse a user-cancelled stop into an error; map_pkexec_exit still applies for genuine failures (pkexec codes 0/1/126/127 semantics unchanged). Keep the old function byte-identical. Extend the existing privileged test module with fake-child tests (bash/sh fake child emitting marker lines): (a) no cancel => full transcript + exit code; (b) cancel mid-stream => child receives SIGINT once, fake child's trap writes a marker to a sentinel file, function still reads to EOF and reports the ended state; (c) cancel after natural end => no extra signal. Live-machine note (A4): add a `#[ignore]`d test running `sleep 30` through the real path for manual confirmation on the Archel box.</action>
  <verify>
    <automated>cargo test -p archel-core --no-default-features privileged && cargo clippy -p archel-core --all-features -- -D warnings</automated>
    <automated>! grep -nE 'SIGKILL|libc::kill|nix::sys::signal' crates/archel-core/src/privileged.rs</automated>
    <fails_when>privileged tests fail; the fake-child SIGINT sentinel is never written (cancel not plumbed into the read loop); a hard-kill token appears in privileged.rs; clippy denies</fails_when>
  </verify>
  <acceptance_criteria>
  - `cargo test -p archel-core --no-default-features privileged` green including the three new cancel scenarios
  - Cancel path sends SIGINT exactly once, drains stdout to EOF, and the returned value carries exit status + cancelled flag (D-14)
  - Old `run_privileged_streaming` signature/behavior untouched; `cargo test -p archel --no-default-features` (CLI snapshots) still green
  </acceptance_criteria>
  <done>Streaming transactions are cancellable with reported outcome, tested against a fake child.</done>
</task>

<task type="auto" id="06-02-T2">
  <name>Task 2: Cancellable yay build</name>
  <read_first>crates/archel-core/src/aur.rs (yay_build ~349 and its streaming reader loop + callers), 06-RESEARCH.md (Core Gaps #1 — mirror the same handle for yay_build), 06-PATTERNS.md (aur.rs section)</read_first>
  <files>crates/archel-core/src/aur.rs</files>
  <action>Add `yay_build_cancellable(name, on_line, cancel: &AtomicBool)` mirroring Task 1's mechanics for the yay path (same which-detection and graceful "yay not installed" state as the existing yay_build; same one-shot SIGINT-to-child, drain-to-EOF, exit-status-reported semantics; yay traps INT and stops makepkg cleanly — same rationale as A4). Factor the shared line-loop + signal helper into one private fn in privileged.rs (or a small internal helper module) so both call sites use ONE implementation of the cancel logic; keep `yay_build` itself unchanged for the CLI. No new packages.</action>
  <verify>
    <automated>cargo test -p archel-core --no-default-features aur && cargo test -p archel-core --no-default-features privileged && cargo test -p archel --no-default-features</automated>
    <fails_when>aur tests fail; yay_build (non-cancellable) behavior changed and CLI AUR snapshots go red; cancel logic duplicated instead of shared</fails_when>
  </verify>
  <acceptance_criteria>
  - `yay_build_cancellable` exists with the same SIGINT/drain/report contract as the privileged variant
  - Shared cancel helper is a single implementation used by both paths
  - CLI aur-related snapshots/tests unchanged and green
  </acceptance_criteria>
  <done>AUR builds cancellable through the same tested helper; core gap #1 closed.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| GUI request -> pkexec child | Stop requests cross into an elevated subprocess tree |
| pacman/yay stdout -> GUI transcript | Untrusted process output flows to display |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-06-02 | Denial of Service | run_privileged_streaming_cancellable / yay_build_cancellable cancel path | high | mitigate | Exactly one SIGINT (catchable -> pacman finalizes/aborts its transaction and releases its lock), never an uncatchable kill; stdout always drained to EOF then reaped (no zombie, no dirty lock — Pitfall 6); exit status surfaced so the modal reports the true end state (D-14) |
| T-06-18 | Tampering | cancel handle type | low | accept | Handle is an `&AtomicBool` created by our own GUI process only; no external input can set it except the user's own Annuler click |
| T-06-SC | Tampering | package installs | high | mitigate | No new packages this plan (RESEARCH audit: none); kill binary resolved via which at runtime |
</threat_model>

<verification>
- Full suite (VALIDATION.md) — core side: `cargo clippy -p archel-core --all-features -- -D warnings && cargo test -p archel-core --no-default-features && cargo test -p archel --no-default-features`
- Run the `#[ignore]`d live test on the Archel machine during execution to confirm A4 (pkexec propagates SIGINT to pacman).
</verification>

<success_criteria>
Both cancellable functions exist, share one tested cancel implementation, and no existing core/CLI behavior changed.
</success_criteria>

## Artifacts this phase produces (this plan)

- `run_privileged_streaming_cancellable`, `yay_build_cancellable`, shared private cancel/signal helper in archel-core, fake-child SIGINT test harness (Wave 0 item satisfied).

<output>
When all tasks pass, create `.planning/phases/06-gui-shell-tabs/06-02-SUMMARY.md`.
</output>
