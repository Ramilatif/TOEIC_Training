//! Configuration lue depuis les variables d'environnement (aucun secret en dur).
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub content_dir: PathBuf,
    pub bind_addr: String,
    pub session_ttl_hours: i64,
    pub cors_origin: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let get = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.to_string());
        Config {
            database_url: get("DATABASE_URL", "sqlite://data/toeic.db?mode=rwc"),
            content_dir: PathBuf::from(get("CONTENT_DIR", "../content")),
            bind_addr: get("BIND_ADDR", "0.0.0.0:3000"),
            session_ttl_hours: get("SESSION_TTL_HOURS", "720").parse().unwrap_or(720),
            cors_origin: std::env::var("CORS_ORIGIN").ok().filter(|s| !s.is_empty()),
        }
    }
}
