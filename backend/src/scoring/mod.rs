//! Scoring TOEIC approximatif et sélection des questions d'examen (logique pure, sans base).
//!
//! La table de conversion est une estimation : ETS ne publie pas de table fixe, elle varie selon
//! la difficulté de chaque session. On interpole linéairement entre des paliers raisonnables.
use rand::seq::SliceRandom;
use rand::Rng;

/// (score brut sur 100, score converti sur 5-495). Même table pour Listening et Reading.
pub const CONVERSION: [(u32, u32); 9] = [
    (0, 5),
    (15, 60),
    (30, 130),
    (45, 215),
    (60, 300),
    (75, 385),
    (85, 440),
    (95, 485),
    (100, 495),
];

/// Répartition officielle par examen complet : (partie, nombre de questions).
pub const OFFICIAL_COUNTS: [(i64, usize); 7] =
    [(1, 6), (2, 25), (3, 39), (4, 30), (5, 30), (6, 16), (7, 54)];

pub const FULL_EXAM_SECONDS: i64 = 7200;

pub fn is_listening(part: i64) -> bool {
    part <= 4
}

/// Convertit un score brut (0-100) en score de section (5-495, multiple de 5).
pub fn scaled_section(raw: u32) -> u32 {
    let raw = raw.min(100);
    for w in CONVERSION.windows(2) {
        let (r0, s0) = w[0];
        let (r1, s1) = w[1];
        if raw <= r1 {
            let frac = (raw - r0) as f64 / (r1 - r0) as f64;
            let v = s0 as f64 + frac * (s1 as f64 - s0 as f64);
            let rounded = ((v / 5.0).round() * 5.0) as u32;
            return rounded.clamp(5, 495);
        }
    }
    495
}

/// Estime le score de section quand l'examen compte moins de 100 questions dans la section :
/// le pourcentage de réussite est ramené sur 100. `None` si la section est vide.
pub fn estimate_scaled(correct: u32, total: u32) -> Option<u32> {
    if total == 0 {
        return None;
    }
    let raw = (correct as f64 / total as f64 * 100.0).round() as u32;
    Some(scaled_section(raw))
}

/// Score total (10-990) si les deux sections sont connues.
pub fn total_score(listening: Option<u32>, reading: Option<u32>) -> Option<u32> {
    Some(listening? + reading?)
}

/// Nombre de tirages tentés pour atteindre exactement la cible.
const PICK_ATTEMPTS: usize = 64;

/// Sélectionne des « unités » (un passage et ses questions, ou une question isolée) jusqu'à
/// atteindre `target` questions sans le dépasser, en préférant les unités non encore vues.
/// Plusieurs tirages sont tentés pour tomber pile sur la cible (les groupes ne sont jamais
/// coupés) ; sinon on garde le tirage le plus rempli. Retourne les ids de questions.
pub fn pick_units<R: Rng>(
    units: Vec<(bool, Vec<String>)>,
    target: usize,
    rng: &mut R,
) -> Vec<String> {
    let (fresh0, seen0): (Vec<_>, Vec<_>) = units.into_iter().partition(|(s, _)| !*s);
    let mut best: Vec<String> = Vec::new();
    for _ in 0..PICK_ATTEMPTS {
        let mut fresh = fresh0.clone();
        let mut seen = seen0.clone();
        fresh.shuffle(rng);
        seen.shuffle(rng);
        let mut out = Vec::new();
        for (_, ids) in fresh.into_iter().chain(seen) {
            if out.len() >= target {
                break;
            }
            if out.len() + ids.len() <= target {
                out.extend(ids);
            }
        }
        if out.len() > best.len() {
            best = out;
        }
        if best.len() == target {
            break;
        }
    }
    best
}

/// Score estimé à partir de la précision par partie : (partie, bonnes réponses, total tenté).
/// Chaque section est la moyenne des précisions des parties travaillées, pondérée par le poids
/// officiel de la partie, ramenée sur 100 puis convertie. Retourne (listening, reading, total).
pub fn estimate_from_parts(stats: &[(i64, u32, u32)]) -> (Option<u32>, Option<u32>, Option<u32>) {
    let section = |listening: bool| -> Option<u32> {
        let (mut num, mut den) = (0.0, 0.0);
        for (part, correct, total) in stats {
            if *total == 0 || is_listening(*part) != listening {
                continue;
            }
            let w = OFFICIAL_COUNTS
                .iter()
                .find(|(p, _)| p == part)
                .map_or(1.0, |(_, n)| *n as f64);
            num += w * (*correct as f64 / *total as f64);
            den += w;
        }
        (den > 0.0).then(|| scaled_section((num / den * 100.0).round() as u32))
    };
    let (l, r) = (section(true), section(false));
    (l, r, total_score(l, r))
}

