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

/// Runs `archel pkg search <query>`: repo hits first, then AUR, newest
/// versions on top (D-07). A missing `yay` binary surfaces the typed
/// French hint with a non-zero exit, never a crash.
pub fn run_search(query: &str, json: bool) -> Result<(), ArchelError> {
    let hits = archel_core::packages::search(query)?;
    if json {
        let payload = serde_json::to_string(&hits).map_err(|err| {
            ArchelError::Internal(format!("cannot encode pkg search JSON: {err}"))
        })?;
        println!("{payload}");
    } else {
        print!("{}", render_search_table(query, &hits));
    }
    Ok(())
}

/// Builds the search result table: the catalog title echoing the query,
/// then one `name | version | source` row per hit. No hits renders the
/// honest empty table (title plus header), never a zero-filled row.
pub fn render_search_table(query: &str, hits: &[PackageInfo]) -> String {    let labels = archel_core::packages::list_table_labels();
    let source_label = archel_core::packages::search_source_label();
    let name_width = hits
        .iter()
        .map(|hit| hit.name.chars().count())
        .max()
        .unwrap_or(0)
        .max(labels.name.chars().count());
    let version_width = hits
        .iter()
        .map(|hit| hit.version.chars().count())
        .max()
        .unwrap_or(0)
        .max(labels.version.chars().count());
    let mut out = format!("{}\n", archel_core::packages::search_title(query));
    out.push_str(&format!(
        "{} | {} | {}\n",
        pad_right(&labels.name, name_width),
        pad_right(&labels.version, version_width),
        source_label
    ));
    for hit in hits {
        out.push_str(&format!(
            "{} | {} | {}\n",
            pad_right(&hit.name, name_width),
            pad_right(&hit.version, version_width),
            hit.source.as_deref().unwrap_or("-")
        ));
    }
    out
}

/// Runs `archel pkg info <name>`: the D-04 pre-install card (sizes,
/// source, candidate versus installed version) or one JSON object.
/// An unknown name surfaces the honest not-found error, never a
/// zero-filled card.
pub fn run_info(name: &str, json: bool) -> Result<(), ArchelError> {
    let card = archel_core::packages::package_card(name)?;
    if json {
        let payload = serde_json::to_string(&card).map_err(|err| {
            ArchelError::Internal(format!("cannot encode pkg info JSON: {err}"))
        })?;
        println!("{payload}");
    } else {
        print!("{}", render_info_table(&card));
    }
    Ok(())
}

/// Builds the pre-install card: sizes, source, and candidate versus
/// installed versions side by side when the package is installed.
/// Absent readings render the honest catalog N/A cell (D-10), never
/// zero.
pub fn render_info_table(card: &archel_core::packages::PackageCard) -> String {
    let labels = archel_core::packages::card_labels();
    let na = archel_core::sensors::unavailable_label();
    let rows: [(String, String); 6] = [
        (labels.name, card.name.clone()),
        (labels.candidate, card.candidate_version.clone()),
        (
            labels.installed,
            card.installed_version.clone().unwrap_or_else(|| na.clone()),
        ),
        (labels.source, card.source.clone()),
        (
            labels.download_size,
            card.download_size.clone().unwrap_or_else(|| na.clone()),
        ),
        (
            labels.installed_size,
            card.installed_size.clone().unwrap_or_else(|| na.clone()),
        ),
    ];
    render_label_table(&archel_core::packages::card_title(&card.name), &rows)
}

/// Label/value layout shared by the card table (copied from `sys_cmds`:
/// character-count padding, so accented labels stay aligned).
fn render_label_table(title: &str, rows: &[(String, String)]) -> String {
    let width = rows
        .iter()
        .map(|(label, _)| label.chars().count())
        .max()
        .unwrap_or(0);
    let mut out = format!("{title}\n");
    for (label, value) in rows {
        out.push_str(&format!("{} : {value}\n", pad_right(label, width)));
    }
    out
}

