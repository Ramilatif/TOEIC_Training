//! Erreur d'API, sérialisée en `{ "error": { "code", "message" } }`.
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),
    #[error("authentification requise")]
    Unauthorized,
    #[error("{0}")]
    NotFound(String),
    #[error("erreur interne")]
    Internal(#[from] anyhow::Error),
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        ApiError::Internal(e.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            ApiError::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            ApiError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            ApiError::Internal(e) => {
                tracing::error!("erreur interne: {e:#}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal")
            }
        };
        let body = serde_json::json!({ "error": { "code": code, "message": self.to_string() } });
        (status, Json(body)).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
