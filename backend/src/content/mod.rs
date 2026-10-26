//! Import du contenu (JSON de questions, catégories, crédits d'images) en base.
//! L'import est idempotent : relancé à chaque démarrage (`ON CONFLICT DO UPDATE`).
pub mod model;

use anyhow::{bail, Context};
use model::*;
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::path::Path;

/// Valide un lot de questions. Retourne la première erreur avec l'id fautif.
pub fn validate_batch(
    b: &Batch,
    categories: &HashSet<String>,
    images: &HashSet<String>,
) -> anyhow::Result<()> {
    let passage_ids: HashSet<&str> = b.passages.iter().map(|p| p.id.as_str()).collect();
    let mut seen = HashSet::new();
    for p in &b.passages {
        if let Some(img) = &p.image {
            if !images.contains(img) {
                bail!("passage {}: image inconnue '{}'", p.id, img);
            }
        }
    }
    for q in &b.questions {
        let id = &q.id;
        if !seen.insert(id.as_str()) {
            bail!("question {id}: id en double");
        }
        if !(1..=7).contains(&q.part) {
            bail!("question {id}: partie invalide {}", q.part);
        }
        if !(1..=3).contains(&q.difficulty) {
            bail!("question {id}: difficulté hors 1-3");
        }
        if !categories.contains(&q.category) {
            bail!("question {id}: catégorie inconnue '{}'", q.category);
        }
        let expected = if q.part == 2 { 3 } else { 4 };
        if q.choices.len() != expected {
            bail!(
                "question {id}: {} choix au lieu de {expected}",
                q.choices.len()
            );
        }
        if q.correct >= q.choices.len() {
            bail!("question {id}: index de bonne réponse hors bornes");
        }
        if let Some(p) = &q.passage {
            if !passage_ids.contains(p.as_str()) {
                bail!("question {id}: passage inconnu '{p}'");
            }
        }
        for (lang, e) in [("fr", &q.explanation.fr), ("en", &q.explanation.en)] {
            if e.rule.trim().is_empty() || e.example.trim().is_empty() {
                bail!("question {id}: explication {lang} incomplète (règle/exemple)");
            }
            if e.why_wrong.len() != q.choices.len() - 1 {
                bail!(
                    "question {id}: why_wrong {lang} doit couvrir les {} mauvaises réponses",
                    q.choices.len() - 1
                );
            }
        }
    }
    Ok(())
}

