//! Single-instance guarantee for `archel settings` (per D-26).
//!
//! The claim is ONE `bind()` on an abstract-namespace Unix socket: the bind
//! result is the test itself (per Pitfall 3: never probe-then-bind, which
//! races, and never unlink a socket you do not own). Abstract sockets are
//! kernel-freed when the owning process exits, so a crash leaves no stale
//! state and the next launch claims cleanly (verified by the re-acquire
//! test). The `v1` suffix allows a clean protocol rename later.

use std::io::ErrorKind;

use archel_core::ArchelError;
use tokio::net::UnixListener;

/// Abstract socket name: the leading NUL byte selects the Linux abstract
/// namespace, so nothing ever appears on the filesystem (RESEARCH A1; the
/// tests below pin that no file is created).
const SOCKET_NAME: &str = "\0archel-settings-v1";

/// The live claim; keeping it alive for the whole `exec()` scope is what
/// makes the second launch refuse.
pub(crate) type Listener = UnixListener;

/// Claims the single-instance slot. `Ok(Some)` when this process is the
/// first (bind won), `Ok(None)` when another instance already holds the
/// abstract name (EADDRINUSE: the caller prints the catalog refusal and
/// exits 0), `Err` only for unexpected I/O failures.
pub(crate) fn acquire() -> Result<Option<Listener>, ArchelError> {
    match UnixListener::bind(SOCKET_NAME) {
        Ok(listener) => Ok(Some(listener)),
        Err(err) if err.kind() == ErrorKind::AddrInUse => Ok(None),
        Err(err) => Err(ArchelError::Internal(format!(
            "single-instance bind failed: {err}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, MutexGuard};

    use super::acquire;

    /// Cargo runs one binary's tests on parallel threads and the abstract
    /// socket name is system-global, so every test here holds this lock
    /// (the locale-touching test too, for the same reason).
    static SOCKET_LOCK: Mutex<()> = Mutex::new(());

    fn serialize() -> MutexGuard<'static, ()> {
        SOCKET_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn single_instance_second_launch_refuses_and_slot_frees_on_drop() {
        let _serial = serialize();
        // The synchronous `UnixListener::bind` registers its socket with
        // the driving reactor, so it needs a runtime context (production
        // enters the GUI runtime in `run_settings` first).
        let runtime = tokio::runtime::Runtime::new().expect("test runtime");
        let _ctx = runtime.enter();

        // First claim wins.
        let first = acquire()
            .expect("first bind must succeed")
            .expect("no other holder at test start");

        // A second launch while the first is alive must be refused
        // (Ok(None)): never an error, never a second success (D-26).
        assert!(
            acquire()
                .expect("probe must not fail I/O")
                .is_none(),
            "second instance must be refused while the first holds the name"
        );

        // The claim is owned by the listener: dropping it (process exit)
        // frees the abstract name with no stale state to clean up.
        drop(first);
        assert!(
            acquire()
                .expect("re-acquire must succeed after drop")
                .is_some(),
            "slot must be claimable again once the holder is gone"
        );
    }

    #[test]
    fn single_instance_never_touches_the_filesystem() {
        let _serial = serialize();
        let runtime = tokio::runtime::Runtime::new().expect("test runtime");
        let _ctx = runtime.enter(); // bind needs the reactor context

        // Abstract sockets are kernel-only objects (RESEARCH A1): claiming
        // one must leave the runtime directory byte-for-byte unchanged.
        let Ok(base) = std::env::var("XDG_RUNTIME_DIR") else {
            return; // no runtime dir here: nothing that could be polluted
        };
        let listing = |dir: &str| -> Vec<std::ffi::OsString> {
            let mut names: Vec<_> = std::fs::read_dir(dir)
                .expect("runtime dir must be readable")
                .map(|entry| entry.expect("entry must be readable").file_name())
                .collect();
            names.sort();
            names
        };
        let before = listing(&base);
        let held = acquire()
            .expect("bind must succeed")
            .expect("test binary is the only holder");
        assert_eq!(
            before,
            listing(&base),
            "abstract-namespace claim must not create a filesystem socket"
        );
        drop(held);
    }

    #[test]
    fn single_instance_refusal_line_comes_from_the_catalog() {
        let _serial = serialize(); // guards the process-global locale too

        archel_core::i18n::set_language("fr");
        let line = archel_core::i18n::settings_already_running();

        // ASCII-safe assertions only (the D-07 guard forbids French
        // literals in `.rs`): the key resolves, the sentence is accented
        // French, and no replacement characters leaked in. The exact
        // wording belongs to locales/fr.json alone.
        assert!(!line.is_empty(), "catalog line must render");
        assert!(
            !line.contains("settings.already_running"),
            "key must resolve, not fall back to its own name"
        );
        assert!(
            line.starts_with("Archel ") && line.ends_with('.'),
            "unexpected shape: {line}"
        );
        assert!(
            line.chars().any(|c| !c.is_ascii()) && !line.contains('\u{FFFD}'),
            "must be the accented French sentence, not mojibake: {line}"
        );
    }
}
