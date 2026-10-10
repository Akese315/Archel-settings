# Phase 02: User Setup Required

**Generated:** 2026-09-30
**Phase:** 02-system-insight
**Status:** Incomplete

`archel landing` works without this step (the repo half degrades to a guided hint), but the pacman-repo update count needs one package only a human can install.

## Local System Package

- [ ] **Install `pacman-contrib` (provides `checkupdates`)**
  - Location: local terminal (needs your sudo password — the agent cannot provide it)
  - Run: `sudo pacman -S pacman-contrib`

## Verification

After completing setup, verify with:

```bash
which checkupdates && cargo run -q -p archel --no-default-features -- landing --refresh
```

Expected results:
- `which checkupdates` prints a path (e.g. `/usr/bin/checkupdates`)
- The `Mises à jour` row shows a full breakdown, e.g. `5 (3 dépôts + 2 AUR)`, with no `checkupdates` hint line underneath

---

**Once all items complete:** Mark status as "Complete" at top of file.
