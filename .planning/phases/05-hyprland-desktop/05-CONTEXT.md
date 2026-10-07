# Phase 5: Hyprland Desktop - Context

**Gathered:** 2026-10-07
**Status:** Ready for planning

## Phase Boundary

Phase 5 delivers look, monitors, and inputs for Hyprland through one API surface, proven headlessly: wallpaper, system font, blur/opacity live-apply, per-monitor resolution/scale/refresh/layout with confirm-and-rollback, keybind view/add/edit/remove, mouse sensitivity + scrolling. No GUI work; CLI proof gate applies to every write path. Night-light scheduling is out of scope (v2 DSP-05).

## Live Environment (probed 2026-10-07)

- Wallpaper daemon: **hyprpaper 0.8.4 only** (no swww/swaybg); `~/.config/hypr/hyprpaper.conf` + `wallpapers/` dir exist
- Config layout: `hyprland.conf` entry is lua (`hyprland.lua`) + `modules/` dir — snippet must fit this structure
- Monitors: single **HDMI-A-2, 1920x1080@144, scale 1** (multi-monitor layout logic untestable live — fixtures only)
- Fonts: 862 families; GTK font currently `Adwaita Sans 11`

## Implementation Decisions

### Wallpaper (APP-01)
- **D-01:** hyprpaper.conf write + hyprpaper reload (user-approved). No `hyprctl hyprpaper` live path — one mechanism only.

### Font (APP-02)
- **D-02:** GTK + Hyprland scope (user-approved): `gsettings org.gnome.desktop.interface font-name` plus Hyprland `misc:font_family` in the app snippet. Qt (qt6ct/kdeglobals) explicitly out.

### Blur / transparency (APP-03/APP-04)
- **D-03:** decoration blur + opacity settings written to the app-owned snippet, live-applied via `hyprctl keyword` (no full reload).

### Display confirm-and-rollback (DSP-01..04)
- **D-04:** 15 s confirm window (user-approved, GNOME-like). On timeout or explicit reject, previous monitor config re-applied automatically.
- **D-05:** Single-monitor live proof only (HDMI-A-2); multi-monitor layout covered by fixtures + argv-shape tests.

### Input (INP-01..04)
- **D-06:** Keybinds managed in the app snippet only (bind lines); never edit user's own bind blocks. Add/edit/remove by exact-match on dispatcher+key combo.
- **D-07:** Mouse sensitivity + scrolling via `input:` section keys in the snippet, live-applied via `hyprctl keyword`.

### Snippet-file rule (phase invariant)
- **D-08:** Main user config (lua entry, modules, hyprland.conf) is never rewritten. All persistence lands in one app-owned file sourced from the user config (`source=` or lua equivalent) with timestamped backup before every write.

### the agent's Discretion
- Exact `appearance`/`display`/`input` subcommand inventory (EN), snippet filename + source-line insertion point for the lua/modules layout, backup rotation policy, table layouts, JSON field names, hyprctl keyword names per setting, wallpaper path validation rules.

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

- `.planning/REQUIREMENTS.md` — APP-01..04, DSP-01..04, INP-01..04 (DSP-05 night-light explicitly v2)
- Phase 4 precedent: confirm-then-execute + NeedsTty gates, spawn discipline (which-gate, exact argv, LC_ALL=C, timeout+kill), French-table + insta patterns
