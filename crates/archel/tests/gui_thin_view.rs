//! PLAT-03 thin-view coverage audit (phase 06-09).
//!
//! This test proves each of the 10 GUI tabs is a thin view over the
//! CLI-proven core: every user-facing entry point in
//! `src/cmds/*_cmds.rs` (plus `landing_cmd.rs`) maps to an invokable
//! on its tab's model, and no tab implements a system mutation that
//! is not backed by an `archel-core` call.
//!
//! Pure source-text scan — no Qt runtime, no cxx-qt, no D-Bus. It runs
//! in the plain `--no-default-features` suite so CI gates the
//! phase-goal criterion "each tab a thin view over CLI-proven core".
//!
//! Three gates:
//!  1. `mapped_invokables_exist` — every curated (cmd, model,
//!     invokable) triple names a real `pub fn` in
//!     `src/gui/models/<tab>.rs`.
//!  2. `no_subprocess_or_ipc_in_gui` — `src/gui/**/*.rs` contains no
//!     subprocess or IPC primitives (`std::process`, `Command::new`,
//!     `tokio::process`, `duct`, `zbus`, direct `hyprland` crate use).
//!     All Hyprland access goes through `archel_core::hyprland`
//!     (core-backed, CLI-proven); the binary links no `hyprland` /
//!     `zbus` / `duct` crate directly (Cargo.toml check). The one
//!     deliberate socket in gui/ — `tokio::net::UnixListener` in
//!     `single_instance.rs` (D-26 second-instance refusal) — is
//!     frontend lifecycle, not a system mutation, and stays allowed.
//!  3. `annotation_drift_gate` — every `pub run_*` / `begin_*` fn in
//!     cmds/ carries exactly one of `// gui: <tab>.<invokable>[, ...]`
//!     or `// cli-only: <reason>`, and the `// gui:` set agrees
//!     two-ways with the curated [`MAP`] below. Add a tab control for
//!     a CLI capability and you must extend both the annotation and
//!     the map, or this gate goes red.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    let path = crate_root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("cannot read {rel}: {err}"))
}

/// Curated tab<->CLI capability map: (cmds file stem, CLI entry fn,
/// model file stem, model invokable). Reads (`*_list`, bare status
/// views) map to the tab's 1 Hz `tick`, which is the GUI read path;
/// writes map to the invokable that spawns the confirm-gated worker.
/// Multi-leg capabilities list every leg (`run_aur` review + build,
/// `run_mouse_set` sensitivity + acceleration).
const MAP: &[(&str, &str, &str, &str)] = &[
    // Landing (1)
    ("landing_cmd", "run_landing", "landing", "tick"),
    // System Info (7): one rolling snapshot behind `tick`.
    ("sys_cmds", "run_cpu", "system", "tick"),
    ("sys_cmds", "run_ram", "system", "tick"),
    ("sys_cmds", "run_disk", "system", "tick"),
    ("sys_cmds", "run_gpu", "system", "tick"),
    ("sys_cmds", "run_temp", "system", "tick"),
    ("sys_cmds", "run_power", "system", "tick"),
    ("sys_cmds", "run_all", "system", "tick"),
    // Apps/Packages (8)
    ("pkg_cmds", "run_list", "pkg", "tick"),
    ("pkg_cmds", "run_search", "pkg", "search"),
    ("pkg_cmds", "run_info", "pkg", "select"),
    ("pkg_cmds", "run_install", "pkg", "install_start"),
    ("pkg_cmds", "run_remove", "pkg", "remove_start"),
    ("pkg_cmds", "run_upgrade", "pkg", "upgrade_start"),
    ("pkg_cmds", "run_aur", "pkg", "aur_review_start"),
    ("pkg_cmds", "run_aur", "pkg", "aur_build_start"),
    // Network (5)
    ("net_cmds", "run_status", "net", "tick"),
    ("net_cmds", "run_dns_show", "net", "tick"),
    ("net_cmds", "run_dns_set", "net", "dns_apply"),
    ("net_cmds", "run_vpn_list", "net", "tick"),
    ("net_cmds", "run_vpn_import", "net", "vpn_import_start"),
    // Power (2)
    ("power_cmds", "run_status", "power", "tick"),
    ("power_cmds", "run_set", "power", "set_profile"),
    // Bluetooth (5)
    ("bt_cmds", "run_devices", "bt", "tick"),
    ("bt_cmds", "run_scan", "bt", "scan_now"),
    ("bt_cmds", "run_pair", "bt", "pair"),
    ("bt_cmds", "run_connect", "bt", "connect"),
    ("bt_cmds", "run_disconnect", "bt", "disconnect"),
    // Sound (3)
    ("audio_cmds", "run_list", "audio", "tick"),
    ("audio_cmds", "run_set_output", "audio", "set_default_output"),
    ("audio_cmds", "run_set_input", "audio", "set_default_input"),
    // Appearance (7): reads surface through `tick`, writes per knob.
    ("appearance_cmds", "run_blur", "appearance", "set_blur"),
    ("appearance_cmds", "run_opacity_set", "appearance", "set_opacity"),
    ("appearance_cmds", "run_opacity", "appearance", "tick"),
    ("appearance_cmds", "run_font", "appearance", "tick"),
    ("appearance_cmds", "run_font_set", "appearance", "set_font"),
    ("appearance_cmds", "run_wallpaper", "appearance", "tick"),
    ("appearance_cmds", "run_wallpaper_set", "appearance", "set_wallpaper"),
    // Display (2): `plan_change` opens the countdown modal legs.
    ("display_cmds", "run_monitors", "display", "tick"),
    ("display_cmds", "run_display_set", "display", "plan_change"),
    // Input (7): `run_keybind_edit` is cli-only (see annotations).
    ("input_cmds", "run_keybinds_list", "input", "tick"),
    ("input_cmds", "run_keybind_add", "input", "add_keybind"),
    ("input_cmds", "run_keybind_remove", "input", "remove_keybind"),
    ("input_cmds", "run_mouse", "input", "tick"),
    ("input_cmds", "run_mouse_set", "input", "set_sensitivity"),
    ("input_cmds", "run_mouse_set", "input", "set_accelerate"),
];

