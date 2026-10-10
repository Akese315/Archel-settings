# Phase 5: Hyprland Desktop - Research

**Researched:** 2026-10-07
**Domain:** Hyprland (Wayland compositor) configuration/IPC integration + hyprpaper wallpaper management from Rust via subprocess
**Confidence:** HIGH (all core mechanisms probed live on the target machine, Hyprland 0.56.2 / hyprpaper 0.8.4)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** hyprpaper.conf write + hyprpaper reload (user-approved). No `hyprctl hyprpaper` live path — one mechanism only.
- **D-02:** GTK + Hyprland scope (user-approved): `gsettings org.gnome.desktop.interface font-name` plus Hyprland `misc:font_family` in the app snippet. Qt (qt6ct/kdeglobals) explicitly out.
- **D-03:** decoration blur + opacity settings written to the app-owned snippet, live-applied via `hyprctl keyword` (no full reload).
- **D-04:** 15 s confirm window (user-approved, GNOME-like). On timeout or explicit reject, previous monitor config re-applied automatically.
- **D-05:** Single-monitor live proof only (HDMI-A-2); multi-monitor layout covered by fixtures + argv-shape tests.
- **D-06:** Keybinds managed in the app snippet only (bind lines); never edit user's own bind blocks. Add/edit/remove by exact-match on dispatcher+key combo.
- **D-07:** Mouse sensitivity + scrolling via `input:` section keys in the snippet, live-applied via `hyprctl keyword`.
- **D-08:** Main user config (lua entry, modules, hyprland.conf) is never rewritten. All persistence lands in one app-owned file sourced from the user config (`source=` or lua equivalent) with timestamped backup before every write.

### the agent's Discretion
- Exact `appearance`/`display`/`input` subcommand inventory (EN), snippet filename + source-line insertion point for the lua/modules layout, backup rotation policy, table layouts, JSON field names, hyprctl keyword names per setting, wallpaper path validation rules.

### Deferred Ideas (OUT OF SCOPE)
- DSP-05 night-light / blue-light scheduling (explicitly v2).
- GUI work of any kind — Phase 5 is API + CLI only (project ordering constraint).
- Qt font integration (qt6ct/kdeglobals) — explicitly excluded by D-02.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| APP-01 | User can set the desktop wallpaper (Hyprland backend: hyprpaper) | hyprpaper 0.8.4 verified: conf write (block syntax) + process restart is the only viable single mechanism (IPC `reload` verb does not exist; see Falsification Ledger) |
| APP-02 | User can change the system font | D-02 two-writer: `gsettings` CLI (proxy.rs pattern reused verbatim) + `hl.config({ misc = { font_family = ... } })` in snippet; both verified live |
| APP-03 | User can configure window blur | `decoration:blur:enabled` (+ size/passes/vibrancy) verified via `hyprctl getoption -j` and live toggled via `hyprctl eval 'hl.config({...})'` |
| APP-04 | User can configure window transparency/opacity | `decoration:active_opacity` / `decoration:inactive_opacity` (float 0..1) verified live via eval + getoption |
| DSP-01 | Monitor resolution change with confirm-and-rollback | `hl.monitor({output, mode, position, scale})` via `hyprctl eval` verified (no-op re-apply rc=0); modes enumerable from `hyprctl monitors -j` → `availableModes` |
| DSP-02 | Monitor scaling per monitor | `scale` field of `hl.monitor` table; user's own monitors.lua proves the exact table shape |
| DSP-03 | Monitor refresh rate | mode string encodes refresh: `1920x1080@144` (verified in live monitors.lua and `availableModes` strings) |
| DSP-04 | Monitor position/layout | `position = "WxH"` or specials (`auto`, `auto-right`, …); live layout HDMI-A-2 @ 2560x0 + DP-2 @ 0x0 verified |
| INP-01 | View current keyboard shortcuts | `hyprctl binds -j` verified; caveat: lua-era binds expose `dispatcher: "__lua"` + opaque numeric `arg` — app-managed truth comes from the snippet itself (D-06) |
| INP-02 | Add/edit/remove keyboard shortcuts | `hl.bind("<MODS>", <action>)` in app snippet; exact-match dedup computed over snippet lines (D-06), user binds read-only displayed |
| INP-03 | Mouse sensitivity | `input:sensitivity` (float, live=0.5) verified via getoption + eval live-set 0.5→0.75→0.5 |
| INP-04 | Mouse scrolling behavior | `input:touchpad:natural_scroll` (bool) + `input:touchpad:scroll_factor` (float) verified present via getoption |
</phase_requirements>

## Summary

Phase 5 adds Hyprland desktop integration to `archel-core` — wallpaper (hyprpaper), system font, blur/opacity, per-monitor configuration with 15 s confirm-and-rollback, keybind management, and mouse input — all proven through `archel` CLI subcommands before any GUI exists. The decisive finding of this research is that **Hyprland 0.55+ replaced hyprlang with Lua as the config language and the IPC surface moved accordingly**: `hyprctl keyword` is dead on this machine ("keyword can't work with non-legacy parsers. Use eval."), `hl.set_option` does not exist, and hyprpaper 0.8.4 has **no IPC reload verb at all**. The correct, live-verified mechanisms are `hyprctl eval 'hl.config({ <category> = { <leaf> = <value> } })'` for runtime option application, `hyprctl eval 'hl.monitor({...})'` / `hl.bind(...)` for monitors/binds, and **conf-write + hyprpaper process restart** for wallpapers.

