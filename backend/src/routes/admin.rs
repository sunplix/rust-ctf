use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{get, patch},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    auth::AuthenticatedUser,
    error::{AppError, AppResult},
    state::AppState,
};

const DIFFICULTY_ALLOWED: &[&str] = &["easy", "normal", "hard", "insane"];
const CHALLENGE_TYPE_ALLOWED: &[&str] = &["static", "dynamic", "internal"];
const FLAG_MODE_ALLOWED: &[&str] = &["static", "dynamic", "script"];
const CONTEST_STATUS_ALLOWED: &[&str] = &["draft", "scheduled", "running", "ended", "archived"];

#[derive(Debug, Serialize, FromRow)]
struct AdminChallengeItem {
    id: Uuid,
    title: String,
    slug: String,
    category: String,
    difficulty: String,
    static_score: i32,
    challenge_type: String,
    flag_mode: String,
    is_visible: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct CreateChallengeRequest {
    title: String,
    slug: String,
    category: String,
    difficulty: Option<String>,
    description: Option<String>,
    static_score: Option<i32>,
    min_score: Option<i32>,
    max_score: Option<i32>,
    challenge_type: Option<String>,
    flag_mode: Option<String>,
    flag_hash: Option<String>,
    compose_template: Option<String>,
    metadata: Option<Value>,
    is_visible: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct UpdateChallengeRequest {
    title: Option<String>,
    slug: Option<String>,
    category: Option<String>,
    difficulty: Option<String>,
    description: Option<String>,
    static_score: Option<i32>,
    challenge_type: Option<String>,
    flag_mode: Option<String>,
    flag_hash: Option<String>,
    compose_template: Option<String>,
    metadata: Option<Value>,
    is_visible: Option<bool>,
}

#[derive(Debug, Serialize, FromRow)]
struct AdminContestItem {
    id: Uuid,
    title: String,
    slug: String,
    visibility: String,
    status: String,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
    freeze_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct UpdateContestStatusRequest {
    status: String,
}

#[derive(Debug, Deserialize)]
struct AdminInstancesQuery {
    status: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize, FromRow)]
struct AdminInstanceItem {
    id: Uuid,
    contest_id: Uuid,
    contest_title: String,
    challenge_id: Uuid,
    challenge_title: String,
    team_id: Uuid,
    team_name: String,
    status: String,
    subnet: String,
    compose_project_name: String,
    entrypoint_url: String,
    started_at: Option<DateTime<Utc>>,
    expires_at: Option<DateTime<Utc>>,
    destroyed_at: Option<DateTime<Utc>>,
    last_heartbeat_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/admin/challenges",
            get(list_challenges).post(create_challenge),
        )
        .route("/admin/challenges/{challenge_id}", patch(update_challenge))
        .route("/admin/contests", get(list_contests))
        .route(
            "/admin/contests/{contest_id}/status",
            patch(update_contest_status),
        )
        .route("/admin/instances", get(list_instances))
}

async fn list_challenges(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
) -> AppResult<Json<Vec<AdminChallengeItem>>> {
    ensure_admin_or_judge(&current_user)?;

    let rows = sqlx::query_as::<_, AdminChallengeItem>(
        "SELECT id,
                title,
                slug,
                category,
                difficulty,
                static_score,
                challenge_type,
                flag_mode,
                is_visible,
                created_at,
                updated_at
         FROM challenges
         ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(rows))
}

async fn create_challenge(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Json(req): Json<CreateChallengeRequest>,
) -> AppResult<Json<AdminChallengeItem>> {
    ensure_admin_or_judge(&current_user)?;

    let title = trim_required(&req.title, "title")?;
    let slug = trim_required(&req.slug, "slug")?.to_lowercase();
    let category = trim_required(&req.category, "category")?.to_lowercase();
    let difficulty = normalize_with_allowed(
        req.difficulty.as_deref().unwrap_or("normal"),
        DIFFICULTY_ALLOWED,
        "difficulty",
    )?;
    let challenge_type = normalize_with_allowed(
        req.challenge_type.as_deref().unwrap_or("static"),
        CHALLENGE_TYPE_ALLOWED,
        "challenge_type",
    )?;
    let flag_mode = normalize_with_allowed(
        req.flag_mode.as_deref().unwrap_or("static"),
        FLAG_MODE_ALLOWED,
        "flag_mode",
    )?;

    let static_score = req.static_score.unwrap_or(100);
    if static_score <= 0 {
        return Err(AppError::BadRequest(
            "static_score must be greater than 0".to_string(),
        ));
    }

    let min_score = req.min_score.unwrap_or(50);
    let max_score = req.max_score.unwrap_or(500);
    if min_score < 0 || max_score < min_score {
        return Err(AppError::BadRequest(
            "min_score/max_score is invalid".to_string(),
        ));
    }

    let description = req.description.unwrap_or_default();
    let flag_hash = req.flag_hash.unwrap_or_default();
    let compose_template = req.compose_template;
    let metadata = req.metadata.unwrap_or(Value::Object(Default::default()));
    let is_visible = req.is_visible.unwrap_or(false);

    let row = sqlx::query_as::<_, AdminChallengeItem>(
        "INSERT INTO challenges (
            title,
            slug,
            category,
            difficulty,
            description,
            static_score,
            min_score,
            max_score,
            challenge_type,
            flag_mode,
            flag_hash,
            compose_template,
            metadata,
            is_visible,
            created_by
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
         RETURNING id,
                   title,
                   slug,
                   category,
                   difficulty,
                   static_score,
                   challenge_type,
                   flag_mode,
                   is_visible,
                   created_at,
                   updated_at",
    )
    .bind(title)
    .bind(slug)
    .bind(category)
    .bind(difficulty)
    .bind(description)
    .bind(static_score)
    .bind(min_score)
    .bind(max_score)
    .bind(challenge_type)
    .bind(flag_mode)
    .bind(flag_hash)
    .bind(compose_template)
    .bind(metadata)
    .bind(is_visible)
    .bind(current_user.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|err| {
        if is_unique_violation(&err) {
            AppError::Conflict("challenge slug already exists".to_string())
        } else {
            AppError::internal(err)
        }
    })?;

    Ok(Json(row))
}

async fn update_challenge(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(challenge_id): Path<Uuid>,
    Json(req): Json<UpdateChallengeRequest>,
) -> AppResult<Json<AdminChallengeItem>> {
    ensure_admin_or_judge(&current_user)?;

    if let Some(score) = req.static_score {
        if score <= 0 {
            return Err(AppError::BadRequest(
                "static_score must be greater than 0".to_string(),
            ));
        }
    }

    let normalized_difficulty = req
        .difficulty
        .as_deref()
        .map(|value| normalize_with_allowed(value, DIFFICULTY_ALLOWED, "difficulty"))
        .transpose()?;
    let normalized_challenge_type = req
        .challenge_type
        .as_deref()
        .map(|value| normalize_with_allowed(value, CHALLENGE_TYPE_ALLOWED, "challenge_type"))
        .transpose()?;
    let normalized_flag_mode = req
        .flag_mode
        .as_deref()
        .map(|value| normalize_with_allowed(value, FLAG_MODE_ALLOWED, "flag_mode"))
        .transpose()?;

    let title = req
        .title
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let slug = req
        .slug
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty());
    let category = req
        .category
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty());

    let row = sqlx::query_as::<_, AdminChallengeItem>(
        "UPDATE challenges
         SET title = COALESCE($2, title),
             slug = COALESCE($3, slug),
             category = COALESCE($4, category),
             difficulty = COALESCE($5, difficulty),
             description = COALESCE($6, description),
             static_score = COALESCE($7, static_score),
             challenge_type = COALESCE($8, challenge_type),
             flag_mode = COALESCE($9, flag_mode),
             flag_hash = COALESCE($10, flag_hash),
             compose_template = COALESCE($11, compose_template),
             metadata = COALESCE($12, metadata),
             is_visible = COALESCE($13, is_visible),
             updated_at = NOW()
         WHERE id = $1
         RETURNING id,
                   title,
                   slug,
                   category,
                   difficulty,
                   static_score,
                   challenge_type,
                   flag_mode,
                   is_visible,
                   created_at,
                   updated_at",
    )
    .bind(challenge_id)
    .bind(title)
    .bind(slug)
    .bind(category)
    .bind(normalized_difficulty)
    .bind(req.description)
    .bind(req.static_score)
    .bind(normalized_challenge_type)
    .bind(normalized_flag_mode)
    .bind(req.flag_hash)
    .bind(req.compose_template)
    .bind(req.metadata)
    .bind(req.is_visible)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| {
        if is_unique_violation(&err) {
            AppError::Conflict("challenge slug already exists".to_string())
        } else {
            AppError::internal(err)
        }
    })?
    .ok_or(AppError::BadRequest("challenge not found".to_string()))?;

    Ok(Json(row))
}

