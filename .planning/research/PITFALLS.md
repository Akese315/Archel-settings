# Pitfalls Research: Archel (Rust Settings Suite for Arch + Hyprland)

**Domain:** Linux system settings utility — shared Rust API + CLI + Qt6 GUI on Arch Linux / Hyprland (Wayland)
**Researched:** 2026-09-30
**Confidence:** MEDIUM (domain knowledge of stable platform facts; live web search unavailable at research time — verify fast-moving items like cxx-qt release numbers during the GUI spike phase)

## Critical Pitfalls

### Pitfall 1: Choosing a Qt-for-Rust binding that stalls the project

**What goes wrong:**
The GUI phase consumes weeks fighting the binding instead of building features. Symptoms: build.rs codegen fights with MOC, Qt signals/slots don't map cleanly, a needed Qt6 module (e.g. a charts/view class for the CPU graph) has no binding coverage, upstream hasn't cut a release for the Qt version Arch ships, and every GUI task blocks on binding workarounds. The classic failure mode is discovering this *after* the API is done and the roadmap assumed "GUI is just views."

**Why it happens:**
- The Rust↔Qt ecosystem is thin and fast-moving. `rust-qt` / Ritual-era bindings are effectively unmaintained; **cxx-qt (KDAB)** is the only actively maintained bridge, and it still lags Qt releases and covers a subset of Qt.
- Qt's MOC (meta-object compiler) must still run on the C++ side — the Rust side goes through `cxx` + build-time codegen, which adds a whole new class of build fragility (toolchain skew between rustc, cxx version, and Arch's `qt6-base`).
- Arch ships very new Qt; a binding pinned to an older Qt breaks on every `qt6-base` upgrade.

**How to avoid:**
1. **Spike first, commit later.** Before the GUI phase, time-box a spike: render one window with vertical tabs + one live widget (e.g. CPU label updating on a timer) using the chosen binding (recommendation: cxx-qt). If the spike exceeds its box, the fallback is a Rust-native GUI (e.g. `iced`/`egui`) or a thin QML frontend — decide explicitly, don't drift.
2. **Isolate Qt behind a trait boundary.** The API crate must have zero Qt dependency. Define a `UiBackend`-style seam so `archel-cli` proves 100% of functionality with no GUI linked. The GUI is a replaceable shell.
3. **Pin and document the Qt toolchain**: record exact `qt6-base` version, cxx-qt version, and the build prerequisites in the repo; CI checks the combination.
4. **Link dynamically against distro Qt (LGPLv3 compliance)** — never static-link Qt into the Rust binary without a license plan (see Pitfall 2 companion note in Security/UX sections; Qt Company has moved some LTS/commercial terms around, so re-check at packaging time).

**Warning signs:**
- Spike needs `unsafe` MOC workarounds for basic signals/slots in week one.
- Upstream binding repo has no release matching Arch's Qt minor version.
- Build breaks after a routine `pacman -Syu` updates `qt6-base`.

**Phase to address:**
Phase 0 / foundation — binding spike and toolchain pinning happen *before* any GUI roadmap phase; the API-first ordering in PROJECT.md already protects this, but the spike must be an explicit gate: GUI phases don't start until the spike passes.

---

### Pitfall 2: Destroying the user's hyprland.conf with naive rewrite

**What goes wrong:**
A settings change (blur, keybind, monitor scale) rewrites `~/.config/hypr/hyprland.conf` via parse→modify→serialize. Comments, blank-line grouping, `source = …` includes, and variable definitions (`$mainMod`) are lost or reordered; worse, a crash mid-write leaves a truncated file, and the next `hyprctl reload` (or compositor restart) loads a broken config — dead keybinds, wrong monitors, or a session that won't start properly.

**Why it happens:**
- `hyprland.conf` is not a simple key=value file: it has sections (`decoration { }`, `input { }`, `bind = …` lists), variables, comments, and includes. A naive line-regex or TOML-style parser mangles all of it.
- Hyprland applies config on `hyprctl reload`; there is no transactional "validate then apply" for a hand-written file.
- Developers test on a clean VM config, not on a real user's 600-line hand-tuned file with includes.

