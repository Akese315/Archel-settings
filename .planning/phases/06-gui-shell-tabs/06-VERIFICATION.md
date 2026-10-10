# Phase 06: GUI Shell & Tabs — Verification Report

**Phase goal (ROADMAP.md):** Users get a modern vertical-tab GUI presenting every proven capability.
**Machine-checkable definition:** shell + 10 tabs, all writes through CLI-proven core,
`just gui-check` green + thin-view audit green.
**Verified:** 2026-10-10
**Status:** gaps_found
**Score:** 3/4 success criteria verified (1 blocking gap)

## Criterion results

| # | ROADMAP success criterion | Verdict | Evidence |
|---|---------------------------|---------|----------|
| SC1 | User can open `archel settings` and navigate Landing + all 9 domain tabs in vertical-tab layout | ✅ PASS | `Main.qml`: 10 `TabButton`s (Accueil, Réseau, Alimentation, Bluetooth, Apparence, Son, Applications, Affichage, Périphériques, Système) over a `StackLayout` of 10 tab files in `qml/tabs/`; offscreen smoke `QT_QPA_PLATFORM=offscreen timeout 10 ./target/debug/archel settings` → rc=124 (stays alive, empty log, no `qrc: load` errors) |
| SC2 | Live system graphs update at 1 Hz without freezing | ✅ PASS | `PollGuard` (`gui/poll.rs`): in-flight flag + sequence tag; tests `guard_skips_a_tick_while_one_is_in_flight`, `guard_drops_stale_sequences`, banner-episode tests — 5/5 pass. SystemModel sparkline + per-tab 1 Hz `Timer` tick reads (e.g. `PkgTab: onTriggered: pkg.tick()`). `QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui` → **241 passed, 0 failed** |
| SC3 | Every write action from the GUI with spinner/disabled states + readable errors | ❌ FAIL (1 gap) | Write invokables exist on all models (net `dns_apply`/`vpn_import_start`, pkg `install_start`/`upgrade_start`/`remove_start`/`aur_review_start`/`aur_build_start`, bt `pair`/`unpair`/`connect`, audio `set_default_*`, power `set_profile`, appearance/input/display setters) and modal layer is single-instance (`ConfirmModal`, `ErrorModal`, `StreamModal`, `PairingModal`, `VpnImportPopup` each ×1 at Main root; `CountdownModal` ×1 in DisplayTab). **But:** `Main.qml:357 `onAurValidated: pkg.aurBuildStart()` references id `pkg`, which is declared inside `PkgTab.qml` (`PkgModel { id: pkg }`) and is out of scope in `Main.qml` (which only has `pkgTab`). Clicking Valider in the AUR trust review throws a QML ReferenceError and the AUR build never starts. See Gaps. |
| SC4 | Nothing in GUI not CLI-proven — thin views over proven core | ✅ PASS | Thin-view audit green: `cargo test -p archel --no-default-features thin_view` → 3/3 (`mapped_invokables_exist`, `no_subprocess_or_ipc_in_gui`, `annotation_drift_gate`). Full headless suites: `archel-core` **269 passed**, `archel` **123 passed**, 0 failures |

## Plan-level checks

- **All 9 SUMMARies exist** (06-01 … 06-09): ✅ (9 files counted)
- **qmllint sweep** (mirror of `just lint-qml`, `just` not installed on this machine): ✅ exit 0 over all QML files. Note: qmllint does not resolve cross-file ids, so it cannot catch the SC3 gap — consistent with the gap surviving `gui-check`.
- **Post-review fixes verified:**
  - `097a989` CR-01: regression test `pair_decline_with_failing_cleanup_surfaces_the_error` ✅ passes (gui suite)
  - `097a989` CR-02: regression test `remove_external_edit_during_confirm_is_not_clobbered` ✅ passes (input suite, 26/26)
  - `097a989` WR-06: justfile `lint-qml` rewritten with presence guard + `-exec` ✅ (sweep ran space-safe above)
  - `ebe84f8` i18n diacritics: core suite 269 green, no `no_hardcoded_french_diacritics` failure ✅
  - `3fabddf` deferred log ✅ (deferred-items.md records all review outcomes)
- **Accepted debt NOT failed on:** camelCase QML bindings, WR-01…WR-05 + IN-01…IN-03 follow-ups, clippy baseline, flaky `live_*` timing tests. None observed blocking during this run.

## Gaps

- **GAP-1 (blocks SC3): AUR "Valider" button is dead — `Main.qml:357` out-of-scope id.**
  Deferred-items.md (06-07 gates row, kind `bug`) already prescribed the fix —
  "expose `property alias` on PkgTab and qualify the call" in the owning plan
  (06-06) or the check plan (06-09) — but neither plan applied it; the line still
  reads `onAurValidated: pkg.aurBuildStart()` while Main scope only defines `pkgTab`.
  **Fix:** add e.g. `property alias pkgModel: pkg` to `PkgTab.qml` root and change
  Main.qml:357 to `onAurValidated: pkgTab.pkgModel.aurBuildStart()`. Re-verify with
  qmllint sweep + offscreen smoke + gui test suite. Impact is confined to the
  AUR-build path (install/remove/upgrade + all other writes unaffected).

## Verdict

**gaps_found** — 1 narrow, prescribed gap (GAP-1). All test suites green
(269 + 123 + 241), smoke + lint green, review blockers confirmed fixed.
Phase closes after the one-line GAP-1 fix; no re-plan needed.