/// Cmds files carrying user-facing entry points (`*_cmds.rs` twins
/// plus the landing singleton). Render helpers (`render_*`) and the
/// top-level `run(cli)` dispatcher in `mod.rs` are out of scope by
/// design: they are formatting/plumbing, not capabilities.
const CMDS_FILES: &[&str] = &[
    "appearance_cmds",
    "audio_cmds",
    "bt_cmds",
    "display_cmds",
    "input_cmds",
    "landing_cmd",
    "net_cmds",
    "pkg_cmds",
    "power_cmds",
    "sys_cmds",
];

fn is_entry_fn(line: &str) -> Option<String> {
    // Top-level only (`^pub`): test-module helpers are indented and
    // never capabilities.
    let rest = line.strip_prefix("pub ")?;
    let rest = rest.strip_prefix("async ").unwrap_or(rest);
    let rest = rest.strip_prefix("fn ")?;
    let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
    if name.starts_with("run_") || name.starts_with("begin_") {
        Some(name)
    } else {
        None
    }
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("gui dir must list") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn mapped_invokables_exist() {
    let mut missing = Vec::new();
    let mut cache: HashMap<String, String> = HashMap::new();
    for (cmd_file, cmd_fn, model, invokable) in MAP {
        let src = cache
            .entry(model.to_string())
            .or_insert_with(|| read(&format!("src/gui/models/{model}.rs")));
        if !src.contains(&format!("pub fn {invokable}(")) {
            missing.push(format!("{cmd_file}::{cmd_fn} -> {model}.{invokable}: no `pub fn {invokable}(` in src/gui/models/{model}.rs"));
        }
    }
    assert!(
        missing.is_empty(),
        "thin-view map points at missing invokables:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn no_subprocess_or_ipc_in_gui() {
    // Literals that must never appear in gui/ sources, no whitelists.
    const FORBIDDEN: &[&str] = &[
        "std::process",
        "Command::new",
        "tokio::process",
        "duct::",
        "use duct",
        "extern crate duct",
        "zbus::",
        "use zbus",
        "extern crate zbus",
        "use hyprland::",
        "use hyprland;",
        "extern crate hyprland",
    ];
    let mut files = Vec::new();
    collect_rs(&crate_root().join("src/gui"), &mut files);
    assert!(!files.is_empty(), "src/gui must contain .rs files");
    let mut violations = Vec::new();
    let mut hypr_users: Vec<String> = Vec::new();
    for path in &files {
        let rel = path.strip_prefix(crate_root()).unwrap().display().to_string();
        let src = std::fs::read_to_string(path).expect("gui source must read");
        for (idx, line) in src.lines().enumerate() {
            for lit in FORBIDDEN {
                if line.contains(lit) {
                    violations.push(format!("{}:{}: forbidden `{lit}`", rel, idx + 1));
                }
            }
        }
        // Any `hyprland::` path must resolve through the core
        // re-export: the binary links no `hyprland` crate itself
        // (Cargo.toml gate below), so a file using `hyprland::`
        // without `archel_core::hyprland` would not even compile —
        // this assertion pins the reason, not just the accident.
        if src.contains("hyprland::") && !src.contains("archel_core::hyprland") {
            hypr_users.push(rel.clone());
        }
    }
    assert!(
        violations.is_empty(),
        "gui/ must stay subprocess/IPC-free (mutations live in core/cmds):\n  {}",
        violations.join("\n  ")
    );
    assert!(
        hypr_users.is_empty(),
        "direct hyprland-crate paths outside archel_core::hyprland:\n  {}",
        hypr_users.join("\n  ")
    );

    // The binary crate itself must not link IPC/subprocess crates
    // directly: gui/ can only reach the system through archel-core.
    let manifest = read("Cargo.toml");
    let in_deps = manifest
        .split("[dependencies]")
        .nth(1)
        .unwrap_or("")
        .split('[')
        .next()
        .unwrap_or("");
    for dep in ["hyprland", "zbus", "duct"] {
        assert!(
            !in_deps.lines().any(|l| l.trim_start().starts_with(dep)
                && l.contains('=')),
            "crates/archel must not depend on `{dep}` directly (gui reaches the system via archel-core only)"
        );
    }
}

#[test]
fn annotation_drift_gate() {
    // (file, fn) -> gui targets + cli-only reason, parsed from the
    // comment lines directly above each top-level entry fn.
    let mut gui_ann: HashSet<(String, String, String, String)> = HashSet::new();
    let mut cli_only: HashSet<(String, String)> = HashSet::new();
    let mut unannotated = Vec::new();
    let mut double = Vec::new();
    for file in CMDS_FILES {
        let src = read(&format!("src/cmds/{file}.rs"));
        let lines: Vec<&str> = src.lines().collect();
        for (idx, line) in lines.iter().enumerate() {
            let Some(fn_name) = is_entry_fn(line) else {
                continue;
            };
            let mut targets: Vec<(String, String)> = Vec::new();
            let mut reason: Option<String> = None;
            // Contiguous `//` comment block directly above the fn.
            let mut j = idx;
            while j > 0 {
                let prev = lines[j - 1].trim_start();
                if !prev.starts_with("//") {
                    break;
                }
                if let Some(rest) = prev.strip_prefix("// gui:") {
                    for item in rest.split(',') {
                        let item = item.trim();
                        if let Some((tab, inv)) = item.split_once('.') {
                            targets.push((tab.trim().to_string(), inv.trim().to_string()));
                        }
                    }
                } else if let Some(rest) = prev.strip_prefix("// cli-only:") {
                    reason = Some(rest.trim().to_string());
                }
                j -= 1;
            }
            let key = (file.to_string(), fn_name.clone());
            match (targets.is_empty(), reason) {
                (true, None) => unannotated.push(format!("{file}::{fn_name}")),
                (false, Some(_)) => double.push(format!("{file}::{fn_name}")),
                (false, None) => {
                    for (tab, inv) in targets {
                        gui_ann.insert((file.to_string(), fn_name.clone(), tab, inv));
                    }
                }
                (true, Some(r)) => {
                    assert!(
                        !r.is_empty(),
                        "{file}::{fn_name} has an empty // cli-only: reason"
                    );
                    cli_only.insert(key);
                }
            }
        }
    }
    assert!(
        unannotated.is_empty(),
        "unannotated CLI capabilities (add // gui: or // cli-only:):\n  {}",
        unannotated.join("\n  ")
    );
    assert!(
        double.is_empty(),
        "commands with both // gui: and // cli-only: (pick one):\n  {}",
        double.join("\n  ")
    );

    // Two-way agreement between annotations and the curated map.
    let map_set: HashSet<(String, String, String, String)> = MAP
        .iter()
        .map(|(f, c, t, i)| (f.to_string(), c.to_string(), t.to_string(), i.to_string()))
        .collect();
    let ann_not_mapped: Vec<String> = gui_ann
        .difference(&map_set)
        .map(|(f, c, t, i)| format!("{f}::{c} -> {t}.{i}"))
        .collect();
    let mapped_not_ann: Vec<String> = map_set
        .difference(&gui_ann)
        .map(|(f, c, t, i)| format!("{f}::{c} -> {t}.{i}"))
        .collect();
    assert!(
        ann_not_mapped.is_empty(),
        "// gui: targets missing from the MAP table:\n  {}",
        ann_not_mapped.join("\n  ")
    );
    assert!(
        mapped_not_ann.is_empty(),
        "MAP entries missing from // gui: annotations:\n  {}",
        mapped_not_ann.join("\n  ")
    );

    // The map must stay honest about cli-only surface: every
    // cli-only command must NOT appear in the map.
    let mapped_cmds: HashSet<(String, String)> = map_set
        .iter()
        .map(|(f, c, _, _)| (f.clone(), c.clone()))
        .collect();
    let overlap: Vec<String> = cli_only
        .intersection(&mapped_cmds)
        .map(|(f, c)| format!("{f}::{c}"))
        .collect();
    assert!(
        overlap.is_empty(),
        "commands both cli-only and mapped:\n  {}",
        overlap.join("\n  ")
    );
}
