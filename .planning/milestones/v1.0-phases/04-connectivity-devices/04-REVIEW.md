# Phase 04 — Connectivity & Devices: Code Review

**Scope:** all Phase 4 source changes on `master` since `56188ff^`
(`net status`/DNS/test, proxy via gsettings, VPN, BlueZ scan/pair,
wpctl audio switching, power profiles, CLI cmds, error taxonomy, fr.json).
**Method:** full read of every file in scope, cross-checked parsers against
each other and validators against every construction site.
**Status:** issues_found — 4 high, 6 medium, 10 low. No critical
(remote-code-execution / auth-bypass) findings.

## High

### H-01 — Proxy password travels on the process command line
**File:** `crates/archel-core/src/proxy.rs:454-459`, `483-511`

`proxy_write_argv` embeds the cleartext password as an argv element
(`authentication-password '<pw>'`) and `run_gsettings_write` spawns it via
`duct::cmd`. Any local user can read `/proc/<pid>/cmdline` while the
`gsettings set` child lives. The module is careful elsewhere (no-echo
prompt, redacted `Debug`, secret-free reports) but the final hop publishes
the secret to the whole machine. `gsettings` offers no stdin-value mode,
so document the exposure in the confirm prompt/help, and consider clearing
the string (`zeroize`) immediately after the spawn instead of holding it in
the cloned `ProxyPlan`.

### H-02 — GVariant quoting assumes validation that only covers `host`
**File:** `crates/archel-core/src/proxy.rs:310-326`, `340-353`, `401-420`, `448-461`

`gsettings_string` wraps values in single quotes with only a
`debug_assert!` against embedded quotes — a no-op in release builds.
`host` and `pac_url` are validated quote-free, but three values reaching
the same quoting function are not:

- `user` (`plan_proxy_manual`, line 382: `user.map(str::to_string)`, no validation),
- `exceptions` entries (`parse_exceptions`, line 357: splits/commas only),
- `password` (never validated by design).

A proxy password containing an apostrophe (very common: `l'été-2026`)
produces `'…l'été-2026'` — gsettings rejects it or stores a truncated
value, so password-auth is silently broken for a large password class.
A crafted `user`/`exceptions` value breaks the strv shape the same way.
Fix: extend the quote/whitespace/control rejection to `user`, each
`exceptions` entry, and fail-closed on `password` (or single-quote-escape
per GVariant `'…\\'…'` rules in one place, covered by unit tests).

### H-03 — Audio switch uses an ephemeral numeric ID across the confirm gate
**File:** `crates/archel-core/src/audio.rs:320-335`, `360-376`, `339-354`, `380-396`

The module's own threat model (T-4-07) states numeric PipeWire IDs
"renumber every restart" and must "never [be] matched across runs" — yet
`set_output_picked_with` resolves name→ID, blocks on an interactive
`[o/N]` confirm (unbounded human pause), then calls `set(id)` with the
pre-confirm ID. If the graph re-enumerates during the pause (device
unplug/replug, PipeWire restart, Bluetooth reconnect), `wpctl set-default`
targets a stale ID: the wrong device switches, or a recycled ID switches
something unrelated. The post-switch read-back only checks the *name* is
default, which can then fail spuriously. Fix: re-list after confirm and
re-resolve the name to a fresh ID (resolve→confirm→re-list→resolve→set),
failing honestly if the name vanished.

### H-04 — NM terse parsers left-split on `:`, breaking names containing colons
**File:** `crates/archel-core/src/network.rs:485-511`, `938-951`

`parse_active_connections` uses `splitn(4, ':')` from the left and
`parse_profiles` uses `split_once(':')`, so a connection/profile name
containing `:` (legal in NM; SSIDs routinely contain them — the wifi
parser at line 566 explicitly handles `corp:lobby:guest`) shifts every
field: the "UUID" becomes garbage, resolution fails or, worse, matches the
wrong profile, and the DNS write path operates on a UUID derived from a
misaligned row — exactly the T-4-01 profile-confusion threat. The VPN
parser in the same file (`parse_terse_row`, line 2054) already does this
correctly (right-split + `\:` unescape). Fix: reuse the right-split
`parse_terse_row` logic for all three NM terse parsers.

## Medium

### M-01 — VPN up/down reports success without verifying state
**File:** `crates/archel-core/src/network.rs:2152-2178`

