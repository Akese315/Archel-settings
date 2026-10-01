# Phase 3: Packages & CLI Harness - Context

**Gathered:** 2026-10-01
**Status:** Ready for planning

## Phase Boundary

Phase 3 delivers end-to-end package management through the CLI: list/search installed packages, install/remove/upgrade via pacman inside the pkexec helper (never as root), AUR via unprivileged yay with a PKGBUILD review step, plus shell completions making the CLI the full API test harness (PLAT-02). PKGBUILD diff review inside the GUI (PKG-06) is explicitly out of scope — later GUI phase.

## Implementation Decisions

### Install/remove/upgrade safety (pacman via helper)
- **D-01:** Archel asks for confirmation (`installer X ? [o/N]`) before every install/remove/upgrade, showing the pulled dependencies — never silent mutation.
- **D-02:** During the operation, pacman output streams live; no summarize-at-end mode.
- **D-03:** Single-package upgrade equivalent to bare `-Sy` is refused with a French message — only full `-Syu` (carries PKG-04; partial upgrades break Arch).

### AUR pre-install review (PKG-05)
- **D-04:** The pre-install card shows download + installed sizes, source (repo name or AUR), and candidate version; when already installed, installed-vs-candidate versions side by side.
- **D-05:** Before any AUR build, the full PKGBUILD is displayed and the user validates — no metadata-only shortcut. — **Reversibility:** costly — weakening this later would reopen a code-execution trust boundary.
- **D-06:** yay runs unprivileged with its native sudo handling (carries forward Phase 1 D-11); Archel never orchestrates its elevation.

### Search scope and ordering
- **D-07:** `pkg search` queries all servers (repos + AUR), sorted by descending version with repo packages before AUR on ties — pacman first, yay second.
- **D-08:** `pkg list` covers installed packages only (narrows display scope; search stays global).

### CLI harness (PLAT-02)
- **D-09:** Shell completions generated for bash + zsh + fish in one shot (clap_complete, near-zero cost) — no per-shell choice needed.
- **D-10:** Every Phase 2–3 capability is drivable with `--json` (global flag already locked); completions cover all subcommands.

### the agent's Discretion
- Exact `pkg` subcommand names beyond search/list/install/remove/upgrade (researcher proposes, EN per D-04), confirmation prompt wording (FR catalog keys), progress rendering details while streaming, JSON field names, completion install paths (user-local vs system-wide — packaging phase decides).

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

No external specs — requirements fully captured in decisions above. Phase sources:
- `.planning/REQUIREMENTS.md` — PKG-01..05, PLAT-02 (PKG-06 explicitly out of scope)
- `.planning/ROADMAP.md` — Phase 3 goal + 4 success criteria
- `.planning/phases/01-foundations-safety-rails/01-CONTEXT.md` — locked D-01..D-15 (single binary, FR/EN split, pkexec-only, yay native sudo)
- `.planning/phases/02-system-insight/02-CONTEXT.md` — CLI shape precedent (`sys` group + top-level commands, French tables, global `--json`)
- `.planning/phases/01-foundations-safety-rails/01-02-SUMMARY.md` — helper allowlist + pkexec client patterns the pacman actions extend

## Existing Code Insights

### Reusable Assets
- `crates/archel-privileged/src/main.rs` (exact-match argv allowlist → v1 JSON) — the model for new `pacman -S/-R/-Syu` allowlisted actions
- `crates/archel-core/src/privileged.rs` (spawn/parse/map, 126/127 → AuthFailed) — reused for helper invocation
- `crates/archel-core/src/updates.rs` (checkupdates/yay -Qu spawn as user) — read-side precedent for yay subprocess handling
- `crates/archel/src/cmds/` (clap derive + French table rendering) — where `pkg` group plugs in

### Established Patterns
- Privileged mutations go through `archel-privileged` via pkexec; reads stay unprivileged in-process
- Every user string via `fr.json` keys (diacritics audit); typed `ArchelError`, no panics
- `--json` global flag + insta snapshot pins for schemas (Phase 2 precedent)

### Integration Points
- New allowlisted helper actions extend the Phase 1 argv matrix (tests per action)
- `pkg` clap group alongside `sys`/`landing`; completions generated via clap_complete for three shells

## Specific Ideas

- User's words for the review card: "la taille, le serveur utilisé, et la version" — size, source, version
- User's search priority: "tous les serveurs, en priorisant la version puis le serveur (pacman en premier sinon yay)"
- User asked what a shell is — bash/zsh/fish explained; decision D-09 avoids the choice entirely

## Deferred Ideas

None — discussion stayed within phase scope (PKG-06 GUI diff review already roadmap-deferred).

---

*Phase: 3-Packages & CLI Harness*
*Context gathered: 2026-10-01*
