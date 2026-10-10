//! Power profiles with backend provenance (PWR-01).
//!
//! Two-tier read/switch: the `net.hadess.PowerProfiles` system-bus
//! name is probed first (cheap `NameHasOwner`, timeout-capped,
//! mirroring the bluetooth daemon probe) and PPD serves the read and
//! the switch when present; otherwise the cpufreq sysfs fallback is
//! the primary path on this machine. Only the
//! `performance`/`powersave` governors exist on this kernel, so both
//! `balanced` and `power-saver` map to `powersave` on write, and a
//! `powersave` read reports `power-saver` — with the catalog note
//! always shown on the cpufreq backend so the collapse is never
//! silent. Every status/set output carries the backend tag
//! (`power-profiles-daemon` or `cpufreq`) for GUI provenance.
//!
//! Sysfs reads copy the sensors.rs hwmon discipline (read file,
//! parse, failure yields a typed error or honest N/A, never
//! zero-fill). Root-owned governor writes without privilege yield
//! [`ArchelError::PolkitDenied`] (hint family, never AuthFailed
//! which is helper-scoped); extending `archel-privileged` is out of
//! scope for v1 (T-4-08). `sys power` (watts readout) stays
//! read-only; `power` is the profile switcher, with distinct catalog
//! titles so the two never collide.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use crate::error::ArchelError;

/// sysfs directory holding the per-policy cpufreq governors.
pub const CPUFREQ_DIR: &str = "/sys/devices/system/cpu/cpufreq";

/// power-profiles-daemon well-known bus name on the system bus.
pub const PPD_BUS_NAME: &str = "net.hadess.PowerProfiles";

/// power-profiles-daemon object path and interface.
pub const PPD_PATH: &str = "/net/hadess/PowerProfiles";
pub const PPD_INTERFACE: &str = "net.hadess.PowerProfiles";

/// Ceiling for the PPD bus-name probe: power commands must degrade
/// to cpufreq in seconds, never hang.
pub const PPD_PROBE_TIMEOUT: Duration = Duration::from_secs(3);

/// Which backend served a read or a switch. Serialized as the plain
/// wire words so `--json` carries provenance (GUI renders it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PowerBackend {
    #[serde(rename = "power-profiles-daemon")]
    Ppd,
    #[serde(rename = "cpufreq")]
    Cpufreq,
}

/// The three PPD profiles. Serialized as the PPD wire words, which
/// double as the CLI vocabulary (`power set performance|...`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PowerProfile {
    #[serde(rename = "performance")]
    Performance,
    #[serde(rename = "balanced")]
    Balanced,
    #[serde(rename = "power-saver")]
    PowerSaver,
}

/// One `power status` / `power set` outcome: the profile, which
/// backend served it, and an optional human note (always present on
/// the cpufreq backend, documenting the governor collapse).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PowerStatus {
    pub profile: PowerProfile,
    pub backend: PowerBackend,
    pub note: Option<String>,
}

/// Parses the CLI profile vocabulary. Exact match only (clap also
/// constrains the values, so `None` is unreachable from the CLI).
pub fn parse_profile_arg(text: &str) -> Option<PowerProfile> {
    match text.trim() {
        "performance" => Some(PowerProfile::Performance),
        "balanced" => Some(PowerProfile::Balanced),
        "power-saver" => Some(PowerProfile::PowerSaver),
        _ => None,
    }
}

/// CLI/catalog display name for a profile (the PPD wire word).
pub fn profile_arg_name(profile: PowerProfile) -> &'static str {
    match profile {
        PowerProfile::Performance => "performance",
        PowerProfile::Balanced => "balanced",
        PowerProfile::PowerSaver => "power-saver",
    }
}

/// Parses one `scaling_governor` file body. Pure over the text so
/// fixtures cover it without sysfs.
pub fn parse_governor_body(body: &str) -> Option<String> {
    let governor = body.trim().to_lowercase();
    (!governor.is_empty()).then_some(governor)
}

/// Maps a governor name to the reported profile. `powersave` reads
/// as `power-saver` (the governor name says it); anything outside
/// the two known governors is `None` (honest error downstream,
/// never a guessed profile).
pub fn governor_to_profile(governor: &str) -> Option<PowerProfile> {
    match governor {
        "performance" => Some(PowerProfile::Performance),
        "powersave" => Some(PowerProfile::PowerSaver),
        _ => None,
    }
}