**How to avoid:**
1. **Never rewrite the user's main file.** Preferred architecture:
   - Live/structural changes via `hyprctl keyword <section>:<key> <value>` (applies immediately, no file touch).
   - Persistence via **one app-owned snippet file** (e.g. `~/.config/hypr/archel.conf`) referenced by a single `source = ~/.config/hypr/archel.conf` line the installer adds *once* (with backup). The app only ever writes its own file.
2. **Atomic writes always**: write to temp file + `fsync` + `rename`. Keep a rotating backup (`archel.conf.bak.<timestamp>`).
3. **Reload-and-verify**: after change, `hyprctl reload`, then read back via `hyprctl getoption` to confirm; on mismatch, restore backup and report.
4. For keybinds: edit only lines the app owns (marked `# archel-managed` block) or maintain a separate `archel-binds.conf` sourced include; never touch user `bind=` lines.

**Warning signs:**
- Code contains a "parse hyprland.conf into HashMap, serialize back" function.
- No backup file is created before a write.
- Tests only cover a minimal fixture, never a config with `source=`, comments, and variables.

**Phase to address:**
Appearance phase (first writer of Hyprland config) must establish the snippet-file + atomic-write + backup pattern as shared infrastructure in the API crate; Display and Input phases reuse it — never re-implement.

---

### Pitfall 3: Running pacman/yay concurrently with another package manager

**What goes wrong:**
Two package operations overlap (GUI "update all" while user runs `yay` in a terminal, or two GUI buttons firing at once): the second fails on `/var/lib/pacman/db.lck`, or worse, interleaved `-Sy` calls produce a **partial-upgrade state** (sync DB newer than installed packages → broken deps). Running `yay` as root corrupts `~/.cache` ownership and lets AUR build scripts run with full privileges.

**Why it happens:**
- `libalpm` serializes on `db.lck`; the lock error is easy to swallow or retry-blindly.
- AUR helpers *must* run as the user (makepkg refuses root for good reason: PKGBUILDs are arbitrary shell).
- GUI developers treat "install" as a fire-and-forget subprocess without a queue.

**How to avoid:**
1. **Single package-operation queue in the API**: a process-wide async mutex/queue; every install/remove/update/refresh goes through it. Concurrent requests wait with visible status, never run in parallel.
2. **Pre-flight lock check**: inspect `db.lck` (and advise the user which process holds it) instead of crashing; surface "another package operation is running" in the GUI.
3. **Never `-Sy` without `-u`**; the update path is always full `-Syu` (+ yay `-Syu` for AUR). Never run `yay`/`makepkg` as root — privilege escalation applies to `pacman -S/-R/-Syu` only; AUR builds run as the user.
4. **AUR review flow**: show PKGBUILD diff / package info and require explicit confirmation before building AUR packages (AUR is user-supplied code). Stream build output to the GUI (progress), don't block on it (see Pitfall 5).
5. Long operations must be cancellable and must leave the system in a clean state (no orphaned locks — always release on cancel/error).

**Warning signs:**
- More than one call site spawns `pacman`/`yay` directly.
- Code runs `pacman -Sy` anywhere.
- `yay` invoked under `sudo`/`pkexec`.
- GUI freezes during `-Syu` (compounds with Pitfall 5).

**Phase to address:**
Packages phase — but the **privilege model (pkexec for pacman vs user-context yay) and the op-queue** must be decided in the foundation phase (PROJECT.md flags this as an early decision), because Network (VPN/DNS) and other privileged ops will reuse the same escalation helper.

---

### Pitfall 4: GPU stats pitfalls — NVIDIA vs AMD vs Intel telemetry

**What goes wrong:**
GPU tab works on the developer's NVIDIA card and shows garbage/zeros/crashes on AMD/Intel: VRAM "used" exceeds "total" (shared-memory confusion), power reads as microwatts displayed as watts (off by 10⁶), temperature node missing on some cards, app requires proprietary NVML on a nouveau system and reports "no GPU."

