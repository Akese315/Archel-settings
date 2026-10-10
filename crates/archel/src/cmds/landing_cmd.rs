//! `archel landing` renderer: one French table (default) or one JSON
//! object (`--json`), with `--refresh` bypass and `--watch` re-render.
//!
//! Bin-calls-core shape: the snapshot and every French string come from
//! `archel-core` (`landing::snapshot`, `updates_cell`, `checker_notes`);
//! this module only lays out columns (`std::fmt` widths, no table crate),
//! frames NDJSON ticks, and owns the 500 ms watch loop.

use std::time::Duration;

use archel_core::landing::LandingSnapshot;
use archel_core::ArchelError;

/// Watch re-render cadence (D-02).
const WATCH_INTERVAL: Duration = Duration::from_millis(500);

/// Runs `archel landing`. `refresh` forces a recompute on the first tick
/// only (re-forcing every 500 ms would spawn the checkers twice a
/// second); later ticks serve the cache. `json` switches the table for
/// one JSON object per print (NDJSON stream under `--watch`, D-03).
// gui: landing.tick
pub fn run_landing(refresh: bool, watch: bool, json: bool) -> Result<(), ArchelError> {
    let mut force = refresh;
    loop {
        let snap = archel_core::landing::snapshot(force)?;
        force = false;
        if json {
            println!(
                "{}",
                serde_json::to_string(&snap).map_err(|err| {
                    ArchelError::Internal(format!("cannot encode landing JSON: {err}"))
                })?
            );
        } else {
            if watch {
                print!("\x1Bc");
            }
            print!("{}", render_table(&snap));
            for note in archel_core::landing::checker_notes(&snap) {
                println!("{note}");
            }
        }
        if !watch {
            return Ok(());
        }
        std::thread::sleep(WATCH_INTERVAL);
    }
}

/// Builds the aligned French table. Labels come from the catalog; column
/// padding counts characters (not bytes) so accented headers stay
/// aligned.
pub fn render_table(snap: &LandingSnapshot) -> String {
    let labels = archel_core::landing::table_labels();
    let rows: [(String, String); 3] = [
        (labels.version, snap.app_version.clone()),
        (labels.kernel, snap.kernel_version.clone()),
        (labels.updates, archel_core::landing::updates_cell(snap)),
    ];
    let width = rows
        .iter()
        .map(|(label, _)| label.chars().count())
        .max()
        .unwrap_or(0);
    let mut out = format!("{}\n", labels.title);
    for (label, value) in rows {
        out.push_str(&format!("{} : {value}\n", pad_right(&label, width)));
    }
    out
}

/// Right-pads `s` with spaces to `width` characters.
fn pad_right(s: &str, width: usize) -> String {
    let missing = width.saturating_sub(s.chars().count());
    format!("{s}{}", " ".repeat(missing))
}

#[cfg(test)]
mod tests {
    use super::*;
    use archel_core::updates::UpdateCount;

    fn fixture() -> LandingSnapshot {
        LandingSnapshot {
            app_version: "0.1.0".to_string(),
            kernel_version: "7.2.6-test".to_string(),
            repo: UpdateCount::Fresh(3),
            aur: UpdateCount::Fresh(2),
        }
    }

    #[test]
    fn table_shows_version_kernel_and_counts() {
        archel_core::i18n::set_language("fr");
        let table = render_table(&fixture());
        assert!(table.contains("0.1.0"), "app version: {table}");
        assert!(table.contains("7.2.6-test"), "kernel: {table}");
        assert!(table.contains('5'), "update total: {table}");
    }

    #[test]
    fn table_rows_share_one_label_column() {
        archel_core::i18n::set_language("fr");
        let table = render_table(&fixture());
        // Character (not byte) index: the "Mises ..." label carries a
        // two-byte accented character, so byte indices legitimately differ.
        let separators: Vec<usize> = table
            .lines()
            .skip(1)
            .filter_map(|line| line.chars().position(|c| c == ':'))
            .collect();
        assert_eq!(separators.len(), 3);
        assert!(
            separators.iter().all(|pos| *pos == separators[0]),
            "columns must align: {table}"
        );
    }

    #[test]
    fn padding_counts_characters_not_bytes() {
        assert_eq!(pad_right("ab", 4), "ab  ");
        assert_eq!(pad_right("abcd", 2), "abcd");
    }

    #[test]
    fn schema_landing_is_pinned() {
        // Same insta contract as the sys metrics: the landing --json
        // schema is pinned so later phases cannot reshape it silently.
        insta::assert_json_snapshot!("landing", fixture());
    }
}
