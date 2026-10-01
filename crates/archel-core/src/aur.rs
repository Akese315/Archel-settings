//! AUR review-before-build core (PKG-05, D-05/D-06).
//!
//! The costly-reversibility trust boundary (D-05): before anything
//! builds, Archel fetches the PKGBUILD with `yay -G` into a temp dir as
//! the invoking user (never elevated, D-06), gates it through `bash -n`
//! plus greppable danger-pattern heuristics, and hands the FULL text to
//! the CLI for display. Archel never executes or sources the file; the
//! build itself runs later as the user via `yay -S` (task 2), never as
//! root.
//!
//! Trust signals (maintainer, votes, popularity, out-of-date) and the
//! dependency preview come from `yay -Si` fields under `LC_ALL=C` —
//! never from `yay -S --print`, which fails on AUR targets (Pitfall 5).
//! Every spawn reuses the `updates.rs` which-gated duct shape: a missing
//! `yay` is the typed [`ArchelError::YayMissing`] with the catalog hint.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Serialize;

use crate::error::ArchelError;
use crate::packages::{self, PackageCard};

/// AUR helper binary: always spawned as the invoking user (D-06), never
/// through pkexec or any Archel-orchestrated elevation.
pub const YAY_TOOL: &str = "yay";

/// What the user decided at the review gate. The build closure only runs
/// through [`apply_decision`], so the abort path structurally spawns
/// nothing (unit-covered: the closure carries a spawn probe).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AurDecision {
    /// The user validated the displayed PKGBUILD: run the build closure.
    Proceed,
    /// The user declined (or validation failed upstream): build nothing.
    Abort,
}

/// Runs the build closure only on [`AurDecision::Proceed`]. Abort maps to
/// the typed [`ArchelError::ConfirmationAborted`] — the caller mutates
/// nothing on that path.
pub fn apply_decision(
    decision: AurDecision,
    build: impl FnOnce() -> Result<String, ArchelError>,
) -> Result<String, ArchelError> {
    match decision {
        AurDecision::Proceed => build(),
        AurDecision::Abort => Err(ArchelError::ConfirmationAborted),
    }
}

/// AUR trust signals from `yay -Si` fields (all display strings: raw
/// `-Si` text is untrusted per T-3-11, shown verbatim, never evaled).
/// Missing signals stay `None` (honest N/A), never zero-filled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TrustSignals {
    pub maintainer: Option<String>,
    pub votes: Option<String>,
    pub popularity: Option<String>,
    pub out_of_date: Option<String>,
}

/// Parses the trust signals from `LC_ALL=C yay -Si` fields. Field names
/// verified live (`Maintainer/Votes/Popularity/Out-of-date`).
pub fn parse_trust_signals(fields: &HashMap<String, String>) -> TrustSignals {
    let signal = |key: &str| {
        fields
            .get(key)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("none"))
    };
    TrustSignals {
        maintainer: signal("Maintainer"),
        votes: signal("Votes"),
        popularity: signal("Popularity"),
        out_of_date: signal("Out-of-date"),
    }
}

/// Parses the AUR dependency preview from `yay -Si` `Depends On` plus
/// `Make Deps` fields (Pitfall 5: `yay -S --print` fails on AUR names,
/// so the preview never touches it). `None`/blank closures contribute
/// nothing — the preview stays honestly empty.
pub fn parse_aur_dependencies(fields: &HashMap<String, String>) -> Vec<String> {
    let mut deps = Vec::new();
    for key in ["Depends On", "Make Deps"] {
        if let Some(raw) = fields.get(key) {
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
                continue;
            }
            deps.extend(trimmed.split_whitespace().map(str::to_string));
        }
    }
    deps
}

/// The complete pre-build review handed to the CLI for display (D-05):
/// the D-04 card, the FULL PKGBUILD text, trust signals, the dependency
/// preview, and heuristic warnings. Field names are the v1 JSON schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AurReview {
    pub card: PackageCard,
    pub pkgbuild: String,
    pub trust: TrustSignals,
    pub dependencies: Vec<String>,
    pub warnings: Vec<String>,
}

/// True when the card comes from AUR (`yay -Si` reports `Repository`
/// as `aur` in lowercase — verified live; the display normalizes it).
pub fn is_aur_card(card: &PackageCard) -> bool {
    card.source.eq_ignore_ascii_case("aur")
}

