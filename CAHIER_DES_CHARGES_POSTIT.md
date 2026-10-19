# Cahier des Charges — Logiciel de Post-it au Démarrage / Sortie de Veille

**Version :** 1.0  
**Date :** 2026-10-19  
**Auteur :** [Votre Nom]  
**Statut :** Brouillon — À valider

---

## 1. Présentation du Projet

### 1.1 Contexte et Origine du Besoin
Développement d'une application légère type "post-it" qui s'affiche automatiquement à deux moments clés :
- Au **démarrage de l'ordinateur** (après allumage / reboot)
- À la **sortie de mise en veille** (réveil du système)

L'objectif est de proposer une note visuelle, personnalisable, non intrusive mais bien visible, pour afficher une pensée, une tâche, une citation ou un rappel au moment où l'utilisateur reprend son activité.

### 1.2 Objectifs du Projet (SMART)
| Objectif | Indicateur de réussite |
|----------|------------------------|
| Afficher un post-it automatiquement au boot et au wake | 100 % des démarrages/réveils déclenchent l'affichage |
| Permettre une personnalisation graphique complète (polices, couleurs, opacité, position) | Toutes les options configurables via fichier `.conf` sans recompilation |
| Garantir que le post-it reste au premier plan (always-on-top) | Z-order maintenu même après alt-tab / focus d'autres fenêtres |
| Offrir un mode "calque" (overlay) sur tout l'écran avec couleur + intensité | Overlay visible, configurable, ne bloque pas les clics (click-through optionnel) |
| Proposer un mode aléatoire ou séquentiel pour le contenu | Sélection aléatoire ou indexée selon config |
| Être multi-plateforme (Windows / Linux / macOS) | Build fonctionnel sur les 3 OS cibles |

### 1.3 Périmètre (In / Out)

| **Dans le périmètre (MVP v1.0)** | **Hors périmètre (v2+)** |
|----------------------------------|---------------------------|
| Affichage post-it au boot / wake | Synchronisation cloud / multi-appareils |
| Configuration via fichier `.conf` (INI/TOML/YAML) | Éditeur graphique de config (GUI) |
| Personnalisation : police titre (onglet), police corps, couleurs, fond, opacité | Historique des post-its affichés |
| Mode calque plein écran (couleur + % intensité) | Catégories / tags / recherche |
| Position centrée (configurable) | Raccourcis clavier globaux pour masquer/afficher |
| Always-on-top + focus non volé | Support multi-écrans avancé (choix écran) |
| Contenu : fixe, liste séquentielle, aléatoire | Export / import de configurations |
| Click-through optionnel sur l'overlay | Notifications système / sons |

---

## 2. Utilisateurs et Cas d'Usage

### 2.1 Personas
| Persona | Description | Besoin principal |
|---------|-------------|------------------|
| **Développeur / Power-user** | Utilise Linux/Windows, à l'aise avec fichiers config | Post-it technique (raccourcis, TODO, snippet) au réveil |
| **Créatif / Étudiant** | Veut un rappel visuel inspirant (citation, objectif) | Personnalisation graphique poussée (polices, couleurs) |
| **Utilisateur minimaliste** | Ne veut pas d'interface, juste un fichier à éditer | Config simple, légère, sans dépendances lourdes |

### 2.2 User Stories (Format : En tant que… je veux… afin de…)

