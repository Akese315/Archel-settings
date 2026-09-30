//! Single `archel` binary (per D-01): every CLI subcommand plus the graphical
//! interface behind the `settings` subcommand (per D-02).
//!
//! Only English identifiers and subcommand names live here (per D-04/D-08).
//! All human-readable output is French (per D-05) and renders from the
//! `archel-core` catalog — never from literals beside the call site.

mod cmds;
#[cfg(feature = "gui")]
mod gui;

use std::process::ExitCode;

fn main() -> ExitCode {
    match cmds::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{}", err.message());
            ExitCode::FAILURE
        }
    }
}
