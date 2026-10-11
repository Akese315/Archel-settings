# Phase 7: GUI Polish Fixes - Research

**Researched:** 2026-10-11
**Domain:** CXX-Qt/QML GUI repair — draft-state editing, FileDialog paths, modal confirm sessions, keybind write ordering, DNS reset affordance, QML casing convention
**Confidence:** HIGH

## Summary

Phase 7 fixes six acknowledged v1.0 GUI code-review debts (WR-01…WR-05 + the 06-06 casing deviation / WINDOWS.md gate) in the shipped `archel settings` GUI. All six fixes were traced to their exact code sites this session: the tick-bound editors in `InputTab.qml`, the two `file://`-strip picker sites, the single global `ModalSession::request_confirm` that resets any prior session, the eval-before-persist order in `add_keybind_worker`, the missing GUI leg for the existing core `plan_dns_reset`, and the camelCase-vs-snake_case split across tab files. The CLI-proven core already contains every backend leg the GUI needs (DNS reset plan/prompt/apply, keybind plan/validate, VPN path validation) — no new core domain logic is expected, only thin GUI/bridge wiring.

**Primary recommendation:** Implement the six fixes as thin QML + bridge + worker-seam changes reusing the three proven in-repo patterns (NetTab draft-state, ModalSession mutex-session + `sync_session` mirror, validate-plan-confirm-apply workers), with all verification through the hand-mirrored `just gui-check` recipes using `--offline` (offline machine, `just` itself is not installed).

## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Input editing (GUI-01 / WR-01)
- **D-01:** Input tab uses draft-local editor state mirroring the NetTab pattern — the 1 Hz tick keeps refreshing read-only displays behind the edit, and Appliquer writes the draft. No tick-freezing.
- **D-02:** If the underlying value changes externally mid-edit (e.g. `hyprland.conf` edited elsewhere), the external value wins — the draft is replaced, not merged.
- **D-03:** Appliquer is enabled only when the draft differs from live state and validates (same enablement rule as NetTab).
- **D-04:** Returning to the Input tab with an unapplied draft shows a popup offering discard-or-apply — the draft is never silently reverted nor silently kept.

#### Confirm sessions (GUI-03 / WR-03)
- **D-05:** Confirms are parallel and per-tab — two tabs may each hold a pending write; no single write-slot guard blocks the second.
- **D-06:** A second confirm modal stacks above the first; each resolves independently.
- **D-07:** Write failures keep the existing global error modal (one consistent error voice, per Phase 6 D-17) — results are not per-tab toasts.
- **D-08:** Cancelling a stacked modal dismisses only the top modal; older pending confirms stay stacked.

#### Automatic DNS (GUI-05 / WR-05)
- **D-09:** NetTab gains an "Automatique" choice that maps to the CLI `reset` leg (automatic DNS) — a real GUI affordance, not a CLI-only annotation.
- **D-10:** Selecting "Automatique" goes through the standard confirm modal (Phase 6 D-12 rule holds) — no inline-apply exception.
- **D-11:** The GUI remembers the last manual DNS servers for the session and pre-fills them when switching back from automatic to manual.
- **D-12:** "Automatique" applies to the active connection profile only; per-profile behavior is otherwise unchanged.

#### QML casing convention (GUI-06 / 06-06 deviation)
- **D-13:** camelCase QML bindings stay — the convention is "QML binds camelCase, bridge qproperties stay snake_case (Qt case-folding)", documented as the going-forward rule. No bridge rename. — **Reversibility:** costly — renaming later would touch the bridge plus every tab file and all binding call sites at once.
- **D-14:** The cleanup pass converts all ~8 affected tab files to the convention now so the WINDOWS.md gate closes in this phase — not opportunistic.
- **D-15:** Enforcement lands in `just gui-check` (qmllint plus a binding-spell check); a violation blocks merge.

### the agent's Discretion
- WR-02 (`toLocalFile()` picker fix) and WR-04 (keybind persist-then-eval ordering) had no user-facing ambiguity — implementation approach is the planner's/executor's call within the existing Phase 6 decisions (D-20 picker popup, live-eval + snippet durability).
- Exact popup wording (French strings) follows the existing rust-i18n catalogs; no new tone decisions needed.

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| GUI-01 | Input tab edits survive the 1 Hz poll tick — editor state decoupled like NetTab (local state, write on Appliquer) (WR-01) | NetTab draft pattern (`dnsRows`/`syncDnsFromCurrent`/`refreshDnsState`); D-01…D-04 design rules; base-compare technique for D-02 |
| GUI-02 | VPN + wallpaper FileDialog pickers resolve percent-encoded paths via `toLocalFile()` (WR-02) | Two exact sites (`VpnImportPopup.qml:33`, `AppearanceTab.qml:32`); `QUrl.toLocalFile()` replacement [ASSUMED Qt API] |
| GUI-03 | Confirm sessions are per-tab (or generation-tagged) — no cross-tab multiplexing on the single global session (WR-03) | `ModalSession::request_confirm` reset semantics; confirm-stack extension design; D-05…D-08 stacking rules |
| GUI-04 | Keybind-add persists before (or compensates after) live-eval — no unevaluated snippet line left behind on post-eval persist failure (WR-04) | `add_keybind_worker` eval-then-persist order; `remove_keybind_worker` post-confirm re-read as the ordering exemplar |
| GUI-05 | GUI exposes a path back to automatic DNS (maps the CLI `reset` leg) or annotates it CLI-only (WR-05) | Core `plan_dns_reset` + `confirm_dns_prompt` reset leg + CLI `run_dns_reset` all exist; needs `dnsReset` invokable + "Automatique" UI (D-09…D-12) |
| GUI-06 | QML/bridge casing convention applied across tab files and the WINDOWS.md gate closed (06-06 deviation) | Bridge snake_case qproperties vs QML bindings inventory; PkgTab is the snake_case outlier; D-13…D-15 rule + gate |

