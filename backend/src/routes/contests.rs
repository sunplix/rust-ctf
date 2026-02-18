use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::header,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use tokio::fs;
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
    description: String,
    poster_url: Option<String>,
    status: String,
    scoring_mode: String,
    dynamic_decay: i32,
    latest_announcement_title: Option<String>,
    latest_announcement_content: Option<String>,
    latest_announcement_published_at: Option<DateTime<Utc>>,
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

#[derive(Debug, FromRow)]
struct ContestPosterAccessRow {
    visibility: String,
    status: String,
    poster_storage_path: Option<String>,
    poster_content_type: Option<String>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/contests", get(list_contests))
        .route("/contests/{contest_id}/poster", get(get_contest_poster))
        .route(
            "/contests/{contest_id}/challenges",
            get(list_contest_challenges),
        )
        .route(
            "/contests/{contest_id}/announcements",
            get(list_contest_announcements),
        )
}

async fn list_contests(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<Vec<ContestListItem>>> {
    let contests = sqlx::query_as::<_, ContestListItem>(
        "SELECT c.id,
                c.title,
                c.slug,
                c.description,
                CASE
                    WHEN c.poster_storage_path IS NULL OR c.poster_storage_path = '' THEN NULL
                    ELSE '/api/v1/contests/' || c.id::text || '/poster'
                END AS poster_url,
                c.status,
                c.scoring_mode,
                c.dynamic_decay,
                latest_announcement.title AS latest_announcement_title,
                latest_announcement.content AS latest_announcement_content,
                COALESCE(latest_announcement.published_at, latest_announcement.created_at) AS latest_announcement_published_at,
                c.start_at,
                c.end_at
         FROM contests c
         LEFT JOIN LATERAL (
             SELECT a.title, a.content, a.published_at, a.created_at
             FROM contest_announcements a
             WHERE a.contest_id = c.id
               AND a.is_published = TRUE
               AND (a.published_at IS NULL OR a.published_at <= NOW())
             ORDER BY a.is_pinned DESC, COALESCE(a.published_at, a.created_at) DESC, a.created_at DESC
             LIMIT 1
         ) AS latest_announcement ON TRUE
         WHERE c.visibility = 'public'
           AND c.status IN ('scheduled', 'running', 'ended')
         ORDER BY CASE c.status
                      WHEN 'running' THEN 0
                      WHEN 'scheduled' THEN 1
                      ELSE 2
                  END ASC,
                  CASE
                      WHEN c.status = 'running' THEN c.end_at
                      ELSE c.start_at
                  END ASC,
                  c.start_at ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(contests))
}

async fn get_contest_poster(
    State(state): State<Arc<AppState>>,
    Path(contest_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let row = sqlx::query_as::<_, ContestPosterAccessRow>(
        "SELECT visibility, status, poster_storage_path, poster_content_type
         FROM contests
         WHERE id = $1
         LIMIT 1",
    )
    .bind(contest_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("contest not found".to_string()))?;

    let public_status =
        row.status == "scheduled" || row.status == "running" || row.status == "ended";
    if row.visibility != "public" || !public_status {
        return Err(AppError::Forbidden);
    }

    let storage_path = row
        .poster_storage_path
        .ok_or(AppError::BadRequest("contest poster not found".to_string()))?;

    let content_type = row
        .poster_content_type
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "application/octet-stream".to_string());

    let bytes = fs::read(&storage_path).await.map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            AppError::BadRequest("contest poster not found".to_string())
        } else {
            AppError::internal(err)
        }
    })?;

    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "public, max-age=60".to_string()),
        ],
        bytes,
    ))
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
