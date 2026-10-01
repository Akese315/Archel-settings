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
    // `.unchecked()` is load-bearing: without it duct turns every
    // non-zero exit (including pacman's exit-1 no-match) into an IO
    // error here, and the exit-code map below becomes unreachable.
    let handle = duct::cmd(&binary, args)
        .unchecked()
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

/// Pre-install review card (D-04): candidate version, installed versus
/// candidate side by side when present, source, and both sizes.
///
/// Field names are the v1 JSON schema (`--json` serializes this struct
/// directly); the schema is unstable until the GUI phase pins it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackageCard {
    pub name: String,
    pub candidate_version: String,
    pub installed_version: Option<String>,
    pub source: String,
    pub download_size: Option<String>,
    pub installed_size: Option<String>,
}

/// Composes the D-04 card for `name`: repo fields first
/// (`pacman -Si`), then AUR (`yay -Si`), installed version from the
/// localdb side by side when present.
///
/// An unknown name is the honest [`ArchelError::PackageNotFound`],
/// never a zero-filled card. A blank name is the same error: there is
/// no package to describe.
pub fn package_card(name: &str) -> Result<PackageCard, ArchelError> {
    if name.trim().is_empty() {
        return Err(ArchelError::PackageNotFound {
            name: name.to_string(),
        });
    }
    let installed_version = installed_version_of(name)?;
    let repo_out = run_capped_query("pacman", &["-Si", name], ArchelError::Internal)?;
    let repo_fields = parse_info_fields(&repo_out);
    if let Some(card) = compose_card(name, &repo_fields, None, installed_version.clone()) {
        return Ok(card);
    }
    let aur_out = run_capped_query("yay", &["-Si", name], |_| ArchelError::YayMissing)?;
    let aur_fields = parse_info_fields(&aur_out);
    compose_card(name, &aur_fields, Some("AUR"), installed_version).ok_or(
        ArchelError::PackageNotFound {
            name: name.to_string(),
        },
    )
}

/// Installed version from the localdb, `None` when the package is not
/// installed (the card then shows the candidate alone).
fn installed_version_of(name: &str) -> Result<Option<String>, ArchelError> {
    let alpm = alpm::Alpm::new("/", "/var/lib/pacman").map_err(|err| {
        ArchelError::Internal(format!("cannot open alpm localdb: {err}"))
    })?;
    Ok(alpm
        .localdb()
        .pkg(name)
        .ok()
        .map(|pkg| pkg.version().to_string()))
}

/// Parses `pacman -Si` / `yay -Si` C-locale output into `Key -> Value`
/// fields. Field lines look like `Repository      : extra`; indented
/// continuation lines (multi-line `Optional Deps`) and blanks are
/// skipped. Only the first colon splits: values keep theirs (URLs).
pub fn parse_info_fields(stdout: &str) -> std::collections::HashMap<String, String> {
    let mut fields = std::collections::HashMap::new();
    for line in stdout.lines() {
        if line.starts_with(' ') || line.starts_with('\t') || line.trim().is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        fields.insert(key.to_string(), value.trim().to_string());
    }
    fields
}

/// Builds the card from parsed `-Si` fields. `None` when the fields
/// carry no package record (unknown name in that source): the caller
/// tries the next source instead of rendering a zero-filled card.
/// `source_default` covers `yay -Si` AUR records, which carry no
/// `Repository` field of their own.
fn compose_card(
    name: &str,
    fields: &std::collections::HashMap<String, String>,
    source_default: Option<&str>,
    installed_version: Option<String>,
) -> Option<PackageCard> {
    let candidate_version = fields.get("Version")?.clone();
    let source = fields
        .get("Repository")
        .cloned()
        .or_else(|| source_default.map(str::to_string))?;
    Some(PackageCard {
        name: name.to_string(),
        candidate_version,
        installed_version,
        source,
        download_size: fields.get("Download Size").cloned(),
        installed_size: fields.get("Installed Size").cloned(),
    })
}

/// Card labels from the catalog, so the binary lays out the card
/// without owning any French copy (bin-calls-core invariant).
pub struct CardLabels {
    pub name: String,
    pub candidate: String,
    pub installed: String,
    pub source: String,
    pub download_size: String,
    pub installed_size: String,
}

/// Collects the six card row labels in one call.
pub fn card_labels() -> CardLabels {
    CardLabels {
        name: rust_i18n::t!("pkg.list_name").to_string(),
        candidate: rust_i18n::t!("pkg.info_candidate").to_string(),
        installed: rust_i18n::t!("pkg.info_installed").to_string(),
        source: rust_i18n::t!("pkg.search_source").to_string(),
        download_size: rust_i18n::t!("pkg.info_download_size").to_string(),
        installed_size: rust_i18n::t!("pkg.info_installed_size").to_string(),
    }
}

