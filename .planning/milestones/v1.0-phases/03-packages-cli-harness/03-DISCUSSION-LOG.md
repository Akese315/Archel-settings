# Phase 3: Packages & CLI Harness - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-01
**Phase:** 3-Packages & CLI Harness
**Areas discussed:** Sécurité install, Révision AUR, Périmètre recherche, Complétions shell

---

## Sécurité install

| Option | Description | Selected |
|--------|-------------|----------|
| Confirmer | Archel demande confirmation avant d'agir | ✓ |
| Direct sans confirmer | Installe sans question | |
| Confirmer + --yes | Confirmation contournable pour scripts | |

**User's choice:** Confirmer
**Notes:** Sortie pacman en direct; dépendances détaillées dans la confirmation; -Sy partiel refusé proprement.

## Révision AUR

| Option | Description | Selected |
|--------|-------------|----------|
| PKGBUILD + valider | Contenu affiché puis validation | ✓ |
| Métadonnées seules | Fiche seule, construction directe | |

**User's choice:** Fiche taille + provenance + version (free-text), les deux tailles, comparaison de version, PKGBUILD + valider
**Notes:** User's words: "la taille, le serveur utilise, et la version" — interpreted as download+installed sizes, repo/AUR source, candidate version.

## Périmètre recherche

| Option | Description | Selected |
|--------|-------------|----------|
| Version décroissante | Triées par version, sources confondues | ✓ |
| Grouper par source | Dépôts groupés d'abord, AUR après | |

**User's choice:** Tous les serveurs, version puis serveur, pacman avant yay (free-text); list = installés seuls
**Notes:** User's words: "dans tous les serveurs, en priorisant la version puis le serveur (pacman en premier sinon yay)".

## Complétions shell

| Option | Description | Selected |
|--------|-------------|----------|
| bash + zsh + fish | Les trois d'un coup via clap_complete | ✓ |

**User's choice:** Accepted on recommendation (free-text "c'est quoi le meilleur?" → explained fish/zsh/bash, recommended all three since near-zero cost; user: "ok")
**Notes:** User asked what a shell is — explained bash (current) / zsh / fish plainly.

---

## the agent's Discretion

pkg subcommand names, confirmation wording, progress rendering, JSON fields, completion install paths.

## Deferred Ideas

None.
