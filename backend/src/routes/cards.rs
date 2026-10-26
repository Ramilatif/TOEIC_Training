//! Flashcards de vocabulaire avec répétition espacée (SM-2).
use crate::auth::{new_id, AuthUser};
use crate::error::{ApiError, ApiResult};
use crate::srs::{self, CardState};
use crate::AppState;
use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Serialize, sqlx::FromRow)]
pub struct CardOut {
    id: String,
    term: String,
    translation: String,
    example: Option<String>,
    source: String,
    ease: f64,
    interval_days: i64,
    repetitions: i64,
    due_date: String,
}

const COLS: &str =
    "id, term, translation, example, source, ease, interval_days, repetitions, due_date";

fn today() -> String {
    Utc::now().date_naive().to_string()
}

pub async fn list(State(state): State<AppState>, user: AuthUser) -> ApiResult<Json<Vec<CardOut>>> {
    let rows = sqlx::query_as::<_, CardOut>(&format!(
        "SELECT {COLS} FROM flashcards WHERE user_id = $1 ORDER BY term"
    ))
    .bind(&user.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

#[derive(Deserialize)]
pub struct DueQuery {
    limit: Option<i64>,
}

#[derive(Serialize)]
pub struct DueOut {
    /// Nombre total de cartes à réviser aujourd'hui.
    total: i64,
    cards: Vec<CardOut>,
}

pub async fn due(
    State(state): State<AppState>,
    user: AuthUser,
    Query(q): Query<DueQuery>,
) -> ApiResult<Json<DueOut>> {
    let day = today();
    let (total,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM flashcards WHERE user_id = $1 AND due_date <= $2")
            .bind(&user.id)
            .bind(&day)
            .fetch_one(&state.pool)
            .await?;
    let cards = sqlx::query_as::<_, CardOut>(&format!(
        "SELECT {COLS} FROM flashcards WHERE user_id = $1 AND due_date <= $2
         ORDER BY due_date, term LIMIT $3"
    ))
    .bind(&user.id)
    .bind(&day)
    .bind(q.limit.unwrap_or(20).clamp(1, 200))
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(DueOut { total, cards }))
}

#[derive(Deserialize)]
pub struct NewCard {
    term: String,
    translation: String,
    example: Option<String>,
}

#[derive(Serialize)]
pub struct AddResult {
    added: usize,
    skipped: usize,
}

/// Insère une carte neuve (due aujourd'hui). Retourne false si le terme existe déjà.
async fn insert_card(
    state: &AppState,
    user_id: &str,
    term: &str,
    translation: &str,
    example: Option<&str>,
    source: &str,
    source_id: Option<&str>,
) -> ApiResult<bool> {
    let res = sqlx::query(
        "INSERT INTO flashcards (id, user_id, term, translation, example, source, source_id,
           ease, interval_days, repetitions, due_date, created_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,0,0,$9,$10)
         ON CONFLICT(user_id, term) DO NOTHING",
    )
    .bind(new_id())
    .bind(user_id)
    .bind(term.trim())
    .bind(translation.trim())
    .bind(example)
    .bind(source)
    .bind(source_id)
    .bind(srs::DEFAULT_EASE)
    .bind(today())
    .bind(Utc::now().to_rfc3339())
    .execute(&state.pool)
    .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn add_manual(
    State(state): State<AppState>,
    user: AuthUser,
    Json(c): Json<NewCard>,
) -> ApiResult<Json<AddResult>> {
    if c.term.trim().is_empty() || c.translation.trim().is_empty() {
        return Err(ApiError::BadRequest("terme et traduction requis".into()));
    }
    if c.term.chars().count() > 100 || c.translation.chars().count() > 200 {
        return Err(ApiError::BadRequest("texte trop long".into()));
    }
    let ok = insert_card(
        &state,
        &user.id,
        &c.term,
        &c.translation,
        c.example.as_deref(),
        "manual",
        None,
    )
    .await?;
    Ok(Json(AddResult {
        added: ok as usize,
        skipped: !ok as usize,
    }))
}

#[derive(Deserialize, Default)]
pub struct FromLesson {
    /// Termes à ajouter ; tous les termes de la leçon si absent.
    terms: Option<Vec<String>>,
}

pub async fn add_from_lesson(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    body: Option<Json<FromLesson>>,
) -> ApiResult<Json<AddResult>> {
    let wanted = body.and_then(|b| b.0.terms);
    let rows: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT term, translation_fr, example FROM lesson_vocab WHERE lesson_id = $1 ORDER BY term",
    )
    .bind(&id)
    .fetch_all(&state.pool)
    .await?;
    if rows.is_empty() {
        return Err(ApiError::NotFound(
            "aucun vocabulaire pour cette leçon".into(),
        ));
    }
    let (mut added, mut skipped) = (0, 0);
    for (term, tr, ex) in rows {
        if wanted.as_ref().is_some_and(|w| !w.contains(&term)) {
            continue;
        }
        if insert_card(
            &state,
            &user.id,
            &term,
            &tr,
            ex.as_deref(),
            "lesson",
            Some(&id),
        )
        .await?
        {
            added += 1;
        } else {
            skipped += 1;
        }
    }
    Ok(Json(AddResult { added, skipped }))
}

pub async fn add_from_question(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<AddResult>> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT term, translation_fr FROM question_vocab WHERE question_id = $1")
            .bind(&id)
            .fetch_all(&state.pool)
            .await?;
    let (mut added, mut skipped) = (0, 0);
    for (term, tr) in rows {
        if insert_card(&state, &user.id, &term, &tr, None, "question", Some(&id)).await? {
            added += 1;
        } else {
            skipped += 1;
        }
    }
    Ok(Json(AddResult { added, skipped }))
}

pub async fn delete(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "DELETE FROM card_reviews WHERE card_id IN (SELECT id FROM flashcards WHERE id = $1 AND user_id = $2)",
    )
    .bind(&id)
    .bind(&user.id)
    .execute(&mut *tx)
    .await?;
    let res = sqlx::query("DELETE FROM flashcards WHERE id = $1 AND user_id = $2")
        .bind(&id)
        .bind(&user.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound("carte introuvable".into()));
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct ReviewReq {
    quality: u8,
}

pub async fn review(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<ReviewReq>,
) -> ApiResult<Json<CardOut>> {
    if req.quality > 5 {
        return Err(ApiError::BadRequest("qualité : 0 à 5".into()));
    }
    let (ease, interval, reps): (f64, i64, i64) = sqlx::query_as(
        "SELECT ease, interval_days, repetitions FROM flashcards WHERE id = $1 AND user_id = $2",
    )
    .bind(&id)
    .bind(&user.id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::NotFound("carte introuvable".into()))?;

    let next = srs::review(
        CardState {
            ease,
            interval_days: interval,
            repetitions: reps,
        },
        req.quality,
    );
    let due = srs::due_date(Utc::now().date_naive(), &next).to_string();
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "UPDATE flashcards SET ease = $1, interval_days = $2, repetitions = $3, due_date = $4 WHERE id = $5",
    )
    .bind(next.ease)
    .bind(next.interval_days)
    .bind(next.repetitions)
    .bind(&due)
    .bind(&id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO card_reviews (id, card_id, quality, reviewed_at) VALUES ($1,$2,$3,$4)",
    )
    .bind(new_id())
    .bind(&id)
    .bind(req.quality as i64)
    .bind(Utc::now().to_rfc3339())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    let card =
        sqlx::query_as::<_, CardOut>(&format!("SELECT {COLS} FROM flashcards WHERE id = $1"))
            .bind(&id)
            .fetch_one(&state.pool)
            .await?;
    Ok(Json(card))
}
