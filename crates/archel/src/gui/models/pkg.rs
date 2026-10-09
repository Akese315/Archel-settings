//! Packages view-model: installed list, live search and package card
//! (read half; plan 06 adds the install/remove/upgrade invokables to
//! this same file).
//!
//! Same proven shape as the other tab models: `tick()` is a QML-called
//! invokable that only spawns - the heavier `list_installed()` (alpm
//! localdb walk) runs on the gui runtime's blocking pool with a
//! sequence tag so a slow list never overwrites a newer one, and the
//! finished payload returns exclusively through `CxxQtThread.queue`,
//! whose closure is the only code allowed to set properties (RESEARCH
//! Pattern 1). Search and card selection follow the same spawn/queue
//! shape with per-query sequence tags (typing fast drops stale hits,
//! T-06-26). Every read goes through the CLI-proven core entry points
//! (`list_installed`, `search` with its core-side `sort_hits`,
//! `package_card` - the `run_list`/`run_search`/`run_info` twins);
//! no GUI-side sorting or field invention (PLAT-03).

use cxx_qt_lib::QString;

use super::super::poll::{BannerPolicy, PollGuard};
use crate::gui::models::system::NOT_AVAILABLE;

/// Fully formatted list payload, built off the Qt thread so the
/// `queue()` closure only sets properties.
pub(crate) struct AppliedPkgList {
    pub installed_json: String,
    pub banner_visible: bool,
}

/// Fully formatted search payload with its dispatch tag, so the
/// `queue()` closure can drop stale hits (T-06-26).
pub(crate) struct AppliedSearch {
    pub tag: u64,
    pub search_json: String,
    pub zero_hit: bool,
    pub busy: bool,
}

/// Fully formatted card payload with its dispatch tag.
pub(crate) struct AppliedCard {
    pub tag: u64,
    pub card_json: String,
}

/// Backing state: property fields plus the poll guard, the banner
/// policy and two plain sequence counters. The counters are only
/// touched on the Qt thread (`search()`/`select()` dispatch there and
/// the `queue()` closures run there), so no atomics are needed for
/// them; the guard covers the 1 Hz list tick.
pub struct PkgModelRust {
    pub(crate) installed_json: QString,
    pub(crate) search_json: QString,
    pub(crate) card_json: QString,
    pub(crate) search_busy: bool,
    pub(crate) zero_hit: bool,
    pub(crate) banner_visible: bool,
    guard: PollGuard,
    banner: BannerPolicy,
    search_seq: u64,
    card_seq: u64,
}

impl Default for PkgModelRust {
    fn default() -> Self {
        Self {
            installed_json: QString::from(""),
            search_json: QString::from(""),
            card_json: QString::from(""),
            search_busy: false,
            zero_hit: false,
            banner_visible: false,
            guard: PollGuard::new(),
            banner: BannerPolicy::new(),
            search_seq: 0,
            card_seq: 0,
        }
    }
}

