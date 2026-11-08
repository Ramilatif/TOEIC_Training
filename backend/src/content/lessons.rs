//! Leçons : Markdown avec front matter YAML, une section par langue.
//! Format documenté dans docs/ARCHITECTURE.md §6.
use anyhow::{bail, Context};
use serde::Deserialize;
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct LessonMeta {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub category: String,
    pub level: String,
    pub minutes: i64,
    pub title: Titles,
    #[serde(default)]
    pub vocab: Vec<LessonVocab>,
}

#[derive(Debug, Deserialize)]
pub struct Titles {
    pub fr: String,
    pub en: String,
}

#[derive(Debug, Deserialize)]
pub struct LessonVocab {
    pub term: String,
    pub fr: String,
    pub example: Option<String>,
}

#[derive(Debug)]
pub struct Lesson {
    pub meta: LessonMeta,
    pub body_fr: String,
    pub body_en: String,
}

const MARK_FR: &str = "<!-- lang:fr -->";
const MARK_EN: &str = "<!-- lang:en -->";

/// Découpe un fichier de leçon. Erreur explicite si le format n'est pas respecté.
pub fn parse_lesson(src: &str) -> anyhow::Result<Lesson> {
    let src = src.replace("\r\n", "\n");
    let rest = src
        .strip_prefix("---\n")
        .context("front matter manquant (le fichier doit commencer par ---)")?;
    let end = rest
        .find("\n---\n")
        .context("front matter non fermé (--- attendu)")?;
    let meta: LessonMeta = serde_yaml::from_str(&rest[..end]).context("front matter invalide")?;
    let body = &rest[end + 5..];

    let (fr_at, en_at) = (body.find(MARK_FR), body.find(MARK_EN));
    let (Some(fr_at), Some(en_at)) = (fr_at, en_at) else {
        bail!("les deux sections {MARK_FR} et {MARK_EN} sont obligatoires");
    };
    if fr_at > en_at {
        bail!("la section fr doit précéder la section en");
    }
    let body_fr = body[fr_at + MARK_FR.len()..en_at].trim().to_string();
    let body_en = body[en_at + MARK_EN.len()..].trim().to_string();
    if body_fr.is_empty() || body_en.is_empty() {
        bail!("section fr ou en vide");
    }
    Ok(Lesson {
        meta,
        body_fr,
        body_en,
    })
}

/// Valide les métadonnées d'une leçon.
pub fn validate_lesson(l: &Lesson, categories: &HashSet<String>) -> anyhow::Result<()> {
    let m = &l.meta;
    if !["grammar", "conjugation", "vocab", "method", "advanced"].contains(&m.kind.as_str()) {
        bail!("{}: type invalide '{}'", m.id, m.kind);
    }
    if !["beginner", "intermediate", "advanced"].contains(&m.level.as_str()) {
        bail!("{}: niveau invalide '{}'", m.id, m.level);
    }
    if !categories.contains(&m.category) {
        bail!("{}: catégorie inconnue '{}'", m.id, m.category);
    }
    if !(1..=120).contains(&m.minutes) {
        bail!("{}: durée hors 1-120 minutes", m.id);
    }
    if !m.id.starts_with(&format!("{}-", m.kind)) {
        bail!("{}: l'id doit commencer par '{}-'", m.id, m.kind);
    }
    Ok(())
}

