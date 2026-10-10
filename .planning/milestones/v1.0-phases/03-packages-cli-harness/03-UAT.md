---
status: complete
phase: 03-packages-cli-harness
source: [03-01-SUMMARY.md, 03-02-SUMMARY.md, 03-03-SUMMARY.md]
started: 2026-10-01T01:05:00Z
updated: 2026-10-06T13:10:00Z
---

## Current Test
<!-- OVERWRITE each test - shows where we are -->

number: 0
name: [testing complete]
expected: |
  [testing complete]
awaiting: none

## Tests

### 1. Install live — confirmation + stream via helper
expected: Pre-install card + [o/N] confirm (empty = No) + pkexec auth + live pacman stream to exit 0, app never as root
result: pass
note: "Redirect proven (`hello` → French `pkg aur` hint, exit 0, no prompt/auth); full stream proven (`blender` reinstall: card with installed-vs-candidate, [o/N], pkexec auth, live pacman to `installe.`). Bonus: `pkg install aur hello` typo → clean French not-found error."

### 2. Remove live — même flow via le helper
expected: `./target/debug/archel pkg remove <le paquet du test 1>` shows card + [o/N], streams `pacman -R` live via pkexec, exit 0
result: pass
note: "Fresh `cowsay` install (download + install stream) then remove: installed-version card, [o/N], live `pacman -R` to `supprime.`. Both directions proven."

### 3. Upgrade live — -Syu complet
expected: `./target/debug/archel pkg upgrade` runs the full -Syu via helper (or reports honestly up-to-date with no prompt); `pkg upgrade <nom>` is refused with the French partial-upgrade message
result: pass
note: "Fully up-to-date → honest no-prompt report; `upgrade firefox` → French partial-upgrade refusal naming the package. (Full -Syu stream untested — machine à jour; flow shares the proven confirm-stream path.)"

### 4. Révision AUR — abort puis confirm
expected: `./target/debug/archel pkg aur <un petit paquet AUR>` shows full PKGBUILD + trust signals; aborting builds nothing; confirming streams `yay -S --noconfirm` with native sudo
result: pass
note: "Both paths live-proven on `hello`: N → `Opération annulée — rien n'a été modifié` (no spawn); o → yay streamed natively (cleanBuild/diffs/makepkg) until user Ctrl+C mid-build; build dir cleaned."

### 5. Complétions — sourcing + tab
expected: `source <(./target/debug/archel completions bash)` then `archel <TAB>` completes subcommands (repeat for zsh/fish if available)
result: pass
note: "bash TAB lists all subcommands + flags live."

### 6. archel pkg list shows every installed package with version in a French table
expected: archel pkg list shows every installed package with version in a French table
result: pass
source: automated
coverage_id: D1

### 7. archel pkg search merges repos + AUR version-descending, repos first on ties
expected: archel pkg search merges repos + AUR version-descending, repos first on ties
result: pass
source: automated
coverage_id: D2

### 8. archel pkg info shows the D-04 pre-install card (sizes, source, candidate vs installed)
expected: archel pkg info shows the D-04 pre-install card (sizes, source, candidate vs installed)
result: pass
source: automated
coverage_id: D3

### 9. Every new pkg subcommand emits one parseable JSON object under --json (D-10)
expected: Every new pkg subcommand emits one parseable JSON object under --json (D-10)
result: pass
source: automated
coverage_id: D4

### 10. Crafted package names (-foo, --overwrite, empty) are rejected with exit 2 and nothing executes
expected: Crafted package names (-foo, --overwrite, empty) are rejected with exit 2 and nothing executes
result: pass
source: automated
coverage_id: D4

### 11. archel completions <bash|zsh|fish> emits working scripts covering every subcommand
expected: archel completions <bash|zsh|fish> emits working scripts covering every subcommand
result: pass
source: automated
coverage_id: D3

### 12. Every Phase 2-3 capability accepts --json (reads emit parseable objects, mutations fast-fail typed)
expected: Every Phase 2-3 capability accepts --json (reads emit parseable objects, mutations fast-fail typed)
result: pass
source: automated
coverage_id: D4

## Summary

total: 12
passed: 12
issues: 0
pending: 0
skipped: 0

## Gaps

- gap_id: G-03-1
  truth: "`pkg install <AUR-only-name>` stops before confirm/helper with a French redirect to `pkg aur <name>` instead of dying obscurely at pacman exit 1"
  status: resolved
  reason: "User reported: `pkg install hello` (Source: aur) flowed into pacman path; helper ran pacman -S, exit 1, generic HelperProtocol-style French error"
  severity: major
  test: 1
  resolved_by: 03-04-SUMMARY.md
  resolved_at: 2026-10-01
  diagnosis: "One-way redirect only: `pkg aur <repo-pkg>` redirects to the pacman path (tested at pkg_cmds.rs:790) but `run_install`/`install_preview` never checks repo presence for candidates. AUR-only names flow through preview (empty sizes/deps) → confirm → helper → pacman exit 1 → generic error. Fix: source check before confirm; AUR-only → French redirect to `pkg aur`, no helper spawn."
  artifacts: [crates/archel/src/cmds/pkg_cmds.rs, crates/archel-core/src/packages.rs]
  missing: []