/// Card title with the package name echoed back, from the catalog.
pub fn card_title(name: &str) -> String {
    rust_i18n::t!("pkg.info_title", name = name).to_string()
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

    /// `LC_ALL=C`-shaped `pacman -Si` sample: repo fields, both sizes,
    /// a multi-line continuation, blanks.
    const PACMAN_SI_FIXTURE: &str = "Repository      : extra\nName            : firefox\nVersion         : 156.0.1-1\nDescription     : Fast, Private & Safe Web Browser\nURL             : https://www.firefox.com/\nOptional Deps   : hunspell-dictionary: Spell checking\n                  libnotify: Notification integration\nDownload Size   : 84.84 MiB\nInstalled Size  : 297.19 MiB\n\n";

    #[test]
    fn info_parse_reads_fields_skipping_continuations() {
        let fields = parse_info_fields(PACMAN_SI_FIXTURE);
        assert_eq!(fields.get("Repository").map(String::as_str), Some("extra"));
        assert_eq!(fields.get("Version").map(String::as_str), Some("156.0.1-1"));
        assert_eq!(
            fields.get("Download Size").map(String::as_str),
            Some("84.84 MiB")
        );
        assert_eq!(
            fields.get("Installed Size").map(String::as_str),
            Some("297.19 MiB")
        );
        assert!(
            fields.get("URL").is_some_and(|url| url.contains("https://")),
            "values keep their colons: {fields:?}"
        );
    }

    #[test]
    fn card_composes_sizes_source_and_both_versions() {
        let fields = parse_info_fields(PACMAN_SI_FIXTURE);
        let card = super::compose_card("firefox", &fields, None, Some("156.0.1-1".to_string()))
            .expect("fixture carries a full repo record");
        assert_eq!(card.name, "firefox");
        assert_eq!(card.candidate_version, "156.0.1-1");
        assert_eq!(card.installed_version.as_deref(), Some("156.0.1-1"));
        assert_eq!(card.source, "extra");
        assert_eq!(card.download_size.as_deref(), Some("84.84 MiB"));
        assert_eq!(card.installed_size.as_deref(), Some("297.19 MiB"));
    }

    #[test]
    fn card_falls_back_to_the_aur_default_source() {
        let mut fields = parse_info_fields(PACMAN_SI_FIXTURE);
        fields.remove("Repository");
        let card = super::compose_card("some-aur-pkg", &fields, Some("AUR"), None)
            .expect("AUR default covers the missing Repository field");
        assert_eq!(card.source, "AUR");
        assert_eq!(card.installed_version, None, "not installed stays absent");
    }

    #[test]
    fn card_refuses_records_without_a_candidate_version() {
        let fields = std::collections::HashMap::new();
        assert!(
            super::compose_card("some-aur-pkg", &fields, Some("AUR"), None).is_none(),
            "no record, no zero-filled card"
        );
    }

    #[test]
    fn live_card_shows_sizes_source_and_version_compare() {
        // `pacman` is always installed on an Arch machine, and its sync
        // record is always present: the card must show both versions.
        let card = package_card("pacman").expect("pacman record must resolve");
        assert!(!card.candidate_version.trim().is_empty());
        assert!(
            card.installed_version.is_some(),
            "installed pacman shows both versions side by side"
        );
        assert!(!card.source.trim().is_empty(), "source must show");
        assert!(
            card.download_size.is_some() && card.installed_size.is_some(),
            "repo record carries both sizes"
        );
    }

    #[test]
    fn live_unknown_name_is_the_honest_not_found_error() {
        // `yay -Si` on the unknown name needs no network to fail: the
        // RPC error exits non-zero and the composer still reports the
        // typed not-found variant, never a zero-filled card.
        let err = package_card("archel-no-such-pkg-xyz")
            .expect_err("unknown name must not render a card");
        assert!(
            matches!(err, ArchelError::PackageNotFound { .. }),
            "wrong variant: {err:?}"
        );
    }

    #[test]
    fn blank_card_name_is_not_found_never_a_dump() {
        let err = package_card("   ").expect_err("blank name must not query");
        assert!(matches!(err, ArchelError::PackageNotFound { .. }));
    }

    #[test]
    fn card_labels_come_from_the_catalog() {
        rust_i18n::set_locale("fr");
        let labels = super::card_labels();
        assert!(!labels.candidate.is_empty());
        assert!(!labels.installed.is_empty());
        assert!(!labels.download_size.is_empty());
        assert!(!labels.installed_size.is_empty());
    }
}
