# Architecture — Appli d'entraînement TOEIC

Document de référence validé. En cas de contradiction : PROMPT.md fait foi, puis ce document.
La liste des catégories (section 7) ne change plus sans migration explicite et réétiquetage du contenu.

## 1. Arborescence

```
backend/                 API Rust (Axum, sqlx)
  Cargo.toml
  migrations/            migrations sqlx (SQL compatible SQLite et PostgreSQL)
  src/
    main.rs              démarrage, config, import du contenu, serveur
    config.rs            variables d'environnement
    db.rs                pool sqlx, migrations
    error.rs             type d'erreur API
    auth/                module isolé : pseudo aujourd'hui, mot de passe plus tard
    content/             import JSON/Markdown -> base, validation de format
    scoring/             table de conversion et calcul du score (logique pure, testée)
    srs/                 algorithme SM-2 (logique pure, testée)
    planning/            génération et réajustement du programme (logique pure, testée)
    routes/              handlers par domaine (questions, exams, stats, lessons, cards, plan)
frontend/                React + TypeScript (Vite)
  src/
    api/                 client HTTP typé
    audio/               interface AudioEngine + WebSpeechEngine
    pages/ components/ i18n/
content/
  categories.json        liste unique des catégories (source de vérité)
  questions/             part{N}-batch{NN}.json
  lessons/               {type}-{slug}.md
  images/part1/          p1-01.jpg ... + credits.json
docs/ARCHITECTURE.md
docker-compose.yml, .env.example, README.md, PROGRESS.md, CLAUDE.md
```

## 2. Choix techniques

- Backend Axum, sqlx (requêtes runtime `query`/`query_as`, pas de macros `query!` pour rester
  indépendant de la base et éviter une base de dev à la compilation). Migrations embarquées
  (`sqlx::migrate!`) et appliquées au démarrage.
- Compatibilité PostgreSQL : types `TEXT`, `INTEGER`, `BIGINT`, `BOOLEAN`; identifiants `TEXT`
  (UUID ou slug) générés par l'application; dates en `TEXT` ISO-8601 UTC; JSON stocké en `TEXT`;
  pas de fonctions propres à SQLite; paramètres `$1, $2` (acceptés par SQLite).
- Frontend servi par nginx en production Docker, proxy `/api` vers le backend. En dev : Vite.
- Sessions : jeton aléatoire opaque stocké en base, envoyé en en-tête `Authorization: Bearer`.
- Ports : frontend 8080, backend 3000 (interne), Vite 5173.

## 3. Schéma de base de données

