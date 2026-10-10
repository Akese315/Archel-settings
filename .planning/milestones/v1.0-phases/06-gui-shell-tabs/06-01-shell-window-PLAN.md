---
phase: "06"
plan: "01"
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/archel/src/main.rs
  - crates/archel/src/gui/mod.rs
  - crates/archel/build.rs
  - crates/archel/src/gui/bridge.rs
  - crates/archel/src/gui/single_instance.rs
  - crates/archel/src/gui/geometry.rs
  - crates/archel/src/gui/qml/Main.qml
  - crates/archel/src/gui/qml/Spike.qml
  - crates/archel/src/gui/qml/tabs/LandingTab.qml
  - crates/archel/src/gui/qml/tabs/NetTab.qml
  - crates/archel/src/gui/qml/tabs/PowerTab.qml
  - crates/archel/src/gui/qml/tabs/BtTab.qml
  - crates/archel/src/gui/qml/tabs/AppearanceTab.qml
  - crates/archel/src/gui/qml/tabs/AudioTab.qml
  - crates/archel/src/gui/qml/tabs/PkgTab.qml
  - crates/archel/src/gui/qml/tabs/DisplayTab.qml
  - crates/archel/src/gui/qml/tabs/InputTab.qml
  - crates/archel/src/gui/qml/tabs/SystemTab.qml
  - crates/archel/src/gui/qml/components/ConfirmModal.qml
  - crates/archel/src/gui/qml/components/ErrorModal.qml
  - crates/archel/src/gui/qml/components/StreamModal.qml
  - crates/archel/src/gui/qml/components/CountdownModal.qml
  - crates/archel/src/gui/qml/components/PairingModal.qml
  - crates/archel/src/gui/qml/components/StatField.qml
  - crates/archel/src/gui/qml/components/PollBanner.qml
  - crates/archel/src/gui/qml/components/Sparkline.qml
  - crates/archel/src/gui/qml/components/VpnImportPopup.qml
  - crates/archel-core/src/i18n.rs
  - crates/archel-core/locales/fr.json
  - crates/archel/tests/gui_shell.rs
autonomous: true
requirements: [PLAT-03]
estimate:
  tokens: 63000
  raw_tokens: 45000
  tasks: 3
  confidence: low
must_haves:
  truths:
    - "User opens `archel settings` and sees a 900x620 window titled `Archel Réglages` with a 200px vertical rail of 10 icon+label TabButtons in the locked French order and a content StackLayout (D-22, D-25)"
    - "Opens on the Landing tab by default (D-22); all 10 tab pages exist as registered placeholders so later plans never touch build.rs (Pitfall 2)"
    - "Window geometry (x, y, w, h, maximized) is saved to ~/.config/archel/window-geometry.json on close and restored on next launch, clamped/centered when off-screen, 900x620 fallback (D-23, Pitfall 4)"
    - "A second `archel settings` launch prints the catalog French message to stdout and exits 0; only the first instance binds (D-26)"
    - "The rail scrolls as its own Flickable when 10 buttons exceed window height — no clipped or unreachable tab button (UI-SPEC rail overflow row)"
    - "Qt build stays isolated behind the `gui` feature — headless `cargo test -p archel --no-default-features` compiles no Qt and stays green"
  artifacts:
    - crates/archel/src/gui/single_instance.rs
    - crates/archel/src/gui/geometry.rs
    - crates/archel/src/gui/qml/Main.qml
    - crates/archel/src/gui/qml/tabs/ (10 files)
    - crates/archel/src/gui/qml/components/ (9 files)
    - crates/archel/tests/gui_shell.rs
  key_links:
    - "main.rs Command::Settings branch -> gui::run_settings() (entered BEFORE any tokio runtime; app.exec() never occupies a runtime worker — RESEARCH Pattern 2)"
    - "run_settings(): OnceLock<tokio::runtime::Runtime> + QmlModule URI qrc:/qt/qml/org/archel/settings/src/gui/qml/Main.qml + listener owned for the whole exec() scope"
    - "build.rs QmlModule file list <-> every .qml on disk (unregistered QML fails only at runtime)"
    - "ShellModel geometry properties <-> Main.qml window x/y/w/h binding + onClosing save"
prohibitions:
  - "The refused-second-instance French sentence must come from the core catalog (rust_i18n t!), never as a literal in .rs source (Phase 1 D-07)"
---

<objective>
Turn the spike window into the real `archel settings` shell: proper entry-point restructure, manual tokio runtime, single-instance refusal, geometry persistence, the 10-button vertical rail, and every QML file pre-registered so later plans only fill in content.

