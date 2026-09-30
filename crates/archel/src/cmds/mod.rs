//! CLI subcommand dispatch for the `archel` binary.
//!
//! Subcommand and flag names are English (per D-04). Displayed strings are
//! French (per D-05); the `version` framing and every error travel through
//! `archel-core` so the binary never owns user-facing copy.

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
/// Returns the French message to display on failure; `main` renders it on
/// stderr with a non-zero exit. Never panics on a failure path.
pub fn run() -> Result<(), String> {
    let cli = Cli::parse();
    match cli.command {
        Command::Version => {
            let line = format!(
                "Archel version {} — moteur archel-core {}",
                env!("CARGO_PKG_VERSION"),
                archel_core::library_version()
            );
            println!("{line}");
            Ok(())
        }
        // Plan 02 implements the CXX-Qt spike behind the `gui` feature; until
        // then `settings` reports the typed GuiNotCompiled catalog error.
        Command::Settings => Err(
            "Archel settings : interface graphique non disponible dans cette version.".to_string(),
        ),
    }
}
