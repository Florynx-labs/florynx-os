---
title: FlorynxOS GUI Roadmap
---

# FlorynxOS GUI Roadmap

## Vision
FlorynxOS doit devenir un OS desktop moderne, fluide et immédiatement reconnaissable.

La cible n'est pas de copier Windows 11 trait pour trait, mais d'atteindre le même niveau de qualité perçue avec une identité propre :

- desktop sombre premium
- accent lumineux cyan / mint issu du logo
- surfaces flottantes élégantes
- interactions rapides et stables
- architecture GUI simple mais sérieuse

## Product Direction
FlorynxOS doit converger vers :

- la clarté de Windows 11
- une identité plus organique et lumineuse
- un shell cohérent pensé comme un vrai produit
- une base technique capable d'accueillir un futur userland

## Phase 1 — GUI Foundation Stabilization
Objectif : rendre l'interface actuelle propre, stable et performante.

### Goals
- éliminer les redraws complets inutiles
- rendre le drag des fenêtres fluide
- fiabiliser les interactions souris
- poser une architecture renderer / shell / widgets claire

### Deliverables
- renderer avec dirty rectangles
- clipping propre
- meilleur contrôle des redraws
- hit testing plus précis
- boutons de fenêtre réellement interactifs
- focus window fiable
- hover states robustes

### Success Criteria
- mouvement de souris sans reload global
- drag fluide et stable
- dock sans scintillement
- aucune interaction ne bloque l'input

## Phase 2 — Desktop Shell v1
Objectif : passer d'une démo graphique à un vrai shell.

### Goals
- structurer le desktop comme un OS moderne
- créer une hiérarchie visuelle forte
- donner un layout cohérent au shell

### Deliverables
- wallpaper system
- dock / taskbar finale
- launcher / start menu
- zone horloge / statut système
- panneau quick settings
- notifications simples

### Success Criteria
- shell immédiatement lisible
- éléments système bien séparés du contenu des apps
- navigation souris naturelle

## Phase 3 — Design System
Objectif : unifier toute l'interface.

### Goals
- centraliser les tokens UI
- éviter les styles codés en dur dans les composants
- rendre les futurs écrans cohérents

### Deliverables
- semantic colors
- elevation levels
- radius scale
- spacing scale
- icon sizing system
- motion durations
- states: hover, pressed, focused, disabled

### Success Criteria
- thème unique sur tout le shell
- composants facilement réutilisables
- moindre coût de maintenance visuelle

## Phase 4 — Native System Apps
Objectif : rendre FlorynxOS crédible comme environnement utilisable.

### Deliverables
- terminal GUI natif
- file manager minimal
- settings app
- system monitor
- notes / editor minimal

### Success Criteria
- possibilité de faire une vraie démo desktop
- cohérence visuelle entre shell et apps
- apps ouvrables depuis le dock / launcher

## Phase 5 — Windowing & Compositor Maturity
Objectif : faire évoluer le système de fenêtres vers une vraie stack desktop.

### Deliverables
- resize windows
- minimize / maximize réels
- snapping
- animations d'ouverture / fermeture
- surfaces composées plus proprement
- z-order robuste
- overlays et panels système

### Success Criteria
- comportement window manager crédible
- stabilité sous interactions rapides
- transitions visuelles propres

## Phase 6 — Userland Bridge
Objectif : préparer la séparation shell / apps / kernel.

### Deliverables
- modèle de processus userland
- protocole d'événements GUI
- surfaces par application
- IPC minimal
- cycle de vie des apps

### Success Criteria
- shell non couplé aux apps système
- base exploitable pour SDK futur

## Phase 7 — Premium OS Experience
Objectif : atteindre un niveau de finition “Windows 11 class”.

### Deliverables
- boot animation
- lock screen
- onboarding
- recherche globale
- virtual desktops
- command palette
- thèmes
- branding sonore et visuel

### Success Criteria
- identité forte
- expérience mémorable
- cohérence produit complète

## Priority Roadmap

### Immediate Priorities
- dirty rectangle engine
- boutons de fenêtre interactifs
- launcher v1
- dock finale
- terminal GUI natif

### Short-Term Priorities
- settings app
- file manager minimal
- notifications
- better text rendering
- animations système

### Mid-Term Priorities
- compositor plus mature
- séparation shell / app
- userland apps
- theming system

## Technical Milestones

### Milestone A — Render & Input Cleanup
- partial redraws
- stable cursor updates
- precise event routing
- input-to-render consistency

### Milestone B — Shell Components
- launcher
- taskbar / dock polished
- status area
- panels

### Milestone C — App Framework
- window lifecycle
- app registry
- launch flow
- shared widgets

### Milestone D — Userland Transition
- IPC minimal
- surface ownership
- app isolation

## Risks
- trop de redraws CPU-side
- coupling trop fort entre desktop et renderer
- complexité croissante sans design system
- userland introduit trop tôt sans shell stable

## Strategy
Toujours avancer dans cet ordre :

1. stabilité
2. clarté visuelle
3. shell complet
4. apps système
5. userland

## Final Target
FlorynxOS doit devenir :

- un desktop sombre premium
- un shell fluide et stable
- un OS avec une vraie personnalité visuelle
- une base crédible pour des apps natives et un futur userland
