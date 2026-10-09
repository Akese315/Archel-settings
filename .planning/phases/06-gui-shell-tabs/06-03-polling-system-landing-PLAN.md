---
phase: "06"
plan: "03"
type: execute
wave: 2
depends_on: ["06-01"]
files_modified:
  - crates/archel/src/gui/poll.rs
  - crates/archel/src/gui/models/mod.rs
  - crates/archel/src/gui/models/system.rs
  - crates/archel/src/gui/models/landing.rs
  - crates/archel/src/gui/bridge.rs
  - crates/archel/src/gui/mod.rs
  - crates/archel/src/gui/qml/tabs/SystemTab.qml
  - crates/archel/src/gui/qml/tabs/LandingTab.qml
  - crates/archel/src/gui/qml/components/Sparkline.qml
  - crates/archel/src/gui/qml/components/PollBanner.qml
  - crates/archel/src/gui/qml/components/StatField.qml
autonomous: true
requirements: [PLAT-03]
estimate:
  tokens: 63000
  raw_tokens: 45000
  tasks: 3
  confidence: low
must_haves:
  truths:
    - "Every ticking model refreshes at most one request in flight — the 1 Hz Timer tick is skipped while the previous call has not returned (D-02); stale results are dropped by a sequence tag so an old answer never overwrites a newer one"
    - "System Tab shows live 1 Hz CPU and RAM Canvas sparklines over a 60 s rolling window, aggregate line plus per-core expander, representative temps only, and the UI never freezes (D-09/D-10/D-11, success criterion 2)"
    - "Landing shows update counts from the 15-min-cached pending_counts(false) path without ever blocking (D-07)"
    - "Before the first successful tick every value reads the ellipsis token; N/A appears only after a failure — never a stale or zero-faked number (D-04)"
    - "The first poll failure shows the dismissible 'Données indisponibles pour cette section.' banner once, later failures stay silent (D-05)"
    - "Missing sensors render N/A at 40% opacity (UI-SPEC partial row, Phase 2 honesty rule)"
    - "A 1-core CPU hides the per-core expander and shows only the aggregate line (UI-SPEC zero-one-many row, D-10)"
  artifacts:
    - crates/archel/src/gui/poll.rs
    - crates/archel/src/gui/models/mod.rs
    - crates/archel/src/gui/models/system.rs
    - crates/archel/src/gui/models/landing.rs
    - "Sparkline.qml / PollBanner.qml / StatField.qml filled from placeholders"
  key_links:
    - "QML Timer (running: tab is current index) -> model.tick() -> RUNTIME.handle().spawn -> CxxQtThread.queue closure -> Q_PROPERTY set (queue() is the ONLY place properties mutate — RESEARCH Pattern 1)"
    - "SystemModel <-> archel_core::system::Monitor (stateful, Mutex-guarded, driven only from the spawn_blocking worker for a real 1 s CPU window)"
    - "models/mod.rs registration <-> gui/mod.rs register call (one registration site; later plans only append their own lines)"
prohibitions:
  - "Only the currently visible tab may tick; a hidden tab's Timer must be stopped — no background polling of invisible tabs (D-03)"
  - "Poll workers must never touch QML-visible state off the queue — property writes only inside CxxQtThread.queue closures"
---

<objective>
Build the reusable polling layer (in-flight guard, sequence tags, banner policy) and deliver the first two real tabs — System Info (1 Hz sparklines, per-core expander, temps) and Landing (cached update counts) — plus the shared StatField/PollBanner/Sparkline components and the models/ architecture every later tab plan copies.

Purpose: Success criterion 2 (live graphs, no freeze) and the D-01..D-05 polling contract; establishes the view-model pattern that keeps PLAT-03's "thin view" guarantee auditable.
Output: poll.rs, models/{mod,system,landing}.rs, three real components, two live tabs.
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
@.planning/phases/06-gui-shell-tabs/06-01-SUMMARY.md
@crates/archel/src/gui/bridge.rs
@crates/archel/src/gui/mod.rs
@crates/archel-core/src/system.rs
@crates/archel-core/src/landing.rs
</context>

<tasks>