## Project Constraints (from AGENTS.md)

- Rust + Qt6 GUI via CXX-Qt; QML-from-Rust (not Widgets) — fixes stay in QML + bridge, no new toolkit.
- Arch Linux + Hyprland (Wayland) only — no X11-only helpers; verify offscreen (`QT_QPA_PLATFORM=offscreen`).
- `archel-core` is the only data source; GUI is a thin view — any GUI/core divergence found is a bug, not a redesign.
- French-only user text: QML strings in French, `.rs` files plain ASCII (diacritics live in catalog/QML, enforced by `i18n::tests::no_hardcoded_french_diacritics_in_rust_sources`).
- `gui` cargo feature isolates all Qt deps; headless builds compile no Qt.
- Offline machine: `cargo ... --offline` mandatory; `just` is not installed — mirror recipes by hand.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Draft-local editor state (InputTab) | GUI (QML) | — | Pure view state; tick keeps refreshing read-only displays behind it (D-01) |
| Confirm session stack | GUI bridge (Rust `ModalSession`) | QML modal layer | Workers block on `mpsc` receivers; Qt-thread mirror via `sync_session` (threading rule) |
| FileDialog path decode | GUI (QML) | — | `selectedFile` is a `QUrl`; decode at the binding site |
| Keybind persist/eval ordering | GUI bridge worker (Rust) | Core (`binds::`) | Reorder existing injected-op calls; core plan/validate/apply unchanged |
| Automatic-DNS affordance | GUI (QML + bridge invokable) | Core (`network::dns::`) | Core reset leg already CLI-proven; GUI only adds the `dnsReset` path |
| Casing convention + gate | GUI (QML) + `justfile` | — | No bridge rename (D-13); lint gate blocks merge (D-15) |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| CXX-Qt (`cxx-qt`, `cxx-qt-lib`, `cxx-qt-build`) | 0.10.0 [ASSUMED — pinned by prior STACK.md, not re-verified; offline] | Qt6-from-Rust bridge (`#[cxx_qt::bridge]`, qproperties, qinvokables) | Locked project stack; all tab models already built on it |
| Qt6 (`qt6-base`, `qt6-declarative`) | system (Arch extra) [ASSUMED] | QML engine, Controls, Dialogs, `FileDialog`, `QUrl.toLocalFile()` | Only GUI toolkit allowed |
| `archel-core` (`network::dns`, `hyprland::binds`, `hyprland::input`, `network::vpn`) | workspace (in-repo) [VERIFIED: crates/archel-core/src/network.rs:1055-1189] | CLI-proven plan/validate/apply legs the fixes reuse | Core Value: GUI is a thin consumer; no new domain logic |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| rust-i18n catalogs (`net.confirm_dns_reset`, `net.dns_reset_title`, …) | in-repo [VERIFIED: crates/archel-core/src/network.rs:1152-1158, 1215-1225 region] | French confirm/prompt wording for the new "Automatique" flow | Any new user-facing string in this phase |
| qmllint (`/usr/bin/qmllint`) | present on machine [VERIFIED: `command -v` this session] | QML lint sweep in `gui-check` | Every QML edit; plus the new D-15 binding-spell check |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Confirm-stack in `ModalSession` | Per-tab confirm state inside each tab model | Per-tab state duplicates the session/mirror plumbing ×N tabs and bypasses the single `sync_session` path; the stack keeps one mirror and one QML modal layer. Prefer the stack. |
| `selectedFile.toLocalFile()` | Manual `decodeURIComponent(strip("file://"))` | Manual decode re-implements `QUrl` parsing (host prefix, `+`, multi-byte) and will rot; `toLocalFile()` is the Qt-canonical conversion [ASSUMED — Qt API from training, not verified against docs this session (offline)]. Prefer `toLocalFile()`. |
| Persist-then-eval (WR-04) | Eval-then-persist + compensating unbind on persist failure | Compensating unbind adds a second live-compositor mutation and its own failure mode; persist-then-eval keeps the failure pre-live (nothing to compensate). Planner's call per CONTEXT discretion; research leans persist-then-eval. |

**Installation:** No new packages. Zero dependency changes in this phase.

**Version verification:** No registry lookups needed — no external packages are added. (Offline machine; `cargo --version` = 1.89.1 [VERIFIED: shell probe this session].)

## Package Legitimacy Audit

No external packages are installed in this phase. All fixes reuse in-repo code (`archel-core`, `ModalSession`, QML) and the locked CXX-Qt/Qt6 stack.

| Package | Registry | Verdict | Disposition |
|---------|----------|---------|-------------|
| *(none)* | — | — | No installs; audit not applicable |

**Packages removed due to SLOP verdict:** none.
**Packages flagged as suspicious (SUS):** none.

## Architecture Patterns

### System Architecture Diagram

