//! CLI subcommand dispatch for the `archel` binary.
//!
//! Subcommand and flag names are English (per D-04). Displayed strings are
//! French (per D-05) and render exclusively through the `archel-core`
//! catalog — the binary owns no user-facing copy (per D-07).

use archel_core::ArchelError;
use clap::{Parser, Subcommand};

mod landing_cmd;
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

/// One package query under `archel pkg` (PKG-01). Info lands in the
/// next task of this plan; list proved the read path first.
#[derive(Debug, Subcommand)]
enum PkgAction {
    /// Every installed package with its version (D-08)
    List,
    /// Repo plus AUR results, newest versions first (D-07)
    Search {
        /// Substring matched against package names and descriptions
        query: String,
    },
}

/// Runs the parsed subcommand.
///
/// Every failure path returns a typed [`ArchelError`] (never a panic, never
/// a silent ok); `main` renders the French catalog message with a non-zero
/// exit.
pub fn run() -> Result<(), ArchelError> {
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
        },
    }
}