Purpose: Establishes the window chrome and lifecycle every other GUI plan hangs from; retires the spike.
Output: Working shell window, single_instance.rs, geometry.rs, full QML skeleton, tests/gui_shell.rs.
</objective>

<execution_context>
@/home/axel/.config/opencode/gsd-core/workflows/execute-plan.md
@/home/axel/.config/opencode/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/phases/06-gui-shell-tabs/06-CONTEXT.md
@.planning/phases/06-gui-shell-tabs/06-RESEARCH.md
@.planning/phases/06-gui-shell-tabs/06-PATTERNS.md
@.planning/phases/06-gui-shell-tabs/06-UI-SPEC.md
@.planning/phases/01-foundations-safety-rails/01-02-SUMMARY.md
@crates/archel/src/main.rs
@crates/archel/src/gui/mod.rs
@crates/archel/build.rs
@crates/archel/src/gui/qml/Spike.qml
</context>

<tasks>

<task type="auto" id="06-01-T1">
  <name>Task 1: Entry rework + shell window + QML skeleton (retire spike)</name>
  <read_first>crates/archel/src/main.rs, crates/archel/src/gui/mod.rs, crates/archel/build.rs, crates/archel/src/gui/qml/Spike.qml, crates/archel/src/cmds/mod.rs:580-597, 06-RESEARCH.md (Pattern 2, Pitfalls 2/8), 06-PATTERNS.md (main.rs, mod.rs, build.rs, Main.qml sections), 06-UI-SPEC.md (Copywriting Contract rail labels row, Layout contracts Window/Rail rows)</read_first>
  <files>crates/archel/src/main.rs, crates/archel/src/gui/mod.rs, crates/archel/build.rs, crates/archel/src/gui/qml/Main.qml, crates/archel/src/gui/qml/Spike.qml (deleted), all 19 placeholder .qml files listed in frontmatter</files>
  <action>Restructure main.rs: remove `#[tokio::main]`; make `fn main()` that parses CLI, branches on `Command::Settings` -> `gui::run_settings()` BEFORE constructing any runtime, else constructs a runtime and enters the existing async path (RESEARCH Pattern 2). Keep the `eprintln!("{}", err.message())` error voice. Rework `run_settings()`: step 1 create `static RUNTIME: OnceLock<tokio::runtime::Runtime>` (Runtime::new() before Qt); set Fusion style via QQuickStyle::set_style unless QT_QPA_PLATFORMTHEME is set (D-24, A3); keep the Option-guard style (`engine.as_mut()`/`app.as_mut()`); load URI `qrc:/qt/qml/org/archel/settings/src/gui/qml/Main.qml`. build.rs: replace QmlModule `org.archel.spike` with `org.archel.settings` registering Main.qml, the 10 tab placeholders, and the 9 component placeholders (including CountdownModal and VpnImportPopup which later plans fill) — every .qml in the same commit it is added (Pitfall 2); keep the `#[cfg(not(feature = "gui"))] fn main() {}` no-op and rely on the QMAKE=/usr/bin/qmake6 pin (Pitfall 8). Main.qml: extend the Spike.qml skeleton — ApplicationWindow title `Archel Réglages`, default 900x620, minimum 720x480; rail Column width 200, 10 TabButtons (16x16 icon, French labels in locked order Accueil/Réseau/Alimentation/Bluetooth/Apparence/Son/Applications/Affichage/Périphériques/Système, spacing 4, item height 32, checked = accent pill) driving StackLayout currentIndex; StackLayout instantiates the 10 tab files; rail Column inside its own Flickable (UI-SPEC rail overflow row). Pure QML for now — no per-tab Rust models yet. Placeholders: tabs = minimal Item with a Heading Label; components = minimal Dialog/Popup/Item that qmllint accepts. Delete Spike.qml (SpikeModel in bridge.rs is removed by plan 03; leave it compiling). All user strings French in .qml, .rs plain ASCII. No settings search box (deferred).</action>
  <verify>
    <automated>QMAKE=/usr/bin/qmake6 cargo build -p archel --features gui && qmllint $(find crates/archel/src/gui/qml -name '*.qml') && cargo test -p archel --no-default-features</automated>
    <fails_when>gui build errors; qmllint reports errors on any registered file; headless (no-default-features) build pulls Qt or fails</fails_when>
    <automated>QMAKE=/usr/bin/qmake6 cargo build -p archel --features gui &amp;&amp; QT_QPA_PLATFORM=offscreen timeout 10 ./target/debug/archel settings &lt;/dev/null &gt;/tmp/archel-smoke.log 2&gt;&amp;1; rc=$?; grep -vE '^(#|//)' /dev/null; test $rc -eq 124 &amp;&amp; ! grep -qiE 'qrc:|cannot load|is not loaded' /tmp/archel-smoke.log</automated>
    <fails_when>the offscreen run exits nonzero before the 10 s timeout or logs any qrc/cannot-load error (rc must be 124 = window alive)</fails_when>
  </verify>
  <acceptance_criteria>
  - `archel settings` opens the shell (offscreen smoke: alive at timeout, zero qrc load errors)
  - Rail shows 10 icon+label buttons in the exact locked French order; default index selects Accueil (D-22, D-25)
  - Window default 900x620, minimum 720x480 (D-23); no in-app theme toggle (D-24)
  - Spike.qml deleted; `org.archel.spike` no longer registered; headless test suite green
  - No deferred feature present (no search box, no tray, no focus plumbing)
  </acceptance_criteria>
  <done>Shell window renders the rail + placeholder tabs offscreen, spike retired, headless and gui builds both green.</done>
