//! i18n seam (per D-07): French catalog wiring.
//!
//! The `rust_i18n::i18n!` backend is declared at the crate root (`lib.rs`);
//! this module owns locale selection plus the D-07 repo-wide guard: no `.rs`
//! file outside `locales/` may contain hardcoded French diacritics.

/// Switches the rendering locale. v1 ships French only; v2 adds a second
/// language by adding one catalog file, without touching Rust code (D-07).
pub fn set_language(locale: &str) {
    rust_i18n::set_locale(locale);
}

/// Currently active locale name.
pub fn language() -> String {
    rust_i18n::locale().to_string()
}

/// Refusal line printed by a second `archel settings` launch (D-26). Lives
/// here so the French sentence stays in the catalog (D-07): the GUI binary
/// only ever prints what this returns.
pub fn settings_already_running() -> String {
    rust_i18n::t!("settings.already_running").to_string()
}

#[cfg(test)]
mod tests {
    /// French diacritic ranges (Pitfall 6): Latin-1 accented block plus the
    /// oe ligatures. Any match in a `.rs` file means a French literal that
    /// belongs in `locales/fr.json`.
    fn is_french_diacritic(c: char) -> bool {
        matches!(c, '\u{00C0}'..='\u{00FF}' | '\u{0152}' | '\u{0153}')
    }

    #[test]
    fn no_hardcoded_french_diacritics_in_rust_sources() {
        // Workspace root is two levels above this crate's manifest dir.
        let crates_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("crates");
        let mut offenders = Vec::new();
        let mut stack = vec![crates_dir];
        while let Some(dir) = stack.pop() {
            let entries = std::fs::read_dir(&dir).expect("crates dir must be readable");
            for entry in entries {
                let path = entry.expect("dir entry must be readable").path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    let content =
                        std::fs::read_to_string(&path).expect("Rust file must be readable");
                    if content.chars().any(is_french_diacritic) {
                        offenders.push(path);
                    }
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "hardcoded French diacritics in Rust sources (move them to locales/fr.json): {offenders:?}"
        );
    }
}
