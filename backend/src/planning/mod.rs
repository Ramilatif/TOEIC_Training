//! Génération du programme de révision (logique pure, sans base de données).
//!
//! Principes :
//! - chaque jour, un budget de `daily_minutes` est réparti entre flashcards, une leçon, la
//!   révision des erreurs et des exercices ciblés sur la catégorie la plus faible ;
//! - les leçons sont servies par priorité (catégories faibles d'abord, puis méthodologie,
//!   grammaire, vocabulaire), sans répétition ;
//! - un examen blanc par semaine (la veille du jour de l'examen : jour léger, sans nouveauté).
use chrono::{Datelike, Duration, NaiveDate};

/// Durée maximale planifiée (au-delà, seuls les 120 premiers jours sont générés).
pub const MAX_DAYS: i64 = 120;
/// Une séance de flashcards quotidienne.
pub const FLASHCARD_MINUTES: i64 = 5;
/// Durée d'une séance de révision des erreurs.
pub const REVIEW_MINUTES: i64 = 10;
/// Minimum pour qu'un bloc d'exercices ait un sens.
pub const MIN_PRACTICE_MINUTES: i64 = 5;
/// Budget minimal quotidien pour planifier un examen blanc complet.
pub const FULL_EXAM_MINUTES: i64 = 120;

#[derive(Debug, Clone, PartialEq)]
pub struct LessonRef {
    pub id: String,
    pub category: String,
    pub kind: String, // grammar | vocab | method
    pub minutes: i64,
}

#[derive(Debug, Clone)]
pub struct PlanInput {
    pub today: NaiveDate,
    pub exam_date: NaiveDate,
    pub daily_minutes: i64,
    /// Catégories travaillées, de la plus faible à la plus forte.
    pub weak_categories: Vec<String>,
    /// Leçons dont le quiz n'est pas encore réussi.
    pub lessons: Vec<LessonRef>,
    pub has_cards: bool,
    pub mistakes: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlannedItem {
    pub day: NaiveDate,
    /// lesson | practice | review | flashcards | mock
    pub kind: &'static str,
    pub ref_id: Option<String>,
    pub minutes: i64,
}

fn kind_rank(kind: &str) -> u8 {
    match kind {
        "method" => 1,
        "grammar" => 2,
        _ => 3,
    }
}

/// Ordonne les leçons : catégories faibles d'abord (dans l'ordre de faiblesse), puis par type.
pub fn prioritize(lessons: &[LessonRef], weak: &[String]) -> Vec<LessonRef> {
    let mut out: Vec<LessonRef> = lessons.to_vec();
    out.sort_by_key(|l| {
        let weak_pos = weak.iter().position(|c| *c == l.category);
        (
            weak_pos.map_or(usize::MAX, |p| p),
            kind_rank(&l.kind),
            l.id.clone(),
        )
    });
    out
}

/// Génère le programme de `today` à `exam_date` inclus.
pub fn generate(input: &PlanInput) -> Vec<PlannedItem> {
    let budget = input.daily_minutes.max(5);
    let last = input
        .exam_date
        .min(input.today + Duration::days(MAX_DAYS - 1));
    let queue = prioritize(&input.lessons, &input.weak_categories);
    let mut next_lesson = 0usize;
    let mut practice_turn = 0usize;
    let fallback_parts = ["part-5", "part-2", "part-7", "part-3", "part-6", "part-4"];
    let mut items = Vec::new();

    let mut day = input.today;
    while day <= last {
        let days_left = (input.exam_date - day).num_days();
        let mut left = budget;
        let mut push = |kind: &'static str, ref_id: Option<String>, minutes: i64| {
            items.push(PlannedItem {
                day,
                kind,
                ref_id,
                minutes,
            });
        };

        // Veille et jour de l'examen : séance légère, pas de nouveauté.
        if days_left <= 1 {
            if input.has_cards {
                push("flashcards", None, FLASHCARD_MINUTES.min(left));
                left -= FLASHCARD_MINUTES.min(left);
            }
            if input.mistakes > 0 && left >= MIN_PRACTICE_MINUTES {
                let m = REVIEW_MINUTES.min(left);
                push("review", None, m);
            }
            day += Duration::days(1);
            continue;
        }

        // Examen blanc hebdomadaire (toutes les 7 jours avant l'examen, à partir de J-7).
        if days_left % 7 == 0 && days_left > 0 && day != input.today {
            let full = budget >= FULL_EXAM_MINUTES;
            push(
                "mock",
                Some(if full { "full" } else { "mini" }.to_string()),
                budget.min(FULL_EXAM_MINUTES),
            );
            day += Duration::days(1);
            continue;
        }

        if input.has_cards && left >= 15 {
            push("flashcards", None, FLASHCARD_MINUTES);
            left -= FLASHCARD_MINUTES;
        }
        if input.mistakes > 0 && day.weekday().num_days_from_monday() % 3 == 2 && left >= 25 {
            push("review", None, REVIEW_MINUTES);
            left -= REVIEW_MINUTES;
        }
        if next_lesson < queue.len() && left >= MIN_PRACTICE_MINUTES {
            let l = &queue[next_lesson];
            // Une leçon plus longue que le temps restant occupe le reste de la séance.
            let m = l.minutes.min(left);
            push("lesson", Some(l.id.clone()), m);
            left -= m;
            next_lesson += 1;
        }
        if left >= MIN_PRACTICE_MINUTES {
            let target = if input.weak_categories.is_empty() {
                fallback_parts[practice_turn % fallback_parts.len()].to_string()
            } else {
                input.weak_categories[practice_turn % input.weak_categories.len().min(3)].clone()
            };
            practice_turn += 1;
            push("practice", Some(target), left);
        }
        day += Duration::days(1);
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn lessons() -> Vec<LessonRef> {
        let mk = |id: &str, cat: &str, kind: &str, minutes| LessonRef {
            id: id.into(),
            category: cat.into(),
            kind: kind.into(),
            minutes,
        };
        vec![
            mk("vocab-office", "voc-office", "vocab", 10),
            mk("grammar-passive-voice", "gram-passive-voice", "grammar", 12),
            mk("method-part-5", "gram-word-forms", "method", 12),
            mk("grammar-modals", "gram-modals", "grammar", 11),
        ]
    }

    fn input(days: i64, daily: i64) -> PlanInput {
        PlanInput {
            today: d(2026, 1, 5),
            exam_date: d(2026, 1, 5) + Duration::days(days),
            daily_minutes: daily,
            weak_categories: vec!["gram-modals".into(), "gram-passive-voice".into()],
            lessons: lessons(),
            has_cards: true,
            mistakes: 3,
        }
    }

    #[test]
    fn priorite_aux_categories_faibles() {
        let p = prioritize(
            &lessons(),
            &["gram-modals".into(), "gram-passive-voice".into()],
        );
        assert_eq!(p[0].id, "grammar-modals");
        assert_eq!(p[1].id, "grammar-passive-voice");
        // Ensuite : méthodologie avant vocabulaire.
        assert_eq!(p[2].id, "method-part-5");
        assert_eq!(p[3].id, "vocab-office");
    }

    #[test]
    fn jamais_plus_que_le_budget_quotidien() {
        let items = generate(&input(30, 45));
        for offset in 0..=30 {
            let day = d(2026, 1, 5) + Duration::days(offset);
            let total: i64 = items
                .iter()
                .filter(|i| i.day == day)
                .map(|i| i.minutes)
                .sum();
            assert!(total <= 45, "{day}: {total} min");
        }
    }

    #[test]
    fn chaque_lecon_une_seule_fois() {
        let items = generate(&input(30, 45));
        let ids: Vec<_> = items
            .iter()
            .filter(|i| i.kind == "lesson")
            .filter_map(|i| i.ref_id.clone())
            .collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len());
        assert_eq!(ids.len(), 4);
        assert_eq!(ids[0], "grammar-modals");
    }

