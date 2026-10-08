//! CLI subcommand dispatch for the `archel` binary.
//!
//! Subcommand and flag names are English (per D-04). Displayed strings are
//! French (per D-05) and render exclusively through the `archel-core`
//! catalog — the binary owns no user-facing copy (per D-07).

use archel_core::ArchelError;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::aot::{generate, Shell};

mod landing_cmd;
mod net_cmds;
mod pkg_cmds;
mod sys_cmds;
mod bt_cmds;
mod audio_cmds;
mod power_cmds;
mod appearance_cmds;

#[derive(Debug, Parser)]
#[command(name = "archel", version, about = "Archel system settings")]
struct Cli {
    /// Machine-readable JSON output instead of French tables (D-12)
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print version information
    Version,
    /// Launch the graphical settings interface
    Settings,
    /// Probe the privileged helper (pkexec boundary)
    Privileged {
        #[command(subcommand)]
        action: PrivilegedAction,
    },
    /// Show version, kernel and pending updates in one glance (D-11)
    Landing {
        /// Force an immediate recomputation, bypassing the cache (D-07)
        #[arg(long)]
        refresh: bool,
        /// Keep the display open and re-render until Ctrl+C (D-01)
        #[arg(long)]
        watch: bool,
    },
    /// Live CPU, memory and system-partition disk insight (D-11)
    Sys {
        #[command(subcommand)]
        metric: SysMetric,
    },
    /// Installed packages plus repo/AUR search (PKG-01, D-07/D-08)
    Pkg {
        #[command(subcommand)]
        action: PkgAction,
    },
    /// Connection state plus addressing of the primary link (NET-01/02)
    Net {
        #[command(subcommand)]
        action: NetAction,
    },
    /// Nearby and known Bluetooth devices plus pairing (BT-01/02)
    Bt {
        #[command(subcommand)]
        action: BtAction,
    },
    /// Audio endpoints by stable name plus default switching (SND-01/02)
    Audio {
        #[command(subcommand)]
        action: AudioAction,
    },
    /// Power profiles with backend provenance (PWR-01)
    Power {
        #[command(subcommand)]
        action: PowerAction,
    },
    /// Window blur live-apply with snippet durability (APP-03)
    Appearance {
        #[command(subcommand)]
        action: AppearanceAction,
    },
    /// Print shell completions for bash, zsh or fish (D-09, D-10: the
    /// full tree including every pkg action, generated never hand-written)
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Debug, Subcommand)]
enum PrivilegedAction {
    /// Run the allowlisted dry-run against the helper and check the v1 JSON
    DryRun,
}

/// One telemetry metric under `archel sys` (D-11). cpu/ram/disk
/// shipped in plan 02; gpu/temp/power/all land here (plan 03).
#[derive(Debug, Subcommand)]
enum SysMetric {
    /// Per-core usage plus current clock speed (SYS-03/04)
    Cpu {
        /// Keep the display open and re-render until Ctrl+C (D-01)
        #[arg(long)]
        watch: bool,
    },
    /// Used/total memory (SYS-01)
    Ram {
        /// Keep the display open and re-render until Ctrl+C (D-01)
        #[arg(long)]
        watch: bool,
    },
    /// System-partition used/total/free with used GB prominent (SYS-02)
    Disk {
        /// Keep the display open and re-render until Ctrl+C (D-01)
        #[arg(long)]
        watch: bool,
    },
    /// GPU model plus VRAM used/total (SYS-05)
    Gpu {
        /// Keep the display open and re-render until Ctrl+C (D-01)
        #[arg(long)]
        watch: bool,
    },
    /// Hottest temperature per category, never an average (SYS-06, D-09)
    Temp {
        /// Keep the display open and re-render until Ctrl+C (D-01)
        #[arg(long)]
        watch: bool,
    },
    /// Real-time CPU and GPU power draw (SYS-07/08)
    Power {
        /// Keep the display open and re-render until Ctrl+C (D-01)
        #[arg(long)]
        watch: bool,
    },
    /// Every metric in one table and one --json object
    All {
        /// Keep the display open and re-render until Ctrl+C (D-01)
        #[arg(long)]
        watch: bool,
    },
}

