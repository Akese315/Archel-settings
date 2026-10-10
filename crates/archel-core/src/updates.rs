//! Pending-update counts (LAND-03): `checkupdates` (repo) + `yay -Qu` (AUR).
//!
//! Mirrors the `privileged.rs` spawn/parse shape exactly: `which` gate on
//! each checker binary, `duct` with the absolute path plus stdout/stderr
//! capture plus a 15 s timeout, stderr kept log-only, and stdout
//! interpreted by line-counting only (package-name text is never evaled,
//! never passed to a shell, and only numeric counts cross into output).
//!
//! Freshness follows D-05/D-06/D-07: counts persist in a versioned JSON
//! envelope under the XDG cache dir, valid for 15 minutes; any recompute
//! failure serves the last cached value with its age instead of an error;
//! `--refresh` skips the read path. A missing checker binary yields the
//! typed [`ArchelError::UpdatesCheckerMissing`] at the detection site and
//! degrades to [`UpdateCount::Unavailable`] — never a hard error. No
//! privilege is involved: both checkers run as the invoking user.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::ArchelError;

/// Cache envelope version. Any file with another `v` is discarded and
/// recomputed, never crashed on (T-2-01).
pub const CACHE_VERSION: u32 = 1;

/// D-05: recomputation runs at most once per 15 minutes.
pub const CACHE_TTL_SECS: u64 = 900;

/// Upper bound per checker subprocess (offline must fail fast, D-06).
pub const CHECK_TIMEOUT: Duration = Duration::from_secs(15);

/// Checker binary for the pacman repo half.
pub const CHECKUPDATES_TOOL: &str = "checkupdates";
/// Checker binary for the AUR half.
pub const YAY_TOOL: &str = "yay";

/// File name of the cache envelope inside the XDG cache dir.
pub const CACHE_FILE_NAME: &str = "updates.json";

/// Why one half of the update count has no value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum UnavailableReason {
    /// The checker binary is not installed (e.g. no `pacman-contrib`).
    CheckerMissing,
    /// The checker failed (non-zero exit, timeout, offline) and no cache
    /// exists to fall back on.
    RefreshFailed,
}

/// One half (repo or AUR) of the pending-update count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum UpdateCount {
    /// Recomputed (or served from a fresh cache) on this call.
    Fresh(u32),
    /// Served from an expired cache after a failed recompute (D-06).
    Stale { count: u32, age_secs: u64 },
    /// No value available; landing still renders everything else.
    Unavailable(UnavailableReason),
}

impl UpdateCount {
    /// Numeric value when there is one (fresh or stale).
    pub fn value(self) -> Option<u32> {
        match self {
            Self::Fresh(n) | Self::Stale { count: n, .. } => Some(n),
            Self::Unavailable(_) => None,
        }
    }
}

/// Both halves of the pending-update count. Each half degrades
/// independently: on this machine `checkupdates` is absent while `yay`
/// works, so repo is `Unavailable` while AUR is `Fresh`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingCounts {
    pub repo: UpdateCount,
    pub aur: UpdateCount,
}

/// Versioned cache envelope. `deny_unknown_fields` keeps the re-parse
/// strict: anything unexpected is discarded and recomputed (T-2-01).
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct UpdateCache {
    v: u32,
    computed_at: u64,
    repo: Option<u32>,
    aur: Option<u32>,
}

/// Fresh while `now - computed_at` stays under the TTL. Pure so the
/// expired-versus-fresh boundary is unit-tested on a fixture clock.
pub fn ttl_valid(computed_at: u64, now: u64) -> bool {
    now.saturating_sub(computed_at) < CACHE_TTL_SECS
}

/// Current Unix time in seconds (0 when the clock is unavailable).
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// XDG cache file for the envelope (`~/.cache/archel/updates.json`).
pub fn cache_file_path() -> PathBuf {
    directories::ProjectDirs::from("", "archel", "archel")
        .map(|dirs| dirs.cache_dir().join(CACHE_FILE_NAME))
        .unwrap_or_else(|| PathBuf::from(CACHE_FILE_NAME))
}

/// Reads and validates the envelope at `path`. Any failure (missing
/// file, unparseable JSON, version skew) is `None`: the caller recomputes
/// instead of crashing (T-2-01).
pub(crate) fn read_cache(path: &Path) -> Option<UpdateCache> {
    let raw = std::fs::read_to_string(path).ok()?;
    let cache: UpdateCache = serde_json::from_str(&raw).ok()?;
    if cache.v != CACHE_VERSION {
        return None;
    }
    Some(cache)
}