`vpn_up` returns `active: true` (and `vpn_down` `active: false`) purely
because the `nmcli` spawn exited 0 — no re-query of the active set. Every
sibling mutation in this phase reads back (DNS effective-DNS, audio
default, power re-read, PPD read-back). A tunnel that fails to come up
asynchronously (auth, peer) is reported active. Fix: re-run the active-set
query and set `active` from membership, degrading to `None`/honest-unknown
if the re-read fails.

### M-02 — PPD write denials surface as generic `Internal`, not `PolkitDenied`
**File:** `crates/archel-core/src/power.rs:302-313`

`ppd_set` maps `set_property` failures (including polkit refusal on a
locked-down PPD) to `ArchelError::Internal`, bypassing the typed
`PolkitDenied` + hyprpolkitagent hint that the NM write path (`M-`
`map_nm_write_error`) and the cpufreq path (`write_governors_at`) both
produce. Same refusal, two different user experiences depending on which
backend served the machine. Fix: match access-denied/not-authorized
markers in the zbus error and map to `PolkitDenied`.

### M-03 — DNS read-back reflects the active profile, not the modified one
**File:** `crates/archel-core/src/network.rs:1088-1109`

`apply_dns_plan` reports `effective_dns` from `net_status_nmcli()`, which
reads the *currently active* connection. For `--profile <inactive>` writes
the report shows a different profile's DNS as the "effective" outcome of
this mutation — misleading confirmation. Fix: read back the modified
profile's effective DNS (or `None`/N/A with a note when it is not the
active profile).

### M-04 — Governor writes are not atomic across policies
**File:** `crates/archel-core/src/power.rs:206-226`

`write_governors_at` writes `policy*/scaling_governor` in a loop and
returns on first error — `policy0` may already carry the new governor
while `policy1+` keep the old one, yet the caller reports a single
`PolkitDenied`/`Internal`. Heterogeneous governors also confuse the
first-sorted-policy-wins read (L-06). Fix: pre-check writability of all
targets before writing any, and on mid-loop failure report which policies
changed (or restore).

### M-05 — Password is prompted before host/port validation
**File:** `crates/archel/src/cmds/net_cmds.rs:130-153`

`run_proxy_manual` reads the store, prompts for the no-echo password, and
only then builds the plan that validates `host`/`port`. A typo'd host
costs the user a secret prompt whose value is immediately discarded (and
needlessly touched). Fix: validate host/port/exceptions (and PAC URL in
`run_proxy_auto` — already fine) before `prompt_proxy_password`.

### M-06 — Untrusted daemon/store strings rendered without terminal sanitization
**File:** `crates/archel-core/src/proxy.rs:197-211`, `crates/archel-core/src/network.rs:688-722`, `crates/archel-core/src/bluetooth.rs`, `crates/archel-core/src/audio.rs`

