//! `archel bt devices/scan/pair/connect/disconnect` renderers:
//! French tables (default) or JSON (`--json`).
//!
//! Bin-calls-core shape: every French string comes from
//! `archel-core::bluetooth` (titles, labels, prompts, ok lines,
//! state wording, N/A cells) — this module only lays out columns
//! (`std::fmt` widths, no table crate) and frames the JSON payloads.
//! Daemon-down maps to the typed `BluetoothUnavailable` catalog
//! error, never a panic, never a hang. Pair/connect/disconnect take
//! a pick from the just-printed scanned set only (scan-then-pick):
//! each runs the timed sweep, prints the numbered live list, then
//! validates the pick client-side — never a free-form MAC.

use archel_core::bluetooth::BtDevice;
use archel_core::ArchelError;

/// Runs `archel bt devices`: known devices (name, MAC, signal,
/// state) or the honest enable-hint error with the daemon down.
pub async fn run_devices(json: bool) -> Result<(), ArchelError> {
    let devices = archel_core::bluetooth::bt_devices().await?;
    if json {
        println!("{}", encode_json(&devices, "bt devices")?);
    } else {
        print!("{}", render_devices_table(&devices));
    }
    Ok(())
}

/// Runs `archel bt scan`: timed discovery sweep into the numbered
/// live list (name, MAC, signal), or the honest-empty line when the
/// air is silent. Returns the scanned set for pick validation.
pub async fn run_scan(json: bool) -> Result<Vec<BtDevice>, ArchelError> {
    let devices = archel_core::bluetooth::bt_scan().await?;
    if json {
        println!("{}", encode_json(&devices, "bt scan")?);
    } else {
        print!("{}", render_scan_table(&devices));
    }
    Ok(devices)
}

/// Runs `archel bt pair <pick>`: scan, print the live list, resolve
/// the pick against it, confirm intent, pair with the BlueZ code
/// confirm, then offer pair-then-connect (each step confirm-gated;
/// piped runs refuse with `NeedsTty` before anything mutates).
pub async fn run_pair(pick: &str, json: bool) -> Result<(), ArchelError> {
    let scanned = run_scan(json).await?;
    let picked = archel_core::bluetooth::resolve_pick(&scanned, pick)?;
    let paired = archel_core::bluetooth::pair_picked_with(
        &scanned,
        pick,
        archel_core::packages::confirm_action,
        |device: BtDevice| async move { archel_core::bluetooth::pair_device(&device).await },
    )
    .await?;
    debug_assert_eq!(picked.address, paired.address);
    if json {
        println!("{}", encode_json(&paired, "bt pair")?);
    } else {
        println!("{}", archel_core::bluetooth::paired_ok_line(&paired));
    }
    // Pair-then-connect: a second explicit confirm reuses the same
    // gate (NeedsTty when piped); a decline ends after pairing.
    if archel_core::packages::confirm_action(
        &archel_core::bluetooth::connect_confirm_prompt(&paired),
    )
    .is_ok()
    {
        archel_core::bluetooth::connect_device(&paired).await?;
        if !json {
            println!("{}", archel_core::bluetooth::connected_ok_line(&paired));
        }
    }
    Ok(())
}

/// Runs `archel bt connect <pick>`: scan, print the live list,
/// resolve the pick, confirm, connect.
pub async fn run_connect(pick: &str, json: bool) -> Result<(), ArchelError> {
    let scanned = run_scan(json).await?;
    let picked = archel_core::bluetooth::resolve_pick(&scanned, pick)?;
    archel_core::packages::confirm_action(
        &archel_core::bluetooth::connect_confirm_prompt(&picked),
    )?;
    archel_core::bluetooth::connect_device(&picked).await?;
    if json {
        println!("{}", encode_json(&picked, "bt connect")?);
    } else {
        println!(
            "{}",
            archel_core::bluetooth::connected_ok_line(&picked)
        );
    }
    Ok(())
}

/// Runs `archel bt disconnect <pick>`: scan, print the live list,
/// resolve the pick, confirm, disconnect.
pub async fn run_disconnect(pick: &str, json: bool) -> Result<(), ArchelError> {
    let scanned = run_scan(json).await?;
    let picked = archel_core::bluetooth::resolve_pick(&scanned, pick)?;
    archel_core::packages::confirm_action(
        &archel_core::bluetooth::disconnect_confirm_prompt(&picked),
    )?;
    archel_core::bluetooth::disconnect_device(&picked).await?;
    if json {
        println!("{}", encode_json(&picked, "bt disconnect")?);
    } else {
        println!(
            "{}",
            archel_core::bluetooth::disconnected_ok_line(&picked)
        );
    }
    Ok(())
}

fn encode_json(payload: &impl serde::Serialize, what: &str) -> Result<String, ArchelError> {
    serde_json::to_string(payload)
        .map_err(|err| ArchelError::Internal(format!("cannot encode {what} JSON: {err}")))
}

/// Right-pads a label on char count (mirrors the `net_cmds` helper —
/// byte widths would misalign accented French labels).
fn pad_right(label: &str, width: usize) -> String {
    let mut out = label.to_string();
    while out.chars().count() < width {
        out.push(' ');
    }
    out
}