pub async fn import_lessons(
    pool: &SqlitePool,
    dir: &Path,
    categories: &HashSet<String>,
) -> anyhow::Result<()> {
    let ldir = dir.join("lessons");
    if !ldir.exists() {
        return Ok(());
    }
    let mut files: Vec<_> = std::fs::read_dir(&ldir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    files.sort();
    for f in files {
        let src = std::fs::read_to_string(&f)?;
        let lesson =
            parse_lesson(&src).with_context(|| format!("leçon invalide : {}", f.display()))?;
        validate_lesson(&lesson, categories)
            .with_context(|| format!("leçon invalide : {}", f.display()))?;
        let m = &lesson.meta;

        let mut tx = pool.begin().await?;
        sqlx::query(
            "INSERT INTO lessons (id, type, category_id, level, minutes, title_fr, title_en, body_fr_md, body_en_md)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
             ON CONFLICT(id) DO UPDATE SET type=excluded.type, category_id=excluded.category_id,
               level=excluded.level, minutes=excluded.minutes, title_fr=excluded.title_fr,
               title_en=excluded.title_en, body_fr_md=excluded.body_fr_md, body_en_md=excluded.body_en_md",
        )
        .bind(&m.id)
        .bind(&m.kind)
        .bind(&m.category)
        .bind(&m.level)
        .bind(m.minutes)
        .bind(&m.title.fr)
        .bind(&m.title.en)
        .bind(&lesson.body_fr)
        .bind(&lesson.body_en)
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM lesson_vocab WHERE lesson_id = $1")
            .bind(&m.id)
            .execute(&mut *tx)
            .await?;
        for v in &m.vocab {
            sqlx::query(
                "INSERT INTO lesson_vocab (lesson_id, term, translation_fr, example) VALUES ($1,$2,$3,$4)
                 ON CONFLICT(lesson_id, term) DO NOTHING",
            )
            .bind(&m.id)
            .bind(&v.term)
            .bind(&v.fr)
            .bind(&v.example)
            .execute(&mut *tx)
            .await?;
        }
        // Le quiz d'une leçon = les questions du lot « lesson-<id> » (5 à 10 exigées).
        sqlx::query("DELETE FROM lesson_quiz_questions WHERE lesson_id = $1")
            .bind(&m.id)
            .execute(&mut *tx)
            .await?;
        let batch = format!("lesson-{}", m.id);
        let quiz: Vec<(String,)> =
            sqlx::query_as("SELECT id FROM questions WHERE batch = $1 ORDER BY id")
                .bind(&batch)
                .fetch_all(&mut *tx)
                .await?;
        if !(5..=10).contains(&quiz.len()) {
            bail!(
                "{}: le quiz doit compter 5 à 10 questions (lot '{}'), trouvé {}",
                m.id,
                batch,
                quiz.len()
            );
        }
        for (qid,) in quiz {
            sqlx::query(
                "INSERT INTO lesson_quiz_questions (lesson_id, question_id) VALUES ($1,$2)",
            )
            .bind(&m.id)
            .bind(qid)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        tracing::info!("leçon {} importée", m.id);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "---\nid: grammar-demo\ntype: grammar\ncategory: gram-verb-tenses\nlevel: intermediate\nminutes: 10\ntitle: { fr: \"Démo\", en: \"Demo\" }\nvocab:\n  - { term: deadline, fr: \"échéance\", example: \"Meet the deadline.\" }\n---\n<!-- lang:fr -->\n# Démo\nTexte.\n\n<!-- lang:en -->\n# Demo\nText.\n";

    fn cats() -> HashSet<String> {
        ["gram-verb-tenses".to_string()].into()
    }

    #[test]
    fn parse_ok() {
        let l = parse_lesson(SAMPLE).unwrap();
        assert_eq!(l.meta.id, "grammar-demo");
        assert_eq!(l.meta.vocab.len(), 1);
        assert!(l.body_fr.starts_with("# Démo"));
        assert!(l.body_en.starts_with("# Demo"));
        assert!(validate_lesson(&l, &cats()).is_ok());
    }

    #[test]
    fn accepte_crlf() {
        assert!(parse_lesson(&SAMPLE.replace('\n', "\r\n")).is_ok());
    }

    #[test]
    fn rejette_format_invalide() {
        assert!(parse_lesson("pas de front matter").is_err());
        assert!(parse_lesson(&SAMPLE.replace("<!-- lang:en -->", "")).is_err());
        assert!(parse_lesson(&SAMPLE.replace(
            "# Demo
Text.
",
            ""
        ))
        .is_err());
    }

    #[test]
    fn rejette_metadonnees_invalides() {
        let bad = |from: &str, to: &str| {
            let l = parse_lesson(&SAMPLE.replace(from, to)).unwrap();
            validate_lesson(&l, &cats()).is_err()
        };
        assert!(bad("type: grammar", "type: autre"));
        assert!(bad("level: intermediate", "level: expert"));
        assert!(bad("gram-verb-tenses", "inconnue"));
        assert!(bad("id: grammar-demo", "id: demo"));
        assert!(bad("minutes: 10", "minutes: 0"));
    }
}
