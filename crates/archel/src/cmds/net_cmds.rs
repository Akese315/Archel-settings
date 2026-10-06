//! `archel net status` renderer: French table (default) or one JSON
//! object (`--json`).
//!
//! Bin-calls-core shape: every French string comes from
//! `archel-core::network` (title, labels, state wording, N/A cells) —
//! this module only lays out columns (`std::fmt` widths, no table
//! crate) and frames the single JSON object. One-shot by design:
//! connection state is not a 500 ms watch tick, so there is no loop
//! here (unlike `sys` metrics).

use archel_core::network::NetStatus;
use archel_core::ArchelError;

/// Runs `archel net status`: primary-connection state, interface,
/// SSID, IPv4, gateway, effective DNS and backing profile (NET-01/02).
pub async fn run_status(json: bool) -> Result<(), ArchelError> {
    let snap = archel_core::network::net_status().await?;
    if json {
        let payload = serde_json::to_string(&snap).map_err(|err| {
            ArchelError::Internal(format!("cannot encode net status JSON: {err}"))
        })?;
        println!("{payload}");
    } else {
        print!("{}", render_status_table(&snap));
    }
    Ok(())
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
    let width = rows
        .iter()
        .map(|(label, _)| label.chars().count())
        .max()
        .unwrap_or(0);
    let mut out = format!("{}\n", archel_core::network::status_title());
    for (label, value) in &rows {
        out.push_str(&format!("{} : {value}\n", pad_right(label, width)));
    }
    out
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
}
