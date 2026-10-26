//! Statistiques, tableau de bord et révision des erreurs.
//! Toutes les statistiques se basent sur la DERNIÈRE tentative de chaque question,
//! pour refléter le niveau actuel plutôt que l'historique complet.
use super::questions::{fetch_by_ids, QuestionOut};
use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::scoring;
use crate::AppState;
use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

/// Jointure « dernière tentative par question » réutilisée par toutes les requêtes.
const LATEST: &str = "FROM attempts a
    JOIN questions q ON q.id = a.question_id
    WHERE a.user_id = $1 AND a.answered_at = (
        SELECT MAX(a2.answered_at) FROM attempts a2
        WHERE a2.user_id = a.user_id AND a2.question_id = a.question_id)";

#[derive(Serialize)]
pub struct PartStat {
    part: i64,
    correct: i64,
    total: i64,
    accuracy: Option<f64>,
}

fn accuracy(correct: i64, total: i64) -> Option<f64> {
    (total > 0).then(|| correct as f64 / total as f64)
}

async fn part_stats(state: &AppState, user_id: &str) -> ApiResult<Vec<PartStat>> {
    let sql = format!(
        "SELECT q.part, SUM(CASE WHEN a.is_correct THEN 1 ELSE 0 END), COUNT(*) {LATEST}
         GROUP BY q.part ORDER BY q.part"
    );
    let rows: Vec<(i64, i64, i64)> = sqlx::query_as(&sql)
        .bind(user_id)
        .fetch_all(&state.pool)
        .await?;
    Ok(rows
        .into_iter()
        .map(|(part, correct, total)| PartStat {
            part,
            correct,
            total,
            accuracy: accuracy(correct, total),
        })
        .collect())
}

pub async fn by_part(
    State(state): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<Vec<PartStat>>> {
    Ok(Json(part_stats(&state, &user.id).await?))
}

#[derive(Serialize)]
pub struct CategoryStat {
    pub(crate) category: String,
    kind: String,
    label_fr: String,
    label_en: String,
    correct: i64,
    total: i64,
    pub(crate) accuracy: Option<f64>,
    /// Leçon recommandée pour cette catégorie (si elle existe).
    lesson_id: Option<String>,
}

#[derive(sqlx::FromRow)]
struct CatRow {
    id: String,
    kind: String,
    label_fr: String,
    label_en: String,
    correct: i64,
    total: i64,
    lesson_id: Option<String>,
}

/// Catégories travaillées, de la plus faible à la plus forte.
pub(crate) async fn category_stats(
    state: &AppState,
    user_id: &str,
) -> ApiResult<Vec<CategoryStat>> {
    let sql = format!(
        "SELECT c.id, c.kind, c.label_fr, c.label_en,
                SUM(CASE WHEN a.is_correct THEN 1 ELSE 0 END) AS correct, COUNT(*) AS total,
                (SELECT l.id FROM lessons l WHERE l.category_id = c.id ORDER BY l.id LIMIT 1) AS lesson_id
         {LATEST}
         GROUP BY c.id, c.kind, c.label_fr, c.label_en"
    )
    .replace(
        "JOIN questions q ON q.id = a.question_id",
        "JOIN questions q ON q.id = a.question_id JOIN categories c ON c.id = q.category_id",
    );
    let mut rows: Vec<CategoryStat> = sqlx::query_as::<_, CatRow>(&sql)
        .bind(user_id)
        .fetch_all(&state.pool)
        .await?
        .into_iter()
        .map(|r| CategoryStat {
            accuracy: accuracy(r.correct, r.total),
            category: r.id,
            kind: r.kind,
            label_fr: r.label_fr,
            label_en: r.label_en,
            correct: r.correct,
            total: r.total,
            lesson_id: r.lesson_id,
        })
        .collect();
    rows.sort_by(|a, b| {
        a.accuracy
            .partial_cmp(&b.accuracy)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.total.cmp(&a.total))
    });
    Ok(rows)
}

