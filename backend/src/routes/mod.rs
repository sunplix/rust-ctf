mod auth;
mod contests;
mod health;
mod scoreboard;
mod submissions;

use std::sync::Arc;

use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .nest("/api/v1", health::router())
        .nest("/api/v1", auth::router())
        .nest("/api/v1", contests::router())
        .nest("/api/v1", scoreboard::router())
        .nest("/api/v1", submissions::router())
}
