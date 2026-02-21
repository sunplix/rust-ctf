use std::{path::PathBuf, sync::Arc};

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

#[derive(Debug, Serialize)]
struct ContestChallengeAttachmentItem {
    id: Uuid,
    challenge_id: Uuid,
    filename: String,
    content_type: String,
    size_bytes: i64,
    created_at: DateTime<Utc>,
    download_url: String,
}

#[derive(Debug, FromRow)]
struct ContestChallengeAttachmentRow {
    id: Uuid,
    challenge_id: Uuid,
    filename: String,
    content_type: String,
    size_bytes: i64,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct ContestChallengeAttachmentFileRow {
    filename: String,
    content_type: String,
    storage_path: String,
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
struct ContestChallengeAccessRow {
    contest_status: String,
    challenge_visible: bool,
    release_at: Option<DateTime<Utc>>,
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
        .route(
            "/contests/{contest_id}/challenges/{challenge_id}/attachments",
            get(list_contest_challenge_attachments),
        )
        .route(
            "/contests/{contest_id}/challenges/{challenge_id}/attachments/{attachment_id}",
            get(download_contest_challenge_attachment),
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

async fn list_contest_challenge_attachments(
    State(state): State<Arc<AppState>>,
    Path((contest_id, challenge_id)): Path<(Uuid, Uuid)>,
    current_user: AuthenticatedUser,
) -> AppResult<Json<Vec<ContestChallengeAttachmentItem>>> {
    ensure_contest_challenge_access(state.as_ref(), contest_id, challenge_id, &current_user).await?;

    let rows = sqlx::query_as::<_, ContestChallengeAttachmentRow>(
        "SELECT id,
                challenge_id,
                filename,
                content_type,
                size_bytes,
                created_at
         FROM challenge_attachments
         WHERE challenge_id = $1
         ORDER BY created_at DESC
         LIMIT 200",
    )
    .bind(challenge_id)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    let attachments = rows
        .into_iter()
        .map(|item| ContestChallengeAttachmentItem {
            id: item.id,
            challenge_id: item.challenge_id,
            filename: item.filename,
            content_type: item.content_type,
            size_bytes: item.size_bytes,
            created_at: item.created_at,
            download_url: format!(
                "/api/v1/contests/{}/challenges/{}/attachments/{}",
                contest_id, challenge_id, item.id
            ),
        })
        .collect();

    Ok(Json(attachments))
}

async fn download_contest_challenge_attachment(
    State(state): State<Arc<AppState>>,
    Path((contest_id, challenge_id, attachment_id)): Path<(Uuid, Uuid, Uuid)>,
    current_user: AuthenticatedUser,
) -> AppResult<impl IntoResponse> {
    ensure_contest_challenge_access(state.as_ref(), contest_id, challenge_id, &current_user).await?;

    let row = sqlx::query_as::<_, ContestChallengeAttachmentFileRow>(
        "SELECT filename, content_type, storage_path
         FROM challenge_attachments
         WHERE id = $1
           AND challenge_id = $2
         LIMIT 1",
    )
    .bind(attachment_id)
    .bind(challenge_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest(
        "challenge attachment not found".to_string(),
    ))?;

    let resolved_path =
        resolve_challenge_attachment_storage_path(state.as_ref(), challenge_id, &row.storage_path);
    let bytes = fs::read(&resolved_path).await.map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            AppError::BadRequest("challenge attachment file missing".to_string())
        } else {
            AppError::internal(err)
        }
    })?;

    let content_type = normalize_content_type(&row.content_type);
    let disposition = build_download_disposition(&row.filename);

    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CONTENT_DISPOSITION, disposition),
            (header::CACHE_CONTROL, "private, max-age=60".to_string()),
        ],
        bytes,
    ))
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

async fn ensure_contest_challenge_access(
    state: &AppState,
    contest_id: Uuid,
    challenge_id: Uuid,
    current_user: &AuthenticatedUser,
) -> AppResult<()> {
    ensure_contest_access(state, contest_id, current_user).await?;

    let row = sqlx::query_as::<_, ContestChallengeAccessRow>(
        "SELECT ct.status AS contest_status,
                c.is_visible AS challenge_visible,
                cc.release_at
         FROM contest_challenges cc
         JOIN contests ct ON ct.id = cc.contest_id
         JOIN challenges c ON c.id = cc.challenge_id
         WHERE cc.contest_id = $1
           AND cc.challenge_id = $2
         LIMIT 1",
    )
    .bind(contest_id)
    .bind(challenge_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest(
        "challenge is not included in this contest".to_string(),
    ))?;

    let is_privileged = current_user.role == "admin" || current_user.role == "judge";
    if !row.challenge_visible && !is_privileged {
        return Err(AppError::BadRequest(
            "challenge runtime is not visible".to_string(),
        ));
    }

    if row.contest_status != "running" && row.contest_status != "ended" && !is_privileged {
        return Err(AppError::BadRequest(
            "contest challenge is not publicly available".to_string(),
        ));
    }

    if let Some(release_at) = row.release_at {
        if release_at > Utc::now() && !is_privileged {
            return Err(AppError::BadRequest(
                "challenge runtime has not been released yet".to_string(),
            ));
        }
    }

    Ok(())
}

fn normalize_content_type(input: &str) -> String {
    let value = input.trim();
    if value.is_empty() {
        "application/octet-stream".to_string()
    } else {
        value.to_string()
    }
}

fn build_download_disposition(filename: &str) -> String {
    let sanitized = filename
        .trim()
        .chars()
        .map(|ch| {
            if ch == '"' || ch == '\\' || ch == '\r' || ch == '\n' || ch.is_control() {
                '_'
            } else {
                ch
            }
        })
        .collect::<String>();
    let fallback = if sanitized.is_empty() {
        "attachment.bin".to_string()
    } else {
        sanitized
    };
    format!("attachment; filename=\"{}\"", fallback)
}

fn challenge_attachments_dir(state: &AppState, challenge_id: Uuid) -> PathBuf {
    PathBuf::from(&state.config.instance_runtime_root)
        .join("_challenge_files")
        .join(challenge_id.to_string())
}

fn resolve_challenge_attachment_storage_path(
    state: &AppState,
    challenge_id: Uuid,
    storage_path: &str,
) -> PathBuf {
    let raw = storage_path.trim();
    if raw.is_empty() {
        return challenge_attachments_dir(state, challenge_id).join("attachment.bin");
    }

    let original = PathBuf::from(raw);
    if original.exists() {
        return original;
    }

    let runtime_root = PathBuf::from(&state.config.instance_runtime_root);
    if !original.is_absolute() {
        let rooted = runtime_root.join(&original);
        if rooted.exists() {
            return rooted;
        }
    }

    if let Some(name) = original.file_name() {
        let fallback = challenge_attachments_dir(state, challenge_id).join(name);
        if fallback.exists() {
            return fallback;
        }
    }

    if original.is_absolute() {
        original
    } else {
        runtime_root.join(original)
    }
}