</task>

<task type="auto" id="06-01-T2">
  <name>Task 2: Second-instance refusal (abstract UDS single bind)</name>
  <read_first>06-RESEARCH.md (Pitfall 3, "Second-instance refusal" code example, A1), 06-PATTERNS.md (single_instance.rs section — no analog, use RESEARCH example), crates/archel-core/src/i18n.rs, crates/archel-core/src/lib.rs:38-50 (t! helper pattern), 06-UI-SPEC.md (Second-instance refusal row)</read_first>
  <files>crates/archel/src/gui/single_instance.rs, crates/archel/src/gui/mod.rs, crates/archel-core/src/i18n.rs, crates/archel-core/locales/fr.json, crates/archel/tests/gui_shell.rs</files>
  <action>New `single_instance.rs`: `pub fn acquire() -> Result<Option<Listener>, ...>` doing a SINGLE bind() attempt on the abstract UDS name `\0archel-settings-v1` (tokio::net::UnixListener::bind; if abstract names unsupported — A1 — fall back to std+libc-free `sockaddr_un` bind via a small unsafe block, never probe-then-bind TOCTOU). Bind success => keep the listener alive for the whole exec() (owned in run_settings scope); bind failure (EADDRINUSE) => caller prints the refusal line and exits 0 (D-26). The refusal line comes from a NEW core helper in i18n.rs (`pub fn settings_already_running() -> String` returning `rust_i18n::t!("settings.already_running")` with the exact UI-SPEC French string stored under that key in locales/fr.json) — never a French literal in .rs. Wire the probe as step 0 of run_settings(). Tests in crates/archel/tests/gui_shell.rs (new file, `single_instance` filter): first acquire succeeds, second acquire on same name reports AlreadyRunning; helper string equals the catalog value; helper output contains no replacement chars. ASCII-only rule applies to the two new .rs files.</action>
  <verify>
    <automated>QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui single_instance && cargo test -p archel-core --no-default-features i18n</automated>
    <fails_when>double-acquire both succeed (TOCTOU/probe bug); second call panics; catalog helper returns the key name instead of the translated sentence</fails_when>
    <automated>! grep -rnP '[^\x00-\x7F]' crates/archel/src/gui/single_instance.rs crates/archel-core/src/i18n.rs</automated>
    <fails_when>the refusal sentence (or any non-ASCII byte) appears in the new .rs files</fails_when>
  </verify>
  <acceptance_criteria>
  - Second launch prints the exact UI-SPEC row string to stdout and exits 0 (D-26); verified via the catalog-backed helper test
  - Abstract-socket name bound exactly once; listener owned by run_settings scope (Pitfall 3)
  - New .rs files are pure ASCII; French lives only in locales/fr.json (Phase 1 D-07)
  </acceptance_criteria>
  <done>Second instance is refused with the catalog French line, exit 0; tests green.</done>
</task>

