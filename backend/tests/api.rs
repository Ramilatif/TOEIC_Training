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
    // Les questions d'un même texte sont servies ensemble (4 blancs par texte).
    assert!(!qs.is_empty() && qs.len() % 4 == 0);
    for group in qs.chunks(4) {
        assert!(group
            .iter()
            .all(|q| q["passage"]["id"] == group[0]["passage"]["id"]));
    }
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
async fn examen_complet_respecte_la_repartition_officielle() {
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

    let mut per_part = std::collections::BTreeMap::new();
    for q in exam["questions"].as_array().unwrap() {
        *per_part.entry(q["part"].as_i64().unwrap()).or_insert(0) += 1;
    }
    // Part 1 n'apparaît qu'une fois les photos déposées dans content/images/part1/.
    let expected = [(2, 25), (3, 39), (4, 30), (5, 30), (6, 16), (7, 54)];
    for (part, n) in expected {
        assert_eq!(per_part.get(&part).copied().unwrap_or(0), n, "Part {part}");
    }
    assert!(per_part.get(&1).copied().unwrap_or(0) <= 6);
}

#[tokio::test]
async fn statistiques_et_revision_des_erreurs() {
    let app = setup().await;
    let (_, v) = call(
        &app,
        "POST",
        "/api/auth/login",
        None,
        json!({"pseudo": "hugo"}),
    )
    .await;
    let token = v["token"].as_str().unwrap();

    // Sans activité : pas d'estimation.
    let (s, o) = call(&app, "GET", "/api/stats/overview", Some(token), Value::Null).await;
    assert_eq!(s, StatusCode::OK);
    assert!(o["estimated"]["total"].is_null());

    // Une erreur volontaire (on lit la bonne réponse via une première réponse).
    let id = "p5-b01-001";
    let (_, a) = call(
        &app,
        "POST",
        &format!("/api/questions/{id}/answer"),
        Some(token),
        json!({"chosen_index": 0}),
    )
    .await;
    let right = a["correct_index"].as_i64().unwrap();
    let wrong = if right == 0 { 1 } else { 0 };
    if right == 0 {
        // La première tentative était juste : on enregistre une erreur ensuite.
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        call(
            &app,
            "POST",
            &format!("/api/questions/{id}/answer"),
            Some(token),
            json!({"chosen_index": wrong}),
        )
        .await;
    }
    let (_, m) = call(
        &app,
        "GET",
        "/api/review/mistakes",
        Some(token),
        Value::Null,
    )
    .await;
    assert_eq!(m["total"], 1);
    assert_eq!(m["questions"][0]["id"], id);

    let (_, o) = call(&app, "GET", "/api/stats/overview", Some(token), Value::Null).await;
    assert_eq!(o["questions_answered"], 1);
    assert_eq!(o["parts_covered"], 1);
    assert!(o["estimated"]["reading"].as_i64().unwrap() >= 5);
    assert!(o["estimated"]["total"].is_null());

    let (_, cats) = call(
        &app,
        "GET",
        "/api/stats/by-category",
        Some(token),
        Value::Null,
    )
    .await;
    assert_eq!(cats[0]["category"], "gram-verb-tenses");
    assert_eq!(cats[0]["accuracy"], 0.0);

    // Rejouer l'erreur correctement la fait sortir de la liste.
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let (s, _) = call(
        &app,
        "POST",
        &format!("/api/questions/{id}/answer"),
        Some(token),
        json!({"chosen_index": right, "mode": "review"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (_, m) = call(
        &app,
        "GET",
        "/api/review/mistakes",
        Some(token),
        Value::Null,
    )
    .await;
    assert_eq!(m["total"], 0);
}

#[tokio::test]
async fn cours_lecture_quiz_et_progression() {
    let app = setup().await;
    let (_, v) = call(
        &app,
        "POST",
        "/api/auth/login",
        None,
        json!({"pseudo": "ines"}),
    )
    .await;
    let token = v["token"].as_str().unwrap();

    let (s, list) = call(&app, "GET", "/api/lessons", Some(token), Value::Null).await;
    assert_eq!(s, StatusCode::OK);
    let list = list.as_array().unwrap();
    assert!(!list.is_empty());
    let id = list[0]["id"].as_str().unwrap().to_string();
    assert_eq!(list[0]["read"], false);

    let (_, detail) = call(
        &app,
        "GET",
        &format!("/api/lessons/{id}"),
        Some(token),
        Value::Null,
    )
    .await;
    let quiz = detail["quiz"].as_array().unwrap();
    assert!((5..=10).contains(&quiz.len()));
    assert!(quiz[0].get("correct_index").is_none());
    assert!(detail["body_fr_md"].as_str().unwrap().len() > 100);

    // Un quiz raté ne valide pas la leçon, mais la marque comme lue.
    let (_, r) = call(
        &app,
        "POST",
        &format!("/api/lessons/{id}/quiz"),
        Some(token),
        json!({"answers": {}}),
    )
    .await;
    assert_eq!(r["passed"], false);
    let (_, p) = call(
        &app,
        "GET",
        "/api/lessons/progress",
        Some(token),
        Value::Null,
    )
    .await;
    let m = p
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["type"] == list[0]["type"])
        .unwrap();
    assert_eq!(m["read"], 1);
    assert_eq!(m["quiz_passed"], 0);

    // Toutes les bonnes réponses : leçon validée.
    let answers: serde_json::Map<String, Value> = r["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            (
                i["question_id"].as_str().unwrap().to_string(),
                i["correct_index"].clone(),
            )
        })
        .collect();
    let (_, r2) = call(
        &app,
        "POST",
        &format!("/api/lessons/{id}/quiz"),
        Some(token),
        json!({ "answers": answers }),
    )
    .await;
    assert_eq!(r2["passed"], true);
    assert_eq!(r2["percent"], 100);
    let (_, list2) = call(&app, "GET", "/api/lessons", Some(token), Value::Null).await;
    assert_eq!(list2[0]["quiz_passed"], true);
    assert_eq!(list2[0]["best_quiz_score"], 100);

    // La correction d'une question de la catégorie renvoie le lien vers la leçon.
    let (_, a) = call(
        &app,
        "POST",
        "/api/questions/p5-b01-001/answer",
        Some(token),
        json!({"chosen_index": 0}),
    )
    .await;
    assert!(a["lesson_id"].is_string(), "leçon liée à gram-verb-tenses");
}