**Why it happens:**
- Three different telemetry stacks: **NVIDIA → NVML** (`libnvidia-ml.so`, proprietary driver only; nouveau has no NVML — fall back to sysfs/hwmon or report unavailable); **AMD → amdgpu sysfs** (`/sys/class/drm/card*/device/gpu_busy_percent`, `mem_info_vram_used`/`mem_info_vram_total`, `hwmon/hwmon*/power1_average`, `temp1_input`); **Intel → i915/xe sysfs + hwmon** (different node names, integrated GPU shares system RAM — "VRAM" is meaningless).
- Units differ: `power1_average` is **microwatts**; `temp*_input` is **millidegrees Celsius**; `gpu_busy_percent` is sampled, not instantaneous.
- `power1_cap` (power cap) exists only on some cards; hwmon index (`hwmon0` vs `hwmon3`) is not stable across boots — must discover by name, not path.
- Multi-GPU (iGPU + dGPU, e.g. laptop) needs per-device enumeration, not "the GPU."

**How to avoid:**
1. **Per-vendor backend trait** in the metrics layer: `enum GpuVendor { Nvidia, Amd, Intel, Unknown }` with a `GpuBackend` trait (`utilization()`, `vram_used/total()`, `temp()`, `power()`); each returns `Option<f64>` — missing sensors are `None` (GUI shows "n/a"), never fake zeros.
2. **Unit conversion at the boundary** with tests: µW→W, m°C→°C; assert with known fixture values.
3. **Discover, don't hardcode**: enumerate `/sys/class/drm/card*`, match driver via `/sys/.../device/driver` or `uevent`; find hwmon by `name` file (`amdgpu`, `nvidia`, `i915`), not index.
4. **Graceful degradation matrix**: NVML absent → sysfs attempt → "unavailable" message naming the cause (e.g. "proprietary NVIDIA driver not loaded"). iGPU VRAM row hidden or labeled "shared memory."
5. Never shell out to `nvidia-smi` per tick (see Pitfall 8) — use NVML bindings or a single long-lived sampler.

**Warning signs:**
- One code path reads "the" GPU from a hardcoded `/sys/class/drm/card0`.
- Power values in the millions (µW displayed raw) or temps of 45000 (°C vs m°C).
- Works on dev machine, `None`/panic on any other vendor (no test fixtures per vendor).

**Phase to address:**
System Info / metrics-sampling phase — define the backend trait and fixtures there; the GUI graphs consume the trait, so vendor bugs can't leak into GUI code.

---

### Pitfall 5: Blocking the Qt event loop with sync D-Bus/subprocess calls

**What goes wrong:**
Clicking "refresh networks" or "check updates" freezes the entire GUI for seconds: the UI thread is sitting in a blocking `zbus` call, `nmcli`/`pacman` subprocess wait, or a `sleep`-based poll loop. On Wayland, an unresponsive Qt app gets dimmed/killed by the compositor watchdog perception ("app hung").

**Why it happens:**
- The natural Rust code — `Command::new("nmcli").output()?`, `blocking::Connection::system()?` — is synchronous; calling it from a Qt slot runs it on the GUI thread.
- cxx-qt slots look like normal methods, so it's easy to put I/O in them.
- Developers test with fast local sysfs reads and never notice until a D-Bus call stalls (BlueZ scan, pacman refresh).

