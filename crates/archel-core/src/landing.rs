//! Landing snapshot (LAND-01..03): app version plus kernel plus pending
//! updates in one struct.
//!
//! Composer shape (mirrors `version_line` in `lib.rs`): core builds the
//! snapshot and the catalog-rendered cells, the binary only lays them out.
//! The snapshot never fails because an update count is missing: halves
//! degrade to stale or unavailable inside [`crate::updates`], so `landing`
//! always renders version and kernel plus whatever the checkers reported.

use serde::Serialize;

use crate::error::ArchelError;
use crate::updates::{
    self, PendingCounts, UnavailableReason, UpdateCount, YAY_TOOL, CHECKUPDATES_TOOL,
};

/// One-glance home-page facts: version (LAND-01), kernel (LAND-02),
/// pending repo + AUR updates (LAND-03).
///
/// Field names are the v1 JSON schema (`--json` serializes this struct
/// directly); the schema is unstable until the GUI phase pins it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LandingSnapshot {
    pub app_version: String,
    pub kernel_version: String,
    pub repo: UpdateCount,
    pub aur: UpdateCount,
}

/// Composes [`crate::library_version`] plus the running kernel plus the
/// cached update counts. `force_refresh` bypasses the cache read (D-07).
/// The only hard failure is an unreadable kernel version; update halves
/// degrade inside [`crate::updates`] and never fail this call.
pub fn snapshot(force_refresh: bool) -> Result<LandingSnapshot, ArchelError> {
    let kernel_version = sysinfo::System::kernel_version().ok_or_else(|| {
        ArchelError::Internal("kernel version unavailable from sysinfo".to_string())
    })?;
    let PendingCounts { repo, aur } = updates::pending_counts(force_refresh);
    Ok(LandingSnapshot {
        app_version: crate::library_version().to_string(),
        kernel_version,
        repo,
        aur,
    })
}

/// Table labels from the catalog, so the binary lays out columns
/// without owning any French copy (bin-calls-core invariant).
pub struct TableLabels {
    pub title: String,
    pub version: String,
    pub kernel: String,
    pub updates: String,
}

/// Collects the four landing table labels in one call.
pub fn table_labels() -> TableLabels {
    TableLabels {
        title: rust_i18n::t!("landing.title").to_string(),
        version: rust_i18n::t!("landing.version").to_string(),
        kernel: rust_i18n::t!("landing.kernel").to_string(),
        updates: rust_i18n::t!("landing.updates").to_string(),
    }
}

/// Renders the update-count cell from the catalog: fresh breakdown, stale
/// breakdown with age (D-06), partial halves, or the honest unavailable
/// state. Pure over the snapshot, unit-tested below.
pub fn updates_cell(snap: &LandingSnapshot) -> String {
    let repo_n = snap.repo.value();
    let aur_n = snap.aur.value();
    let stale_age = [snap.repo, snap.aur]
        .into_iter()
        .filter_map(|half| match half {
            UpdateCount::Stale { age_secs, .. } => Some(age_secs),
            _ => None,
        })
        .max();
    let counts = match (repo_n, aur_n) {
        (Some(repo), Some(aur)) => rust_i18n::t!(
            "landing.updates_breakdown",
            total = repo + aur,
            repo = repo,
            aur = aur
        )
        .to_string(),
        (Some(repo), None) => {
            rust_i18n::t!("landing.updates_repo_only", repo = repo).to_string()
        }
        (None, Some(aur)) => rust_i18n::t!("landing.updates_aur_only", aur = aur).to_string(),
        (None, None) => rust_i18n::t!("landing.updates_unavailable").to_string(),
    };
    match stale_age {
        Some(age_secs) => rust_i18n::t!(
            "landing.updates_stale",
            counts = counts,
            age = updates::age_label(age_secs)
        )
        .to_string(),
        None => counts,
    }
}

/// Note lines printed under the table: one guided hint per missing
/// checker, plus a single refresh-failed line when a half has no cache
/// to fall back on. Empty when every half has a value.
pub fn checker_notes(snap: &LandingSnapshot) -> Vec<String> {
    let mut notes = Vec::new();
    for (half, tool) in [(snap.repo, CHECKUPDATES_TOOL), (snap.aur, YAY_TOOL)] {
        if half == UpdateCount::Unavailable(UnavailableReason::CheckerMissing) {
            notes.push(updates::checker_missing_hint(tool));
        }
    }
    if [snap.repo, snap.aur].into_iter().any(|half| {
        half == UpdateCount::Unavailable(UnavailableReason::RefreshFailed)
    }) {
        notes.push(rust_i18n::t!("updates.refresh_failed").to_string());
    }
    notes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap_with(repo: UpdateCount, aur: UpdateCount) -> LandingSnapshot {
        LandingSnapshot {
            app_version: "0.1.0".to_string(),
            kernel_version: "7.2.6-test".to_string(),
            repo,
            aur,
        }
    }

    #[test]
    fn fresh_snapshot_renders_the_breakdown() {
        rust_i18n::set_locale("fr");
        let cell = updates_cell(&snap_with(UpdateCount::Fresh(3), UpdateCount::Fresh(2)));
        assert!(cell.contains('5'), "total must show: {cell}");
        assert!(cell.contains('3') && cell.contains('2'), "halves: {cell}");
    }

    #[test]
    fn stale_snapshot_shows_counts_with_age() {
        rust_i18n::set_locale("fr");
        let cell = updates_cell(&snap_with(
            UpdateCount::Stale {
                count: 4,
                age_secs: 1800,
            },
            UpdateCount::Fresh(1),
        ));
        assert!(cell.contains('5'), "stale counts stay visible: {cell}");
        assert!(cell.contains("30"), "age in minutes must show: {cell}");
    }

    #[test]
    fn missing_repo_half_renders_aur_only() {
        rust_i18n::set_locale("fr");
        let snap = snap_with(
            UpdateCount::Unavailable(UnavailableReason::CheckerMissing),
            UpdateCount::Fresh(3),
        );
        let cell = updates_cell(&snap);
        assert!(cell.contains('3'), "AUR half stays visible: {cell}");
        let notes = checker_notes(&snap);
        assert_eq!(notes.len(), 1, "one guided hint expected");
        assert!(
            notes[0].contains("pacman-contrib"),
            "hint must name the install step: {}",
            notes[0]
        );
    }

    #[test]
    fn both_halves_missing_stays_honest() {
        rust_i18n::set_locale("fr");
        let snap = snap_with(
            UpdateCount::Unavailable(UnavailableReason::RefreshFailed),
            UpdateCount::Unavailable(UnavailableReason::RefreshFailed),
        );
        assert!(!updates_cell(&snap).is_empty());
        assert_eq!(checker_notes(&snap).len(), 1);
    }

    #[test]
    fn live_kernel_version_is_non_empty() {
        let kernel = sysinfo::System::kernel_version();
        assert!(
            kernel.is_some_and(|v| !v.trim().is_empty()),
            "LAND-02: kernel version must be readable on this machine"
        );
    }
}