/// Maps a profile to the governor to write. Both `balanced` and
/// `power-saver` collapse to `powersave` — the only two governors
/// on this kernel — with the catalog note always shown.
pub fn profile_to_governor(profile: PowerProfile) -> &'static str {
    match profile {
        PowerProfile::Performance => "performance",
        PowerProfile::Balanced | PowerProfile::PowerSaver => "powersave",
    }
}

/// Backend wire tag for display and logs.
pub fn backend_tag(backend: PowerBackend) -> &'static str {
    match backend {
        PowerBackend::Ppd => "power-profiles-daemon",
        PowerBackend::Cpufreq => "cpufreq",
    }
}

/// Catalog note documenting the cpufreq governor collapse.
pub fn cpufreq_note() -> String {
    rust_i18n::t!("power.cpufreq_note").to_string()
}

/// Reads the cpufreq governors under `base` (fixtures inject their
/// own dir) and reports the profile with the cpufreq backend tag.
/// First sorted policy wins; missing dirs, unreadable files and
/// unknown governors all fail as typed [`ArchelError::SensorUnavailable`],
/// never zero-fill.
pub fn cpufreq_status_at(base: &Path) -> Result<PowerStatus, ArchelError> {
    let governor = first_governor_at(base)?;
    let profile = governor_to_profile(&governor).ok_or_else(|| ArchelError::SensorUnavailable {
        sensor: format!("cpufreq governor '{governor}'"),
    })?;
    Ok(PowerStatus {
        profile,
        backend: PowerBackend::Cpufreq,
        note: Some(cpufreq_note()),
    })
}

/// Reads the first sorted `policy*` governor under `base`.
fn first_governor_at(base: &Path) -> Result<String, ArchelError> {
    let mut policies: Vec<PathBuf> = std::fs::read_dir(base)
        .map_err(|_| ArchelError::SensorUnavailable {
            sensor: "cpufreq".to_string(),
        })?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("policy"))
        })
        .collect();
    policies.sort();
    let missing = || ArchelError::SensorUnavailable {
        sensor: "cpufreq".to_string(),
    };
    let first = policies.first().ok_or_else(missing)?;
    let body = std::fs::read_to_string(first.join("scaling_governor")).map_err(|_| missing())?;
    parse_governor_body(&body).ok_or_else(missing)
}

/// Lists every `policy*` dir under `base`, sorted (stable write
/// order across runs).
fn policy_dirs(base: &Path) -> Result<Vec<PathBuf>, ArchelError> {
    let mut policies: Vec<PathBuf> = std::fs::read_dir(base)
        .map_err(|_| ArchelError::SensorUnavailable {
            sensor: "cpufreq".to_string(),
        })?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("policy"))
        })
        .collect();
    policies.sort();
    if policies.is_empty() {
        return Err(ArchelError::SensorUnavailable {
            sensor: "cpufreq".to_string(),
        });
    }
    Ok(policies)
}

/// Writes `governor` to every `policy*/scaling_governor` under
/// `base`. Returns the written paths. Permission failures map to
/// [`ArchelError::PolkitDenied`] with the attempted action (hint,
/// never silent escalation — T-4-08); other IO failures are
/// [`ArchelError::Internal`] log detail.
pub fn write_governors_at(base: &Path, governor: &str) -> Result<Vec<PathBuf>, ArchelError> {
    let mut written = Vec::new();
    for policy in policy_dirs(base)? {
        let target = policy.join("scaling_governor");
        match std::fs::write(&target, format!("{governor}\n")) {
            Ok(()) => written.push(target),
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                return Err(ArchelError::PolkitDenied {
                    action: format!("power set ({})", target.display()),
                });
            }
            Err(err) => {
                return Err(ArchelError::Internal(format!(
                    "cannot write '{}': {err}",
                    target.display()
                )));
            }
        }
    }
    Ok(written)
}

/// Confirm-then-switch gate for the cpufreq path: the prompt names
/// the target profile, refusal happens before `writer`, so `writer`
/// structurally never runs on abort (mirrors the audio gate). The
/// status is re-read after the write (read-back, never a silent ok).
pub fn cpufreq_set_with(
    base: &Path,
    profile: PowerProfile,
    confirm: impl FnOnce(&str) -> Result<(), ArchelError>,
    writer: impl FnOnce(&str) -> Result<(), ArchelError>,
) -> Result<PowerStatus, ArchelError> {
    confirm(&confirm_set_prompt(profile))?;
    writer(profile_to_governor(profile))?;
    cpufreq_status_at(base)
}

