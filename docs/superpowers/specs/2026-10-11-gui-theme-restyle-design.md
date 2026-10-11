# Design — Modernisation du style GUI (thème Archel)

Date : 2026-10-11
Statut : validé par l'utilisateur (brainstorming)
Portée : restyle complet **sans changement de structure** des onglets.

## Contexte

Le GUI (`archel settings`, CXX-Qt + QML) utilise aujourd'hui le style par défaut de
QtQuick.Controls : aucun thème, couleurs système brutes. L'utilisateur veut un rendu
« moderne, esprit Hyprland / catppuccin » : fond très sombre, surfaces semi-transparentes
teintées, rail vertical dont l'onglet actif est teinté par la couleur d'accent.

Décisions prises en brainstorming (maquettes `.superpowers/brainstorm/mockups/`) :

1. **Direction C** — « sombre translucide » (catppuccin-like) comme mode par défaut.
2. **Bascule manuelle** clair/sombre dans l'onglet Apparence, persistée.
3. **Six accents sélectionnables** : bleu, mauve, vert, sarcelle, rose, pêche (catppuccin).
4. **Police système** — aucun fichier de police embarqué ; seule la graisse/taille est ajustée.
5. Structure des onglets, navigation et contenu : **inchangés**.

## Contrainte technique déterminante