/// One package query under `archel pkg` (PKG-01). Info completes the
/// read-side trio: list proved the localdb path, search the merge, info
/// the pre-install card.
#[derive(Debug, Subcommand)]
enum PkgAction {
    /// Every installed package with its version (D-08)
    List,
    /// Repo plus AUR results, newest versions first (D-07)
    Search {
        /// Substring matched against package names and descriptions
        query: String,
    },
    /// Pre-install card: sizes, source, candidate vs installed (D-04)
    Info {
        /// Exact package name to describe
        name: String,
    },
    /// Install packages: preview, confirm, then stream (PKG-02, D-01/D-02)
    Install {
        /// One or more exact package names to install
        #[arg(required = true, num_args(1..))]
        packages: Vec<String>,
    },
    /// Remove packages: preview, confirm, then stream (PKG-03, D-01/D-02)
    Remove {
        /// One or more exact installed package names to remove
        #[arg(required = true, num_args(1..))]
        packages: Vec<String>,
    },
    /// Full-system upgrade via the helper (PKG-04, D-03: a package
    /// argument is refused — only the full upgrade exists)
    Upgrade {
        /// Never set: any value is the refused partial-upgrade path
        package: Option<String>,
    },
    /// AUR install with mandatory PKGBUILD review (PKG-05, D-05/D-06)
    Aur {
        /// Exact AUR package name to review, then build after confirm
        name: String,
    },
}

/// One network query under `archel net` (NET-01/02/03/06). Status and
/// dns-show ship reads; dns-set/reset mutate after confirmation;
/// test runs the D-09 triple.
#[derive(Debug, Subcommand)]
enum NetAction {
    /// Primary-connection state, interface, SSID, IP, gateway and DNS
    Status,
    /// Per-profile DNS: show, set, or reset to DHCP
    Dns {
        #[command(subcommand)]
        action: DnsAction,
    },
    /// Gateway ping, DNS resolution and internet reachability (raw ms)
    Test,
    /// Session proxy state and mode transitions (gsettings store, NET-04)
    Proxy {
        #[command(subcommand)]
        action: Option<ProxyAction>,
    },
    /// VPN tunnels: list, connect, disconnect, import (NET-05)
    Vpn {
        #[command(subcommand)]
        action: VpnAction,
    },
}

/// One VPN operation under `archel net vpn` (NET-05, D-07/D-08).
/// Import delegates to the nmcli plugin (`--ask` prompts for secrets);
/// `.ovpn`/`.conf` files are never parsed by Archel.
#[derive(Debug, Subcommand)]
enum VpnAction {
    /// Configured VPN tunnels and their active state (read-only)
    List,
    /// Activate one VPN connection by exact name or UUID
    Up {
        /// Exact connection name or UUID (see `net vpn list`)
        connection: String,
    },
    /// Deactivate one VPN connection by exact name or UUID
    Down {
        /// Exact connection name or UUID (see `net vpn list`)
        connection: String,
    },
    /// Import one .ovpn (OpenVPN) or .conf (WireGuard) file
    Import {
        /// Path to the .ovpn or .conf file (never /proc, /sys, /dev)
        path: String,
    },
}

/// One proxy operation under `archel net proxy` (NET-04). Bare
/// `proxy` (no subcommand) prints the one-line status; the arms below
/// mutate after confirmation. Auth passwords are prompted at use time
/// via `--password` (no-echo stdin), never passed as CLI values.
#[derive(Debug, Subcommand)]
enum ProxyAction {
    /// Direct connection: clear any session proxy
    Off,
    /// Per-scheme host/port with optional bypass list and auth
    Manual {
        /// Proxy host for the http and https schemes
        host: String,
        /// Proxy port for the http and https schemes (1-65535)
        port: u16,
        /// Comma-separated bypass list (gsettings ignore-hosts)
        #[arg(long)]
        exceptions: Option<String>,
        /// Proxy auth username (gsettings auth keys)
        #[arg(long)]
        user: Option<String>,
        /// Prompt for the proxy auth password on stdin (no-echo)
        #[arg(long)]
        password: bool,
    },
    /// Automatic mode via a PAC URL
    Auto {
        /// Proxy auto-config URL (http(s)://host/path)
        pac_url: String,
    },
}

