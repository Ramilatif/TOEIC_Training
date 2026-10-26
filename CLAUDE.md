# Appli d'entraînement TOEIC

Application web d'entraînement au TOEIC Listening & Reading (objectif utilisateur : 800 points),
utilisée par un petit groupe d'amis/collègues. Contenus et design 100 % originaux.

## Documents de référence

- Instructions complètes du projet : @PROMPT.md
- Avancement, décisions et points à vérifier : @PROGRESS.md
- Architecture validée : @docs/ARCHITECTURE.md

En cas de contradiction : PROMPT.md fait foi, puis ARCHITECTURE.md.

## Stack

- Backend : Rust (Axum), sqlx, SQLite (requêtes compatibles PostgreSQL)
- Frontend : React + TypeScript, responsive
- Environnement : Docker Compose, poste de dev sous Windows 11 (shell Git Bash / PowerShell)

## Architecture (vue d'ensemble)

- Backend : `lib.rs` expose `app(AppState)` ; `main.rs` ne fait que démarrer (config, migrations, import du contenu, serveur, `/media` pour les photos). Les tests d'intégration (`backend/tests/api.rs`) importent le VRAI contenu de `content/` dans une base SQLite en mémoire : un JSON ou une leçon invalide fait donc échouer `cargo test`.
- Logique métier pure et testée, sans base : `scoring/` (conversion 5-495, sélection des questions d'examen, estimation), `srs/` (SM-2), `planning/` (génération du programme). Les routes (`routes/`) ne font que lire/écrire en base et appeler ces modules.
- Auth isolée dans `auth/` (trait `Authenticator`, jeton de session opaque en base) : le mot de passe se branchera là.
- Contenu : `content/` est importé à chaque démarrage (idempotent, `ON CONFLICT DO UPDATE`). Le quiz d'une leçon = questions du lot `lesson-<id de la leçon>` (5 à 10 exigées). Part 1 : une question dont la photo est absente de `content/images/part1/` n'est pas importée.
- Frontend : React Router, un client typé dans `src/api/client.ts`, i18n FR/EN dans `src/i18n/index.ts` (la langue vient du profil). Audio derrière l'interface `AudioEngine` (`src/audio/`).
- Statistiques et dashboard : basés sur la DERNIÈRE tentative de chaque question ; les quiz de leçon n'enregistrent pas de tentatives.

## Structure

- backend/ : API Rust
- backend/migrations/ : migrations sqlx
- frontend/ : application React
- content/questions/ : banque de questions (JSON, un fichier par partie et par lot)
- content/lessons/ : leçons (Markdown avec métadonnées)
- content/images/part1/ : photos libres de droits pour la Part 1
- docs/ : documentation technique

## Commandes

- Lancer l'appli : docker compose up --build (http://localhost:8080)
- Backend seul : cd backend && CONTENT_DIR=../content cargo run ; frontend seul : cd frontend && npm run dev
- Un seul test : cargo test <nom> (backend) ; npx vitest run <fichier> (frontend)
- Tests backend : cd backend && cargo test
- Lint backend : cd backend && cargo clippy -- -D warnings && cargo fmt --check
- Tests frontend : cd frontend && npm test
- Lint frontend : cd frontend && npm run lint
- Nouvelle migration : ajouter backend/migrations/NNNN_nom.sql (embarquée par sqlx::migrate!, pas de sqlx-cli requis)
- Node.js n'est pas dans le PATH de Git Bash : export PATH="$PATH:/c/Program Files/nodejs"
- Éviter les heredocs avec apostrophes dans l'outil Bash : utiliser l'outil Write
- Ajouter du contenu : respecter le format de docs/ARCHITECTURE.md §5-6 ; répartir les bonnes réponses (pas de biais vers « A ») ; relancer `cd backend && cargo test` pour valider tout le contenu

## Règles de travail

- Exécution autonome : enchaîner les phases sans attendre de validation,
  sauf blocage réel (accès ou installation système imprévu, erreur insoluble)
- Ne rien installer au niveau système et ne rien modifier hors du dossier du projet
- Une phase n'est terminée que si tout compile, les lints passent et les tests sont verts
- Au moins un commit par phase, messages clairs (ex. "phase 3: examens blancs et scoring")
- Mettre à jour PROGRESS.md à la fin de chaque phase et à chaque décision non prévue par PROMPT.md
- Aucun secret en dur : configuration par variables d'environnement, .env dans .gitignore,
  .env.example versionné

## Règles métier

- La liste des catégories (docs/ARCHITECTURE.md) est la référence unique pour les questions,
  leçons, statistiques et recommandations. Ne pas la modifier sans migration explicite
  et réétiquetage du contenu existant
- Répartition des questions conforme au TOEIC officiel (Parts 1 à 7)
- Chaque question et chaque leçon : explications en français ET en anglais
- Contenus strictement originaux : rien de copié depuis ETS ou une plateforme existante
- Valider le format de chaque lot de contenu (JSON ou Markdown) avant de passer au suivant
- L'audio passe par une interface interchangeable : Web Speech API aujourd'hui,
  moteur TTS local (ex. Piper) possible plus tard
- Connexion par pseudo sans mot de passe pour l'instant : garder l'auth isolée dans un module
  pour pouvoir ajouter un mot de passe avant la mise en ligne

## Reprise après interruption

En début de session, si le projet est déjà entamé :

1. Lire PROGRESS.md
2. Vérifier l'état du dépôt (git log --oneline -10, git status)
3. Terminer ou annuler proprement tout travail laissé à moitié
4. Reprendre à la phase indiquée dans PROGRESS.md
