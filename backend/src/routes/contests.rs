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
    scoring_mode: String,
    dynamic_decay: i32,
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

#[derive(Debug, Serialize, FromRow)]
struct ContestAnnouncementItem {
    id: Uuid,
    title: String,
    content: String,
    is_pinned: bool,
    published_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct ContestAccessRow {
    visibility: String,
    status: String,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/contests", get(list_contests))
        .route("/contests/{contest_id}/challenges", get(list_contest_challenges))
        .route(
            "/contests/{contest_id}/announcements",
            get(list_contest_announcements),
        )
}

async fn list_contests(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<Vec<ContestListItem>>> {
    let contests = sqlx::query_as::<_, ContestListItem>(
        "SELECT id, title, slug, status, scoring_mode, dynamic_decay, start_at, end_at
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
    current_user: AuthenticatedUser,
) -> AppResult<Json<Vec<ContestChallengeItem>>> {
    ensure_contest_access(state.as_ref(), contest_id, &current_user).await?;

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

async fn list_contest_announcements(
    State(state): State<Arc<AppState>>,
    Path(contest_id): Path<Uuid>,
    current_user: AuthenticatedUser,
) -> AppResult<Json<Vec<ContestAnnouncementItem>>> {
    ensure_contest_access(state.as_ref(), contest_id, &current_user).await?;

    let rows = sqlx::query_as::<_, ContestAnnouncementItem>(
        "SELECT id,
                title,
                content,
                is_pinned,
                published_at,
                created_at
         FROM contest_announcements
         WHERE contest_id = $1
           AND is_published = TRUE
           AND (published_at IS NULL OR published_at <= NOW())
         ORDER BY is_pinned DESC, COALESCE(published_at, created_at) DESC, created_at DESC",
    )
    .bind(contest_id)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(rows))
}

async fn ensure_contest_access(
    state: &AppState,
    contest_id: Uuid,
    current_user: &AuthenticatedUser,
) -> AppResult<()> {
    let contest = sqlx::query_as::<_, ContestAccessRow>(
        "SELECT visibility, status
         FROM contests
         WHERE id = $1
         LIMIT 1",
    )
    .bind(contest_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("contest not found".to_string()))?;

    let is_privileged = current_user.role == "admin" || current_user.role == "judge";
    if contest.visibility == "private" && !is_privileged {
        return Err(AppError::Forbidden);
    }

    if (contest.status == "draft" || contest.status == "archived") && !is_privileged {
        return Err(AppError::Forbidden);
    }

    Ok(())
}
