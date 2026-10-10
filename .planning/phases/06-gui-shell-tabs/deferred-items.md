# Phase 06 — Deferred Items

Out-of-scope discoveries logged during execution; not fixed in the plan that found them.

| Found in | Kind | Item |
|----------|------|------|
| 06-06 (Task 2 gates) | deviation | Tab QML files bind model properties in camelCase while the bridge qproperties are snake_case (works via Qt's case-insensitive-ish lookup today, but is inconsistent debt). Spans ~8 tab QML files. Needs one dedicated cleanup pass, not per-plan patches. Mirrored in `.planning/WINDOWS.md`. |
| 06-06 (gates) | flaky-test | `archel-core` `live_*` timing tests (install/update duration thresholds) fail under heavy parallel cargo load; pass on rerun. Pre-existing (06-02), timing-threshold sensitivity. |
| 06-06 (gates) | clippy-baseline | 29 pre-existing `archel-core` clippy findings block a clean `-D warnings` run repo-wide; out of execution scope, own-file warnings kept at zero. |
| 06-07 (Task 2 gates) | bug | `Main.qml:357 onAurValidated: pkg.aurBuildStart()` references the `pkg` id declared inside `PkgTab.qml` (component-scoped, no `property alias`) — a latent QML ReferenceError when the AUR Valider button is clicked. 06-07's own popup wiring uses the alias pattern (`netTab.net.…`) and is unaffected. Fix in owning plan (06-06) or the 06-09 check plan: expose `property alias` on PkgTab and qualify the call. |
