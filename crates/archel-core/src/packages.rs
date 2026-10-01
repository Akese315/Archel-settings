//! Installed-package reads (PKG-01, D-08): alpm localdb list.
//!
//! The installed list comes from the alpm localdb in-process: no fork,
//! no locale hazard, epoch-aware version ordering through `Ver`'s
//! `PartialOrd` (`alpm_pkg_vercmp`). Repo and AUR queries (search, card)
//! stay `LC_ALL=C`-pinned subprocesses and land in later tasks.

use std::time::Duration;

use serde::Serialize;

use crate::error::ArchelError;

/// Upper bound per package-query subprocess (offline must fail fast,
/// same budget as the update checkers).
pub const QUERY_TIMEOUT: Duration = Duration::from_secs(15);

/// One installed package: name plus version, as shown by
/// `archel pkg list`.
///
/// Field names are the v1 JSON schema (`--json` serializes this struct
/// directly); the schema is unstable until the GUI phase pins it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    /// Origin repository (`extra`, `core`, …) or `AUR`. `None` for the
    /// installed list, which has no source concept (D-08).
    pub source: Option<String>,
}

/// Reads every installed package from the alpm localdb, sorted by
/// descending version so the newest builds surface first.
///
/// A corrupt or version-skewed DB maps to a typed error with a French
/// catalog message (T-3-03): never a panic, never a partial list.
pub fn list_installed() -> Result<Vec<PackageInfo>, ArchelError> {
    let alpm =
        alpm::Alpm::new("/", "/var/lib/pacman").map_err(|err| {
            ArchelError::Internal(format!("cannot open alpm localdb: {err}"))
        })?;
    let local = alpm.localdb();
    let mut pkgs: Vec<_> = local.pkgs().iter().collect();
    // `Ver` implements `PartialOrd` only (A6): descending compare with
    // the Equal fallback so incomparable pairs keep localdb order.
    pkgs.sort_by(|a, b| {
        b.version()
            .partial_cmp(a.version())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(pkgs
        .iter()
        .map(|pkg| PackageInfo {
            name: pkg.name().to_string(),
            version: pkg.version().to_string(),
            source: None,
        })
        .collect())
}

/// Table labels from the catalog, so the binary lays out columns
/// without owning any French copy (bin-calls-core invariant).
pub struct PkgListLabels {
    pub title: String,
    pub name: String,
    pub version: String,
}

/// Collects the three pkg-list table labels in one call.
pub fn list_table_labels() -> PkgListLabels {
    PkgListLabels {
        title: rust_i18n::t!("pkg.list_title").to_string(),
        name: rust_i18n::t!("pkg.list_name").to_string(),
        version: rust_i18n::t!("pkg.list_version").to_string(),
    }
}

/// Searches repos first (`pacman -Ss`) then AUR (`yay -Ss`), merged
/// into one version-descending list with repo hits before AUR on ties
/// (D-07).
///
/// A blank query short-circuits to an empty list: the PKG-01
/// empty-query semantics are an unresolved specless-probe item, so this
/// task surfaces the choice instead of guessing (never a full-DB dump).
/// A missing `yay` binary is the typed [`ArchelError::YayMissing`],
/// never a crash.
pub fn search(query: &str) -> Result<Vec<PackageInfo>, ArchelError> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let repo_out =
        run_capped_query("pacman", &["-Ss", query], ArchelError::Internal)?;
    let mut hits: Vec<PackageInfo> = parse_search_output(&repo_out)
        .into_iter()
        .map(|(source, name, version)| PackageInfo {
            name,
            version,
            source: Some(source),
        })
        .collect();
    let aur_out = run_capped_query("yay", &["-Ss", query], |_| {
        ArchelError::YayMissing
    })?;
    hits.extend(parse_search_output(&aur_out).into_iter().map(
        |(source, name, version)| PackageInfo {
            name,
            version,
            source: Some(source),
        },
    ));
    sort_hits(&mut hits);
    Ok(hits)
}

/// D-07 ordering: epoch-aware version-descending (via `alpm_pkg_vercmp`,
/// so `2:1-2` outranks `155.6.3-1`), repo hits before AUR on ties.
pub fn sort_hits(hits: &mut [PackageInfo]) {
    hits.sort_by(|a, b| {
        alpm::vercmp(b.version.clone(), a.version.clone())
            .then_with(|| is_aur_source(&a.source).cmp(&is_aur_source(&b.source)))
    });
}

/// True only for AUR hits (`yay -Ss` normalizes `aur/` to `AUR` at
/// parse time); everything else — repos and source-less entries —
/// sorts repo-side on ties.
fn is_aur_source(source: &Option<String>) -> bool {
    source.as_deref() == Some("AUR")
}

/// Parses `pacman -Ss` / `yay -Ss` C-locale output into
/// (source, name, version) triples. Header lines look like
/// `extra/firefox 156.0.1-1 [installed]` or
/// `aur/phantom-browser-bin 155.6.3-1 (+0 0.00) [175d4h]`; only the
/// first two fields are read, the trailing flags stay unread.
/// Indented description lines, blanks, and malformed headers are
/// skipped. `aur` sources normalize to `AUR`.
pub fn parse_search_output(stdout: &str) -> Vec<(String, String, String)> {
    let mut hits = Vec::new();
    for line in stdout.lines() {
        if line.starts_with(' ') || line.starts_with('\t') || line.trim().is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let (Some(repo_name), Some(version)) = (parts.next(), parts.next()) else {
            continue;
        };
        let Some((repo, name)) = repo_name.split_once('/') else {
            continue;
        };
        if name.is_empty() || version.is_empty() {
            continue;
        }
        let source = if repo == "aur" { "AUR" } else { repo };
        hits.push((source.to_string(), name.to_string(), version.to_string()));
    }
    hits
}

/// Runs one parse-bound query (`pacman` / `yay`) and returns its stdout.
/// Mirrors the `updates.rs` spawn shape exactly, plus the T-3-01
/// `LC_ALL=C` pin (field parsing breaks on localized output otherwise).
/// A missing binary is the caller-chosen `missing` variant; every other
/// failure is an [`ArchelError::Internal`] log detail (stderr stays
/// log-only per T-3-02).
fn run_capped_query(
    tool: &str,
    args: &[&str],
    missing: impl FnOnce(String) -> ArchelError,
) -> Result<String, ArchelError> {
    let binary =
        which::which(tool).map_err(|err| missing(format!("{tool} ({err})")))?;
    let handle = duct::cmd(&binary, args)
        .env("LC_ALL", "C")
        .stdout_capture()
        .stderr_capture()
        .start()
        .map_err(|err| ArchelError::Internal(format!("cannot run '{tool}': {err}")))?;
    let output = match handle.wait_timeout(QUERY_TIMEOUT).map_err(|err| {
        ArchelError::Internal(format!("cannot wait on '{tool}': {err}"))
    })? {
        Some(output) => output.clone(),
        None => {
            let _ = handle.kill();
            return Err(ArchelError::Internal(format!(
                "'{tool}' exceeded the query timeout"
            )));
        }
    };
    // Log-only capture of query stderr (kept for diagnostics; the user
    // only ever sees catalog French, never this raw text).
    let _stderr_for_logs = String::from_utf8_lossy(&output.stderr);
    map_query_exit(tool, output.status.code())?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Maps a query exit code: 0 is hits, 1 is pacman's no-match code
/// (verified live: empty stdout, renders the honest empty table, not an
/// error); anything else is a query failure.
pub fn map_query_exit(tool: &str, code: Option<i32>) -> Result<(), ArchelError> {
    match code {
        Some(0) | Some(1) => Ok(()),
        _ => Err(ArchelError::Internal(format!(
            "{tool} query failed with exit code {code:?}"
        ))),
    }
}

/// Search result title with the query echoed back, from the catalog so
/// the binary owns no French copy.
pub fn search_title(query: &str) -> String {
    rust_i18n::t!("pkg.search_title", query = query).to_string()
}

/// Search result source-column label, from the catalog.
pub fn search_source_label() -> String {
    rust_i18n::t!("pkg.search_source").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_list_reads_from_the_localdb() {
        let pkgs = list_installed().expect("localdb must open on this machine");
        assert!(
            !pkgs.is_empty(),
            "an Arch install always has packages installed"
        );
        for pkg in &pkgs {
            assert!(!pkg.name.trim().is_empty(), "name must show");
            assert!(!pkg.version.trim().is_empty(), "version must show");
        }
    }

    #[test]
    fn installed_list_is_sorted_version_descending() {
        let pkgs = list_installed().expect("localdb must open on this machine");
        // Re-check the tracer sort with the total `vercmp` order: every
        // adjacent pair must already be descending.
        for pair in pkgs.windows(2) {
            let ordering =
                alpm::vercmp(pair[0].version.clone(), pair[1].version.clone());
            assert!(
                ordering != std::cmp::Ordering::Less,
                "not descending: {} {} before {} {}",
                pair[0].name,
                pair[0].version,
                pair[1].name,
                pair[1].version
            );
        }
    }

    #[test]
    fn list_labels_come_from_the_catalog() {
        rust_i18n::set_locale("fr");
        let labels = list_table_labels();
        assert!(!labels.title.is_empty());
        assert!(!labels.name.is_empty());
        assert!(!labels.version.is_empty());
    }

    /// `LC_ALL=C`-shaped `pacman -Ss` sample: repo headers with flags
    /// and groups, indented descriptions, blanks.
    const PACMAN_SS_FIXTURE: &str = "extra/firefox 156.0.1-1 [installed]\n    Standalone web browser from mozilla.org\n\nextra/firefox-adblock-plus 4.43.4-1 (firefox-addons)\n    Adblock Plus extension\n";

    /// `LC_ALL=C`-shaped `yay -Ss` sample: AUR headers carry vote and
    /// age trailers plus orphan flags after the version.
    const YAY_SS_FIXTURE: &str = "aur/phantom-browser-bin 155.6.3-1 (+0 0.00) [175d4h]\n    Privacy-focused Firefox fork\n\naur/zen-browser-ublock-origin 2:1-2 (+0 0.00) [193d4h]\n    UBlock origin for zen\n";

    #[test]
    fn search_parse_reads_repo_headers_skipping_descriptions() {
        let hits = parse_search_output(PACMAN_SS_FIXTURE);
        assert_eq!(hits.len(), 2, "two headers, rest skipped: {hits:?}");
        assert_eq!(
            hits[0],
            (
                "extra".to_string(),
                "firefox".to_string(),
                "156.0.1-1".to_string()
            )
        );
        assert_eq!(hits[1].0, "extra");
        assert_eq!(hits[1].1, "firefox-adblock-plus");
    }

    #[test]
    fn search_parse_normalizes_aur_source_and_keeps_epochs() {
        let hits = parse_search_output(YAY_SS_FIXTURE);
        assert_eq!(hits.len(), 2, "two AUR headers: {hits:?}");
        assert!(hits.iter().all(|(source, _, _)| source == "AUR"));
        assert!(
            hits.iter().any(|(_, _, version)| version == "2:1-2"),
            "epoch versions survive parsing: {hits:?}"
        );
    }

    #[test]
    fn search_parse_skips_malformed_headers() {
        let out = "garbage line without slash\n\
            extra/ 1.0-1\n\
            extra/good 2.0-1\n   \n";
        let hits = parse_search_output(out);
        assert_eq!(hits.len(), 1, "only the well-formed header: {hits:?}");
        assert_eq!(hits[0].1, "good");
    }

    #[test]
    fn blank_query_short_circuits_to_empty_without_spawning() {
        for query in ["", "   ", "\t\n "] {
            assert_eq!(
                search(query).expect("blank query must not fail"),
                Vec::new(),
                "blank query renders the honest empty table"
            );
        }
    }

    #[test]
    fn missing_binary_maps_to_the_caller_chosen_variant() {
        let err = run_capped_query("archel-no-such-tool", &[], |_| {
            ArchelError::YayMissing
        })
        .expect_err("unknown binary must fail the which gate");
        assert!(
            matches!(err, ArchelError::YayMissing),
            "yay-missing path must stay typed: {err:?}"
        );
    }

    #[test]
    fn query_exit_one_means_no_match_not_failure() {
        // Verified live: `pacman -Ss <unknown>` exits 1 with empty stdout.
        assert!(map_query_exit("pacman", Some(0)).is_ok());
        assert!(map_query_exit("pacman", Some(1)).is_ok());
        assert!(map_query_exit("yay", Some(0)).is_ok());
        assert!(map_query_exit("yay", Some(2)).is_err());
        assert!(map_query_exit("pacman", None).is_err());
    }

    fn hit(name: &str, version: &str, source: Option<&str>) -> PackageInfo {
        PackageInfo {
            name: name.to_string(),
            version: version.to_string(),
            source: source.map(str::to_string),
        }
    }

    #[test]
    fn hits_sort_version_descending_with_repos_before_aur_on_ties() {
        let mut hits = vec![
            hit("a-low", "1.0-1", Some("extra")),
            hit("an-aur", "155.6.3-1", Some("AUR")),
            hit("a-repo", "157.0b5-1", Some("extra")),
            hit("dup-aur", "2.0-1", Some("AUR")),
            hit("dup-repo", "2.0-1", Some("extra")),
        ];
        sort_hits(&mut hits);
        let names: Vec<_> = hits.iter().map(|h| h.name.as_str()).collect();
        assert_eq!(
            names,
            ["a-repo", "an-aur", "dup-repo", "dup-aur", "a-low"],
            "version-desc, repos first on ties: {names:?}"
        );
    }

    #[test]
    fn hits_sort_handles_epoch_versions() {
        let mut hits = vec![
            hit("plain", "155.6.3-1", Some("extra")),
            hit("epoch", "2:1-2", Some("AUR")),
        ];
        sort_hits(&mut hits);
        assert_eq!(hits[0].name, "epoch", "epoch 2 outranks plain 155");
    }
}
