---
title: FlorynxOS GUI PRD
---

# FlorynxOS GUI PRD

## Product Name
FlorynxOS Desktop Shell

## Product Goal
Créer une interface desktop moderne, premium, stable et immédiatement identifiable, capable de faire évoluer FlorynxOS d'une base framebuffer vers un véritable environnement OS inspiré par la qualité perçue de Windows 11, tout en gardant une identité visuelle propre.

## Product Vision
FlorynxOS ne doit pas être un clone.

Il doit devenir :

- plus doux que les desktops techniques classiques
- plus lumineux et plus vivant qu'un shell minimal
- plus distinctif visuellement que la plupart des prototypes OS

La signature de FlorynxOS doit venir de son logo :

- halo cyan / mint
- énergie organique
- courbes fluides
- contraste entre obscurité profonde et lumière douce

## Users

### Primary Users
- développeur principal du kernel
- contributeurs système / GUI
- utilisateurs qui testent les builds QEMU

### Secondary Users
- visiteurs GitHub
- recruteurs / viewers portfolio
- passionnés OS / low-level UI

## Core Product Principles

### Clarity
Chaque surface doit être immédiatement lisible.

### Fluidity
Les interactions doivent sembler rapides, calmes et précises.

### Brand Identity
Le produit doit être reconnaissable au premier regard.

### Lightweight Engineering
Les effets doivent rester compatibles avec une stack CPU-only et un renderer framebuffer.

### Stability First
La beauté ne doit jamais casser la fiabilité.

## UX Goals
Au boot, le user doit ressentir :

- un desktop premium
- une ambiance sombre et moderne
- une hiérarchie claire entre shell et contenu
- une sensation d'OS réel, pas de simple démo technique

## Functional Scope

### Desktop
- wallpaper system
- desktop layout structuré
- dock / taskbar
- status surface
- launcher

### Windowing
- drag
- focus
- z-order
- minimize
- maximize
- close
- resize plus tard

### System UI
- quick settings
- notifications
- search / launcher
- clock / status area

### Native Apps
- terminal
- files
- settings
- monitor
- notes

## Non-Functional Requirements

### Performance
- éviter les redraws complets inutiles
- viser une sensation fluide sur interaction
- budget d'effets strict

### Stability
- lock ordering cohérent
- pas de deadlock renderer / desktop / input
- aucune interaction ne doit figer l'UI

### Maintainability
- séparation renderer / shell / widgets / apps
- thème centralisé
- composants réutilisables

## Design Direction

### Aesthetic Theme
**Bioluminescent Glass Desktop**

Direction choisie :
- dark luxury futurism
- surfaces vitrées sombres
- lumière cyan / mint douce
- profondeur calme, jamais agressive
- formes souples et premium

### Visual Identity Derived From Logo
Le logo montre :

- une forme fluide
- une énergie interne lumineuse
- un contraste sombre / vert-cyan très fort

Le GUI doit reprendre exactement cela :

- surfaces sombres profondes
- accent lumineux local
- glow concentré sur les éléments actifs
- présence d'une énergie visuelle interne au système

## Color System

### Core Palette
- background-0: `#0D1117`
- background-1: `#111723`
- background-2: `#161D29`
- panel: `#1B2330`
- panel-elevated: `#222C3A`
- border-soft: `#2B3646`
- text-primary: `#F3F7FA`
- text-secondary: `#A8B6C6`
- accent-cyan: `#29D3D0`
- accent-mint: `#6EF0A2`
- accent-lime: `#D9FF72`
- danger-soft: `#F26D6D`
- shadow-deep: `#05070B`

### Usage Rules
- accent lumineux réservé aux éléments actifs
- éviter les aplats trop saturés
- glow uniquement sur focus / hover importants
- textes secondaires toujours désaturés

## Material Language
- coins arrondis 12 à 16 px
- panneaux semi-mats
- bordures fines légèrement froides
- ombres diffuses profondes
- glow subtil, jamais néon agressif
- surfaces flottantes séparées du fond par contraste et profondeur

## Motion Language
- animations courtes
- transitions douces
- easing calme
- ouverture des fenêtres avec légère élévation + fade
- hover de dock avec lift très léger
- feedback d'activation par halo, pas par flash brutal

## Information Architecture

### Desktop Layer Order
1. wallpaper
2. atmospheric overlays
3. desktop icons
4. app windows
5. dock / taskbar
6. panels / launcher / notifications
7. cursor

### Shell Layout
- dock centrée en bas
- zone système compacte en haut à droite ou dans la barre
- launcher centré
- fenêtres légèrement au-dessus du fond avec respiration autour

