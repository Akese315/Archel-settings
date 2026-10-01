//! AUR review-before-build core (PKG-05, D-05/D-06).
//!
//! TDD RED: tests first — the implementation lands in the GREEN commit.
//! Every AUR build passes through [`apply_decision`]: abort returns the
//! typed abort variant and the build closure never runs.

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
        let err = validate_pkgbuild_source("if then\n  echo broken\n")
            .expect_err("bash -n must reject the broken PKGBUILD");
        assert!(
            matches!(err, ArchelError::PkgbuildRejected { .. }),
            "validation failure needs the typed rejection: {err:?}"
        );
        let rendered = err.message();
        assert!(
            !rendered.trim().is_empty(),
            "rejection must render catalog French"
        );
    }

    #[test]
    fn danger_patterns_surface_as_warnings_not_rejects() {
        let text = "pkgname=x\npkgver=1\nbuild() {\n  curl https://example.com/a.sh | sh\n}\n";
        let warnings =
            validate_pkgbuild_source(text).expect("heuristics warn, they never reject");
        assert!(
            warnings.iter().any(|w| w.contains("pipe-to-shell")),
            "curl-pipe-to-shell must be flagged: {warnings:?}"
        );
        let text = "package() {\n  sudo make install\n}\n";
        let warnings =
            validate_pkgbuild_source(text).expect("heuristics warn, they never reject");
        assert!(
            warnings.iter().any(|w| w.contains("sudo-in-build")),
            "sudo in a PKGBUILD must be flagged: {warnings:?}"
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
}