/// Builds the installed-package table: one `name : version` row per
/// package under the catalog title. An empty DB renders the honest
/// empty table (title only), never a zero-filled placeholder (T-3-03).
pub fn render_list_table(pkgs: &[PackageInfo]) -> String {    let labels = archel_core::packages::list_table_labels();
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
                source: None,
            },
            PackageInfo {
                name: "vim".to_string(),
                version: "9.1-1".to_string(),
                source: None,
            },
        ]
    }

    fn search_fixture() -> Vec<PackageInfo> {
        vec![
            PackageInfo {
                name: "firefox".to_string(),
                version: "156.0.1-1".to_string(),
                source: Some("extra".to_string()),
            },
            PackageInfo {
                name: "phantom-browser-bin".to_string(),
                version: "155.6.3-1".to_string(),
                source: Some("AUR".to_string()),
            },
        ]
    }

    fn info_fixture() -> archel_core::packages::PackageCard {
        archel_core::packages::PackageCard {
            name: "firefox".to_string(),
            candidate_version: "156.0.1-1".to_string(),
            installed_version: Some("155.0-1".to_string()),
            source: "extra".to_string(),
            download_size: Some("84.84 MiB".to_string()),
            installed_size: Some("297.19 MiB".to_string()),
        }
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
    fn search_table_shows_name_version_and_source_per_hit() {
        archel_core::i18n::set_language("fr");
        let table = render_search_table("firefox", &search_fixture());
        assert!(table.contains("firefox"), "query echo or hit: {table}");
        assert!(table.contains("156.0.1-1"), "repo version: {table}");
        assert!(table.contains("extra"), "repo source: {table}");
        assert!(table.contains("phantom-browser-bin"), "AUR hit: {table}");
        assert!(table.contains("AUR"), "AUR source: {table}");
    }

    #[test]
    fn search_table_stays_honest_without_hits() {
        archel_core::i18n::set_language("fr");
        let table = render_search_table("archel-no-such-package-xyz", &[]);
        assert!(table.contains("archel-no-such-package-xyz"), "query echo: {table}");
        assert!(!table.contains("0.0"), "never a zero-filled row: {table}");
    }

    #[test]
    fn search_payload_serializes_to_parseable_json() {
        let payload =
            serde_json::to_string(&search_fixture()).expect("search encodes");
        serde_json::from_str::<serde_json::Value>(&payload)
            .expect("search payload must be one complete JSON value");
    }

    #[test]
    fn info_table_shows_sizes_source_and_both_versions() {
        archel_core::i18n::set_language("fr");
        let table = render_info_table(&info_fixture());
        assert!(table.contains("firefox"), "name: {table}");
        assert!(table.contains("156.0.1-1"), "candidate: {table}");
        assert!(table.contains("155.0-1"), "installed side by side: {table}");
        assert!(table.contains("extra"), "source: {table}");
        assert!(table.contains("84.84 MiB"), "download size: {table}");
        assert!(table.contains("297.19 MiB"), "installed size: {table}");
    }

    #[test]
    fn info_table_renders_na_for_absent_readings() {
        archel_core::i18n::set_language("fr");
        let mut card = info_fixture();
        card.installed_version = None;
        card.download_size = None;
        let table = render_info_table(&card);
        assert!(
            table.contains(&archel_core::sensors::unavailable_label()),
            "absent readings stay honest N/A, never zero: {table}"
        );
        assert!(!table.contains("0.0"), "never a zero-filled cell: {table}");
    }

    #[test]
    fn info_payload_serializes_to_parseable_json() {
        let payload =
            serde_json::to_string(&info_fixture()).expect("info encodes");
        serde_json::from_str::<serde_json::Value>(&payload)
            .expect("info payload must be one complete JSON value");
    }

    // insta pins: every pkg --json schema in snake_case English keys
    // mirroring the struct fields (Phase 2 precedent). A silent field
    // reshape fails here before reaching scripts.

    #[test]
    fn schema_pkg_list_is_pinned() {
        insta::assert_json_snapshot!("pkg_list", list_fixture());
    }

    #[test]
    fn schema_pkg_search_is_pinned() {
        insta::assert_json_snapshot!("pkg_search", search_fixture());
    }

    #[test]
    fn schema_pkg_info_is_pinned() {
        insta::assert_json_snapshot!("pkg_info", info_fixture());
    }

    #[test]
    fn padding_counts_characters_not_bytes() {
        assert_eq!(pad_right("ab", 4), "ab  ");
        assert_eq!(pad_right("abcd", 2), "abcd");
    }
}