/// Resolves one AUR-flow tool. A missing binary aborts the AUR path with
/// the typed [`ArchelError::YayMissing`] (yay is the only which-gated
/// binary here — `bash` falls back to its absolute path below).
fn resolve_tool(tool: &str) -> Result<PathBuf, ArchelError> {
    which::which(tool).map_err(|_| ArchelError::YayMissing)
}

/// Scratch dir for one AUR fetch, scoped by PID plus package name. Names
/// pass the strict charset gate before reaching this path, so no
/// traversal or shell metacharacter can ride along.
fn aur_workdir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("archel-aur-{}-{name}", std::process::id()))
}

/// Unique scratch dir for one validation probe. Validation runs
/// repeatedly (and under `cargo test`, concurrently), so the probe path
/// carries a process-wide counter — a shared path would race between
/// the write and the `bash -n` read.
fn probe_workdir() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static PROBE_SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = PROBE_SEQ.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!(
        "archel-aur-{}-validate-{seq}",
        std::process::id()
    ))
}

/// Runs one yay query spawn (`-G` fetch, `-Si` fields) with the
/// `updates.rs` shape: `LC_ALL=C` pin (T-3-01), stdout/stderr capture,
/// stderr log-only. Non-zero exits are the caller's to map.
fn run_yay(args: &[&str], workdir: Option<&PathBuf>) -> Result<std::process::Output, ArchelError> {
    let yay = resolve_tool(YAY_TOOL)?;
    let mut cmd = duct::cmd(&yay, args);
    cmd = cmd.unchecked().env("LC_ALL", "C");
    if let Some(dir) = workdir {
        cmd = cmd.dir(dir);
    }
    let handle = cmd
        .stdout_capture()
        .stderr_capture()
        .start()
        .map_err(|err| ArchelError::Internal(format!("cannot run 'yay': {err}")))?;
    match handle
        .wait_timeout(packages::QUERY_TIMEOUT)
        .map_err(|err| ArchelError::Internal(format!("cannot wait on 'yay': {err}")))? {
        Some(output) => Ok(output.clone()),
        None => {
            let _ = handle.kill();
            Err(ArchelError::Internal(
                "'yay' exceeded the query timeout".to_string(),
            ))
        }
    }
}

/// Fetches the PKGBUILD for `name` with `yay -G` into a fresh temp dir
/// (verified live: clones `<name>/PKGBUILD` under the working dir, no
/// build, runs as the user). Returns the workdir plus the FULL file text
/// for CLI display. The name is validated first; the card having already
/// resolved means fetch failures are environmental ([`ArchelError::Internal`]).
fn fetch_pkgbuild(name: &str) -> Result<(PathBuf, String), ArchelError> {
    packages::validate_package_names(&[name.to_string()])?;
    let dir = aur_workdir(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)
        .map_err(|err| ArchelError::Internal(format!("cannot create AUR workdir: {err}")))?;
    let output = run_yay(&["-G", name], Some(&dir))?;
    // Log-only capture of yay stderr (untrusted text, never terminal).
    let _stderr_for_logs = String::from_utf8_lossy(&output.stderr);
    if output.status.code() != Some(0) {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(ArchelError::Internal(format!(
            "'yay -G {name}' failed with exit code {:?}",
            output.status.code()
        )));
    }
    let text = std::fs::read_to_string(dir.join(name).join("PKGBUILD")).map_err(|err| {
        let _ = std::fs::remove_dir_all(&dir);
        ArchelError::Internal(format!("cannot read fetched PKGBUILD for '{name}': {err}"))
    })?;
    Ok((dir, text))
}

/// Validates PKGBUILD text without ever executing it: the `bash -n`
/// static gate (hard reject → [`ArchelError::PkgbuildRejected`]) plus
/// greppable danger-pattern heuristics (warnings, never rejects — the
/// user validates per D-05). Returns the warning list on success.
pub fn validate_pkgbuild_source(name: &str, text: &str) -> Result<Vec<String>, ArchelError> {
    let dir = probe_workdir();
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)
        .map_err(|err| ArchelError::Internal(format!("cannot create AUR workdir: {err}")))?;
    let probe = dir.join("PKGBUILD.probe");
    std::fs::write(&probe, text)
        .map_err(|err| ArchelError::Internal(format!("cannot stage PKGBUILD probe: {err}")))?;
    // `bash` by PATH with the absolute fallback: a missing bash is an
    // environmental Internal, never the yay hint.
    let bash = which::which("bash").unwrap_or_else(|_| PathBuf::from("/usr/bin/bash"));
    // `-n` FIRST: `bash <file> -n` would execute the file, `bash -n
    // <file>` only checks it. The order here is the trust boundary.
    let mut argv = vec![std::ffi::OsString::from("-n")];
    argv.push(probe.as_os_str().to_os_string());
    let output = duct::cmd(&bash, argv)
        .unchecked()
        .env("LC_ALL", "C")
        .stdout_capture()
        .stderr_capture()
        .run()
        .map_err(|err| ArchelError::Internal(format!("cannot run 'bash -n': {err}")))?;
    let _ = std::fs::remove_dir_all(&dir);
    if output.status.code() != Some(0) {
        let detail = String::from_utf8_lossy(&output.stderr);
        let first_line = detail.lines().next().unwrap_or("syntax error").trim();
        let reason: String = first_line.chars().take(200).collect();
        return Err(ArchelError::PkgbuildRejected {
            name: name.to_string(),
            reason,
        });
    }
    Ok(danger_warnings(text))
}

