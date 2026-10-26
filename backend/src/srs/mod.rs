//! Répétition espacée : algorithme SM-2 (SuperMemo 2), logique pure sans base de données.
//!
//! Qualité de réponse (0-5) : 0-2 = oublié, 3 = difficile, 4 = bien, 5 = facile.
//! - Qualité < 3 : on repart de zéro (répétitions = 0, intervalle = 1 jour) SANS toucher au facteur
//!   de facilité (comportement du SM-2 d'origine).
//! - Sinon : intervalles 1 jour, puis 6 jours, puis `intervalle × facilité`.
//! - Facilité : `EF' = max(1.3, EF + 0.1 - (5-q) × (0.08 + (5-q) × 0.02))`.
use chrono::{Duration, NaiveDate};

pub const MIN_EASE: f64 = 1.3;
pub const DEFAULT_EASE: f64 = 2.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardState {
    pub ease: f64,
    pub interval_days: i64,
    pub repetitions: i64,
}

impl Default for CardState {
    fn default() -> Self {
        CardState {
            ease: DEFAULT_EASE,
            interval_days: 0,
            repetitions: 0,
        }
    }
}

/// Applique une révision de qualité `quality` (0-5) et retourne le nouvel état.
pub fn review(state: CardState, quality: u8) -> CardState {
    let q = quality.min(5);
    if q < 3 {
        return CardState {
            ease: state.ease,
            interval_days: 1,
            repetitions: 0,
        };
    }
    let repetitions = state.repetitions + 1;
    let interval_days = match repetitions {
        1 => 1,
        2 => 6,
        _ => ((state.interval_days.max(1) as f64) * state.ease).round() as i64,
    };
    let d = (5 - q) as f64;
    let ease = (state.ease + 0.1 - d * (0.08 + d * 0.02)).max(MIN_EASE);
    CardState {
        ease,
        interval_days,
        repetitions,
    }
}

/// Date de prochaine révision.
pub fn due_date(today: NaiveDate, state: &CardState) -> NaiveDate {
    today + Duration::days(state.interval_days.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(qs: &[u8]) -> Vec<CardState> {
        let mut s = CardState::default();
        qs.iter()
            .map(|q| {
                s = review(s, *q);
                s
            })
            .collect()
    }

    #[test]
    fn intervalles_1_6_puis_facilite() {
        let r = run(&[4, 4, 4]);
        assert_eq!(r[0].interval_days, 1);
        assert_eq!(r[1].interval_days, 6);
        // 6 × 2.5 = 15 (la facilité reste à 2.5 avec qualité 4)
        assert_eq!(r[2].interval_days, 15);
        assert_eq!(r[2].repetitions, 3);
    }

    #[test]
    fn qualite_5_augmente_la_facilite_et_4_la_conserve() {
        assert!((review(CardState::default(), 5).ease - 2.6).abs() < 1e-9);
        assert!((review(CardState::default(), 4).ease - 2.5).abs() < 1e-9);
        assert!((review(CardState::default(), 3).ease - 2.36).abs() < 1e-9);
    }

    #[test]
    fn echec_remet_a_zero_sans_changer_la_facilite() {
        let s = run(&[5, 5, 5]).pop().unwrap();
        let f = review(s, 1);
        assert_eq!(f.repetitions, 0);
        assert_eq!(f.interval_days, 1);
        assert_eq!(f.ease, s.ease);
        // Après un échec, on repasse par 1 puis 6 jours.
        assert_eq!(review(f, 4).interval_days, 1);
    }

    #[test]
    fn facilite_jamais_sous_le_plancher() {
        let mut s = CardState::default();
        for _ in 0..30 {
            s = review(s, 3);
        }
        assert!(s.ease >= MIN_EASE - 1e-9);
        assert!((s.ease - MIN_EASE).abs() < 1e-9);
    }

    #[test]
    fn qualite_hors_bornes_plafonnee() {
        assert_eq!(
            review(CardState::default(), 9),
            review(CardState::default(), 5)
        );
    }

    #[test]
    fn date_de_revision() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let s = CardState {
            ease: 2.5,
            interval_days: 6,
            repetitions: 2,
        };
        assert_eq!(
            due_date(today, &s),
            NaiveDate::from_ymd_opt(2026, 3, 7).unwrap()
        );
        // Jamais le jour même, même pour un intervalle nul.
        assert_eq!(
            due_date(today, &CardState::default()),
            NaiveDate::from_ymd_opt(2026, 3, 2).unwrap()
        );
    }
}