```
┌─ QML tabs (1 Hz Timer → tick) ─────────────────────────────┐
│ InputTab (draft-local editors)  NetTab (dnsRows draft)      │
│ VpnImportPopup / wallpaper FileDialog (toLocalFile)         │
│ ConfirmModal ×N (stacked, per pending confirm)              │
│ ErrorModal ×1 (global, unchanged)  discard/apply popup      │
└──────────▲───────────────────────────────▲─────────────────┘
           │ qinvokables (tick/dnsApply/    │ 120 ms syncSession
           │ dnsReset/addKeybind/…)         │ mirror (Qt thread only)
┌──────────┴───────────────────────────────┴─────────────────┐
│ Rust bridge (models/*.rs + modal::ModalSession)             │
│  - draft base-compare (D-02)                                │
│  - confirm STACK: Vec<(question, tx)> — reset() no longer   │
│    drops sibling confirms                                   │
│  - workers: validate → plan → confirm → apply (blocking pool│
│    + CxxQtThread.queue for property writes)                 │
└──────────▲─────────────────────────────────────────────────┘
           │ injected ops (NetDnsOps / InputOps / NetVpnOps)
┌──────────┴─────────────────────────────────────────────────┐
│ archel-core (CLI-proven, UNCHANGED): plan_dns_set/reset,    │
│ confirm_dns_prompt, apply_dns_plan, binds plan/validate,    │
│ vpn validate_path/infer, input plan/apply                   │
└────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure

No new files required by default. Expected touch surface (planner confirms):

```
crates/archel/src/gui/qml/tabs/InputTab.qml      # draft-state decoupling (GUI-01)
crates/archel/src/gui/qml/tabs/NetTab.qml        # "Automatique" choice (GUI-05)
crates/archel/src/gui/qml/tabs/PkgTab.qml        # snake→camel exemplar (GUI-06)
crates/archel/src/gui/qml/tabs/*.qml             # ~8-file casing pass (GUI-06)
crates/archel/src/gui/qml/components/
  VpnImportPopup.qml  AppearanceTab.qml          # toLocalFile (GUI-02)
  ConfirmModal.qml / Main.qml                    # modal stack rendering (GUI-03)
crates/archel/src/gui/modal.rs                   # confirm stack (GUI-03)
crates/archel/src/gui/shell.rs                   # mirror stacked questions (GUI-03)
crates/archel/src/gui/models/input.rs            # keybind ordering (GUI-04)
crates/archel/src/gui/models/net.rs              # dnsReset worker (GUI-05)
crates/archel/src/gui/bridge_net.rs              # dnsReset invokable (GUI-05)
justfile                                         # D-15 binding-spell check (GUI-06)
 планирования/phases/07-gui-polish-fixes/        # VALIDATION.md, PLAN.md
```

### Pattern 1: NetTab draft-state (the D-01/D-03 template to copy)

**What:** Editors write to a QML-local `ListModel`/properties, never directly to tick-bound model properties. The tick refreshes read-only displays only. `Appliquer` is enabled by a pure function of (draft ≠ live ∧ validates).

**When to use:** GUI-01 (InputTab) — and any future tick-bound editor.

**Example:**
```qml
// Source: crates/archel/src/gui/qml/tabs/NetTab.qml:48-90 [VERIFIED]
ListModel { id: dnsRows }

function dnsArray() {
    var out = []
    for (var i = 0; i < dnsRows.count; i++) {
        var server = (dnsRows.get(i).server || "").trim()
        if (server !== "") { out.push(server) }
    }
    return out
}
function refreshDnsState() {
    var servers = dnsArray()
    var err = net.dnsValidate(JSON.stringify(servers))
    dnsError = err
    dnsValid = err === "" && servers.length > 0
}
function syncDnsFromCurrent() {
    dnsRows.clear()
    var parts = net.dnsText.split(",")
    /* …split, skip "" and "N/A", append rows… */
    refreshDnsState()
}
```
```qml
// Enablement rule — Source: NetTab.qml:218-223 [VERIFIED]
Button {
    text: "Appliquer"
    highlighted: true
    enabled: dnsValid && !net.dnsBusy
    onClicked: net.dnsApply(JSON.stringify(dnsArray()))
}
```

### Pattern 2: D-02 external-wins via base-compare

**What:** Keep a `draftBase` snapshot of the live value the draft was derived from. On each tick: if `live == draftBase`, leave the draft untouched (user is editing, nothing changed underneath); if `live != draftBase`, the world moved — replace both `draftBase` and the draft with `live` (external value wins, never merged). This satisfies D-01 (no tick-freezing: read-only displays still update every tick) and D-02 simultaneously.

**When to use:** GUI-01 sensitivity slider + accelerate field (+ keybind combo/action fields are already unbound and safe — see Pitfall 4).

**Example (shape — planner adapts to InputTab controls):**
```qml
property string sensBase: ""
property string sensDraft: ""
// on tick (live changed):
//   if (input.sensitivityText !== sensBase) {
//       if (sensDraft === sensBase) sensDraft = input.sensitivityText  // untouched → follow
//       else sensDraft = input.sensitivityText                        // edited → external wins (D-02)
//       sensBase = input.sensitivityText
//   }
// Appliquer enabled: sensDraft !== sensBase && valid
```

### Pattern 3: ModalSession confirm-stack (GUI-03 design)

**What:** Today `request_confirm` calls `self.reset()` first, which clears `confirm_tx` — a second tab's confirm silently orphans the first waiter's channel (its `recv()` then returns `Err` → `unwrap_or(false)` = silent decline). [VERIFIED: crates/archel/src/gui/modal.rs:183-190]

```rust
// Current — Source: crates/archel/src/gui/modal.rs:183-190 [VERIFIED]
pub fn request_confirm(&self, question: &str) -> std::sync::mpsc::Receiver<bool> {
    self.reset();   // <-- drops any pending sibling confirm
    *self.question.lock().expect("question lock") = question.to_string();
    *self.kind.lock().expect("kind lock") = ModalKind::Confirm;
    let (tx, rx) = std::sync::mpsc::channel();
    *self.confirm_tx.lock().expect("confirm lock") = Some(tx);
    rx
}
```

The D-05/D-06/D-08-compliant shape is a **pending-confirm stack**: `Vec<PendingConfirm { id/generation, question, tx }>` behind the existing Mutex discipline; `request_confirm` pushes (never resets siblings); `resolve_confirm(id, accepted)` removes exactly one entry — cancel/decline targets the **top** entry only (D-08); `sync_session` mirrors the stack as an ordered question list (`modal_question` keeps index 0 for the existing single-`ConfirmModal` binding compatibility, plus new `modal_questions_json` or `modal_question_2` for the stacked instance); Main.qml renders a second `ConfirmModal` with higher `z` bound to the top entry (existing `z: 100/101/102/103/104` layering convention extends naturally [VERIFIED: Main.qml:324-391]). Error path stays exactly as-is (single global `ErrorModal`, D-07 [VERIFIED: Main.qml:332-337]).

**When to use:** GUI-03 only. Pairing session is already independent and must stay so [VERIFIED: modal.rs:132-143].

**Threading invariant (must preserve):** workers touch only Mutex/Atomic state; property writes happen only in `sync_session` on the Qt thread via the QML 120 ms Timer [VERIFIED: shell.rs:282-331, Main.qml:396-401]. The stack must follow the same rule — no `CxxQtThread::queue` from workers for confirm state.

### Pattern 4: Validate-plan-confirm-apply worker (GUI-04/GUI-05 template)

**What:** Every write worker follows: validate (typed error) → plan → `request_confirm(plan verbatim question)` → apply → error modal on failure. Proven in `dns_worker` [VERIFIED: models/net.rs:362-388] and `add_keybind_worker` [VERIFIED: models/input.rs:618-671]. GUI-04 reorders inside this shape (persist ⇄ eval); GUI-05 instantiates it for reset via the existing core legs:

- `plan_dns_reset(profile: None)` → `DnsPlan { reset: true, … }` [VERIFIED: network.rs:1072-1082]
- `confirm_dns_prompt(&plan)` serves the reset question verbatim (`net.confirm_dns_reset` catalog key) [VERIFIED: network.rs:1215-1225 region]
- `apply_dns_plan(&plan)` branches on `plan.reset` to `dns_reset_argv` (clears `ipv4.dns`, restores DHCP) [VERIFIED: network.rs:1088-1099]
- CLI twin `run_dns_reset` proves the order (plan → confirm → apply) [VERIFIED: cmds/net_cmds.rs:55-58]

### Pattern 5: Casing convention (GUI-06)

**What:** Bridge `qproperty` names stay `snake_case`; QML bindings use `camelCase`; Qt case-folding resolves them (observed working — the offscreen smoke gate passes on the current mixed tree). Documented rule per D-13; no bridge rename.

**Evidence of the split** [all VERIFIED this session]:
- Bridge declares snake_case — `"#[qproperty(QString, sensitivity_text)] … #[qproperty(bool, banner_visible)]"` [VERIFIED: bridge_input.rs:17-24]; `"#[qproperty(QString, dns_text)] … #[qproperty(bool, dns_busy)]"` [VERIFIED: bridge_net.rs:17-29].
- Seven tab files bind camelCase (`input.bannerVisible`, `net.dnsBusy`, `display.operationBusy`, `power.operationBusy`, `bt.busyAddress`, `audio.operationBusy`, `appearance.operationBusy`, `landing.bannerVisible`) while **PkgTab alone binds snake_case** (`pkg.banner_visible`, `pkg.search_busy`, `pkg.operation_busy`) and even documents it: `"cxx-qt registers qproperties verbatim, so every model binding below uses the snake_case names from bridge_pkg.rs"` [VERIFIED: PkgTab.qml:8-10 + grep inventory across `qml/tabs/`].
- Consequence for D-14: the "convert all ~8 files" pass means converting **PkgTab's snake_case bindings → camelCase** (the outlier) plus normalizing any remaining snake_case bindings/invokable spellings in the other seven so the whole tree matches "QML binds camelCase"; the comment header in PkgTab.qml:8-10 must be rewritten to state the convention. The `property alias pkg: pkg` + qualified `pkgTab.pkg.aurBuildStart()` call [VERIFIED: PkgTab.qml:21, Main.qml:357] must keep working after the rename — invokables already expose camelCase `cxx_name`s (`tick`, `dnsApply`, `addKeybind`, `aurBuildStart`), so only *property* bindings change, never invokable call sites.

### Anti-Patterns to Avoid
- **Tick-freezing to "fix" WR-01:** stopping the 1 Hz Timer during edits violates D-01 and the Phase 6 D-02/D-03 all-tabs-live contract. Use draft-state + base-compare instead.
- **Binding `TextField.text` directly to a tick property:** this is the WR-01 bug shape (`text: input.scrollText === "N/A" ? "" : input.scrollText` [VERIFIED: InputTab.qml:124] re-applies every tick). Draft properties must be the `text` source; live values only feed `StatField` displays and the base snapshot.
- **Imperatively closing a binding-driven Dialog:** `ConfirmModal` must never close itself — Main.qml binds `visible: shell.modal_kind === "confirm"` and an imperative close would permanently break the binding [VERIFIED: ConfirmModal.qml:1-8 header comment]. Stacked modals follow the same rule (visibility from mirrored state only).
- **Second-instance confirm singletons in QML:** confirm state lives in `ModalSession` (Rust), not in QML `property` singletons — QML owns no session truth (Pattern 3).
- **New DNS plan/apply code in the GUI:** the reset leg exists (`plan_dns_reset`/`apply_dns_plan`/`confirm_dns_prompt`); reimplementing `nmcli` argv in the GUI would fork the CLI-proven logic. Add only the `dnsReset` invokable + worker wiring.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Percent-decoding `file://` URLs | `replace("file://","")` + manual `decodeURIComponent` ladder | `selectedFile.toLocalFile()` (QUrl) at both picker sites [ASSUMED Qt API — verify via qmllint + smoke] | Manual decode mishandles host prefixes, `+`, multi-byte sequences; `toLocalFile()` is the Qt-canonical conversion |
| Confirm correlation across tabs | Ad-hoc QML flags / per-tab booleans | Generation-tagged entries in `ModalSession` (Pattern 3) | Ad-hoc flags race across the worker/Qt-thread boundary; the session's Mutex + `mpsc` pairing is already proven |
| DNS reset nmcli argv | New GUI-side modify/up command construction | `plan_dns_reset` + `apply_dns_plan` (core) | Duplicates `dns_reset_argv` (clear `ipv4.dns` + DHCP restore) and the reactivation/read-back sequence; CLI-proven |
| Keybind validation | QML-side combo/action parsing | `plan_keybind_from_fields` (already shared by invokable pre-check + worker) [VERIFIED: models/input.rs:605-613] | Single definition of "valid"; D-03 enablement and worker must agree |
| Casing lint | Eyeballing ~8 files | `just gui-check` binding-spell check (D-15) + qmllint sweep | Mechanical, blocks merge; eyeballing regresses (this debt existed because nothing enforced it) |