The integration stays 100 % in the Phase 4 idiom: subprocess over `duct` with which-gate, exact argv, `LC_ALL=C`, `wait_timeout` + kill; pure parse/plan functions over injected fixtures; typed `ArchelError`; preview → `confirm_action` (`packages::confirm_action`, packages.rs:484) → apply. **No new crates are required for this phase** — the STACK.md-suggested `hyprland` (hyprland-rs) crate is deliberately NOT recommended: it is GPL-3.0 (license clash with `archel-core`'s MIT OR Apache-2.0), and its typed request wrappers predate/target the same keyword-based IPC that is broken here, while everything the phase needs is reachable through argv-shaped `hyprctl` calls we already know how to discipline and mock.

The persistence design follows D-08: one app-owned Lua snippet `~/.config/hypr/modules/archel.lua` (requires-able module matching the user's existing `modules/` layout), loaded by a single guarded `require("modules.archel")` line appended to `hyprland.lua`, plus an app-owned `hyprpaper.conf` slice sourced at the end of the user's `hyprpaper.conf`. Every write is preceded by a timestamped backup; the user's own config files are never rewritten beyond those one-time, idempotent source/require lines.

**Primary recommendation:** Implement a `hyprland` core module (hyprctl subprocess wrapper + pure plan/parse fns + snippet writer) mirroring `proxy.rs`/`power.rs` discipline; live-apply ALL Hyprland options via `hyprctl eval 'hl.config({...})'` (never `keyword`, never `reload`); wallpaper via conf write + hyprpaper kill/respawn.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Reading live Hyprland state (monitors, options, binds) | `archel-core` (subprocess hyprctl) | OS session (Hyprland socket) | CLI-first architecture: core owns all system integration; frontends are thin consumers |
| Live option application (blur/opacity/input/font) | `archel-core` (`hyprctl eval`) | — | Session-scoped, no privilege; compositor is the state owner |
| Durable config persistence (snippet, hyprpaper conf) | `archel-core` (filesystem writer) | `~/.config/hypr/` (user-owned) | Files are user-owned, mode 644, no root needed; D-08 invariants enforced in core |
| Privilege boundary | `archel-privileged` | — | **Not needed this phase** — every Phase 5 operation is user-session scope |
| Confirmation UX (mutating commands, 15 s monitor confirm) | `archel` CLI binary | `archel-core` (prompt strings via i18n) | Phase 4 precedent: `confirm_action` lives core-side (packages.rs), TTY gate + `[o/N]` parsing; binary wires preview→confirm→apply |
| Font store (GTK side) | gsettings CLI (`org.gnome.desktop.interface`) | `archel-core` wrapper | proxy.rs already wraps `gsettings` with identical discipline |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| duct | 1.1.2 (already in core) | hyprctl/hyprpaper/gsettings subprocess | Phase 1–4 precedent; which-gate + argv + `LC_ALL=C` + `wait_timeout` discipline already codified in `proxy.rs:108-127` |
| which | 8.0.6 (already in core) | Gate on `hyprctl`, `gsettings`, `hyprpaper` presence | Same discipline; missing tool → typed Unavailable-style error, never a panic |
| serde + serde_json | 1.0.229 / 1.0.151 (already in core) | Parse `hyprctl … -j` output | `hyprctl` has first-class `-j` JSON flag for monitors/binds/getoption/devices (verified live) |
| thiserror | 2.0 (already in core) | `ArchelError` extension | Phase invariant: typed errors the frontends match on |
| rust-i18n | 4.2 (already in core) | French display strings, EN command names | `locales/fr.json`, `rust_i18n::t!` — existing pattern (power.rs:136) |

**No new runtime dependencies.** [VERIFIED: crates/archel-core/Cargo.toml:9-38 — duct, which, serde_json, thiserror, rust-i18n, directories, tokio all already present; nothing Phase 5 needs is missing.]

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| directories | 6.0 (already in core) | Locate `~/.config/hypr/` via XDG config dir | Snippet + backup paths |
| insta | 1.48 (already dev-dep of `archel` crate) | Snapshot `--json` schemas of new subcommands | crates/archel/Cargo.toml:33 — existing pattern in `cmds/snapshots/` |
| std::os::unix (pre_exec setsid) | — | Detached respawn of hyprpaper | Only if the restart path is taken (D-01); see Pattern 4 |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| hyprctl subprocess | `hyprland` crate (hyprland-rs 0.3.13, STACK.md pick) | **Rejected for this phase**: GPL-3.0 license vs core's MIT OR Apache-2.0 (license vector); typed wrappers target the keyword/legacy IPC that is falsified broken on Lua-parser machines [ASSUMED re: crate internals — docs.rs scrape returned only navigation shell]; every verb Phase 5 needs is a thin argv call anyway, and subprocess keeps the Phase 4 mock story (injectable command). Revisit for event streams (socket2 listen) if a tray/watcher lands later. |
| `hyprctl eval 'hl.config({...})'` | `hyprctl keyword` | keyword literally errors on this machine (non-legacy parser). No choice. |
| hyprpaper restart | `hyprctl hyprpaper wallpaper …` IPC | Blocked by D-01 ("no hyprctl hyprpaper live path") AND by live probe (ipc=off → "failed to connect to hyprpaper" even though running). Restart is the one mechanism that satisfies D-01's intent. |
| Plain string ops for snippet | `regex` / `toml` crate | Snippet is app-owned & machine-generated → string `contains`/line filtering suffices; adding regex/toml for this is over-engineering (STACK lists them but nothing here needs them). |

**Installation:** none — all dependencies already in the workspace.

## Package Legitimacy Audit

No external packages are installed by this phase (verified: `crates/archel-core/Cargo.toml`, `crates/archel/Cargo.toml` already carry everything needed).

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none. (`hyprland` crate was considered and rejected on license + compatibility grounds, not legitimacy; `networkmanager` 0.5.0 remains excluded per Phase 4 [SUS] precedent — untouched here.)

## Architecture Patterns

### System Architecture Diagram

```
 archel CLI (cmds/appearance_cmds.rs, display_cmds.rs, input_cmds.rs)
   │  parse args (EN) ── preview (FR tables / --json) ── confirm_action [o/N]
   ▼
 archel-core::hyprland
   │
   ├── plan layer (PURE) ──────────────── fixtures inject every parse:
   │     parse_monitors_json(–)   → Vec<Monitor>       (hyprctl monitors -j)
   │     parse_getoption_json(–)  → OptionValue        (hyprctl getoption -j)
   │     parse_binds_json(–)      → Vec<Bind>          (hyprctl binds -j)
   │     plan_monitor_change(…)   → MonitorPlan { apply, rollback, confirm }
   │     snippet ops: render_snippet(…) / merge_bind_line(…)  (pure text)
   │
   ├── command layer (SPAWN, proxy.rs discipline)
   │     hyprctl(&["monitors","-j"])           ← which-gate, LC_ALL=C, wait_timeout
   │     hyprctl(&["eval", "hl.config({...})"])  ← live apply (ALL options)
   │     hyprctl(&["eval", "hl.monitor({...})"]) ← live monitor change (DSP)
   │     gsettings [get|set] org.gnome.desktop.interface font-name
   │
   ├── persistence layer (FILESYSTEM, D-08)
   │     backup_ts(~/.config/hypr/modules/archel.lua → .bak-<UTC ts>)  + rotation
   │     write snippet atomically (tmp + rename)
   │     ensure once: require("modules.archel") line in hyprland.lua
   │     ensure once: source= line in hyprpaper.conf → archel-hyprpaper.conf
   │
   └── hyprpaper lifecycle (APP-01)
         write app conf (preload+wallpaper) → pkill -x hyprpaper → respawn detached
                                          (setsid, stdio null, HYPRLAND_INSTANCE_SIGNATURE from env)
```

File-to-module mapping (recommended, mirrors one-file-per-domain layout in `crates/archel-core/src/`):

| File | Purpose |
|------|---------|
| `crates/archel-core/src/hyprland.rs` (or `hyprland/mod.rs` + submodules if >~1200 lines; network.rs at 2100+ lines shows a single file is also acceptable) | hyprctl wrapper, parse fns, option set/reset, snippet writer, backup/rotation |
| `crates/archel-core/src/hyprland/monitors.rs` | `Monitor` model, `availableModes` parsing, `plan_monitor_change` + rollback plan, 15 s confirm state design |
| `crates/archel-core/src/hyprland/binds.rs` | bind line render/parse for snippet, dispatcher+key exact-match, modmask<→mods codec |
| `crates/archel-core/src/hyprland/wallpaper.rs` | hyprpaper conf render/parse, path validation, restart/respawn |
| `crates/archel/src/cmds/appearance_cmds.rs`, `display_cmds.rs`, `input_cmds.rs` | clap subcommands wired into `cmds/mod.rs` enum `Command` (existing tree), preview→confirm→apply like `run_dns_plan_confirmed` (net_cmds.rs:64) |

### Pattern 1: hyprctl command discipline (copy of proxy.rs:108-127)
**When to use:** every hyprctl interaction. Keep a single internal `run_hyprctl(&["…","…"]) -> Result<String>` so tests inject a mock by passing closure over argv instead of spawning.
```rust
// Source: crates/archel-core/src/proxy.rs:108-127 (live repo pattern)
let binary = which::which("hyprctl").map_err(|_| ArchelError::/*session unavailable variant*/)?;
let handle = duct::cmd(&binary, args)
    .env("LC_ALL", "C")
    // .stderr_to_stdout() avoided for -j calls: parse stdout only, stderr log-only
    ;
let output = handle.wait_timeout(QUERY_TIMEOUT)? // map err → timeout + kill
```
Gate: no `hyprctl` on PATH, or `$HYPRLAND_INSTANCE_SIGNATURE` unset → return a typed "not running under Hyprland" error (add `ArchelError::HyprlandUnavailable { detail }` alongside `BluetoothUnavailable` error.rs:99; the 404-page of "is the compositor even running?" is a question, not a crash).

### Pattern 2: Live option apply — `eval` + `hl.config` table (replaces D-03/D-07's "keyword")
**When to use:** every D-03/D-07 live-apply (blur, opacity, sensitivity, natural scroll, font_family). Leaf-scoped: only the leaves you name change.
```rust
// Verified live 2026-10-07 on Hyprland 0.56.2 (blur off→on, sensitivity 0.5→0.75→0.5):
// hyprctl eval 'hl.config({ decoration = { blur = { enabled = false } } })'  → "ok", getoption confirms
// hyprctl eval 'hl.config({ input = { sensitivity = 0.75 } })'               → "ok", getoption confirms
fn apply_option(category: &str, body: &str) -> Result<(), ArchelError> {
    // body must be machine-generated Lua (never interpolated user text unescaped!)
    run_hyprctl(&["eval", &format!("hl.config({ {category} = {body} })")])?; // "ok" | "error: …"
    ...
}
```
Lua string-injection rule: values reaching an `eval` string (font family! wallpaper path!) must be built through a `lua_quote(&str) -> String` escape helper (backslash, `"`, `\n`, `\0` rejection) or — safer — validated to a strict allowlist charset. See Security Domain.

### Pattern 3: Monitor change + 15 s confirm-and-rollback (D-04)
```lua
-- Source: ~/.config/hypr/modules/monitors.lua (live, verified table shape)
hl.monitor({ output = "HDMI-A-2", mode = "1920x1080@144", position = "2560x0", scale = 1 })
-- verified live via `hyprctl eval 'hl.monitor({ output = "HDMI-A-2", mode = "1920x1080@144", position = "2560x0", scale = 1 })'` → "ok", rc=0, configerrors clean
```
Rust plan object (pure, fixtures-testable, D-05-compliant):
```rust
pub struct MonitorPlan {          // built by plan_monitor_change(current:&Monitor, want:&MonitorWant)
    pub apply: Vec<String>,       // eval lua statements, in layout order
    pub rollback: Vec<String>,    // re-apply `current` verbatim (reverse order)
}
// Flow: preview → confirm_action ("[o/N]") → apply all → 15 s timed re-confirm
//       → on reject/timeout → run rollback → report. All steps argv-fixture testable.
```
15 s timed confirm: `confirm_action` blocks without timeout (packages.rs:484-496). For D-04 add `confirm_action_timed(prompt, Duration) -> Result<bool>`: spawn a reader thread over stdin + `mpsc::recv_timeout(Duration::from_secs(15))`; `Ok(true)` on `[o/N]` yes, `Ok(false)` on N or timeout. Keep `ConfirmationAborted` vs new `ConfirmTimeout` distinction so the binary can always run rollback and still exit distinctively.
Monitor-validity rules to encode (wiki + live): scale must divide resolution cleanly (1.4 invalid at 1920x1080); positions are in the *scaled* coordinate space; monitors must not overlap unless intentionally stacked; `disabled = true` removes a monitor; special modes `preferred`/`highrr`/`maxwidth`, positions `auto`/`auto-right`/`auto-left` [CITED: wiki.hypr.land 0.56.0 Configuring/Monitors].

### Pattern 4: Wallpaper via conf + restart (D-01, adapted to hyprpaper 0.8.4 reality)
App-owned slice `~/.config/hypr/archel-hyprpaper.conf` (full rewrite each time — the app owns it; user's `hyprpaper.conf` gets one guarded `source = …archel-hyprpaper.conf` append, the hyprlang equivalent of the lua require-line):
```conf
# verified live: user's hyprpaper.conf uses 0.8.x block syntax today:
wallpaper {
    monitor = HDMI-A-2            # empty = all monitors
    path = /home/axel/.config/hypr/wallpapers/the-beautiful-world-bloom.png
    fit_mode = cover
}
```
Apply sequence: write slice (backup first) → `pkill -x hyprpaper` (wait_timeout+kill) → respawn detached:
```rust
// detached spawn: duct + std::process pre_exec setsid (no job-control signals from CLI exit)
let mut cmd = std::process::Command::new(hyprpaper_bin);
cmd.env("HYPRLAND_INSTANCE_SIGNATURE", hit).env("XDG_RUNTIME_DIR", xrd)
   .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
unsafe { cmd.pre_exec(|| { libc::setsid(); Ok(()) }) }  // or `setsid` argv shim to avoid libc dep: duct::cmd("setsid", …)
```
(Prefer `setsid hyprpaper` argv shim through duct to avoid a new `libc` dependency — consistent with "no new crates".)

### Pattern 5: Keybind exact-match over the snippet (D-06) + user binds read-only
App snippet emits only `hl.bind("<MODS>", hl.dsp.<dispatcher>({ … }))` lines the app generated; add/edit/remove = line-level exact match on the (normalized-mods, key, dispatcher, arg) tuple parsed from the snippet text — never touch user's `keybinds.lua`. `hyprctl binds -j` supplies the *user* bind list for INP-01 display; decoded modmask bits (live-verified): SHIFT=1, CTRL=4, ALT=8, SUPER=64 (CTRL+ALT→12, SUPER→64, SUPER+SHIFT→65).

### Anti-Patterns to Avoid
- **`hyprctl keyword`** — hard error on this machine ("keyword can't work with non-legacy parsers. Use eval."). Also beware: it prints the error yet **exits 0** — an rc-only check would silently "succeed".
- **`hyprctl reload` as an apply path** — re-executes the whole Lua entry (re-fires `hyprland.start` handlers, can respawn waybar/hyprpaper duplicates). D-03/D-07 say no full reload; honor it.
- **`hyprctl hyprpaper wallpaper/reload`** for live wallpaper — blocked by D-01 and by live ipc state.
- **Interpolating user strings into `eval` Lua** without quoting/allowlisting — injection vector (font names, wallpaper paths).
- **Parsing `hyprctl` plain-text output** — always pass `-j`; text format is not an ABI.
- **Reading the compositor socket directly in v1** — hyprwire event streams are Phase-7+ territory.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Session IPC to Hyprland | socket connect/pickle code | `hyprctl` argv subprocess (Phase 4 discipline) | hyprctl is version-skew tolerant, `-j` gives stable JSON; socket protocol (hyprwire) changed in 0.5x |
| Running option changes | keyword re-implementation or config-edit-then-reload | `hyprctl eval 'hl.config({...})'` | Verified leaf-merge live; no restart of user's environment |
| Monitor enumeration/validity | probing DRM yourself | `hyprctl monitors -j` `availableModes` strings (`'1920x1080@144.00Hz'`) | Compositor already arbitrates what's settable |
| GTK font store | dconf/gsettings binary format parsing | `gsettings` CLI (proxy.rs wrapper reused) | proxy.rs:108 already solves which-gate + timeout + LC_ALL |
| Confirmation + TTY gate | custom stdin loop per command | `packages::confirm_action` (packages.rs:484) + `NeedsTty` (error.rs:152) | Phase 4 shipped this exact semantics; only the 15 s variant is new |
| Modifiers ↔ modmask | (keep tiny — 4 constants + codec) | X mask bits 1/4/8/64 verified live | Hand-rolling is fine at this size; but decode from `-j`, never from text |
| Detached daemon respawn | full double-fork code | `setsid hyprpaper` argv shim via duct | One exec, no new crate, no zombie reaping thread |

**Key insight:** every "hard" integration in this phase (IPC, wallpaper daemon control, font store) collapses to argv-shaped subprocesses with JSON output — the single hardest lesson of Phases 1–4 applies unchanged.

## Common Pitfalls

### Pitfall 1: Trusting rc from `hyprctl` failure modes
**What goes wrong:** `keyword` prints an error but exits 0; `eval` exits 7 and prints `error: …`. Mixed signal semantics per verb.
**How to avoid:** the hyprctl wrapper checks **both**: rc==0 AND stdout != ""-error-prefix. For `eval`, treat any stdout starting with `error:` as failure (and rc!=0 is already proven reliable there — `hl.set_option` probe returned rc=7 with the error text).
**Warning signs:** "successful" sets that `getoption` doesn't reflect → always read back after apply (Phase 4's "honest read-back" lesson, 04-05 commit `3fb5336`).

### Pitfall 2: Config-time vs run-time Lua semantics
**What goes wrong:** the snippet's `hl.config({...})`/`hl.bind(...)` calls only execute when Hyprland (re)loads config; they are **not** live-apply mechanisms. Calling them from `eval` works, but the snippet alone changes nothing until reload.
**How to avoid:** every mutating core fn does BOTH: snippet rewrite (durability) + `eval` call (liveness) — same values, one code path building each. Never reload to flush the snippet.
**Warning signs:** setting looks right after `hyprctl reload` but user's `hyprland.start` side effects re-fire.

### Pitfall 3: hyprpaper IPC assumptions (D-01's literal wording is unsatisfiable)
**What goes wrong:** hyprpaper 0.8.4 has **no** `reload` command (only `--config/--verbose/--version/--help`) [VERIFIED: /usr/bin/hyprpaper --help output, this session]; its IPC is the hyprwire object protocol, socket gated by `ipc` config — user's conf has `ipc = off`, and `hyprctl hyprpaper …` fails with "failed to connect to hyprpaper (is it running?)" rc=1 while the process runs.
**How to avoid:** D-01 mechanism = conf write + kill/respawn (see Pattern 4). Document the interpretation in the plan header so the user-approved "hyprpaper reload" wording maps to restart.
**Warning signs:** a plan task that shells `hyprctl hyprpaper reload` and assumes rc 0 means applied.

### Pitfall 4: `hyprctl binds` opacity in the Lua era
**What goes wrong:** all hl.bind-defined binds (user's AND the app snippet's) appear as `dispatcher: "__lua"`, `arg: "<number>"` — the real dispatcher is invisible.
**How to avoid:** D-06 already mandates snippet-only management → the snippet file is the source of truth for add/edit/remove and for app-bind display; `hyprctl binds -j` contributes key+modmask+submap for the user's binds (display INP-01) and duplicate-collision hints only.
**Warning signs:** a dedup design that string-matches dispatcher names against `hyprctl binds` output — it will never match `__lua`.

### Pitfall 5: Snippet re-entry / duplicate requires
**What goes wrong:** naive "append source line" runs twice → duplicate `require` lines (harmless in lua, ugly in hyprpaper conf) or duplicate `wallpaper` blocks (hyprpaper loads each; last wins).
**How to avoid:** idempotent insertion (grep line equality before append, same helper for both files), atomic tmp+rename writes, timestamped backups **before** the first-ever insertion too (D-08 says every write). Rotation policy: keep newest N=5 per file (discretion → state in plan).
**Warning signs:** `hyprctl configerrors` shows duplicate-parse noise.

### Pitfall 6: Headless test environment (CI, no compositor)
**What goes wrong:** tests that spawn real hyprctl/hyprpaper/gsettings fail off-session.
**How to avoid:** the established split — pure parse/plan fns over fixtures (audio.rs:183, network.rs:1891 "never spawn nmcli" precedent), thin spawn layer excluded from unit tests except argv-shape assertions (`["eval", lua]` pairs). `cargo test --workspace --no-default-features` gate must stay green (headless gate).

### Pitfall 7: Multi-monitor reality vs D-05
**What goes wrong:** CONTEXT.md probed a single HDMI-A-2; this session found **two active monitors** (DP-2 3840x2160@60 @0x0 scale 1.5 + HDMI-A-2 1920x1080@144 @2560x0 scale 1). Fixtures-only multi-monitor coverage is still the D-05 rule, but the plan can optionally add a live two-monitor assertion.
**Warning signs:** position/overlap validation that assumes only one connected output.

## Code Examples

### Verified live option keys (this session, `hyprctl getoption <k> -j`)
```json
decoration:blur:enabled      {"bool": true,  "set": true}
decoration:active_opacity    {"float": 1.0,  "set": true}
decoration:inactive_opacity  {"float": 1.0,  "set": true}
input:sensitivity            {"float": 0.5,  "set": true}
input:accel_profile          {"str": "[[EMPTY]]", "set": false}
input:touchpad:natural_scroll{"bool": true,  "set": true}
input:touchpad:scroll_factor {"float": 1.0,  "set": false}
misc:font_family             {"str": "Adwaita Sans", "set": true}
```
App settings → key map (per-setting naming is discretion): blur on/off→`decoration:blur:enabled`; blur size/passes/vibrancy→`decoration:blur:{size,passes,vibrancy}` (present in user's appearance.lua); opacity→`decoration:{active,inactive}_opacity` (float clamp 0..1); sensitivity→`input:sensitivity` (Hyprland expects ~ -1..1, applied after `×2` scaling vs 100-scale UX — keep raw Hyprland semantics, CLI flag `--sensitivity <f32>`); natural scroll→`input:touchpad:natural_scroll`; scroll speed→`input:touchpad:scroll_factor`.

### Rendered snippet shape (app-owned `~/.config/hypr/modules/archel.lua`)
```lua
-- archel managed block — regenerated by `archel`; edits inside are lost.
hl.config({
    misc = { font_family = "Adwaita Sans" },
    decoration = { blur = { enabled = true }, active_opacity = 1.0, inactive_opacity = 1.0 },
    input = { sensitivity = 0.5, touchpad = { natural_scroll = true, scroll_factor = 1.0 } },
})
hl.monitor({ output = "HDMI-A-2", mode = "1920x1080@144", position = "2560x0", scale = 1 })
hl.bind("SUPER + CTRL + ALT + V", hl.dsp.exec_cmd("pavucontrol"))
```
One-time guarded insertions (each backup-on-first-touch, idempotent):
- `~/.config/hypr/hyprland.lua` → append `require("modules.archel")` [matches user's existing `require("modules.appearance")` style]
- `~/.config/hypr/hyprpaper.conf` → append `source = ~/.config/hypr/archel-hyprpaper.conf` (append at END so app leaves win hyprlang last-wins ordering)

### gsettings font (D-02, reuse proxy.rs runner verbatim)
```bash
gsettings get org.gnome.desktop.interface font-name   # → 'Adwaita Sans 11'  (verified live)
gsettings set org.gnome.desktop.interface font-name 'Adwaita Sans 11'
```
Parse helper strips surrounding `'…'`; setter validates `family [size]` shape (size optional, positive float), rejects control chars/newlines (argv injection is already contained by argv-not-shell, but the *value* re-enters Lua for `misc:font_family` → strict allowlist `[A-Za-z0-9 ._-]` recommended for the font path).

### 15 s timed confirm sketch (D-04)
```rust
pub fn confirm_action_timed(prompt: &str, window: Duration) -> Result<bool, ArchelError> {
    if !std::io::stdin().is_terminal() { return Err(ArchelError::NeedsTty); }
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || { let mut s = String::new();
        if std::io::stdin().read_line(&mut s).is_ok() { let _ = tx.send(s); } });
    match rx.recv_timeout(window) {
        Ok(line) => Ok(parse_confirm_answer(&line)),   // reuse packages' parser
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Ok(false), // → rollback
        Err(_) => Err(ArchelError::NeedsTty),
    }
}
```

### Existing seams to reuse (do not duplicate)
- `packages::confirm_action` + `parse_confirm_answer` — packages.rs:484 [VERIFIED this session]
- `ArchelError` family incl. `Cancelled:16, AuthFailed:27, ConfirmationAborted:146, NeedsTty:152, PolkitDenied:91, BluetoothUnavailable:99, YayMissing:141, GuiNotCompiled:165` — error.rs [VERIFIED: crates/archel-core/src/error.rs, this session; add `HyprlandUnavailable { detail }` in the same hint-bearing style]
- `cmds/mod.rs` `Command` enum + per-domain `*_cmds.rs` with `run_*_confirmed(plan, json, apply)` shape — net_cmds.rs:39-73 [VERIFIED this session]
- i18n: `rust_i18n::t!("domain.key")` + `crates/archel-core/locales/fr.json`; EN commands, FR display (cmds/mod.rs doc header).

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| hyprlang config (`hyprland.conf`, keyword syntax) | Lua (`hyprland.lua`, `hl.*` API) — legacy parser removed in 0.55 | Hyprland 0.55 (2026) | Snippet + insertion line are **Lua**, not conf; `source=` only lives in hyprpaper.conf (still hyprlang) |
| `hyprctl keyword` dynamic config | `hyprctl eval` + `hl.*` functions | 0.55 removal of non-legacy parsers | D-03/D-07 wording maps to eval; keyword rc-0-with-error trap retired |
| `hyprpaper <cmd>` / `killall -USR1 hyprpaper` | hyprwire IPC socket, `hyprctl hyprpaper` proxy, conf-only CLI | hyprpaper 0.8.x | No reload signal exists → restart lifecycle (D-01 interpretation) |
| `hyprctl monitors` text table | `-j` JSON with `availableModes` string array | — | Always `-j`; monitor "mode" derived from `description`/availableModes, not a stable `mode` int key (KeyError seen live this session) |

**Deprecated/outdated:**
- `hyprland` crate 0.4.0-beta.3 line — superseded; 0.3.13 (2026-08-29) is STACK's pin, but this phase rejects even that (license + IPC era mismatch).
- `hl.set_option` / `hl.raw_config` / `hl.source` — do not exist (falsified live; beware blog posts referencing them).

## Falsification Ledger (what this session *broke*)

| Claim probed | Method | Result |
|--------------|--------|--------|
| "`hyprctl keyword` live-applies options" (D-03/D-07 wording) | ran `hyprctl keyword decoration:blur:enabled false` | **REFUTED**: "keyword can't work with non-legacy parsers. Use eval." (and rc=0!) |
| "`hl.set_option` is the Lua setter" | `hyprctl eval 'hl.set_option(...)'` | **REFUTED**: `attempt to call a nil value (field 'set_option')` rc=7; also `hl.raw_config`, `hl.source` nil |
| "`hl.config(a, b, c)` positional form" | eval positional call | **REFUTED**: "argument must be a table" — table form only |
| "hyprpaper reload works over IPC" | `hyprctl hyprpaper reload ''` / `wallpaper …` while pid 1294 alive | **REFUTED** (this machine): "failed to connect to hyprpaper (is it running?)" rc=1; `ipc = off` in conf; `--help` lists no reload verb |
| "D-05: single monitor" | `hyprctl monitors -j` | **REFUTED (context drift)**: DP-2 + HDMI-A-2 both active |
| "`hyprctl binds` shows dispatchers" | `hyprctl binds -j` | **PARTIAL**: user lua binds opaque `__lua`; key/modmask/submap still visible |
| eval+hl.config leaf-apply | blur off→getoption→on→getoption; sensitivity 0.5→0.75→0.5 | **CONFIRMED**, restored to exact prior values; `configerrors` clean |
| `hl.monitor` table form runtime no-op re-apply | eval same-values rule | **CONFIRMED** rc=0 |
| gsettings font-name readable | `gsettings get` | **CONFIRMED** `'Adwaita Sans 11'` |
| misc:font_family exists & set | getoption | **CONFIRMED** "Adwaita Sans" |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | hyprpaper conf `source =` directive (with glob) is supported in 0.8.4 | Pattern 4 | Fallback: app fully owns `hyprpaper.conf` path passed via autostart `--config`… verify with one live probe at execution; low risk (hyprlang lineage) |
| A2 | hyprpaper reads config only at startup; no SIGHUP handler | Pattern 4 | If SIGHUP worked, restart could be gentler — restart still correct |
| A3 | `hl.config` at config-time deep-merges leaves (mirrors verified eval leaf-merge) | Pitfall 2 / snippet shape | Worst case user's sibling keys reset — mitigated because app snippet only ever names app-owned leaves; verify by setting snippet blur=false, reload in a UAT, confirm rounding/shadow survive |
| A4 | `setsid` binary exists (util-linux, Arch base) for detached respawn | Pattern 4 | Trivial: fallback `nohup`+disown-ish or std pre_exec + libc — verify at exec time |
| A5 | hyprland-rs crate internals (keyword-based wrappers) are the reason of breakage | Alternatives | Only affects the rejected-alternative rationale, not the chosen path |
| A6 | `input:sensitivity` valid range ~[-1,1] (accel flat default), `scroll_factor` [0,~10], opacity [0,1] | Code Examples | Validation bounds too tight/loose → CLI accepts something compositor rejects; clamp per getoption float read-back anyway |
| A7 | 15 s confirm uses stdin-thread timeout pattern (no existing timed-confirm in repo) | Pattern 3 | Thread+stdin interaction quirks; reuse path is proven-adjacent (`confirm_action` semantics preserved) |
| A8 | DP-2/HDMI-A-2 live two-monitor config persists during execution window (free multi-monitor proof) | Pitfall 7 | Plan must not *require* the second monitor; D-05 fixtures rule stands |

**If this table is empty:** n/a — A1–A8 need planner/user attention; A1–A4 are execution-time verifiable in minutes.

## Open Questions

1. **Does the user accept "hyprpaper reload" ≡ process restart?** (D-01 wording vs 0.8.4 reality)
   - What we know: IPC reload doesn't exist/usable; restart is invisible except a ~1-frame wallpaper flash.
   - Recommendation: proceed with restart, note it in the plan's deviation box; flag as checkpoint:human-verify during UAT (wallpaper swap visibly lands).
2. **Should `ipc = on` be written into the app hyprpaper slice?** Enabling IPC would allow a future non-restart path, but violates "one mechanism" (D-01) for now.
   - Recommendation: write the slice WITHOUT `ipc` (leave user's `off` authoritative — appended source line keeps app values last-wins only for app keys); do not enable.
3. **Reset semantics:** does `archel appearance font reset` delete the app snippet keys (reverting to user defaults) or write defaults?
   - Recommendation: delete-the-leaf model (snippet keys removed; next reload user values win again), with eval re-apply of *read-back* user default only when cheaply known; simplest v1: remove key + tell user re-login/reload needed for full revert of removed keys (eval can only set, Hyprland lacks "unset via lua"). Flag for planner.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| hyprctl | everything Hyprland | ✓ | Hyprland 0.56.2 | none needed (target machine IS the test machine) |
| hyprpaper | APP-01 | ✓ (running, pid 1294) | 0.8.4 | none — hard dep for wallpaper feature; degrade with typed error if absent |
| gsettings | APP-02 (GTK store) | ✓ | glib2 /usr/bin/gsettings | gsettings-only store (glib2 is Arch base) |
| setsid | hyprpaper respawn | expected ✓ (util-linux) | — | std `pre_exec` or `nohup` |
| HYPRLAND_INSTANCE_SIGNATURE | hyprctl calls | ✓ in session | — | typed "not in Hyprland" error |
| Second physical monitor (DP-2) | optional live proof | ✓ live now | 4K@60 | fixtures (D-05 rule anyway) |
| pkexec/root | — | n/a | — | **none required this phase** (all user-session) |

## Security Domain

`security_enforcement: true` (config.json), ASVS L1. Phase 5 introduces no network listeners, no privilege escalation, no secret handling. All state is the user's own session/desktop.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — (session-local, user-owned files) |
| V4 Access Control | no | — files at user's own 0644 in `~/.config/hypr/` |
| V5 Input Validation | **yes** | argv-only (duct, no shell) + `-j` parse; **strict validation of every string re-entering Lua `eval`/snippet**: wallpaper path (canonicalize, must exist, regular image file by extension, reject `"`/`\`/newline/NUL, allowlist charset for the lua quote path), font family+size (charset + float bounds), monitor name/mode/position/scale (pattern-validated against `monitors -j` vocabulary), bind key+mods (must match `hyprctl` keysym-ish token set + known mod words; dispatcher name from app-side enum, free-form user command args for `exec_cmd` binds go through `lua_quote`), opacity/sensitivity floats (range-checked, reject NaN/inf) |
| V6 Cryptography | no | — |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Lua injection via `--font 'x"}) evil( ('` into eval/snippet | Tampering | lua_quote escape or allowlist charset (P-2 rule above); T-5-xx |
| Path traversal / arbitrary file preload via `--set-wallpaper ~/../../etc/x` (hyprpaper preload reads the file) | Info Disc (own user scope only) | canonicalize + existence + extension check; no symlink escape beyond user-owned dirs; worst case is the user's own wallpaper — low severity by design |
| Command injection via bind action (`exec_cmd("… ; rm -rf ~")`) | Tampering | accept the action as an opaque string (Hyprland semantics — it's a shell command by design), but it runs as the *user*, at user request, listed in `archel input keybinds list`; require confirmation on add; never build argv strings from it in core |
| clobbering user config (D-08 breach) | Tampering/Integrity | write paths limited to app-owned files + two guarded insertion lines; timestamped backup before EVERY write; tmp+rename atomicity; tests assert no other path is opened for writing |
| Hung hyprctl (compositor frozen) blocking CLI | DoS | `wait_timeout` + kill (Phase 4 discipline, proxy.rs:119) |
| 15 s confirm DoS-by-abyss (locked screen during confirm window) | — | rollback-on-timeout means worst case self-heals in 15 s (D-04); document in CLI help |

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (inline `#[cfg(test)] mod tests` per core module — 14/14 core modules already do this) + insta 1.48 snapshots for `archel` cmds |
| Config file | none needed (workspace default; `archel` dev-dep insta already configured) |
| Quick run command | `cargo test -p archel-core --no-default-features hyprland` |
| Full suite command | `cargo test --workspace --no-default-features` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| APP-01 | hyprpaper conf render/parse, path validation, respawn argv shape | unit (fixtures) | `cargo test -p archel-core --no-default-features wallpaper` | ❌ Wave 0 |
| APP-02 | gsettings value parse, font charset validation, snippet font leaf render | unit | `… --no-default-features font` | ❌ Wave 0 |
| APP-03/04 | blur/opacity plan→eval-lua golden strings, float clamp/reject NaN tests | unit | `… --no-default-features appearance` | ❌ Wave 0 |
| DSP-01..04 | `monitors -j` fixture parse, plan_monitor_change apply/rollback argv, overlap/scale-divisibility validators, confirm-timeout rollback path (no TTY → NeedsTty) | unit | `… --no-default-features monitor` | ❌ Wave 0 |
| INP-01 | `binds -j` fixture parse + modmask codec roundtrip (1/4/8/64 bits verified live) | unit | `… --no-default-features binds` | ❌ Wave 0 |
| INP-02 | snippet bind add/edit/remove exact-match, idempotent render, no-touch-user-files invariant | unit | `… --no-default-features keybind` | ❌ Wave 0 |
| INP-03/04 | input leaf render/parse + range validation | unit | `… --no-default-features input` | ❌ Wave 0 |
| all | `--json` schema pinning per new subcommand | snapshot | `cargo test -p archel --no-default-features` (insta) | ❌ Wave 0 (snapshots dir exists: crates/archel/src/cmds/snapshots/) |

### Sampling Rate
- **Per task commit:** `cargo test -p archel-core --no-default-features hyprland && cargo clippy -p archel-core --all-features -- -D warnings`
- **Per wave merge:** `cargo test --workspace --no-default-features`
- **Phase gate:** full suite green + `archel` CLI live smoke (the D-xx CLI proof) before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/archel-core/src/hyprland*` test module(s) — covers all rows above (fixtures inline or in-module const strings, per audio.rs/network.rs precedent — repo has no tests/ dir; keep that convention)
- [ ] insta snapshot files for appearance/display/input `--json` — auto-created via `INSTA_UPDATE=always` first run then reviewed
- [ ] fixture JSON strings captured live: `hyprctl monitors -j`, `hyprctl binds -j`, `hyprctl getoption -j` outputs from THIS machine (already captured this session, embed trimmed copies)

## Sources

### Primary (HIGH confidence — tool-verified on the live target this session)
- Live target machine: `hyprctl --help`, `eval` probes (hl table enumeration via error-message trick), `getoption -j` × 8 keys, `monitors -j`, `binds -j`, `hyprctl hyprpaper` probes, `hyprpaper --help` (no reload verb), `gsettings get`, `~/.config/hypr/**` (hyprland.lua, modules/{appearance,input,monitors,keybinds,autostart}.lua, hyprpaper.conf), `hyprctl keyword` falsification.
- Repo source reads this session: `crates/archel-core/src/{error.rs,packages.rs,proxy.rs,lib.rs,Cargo.toml}`, `crates/archel/{Cargo.toml,src/cmds/{mod.rs,net_cmds.rs}}`.
- [CITED: wiki.hypr.land/0.56.0/Configuring/Advanced-and-Cool/Using-hyprctl/] — eval semantics ("returns ok or any error raised"), `reload full-reset`, batch/`-j` flags.
- [CITED: wiki.hypr.land 0.56.0 Configuring/Monitors, fetched via crw] — monitor syntax incl. special modes/positions, scale divisibility & overlap rules.

### Secondary (MEDIUM)
- crw-scraped docs.rs/hyprland (0.3.13, 2026-08-29, GPL-3.0) — version/license only; internals unexamined → A5.

### Tertiary (LOW)
- hyprpaper `source =` support, SIGHUP absence, sensitivity ranges (A1, A2, A6) — hyprlang lineage + training knowledge; verify at execution.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — zero new deps; all mechanisms executed live on the target
- Architecture: HIGH — mirrors shipped Phase 4 code paths read this session
- Pitfalls: HIGH — the big three (keyword, hyprpaper-ipc, binds-opacity) falsified personally; A1–A8 are LOW/MEDIUM but each has an in-plan verification path
- hyprland-rs rejection: MEDIUM — license fact solid (docs.rs GPL-3.0), internals inference not read

**Research date:** 2026-10-07
**Valid until:** 2026-11-06 (re-probe if Hyprland upgrades major or hyprpaper releases — both track Arch extra rolling)
