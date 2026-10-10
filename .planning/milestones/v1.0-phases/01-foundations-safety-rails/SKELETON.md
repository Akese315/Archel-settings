# Walking Skeleton — Archel

**Phase:** 1
**Generated:** 2026-09-30

## Capability Proven End-to-End

A user can run one `archel` binary whose subcommands call into the shared `archel-core` library and render French diagnostics, escalate a harmless dry-run through the pkexec helper boundary as versioned JSON, and open `archel settings` to a window with vertical tabs and a live-updating label.

## Architectural Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Binary layout | Single `archel` bin + subcommands + separate `archel-privileged` helper (D-01/D-03, one-way) | One installed binary, one tiny allowlisted root surface; GUI is a `settings` subcommand, not a second binary |
| GUI isolation | `gui` cargo feature (default-on); all Qt code behind `#[cfg(feature = "gui")]` | Headless builds/tests never link Qt; reconciles D-01 with the no-Qt CLI invariant |
| Qt bridge | CXX-Qt 0.10.0 pinned trio + QML, cargo-only build (`cargo_without_cmake`), `QMAKE=/usr/bin/qmake6` | Only production-viable Qt-from-Rust path; bare `qmake` on this machine is Qt 5.15.19 |
| Privilege | pkexec-only (no sudo path); exact argv allowlist; `{"v":1,…}` JSON protocol; 126/127 → friendly FR AuthFailed | pkexec does no argv validation — the helper is the second gate; raw polkit text never reaches the user |
| i18n | rust-i18n 4.2.4 catalog, `locales/fr.json` sole source of French; EN identifiers, FR display strings | v2 adds a language by adding a file; diacritics audit enforces the seam |
| Errors | thiserror `ArchelError` in core; `main` returns Result, no panics on failure paths | GUI/CLI match on variants; strings can't be matched or i18n-keyed |
| Toolchain | rustup stable, edition 2024, committed Cargo.lock | Reproducible builds for a binary that ships |

## Stack Touched in Phase 1

- [ ] Project scaffold (cargo workspace, rust-toolchain.toml, lint via diacritics audit, `cargo test` runner)
- [ ] Routing — clap subcommands (`version`, `privileged dry-run`, `settings`) dispatching to core
- [ ] Privilege boundary — one real allowlisted round-trip (dry-run JSON) through pkexec-shaped spawn surface
- [ ] UI — spike window with vertical tabs + 1 Hz live label wired through the cxx-qt bridge (proves the Rust→QML pattern)
- [ ] Local full-stack run commands — `cargo build --workspace --no-default-features` (headless) and `QMAKE=/usr/bin/qmake6 cargo run -p archel --features gui -- settings` (GUI)

## Out of Scope (Deferred to Later Slices)

- All domain logic (network, power, bluetooth, appearance, sound, packages, display, input, system stats) — each arrives with its owning phase behind the CLI-proof gate
- Real privileged actions (pacman transactions) — helper carries dry-run only until Phase 3
- hyprland.conf handling of any kind — Phase 5 owns the snippet-file strategy; the main conf is never rewritten
- Second UI language (PLAT-05, v2) — catalog is ready, English file is not shipped
- Slint wiring — pivot-only on hard CXX-Qt/Qt 6.11 incompatibility (D-13/D-15)
- Resident root daemon, sudo paths, separate archel-cli/archel-settings binaries — explicitly rejected by locked decisions

## Subsequent Slice Plan

Each later phase adds one vertical slice on top of this skeleton without altering its architectural decisions:

- Phase 2: System Insight — landing facts + live telemetry reads through archel-core, CLI-proven with `--json`
- Phase 3: Packages & CLI Harness — real helper actions (install/remove/Syu) + yay unprivileged + full CLI surface
- Phase 4: Connectivity & Devices — network, Bluetooth, sound, power over the proven API + privilege client
- Phase 5: Hyprland Desktop — appearance, display, input via snippet-file writes + live-apply
- Phase 6: GUI Shell & Tabs — vertical-tab GUI composing the already-proven API; no new capability unproven by CLI
