//! Examens blancs (complet) et mini-tests : création, sauvegarde des réponses, correction.
use super::questions::{fetch_by_ids, QuestionOut};
use crate::auth::{new_id, AuthUser};
use crate::content::model::Explanation;
use crate::error::{ApiError, ApiResult};
use crate::scoring::{self, FULL_EXAM_SECONDS, OFFICIAL_COUNTS};
use crate::AppState;
use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Secondes accordées par question dans un mini-test chronométré.
const MINI_SECONDS_PER_QUESTION: i64 = 45;
/// Tolérance réseau sur la fin du chrono.
const GRACE_SECONDS: i64 = 15;

#[derive(Deserialize)]
pub struct CreateExam {
    /// "full" ou "mini"
    kind: String,
    parts: Option<Vec<i64>>,
    count: Option<usize>,
    timed: Option<bool>,
}

#[derive(Serialize)]
pub struct ExamOut {
    id: String,
    kind: String,
    started_at: String,
    finished_at: Option<String>,
    duration_limit_s: Option<i64>,
    remaining_s: Option<i64>,
    questions: Vec<QuestionOut>,
    /// question_id -> choix enregistré
    answers: HashMap<String, i64>,
}

#[derive(sqlx::FromRow)]
struct ExamRow {
    id: String,
    user_id: String,
    kind: String,
    started_at: String,
    finished_at: Option<String>,
    duration_limit_s: Option<i64>,
}

