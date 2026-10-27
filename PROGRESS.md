# Suivi d'avancement

## État actuel

- Phase en cours : terminée (7/7). Reste à votre charge : photos Part 1, relecture du contenu
- Dernière mise à jour : 2026-10-26
- Prochaine action : déposer les 12 photos Part 1 (content/images/part1/), relire le contenu, tester l'audio dans Chrome/Edge

## Phases

- [x] 0. Prérequis confirmés
- [x] Architecture documentée (docs/ARCHITECTURE.md)
- [x] 1. MVP : pseudo, import JSON, entraînement Reading (Parts 5-7)
- [x] 2. Listening : audio, Parts 1-4, transcriptions
- [x] 3. Examens blancs et mini-tests, scoring
- [x] 4. Tableau de bord et révision des erreurs
- [x] 5. Zone de cours
- [x] 6. Flashcards SM-2 et planning
- [x] 7. Banque complète (400 questions)

## Banque de contenu

| Partie | Questions prévues | Questions rédigées | Format validé |
| ------ | ----------------- | ------------------ | ------------- |
| Part 1 | 12                | 12                  | oui           |
| Part 2 | 50                | 50                  | oui           |
| Part 3 | 78                | 78                  | oui           |
| Part 4 | 60                | 60                  | oui           |
| Part 5 | 60 (+60 extension)| 120                | oui           |
| Part 6 | 32                | 32                  | oui           |
| Part 7 | 108 (+54 extension)| 162                 | oui           |

Leçons : grammaire 18 · vocabulaire 17 · méthodologie 7 = 42 leçons ; tous les quiz ont 8 questions (336 au total). Chaque cours d'origine a une section de complément (pièges de francophones, mots complémentaires ou plan d'entraînement).

## Journal des décisions

<!-- Format : date — décision — raison -->
2026-10-26 — Nom de l'appli « Cap 800 » — original, rappelle l'objectif
2026-10-26 — Migrations embarquées (sqlx::migrate!), requêtes runtime sans macros query! — pas de sqlx-cli ni de base de dev nécessaires
2026-10-26 — Le transcript d'un passage Listening est envoyé au client avec la question (nécessaire à la synthèse vocale) ; masqué dans l'interface jusqu'à la réponse. Acceptable pour un usage entre amis ; un moteur audio pré-généré pourra supprimer cette fuite
2026-10-26 — Audio : transcription au format « M:/W:/N: texte » (une ligne par réplique), voix M/W choisies par heuristique de nom parmi les voix de l'accent — la Web Speech API n'expose pas le genre
2026-10-26 — Parts 1 et 2 : les choix ne sont affichés qu'après la réponse (lettres seules avant), comme au vrai TOEIC
2026-10-26 — Les bonnes réponses sont réparties par mélange déterministe (script de génération hors dépôt) : sinon biais vers « A »
2026-10-26 — Part 1 : une question dont la photo n'est pas dans content/images/part1/ n'est pas importée (warning) au lieu de bloquer le démarrage
2026-10-26 — Score d'examen : pourcentage de réussite par section ramené sur 100 puis converti (permet de scorer un examen partiel tant que la banque est incomplète et les mini-tests) ; question sans réponse = fausse
2026-10-26 — Examen complet : sélection par unités (passage entier) en privilégiant les questions jamais vues en examen ; servi même si la banque est incomplète (jusqu'au quota officiel par partie)
2026-10-26 — Examen chronométré : fin automatique à 0, réponses refusées après 15 s de tolérance ; audio en une seule écoute (bouton désactivé ensuite)
2026-10-26 — Statistiques basées sur la DERNIÈRE tentative de chaque question (niveau actuel, pas l'historique) ; une erreur rejouée juste sort de la liste de révision
2026-10-26 — Score estimé du tableau de bord : moyenne des précisions par partie pondérée par le poids officiel ; l'estimation est signalée comme moins fiable tant que les 7 parties ne sont pas travaillées
2026-10-26 — Leçons : 5 questions de quiz par leçon (minimum du PROMPT) ; validation à 70 % ; les quiz n'enregistrent pas de tentatives (ne faussent ni statistiques ni score estimé)
2026-10-26 — Lien « correction ratée → leçon » : leçon grammaire/vocabulaire de la catégorie, sinon leçon de méthodologie de la partie (method-part-N)
2026-10-26 — Planning : jours calculés en UTC (peut décaler d'un jour autour de minuit en France) ; réajustement automatique à la première consultation de chaque jour, les éléments déjà cochés ou passés sont conservés
2026-10-26 — Planning : examen blanc hebdomadaire (J-7, J-14…), complet si ≥ 120 min/jour sinon mini-test ; veille et jour de l'examen = flashcards + révision seulement
2026-10-26 — Banque : 400 questions, répartition officielle exacte ; Part 7 = 58 questions sur passage simple + 50 sur passages multiples (4 doubles + 6 triples) ; 8 conversations Part 3 et 5 exposés Part 4 comportent un document associé (catégorie qt-visual-info)
2026-10-26 — Sélection d'un examen complet : 64 tirages aléatoires pour tomber pile sur le quota de chaque partie (les passages ne sont jamais coupés)
2026-10-26 — Les générateurs de contenu (scripts Python) ne sont pas versionnés : les fichiers JSON/Markdown de content/ sont la source de vérité
2026-10-26 — Questions des quiz de leçon (batch « lesson-* ») exclues de l'entraînement général
2026-10-26 — Poste de dev Windows 11 (et non Arch comme dans PROMPT.md) — décision de l utilisateur ; commandes Docker/Node identiques

2026-10-27 — Extension du contenu à la demande de l'utilisateur : cours de grammaire enrichis (pièges de francophones, exemples, quiz 8 questions), 5 nouveaux cours, lot part5-batch03 (60 questions, dont 28 de vocabulaire). Les catégories ne changent pas ; plusieurs leçons peuvent partager une catégorie (le lien « leçon liée » prend la première par id)
2026-10-27 — Extension terminée : quiz de vocabulaire et de méthodologie à 8 questions (+ « mots complémentaires » / « plan d'entraînement »), lot part7-batch05 (54 questions : 10 passages simples, 2 doubles, 3 triples). Banque d'examen : 514 questions

## Problèmes rencontrés

<!-- Format : date — problème — solution ou statut -->

## Points à vérifier par l'utilisateur

- Contenu des 37 leçons et 185 questions de quiz rédigé en une passe : une relecture par vous (ou un enseignant) reste souhaitable, surtout les distracteurs des quiz de vocabulaire et les tableaux de grammaire.

- **Photos Part 1** : déposer p1-01.jpg … p1-12.jpg dans content/images/part1/ et compléter credits.json (voir le README du dossier). Sans elles, les 12 questions Part 1 n'apparaissent pas.
- Tester l'audio dans Chrome ou Edge : le rendu des voix dépend de celles installées sur Windows (Paramètres > Heure et langue > Voix).

- Part 7 : le format du lot est validé, mais les passages doubles/triples sont à tester en phase 7.
- La table de conversion du score (docs/ARCHITECTURE.md §8) est une approximation de ma part : à comparer avec vos scores réels.

## Travail en cours (en cas d'interruption)

<!-- Ce qui était en train d'être fait, pour reprendre proprement -->

