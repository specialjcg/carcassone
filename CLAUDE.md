# Carcassonne — Project Context for Claude

## Stack
- **Backend** : Rust + Axum + SQLite + JWT (mirror Qwirkle structure)
- **Frontend** : Elm 0.19.1 + ports pour interactions dynamiques
- **ML (optionnel)** : tch (libtorch bindings), feature-gated `neural`

## Architecture cible (mirror Qwirkle)
```
backend/
  src/
    api/         # handlers HTTP (Axum)
    domain/      # logique pure (tiles, board, scoring, AI)
    neural/      # [optionnel] graph transformer, MCTS, tensor conversion
    bin/         # binaires recherche : selfplay, evaluate, eval_paired, oracle_bound
    main.rs
frontend/
  src/
    Main.elm
    Page/
    Port.elm     # ports pour canvas/3D/interactions
  static/
    index.html
    elm.js       # généré
    style.css
```

## Règles essentielles Carcassonne (base game, 72 tiles)

**Tuiles** : 72 tuiles terrain. Bords = route, ville, pré (farm), ou cloître (intérieur).
**Placement** : tuile adjacente à existante, bords matching exactement.
**Meeples** : 7 par joueur. Placés sur route/ville/pré/cloître de la tuile posée.
**Scoring immédiat** (à la fermeture) :
- Route : 1 pt/tuile
- Ville : 2 pts/tuile + 2 pts/blason (1 pt/tuile si incomplète à la fin)
- Cloître : 9 pts (1 + 8 tuiles autour)
**Scoring endgame (fermes)** :
- 3 pts par ville complétée adjacente au pré
- C'est LE scoring stratégique critique

**Fin de partie** : sac vide + tuile finale posée.

## Leçons de Qwirkle (à appliquer ici)

### Méthodologie d'évaluation (OBLIGATOIRE dès le début)
**Paired evaluation avec bootstrap CI**. Ne jamais comparer des bots sur un petit N sans CI.
- Variance inter-partie énorme à cause du tirage de tuiles
- 50 pairs minimum pour CI utilisable
- Réutiliser le pattern de `qwirkle/backend/src/bin/eval_paired.rs`

### Greedy n'est PAS près-optimal ici (contrairement à Qwirkle)
- Fermes = signal long-terme de 30-50 pts
- Greedy ignore les fermes → bat largement en endgame
- **ML devrait réellement gagner contre greedy** (≥ 60% winrate)

### Oracle upper bound
- Au début, implémenter `oracle_bound.rs` avec info-future
- Si oracle >> greedy → signal exploitable → ML project viable
- Si oracle ≈ greedy → game over comme pour Qwirkle

### Action space
- Tile placement : rotation (4) × position (~30-50) × meeple (~5 options)
- Total : ~600-1000 actions/tour
- Ne PAS sous-modéliser comme on l'a fait en Qwirkle (seulement 1ère tuile)

### Pas d'AlphaZero en premier
- Commencer par MCTS pur avec rollouts greedy
- Paired eval vs greedy à chaque itération
- AlphaZero seulement si MCTS pur prouve un edge > 10 pts

### Reward shaping
- Score immédiat par tour (road/city closure)
- Score endgame (fermes) = composante distincte critique
- Value head doit séparer les deux signaux

## Règles ML héritées

**Tech stack ML** :
- libtorch 2.4.0+cu121 avec CXX11 ABI
- tch 0.17 (tch 0.18 a cassé CUDA detection)
- CUDA_VISIBLE_DEVICES=0, LIBTORCH=~/libtorch-cuda/libtorch

**Machine GPU** : 10.244.128.131 (RTX avec 3.6GB VRAM)

**Batch sizes** : LARGE model = batch 32 max (128 OOM)

## Références
- Projet Qwirkle : `/home/jcgouleau/Projects/personal/games/qwirkle/`
- Research binaires Qwirkle : `qwirkle/backend/src/bin/{eval_paired,instrument_greedy,oracle_bound}.rs`
- Leçons détaillées : commits `804c0ef`, `ed1ff0d` dans qwirkle
