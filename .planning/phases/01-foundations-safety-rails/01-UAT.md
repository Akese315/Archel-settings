---
status: partial
phase: 01-foundations-safety-rails
source: [01-01-SUMMARY.md, 01-02-SUMMARY.md]
started: 2026-09-30T19:50:00Z
updated: 2026-09-30T23:05:00Z
---

## Current Test
<!-- OVERWRITE each test - shows where we are -->

number: 0
name: [testing complete]
expected: |
  [testing complete]
awaiting: none

## Tests

### 1. Spike window — vertical tabs + live label on Hyprland
expected: Window shows vertical tab rail (Réseau, Alimentation, Bluetooth, Apparence, Audio) + label ticking at 1 Hz
result: pass

### 2. pkexec desktop dialog — allow and cancel paths
expected: `pkexec ./target/debug/archel-privileged dry-run` shows the desktop auth dialog; allowing prints v1 JSON exit 0, cancelling yields the friendly French cancelled-action error (never raw polkit text)
result: blocked
blocked_by: other
reason: "No polkit session in this terminal (journal: blank-password account + 'No session for cookie'); helper never executed — polkit rejects before exec. Needs a terminal inside the live Hyprland session with a running polkit agent."

### 3. One cargo command builds all three crates headless without Qt
expected: One cargo command builds all three crates headless without Qt
result: pass
source: automated
coverage_id: D1

### 4. archel version prints a French line sourced from archel-core
expected: archel version prints a French line sourced from archel-core
result: pass
source: automated
coverage_id: D2

### 5. Every ArchelError variant renders non-empty French from the catalog; failure paths return Err
expected: Every ArchelError variant renders non-empty French from the catalog; failure paths return Err
result: pass
source: automated
coverage_id: D3

### 6. No hardcoded French diacritics in any .rs file outside locales/
expected: No hardcoded French diacritics in any .rs file outside locales/
result: pass
source: automated
coverage_id: D4

### 7. Allowlisted helper dry-run returns v1 JSON exit 0; smuggled/empty/unknown argv exit 2 with error JSON
expected: Allowlisted helper dry-run returns v1 JSON exit 0; smuggled/empty/unknown argv exit 2 with error JSON
result: pass
source: automated
coverage_id: D1

### 8. pkexec 126/127 map to distinct friendly French AuthFailed errors naming the action; stderr log-only
expected: pkexec 126/127 map to distinct friendly French AuthFailed errors naming the action; stderr log-only
result: pass
source: automated
coverage_id: D2

### 9. Polkit policy draft: dry-run action, auth_admin_keep triple, never yes
expected: Polkit policy draft: dry-run action, auth_admin_keep triple, never yes
result: pass
source: automated
coverage_id: D3

## Summary

total: 9
passed: 8
issues: 0
pending: 0
skipped: 0
blocked: 1

## Gaps

- gap_id: G-01-1
  truth: "Window shows vertical tab rail + label ticking at 1 Hz via qrc:/ QML load"
  status: resolved
  reason: "User reported: QQmlApplicationEngine failed to load component — qrc:/qt/qml/org/archel/spike/qml/Spike.qml: No such file or directory"
  severity: major
  test: 1
  resolved_by: 01-03-SUMMARY.md
  resolved_at: 2026-09-30
  diagnosis: "qrc alias mismatch. Generated qrc (target/.../qml_module_resources_org_archel_spike.qrc) registers Spike.qml under alias src/gui/qml/Spike.qml (crate-root-relative path preserved by cxx-qt-build QmlModule::qml_file), i.e. real URL is qrc:/qt/qml/org/archel/spike/src/gui/qml/Spike.qml. gui/mod.rs loads qrc:/qt/qml/org/archel/spike/qml/Spike.qml (missing src/gui/ segment). One-line fix: correct the load URL in crates/archel/src/gui/mod.rs to match the registered alias."
  artifacts: [crates/archel/src/gui/mod.rs]
  missing: []
