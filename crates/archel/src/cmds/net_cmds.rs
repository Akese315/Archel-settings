//! `archel net status` renderer: French table (default) or one JSON
//! object (`--json`).
//!
//! Bin-calls-core shape: every French string comes from
//! `archel-core::network` (title, labels, state wording, N/A cells) —
//! this module only lays out columns (`std::fmt` widths, no table
//! crate) and frames the single JSON object. One-shot by design:
//! connection state is not a 500 ms watch tick, so there is no loop
//! here (unlike `sys` metrics).

use archel_core::network::selftest::NetTest;
use archel_core::network::{dns::DnsReport, dns::DnsShow, NetStatus};
use archel_core::ArchelError;

/// Runs `archel net status`: primary-connection state, interface,
/// SSID, IPv4, gateway, effective DNS and backing profile (NET-01/02).
pub async fn run_status(json: bool) -> Result<(), ArchelError> {
    let snap = archel_core::network::net_status().await?;
    if json {
        println!("{}", encode_json(&snap, "net status")?);
    } else {
        print!("{}", render_status_table(&snap));
    }
    Ok(())
}

/// Runs `archel net dns show`: configured profile DNS plus the
/// effective (post-DHCP-or-custom) DNS (NET-03, read-only).
pub async fn run_dns_show(profile: Option<String>, json: bool) -> Result<(), ArchelError> {
    let show = archel_core::network::dns::dns_show(profile.as_deref()).await?;
    if json {
        println!("{}", encode_json(&show, "net dns show")?);
    } else {
        print!("{}", render_dns_show_table(&show));
    }
    Ok(())
}

/// Runs `archel net dns set`: validate, preview, confirm, then replace
/// the profile DNS and reactivate (NET-03, D-01/D-02).
pub fn run_dns_set(
    profile: Option<String>,
    servers: Vec<String>,
    json: bool,
) -> Result<(), ArchelError> {
    // Read-only planning first: validation + resolution + old-config
    // read run before any prompt, so typos fail fast with no TTY need.
    let plan = archel_core::network::dns::plan_dns_set(profile.as_deref(), &servers)?;
    run_dns_plan_confirmed(&plan, json, archel_core::network::dns::apply_dns_plan)
}

/// Runs `archel net dns reset`: same gate as set, back to DHCP (D-03).
pub fn run_dns_reset(profile: Option<String>, json: bool) -> Result<(), ArchelError> {
    let plan = archel_core::network::dns::plan_dns_reset(profile.as_deref())?;
    run_dns_plan_confirmed(&plan, json, archel_core::network::dns::apply_dns_plan)
}

/// Confirm-then-execute gate for DNS plans (mirrors the `run_aur`
/// shape): the preview renders, `confirm_action` enforces the TTY +
/// `[o/N]` rule, and only a yes reaches `apply`. The injectable
/// `apply` keeps the abort path unit-coverable headless: refusal
/// happens before the closure, so the closure structurally never runs
/// on abort.
fn run_dns_plan_confirmed(
    plan: &archel_core::network::dns::DnsPlan,
    json: bool,
    apply: impl FnOnce(&archel_core::network::dns::DnsPlan) -> Result<DnsReport, ArchelError>,
) -> Result<(), ArchelError> {
    if !json {
        print!("{}", render_dns_plan_preview(plan));
    }
    archel_core::packages::confirm_action(
        &archel_core::network::dns::confirm_dns_prompt(plan),
    )?;
    let report = apply(plan)?;
    if json {
        println!("{}", encode_json(&report, "net dns")?);
    } else {
        print!("{}", render_dns_report_table(&report));
    }
    Ok(())
}

/// Runs `archel net test`: gateway ping, DNS resolution, internet
/// reachability with raw ms figures (NET-06, D-09/D-10). Read-only
/// probes (ping/dig); the blocking spawns run off-executor.
pub async fn run_test(json: bool) -> Result<(), ArchelError> {
    let test = tokio::task::spawn_blocking(archel_core::network::selftest::run_net_test)
        .await
        .map_err(|err| {
            ArchelError::Internal(format!("net test task failed: {err}"))
        })??;
    if json {
        println!("{}", encode_json(&test, "net test")?);
    } else {
        print!("{}", render_test_table(&test));
    }
    Ok(())
}