**Key insight:** Every GUI-01…GUI-05 fix is a reordering or rewiring of already-proven pieces (NetTab draft shape, session Mutex discipline, core reset/bind legs). The risk is not missing functionality but breaking the threading and confirm invariants — preserve them and the fixes are small.

## Common Pitfalls

### Pitfall 1: Slider `value:` expression re-fires every tick (WR-01 crash-site)
**What goes wrong:** `sensitivitySlider.value` is computed from `input.sensitivityText` [VERIFIED: InputTab.qml:78-83]; each 1 Hz tick re-evaluates it and yanks the slider out from under a drag. Same for `accelerateField.text` [VERIFIED: InputTab.qml:124].
**Why it happens:** QML bindings are re-evaluated on every property change notification — a tick that sets the same-or-new value still re-runs the expression.
**How to avoid:** Bind editor controls to draft properties; feed drafts from live only via the base-compare sync (Pattern 2). `StatField` keeps the live binding (display-only is correct).
**Warning signs:** slider "fights" the mouse; typed text vanishes once per second.

### Pitfall 2: Second confirm silently declines the first (WR-03 crash-site)
**What goes wrong:** `request_confirm` → `reset()` drops the first tab's `confirm_tx`; its worker's `recv()` yields `Err` → `unwrap_or(false)` declines a write the user never saw resolved.
**Why it happens:** Single-slot session design (06-06) predates parallel confirms.
**How to avoid:** Confirm-stack (Pattern 3); each `await_confirm` holds its own receiver until its own entry resolves. Regression test: two concurrent `request_confirm`s, resolve top only, assert older still pending (mirrors the existing `confirm_round_trip_resolves_the_waiter` test shape [VERIFIED: modal.rs:479-487]).
**Warning signs:** a tab's write "does nothing" after using two tabs quickly.

