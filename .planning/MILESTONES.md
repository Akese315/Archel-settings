# Milestones

## v1.0 Archel Settings Suite v1.0 (Shipped: 2026-10-11)

**Phases completed:** 6 phases, 28 plans, 73 tasks

**Key accomplishments:**
- Compiling headless Rust workspace (archel bin + archel-core lib + privileged stub) with `archel version` proving the bin-to-core link and a typed ArchelError taxonomy rendered from a French rust-i18n catalog
- Allowlisted pkexec helper with versioned v1 JSON, D-12 exit-code mapping, least-privilege policy draft, and a CXX-Qt 0.10 spike window (vertical tabs + 1 Hz live label) proven against system Qt 6.11
- One-line qrc URL fix (`qml/Spike.qml` → `src/gui/qml/Spike.qml`) confirmed against the generated qrc, proven by Qt rebuild + zero-error offscreen load + Qt-free headless build
- `archel landing` shows version, kernel and pending updates in one French table (plus `--json`/`--refresh`/`--watch`), backed by a 15-minute XDG cache with stale-with-age degradation
- `archel sys cpu|ram|disk` shows live per-core CPU + clocks, RAM and root-partition disk in French tables (plus `--json`/`--watch` NDJSON), backed by a two-pass `archel-core::system` sampler with typed partition errors
- `archel sys gpu|temp|power|all` shows live VRAM, hottest temps and real-time watts with honest French N/A (root-gated RAPL), backed by `archel-core::sensors` (NVML + RAPL + hwmon discovery-by-name), with insta-pinned `--json` schemas
- alpm localdb installed list, repo+AUR version-descending search, and D-04 pre-install card — all drivable from `archel pkg list|search|info` with French tables and `--json`
- Helper pkg-install/remove/upgrade verbs with injection-proof validator, reader-tee streaming pkexec client, and confirm-then-stream CLI with French D-03 partial-upgrade refusal — PKG-02/03/04 without ever running Archel as root
- AUR review-before-build with the D-05 trust boundary intact, unprivileged yay with native sudo handling, and shell completions closing the PLAT-02 harness — PKG-05 delivered, every Phase 2-3 capability terminal-drivable
- `pkg install` now redirects AUR-only and mixed lists to `pkg aur <name>` in French before any confirm prompt or helper spawn, closing UAT gap G-03-1
- `archel net status|dns|test` complete end-to-end over zbus-direct NM access with nmcli fallback — French tables + pinned `--json` schemas, strict IPv4 DNS validation, preview-confirm-apply mutations with reactivation read-back, verdict-free triple self-test — COMPLETE (wrapper checkpoint rejected, zbus-direct confirmed)
- Session-layer proxy (`off`/`manual`/`auto`) over the gsettings store plus multi-protocol VPN `list`/`up`/`down`/`import` on the NM path — pure argv builders, client-side validators, confirm-gated writes, secrets never in argv/logs/snapshots/`--json`
- Timed `bt scan` live list plus scan-then-pick pair/connect/disconnect behind a confirm-only bluer agent, proven live against hci0
- Name-keyed audio switching (`wpctl` parse with Video-tree gate, read-back) plus two-tier power profiles (PPD probe, cpufreq fallback, backend tag on every output), all CLI-proven with pinned --json
- Provenance-aware `vpn list` (stocké/éphémère in table and --json), tracking-loss warning with `vpn import` hint on ephemeral `vpn down`, and post-deactivate read-back through the single `vpn_down_argv` builder — closing UAT gap G-04-2 (NET-05)
- Live blur/opacity control via `hyprctl eval` with app-snippet durability — tracer slice proven live, opacity expanded on the proven shape
- System font (GTK gsettings + Hyprland leaf) and wallpaper (hyprpaper slice + kill/respawn) on the CLI with lua_quote on every string path, plus the four shared-seam hardenings later waves inherit
- Per-monitor resolution/scale/refresh/layout via `hl.monitor` eval with 15 s confirm-and-rollback, vocabulary-validated planning, and CLI tables plus JSON — live-proven with a no-op re-apply on HDMI-A-2
- `archel input keybinds` (list/add/edit/remove on snippet exact-match with live hl.bind/hl.unbind) plus `archel input mouse` (validated sensitivity/scroll leaves with eval live-apply and read-back) — Phase 05 requirement coverage complete
- Real settings shell window with rail, single-instance refusal, and geometry persistence — tasks 1-2 by worker, task 3 finished inline after worker death.
- Privileged streaming and AUR builds are cancellable through one shared SIGINT-drain-report implementation, with the stop reported as data — core gap #1 closed.
- Reusable polling layer plus the first two live tabs: System Info graphs at 1 Hz and Landing from the update-count cache.
- Four read-only thin tabs over the CLI-proven core: Network, Power, Sound, Bluetooth with the D-08 burst scan.
- The last four read tabs over CLI-proven core entry points: Appearance, Display, Input, and Packages — all ten tabs now navigate with live content.
- Single-command GUI verification plus the executable PLAT-03 guarantee: `just gui-check` (build + 20/20 qmllint + offscreen smoke rc=124 + 239/239 gui tests) and a 47-entry tab<->CLI coverage audit green in both feature suites.

---
