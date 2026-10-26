//! Routes HTTP, regroupées par domaine.
mod account;
mod exams;
mod lessons;
pub(crate) mod questions;
mod stats;

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
        .route("/stats/overview", get(stats::overview))
        .route("/stats/by-part", get(stats::by_part))
        .route("/stats/by-category", get(stats::by_category))
        .route("/review/mistakes", get(stats::mistakes))
        .route("/lessons", get(lessons::list))
        .route("/lessons/progress", get(lessons::progress))
        .route("/lessons/{id}", get(lessons::get))
        .route("/lessons/{id}/read", post(lessons::mark_read))
        .route("/lessons/{id}/quiz", post(lessons::submit_quiz))
        .route("/exams", get(exams::list).post(exams::create))
        .route("/exams/{id}", get(exams::get))
        .route("/exams/{id}/answers", post(exams::save_answer))
        .route("/exams/{id}/finish", post(exams::finish))
        .route("/exams/{id}/results", get(exams::results_get))
}