/// Cheap PPD probe: `NameHasOwner net.hadess.PowerProfiles` on the
/// system bus, timeout-capped. `false` on ANY failure — callers fall
/// back to cpufreq.
pub async fn ppd_present() -> bool {
    let probe = async {
        let conn = zbus::Connection::system().await.ok()?;
        let proxy = zbus::Proxy::new(
            &conn,
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
        )
        .await
        .ok()?;
        let owned: bool = proxy.call("NameHasOwner", &(PPD_BUS_NAME,)).await.ok()?;
        Some(owned)
    };
    tokio::time::timeout(PPD_PROBE_TIMEOUT, probe)
        .await
        .ok()
        .flatten()
        .unwrap_or(false)
}

/// Opens the PPD properties proxy on the system bus.
async fn ppd_proxy(conn: &zbus::Connection) -> Result<zbus::Proxy<'_>, ArchelError> {
    zbus::Proxy::new(conn, PPD_BUS_NAME, PPD_PATH, PPD_INTERFACE)
        .await
        .map_err(|err| ArchelError::Internal(format!("PPD proxy: {err}")))
}

/// Parses one PPD `ActiveProfile` wire value. Unknown values map to
/// `None` (the caller reports them typed, never guessed).
fn parse_ppd_profile(value: &str) -> Option<PowerProfile> {
    parse_profile_arg(value)
}

/// Reads the PPD `ActiveProfile` property. Unknown wire values map
/// to [`ArchelError::Internal`] (daemon newer than this client),
/// never a guessed profile.
pub async fn ppd_status() -> Result<PowerStatus, ArchelError> {
    let conn = zbus::Connection::system()
        .await
        .map_err(|err| ArchelError::Internal(format!("PPD bus: {err}")))?;
    let proxy = ppd_proxy(&conn).await?;
    let active: String = proxy
        .get_property("ActiveProfile")
        .await
        .map_err(|err| ArchelError::Internal(format!("PPD ActiveProfile: {err}")))?;
    let profile = parse_ppd_profile(&active).ok_or_else(|| {
        ArchelError::Internal(format!("PPD ActiveProfile unknown: '{active}'"))
    })?;
    Ok(PowerStatus {
        profile,
        backend: PowerBackend::Ppd,
        note: None,
    })
}

/// Sets the PPD `ActiveProfile` property, then reads back.
pub async fn ppd_set(profile: PowerProfile) -> Result<PowerStatus, ArchelError> {
    let conn = zbus::Connection::system()
        .await
        .map_err(|err| ArchelError::Internal(format!("PPD bus: {err}")))?;
    let proxy = ppd_proxy(&conn).await?;
    proxy
        .set_property("ActiveProfile", profile_arg_name(profile))
        .await
        .map_err(|err| ArchelError::Internal(format!("PPD set ActiveProfile: {err}")))?;
    ppd_status_via(&conn).await
}

/// Reads `ActiveProfile` over an existing connection (read-back
/// after a set, without reconnecting).
async fn ppd_status_via(conn: &zbus::Connection) -> Result<PowerStatus, ArchelError> {
    let proxy = ppd_proxy(conn).await?;
    let active: String = proxy
        .get_property("ActiveProfile")
        .await
        .map_err(|err| ArchelError::Internal(format!("PPD ActiveProfile: {err}")))?;
    let profile = parse_ppd_profile(&active).ok_or_else(|| {
        ArchelError::Internal(format!("PPD ActiveProfile unknown: '{active}'"))
    })?;
    Ok(PowerStatus {
        profile,
        backend: PowerBackend::Ppd,
        note: None,
    })
}

/// Live `power status`: PPD when present, cpufreq otherwise.
pub async fn power_status() -> Result<PowerStatus, ArchelError> {
    if ppd_present().await {
        match ppd_status().await {
            Ok(status) => Ok(status),
            Err(_) => cpufreq_status_at(Path::new(CPUFREQ_DIR)),
        }
    } else {
        cpufreq_status_at(Path::new(CPUFREQ_DIR))
    }
}

/// Live `power set`: confirm-gated switch on whichever backend
/// serves this machine (PPD property or cpufreq governor writes).
pub async fn power_set(profile: PowerProfile) -> Result<PowerStatus, ArchelError> {
    crate::packages::confirm_action(&confirm_set_prompt(profile))?;
    power_set_direct(profile).await
}

