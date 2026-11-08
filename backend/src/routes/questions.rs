//! Sélection de questions pour l'entraînement et correction.
use crate::auth::{new_id, AuthUser};
use crate::content::model::Explanation;
use crate::error::{ApiError, ApiResult};
use crate::AppState;
use axum::extract::{Path, Query, State};
use axum::Json;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ListQuery {
    part: Option<i64>,
    category: Option<String>,
    difficulty: Option<i64>,
    limit: Option<usize>,
    /// Si vrai, exclut les questions déjà tentées par l'utilisateur.
    unseen: Option<bool>,
}

#[derive(Serialize)]
pub struct PassageOut {
    id: String,
    kind: String,
    title: Option<String>,
    body: Option<String>,
    /// Texte à lire par la synthèse vocale (Listening). Masqué côté client avant la réponse.
    transcript: Option<String>,
    image_url: Option<String>,
}

#[derive(Serialize)]
pub struct QuestionOut {
    id: String,
    part: i64,
    difficulty: i64,
    category: String,
    stem: String,
    choices: Vec<String>,
    position: i64,
    passage: Option<PassageOut>,
}

#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    part: i64,
    difficulty: i64,
    category_id: String,
    stem: String,
    choices_json: String,
    position_in_passage: i64,
    passage_id: Option<String>,
    p_kind: Option<String>,
    p_title: Option<String>,
    p_body: Option<String>,
    p_transcript: Option<String>,
    p_image: Option<String>,
}

const SELECT: &str = "SELECT q.id, q.part, q.difficulty, q.category_id, q.stem, q.choices_json, q.position_in_passage,
    q.passage_id, p.kind AS p_kind, p.title AS p_title, p.body_text AS p_body,
    p.transcript AS p_transcript, i.path AS p_image
    FROM questions q
    LEFT JOIN passages p ON p.id = q.passage_id
    LEFT JOIN images i ON i.id = p.image_id";

fn to_out(r: Row) -> ApiResult<QuestionOut> {
    let passage = r.passage_id.map(|id| PassageOut {
        id,
        kind: r.p_kind.unwrap_or_default(),
        title: r.p_title,
        body: r.p_body,
        transcript: r.p_transcript,
        image_url: r.p_image.map(|f| format!("/media/part1/{f}")),
    });
    Ok(QuestionOut {
        id: r.id,
        part: r.part,
        difficulty: r.difficulty,
        category: r.category_id,
        stem: r.stem,
        choices: serde_json::from_str(&r.choices_json).map_err(|e| ApiError::Internal(e.into()))?,
        position: r.position_in_passage,
        passage,
    })
}

/// Sélection pour l'entraînement. Les questions d'un même passage restent groupées :
/// on tire des « unités » (passage ou question isolée) puis on les déplie.
pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Vec<QuestionOut>>> {
    // Les questions de quiz de leçon n'entrent pas dans l'entraînement général.
    let mut sql = format!("{SELECT} WHERE q.batch NOT LIKE 'lesson-%'");
    let mut binds: Vec<String> = Vec::new();
    if let Some(p) = q.part {
        binds.push(p.to_string());
        sql.push_str(&format!(" AND q.part = CAST(${} AS INTEGER)", binds.len()));
    }
    if let Some(c) = &q.category {
        binds.push(c.clone());
        sql.push_str(&format!(" AND q.category_id = ${}", binds.len()));
    }
    if let Some(d) = q.difficulty {
        binds.push(d.to_string());
        sql.push_str(&format!(
            " AND q.difficulty = CAST(${} AS INTEGER)",
            binds.len()
        ));
    }
    if q.unseen.unwrap_or(false) {
        binds.push(user.id.clone());
        sql.push_str(&format!(
            " AND q.id NOT IN (SELECT question_id FROM attempts WHERE user_id = ${})",
            binds.len()
        ));
    }
    sql.push_str(" ORDER BY q.passage_id, q.position_in_passage, q.id");
    let mut query = sqlx::query_as::<_, Row>(&sql);
    for b in &binds {
        query = query.bind(b);
    }
    let rows = query.fetch_all(&state.pool).await?;

    // Regroupe en unités.
    let mut units: Vec<Vec<Row>> = Vec::new();
    for r in rows {
        match (&r.passage_id, units.last()) {
            (Some(pid), Some(last)) if last[0].passage_id.as_ref() == Some(pid) => {
                units.last_mut().unwrap().push(r)
            }
            _ => units.push(vec![r]),
        }
    }
    units.shuffle(&mut rand::thread_rng());
    let limit = q.limit.unwrap_or(10).clamp(1, 200);
    let mut out = Vec::new();
    for unit in units {
        if !out.is_empty() && out.len() + unit.len() > limit {
            continue;
        }
        for r in unit {
            out.push(to_out(r)?);
        }
        if out.len() >= limit {
            break;
        }
    }
    Ok(Json(out))
}

#[derive(Serialize)]
pub struct PartSummary {
    part: i64,
    /// Questions disponibles (hors quiz de cours).
    total: i64,
    /// Questions déjà tentées par l'utilisateur.
    seen: i64,
    /// Questions dont la dernière tentative est juste.
    mastered: i64,
}

#[derive(Serialize)]
pub struct CategorySummary {
    part: i64,
    category: String,
    total: i64,
}

#[derive(Serialize)]
pub struct Summary {
    parts: Vec<PartSummary>,
    categories: Vec<CategorySummary>,
}

