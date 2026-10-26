//! Connexion, profil et catégories.
use crate::auth::{create_session, AuthUser, Authenticator, Credentials, PseudoAuthenticator};
use crate::error::{ApiError, ApiResult};
use crate::AppState;
use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct LoginReq {
    pseudo: String,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct UserOut {
    id: String,
    pseudo: String,
    goal_score: i64,
    exam_date: Option<String>,
    daily_minutes: i64,
    lang_pref: String,
}

#[derive(Serialize)]
pub struct LoginOut {
    token: String,
    user: UserOut,
}

async fn load_user(state: &AppState, id: &str) -> ApiResult<UserOut> {
    sqlx::query_as::<_, UserOut>(
        "SELECT id, pseudo, goal_score, exam_date, daily_minutes, lang_pref FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::Unauthorized)
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginReq>,
) -> ApiResult<Json<LoginOut>> {
    let id = PseudoAuthenticator
        .authenticate(&state.pool, &Credentials { pseudo: req.pseudo })
        .await?;
    let token = create_session(&state, &id).await?;
    Ok(Json(LoginOut {
        token,
        user: load_user(&state, &id).await?,
    }))
}

pub async fn logout(
    State(state): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    sqlx::query("DELETE FROM sessions WHERE token = $1")
        .bind(&user.token)
        .execute(&state.pool)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn me(State(state): State<AppState>, user: AuthUser) -> ApiResult<Json<UserOut>> {
    Ok(Json(load_user(&state, &user.id).await?))
}

#[derive(Deserialize)]
pub struct UpdateMe {
    goal_score: Option<i64>,
    exam_date: Option<String>,
    daily_minutes: Option<i64>,
    lang_pref: Option<String>,
}

pub async fn update_me(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<UpdateMe>,
) -> ApiResult<Json<UserOut>> {
    if let Some(g) = req.goal_score {
        if !(10..=990).contains(&g) {
            return Err(ApiError::BadRequest("objectif hors 10-990".into()));
        }
        sqlx::query("UPDATE users SET goal_score = $1 WHERE id = $2")
            .bind(g)
            .bind(&user.id)
            .execute(&state.pool)
            .await?;
    }
    if let Some(m) = req.daily_minutes {
        if !(5..=600).contains(&m) {
            return Err(ApiError::BadRequest("minutes par jour hors 5-600".into()));
        }
        sqlx::query("UPDATE users SET daily_minutes = $1 WHERE id = $2")
            .bind(m)
            .bind(&user.id)
            .execute(&state.pool)
            .await?;
    }
    if let Some(d) = req.exam_date {
        // Chaîne vide = suppression de la date.
        let v = if d.is_empty() {
            None
        } else {
            chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d")
                .map_err(|_| ApiError::BadRequest("date attendue au format AAAA-MM-JJ".into()))?;
            Some(d)
        };
        sqlx::query("UPDATE users SET exam_date = $1 WHERE id = $2")
            .bind(v)
            .bind(&user.id)
            .execute(&state.pool)
            .await?;
    }
    if let Some(l) = req.lang_pref {
        if l != "fr" && l != "en" {
            return Err(ApiError::BadRequest("langue : fr ou en".into()));
        }
        sqlx::query("UPDATE users SET lang_pref = $1 WHERE id = $2")
            .bind(l)
            .bind(&user.id)
            .execute(&state.pool)
            .await?;
    }
    Ok(Json(load_user(&state, &user.id).await?))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct CategoryOut {
    id: String,
    kind: String,
    label_fr: String,
    label_en: String,
}

pub async fn categories(State(state): State<AppState>) -> ApiResult<Json<Vec<CategoryOut>>> {
    let rows = sqlx::query_as::<_, CategoryOut>(
        "SELECT id, kind, label_fr, label_en FROM categories ORDER BY kind, id",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}