/// Runs `archel net proxy` (bare): one-line session proxy state plus
/// the shell export hint for Hyprland terminals (NET-04 tracer).
/// Read-only over the `gsettings` CLI; a missing store maps to the
/// typed unavailable error, never a panic.
pub fn run_proxy(json: bool) -> Result<(), ArchelError> {
    let state = archel_core::proxy::proxy_status()?;
    if json {
        println!("{}", encode_json(&state, "net proxy")?);
    } else {
        println!("{}", archel_core::proxy::status_line(&state));
        println!("{}", archel_core::proxy::export_hint());
    }
    Ok(())
}

/// Runs `archel net proxy off`: clear the session proxy after
/// confirmation (NET-04). Planned read-only first (old mode read
/// before any prompt), so store failures surface typed with no TTY
/// need.
pub fn run_proxy_off(json: bool) -> Result<(), ArchelError> {
    let old = archel_core::proxy::proxy_status()?;
    let plan = archel_core::proxy::ProxyPlan::off(&old.mode);
    run_proxy_plan_confirmed(&plan, json, archel_core::proxy::apply_proxy_plan)
}

/// Runs `archel net proxy manual`: per-scheme host/port with optional
/// bypass list and auth (NET-04). `--password` triggers a no-echo
/// stdin prompt at use time — the secret travels straight into the
/// plan, never through argv, logs or snapshots.
pub fn run_proxy_manual(
    host: &str,
    port: u16,
    exceptions: Option<&str>,
    user: Option<&str>,
    prompt_password: bool,
    json: bool,
) -> Result<(), ArchelError> {
    let old = archel_core::proxy::proxy_status()?;
    let password = if prompt_password {
        Some(archel_core::proxy::prompt_proxy_password()?)
    } else {
        None
    };
    let plan = archel_core::proxy::plan_proxy_manual(
        &old.mode,
        host,
        port,
        exceptions.unwrap_or(""),
        user,
        password,
    )?;
    run_proxy_plan_confirmed(&plan, json, archel_core::proxy::apply_proxy_plan)
}

/// Runs `archel net proxy auto`: PAC-URL mode after confirmation.
pub fn run_proxy_auto(pac_url: &str, json: bool) -> Result<(), ArchelError> {
    let old = archel_core::proxy::proxy_status()?;
    let plan = archel_core::proxy::plan_proxy_auto(&old.mode, pac_url)?;
    run_proxy_plan_confirmed(&plan, json, archel_core::proxy::apply_proxy_plan)
}

/// Confirm-then-execute gate for proxy plans (mirrors the DNS shape):
/// the preview renders, `confirm_action` enforces the TTY + `[o/N]`
/// rule, and only a yes reaches `apply`. The injectable `apply` keeps
/// the abort path unit-coverable headless.
fn run_proxy_plan_confirmed(
    plan: &archel_core::proxy::ProxyPlan,
    json: bool,
    apply: impl FnOnce(&archel_core::proxy::ProxyPlan) -> Result<
        archel_core::proxy::ProxyApplyReport,
        ArchelError,
    >,
) -> Result<(), ArchelError> {
    use archel_core::proxy as core_proxy;
    if !json {
        print!(
            "{}",
            render_label_table(&core_proxy::proxy_preview_title(plan), &core_proxy::proxy_preview_rows(plan))
        );
    }
    archel_core::packages::confirm_action(&core_proxy::confirm_proxy_prompt(plan))?;
    let report = apply(plan)?;
    if json {
        println!("{}", encode_json(&report, "net proxy")?);
    } else {
        print!(
            "{}",
            render_label_table(&core_proxy::proxy_report_title(), &core_proxy::proxy_report_rows(&report))
        );
    }
    Ok(())
}