/// Applies a profile switch WITHOUT any terminal confirmation.
///
/// The confirm moment belongs to the caller: the CLI passes through
/// [`power_set`] (terminal `[o/N]` gate); the GUI shows
/// [`confirm_set_prompt`] in its confirm modal and calls this only on
/// accept. Backend dispatch is identical in both paths (thin-view
/// contract; PLAT-03).
pub async fn power_set_direct(profile: PowerProfile) -> Result<PowerStatus, ArchelError> {
    if ppd_present().await {
        ppd_set(profile).await
    } else {
        cpufreq_set_with(
            Path::new(CPUFREQ_DIR),
            profile,
            |_| Ok(()),
            |governor| write_governors_at(Path::new(CPUFREQ_DIR), governor).map(|_| ()),
        )
    }
}

/// French status title from the catalog (distinct from the
/// `sys power` watts title — the two commands never collide).
pub fn status_title() -> String {
    rust_i18n::t!("power.status_title").to_string()
}

/// French status row labels from the catalog.
pub struct PowerLabels {
    pub profile: String,
    pub backend: String,
    pub note: String,
}

/// Collects the status row labels in one call.
pub fn power_labels() -> PowerLabels {
    PowerLabels {
        profile: rust_i18n::t!("power.label_profile").to_string(),
        backend: rust_i18n::t!("power.label_backend").to_string(),
        note: rust_i18n::t!("power.label_note").to_string(),
    }
}

/// Confirmation prompt naming the target profile, from the catalog.
pub fn confirm_set_prompt(profile: PowerProfile) -> String {
    rust_i18n::t!("power.confirm_set", profile = profile_arg_name(profile)).to_string()
}