/// One Bluetooth query under `archel bt` (BT-01/02). `devices` is
/// the read-only known list; `scan` is the timed live sweep whose
/// numbered rows are the ONLY valid `pair/connect/disconnect` picks
/// (scan-then-pick, D-06 — never a free-form MAC).
#[derive(Debug, Subcommand)]
enum BtAction {
    /// Known devices with name, MAC, signal and state (read-only)
    Devices,
    /// Timed discovery sweep into a numbered live list (read-only)
    Scan,
    /// Pair the picked scan-list device (index or in-list MAC)
    Pair {
        /// 1-based scan-list index or MAC from `archel bt scan`
        pick: String,
    },
    /// Connect the picked scan-list device (index or in-list MAC)
    Connect {
        /// 1-based scan-list index or MAC from `archel bt scan`
        pick: String,
    },
    /// Disconnect the picked scan-list device (index or in-list MAC)
    Disconnect {
        /// 1-based scan-list index or MAC from `archel bt scan`
        pick: String,
    },
}
/// One audio query under `archel audio` (SND-01/02). `list` is the
/// read-only device inventory; `set-output` / `set-input` switch the
/// default sink/source by exact stable `node.name` (never a numeric
/// ID — IDs renumber every PipeWire restart).
#[derive(Debug, Subcommand)]
enum AudioAction {
    /// Sinks and sources by stable name, with default flags (read-only)
    List,
    /// Switch the default sink by exact stable name (see `audio list`)
    SetOutput {
        /// Exact stable device name (`node.name` from `archel audio list`)
        name: String,
    },
    /// Switch the default source by exact stable name (see `audio list`)
    SetInput {
        /// Exact stable device name (`node.name` from `archel audio list`)
        name: String,
    },
}
/// One power query under `archel power` (PWR-01). `status` is the
/// read-only profile-plus-backend readout; `set` switches the
/// profile after confirmation (PPD property when PPD is present,
/// cpufreq governor writes otherwise).
#[derive(Debug, Subcommand)]
enum PowerAction {
    /// Active profile plus backend provenance (read-only)
    Status,
    /// Switch the power profile (confirm-gated)
    Set {
        /// Target profile: performance, balanced or power-saver
        #[arg(value_parser = ["performance", "balanced", "power-saver"])]
        profile: String,
    },
}
/// One appearance query under `archel appearance` (APP-02/03/04).
/// `blur` flips the live `decoration:blur:enabled` switch after
/// confirmation; bare `opacity` reads both leaves, `opacity set`
/// mutates them after confirmation (floats validated before any
/// prompt or spawn); bare `font` reads both stores (GTK gsettings
/// plus live Hyprland leaf), `font set` mutates them after
/// confirmation (spec validated before any prompt or spawn).
#[derive(Debug, Subcommand)]
enum AppearanceAction {
    /// Flip the window-blur switch live (confirm-gated)
    Blur {
        /// Target switch state: on or off
        #[arg(value_parser = ["on", "off"])]
        state: String,
    },
    /// Window transparency: read both leaves, or set them (APP-04)
    Opacity {
        #[command(subcommand)]
        action: Option<OpacityAction>,
    },
    /// System font: read both stores, or set family plus size (APP-02)
    Font {
        #[command(subcommand)]
        action: Option<FontAction>,
    },
}

/// One opacity operation under `archel appearance opacity` (APP-04).
/// Bare `opacity` (no subcommand) reads both live leaves; `set`
/// needs at least one of `--active` / `--inactive` (floats 0..1).
#[derive(Debug, Subcommand)]
enum OpacityAction {
    /// Set active/inactive opacity (confirm-gated, at least one flag)
    Set {
        /// Active-window opacity, 0.0 (transparent) to 1.0 (opaque)
        #[arg(long)]
        active: Option<String>,
        /// Inactive-window opacity, 0.0 (transparent) to 1.0 (opaque)
        #[arg(long)]
        inactive: Option<String>,
    },
}

/// One font operation under `archel appearance font` (APP-02).
/// Bare `font` (no subcommand) reads both stores; `set` takes one
/// `'<family> [size]'` spec (family allowlisted, size optional).
#[derive(Debug, Subcommand)]
enum FontAction {
    /// Set the system font: GTK store plus Hyprland leaf (confirm-gated)
    Set {
        /// Font spec: family plus optional size, e.g. 'Adwaita Sans 11'
        value: String,
    },
}
#[derive(Debug, Subcommand)]
enum DnsAction {
    /// Configured profile DNS plus effective DNS (read-only)
    Show {
        /// Exact profile name or UUID (default: active profile)
        #[arg(long)]
        profile: Option<String>,
    },
    /// Replace the profile DNS (IPv4 only) and reactivate
    Set {
        /// Exact profile name or UUID (default: active profile)
        #[arg(long)]
        profile: Option<String>,
        /// One or more IPv4 DNS servers (replaces DHCP DNS)
        #[arg(required = true, num_args(1..))]
        servers: Vec<String>,
    },
    /// Clear custom DNS back to DHCP and reactivate
    Reset {
        /// Exact profile name or UUID (default: active profile)
        #[arg(long)]
        profile: Option<String>,
    },
}

