use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    auth::AuthenticatedUser,
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Debug, Serialize, FromRow)]
struct ContestListItem {
    id: Uuid,
    title: String,
    slug: String,
    status: String,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
struct ContestChallengeItem {
    id: Uuid,
    title: String,
    category: String,
    difficulty: String,
    challenge_type: String,
    static_score: i32,
    release_at: Option<DateTime<Utc>>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/contests", get(list_contests)).route(
        "/contests/{contest_id}/challenges",
        get(list_contest_challenges),
    )
}

async fn list_contests(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<Vec<ContestListItem>>> {
    let contests = sqlx::query_as::<_, ContestListItem>(
        "SELECT id, title, slug, status, start_at, end_at
         FROM contests
         WHERE visibility = 'public' AND status IN ('scheduled', 'running', 'ended')
         ORDER BY start_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(contests))
}

async fn list_contest_challenges(
    State(state): State<Arc<AppState>>,
    Path(contest_id): Path<Uuid>,
    _current_user: AuthenticatedUser,
) -> AppResult<Json<Vec<ContestChallengeItem>>> {
    let challenge_items = sqlx::query_as::<_, ContestChallengeItem>(
        "SELECT c.id, c.title, c.category, c.difficulty, c.challenge_type, c.static_score, cc.release_at
         FROM contest_challenges cc
         JOIN challenges c ON c.id = cc.challenge_id
         JOIN contests ct ON ct.id = cc.contest_id
         WHERE cc.contest_id = $1
           AND c.is_visible = TRUE
           AND ct.status IN ('running', 'ended')
           AND (cc.release_at IS NULL OR cc.release_at <= NOW())
         ORDER BY cc.sort_order ASC, c.created_at ASC",
    )
    .bind(contest_id)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(challenge_items))
}