pub async fn import_all(pool: &SqlitePool, dir: &Path) -> anyhow::Result<()> {
    let cats: Vec<CategoryDef> = read_json(&dir.join("categories.json"))?;
    let cat_ids: HashSet<String> = cats.iter().map(|c| c.id.clone()).collect();
    for c in &cats {
        sqlx::query(
            "INSERT INTO categories (id, kind, label_fr, label_en) VALUES ($1,$2,$3,$4)
             ON CONFLICT(id) DO UPDATE SET kind=excluded.kind, label_fr=excluded.label_fr, label_en=excluded.label_en",
        )
        .bind(&c.id)
        .bind(&c.kind)
        .bind(&c.fr)
        .bind(&c.en)
        .execute(pool)
        .await?;
    }

    let credits_path = dir.join("images/part1/credits.json");
    let credits: Vec<ImageCredit> = if credits_path.exists() {
        read_json(&credits_path)?
    } else {
        vec![]
    };
    let mut image_ids = HashSet::new();
    for i in &credits {
        // Une image listée mais absente du disque n'est pas importée (photo pas encore déposée).
        if !dir.join("images/part1").join(&i.file).exists() {
            tracing::warn!("image {} absente du dossier, ignorée", i.file);
            continue;
        }
        image_ids.insert(i.id.clone());
        sqlx::query(
            "INSERT INTO images (id, path, credit, source_url, alt_text) VALUES ($1,$2,$3,$4,$5)
             ON CONFLICT(id) DO UPDATE SET path=excluded.path, credit=excluded.credit,
               source_url=excluded.source_url, alt_text=excluded.alt_text",
        )
        .bind(&i.id)
        .bind(&i.file)
        .bind(&i.credit)
        .bind(&i.source_url)
        .bind(&i.alt)
        .execute(pool)
        .await?;
    }

    let qdir = dir.join("questions");
    if qdir.exists() {
        let mut files: Vec<_> = std::fs::read_dir(&qdir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        files.sort();
        for f in files {
            let batch: Batch = read_json(&f)?;
            validate_batch(&batch, &cat_ids, &image_ids)
                .with_context(|| format!("lot invalide : {}", f.display()))?;
            import_batch(pool, &batch).await?;
            tracing::info!(
                "lot {} importé ({} questions)",
                batch.batch,
                batch.questions.len()
            );
        }
    }
    Ok(())
}

async fn import_batch(pool: &SqlitePool, b: &Batch) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    for p in &b.passages {
        sqlx::query(
            "INSERT INTO passages (id, part, kind, title, body_text, transcript, image_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7)
             ON CONFLICT(id) DO UPDATE SET part=excluded.part, kind=excluded.kind, title=excluded.title,
               body_text=excluded.body_text, transcript=excluded.transcript, image_id=excluded.image_id",
        )
        .bind(&p.id)
        .bind(b.part)
        .bind(&p.kind)
        .bind(&p.title)
        .bind(&p.body)
        .bind(&p.transcript)
        .bind(&p.image)
        .execute(&mut *tx)
        .await?;
    }
    for q in &b.questions {
        sqlx::query(
            "INSERT INTO questions (id, part, difficulty, category_id, passage_id, position_in_passage, stem,
               choices_json, correct_index, explanation_fr, explanation_en, batch)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
             ON CONFLICT(id) DO UPDATE SET part=excluded.part, difficulty=excluded.difficulty,
               category_id=excluded.category_id, passage_id=excluded.passage_id,
               position_in_passage=excluded.position_in_passage, stem=excluded.stem,
               choices_json=excluded.choices_json, correct_index=excluded.correct_index,
               explanation_fr=excluded.explanation_fr, explanation_en=excluded.explanation_en,
               batch=excluded.batch",
        )
        .bind(&q.id)
        .bind(q.part)
        .bind(q.difficulty)
        .bind(&q.category)
        .bind(&q.passage)
        .bind(q.position.unwrap_or(1))
        .bind(&q.stem)
        .bind(serde_json::to_string(&q.choices)?)
        .bind(q.correct as i64)
        .bind(serde_json::to_string(&q.explanation.fr)?)
        .bind(serde_json::to_string(&q.explanation.en)?)
        .bind(&b.batch)
        .execute(&mut *tx)
        .await?;
        for v in &q.vocab {
            sqlx::query(
                "INSERT INTO question_vocab (question_id, term, translation_fr) VALUES ($1,$2,$3)
                 ON CONFLICT(question_id, term) DO UPDATE SET translation_fr=excluded.translation_fr",
            )
            .bind(&q.id)
            .bind(&v.term)
            .bind(&v.fr)
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

fn read_json<T: serde::de::DeserializeOwned>(p: &Path) -> anyhow::Result<T> {
    let s = std::fs::read_to_string(p).with_context(|| format!("lecture de {}", p.display()))?;
    serde_json::from_str(&s).with_context(|| format!("JSON invalide : {}", p.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Batch {
        serde_json::from_str(
            r#"{
          "batch":"t","part":5,"passages":[],
          "questions":[{"id":"q1","part":5,"difficulty":1,"category":"gram-verb-tenses","stem":"x ___",
            "choices":["a","b","c","d"],"correct":1,
            "explanation":{
              "fr":{"rule":"r","why_wrong":["A","C","D"],"example":"e"},
              "en":{"rule":"r","why_wrong":["A","C","D"],"example":"e"}}}]}"#,
        )
        .unwrap()
    }
    fn cats() -> HashSet<String> {
        ["gram-verb-tenses".to_string()].into()
    }

    #[test]
    fn lot_valide() {
        assert!(validate_batch(&sample(), &cats(), &HashSet::new()).is_ok());
    }
    #[test]
    fn rejette_categorie_inconnue() {
        let mut b = sample();
        b.questions[0].category = "nope".into();
        assert!(validate_batch(&b, &cats(), &HashSet::new()).is_err());
    }
    #[test]
    fn rejette_index_hors_bornes() {
        let mut b = sample();
        b.questions[0].correct = 4;
        assert!(validate_batch(&b, &cats(), &HashSet::new()).is_err());
    }
    #[test]
    fn rejette_why_wrong_incomplet() {
        let mut b = sample();
        b.questions[0].explanation.en.why_wrong.pop();
        assert!(validate_batch(&b, &cats(), &HashSet::new()).is_err());
    }
}
