# Phase 2: System Insight - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-30
**Phase:** 2-System Insight
**Areas discussed:** Rafraîchissement mesures, Fraîcheur màj, Périmètre capteurs, Forme CLI

---

## Rafraîchissement mesures

| Option | Description | Selected |
|--------|-------------|----------|
| Snapshot unique | Affichage figé, simple et scriptable | |
| Mode watch intégré | Réaffichage continu type htop | |
| Les deux modes | Snapshot par défaut + --watch | ✓ |

**User's choice:** Les deux modes (free-text "c", clarified after confusion — user first asked "de quoi tu parles?", re-explained with concrete `archel sys cpu` example; user then tried running it and learned `sys` doesn't exist yet)
**Notes:** Watch à 500 ms; --watch + --json = un objet JSON complet par tick; échantillonnage CPU ~1 s en snapshot. User ran `./target/debug/archel sys cpu` and got clap's English `unrecognized subcommand` error — noted as future francization concern, not Phase 2.

## Fraîcheur màj

| Option | Description | Selected |
|--------|-------------|----------|
| Recalculé à chaque fois | Toujours frais, possiblement lent | |
| Mis en cache | Cache avec durée de vie | ✓ |

**User's choice:** Mis en cache (free-text "mis en cache")
**Notes:** Cache 15 min; hors-ligne/échec = valeur + âge affichés; --refresh pour forcer.

## Périmètre capteurs

| Option | Description | Selected |
|--------|-------------|----------|
| Partition système | Go utilisés sur la partition du système | ✓ |
| Tous les mounts | Liste complète per-mount (SYS-02 brut) | |

**User's choice:** Partition système (free-text "les giga utilise sur la partition qu'utilise ce systen")
**Notes:** Détail complet utilisé/total/libre; températures = une représentative par catégorie = la plus chaude. Narrows SYS-02 deliberately.

## Forme CLI

| Option | Description | Selected |
|--------|-------------|----------|
| Groupe sys | `archel sys cpu`, `sys ram`… | ✓ |
| Commandes directes | `archel cpu`, `archel ram`… | |

**User's choice:** Groupe sys (free-text "a)")
**Notes:** User confused "forme" with language — clarified it means command names/layout, not French output. Tableau humain; `archel landing` top-level; --json global.

---

## the agent's Discretion

Per-metric subcommand list beyond cpu/ram/disk/gpu, cache file location, sampling implementation, per-vendor sensor sources, table layout, JSON field names.

## Deferred Ideas

None — discussion stayed within phase scope. (Clap English error francization noted as future-phase concern.)
