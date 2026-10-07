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
    /// Session proxy state (gsettings store, NET-04)
    Proxy,
}

/// One DNS operation under `archel net dns` (NET-03, D-01/D-02/D-03).
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
            NetAction::Proxy => net_cmds::run_proxy(cli.json),
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
                "status",
                "dns",
                "show",
                "set",
                "reset",
                "test",
                "proxy",
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
