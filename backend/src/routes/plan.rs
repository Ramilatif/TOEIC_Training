//! Planning de révision : génération, réajustement quotidien et suivi.
use super::stats::{category_stats, mistake_count};
use crate::auth::{new_id, AuthUser};
use crate::error::{ApiError, ApiResult};
use crate::planning::{self, LessonRef, PlanInput};
use crate::AppState;
use axum::extract::{Path, State};
use axum::Json;
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, sqlx::FromRow)]
pub struct PlanItemOut {
    id: String,
    day: String,
    kind: String,
    ref_id: Option<String>,
    minutes: i64,
    done: bool,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct PlanHead {
    id: String,
    generated_at: String,
    exam_date: String,
    daily_minutes: i64,
}

#[derive(Serialize)]
pub struct PlanOut {
    plan: Option<PlanHead>,
    items: Vec<PlanItemOut>,
}

fn today() -> NaiveDate {
    Utc::now().date_naive()
}

async fn load(state: &AppState, user_id: &str) -> ApiResult<PlanOut> {
    let plan = sqlx::query_as::<_, PlanHead>(
        "SELECT id, generated_at, exam_date, daily_minutes FROM plans WHERE user_id = $1
         ORDER BY generated_at DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?;
    let items = match &plan {
        Some(p) => {
            sqlx::query_as::<_, PlanItemOut>(
                "SELECT id, day, kind, ref_id, minutes,
                        CASE WHEN done_at IS NULL THEN FALSE ELSE TRUE END AS done
                 FROM plan_items WHERE plan_id = $1 ORDER BY day, kind",
            )
            .bind(&p.id)
            .fetch_all(&state.pool)
            .await?
        }
        None => vec![],
    };
    Ok(PlanOut { plan, items })
}

/// (Re)génère les jours à venir à partir du profil et des résultats actuels.
/// Les éléments passés ou déjà cochés sont conservés.
async fn regenerate(state: &AppState, user_id: &str) -> ApiResult<()> {
    let (exam_date, daily): (Option<String>, i64) =
        sqlx::query_as("SELECT exam_date, daily_minutes FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&state.pool)
            .await?;
    let exam_date = exam_date
        .ok_or_else(|| ApiError::BadRequest("renseignez d'abord votre date d'examen".into()))?;
    let exam = NaiveDate::parse_from_str(&exam_date, "%Y-%m-%d")
        .map_err(|e| ApiError::Internal(e.into()))?;
    let day0 = today();
    if exam < day0 {
        return Err(ApiError::BadRequest(
            "la date d'examen est dans le passé".into(),
        ));
    }

    let weak: Vec<String> = category_stats(state, user_id)
        .await?
        .into_iter()
        .filter(|c| c.accuracy.is_some_and(|a| a < 0.8))
        .take(5)
        .map(|c| c.category)
        .collect();
    let lessons: Vec<(String, String, String, i64)> = sqlx::query_as(
        "SELECT l.id, l.category_id, l.type, l.minutes FROM lessons l
         LEFT JOIN lesson_progress p ON p.lesson_id = l.id AND p.user_id = $1
         WHERE p.quiz_passed_at IS NULL ORDER BY l.id",
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?;
    let (cards,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM flashcards WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&state.pool)
        .await?;
    let input = PlanInput {
        today: day0,
        exam_date: exam,
        daily_minutes: daily,
        weak_categories: weak,
        lessons: lessons
            .into_iter()
            .map(|(id, category, kind, minutes)| LessonRef {
                id,
                category,
                kind,
                minutes,
            })
            .collect(),
        has_cards: cards > 0,
        mistakes: mistake_count(state, user_id).await?,
    };
    let items = planning::generate(&input);

    let now = Utc::now().to_rfc3339();
    let mut tx = state.pool.begin().await?;
    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM plans WHERE user_id = $1 ORDER BY generated_at DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;
    let plan_id = match existing {
        Some((id,)) => {
            sqlx::query(
                "UPDATE plans SET generated_at = $1, exam_date = $2, daily_minutes = $3 WHERE id = $4",
            )
            .bind(&now)
            .bind(&exam_date)
            .bind(daily)
            .bind(&id)
            .execute(&mut *tx)
            .await?;
            // On retire seulement le futur non fait.
            sqlx::query(
                "DELETE FROM plan_items WHERE plan_id = $1 AND day >= $2 AND done_at IS NULL",
            )
            .bind(&id)
            .bind(day0.to_string())
            .execute(&mut *tx)
            .await?;
            id
        }
        None => {
            let id = new_id();
            sqlx::query(
                "INSERT INTO plans (id, user_id, generated_at, exam_date, daily_minutes) VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(&id)
            .bind(user_id)
            .bind(&now)
            .bind(&exam_date)
            .bind(daily)
            .execute(&mut *tx)
            .await?;
            id
        }
    };
    let kept: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT day, kind, ref_id FROM plan_items WHERE plan_id = $1 AND done_at IS NOT NULL",
    )
    .bind(&plan_id)
    .fetch_all(&mut *tx)
    .await?;
    for it in items {
        let day = it.day.to_string();
        if kept
            .iter()
            .any(|(d, k, r)| *d == day && k == it.kind && *r == it.ref_id)
        {
            continue;
        }
        sqlx::query(
            "INSERT INTO plan_items (id, plan_id, day, kind, ref_id, minutes) VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(new_id())
        .bind(&plan_id)
        .bind(day)
        .bind(it.kind)
        .bind(&it.ref_id)
        .bind(it.minutes)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Plan courant. S'il date d'avant aujourd'hui, les jours à venir sont réajustés
/// d'après les derniers résultats (une fois par jour au plus).
pub async fn get(State(state): State<AppState>, user: AuthUser) -> ApiResult<Json<PlanOut>> {
    let current = load(&state, &user.id).await?;
    if let Some(p) = &current.plan {
        let stale = p
            .generated_at
            .get(..10)
            .is_some_and(|d| d < today().to_string().as_str());
        let exam_ahead = p.exam_date.as_str() >= today().to_string().as_str();
        if stale && exam_ahead {
            regenerate(&state, &user.id).await?;
            return Ok(Json(load(&state, &user.id).await?));
        }
    }
    Ok(Json(current))
}

pub async fn generate(State(state): State<AppState>, user: AuthUser) -> ApiResult<Json<PlanOut>> {
    regenerate(&state, &user.id).await?;
    Ok(Json(load(&state, &user.id).await?))
}

#[derive(Deserialize)]
pub struct DoneReq {
    done: bool,
}

pub async fn set_done(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<DoneReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let res = sqlx::query(
        "UPDATE plan_items SET done_at = $1 WHERE id = $2 AND plan_id IN
           (SELECT id FROM plans WHERE user_id = $3)",
    )
    .bind(req.done.then(|| Utc::now().to_rfc3339()))
    .bind(&id)
    .bind(&user.id)
    .execute(&state.pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound("élément introuvable".into()));
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}
