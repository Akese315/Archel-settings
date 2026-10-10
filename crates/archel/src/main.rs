//! Single `archel` binary (per D-01): every CLI subcommand plus the graphical
//! interface behind the `settings` subcommand (per D-02).
//!
//! Only English identifiers and subcommand names live here (per D-04/D-08).
//! All human-readable output is French (per D-05) and renders from the
//! `archel-core` catalog — never from literals beside the call site.

mod cmds;
#[cfg(feature = "gui")]
mod gui;
// PLAT-03 thin-view audit lives here (06-09): `#[cfg(test)]`-only,
// std-only source scan, so it runs in both feature suites.
#[cfg(test)]
mod thin_view_audit;

use std::process::ExitCode;

use archel_core::ArchelError;
use clap::Parser;

/// The binary's single error voice (kept from the spike): the catalog French
/// sentence to stderr, then the failure exit code.
fn failed(err: ArchelError) -> ExitCode {
    eprintln!("{}", err.message());
    ExitCode::FAILURE
}

/// The entry is sync on purpose (per RESEARCH Pattern 2): `settings` enters
/// `gui::run_settings()` BEFORE any tokio runtime exists, so the GUI owns its
/// `Runtime` through the gui module's `OnceLock` and `app.exec()` never
/// occupies a runtime worker. Every other subcommand gets a runtime here.
fn main() -> ExitCode {
    let cli = cmds::Cli::parse();
    // The headless build has no gui module: `settings` then flows into
    // cmds::run and reports the typed GuiNotCompiled catalog error.
    #[cfg(feature = "gui")]
    if cmds::wants_settings(&cli) {
        return match gui::run_settings() {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => failed(err),
        };
    }
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(err) => {
            return failed(ArchelError::Internal(format!(
                "async runtime unavailable: {err}"
            )))
        }
    };
    match runtime.block_on(cmds::run(cli)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => failed(err),
    }
}