    #[test]
    fn veille_d_examen_sans_nouveaute() {
        let p = input(10, 60);
        let items = generate(&p);
        let last = p.exam_date;
        for i in items.iter().filter(|i| i.day >= last - Duration::days(1)) {
            assert!(i.kind == "flashcards" || i.kind == "review", "{:?}", i);
        }
    }

    #[test]
    fn examen_blanc_hebdomadaire_et_taille_selon_le_budget() {
        let items = generate(&input(21, 150));
        let mocks: Vec<_> = items.iter().filter(|i| i.kind == "mock").collect();
        assert_eq!(mocks.len(), 2); // J-14 et J-7 (jamais aujourd'hui)
        assert!(mocks.iter().all(|m| m.ref_id.as_deref() == Some("full")));
        let items = generate(&input(21, 40));
        assert!(items
            .iter()
            .filter(|i| i.kind == "mock")
            .all(|m| m.ref_id.as_deref() == Some("mini") && m.minutes <= 40));
    }

    #[test]
    fn exercices_cibles_sur_les_categories_faibles() {
        let items = generate(&input(10, 60));
        let targets: Vec<_> = items
            .iter()
            .filter(|i| i.kind == "practice")
            .filter_map(|i| i.ref_id.clone())
            .collect();
        assert!(targets.contains(&"gram-modals".to_string()));
        assert!(targets.contains(&"gram-passive-voice".to_string()));
    }

    #[test]
    fn sans_donnees_on_alterne_les_parties() {
        let mut p = input(10, 60);
        p.weak_categories.clear();
        let targets: Vec<_> = generate(&p)
            .into_iter()
            .filter(|i| i.kind == "practice")
            .filter_map(|i| i.ref_id)
            .collect();
        assert!(targets.iter().all(|t| t.starts_with("part-")));
        assert!(targets.contains(&"part-5".to_string()));
    }

    #[test]
    fn examen_aujourd_hui_ou_dans_le_passe() {
        let mut p = input(0, 60);
        let items = generate(&p);
        assert!(items.iter().all(|i| i.day == p.today));
        p.exam_date = p.today - Duration::days(3);
        assert!(generate(&p).is_empty());
    }

    #[test]
    fn plafond_de_120_jours() {
        let items = generate(&input(400, 30));
        let max_day = items.iter().map(|i| i.day).max().unwrap();
        assert_eq!(max_day, d(2026, 1, 5) + Duration::days(MAX_DAYS - 1));
    }

    #[test]
    fn budget_minimal_respecte() {
        let items = generate(&input(5, 5));
        assert!(items.iter().all(|i| i.minutes <= 5 && i.minutes > 0));
    }
}
