//! `archel bt devices` renderer: French table (default) or one JSON
//! array (`--json`).
//!
//! Bin-calls-core shape: every French string comes from
//! `archel-core::bluetooth` (title, labels, state wording, N/A cells) —
//! this module only lays out columns (`std::fmt` widths, no table
//! crate) and frames the JSON array. Daemon-down maps to the typed
//! `BluetoothUnavailable` catalog error, never a panic, never a hang.

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
fn render_devices_table(devices: &[BtDevice]) -> String {
    if devices.is_empty() {
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
}
