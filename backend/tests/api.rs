//! Test d'intégration : import du vrai contenu, login, entraînement, réponse.
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::sync::Arc;
use toeic_backend::{app, config::Config, content, db, AppState};
use tower::ServiceExt;

async fn setup() -> axum::Router {
    let pool = db::connect("sqlite::memory:").await.unwrap();
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../content");
    content::import_all(&pool, &dir).await.unwrap();
    let mut cfg = Config::from_env();
    cfg.content_dir = dir;
    app(AppState {
        pool,
        config: Arc::new(cfg),
    })
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(t) = token {
        req = req.header("authorization", format!("Bearer {t}"));
    }
    let res = app
        .clone()
        .oneshot(req.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn login_requis_puis_entrainement() {
    let app = setup().await;

    let (s, _) = call(&app, "GET", "/api/questions?part=5", None, Value::Null).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);

    let (s, v) = call(
        &app,
        "POST",
        "/api/auth/login",
        None,
        json!({"pseudo": "Alice"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let token = v["token"].as_str().unwrap().to_string();

    // Même pseudo (casse différente) = même utilisateur.
    let (_, v2) = call(
        &app,
        "POST",
        "/api/auth/login",
        None,
        json!({"pseudo": "alice"}),
    )
    .await;
    assert_eq!(v["user"]["id"], v2["user"]["id"]);

    let (s, qs) = call(
        &app,
        "GET",
        "/api/questions?part=5&limit=5",
        Some(&token),
        Value::Null,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let qs = qs.as_array().unwrap();
    assert_eq!(qs.len(), 5);
    // La bonne réponse ne doit jamais être exposée avant la réponse.
    assert!(qs[0].get("correct").is_none() && qs[0].get("correct_index").is_none());

    let id = qs[0]["id"].as_str().unwrap();
    let (s, a) = call(
        &app,
        "POST",
        &format!("/api/questions/{id}/answer"),
        Some(&token),
        json!({"chosen_index": 0}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert!(a["correct_index"].is_number());
    assert!(a["explanation_fr"]["rule"].is_string());
}

#[tokio::test]
async fn passage_groupe_en_part6() {
    let app = setup().await;
    let (_, v) = call(
        &app,
        "POST",
        "/api/auth/login",
        None,
        json!({"pseudo": "bob"}),
    )
    .await;
    let token = v["token"].as_str().unwrap();
    let (_, qs) = call(
        &app,
        "GET",
        "/api/questions?part=6&limit=10",
        Some(token),
        Value::Null,
    )
    .await;
    let qs = qs.as_array().unwrap();
    assert_eq!(
        qs.len(),
        4,
        "les 4 questions du texte sont servies ensemble"
    );
    assert!(qs[0]["passage"]["body"].is_string());
}

#[tokio::test]
async fn reponse_invalide_rejetee() {
    let app = setup().await;
    let (_, v) = call(
        &app,
        "POST",
        "/api/auth/login",
        None,
        json!({"pseudo": "carol"}),
    )
    .await;
    let token = v["token"].as_str().unwrap();
    let (s, _) = call(
        &app,
        "POST",
        "/api/questions/p5-b01-001/answer",
        Some(token),
        json!({"chosen_index": 9}),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = call(
        &app,
        "POST",
        "/api/auth/login",
        None,
        json!({"pseudo": "x"}),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn listening_parts_servent_transcription() {
    let app = setup().await;
    let (_, v) = call(
        &app,
        "POST",
        "/api/auth/login",
        None,
        json!({"pseudo": "dave"}),
    )
    .await;
    let token = v["token"].as_str().unwrap();

    // Part 3 : un passage = 3 questions servies ensemble avec la transcription à lire.
    let (_, qs) = call(
        &app,
        "GET",
        "/api/questions?part=3&limit=3",
        Some(token),
        Value::Null,
    )
    .await;
    let qs = qs.as_array().unwrap();
    assert_eq!(qs.len(), 3);
    assert!(qs[0]["passage"]["transcript"]
        .as_str()
        .unwrap()
        .contains("W:"));
    assert_eq!(qs[0]["passage"]["id"], qs[2]["passage"]["id"]);

    // Part 2 : trois réponses seulement.
    let (_, qs) = call(
        &app,
        "GET",
        "/api/questions?part=2&limit=1",
        Some(token),
        Value::Null,
    )
    .await;
    assert_eq!(qs[0]["choices"].as_array().unwrap().len(), 3);

    // Part 1 : sans photo déposée, aucune question n'est importée (et le serveur démarre quand même).
    let (s, qs) = call(
        &app,
        "GET",
        "/api/questions?part=1&limit=5",
        Some(token),
        Value::Null,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert!(qs.as_array().unwrap().len() <= 12);
}