/// Encodes one `--json` payload (single object per line, NDJSON-safe).
fn encode_json(payload: &impl serde::Serialize, what: &str) -> Result<String, ArchelError> {
    serde_json::to_string(payload)
        .map_err(|err| ArchelError::Internal(format!("cannot encode {what} JSON: {err}")))
}
/// Right-pads a label on char count (mirrors the `sys_cmds` helper —
/// byte widths would misalign accented French labels).
fn pad_right(label: &str, width: usize) -> String {
    let mut out = label.to_string();
    while out.chars().count() < width {
        out.push(' ');
    }
    out
}

/// French label table for one [`NetStatus`] snapshot.
fn render_status_table(snap: &NetStatus) -> String {
    let rows = archel_core::network::status_rows(snap);
    render_label_table(&archel_core::network::status_title(), &rows)
}

/// Generic French label table (title + padded `label : value` rows).
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

/// French table for `net dns show`: configured profile DNS, the DHCP
/// override flag, and the effective DNS.
fn render_dns_show_table(show: &DnsShow) -> String {
    use archel_core::network::dns as core_dns;
    render_label_table(&core_dns::show_title(), &core_dns::show_rows(show))
}

/// French preview table for a [`DnsPlan`] (shown before `[o/N]`).
fn render_dns_plan_preview(plan: &archel_core::network::dns::DnsPlan) -> String {
    use archel_core::network::dns as core_dns;
    render_label_table(
        &core_dns::plan_preview_title(plan),
        &core_dns::plan_preview_rows(plan),
    )
}

/// French table for a [`DnsReport`] (shown after apply).
fn render_dns_report_table(report: &DnsReport) -> String {
    use archel_core::network::dns as core_dns;
    render_label_table(&core_dns::report_title(), &core_dns::report_rows(report))
}

