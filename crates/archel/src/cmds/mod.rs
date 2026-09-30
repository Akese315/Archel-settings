//! CLI subcommand dispatch for the `archel` binary.
//!
//! Subcommand and flag names are English (per D-04). Displayed strings are
//! French (per D-05) and render exclusively through the `archel-core`
//! catalog — the binary owns no user-facing copy (per D-07).

use archel_core::ArchelError;
use clap::{Parser, Subcommand};

mod landing_cmd;

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
}

#[derive(Debug, Subcommand)]
enum PrivilegedAction {
    /// Run the allowlisted dry-run against the helper and check the v1 JSON
    DryRun,
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
    }
}
