---
status: complete
phase: 02-system-insight
source: [02-01-SUMMARY.md, 02-02-SUMMARY.md, 02-03-SUMMARY.md]
started: 2026-10-01T00:10:00Z
updated: 2026-10-01T00:25:00Z
---

## Current Test
<!-- OVERWRITE each test - shows where we are -->

number: 0
name: [testing complete]
expected: |
  [testing complete]
awaiting: none

## Tests

### 1. Forme CLI — groupe sys + landing + --json global
expected: --help shows sys group (cpu/ram/disk/gpu/temp/power/all) + top-level landing + global --json, matching approved D-11 shape
result: pass

### 2. Watch à 500 ms — réaffichage sans scintillement
expected: `sys cpu --watch` re-renders about twice per second, readable without flicker; Ctrl+C exits cleanly
result: pass

### 3. Sans checkupdates — indice français guidé
expected: `landing` shows the guided French hint to install pacman-contrib (exit 0, no blank, no crash)
result: pass
note: "Superseded live — user installed pacman-contrib mid-UAT; full path now proven (67 repo + 3 AUR, --json parses). Graceful missing-checker state was proven pre-install by unit tests + verifier live observation."

### 4. archel landing prints one French table with app version, kernel, pending updates
expected: archel landing prints one French table with app version, kernel, pending updates
result: pass
source: automated
coverage_id: D1

### 5. archel landing --json prints one parseable JSON object (global flag, both orders)
expected: archel landing --json prints one parseable JSON object (global flag, both orders)
result: pass
source: automated
coverage_id: D2

### 6. archel landing --refresh forces recompute bypassing the cache
expected: archel landing --refresh forces recompute bypassing the cache
result: pass
source: automated
coverage_id: D3

### 7. Offline / missing-checker landing degrades to stale counts with age, never blank-or-crash
expected: Offline / missing-checker landing degrades to stale counts with age, never blank-or-crash
result: pass
source: automated
coverage_id: D4

### 8. Corrupt or version-skewed cache is discarded and recomputed, never crashed on
expected: Corrupt or version-skewed cache is discarded and recomputed, never crashed on
result: pass
source: automated
coverage_id: D5

### 9. archel sys cpu prints per-core usage percentages plus MHz clocks in a French table
expected: archel sys cpu prints per-core usage percentages plus MHz clocks in a French table
result: pass
source: automated
coverage_id: D1

### 10. archel sys ram and archel sys disk print RAM used/total plus system-partition detail
expected: archel sys ram and archel sys disk print RAM used/total plus system-partition used/total/free
result: pass
source: automated
coverage_id: D2

### 11. Any sys metric with --watch re-renders every 500 ms; --watch --json emits NDJSON
expected: Any sys metric with --watch re-renders every 500 ms; with --watch --json emits one parseable JSON object per tick
result: pass
source: automated
coverage_id: D3

### 12. archel sys gpu prints model plus VRAM used/total (GTX 1070 Ti 8 Gio live)
expected: archel sys gpu prints model plus VRAM used/total (GTX 1070 Ti 8 Gio live)
result: pass
source: automated
coverage_id: D1

### 13. archel sys temp prints one hottest value per category (CPU Tccd1 wins, GPU NVML)
expected: archel sys temp prints one hottest value per category (CPU Tccd1 wins, GPU NVML)
result: pass
source: automated
coverage_id: D2

### 14. archel sys power prints real-time GPU watts plus honest CPU N/A (root-only RAPL)
expected: archel sys power prints real-time GPU watts plus honest CPU N/A (root-only RAPL)
result: pass
source: automated
coverage_id: D3

### 15. archel sys all composes every metric into one table and one --json object
expected: archel sys all composes every metric into one table and one --json object
result: pass
source: automated
coverage_id: D4

### 16. Every --json schema (landing + all sys metrics) pinned by insta snapshots
expected: Every --json schema (landing + all sys metrics) pinned by insta snapshots
result: pass
source: automated
coverage_id: D5

## Summary

total: 16
passed: 16
issues: 0
pending: 0
skipped: 0

## Gaps

[none yet]