`cxx-qt-build` 0.10 **ne supporte pas les singletons QML** (le `qmldir` généré
n'enregistre que des composants). Un `Theme.qml` singleton n'est donc pas une option.

Solution retenue : l'état du thème vit côté Rust dans un modèle CXX-Qt, et les couleurs
sont propagées via la **QPalette standard** de `ApplicationWindow`. Chaque contrôle Qt
hérite automatiquement de la palette ; tout QML peut lire un token via sa propre
propriété `palette` héritée — sans référence à un id, sans singleton. C'est la
généralisation d'un pattern déjà présent (`ErrorModal.qml` lit `palette.brightText`).

## Architecture

### ThemeModel (Rust) — `crates/archel/src/gui/theme.rs`

Nouveau fichier sur le pattern des bridges existantes (`bridge_system.rs`) :

- État : `mode: Sombre | Clair` (défaut : Sombre), `accent: Blue | Mauve | Green | Teal
  | Pink | Peach` (défaut : Blue).
- 12 ensembles de couleurs dérivés (2 modes × 6 accents), calculés en Rust purs et
  testables unitairement. Chaque accent fournit : couleur pleine, variante claire pour
  texte, teinte alpha pour fonds actifs.
- Les couleurs sont exposées comme propriétés QColor mappées sur les **rôles QPalette** :

| Rôle QPalette | Token (mode sombre, style C) |
|---|---|
| `window` | fond global `#101018` |
| `base` | surface de carte `rgba(255,255,255,0.05)` |
| `alternateBase` | surface élevée / survol |
| `mid` | bordure subtile `rgba(255,255,255,0.07)` |
| `button` | fond de bouton `rgba(255,255,255,0.06)` |
| `accent` / `highlight` / `highlightedText` | accent + sélection teintée |
| `text` / `windowText` / `buttonText` | textes principaux |
| `brightText` | texte accent-clair (valeurs en surbrillance) |
| `placeholderText` | texte secondaire (~60 % d'opacité) |

  Le mode clair duplique chaque rôle (fond `#f2f3f7`, cartes blanches, ombres douces,
  palette latte) avec un contraste texte/fond visé AAA.

- Invokables : `setDark(bool)`, `setAccent(key)`, lecture au démarrage, écriture à chaud.
- Enregistrement dans `build.rs` via `.files([...])` (type Rust — pas de `qml_files`).

### Propagation QML (Main.qml)

- `ApplicationWindow` instancie `ThemeModel { id: theme }` et lie chaque sous-propriété
  de palette au modèle : `palette.window: theme.colorWindow`, `palette.base:
  theme.colorBase`, etc. Qt propage la palette à tous les contrôles et popups —
  boutons, champs, combos, listes, tooltips, modals se restylent sans toucher à leur code.
- Changement de mode ou d'accent = une écriture de propriété Rust ; tout se redessine
  (les bindings de palette notifient tous les enfants).

### Rail vertical (Main.qml)

- Onglet actif : fond teinté alpha de l'accent + libellé accent-clair.
- Survol : `palette.alternateBase` ; coins arrondis 8 px ; hauteur 36 px.
- Icônes SVG existantes : inchangées.

### Nouveaux composants QML

- **`SectionCard.qml`** — remplace mécaniquement chaque `GroupBox` de section dans les
  10 onglets : fond `palette.base`, bordure `palette.mid`, rayon 12 px, titre de section.
  Même contenu, même layout ; seul le conteneur change.
- **`PageHeader.qml`** — titre de page gras (~1.3 em) + libellé discret, en tête du
  contenu de chaque onglet. Restyle pur, aucune navigation modifiée.
- Les deux fichiers doivent être **ajoutés au `qml_files` de `build.rs`** (Pitfall 2 :
  un fichier non enregistré compile et échoue à l'exécution).

### Onglet Apparence

Le `ThemeModel` est instancié **une seule fois dans `Main.qml`** (comme `ShellModel`,
id `theme`) et passé à l'onglet Apparence via une propriété (`AppearanceTab { theme:
theme }`) — les autres onglets n'en ont pas besoin : ils lisent la palette héritée.

Nouveau `SectionCard` « Thème » en tête d'onglet :

- Mode : commutateur Sombre/Clair, reflétant `ThemeModel`, changement à chaud immédiat.
- Accent : 6 pastilles catppuccin, active cerclée de sa couleur, nom affiché à côté ;
  clic = changement immédiat.
- Tout est live (pas de bouton « Appliquer »), comme le reste de l'app.
- Le reste de l'onglet (effets, fond d'écran, police Hyprland) : inchangé.

### Persistance

`~/.config/archel/theme.json` (même schéma que `window-geometry.json`, `directories::
ProjectDirs`), lu au démarrage, réécrit à chaque changement. Fichier absent ou corrompu
→ défauts (sombre + bleu), jamais d'erreur bloquante.

## Ce qui ne change pas

- **Couleurs sémantiques** (vert/orange/rouge de la matrice de cœurs et seuils de
  charge) : codées en dur — ce sont des états, pas de la décoration ; elles doivent
  rester lisibles quel que soit l'accent. Les sparklines prennent l'accent pour les
  courbes normales.
- Polices système ; seules graisse/taille des titres sont ajustées.
- Structure des onglets, navigation, modèles de données, contenu des onglets.
- Icônes SVG du rail.

## Gestion d'erreurs

- `theme.json` illisible/invalide → valeurs par défaut + log `tracing` warning, l'app démarre.
- Clé d'accent inconnue dans le fichier → bleu par défaut.
- L'écriture de `theme.json` échoue → le changement reste actif en mémoire pour la
  session (best-effort, log warning) — même philosophie que `geometry.rs`.

## Tests

1. **Unitaires Rust** (nouveaux) :
   - dérivation des 12 ensembles : chaque rôle non transparent, contraste
     texte/fond satisfaisant, cohérence des variantes claires/foncées de l'accent ;
   - `theme.json` : défauts, round-trip, fichier corrompu → défauts, accent inconnu → bleu.
2. **Régression** : `cargo test -p archel-core -p archel` doit rester vert (aucun
   changement de comportement côté modèles/bridge existants).
3. **Harnais QML offscreen** (`/tmp/archel-qml-harness/`) étendu : vérifications rail
   (onglet actif teinté), `SectionCard` (fond/rayon depuis la palette), `PageHeader`,
   pastilles d'accent cliquables.
4. **Visuel final** : relance de l'app par l'utilisateur (redémarrage de toute façon
   requis — les QML sont compilés dans le binaire via qmlcachegen).

## Volume estimé

| Zone | Fichiers |
|---|---|
| Rust | `theme.rs` (nouveau), `mod.rs`, `build.rs` (`.files`), tests |
| QML central | `Main.qml` (palette + rail) |
| Composants | `SectionCard.qml`, `PageHeader.qml` (nouveaux) |
| Onglets | 10 fichiers : substitution `GroupBox`→`SectionCard` + `PageHeader` |
| Composants partagés | 9 fichiers : vérification, au plus quelques fonds/rayons à aligner |
| Onglet Apparence | `AppearanceTab.qml` (SectionCard Thème) + binding `ThemeModel` |

## Hors périmètre

- Vrai flou derrière la fenêtre (compositing Wayland peu fiable) — la translucence est
  des surfaces teintées à l'intérieur de la fenêtre, comme dans les maquettes.
- Style plugin QtQuick.Controls complet, thèmes catppuccin additionnels au-delà des
  6 accents, embarquement de polices, refonte de structure.
