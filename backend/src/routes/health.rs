use std::sync::Arc;

use axum::{extract::State, routing::get, Json, Router};
use chrono::{SecondsFormat, Utc};
use serde::Serialize;

use crate::state::AppState;

#[derive(Debug, Serialize)]
struct Dependencies {
    database: bool,
    redis: bool,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
    now_utc: String,
    api_port: u16,
    dependencies: Dependencies,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/health", get(health))
}

async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let database_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .is_ok();

    let mut redis_conn = state.redis.clone();
    let redis_ok = redis::cmd("PING")
        .query_async::<String>(&mut redis_conn)
        .await
        .map(|pong| pong.eq_ignore_ascii_case("PONG"))
        .unwrap_or(false);

    Json(HealthResponse {
        status: if database_ok && redis_ok {
            "ok"
        } else {
            "degraded"
        },
        service: "rust-ctf-backend",
        version: env!("CARGO_PKG_VERSION"),
        now_utc: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        api_port: state.config.app_port,
        dependencies: Dependencies {
            database: database_ok,
            redis: redis_ok,
        },
    })
}
