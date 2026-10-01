//! `archel pkg list|search|info` renderers: French tables (default)
//! or one JSON object (`--json`).
//!
//! Bin-calls-core shape: package data plus every French string comes
//! from `archel-core::packages`; this module only lays out columns
//! (character-count padding, no table crate) and frames the single JSON
//! object. Package output is single-shot: no `--watch` here.

use archel_core::packages::{PackageCard, PackageInfo};
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

/// Runs `archel pkg install <pkgs...>`: unprivileged preview first
/// (one card per package plus the dependency closure), TTY-gated
/// [o/N] confirmation (D-01), then the confirmed transaction through
/// pkexec with live streaming (D-02). `--json` emits one report object
/// (transcript included) instead of printing live lines.
///
/// AUR guard (G-03-1, mirror of the `run_aur` repo redirect): when ANY
/// preview card is AUR-sourced the whole list redirects to `pkg aur`
/// and nothing installs — no card render, no confirm, no helper spawn.
/// Mixed repo+AUR lists use refuse-all semantics on purpose: a partial
/// transaction the user did not confirm as partial would surprise, so
/// the redirect lists the AUR names and the repo names stay untouched.
/// The guard is read-only over the preview cards: the original
/// `packages` slice reaches the installs path unchanged (T-3-12), and
/// the AUR check reuses the exact `is_aur_card` predicate the `run_aur`
/// redirect trusts (T-3-13).
pub fn run_install(packages: &[String], json: bool) -> Result<(), ArchelError> {
    let preview = archel_core::packages::install_preview(packages)?;
    let aur_names = aur_names_in_cards(&preview.cards);
    if !aur_names.is_empty() {
        print_install_aur_redirect(&preview.cards, &aur_names, json);
        return Ok(());
    }
    if json {
        let transcript = stream_install(packages, true)?;
        print_mutation_report("install", packages, transcript, None);
    } else {
        for card in &preview.cards {
            print!("{}", render_info_table(card));
        }
        print_preview_list(
            &archel_core::packages::preview_dependencies_label(),
            &preview.dependencies,
        );
        archel_core::packages::confirm_action(
            &archel_core::packages::confirm_install_prompt(packages),
        )?;
        let transcript = stream_install(packages, false)?;
        let _ = transcript;
        println!("{}", archel_core::packages::install_ok_line(packages));
    }
    Ok(())
}

/// Names of the preview cards sourced from AUR. A non-empty result
/// means the install list must redirect to `pkg aur` instead of ever
/// reaching pacman. Pure over the cards: no spawn, no TTY, no network.
fn aur_names_in_cards(cards: &[PackageCard]) -> Vec<String> {
    cards
        .iter()
        .filter(|card| archel_core::aur::is_aur_card(card))
        .map(|card| card.name.clone())
        .collect()
}

/// Prints the install-to-AUR redirect (G-03-1): the human branch shows
/// the card-free catalog hint line, the `--json` branch emits one
/// parseable object with the preview cards plus the redirect note
/// (same card-plus-note framing as the `run_aur` repo redirect).
fn print_install_aur_redirect(cards: &[PackageCard], aur_names: &[String], json: bool) {
    let note = if aur_names.len() == 1 {
        archel_core::aur::is_aur_hint(&aur_names[0])
    } else {
        archel_core::aur::is_aur_hint_list(aur_names)
    };
    if json {
        println!(
            "{}",
            serde_json::json!({
                "cards": cards,
                "note": note,
            })
        );
    } else {
        println!("{note}");
    }
}

/// Streams an install through the helper: live lines in human mode,
/// silent accumulation under `--json` (D-02 streams live; the script
/// path keeps the transcript in the report object instead).
fn stream_install(packages: &[String], silent: bool) -> Result<String, ArchelError> {
    if silent {
        let mut sink = |_: &str| {};
        archel_core::privileged::pkg_install(packages, &mut sink)
    } else {
        let mut live = |line: &str| println!("{line}");
        archel_core::privileged::pkg_install(packages, &mut live)
    }
}

/// Runs `archel pkg remove <pkgs...>`: same confirm-then-stream flow
/// as install, over the removal preview (installed versions plus the
/// removal closure). Plain `-R` underneath: recursion stays opt-in.
pub fn run_remove(packages: &[String], json: bool) -> Result<(), ArchelError> {
    let preview = archel_core::packages::remove_preview(packages)?;
    if json {
        let transcript = stream_remove(packages, true)?;
        print_mutation_report("remove", packages, transcript, None);
    } else {
        for (name, version) in &preview.installed {
            println!("{name} : {version}");
        }
        print_preview_list(
            &archel_core::packages::preview_removals_label(),
            &preview.removals,
        );
        archel_core::packages::confirm_action(
            &archel_core::packages::confirm_remove_prompt(packages),
        )?;
        let transcript = stream_remove(packages, false)?;
        let _ = transcript;
        println!("{}", archel_core::packages::remove_ok_line(packages));
    }
    Ok(())
}

