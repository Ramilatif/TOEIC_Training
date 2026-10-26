//! Structures du format JSON documenté dans docs/ARCHITECTURE.md.
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CategoryDef {
    pub id: String,
    pub kind: String,
    pub fr: String,
    pub en: String,
}

#[derive(Deserialize)]
pub struct ImageCredit {
    pub id: String,
    pub file: String,
    pub credit: String,
    pub source_url: String,
    pub alt: String,
}

#[derive(Deserialize)]
pub struct Batch {
    pub batch: String,
    pub part: i64,
    #[serde(default)]
    pub passages: Vec<PassageDef>,
    pub questions: Vec<QuestionDef>,
}

#[derive(Deserialize)]
pub struct PassageDef {
    pub id: String,
    pub kind: String,
    pub title: Option<String>,
    pub body: Option<String>,
    pub transcript: Option<String>,
    pub image: Option<String>,
}

#[derive(Deserialize)]
pub struct QuestionDef {
    pub id: String,
    pub part: i64,
    pub difficulty: i64,
    pub category: String,
    pub passage: Option<String>,
    pub position: Option<i64>,
    pub stem: String,
    pub choices: Vec<String>,
    pub correct: usize,
    pub explanation: Explanations,
    #[serde(default)]
    pub vocab: Vec<VocabDef>,
}

#[derive(Deserialize)]
pub struct Explanations {
    pub fr: Explanation,
    pub en: Explanation,
}

/// Explication stockée telle quelle (JSON) et renvoyée au client après la réponse.
#[derive(Deserialize, Serialize, Clone)]
pub struct Explanation {
    pub rule: String,
    pub why_wrong: Vec<String>,
    pub example: String,
}

#[derive(Deserialize)]
pub struct VocabDef {
    pub term: String,
    pub fr: String,
}
