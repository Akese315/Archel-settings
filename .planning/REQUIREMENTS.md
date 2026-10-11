# Requirements: Archel v1.1 Debt + Polish

**Defined:** 2026-10-11
**Core Value:** One reliable API through which every system setting can be read and changed on Arch + Hyprland — proven via the CLI before the GUI ever draws it.

## v1.1 Requirements

Debt-clearing milestone. Every item below is acknowledged v1.0 debt (see `.planning/milestones/v1.0-phases/*/deferred-items.md`, `05-UAT.md` deferred follow-ups, `.planning/WINDOWS.md`). Zero new features.

### Live UAT

Deferred verifications requiring the user's live Hyprland session.

- [ ] **UAT-01**: Phase-05 appearance live checks confirmed — blur toggle end-to-end via CLI with live-apply, read-back, and snippet durability
- [ ] **UAT-02**: Display keep-or-rollback confirmed on a real monitor — KEEP within 15 s applies visibly; reject/timeout restores previous config
- [ ] **UAT-03**: Keybind live behavior confirmed — added combo fires on the live compositor; edit-rekey leaves only the new chord live (old chord unbound), no Hyprland restart
- [ ] **UAT-04**: Timed-confirm reader thread exit confirmed on a real TTY (WR-06 follow-up)
- [ ] **UAT-05**: pkexec escalation dialog verified live for a helper-gated package operation
- [ ] **UAT-06**: GUI visual pass confirmed on the live desktop (10 tabs render, vertical rail, French strings)
- [ ] **UAT-07**: VPN `down` re-tested live after the 04-05 fix (tracking-loss warning + `vpn import` hint)

### GUI Polish

Code-review follow-ups from waves 6-7 plus the open Windows-gate deviation.

- [x] **GUI-01**: Input tab edits survive the 1 Hz poll tick — editor state decoupled like NetTab (local state, write on Appliquer) (WR-01)
- [ ] **GUI-02**: VPN + wallpaper FileDialog pickers resolve percent-encoded paths via `toLocalFile()` (WR-02)
- [ ] **GUI-03**: Confirm sessions are per-tab (or generation-tagged) — no cross-tab multiplexing on the single global session (WR-03)
- [ ] **GUI-04**: Keybind-add persists before (or compensates after) live-eval — no unevaluated snippet line left behind on post-eval persist failure (WR-04)
- [ ] **GUI-05**: GUI exposes a path back to automatic DNS (maps the CLI `reset` leg) or annotates it CLI-only (WR-05)
- [ ] **GUI-06**: QML/bridge casing convention applied across tab files and the WINDOWS.md gate closed (06-06 deviation)

### Hygiene

Repo-wide cleanliness items acknowledged at v1.0 close.

- [ ] **HYG-01**: `cargo clippy` clean repo-wide under `-D warnings` — 29 pre-existing `archel-core` findings cleared (own-file warnings stay at zero)
- [ ] **HYG-02**: `live_*` timing tests stable under heavy parallel cargo load (no duration-threshold flakes)
- [ ] **HYG-03**: Nyquist validation draft debt cleared for phases 01–06 (`/gsd-validate-phase`)
- [ ] **HYG-04**: Advisory fixes landed — IN-01 (audio read-back matches on id, not name), IN-02 (NetTab hardcoded `"N/A"`), IN-03 (thin-view audit parses Cargo.toml properly)

## Future Requirements

Acknowledged but not in v1.1 scope.

- **FUT-01**: Event-driven GUI updates (D-Bus/PipeWire subscriptions) to replace 1 Hz snapshot polling — revisit post-v1.1 (PROJECT.md Key Decisions)

## Out of Scope

| Feature | Reason |
|---------|--------|
| New settings capabilities | Debt-clearing milestone; new features wait for v1.2 |
| Non-Hyprland / non-Arch support | Standing project boundary (PROJECT.md Out of Scope) |
| Background daemon / tray / auto-update | Standing project boundary, revisit only in v2 |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| UAT-01 | Phase 9 | Pending |
| UAT-02 | Phase 9 | Pending |
| UAT-03 | Phase 9 | Pending |
| UAT-04 | Phase 9 | Pending |
| UAT-05 | Phase 9 | Pending |
| UAT-06 | Phase 9 | Pending |
| UAT-07 | Phase 9 | Pending |
| GUI-01 | Phase 7 | Complete |
| GUI-02 | Phase 7 | Pending |
| GUI-03 | Phase 7 | Pending |
| GUI-04 | Phase 7 | Pending |
| GUI-05 | Phase 7 | Pending |
| GUI-06 | Phase 7 | Pending |
| HYG-01 | Phase 8 | Pending |
| HYG-02 | Phase 8 | Pending |
| HYG-03 | Phase 8 | Pending |
| HYG-04 | Phase 8 | Pending |

**Coverage:**
- v1.1 requirements: 17 total
- Mapped to phases: 17
- Unmapped: 0 ✓

---
*Requirements defined: 2026-10-11*
*Last updated: 2026-10-11 after initial definition*
