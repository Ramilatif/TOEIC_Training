# Cap 800 — Entraînement TOEIC

Application web d'entraînement au TOEIC Listening & Reading (objectif 800), contenus originaux.

## Lancer l'appli

Prérequis : Docker (Docker Desktop sous Windows, démon démarré).

```
docker compose up --build
```

Puis ouvrir http://localhost:8080 et saisir un pseudo (aucun mot de passe pour l'instant).
Configuration : copier `.env.example` en `.env` (facultatif). Les données (SQLite) sont dans le volume Docker `data`.

## Développement sans Docker

Prérequis : Rust stable, Node.js 20+.

```
cd backend  && CONTENT_DIR=../content cargo run     # API sur :3000
cd frontend && npm install && npm run dev            # interface sur :5173 (proxy vers :3000)
```

Tests et lint : voir CLAUDE.md. Format des contenus et architecture : docs/ARCHITECTURE.md.

## Contenu

- `content/questions/` : lots JSON importés au démarrage (idempotent)
- `content/images/part1/` : photos Part 1 (+ `credits.json`)