async fn list_contests(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
) -> AppResult<Json<Vec<AdminContestItem>>> {
    ensure_admin_or_judge(&current_user)?;

    let rows = sqlx::query_as::<_, AdminContestItem>(
        "SELECT id,
                title,
                slug,
                visibility,
                status,
                start_at,
                end_at,
                freeze_at,
                created_at,
                updated_at
         FROM contests
         ORDER BY start_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(rows))
}

async fn update_contest_status(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(contest_id): Path<Uuid>,
    Json(req): Json<UpdateContestStatusRequest>,
) -> AppResult<Json<AdminContestItem>> {
    ensure_admin_or_judge(&current_user)?;

    let status = normalize_with_allowed(&req.status, CONTEST_STATUS_ALLOWED, "status")?;

    let row = sqlx::query_as::<_, AdminContestItem>(
        "UPDATE contests
         SET status = $2,
             updated_at = NOW()
         WHERE id = $1
         RETURNING id,
                   title,
                   slug,
                   visibility,
                   status,
                   start_at,
                   end_at,
                   freeze_at,
                   created_at,
                   updated_at",
    )
    .bind(contest_id)
    .bind(status)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("contest not found".to_string()))?;

    Ok(Json(row))
}

async fn list_instances(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Query(query): Query<AdminInstancesQuery>,
) -> AppResult<Json<Vec<AdminInstanceItem>>> {
    ensure_admin_or_judge(&current_user)?;

    let status_filter = query
        .status
        .as_deref()
        .map(|v| {
            normalize_with_allowed(
                v,
                &[
                    "creating",
                    "running",
                    "stopped",
                    "destroyed",
                    "expired",
                    "failed",
                ],
                "status",
            )
        })
        .transpose()?;

    let limit = query.limit.unwrap_or(100).clamp(1, 500);

    let rows = sqlx::query_as::<_, AdminInstanceItem>(
        "SELECT i.id,
                i.contest_id,
                ct.title AS contest_title,
                i.challenge_id,
                c.title AS challenge_title,
                i.team_id,
                t.name AS team_name,
                i.status,
                i.subnet::text AS subnet,
                i.compose_project_name,
                i.entrypoint_url,
                i.started_at,
                i.expires_at,
                i.destroyed_at,
                i.last_heartbeat_at,
                i.created_at,
                i.updated_at
         FROM instances i
         JOIN contests ct ON ct.id = i.contest_id
         JOIN challenges c ON c.id = i.challenge_id
         JOIN teams t ON t.id = i.team_id
         WHERE ($1::text IS NULL OR i.status = $1)
         ORDER BY i.updated_at DESC
         LIMIT $2",
    )
    .bind(status_filter)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(rows))
}

fn ensure_admin_or_judge(user: &AuthenticatedUser) -> AppResult<()> {
    if user.role == "admin" || user.role == "judge" {
        return Ok(());
    }

    Err(AppError::Forbidden)
}

fn trim_required(value: &str, field: &str) -> AppResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest(format!("{} is required", field)));
    }

    Ok(trimmed.to_string())
}

fn normalize_with_allowed(value: &str, allowed: &[&str], field: &str) -> AppResult<String> {
    let lowered = value.trim().to_lowercase();
    if lowered.is_empty() {
        return Err(AppError::BadRequest(format!("{} is required", field)));
    }

    if allowed.contains(&lowered.as_str()) {
        Ok(lowered)
    } else {
        Err(AppError::BadRequest(format!(
            "{} is invalid, allowed: {}",
            field,
            allowed.join(",")
        )))
    }
}

fn is_unique_violation(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db_err) => db_err.code().as_deref() == Some("23505"),
        _ => false,
    }
}
