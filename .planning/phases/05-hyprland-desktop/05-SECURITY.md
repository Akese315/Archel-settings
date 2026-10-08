---
phase: "05"
slug: "hyprland-desktop"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-10-08"
---

# Phase 05 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| CLI args -> eval Lua string / snippet text | Untrusted user values (font family, wallpaper path, monitor name/mode/position/scale, mods/key/dispatcher/exec-arg, mouse floats) cross into compositor config syntax | User-supplied strings reaching Hyprland Lua eval + app-owned snippet |
| app writer -> ~/.config/hypr/** | App code writes snippet, hyprpaper slice, and two guarded insertion lines in the user's own config tree | Generated config text; D-08 write-scope restricted to app-owned files |
| hyprctl/gsettings stdout -> parser | External compositor and settings-daemon output crosses into display tables/JSON | Untrusted external process output parsed for display/JSON |
| app spawner -> hyprpaper process table | pkill + detached respawn touches the user's session processes | Process lifecycle control (exact argv, no shell) |
| timed stdin thread -> rollback decision | A 15 s window decides between keep and automatic rollback (D-04) | Confirmation signal gating live monitor mutation |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-05-01 | Tampering | run_hyprctl eval construction | high | mitigate | lua_quote escape + newline/NUL reject + debug_assert post-validation (hyprland/mod.rs:133-146); machine-rendered statements only; golden argv tests pin exact statements (appearance.rs:513, :672) | closed |
| T-05-02 | Tampering | snippet writer (D-08 breach) | high | mitigate | Backup-first + tmp+rename atomic writes (mod.rs:272-287); guarded require-line (mod.rs:297-323); write-scope spy tests in mod.rs:509-536, wallpaper.rs:599-630, input_cmds.rs:669-698, display_cmds.rs:403-415 | closed |
| T-05-03 | Repudiation | unlogged config mutation | low | mitigate | Timestamped backup set with N=5 rotation (mod.rs:45, :238-265); FR ok-lines name the changed key | closed |
| T-05-04 | Information Disclosure | hyprctl stderr -> logs | low | accept | Stderr captured log-only/discarded (mod.rs:104); no secrets in desktop-option scope; see Accepted Risks Log | closed |
| T-05-05 | Denial of Service | frozen compositor hangs CLI | high | mitigate | wait_timeout + kill on every run_hyprctl leg (mod.rs:93-103); HIS-unset and timeout map to typed HyprlandUnavailable pre-spawn (mod.rs:78-85, :99-101) | closed |
| T-05-06 | Spoofing | HYPRLAND_INSTANCE_SIGNATURE from env | low | accept | HIS consumed as the user's own session identity (mod.rs:78); spoofing gains nothing beyond the user's own desktop; see Accepted Risks Log | closed |
| T-05-07 | Tampering | font/wallpaper eval + snippet construction | high | mitigate | lua_quote + strict charset validation pre-spawn; IN-02 hostile-family golden (font.rs:503-522); wallpaper path canonicalize + regular-file + extension allowlist + barred-char reject (wallpaper.rs:53-94) | closed |
| T-05-08 | Denial of Service | hyprpaper kill/respawn lifecycle | high | mitigate | Exact argv pure builders (`pkill -x hyprpaper`, `setsid hyprpaper`), never through a shell (wallpaper.rs:185-198); which-gated; wait_timeout+kill on kill leg; respawn failure -> typed HyprlandUnavailable | closed |
| T-05-09 | Tampering | monitor eval statement construction | high | mitigate | Mode/output validated against live monitors -j vocabulary (monitors.rs:177-198, :270-277); NaN/inf rejected; lua_quote string legs; golden eval-pair tests (monitors.rs:896) | closed |
| T-05-10 | Denial of Service | bad mode / hung compositor / missed confirm | high | mitigate | Pre-confirm before any mutation; 15 s confirm_action_timed with automatic rollback on reject AND timeout AND apply-failure (monitors.rs:609-642; packages.rs:506-526); wait_timeout+kill on apply and rollback legs | closed |
| T-05-11 | Tampering | bind line + exec-arg construction | high | mitigate | BindDispatcher app-side enum (binds.rs:62-73); exec args lua_quote-escaped + confirm-gated; exact-match scoped to app-generated lines, user-block sentinels never match (binds.rs:376-380); D-08 spy asserts user keybinds.lua unchanged (input_cmds.rs:680-693) | closed |
| T-05-12 | Denial of Service | bad sensitivity / bind storm hangs UX | medium | mitigate | Float/enum validation pre-spawn with NaN/inf/out-of-range typed reject (input.rs:78-103); shared wait_timeout+kill runner; read-back before ok; every mutation reversible with snippet backups retained | closed |
| T-05-SC | Tampering | npm/pip/cargo installs (supply chain) | high | mitigate | Zero new crate dependencies: Cargo.lock/Cargo.toml untouched across the phase (verified via git log 552d708..HEAD); timed confirm std-only; setsid is an argv shim | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on (high) count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-05-01 | T-05-04 | hyprctl stderr is captured log-only/discarded; no secrets exist in the desktop-option scope and everything runs in the user's own session | gsd-security-auditor (plan-time disposition, confirmed 2026-10-08) | 2026-10-08 |
| AR-05-02 | T-05-06 | HYPRLAND_INSTANCE_SIGNATURE is consumed as the user's own session identity; spoofing it gains an attacker nothing beyond the user's own desktop | gsd-security-auditor (plan-time disposition, confirmed 2026-10-08) | 2026-10-08 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-08 | 13 | 13 | 0 | gsd-security-auditor (ASVS L1, grep-depth, read-only) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-08