<task type="auto" id="06-01-T3">
  <name>Task 3: Geometry persistence + ShellModel + rail icons</name>
  <read_first>crates/archel-core/src/updates.rs:106-140 (atomic XDG JSON pattern), 06-PATTERNS.md (geometry.rs section, bridge.rs bridge-declaration pattern, Main.qml section), 06-RESEARCH.md (Pitfall 4, "Geometry persistence" example), 06-UI-SPEC.md (Window/Rail layout rows)</read_first>
  <files>crates/archel/src/gui/geometry.rs, crates/archel/src/gui/bridge.rs, crates/archel/src/gui/mod.rs, crates/archel/src/gui/qml/Main.qml, crates/archel/tests/gui_shell.rs, crates/archel/src/gui/qml/images/ (10 vendored MIT SVG icons)</files>
  <action>New `geometry.rs`: `Geometry { x, y, w, h, maximized }` serde struct at `directories::ProjectDirs::from("", "archel", "archel").config_dir().join("window-geometry.json")`; read: any failure => None and caller falls back to 900x620 centered (D-23); write: atomic temp-file + rename, failures swallowed (mirror updates.rs:128-140). At startup validate saved rect against QGuiApplication::screens() extents; NaN/zero/off-all-screens => clamp to minimum and center (Pitfall 4). Add `ShellModel` qobject to bridge.rs (namespace org.archel.settings, bare `Pin&lt;&amp;mut Self>` invokables per the 01-02-SUMMARY quirk): geometry qproperties + `save_geometry()` invokable reading window props passed from QML; Main.qml binds x/y/w/h/maximized from `shell` initial property and calls save on `ApplicationWindow.onClosing`; run_settings() loads geometry before engine.load() and exposes it. Vendored 10 simple MIT-licensed SVGs (Material-style) in qml/images/, registered in build.rs, referenced by rail buttons (D-25, agent's discretion; license noted in commit message). Extend tests/gui_shell.rs (`geometry` filter): round-trip save/read, corrupt/zeroed JSON => fallback, off-screen clamp — plain asserts, no new dev-deps.</action>
  <verify>
    <automated>QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui geometry &amp;&amp; cargo test -p archel --no-default-features &amp;&amp; qmllint $(find crates/archel/src/gui/qml -name '*.qml')</automated>
    <fails_when>geometry test fails round-trip, fallback, or clamp; writing window-geometry.json leaves a stray .tmp file; qmllint errors on Main.qml or image paths fail to resolve</fails_when>
  </verify>
  <acceptance_criteria>
  - Close at a non-default size/position -> next launch restores it (D-23); maximized state restored
  - Hand-corrupted JSON (NaN, zero, off-screen x) -> sane 900x620 centered window, never invisible (Pitfall 4)
  - Each rail button shows a 16x16 icon beside its French label (D-25)
  - Headless suite still compiles/tests no Qt
  </acceptance_criteria>
  <done>Geometry persists and restores with clamps; icons render; tests green.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| filesystem -> geometry loader | Hand-editable window-geometry.json crosses into the app |
| same-user processes -> abstract UDS socket | \0archel-settings-v1 bind namespace is claimable by any same-user process |
| QML <-> Rust bridge | Window props cross via ShellModel invokables |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-06-07 | Tampering | single_instance.rs second-instance probe | low | mitigate | Single bind() attempt is the test (no probe-then-bind TOCTOU, Pitfall 3); abstract socket dies with the process; refused instance cannot touch the live listener's state |
| T-06-09 | Tampering | geometry.rs JSON load | low | mitigate | serde-strict struct with clamps; any read/parse failure => None => safe fallback; off-screen/NaN geometry clamped against screens() (Pitfall 4); write is atomic temp+rename |
| T-06-17 | Spoofing | Abstract UDS name squatting by another user process | low | accept | Single-user local threat model; a squatter could make the real launch refuse — impact is annoyance, self-heals when squatter exits; revisit with daemon v2 |
| T-06-SC | Tampering | npm/pip/cargo installs | high | mitigate | No new packages this plan (RESEARCH Package Legitimacy Audit: none); vendored SVGs are reviewed MIT assets committed with license note |
</threat_model>

<verification>
- Full suite (VALIDATION.md): `cargo clippy -p archel-core --all-features -- -D warnings && cargo test -p archel-core --no-default-features && cargo test -p archel --no-default-features && QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui`
- Offscreen smoke green; qmllint sweep green.
</verification>

<success_criteria>
All three tasks' acceptance criteria green; shell boots offscreen; spike fully retired from the qrc surface.
</success_criteria>

## Artifacts this phase produces (this plan)

- `gui::run_settings()` real bootstrap; `fn main()` Settings branch; `single_instance::acquire()`; `geometry::{Geometry, load, save}`; `ShellModel` qobject; Main.qml shell; 19 registered QML files; `i18n::settings_already_running()` + `settings.already_running` catalog key; `crates/archel/tests/gui_shell.rs`.

<output>
When all tasks pass, create `.planning/phases/06-gui-shell-tabs/06-01-SUMMARY.md`.
</output>
