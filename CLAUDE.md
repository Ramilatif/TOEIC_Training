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
- Environnement : Docker Compose, poste de dev sous Arch Linux

## Structure

- backend/ : API Rust
- backend/migrations/ : migrations sqlx
- frontend/ : application React
- content/questions/ : banque de questions (JSON, un fichier par partie et par lot)
- content/lessons/ : leçons (Markdown avec métadonnées)
- content/images/part1/ : photos libres de droits pour la Part 1
- docs/ : documentation technique

## Commandes

- Lancer l'appli : docker compose up --build
- Tests backend : cd backend && cargo test
- Lint backend : cd backend && cargo clippy -- -D warnings && cargo fmt --check
- Tests frontend : cd frontend && npm test
- Lint frontend : cd frontend && npm run lint
- Nouvelle migration : cd backend && sqlx migrate add <nom>

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
