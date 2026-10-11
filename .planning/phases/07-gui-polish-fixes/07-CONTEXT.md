# Phase 7: GUI Polish Fixes - Context

**Gathered:** 2026-10-11
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 7 fixes the six acknowledged GUI code-review debts in the shipped `archel settings` GUI (CXX-Qt/QML shell + Rust bridge in `crates/archel`): Input tab tick overwrite (WR-01), FileDialog percent-encoded paths (WR-02), global confirm multiplexing (WR-03), keybind-add persist ordering (WR-04), missing automatic-DNS path (WR-05), and the camelCase/snake_case QML binding convention (06-06 deviation + WINDOWS.md gate). Thin fixes over CLI-proven core — no new capabilities, no new core domain logic unless a fix requires it. Requirements GUI-01…GUI-06.

</domain>

<decisions>
## Implementation Decisions

### Input editing (GUI-01 / WR-01)
- **D-01:** Input tab uses draft-local editor state mirroring the NetTab pattern — the 1 Hz tick keeps refreshing read-only displays behind the edit, and Appliquer writes the draft. No tick-freezing.
- **D-02:** If the underlying value changes externally mid-edit (e.g. `hyprland.conf` edited elsewhere), the external value wins — the draft is replaced, not merged.
- **D-03:** Appliquer is enabled only when the draft differs from live state and validates (same enablement rule as NetTab).
- **D-04:** Returning to the Input tab with an unapplied draft shows a popup offering discard-or-apply — the draft is never silently reverted nor silently kept.

### Confirm sessions (GUI-03 / WR-03)
- **D-05:** Confirms are parallel and per-tab — two tabs may each hold a pending write; no single write-slot guard blocks the second.
- **D-06:** A second confirm modal stacks above the first; each resolves independently.
- **D-07:** Write failures keep the existing global error modal (one consistent error voice, per Phase 6 D-17) — results are not per-tab toasts.
- **D-08:** Cancelling a stacked modal dismisses only the top modal; older pending confirms stay stacked.

### Automatic DNS (GUI-05 / WR-05)
- **D-09:** NetTab gains an "Automatique" choice that maps to the CLI `reset` leg (automatic DNS) — a real GUI affordance, not a CLI-only annotation.
- **D-10:** Selecting "Automatique" goes through the standard confirm modal (Phase 6 D-12 rule holds) — no inline-apply exception.
- **D-11:** The GUI remembers the last manual DNS servers for the session and pre-fills them when switching back from automatic to manual.
- **D-12:** "Automatique" applies to the active connection profile only; per-profile behavior is otherwise unchanged.

### QML casing convention (GUI-06 / 06-06 deviation)
- **D-13:** camelCase QML bindings stay — the convention is "QML binds camelCase, bridge qproperties stay snake_case (Qt case-folding)", documented as the going-forward rule. No bridge rename. — **Reversibility:** costly — renaming later would touch the bridge plus every tab file and all binding call sites at once.
- **D-14:** The cleanup pass converts all ~8 affected tab files to the convention now so the WINDOWS.md gate closes in this phase — not opportunistic.
- **D-15:** Enforcement lands in `just gui-check` (qmllint plus a binding-spell check); a violation blocks merge.

### the agent's Discretion
- WR-02 (`toLocalFile()` picker fix) and WR-04 (keybind persist-then-eval ordering) had no user-facing ambiguity — implementation approach is the planner's/executor's call within the existing Phase 6 decisions (D-20 picker popup, live-eval + snippet durability).
- Exact popup wording (French strings) follows the existing rust-i18n catalogs; no new tone decisions needed.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase requirements
- `.planning/REQUIREMENTS.md` — GUI-01…GUI-06 definitions (v1.1 scope, traceability)
- `.planning/ROADMAP.md` — Phase 7 goal, success criteria, ordering (Phase 7 → 8 → 9)

### Debt source records
- `.planning/milestones/v1.0-phases/06-gui-shell-tabs/deferred-items.md` — WR-01…WR-05, IN-01…IN-03, 06-06 deviation + flaky-test/clippy entries as written at v1.0 close
- `.planning/milestones/v1.0-phases/06-gui-shell-tabs/REVIEW-waves-6-7.md` — original code-review findings behind WR-01…WR-05
- `.planning/WINDOWS.md` — open gate item on camelCase/snake_case bindings (must close in this phase)

### Prior GUI decisions (constraints these fixes must preserve)
- `.planning/milestones/v1.0-phases/06-gui-shell-tabs/06-CONTEXT.md` — Phase 6 D-01…D-26 (1 Hz cadence D-02, modal-confirm D-12, display countdown D-16, error voice D-17, picker popup D-20, pairing modal D-21)

### Code under repair
- `crates/archel/src/gui/qml/tabs/InputTab.qml` — binds tick values directly; needs draft-state decoupling
- `crates/archel/src/gui/qml/tabs/NetTab.qml` — reference pattern (local DNS edit state, Appliquer enablement) + home of the new "Automatique" choice
- `crates/archel/src/gui/qml/tabs/PkgTab.qml` — WINDOWS.md gate file; casing cleanup exemplar

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- NetTab local DNS edit state (`crates/archel/src/gui/qml/tabs/NetTab.qml`) — the draft-state + Appliquer-enablement pattern D-01/D-03 copies into InputTab
- Global confirm session + error modal — the existing modal infrastructure D-05…D-08 splits per-tab while D-07 keeps the shared error voice
- `just gui-check` — existing GUI gate (build + qmllint + offscreen smoke + gui tests) that D-15 extends with a binding-spell check

### Established Patterns
- 1 Hz poll cadence on all tabs (Phase 6 D-02) — fixes must preserve live freshness, never stop the tick
- Modal-confirm for every write (Phase 6 D-12) — no inline-apply exceptions, including the new "Automatique" choice
- French-first strings via rust-i18n catalogs — popup/affordance wording follows existing catalogs

### Integration Points
- Fixes touch QML tabs + the Rust CXX-Qt bridge + (only if needed) thin `archel-core` read paths; CLI behavior is the unchanged reference — any GUI/core divergence found during the fixes is a bug to fix, not a redesign

</code_context>

<specifics>
## Specific Ideas

- InputTab draft popup on tab-return: "something like that" — a discard-or-apply choice; exact French wording follows existing catalog tone
- No specific visual references — fixes preserve the shipped v1.0 look

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 7-GUI Polish Fixes*
*Context gathered: 2026-10-11*
