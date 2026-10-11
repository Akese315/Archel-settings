# Roadmap: Archel

## Milestones

- ✅ **v1.0 — Archel Settings Suite** — Phases 1–6 (shipped 2026-10-11, tag `v1.0`)
- 🚧 **v1.1 — Debt + Polish** — Phases 7–9 (in progress)

## Phases

<details>
<summary>✅ v1.0 — Phases 1–6 — SHIPPED 2026-10-11 (full details: <code>.planning/milestones/v1.0-ROADMAP.md</code>)</summary>

- [x] Phase 1: Foundations & Safety Rails (3/3 plans) — workspace, typed error model, pkexec helper skeleton, CXX-Qt spike gate — completed 2026-09-30
- [x] Phase 2: System Insight (3/3 plans) — landing snapshot, live stats sampler, GPU/temp/power sensors — completed 2026-10-01
- [x] Phase 3: Packages & CLI Harness (4/4 plans) — alpm list/search, pkexec install/remove/upgrade, AUR review-before-build, shell completions — completed 2026-10-06
- [x] Phase 4: Connectivity & Devices (5/5 plans) — network/DNS/proxy/VPN, Bluetooth, sound, power profiles — completed 2026-10-07
- [x] Phase 5: Hyprland Desktop (4/4 plans) — appearance (wallpaper/font/blur), display (per-monitor), input (keybinds/mouse) via snippet writes — completed 2026-10-08
- [x] Phase 6: GUI Shell & Tabs (9/9 plans) — CXX-Qt/QML vertical-tab shell, 10 live tabs, confirm-token mutations, gui-check gate — completed 2026-10-10

</details>

### 🚧 v1.1 Debt + Polish (In Progress)

**Milestone Goal:** Clear the acknowledged v1.0 debt so every deferred item is verified-fixed or re-acknowledged, with zero new features.

- [ ] **Phase 7: GUI Polish Fixes** - WR-01–WR-05 code-review fixes plus the QML casing convention
- [ ] **Phase 8: Repo Hygiene** - Clean clippy baseline, stable timing tests, Nyquist debt, advisories
- [ ] **Phase 9: Live UAT Verification** - Deferred live-session confirmations on the user's Hyprland session

## Phase Details

### Phase 7: GUI Polish Fixes
**Goal**: GUI code-review debt is fixed — tabs behave correctly under polling, pickers handle real paths, confirms are isolated, and the casing convention is settled
**Depends on**: Phase 6 (v1.0 shipped)
**Requirements**: GUI-01, GUI-02, GUI-03, GUI-04, GUI-05, GUI-06
**Success Criteria** (what must be TRUE):
  1. User can type in the Input tab without the 1 Hz tick overwriting in-progress edits
  2. User can pick a VPN file or wallpaper with spaces/accents in the path and the selection resolves correctly
  3. User can trigger confirms in two tabs without one tab's session affecting the other
  4. User can add a keybind and the new combo fires live with no stale snippet line left behind on failure
  5. User can return to automatic DNS from the GUI, or the GUI clearly marks it CLI-only; QML binding convention is applied and the WINDOWS.md gate is closed
**Plans**: TBD
**UI hint**: yes

### Phase 8: Repo Hygiene
**Goal**: The repo is clean and stable — clippy is green under `-D warnings`, timing tests don't flake, Nyquist drafts exist, advisories are fixed
**Depends on**: Phase 7
**Requirements**: HYG-01, HYG-02, HYG-03, HYG-04
**Success Criteria** (what must be TRUE):
  1. Maintainer can run `cargo clippy --offline -- -D warnings` repo-wide with zero findings
  2. Maintainer can run the full test suite under heavy parallel load with no `live_*` timing flakes
  3. Reviewer can find Nyquist validation drafts for phases 01–06 covering the deferred validation debt
  4. User gets correct audio read-back (matched on device id), no hardcoded "N/A" in NetTab, and the thin-view audit parses Cargo.toml properly
**Plans**: TBD

### Phase 9: Live UAT Verification
**Goal**: Every deferred live check is confirmed on the user's real Hyprland session — display, input, appearance, escalation, visual pass, and VPN
**Depends on**: Phase 8
**Requirements**: UAT-01, UAT-02, UAT-03, UAT-04, UAT-05, UAT-06, UAT-07
**Success Criteria** (what must be TRUE):
  1. User confirms blur toggle works end-to-end via CLI with live-apply, read-back, and snippet durability (UAT-01)
  2. User confirms display KEEP within 15 s applies visibly and reject/timeout restores the previous config on a real monitor (UAT-02)
  3. User confirms an added keybind fires live and edit-rekey leaves only the new chord bound with no restart (UAT-03)
  4. User confirms the timed-confirm reader exits cleanly on a real TTY and the pkexec dialog appears for a helper-gated package operation (UAT-04, UAT-05)
  5. User confirms the GUI visual pass (10 tabs, vertical rail, French strings) and VPN `down` re-test shows the tracking-loss warning with the `vpn import` hint (UAT-06, UAT-07)
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 7 → 8 → 9

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Foundations & Safety Rails | v1.0 | 3/3 | Complete | 2026-09-30 |
| 2. System Insight | v1.0 | 3/3 | Complete | 2026-10-01 |
| 3. Packages & CLI Harness | v1.0 | 4/4 | Complete | 2026-10-06 |
| 4. Connectivity & Devices | v1.0 | 5/5 | Complete | 2026-10-07 |
| 5. Hyprland Desktop | v1.0 | 4/4 | Complete | 2026-10-08 |
| 6. GUI Shell & Tabs | v1.0 | 9/9 | Complete | 2026-10-10 |
| 7. GUI Polish Fixes | v1.1 | 0/TBD | Not started | - |
| 8. Repo Hygiene | v1.1 | 0/TBD | Not started | - |
| 9. Live UAT Verification | v1.1 | 0/TBD | Not started | - |
