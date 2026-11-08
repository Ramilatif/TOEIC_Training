# Cap 800 — Entraînement TOEIC

Application web d'entraînement au TOEIC Listening & Reading (objectif 800 points). Noms, design et
contenus 100 % originaux, pour un usage entre amis et collègues.

## Fonctionnalités

- Connexion par pseudo (sans mot de passe pour l'instant, module d'authentification isolé)
- Banque de **608 questions d'examen originales** (plus de 2 examens complets, auxquelles s'ajoutent les quiz de cours), explications en français et en anglais
- Entraînement par partie (Parts 1 à 7) avec correction immédiate ; audio par synthèse vocale du navigateur
  (accents en-US, en-GB, en-AU quand ils sont installés)
- Examens blancs : complet (200 questions, 2 h) et mini-tests paramétrables, correction détaillée
- Tableau de bord : score estimé (10-990), évolution, réussite par partie et par catégorie, écart avec l'objectif
- Révision des erreurs, priorisée par catégorie faible
- 67 cours (18 grammaire, 10 conjugaison, 17 vocabulaire, 7 méthodologie, 15 niveau avancé C1) avec mini-quiz et suivi de progression
- Flashcards à répétition espacée (SM-2) alimentées par les questions, les cours et vos ajouts
- Planning adaptatif jusqu'à la date d'examen

## Lancer l'appli

Prérequis : Docker (sous Windows, Docker Desktop démarré).

```
docker compose up --build
```

Puis ouvrir http://localhost:8080 et saisir un pseudo.
Configuration facultative : copier `.env.example` en `.env`. Les données (SQLite) sont dans le volume Docker `data`.
Pour l'audio, utilisez Chrome ou Edge (voix installées dans Windows).

## Photos de la Part 1

Les 12 questions de la Part 1 attendent des photos libres de droits (Unsplash, Pexels). Déposez
`p1-01.jpg` … `p1-12.jpg` dans `content/images/part1/` et complétez `credits.json` : voir
[content/images/part1/README.md](content/images/part1/README.md) pour la liste des scènes à chercher.
Sans les photos, ces questions ne sont simplement pas proposées.

## Développement sans Docker

Prérequis : Rust stable, Node.js 20+.

```
cd backend  && CONTENT_DIR=../content cargo run     # API sur :3000
cd frontend && npm install && npm run dev            # interface sur :5173 (proxy vers :3000)
```

Tests et lint : voir CLAUDE.md. Architecture, schéma, formats de contenu : docs/ARCHITECTURE.md.
Avancement et décisions : PROGRESS.md.

## Mise en ligne (plus tard)

Aucun secret en dur : tout passe par des variables d'environnement (`.env.example`). Avant d'ouvrir
l'appli au public, ajouter un mot de passe dans le module `backend/src/auth/` (voir docs/ARCHITECTURE.md §11).