/// French state cell: connected wins over paired over known.
fn state_cell(device: &BtDevice) -> String {
    archel_core::bluetooth::state_label(device)
}

/// French table for one device snapshot; empty input renders the
/// honest-empty line (never a fake row).
fn render_devices_table(devices: &[BtDevice]) -> String {    if devices.is_empty() {
        return format!("{}\n", archel_core::bluetooth::devices_empty_line());
    }
    let (name_label, mac_label, signal_label, state_label) =
        archel_core::bluetooth::device_columns();
    let name_width = devices
        .iter()
        .map(|row| row.name.chars().count())
        .chain(std::iter::once(name_label.chars().count()))
        .max()
        .unwrap_or(0);
    let mut out = format!("{}\n", archel_core::bluetooth::devices_title());
    out.push_str(&format!(
        "{}  {}  {}  {}\n",
        pad_right(&name_label, name_width),
        pad_right(&mac_label, 17),
        pad_right(&signal_label, 9),
        state_label
    ));
    for row in devices {
        out.push_str(&format!(
            "{}  {}  {}  {}\n",
            pad_right(&row.name, name_width),
            pad_right(&row.address, 17),
            pad_right(&archel_core::bluetooth::signal_cell(row.rssi), 9),
            state_cell(row)
        ));
    }
    out
}

/// French numbered live list for a scan sweep (BT-01): the 1-based
/// index is the `pair/connect/disconnect` pick, and every row shows
/// name plus MAC plus signal (T-4-05). Empty input renders the
/// honest-empty line, never a fake row.
fn render_scan_table(devices: &[BtDevice]) -> String {
    if devices.is_empty() {
        return format!("{}\n", archel_core::bluetooth::scan_empty_line());
    }
    let (name_label, mac_label, signal_label, _) =
        archel_core::bluetooth::device_columns();
    let name_width = devices
        .iter()
        .map(|row| row.name.chars().count())
        .chain(std::iter::once(name_label.chars().count()))
        .max()
        .unwrap_or(0);
    let index_width = format!("{}", devices.len()).len().max(1);
    let mut out = format!("{}\n", archel_core::bluetooth::scan_title());
    out.push_str(&format!(
        "{:>index_width$}  {}  {}  {}\n",
        "#",
        pad_right(&name_label, name_width),
        pad_right(&mac_label, 17),
        pad_right(&signal_label, 9),
        index_width = index_width,
    ));
    for (position, row) in devices.iter().enumerate() {
        out.push_str(&format!(
            "{:>index_width$}  {}  {}  {}\n",
            position + 1,
            pad_right(&row.name, name_width),
            pad_right(&row.address, 17),
            pad_right(&archel_core::bluetooth::signal_cell(row.rssi), 9),
            index_width = index_width,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_row() -> BtDevice {
        BtDevice {
            address: "AA:BB:CC:DD:EE:FF".to_string(),
            name: "Mouse".to_string(),
            rssi: Some(-50),
            paired: true,
            connected: false,
        }
    }

    #[test]
    fn devices_table_names_name_mac_and_signal() {
        archel_core::i18n::set_language("fr");
        let table = render_devices_table(&[sample_row()]);
        assert!(
            table.contains("AA:BB:CC:DD:EE:FF"),
            "table must show the MAC (T-4-05): {table:?}"
        );
        assert!(table.contains("Mouse"), "table must show the name");
        assert!(table.contains("-50 dBm"), "table must show the signal");
    }

    #[test]
    fn devices_empty_table_is_honest_without_fake_rows() {
        archel_core::i18n::set_language("fr");
        let table = render_devices_table(&[]);
        assert!(!table.contains(':'), "empty table must hold no MAC: {table:?}");
        assert!(
            table.contains(&archel_core::bluetooth::devices_empty_line()),
            "empty table must print the honest line"
        );
    }

    #[test]
    fn devices_json_pins_row_fields() {
        let value = serde_json::to_value(vec![sample_row()]).expect("serializes");
        let row = &value[0];
        for field in ["address", "name", "rssi", "paired", "connected"] {
            assert!(
                row.get(field).is_some(),
                "bt devices --json must pin `{field}`"
            );
        }
        assert_eq!(row["address"], "AA:BB:CC:DD:EE:FF");
    }

    #[test]
    fn scan_table_numbers_rows_with_name_mac_and_signal() {
        archel_core::i18n::set_language("fr");
        let table = render_scan_table(&[sample_row()]);
        assert!(table.contains('1'), "scan list must number picks: {table:?}");
        assert!(
            table.contains("AA:BB:CC:DD:EE:FF"),
            "scan list must show the MAC (T-4-05): {table:?}"
        );
        assert!(table.contains("Mouse"), "scan list must show the name");
        assert!(table.contains("-50 dBm"), "scan list must show the signal");
    }

    #[test]
    fn scan_empty_table_is_honest_without_fake_rows() {
        archel_core::i18n::set_language("fr");
        let table = render_scan_table(&[]);
        assert!(!table.contains(':'), "empty scan must hold no MAC: {table:?}");
        assert!(
            table.contains(&archel_core::bluetooth::scan_empty_line()),
            "empty scan must print the honest line"
        );
    }
}
