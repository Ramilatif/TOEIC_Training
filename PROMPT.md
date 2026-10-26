# Contexte

Je prépare le TOEIC Listening & Reading avec un objectif de 800 points. Je veux une application web
d'entraînement, inspirée du fonctionnement de plateformes comme GlobalExam (cours, entraînements
ciblés, examens blancs, suivi de progression), avec un nom, un design et des contenus 100 % originaux :
ne reprends ni marque, ni interface, ni questions d'une plateforme existante ou d'annales ETS.
L'appli sera utilisée par moi et quelques amis/collègues.

# Stack et contraintes techniques

- Backend : Rust (Axum), sqlx avec migrations
- Base de données : SQLite, mais schéma et requêtes compatibles PostgreSQL pour une migration future
- Frontend : React (TypeScript), responsive PC et mobile
- Lancement local via Docker Compose (`docker compose up`) ; je suis sous Arch Linux
- Mise en ligne prévue plus tard : configuration par variables d'environnement (.env),
  aucun secret en dur, README avec instructions de lancement
- Style visuel sobre et professionnel
- Code commenté, tests unitaires sur la logique métier (scoring, répétition espacée, planning)

# Utilisateurs

- Connexion par pseudo simple, sans mot de passe
- Chaque utilisateur a ses propres données (historique, scores, progression des cours,
  flashcards, planning)
- Prévois l'architecture pour ajouter un mot de passe facilement avant la mise en ligne

# Structure du TOEIC à respecter

- Listening (45 min, 100 questions) : Part 1 Photographies, Part 2 Questions-réponses,
  Part 3 Conversations, Part 4 Exposés
- Reading (75 min, 100 questions) : Part 5 Phrases à compléter, Part 6 Textes à compléter,
  Part 7 Compréhension écrite (passages simples et multiples)
- Score estimé de 10 à 990 (5 à 495 par section), avec une table de conversion
  approximative documentée

# Catégories

- Définis dès l'architecture une liste unique de catégories (grammaire, thèmes de vocabulaire,
  types de questions). Questions, leçons, statistiques et recommandations s'appuient toutes
  sur cette liste : elle ne doit plus changer ensuite sans migration explicite.

# Banque de questions

- Tu rédiges toi-même une banque de 400 questions originales, soit l'équivalent de
  2 examens blancs complets, avec la répartition officielle par partie
- Production par lots, une partie à la fois, avec vérification du format à chaque lot
- Stockage en fichiers JSON importés en base au démarrage, schéma documenté
- Chaque question contient : partie, difficulté, catégorie, réponse correcte, explication
  en français ET en anglais (règle, pourquoi les autres réponses sont fausses, exemple)
- Part 1 : je fournirai des photos libres de droits (Unsplash, Pexels). Fournis-moi la liste
  des scènes à chercher et un système simple pour associer une photo à ses questions
- Aucune génération de contenu par API pendant l'utilisation

# Audio (Listening)

- Synthèse vocale du navigateur (Web Speech API), avec sélection de voix en-US, en-GB, en-AU
  quand elles sont disponibles
- Audio derrière une interface interchangeable, pour pouvoir brancher plus tard
  un moteur TTS local (ex. Piper) qui pré-génère des fichiers audio
- Transcription affichable après réponse

# Fonctionnalités

- Mode entraînement par partie : correction immédiate + explication (choix FR/EN)
- Examens blancs : complet (200 questions, 2h, chronométré, correction à la fin)
  et mini-tests paramétrables
- Tableau de bord : score estimé, évolution, score par partie et par catégorie,
  écart avec l'objectif 800, progression dans les cours
- Révision des erreurs : liste des questions ratées, rejouables, priorisées par catégorie faible
- Flashcards de vocabulaire avec répétition espacée (algorithme SM-2), alimentées par le
  vocabulaire des questions, des leçons et des ajouts manuels