/// Greppable danger-pattern heuristics over PKGBUILD text (comment lines
/// skipped). Each hit becomes a `pattern: line` warning for the review
/// display — heuristic signal for the human, never an automatic reject.
fn danger_warnings(text: &str) -> Vec<String> {
    let mut warnings = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        let lower = trimmed.to_lowercase();
        let piped_shell = (lower.contains("curl") || lower.contains("wget"))
            && (lower.contains("| sh")
                || lower.contains("|sh")
                || lower.contains("| bash")
                || lower.contains("|bash"));
        if piped_shell {
            warnings.push(format!("pipe-to-shell: {trimmed}"));
        }
        if lower.contains("sudo ") || lower.ends_with("sudo") {
            warnings.push(format!("sudo-in-build: {trimmed}"));
        }
        if forced_remove(&lower) {
            warnings.push(format!("forced-remove: {trimmed}"));
        }
    }
    warnings
}

/// Matches destructive `rm -rf` shapes (`/`, `$vars`, `~`) without
/// flagging ordinary `rm -rf build/` cleanups.
fn forced_remove(lower: &str) -> bool {
    let Some(rest) = lower.split_once("rm -rf") else {
        return false;
    };
    let (_, target) = rest;
    let target = target.trim_start();
    target.starts_with('/')
        || target.starts_with("${")
        || target.starts_with("$(")
        || (target.starts_with('$')
            && target
                .chars()
                .nth(1)
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_'))
        || target.starts_with('~')
        || target.contains("--no-preserve-root")
}

/// Fetches `yay -Si` fields for trust signals plus the dep preview. The
/// card already proved the name, so failures here are environmental.
fn aur_info_fields(name: &str) -> Result<HashMap<String, String>, ArchelError> {
    let output = run_yay(&["-Si", name], None)?;
    let _stderr_for_logs = String::from_utf8_lossy(&output.stderr);
    if output.status.code() != Some(0) {
        return Err(ArchelError::Internal(format!(
            "'yay -Si {name}' failed with exit code {:?}",
            output.status.code()
        )));
    }
    Ok(packages::parse_info_fields(
        &String::from_utf8_lossy(&output.stdout),
    ))
}

/// Composes the full pre-build review (D-05): honest not-found via the
/// card, PKGBUILD fetch plus validation, trust signals, dep preview.
/// The temp workdir is removed before returning — the CLI displays from
/// the in-memory text, leaving no residue on disk.
pub fn review_package(name: &str) -> Result<AurReview, ArchelError> {
    let card = packages::package_card(name)?;
    let (workdir, pkgbuild) = fetch_pkgbuild(name)?;
    let warnings = match validate_pkgbuild_source(name, &pkgbuild) {
        Ok(warnings) => warnings,
        Err(err) => {
            let _ = std::fs::remove_dir_all(&workdir);
            return Err(err);
        }
    };
    let _ = std::fs::remove_dir_all(&workdir);
    let fields = aur_info_fields(name)?;
    Ok(AurReview {
        card,
        pkgbuild,
        trust: parse_trust_signals(&fields),
        dependencies: parse_aur_dependencies(&fields),
        warnings,
    })
}

/// Review title echoing the package name, from the catalog.
pub fn review_title(name: &str) -> String {
    rust_i18n::t!("aur.review_title", name = name).to_string()
}

/// Trust-signal row labels, from the catalog.
pub struct TrustLabels {
    pub maintainer: String,
    pub votes: String,
    pub popularity: String,
    pub out_of_date: String,
}

/// Collects the four trust-signal labels in one call.
pub fn trust_labels() -> TrustLabels {
    TrustLabels {
        maintainer: rust_i18n::t!("aur.trust_maintainer").to_string(),
        votes: rust_i18n::t!("aur.trust_votes").to_string(),
        popularity: rust_i18n::t!("aur.trust_popularity").to_string(),
        out_of_date: rust_i18n::t!("aur.trust_out_of_date").to_string(),
    }
}

/// AUR dependency-preview section label, from the catalog.
pub fn aur_dependencies_label() -> String {
    rust_i18n::t!("aur.preview_dependencies").to_string()
}

/// Full-PKGBUILD display header, from the catalog.
pub fn pkgbuild_title() -> String {
    rust_i18n::t!("aur.pkgbuild_title").to_string()
}

/// Heuristic-warnings section header, from the catalog.
pub fn warnings_title() -> String {
    rust_i18n::t!("aur.warnings_title").to_string()
}

/// Post-review build confirmation prompt, from the catalog.
pub fn confirm_aur_prompt(name: &str) -> String {
    rust_i18n::t!("aur.confirm_build", name = name).to_string()
}

/// AUR build success line, from the catalog.
pub fn aur_ok_line(name: &str) -> String {
    rust_i18n::t!("aur.build_ok", name = name).to_string()
}

/// Hint shown when the reviewed package comes from the repos instead of
/// AUR (directs to `pkg install`, builds nothing).
pub fn not_aur_hint(name: &str) -> String {
    rust_i18n::t!("aur.not_aur_hint", name = name).to_string()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::*;
    use crate::ArchelError;

    #[test]
    fn abort_decision_returns_the_abort_variant_without_spawning_a_build() {
        let builds = Arc::new(AtomicUsize::new(0));
        let probe = Arc::clone(&builds);
        let err = apply_decision(AurDecision::Abort, move || {
            probe.fetch_add(1, Ordering::SeqCst);
            Ok(String::new())
        })
        .expect_err("abort must never build");
        assert!(
            matches!(
                err,
                ArchelError::ConfirmationAborted | ArchelError::PkgbuildRejected { .. }
            ),
            "abort needs the typed variant, never a build: {err:?}"
        );
        assert_eq!(
            builds.load(Ordering::SeqCst),
            0,
            "the abort path spawns no build process"
        );
    }

    #[test]
    fn proceed_decision_runs_the_build_exactly_once() {
        let builds = Arc::new(AtomicUsize::new(0));
        let probe = Arc::clone(&builds);
        let out = apply_decision(AurDecision::Proceed, move || {
            probe.fetch_add(1, Ordering::SeqCst);
            Ok("built".to_string())
        })
        .expect("proceed must build");
        assert_eq!(out, "built");
        assert_eq!(builds.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn invalid_bash_maps_to_the_typed_rejection_with_french() {
        crate::i18n::set_language("fr");
        let err = validate_pkgbuild_source("some-aur-pkg", "if then\n  echo broken\n")
            .expect_err("bash -n must reject the broken PKGBUILD");
        assert!(
            matches!(err, ArchelError::PkgbuildRejected { .. }),
            "validation failure needs the typed rejection: {err:?}"
        );
        let rendered = err.message();
        assert!(
            !rendered.trim().is_empty() && rendered.contains("some-aur-pkg"),
            "rejection must render catalog French naming the package: {rendered}"
        );
    }

    #[test]
    fn danger_patterns_surface_as_warnings_not_rejects() {
        let text = "pkgname=x\npkgver=1\nbuild() {\n  curl https://example.com/a.sh | sh\n}\n";
        let warnings =
            validate_pkgbuild_source("some-aur-pkg", text).expect("heuristics warn, they never reject");
        assert!(
            warnings.iter().any(|w| w.contains("pipe-to-shell")),
            "curl-pipe-to-shell must be flagged: {warnings:?}"
        );
        let text = "package() {\n  sudo make install\n}\n";
        let warnings =
            validate_pkgbuild_source("some-aur-pkg", text).expect("heuristics warn, they never reject");
        assert!(
            warnings.iter().any(|w| w.contains("sudo-in-build")),
            "sudo in a PKGBUILD must be flagged: {warnings:?}"
        );
    }

    #[test]
    fn forced_remove_flags_roots_and_vars_not_build_dirs() {
        assert!(forced_remove("rm -rf /"));
        assert!(forced_remove("rm -rf / --no-preserve-root"));
        assert!(forced_remove("rm -rf ${pkgdir}/etc"));
        assert!(forced_remove("rm -rf $HOME/.config"));
        assert!(!forced_remove("rm -rf build/"));
    }

    #[test]
    fn comment_lines_never_warn() {
        // Comment-skipping lives in `danger_warnings` (the shape matcher
        // itself stays comment-unaware by design).
        let text = "# rm -rf /\n# curl https://example.com/a.sh | sh\npkgname=x\n";
        let warnings = validate_pkgbuild_source("some-aur-pkg", text)
            .expect("comments are not code");
        assert!(
            warnings.is_empty(),
            "comment lines must not warn: {warnings:?}"
        );
    }

    #[test]
    fn aur_dep_preview_parses_depends_and_make_deps() {
        // `LC_ALL=C`-shaped `yay -Si` dependency fields (live shape):
        // space-separated atoms, `None` when the closure is empty.
        let mut fields = std::collections::HashMap::new();
        fields.insert(
            "Depends On".to_string(),
            "pacman>6.1  git".to_string(),
        );
        fields.insert("Make Deps".to_string(), "go".to_string());
        assert_eq!(
            parse_aur_dependencies(&fields),
            vec![
                "pacman>6.1".to_string(),
                "git".to_string(),
                "go".to_string()
            ],
            "Pitfall 5: AUR deps come from -Si fields, never -S --print"
        );
        let mut empty = std::collections::HashMap::new();
        empty.insert("Depends On".to_string(), "None".to_string());
        empty.insert("Make Deps".to_string(), "None".to_string());
        assert!(
            parse_aur_dependencies(&empty).is_empty(),
            "None closures stay empty, never a fake dep"
        );
    }

    #[test]
    fn trust_signals_read_maintainer_votes_popularity_out_of_date() {
        // Live `LC_ALL=C yay -Si yay-bin` shape (verified 2026-10-01).
        let mut fields = std::collections::HashMap::new();
        fields.insert("Maintainer".to_string(), "jguer".to_string());
        fields.insert("Votes".to_string(), "369".to_string());
        fields.insert("Popularity".to_string(), "4.350966".to_string());
        fields.insert("Out-of-date".to_string(), "No".to_string());
        let trust = parse_trust_signals(&fields);
        assert_eq!(trust.maintainer.as_deref(), Some("jguer"));
        assert_eq!(trust.votes.as_deref(), Some("369"));
        assert_eq!(trust.popularity.as_deref(), Some("4.350966"));
        assert_eq!(trust.out_of_date.as_deref(), Some("No"));
        let bare = std::collections::HashMap::new();
        let trust = parse_trust_signals(&bare);
        assert!(
            trust.maintainer.is_none() && trust.votes.is_none(),
            "missing signals stay absent, never zero-filled"
        );
    }

    #[test]
    fn missing_yay_maps_to_the_typed_hint() {
        let err = resolve_tool("archel-no-such-tool")
            .expect_err("unknown binary must fail the which gate");
        assert!(
            matches!(err, ArchelError::YayMissing),
            "the AUR path stays typed on a missing yay: {err:?}"
        );
        crate::i18n::set_language("fr");
        assert!(
            !err.message().trim().is_empty(),
            "yay-missing renders the catalog hint"
        );
    }

    #[test]
    fn aur_source_matches_case_insensitively() {
        let aur_card = PackageCard {
            name: "x".to_string(),
            candidate_version: "1-1".to_string(),
            installed_version: None,
            source: "aur".to_string(),
            download_size: None,
            installed_size: None,
        };
        assert!(is_aur_card(&aur_card));
        let repo_card = PackageCard {
            source: "extra".to_string(),
            ..aur_card
        };
        assert!(!is_aur_card(&repo_card));
    }

    #[test]
    fn aur_labels_come_from_the_catalog() {
        crate::i18n::set_language("fr");
        assert!(!review_title("x").is_empty());
        let labels = trust_labels();
        assert!(!labels.maintainer.is_empty());
        assert!(!labels.votes.is_empty());
        assert!(!labels.popularity.is_empty());
        assert!(!labels.out_of_date.is_empty());
        assert!(!aur_dependencies_label().is_empty());
        assert!(!pkgbuild_title().is_empty());
        assert!(!warnings_title().is_empty());
        assert!(confirm_aur_prompt("x").contains('x'));
        assert!(aur_ok_line("x").contains('x'));
        assert!(not_aur_hint("x").contains('x'));
    }
}