SSID, profile names, DNS strings, BT names, PAC URLs and audio
descriptions are "shown verbatim" (documented as safe because "never
passed to a shell"). But the terminal *is* an interpreter: `status_line`
flattens `\n`/`\r` yet passes `\x1b` (ANSI escapes), C1 controls and
Unicode bidi overrides straight through, and the table renderers do not
even flatten newlines. A rogue AP/VPN profile/BT alias can repaint or
spoof terminal lines. Fix: one shared `sanitize_display` helper (strip
`\x1b`, C0/C1, bidi overrides; flatten newlines) applied at every
table/line renderer boundary.

## Low

### L-01 — VPN up/down lists and resolves twice (TOCTOU + wasted spawns)
**File:** `crates/archel/src/cmds/net_cmds.rs:232-267`, `crates/archel-core/src/network.rs:2152-2178`

`run_vpn_up`/`run_vpn_down` fetch `vpn_list`, resolve, then hand the UUID
to `core_vpn::vpn_up/down`, which list and resolve *again*. The profile set
can change between the two lists. Fix: add `vpn_up_uuid`/`vpn_down_uuid`
(or accept the resolved struct) so resolution happens once.

### L-02 — Dead `confirm_vpn_prompt("import", …)` arm renders an empty kind
**File:** `crates/archel-core/src/network.rs:2409-2416`

The `"import"` arm interpolates `kind = ""`, producing
`Importer « x » () ?`. `net_cmds` never calls it (uses
`confirm_vpn_import_prompt`), so it is latent — but the next caller gets a
broken prompt. Fix: delete the arm or delegate to
`confirm_vpn_import_prompt`.

### L-03 — Hardcoded French fragment inside a core error payload
**File:** `crates/archel-core/src/audio.rs:289-305`

The ambiguous-duplicate branch builds the `AudioDeviceNotFound.query` as
`"{name} (candidats : {candidates})"` — French copy in Rust source,
bypassing the catalog (D-05/D-07) and then interpolated into the catalog
`audio_device_not_found` string. Fix: carry candidates structurally (or
English `candidates:` log detail) and render the French label from
`fr.json`.

### L-04 — Dead catalog keys `net.label_target` / `net.label_latency`
**File:** `crates/archel-core/locales/fr.json:148-149`

`test_rows` emits only `hop (target)` + latency cells; nothing references
these two keys (confirmed by repo-wide grep). Fix: remove them or wire them
into the test table.

### L-05 — `VpnImportFailed.file` receives an action string by construction
**File:** `crates/archel-core/src/network.rs:2259-2284`, `2333-2347`

`map_vpn_import_error` sets `file: action.to_string()`. It is correct
today only because `vpn_import` passes the path as `action`. Any future
caller passing a real action label_poison the `file` field shown in
`vpn_import_failed`. Fix: take `file` and `action` as separate parameters.

### L-06 — Heterogeneous governors collapse to first-sorted-policy silently
**File:** `crates/archel-core/src/power.rs:156-176`

`first_governor_at` reports one profile from the lexicographically first
`policy*` dir; mixed governors across policies (common on hybrid/big.LITTLE
or after M-04 partial writes) are reported as a single confident profile.
Fix: detect disagreement and report mixed/unknown honestly.

### L-07 — PAC-URL userinfo credentials echo to terminal and `--json`
**File:** `crates/archel-core/src/proxy.rs:197-233`

Auto-mode detail shows `autoconfig_url` verbatim in the status line and
`ProxyApplyReport.endpoint`. A `http://user:pass@host/proxy.pac` URL leaks
credentials to scrollback, logs and scripts. Fix: redact userinfo at the
display/report boundary (parse and show host/path only).

### L-08 — `ProxyPlan` public fields bypass all validation
**File:** `crates/archel-core/src/proxy.rs:257-274`

Every field is `pub` and `proxy_write_argv` trusts them (only a
`debug_assert` guards quoting), so any in-tree constructor can smuggle
quote-bearing values past H-02's validators. Fix: make fields private with
validated constructors (`off` already exists as the pattern).

### L-09 — Password held as plain `String`, never zeroized
**File:** `crates/archel-core/src/proxy.rs:257-289`

`Debug` redaction and secret-free reports are done well, but the password
lives as a clonable `String` with no `zeroize` on drop — it lingers in
memory (and in any core dump) after the spawn. Fix: wrap in `zeroize::Zeroizing`.

### L-10 — Scan only merges `DeviceAdded`, missing cache and signal updates
**File:** `crates/archel-core/src/bluetooth.rs:383-423`

Devices BlueZ already knew before discovery starts, and RSSI updates on
`PropertyChanged`, never enter `sightings` — the live list can omit nearby
devices or show stale signal. Fix: seed from `device_addresses()` at sweep
start and/or fold `PropertyChanged` RSSI into merged rows.

## Notes (checked, no finding)

- `nmcli -t` general/state parsing: terse single-field queries emit bare
  values, so `general_word == "connected"` and `parse_general_state` are
  live, not dead — no bug.
- VPN import path confinement (raw + canonical forbidden-prefix checks,
  extension allowlist, `is_file`) is sound; `.ovpn`-named symlinks to
  non-configs fail cleanly inside the plugin, which never executes file
  content as code. `nmcli --ask` keeps VPN secrets out of Archel argv —
  the correct pattern H-01 asks proxy to follow structurally.
- BT scan-then-pick (`resolve_pick` rejects free-form MACs/names),
  confirm-before-pair with no auto-accept agent constructor, and
  constant-shape passkey handling are the strongest parts of this phase.
- `fr.json` additions: all `t!` keys referenced by the new modules exist;
  diacritics are correct throughout (`connecté`, `chaîne`-class words,
  `Vérifiez`, `affichée`); no regression found.
