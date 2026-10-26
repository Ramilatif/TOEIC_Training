//! Authentification isolée derrière le trait `Authenticator`.
//! Aujourd'hui : pseudo seul. Pour ajouter un mot de passe : vérifier `users.password_hash`
//! (argon2) dans `PseudoAuthenticator::authenticate` et passer le mot de passe dans `Credentials`.
use crate::error::{ApiError, ApiResult};
use crate::AppState;
use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use rand::RngCore;
use sqlx::SqlitePool;

pub struct Credentials {
    pub pseudo: String,
}

#[async_trait]
pub trait Authenticator: Send + Sync {
    /// Retourne l'id de l'utilisateur authentifié (créé si nécessaire).
    async fn authenticate(&self, pool: &SqlitePool, c: &Credentials) -> ApiResult<String>;
}

pub struct PseudoAuthenticator;

/// Normalise un pseudo : 3-30 caractères alphanumériques, `_` ou `-`.
pub fn normalize_pseudo(p: &str) -> ApiResult<String> {
    let p = p.trim();
    let ok = (3..=30).contains(&p.chars().count())
        && p.chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-');
    if !ok {
        return Err(ApiError::BadRequest(
            "pseudo invalide (3 à 30 caractères : lettres, chiffres, _ ou -)".into(),
        ));
    }
    Ok(p.to_lowercase())
}

pub fn new_id() -> String {
    random_hex(16)
}

pub fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

#[async_trait]
impl Authenticator for PseudoAuthenticator {
    async fn authenticate(&self, pool: &SqlitePool, c: &Credentials) -> ApiResult<String> {
        let norm = normalize_pseudo(&c.pseudo)?;
        if let Some((id,)) =
            sqlx::query_as::<_, (String,)>("SELECT id FROM users WHERE pseudo_norm = $1")
                .bind(&norm)
                .fetch_optional(pool)
                .await?
        {
            return Ok(id);
        }
        let id = new_id();
        sqlx::query("INSERT INTO users (id, pseudo, pseudo_norm, created_at) VALUES ($1,$2,$3,$4)")
            .bind(&id)
            .bind(c.pseudo.trim())
            .bind(&norm)
            .bind(chrono::Utc::now().to_rfc3339())
            .execute(pool)
            .await?;
        Ok(id)
    }
}

pub async fn create_session(state: &AppState, user_id: &str) -> ApiResult<String> {
    let token = random_hex(32);
    let now = chrono::Utc::now();
    let exp = now + chrono::Duration::hours(state.config.session_ttl_hours);
    sqlx::query(
        "INSERT INTO sessions (token, user_id, created_at, expires_at) VALUES ($1,$2,$3,$4)",
    )
    .bind(&token)
    .bind(user_id)
    .bind(now.to_rfc3339())
    .bind(exp.to_rfc3339())
    .execute(&state.pool)
    .await?;
    Ok(token)
}

/// Extracteur : utilisateur courant déduit du jeton `Authorization: Bearer`.
pub struct AuthUser {
    pub id: String,
    pub token: String,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let token = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(ApiError::Unauthorized)?
            .to_string();
        let row: Option<(String, String)> =
            sqlx::query_as("SELECT user_id, expires_at FROM sessions WHERE token = $1")
                .bind(&token)
                .fetch_optional(&state.pool)
                .await?;
        let (user_id, expires) = row.ok_or(ApiError::Unauthorized)?;
        let valid = chrono::DateTime::parse_from_rfc3339(&expires)
            .map(|e| e > chrono::Utc::now())
            .unwrap_or(false);
        if !valid {
            return Err(ApiError::Unauthorized);
        }
        Ok(AuthUser { id: user_id, token })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pseudo_valid_et_invalide() {
        assert_eq!(normalize_pseudo("  Rami_1 ").unwrap(), "rami_1");
        assert!(normalize_pseudo("ab").is_err());
        assert!(normalize_pseudo("a b c").is_err());
        assert!(normalize_pseudo(&"x".repeat(31)).is_err());
    }
}
