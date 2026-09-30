//! CLI subcommand dispatch for the `archel` binary.
//!
//! Subcommand and flag names are English (per D-04). Displayed strings are
//! French (per D-05) and render exclusively through the `archel-core`
//! catalog — the binary owns no user-facing copy (per D-07).

use archel_core::ArchelError;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "archel", version, about = "Archel system settings")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print version information
    Version,
    /// Launch the graphical settings interface
    Settings,
}

/// Runs the parsed subcommand.
///
/// Every failure path returns a typed [`ArchelError`] (never a panic, never
/// a silent ok); `main` renders the French catalog message with a non-zero
/// exit.
pub fn run() -> Result<(), ArchelError> {
    let cli = Cli::parse();
    match cli.command {
        Command::Version => {
            println!("{}", archel_core::version_line());
            Ok(())
        }
        // Plan 02 implements the CXX-Qt spike behind the `gui` feature; until
        // then `settings` reports the typed GuiNotCompiled catalog error.
        Command::Settings => Err(ArchelError::GuiNotCompiled),
    }
}
