pub(crate) mod admin;
pub(crate) mod contest_access;
mod auth;
mod contests;
mod health;
pub(crate) mod instances;
mod scoreboard;
mod site;
mod submissions;
mod teams;

use std::sync::Arc;

use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .nest("/api/v1", health::router())
        .nest("/api/v1", auth::router())
        .nest("/api/v1", admin::router())
        .nest("/api/v1", contests::router())
        .nest("/api/v1", instances::router())
        .nest("/api/v1", scoreboard::router())
        .nest("/api/v1", site::router())
        .nest("/api/v1", submissions::router())
        .nest("/api/v1", teams::router())
}
