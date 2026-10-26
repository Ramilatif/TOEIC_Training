//! Zone de cours : liste, lecture, quiz et progression par module.
use super::questions::{fetch_by_ids, QuestionOut};
use crate::auth::AuthUser;
use crate::content::model::Explanation;
use crate::error::{ApiError, ApiResult};
use crate::AppState;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pourcentage de bonnes réponses au quiz pour valider une leçon.
pub const PASS_PERCENT: i64 = 70;

#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(rename = "type")]
    kind: Option<String>,
    category: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct LessonItem {
    id: String,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    kind: String,
    category_id: String,
    level: String,
    minutes: i64,
    title_fr: String,
    title_en: String,
    read: bool,
    quiz_passed: bool,
    best_quiz_score: Option<i64>,
}

const LIST_SQL: &str =
    "SELECT l.id, l.type, l.category_id, l.level, l.minutes, l.title_fr, l.title_en,
        CASE WHEN p.read_at IS NULL THEN FALSE ELSE TRUE END AS read,
        CASE WHEN p.quiz_passed_at IS NULL THEN FALSE ELSE TRUE END AS quiz_passed,
        p.best_quiz_score
    FROM lessons l
    LEFT JOIN lesson_progress p ON p.lesson_id = l.id AND p.user_id = $1";

pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Vec<LessonItem>>> {
    let mut rows = sqlx::query_as::<_, LessonItem>(&format!("{LIST_SQL} ORDER BY l.type, l.id"))
        .bind(&user.id)
        .fetch_all(&state.pool)
        .await?;
    rows.retain(|r| {
        q.kind.as_ref().is_none_or(|k| *k == r.kind)
            && q.category.as_ref().is_none_or(|c| *c == r.category_id)
    });
    Ok(Json(rows))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct VocabItem {
    term: String,
    translation_fr: String,
    example: Option<String>,
}

#[derive(Serialize)]
pub struct LessonDetail {
    #[serde(flatten)]
    item: LessonItem,
    body_fr_md: String,
    body_en_md: String,
    vocab: Vec<VocabItem>,
    quiz: Vec<QuestionOut>,
}

pub async fn get(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<LessonDetail>> {
    let item = sqlx::query_as::<_, LessonItem>(&format!("{LIST_SQL} WHERE l.id = $2"))
        .bind(&user.id)
        .bind(&id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| ApiError::NotFound("leçon introuvable".into()))?;
    let (body_fr_md, body_en_md): (String, String) =
        sqlx::query_as("SELECT body_fr_md, body_en_md FROM lessons WHERE id = $1")
            .bind(&id)
            .fetch_one(&state.pool)
            .await?;
    let vocab = sqlx::query_as::<_, VocabItem>(
        "SELECT term, translation_fr, example FROM lesson_vocab WHERE lesson_id = $1 ORDER BY term",
    )
    .bind(&id)
    .fetch_all(&state.pool)
    .await?;
    let qids: Vec<(String,)> = sqlx::query_as(
        "SELECT question_id FROM lesson_quiz_questions WHERE lesson_id = $1 ORDER BY question_id",
    )
    .bind(&id)
    .fetch_all(&state.pool)
    .await?;
    let qids: Vec<String> = qids.into_iter().map(|q| q.0).collect();
    Ok(Json(LessonDetail {
        item,
        body_fr_md,
        body_en_md,
        vocab,
        quiz: fetch_by_ids(&state, &qids).await?,
    }))
}

async fn ensure_lesson(state: &AppState, id: &str) -> ApiResult<()> {
    let found: Option<(String,)> = sqlx::query_as("SELECT id FROM lessons WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?;
    found
        .map(|_| ())
        .ok_or_else(|| ApiError::NotFound("leçon introuvable".into()))
}

pub async fn mark_read(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_lesson(&state, &id).await?;
    sqlx::query(
        "INSERT INTO lesson_progress (user_id, lesson_id, read_at) VALUES ($1,$2,$3)
         ON CONFLICT(user_id, lesson_id) DO UPDATE SET read_at = COALESCE(lesson_progress.read_at, excluded.read_at)",
    )
    .bind(&user.id)
    .bind(&id)
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(&state.pool)
    .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct QuizSubmit {
    /// question_id -> index choisi
    answers: HashMap<String, usize>,
}

#[derive(Serialize)]
pub struct QuizItem {
    question_id: String,
    chosen_index: Option<usize>,
    correct_index: i64,
    correct: bool,
    explanation_fr: Explanation,
    explanation_en: Explanation,
}

#[derive(Serialize)]
pub struct QuizResult {
    score: i64,
    total: i64,
    percent: i64,
    passed: bool,
    items: Vec<QuizItem>,
}

/// Pourcentage entier (arrondi au plus proche) ; 0 si total nul.
pub fn percent(score: i64, total: i64) -> i64 {
    if total == 0 {
        0
    } else {
        (score * 100 + total / 2) / total
    }
}

pub async fn submit_quiz(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<QuizSubmit>,
) -> ApiResult<Json<QuizResult>> {
    ensure_lesson(&state, &id).await?;
    let rows: Vec<(String, i64, String, String)> = sqlx::query_as(
        "SELECT q.id, q.correct_index, q.explanation_fr, q.explanation_en
         FROM lesson_quiz_questions z JOIN questions q ON q.id = z.question_id
         WHERE z.lesson_id = $1 ORDER BY q.id",
    )
    .bind(&id)
    .fetch_all(&state.pool)
    .await?;
    let total = rows.len() as i64;
    let mut score = 0;
    let mut items = Vec::new();
    for (qid, correct_index, efr, een) in rows {
        let chosen = req.answers.get(&qid).copied();
        let ok = chosen.is_some_and(|c| c as i64 == correct_index);
        score += ok as i64;
        let parse = |s: &str| {
            serde_json::from_str::<Explanation>(s).map_err(|e| ApiError::Internal(e.into()))
        };
        items.push(QuizItem {
            question_id: qid,
            chosen_index: chosen,
            correct_index,
            correct: ok,
            explanation_fr: parse(&efr)?,
            explanation_en: parse(&een)?,
        });
    }
    let pct = percent(score, total);
    let passed = pct >= PASS_PERCENT;
    let now = chrono::Utc::now().to_rfc3339();
    // Réussir le quiz implique d'avoir lu la leçon.
    sqlx::query(
        "INSERT INTO lesson_progress (user_id, lesson_id, read_at, quiz_passed_at, best_quiz_score)
         VALUES ($1,$2,$3,$4,$5)
         ON CONFLICT(user_id, lesson_id) DO UPDATE SET
           read_at = COALESCE(lesson_progress.read_at, excluded.read_at),
           quiz_passed_at = COALESCE(lesson_progress.quiz_passed_at, excluded.quiz_passed_at),
           best_quiz_score = CASE WHEN lesson_progress.best_quiz_score IS NULL
             OR lesson_progress.best_quiz_score < excluded.best_quiz_score
             THEN excluded.best_quiz_score ELSE lesson_progress.best_quiz_score END",
    )
    .bind(&user.id)
    .bind(&id)
    .bind(&now)
    .bind(passed.then_some(now.clone()))
    .bind(pct)
    .execute(&state.pool)
    .await?;
    Ok(Json(QuizResult {
        score,
        total,
        percent: pct,
        passed,
        items,
    }))
}

#[derive(Serialize)]
pub struct ModuleProgress {
    #[serde(rename = "type")]
    kind: String,
    lessons: i64,
    read: i64,
    quiz_passed: i64,
    /// 50 % pour la lecture, 50 % pour le quiz réussi, moyenne sur le module.
    percent: i64,
}

pub async fn progress(
    State(state): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<Vec<ModuleProgress>>> {
    let rows: Vec<(String, i64, i64, i64)> = sqlx::query_as(
        "SELECT l.type, COUNT(*),
                SUM(CASE WHEN p.read_at IS NULL THEN 0 ELSE 1 END),
                SUM(CASE WHEN p.quiz_passed_at IS NULL THEN 0 ELSE 1 END)
         FROM lessons l LEFT JOIN lesson_progress p ON p.lesson_id = l.id AND p.user_id = $1
         GROUP BY l.type ORDER BY l.type",
    )
    .bind(&user.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|(kind, lessons, read, quiz_passed)| ModuleProgress {
                percent: percent(read + quiz_passed, 2 * lessons),
                kind,
                lessons,
                read,
                quiz_passed,
            })
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::percent;

    #[test]
    fn pourcentages() {
        assert_eq!(percent(0, 0), 0);
        assert_eq!(percent(7, 10), 70);
        assert_eq!(percent(2, 3), 67);
        assert_eq!(percent(5, 5), 100);
    }
}