/// Répartit `count` questions entre les parties demandées, proportionnellement à la répartition
/// officielle (au moins 1 par partie quand c'est possible).
pub fn distribute(parts: &[i64], count: usize) -> Vec<(i64, usize)> {
    let weight = |p: i64| {
        OFFICIAL_COUNTS
            .iter()
            .find(|(q, _)| *q == p)
            .map_or(1, |(_, n)| *n)
    };
    let total_w: usize = parts.iter().map(|p| weight(*p)).sum();
    if parts.is_empty() || total_w == 0 {
        return vec![];
    }
    let mut out: Vec<(i64, usize)> = parts
        .iter()
        .map(|p| {
            (
                *p,
                (count * weight(*p) / total_w).max(1.min(count / parts.len())),
            )
        })
        .collect();
    // Répartit le reste (ou retire l'excédent) sur les parties les plus lourdes.
    let mut order: Vec<usize> = (0..out.len()).collect();
    order.sort_by_key(|i| std::cmp::Reverse(weight(out[*i].0)));
    let mut sum: usize = out.iter().map(|(_, n)| n).sum();
    let mut k = 0;
    while sum < count {
        out[order[k % order.len()]].1 += 1;
        sum += 1;
        k += 1;
    }
    while sum > count {
        let i = order[k % order.len()];
        if out[i].1 > 0 {
            out[i].1 -= 1;
            sum -= 1;
        }
        k += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn bornes_de_la_table() {
        assert_eq!(scaled_section(0), 5);
        assert_eq!(scaled_section(100), 495);
        assert_eq!(scaled_section(60), 300);
        assert_eq!(scaled_section(150), 495); // plafonné
    }

    #[test]
    fn interpolation_et_arrondi_a_5() {
        // Entre 30 (130) et 45 (215) : 37 -> 130 + 7/15*85 = 169.67 -> 170
        assert_eq!(scaled_section(37), 170);
        for raw in 0..=100 {
            assert_eq!(scaled_section(raw) % 5, 0);
        }
    }

    #[test]
    fn croissance_monotone() {
        let mut prev = 0;
        for raw in 0..=100 {
            let s = scaled_section(raw);
            assert!(s >= prev, "{raw}: {s} < {prev}");
            prev = s;
        }
    }

    #[test]
    fn estimation_sur_examen_partiel() {
        assert_eq!(estimate_scaled(0, 0), None);
        assert_eq!(estimate_scaled(5, 10), Some(scaled_section(50)));
        assert_eq!(estimate_scaled(100, 100), Some(495));
    }

    #[test]
    fn total() {
        assert_eq!(total_score(Some(400), Some(350)), Some(750));
        assert_eq!(total_score(Some(400), None), None);
        assert_eq!(total_score(Some(5), Some(5)), Some(10));
        assert_eq!(total_score(Some(495), Some(495)), Some(990));
    }

    #[test]
    fn repartition_officielle_somme_200() {
        assert_eq!(OFFICIAL_COUNTS.iter().map(|(_, n)| n).sum::<usize>(), 200);
        assert_eq!(
            OFFICIAL_COUNTS
                .iter()
                .filter(|(p, _)| is_listening(*p))
                .map(|(_, n)| n)
                .sum::<usize>(),
            100
        );
    }

    #[test]
    fn estimation_par_parties() {
        // Aucune donnée.
        assert_eq!(estimate_from_parts(&[]), (None, None, None));
        // Reading seul : pas de total.
        let (l, r, tot) = estimate_from_parts(&[(5, 8, 10)]);
        assert_eq!((l, tot), (None, None));
        assert_eq!(r, Some(scaled_section(80)));
        // Pondération : la Part 2 (25) pèse plus que la Part 1 (6).
        let (l, _, _) = estimate_from_parts(&[(1, 0, 10), (2, 10, 10)]);
        assert!(l.unwrap() > scaled_section(50));
        // Les deux sections donnent un total.
        let (_, _, tot) = estimate_from_parts(&[(3, 5, 10), (7, 5, 10)]);
        assert_eq!(tot, Some(2 * scaled_section(50)));
    }

    #[test]
    fn distribution_somme_exacte() {
        for count in [5, 10, 20, 33, 100] {
            let d = distribute(&[1, 2, 3, 4, 5, 6, 7], count);
            assert_eq!(d.iter().map(|(_, n)| n).sum::<usize>(), count);
        }
        assert_eq!(distribute(&[5], 12), vec![(5, 12)]);
        assert!(distribute(&[], 10).is_empty());
        // Plus de poids pour la Part 7 que pour la Part 1.
        let d = distribute(&[1, 7], 30);
        assert!(d[1].1 > d[0].1);
    }

    #[test]
    fn pick_units_respecte_la_cible_et_les_groupes() {
        let mut rng = StdRng::seed_from_u64(1);
        let units = vec![
            (false, vec!["a1".into(), "a2".into(), "a3".into()]),
            (false, vec!["b1".into(), "b2".into(), "b3".into()]),
            (true, vec!["c1".into(), "c2".into(), "c3".into()]),
        ];
        let out = pick_units(units, 7, &mut rng);
        assert_eq!(out.len(), 6); // 7 serait dépassé par un 3e groupe
        assert!(
            !out.contains(&"c1".to_string()),
            "les unités déjà vues passent en dernier"
        );
        // Un groupe n'est jamais coupé.
        assert_eq!(out.iter().filter(|i| i.starts_with('a')).count() % 3, 0);
    }

    #[test]
    fn pick_units_complete_avec_les_unites_vues() {
        let mut rng = StdRng::seed_from_u64(2);
        let units = vec![(true, vec!["x".into()]), (false, vec!["y".into()])];
        assert_eq!(pick_units(units, 5, &mut rng).len(), 2);
    }
}
