use std::sync::Arc;
use toeic_backend::{app, config::Config, content, db, AppState};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=warn".into()),
        )
        .init();

    let config = Config::from_env();
    let pool = db::connect(&config.database_url).await?;
    content::import_all(&pool, &config.content_dir).await?;

    let images_dir = config.content_dir.join("images");
    let mut router = app(AppState {
        pool,
        config: Arc::new(config.clone()),
    })
    .nest_service("/media", ServeDir::new(images_dir));
    if let Some(origin) = &config.cors_origin {
        let cors = CorsLayer::new()
            .allow_origin(origin.parse::<axum::http::HeaderValue>()?)
            .allow_headers(Any)
            .allow_methods(Any);
        router = router.layer(cors);
    }

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    tracing::info!("API sur http://{}", config.bind_addr);
    axum::serve(listener, router).await?;
    Ok(())
}