### Pitfall 3: `file://` strip leaves `%20` paths that fail validation (WR-02 crash-site)
**What goes wrong:** `selectedFile.toString().replace("file://", "")` [VERIFIED: VpnImportPopup.qml:33, AppearanceTab.qml:32] keeps percent-encoding (`/home/u/mon%20fichier.ovpn`); core `validate_vpn_import_path` then rejects a file that exists. Wallpaper preview `"file://" + wallpaperField.text` [VERIFIED: AppearanceTab.qml:178] double-schemes an already-encoded path.
**Why it happens:** `FileDialog.selectedFile` is a `QUrl`, not a path; `.toString()` serializes the URL form.
**How to avoid:** `selectedFile.toLocalFile()` at both sites; keep the typed-path field as-is (already a raw path). Test with a space/accents filename through the existing worker unit tests (fake ops) + offscreen smoke.
**Warning signs:** VPN import / wallpaper fails only for paths containing spaces or non-ASCII.

### Pitfall 4: Over-decoupling the keybind fields
**What goes wrong:** `comboField`/`actionField` have **no** `text:` binding to tick state [VERIFIED: InputTab.qml:214-225] — they already survive the tick. "Fixing" them with draft machinery adds complexity for zero gain and risks breaking the inline `keybindError` label wiring [VERIFIED: InputTab.qml:227-233].
**Why it happens:** Blanket application of the draft pattern without checking which controls are actually tick-bound.
**How to avoid:** Scope GUI-01 to the three tick-bound controls (sensitivity slider, accelerate field, and their Appliquer buttons [VERIFIED: InputTab.qml:92-96, 133-137]); leave combo/action fields alone except where WR-04 worker ordering touches them.

### Pitfall 5: Casing pass breaks the AUR `property alias` chain
**What goes wrong:** Renaming `pkg.*` bindings touches the `property alias pkg: pkg` [VERIFIED: PkgTab.qml:21] and Main.qml's `pkgTab.pkg.aurBuildStart()` [VERIFIED: Main.qml:357]; a sloppy rename breaks the D-19 trust-gate call path.
**Why it happens:** Alias + qualified invokable call span two files.
**How to avoid:** Rename property bindings only; invokable call sites (`cxx_name` camelCase) are already correct and must not change. The D-15 spell check must distinguish properties (must be camelCase) from invokables (already camelCase) so it doesn't flag correct calls. Verify with offscreen smoke (AUR review modal path renders at startup).

