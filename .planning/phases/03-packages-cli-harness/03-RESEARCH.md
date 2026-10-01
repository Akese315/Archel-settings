# Phase 3: Packages & CLI Harness - Research

**Researched:** 2026-10-01
**Domain:** Arch package management (pacman/alpm, yay/AUR) + clap CLI surface
**Confidence:** HIGH

## Summary

Phase 3 builds end-to-end package management on top of the Phase 1 privilege boundary and the Phase 2 CLI-shape precedent. All live-machine probes succeeded: pacman 7.1.0 and yay 13.0.1 are present, `pacman -Si/-S --print/-R --print` all work unprivileged (only `-Syu --print` requires root), `yay -G` downloads a PKGBUILD cleanly as the user with no build, AUR RPC v5 answers directly, sync DBs are world-readable, and both new crates (`alpm` 5.0.2, `clap_complete` 4.6.11) pass the legitimacy gate with OK verdicts and were confirmed on the crates.io index via `cargo search`.

The single most important architectural finding: the Phase 1 helper model (exact-match argv allowlist) does **not** extend verbatim to `pacman -S/-R <packages>` because package names are open-ended argv. The helper's allowlist must become verb + strictly-validated-args (reject leading `-`, enforce a package-name charset, insert `--` before the package list, hardcode all flags server-side). Exact-match remains correct for the zero-argument `upgrade` → `pacman -Syu` action.

The second most important finding: pacman/yay human output is **localized** (French observed live: `installé`, `erreur : impossible de trouver la cible`). Every spawn whose stdout is parsed must pin `LC_ALL=C` via duct's `.env()` — line-counting in `updates.rs` is locale-proof, but Phase 3 field parsing is not.

**Primary recommendation:** alpm localdb for installed list/search (fast, locale-proof, structured `Ver` ordering via `alpm_pkg_vercmp`); `LC_ALL=C`-pinned `pacman -Ss/-Si` + `yay -Ss/-Si` subprocesses for repo/AUR queries; `yay -G` into a temp dir for the mandatory PKGBUILD review; helper verbs `pkg-install`/`pkg-remove`/`pkg-upgrade` with validated args + hardcoded `--noconfirm` (Archel already confirmed per D-01); live streaming via duct `reader()`; completions via `clap_complete::aot::generate()` to stdout for bash+zsh+fish in one subcommand.

## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Archel asks for confirmation (`installer X ? [o/N]`) before every install/remove/upgrade, showing the pulled dependencies — never silent mutation.
- **D-02:** During the operation, pacman output streams live; no summarize-at-end mode.
- **D-03:** Single-package upgrade equivalent to bare `-Sy` is refused with a French message — only full `-Syu` (carries PKG-04; partial upgrades break Arch).
- **D-04:** The pre-install card shows download + installed sizes, source (repo name or AUR), and candidate version; when already installed, installed-vs-candidate versions side by side.
- **D-05:** Before any AUR build, the full PKGBUILD is displayed and the user validates — no metadata-only shortcut. — **Reversibility:** costly — weakening this later would reopen a code-execution trust boundary.
- **D-06:** yay runs unprivileged with its native sudo handling (carries forward Phase 1 D-11); Archel never orchestrates its elevation.
- **D-07:** `pkg search` queries all servers (repos + AUR), sorted by descending version with repo packages before AUR on ties — pacman first, yay second.
- **D-08:** `pkg list` covers installed packages only (narrows display scope; search stays global).
- **D-09:** Shell completions generated for bash + zsh + fish in one shot (clap_complete, near-zero cost) — no per-shell choice needed.
- **D-10:** Every Phase 2–3 capability is drivable with `--json` (global flag already locked); completions cover all subcommands.

### The Agent's Discretion
- Exact `pkg` subcommand names beyond search/list/install/remove/upgrade (researcher proposes, EN per D-04), confirmation prompt wording (FR catalog keys), progress rendering details while streaming, JSON field names, completion install paths (user-local vs system-wide — packaging phase decides).