/// French table for [`NetTest`]: one row per hop, target plus raw ms
/// or N/A. Deliberately verdict-free (D-10 prohibition): no diagnosis,
/// health or pass/fail word appears here.
fn render_test_table(test: &NetTest) -> String {
    use archel_core::network::selftest as core_test;
    render_label_table(&core_test::test_title(), &core_test::test_rows(test))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_status() -> NetStatus {
        NetStatus {
            source: "nmcli".to_string(),
            connected: true,
            state_detail: archel_core::network::describe_nm_state(70),
            interface: Some("wlan0".to_string()),
            ssid: Some("KLEY Rennes priv\u{e9}e".to_string()),
            ipv4_address: Some("10.192.61.76/24".to_string()),
            gateway: Some("10.192.61.1".to_string()),
            dns: vec!["10.192.61.1".to_string()],
            profile_name: Some("KLEY Rennes priv\u{e9}e".to_string()),
            profile_uuid: Some("cbb5b16c-02d9-4ac7-a99c-d02b909cfb69".to_string()),
            connection_type: Some("802-11-wireless".to_string()),
        }
    }

    #[test]
    fn status_table_shows_title_and_every_row() {
        archel_core::i18n::set_language("fr");
        let table = render_status_table(&sample_status());
        assert!(table.contains(&archel_core::network::status_title()));
        for needle in ["wlan0", "KLEY Rennes priv\u{e9}e", "10.192.61.76/24", "10.192.61.1"] {
            assert!(table.contains(needle), "table must show `{needle}`");
        }
    }

    #[test]
    fn status_json_schema_pins_snapshot_fields() {
        let snap = sample_status();
        let payload =
            serde_json::to_string(&snap).expect("NetStatus must encode to JSON");
        let value: serde_json::Value =
            serde_json::from_str(&payload).expect("payload must parse");
        for field in [
            "source",
            "connected",
            "state_detail",
            "interface",
            "ssid",
            "ipv4_address",
            "gateway",
            "dns",
            "profile_name",
            "profile_uuid",
            "connection_type",
        ] {
            assert!(
                value.get(field).is_some(),
                "net status JSON must carry `{field}`"
            );
        }
        insta::assert_json_snapshot!(value);
    }

    fn sample_plan() -> archel_core::network::dns::DnsPlan {
        archel_core::network::dns::DnsPlan {
            profile_name: "work".to_string(),
            profile_uuid: "uuid-3".to_string(),
            old_servers: Vec::new(),
            reset: false,
            new_servers: vec!["1.1.1.1".to_string(), "9.9.9.9".to_string()],
        }
    }

    fn sample_test() -> NetTest {
        use archel_core::network::selftest::HopResult;
        NetTest {
            hops: vec![
                HopResult {
                    hop: "Passerelle".to_string(),
                    target: "10.192.61.1".to_string(),
                    ms: Some(0.317),
                },
                HopResult {
                    hop: "DNS".to_string(),
                    target: "archlinux.org".to_string(),
                    ms: Some(12.0),
                },
                HopResult {
                    hop: "Internet".to_string(),
                    target: "1.1.1.1".to_string(),
                    ms: None,
                },
            ],
        }
    }

    #[test]
    fn dns_preview_names_profile_and_old_to_new() {
        archel_core::i18n::set_language("fr");
        let preview = render_dns_plan_preview(&sample_plan());
        for needle in ["work", "uuid-3", "1.1.1.1 9.9.9.9"] {
            assert!(preview.contains(needle), "preview must show `{needle}`");
        }
    }

    #[test]
    fn dns_report_json_schema_pins_mutation_fields() {
        let report = DnsReport {
            profile_name: "work".to_string(),
            profile_uuid: "uuid-3".to_string(),
            old_servers: Vec::new(),
            new_servers: vec!["1.1.1.1".to_string()],
            effective_dns: Some(vec!["1.1.1.1".to_string()]),
            reactivated: true,
        };
        let value: serde_json::Value =
            serde_json::to_value(&report).expect("DnsReport serializes");
        for field in [
            "profile_name",
            "profile_uuid",
            "old_servers",
            "new_servers",
            "effective_dns",
            "reactivated",
        ] {
            assert!(value.get(field).is_some(), "report must carry `{field}`");
        }
        let joined = serde_json::to_string(&value).expect("report encodes");
        assert!(
            !joined.contains("ipv6"),
            "D-02: the report never touches IPv6 keys"
        );
        insta::assert_json_snapshot!(value);
    }

    #[test]
    fn dns_confirm_abort_spawns_no_write() {
        // The confirm gate with an injectable apply probe: headless
        // stdin is not a TTY, so the gate refuses before the closure —
        // the abort path structurally spawns no `nmcli` modify/up.
        use std::io::IsTerminal;
        if std::io::stdin().is_terminal() {
            return;
        }
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let writes = Arc::new(AtomicUsize::new(0));
        let probe = Arc::clone(&writes);
        let plan = sample_plan();
        let err = run_dns_plan_confirmed(&plan, false, move |_| {
            probe.fetch_add(1, Ordering::SeqCst);
            Ok(DnsReport {
                profile_name: String::new(),
                profile_uuid: String::new(),
                old_servers: Vec::new(),
                new_servers: Vec::new(),
                effective_dns: None,
                reactivated: false,
            })
        })
        .expect_err("headless confirm must refuse");
        assert!(
            matches!(err, ArchelError::NeedsTty),
            "abort path needs the typed refusal, never a write: {err:?}"
        );
        assert_eq!(
            writes.load(Ordering::SeqCst),
            0,
            "the abort path spawns no nmcli write"
        );
    }

    #[test]
    fn test_table_shows_raw_ms_and_na_without_verdicts() {
        archel_core::i18n::set_language("fr");
        let table = render_test_table(&sample_test());
        assert!(table.contains("0.32 ms"), "gateway hop shows raw ms");
        assert!(table.contains("12.00 ms"), "dns hop shows raw ms");
        assert!(
            table.contains(&archel_core::sensors::unavailable_label()),
            "unreachable hop renders N/A"
        );
        assert!(!table.contains("0.00 ms"), "no fake-zero for dead hops");
        for verdict in [
            "diagnostic",
            "verdict",
            "conclusion",
            "recommandation",
            "sante",
        ] {
            assert!(
                !table.to_lowercase().contains(verdict),
                "D-10: no verdict word `{verdict}` in test output"
            );
        }
    }

    #[test]
    fn test_json_schema_pins_hop_fields() {        let value: serde_json::Value =
            serde_json::to_value(&sample_test()).expect("NetTest serializes");
        let hops = value["hops"].as_array().expect("three hops");
        assert_eq!(hops.len(), 3);
        for hop in hops {
            for field in ["hop", "target", "ms"] {
                assert!(hop.get(field).is_some(), "hop must carry `{field}`");
            }
        }
        assert!(hops[2]["ms"].is_null(), "dead hop encodes null, never 0");
        insta::assert_json_snapshot!(value);
    }

    fn sample_proxy_plan() -> archel_core::proxy::ProxyPlan {
        archel_core::proxy::plan_proxy_manual(
            "none",
            "proxy.example",
            8080,
            "localhost",
            Some("alice"),
            Some("s3cret-pw".to_string()),
        )
        .expect("sample proxy plan validates")
    }

    #[test]
    fn proxy_preview_names_mode_and_endpoint() {
        use archel_core::proxy as core_proxy;
        archel_core::i18n::set_language("fr");
        let plan = sample_proxy_plan();
        let preview = render_label_table(
            &core_proxy::proxy_preview_title(&plan),
            &core_proxy::proxy_preview_rows(&plan),
        );
        for needle in ["proxy.example:8080", "localhost"] {
            assert!(preview.contains(needle), "preview must show `{needle}`");
        }
        let prompt = core_proxy::confirm_proxy_prompt(&plan);
        assert!(
            prompt.contains("proxy.example:8080"),
            "confirm prompt names the endpoint: {prompt:?}"
        );
    }

    #[test]
    fn proxy_report_json_schema_pins_apply_fields_without_secrets() {
        let plan = sample_proxy_plan();
        let report = archel_core::proxy::ProxyApplyReport::from_plan(&plan);
        let value: serde_json::Value =
            serde_json::to_value(&report).expect("ProxyApplyReport serializes");
        for field in ["mode", "endpoint", "exceptions", "auth_configured"] {
            assert!(value.get(field).is_some(), "report must carry `{field}`");
        }
        let joined = serde_json::to_string(&value).expect("report encodes");
        assert!(
            !joined.contains("s3cret-pw"),
            "report JSON never carries the password"
        );
        for marker in ["password", "secret"] {
            assert!(
                !joined.to_lowercase().contains(marker),
                "no secret-shaped field `{marker}` in proxy report JSON"
            );
        }
        insta::assert_json_snapshot!(value);
    }

    #[test]
    fn proxy_confirm_abort_spawns_no_write() {
        // Same injectable-closure abort gate as DNS: headless stdin is
        // not a TTY, so the gate refuses before the closure — the abort
        // path structurally spawns no `gsettings set`.
        use std::io::IsTerminal;
        if std::io::stdin().is_terminal() {
            return;
        }
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let writes = Arc::new(AtomicUsize::new(0));
        let probe = Arc::clone(&writes);
        let plan = sample_proxy_plan();
        let err = run_proxy_plan_confirmed(&plan, false, move |planned| {
            probe.fetch_add(1, Ordering::SeqCst);
            Ok(archel_core::proxy::ProxyApplyReport::from_plan(planned))
        })
        .expect_err("headless confirm must refuse");
        assert!(
            matches!(err, ArchelError::NeedsTty),
            "abort path needs the typed refusal, never a write: {err:?}"
        );
        assert_eq!(
            writes.load(Ordering::SeqCst),
            0,
            "the abort path spawns no gsettings write"
        );
    }
}
