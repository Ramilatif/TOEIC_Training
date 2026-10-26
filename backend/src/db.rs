//! Pool de connexions et migrations embarquées.
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;

pub async fn connect(url: &str) -> anyhow::Result<SqlitePool> {
    if let Some(path) = url.strip_prefix("sqlite://") {
        let path = path.split('?').next().unwrap_or(path);
        if !path.starts_with(':') {
            if let Some(dir) = std::path::Path::new(path).parent() {
                if !dir.as_os_str().is_empty() {
                    std::fs::create_dir_all(dir)?;
                }
            }
        }
    }
    let opts = SqliteConnectOptions::from_str(url)?
        .create_if_missing(true)
        .foreign_keys(true);
    // Une seule connexion pour une base en mémoire (sinon chaque connexion a sa propre base).
    let max = if url.contains(":memory:") { 1 } else { 5 };
    let pool = SqlitePoolOptions::new()
        .max_connections(max)
        .connect_with(opts)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
