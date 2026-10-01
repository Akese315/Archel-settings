//! Installed-package reads (PKG-01, D-08): alpm localdb list.
//!
//! The installed list comes from the alpm localdb in-process: no fork,
//! no locale hazard, epoch-aware version ordering through `Ver`'s
//! `PartialOrd` (`alpm_pkg_vercmp`). Repo and AUR queries (search, card)
//! stay `LC_ALL=C`-pinned subprocesses and land in later tasks.

use serde::Serialize;

use crate::error::ArchelError;

/// One installed package: name plus version, as shown by
/// `archel pkg list`.
///
/// Field names are the v1 JSON schema (`--json` serializes this struct
/// directly); the schema is unstable until the GUI phase pins it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
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
}