/// Runs the parsed subcommand.
///
/// Every failure path returns a typed [`ArchelError`] (never a panic, never
/// a silent ok); `main` renders the French catalog message with a non-zero
/// exit.
///
/// Async (single `#[tokio::main]` at the binary): core exposes `async fn`s
/// for the D-Bus paths while subprocess helpers stay sync and run off the
/// executor via `spawn_blocking` — never `block_on` inside core.
pub async fn run() -> Result<(), ArchelError> {
    // The catalog backend defaults to the fallback locale; pin French so
    // every rendering below is deterministic (D-05/D-07).
    archel_core::i18n::set_language("fr");
    let cli = Cli::parse();
    match cli.command {
        Command::Version => {
            println!("{}", archel_core::version_line());
            Ok(())
        }
        // The spike window lives behind the `gui` feature (Pitfall 2):
        // headless builds report the typed GuiNotCompiled catalog error.
        Command::Settings => {
            #[cfg(feature = "gui")]
            return crate::gui::run_settings();
            #[cfg(not(feature = "gui"))]
            return Err(ArchelError::GuiNotCompiled);
        }
        Command::Privileged { action } => match action {
            PrivilegedAction::DryRun => {
                let echoed = archel_core::privileged::dry_run_direct()?;
                println!("{}", archel_core::privileged::dry_run_success_line(&echoed));
                Ok(())
            }
        },
        Command::Landing { refresh, watch } => {
            landing_cmd::run_landing(refresh, watch, cli.json)
        }
        Command::Sys { metric } => match metric {
            SysMetric::Cpu { watch } => sys_cmds::run_cpu(watch, cli.json),
            SysMetric::Ram { watch } => sys_cmds::run_ram(watch, cli.json),
            SysMetric::Disk { watch } => sys_cmds::run_disk(watch, cli.json),
            SysMetric::Gpu { watch } => sys_cmds::run_gpu(watch, cli.json),
            SysMetric::Temp { watch } => sys_cmds::run_temp(watch, cli.json),
            SysMetric::Power { watch } => sys_cmds::run_power(watch, cli.json),
            SysMetric::All { watch } => sys_cmds::run_all(watch, cli.json),
        },
        Command::Pkg { action } => match action {
            PkgAction::List => pkg_cmds::run_list(cli.json),
            PkgAction::Search { query } => pkg_cmds::run_search(&query, cli.json),
            PkgAction::Info { name } => pkg_cmds::run_info(&name, cli.json),
            PkgAction::Install { packages } => pkg_cmds::run_install(&packages, cli.json),
            PkgAction::Remove { packages } => pkg_cmds::run_remove(&packages, cli.json),
            PkgAction::Upgrade { package } => pkg_cmds::run_upgrade(package, cli.json),
            PkgAction::Aur { name } => pkg_cmds::run_aur(&name, cli.json),
        },
        Command::Net { action } => match action {
            NetAction::Status => net_cmds::run_status(cli.json).await,
            NetAction::Dns { action } => match action {
                DnsAction::Show { profile } => net_cmds::run_dns_show(profile, cli.json).await,
                DnsAction::Set { profile, servers } => {
                    net_cmds::run_dns_set(profile, servers, cli.json)
                }
                DnsAction::Reset { profile } => net_cmds::run_dns_reset(profile, cli.json),
            },
            NetAction::Test => net_cmds::run_test(cli.json).await,
            NetAction::Vpn { action } => match action {
                VpnAction::List => net_cmds::run_vpn_list(cli.json),
                VpnAction::Up { connection } => net_cmds::run_vpn_up(&connection, cli.json),
                VpnAction::Down { connection } => net_cmds::run_vpn_down(&connection, cli.json),
                VpnAction::Import { path } => net_cmds::run_vpn_import(&path, cli.json),
            },            NetAction::Proxy { action } => match action {
                None => net_cmds::run_proxy(cli.json),
                Some(ProxyAction::Off) => net_cmds::run_proxy_off(cli.json),
                Some(ProxyAction::Manual {
                    host,
                    port,
                    exceptions,
                    user,
                    password,
                }) => net_cmds::run_proxy_manual(
                    &host,
                    port,
                    exceptions.as_deref(),
                    user.as_deref(),
                    password,
                    cli.json,
                ),
                Some(ProxyAction::Auto { pac_url }) => {
                    net_cmds::run_proxy_auto(&pac_url, cli.json)
                }
            },
        },
        Command::Bt { action } => match action {
            BtAction::Devices => bt_cmds::run_devices(cli.json).await,
            BtAction::Scan => bt_cmds::run_scan(cli.json).await.map(|_| ()),
            BtAction::Pair { pick } => bt_cmds::run_pair(&pick, cli.json).await,
            BtAction::Connect { pick } => bt_cmds::run_connect(&pick, cli.json).await,
            BtAction::Disconnect { pick } => {
                bt_cmds::run_disconnect(&pick, cli.json).await
            }
        },
        Command::Audio { action } => match action {
            AudioAction::List => audio_cmds::run_list(cli.json),
            AudioAction::SetOutput { name } => audio_cmds::run_set_output(&name, cli.json),
            AudioAction::SetInput { name } => audio_cmds::run_set_input(&name, cli.json),
        },
        Command::Power { action } => match action {
            PowerAction::Status => power_cmds::run_status(cli.json).await,
            PowerAction::Set { profile } => power_cmds::run_set(&profile, cli.json).await,
        },
        Command::Appearance { action } => match action {
            AppearanceAction::Blur { state } => {
                appearance_cmds::run_blur(state == "on", cli.json)
            }
            AppearanceAction::Opacity { action } => match action {
                None => appearance_cmds::run_opacity(cli.json),
                Some(OpacityAction::Set { active, inactive }) => {
                    appearance_cmds::run_opacity_set(active, inactive, cli.json)
                }
            },
            AppearanceAction::Font { action } => match action {
                None => appearance_cmds::run_font(cli.json),
                Some(FontAction::Set { value }) => {
                    appearance_cmds::run_font_set(value, cli.json)
                }
            },
        },
        Command::Completions { shell } => {
            // Generated from the single clap definition via the `aot`
            // path (never the deprecated root re-exports): stdout only,
            // install paths stay packaging-phase discretion.
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "archel", &mut std::io::stdout());
            Ok(())
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn completions_for(shell: Shell) -> String {
        let mut cmd = Cli::command();
        let mut buf = Vec::new();
        generate(shell, &mut cmd, "archel", &mut buf);
        String::from_utf8(buf).expect("completions are UTF-8 text")
    }

    #[test]
    fn completions_are_non_empty_and_name_archel_per_shell() {
        for shell in [Shell::Bash, Shell::Zsh, Shell::Fish] {
            let out = completions_for(shell);
            assert!(
                !out.trim().is_empty(),
                "{shell:?} completions must not be empty"
            );
            assert!(
                out.contains("archel"),
                "{shell:?} completions must reference archel"
            );
        }
    }

    #[test]
    fn completions_cover_the_full_command_tree() {
        // D-10: every subcommand including every new pkg action — the
        // generator walks the single clap definition, so drift fails here.
        for shell in [Shell::Bash, Shell::Zsh, Shell::Fish] {
            let out = completions_for(shell);
            for word in [
                "pkg",
                "aur",
                "completions",
                "install",
                "remove",
                "upgrade",
                "list",
                "search",
                "info",
                "sys",
                "net",
                "bt",
                "audio",
                "power",
                "appearance",
                "blur",
                "opacity",
                "font",
                "devices",
                "status",
                "dns",
                "show",
                "set",
                "reset",
                "test",
                "proxy",
                "off",
                "manual",
                "auto",
                "vpn",
                "list",
                "up",
                "down",
                "import",
                "set-output",
                "set-input",
                "landing",
                "privileged",
                "settings",
                "version",
            ] {
                assert!(
                    out.contains(word),
                    "{shell:?} completions must cover `{word}`"
                );
            }
        }
    }
}
