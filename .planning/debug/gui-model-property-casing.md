---
status: resolved
trigger: GUI — « la recherche Bluetooth ne fonctionne pas » + erreurs répétées `InputTab.qml:56: Error: Cannot assign [undefined] to QString`
created: 2026-10-11
updated: 2026-10-11
---

# Debug Session: gui-model-property-casing

## Symptoms

- Expected: l'onglet Bluetooth liste les appareils et le bouton Rechercher rafraîchit la liste après le scan de 12 s.
- Actual: la liste reste vide en permanence ; l'onglet Périphériques log `Cannot assign [undefined] to QString` à `InputTab.qml:56` (`sensBase = readSensLive()`), répété à chaque tick 1 Hz.
- Reproduction: `archel settings` (instance réelle ou `QT_QPA_PLATFORM=offscreen`) — warnings QML immédiats.

## Evidence

- timestamp: 2026-10-11 — Le scan lui-même fonctionne (invokable `scanNow` avec `cxx_name` explicite) ; `bt_scan()` retourne des appareils via le CLI. C'est l'affichage qui est mort : `bt.devicesJson` lit `undefined` → `JSON.parse(undefined)` échoue → le modèle ListView reste vide.
- timestamp: 2026-10-11 — Preuve dans le C++ généré (`target/.../bridge_input.cxxqt.h`) : `Q_PROPERTY(QString sensitivity_text ...)` — cxx-qt 0.10 nomme les `Q_PROPERTY` d'après l'ident Rust **verbatim** (snake_case), sans conversion automatique. Toute la QML des onglets lit en camelCase (`sensitivityText`, `devicesJson`) → lectures `undefined` sur les 10 onglets + 3 refs Main.qml (`shell.modalKind`, `modalQuestionTop`, `modalConfirmCount` → le confirm empilé ne s'affichait jamais). Les refs snake_case du ShellModel (`shell.modal_question`, `launch_x`) fonctionnaient, ce qui masquait l'ampleur.
- timestamp: 2026-10-11 — Vérifié dans les sources cxx-qt-gen 0.10 (`naming/name.rs::with_options`) : `#[auto_cxx_name]` sur le bloc `extern "RustQt"` convertit les noms générés sans `cxx_name` explicite en camelCase (`my_property` → `myProperty`, notify → `myPropertyChanged`, côté Rust inchangé) ; les `cxx_name` explicites des invokables gagnent.
- timestamp: 2026-10-11 — Collision détectée au build après activation : `adapter_on` génère le setter `setAdapterOn(bool)`, clash exact avec l'invokable `#[cxx_name = "setAdapterOn"]` de `set_adapter_powered`. Invokable renommé `applyAdapterOn` (BtTab.qml mis à jour).

## Resolution

- root_cause: cxx-qt 0.10 n'applique AUCUNE conversion de casse par défaut : les `Q_PROPERTY` générés portent l'ident Rust verbatim (snake_case) alors que la convention QML du projet (pass 07-04, gate binding-spell du justfile) est camelCase. Chaque liaison de propriété d'onglet lisait `undefined` ; seuls les invokables (avec `cxx_name` explicites) fonctionnaient.
- fix: `#[auto_cxx_name]` sur les 12 blocs `extern "RustQt"` (11 bridges + shell.rs) ; Main.qml : ~19 refs `shell.*` snake_case renommées camelCase ; invokable BT `setAdapterOn` → `applyAdapterOn` (collision setter) ; rustines masquantes de la session précédente (`|| ""`, `x ? x : false`, bindings `font.pixelSize` auto-référentiels) écartées.
- prevention: la gate `lint-qml` (binding-spell) vérifiait la QML mais rien ne vérifiait l'accord QML↔noms générés ; `#[auto_cxx_name]` rend désormais la génération conforme à la convention que la gate impose — toute nouvelle propriété snake_case côté QML sera attrapée par la gate, tout renommage de pont se voit au build.

## Verification

- `cargo build -p archel --features gui` : OK (Q_PROPERTY camelCase confirmés dans le C++ généré : `devicesJson`, `modalKind`, `launchX`...).
- qmllint : propre sur les 19 fichiers QML ; gate binding-spell : PASS.
- `cargo test -p archel --features gui` : 250 passed, 0 failed (les 2 échecs single_instance initiaux venaient de l'instance GUI ouverte de l'utilisateur tenant le socket abstrait).
- Smoke offscreen 15 s : exit 124 (event loop sain), **zéro warning QML** (avant : erreurs `Cannot assign [undefined]` à chaque tick).
- End-to-end BT : `archel bt devices` → WH-1000XM4 apparié ; `archel bt scan` → 12 appareils découverts en 12 s (même chemin `archel_core::bluetooth` que la GUI).

## Eliminated

- Hypothèse « bluetoothd absent » : le daemon est actif, l'adaptateur Powered (busctl + bluetoothctl).
- Hypothèse « scan ne collecte rien » : le core collecte via le CLI ; le défaut était purement la liaison QML.
- Rustines `|| ""` / `?:` de la session précédente : masquaient le symptôme sans traiter la cause (dont deux bindings auto-référentiels = boucles).

## Current Focus

- none — fix vérifié (build, lint, 250 tests, smoke zéro-warning, scan BT end-to-end).