### Deferred Ideas (OUT OF SCOPE)
- None — discussion stayed within phase scope (PKG-06 GUI diff review already roadmap-deferred).

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PKG-01 | List and search installed packages (pacman metadata, read-only) | alpm localdb (`Alpm::new` + `localdb().pkgs()`) for list; `pacman -Ss`/`-Si` under `LC_ALL=C` for search; locale-pinning pitfall documented |
| PKG-02 | Install packages via pacman (through privileged helper) | Helper verbs + validated-args pattern; `pacman -S --print` unprivileged dep preview; `--noconfirm`-in-helper after Archel confirms |
| PKG-03 | Remove packages via pacman (through privileged helper) | Same helper pattern; `pacman -R --print` unprivileged preview; `-Rns` vs `-R` flag choice |
| PKG-04 | Full `-Syu` update, never `-Sy` alone | No partial-upgrade code path exists; French refusal message; `-Syu --print` needs root so upgrade preview differs from install preview |
| PKG-05 | AUR via yay as user, with review step | `yay -G` temp-dir PKGBUILD fetch (verified); `yay -Si`/AUR RPC for metadata; D-05 display+validate flow; `--noconfirm` interplay |
| PLAT-02 | Full API surface drivable from CLI with `--json` + completions | `clap_complete::aot::generate()` stdout pattern for 3 shells; global `--json` precedent; insta snapshot precedent |

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Installed list/search (PKG-01) | API / Backend (`archel-core`) | — | Pure read logic; CLI renders, GUI reuses later |
| Pre-install card + dep preview (D-01/D-04) | API / Backend | — | Composed in core from alpm + `pacman -S --print`; frontends only display |
| Confirmation prompt (D-01) | CLI frontend (`archel` bin) | — | TTY interaction belongs to the frontend; core exposes a non-interactive API + the bin gates on TTY |
| Privileged mutation (PKG-02/03/04) | Privileged helper (`archel-privileged`) | API client (`privileged.rs`) | Root execution isolated in the tiny helper; core only spawns via pkexec |
| Live output streaming (D-02) | CLI frontend | Helper (produces the stream) | Helper's stdout pipes through pkexec to the bin, which tees to terminal + transcript |
| AUR fetch/review/build (PKG-05) | API / Backend (fetch+parse) + CLI (display+confirm) | — | yay runs as the user, no privilege boundary; review display is a CLI concern |
| Completions (D-09) | CLI frontend | — | Generated from the clap `Command` definition at runtime; no core logic |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `alpm` | 5.0.2 [VERIFIED: cargo search] | libalpm bindings: installed-DB list/search, version compare | Official Arch bindings (`github.com/archlinux/alpm.rs`), links system libalpm 16.0.1; `Ver: PartialOrd` via `alpm_pkg_vercmp` gives correct epoch-aware version-desc sort for D-07; locale-proof (no output parsing). Legitimacy gate: OK |
| `clap_complete` | 4.6.11 [VERIFIED: cargo search + vendored source] | Shell completion generation for bash/zsh/fish | Companion of locked-in clap 4.6.7; `aot::{generate, shells::{Bash,Zsh,Fish}}` API read verbatim from vendored 4.6.11 source. Legitimacy gate: OK (2.3M downloads/wk) |
| `duct` (already pinned) | 1.1.2 [VERIFIED: updates.rs:179-194] | Subprocess spawn for pacman/yay reads + streaming | Established Phase 1/2 pattern; `.env()` for `LC_ALL=C` pinning; `.reader()` → `ReaderHandle` for D-02 live streaming; `start()` + `wait_timeout()` (no timeout builder — Phase 2 deviation #1) |
| `which` (already pinned) | 8.0.6 [VERIFIED: updates.rs:176] | Capability detection for `pacman`/`yay` binaries | Established pattern: missing tool → typed error + French hint, never a crash |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| system `pacman` CLI | 7.1.0 / libalpm 16.0.1 [VERIFIED: live probe] | Sync-DB queries (`-Ss`/`-Si`), unprivileged `-S/-R --print` dep previews | Whenever sync-DB data is needed — avoids alpm syncdb registration + `/etc/pacman.conf` parsing |
| system `yay` | 13.0.1 (`yay-bin`) [VERIFIED: live probe] | AUR search/info/fetch/build, `-Qu --aur` | All PKG-05 paths; always spawned as the invoking user (D-06/D-11) |
| AUR RPC v5 | `https://aur.archlinux.org/rpc/v5/` [VERIFIED: live probe] | Structured AUR metadata (votes, popularity, OutOfDate, deps arrays) | Trust signals on the review card + fallback when `yay -Si` parsing is awkward; plain HTTPS GET, no auth |
| `bash -n` (system) | n/a | PKGBUILD syntax check without execution | D-05 validation step: static syntax gate before display+confirm, never executes the file |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| alpm localdb reads | `pacman -Q`/`-Qi` parsing | Subprocess parsing works but pays fork+parse per call and needs `LC_ALL=C`; alpm is in-process, structured, and already STACK-blessed read-only |
| `pacman -Ss` subprocess | alpm syncdb registration | Registering syncdbs requires parsing `/etc/pacman.conf` for servers; subprocess inherits pacman.conf/mirrorlist for free — same argument as STACK's "alpm stays read-side" for mutations |
| `yay -Si` for AUR metadata | AUR RPC only | RPC is cleaner JSON but adds an HTTP client dep (`reqwest`/`ureq`) the workspace doesn't have; `yay -Si` reuses the existing duct+which pattern with zero new deps — use RPC only if `yay` is absent is NOT an option (yay is required for AUR anyway) |
| Hand-rolled stdin confirm | `dialoguer`/`inquire` crate | A one-line `[o/N]` prompt does not justify a new interactive dependency + its transitive tree next to a privilege boundary; hand-roll with strict default-No + non-TTY refusal |
| `pacman -Sp` for AUR dep preview | `yay -Si` Depends/MakeDeps parsing | `yay -S --print`/`-Sp` FAILS on AUR targets (`target not found` — verified live); AUR dep preview must come from `-Si` or RPC |

**Installation:**
```bash
# No system packages to install. Crate additions (archel-core):
cargo add -p archel-core alpm --features checkver
cargo add -p archel completions-dep  # actually: cargo add -p archel clap_complete
```

**Version verification:** `cargo search` (sparse index, live): `alpm = "5.0.2"`, `clap_complete = "4.6.11"`, companion `clap = "4.6.7"` matches the workspace lock [VERIFIED: live cargo search + /tmp/ccprobe Cargo.lock]. `checkver` and `generate` features confirmed in alpm 5.0.2's Cargo.toml [VERIFIED: vendored registry source]. Enable `checkver` so the build fails loudly on libalpm mismatch (STACK carry-forward).

## Package Legitimacy Audit

Gate run this session via `query package-legitimacy check --ecosystem crates alpm clap_complete` — both OK.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `alpm` | crates.io | 10 yrs (since 2016) | 12.5k/wk | github.com/archlinux/alpm.rs | OK | Approved |
| `clap_complete` | crates.io | 4+ yrs (since 2021) | 2.3M/wk | github.com/clap-rs/clap | OK | Approved |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
archel pkg <sub> ── reads ──► archel-core packages.rs ─┬─ alpm localdb (installed list/search, Ver compare)
                │                                       ├─ pacman -Ss/-Si (LC_ALL=C) (repo search/info)
                │                                       └─ yay -Ss/-Si + AUR RPC (AUR search/info)
                │
                ├── mutation ──► confirm [o/N] (TTY-gated, default N)
                │                     │
                │                     ├── repo: pre-install card ──► privileged.rs ──► pkexec archel-privileged
                │                     │     (sizes+source+version,    pkg-install/pkg-remove/
                │                     │      dep preview via          pkg-upgrade verbs,
                │                     │      -S/-R --print)           validated args) ──► pacman -S/-R/-Syu
                │                     │                                                     │ (live stdout)
                │                     │◄────────────── streams back (D-02) ───────────────┘
                │                     │
                │                     └── AUR: yay -G PKGBUILD ──► display + bash -n ──► confirm
                │                                                       ──► yay -S (as user, native sudo)
                │
                └── completions <shell> ──► clap_complete::aot::generate() ──► stdout