pub async fn by_category(
    State(state): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<Vec<CategoryStat>>> {
    Ok(Json(category_stats(&state, &user.id).await?))
}

#[derive(Serialize)]
pub struct Estimated {
    listening: Option<u32>,
    reading: Option<u32>,
    total: Option<u32>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ExamPoint {
    finished_at: String,
    kind: String,
    listening_scaled: Option<i64>,
    reading_scaled: Option<i64>,
}

#[derive(Serialize)]
pub struct Overview {
    goal_score: i64,
    estimated: Estimated,
    /// Points restants jusqu'à l'objectif (négatif ou nul = atteint).
    gap: Option<i64>,
    questions_answered: i64,
    accuracy: Option<f64>,
    /// Parties sur 7 déjà travaillées : plus il y en a, plus l'estimation est fiable.
    parts_covered: usize,
    exams: Vec<ExamPoint>,
    weak_categories: Vec<CategoryStat>,
}

pub async fn overview(State(state): State<AppState>, user: AuthUser) -> ApiResult<Json<Overview>> {
    let (goal,): (i64,) = sqlx::query_as("SELECT goal_score FROM users WHERE id = $1")
        .bind(&user.id)
        .fetch_one(&state.pool)
        .await?;
    let parts = part_stats(&state, &user.id).await?;
    let input: Vec<(i64, u32, u32)> = parts
        .iter()
        .map(|p| (p.part, p.correct as u32, p.total as u32))
        .collect();
    let (l, r, total) = scoring::estimate_from_parts(&input);
    let answered: i64 = parts.iter().map(|p| p.total).sum();
    let correct: i64 = parts.iter().map(|p| p.correct).sum();
    let exams = sqlx::query_as::<_, ExamPoint>(
        "SELECT finished_at, kind, listening_scaled, reading_scaled FROM exams
         WHERE user_id = $1 AND finished_at IS NOT NULL ORDER BY finished_at LIMIT 50",
    )
    .bind(&user.id)
    .fetch_all(&state.pool)
    .await?;
    let weak = category_stats(&state, &user.id)
        .await?
        .into_iter()
        .filter(|c| c.total >= 2 && c.accuracy.is_some_and(|a| a < 0.8))
        .take(5)
        .collect();
    Ok(Json(Overview {
        goal_score: goal,
        estimated: Estimated {
            listening: l,
            reading: r,
            total,
        },
        gap: total.map(|t| goal - t as i64),
        questions_answered: answered,
        accuracy: accuracy(correct, answered),
        parts_covered: parts.len(),
        exams,
        weak_categories: weak,
    }))
}

#[derive(Deserialize)]
pub struct MistakesQuery {
    part: Option<i64>,
    category: Option<String>,
    limit: Option<usize>,
}

#[derive(Serialize)]
pub struct Mistakes {
    /// Nombre total d'erreurs en attente de révision (avant `limit`).
    total: usize,
    questions: Vec<QuestionOut>,
}

/// Questions dont la dernière tentative est fausse, les catégories les plus faibles d'abord.
/// Une question réussie à la révision sort de la liste (elle n'est plus « ratée »).
pub async fn mistakes(
    State(state): State<AppState>,
    user: AuthUser,
    Query(q): Query<MistakesQuery>,
) -> ApiResult<Json<Mistakes>> {
    let sql = format!(
        "SELECT q.id, q.category_id, q.part, q.passage_id {LATEST} AND a.is_correct = FALSE"
    );
    let rows: Vec<(String, String, i64, Option<String>)> = sqlx::query_as(&sql)
        .bind(&user.id)
        .fetch_all(&state.pool)
        .await?;
    let weakness: std::collections::HashMap<String, f64> = category_stats(&state, &user.id)
        .await?
        .into_iter()
        .map(|c| (c.category, c.accuracy.unwrap_or(0.0)))
        .collect();
    let mut rows: Vec<_> = rows
        .into_iter()
        .filter(|(_, cat, part, _)| {
            q.part.is_none_or(|p| p == *part) && q.category.as_ref().is_none_or(|c| c == cat)
        })
        .collect();
    // Catégorie la plus faible d'abord, puis passage (garde les groupes ensemble), puis id.
    rows.sort_by(|a, b| {
        let wa = weakness.get(&a.1).copied().unwrap_or(1.0);
        let wb = weakness.get(&b.1).copied().unwrap_or(1.0);
        wa.partial_cmp(&wb)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.3.cmp(&b.3))
            .then(a.0.cmp(&b.0))
    });
    let total = rows.len();
    let ids: Vec<String> = rows
        .into_iter()
        .take(q.limit.unwrap_or(20).clamp(1, 200))
        .map(|r| r.0)
        .collect();
    Ok(Json(Mistakes {
        total,
        questions: fetch_by_ids(&state, &ids).await?,
    }))
}

/// Nombre d'erreurs en attente de révision (dernière tentative fausse).
pub(crate) async fn mistake_count(state: &AppState, user_id: &str) -> ApiResult<usize> {
    let sql = format!("SELECT COUNT(*) {LATEST} AND a.is_correct = FALSE");
    let (n,): (i64,) = sqlx::query_as(&sql)
        .bind(user_id)
        .fetch_one(&state.pool)
        .await?;
    Ok(n as usize)
}