/// Persists the envelope atomically (temp file plus rename). Write
/// failures are swallowed: the fresh values are still returned, they
/// just are not cached for the next call.
pub(crate) fn write_cache(path: &Path, cache: &UpdateCache) {
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    let Ok(payload) = serde_json::to_string(cache) else {
        return;
    };
    let tmp = path.with_extension("tmp");
    if std::fs::write(&tmp, payload).is_err() {
        return;
    }
    let _ = std::fs::rename(&tmp, path);
}

/// Counts pending updates in checker stdout: one non-blank line per
/// package (A1). Nothing is parsed beyond line-counting: package names
/// and versions never cross into logic, only the numeric total does
/// (T-2-02).
pub fn count_package_lines(stdout: &str) -> u32 {
    stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count() as u32
}

/// Maps a checker exit code to pass/fail, mirroring the `map_pkexec_exit`
/// matrix shape. `checkupdates` exits 2 when the system is up to date
/// (A1: success with empty output, hence zero lines); any other
/// non-zero code, or a missing code (signal/timeout kill), is a refresh
/// failure that sends the caller down the stale-with-age path (D-06).
pub fn map_update_exit(tool: &str, code: Option<i32>) -> Result<(), ArchelError> {
    match (tool, code) {
        (_, Some(0)) => Ok(()),
        ("checkupdates", Some(2)) => Ok(()),
        _ => Err(ArchelError::Internal(format!(
            "{tool} refresh failed with exit code {code:?}"
        ))),
    }
}

