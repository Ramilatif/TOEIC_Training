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

#[tokio::test]
async fn examen_complet_de_bout_en_bout() {
    let app = setup().await;
    let (_, v) = call(
        &app,
        "POST",
        "/api/auth/login",
        None,
        json!({"pseudo": "erin"}),
    )
    .await;
    let token = v["token"].as_str().unwrap();

    let (s, exam) = call(
        &app,
        "POST",
        "/api/exams",
        Some(token),
        json!({"kind": "mini", "parts": [5, 6], "count": 8, "timed": true}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let id = exam["id"].as_str().unwrap().to_string();
    let qs = exam["questions"].as_array().unwrap();
    assert!(!qs.is_empty() && qs.len() <= 8);
    assert!(exam["remaining_s"].as_i64().unwrap() > 0);
    // La correction n'est jamais exposée pendant l'examen.
    assert!(qs[0].get("correct_index").is_none());

    // Réponse à la première question, puis remplacement (une seule tentative gardée).
    let q0 = qs[0]["id"].as_str().unwrap();
    for choice in [0, 1] {
        let (s, _) = call(
            &app,
            "POST",
            &format!("/api/exams/{id}/answers"),
            Some(token),
            json!({"question_id": q0, "chosen_index": choice}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
    }
    let (_, again) = call(
        &app,
        "GET",
        &format!("/api/exams/{id}"),
        Some(token),
        Value::Null,
    )
    .await;
    assert_eq!(again["answers"][q0], 1);

    // Les résultats ne sont pas accessibles avant la fin.
    let (s, _) = call(
        &app,
        "GET",
        &format!("/api/exams/{id}/results"),
        Some(token),
        Value::Null,
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    let (s, r) = call(
        &app,
        "POST",
        &format!("/api/exams/{id}/finish"),
        Some(token),
        Value::Null,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(r["listening"]["total"], 0);
    assert!(r["reading"]["total"].as_i64().unwrap() >= 1);
    assert!(r["listening"]["scaled"].is_null());
    assert!(r["reading"]["scaled"].as_i64().unwrap() >= 5);
    assert_eq!(r["items"].as_array().unwrap().len(), qs.len());

    // Un autre utilisateur ne peut pas lire cet examen.
    let (_, v2) = call(
        &app,
        "POST",
        "/api/auth/login",
        None,
        json!({"pseudo": "frank"}),
    )
    .await;
    let t2 = v2["token"].as_str().unwrap();
    let (s, _) = call(
        &app,
        "GET",
        &format!("/api/exams/{id}"),
        Some(t2),
        Value::Null,
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn examen_complet_respecte_la_repartition_disponible() {
    let app = setup().await;
    let (_, v) = call(
        &app,
        "POST",
        "/api/auth/login",
        None,
        json!({"pseudo": "gina"}),
    )
    .await;
    let token = v["token"].as_str().unwrap();
    let (s, exam) = call(
        &app,
        "POST",
        "/api/exams",
        Some(token),
        json!({"kind": "full"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(exam["duration_limit_s"], 7200);
    // Tant que la banque est incomplète, on sert tout ce qui existe sans dépasser le quota officiel.
    let n = exam["questions"].as_array().unwrap().len();
    assert!(n > 0 && n <= 200);
}