/// Installed rows as QML-ready JSON: `[{name,version}]` in the order
/// the core list returns (its `render_list_table` twin chooses the
/// same columns).
pub(crate) fn installed_json_for(pkgs: &[archel_core::packages::PackageInfo]) -> String {
    let rows: Vec<serde_json::Value> = pkgs
        .iter()
        .map(|pkg| {
            serde_json::json!({
                "name": pkg.name,
                "version": pkg.version,
            })
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string())
}

/// Search hits as QML-ready JSON: `[{name,version,source}]` in the
/// core's `sort_hits` order (newest first, repo before AUR) - never
/// re-sorted here.
pub(crate) fn search_json_for(hits: &[archel_core::packages::PackageInfo]) -> String {
    let rows: Vec<serde_json::Value> = hits
        .iter()
        .map(|hit| {
            serde_json::json!({
                "name": hit.name,
                "version": hit.version,
                "source": hit.source.as_deref().unwrap_or("-"),
            })
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string())
}

/// Package card as QML-ready JSON: the core `PackageCard` fields
/// verbatim (name, candidate/installed versions, source, sizes), in
/// the `card_labels` field set - no GUI-side field invention.
pub(crate) fn card_json_for(card: &archel_core::packages::PackageCard) -> String {
    serde_json::to_string(card).unwrap_or_else(|_| "{}".to_string())
}

/// Honest not-found card: every value reads N/A, never a zero-filled
/// card (mirrors the CLI's typed not-found error).
pub(crate) fn na_card_json(name: &str) -> String {
    serde_json::json!({
        "name": name,
        "candidate_version": NOT_AVAILABLE,
        "installed_version": NOT_AVAILABLE,
        "source": NOT_AVAILABLE,
        "download_size": NOT_AVAILABLE,
        "installed_size": NOT_AVAILABLE,
    })
    .to_string()
}

impl PkgModelRust {
    /// Applies a finished list outcome on the Qt thread. `Err` (alpm
    /// localdb unreadable) reads the honest empty list plus the
    /// once-per-episode banner.
    fn apply_list(
        &mut self,
        outcome: Result<Vec<archel_core::packages::PackageInfo>, archel_core::ArchelError>,
    ) -> AppliedPkgList {
        match outcome {
            Ok(pkgs) => {
                self.banner.on_success();
                AppliedPkgList {
                    installed_json: installed_json_for(&pkgs),
                    banner_visible: false,
                }
            }
            Err(_) => {
                self.banner.on_failure();
                AppliedPkgList {
                    installed_json: "[]".to_string(),
                    banner_visible: self.banner.wants_visible(),
                }
            }
        }
    }

    /// Applies a finished search outcome. Only the newest tag applies
    /// (T-06-26 stale-drop); `zero_hit` arms the refinement copy only
    /// for a completed non-blank query with no hits.
    fn apply_search(
        &mut self,
        tag: u64,
        query_blank: bool,
        hits: Vec<archel_core::packages::PackageInfo>,
    ) -> Option<AppliedSearch> {
        if self.search_seq != tag {
            return None;
        }
        let zero_hit = !query_blank && hits.is_empty();
        Some(AppliedSearch {
            tag,
            search_json: search_json_for(&hits),
            zero_hit,
            busy: false,
        })
    }

    /// Applies a finished card outcome. Only the newest tag applies;
    /// `Err` (unknown name) reads the N/A card, never a zero-filled one.
    fn apply_card(
        &mut self,
        tag: u64,
        name: String,
        outcome: Result<archel_core::packages::PackageCard, archel_core::ArchelError>,
    ) -> Option<AppliedCard> {
        if self.card_seq != tag {
            return None;
        }
        Some(AppliedCard {
            tag,
            card_json: match outcome {
                Ok(card) => card_json_for(&card),
                Err(_) => na_card_json(&name),
            },
        })
    }
}

impl crate::gui::bridge_pkg::qobject::PkgModel {
    /// 1 Hz entry point: skips while a tick is in flight (D-02),
    /// spawns the heavier installed-list read, marshals back through
    /// `queue()` only.
    pub fn tick(mut self: core::pin::Pin<&mut Self>) {
        use cxx_qt::{CxxQtType, Threading};
        let tag = match self.as_mut().rust_mut().guard.try_begin() {
            Some(tag) => tag,
            None => return,
        };
        let qt_thread = self.qt_thread();
        let runtime = super::super::RUNTIME
            .get()
            .expect("gui runtime is set before Qt starts");
        runtime.spawn_blocking(move || {
            let outcome = archel_core::packages::list_installed();
            let _ = qt_thread.queue(
                move |mut obj: core::pin::Pin<&mut Self>| {
                    let applied = {
                        let mut rust = obj.as_mut().rust_mut();
                        if !rust.guard.is_newest(tag) {
                            rust.guard.finish();
                            return;
                        }
                        rust.guard.finish();
                        rust.apply_list(outcome)
                    };
                    obj.as_mut()
                        .set_installed_json(QString::from(&applied.installed_json));
                    obj.as_mut().set_banner_visible(applied.banner_visible);
                },
            );
        });
    }

    /// Live search: spawns `packages::search` on the blocking pool
    /// with a per-query sequence tag - rapid typing drops stale hits
    /// (T-06-26). A blank query clears the search back to the
    /// installed list without spawning.
    pub fn search(mut self: core::pin::Pin<&mut Self>, query: QString) {
        use cxx_qt::{CxxQtType, Threading};
        let text = query.to_string();
        if text.trim().is_empty() {
            let mut rust = self.as_mut().rust_mut();
            rust.search_seq = rust.search_seq.wrapping_add(1);
            self.as_mut().set_search_json(QString::from(""));
            self.as_mut().set_zero_hit(false);
            self.as_mut().set_search_busy(false);
            return;
        }
        let tag = {
            let mut rust = self.as_mut().rust_mut();
            rust.search_seq = rust.search_seq.wrapping_add(1);
            rust.search_seq
        };
        self.as_mut().set_search_busy(true);
        let qt_thread = self.qt_thread();
        let runtime = super::super::RUNTIME
            .get()
            .expect("gui runtime is set before Qt starts");
        runtime.spawn_blocking(move || {
            let query_blank = text.trim().is_empty();
            let hits = archel_core::packages::search(&text).unwrap_or_default();
            let _ = qt_thread.queue(
                move |mut obj: core::pin::Pin<&mut Self>| {
                    let applied = {
                        let mut rust = obj.as_mut().rust_mut();
                        match rust.apply_search(tag, query_blank, hits) {
                            Some(applied) => applied,
                            None => return,
                        }
                    };
                    let _ = applied.tag;
                    obj.as_mut()
                        .set_search_json(QString::from(&applied.search_json));
                    obj.as_mut().set_zero_hit(applied.zero_hit);
                    obj.as_mut().set_search_busy(applied.busy);
                },
            );
        });
    }

    /// Card selection: spawns `package_card` on the blocking pool with
    /// a per-selection sequence tag; an unknown name reads the N/A
    /// card.
    pub fn select(mut self: core::pin::Pin<&mut Self>, name: QString) {
        use cxx_qt::{CxxQtType, Threading};
        let text = name.to_string();
        let tag = {
            let mut rust = self.as_mut().rust_mut();
            rust.card_seq = rust.card_seq.wrapping_add(1);
            rust.card_seq
        };
        let qt_thread = self.qt_thread();
        let runtime = super::super::RUNTIME
            .get()
            .expect("gui runtime is set before Qt starts");
        runtime.spawn_blocking(move || {
            let outcome = archel_core::packages::package_card(&text);
            let _ = qt_thread.queue(
                move |mut obj: core::pin::Pin<&mut Self>| {
                    let applied = {
                        let mut rust = obj.as_mut().rust_mut();
                        match rust.apply_card(tag, text.clone(), outcome) {
                            Some(applied) => applied,
                            None => return,
                        }
                    };
                    let _ = applied.tag;
                    obj.as_mut()
                        .set_card_json(QString::from(&applied.card_json));
                },
            );
        });
    }

    /// Hides the D-05 banner after the user dismisses it.
    pub fn dismiss_banner(mut self: core::pin::Pin<&mut Self>) {
        use cxx_qt::CxxQtType;
        self.as_mut().rust_mut().banner.dismiss();
        self.as_mut().set_banner_visible(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archel_core::packages::{PackageCard, PackageInfo};

    fn installed_fixture() -> Vec<PackageInfo> {
        vec![
            PackageInfo {
                name: "linux".to_string(),
                version: "6.16.1-1".to_string(),
                source: None,
            },
            PackageInfo {
                name: "neovim".to_string(),
                version: "0.11.3-1".to_string(),
                source: None,
            },
        ]
    }

    fn search_fixture() -> Vec<PackageInfo> {
        vec![PackageInfo {
            name: "helix".to_string(),
            version: "25.07-1".to_string(),
            source: Some("extra".to_string()),
        }]
    }

    fn card_fixture() -> PackageCard {
        PackageCard {
            name: "helix".to_string(),
            candidate_version: "25.07-1".to_string(),
            installed_version: None,
            source: "extra".to_string(),
            download_size: Some("8.1 MiB".to_string()),
            installed_size: Some("30.4 MiB".to_string()),
        }
    }

    #[test]
    fn stale_search_hits_are_dropped_never_applied() {
        let mut model = PkgModelRust::default();
        model.search_seq = 2;
        let stale = model.apply_search(1, false, search_fixture());
        assert!(stale.is_none(), "an overtaken tag must not apply");
        let fresh = model
            .apply_search(2, false, search_fixture())
            .expect("newest tag applies");
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(&fresh.search_json).expect("search must be JSON");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], "helix");
        assert_eq!(rows[0]["source"], "extra");
        assert!(!fresh.busy, "the accepted hit clears the busy flag");
    }

    #[test]
    fn zero_hit_arms_the_refinement_copy_key() {
        let mut model = PkgModelRust::default();
        model.search_seq = 1;
        let applied = model
            .apply_search(1, false, Vec::new())
            .expect("newest tag applies");
        assert!(applied.zero_hit, "zero hits arm the refinement copy");
        assert_eq!(applied.search_json, "[]");
        model.search_seq = 2;
        let hits = model
            .apply_search(2, false, search_fixture())
            .expect("newest tag applies");
        assert!(!hits.zero_hit, "hits clear the refinement copy");
    }

    #[test]
    fn card_error_reads_na_never_zero_filled() {
        let mut model = PkgModelRust::default();
        model.card_seq = 1;
        let err: Result<PackageCard, archel_core::ArchelError> =
            Err(archel_core::ArchelError::PackageNotFound {
                name: "nope".to_string(),
            });
        let applied = model
            .apply_card(1, "nope".to_string(), err)
            .expect("newest tag applies");
        let card: serde_json::Value =
            serde_json::from_str(&applied.card_json).expect("card must be JSON");
        assert_eq!(card["name"], "nope", "the requested name survives");
        for key in [
            "candidate_version",
            "installed_version",
            "source",
            "download_size",
            "installed_size",
        ] {
            assert_eq!(
                card[key], NOT_AVAILABLE,
                "unknown package reads N/A for {key}"
            );
        }
        model.card_seq = 2;
        let stale = model.apply_card(1, "nope".to_string(), Ok(card_fixture()));
        assert!(stale.is_none(), "overtaken card selections never apply");
    }

    #[test]
    fn list_success_keeps_core_order_and_failure_reads_empty() {
        let mut model = PkgModelRust::default();
        let applied = model.apply_list(Ok(installed_fixture()));
        assert!(!applied.banner_visible, "success hides the banner");
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(&applied.installed_json).expect("list must be JSON");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], "linux");
        assert!(rows[0].get("version").is_some());
        let err: Result<Vec<PackageInfo>, archel_core::ArchelError> =
            Err(archel_core::ArchelError::Internal(
                "alpm unavailable in test".to_string(),
            ));
        let failed = model.apply_list(err);
        assert_eq!(failed.installed_json, "[]");
        assert!(failed.banner_visible, "first failure shows the banner");
    }

    #[test]
    fn card_success_follows_the_core_label_set() {
        let json = card_json_for(&card_fixture());
        let card: serde_json::Value =
            serde_json::from_str(&json).expect("card must be JSON");
        for key in [
            "name",
            "candidate_version",
            "installed_version",
            "source",
            "download_size",
            "installed_size",
        ] {
            assert!(card.get(key).is_some(), "card keeps the core field {key}");
        }
        assert_eq!(card["candidate_version"], "25.07-1");
    }
}