```

### Recommended Project Structure

```
crates/archel-core/src/
├── packages.rs       # NEW: PackageInfo types, alpm reads, pacman/yay query parsing,
│                     #         pre-install card composer, version-desc sort (D-07)
├── aur.rs            # NEW (or inside packages.rs): yay -G fetch, PKGBUILD validate,
│                     #         AUR RPC trust signals — planner picks the split
├── privileged.rs     # EXTEND: pkg_install/pkg_remove/pkg_upgrade via pkexec (streaming)
└── error.rs          # EXTEND: PackageNotFound, YayMissing, NeedsTty, PkgbuildRejected…
crates/archel-privileged/src/main.rs  # EXTEND: pkg-install/pkg-remove/pkg-upgrade verbs
crates/archel/src/cmds/
├── pkg_cmds.rs       # NEW: pkg group (list/search/install/remove/upgrade/info/completions)
policy/org.archel.privileged.policy   # EXTEND: one action id per new verb
```

### Pattern 1: Helper verb + strictly-validated args (NOT exact-match argv)

**What:** The Phase 1 exact-match allowlist (`["dry-run"]`) cannot cover `pacman -S <open-ended packages>`. New helper verbs take a fixed operation plus a validated package list: every package arg must match `^[a-zA-Z0-9@._+\-]+$` and must not start with `-`; the helper inserts `--` between flags and packages; ALL flags (`--noconfirm`, `-S`/`-Rns`/`-Syu`) are hardcoded server-side, never taken from argv. Zero-arg `pkg-upgrade` stays exact-match (`["pkg-upgrade"]` → `pacman -Syu`).

**When to use:** Every new privileged action in this phase.

**Why:** Without the leading-dash reject + `--` separator, a package named `-foo` becomes flag injection into a root pacman (`pacman -S --noconfirm -foo`); without server-side flags, argv smuggling re-adds `--overwrite`/`--needed`. No shell is ever involved (spawn argv directly — Phase 1 rule carries over).

### Pattern 2: Unprivileged `--print` preview, privileged execution

**What:** `pacman -S --print <pkgs>` (verified: prints 20 download URLs for gimp, exit 0, no root) and `pacman -R --print <pkgs>` give the D-01 dependency preview WITHOUT privilege. The Archel-side confirmation happens against this preview; only the confirmed transaction goes through pkexec, where the helper passes `--noconfirm` (no double-prompt). Exception: `-Syu --print` REQUIRES root (verified live: `you cannot perform this operation unless you are root`) — so the upgrade preview cannot use `--print`; compose it from `pacman -Qu` (pending list, works unprivileged — verified empty/exit-0 when clean) plus per-package `-Si` sizes.

**When to use:** D-01 confirmation flow for install/remove; upgrade uses the `-Qu` variant.

### Pattern 3: `LC_ALL=C`-pinned parse-bound spawns

**What:** Every duct spawn whose stdout is field-parsed sets `.env("LC_ALL", "C")` (duct `.env()` confirmed in vendored 1.x source). Under `LC_ALL=C`, `pacman -Si` fields are `Repository/Name/Version/Download Size/Installed Size/Depends On/Optional Deps` [VERIFIED: live `LC_ALL=C pacman -Si firefox` full output] and `yay -Si` adds `AUR URL/Maintainer/Popularity/Votes/Out-of-date` [VERIFIED: live].

**When to use:** All `pacman -Ss/-Si`, `yay -Ss/-Si` parsing. Line-counting (`updates.rs`) stays locale-free and needs no change.

### Pattern 4: TTY-gated confirmation with default-No

**What:** Hand-rolled `eprint!` prompt + `stdin().read_line()`; empty input, EOF, or `o`≠yes-prefix (accept `o`/`O`/`oui…` prefix, FR) → abort with a typed `ConfirmationAborted` (exit non-zero, no mutation). Non-TTY stdin (piped/`--json` scripts) → typed `NeedsTty` French error refusing the mutation (safe default: scripts must not mutate blindly; `--json` still works for all reads and for rendering card/result objects).

**When to use:** Every D-01 confirmation (install/remove/upgrade/AUR build).

### Anti-Patterns to Avoid
- **Exact-match allowlist for package verbs:** `handle_args(&["pkg-install", "firefox"])` can never be an exact string match — validate, don't enumerate.
- **Passing user-influenced flags to the helper:** flags are hardcoded per verb; the CLI never forwards `--overwrite`, `--needed`, `--noconfirm`, or `-y` from the user.
- **Bare `-Sy` anywhere:** no subcommand, flag, or helper verb constructs `-Sy`; DB refresh happens only inside `-Syu`. The D-03 French refusal is rendered when the user attempts the closest mappable thing (planner picks the trigger: e.g. a rejected `--refresh-only`-style input — there is deliberately no such flag).
- **`yay --noconfirm` for the whole AUR flow:** [ASSUMED — verify live] `--noconfirm` would also skip yay's diff/edit menus, which is only acceptable BECAUSE Archel performs its own D-05 review first. Never pass it before the review; scope it to the post-confirm build invocation.
- **Parsing localized output:** any `grep "^Name"`-style parse without `LC_ALL=C` breaks on French systems (live: fields render in French locale otherwise).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Installed-DB reads + version compare | `pacman -Q` text scraping | `alpm` localdb + `Ver: PartialOrd` (`alpm_pkg_vercmp`) | Epoch-aware `1:2.0-1` vs `2.0-1` ordering, no fork, no locale hazard; 1147 local entries iterate in-process |
| Shell completions | Hand-written bash/zsh/fish scripts | `clap_complete::aot::generate()` | Generated from the single clap definition — never drifts from actual subcommands; 3 shells in a loop |
| AUR metadata | AUR RPC HTTP client (`reqwest`) | `yay -Si` parsing (+ RPC only as enhancement later) | Zero new deps; yay is mandatory for AUR builds anyway; RPC shape stays an option if `yay -Si` proves brittle |
| Privilege escalation | sudo wrappers, setuid, embedded password prompts | Existing `run_action_via_pkexec` + extended policy | Phase 1–proven 126/127 French mapping; one consistent polkit experience (D-09/D-10) |
| pacman transactions via alpm | `alpm` transaction/commit API | `pacman` subprocess in the helper | STACK rule carries over: reimplementing signature/keyring/hook handling duplicates pacman; subprocess inherits pacman.conf semantics |
| PKGBUILD execution sandbox | Custom build isolation | `yay`/`makepkg` as the user (D-06) | makepkg already drops privileges/fakeroot handling; Archel's job is review-before-build, not sandboxing |

**Key insight:** This phase is a composition phase, not an invention phase — every hard problem (privilege, parsing, version compare, completions, AUR trust) already has a system tool or pinned crate that owns it. The new code is allowlist validation, French rendering, and JSON framing.

## Common Pitfalls

### Pitfall 1: Flag injection through package names
**What goes wrong:** Helper runs `pacman -S <user-pkg>` as root; a crafted name starting with `-` becomes a pacman flag.
**Why it happens:** Exact-match thinking carried over from `dry-run` to open-ended verbs.
**How to avoid:** Charset validation + leading-dash reject + `--` separator + server-side-only flags (Pattern 1). Unit-test the validator with `-foo`, `--overwrite`, `foo;bar`, empty string.
**Warning signs:** Any helper code that concatenates argv into a command string, or any test missing the `-`-prefix case.

### Pitfall 2: Locale-dependent parse breaks on French systems
**What goes wrong:** `pacman -Si` field grep works in dev (C locale) but returns nothing on the user's French machine.
**Why it happens:** pacman/yay localize human output (verified live: `installé`, `erreur : impossible…`).
**How to avoid:** `.env("LC_ALL", "C")` on every parse-bound spawn; fixture tests feed `LC_ALL=C`-shaped samples (the verified field list above).
**Warning signs:** Tests that spawn the real tool without env pinning, or that pass locally but fail under `LANG=fr_FR.UTF-8`.

### Pitfall 3: `pacman -S` on a stale DB = accidental partial upgrade
**What goes wrong:** Installing one package weeks after the last `-Syu` pulls a new package against old deps — the exact breakage D-03 exists to prevent.
**Why it happens:** `pacman -S pkg` never refreshes DBs by itself, and Phase 3 must NOT run `-Sy` first (that would be the forbidden bare sync).
**How to avoid:** Never auto-refresh in the install path; surface DB age on the pre-install card if cheaply available [ASSUMED — planner to decide: `stat /var/lib/pacman/sync/*.db` mtime is world-readable, verified `-rw-r--r--`]; the upgrade path (`-Syu`) is the only refresh. Document the rule in the card footnote.
**Warning signs:** Any code path that runs `pacman -Sy` without `-u` in the same transaction.

### Pitfall 4: `-Syu --print` needs root — upgrade preview needs a different source
**What goes wrong:** Reusing the install-preview (`--print`) code for `pkg upgrade` fails with `you cannot perform this operation unless you are root` (verified live).
**Why it happens:** `-Syu` touches sync DBs even in `--print` mode.
**How to avoid:** Upgrade preview = `pacman -Qu` pending list + per-package `-Si` sizes (both unprivileged); the privileged helper runs the real `-Syu` with streaming.
**Warning signs:** A shared `preview(action)` function that doesn't branch on upgrade.

### Pitfall 5: `yay -S --print` cannot preview AUR targets
**What goes wrong:** Dep preview for an AUR package errors `target not found` (verified live twice) while `yay -Si` on the same name works.
**Why it happens:** `-S --print`/`-Sp` resolves through repo sync DBs only.
**How to avoid:** AUR dep preview = `yay -Si` `Depends On` + `Make Deps` fields (or AUR RPC `Depends`/`MakeDepends` arrays) — never `--print`.
**Warning signs:** Copy-pasting the repo preview path for AUR packages.

### Pitfall 6: Double-prompt (Archel confirm + pacman/yay confirm)
**What goes wrong:** User confirms once in Archel, then pacman asks `Proceed? [Y/n]` again inside pkexec (or yay shows its diff/edit/proceed menus after the D-05 review).
**Why it happens:** Confirmation lives at two layers by default.
**How to avoid:** Archel confirms (D-01, TTY); helper hardcodes `--noconfirm` for pacman; yay gets `--noconfirm` (or targeted `--answerdiff N --answeredit N…` [ASSUMED — verify which combination skips menus but keeps sudo auth]) ONLY after the D-05 review + Archel confirm both pass.
**Warning signs:** A mutation path with no `--noconfirm` anywhere, or `--noconfirm` passed before the review step.

### Pitfall 7: Streaming deadlock on full pipe buffers
**What goes wrong:** Capturing a long `pacman -Syu` stdout+stderr with `stdout_capture().stderr_capture().run()` can block if the child fills the OS pipe while the parent waits.
**Why it happens:** `run()` waits for exit before draining when both streams are captured without a reader thread.
**How to avoid:** D-02 streaming via `Expression::reader()` (confirmed in duct 1.x source) + `BufRead::lines()` tee: print each line immediately AND append to the transcript buffer. Never `run()` with both captures on a long-lived child.
**Warning signs:** `run()` + `stdout_capture` on the pkexec/pacman path; tests that only cover short outputs.

## Code Examples

Verified patterns from local sources (vendored crate sources + live probes):

### alpm installed-DB read + version-desc sort
```rust
// Source: vendored alpm-5.0.2 src (Alpm::new: alpm.rs:48, localdb: handle.rs:373,
//   Db::pkgs: db.rs:151, Ver PartialOrd via alpm_pkg_vercmp: version.rs:63)
use alpm::Alpm;

let alpm = Alpm::new("/", "/var/lib/pacman")?; // root + dbpath, links system libalpm 16.0.1
let local = alpm.localdb();
let mut pkgs: Vec<_> = local.pkgs().iter().collect();
// D-07 sort: descending version; Ver implements PartialOrd (epoch-aware).
// NOTE: PartialOrd only (no Ord) — sort with partial_cmp + fallback. [ASSUMED detail]
pkgs.sort_by(|a, b| b.version().partial_cmp(a.version()).unwrap_or(std::cmp::Ordering::Equal));
```

### clap_complete stdout generation for 3 shells (D-09)
```rust
// Source: vendored clap_complete-4.6.11 src/aot/generator/mod.rs (generate: takes
//   `&mut Command`, writes to `&mut dyn Write`) + shells/{bash,zsh,fish}.rs structs.
// NOTE: root re-exports are marked Deprecated — use the `aot::` path.
use clap_complete::aot::{Shell, generate}; // Shell: Bash/Fish/Zsh (+Elvish/PowerShell) [VERIFIED: shell.rs:15]
use std::io;

// `archel completions <shell>` — or loop [Shell::Bash, Shell::Zsh, Shell::Fish] for one-shot output.
let mut cmd = Cli::command(); // clap CommandFactory
generate(shell, &mut cmd, "archel", &mut io::stdout());
```

### Live-streaming spawn (D-02) with transcript
```rust
// Source: vendored duct-1.x src/lib.rs (Expression::reader:357, ReaderHandle) +
//   Phase 2 precedent (updates.rs:179-194: start + wait_timeout; NO timeout builder).
// Stream lines live AND keep a transcript for --json; stderr stays log-only.
use std::io::{BufRead, BufReader};

let handle = duct::cmd(&pkexec, [&helper, &action_arg])
    .env("LC_ALL", "C") // duct .env confirmed (lib.rs:772); harmless on the privileged path
    .stderr_capture()
    .reader()?; // ReaderHandle: child stdout as a Read — no pipe deadlock (Pitfall 7)
let mut transcript = String::new();
for line in BufReader::new(&handle).lines().map_while(Result::ok) {
    println!("{line}"); // D-02: live
    transcript.push_str(&line);
    transcript.push('\n');
}
```

### PKGBUILD fetch + static validation (D-05)
```bash
# VERIFIED live on this machine (no build, runs as user):
mkdir -p "$(mktemp -d)/aur-review" && cd "$_"
yay -G <pkgname>        # -> ":: (1/1) PKGBUILD téléchargé : <pkgname>", clones AUR repo locally
bash -n <pkgname>/PKGBUILD && echo SYNTAX-OK   # static syntax check, never executes [ASSUMED safe]
# Then: display the FULL file, show AUR RPC OutOfDate/Votes/Popularity, ask confirm (Pattern 4).
```

### Parse-bound query shape (repo + AUR info)
```rust
// Field names VERIFIED live under LC_ALL=C:
//   pacman -Si: Repository|Name|Version|Download Size|Installed Size|Depends On|Optional Deps
//   yay -Si (+AUR): AUR URL|Maintainer|Popularity|Votes|Out-of-date (+ Make Deps|Check Deps)
let out = duct::cmd(which::which("pacman")?, ["-Si", name])
    .env("LC_ALL", "C")
    .stdout_capture().stderr_capture().run()?;
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `qmetaobject`/hand-written completions | `clap_complete` 4.x `aot::` API | clap 4.x era; root re-exports now deprecated in 4.6.11 | Use `clap_complete::aot::{generate, shells::…}` — the doc examples showing root imports are stale |
| `pacman -Q` scraping for installed data | `alpm` 5.x localdb | alpm.rs matured (2016→, 5.0.2 Jan 2026) | Structured reads, no locale hazard, epoch-correct `Ver` ordering |
| Bare `-Sy` before `-S` (forum folklore) | Never sync without `-u` | Long-standing Arch doctrine | D-03: no code path constructs `-Sy`; only full `-Syu` exists |
| yay handling its own diff/edit review | Archel-side full PKGBUILD display (D-05) | This phase (costly reversibility) | yay menus get pre-answered/skipped post-confirm; the trust boundary lives in Archel's review, not yay's pager |

**Deprecated/outdated:**
- `clap_complete` root-level `generate`/`generate_to` imports: marked `Deprecated, see aot` in 4.6.11 source — use `aot::` path in new code.
- `yay -S --print` / `yay -Sp` for AUR dep previews: resolves repos only, `target not found` on AUR names — do not use.
- `wireplumber`-style single-purpose crates for yay: none exist — subprocess is the blessed pattern (STACK carry-forward).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `yay --noconfirm` (post-review) skips menus but keeps native sudo auth working | Pitfalls-6 | If it also breaks sudo, AUR installs hang/fail — verify live with a trivial AUR package before locking the flag set |
| A2 | `--answerdiff N --answeredit N` (+`--answerclean/--answerupgrade`) as finer-grained alternative to blanket `--noconfirm` | Pitfalls-6 | Wrong flag combo → double-prompt or skipped review; probe `yay -S --help` combos live |
| A3 | `bash -n PKGBUILD` is a sufficient static gate + greppable danger patterns (curl-pipe, sudo, `rm -rf /`) are a reasonable heuristic layer | Code Examples (PKGBUILD) | Over-blocking legit PKGBUILDs or false confidence; planner defines the exact validation set (agent discretion covers prompt wording, validation depth is planner's call) |
| A4 | DB-age footnote via `stat /var/lib/pacman/sync/*.db` mtime is acceptable for Pitfall 3 | Pitfalls-3 | Minor: card shows misleading age if user syncs via other tools — cosmetic only |
| A5 | pkexec forwards child stdout as a live pipe (no buffering that defeats D-02) | Pattern 2 / Arch diagram | If pkexec buffers, streaming degrades to batch — fallback: helper writes progress markers; verify on the live desktop (needs polkit agent, unavailable in this shell) |
| A6 | alpm `Ver` has only `PartialOrd` (no `Ord`) — sort needs `partial_cmp` + fallback | Code Examples (alpm) | Build error surfaces it immediately — trivially fixable, zero plan risk |
| A7 | `Shell` implements `ValueEnum`/`FromStr` for the `completions <shell>` arg (imports suggest it) | Code Examples (completions) | If not, wrap in a local `#[derive(ValueEnum)]` enum mapping to `Shell::Bash/Zsh/Fish` — 10 lines |
| A8 | Orphan-autoremove flag choice: `-Rns` (recursive + config backup removal) as the default remove verb | Architecture (structure) | Destructive default questioned at plan-check; alternative: `-R` plain + `--recursive` opt-in flag — planner + plan-checker decide |

## Open Questions

1. **Exact `pkg` subcommand inventory**
   - What we know: search/list/install/remove/upgrade required; EN names, FR output (D-04/05 Phase 1). `info` (pre-install card on demand) and `completions` are near-certain additions.
   - What's unclear: `update` vs `upgrade` naming; whether `info`/`completions`/`aur review` are separate subcommands or flags.
   - Recommendation: planner proposes (`pkg search|list|install|remove|upgrade|info`, `archel completions <shell>`); plan-checker validates against D-07/D-08 wording.

2. **New `ArchelError` variants + fr.json keys**
   - What we know: `error.rs` has 8 variants [VERIFIED: error.rs:24-58]; none cover package-not-found, yay-missing, non-TTY refusal, confirmation-aborted, PKGBUILD-rejected.
   - What's unclear: exact variant set + French wording (agent discretion covers wording).
   - Recommendation: planner adds `PackageNotFound{…}`, `YayMissing`, `NeedsTty`, `ConfirmationAborted`, `PkgbuildRejected{…}` (or fewer, merged) with catalog keys + diacritics-audit compliance.

3. **Policy action ids for the three verbs**
   - What we know: policy shape is one-action-per-verb, `auth_admin_keep` triple, never `yes` [VERIFIED: policy file:19-27].
   - What's unclear: id strings (`org.archel.privileged.pkg-install` …) and install path (packaging phase owns install; Phase 3 needs a dev-install story for testing).
   - Recommendation: planner mirrors the dry-run block ×3; executor tests policy XML validity headless (minidom precedent) and live auth on the desktop.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `pacman` | PKG-01..04 queries + helper transactions | ✓ | 7.1.0 / libalpm 16.0.1 | — (Arch-only app, no fallback) |
| `yay` | PKG-05 all AUR paths | ✓ | 13.0.1 (`yay-bin`) | Typed `YayMissing` + French install hint (updates.rs precedent) |
| `checkupdates` | Untouched (Phase 2 cache) | ✓ | present | Already handled in Phase 2 |
| `vercmp` | Optional: shell-side version compare experiments | ✓ | present | alpm `Ver` ordering (preferred) |
| `crates.io` sparse index | Adding `alpm` + `clap_complete` | ✓ (`cargo search`/`cargo add` work) | — | Vendored lockfile (already fetched 4.6.11 once) |
| AUR RPC (`aur.archlinux.org/rpc`) | Trust signals (votes/OutOfDate) | ✓ (live 200 with JSON) | v5 | `yay -Si` fields (same data, no HTTP) |
| polkit agent (desktop dialog) | Live pkexec streaming test (A5) | ✗ (this shell: no `/dev/tty` agent) | — | Unit tests (126/127 matrix) + verify-work human check on Hyprland session |
| C++ / Qt | Nothing in Phase 3 | n/a | — | Headless gates (`--no-default-features`) apply to all new tests |

**Missing dependencies with no fallback:** none — every tool probed present except the interactive polkit agent, which only affects live (not automated) verification.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in) + `insta` 1.48 (snapshot pins, STACK-blessed) |
| Config file | none — see Wave 0 |
| Quick run command | `cargo test -p archel-core -p archel --no-default-features` |
| Full suite command | `cargo test --workspace --no-default-features && QMAKE=/usr/bin/qmake6 cargo test --workspace` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PKG-01 | alpm list returns installed set; search finds known pkg | unit (fixture alpm data where possible; live `pacman -Q` cross-check) | `cargo test -p archel-core packages::` | ❌ Wave 0 |
| PKG-02 | Helper rejects `-foo`/`--overwrite`/empty; accepts `firefox`; LIVE install of a trivial pkg streams + exits 0 | unit (validator matrix) + live manual | `cargo test -p archel-privileged` + `archel pkg install <tiny>` | ❌ Wave 0 |
| PKG-03 | Same validator matrix; `-R --print` preview parses under `LC_ALL=C` | unit + live manual | `cargo test -p archel-privileged` | ❌ Wave 0 |
| PKG-04 | No `-Sy` constructor exists (grep gate); upgrade streams to completion | unit (grep/assert no `-Sy` literal outside the refusal test) + live | `cargo test -p archel-core` + `rg '"-Sy"' crates/` | ❌ Wave 0 |
| PKG-05 | `yay -G` fetch + `bash -n` gate + full display + confirm-abort path (no build on abort) | unit (abort path) + live manual (real AUR review, abort first) | `cargo test -p archel-core aur::` | ❌ Wave 0 |
| PLAT-02 | `completions <bash\|zsh\|fish>` exits 0, non-empty, contains `archel`; `--json` parses on every new subcommand | unit (generate-to-buffer) + snapshot | `cargo test -p archel` + `… \| python3 -c json.load` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p archel-core -p archel -p archel-privileged --no-default-features`
- **Per wave merge:** full headless suite + `rg '"-Sy"'` negative gate + diacritics audit (Phase 1 precedent)
- **Phase gate:** Full suite green before `/gsd-verify-work` (live install/remove/upgrade/AUR-review are human-gated: real mutations on the dev machine)

### Wave 0 Gaps
- [ ] `crates/archel-core/src/packages.rs` (+ tests: `LC_ALL=C` fixture parsing, version-desc sort, card composer)
- [ ] `crates/archel-core/src/aur.rs` or same file (tests: abort-before-build, `bash -n` gate wiring)
- [ ] `crates/archel/src/cmds/pkg_cmds.rs` (tests: TTY-gate, `--json` framing, completions non-empty per shell)
- [ ] Helper validator unit tests in `crates/archel-privileged/src/main.rs` (allowlist matrix ×3 verbs + injection cases)
- [ ] `insta` snapshot pins for new `--json` schemas (card, search, list) — Phase 2 precedent
- [ ] Framework install: `cargo add -p archel-core alpm --features checkver && cargo add -p archel clap_complete` — versions verified above

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | Partial | polkit `auth_admin_keep` per verb (Phase 1 shape); 126/127 → friendly French, no raw leaks |
| V4 Access Control | Yes | Verb allowlist + arg validation + server-side flags; helper re-validates every invocation (never trusts the CLI) |
| V5 Input Validation | Yes | Package-name charset + leading-dash reject + `--` separator (Pattern 1); `LC_ALL=C` canonicalization before parsing |
| V6 Cryptography | No | No new crypto; pacman signature checking inherited from system pacman.conf |
| V1 Architecture | Yes | Least-privilege split preserved: helper stays zero-dep fixed-JSON (Phase 1 decision); new verbs add no deserialization in the root process |

### Known Threat Patterns for pacman/yay-via-helper

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Flag injection via crafted package name (`-foo`, `--overwrite`) | Tampering / Elevation | Charset + leading-dash reject + `--` + hardcoded flags; matrix tests |
| PATH trojan redirecting `pacman`/`yay` in helper | Spoofing | Absolute paths in helper (`/usr/bin/pacman`); `which` gate only on the unprivileged side |
| Malicious PKGBUILD = arbitrary code at build | Tampering | D-05 mandatory full display + `bash -n` + explicit confirm; build as user (D-06), never as root; OutOfDate/votes signals |
| `--noconfirm` passed before review | Repudiation bypass | Flag ordering constraint: review + Archel-confirm precede any skip-confirm flag (Pitfall 6) |
| Stale-DB partial upgrade breaking the system | Tampering (integrity) | No `-Sy` path; `-Syu` is the only refresh (D-03); DB-age footnote (A4) |
| Raw polkit/pacman errors leaking internals | Information disclosure | Established rule: stderr log-only, catalog French to terminal (T-1-02 carries over) |

## Sources

### Primary (HIGH confidence)
- Live machine probes (this session): `pacman -Si/-Qi/-Ss/-S --print/-R --print/-Syu --print/-Qu` shapes + exit codes; `yay -Ss/-Si/-G/-S --print/-Sp` shapes; `yay-bin 13.0.1`, `pacman 7.1.0/libalpm 16.0.1`; sync-DB world-readable; AUR RPC v5 live JSON; `cargo search` versions (alpm 5.0.2, clap_complete 4.6.11, clap 4.6.7)
- Vendored crate sources read this session: `clap_complete-4.6.11/src/aot/{generator/mod.rs,shells/shell.rs,shells/{bash,zsh,fish}.rs}` (generate signature, Shell enum, deprecated root re-exports); `alpm-5.0.2/src/{alpm.rs,handle.rs,db.rs,package.rs,version.rs,Cargo.toml}` (new/localdb/pkgs/Ver PartialOrd, checkver feature); duct 1.x `lib.rs` (reader:357, env:772)
- In-repo sources read this session [VERIFIED: paths]: `crates/archel-privileged/src/main.rs:1-90` (exact-match allowlist, exit 0/2); `crates/archel-core/src/privileged.rs:1-271` (envelope parse, 126/127 map, pkexec spawn); `crates/archel-core/src/updates.rs:170-200` (which+duct+timeout spawn shape); `crates/archel-core/src/error.rs:24-58` (8-variant taxonomy); `crates/archel/src/cmds/mod.rs:1-147` (clap derive, global --json, sys-group precedent); `policy/org.archel.privileged.policy:19-27` (least-privilege block); `Cargo.toml` workspace members
- Package legitimacy gate: `query package-legitimacy check` → `alpm: OK`, `clap_complete: OK` (this session)

### Secondary (MEDIUM confidence)
- STACK.md / Phase 1-02-SUMMARY / 02-01-SUMMARY (project-blessed versions and patterns; cross-checked against live probes where they intersect — pacman/alpm/clap versions all confirmed current)

### Tertiary (LOW confidence)
- yay `--noconfirm` vs `--answer*` exact interplay; `bash -n` sufficiency; DB-age footnote mechanism; pkexec streaming liveness; `Shell: ValueEnum` derivation — all tagged [ASSUMED] (A1–A8) with live-verification steps for the planner

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — both new crates gate-OK + index-verified; system tools probed live with outputs captured
- Architecture: HIGH — helper-extension and preview/streaming patterns derived from verified code + live exit-code probes; only pkexec-pipe liveness is assumed (A5)
- Pitfalls: HIGH — 4 of 7 pitfalls are direct live-probe failures (`-Syu --print` root wall, `yay -S --print` AUR miss, localized output, no-timeout-builder carry-forward); 3 are design constraints with verification steps

**Research date:** 2026-10-01
**Valid until:** 2026-10-31 (stable domain; re-probe `yay --noconfirm` semantics + alpm 5.x line if planning slips past)

<!-- GSD:research-citations
- crates/archel-privileged/src/main.rs:1-90 — exact-match argv allowlist, exit 0/2, zero-dep fixed JSON
- crates/archel-core/src/privileged.rs:1-271 — v1 envelope parse, map_pkexec_exit 126/127, pkexec spawn shape
- crates/archel-core/src/updates.rs:170-200 — which-gated duct spawn + wait_timeout pattern, UpdatesCheckerMissing
- crates/archel-core/src/error.rs:24-58 — 8-variant ArchelError taxonomy (extension point for new variants)
- crates/archel/src/cmds/mod.rs:1-147 — clap derive EN/French-split, global --json, sys-group precedent for pkg group
- policy/org.archel.privileged.policy:19-27 — least-privilege action block to mirror per verb
- Live probes: pacman 7.1.0 -Si/-S --print/-R --print/-Syu --print/-Qu; yay 13.0.1 -Ss/-Si/-G; AUR RPC v5; cargo search alpm/clap_complete
- Vendored sources: clap_complete-4.6.11 aot API; alpm-5.0.2 localdb/Ver/checkver; duct-1.x reader/env
-->
