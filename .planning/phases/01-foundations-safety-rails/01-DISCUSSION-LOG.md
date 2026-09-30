# Phase 1: Foundations & Safety Rails - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-30
**Phase:** 1-Foundations & Safety Rails
**Areas discussed:** Binary layout, Language FR/EN, Privilege UX, Spike fallback

---

## Binary layout

| Option | Description | Selected |
|--------|-------------|----------|
| Single binary | `archel settings` ouvre le GUI, le reste est CLI direct — un seul binaire, installation simple | ✓ |
| Separate binaries | `archel-cli` et `archel-settings` installés séparément | |

**User's choice:** Single binary
**Notes:** User's original phrasing "archel-cli et archel settings" was ambiguous — clarified to single `archel` binary. GUI subcommand confirmed as `archel settings` (not `archel gui`).

| Option | Description | Selected |
|--------|-------------|----------|
| `archel settings` | Comme écrit par l'utilisateur | ✓ |
| `archel gui` | `settings` reste libre pour une future commande CLI | |

| Option | Description | Selected |
|--------|-------------|----------|
| Separate helper | Petit binaire dédié via pkexec, surface d'attaque minimale | ✓ |
| Hidden subcommand | Sous-commande cachée du même binaire, un seul fichier | |

| Option | Description | Selected |
|--------|-------------|----------|
| archel-privileged | Explicite sur son rôle pacman-via-pkexec | ✓ |
| archel-helper | Plus court, rôle générique | |
| archel-pkexec | Nomme le mécanisme | |

---

## Language FR/EN

| Option | Description | Selected |
|--------|-------------|----------|
| French | Spec en français, distro perso | ✓ |
| English | Convention outils Arch/Linux | |

**User's choice:** French for displayed text, then freeform refinement: "I want the cli command to be in english" — confirmed as: command/flag names English, human-readable output French.
**Notes:** Two-step clarification — first French selected for CLI+errors, then user refined that command names stay English. i18n-ready infra from Phase 1 (user chose over deferring entirely to v2). Code identifiers English.

| Option | Description | Selected |
|--------|-------------|----------|
| Same: French | Une seule langue partout | ✓ |
| Decide later | GUI indépendant | |

| Option | Description | Selected |
|--------|-------------|----------|
| Stay in v2 | PLAT-05 reste en v2 | |
| i18n-ready now | Infra clés de traduction dès phase 1, sans 2e langue | ✓ |

| Option | Description | Selected |
|--------|-------------|----------|
| English code | Convention Rust, FR seulement dans les chaînes | ✓ |
| French code | Tout en français | |

---

## Privilege UX

| Option | Description | Selected |
|--------|-------------|----------|
| pkexec everywhere | Même méthode partout, une seule expérience mot de passe | ✓ |
| sudo in CLI | sudo classique en terminal, pkexec pour le GUI | |

| Option | Description | Selected |
|--------|-------------|----------|
| Remember briefly | Autorisation valide quelques minutes (standard polkit) | ✓ |
| Ask every time | Redemande à chaque opération | |

| Option | Description | Selected |
|--------|-------------|----------|
| yay native | yay gère son sudo lui-même, Archel ne s'en mêle pas | ✓ |
| Archel handles it | Archel orchestre l'élévation de yay | |

| Option | Description | Selected |
|--------|-------------|----------|
| Friendly error | Erreur FR lisible nommant l'action rejetée | ✓ |
| Raw error | Erreur brute du système | |

---

## Spike fallback

| Option | Description | Selected |
|--------|-------------|----------|
| Auto-pivot | Échec CXX-Qt → bascule Slint sans revenir vers l'utilisateur | ✓ |
| Stop and ask | Arrêt au constat, décision ensemble | |

| Option | Description | Selected |
|--------|-------------|----------|
| Tabs + live label | Prouve le vrai pattern GUI | ✓ |
| Hello window | Prouve seulement que le bridge link | |

| Option | Description | Selected |
|--------|-------------|----------|
| Time-boxed | Au-delà d'un budget, pivot | |
| No limit | Insistance sans limite | ✓ |

**User's choice:** No time limit, then clarified: only a hard build/link incompatibility with Qt 6.11 triggers the pivot ("Hard failure only") — debugging friction does not.

---

## the agent's Discretion

- Workspace crate names and internal module layout
- Which i18n mechanism for the FR catalog
- Exact pkexec polkit policy contents and allowlisted action set
- Spike implementation details beyond the success bar

## Deferred Ideas

None — discussion stayed within phase scope.