## Concrete Art Direction — Next Desktop Screen

## Objective
Définir précisément le prochain écran desktop à implémenter, pas juste une direction abstraite.

## Screen Name
**FlorynxOS Desktop vNext — Luminous Shell**

## Composition

### Background
Le fond doit être un mélange de :
- gradient profond bleu-noir vers graphite
- texture noise très fine
- vignette douce
- grand halo organique cyan/green derrière la fenêtre principale, rappelant la forme du logo

Le fond ne doit pas être vide.
Il doit avoir une présence atmosphérique mais rester calme.

### Dock
Dock flottante centrée, plus premium que l'actuelle.

#### Visual Specs
- hauteur : 58-64 px
- coins : 18 px
- fond : panel sombre translucide simulé
- bord : ligne fine froide
- ombre : diffuse, basse
- icônes : 20-24 px au centre de tuiles 40-44 px

#### Interaction Specs
- hover : légère montée + halo discret
- actif : petit indicateur lumineux mint
- pressed : assombrissement temporaire

### Main Window
Une grande fenêtre centrale légèrement décalée vers le haut.

#### Visual Specs
- taille cible : ~480x320 ou ~540x360
- coins : 16 px
- ombre : plus douce et plus large que maintenant
- titlebar : compacte, premium, moins brute
- séparateur interne plus discret

#### Content Direction
La fenêtre doit devenir une vraie surface d'accueil système.
Contenu proposé :
- titre : `Welcome to FlorynxOS`
- sous-titre : `Bioluminescent desktop shell prototype`
- 3 cards système :
  - Terminal
  - Files
  - Settings
- zone statut en bas : build, uptime, memory

### Secondary Surface
Ajouter une petite carte secondaire ancrée en haut à droite ou bas droit.
Exemple :
- widget système
- horloge
- CPU / memory miniature
- "System Ready"

Cette petite surface donnera au desktop une sensation plus vivante.

## Desktop vNext Layout Spec

### Left / Center Balance
- centre : fenêtre principale
- bas centre : dock
- coin secondaire : widget système
- fond : halo organique derrière la fenêtre

### Spacing
- marges généreuses
- pas d'éléments collés au bord
- respiration visuelle forte

## Icon Direction
Les icônes ne doivent pas rester purement utilitaires.
Direction recommandée :
- style simple, géométrique, propre
- formes pleines + quelques découpes
- cohérence d'épaisseur
- palette monochrome claire dans surfaces sombres

## Window Button Direction
Les boutons close/min/max doivent être :
- plus petits
- mieux centrés
- plus élégants
- moins “dessinés à la main”

Option future :
- cercle coloré très discret
- glyph fin centré
- hover avec glow interne

## Launcher Art Direction
Le futur launcher doit être :
- centré
- large mais bas
- avec une barre de recherche très clean
- grille d'apps espacée
- récents en bas

Visuellement :
- panneau dark glass
- glow mint léger sur focus search
- ombre large et douce

## Desktop States To Design Next

### State 1 — Idle
- dock passive
- fenêtre welcome ouverte
- widget système visible
- fond vivant mais calme

### State 2 — Hover Dock
- une icône surélevée
- halo discret
- indicateur actif visible

### State 3 — Active Window
- titlebar plus lumineuse
- légère accent line ou glow interne

### State 4 — Launcher Open
- fond légèrement atténué
- launcher centré
- recherche immédiate

## Immediate Implementation Guidance

### Next Technical Slice
1. améliorer la composition du fond
2. refaire la dock en version premium
3. transformer la fenêtre welcome en vraie home surface
4. ajouter un petit widget système secondaire
5. commencer le launcher

### Files To Introduce Later
- `src/gui/theme/tokens.rs`
- `src/gui/icons.rs` version étendue
- `src/gui/launcher.rs`
- `src/gui/panel.rs`
- `src/gui/widgets/system_card.rs`

## Acceptance Criteria For Desktop vNext
- le desktop semble intentionnel et premium
- le logo influence vraiment l'identité visuelle
- la hiérarchie des surfaces est claire
- le shell paraît plus proche d'un OS réel que d'un test renderer
- l'utilisateur retient la lumière cyan/mint comme signature FlorynxOS

## Final Positioning Statement
FlorynxOS doit ressembler à :

- un desktop dark premium
- plus organique que Windows 11
- plus lumineux qu'un shell minimal classique
- plus signature qu'un prototype OS standard

En une phrase :

**FlorynxOS est un desktop bioluminescent premium, où la lumière du logo devient la matière principale de l'interface.**