async fn load_exam(state: &AppState, user: &AuthUser, id: &str) -> ApiResult<ExamRow> {
    let row = sqlx::query_as::<_, ExamRow>(
        "SELECT id, user_id, kind, started_at, finished_at, duration_limit_s FROM exams WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .filter(|r| r.user_id == user.id)
    .ok_or_else(|| ApiError::NotFound("examen introuvable".into()))?;
    Ok(row)
}

fn remaining(row: &ExamRow) -> Option<i64> {
    let limit = row.duration_limit_s?;
    let start = chrono::DateTime::parse_from_rfc3339(&row.started_at).ok()?;
    let elapsed = (chrono::Utc::now() - start.with_timezone(&chrono::Utc)).num_seconds();
    Some((limit - elapsed).max(0))
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateExam>,
) -> ApiResult<Json<ExamOut>> {
    let (targets, timed): (Vec<(i64, usize)>, bool) = match req.kind.as_str() {
        "full" => (OFFICIAL_COUNTS.to_vec(), true),
        "mini" => {
            let parts = req.parts.unwrap_or_else(|| (1..=7).collect());
            if parts.is_empty() || parts.iter().any(|p| !(1..=7).contains(p)) {
                return Err(ApiError::BadRequest("parties : valeurs de 1 à 7".into()));
            }
            let count = req.count.unwrap_or(20);
            if !(1..=100).contains(&count) {
                return Err(ApiError::BadRequest("nombre de questions : 1 à 100".into()));
            }
            (
                scoring::distribute(&parts, count),
                req.timed.unwrap_or(false),
            )
        }
        _ => return Err(ApiError::BadRequest("kind : full ou mini".into())),
    };

    let mut ids: Vec<String> = Vec::new();
    for (part, target) in &targets {
        // Unités = un passage et ses questions, ou une question isolée.
        let rows: Vec<(String, Option<String>, i64)> = sqlx::query_as(
            "SELECT q.id, q.passage_id,
                    CASE WHEN EXISTS (SELECT 1 FROM attempts a WHERE a.user_id = $2
                         AND a.question_id = q.id AND a.exam_id IS NOT NULL) THEN 1 ELSE 0 END
             FROM questions q
             WHERE q.part = $1 AND q.batch NOT LIKE 'lesson-%'
             ORDER BY q.passage_id, q.position_in_passage, q.id",
        )
        .bind(part)
        .bind(&user.id)
        .fetch_all(&state.pool)
        .await?;
        let mut units: Vec<(bool, Vec<String>)> = Vec::new();
        let mut last_passage: Option<String> = None;
        for (qid, pid, seen) in rows {
            match (&pid, &last_passage) {
                (Some(p), Some(l)) if p == l => {
                    let u = units.last_mut().expect("unité en cours");
                    u.0 |= seen == 1;
                    u.1.push(qid);
                }
                _ => units.push((seen == 1, vec![qid])),
            }
            last_passage = pid;
        }
        ids.extend(scoring::pick_units(units, *target, &mut rand::thread_rng()));
    }
    if ids.is_empty() {
        return Err(ApiError::BadRequest(
            "aucune question disponible pour cette sélection".into(),
        ));
    }

    let duration = if !timed {
        None
    } else if req.kind == "full" {
        Some(FULL_EXAM_SECONDS)
    } else {
        Some(ids.len() as i64 * MINI_SECONDS_PER_QUESTION)
    };
    let exam_id = new_id();
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "INSERT INTO exams (id, user_id, kind, config_json, started_at, duration_limit_s)
         VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(&exam_id)
    .bind(&user.id)
    .bind(&req.kind)
    .bind(serde_json::json!({ "targets": targets, "timed": timed }).to_string())
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(duration)
    .execute(&mut *tx)
    .await?;
    for (pos, qid) in ids.iter().enumerate() {
        sqlx::query(
            "INSERT INTO exam_questions (exam_id, question_id, position) VALUES ($1,$2,$3)",
        )
        .bind(&exam_id)
        .bind(qid)
        .bind(pos as i64)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    get(State(state), user, Path(exam_id)).await
}

async fn exam_question_ids(state: &AppState, exam_id: &str) -> ApiResult<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT question_id FROM exam_questions WHERE exam_id = $1 ORDER BY position",
    )
    .bind(exam_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

pub async fn get(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<ExamOut>> {
    let row = load_exam(&state, &user, &id).await?;
    let ids = exam_question_ids(&state, &id).await?;
    let answers: Vec<(String, i64)> =
        sqlx::query_as("SELECT question_id, chosen_index FROM attempts WHERE exam_id = $1")
            .bind(&id)
            .fetch_all(&state.pool)
            .await?;
    Ok(Json(ExamOut {
        remaining_s: if row.finished_at.is_some() {
            Some(0)
        } else {
            remaining(&row)
        },
        id: row.id,
        kind: row.kind,
        started_at: row.started_at,
        finished_at: row.finished_at,
        duration_limit_s: row.duration_limit_s,
        questions: fetch_by_ids(&state, &ids).await?,
        answers: answers.into_iter().collect(),
    }))
}

#[derive(Deserialize)]
pub struct SaveAnswer {
    question_id: String,
    chosen_index: usize,
}

/// Enregistre (ou remplace) la réponse à une question, sans révéler la correction.
pub async fn save_answer(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<SaveAnswer>,
) -> ApiResult<Json<serde_json::Value>> {
    let row = load_exam(&state, &user, &id).await?;
    if row.finished_at.is_some() {
        return Err(ApiError::BadRequest("examen déjà terminé".into()));
    }
    if let Some(left) = remaining(&row) {
        if left == 0 && expired_beyond_grace(&row) {
            return Err(ApiError::BadRequest("temps écoulé".into()));
        }
    }
    let q: Option<(i64, String)> = sqlx::query_as(
        "SELECT q.correct_index, q.choices_json FROM exam_questions e
         JOIN questions q ON q.id = e.question_id
         WHERE e.exam_id = $1 AND e.question_id = $2",
    )
    .bind(&id)
    .bind(&req.question_id)
    .fetch_optional(&state.pool)
    .await?;
    let (correct_index, choices_json) =
        q.ok_or_else(|| ApiError::NotFound("question absente de cet examen".into()))?;
    let n = serde_json::from_str::<Vec<String>>(&choices_json)
        .map_err(|e| ApiError::Internal(e.into()))?
        .len();
    if req.chosen_index >= n {
        return Err(ApiError::BadRequest("choix hors bornes".into()));
    }
    let mut tx = state.pool.begin().await?;
    sqlx::query("DELETE FROM attempts WHERE exam_id = $1 AND question_id = $2")
        .bind(&id)
        .bind(&req.question_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO attempts (id, user_id, question_id, chosen_index, is_correct, mode, exam_id, answered_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(new_id())
    .bind(&user.id)
    .bind(&req.question_id)
    .bind(req.chosen_index as i64)
    .bind(req.chosen_index as i64 == correct_index)
    .bind(&row.kind)
    .bind(&id)
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

fn expired_beyond_grace(row: &ExamRow) -> bool {
    let (Some(limit), Ok(start)) = (
        row.duration_limit_s,
        chrono::DateTime::parse_from_rfc3339(&row.started_at),
    ) else {
        return false;
    };
    (chrono::Utc::now() - start.with_timezone(&chrono::Utc)).num_seconds() > limit + GRACE_SECONDS
}

#[derive(Serialize)]
pub struct SectionScore {
    correct: i64,
    total: i64,
    scaled: Option<i64>,
}

#[derive(Serialize)]
pub struct PartScore {
    part: i64,
    correct: i64,
    total: i64,
}

#[derive(Serialize)]
pub struct ReviewItem {
    question: QuestionOut,
    chosen_index: Option<i64>,
    correct_index: i64,
    correct: bool,
    explanation_fr: Explanation,
    explanation_en: Explanation,
    transcript: Option<String>,
}

#[derive(Serialize)]
pub struct ResultsOut {
    id: String,
    kind: String,
    started_at: String,
    finished_at: String,
    listening: SectionScore,
    reading: SectionScore,
    total_scaled: Option<i64>,
    by_part: Vec<PartScore>,
    items: Vec<ReviewItem>,
}

#[derive(sqlx::FromRow)]
struct KeyRow {
    id: String,
    part: i64,
    correct_index: i64,
    explanation_fr: String,
    explanation_en: String,
    transcript: Option<String>,
}

/// Termine l'examen (idempotent), calcule les scores et renvoie la correction complète.
pub async fn finish(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<ResultsOut>> {
    results(&state, &user, &id, true).await.map(Json)
}

pub async fn results_get(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<ResultsOut>> {
    results(&state, &user, &id, false).await.map(Json)
}

async fn results(
    state: &AppState,
    user: &AuthUser,
    id: &str,
    close: bool,
) -> ApiResult<ResultsOut> {
    let row = load_exam(state, user, id).await?;
    if row.finished_at.is_none() && !close {
        return Err(ApiError::BadRequest("examen en cours".into()));
    }
    let keys = sqlx::query_as::<_, KeyRow>(
        "SELECT q.id, q.part, q.correct_index, q.explanation_fr, q.explanation_en, p.transcript
         FROM exam_questions e JOIN questions q ON q.id = e.question_id
         LEFT JOIN passages p ON p.id = q.passage_id
         WHERE e.exam_id = $1 ORDER BY e.position",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;
    let chosen: HashMap<String, i64> = sqlx::query_as::<_, (String, i64)>(
        "SELECT question_id, chosen_index FROM attempts WHERE exam_id = $1",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .collect();

    let ids: Vec<String> = keys.iter().map(|k| k.id.clone()).collect();
    let questions = fetch_by_ids(state, &ids).await?;
    let (mut lc, mut lt, mut rc, mut rt) = (0i64, 0i64, 0i64, 0i64);
    let mut parts: std::collections::BTreeMap<i64, (i64, i64)> = Default::default();
    let mut items = Vec::with_capacity(keys.len());
    for (k, question) in keys.iter().zip(questions) {
        let pick = chosen.get(&k.id).copied();
        let ok = pick == Some(k.correct_index);
        let e = parts.entry(k.part).or_default();
        e.1 += 1;
        e.0 += ok as i64;
        if scoring::is_listening(k.part) {
            lt += 1;
            lc += ok as i64;
        } else {
            rt += 1;
            rc += ok as i64;
        }
        let parse = |s: &str| {
            serde_json::from_str::<Explanation>(s).map_err(|e| ApiError::Internal(e.into()))
        };
        items.push(ReviewItem {
            question,
            chosen_index: pick,
            correct_index: k.correct_index,
            correct: ok,
            explanation_fr: parse(&k.explanation_fr)?,
            explanation_en: parse(&k.explanation_en)?,
            transcript: k.transcript.clone(),
        });
    }
    let ls = scoring::estimate_scaled(lc as u32, lt as u32).map(i64::from);
    let rs = scoring::estimate_scaled(rc as u32, rt as u32).map(i64::from);

    let finished_at = match row.finished_at {
        Some(f) => f,
        None => {
            let now = chrono::Utc::now().to_rfc3339();
            sqlx::query(
                "UPDATE exams SET finished_at = $1, listening_raw = $2, reading_raw = $3,
                   listening_scaled = $4, reading_scaled = $5 WHERE id = $6",
            )
            .bind(&now)
            .bind(lc)
            .bind(rc)
            .bind(ls)
            .bind(rs)
            .bind(id)
            .execute(&state.pool)
            .await?;
            now
        }
    };
    Ok(ResultsOut {
        id: row.id,
        kind: row.kind,
        started_at: row.started_at,
        finished_at,
        listening: SectionScore {
            correct: lc,
            total: lt,
            scaled: ls,
        },
        reading: SectionScore {
            correct: rc,
            total: rt,
            scaled: rs,
        },
        total_scaled: ls.zip(rs).map(|(a, b)| a + b),
        by_part: parts
            .into_iter()
            .map(|(part, (correct, total))| PartScore {
                part,
                correct,
                total,
            })
            .collect(),
        items,
    })
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ExamSummary {
    id: String,
    kind: String,
    started_at: String,
    finished_at: Option<String>,
    listening_scaled: Option<i64>,
    reading_scaled: Option<i64>,
}

pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<Vec<ExamSummary>>> {
    let rows = sqlx::query_as::<_, ExamSummary>(
        "SELECT id, kind, started_at, finished_at, listening_scaled, reading_scaled
         FROM exams WHERE user_id = $1 ORDER BY started_at DESC LIMIT 100",
    )
    .bind(&user.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}
