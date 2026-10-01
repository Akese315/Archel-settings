//! `archel pkg list|search|info` renderers: French tables (default)
//! or one JSON object (`--json`).
//!
//! Bin-calls-core shape: package data plus every French string comes
//! from `archel-core::packages`; this module only lays out columns
//! (character-count padding, no table crate) and frames the single JSON
//! object. Package output is single-shot: no `--watch` here.

use archel_core::packages::PackageInfo;
use archel_core::ArchelError;

/// Runs `archel pkg list`: every installed package with its version
/// (D-08: installed only, search stays global in a later task).
pub fn run_list(json: bool) -> Result<(), ArchelError> {
    let pkgs = archel_core::packages::list_installed()?;
    if json {
        let payload = serde_json::to_string(&pkgs).map_err(|err| {
            ArchelError::Internal(format!("cannot encode pkg list JSON: {err}"))
        })?;
        println!("{payload}");
    } else {
        print!("{}", render_list_table(&pkgs));
    }
    Ok(())
}

/// Builds the installed-package table: one `name : version` row per
/// package under the catalog title. An empty DB renders the honest
/// empty table (title only), never a zero-filled placeholder (T-3-03).
pub fn render_list_table(pkgs: &[PackageInfo]) -> String {
    let labels = archel_core::packages::list_table_labels();
    let width = pkgs
        .iter()
        .map(|pkg| pkg.name.chars().count())
        .max()
        .unwrap_or(0)
        .max(labels.name.chars().count());
    let mut out = format!("{}\n", labels.title);
    out.push_str(&format!(
        "{} : {}\n",
        pad_right(&labels.name, width),
        labels.version
    ));
    for pkg in pkgs {
        out.push_str(&format!(
            "{} : {}\n",
            pad_right(&pkg.name, width),
            pkg.version
        ));
    }
    out
}

/// Right-pads `s` with spaces to `width` characters (copied from
/// `sys_cmds`: counts characters, not bytes, so accented headers stay
/// aligned).
fn pad_right(s: &str, width: usize) -> String {
    let missing = width.saturating_sub(s.chars().count());
    format!("{s}{}", " ".repeat(missing))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list_fixture() -> Vec<PackageInfo> {
        vec![
            PackageInfo {
                name: "firefox".to_string(),
                version: "140.0-1".to_string(),
            },
            PackageInfo {
                name: "vim".to_string(),
                version: "9.1-1".to_string(),
            },
        ]
    }

    #[test]
    fn list_table_shows_every_name_with_its_version() {
        archel_core::i18n::set_language("fr");
        let table = render_list_table(&list_fixture());
        assert!(table.contains("firefox"), "name: {table}");
        assert!(table.contains("140.0-1"), "version: {table}");
        assert!(table.contains("vim"), "second name: {table}");
        assert!(table.contains("9.1-1"), "second version: {table}");
    }

    #[test]
    fn list_table_stays_honest_on_an_empty_db() {
        archel_core::i18n::set_language("fr");
        let table = render_list_table(&[]);
        assert!(!table.trim().is_empty(), "title must show");
        assert!(!table.contains("0.0"), "never a zero-filled row: {table}");
    }

    #[test]
    fn list_payload_serializes_to_parseable_json() {
        let payload =
            serde_json::to_string(&list_fixture()).expect("list encodes");
        serde_json::from_str::<serde_json::Value>(&payload)
            .expect("list payload must be one complete JSON value");
    }

    #[test]
    fn padding_counts_characters_not_bytes() {
        assert_eq!(pad_right("ab", 4), "ab  ");
        assert_eq!(pad_right("abcd", 2), "abcd");
    }
}
