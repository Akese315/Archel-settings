---
phase: 06-gui-shell-tabs
verified: 2026-10-10T18:05:00Z
status: passed
score: 4/4 goal-backward checks satisfied (0 partial, 0 missing)
behavior_unverified: 0
overrides_applied: 0
covered_files:
  - .planning/phases/06-gui-shell-tabs/06-01-SUMMARY.md
  - .planning/phases/06-gui-shell-tabs/06-01-shell-window-PLAN.md
  - .planning/phases/06-gui-shell-tabs/06-02-SUMMARY.md
  - .planning/phases/06-gui-shell-tabs/06-02-core-cancel-stream-PLAN.md
  - .planning/phases/06-gui-shell-tabs/06-03-SUMMARY.md
  - .planning/phases/06-gui-shell-tabs/06-03-polling-system-landing-PLAN.md
  - .planning/phases/06-gui-shell-tabs/06-04-SUMMARY.md
  - .planning/phases/06-gui-shell-tabs/06-04-read-net-power-audio-bt-PLAN.md
  - .planning/phases/06-gui-shell-tabs/06-05-SUMMARY.md
  - .planning/phases/06-gui-shell-tabs/06-05-read-hyprland-pkg-PLAN.md
  - .planning/phases/06-gui-shell-tabs/06-06-SUMMARY.md
  - .planning/phases/06-gui-shell-tabs/06-06-modal-infra-pkg-write-PLAN.md
  - .planning/phases/06-gui-shell-tabs/06-07-SUMMARY.md
  - .planning/phases/06-gui-shell-tabs/06-07-write-system-domains-PLAN.md
  - .planning/phases/06-gui-shell-tabs/06-08-SUMMARY.md
  - .planning/phases/06-gui-shell-tabs/06-08-write-hyprland-domains-PLAN.md
  - .planning/phases/06-gui-shell-tabs/06-09-SUMMARY.md
  - .planning/phases/06-gui-shell-tabs/06-09-gui-check-thin-view-PLAN.md
  - crates/archel/src/main.rs
  - crates/archel/src/gui/mod.rs
  - crates/archel/build.rs
  - crates/archel/src/gui/single_instance.rs
  - crates/archel/src/gui/geometry.rs
  - crates/archel/src/gui/qml/Main.qml
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
  - crates/archel-core/src/privileged.rs
  - crates/archel-core/src/aur.rs
  - crates/archel/src/gui/poll.rs
  - crates/archel/src/gui/models/mod.rs
  - crates/archel/src/gui/models/system.rs
  - crates/archel/src/gui/models/landing.rs
  - crates/archel/src/gui/models/net.rs
  - crates/archel/src/gui/models/power.rs
  - crates/archel/src/gui/models/audio.rs
  - crates/archel/src/gui/models/bt.rs
  - crates/archel/src/gui/models/appearance.rs
  - crates/archel/src/gui/models/display.rs
  - crates/archel/src/gui/models/input.rs
  - crates/archel/src/gui/models/pkg.rs
  - crates/archel-core/src/network.rs
  - justfile
covered_digest: "v2:sha256:dcaf223b511bcbf1eb598ccf96995b4eae06002d20ca3171e0270cf3512de237"
---

# Phase 06: GUI Shell & Tabs — Verification Report

**Phase goal (ROADMAP.md):** Users get a modern vertical-tab GUI presenting every proven capability.
**Machine-checkable definition:** shell + 10 tabs, all writes through CLI-proven core,
`just gui-check` green + thin-view audit green.
**Verified:** 2026-10-10
**Status:** passed (gap closed post-verification — see Addendum)
**Score:** 4/4 success criteria verified

## Criterion results

| # | ROADMAP success criterion | Verdict | Evidence |
|---|---------------------------|---------|----------|
| SC1 | User can open `archel settings` and navigate Landing + all 9 domain tabs in vertical-tab layout | ✅ PASS | `Main.qml`: 10 `TabButton`s (Accueil, Réseau, Alimentation, Bluetooth, Apparence, Son, Applications, Affichage, Périphériques, Système) over a `StackLayout` of 10 tab files in `qml/tabs/`; offscreen smoke `QT_QPA_PLATFORM=offscreen timeout 10 ./target/debug/archel settings` → rc=124 (stays alive, empty log, no `qrc: load` errors) |
| SC2 | Live system graphs update at 1 Hz without freezing | ✅ PASS | `PollGuard` (`gui/poll.rs`): in-flight flag + sequence tag; tests `guard_skips_a_tick_while_one_is_in_flight`, `guard_drops_stale_sequences`, banner-episode tests — 5/5 pass. SystemModel sparkline + per-tab 1 Hz `Timer` tick reads (e.g. `PkgTab: onTriggered: pkg.tick()`). `QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui` → **241 passed, 0 failed** |
| SC3 | Every write action from the GUI with spinner/disabled states + readable errors | ✅ PASS (after GAP-1 closure, see Addendum) | Write invokables exist on all models (net `dns_apply`/`vpn_import_start`, pkg `install_start`/`upgrade_start`/`remove_start`/`aur_review_start`/`aur_build_start`, bt `pair`/`unpair`/`connect`, audio `set_default_*`, power `set_profile`, appearance/input/display setters) and modal layer is single-instance (`ConfirmModal`, `ErrorModal`, `StreamModal`, `PairingModal`, `VpnImportPopup` each ×1 at Main root; `CountdownModal` ×1 in DisplayTab). **Gap at verification time:** `Main.qml:357 `onAurValidated: pkg.aurBuildStart()` referenced id `pkg` scoped inside `PkgTab.qml` — now fixed (Addendum). |
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

**passed** — GAP-1 closed (Addendum). All test suites green
(269 + 123 + 241), smoke + lint green, review blockers confirmed fixed.

## Addendum — GAP-1 closure (orchestrator, 2026-10-10, commit `05fb455`)

Applied the prescribed fix with the 06-07 alias naming (`pkg`, mirroring
`netTab.net`): `property alias pkg: pkg` on `PkgTab.qml` root +
`onAurValidated: pkgTab.pkg.aurBuildStart()` in `Main.qml:357`.
Evidence: qmllint rc=0 on both files; no remaining bare-`pkg.` references
in `qml/`; offscreen smoke rc=124 with zero `ReferenceError` lines;
`QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui` → 241 passed.
SC3 flips to PASS — 4/4.
