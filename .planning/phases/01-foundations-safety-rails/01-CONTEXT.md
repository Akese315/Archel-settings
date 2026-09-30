# Phase 1: Foundations & Safety Rails - Context

**Gathered:** 2026-09-30
**Status:** Ready for planning

## Phase Boundary

Phase 1 delivers the decided foundations everything else builds on: a compiling Cargo workspace (single `archel` binary + `archel-privileged` helper + core library), a typed French-language error model, a pkexec privilege boundary proven by an allowlisted dry-run, and a CXX-Qt spike that proves Qt 6.11 viability or triggers the Slint pivot. No domain logic, no GUI beyond the spike window.

## Implementation Decisions

### Binary layout
- **D-01:** Single `archel` binary with subcommands — CLI commands and GUI share one installed binary instead of separate `archel-cli` / `archel-settings` binaries. — **Reversibility:** one-way — splitting a shipped single binary into two published binaries breaks install scripts, PKGBUILD package layout, and user muscle memory (`archel <cmd>` paths).
- **D-02:** `archel settings` (no dash) launches the graphical interface; all other subcommands are headless CLI operations.
- **D-03:** Privileged operations live in a separate `archel-privileged` binary invoked via pkexec — not a hidden subcommand — keeping the attack surface to one tiny allowlisted executable.
- **D-04:** CLI subcommand and flag names are English (`archel settings`, `archel power`, `--json`); all human-readable output (help text, messages, errors) is French.

### Language FR/EN
- **D-05:** French is the primary user-facing language for CLI output, error messages, and (later) GUI text.
- **D-06:** GUI follows the same language as the CLI — one language everywhere, no per-frontend split.
- **D-07:** i18n infrastructure is prepared from Phase 1 (translation keys, no hardcoded French outside the FR catalog) even though the second language ships in v2 (PLAT-05 stays in v2).
- **D-08:** Code identifiers (functions, types, error variants) are English per Rust convention — French appears only in displayed strings.

### Privilege UX
- **D-09:** pkexec is the single escalation mechanism for both CLI and GUI — one consistent password experience everywhere, no `sudo` path in the CLI.
- **D-10:** Standard polkit auth persistence applies — after one successful authentication, subsequent privileged ops within the retention window don't re-prompt.
- **D-11:** yay keeps its native sudo handling for the AUR build/install step — Archel shells out to yay unprivileged and does not orchestrate its elevation.
- **D-12:** Failed/refused authentication surfaces a friendly French error naming the rejected action — no raw polkit/D-Bus error text leaks to the user.

### Spike fallback
- **D-13:** If CXX-Qt proves unbuildable against system Qt, Phase 1 auto-pivots to Slint without coming back to the user — the spike's job is to produce a working GUI foundation, not a report.
- **D-14:** Spike success bar is a window with vertical tabs and a live-updating label (proves the real GUI pattern), not a hello-world window (which would only prove the bridge links).
- **D-15:** No time-box on making CXX-Qt work — debugging effort is unbounded; only a hard build/link incompatibility with Qt 6.11 triggers the Slint pivot.

### the agent's Discretion
- Workspace crate names and internal module layout (research suggests `archel-api`/`archel-core` lib + bins — planner picks).
- Which i18n mechanism to use for the FR catalog (planner/researcher decision, e.g. gettext vs fluent vs project-fluent).
- Exact pkexec polkit policy file contents and allowlisted action set (privilege architecture detail for planner + security review).
- Spike implementation details beyond the agreed success bar.

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project definition
- `.planning/PROJECT.md` — What Archel is, core value (API-first, CLI-proven), constraints (Rust + Qt6, Arch + Hyprland, API before GUI), key decisions table
- `.planning/REQUIREMENTS.md` — 43 v1 requirements; Phase 1 owns PLAT-01 (shared Rust API library) and PLAT-04 (pkexec helper, yay unprivileged)
- `.planning/ROADMAP.md` — Phase 1 entry: goal, 4 success criteria (one-cargo-command build, spike window with vertical tabs + live label or explicit Slint pivot, pkexec dry-run JSON, typed ArchelError), Mode mvp

### Research
- `.planning/research/SUMMARY.md` — Synthesized findings: library-first no-daemon v1, CXX-Qt 0.10 + QML only viable Qt path (Slint 1.18.1 escape hatch), hyprland.conf never rewritten, pkexec-for-pacman-only, per-vendor GPU trait, CLI-proof gate rule
- `.planning/research/STACK.md` — Pinned versions (cxx-qt 0.10.0, tokio 1.53, zbus 5.19, hyprland 0.3.13 stable, alpm 5.0.2 read-only, sysinfo, nvml-wrapper), workspace layout, PKGBUILD split, what-not-to-use list
- `.planning/research/ARCHITECTURE.md` — Component boundaries, Qt/tokio coexistence via worker thread + queued signals, privilege boundary design, build order
- `.planning/research/PITFALLS.md` — 9 critical pitfalls incl. Qt binding risk (#1), hyprland.conf rewrite ban, pacman lock discipline, GPU telemetry backends, no-X11-tools rule

## Existing Code Insights

No existing code — greenfield repository. No reusable assets, patterns, or integration points. The workspace, conventions, and CI are all created in this phase.

## Specific Ideas

- User's own words framing the product shape: "dug genre archel-cli et archel settings" — resolved into single-binary layout (D-01/D-02).
- User writes specs in French with English technical terms mixed — matches D-04/D-05 split (English commands, French prose).

## Deferred Ideas

None — discussion stayed within phase scope. (PLAT-05 UI language toggle already lives in v2 requirements.)

---

*Phase: 1-Foundations & Safety Rails*
*Context gathered: 2026-09-30*