| ID | User Story | Priorité (MoSCoW) |
|----|------------|-------------------|
| US-01 | En tant qu'utilisateur, je veux qu'un post-it s'affiche automatiquement au démarrage de mon PC, afin de voir ma priorité du jour sans action de ma part. | **Must** |
| US-02 | En tant qu'utilisateur, je veux qu'un post-it s'affiche à la sortie de veille, afin de reprendre le contexte immédiatement. | **Must** |
| US-03 | En tant qu'utilisateur, je veux configurer l'apparence (polices, couleurs, fonds, opacité) via un fichier `.conf`, afin de ne pas avoir d'interface lourde. | **Must** |
| US-04 | En tant qu'utilisateur, je veux que le titre du post-it ressemble à un onglet (police, fond distincts), afin de l'identifier visuellement. | **Must** |
| US-05 | En tant qu'utilisateur, je veux un calque plein écran (overlay) avec couleur et intensité réglables, afin de créer une ambiance visuelle derrière le post-it. | **Must** |
| US-06 | En tant qu'utilisateur, je veux que le post-it soit centré à l'écran et toujours au premier plan, afin qu'il soit bien visible. | **Must** |
| US-07 | En tant qu'utilisateur, je veux choisir entre contenu fixe, séquentiel ou aléatoire, afin de varier les messages. | **Must** |
| US-08 | En tant qu'utilisateur, je veux pouvoir rendre l'overlay "click-through", afin de cliquer sur les fenêtres dessous sans fermer le post-it. | **Should** |
| US-09 | En tant qu'utilisateur, je veux charger une police personnalisée (fichier .ttf/.otf) pour le titre et le corps, afin d'avoir mon style. | **Should** |
| US-10 | En tant qu'utilisateur, je veux que l'app soit portable (pas d'installation, un binaire + config), afin de l'emmener sur clé USB. | **Could** |

---

## 3. Spécifications Fonctionnelles

### 3.1 Déclencheurs d'Affichage
| Déclencheur | Description | Détection technique |
|-------------|-------------|---------------------|
| **Boot / Login** | Affichage à l'ouverture de session utilisateur | - Windows : Registre `Run` / Task Scheduler `AtLogon`<br>- Linux : systemd user service `graphical-session.target` / `.desktop` autostart<br>- macOS : LaunchAgent `RunAtLoad` + `KeepAlive` |
| **Wake / Sortie veille** | Affichage immédiat au réveil du système | - Windows : `WM_POWERBROADCAST` `PBT_APMRESUMEAUTOMATIC`<br>- Linux : systemd `sleep.target` + `systemd-sleep` hook / `logind` `PrepareForSleep=false`<br>- macOS : `NSWorkspaceDidWakeNotification` / `IOPMrootDomain` |

> **Note :** L'application doit tourner en arrière-plan (daemon/service) ou être relancée à chaque événement. Approche recommandée : **daemon léger** (tray/background) qui écoute les événements système.

### 3.2 Contenu du Post-it

#### Sources de contenu (configurable)
| Mode | Comportement |
|------|--------------|
| **Fixe** | Un seul message défini dans la config |
| **Séquentiel** | Liste ordonnée ; passage au suivant à chaque affichage (index persisté) |
| **Aléatoire** | Choix uniforme dans la liste à chaque affichage |
| **Fichier externe** | Chemin vers un fichier texte (1 ligne = 1 post-it) — rechargé à chaque affichage |

#### Structure d'un post-it
```
┌─────────────────────────────────────┐
│  ██████████████████████████████████  │  ← TITRE (style "onglet")
│  █  Titre / Onglet  ███████████████  │     - Police configurable
│  ██████████████████████████████████  │     - Fond configurable (couleur/image)
├─────────────────────────────────────┤
│                                     │
│        Corps du texte               │  ← CORPS
│        (multi-lignes,               │     - Police configurable
│         word-wrap)                  │     - Alignement : gauche/centré/droite
│                                     │
└─────────────────────────────────────┘
```

### 3.3 Personnalisation Graphique (via `.conf`)

#### 3.3.1 Titre (Onglet)
| Propriété | Type | Défaut | Description |
|-----------|------|--------|-------------|
| `title.text` | string | "" | Texte du titre (peut être vide pour masquer l'onglet) |
| `title.font_path` | path | System UI | Chemin vers police `.ttf`/`.otf` custom |
| `title.font_size` | int | 14 | Taille en points |
| `title.font_weight` | enum | normal | `normal`, `bold`, `light`, `medium` |
| `title.color` | hex/rgb | `#FFFFFF` | Couleur du texte |
| `title.bg_color` | hex/rgb | `#333333` | Fond de l'onglet |
| `title.bg_image` | path | none | Image de fond (optionnelle, tile/stretch) |
| `title.padding` | int (px) | 8 | Marge interne |
| `title.border_radius` | int (px) | 6 | Coins arrondis (haut seulement ?) |
| `title.alignment` | enum | center | `left`, `center`, `right` |

#### 3.3.2 Corps du Post-it
| Propriété | Type | Défaut | Description |
|-----------|------|--------|-------------|
| `body.font_path` | path | System UI | Police custom |
| `body.font_size` | int | 13 | Taille en points |
| `body.font_weight` | enum | normal | Poids |
| `body.color` | hex/rgb | `#FFFFFF` | Couleur texte |
| `body.bg_color` | hex/rgb | `#2A2A2A` | Fond du post-it |
| `body.bg_image` | path | none | Image de fond |
| `body.padding` | int (px) | 16 | Marge interne |
| `body.border_radius` | int (px) | 8 | Coins arrondis |
| `body.alignment` | enum | left | Alignement texte |
| `body.line_height` | float | 1.4 | Interligne |
| `body.max_width` | int (px) | 400 | Largeur max (0 = auto) |
| `body.shadow` | bool | true | Ombre portée |

#### 3.3.3 Overlay Plein Écran (Calque arrière)
| Propriété | Type | Défaut | Description |
|-----------|------|--------|-------------|
| `overlay.enabled` | bool | true | Activer/désactiver |
| `overlay.color` | hex/rgb | `#000000` | Couleur du voile |
| `overlay.opacity` | float (0-1) | 0.35 | Intensité (0 = transparent, 1 = opaque) |
| `overlay.click_through` | bool | false | Clics traversants (ignore input) |
| `overlay.blur_background` | bool | false | Flou arrière-plan (si OS supporte) |
| `overlay.animation_ms` | int | 200 | Durée fade-in/out |

#### 3.3.4 Position & Fenêtre
| Propriété | Type | Défaut | Description |
|-----------|------|--------|-------------|
| `window.position` | enum | `center` | `center`, `top-center`, `bottom-center`, `top-left`, `top-right`, `bottom-left`, `bottom-right`, `custom` |
| `window.custom_x` | int | - | Si `position=custom` : X en px ou % |
| `window.custom_y` | int | - | Si `position=custom` : Y en px ou % |
| `window.always_on_top` | bool | true | Z-order topmost |
| `window.accept_focus` | bool | false | Ne pas voler le focus au clavier |
| `window.show_in_taskbar` | bool | false | Cacher de la barre des tâches / Alt-Tab |
| `window.resizeable` | bool | false | Redimensionnable par l'utilisateur |
| `window.close_on_click` | bool | false | Fermer au clic n'importe où (sinon bouton fermer) |
| `window.auto_close_ms` | int | 0 | 0 = pas d'auto-fermeture, sinon ms |

### 3.4 Configuration — Fichier `.conf`

**Format recommandé : TOML** (lisible, typé, supporté partout)  
Alternative : INI (simple) ou YAML (si déjà dépendance YAML)

#### Exemple complet `postit.conf.toml`
```toml
# ============================================================
# POST-IT CONFIG — À placer à côté de l'exécutable ou dans :
#   Windows: %APPDATA%\postit\postit.conf.toml
#   Linux:   ~/.config/postit/postit.conf.toml
#   macOS:   ~/Library/Application Support/postit/postit.conf.toml
# ============================================================

[general]
# Mode de sélection du contenu : "fixed" | "sequential" | "random" | "file"
content_mode = "random"
# Chemin vers fichier liste (1 post-it par ligne, ou JSON/TOML)
content_file = "postits.txt"
# Index persistant pour mode sequential (géré auto)
# sequential_index = 0

[trigger]
# Déclencheurs activés
on_boot = true
on_wake = true
# Délai après déclenchement (ms) — laisse le temps au bureau de charger
delay_ms = 800

[title]
enabled = true
text = "📌  Post-it du jour"
font_path = ""           # "" = police système
font_size = 14
font_weight = "bold"
color = "#FFFFFF"
bg_color = "#2D6BFF"
bg_image = ""
padding = 10
border_radius = 8
alignment = "center"

[body]
font_path = ""
font_size = 13
font_weight = "normal"
color = "#E0E0E0"
bg_color = "#1E1E1E"
bg_image = ""
padding = 16
border_radius = 10
alignment = "left"
line_height = 1.5
max_width = 420
shadow = true

[overlay]
enabled = true
color = "#000000"
opacity = 0.40        # 40% d'intensité
click_through = false
blur_background = false
animation_ms = 250

[window]
position = "center"
custom_x = 0
custom_y = 0
always_on_top = true
accept_focus = false
show_in_taskbar = false
resizeable = false
close_on_click = false
auto_close_ms = 0       # 0 = reste ouvert jusqu'à fermeture manuelle

[fonts]
# Dossier de polices custom (optionnel, relatif à l'exe ou absolu)
custom_fonts_dir = "fonts/"

[advanced]
# Logs
log_level = "info"      # debug, info, warn, error
log_file = ""           # "" = stdout/stderr uniquement
# Pour multi-écrans : index écran (0 = principal, -1 = curseur)
monitor_index = 0
# Hotkey globale pour afficher/masquer (optionnel, ex: "Ctrl+Alt+P")
toggle_hotkey = ""
```

#### Exemple `postits.txt` (mode file)
```
# Lines starting with # are comments
# Format: [title]|[body]  — title optional
|Buvez de l'eau et étirez-vous !
Rappel: Revue de code à 10h|Préparez les notes pour l'API auth
Citation du jour|"Le code est lu plus souvent qu'il n'est écrit." — Guido van Rossum
|Pensez à commit & push avant le déjeuner
```

---

## 4. Spécifications Techniques

### 4.1 Architecture
```
┌─────────────────────────────────────────────────────────────┐
│                     POST-IT APP                              │
├─────────────────────────────────────────────────────────────┤
│  System Listener (Daemon/Service)                           │
│  ├─ Windows: WM_POWERBROADCAST + Task Scheduler             │
│  ├─ Linux:   systemd user service + logind D-Bus            │
│  └─ macOS:   LaunchAgent + NSWorkspace notifications        │
├─────────────────────────────────────────────────────────────┤
│  Config Loader (TOML parser)                                │
│  ├─ Watch file changes (hot-reload optionnel)               │
│  └─ Validation + valeurs par défaut                         │
├─────────────────────────────────────────────────────────────┤
│  Content Provider                                           │
│  ├─ Fixed / Sequential / Random / File                      │
│  └─ Persistence (index séquentiel)                          │
├─────────────────────────────────────────────────────────────┤
│  UI Renderer (Fenêtre unique, always-on-top)                │
│  ├─ Overlay fullscreen (couleur + opacité)                  │
│  ├─ Post-it centré (titre "onglet" + corps)                 │
│  ├─ Custom fonts (FreeType / DirectWrite / Core Text)       │
│  └─ Click-through / focus management                        │
└─────────────────────────────────────────────────────────────┘
```

### 4.2 Stack Technologique Recommandée

| Couche | Options recommandées | Justification |
|--------|---------------------|---------------|
| **Langage** | **Rust** (tauri, egui, iced, slint) ou **Go** (fyne, wails) ou **C#** (.NET 8+ / Avalonia) | Binaire unique, multi-plateforme, perf, taille réduite |
| **UI Framework** | `egui` (Rust), `Avalonia` (C#), `Fyne` (Go), `Tauri` (WebView) | Natif, pas de WebView lourd, custom drawing facile |
| **Config Parsing** | `toml` (Rust/Go/C#), `serde` | Standard, typé, commentaires supportés |
| **Fonts** | `rusttype`/`fontdue`/`ab_glyph` (Rust), `FreeType` bindings | Chargement `.ttf`/`.otf` runtime |
| **System Events** | `windows-rs` / `tokio` + `zbus` (Linux) / `objc2` (macOS) | Écoute boot/wake bas niveau |
| **Packaging** | `cargo-binstall`, `cargo-deb`, `wix` (MSI), `appimage`, `dmg` | Distribution native par OS |

> **Alternative "Rapid Prototyping" : Python + `pystray` + `tkinter`/`customtkinter`/`pywebview`** — plus lent au démarrage, mais dev plus rapide. À éviter pour la version finale "portable".

### 4.3 Contraintes Non-Fonctionnelles

| Exigence | Critère |
|----------|---------|
| **Temps d'affichage** | < 500 ms après événement déclencheur (boot/wake) |
| **Empreinte mémoire** | < 30 Mo RAM (idle), < 50 Mo (affiché) |
| **Taille binaire** | < 10 MB (strip + LTO) |
| **Démarrage à froid** | < 200 ms (daemon déjà en mémoire) |
| **Compatibilité OS** | Windows 10/11, Ubuntu 22.04+/Fedora 38+/Arch, macOS 12+ (Intel + Apple Silicon) |
| **DPI / HiDPI** | Support natif scaling (fractional scaling Wayland, 125-250% Windows) |
| **Accessibilité** | Contraste configurable, lecture d'écran (ARIA/UIA) — titre + corps exposés |
| **Sécurité** | Pas de réseau, pas de privilèges admin, config user-writable only |
| **Persistance** | Index séquentiel sauvé dans `%APPDATA%` / `~/.local/share/postit/` |

### 4.4 Gestion des Polices Custom
- Recherche dans l'ordre :
  1. Chemin absolu dans `font_path`
  2. Relatif au dossier de l'exécutable
  3. Dossier `fonts/` à côté de l'exe (via `custom_fonts_dir`)
  4. Dossiers système standards
- Cache des glyphes pour perf
- Fallback gracieux vers police système si fichier introuvable

---

## 5. Interfaces et Intégrations

| Interface | Description |
|-----------|-------------|
| **Fichier config** | TOML (lecture seule au démarrage + watch optionnel) |
| **Fichier contenu** | Texte brut (1 post-it/ligne) ou TOML/JSON structuré |
| **Variables d'env** | `POSTIT_CONFIG_PATH` pour forcer l'emplacement du `.conf` |
| **Logs** | Stdout/stderr + fichier rotating (optionnel) |
| **IPC (optionnel v2)** | Socket Unix / Named Pipe pour commande `show` / `hide` / `next` / `reload` |

---

## 6. Plan de Tests et Critères de Recette

### 6.1 Tests Unitaires
- Parsing config (valides, invalides, valeurs par défaut)
- Sélection contenu : fixed, sequential (persistance index), random (distribution), file (rechargement)
- Calcul position (centré, coins, custom, multi-écran)
- Chargement polices (valide, invalide, fallback)

### 6.2 Tests d'Intégration
| Scénario | Critère de succès |
|----------|-------------------|
| Boot cold (PC éteint → allumé) | Post-it visible < 2s après bureau chargé |
| Wake from sleep (veille RAM) | Post-it visible < 1s après déverrouillage session |
| Wake from hibernation | Idem |
| Changement config (hot-reload) | UI se met à jour sans relancer (si implémenté) |
| Multi-écran (débranchement) | Post-it reste sur écran principal ou suit config |
| DPI change (dock/undock laptop) | Pas de flou, taille correcte |
| Overlay click-through | Clics traversent vers fenêtre dessous |
| Always-on-top | Post-it reste au-dessus après Alt-Tab, Win+D, nouvelle fenêtre plein écran |

### 6.3 Tests Non-Fonctionnels
- **Charge** : 100 affichages consécutifs sans fuite mémoire
- **Longue durée** : Daemon 7j sans crash / fuite
- **Concurrences** : Double déclenchement boot+wake rapprochés → 1 seul post-it

### 6.4 Critères d'Acceptation (Definition of Done)
- [ ] Binaire unique par OS (< 10 MB)
- [ ] Config TOML documentée + exemple complet
- [ ] Détection boot + wake fonctionnelle sur 3 OS
- [ ] Toutes options graphiques appliquées visuellement
- [ ] Overlay couleur + % intensité fonctionnel
- [ ] Post-it centré + always-on-top + no focus steal
- [ ] Polices custom `.ttf`/`.otf` chargées pour titre + corps
- [ ] Modes contenu : fixed, sequential, random, file
- [ ] Pas de régression après veille/hibernation/reboot
- [ ] README d'installation + config par OS

---

## 7. Livrables

| Livrable | Format | Description |
|----------|--------|-------------|
| **Exécutables** | `.exe` / `AppImage` / `.dmg` / `.deb` / `.apk` (opt) | Signés si possible |
| **Config exemple** | `postit.conf.toml.example` | Commenté exhaustivement |
| **Contenu exemple** | `postits.txt.example` | 10-15 post-its variés |
| **Documentation** | `README.md` + `CONFIG.md` | Installation, config, FAQ, dépannage |
| **Sources** | Git repo (GitHub/GitLab) | Licence MIT ou Apache-2.0 |
| **CI/CD** | GitHub Actions / GitLab CI | Build matrix 3 OS, artifacts release |

---

## 8. Planning et Jalons (Estimatif)

| Jalon | Description | Durée estimée |
|-------|-------------|---------------|
| **M1 — Architecture & Config** | Setup projet, parsing TOML, structure données, logging | 1-2 semaines |
| **M2 — System Listeners** | Détection boot/wake sur 3 OS (daemon/service) | 2-3 semaines |
| **M3 — UI Core** | Fenêtre always-on-top, overlay, positionnement, polices | 2-3 semaines |
| **M4 — Content Engine** | Modes fixed/sequential/random/file, persistance index | 1 semaine |
| **M5 — Polish & Config** | Toutes options graphiques, hot-reload, validation | 1-2 semaines |
| **M6 — Tests & Packaging** | Tests auto, build matrix, installers, signature | 1-2 semaines |
| **M7 — Release v1.0** | Tag, release notes, binaires, doc | 3 jours |

**Total estimé : 8-13 semaines** (1 dev full-time)  
*Peut être parallélisé (M2 + M3 en //)*

---

## 9. Risques et Mitigations

| Risque | Probabilité | Impact | Mitigation |
|--------|-------------|--------|------------|
| Détection wake peu fiable sur Linux (Wayland/hybrid sleep) | Moyenne | Haute | Fallback : timer périodique + check `logind` `IdleHint` ; doc known issues |
| Polices custom non trouvées au boot (disque chiffré non monté) | Faible | Moyenne | Fallback police système ; log warning non bloquant |
| Overlay click-through cassé sur Wayland | Moyenne | Moyenne | Option `blur_background` alternative ; doc limitation |
| Conflit always-on-top avec autres apps (barre de jeux, overlay Discord) | Faible | Faible | Z-order "Topmost" standard ; option `window.level` configurable |
| Taille binaire trop grosse (dépendances UI lourdes) | Moyenne | Moyenne | Choix framework léger (egui/slint vs Tauri/WebView) ; LTO + strip |

---

## 10. Annexes

### A. Glossaire
| Terme | Définition |
|-------|------------|
| **Boot** | Démarrage complet de l'OS jusqu'au bureau utilisateur |
| **Wake** | Sortie de veille (S3/S0ix) ou hibernation |
| **Overlay** | Calque plein écran derrière le post-it, coloré + semi-transparent |
| **Click-through** | Propriété fenêtre : les événements souris traversent vers la fenêtre dessous |
| **Always-on-top** | Fenêtre maintenue au-dessus de toutes les autres (Z-order maximal) |
| **TOML** | Tom's Obvious, Minimal Language — format config lisible, typé |

### B. Exemples de Configs Thématiques

#### Thème "Minimaliste Noir/Blanc"
```toml
[title]
enabled = false
[body]
font_path = "fonts/IBM_Plex_Mono/IBMPlexMono-Regular.ttf"
font_size = 14
color = "#FFFFFF"
bg_color = "#000000"
padding = 20
border_radius = 0
[overlay]
color = "#000000"
opacity = 0.50
```

#### Thème "Papier Jaune Classique"
```toml
[title]
text = "📝 Note"
bg_color = "#D4A843"
color = "#2C1810"
font_path = "fonts/Patrick_Hand/PatrickHand-Regular.ttf"
[body]
font_path = "fonts/Patrick_Hand/PatrickHand-Regular.ttf"
font_size = 16
color = "#2C1810"
bg_color = "#FDF6D9"
bg_image = "textures/paper_yellow.png"
[overlay]
enabled = false
```

#### Thème "Cyberpunk Neon"
```toml
[title]
text = "// SYSTÈME PRÊT //"
bg_color = "#0A0A0F"
color = "#00FFCC"
font_path = "fonts/JetBrains_Mono/JetBrainsMono-Bold.ttf"
border_radius = 0
[body]
font_path = "fonts/JetBrains_Mono/JetBrainsMono-Regular.ttf"
color = "#00FFCC"
bg_color = "#0D0D14"
border_radius = 0
[overlay]
color = "#000000"
opacity = 0.70
blur_background = true
```

### C. Références Techniques
- [Windows WM_POWERBROADCAST](https://learn.microsoft.com/en-us/windows/win32/power/wm-powerbroadcast)
- [Linux systemd-sleep hooks](https://www.freedesktop.org/software/systemd/man/systemd-sleep.html)
- [macOS NSWorkspace notifications](https://developer.apple.com/documentation/appkit/nsworkspace/1413307-didwakenotification)
- [TOML Spec v1.0](https://toml.io/en/v1.0.0)
- [egui / slint / Avalonia / Fyne] — choisir selon stack finale

---

## 11. Validation et Approbation

| Rôle | Nom | Signature | Date |
|------|-----|-----------|------|
| Product Owner / Commanditaire | | | |
| Lead Développeur | | | |
| UX / Design (si applicable) | | | |
| Ops / IT (déploiement) | | | |

---

> **Note :** Ce document est vivant. Toute modification doit être versionnée (Git) et tracée dans l'historique. Les versions ultérieures incrémenteront le numéro de version (1.1, 1.2, 2.0...).