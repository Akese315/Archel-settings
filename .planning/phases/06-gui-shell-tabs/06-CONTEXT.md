# Phase 6: GUI Shell & Tabs - Context

**Gathered:** 2026-10-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 6 delivers `archel settings`: the CXX-Qt/QML graphical shell with a vertical tab rail presenting Landing + the 9 domain tabs (Network, Power, Bluetooth, Appearance, Sound, Apps/Packages, Display, Input, System Info), all as thin views over the CLI-proven `archel-core` API. Every read renders live data; every write action routes through the same core functions the CLI already proved, with confirmation modals, spinner/disabled states, and French error display. No new domain logic in the GUI — if a capability isn't in the core/CLI, it's a bug in the earlier phase, not GUI work.

</domain>

<decisions>
## Implementation Decisions

### Live-data rhythm
- **D-01:** Every tab polls live — not just Landing/System Info graphs. The researcher must prove the worker-thread pattern holds under ~10 concurrent polling sources without freezing the UI thread (the roadmap's no-freeze criterion).
- **D-02:** Uniform 1 Hz cadence for all live sources (matches the spike's Timer pattern — simplest mental model).
- **D-03:** All tabs keep their 1 Hz timers running even when hidden; only the visible tab is rendered, but switching is always to fresh data.
- **D-04:** When a source is offline/slow, its fields show honest French N/A immediately (same rule as missing sensors in Phase 2) — never a number that pretends to be fresh.
- **D-05:** First poll failure shows a dismissible banner in the tab; repeated failures stay silent (N/A fallback) until recovery. No per-tick error spam.
- **D-06:** Polling continues while the window is minimized or on another workspace — returning to the window always shows current data.
- **D-07:** Package update counts keep the CLI's 15-minute cache TTL (mirror-friendly); the 1 Hz rule applies to live sources only, not to `checkupdates`/AUR queries.
- **D-08:** Bluetooth discovery uses scan bursts (~12 s on tab open / refresh press, then stop), not continuous discovery — radio/battery respect. Standard BT-UX pattern.
- **D-09:** CPU/RAM graphs draw a rolling 60-second window fed by the 1 Hz ticks — bounded memory, spikes visible.
- **D-10:** CPU graph shows aggregate line + clock speed by default, with an expander revealing per-core lines (like the CLI's per-core view).
- **D-11:** Temperatures show representative values only (hottest per sensor class), consistent with Phase 2 D-09. No full sensor list.

### Mutation dialogs
- **D-12:** Every write action confirms in a modal dialog first — the GUI twin of the CLI's confirm-before-mutation rule (Phase 3 D-01). No silent writes, no inline-confirm exceptions for "simple" toggles. — **Reversibility:** costly — dropping confirmations later reopens the mutation-trust surface the CLI phase deliberately locked.
- **D-13:** Package install/upgrade/remove shows a modal with a scrolling terminal-like live view of pacman/yay output plus an Annuler button — mirroring the CLI's streaming output (Phase 3 D-02).
- **D-14:** Annuler during a running transaction requests a stop and reports pacman's final state; it never hard-kills the subprocess (lock/partial-transaction safety). The GUI reports how the transaction ended.
- **D-15:** During a long mutation only the acting tab locks; all other tabs stay usable (consistent with all-tabs-live — user can watch graphs during an upgrade).
- **D-16:** Display apply uses a countdown modal: 15 s progress bar with Conserver/Annuler — the direct GUI twin of the CLI confirm-and-rollback (Phase 5 D-04). Timeout auto-rolls back.
- **D-17:** Write failures surface in an error modal with a friendly French sentence + concrete cause and a Réessayer button. One consistent error voice across all tabs.
- **D-18:** pkexec password is asked up front, inside the confirmation modal, then the operation runs elevated in one shot — one interruption, predictable timing.

### Review surfaces
- **D-19:** AUR install shows a summary card (name, version, deps, sources) with a "Voir le PKGBUILD" expander containing the full PKGBUILD text. — **Reversibility:** costly — the Valider button stays disabled until the expander has been opened at least once, preserving Phase 3 D-05's full-display trust boundary in summary-first form.
- **D-20:** VPN import closes Phase 4 D-08's deferral: a popup form with file-picker for `.ovpn`/WireGuard `.conf` OR inline fields (username, password, certificate, key) — single popup, not a multi-step wizard.
- **D-21:** Bluetooth pairing-code confirmation is a global modal (pops anywhere in the app, showing the code with Confirmer/Refuser) — never auto-accept, can't be missed on another tab (GUI twin of Phase 4 D-05).

### Window chrome
- **D-22:** The window opens on the Landing tab (overview with update counts + system snapshot — GNOME Settings-like home).
- **D-23:** Window geometry (size, position, maximized state) is remembered in `~/.config/archel/` and restored on next launch; sensible default (~900×620, the spike size) on first run.
- **D-24:** Theme follows the system style hint (QT_QPA_PLATFORMTHEME / Hyprland setting) automatically — no in-app dark/light toggle in v1.
- **D-25:** The vertical rail shows an icon beside each French tab label (Qt built-in icons or simple inline SVGs). No settings search box in v1.
- **D-26:** A second `archel settings` launch prints a French message and exits (refuses the second instance) — no single-instance focus plumbing in v1.

### the agent's Discretion
- Bridge architecture (one QObject per tab vs. one root model + sub-models), QML file layout, and how the 1 Hz worker fan-out is structured (tokio tasks behind the bridge) are the planner's/executor's call — the spike's `#[cxx_qt::bridge]` pattern is the starting point, not a mandate.
- Icon sourcing (Qt Quick Controls built-ins vs. bundled SVGs) is agent's choice within D-25.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition & requirements
- `.planning/ROADMAP.md` (Phase 6 section) — success criteria, mode: mvp, UI hint yes
- `.planning/REQUIREMENTS.md` — PLAT-03 (GUI capability coverage) plus the per-tab requirement IDs the tabs present

### Prior-phase decisions the GUI must mirror
- `.planning/phases/01-foundations-safety-rails/01-CONTEXT.md` — D-02 (`archel settings` launches GUI), D-04/D-05/D-06 (English commands, French output everywhere incl. GUI), D-07 (French diacritics live in catalog/QML, never `.rs`), gui feature gating
- `.planning/phases/03-packages-cli-harness/03-CONTEXT.md` — D-01 confirm-before-mutation, D-02 live streaming, D-05 full PKGBUILD validation (GUI twin in D-19 above), D-06 yay unprivileged
- `.planning/phases/04-connectivity-devices/04-CONTEXT.md` — D-05 pairing code never auto-accepted, D-08 deferred VPN popup (closed by D-20 above)
- `.planning/phases/05-hyprland-desktop/05-CONTEXT.md` — D-04 15 s confirm-and-rollback (GUI twin in D-16), snippet/live-apply decisions the Appearance/Display tabs drive

### GUI foundation (proven in Phase 1)
- `.planning/phases/01-foundations-safety-rails/01-02-SUMMARY.md` — CXX-Qt spike: cargo_without_cmake build, QMAKE=/usr/bin/qmake6, gui feature gating, Spike.qml vertical-tab pattern (Column of TabButtons + StackLayout)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/archel/src/gui/bridge.rs` — working `#[cxx_qt::bridge]` QObject pattern (Q_PROPERTY + NOTIFY + qinvokable) to extend from one SpikeModel to per-tab view-models
- `crates/archel/src/gui/qml/Spike.qml` — proven vertical-tab layout (Column of TabButtons driving a StackLayout — TabBar is horizontal-only) and 1 Hz Timer→Rust tick pattern; the shell replaces this window
- `crates/archel/src/gui/mod.rs` — `run_settings()` entry (QGuiApplication + QQmlApplicationEngine); becomes the real shell bootstrap
- `crates/archel-core/src/` — every domain module (network, power, bluetooth, audio, landing, hyprland/*, packages, sysinfo paths) is the only data source; CLI snapshot types (`--json` structs) are ready-made view-model payloads
- `crates/archel/src/cmds/` — the CLI command implementations show exactly which core calls each tab should make (1:1 tab↔subcommand mapping)

### Established Patterns
- `gui` cargo feature isolates ALL Qt deps in the binary's gui path — headless builds compile no Qt; keep the new shell behind it
- French-only user text (D-06 Phase 1): QML strings French, `.rs` plain ASCII; error catalog already provides French sentences + causes
- Typed `ArchelError` (+ `NeedsPrivilege`) from core — GUI renders these uniformly per D-17/D-18 rather than inventing error strings
- `--json` output structs are serde-serializable — reusable as bridge payloads between Rust models and QML

### Integration Points
- `archel settings` dispatch in `crates/archel/src/main.rs` → `gui::run_settings()` (already wired for the spike)
- Long-running/polling work must live on the tokio runtime behind the bridge, never on the Qt main thread (the no-freeze criterion, D-01)
- Geometry persistence file joins the existing `~/.config/archel/` app config (directories crate)

</code_context>

<specifics>
## Specific Ideas

- "GNOME Settings-like home" for Landing (update counts + snapshot summary cards)
- Streaming terminal view is explicitly the GUI twin of the CLI's live pacman output
- Pairing modal must be genuinely global (user emphasized it can't be missed on another tab)
- Expander-before-validate for PKGBUILD keeps the Phase 3 trust boundary while being summary-first

</specifics>

<deferred>
## Deferred Ideas

- Settings search box above the rail — nice-to-have, cut from v1 (D-25)
- In-app dark/light toggle — v2 at earliest (system-follow locked for v1, D-24)
- Single-instance focus (second launch focuses existing window) — refused-instance behavior chosen instead (D-26)
- System tray / background watcher — belongs to a future daemon v2, not this phase

</deferred>

---

*Phase: 06-gui-shell-tabs*
*Context gathered: 2026-10-08*