### Pitfall 6: "Automatique" bypassing confirm or forgetting the session memory
**What goes wrong:** Implementing reset as an inline toggle (violates D-10 / Phase 6 D-12 every-write-confirms) or losing the last-manual servers on tab switch (violates D-11).
**Why it happens:** Reset feels like a "mode switch" rather than a write — but it mutates NM state via `nmcli connection modify + up`, same as set.
**How to avoid:** Route reset through the standard `request_confirm(confirm_dns_prompt(&reset_plan))` worker; stash last-manual servers in a QML `property var lastManualServers` on the NetTab root (session lifetime = tab lifetime; StackLayout keeps children alive so it survives tab switches — same lifetime argument as D-04's draft).

## Code Examples

### FileDialog `toLocalFile()` fix (GUI-02)
```qml
// BEFORE (both sites) — Source: VpnImportPopup.qml:33, AppearanceTab.qml:32 [VERIFIED]
onAccepted: pathField.text = selectedFile.toString().replace("file://", "")
// AFTER [ASSUMED — QUrl.toLocalFile() Qt API, verify with qmllint + smoke]
onAccepted: pathField.text = selectedFile.toLocalFile()
```

### Keybind-add reorder (GUI-04, planner chooses leg)
```rust
// CURRENT order — Source: models/input.rs:650-670 [VERIFIED, abbreviated]
if let Err(err) = (ops.eval_bind_line)(&line) { sess.open_error(&err.message()); return; }
let body = match (ops.read_body)(base) { /* … */ };
let merged = match core_binds::add_bind_line(&body, &spec) { /* … */ };
if let Err(err) = (ops.write_body)(base, &merged) { sess.open_error(&err.message()); }
// PREFERRED (persist-then-eval): move read_body → add_bind_line → write_body
// BEFORE eval_bind_line, so a persist failure never leaves a live-but-unsaved bind.
// (Compensating-unbind alternative keeps the order and calls ops.eval_unbind on
// write_body failure — planner's call per CONTEXT discretion.)
```
Note the remove worker already models the safe shape: post-confirm re-read before writing so external edits aren't clobbered [VERIFIED: models/input.rs:711-728].

### DNS reset worker (GUI-05, mirrors `dns_worker`)
```rust
// Core legs (all exist, CLI-proven) — Sources: network.rs:1072-1082,
// network.rs:1088-1099, net_cmds.rs:55-58 [all VERIFIED]
// pub fn plan_dns_reset(profile: Option<&str>) -> Result<DnsPlan, ArchelError>
// pub fn apply_dns_plan(plan: &DnsPlan) -> Result<DnsReport, ArchelError>
// pub fn confirm_dns_prompt(plan: &DnsPlan) -> String  (serves net.confirm_dns_reset)
// pub fn run_dns_reset(profile: Option<String>, json: bool)  (CLI twin)
// GUI adds: NetDnsOps { plan_reset, … } + dns_reset_worker (validate skipped —
// reset takes no servers; plan → confirm → apply) + `dnsReset` invokable in
// bridge_net.rs beside dnsApply [VERIFIED existing: bridge_net.rs:49-51].
```

### Binding-spell check sketch (GUI-06 / D-15)
```bash
# D-15 enforcement joins the existing lint-qml recipe [VERIFIED justfile:30-32]:
#   find crates/archel/src/gui/qml -name '*.qml' -exec "$LINTER" {} +
# New check (planner finalizes): fail on snake_case model-property bindings,
# e.g. grep for '\b(pkg|net|input|display|power|bt|audio|appearance|landing|system)\.[a-z]+_[a-z]'
# excluding invokable call sites (known camelCase cxx_names: tick, dnsApply,
# dnsValidate, vpnImportStart, addKeybind, removeKeybind, setSensitivity,
# setAccelerate, aurBuildStart, … — enumerate from bridge_*.rs #[cxx_name]).
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Single global confirm slot (`request_confirm` resets) | Per-tab parallel confirm stack (D-05/D-06) | This phase | Two tabs can hold pending writes; cancel closes top only |
| Tick-bound editors (`text: input.*`) | Draft-local state + base-compare sync (D-01/D-02) | This phase | Edits survive 1 Hz poll; external edits win |
| `selectedFile.toString().replace("file://","")` | `selectedFile.toLocalFile()` | This phase | Spaces/accents in VPN + wallpaper paths work |
| Eval-before-persist keybind-add | Persist-before-eval (or compensating unbind) | This phase | No live-but-unsaved snippet lines |
| CLI-only DNS reset | "Automatique" NetTab affordance via confirm | This phase | Full DNS round-trip available in GUI |
| Mixed camelCase/snake_case QML bindings | "QML camelCase, bridge snake_case" + gate | This phase | WINDOWS.md gate closes; merge blocked on violation |

**Deprecated/outdated:**
- `replace("file://", "")` URL→path conversion: broken for encoded paths, replaced by `toLocalFile()` at both picker sites.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `QUrl.toLocalFile()` exists on `FileDialog.selectedFile` in Qt 6.11 and percent-decodes | Code Examples / Pitfall 3 | Low — canonical Qt API; qmllint + smoke will confirm or reject immediately |
| A2 | Qt case-folding resolves camelCase QML bindings to snake_case `Q_PROPERTY`s (i.e. current mixed tree genuinely works, not silently binding `undefined`) | Pattern 5 | Medium — deferred-item asserts it "works today" and smoke passes; planner should add a smoke assertion or qmltest binding check rather than trust the comment |
| A3 | StackLayout keeps hidden tab children alive (draft + `lastManualServers` survive tab switches) | Pitfalls 6, Pattern 2 | Low — Qt `StackLayout` does not destroy inactive children by default; verify by code-read during planning if uncertain |
| A4 | Multiple `Dialog`s stack with `z` ordering in the Qt 6 build used here | Pattern 3 | Low — standard Qt behavior; offscreen smoke + a stacked-modal unit/QML check confirms |
| A5 | CXX-Qt / Qt6 / toolchain versions from prior STACK.md still current (offline, not re-verified) | Standard Stack | Low — no dependency changes in this phase regardless |

**If this table is empty:** n/a — five assumptions above need planner/executor confirmation.

## Open Questions

1. **Confirm-stack rendering shape**
   - What we know: `sync_session` mirrors one question; Main.qml binds one `ConfirmModal` to `shell.modal_kind === "confirm"`. D-05/D-06/D-08 require N parallel + stacked + top-only-cancel.
   - What's unclear: Whether to mirror `modal_questions_json` (QML Repeater/Instantitor of ConfirmModal) vs. fixed `modal_question` + `modal_question_2` properties (two static modals, second with higher z). Former scales, latter is simpler and D-05 only ever needs two in practice.
   - Recommendation: Fixed two-modal shape (question + question_top) unless the planner foresees >2 concurrent confirms; cap the stack at 2–3 with a defined overflow (oldest auto-declined with error-modal note, or refuse newest with inline busy — planner decides, must stay D-12-compliant).

2. **D-04 popup trigger mechanics**
   - What we know: Draft lives on the InputTab root; "returning" = `stack.currentIndex` changing to the Input index with a dirty draft.
   - What's unclear: Exact signal wiring (Main.qml `onCurrentIndexChanged` vs. InputTab visibility listener) and whether the popup is a new `Dialog` component or reuses `ConfirmModal` via the session (reusing the global session would collide with GUI-03 pending confirms).
   - Recommendation: Dedicated lightweight `Dialog`/`Popup` owned by InputTab (not the shared session) offering Appliquer/Abandonner; planner specifies.

3. **"Automatique" visual shape in the DNS group**
   - What we know: D-09…D-12 fix behavior (reset leg, confirm, session memory, active profile only). No visual reference (CONTEXT specifics: "no specific visual references").
   - What's unclear: Radio "Automatique / Manuel" vs. "Automatique" toggle-button vs. combobox above the server rows.
   - Recommendation: Planner picks the cheapest shape consistent with the existing button row (Réinitialiser/Appliquer); behavior contract is fully locked so any shape works.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo (offline) | All builds/tests | ✓ [VERIFIED: shell probe] | 1.89.1 | — |
| rustc via QMAKE pin (`/usr/bin/qmake6`) | `gui-build` | unknown — verify at plan time (`command -v qmake6`) | — | Plan must include the presence check; bare `qmake` (Qt5) breaks cxx-qt-build per justfile header |
| qmllint | `lint-qml` + D-15 check | ✓ [VERIFIED: `/usr/bin/qmllint`] | — | None — lint is a merge gate; recipe fails loudly without it (WR-06 fix) |
| `just` | Recipe runner | ✗ [VERIFIED: `command -v just` empty] | — | Mirror recipes by hand (literal lines in justfile header [VERIFIED: justfile:12-18]); all verify commands append `--offline` |
| offscreen Qt platform | `smoke` | presumed (Qt6 installed per v1.0 ship) — verify at plan time | — | None — smoke is the startup-regression gate |
| Network (crates.io) | cargo fetch | ✗ (offline machine per STATE.md) | — | `cargo ... --offline` everywhere; Cargo.lock committed |

**Missing dependencies with no fallback:**
- None beyond the above; plan must use `--offline` on every cargo invocation and hand-mirrored recipes.

**Missing dependencies with fallback:**
- `just` → hand-mirrored shell lines (documented in justfile header).

## Validation Architecture

Test framework: Rust inline `#[cfg(test)]` modules (`cargo test`), no separate `tests/` dir [VERIFIED: glob found no `crates/archel/tests/`; `mod tests` in `modal.rs:443`, `models/net.rs:461`, `models/input.rs:739`]. QML verified via qmllint sweep + offscreen smoke (`QT_QPA_PLATFORM=offscreen timeout 10 ./target/debug/archel settings; test $? -eq 124` [VERIFIED: justfile:34-36]) + `gui_thin_view` coverage audit referenced in justfile header.

| Property | Value |
|----------|-------|
| Framework | `cargo test` (inline unit tests) + qmllint + offscreen smoke |
| Config file | `justfile` (recipes are the config); `Cargo.toml` workspace |
| Quick run command | `cargo test -p archel --features gui --offline <module>` (scoped) |
| Full suite command | Hand-mirror of `just gui-check` with `--offline`: `cargo test -p archel-core --no-default-features --offline && cargo test -p archel --no-default-features --offline` → `QMAKE=/usr/bin/qmake6 cargo build -p archel --features gui --offline` → qmllint sweep → offscreen smoke → `QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui --offline` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| GUI-01 | Draft decoupled from tick; external-wins; Appliquer enablement; return-popup | unit (base-compare pure fns if extracted to Rust) + qmllint + smoke | `cargo test -p archel --features gui --offline input` | ❌ Wave 0 (new tests) |
| GUI-02 | `toLocalFile()` at both pickers; encoded path survives to worker | qmllint + smoke + worker unit (fake ops, pre-encoded path string) | qmllint sweep; `cargo test -p archel --features gui --offline net` / `appearance` | ❌ Wave 0 (picker binding has no unit home — QML-only; assert via lint grep + smoke) |
| GUI-03 | Two concurrent confirms coexist; top-cancel leaves older pending; error modal unchanged | unit (`modal.rs` stack tests) | `cargo test -p archel --features gui --offline modal` | ❌ Wave 0 (new stack tests) |
| GUI-04 | Persist failure leaves no live-evaluated bind (or compensating unbind ran) | unit (fake `InputOps`, failing `write_body`) | `cargo test -p archel --features gui --offline input` | ❌ Wave 0 (new ordering test; existing fake-seam shape at models/input.rs:836+ to extend) |
| GUI-05 | Reset worker plans/confirms/applies via core reset leg; session memory pre-fill (QML) | unit (fake `NetDnsOps` incl. `plan_reset`) | `cargo test -p archel --features gui --offline net` | ❌ Wave 0 (new reset worker tests) |
| GUI-06 | All tab bindings camelCase; gate fails on snake_case property binding | shell/grep gate + qmllint + smoke | binding-spell check in `lint-qml` path | ❌ Wave 0 (new gate script assertions) |

### Sampling Rate
- **Per task commit:** scoped `cargo test -p archel --features gui --offline <module>` + qmllint on touched QML files
- **Per wave merge:** full hand-mirrored `gui-check` with `--offline`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `modal.rs` confirm-stack unit tests (parallel confirms, top-only resolve, error path untouched) — covers GUI-03
- [ ] `models/input.rs` persist-then-eval (or compensating-unbind) ordering test with failing `write_body` fake — covers GUI-04
- [ ] `models/net.rs` DNS-reset worker tests (plan_reset → confirm → apply; decline writes nothing) — covers GUI-05
- [ ] Draft base-compare tests (GUI-01; QML-side logic — if kept in QML, cover via documented manual/offscreen check + enablement unit test if validation helper is Rust-side)
- [ ] D-15 binding-spell check implementation + its own positive/negative fixture assertions — covers GUI-06
- [ ] Framework install: none (cargo + qmllint present; just absent → hand-mirror)

## Security Domain

ASVS Level 1 (per `.planning/config.json`: `security_enforcement: true`, `security_asvs_level: 1`) [VERIFIED: config.json:48-51].

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | n/a (single-user local app, no auth surface in this phase) |
| V3 Session Management | no | n/a |
| V4 Access Control | partial | Privilege escalation stays the Phase-3 model (`run_nm_write`/pkexec via core; GUI never embeds passwords — Phase 6 D-18 preserved, no change in this phase) |
| V5 Input Validation | yes | Core validators reused unchanged (`validate_dns_servers`, `plan_keybind_from_fields`, `validate_vpn_import_path`); draft state validated through the same gates before enablement (D-03) |
| V6 Cryptography | no | n/a |

### Known Threat Patterns for Rust + CXX-Qt/QML stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal via picker/typed path (VPN import, wallpaper) | Tampering | Unchanged core `validate_vpn_import_path`; `toLocalFile()` does not bypass validation — worker validates the decoded path as before |
| QML-injected confirm questions | Spoofing | Workers use core-verbatim prompts (`confirm_dns_prompt`, `confirm_bind_prompt`); new "Automatique" question comes from the same core function, never a QML string |
| Stale-confirm clobbering (confirm-window races) | Tampering | Generation-tagged stack entries (Pattern 3) + existing post-confirm re-read discipline (remove worker exemplar); keybind-add reorder keeps failures pre-live |
| Privilege confusion on DNS reset | Elevation of privilege | Reset routes through `apply_dns_plan` → `run_nm_write` (same gate as set); confirm-first (D-10) preserved |

## Sources

### Primary (HIGH confidence)
- `crates/archel/src/gui/modal.rs` (request_confirm/reset, pairing independence, existing confirm tests) — read this session, exact lines cited
- `crates/archel/src/gui/shell.rs` (sync_session mirror, Qt-thread rule) — read this session
- `crates/archel/src/gui/models/input.rs` (add/remove keybind workers, InputOps seam, fake-based tests) — read this session
- `crates/archel/src/gui/models/net.rs` (dns_worker, NetDnsOps/NetVpnOps seams) — read this session
- `crates/archel/src/gui/qml/tabs/NetTab.qml` + `InputTab.qml` + `PkgTab.qml` + `AppearanceTab.qml` + `Main.qml` + `ConfirmModal.qml` + `VpnImportPopup.qml` — read this session
- `crates/archel/src/gui/bridge_input.rs` + `bridge_net.rs` (snake_case qproperties, camelCase cxx_names) — read this session
- `crates/archel-core/src/network.rs` (plan_dns_set/reset, apply_dns_plan, confirm/prompt fns, catalog keys) — read this session
- `crates/archel/src/cmds/net_cmds.rs` + `cmds/mod.rs` (CLI `run_dns_reset` twin) — grepped this session
- `justfile` (gui-check recipes, hand-mirror lines, WR-06 space-safe lint) — read this session
- `.planning/*` (07-CONTEXT.md, REQUIREMENTS.md GUI-01…06, STATE.md offline note, WINDOWS.md gate, deferred-items.md WR-01…05, 06-CONTEXT.md D-01…D-26, config.json) — read this session

### Secondary (MEDIUM confidence)
- None (offline — no documentation lookups performed)

### Tertiary (LOW confidence)
- `QUrl.toLocalFile()` semantics, Qt Dialog stacking with `z`, StackLayout child lifetime, Qt case-folding detail — training knowledge, tagged [ASSUMED], each with an in-gate verification (qmllint/smoke/unit)

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — no new deps; in-repo legs verified by direct read
- Architecture: HIGH — all three patterns (draft, session, worker) are proven in-repo shapes with exact source lines
- Pitfalls: HIGH — each crash-site traced to exact lines read this session

**Research date:** 2026-10-11
**Valid until:** 30 days (stable debt-fix scope; no fast-moving deps involved)