/// Résumé de la banque pour l'écran d'entraînement : avancement par partie et catégories disponibles.
pub async fn summary(State(state): State<AppState>, user: AuthUser) -> ApiResult<Json<Summary>> {
    let parts: Vec<(i64, i64, i64, i64)> = sqlx::query_as(
        "SELECT q.part, COUNT(*),
                SUM(CASE WHEN l.qid IS NULL THEN 0 ELSE 1 END),
                SUM(CASE WHEN l.ok THEN 1 ELSE 0 END)
         FROM questions q
         LEFT JOIN (
             SELECT a.question_id AS qid, a.is_correct AS ok FROM attempts a
             WHERE a.user_id = $1 AND a.answered_at = (
                 SELECT MAX(a2.answered_at) FROM attempts a2
                 WHERE a2.user_id = a.user_id AND a2.question_id = a.question_id)
         ) l ON l.qid = q.id
         WHERE q.batch NOT LIKE 'lesson-%'
         GROUP BY q.part ORDER BY q.part",
    )
    .bind(&user.id)
    .fetch_all(&state.pool)
    .await?;
    let categories: Vec<(i64, String, i64)> = sqlx::query_as(
        "SELECT part, category_id, COUNT(*) FROM questions
         WHERE batch NOT LIKE 'lesson-%'
         GROUP BY part, category_id ORDER BY part, category_id",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(Summary {
        parts: parts
            .into_iter()
            .map(|(part, total, seen, mastered)| PartSummary {
                part,
                total,
                seen,
                mastered,
            })
            .collect(),
        categories: categories
            .into_iter()
            .map(|(part, category, total)| CategorySummary {
                part,
                category,
                total,
            })
            .collect(),
    }))
}

#[derive(Deserialize)]
pub struct AnswerReq {
    chosen_index: usize,
    #[serde(default)]
    time_ms: Option<i64>,
    /// "practice" (défaut) ou "review" (rejeu d'une erreur).
    #[serde(default)]
    mode: Option<String>,
}

#[derive(Serialize)]
pub struct AnswerOut {
    correct: bool,
    correct_index: usize,
    explanation_fr: Explanation,
    explanation_en: Explanation,
    transcript: Option<String>,
    category: String,
    /// Leçon liée à la catégorie (lien depuis une correction ratée).
    lesson_id: Option<String>,
}

#[derive(sqlx::FromRow)]
struct AnswerRow {
    correct_index: i64,
    explanation_fr: String,
    explanation_en: String,
    category_id: String,
    part: i64,
    choices_json: String,
    transcript: Option<String>,
}

pub async fn answer(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<AnswerReq>,
) -> ApiResult<Json<AnswerOut>> {
    let row = sqlx::query_as::<_, AnswerRow>(
        "SELECT q.correct_index, q.explanation_fr, q.explanation_en, q.category_id, q.part, q.choices_json, p.transcript
         FROM questions q LEFT JOIN passages p ON p.id = q.passage_id WHERE q.id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::NotFound("question introuvable".into()))?;

    let n: Vec<String> =
        serde_json::from_str(&row.choices_json).map_err(|e| ApiError::Internal(e.into()))?;
    if req.chosen_index >= n.len() {
        return Err(ApiError::BadRequest("choix hors bornes".into()));
    }
    let correct = req.chosen_index as i64 == row.correct_index;
    let mode = match req.mode.as_deref() {
        None | Some("practice") => "practice",
        Some("review") => "review",
        Some(_) => return Err(ApiError::BadRequest("mode : practice ou review".into())),
    };
    sqlx::query(
        "INSERT INTO attempts (id, user_id, question_id, chosen_index, is_correct, mode, answered_at, time_ms)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(new_id())
    .bind(&user.id)
    .bind(&id)
    .bind(req.chosen_index as i64)
    .bind(correct)
    .bind(mode)
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(req.time_ms)
    .execute(&state.pool)
    .await?;

    let lesson_id = lesson_for(&state, &row.category_id, row.part).await?;
    let parse =
        |s: &str| serde_json::from_str::<Explanation>(s).map_err(|e| ApiError::Internal(e.into()));
    Ok(Json(AnswerOut {
        correct,
        correct_index: row.correct_index as usize,
        explanation_fr: parse(&row.explanation_fr)?,
        explanation_en: parse(&row.explanation_en)?,
        transcript: row.transcript,
        category: row.category_id,
        lesson_id,
    }))
}

/// Charge des questions par ids, en conservant l'ordre demandé.
pub(crate) async fn fetch_by_ids(state: &AppState, ids: &[String]) -> ApiResult<Vec<QuestionOut>> {
    let sql = format!("{SELECT} WHERE q.id = $1");
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(r) = sqlx::query_as::<_, Row>(&sql)
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
        {
            out.push(to_out(r)?);
        }
    }
    Ok(out)
}

/// Leçon à recommander pour une catégorie : leçon de grammaire/vocabulaire de la catégorie,
/// à défaut la leçon de méthodologie de la partie (`method-part-N`).
pub(crate) async fn lesson_for(
    state: &AppState,
    category: &str,
    part: i64,
) -> ApiResult<Option<String>> {
    let direct: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM lessons WHERE category_id = $1 AND type <> 'method' ORDER BY (type IN ('conjugation', 'advanced')), id LIMIT 1",
    )
    .bind(category)
    .fetch_optional(&state.pool)
    .await?;
    if let Some((id,)) = direct {
        return Ok(Some(id));
    }
    let method: Option<(String,)> = sqlx::query_as("SELECT id FROM lessons WHERE id = $1")
        .bind(format!("method-part-{part}"))
        .fetch_optional(&state.pool)
        .await?;
    Ok(method.map(|m| m.0))
}
