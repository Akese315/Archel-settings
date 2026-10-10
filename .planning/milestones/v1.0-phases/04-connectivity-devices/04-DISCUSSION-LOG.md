# Phase 4: Connectivity & Devices - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-06
**Phase:** 4-Connectivity & Devices
**Areas discussed:** DNS par profil, Proxy session, Appairage BT, VPN et tests

---

## DNS par profil

| Option | Description | Selected |
|--------|-------------|----------|
| Remplacer | Les DNS DHCP sont ignores sur ce profil | ✓ |
| Ajouter devant | Perso d'abord, DHCP en secours | |
| IPv4 seul | Un seul reglage pour tout | ✓ |
| IPv4 + IPv6 | Pour les reseaux full IPv6 | |
| Reset auto | Retour au DHCP en une commande | ✓ |
| Reset manuel | Remettre l'ancien a la main | |

**User's choice:** Remplacer, IPv4 seul, Reset auto
**Notes:** Straightforward triple; no debate.

## Proxy session

| Option | Description | Selected |
|--------|-------------|----------|
| Proposition agent | off / manual host port / auto pac-url + etat | ✓ |

**User's choice:** Approved as-is (free-text "propose moi la maniere la plus ergonomique intuitive et fluide" → GNOME-like 3-mode proposal → "ca me va")
**Notes:** Authenticated-proxy credentials (user/password) raised by agent as possible extra; left open.

## Appairage BT

| Option | Description | Selected |
|--------|-------------|----------|
| Confirmer en CLI | Archel affiche le code, o/N | ✓ |
| Auto-accepter | Aveugle, pratique | |
| Scan puis choisir | Voir avant de choisir | ✓ |
| Adresse directe | Si on connait la MAC | |

**User's choice:** Confirmer en CLI, Scan puis choisir
**Notes:** No debate.

## VPN et tests

| Option | Description | Selected |
|--------|-------------|----------|
| Multi-protocoles + import fichier | OpenVPN, WireGuard, .ovpn/.conf | ✓ |
| Les trois hops | Box + DNS + internet | ✓ |
| Internet seul | Juste si internet repond | |
| Chiffres bruts | Millisecondes affichees | ✓ |
| Diagnostic auto | Archel dit bon/degrade/coupe | |

**User's choice:** Multi-protocol + file import (free-text with GUI popup form request); triple-hop tests; raw ms
**Notes:** User's words: "plusieurs protocoles comme openvpn ou wireguard etc... une fenetre popup qui m'affiche un formulaire, mdp utilisateurs certificat cle etc... ou meme fichier config". Popup form deferred to GUI phase with fields recorded (username/password/certificate/key/file).

---

## the agent's Discretion

Subcommand inventory, audio device identification, proxy storage mechanism, VPN secret handling, ping counts/timeouts, table layouts, JSON fields.

## Deferred Ideas

- VPN popup GUI form — GUI phase
- Authenticated-proxy credentials — open, researcher evaluates