| Table | Colonnes principales |
|---|---|
| users | id, pseudo (unique, insensible à la casse), password_hash (NULL pour l'instant), goal_score (défaut 800), exam_date, daily_minutes, lang_pref (fr/en), created_at |
| sessions | token (PK), user_id, created_at, expires_at |
| categories | id (slug), kind (grammar/vocab/question_type), label_fr, label_en |
| passages | id, part, kind (single/double/triple/conversation/talk/text), title, body_text, transcript, image_id (nullable) |
| questions | id (slug stable), part, difficulty (1-3), category_id, passage_id (nullable), position_in_passage, stem, choices_json, correct_index, explanation_fr, explanation_en, batch |
| question_vocab | question_id, term, translation_fr (pour alimenter les flashcards) |
| images | id, path, credit, source_url, alt_text |
| attempts | id, user_id, question_id, chosen_index, is_correct, mode (practice/exam/mini/review), exam_id (nullable), answered_at, time_ms |
| exams | id, user_id, kind (full/mini), config_json, started_at, finished_at, duration_limit_s, listening_raw, reading_raw, listening_scaled, reading_scaled |
| exam_questions | exam_id, question_id, position |
| lessons | id (slug), type (grammar/vocab/method), category_id, level, minutes, title_fr, title_en, body_fr_md, body_en_md |
| lesson_quiz_questions | lesson_id, question_id (les quiz réutilisent la table questions avec batch = lesson) |
| lesson_progress | user_id, lesson_id, read_at, quiz_passed_at, best_quiz_score |
| lesson_vocab | lesson_id, term, translation_fr, example |
| flashcards | id, user_id, term, translation, example, source (question/lesson/manual), source_id, ease, interval_days, repetitions, due_date, created_at |
| card_reviews | id, card_id, quality (0-5), reviewed_at |
| plans | id, user_id, generated_at, exam_date, daily_minutes |
| plan_items | id, plan_id, day (date), kind (lesson/practice/review/flashcards/mock), ref_id, minutes, done_at |

Index : attempts(user_id, answered_at), attempts(user_id, question_id), flashcards(user_id, due_date),
questions(part, category_id).

## 4. Endpoints (préfixe `/api`)

| Méthode | Route | Rôle |
|---|---|---|
| POST | /auth/login | { pseudo } -> crée l'utilisateur si absent, renvoie { token, user } |
| POST | /auth/logout | invalide la session |
| GET/PATCH | /me | profil (objectif, date d'examen, minutes/jour, langue) |
| GET | /categories | liste des catégories |
| GET | /questions?part=&category=&difficulty=&limit=&unseen= | sélection pour l'entraînement (sans la réponse) |
| POST | /questions/{id}/answer | { chosen_index } -> correction, explications, transcription, lien vers leçon |
| GET | /images/{id} | photo Part 1 (fichiers statiques) |
| POST | /exams | { kind, parts, count, timed } -> crée l'examen (full = 200 questions, 2 h) |
| GET | /exams/{id} | questions de l'examen (sans réponses) et temps restant |
| POST | /exams/{id}/answers | enregistre une réponse (sauvegarde au fil de l'eau) |
| POST | /exams/{id}/finish | calcule les scores, renvoie la correction complète (idempotent) |
| GET | /exams/{id}/results | résultats d'un examen terminé |
| GET | /exams | historique |
| GET | /stats/overview | score estimé, évolution, écart à l'objectif |
| GET | /stats/by-part, /stats/by-category | précision par partie / catégorie |
| GET | /review/mistakes?part=&category=&limit= | erreurs (dernière tentative fausse) triées par catégorie faible ; rejouées via POST /questions/{id}/answer avec mode=review |
| GET | /lessons?type=&category= | liste avec progression |
| GET | /lessons/{id}?lang= | contenu |
| POST | /lessons/{id}/read, /lessons/{id}/quiz | marquer lue, soumettre le quiz |
| GET | /lessons/progress | pourcentage par module |
| GET/POST | /cards | liste, ajout manuel |
| POST | /cards/from-lesson/{id}, /cards/from-question/{id} | ajoute le vocabulaire (sans doublon) |
| DELETE | /cards/{id} | suppression |
| GET | /cards/due?limit= | cartes à réviser aujourd'hui (+ total) |
| POST | /cards/{id}/review | { quality 0-5 } -> mise à jour SM-2 |
| GET/POST | /plan | plan courant (réajusté automatiquement une fois par jour) ; (re)génération |
| POST | /plan/items/{id}/done | { done } coche / décoche un élément |

Erreurs : JSON `{ "error": { "code", "message" } }` avec codes HTTP standards.

## 5. Format JSON d'une question

Un fichier = un lot : `content/questions/part5-batch01.json`.

```json
{
  "batch": "part5-batch01",
  "part": 5,
  "passages": [
    { "id": "p7-b01-001", "kind": "double", "title": "Email + Invoice",
      "body": "…texte(s) du passage…", "transcript": null, "image": null }
  ],
  "questions": [
    {
      "id": "p5-b01-001",
      "part": 5,
      "difficulty": 2,
      "category": "gram-verb-tenses",
      "passage": null,
      "position": 1,
      "stem": "The manager ___ the report before the meeting started.",
      "choices": ["has finished", "had finished", "finishes", "will finish"],
      "correct": 1,
      "explanation": {
        "fr": { "rule": "…", "why_wrong": ["A: …", "C: …", "D: …"], "example": "…" },
        "en": { "rule": "…", "why_wrong": ["A: …", "C: …", "D: …"], "example": "…" }
      },
      "vocab": [ { "term": "report", "fr": "rapport" } ]
    }
  ]
}
```

Règles de validation (à l'import, erreur explicite avec fichier et id) : id unique et stable
(l'import est idempotent, `INSERT … ON CONFLICT DO UPDATE`), catégorie existante, `part` cohérent,
`correct` dans les bornes, 4 choix (3 pour Part 2), `why_wrong` couvrant toutes les mauvaises réponses,
FR et EN présents. Part 1 : `passage.image` référence un id d'image ; Parts 2-4 : `transcript`
contient le texte lu par la synthèse vocale (Part 2 : question et 3 réponses lues, les choix ne sont
pas affichés avant la correction). Photos : `content/images/part1/credits.json` associe
`{ id, file, credit, source_url, alt }`, une question Part 1 pointe vers `image: "p1-03"`.

Répartition officielle : Part 1 : 6, Part 2 : 25, Part 3 : 39, Part 4 : 30, Part 5 : 30, Part 6 : 16,
Part 7 : 54 par examen. Banque pour 2 examens : 12 / 50 / 78 / 60 / 60 / 32 / 108 = 400.

## 6. Format Markdown d'une leçon

`content/lessons/grammar-verb-tenses.md`, avec front matter YAML :

```markdown
---
id: grammar-verb-tenses
type: grammar          # grammar | conjugation | vocab | method
category: gram-verb-tenses
level: intermediate    # beginner | intermediate | advanced
minutes: 12
title: { fr: "Les temps verbaux", en: "Verb tenses" }
vocab:
  - { term: "deadline", fr: "date limite", example: "The deadline is Friday." }
---
<!-- lang:fr -->
# Les temps verbaux
…explication, exemples, tableau récapitulatif…

<!-- lang:en -->
# Verb tenses
…
```

Le quiz d'une leçon (5 à 10 questions, exigé à l'import) = les questions du lot
`content/questions/lesson-<id de la leçon>.json` (champ `batch` = `lesson-<id>`). Ces questions sont
exclues de l'entraînement, des examens, des statistiques et du score estimé. Le quiz est validé à
70 % ; réussir le quiz marque aussi la leçon comme lue. Progression d'un module = moyenne de
50 % (lue) + 50 % (quiz réussi) par leçon. L'id d'une leçon commence par son type (`grammar-`,
`conjugation-`, `vocab-`, `method-`) ; les leçons de méthodologie s'appellent `method-part-1` à `method-part-7` ; les leçons de conjugaison sont numérotées (`conjugation-01-…` à `conjugation-10-…`) pour s'afficher dans l'ordre pédagogique. Elles partagent les catégories de grammaire existantes (temps, passif, modaux, conditionnels, familles de mots) : le lien « leçon liée » préfère toujours la leçon de grammaire.

## 7. Liste des catégories (référence unique)

Fichier source : `content/categories.json`. Identifiants stables, jamais renommés.

**Grammaire (`grammar`)** : gram-verb-tenses (temps verbaux), gram-subject-verb-agreement (accord
sujet-verbe), gram-passive-voice (voix passive), gram-prepositions (prépositions), gram-conjunctions
(conjonctions et connecteurs), gram-relative-pronouns (pronoms relatifs), gram-pronouns (pronoms),
gram-comparatives-superlatives (comparatifs et superlatifs), gram-articles-determiners (articles et
déterminants), gram-modals (modaux), gram-conditionals (conditionnels), gram-gerund-infinitive
(gérondif et infinitif), gram-participles (participes), gram-word-forms (familles de mots :
nom/verbe/adjectif/adverbe), gram-quantifiers (quantificateurs), gram-reported-speech (discours
rapporté et sujets inversés).

**Vocabulaire (`vocab`)** : voc-office (bureau), voc-meetings (réunions), voc-travel (voyages),
voc-finance (finance et banque), voc-hr (ressources humaines), voc-purchasing (achats et ventes),
voc-health (santé), voc-marketing (marketing), voc-manufacturing (production et logistique),
voc-real-estate (immobilier), voc-dining (restauration et événements), voc-technology (technologie),
voc-general (vocabulaire général des affaires), voc-collocations (collocations et expressions).

**Types de questions (`question_type`)** : qt-photo-people (Part 1 personnes), qt-photo-objects
(Part 1 objets et lieux), qt-wh-questions (Part 2 questions en wh-), qt-yes-no-questions (Part 2
questions fermées), qt-statements-requests (Part 2 propositions et demandes), qt-indirect-answers
(Part 2 réponses indirectes), qt-conversation-gist (Part 3 sujet principal), qt-conversation-detail
(Part 3 détail), qt-conversation-inference (Part 3 inférence et intention), qt-visual-info (Part 3/4
document associé), qt-talk-gist (Part 4 sujet principal), qt-talk-detail (Part 4 détail),
qt-talk-inference (Part 4 inférence et intention), qt-text-completion (Part 6 complétion),
qt-sentence-insertion (Part 6 insertion de phrase), qt-reading-main-idea (Part 7 idée principale),
qt-reading-detail (Part 7 détail), qt-reading-inference (Part 7 inférence), qt-reading-vocab-context
(Part 7 vocabulaire en contexte), qt-reading-sentence-position (Part 7 position d'une phrase),
qt-multi-passage (Part 7 passages multiples).

Une question porte exactement une catégorie principale (grammaire/vocabulaire pour les Parts 5-6,
type de question pour les autres). Les statistiques par catégorie, les leçons liées et les
recommandations utilisent cet identifiant.

## 8. Scoring

- Score brut = bonnes réponses (0-100) par section. Pas de pénalité.
- Table de conversion approximative (non officielle, ETS ne publie pas de table fixe) : interpolation
  linéaire par paliers stockée dans `scoring/` : brut 0 -> 5, 15 -> 60, 30 -> 130, 45 -> 215, 60 -> 300,
  75 -> 385, 85 -> 440, 95 -> 485, 100 -> 495 (arrondi au multiple de 5, 5 minimum).
  Total = Listening + Reading (10 à 990). Cette table est affichée à l'utilisateur avec la mention
  « estimation ».
- Score estimé du tableau de bord : calculé sur les dernières réponses pondérées par partie, selon la
  répartition officielle.

## 9. Audio

Interface frontend :

```ts
interface AudioEngine {
  listVoices(): Voice[];                       // en-US, en-GB, en-AU si disponibles
  speak(segments: Segment[], opts): Promise<void>; // Segment = { text, voice?, pauseAfterMs }
  stop(): void;
}
```

`WebSpeechEngine` est l'implémentation actuelle. Un `PreGeneratedAudioEngine` (fichiers audio
produits par Piper) pourra s'y substituer : le champ `audio_url` optionnel d'un passage servira
alors de source. Les conversations Part 3 utilisent deux voix différentes.

## 10. Planning et répétition espacée

- SM-2 : qualité 0-5; si q < 3 répétitions = 0, intervalle = 1 jour et facilité inchangée (SM-2
  d'origine); sinon intervalles 1, 6, puis `interval × ease`; l'interface envoie 1 (À revoir), 3 (Difficile),
  4 (Bien), 5 (Facile); `ease = max(1.3, ease + 0.1 − (5−q)(0.08 + (5−q)·0.02))`.
- Planning : de aujourd'hui à la date d'examen, répartit chaque jour le temps disponible entre
  leçons des 3 catégories les plus faibles, exercices ciblés, révision d'erreurs, flashcards dues, et
  un examen blanc par semaine à l'approche de la date. Réajusté après chaque élément terminé et au
  plus une fois par jour (la précision par catégorie est recalculée).

## 11. Authentification

Module `auth/` isolé derrière un trait `Authenticator`. Aujourd'hui : pseudo seul. Pour ajouter un
mot de passe : renseigner `users.password_hash` (argon2), ajouter `password` à `/auth/login`; aucune
autre table à modifier. Le pseudo est normalisé (trim, 3-30 caractères alphanumériques, `_` et `-`).

## 12. Docker et configuration

- `docker-compose.yml` : services `backend` (build multi-étapes Rust) et `frontend` (build Node puis
  nginx), volume `data` pour la base SQLite, `content/` monté en lecture seule.
- Variables (voir `.env.example`) : `DATABASE_URL` (défaut `sqlite:///data/toeic.db`),
  `CONTENT_DIR`, `BIND_ADDR`, `SESSION_TTL_HOURS`, `CORS_ORIGIN`.
- Aucun secret en dur; `.env` dans `.gitignore`.

## 13. Tests

- Rust : tests unitaires de `scoring`, `srs`, `planning`, de la validation d'import et des routes
  principales (base SQLite en mémoire).
- Frontend : Vitest + Testing Library pour les composants critiques (entraînement, minuteur d'examen,
  moteur audio simulé).
