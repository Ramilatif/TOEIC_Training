//! API de l'appli d'entraînement TOEIC.
pub mod auth;
pub mod config;
pub mod content;
pub mod db;
pub mod error;
pub mod planning;
pub mod routes;
pub mod scoring;
pub mod srs;

use axum::Router;
use sqlx::SqlitePool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<config::Config>,
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .nest("/api", routes::router())
        .with_state(state)
}
