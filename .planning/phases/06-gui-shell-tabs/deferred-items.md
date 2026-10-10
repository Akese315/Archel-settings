# Phase 06 — Deferred Items

Out-of-scope discoveries logged during execution; not fixed in the plan that found them.

| Found in | Kind | Item |
|----------|------|------|
| 06-06 (Task 2 gates) | deviation | Tab QML files bind model properties in camelCase while the bridge qproperties are snake_case (works via Qt's case-insensitive-ish lookup today, but is inconsistent debt). Spans ~8 tab QML files. Needs one dedicated cleanup pass, not per-plan patches. Mirrored in `.planning/WINDOWS.md`. |
| 06-06 (gates) | flaky-test | `archel-core` `live_*` timing tests (install/update duration thresholds) fail under heavy parallel cargo load; pass on rerun. Pre-existing (06-02), timing-threshold sensitivity. |
| 06-06 (gates) | clippy-baseline | 29 pre-existing `archel-core` clippy findings block a clean `-D warnings` run repo-wide; out of execution scope, own-file warnings kept at zero. |
| 06-07 (Task 2 gates) | bug | `Main.qml:357 onAurValidated: pkg.aurBuildStart()` references the `pkg` id declared inside `PkgTab.qml` (component-scoped, no `property alias`) — a latent QML ReferenceError when the AUR Valider button is clicked. 06-07's own popup wiring uses the alias pattern (`netTab.net.…`) and is unaffected. Fix in owning plan (06-06) or the 06-09 check plan: expose `property alias` on PkgTab and qualify the call. |
| 06-09 (gates) | failing-test | ~~`archel-core` `i18n::tests::no_hardcoded_french_diacritics_in_rust_sources` red~~ FIXED in `ebe84f8` (doc-comment diacritics removed; core suite 269 green). |
| code-review (waves 6-7) | fixed-blocker | CR-01: pairing-decline unpair failure swallowed (`bt.rs`) — now surfaces in ErrorModal + regression test. Fixed in `097a989`. |
| code-review (waves 6-7) | fixed-blocker | CR-02: keybind removal wrote a pre-confirm snapshot (`input.rs`) — now re-reads post-confirm, miss still fails pre-confirm + drift regression test. Fixed in `097a989`. |
| code-review (waves 6-7) | fixed | WR-06: `just lint-qml` word-split paths + cryptic no-qmllint failure — rewritten with presence guard + `-exec` (space-safe). Fixed in `097a989`. |
| code-review (waves 6-7) | follow-up | WR-01: 1 Hz tick overwrites in-progress Input tab edits — decouple editor state like NetTab (local state, write on Appliquer). |
| code-review (waves 6-7) | follow-up | WR-02: FileDialog `file://` strip breaks percent-encoded paths (VPN + wallpaper pickers) — resolve via `toLocalFile()`/`decodeURIComponent`. |
| code-review (waves 6-7) | follow-up | WR-03: single global confirm session multiplexes all tabs — add write-slot guard or generation-tagged confirms. |
| code-review (waves 6-7) | follow-up | WR-04: keybind-add live-evals before persisting — compensating unbind or persist-then-eval on post-eval persist failure. |
| code-review (waves 6-7) | follow-up | WR-05: GUI has no path back to automatic DNS (CLI `reset` leg unmapped) — add "Automatique" affordance or annotate cli-only. |
| code-review (waves 6-7) | advisory | IN-01: audio read-back matches on name not id; IN-02: NetTab hardcodes `"N/A"`; IN-03: thin-view audit parses Cargo.toml with string splits. |
