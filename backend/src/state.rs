use crate::config::AppConfig;
use anyhow::Context;
use redis::aio::ConnectionManager;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing::warn;

pub struct AppState {
    pub config: AppConfig,
    pub db: PgPool,
    pub redis: ConnectionManager,
}

impl AppState {
    pub async fn new(config: AppConfig) -> anyhow::Result<Self> {
        if config.jwt_secret.len() < 32 {
            warn!("JWT_SECRET is shorter than 32 characters; use a stronger secret");
        }

        let db = PgPoolOptions::new()
            .max_connections(20)
            .connect(&config.database_url)
            .await
            .context("failed to connect postgres")?;

        sqlx::migrate!("./migrations")
            .run(&db)
            .await
            .context("failed to run database migrations")?;

        let redis_client =
            redis::Client::open(config.redis_url.clone()).context("invalid redis url")?;
        let redis = ConnectionManager::new(redis_client)
            .await
            .context("failed to connect redis")?;

        Ok(Self { config, db, redis })
    }
}