/// Runs one checker (`checkupdates` with no args, `yay -Qu --aur`) and
/// returns its stdout. Missing binary is the typed
/// [`ArchelError::UpdatesCheckerMissing`]; every other failure is an
/// [`ArchelError::Internal`] log detail (the caller converts both into
/// degraded [`UpdateCount`] values, never a user-facing error).
fn run_checker(tool: &str, args: &[&str]) -> Result<String, ArchelError> {
    let binary = which::which(tool).map_err(|err| ArchelError::UpdatesCheckerMissing {
        tool: format!("{tool} ({err})"),
    })?;
    let handle = duct::cmd(&binary, args)
        .stdout_capture()
        .stderr_capture()
        // `.unchecked()` is load-bearing: without it duct turns any
        // non-zero status into an Err at wait time, so `checkupdates`
        // exit 2 (fully-up-to-date system) never reaches map_update_exit
        // and degrades to stale instead of Fresh(0). Same as
        // packages.rs::run_capped_query.
        .unchecked()
        .start()
        .map_err(|err| ArchelError::Internal(format!("cannot run '{tool}': {err}")))?;
    let output = match handle.wait_timeout(CHECK_TIMEOUT).map_err(|err| {
        ArchelError::Internal(format!("cannot wait on '{tool}': {err}"))
    })? {
        Some(output) => output.clone(),
        None => {
            let _ = handle.kill();
            return Err(ArchelError::Internal(format!(
                "'{tool}' exceeded the refresh timeout"
            )));
        }
    };
    // Log-only capture of checker stderr (kept for diagnostics; only the
    // numeric count below ever reaches the user).
    let _stderr_for_logs = String::from_utf8_lossy(&output.stderr);
    map_update_exit(tool, output.status.code())?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Refreshes one half. Checker failures become `Unavailable` with the
/// reason preserved so the renderer can pick the right hint.
fn refresh_half(tool: &str, args: &[&str]) -> UpdateCount {
    match run_checker(tool, args) {
        Ok(stdout) => UpdateCount::Fresh(count_package_lines(&stdout)),
        Err(ArchelError::UpdatesCheckerMissing { .. }) => {
            UpdateCount::Unavailable(UnavailableReason::CheckerMissing)
        }
        Err(_) => UpdateCount::Unavailable(UnavailableReason::RefreshFailed),
    }
}

/// Human age for a stale value, rendered from the catalog (D-06: the age
/// is always shown next to a cached number, never a bare number).
pub fn age_label(age_secs: u64) -> String {
    if age_secs < 90 {
        rust_i18n::t!("updates.age_moments").to_string()
    } else if age_secs < 5400 {
        rust_i18n::t!(
            "updates.age_minutes",
            minutes = age_secs.max(60) / 60
        )
        .to_string()
    } else {
        rust_i18n::t!("updates.age_hours", hours = age_secs / 3600).to_string()
    }
}

/// Guided hint naming the human install step for a missing checker.
/// `checkupdates` names the exact `pacman-contrib` command; any other
/// tool gets the generic hint.
pub fn checker_missing_hint(tool: &str) -> String {
    if tool == CHECKUPDATES_TOOL {
        rust_i18n::t!("updates.checker_missing_checkupdates").to_string()
    } else {
        rust_i18n::t!("updates.checker_missing_generic", tool = tool).to_string()
    }
}

/// Full resolution against the cache at `path` (D-05/D-06/D-07):
/// a fresh cache is served; anything else (missing, expired, corrupt,
/// `--refresh`) recomputes both halves live. Halves that fail fall back
/// to the last cached value with its age (stale, never a bare number);
/// halves with no cached value stay `Unavailable`. A partial refresh
/// keeps the previous timestamp so stale numbers never look rejuvenated.
pub fn pending_counts_at(path: &Path, force_refresh: bool) -> PendingCounts {
    pending_counts_with(path, force_refresh, refresh_half)
}

/// Resolution with an injectable recompute step so fixture-clock tests
/// cover expired-versus-fresh, stale fallback, refresh bypass, and
/// corrupt-envelope recompute without spawning live checkers.
fn pending_counts_with(
    path: &Path,
    force_refresh: bool,
    refresh: impl Fn(&str, &[&str]) -> UpdateCount,
) -> PendingCounts {
    let now = now_secs();
    let cached = if force_refresh {
        None
    } else {
        read_cache(path)
    };
    if let Some(cache) = cached.as_ref() {
        if ttl_valid(cache.computed_at, now) {
            return PendingCounts {
                repo: refresh_or_cached(cache.repo, CHECKUPDATES_TOOL, &[], &refresh),
                aur: refresh_or_cached(cache.aur, YAY_TOOL, &["-Qu", "--aur"], &refresh),
            };
        }
    }
    let fresh_repo = refresh(CHECKUPDATES_TOOL, &[]);
    let fresh_aur = refresh(YAY_TOOL, &["-Qu", "--aur"]);
    let prev_repo = cached.as_ref().and_then(|c| c.repo);
    let prev_aur = cached.as_ref().and_then(|c| c.aur);
    let prev_at = cached.as_ref().map(|c| c.computed_at).unwrap_or(now);
    let age_secs = now.saturating_sub(prev_at);
    let repo = with_stale_fallback(fresh_repo, prev_repo, age_secs);
    let aur = with_stale_fallback(fresh_aur, prev_aur, age_secs);
    let both_fresh = matches!(fresh_repo, UpdateCount::Fresh(_))
        && matches!(fresh_aur, UpdateCount::Fresh(_));
    let merged = UpdateCache {
        v: CACHE_VERSION,
        computed_at: if both_fresh { now } else { prev_at },
        repo: repo.value(),
        aur: aur.value(),
    };
    // Only persist when at least one half has a value: an all-empty
    // record would freeze a transient failure (offline) into the cache
    // instead of retrying on the next call.
    if merged.repo.is_some() || merged.aur.is_some() {
        write_cache(path, &merged);
    }
    PendingCounts { repo, aur }
}

/// D-06: a failed refresh serves the last cached value with its age;
/// without any cached value the half stays `Unavailable`.
fn with_stale_fallback(
    fresh: UpdateCount,
    cached: Option<u32>,
    age_secs: u64,
) -> UpdateCount {
    match fresh {
        UpdateCount::Fresh(_) => fresh,
        UpdateCount::Unavailable(_) => match cached {
            Some(count) => UpdateCount::Stale { count, age_secs },
            None => fresh,
        },
        stale @ UpdateCount::Stale { .. } => stale,
    }
}

/// Serves one cached half when the cache is fresh: a cached number is
/// `Fresh`; a cached miss (`None`, e.g. checker absent at cache time) is
/// retried live in case the machine changed since.
fn refresh_or_cached(
    cached: Option<u32>,
    tool: &str,
    args: &[&str],
    refresh: &impl Fn(&str, &[&str]) -> UpdateCount,
) -> UpdateCount {
    match cached {
        Some(n) => UpdateCount::Fresh(n),
        None => refresh(tool, args),
    }
}

/// Entry point: resolution against the default XDG cache file.
pub fn pending_counts(force_refresh: bool) -> PendingCounts {
    pending_counts_at(&cache_file_path(), force_refresh)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_one_non_blank_line_per_package() {
        let out = "firefox 1.0-1 -> 1.1-1\nvim 9.0-1 -> 9.1-1 [1d]\n";
        assert_eq!(count_package_lines(out), 2);
    }

    #[test]
    fn blank_lines_are_not_packages() {
        assert_eq!(count_package_lines(""), 0);
        assert_eq!(count_package_lines("\n  \n\t\n"), 0);
        assert_eq!(
            count_package_lines("pkg 1-1 -> 2-1\n\n   \n(pkg2 1-1 -> 2-1)\n"),
            2
        );
    }

    #[test]
    fn update_exit_zero_means_count_stdout() {
        assert!(map_update_exit("checkupdates", Some(0)).is_ok());
        assert!(map_update_exit("yay", Some(0)).is_ok());
    }

    #[test]
    fn checkupdates_exit_two_means_clean_zero() {
        // A1: up-to-date system, empty output, zero lines counted.
        assert!(map_update_exit("checkupdates", Some(2)).is_ok());
    }

    #[test]
    fn other_codes_mean_refresh_failure() {
        for (tool, code) in [
            ("checkupdates", Some(1)),
            ("checkupdates", None),
            ("yay", Some(1)),
            ("yay", Some(2)),
            ("yay", None),
        ] {
            let err = map_update_exit(tool, code)
                .expect_err("unexpected code must fail the refresh");
            assert!(
                matches!(err, ArchelError::Internal(_)),
                "wrong variant for {tool} code {code:?}"
            );
        }
    }

    #[test]
    fn missing_binary_maps_to_typed_checker_missing() {
        let err = run_checker("archel-no-such-tool", &[])
            .expect_err("unknown binary must fail the which gate");
        assert!(
            matches!(err, ArchelError::UpdatesCheckerMissing { .. }),
            "wrong variant: {err:?}"
        );
    }

    #[test]
    fn exit_two_spawn_reaches_exit_mapping() {
        // Regression for the deferred 03-01 task-3 finding: without
        // `.unchecked()` on the run_checker spawn, duct converts any
        // non-zero status into an Err at wait_timeout, so a fully
        // up-to-date system (checkupdates exit 2) never reaches
        // map_update_exit's Ok arm and the repo half degrades to
        // Unavailable instead of Fresh(0). This mirrors run_checker's
        // exact spawn/wait shape for a benign exit-2 command.
        let handle = duct::cmd("sh", ["-c", "exit 2"])
            .stdout_capture()
            .stderr_capture()
            .unchecked()
            .start()
            .expect("sh must be runnable");
        let output = handle
            .wait_timeout(CHECK_TIMEOUT)
            .expect("wait must not error")
            .expect("sh exits well within the timeout");
        assert_eq!(output.status.code(), Some(2));
        // The mapping arm itself: exit 2 from checkupdates is success.
        map_update_exit("checkupdates", Some(2)).expect("checkupdates exit 2 is Fresh(0), not a failure");
    }

    #[test]
    fn ttl_boundary_is_fifteen_minutes() {
        let now = 1_000_000;
        assert!(ttl_valid(now - 899, now));
        assert!(!ttl_valid(now - 900, now));
        assert!(!ttl_valid(now - 3600, now));
    }

    #[test]
    fn cache_envelope_round_trips_through_fixture_file() {
        let dir = std::env::temp_dir().join("archel-updates-test-roundtrip");
        let path = dir.join("updates.json");
        let _ = std::fs::remove_dir_all(&dir);
        let cache = UpdateCache {
            v: CACHE_VERSION,
            computed_at: 1_700_000_000,
            repo: Some(3),
            aur: Some(2),
        };
        write_cache(&path, &cache);
        assert_eq!(read_cache(&path), Some(cache));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn version_skew_is_discarded_never_crashed_on() {
        let dir = std::env::temp_dir().join("archel-updates-test-skew");
        let path = dir.join("updates.json");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("fixture dir must be creatable");
        std::fs::write(&path, r#"{"v":99,"computed_at":1,"repo":1,"aur":1}"#)
            .expect("fixture must be writable");
        assert_eq!(read_cache(&path), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn age_labels_come_from_the_catalog() {
        rust_i18n::set_locale("fr");
        assert!(!age_label(10).is_empty());
        assert!(age_label(600).contains("10"));
        assert!(age_label(7200).contains('2'));
    }

    #[test]
    fn missing_hint_names_the_human_install_step() {
        rust_i18n::set_locale("fr");
        let hint = checker_missing_hint(CHECKUPDATES_TOOL);
        assert!(
            hint.contains("pacman-contrib"),
            "hint must name the install step: {hint}"
        );
        assert!(
            checker_missing_hint("yay").contains("yay"),
            "generic hint must name the tool"
        );
    }

    // --- D-05/D-06/D-07 fixture-clock tests (no live checkers) ---

    fn fixture_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("fixture dir must be creatable");
        dir
    }

    fn write_envelope(path: &Path, computed_at: u64, repo: Option<u32>, aur: Option<u32>) {
        let cache = UpdateCache {
            v: CACHE_VERSION,
            computed_at,
            repo,
            aur,
        };
        write_cache(path, &cache);
    }

    fn fail_refresh(_tool: &str, _args: &[&str]) -> UpdateCount {
        UpdateCount::Unavailable(UnavailableReason::RefreshFailed)
    }

    #[test]
    fn fresh_cache_is_served_without_recompute() {
        let dir = fixture_dir("archel-updates-test-fresh");
        let path = dir.join("updates.json");
        let now = now_secs();
        write_envelope(&path, now - 60, Some(7), Some(5));
        let counts = pending_counts_with(&path, false, |_, _| {
            panic!("fresh cache must not recompute")
        });
        assert_eq!(counts.repo, UpdateCount::Fresh(7));
        assert_eq!(counts.aur, UpdateCount::Fresh(5));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn expired_cache_falls_back_to_stale_with_age() {
        let dir = fixture_dir("archel-updates-test-stale");
        let path = dir.join("updates.json");
        let now = now_secs();
        write_envelope(&path, now - 1800, Some(7), Some(5));
        let counts = pending_counts_with(&path, false, fail_refresh);
        match counts.repo {
            UpdateCount::Stale { count, age_secs } => {
                assert_eq!(count, 7);
                assert!(
                    (1790..=1810).contains(&age_secs),
                    "age must track the expired timestamp: {age_secs}"
                );
            }
            other => panic!("repo must go stale, got {other:?}"),
        }
        assert!(matches!(
            counts.aur,
            UpdateCount::Stale { count: 5, .. }
        ));
        // The failed refresh must not rejuvenate the timestamp (D-06).
        let kept = read_cache(&path).expect("cache must survive");
        assert_eq!(kept.computed_at, now - 1800);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn partial_success_mixes_fresh_and_stale() {
        let dir = fixture_dir("archel-updates-test-partial");
        let path = dir.join("updates.json");
        let now = now_secs();
        write_envelope(&path, now - 2000, Some(7), Some(5));
        let counts = pending_counts_with(&path, false, |tool, _| {
            if tool == CHECKUPDATES_TOOL {
                UpdateCount::Fresh(9)
            } else {
                UpdateCount::Unavailable(UnavailableReason::RefreshFailed)
            }
        });
        assert_eq!(counts.repo, UpdateCount::Fresh(9));
        assert!(matches!(
            counts.aur,
            UpdateCount::Stale { count: 5, .. }
        ));
        let merged = read_cache(&path).expect("partial success must persist");
        assert_eq!((merged.repo, merged.aur), (Some(9), Some(5)));
        assert_eq!(merged.computed_at, now - 2000, "stale half keeps its age");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refresh_bypasses_a_fresh_cache() {
        let dir = fixture_dir("archel-updates-test-bypass");
        let path = dir.join("updates.json");
        write_envelope(&path, now_secs() - 10, Some(1), Some(1));
        let counts = pending_counts_with(&path, true, |_, _| UpdateCount::Fresh(42));
        assert_eq!(counts.repo, UpdateCount::Fresh(42));
        assert_eq!(counts.aur, UpdateCount::Fresh(42));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_cache_recomputes_without_crashing() {
        let dir = fixture_dir("archel-updates-test-corrupt");
        let path = dir.join("updates.json");
        std::fs::write(&path, "{tampered,,,}").expect("fixture must be writable");
        let counts = pending_counts_with(&path, false, fail_refresh);
        assert_eq!(
            counts.repo,
            UpdateCount::Unavailable(UnavailableReason::RefreshFailed)
        );
        assert_eq!(
            counts.aur,
            UpdateCount::Unavailable(UnavailableReason::RefreshFailed)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn total_failure_writes_no_cache_and_stays_unavailable() {
        let dir = fixture_dir("archel-updates-test-nocache");
        let path = dir.join("updates.json");
        let counts = pending_counts_with(&path, false, fail_refresh);
        assert_eq!(
            counts.repo,
            UpdateCount::Unavailable(UnavailableReason::RefreshFailed)
        );
        assert!(
            !path.exists(),
            "an all-empty record must not freeze the failure"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
