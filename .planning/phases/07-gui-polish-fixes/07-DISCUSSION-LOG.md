# Phase 7: GUI Polish Fixes - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-11
**Phase:** 7-GUI Polish Fixes
**Areas discussed:** Input editing UX, Confirm behavior, Auto-DNS option, QML casing rule

---

## Input editing UX

| Option | Description | Selected |
|--------|-------------|----------|
| Tick while typing: Draft state (NetTab) | Edits live in local fields; tick refreshes read-only displays behind; Appliquer writes | ✓ |
| Tick while typing: Freeze tick on focus | Tick pauses for the focused control while typing; resumes on blur/apply | |
| Tick while typing: You decide | Leave mechanism to implementer | |
| External change mid-edit: Take external value | External change replaces the draft and notifies | ✓ |
| External change mid-edit: Keep draft, hint | Typing stays untouched; hint notes fresh data available | |
| Appliquer enablement: Apply on change | Enabled only when draft differs and validates (NetTab rule) | ✓ |
| Appliquer enablement: Always enabled | Always clickable; no-op apply when nothing changed | |
| Tab switch with draft: popup (freeform) | Popup on return offering discard-or-apply | ✓ |
| Tab switch with draft: Revert to live | Abandon draft, show tick value | |
| Tab switch with draft: Keep draft | Draft stays for deliberate apply | |

**User's choice:** Draft state (NetTab) / Take external value / Apply on change / popup message (discard or apply) "something like that" (freeform)
**Notes:** User described the tab-return behavior in own words instead of picking listed options; captured as D-04 popup.

---

## Confirm behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Parallel confirms | Each tab runs its own modal flow; two tabs can confirm at once | ✓ |
| Single write slot | One write at a time; second confirm waits or is refused | |
| Second modal: Stack modals | Second modal stacks above; each resolves independently | ✓ |
| Second modal: Replace older | New request replaces the older pending one | |
| Result surface: Global error modal | One shared error modal regardless of acting tab | ✓ |
| Result surface: Per-tab result | Tab A's result shows in Tab A only | |
| Cancel: Close top only | Only the top modal closes; older pending confirms stay | ✓ |
| Cancel: Cancel clears all | One cancel clears the whole stack | |

**User's choice:** Parallel confirms / Stack modals / Global error modal / Close top only
**Notes:** None.

---

## Auto-DNS option

| Option | Description | Selected |
|--------|-------------|----------|
| Add Auto option | NetTab gains 'Automatique' mapping to the CLI reset leg | ✓ |
| Mark CLI-only | No GUI change; note reset is CLI-only | |
| Apply: Confirm modal | One click restores auto DNS via standard confirm modal | ✓ |
| Apply: Apply inline | Auto choice applies immediately when selected | |
| Back to manual: Remember last | Pre-fill last manual servers for the session | ✓ |
| Back to manual: No memory | Manual starts empty | |
| Scope: Current profile | Applies to the active connection profile only | ✓ |

**User's choice:** Add Auto option / Confirm modal / Remember last / Current profile
**Notes:** None.

---

## QML casing rule

| Option | Description | Selected |
|--------|-------------|----------|
| Keep camelCase | Existing camelCase bindings stay; documented convention going forward | ✓ |
| Rename bridge | Rename bridge qproperties so spellings match exactly | |
| Convert all now | Cleanup pass converts every tab file now; gate closes in this phase | ✓ |
| Going forward | New code follows rule; old files convert opportunistically | |
| Gate in gui-check | qmllint plus binding-spell check in just gui-check; red blocks merge | ✓ |
| Rule on paper | Written rule only, no automated check | |

**User's choice:** Keep camelCase / Convert all now / Gate in gui-check
**Notes:** None.

---

## the agent's Discretion

WR-02 (`toLocalFile()` picker fix) and WR-04 (keybind persist-then-eval ordering) had no user-facing ambiguity — implementation approach left to planner/executor. Exact French popup wording follows existing catalogs.

## Deferred Ideas

None — discussion stayed within phase scope.