**How to avoid:**
1. **Hard rule: no I/O on the GUI thread.** All system calls (D-Bus, subprocess, sysfs sampling loops) run on Tokio worker tasks / `std::thread` workers; results cross to the GUI via channels + queued signals (`QMetaObject::invokeMethod` / cxx-qt queued connections).
2. Use **async `zbus`** (not `blocking`) inside the API crate; the API exposes async methods or callback/channel APIs, never blocking ones.
3. Subprocess calls get timeouts (`tokio::process` + `timeout()`); every slot that triggers I/O shows a spinner/disabled state and is re-entrant-safe (double-click doesn't spawn two scans).
4. The sampler (Pitfall 8) pushes at fixed cadence; the GUI never pulls synchronously.

**Warning signs:**
- `Command::output`, `std::process`, or `blocking::` zbus calls inside code reachable from a Qt slot.
- No timeout on any subprocess/D-Bus call.
- "It works but the window freezes for a bit" in manual testing.

**Phase to address:**
Foundation phase — the async boundary (Tokio runtime + GUI-thread delivery mechanism) is proven in the binding spike (Pitfall 1: the live-updating label test) and enforced as a lint/review checklist for every later phase.

---

### Pitfall 6: Running the GUI as root instead of a polkit-privileged helper

**What goes wrong:**
App is launched with `sudo` to make pacman work: Wayland socket/`XDG_RUNTIME_DIR` breaks or connects insecurely, `~/.config/hypr/archel.conf` gets written as root (user can't edit their own config afterwards), Qt theme/platform plugins misbehave, and the entire GUI process (including AUR build-script output rendering) runs with full privileges — a needless security crater.

**Why it happens:**
- "Just sudo the app" is the fastest way to make privileged ops work in development.
- PROJECT.md explicitly scopes *out* a sudo/polkit administration UI, which some readers misinterpret as "no privilege design needed" — the opposite is true: privilege *escalation mechanics* are still required.

**How to avoid:**
1. **GUI and API always run as the user.** Only narrow privileged operations escalate: `pacman -S/-R/-Syu`, system-wide NetworkManager changes, power-profile writes — via `pkexec` (prompt per action) or, preferably, a minimal **D-Bus system-service helper with a polkit policy** (fine-grained actions like `com.archel.settings.packages.install`, least privilege, auditable).
2. AUR builds (`yay`/`makepkg`) **never** escalate — they run as the user by design.
3. Files under the user's `$HOME` are never written by the elevated helper (pass data back over D-Bus/stdout; the user-side process writes).
4. Decide the mechanism in foundation (pkexec helper = simpler, polkit service = better UX); the API exposes `requires_privilege: bool` per operation so the GUI knows when to expect an auth dialog.

**Warning signs:**
- README says "run with sudo."
- Root-owned files appear in `~/.config/hypr/` or `~/.cache/`.
- AUR build code path shares the escalation wrapper with pacman.

**Phase to address:**
Foundation phase (privilege architecture decision + helper skeleton); Packages and Network phases implement their specific privileged actions against it.

---

### Pitfall 7: Wayland-vs-X11 tool assumptions (xrandr/xinput don't work on Hyprland)

**What goes wrong:**
Display tab shells out to `xrandr` (fails silently or lists a fake XWayland output), input tab uses `xinput` (no devices), font change writes `~/.Xresources` (nothing reads it), scaling sets Xft.dpi only (Qt/Wayland apps ignore it or double-scale).

**Why it happens:**
- 90% of "Linux settings" StackOverflow answers and crates assume X11. Under Hyprland (Wayland), the compositor owns outputs and input; X11 tools can only see the XWayland compatibility layer.
- Scaling on Wayland is per-output, compositor-side, with the fractional-scale protocol; DPI hacks from the X11 era conflict with it.

**How to avoid:**
1. **Display**: drive monitors via Hyprland's `monitor=<name>,<res>@<hz>,<pos>,<scale>` lines (persisted in the app snippet file, Pitfall 2) + `hyprctl` for live apply; enumerate outputs via `hyprctl monitors -j`, not xrandr. Validate mode/hz/scale combos against the compositor's reported modes.
2. **Input**: mouse sensitivity/scroll live in Hyprland's `input { sensitivity, scroll_method, … }` + `hyprctl keyword`; device names from `hyprctl devices` / libinput, never xinput.
3. **Fonts**: set via **fontconfig** (`~/.config/fontconfig/fonts.conf`) for app fonts + Qt platform theme settings (`qt6ct`/env `QT_FONT`) and GTK settings where applicable; understand Hyprland itself renders almost no text (only a few UI elements) — "system font" means toolkit propagation, and must be verified per toolkit (Qt + GTK + terminal).
4. Add a startup **environment assertion**: if `$XDG_SESSION_TYPE`/`$WAYLAND_DISPLAY` indicate X11 or the Hyprland socket is missing, show a clear "unsupported session" state instead of silently failing tools.

**Warning signs:**
- `xrandr`, `xinput`, `xset`, or `Xresources` appear anywhere in code or dependency lists.
- Scaling implemented as DPI-only with no per-output compositor scale.
- Font change verified only in one toolkit.

**Phase to address:**
Display, Input, and Appearance phases respectively — with the session-assertion and the "no X11 tools" rule recorded as a global architecture constraint in foundation so no phase re-discovers it.

---

### Pitfall 8: Live-stat polling hammering D-Bus/sockets/subprocesses

**What goes wrong:**
The System Info tab spawns `nvidia-smi`, queries UPower/NetworkManager over D-Bus, and re-reads everything at 10 Hz per widget: CPU spikes from the monitoring itself, D-Bus flood warnings in the journal, PipeWire round-trips per frame, battery/temps sampled pointlessly fast. Laptop battery drains *because* of the settings app.

**Why it happens:**
- Each widget independently polls ("CPU graph timer + temp timer + net timer"), each poll is a fresh subprocess or D-Bus proxy creation, and nobody measures the observer overhead.
- D-Bus signals (`PropertiesChanged`) and sysfs (cheap reads) exist but polling is what gets written first.

**How to avoid:**
1. **One sampler, many subscribers**: a single metrics task in the API samples at fixed cadence (recommend 1 Hz default; CPU graph ring-buffer at 1 Hz is plenty; power/temps at 0.5–1 Hz) and broadcasts via `tokio::watch`/`broadcast`; GUI widgets subscribe, never spawn their own loops.
2. **Prefer events over polls**: NetworkManager/BlueZ/UPower state via D-Bus signal subscriptions; poll only true time-series (CPU, net counters, temps).
3. **No subprocess per tick**: parse `/proc/stat`, `/proc/meminfo`, sysfs/hwmon directly (cheap file reads); keep NVML handle open across ticks; reuse one `zbus` connection for the process lifetime.
4. **Back off when hidden**: pause/stop sampling when the System Info tab isn't visible (or drop to 0.2 Hz); resume on show.
5. Measure observer cost in the CLI (`archel-cli stats --benchmark`): sampler CPU% must be a line-item acceptance criterion.

**Warning signs:**
- `Command::new` inside any function called by a timer.
- Multiple timers across widgets with overlapping periods.
- New D-Bus `Connection` per call instead of one shared connection.
- Journal full of D-Bus traffic when the app idles on the stats tab.

**Phase to address:**
System Info phase owns the sampler design; but the "one shared zbus connection, no subprocess-per-tick" rules are foundation-level constraints since Network/Bluetooth/Audio phases also touch D-Bus.

---

### Pitfall 9: NetworkManager DNS/proxy profile-model misunderstandings

**What goes wrong:**
- "Set DNS to 1.1.1.1" writes a global `/etc/resolv.conf` (fights systemd-resolved/NetworkManager, gets overwritten on reconnect) or sets DNS on the wrong profile (Wi-Fi profile while on Ethernet).
- VPN connects but DNS leaks: `ipv4.dns-priority` left at default so the physical connection's DNS wins over the VPN's pushed DNS.
- Proxy "setting" is stored in a NetworkManager profile where nothing reads it — apps ignore it because **proxy is not a NetworkManager concept**.

**Why it happens:**
- NM models DNS **per-connection-profile** (`ipv4.dns`, `ipv6.dns`, `ipv4.ignore-auto-dns`, `ipv4.dns-priority`, `ipv4.dns-search`), plus systemd-resolved globals; developers assume one global switch.
- `dns-priority` semantics are subtle: lower value = higher priority; VPN profiles typically need negative priority (e.g. `-100`) to win; default `0` ties get resolved by route metrics — leak-prone.
- Proxy on Linux lives in **desktop gsettings (GNOME), `plasma` kioslaverc, or env vars (`http_proxy`)**, plus per-app configs — there is no NM field for it. A settings app must model proxy as its own layer.

**How to avoid:**
1. **DNS edits are per-active-profile operations**: resolve the *active* connection (`nmcli -g NAME,UUID,TYPE connection show --active`), modify `ipv4.dns` + set `ipv4.ignore-auto-dns yes` when custom DNS is desired; expose `dns-priority` with a sane default and document the VPN case (`-100` preset labeled "prefer VPN DNS").
2. Always show *effective* DNS (from `nmcli device show` / resolved `resolvectl status`), not just configured values — effective-vs-configured mismatch is the leak detector. Add a "DNS leak check" to network tests (resolve a test name and show which server answered).
3. **Model proxy separately from NM**: a Proxy section that writes the layers that actually work — session env (`/etc/environment` or user session), GNOME gsettings `org.gnome.system.proxy` (if present), and per-profile PAC/manual fields for browsers — with explicit UI copy: "Proxy is applied to apps via environment and desktop settings, not to the network connection."
4. After any DNS change: `nmcli connection up <uuid>` (reactivate to apply) + verify effective DNS; never hand-edit `/etc/resolv.conf` when it's a symlink to systemd-resolved/NetworkManager.

**Warning signs:**
- Code writes `/etc/resolv.conf` directly.
- DNS UI has no notion of "which connection" — one global field.
- `dns-priority` never mentioned in VPN code.
- Proxy stored as an NM connection property.

**Phase to address:**
Network phase — DNS model, VPN priority preset, effective-DNS verification, and the separate proxy layer are all acceptance criteria of that phase's plan.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Shell out to `nmcli`/`hyprctl`/`bluetoothctl` text output instead of D-Bus/socket APIs | Working feature in an afternoon | Brittle parsing breaks on locale/version output changes; no change-events | MVP only, behind a trait so a D-Bus implementation can replace it; never for hot paths (sampler) |
| Single hardcoded `/sys/class/drm/card0` + `hwmon0` paths | GPU tab works on dev machine | Wrong/missing data on every other machine | Never — enumerate from day one, it's 20 lines |
| One global 100ms QTimer refreshing all stats widgets | Simple code | Observer overhead + frozen UI risk | Never — single sampler + subscriptions (Pitfall 8) |
| `sudo archel` during development | Privileged ops "just work" | Root-owned configs, Wayland breakage, shipped habit (Pitfall 6) | Local dev only with a big warning; CI must run user-mode |
| Rewrite whole `hyprland.conf` for one keybind change | Simple implementation | Comment/format loss, bricked sessions (Pitfall 2) | Never — snippet file + `hyprctl keyword` from the start |
| `pacman -Sy` quick refresh button | Feels fast | Partial-upgrade breakage (Pitfall 3) | Never — always full `-Syu` path |
| Skip AUR PKGBUILD review ("just --noconfirm everything") | One-click AUR UX | Arbitrary-code execution as the user without consent | Never — review/confirm is a security requirement |
| Global DNS field with no profile context | Simple UI | Wrong-profile edits, VPN leaks (Pitfall 9) | Never — per-active-profile model from the start |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|---------------|------------------|
| Hyprland config | Regex-editing `hyprland.conf` in place | `hyprctl keyword` live + app-owned `source=` snippet, atomic writes, backups |
| Hyprland monitors | Parsing `xrandr` | `hyprctl monitors -j`; `monitor=` lines for persistence |
| Hyprland input devices | `xinput` names/props | `hyprctl devices`; `input { }` section keywords |
| Wallpaper | Assuming `feh`/`nitrogen` (X11) | hyprpaper/wallpaper daemon via its IPC, not X11 setters |
| NetworkManager | Editing `/etc/resolv.conf`; global DNS field | Per-profile `ipv4.dns` + `ignore-auto-dns` + `dns-priority`; verify via `resolvectl`/effective DNS |
| Proxy | Storing proxy in NM profile | Separate layer: session env + desktop gsettings + per-app; say so in UI |
| BlueZ | Polling adapters/devices on a timer | D-Bus `ObjectManager` + `PropertiesChanged` subscriptions; async zbus |
| PipeWire/WirePlumber | Hardcoding node IDs; parsing `pactl` per tick | Default-node tracking via subscriptions; IDs resolved dynamically each change |
| power-profiles-daemon vs TLP | Assuming one exists / fighting the other | Detect PPD presence; PPD and TLP conflict — pick/honor the active one, don't drive both |
| pacman/libalpm | Parallel invocations; `-Sy` alone | Single op queue; lock check; full `-Syu`; yay as user |
| NVML | Linking/assuming NVIDIA proprietary stack | Optional backend; degrade to sysfs/unavailable with reason |
| Qt6 packaging (Arch) | Static link or bundled Qt | Dynamic link to system `qt6-base`; pin versions; re-verify on Qt minor bumps |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Subprocess-per-tick sampling (`nvidia-smi`, `pactl`, `nmcli` in timers) | Settings app itself tops CPU; fan spins on stats tab | Direct sysfs/`/proc`/NVML-handle reads; one sampler | Immediately on laptops; ~always at 1 Hz+ |
| Per-widget timers each creating D-Bus connections | Journal D-Bus flood; latency growth | One shared `zbus::Connection`; single sampler broadcasting | As soon as 3+ widgets poll |
| Synchronous pacman refresh on GUI thread | Multi-second UI freeze on "check updates" | Worker task + progress stream + cancellable op queue | First slow mirror / large `-Syu` |
| Full `yay -Syu` query on every landing-page view | Landing tab slow; AUR RPC hammered | Cache update count with TTL (e.g. 15–60 min) + manual refresh button | Every launch with slow network |
| Unthrottled CPU-graph repaint (60 fps full redraw) | High idle CPU in GUI | 1 Hz data + decimated repaint; only when tab visible | As soon as graph ships without gating |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Running whole GUI as root | Full-privilege UI bugs, root-owned user files, Wayland insecurity | User-mode app + pkexec/polkit helper scoped to privileged ops only |
| Auto-building AUR packages without PKGBUILD review | Arbitrary code execution as the user | Show package info/PKGBUILD diff, require explicit confirm per AUR install |
| `yay`/`makepkg` under sudo | Build scripts run as root → full system compromise via malicious PKGBUILD | AUR path never escalates; separate code path from pacman ops |
| Over-broad polkit policy (`allow_any: yes` for package ops) | Any local process can install/remove packages silently | Narrow actions per operation; `auth_admin`/`auth_self` as appropriate; no blanket allows |
| Storing proxy/VPN credentials in plaintext config | Credential theft from readable files | Secret Service/keyring for secrets; `802-1x`/VPN secrets flagged `not-saved` or keyring-backed via NM |
| Blindly `source=`-including files or executing helper output | Config injection if helper output is untrusted | App writes only its own snippet; validate values (resolution, scale, keybind strings) before writing |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Showing configured DNS instead of effective DNS | User thinks VPN DNS applies while leaking | Always show effective DNS + leak-check test; flag mismatch |
| Reporting GPU "0 MB / 0 MB" when sensor missing | User distrusts the whole app | Show "n/a — reason" (e.g. nouveau driver, iGPU shared memory) |
| Freezing UI during scans/installs with no feedback | "App is broken" perception | Spinner + progress stream + cancel on every long op |
| Silently failing X11 tools under Wayland | Buttons that do nothing | Session assertion at startup; disable with explanation if unsupported |
| Destroying hand-tuned hyprland.conf formatting | Power-user rage; lost config art | Snippet-file approach; show exactly what the app will write (diff preview) before applying |
| Language mix (French/English) inconsistent per tab | Unprofessional feel for FR/EN user | Decide UI locale strategy in foundation (i18n via Qt Linguist from day one or single-language v1) |

## "Looks Done But Isn't" Checklist

- [ ] **Hyprland edit:** Often missing backup + atomic write + reload-verify — verify by killing the process mid-write in a test and confirming the config survives.
- [ ] **DNS change:** Often missing reactivation + effective-DNS check — verify with `resolvectl status` after apply, not just the profile field.
- [ ] **VPN:** Often missing `dns-priority` handling — verify no leak with the in-app leak check on connect.
- [ ] **GPU tab:** Often missing non-dev-vendor coverage — verify with sysfs fixtures for all three vendors + missing-sensor case.
- [ ] **Package update:** Often missing lock-contention path — verify behavior when `db.lck` is held (clear message, no crash, queued retry).
- [ ] **AUR install:** Often missing review step — verify PKGBUILD shown + explicit confirm before build.
- [ ] **Stats tab:** Often missing hidden-tab backoff — verify sampler idles when tab not visible (measure observer CPU%).
- [ ] **Privileged op:** Often missing non-root end-to-end test — verify whole flow as unprivileged user with auth prompt, and confirm no root-owned files in `$HOME`.
- [ ] **Display change:** Often missing invalid-mode rejection — verify against `hyprctl monitors` reported modes before applying.
- [ ] **GUI responsiveness:** Often missing slow-D-Bus test — verify UI stays interactive with an artificially delayed D-Bus response (all I/O off GUI thread).
- [ ] **Qt toolchain:** Often missing version-pin record — verify clean build after `qt6-base` upgrade documented in repo.

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Trashed hyprland.conf | HIGH (user session broken) | Restore from rotating backup the app kept; if none, fall back to Hyprland defaults + re-apply snippet; post-mortem: add snippet-file architecture |
| Binding stall mid-GUI-phase | HIGH (weeks lost) | Pivot GUI shell (QML/native) while API+CLI keep all value; the API-first order makes this survivable — this is why CLI proves everything first |
| Partial upgrade from `-Sy` | MEDIUM-HIGH (broken deps) | Immediate full `-Syu`; document; remove the `-Sy`-only path |
| Root-owned files in `$HOME` | LOW-MEDIUM | `chown` back; fix helper to never write user files elevated; add CI check for root-owned files after e2e run |
| Wrong-vendor GPU code shipped | MEDIUM | Add backend trait + fixtures; ship degraded-but-honest `n/a` states per vendor |
| D-Bus/subprocess-per-tick shipped | LOW-MEDIUM | Centralize into sampler; measure before/after observer CPU |
| DNS leak via VPN default priority | MEDIUM (user exposed until fixed) | Ship `dns-priority` preset + effective-DNS display + leak check; notify |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Qt binding stall (1) | Foundation / pre-GUI spike gate | Spike renders live-updating window; toolchain pinned; GUI phases gated on spike pass |
| hyprland.conf destruction (2) | Appearance (establishes snippet+atomic+backup infra) | Mid-write-kill test; user fixture with comments/`source=` round-trips byte-identical outside app block |
| pacman/yay concurrency (3) | Foundation (privilege model + op queue design); Packages (implementation) | Lock-held test shows clean message; `yay` never under sudo; no `-Sy`-only path exists |
| GPU telemetry (4) | System Info (backend trait + fixtures) | All-vendor fixture tests; µW/m°C unit tests; `n/a`-with-reason on missing sensors |
| Qt event-loop blocking (5) | Foundation (async boundary + delivery mechanism) | Delayed-D-Bus test: UI interactive; no `Command::output`/`blocking::` reachable from slots |
| GUI-as-root (6) | Foundation (pkexec vs polkit-service decision) | Full e2e as unprivileged user; no root-owned files in `$HOME` after run |
| X11-tool assumptions (7) | Foundation (global "no X11 tools" constraint + session assertion); Display/Input/Appearance | Grep: no xrandr/xinput/Xresources; mode validation against `hyprctl monitors` |
| Stat-polling hammer (8) | System Info (sampler) + foundation (shared-connection rule) | Observer CPU% benchmark in CLI; hidden-tab backoff verified |
| NM DNS/proxy model (9) | Network | Effective-DNS verification + leak check pass; proxy stored outside NM with honest UI copy |

## Sources

- Platform knowledge: Hyprland docs (`hyprctl keyword/getoption/monitors`, `monitor=`/`input { }`/`source=` semantics), NetworkManager `nm-settings`/`nmcli` per-connection DNS model (`ipv4.dns`, `ignore-auto-dns`, `dns-priority`), BlueZ D-Bus `ObjectManager` patterns, PipeWire/WirePlumber default-node model, ALSA/PipeWire on Arch.
- Arch packaging knowledge: `pacman` `db.lck` serialization, `makepkg`/`yay` non-root requirement, partial-upgrade (`-Sy` without `-u`) breakage, PPD-vs-TLP conflict, dynamic linking to system `qt6-base` for LGPLv3 compliance.
- Kernel/sysfs knowledge: amdgpu (`gpu_busy_percent`, `mem_info_vram_*`, `power1_average` in µW), hwmon temp (`temp*_input` in m°C) discovery-by-name, NVML proprietary-only limitation, i915/xe integrated-memory semantics.
- Cross-cutting: Qt event-loop threading rules, polkit/pkexec least-privilege patterns, Wayland compositor-owns-outputs model vs X11 tooling.
- Research cache: per-question digests stored via `research-store` (keys from `research-plan`); live web search was unavailable (no Brave key, empty CRW results), so fast-moving claims (cxx-qt release numbers, Qt commercial-term changes) are flagged for re-verification in the GUI spike phase.

---
*Pitfalls research for: Archel — Rust settings API + CLI + Qt6 GUI on Arch + Hyprland*
*Researched: 2026-09-30*
