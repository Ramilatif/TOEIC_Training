//! Routes HTTP, regroupées par domaine.
mod account;
mod questions;

use crate::AppState;
use axum::routing::{get, post};
use axum::Router;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(account::login))
        .route("/auth/logout", post(account::logout))
        .route("/me", get(account::me).patch(account::update_me))
        .route("/categories", get(account::categories))
        .route("/questions", get(questions::list))
        .route("/questions/{id}/answer", post(questions::answer))
}