- Planning : l'utilisateur saisit sa date d'examen et son temps disponible par jour,
  l'appli génère un programme (leçons + exercices) adapté à ses points faibles
  et le réajuste selon ses résultats

# Zone de cours

- Trois types de leçons :
  - Grammaire : une leçon par catégorie grammaticale
    (temps verbaux, prépositions, pronoms relatifs, comparatifs, voix passive, etc.)
  - Vocabulaire par thème TOEIC : bureau, réunions, voyages, finance, RH, achats, santé…
  - Méthodologie : une leçon par partie (1 à 7) avec les stratégies, pièges fréquents
    et gestion du temps
- Chaque leçon contient : explication (choix FR/EN), exemples, tableau récapitulatif si utile,
  et un mini-quiz de 5 à 10 questions à la fin
- Leçons rédigées par toi en Markdown (avec métadonnées : niveau, catégorie, durée estimée),
  importées en base comme la banque de questions
- Suivi par utilisateur : leçon lue / quiz réussi, avec pourcentage de progression par module
- Liens entre modules :
  - depuis une correction ratée → lien vers la leçon de la catégorie concernée
  - le tableau de bord et le planning recommandent les leçons liées aux catégories faibles
  - le vocabulaire des leçons peut être ajouté aux flashcards en un clic

# Phases

1. MVP : comptes par pseudo, import JSON, mode entraînement Reading (Parts 5-7), un premier lot
   de questions pour tester
2. Listening : audio, Parts 1-4, transcriptions
3. Examens blancs complets et mini-tests, scoring
4. Tableau de bord et révision des erreurs
5. Zone de cours : grammaire et méthodologie d'abord, puis vocabulaire par thème
6. Flashcards SM-2 et planning
7. Complétion de la banque à 400 questions

# Méthode

## Étape 0 — Prérequis (seul moment où tu attends ma réponse)

Avant d'écrire du code, fais-moi en une seule fois la liste complète de ce dont tu as besoin :

- Outils et dépendances système à installer (avec la commande exacte pour Arch Linux, pacman
  ou AUR, et la version minimale requise), par ex. toolchain Rust, Node.js, Docker / Docker
  Compose, sqlx-cli
- Accès ou permissions nécessaires (dossiers, droits Docker, ports utilisés, etc.)
- Comptes, clés ou services externes éventuels, en précisant s'ils sont indispensables
  ou optionnels
- Ce que tu vas installer toi-même dans le projet (crates, paquets npm) sans action de ma part
  Vérifie ce qui est déjà présent sur ma machine avant de me demander de l'installer.
  Anticipe tous les besoins de toutes les phases, pour ne plus avoir à m'interrompre ensuite.
  N'installe rien au niveau système et ne modifie rien hors du dossier du projet sans mon accord.

## Ensuite — Exécution autonome

Une fois les prérequis confirmés, enchaîne toutes les phases sans attendre de validation :

- Commence par documenter l'architecture (arborescence, schéma de base de données, endpoints,
  format JSON d'une question, format Markdown d'une leçon, liste des catégories)
  dans docs/ARCHITECTURE.md, puis respecte-la
- Initialise un dépôt git et fais au moins un commit par phase, avec un message clair
- Ne passe à la phase suivante que si le projet compile et que les tests passent
- Tiens à jour PROGRESS.md (le modèle existe déjà à la racine) : phases terminées, choix
  techniques faits et pourquoi, problèmes rencontrés, points à vérifier par moi
- Si les commandes ou la structure du projet changent, mets à jour CLAUDE.md en conséquence
- Face à un choix non précisé dans ce prompt, prends la décision la plus raisonnable,
  note-la dans PROGRESS.md et continue
- Ne t'arrête que si tu es réellement bloqué (accès ou installation système imprévu,
  erreur que tu ne peux pas résoudre) : dans ce cas, explique précisément ce qu'il te faut
- À la fin, fais-moi un résumé global : ce qui fonctionne, comment lancer l'appli,
  et ce qui reste à améliorer