/// Streams a removal through the helper (live vs silent, as install).
fn stream_remove(packages: &[String], silent: bool) -> Result<String, ArchelError> {
    if silent {
        let mut sink = |_: &str| {};
        archel_core::privileged::pkg_remove(packages, &mut sink)
    } else {
        let mut live = |line: &str| println!("{line}");
        archel_core::privileged::pkg_remove(packages, &mut live)
    }
}

/// Runs `archel pkg upgrade`: the full `-Syu` only (PKG-04, D-03). A
/// package argument is the partial-upgrade equivalent and is refused
/// with the catalog French message — nothing executes. An empty
/// pending list reports honestly up-to-date with no prompt.
pub fn run_upgrade(package: Option<String>, json: bool) -> Result<(), ArchelError> {
    if let Some(name) = package {
        return Err(ArchelError::PartialUpgradeRefused { name });
    }
    let pending = archel_core::packages::upgrade_preview()?;
    if pending.is_empty() {
        if json {
            print_mutation_report(
                "upgrade",
                &[],
                String::new(),
                Some(archel_core::packages::up_to_date_line()),
            );
        } else {
            println!("{}", archel_core::packages::up_to_date_line());
        }
        return Ok(());
    }
    if json {
        let transcript = stream_upgrade(true)?;
        print_mutation_report("upgrade", &[], transcript, None);
    } else {
        print_preview_list(
            &archel_core::packages::preview_pending_label(),
            &pending,
        );
        archel_core::packages::confirm_action(
            &archel_core::packages::confirm_upgrade_prompt(),
        )?;
        let transcript = stream_upgrade(false)?;
        let _ = transcript;
        println!("{}", archel_core::packages::upgrade_ok_line());
    }
    Ok(())
}

/// Streams the full upgrade through the helper (live vs silent).
fn stream_upgrade(silent: bool) -> Result<String, ArchelError> {
    if silent {
        let mut sink = |_: &str| {};
        archel_core::privileged::pkg_upgrade(&mut sink)
    } else {
        let mut live = |line: &str| println!("{line}");
        archel_core::privileged::pkg_upgrade(&mut live)
    }
}

/// Runs `archel pkg aur <name>`: AUR review-then-build (PKG-05).
/// The D-04 card proves the name first: repo sources redirect to
/// `pkg install` before any fetch. AUR sources get the full review
/// (card plus trust signals plus dependency preview plus the ENTIRE
/// PKGBUILD text), then the TTY-gated [o/N] confirm shared with plan 02,
/// then `yay -S` as the user with native sudo handling (D-06 — Archel
/// never orchestrates elevation). Abort at any gate builds nothing:
/// `yay -S` is reachable only past the confirm.
pub fn run_aur(name: &str, json: bool) -> Result<(), ArchelError> {
    archel_core::packages::validate_package_names(&[name.to_string()])?;
    let card = archel_core::packages::package_card(name)?;
    if !archel_core::aur::is_aur_card(&card) {
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "card": card,
                    "note": archel_core::aur::not_aur_hint(name),
                })
            );
        } else {
            print!("{}", render_info_table(&card));
            println!("{}", archel_core::aur::not_aur_hint(name));
        }
        return Ok(());
    }
    let review = archel_core::aur::review_package(name)?;
    if json {
        let payload = serde_json::to_string(&review).map_err(|err| {
            ArchelError::Internal(format!("cannot encode aur review JSON: {err}"))
        })?;
        println!("{payload}");
    } else {
        print!("{}", render_aur_review(&review));
    }
    run_aur_confirmed(name, json, |on_line| {
        archel_core::aur::yay_build(name, on_line)
    })
}

/// Post-review half of `run_aur`: the TTY-gated [o/N] confirm, then the
/// build closure. Abort (or a non-TTY stdin) builds nothing — the
/// closure is reachable only past the gate. The closure is injectable so
/// the abort path is unit-covered headless with a spawn probe (no
/// network, no TTY): live `yay -S` stays a manual verify-work item.
fn run_aur_confirmed(
    name: &str,
    json: bool,
    build: impl FnOnce(&mut dyn FnMut(&str)) -> Result<String, ArchelError>,
) -> Result<(), ArchelError> {
    archel_core::packages::confirm_action(&archel_core::aur::confirm_aur_prompt(name))?;
    run_aur_stream(name, json, build)
}