#[tokio::test]
async fn flashcards_sm2_de_bout_en_bout() {
    let app = setup().await;
    let (_, v) = call(
        &app,
        "POST",
        "/api/auth/login",
        None,
        json!({"pseudo": "jade"}),
    )
    .await;
    let token = v["token"].as_str().unwrap();

    // Ajout depuis la leçon : tout le vocabulaire, sans doublon à la seconde demande.
    let (_, r) = call(
        &app,
        "POST",
        "/api/cards/from-lesson/vocab-office",
        Some(token),
        json!({}),
    )
    .await;
    assert_eq!(r["added"], 12);
    let (_, r) = call(
        &app,
        "POST",
        "/api/cards/from-lesson/vocab-office",
        Some(token),
        json!({}),
    )
    .await;
    assert_eq!(
        (r["added"].as_i64(), r["skipped"].as_i64()),
        (Some(0), Some(12))
    );

    // Ajout manuel et depuis une question.
    let (_, r) = call(
        &app,
        "POST",
        "/api/cards",
        Some(token),
        json!({"term": "on hold", "translation": "en attente"}),
    )
    .await;
    assert_eq!(r["added"], 1);
    let (_, r) = call(
        &app,
        "POST",
        "/api/cards/from-question/p5-b01-003",
        Some(token),
        Value::Null,
    )
    .await;
    assert_eq!(r["added"], 2);

    // Toutes les cartes neuves sont dues aujourd'hui.
    let (_, due) = call(
        &app,
        "GET",
        "/api/cards/due?limit=5",
        Some(token),
        Value::Null,
    )
    .await;
    assert_eq!(due["total"], 15);
    assert_eq!(due["cards"].as_array().unwrap().len(), 5);

    // Révision : bonne réponse => intervalle 1 jour, plus due aujourd'hui.
    let id = due["cards"][0]["id"].as_str().unwrap().to_string();
    let (s, card) = call(
        &app,
        "POST",
        &format!("/api/cards/{id}/review"),
        Some(token),
        json!({"quality": 4}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(card["interval_days"], 1);
    assert_eq!(card["repetitions"], 1);
    let (_, due) = call(&app, "GET", "/api/cards/due", Some(token), Value::Null).await;
    assert_eq!(due["total"], 14);

    // Qualité invalide et carte d'un autre utilisateur.
    let (s, _) = call(
        &app,
        "POST",
        &format!("/api/cards/{id}/review"),
        Some(token),
        json!({"quality": 7}),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (_, v2) = call(
        &app,
        "POST",
        "/api/auth/login",
        None,
        json!({"pseudo": "kim"}),
    )
    .await;
    let t2 = v2["token"].as_str().unwrap();
    let (s, _) = call(
        &app,
        "POST",
        &format!("/api/cards/{id}/review"),
        Some(t2),
        json!({"quality": 4}),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    let (s, _) = call(
        &app,
        "DELETE",
        &format!("/api/cards/{id}"),
        Some(token),
        Value::Null,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
}

#[tokio::test]
async fn planning_genere_et_suivi() {
    let app = setup().await;
    let (_, v) = call(
        &app,
        "POST",
        "/api/auth/login",
        None,
        json!({"pseudo": "leo"}),
    )
    .await;
    let token = v["token"].as_str().unwrap();

    // Sans date d'examen : refus explicite.
    let (s, _) = call(&app, "POST", "/api/plan", Some(token), Value::Null).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (_, p) = call(&app, "GET", "/api/plan", Some(token), Value::Null).await;
    assert!(p["plan"].is_null());

    let exam = (chrono::Utc::now().date_naive() + chrono::Duration::days(30)).to_string();
    call(
        &app,
        "PATCH",
        "/api/me",
        Some(token),
        json!({"exam_date": exam, "daily_minutes": 45}),
    )
    .await;
    let (s, p) = call(&app, "POST", "/api/plan", Some(token), Value::Null).await;
    assert_eq!(s, StatusCode::OK);
    let items = p["items"].as_array().unwrap();
    assert!(items.len() > 30);
    assert!(items.iter().any(|i| i["kind"] == "lesson"));
    assert!(items.iter().all(|i| i["minutes"].as_i64().unwrap() <= 45));

    // Cocher un élément, puis régénérer : l'élément fait est conservé sans doublon.
    let first = items[0]["id"].as_str().unwrap().to_string();
    let (s, _) = call(
        &app,
        "POST",
        &format!("/api/plan/items/{first}/done"),
        Some(token),
        json!({"done": true}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (_, p2) = call(&app, "POST", "/api/plan", Some(token), Value::Null).await;
    let items2 = p2["items"].as_array().unwrap();
    assert!(items2
        .iter()
        .any(|i| i["id"] == first.as_str() && i["done"] == true));
    assert_eq!(items2.len(), items.len());

    // Un autre utilisateur ne peut pas cocher cet élément.
    let (_, v2) = call(
        &app,
        "POST",
        "/api/auth/login",
        None,
        json!({"pseudo": "mia"}),
    )
    .await;
    let (s, _) = call(
        &app,
        "POST",
        &format!("/api/plan/items/{first}/done"),
        Some(v2["token"].as_str().unwrap()),
        json!({"done": true}),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}
