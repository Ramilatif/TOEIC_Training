//! Routes HTTP, regroupées par domaine.
mod account;
mod cards;
mod exams;
mod lessons;
mod plan;
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
        .route("/questions/summary", get(questions::summary))
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
        .route("/cards", get(cards::list).post(cards::add_manual))
        .route("/cards/due", get(cards::due))
        .route("/cards/from-lesson/{id}", post(cards::add_from_lesson))
        .route("/cards/from-question/{id}", post(cards::add_from_question))
        .route("/cards/{id}", axum::routing::delete(cards::delete))
        .route("/cards/{id}/review", post(cards::review))
        .route("/plan", get(plan::get).post(plan::generate))
        .route("/plan/items/{id}/done", post(plan::set_done))
        .route("/exams", get(exams::list).post(exams::create))
        .route("/exams/{id}", get(exams::get))
        .route("/exams/{id}/answers", post(exams::save_answer))
        .route("/exams/{id}/finish", post(exams::finish))
        .route("/exams/{id}/results", get(exams::results_get))
}