<task type="auto" id="06-03-T1">
  <name>Task 1: Polling layer + model architecture + component shells</name>
  <read_first>crates/archel/src/gui/bridge.rs (SpikeModel qobject + bridge declaration pattern), crates/archel/src/gui/mod.rs (registration call site), 06-RESEARCH.md (Pattern 1, Pitfalls 1/7 — queue() and in-flight guard), 06-PATTERNS.md (poll.rs and bridge.rs sections), crates/archel/src/cmds/mod.rs:768 (inline cfg(test) convention)</read_first>
  <files>crates/archel/src/gui/poll.rs, crates/archel/src/gui/models/mod.rs, crates/archel/src/gui/mod.rs, crates/archel/src/gui/bridge.rs, crates/archel/src/gui/qml/components/{Sparkline,PollBanner,StatField}.qml</files>
  <action>New poll.rs: helper that takes the OnceLock runtime handle, a per-model in-flight flag and a sequence counter, spawns a blocking core call, and marshals the result back EXCLUSIVELY through CxxQtThread.queue (RESEARCH Pattern 1; the queue closure is the only code allowed to set properties). Drop any result whose sequence tag is older than the newest dispatched one (D-02 stale-drop). Expose a small struct (e.g. PollGuard { busy: AtomicBool, seq: AtomicU64 }) models embed; tick() returns early when busy (skipped tick, D-02). New gui/models/ directory with models/mod.rs declaring the module list plus one register_models(engine) called from gui/mod.rs — later tab plans only append their own model lines there. Remove the leftover SpikeModel from bridge.rs (06-01 left it compiling; this plan deletes it — bridge.rs keeps only ShellModel). Fill the three component placeholders: StatField.qml (Label pair with N/A-at-40%-opacity and ellipsis states per UI-SPEC loading/partial rows), PollBanner.qml (dismissible Frame with the exact UI-SPEC copywriting-contract sentence, only shown once per session per model — D-05), Sparkline.qml (Canvas drawing a normalized polyline from a ListModel/variant-list of floats, 60-sample window, accent color token; QtCharts is NOT installed — Canvas only). Unit tests inline in poll.rs (guard skip, sequence drop) using plain #[cfg(test)] mod tests.</action>
  <verify>
    <automated>QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui poll && cargo test -p archel --no-default-features</automated>
    <fails_when>poll guard tests fail; headless suite fails or pulls Qt</fails_when>
    <automated>qmllint crates/archel/src/gui/qml/components/Sparkline.qml crates/archel/src/gui/qml/components/PollBanner.qml crates/archel/src/gui/qml/components/StatField.qml && ! grep -rn 'SpikeModel' crates/archel/src</automated>
    <fails_when>qmllint errors on a component; any SpikeModel reference remains in crates/archel/src</fails_when>
  </verify>
  <acceptance_criteria>
  - PollGuard skips a tick while in flight and drops stale sequences (tested)
  - SpikeModel gone from bridge.rs; shell still boots (06-01's offscreen smoke stays green)
  - Components pass qmllint and render the loading/N/A/banner states defined in UI-SPEC
  </acceptance_criteria>
  <done>Polling machinery + components exist, tested, spike fully retired.</done>
</task>

<task type="auto" id="06-03-T2">
  <name>Task 2: SystemModel + SystemTab (1 Hz graphs, D-09/D-10/D-11)</name>
  <read_first>crates/archel-core/src/system.rs (Monitor::new/cpu/ram/disk, cpu_snapshot/ram_snapshot/disk_snapshot, clamp_usage/bytes_to_gib/mhz_to_ghz), crates/archel/src/cmds/sys_cmds.rs (run_cpu/run_temp/run_all — the CLI entry point this tab mirrors, incl. the sensors temp path), crates/archel/src/gui/qml/Spike.qml history (Timer + queue pattern, from 06-01-SUMMARY), 06-UI-SPEC.md (Sparkline rows, zero-one-many row, D-09/D-10/D-11 contracts), 06-RESEARCH.md (Pitfall 1 — never block the Qt loop)</read_first>
  <files>crates/archel/src/gui/models/system.rs, crates/archel/src/gui/models/mod.rs, crates/archel/src/gui/qml/tabs/SystemTab.qml</files>
  <action>SystemModel qobject (own #[cxx_qt] bridge mod in this file, registered via models/mod.rs — mirror SpikeModel's declaration pattern from bridge.rs): embeds Mutex<archel_core::system::Monitor> + PollGuard; tick() invokable spawns blocking cpu/ram/disk refresh on the runtime (the 1 s CPU window therefore never blocks Qt — Pitfall 1), queue() sets typed Q_PROPERTYs (cpu aggregate %, ram used/total GiB via bytes_to_gib, disk used/total, kernel_version) and a serde_json QString for the per-core array + rolling 60-sample series (VecDeque capacity 60, D-09). Representative temps only (D-11): reuse the exact core read path archel sys temp uses (sensors/TempSnapshot — read sys_cmds.rs:103 and call the same core fn); GPU line only when the GPU snapshot yields data; missing values => N/A token property, never 0 (D-04). SystemTab.qml: Timer running ONLY when Main.qml currentIndex is this tab (D-03), interval 1000ms, calls system.tick(); two Sparkline items bound to the series properties, aggregate stat rows via StatField, per-core expander (DisclosureRow hidden when cores.len() == 1 — zero-one-many row). Inline tests: series cap at 60, N/A on Err, core-count logic.</action>
  <verify>
    <automated>QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui system_model && QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui poll</automated>
    <fails_when>system model tests fail (series overflow, stale overwrite, N/A not emitted)</fails_when>
    <automated>QT_QPA_PLATFORM=offscreen timeout 10 ./target/debug/archel settings &lt;/dev/null >/tmp/archel-systab.log 2>&1; rc=$?; test $rc -eq 124 && ! grep -qiE 'qml:|is not loaded|undefined' /tmp/archel-systab.log</automated>
    <fails_when>offscreen run dies early or QML logs warnings/errors for SystemTab bindings</fails_when>
  </verify>
  <acceptance_criteria>
  - System tab shows moving 1 Hz CPU/RAM sparklines (60 s window) without freezing the window (success criterion 2)
  - Per-core expander present, hidden for 1-core; representative temps only (D-11); N/A honest states (D-04)
  - Hidden-tab Timer provably stopped (running bound to current index, D-03)
  </acceptance_criteria>
  <done>System Info tab live at 1 Hz with graphs, expander, honest N/A behavior.</done>
</task>

<task type="auto" id="06-03-T3">
  <name>Task 3: LandingModel + LandingTab (cached update counts, D-07)</name>
  <read_first>crates/archel-core/src/landing.rs (snapshot(false), LandingSnapshot, updates_cell, checker_notes), crates/archel-core/src/updates.rs:331 (pending_counts(false) 15-min cache), crates/archel/src/cmds/landing_cmd.rs (the CLI entry point this tab mirrors), 06-UI-SPEC.md (Landing rows + spacing 2xl/3xl hero block)</read_first>
  <files>crates/archel/src/gui/models/landing.rs, crates/archel/src/gui/models/mod.rs, crates/archel/src/gui/qml/tabs/LandingTab.qml</files>
  <action>LandingModel qobject (same bridge-mod-per-file pattern): tick() -> spawn_blocking landing::snapshot(false) (cache-backed — never forces an alpm refresh, D-07; in-flight guard from PollGuard), queue() sets properties: pending updates count, checker notes (checker_notes joined), uptime/kernel summary fields as the snapshot exposes them. LandingTab.qml: hero block (2xl/3xl spacing rows), update counts via StatField with the ellipsis-until-first-tick rule, dismissible PollBanner wired to the model's banner-once signal (D-05), Timer running only while the Landing index is current (D-03; Landing is the default tab so it ticks at launch). Inline tests: snapshot Err -> N/A properties + banner flag; success -> counts populated; second consecutive failure does not re-signal the banner.</action>
  <verify>
    <automated>QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui landing</automated>
    <fails_when>landing model tests fail (banner re-fires, error path fakes zero counts)</fails_when>
    <automated>QT_QPA_PLATFORM=offscreen timeout 10 ./target/debug/archel settings &lt;/dev/null >/tmp/archel-landing.log 2>&1; rc=$?; test $rc -eq 124 && ! grep -qiE 'qml:|is not loaded' /tmp/archel-landing.log</automated>
    <fails_when>offscreen smoke dies or logs LandingTab binding errors</fails_when>
  </verify>
  <acceptance_criteria>
  - Landing renders update counts from the cached path within one tick of opening; no forced refresh (D-07)
  - Banner appears once on first failure, dismissible, silent afterwards (D-05)
  - Both new tabs keep the offscreen smoke green
  </acceptance_criteria>
  <done>Landing tab live from the cache; the model/poll pattern is proven for the six later tab plans.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| sysfs/sensors output -> QML display | Kernel-exposed metric strings flow into the UI |
| core Err -> banner/property text | Error strings from archel-core cross into QML-visible state |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-06-19 | Tampering | stale poll results racing a newer tick | medium | mitigate | Sequence tag on every dispatch; queue() applies only the newest tag (tested); in-flight skip prevents overlapping requests (D-02) |
| T-06-20 | Denial of Service | 1 Hz tick blocking the Qt event loop | high | mitigate | All core calls via runtime spawn_blocking; queue() only marshals finished values (Pitfall 1); offscreen smoke asserts the window stays alive |
| T-06-21 | Information Disclosure | sysfs/error text rendered raw in QML | low | accept | Values are read-only kernel counters and catalog-backed error strings; QML Label never interprets markup; single-user local app |
| T-06-SC | Tampering | crate installs | high | mitigate | No new packages this plan (RESEARCH audit: none) — pure cxx-qt/tokio/serde reuse |
</threat_model>

<verification>
- Plan suite: `cargo test -p archel-core --no-default-features && cargo test -p archel --no-default-features && QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui` + qmllint sweep + offscreen smoke (VALIDATION.md commands)
</verification>

<success_criteria>
Both tabs tick live at 1 Hz on screen with zero freezes; guard/sequence/banner tests green; spike fully gone.
</success_criteria>

## Artifacts this phase produces (this plan)

- `gui/poll.rs` (PollGuard + spawn/queue helper), `gui/models/` architecture, `SystemModel`, `LandingModel`, Sparkline.qml, PollBanner.qml, StatField.qml, per-file bridge-mod model pattern reused by plans 04-08.

<output>
When all tasks pass, create `.planning/phases/06-gui-shell-tabs/06-03-SUMMARY.md`.
</output>
