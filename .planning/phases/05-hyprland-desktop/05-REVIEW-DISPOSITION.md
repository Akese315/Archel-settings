---
phase: 05-hyprland-desktop
review: 05-REVIEW.md
recorded: 2026-10-10
reviewed: 2026-10-10T12:30:00Z
open: 8 of 16
---

# Phase 05: Code Review Disposition

Review status: `issues_found` — 8 findings (1 critical, 7 warning, 0 info), depth standard.
All 8 current findings were fixed on iteration 1 via `gsd-code-reviewer` → `gsd-code-fixer`
(see `05-REVIEW-FIX.md`, `status: all_fixed`). Advisory only — none block wave completion.

> Note: the 2026-10-08 review (15 findings) was STALE — it predated the 05-02/03/04 commits and
> was fully superseded by the 2026-10-10 review (commit cea63df). Its finding IDs WR-01–WR-07 are
> reused below by the NEW review's findings; the old same-ID rows are gone with the superseded
> review (preserved in git history). Old findings that the new review no longer reports and that
> were never triaged are carried verbatim, marked "not in the current review".

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| CR-01 | critical | fixed | fix 58a0b2a — anchor upsert_monitor_line on exact quoted output name; regression test for prefix-colliding outputs |
| WR-01 | warning | fixed | fix 3b48eaa — appearance parsing scoped to managed block span; hostile-snippet regression test |
| WR-02 | warning | fixed | fix 3b48eaa — left-boundary-anchored key matcher so active_opacity cannot match inside inactive_opacity; order-independent test |
| WR-03 | warning | fixed | fix 4cb3062 — run_rollback collects leg failures, upgrades to HyprlandUnavailable naming the leg; false "restored" gate neutralized |
| WR-04 | warning | fixed | fix 6ac1bbd — unquote made exact inverse of lua_quote (\\/\"); round-trip tests over 7 hostile strings; doc comment corrected |
| WR-05 | warning | fixed | fix bd7bdde — InvalidPackageName pre-spawn guards on search/package_card + `--` terminator on pacman; yay relies on validator alone (deviation recorded: `yay -Si --` hangs) |
| WR-06 | warning | fixed | fix bd7bdde — timed-confirm drops receiver on timeout so late reader's send fails and thread exits; TTY spot-check flagged for human UAT |
| WR-07 | warning | fixed | fix fb0408d — keybind edit live-unbinds old chord via hl.unbindkey when bind identity changes; decision + refusal-gate tests; live ordering flagged for human UAT |
| WR-08 | warning | open | review — carried from superseded 2026-10-08 review; not in the current review |
| WR-09 | warning | open | review — carried from superseded 2026-10-08 review; not in the current review |
| IN-01 | info | open | review — carried from superseded 2026-10-08 review; not in the current review |
| IN-02 | info | open | review — carried from superseded 2026-10-08 review; not in the current review |
| IN-03 | info | open | review — carried from superseded 2026-10-08 review; not in the current review |
| IN-04 | info | open | review — carried from superseded 2026-10-08 review; not in the current review |
| IN-05 | info | open | review — carried from superseded 2026-10-08 review; not in the current review |
| IN-06 | info | open | review — carried from superseded 2026-10-08 review; not in the current review |

Carried rows are untriaged leftovers of the superseded review; the current review does not report
them against the present source. To close them: verify against current code and mark
`deferred` with a reason, or `skipped`.