/// Success line naming the new profile, from the catalog.
pub fn set_ok_line(profile: PowerProfile) -> String {
    rust_i18n::t!("power.set_ok", profile = profile_arg_name(profile)).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_dir(case: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "archel-power-test-{}-{}",
            std::process::id(),
            case
        ))
    }

    fn write_policy(base: &Path, policy: &str, governor: &str) {
        let dir = base.join(policy);
        std::fs::create_dir_all(&dir).expect("fixture dir");
        std::fs::write(dir.join("scaling_governor"), governor).expect("governor file");
    }

    #[test]
    fn governor_map_yields_performance_powersave_powersave_with_backend_note() {
        assert_eq!(profile_to_governor(PowerProfile::Performance), "performance");
        assert_eq!(profile_to_governor(PowerProfile::Balanced), "powersave");
        assert_eq!(profile_to_governor(PowerProfile::PowerSaver), "powersave");
        assert_eq!(governor_to_profile("performance"), Some(PowerProfile::Performance));
        assert_eq!(governor_to_profile("powersave"), Some(PowerProfile::PowerSaver));
        assert_eq!(governor_to_profile("schedutil"), None);
        let base = fixture_dir("map");
        write_policy(&base, "policy0", "powersave\n");
        write_policy(&base, "policy1", "powersave\n");
        let status = cpufreq_status_at(&base).expect("fixture governors must read");
        assert_eq!(status.profile, PowerProfile::PowerSaver);
        assert_eq!(status.backend, PowerBackend::Cpufreq);
        assert_eq!(backend_tag(status.backend), "cpufreq");
        assert!(
            status.note.is_some_and(|note| !note.trim().is_empty()),
            "cpufreq backend always carries the collapse note"
        );
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn sysfs_parse_handles_missing_and_broken_governor_files() {
        // Missing dir: honest typed error, never zero-fill.
        let missing = fixture_dir("missing-no-such-dir");
        std::fs::remove_dir_all(&missing).ok();
        let err = cpufreq_status_at(&missing).expect_err("no policies must fail");
        assert!(matches!(err, ArchelError::SensorUnavailable { .. }), "{err:?}");
        // Empty body and unknown governors: same honest error.
        assert!(parse_governor_body("   \n").is_none());
        let base = fixture_dir("broken");
        write_policy(&base, "policy0", "\n");
        let err = cpufreq_status_at(&base).expect_err("empty governor must fail");
        assert!(matches!(err, ArchelError::SensorUnavailable { .. }), "{err:?}");
        std::fs::remove_dir_all(&base).ok();
        let unknown = fixture_dir("unknown");
        write_policy(&unknown, "policy0", "schedutil\n");
        let err = cpufreq_status_at(&unknown).expect_err("unknown governor must fail");
        assert!(matches!(err, ArchelError::SensorUnavailable { .. }), "{err:?}");
        std::fs::remove_dir_all(&unknown).ok();
    }

    #[test]
    fn power_status_serializes_profile_backend_and_note() {
        let value = serde_json::to_value(PowerStatus {
            profile: PowerProfile::PowerSaver,
            backend: PowerBackend::Cpufreq,
            note: Some("collapse note".to_string()),
        })
        .expect("serializes");
        assert_eq!(value["profile"], "power-saver");
        assert_eq!(value["backend"], "cpufreq");
        assert_eq!(value["note"], "collapse note");
        let ppd = serde_json::to_value(PowerStatus {
            profile: PowerProfile::Balanced,
            backend: PowerBackend::Ppd,
            note: None,
        })
        .expect("serializes");
        assert_eq!(ppd["profile"], "balanced");
        assert_eq!(ppd["backend"], "power-profiles-daemon");
        assert!(ppd["note"].is_null());
    }

    #[test]
    fn power_set_abort_path_writes_nothing() {
        use std::cell::Cell;
        let base = fixture_dir("abort");
        write_policy(&base, "policy0", "powersave\n");
        let wrote = Cell::new(false);
        let err = cpufreq_set_with(
            &base,
            PowerProfile::Performance,
            |prompt| {
                assert!(prompt.contains("performance"));
                Err(ArchelError::ConfirmationAborted)
            },
            |_| {
                wrote.set(true);
                Ok(())
            },
        )
        .expect_err("declined confirm must abort");
        assert!(matches!(err, ArchelError::ConfirmationAborted));
        assert!(!wrote.get(), "abort must write nothing");
        let body = std::fs::read_to_string(base.join("policy0").join("scaling_governor"))
            .expect("fixture readable");
        assert_eq!(body, "powersave\n", "governor file untouched");
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn power_set_confirm_path_writes_then_reads_back() {
        let base = fixture_dir("roundtrip");
        write_policy(&base, "policy0", "powersave\n");
        let status = cpufreq_set_with(
            &base,
            PowerProfile::Performance,
            |_| Ok(()),
            |governor| {
                assert_eq!(governor, "performance");
                write_governors_at(&base, governor).map(|_| ())
            },
        )
        .expect("confirmed set must round-trip");
        assert_eq!(status.profile, PowerProfile::Performance);
        assert_eq!(status.backend, PowerBackend::Cpufreq);
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn governor_writes_without_privilege_yield_the_hint_error() {
        // T-4-08: a root-owned governor file the user cannot write
        // maps to PolkitDenied (hint), never a silent escalation and
        // never AuthFailed (helper-scoped). Read-only fixture file
        // simulates the root-owned sysfs node; skipped when running
        // as root (which bypasses file modes).
        use std::os::unix::fs::PermissionsExt;
        let base = fixture_dir("readonly");
        let dir = base.join("policy0");
        std::fs::create_dir_all(&dir).expect("fixture dir");
        let target = dir.join("scaling_governor");
        std::fs::write(&target, "powersave\n").expect("governor file");
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o444))
            .expect("chmod");
        if std::fs::write(&target, "x").is_ok() {
            std::fs::remove_dir_all(&base).ok();
            return;
        }
        let err = write_governors_at(&base, "performance")
            .expect_err("unwritable governor must fail typed");
        assert!(
            matches!(err, ArchelError::PolkitDenied { .. }),
            "wrong variant: {err:?}"
        );
        rust_i18n::set_locale("fr");
        assert!(!err.message().trim().is_empty());
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644)).ok();
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn profile_args_parse_exactly_or_not_at_all() {
        assert_eq!(parse_profile_arg("performance"), Some(PowerProfile::Performance));
        assert_eq!(parse_profile_arg("balanced"), Some(PowerProfile::Balanced));
        assert_eq!(parse_profile_arg("power-saver"), Some(PowerProfile::PowerSaver));
        assert_eq!(parse_profile_arg("powersave"), None);
        assert_eq!(parse_profile_arg(""), None);
        assert_eq!(parse_profile_arg("Performance"), None);
    }

    #[test]
    fn live_cpufreq_status_reads_this_kernel() {
        let status = cpufreq_status_at(Path::new(CPUFREQ_DIR)).expect("cpufreq readable here");
        assert_eq!(status.backend, PowerBackend::Cpufreq);
        assert!(status.note.is_some(), "collapse note always shown");
    }

    #[test]
    fn power_labels_come_from_the_catalog() {
        rust_i18n::set_locale("fr");
        assert!(!status_title().trim().is_empty());
        let labels = power_labels();
        assert!(!labels.profile.is_empty() && !labels.backend.is_empty());
        assert!(!confirm_set_prompt(PowerProfile::Balanced).is_empty());
        assert!(!set_ok_line(PowerProfile::Balanced).is_empty());
        assert!(!cpufreq_note().trim().is_empty());
    }
}