/// Streams the confirmed build: live lines in human mode, silent
/// accumulation under `--json` with the transcript inside the report
/// object (same framing as the plan-02 mutations).
fn run_aur_stream(
    name: &str,
    json: bool,
    build: impl FnOnce(&mut dyn FnMut(&str)) -> Result<String, ArchelError>,
) -> Result<(), ArchelError> {
    // `FnOnce` into `FnMut` call sites: exactly one branch runs.
    let mut build = Some(build);
    if json {
        let mut sink = |_: &str| {};
        let transcript = build.take().expect("build runs once")(&mut sink)?;
        println!(
            "{}",
            mutation_report_value("aur", &[name.to_string()], transcript, None)
        );
    } else {
        let mut live = |line: &str| println!("{line}");
        let transcript = build.take().expect("build runs once")(&mut live)?;
        let _ = transcript;
        println!("{}", archel_core::aur::aur_ok_line(name));
    }
    Ok(())
}

/// Builds the full AUR review display: the D-04 card, trust signals
/// (N/A where the record carries none), the dependency preview, any
/// heuristic warnings, then the ENTIRE PKGBUILD text (D-05: no
/// metadata-only shortcut — the user reads the code before validating).
pub fn render_aur_review(review: &archel_core::aur::AurReview) -> String {
    let mut out = String::new();
    out.push_str(&render_info_table(&review.card));
    let trust = archel_core::aur::trust_labels();
    let na = archel_core::sensors::unavailable_label();
    let rows: [(String, String); 4] = [
        (
            trust.maintainer,
            review.trust.maintainer.clone().unwrap_or_else(|| na.clone()),
        ),
        (
            trust.votes,
            review.trust.votes.clone().unwrap_or_else(|| na.clone()),
        ),
        (
            trust.popularity,
            review.trust.popularity.clone().unwrap_or_else(|| na.clone()),
        ),
        (
            trust.out_of_date,
            review.trust.out_of_date.clone().unwrap_or_else(|| na.clone()),
        ),
    ];
    out.push_str(&render_label_table(
        &archel_core::aur::review_title(&review.card.name),
        &rows,
    ));
    out.push_str(&archel_core::aur::aur_dependencies_label());
    out.push('\n');
    for dep in &review.dependencies {
        out.push_str(dep);
        out.push('\n');
    }
    if !review.warnings.is_empty() {
        out.push_str(&archel_core::aur::warnings_title());
        out.push('\n');
        for warning in &review.warnings {
            out.push_str(warning);
            out.push('\n');
        }
    }
    out.push_str(&archel_core::aur::pkgbuild_title());
    out.push('\n');
    out.push_str(&review.pkgbuild);
    if !review.pkgbuild.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Prints one preview section: the catalog label then one entry per
/// line. An empty section prints the label alone (honest, never
/// zero-filled).
fn print_preview_list(label: &str, entries: &[String]) {
    println!("{label}");
    for entry in entries {
        println!("{entry}");
    }
}

/// Prints the single `--json` report object for a mutation: the action
/// name, the package set, the full transcript lines, and an optional
/// human note (up-to-date path). One object, always parseable (D-10).
fn print_mutation_report(
    action: &str,
    packages: &[String],
    transcript: String,
    note: Option<String>,
) {
    println!("{}", mutation_report_value(action, packages, transcript, note));
}

/// Builds the mutation report value: pure framing over the transcript
/// lines, so the `--json` schema is unit-pinned without capturing
/// stdout.
fn mutation_report_value(
    action: &str,
    packages: &[String],
    transcript: String,
    note: Option<String>,
) -> serde_json::Value {
    let lines: Vec<String> = transcript.lines().map(str::to_string).collect();
    serde_json::json!({
        "action": action,
        "packages": packages,
        "ok": true,
        "transcript": lines,
        "note": note,
    })
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
    fn schema_aur_review_is_pinned() {
        insta::assert_json_snapshot!("aur_review", aur_review_fixture());
    }

    #[test]
    fn schema_aur_result_is_pinned() {
        insta::assert_json_snapshot!(
            "aur_result",
            mutation_report_value(
                "aur",
                &["yay-bin".to_string()],
                "downloading\ninstalling\n".to_string(),
                None,
            )
        );
    }

    #[test]
    fn padding_counts_characters_not_bytes() {
        assert_eq!(pad_right("ab", 4), "ab  ");
        assert_eq!(pad_right("abcd", 2), "abcd");
    }

    #[test]
    fn upgrade_with_a_package_name_is_the_refused_partial_path() {
        archel_core::i18n::set_language("fr");
        // D-03: the refusal lands before any preview spawn or prompt,
        // so this is deterministic headless (no TTY, no pkexec).
        let err = run_upgrade(Some("firefox".to_string()), false)
            .expect_err("single-package upgrade must be refused");
        assert!(
            matches!(err, ArchelError::PartialUpgradeRefused { .. }),
            "wrong variant: {err:?}"
        );
        let rendered = err.message();
        assert!(
            rendered.contains("firefox"),
            "refusal must name the package: {rendered}"
        );
    }

    #[test]
    fn upgrade_without_args_never_refuses_before_the_preview() {
        // The bare upgrade path reaches the preview/confirm flow: in a
        // headless suite stdin is not a TTY, so it stops at the TTY
        // gate (NeedsTty) or the up-to-date early return — never at
        // the partial-upgrade refusal and never at a mutation.
        use std::io::IsTerminal;
        if std::io::stdin().is_terminal() {
            return;
        }
        let result = run_upgrade(None, false);
        match result {
            Ok(()) => {}
            Err(ArchelError::NeedsTty) => {}
            Err(ArchelError::ConfirmationAborted) => {}
            Err(other) => panic!("bare upgrade must not fail this way: {other:?}"),
        }
    }

    #[test]
    fn mutation_report_splits_the_transcript_into_lines() {
        let value = mutation_report_value(
            "install",
            &["firefox".to_string()],
            "downloading\ninstalling\n".to_string(),
            None,
        );
        assert_eq!(value["action"], "install");
        assert_eq!(value["ok"], true);
        assert_eq!(value["packages"][0], "firefox");
        assert_eq!(value["transcript"], serde_json::json!(["downloading", "installing"]));
        assert!(value["note"].is_null());
        // Round-trips as one parseable object (D-10).
        serde_json::from_str::<serde_json::Value>(&value.to_string())
            .expect("report must be one complete JSON value");
    }

    #[test]
    fn install_action_validates_names_before_spawning() {
        // Crafted names fail at the client gate: no preview spawn, no
        // pkexec prompt, nothing executes.
        let err = run_install(&["-foo".to_string()], false)
            .expect_err("crafted name must fail fast");
        assert!(
            matches!(err, ArchelError::InvalidPackageName { .. }),
            "wrong variant: {err:?}"
        );
    }

    fn aur_review_fixture() -> archel_core::aur::AurReview {
        archel_core::aur::AurReview {
            card: info_fixture(),
            pkgbuild: "pkgname=firefox\npkgver=156.0.1\nbuild() {\n  echo building\n}\n"
                .to_string(),
            trust: archel_core::aur::TrustSignals {
                maintainer: Some("mozilla".to_string()),
                votes: Some("1200".to_string()),
                popularity: Some("9.5".to_string()),
                out_of_date: Some("No".to_string()),
            },
            dependencies: vec!["gtk3".to_string(), "nss".to_string()],
            warnings: vec![],
        }
    }

    #[test]
    fn aur_action_validates_names_before_spawning() {
        // Crafted names fail at the client gate: no card spawn, no
        // fetch, no pkexec prompt, nothing executes.
        let err = run_aur("-foo", false).expect_err("crafted name must fail fast");
        assert!(
            matches!(err, ArchelError::InvalidPackageName { .. }),
            "wrong variant: {err:?}"
        );
    }

    #[test]
    fn aur_review_renders_card_trust_deps_and_full_pkgbuild() {
        archel_core::i18n::set_language("fr");
        let table = render_aur_review(&aur_review_fixture());
        assert!(table.contains("firefox"), "card name: {table}");
        assert!(table.contains("mozilla"), "maintainer: {table}");
        assert!(table.contains("1200"), "votes: {table}");
        assert!(table.contains("gtk3"), "dep preview: {table}");
        assert!(
            table.contains("pkgname=firefox"),
            "full PKGBUILD text, never metadata-only: {table}"
        );
        assert!(
            table.contains("echo building"),
            "build body visible for review: {table}"
        );
    }

    #[test]
    fn aur_review_renders_na_for_absent_trust_signals() {
        archel_core::i18n::set_language("fr");
        let mut review = aur_review_fixture();
        review.trust.maintainer = None;
        review.trust.votes = None;
        let table = render_aur_review(&review);
        assert!(
            table.contains(&archel_core::sensors::unavailable_label()),
            "absent signals stay honest N/A: {table}"
        );
    }

    #[test]
    fn aur_review_payload_serializes_to_parseable_json() {
        let payload =
            serde_json::to_string(&aur_review_fixture()).expect("aur review encodes");
        let value: serde_json::Value =
            serde_json::from_str(&payload).expect("one complete JSON value");
        assert_eq!(value["card"]["name"], "firefox");
        assert_eq!(value["trust"]["maintainer"], "mozilla");
        assert!(
            value["pkgbuild"].as_str().is_some_and(|t| t.contains("pkgname=")),
            "full PKGBUILD text travels in the JSON review"
        );
    }

    #[test]
    fn aur_repo_package_redirects_without_fetching_or_building() {
        // `pacman` lives in the repos: the D-04 card proves it, the
        // redirect fires before any `yay -G` fetch, and no TTY is needed
        // because nothing interactive happens.
        archel_core::i18n::set_language("fr");
        run_aur("pacman", false).expect("repo redirect never fails the name");
    }

    #[test]
    fn aur_confirm_abort_spawns_no_build() {
        // The confirm gate with an injectable build probe: headless stdin
        // is not a TTY, so the gate refuses before the closure — the
        // abort path structurally spawns no `yay -S` (no network here).
        use std::io::IsTerminal;
        if std::io::stdin().is_terminal() {
            return;
        }
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let builds = Arc::new(AtomicUsize::new(0));
        let probe = Arc::clone(&builds);
        let err = run_aur_confirmed("yay-bin", false, move |_: &mut dyn FnMut(&str)| {
            probe.fetch_add(1, Ordering::SeqCst);
            Ok(String::new())
        })
        .expect_err("headless confirm must refuse");
        assert!(
            matches!(err, ArchelError::NeedsTty),
            "abort path needs the typed refusal, never a build: {err:?}"
        );
        assert_eq!(
            builds.load(Ordering::SeqCst),
            0,
            "the abort path spawns no build process"
        );
    }

    fn card_fixture(name: &str, source: &str) -> PackageCard {
        archel_core::packages::PackageCard {
            name: name.to_string(),
            candidate_version: "1-1".to_string(),
            installed_version: None,
            source: source.to_string(),
            download_size: None,
            installed_size: None,
        }
    }

    #[test]
    fn install_guard_redirects_all_aur_lists_with_every_name() {
        // G-03-1 mirror of the aur->repo redirect: every card AUR means
        // the installs path never runs, and the redirect names each one.
        archel_core::i18n::set_language("fr");
        let cards = vec![
            card_fixture("hello", "AUR"),
            card_fixture("yay-bin", "aur"),
        ];
        for card in &cards {
            assert!(
                archel_core::aur::is_aur_card(card),
                "predicate agreement, not copy matching: {card:?}"
            );
        }
        let names = aur_names_in_cards(&cards);
        assert_eq!(names, vec!["hello".to_string(), "yay-bin".to_string()]);
        assert!(archel_core::aur::is_aur_hint(&names[0]).contains("hello"));
    }

    #[test]
    fn install_guard_refuses_mixed_lists_listing_only_aur_names() {
        // Refuse-all: the repo card stays installable in principle, but
        // the list as given installs nothing — only AUR names redirect.
        archel_core::i18n::set_language("fr");
        let cards = vec![
            card_fixture("firefox", "extra"),
            card_fixture("hello", "AUR"),
        ];
        assert!(!archel_core::aur::is_aur_card(&cards[0]));
        assert!(archel_core::aur::is_aur_card(&cards[1]));
        let names = aur_names_in_cards(&cards);
        assert_eq!(names, vec!["hello".to_string()]);
        let note = archel_core::aur::is_aur_hint_list(&names);
        assert!(note.contains("hello"), "AUR names listed: {note}");
        assert!(!note.contains("firefox"), "repo names never listed: {note}");
    }

    #[test]
    fn install_guard_passes_repo_only_lists_through() {
        // Empty redirect set: the installs path (card + confirm +
        // stream) is unaffected for repo-only arguments.
        let cards = vec![
            card_fixture("firefox", "extra"),
            card_fixture("vim", "core"),
        ];
        for card in &cards {
            assert!(
                !archel_core::aur::is_aur_card(card),
                "repo cards never trigger the guard: {card:?}"
            );
        }
        assert!(
            aur_names_in_cards(&cards).is_empty(),
            "repo-only lists reach the installs path"
        );
    }
}
