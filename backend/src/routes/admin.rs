use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, patch},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;
use tracing::warn;
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
const CONTEST_VISIBILITY_ALLOWED: &[&str] = &["public", "private"];
const INSTANCE_STATUS_ALLOWED: &[&str] = &[
    "creating",
    "running",
    "stopped",
    "destroyed",
    "expired",
    "failed",
];

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
    description: String,
    visibility: String,
    status: String,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
    freeze_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct CreateContestRequest {
    title: String,
    slug: String,
    description: Option<String>,
    visibility: Option<String>,
    status: Option<String>,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
    freeze_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct UpdateContestRequest {
    title: Option<String>,
    slug: Option<String>,
    description: Option<String>,
    visibility: Option<String>,
    status: Option<String>,
    start_at: Option<DateTime<Utc>>,
    end_at: Option<DateTime<Utc>>,
    freeze_at: Option<DateTime<Utc>>,
    clear_freeze_at: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct UpdateContestStatusRequest {
    status: String,
}

#[derive(Debug, Serialize, FromRow)]
struct AdminContestChallengeItem {
    contest_id: Uuid,
    challenge_id: Uuid,
    challenge_title: String,
    challenge_category: String,
    challenge_difficulty: String,
    sort_order: i32,
    release_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct UpsertContestChallengeRequest {
    challenge_id: Uuid,
    sort_order: Option<i32>,
    release_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct UpdateContestChallengeRequest {
    sort_order: Option<i32>,
    release_at: Option<DateTime<Utc>>,
    clear_release_at: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct AdminInstancesQuery {
    status: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AdminAuditLogsQuery {
    action: Option<String>,
    target_type: Option<String>,
    actor_user_id: Option<Uuid>,
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

#[derive(Debug, Serialize, FromRow)]
struct AdminAuditLogItem {
    id: i64,
    actor_user_id: Option<Uuid>,
    actor_username: Option<String>,
    actor_role: String,
    action: String,
    target_type: String,
    target_id: Option<Uuid>,
    detail: Value,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
struct AdminRuntimeInstanceAlertItem {
    id: Uuid,
    contest_id: Uuid,
    contest_title: String,
    challenge_id: Uuid,
    challenge_title: String,
    team_id: Uuid,
    team_name: String,
    status: String,
    expires_at: Option<DateTime<Utc>>,
    last_heartbeat_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct AdminRuntimeOverview {
    generated_at: DateTime<Utc>,
    total_users: i64,
    total_teams: i64,
    total_contests: i64,
    running_contests: i64,
    total_challenges: i64,
    total_submissions: i64,
    submissions_last_24h: i64,
    instances_total: i64,
    instances_running: i64,
    instances_failed: i64,
    instances_expiring_within_30m: i64,
    instances_expired_not_destroyed: i64,
    recent_failed_instances: Vec<AdminRuntimeInstanceAlertItem>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/admin/challenges",
            get(list_challenges).post(create_challenge),
        )
        .route("/admin/challenges/{challenge_id}", patch(update_challenge))
        .route("/admin/contests", get(list_contests).post(create_contest))
        .route("/admin/contests/{contest_id}", patch(update_contest))
        .route(
            "/admin/contests/{contest_id}/status",
            patch(update_contest_status),
        )
        .route(
            "/admin/contests/{contest_id}/challenges",
            get(list_contest_challenges).post(upsert_contest_challenge),
        )
        .route(
            "/admin/contests/{contest_id}/challenges/{challenge_id}",
            patch(update_contest_challenge).delete(remove_contest_challenge),
        )
        .route("/admin/instances", get(list_instances))
        .route("/admin/audit-logs", get(list_audit_logs))
        .route("/admin/runtime/overview", get(get_runtime_overview))
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

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.challenge.create",
        "challenge",
        Some(row.id),
        json!({
            "title": &row.title,
            "slug": &row.slug,
            "category": &row.category,
            "difficulty": &row.difficulty,
            "challenge_type": &row.challenge_type,
            "flag_mode": &row.flag_mode,
            "is_visible": row.is_visible
        }),
    )
    .await;

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

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.challenge.update",
        "challenge",
        Some(row.id),
        json!({
            "title": &row.title,
            "slug": &row.slug,
            "category": &row.category,
            "difficulty": &row.difficulty,
            "challenge_type": &row.challenge_type,
            "flag_mode": &row.flag_mode,
            "is_visible": row.is_visible
        }),
    )
    .await;

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
                description,
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

async fn create_contest(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Json(req): Json<CreateContestRequest>,
) -> AppResult<Json<AdminContestItem>> {
    ensure_admin_or_judge(&current_user)?;

    validate_contest_time_window(req.start_at, req.end_at, req.freeze_at)?;

    let title = trim_required(&req.title, "title")?;
    let slug = trim_required(&req.slug, "slug")?.to_lowercase();
    let description = req.description.unwrap_or_default();
    let visibility = normalize_with_allowed(
        req.visibility.as_deref().unwrap_or("public"),
        CONTEST_VISIBILITY_ALLOWED,
        "visibility",
    )?;
    let status = normalize_with_allowed(
        req.status.as_deref().unwrap_or("draft"),
        CONTEST_STATUS_ALLOWED,
        "status",
    )?;

    let row = sqlx::query_as::<_, AdminContestItem>(
        "INSERT INTO contests (
            title,
            slug,
            description,
            visibility,
            status,
            start_at,
            end_at,
            freeze_at,
            created_by
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         RETURNING id,
                   title,
                   slug,
                   description,
                   visibility,
                   status,
                   start_at,
                   end_at,
                   freeze_at,
                   created_at,
                   updated_at",
    )
    .bind(title)
    .bind(slug)
    .bind(description)
    .bind(visibility)
    .bind(status)
    .bind(req.start_at)
    .bind(req.end_at)
    .bind(req.freeze_at)
    .bind(current_user.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|err| {
        if is_unique_violation(&err) {
            AppError::Conflict("contest slug already exists".to_string())
        } else {
            AppError::internal(err)
        }
    })?;

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.contest.create",
        "contest",
        Some(row.id),
        json!({
            "title": &row.title,
            "slug": &row.slug,
            "status": &row.status,
            "visibility": &row.visibility,
            "start_at": row.start_at,
            "end_at": row.end_at,
            "freeze_at": row.freeze_at
        }),
    )
    .await;

    Ok(Json(row))
}

async fn update_contest(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(contest_id): Path<Uuid>,
    Json(req): Json<UpdateContestRequest>,
) -> AppResult<Json<AdminContestItem>> {
    ensure_admin_or_judge(&current_user)?;

    let existing = sqlx::query_as::<_, AdminContestItem>(
        "SELECT id,
                title,
                slug,
                description,
                visibility,
                status,
                start_at,
                end_at,
                freeze_at,
                created_at,
                updated_at
         FROM contests
         WHERE id = $1
         LIMIT 1",
    )
    .bind(contest_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("contest not found".to_string()))?;

    let title = match req.title {
        Some(value) => trim_required(&value, "title")?,
        None => existing.title,
    };
    let slug = match req.slug {
        Some(value) => trim_required(&value, "slug")?.to_lowercase(),
        None => existing.slug,
    };
    let description = req.description.unwrap_or(existing.description);
    let visibility = match req.visibility {
        Some(value) => normalize_with_allowed(&value, CONTEST_VISIBILITY_ALLOWED, "visibility")?,
        None => existing.visibility,
    };
    let status = match req.status {
        Some(value) => normalize_with_allowed(&value, CONTEST_STATUS_ALLOWED, "status")?,
        None => existing.status,
    };

    let start_at = req.start_at.unwrap_or(existing.start_at);
    let end_at = req.end_at.unwrap_or(existing.end_at);
    let freeze_at = if req.clear_freeze_at.unwrap_or(false) {
        None
    } else {
        req.freeze_at.or(existing.freeze_at)
    };

    validate_contest_time_window(start_at, end_at, freeze_at)?;

    let row = sqlx::query_as::<_, AdminContestItem>(
        "UPDATE contests
         SET title = $2,
             slug = $3,
             description = $4,
             visibility = $5,
             status = $6,
             start_at = $7,
             end_at = $8,
             freeze_at = $9,
             updated_at = NOW()
         WHERE id = $1
         RETURNING id,
                   title,
                   slug,
                   description,
                   visibility,
                   status,
                   start_at,
                   end_at,
                   freeze_at,
                   created_at,
                   updated_at",
    )
    .bind(contest_id)
    .bind(title)
    .bind(slug)
    .bind(description)
    .bind(visibility)
    .bind(status)
    .bind(start_at)
    .bind(end_at)
    .bind(freeze_at)
    .fetch_one(&state.db)
    .await
    .map_err(|err| {
        if is_unique_violation(&err) {
            AppError::Conflict("contest slug already exists".to_string())
        } else {
            AppError::internal(err)
        }
    })?;

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.contest.update",
        "contest",
        Some(row.id),
        json!({
            "title": &row.title,
            "slug": &row.slug,
            "status": &row.status,
            "visibility": &row.visibility,
            "start_at": row.start_at,
            "end_at": row.end_at,
            "freeze_at": row.freeze_at
        }),
    )
    .await;

    Ok(Json(row))
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
                   description,
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

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.contest.status.update",
        "contest",
        Some(row.id),
        json!({
            "status": &row.status
        }),
    )
    .await;

    Ok(Json(row))
}

async fn list_contest_challenges(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(contest_id): Path<Uuid>,
) -> AppResult<Json<Vec<AdminContestChallengeItem>>> {
    ensure_admin_or_judge(&current_user)?;
    ensure_contest_exists(state.as_ref(), contest_id).await?;

    let rows = sqlx::query_as::<_, AdminContestChallengeItem>(
        "SELECT cc.contest_id,
                cc.challenge_id,
                c.title AS challenge_title,
                c.category AS challenge_category,
                c.difficulty AS challenge_difficulty,
                cc.sort_order,
                cc.release_at
         FROM contest_challenges cc
         JOIN challenges c ON c.id = cc.challenge_id
         WHERE cc.contest_id = $1
         ORDER BY cc.sort_order ASC, c.created_at ASC",
    )
    .bind(contest_id)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(rows))
}

async fn upsert_contest_challenge(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(contest_id): Path<Uuid>,
    Json(req): Json<UpsertContestChallengeRequest>,
) -> AppResult<Json<AdminContestChallengeItem>> {
    ensure_admin_or_judge(&current_user)?;

    let sort_order = req.sort_order.unwrap_or(0);

    let row = sqlx::query_as::<_, AdminContestChallengeItem>(
        "WITH upserted AS (
            INSERT INTO contest_challenges (contest_id, challenge_id, sort_order, release_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (contest_id, challenge_id)
            DO UPDATE SET sort_order = EXCLUDED.sort_order,
                          release_at = EXCLUDED.release_at
            RETURNING contest_id, challenge_id, sort_order, release_at
         )
         SELECT u.contest_id,
                u.challenge_id,
                c.title AS challenge_title,
                c.category AS challenge_category,
                c.difficulty AS challenge_difficulty,
                u.sort_order,
                u.release_at
         FROM upserted u
         JOIN challenges c ON c.id = u.challenge_id",
    )
    .bind(contest_id)
    .bind(req.challenge_id)
    .bind(sort_order)
    .bind(req.release_at)
    .fetch_one(&state.db)
    .await
    .map_err(|err| {
        if is_foreign_key_violation(&err) {
            AppError::BadRequest("contest or challenge not found".to_string())
        } else {
            AppError::internal(err)
        }
    })?;

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.contest_challenge.upsert",
        "contest_challenge",
        Some(row.challenge_id),
        json!({
            "contest_id": row.contest_id,
            "challenge_id": row.challenge_id,
            "sort_order": row.sort_order,
            "release_at": row.release_at
        }),
    )
    .await;

    Ok(Json(row))
}

async fn update_contest_challenge(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path((contest_id, challenge_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateContestChallengeRequest>,
) -> AppResult<Json<AdminContestChallengeItem>> {
    ensure_admin_or_judge(&current_user)?;

    if req.sort_order.is_none()
        && req.release_at.is_none()
        && !req.clear_release_at.unwrap_or(false)
    {
        return Err(AppError::BadRequest(
            "at least one field is required for update".to_string(),
        ));
    }

    let row = sqlx::query_as::<_, AdminContestChallengeItem>(
        "WITH updated AS (
            UPDATE contest_challenges
            SET sort_order = COALESCE($3, sort_order),
                release_at = CASE
                    WHEN $4 THEN NULL
                    ELSE COALESCE($5, release_at)
                END
            WHERE contest_id = $1 AND challenge_id = $2
            RETURNING contest_id, challenge_id, sort_order, release_at
         )
         SELECT u.contest_id,
                u.challenge_id,
                c.title AS challenge_title,
                c.category AS challenge_category,
                c.difficulty AS challenge_difficulty,
                u.sort_order,
                u.release_at
         FROM updated u
         JOIN challenges c ON c.id = u.challenge_id",
    )
    .bind(contest_id)
    .bind(challenge_id)
    .bind(req.sort_order)
    .bind(req.clear_release_at.unwrap_or(false))
    .bind(req.release_at)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest(
        "contest challenge binding not found".to_string(),
    ))?;

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.contest_challenge.update",
        "contest_challenge",
        Some(row.challenge_id),
        json!({
            "contest_id": row.contest_id,
            "challenge_id": row.challenge_id,
            "sort_order": row.sort_order,
            "release_at": row.release_at
        }),
    )
    .await;

    Ok(Json(row))
}

async fn remove_contest_challenge(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path((contest_id, challenge_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    ensure_admin_or_judge(&current_user)?;

    let result =
        sqlx::query("DELETE FROM contest_challenges WHERE contest_id = $1 AND challenge_id = $2")
            .bind(contest_id)
            .bind(challenge_id)
            .execute(&state.db)
            .await
            .map_err(AppError::internal)?;

    if result.rows_affected() == 0 {
        return Err(AppError::BadRequest(
            "contest challenge binding not found".to_string(),
        ));
    }

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.contest_challenge.delete",
        "contest_challenge",
        Some(challenge_id),
        json!({
            "contest_id": contest_id,
            "challenge_id": challenge_id
        }),
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
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
        .map(|v| normalize_with_allowed(v, INSTANCE_STATUS_ALLOWED, "status"))
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

async fn list_audit_logs(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Query(query): Query<AdminAuditLogsQuery>,
) -> AppResult<Json<Vec<AdminAuditLogItem>>> {
    ensure_admin_or_judge(&current_user)?;

    let action_filter = normalize_optional_filter(query.action);
    let target_type_filter = normalize_optional_filter(query.target_type);
    let limit = query.limit.unwrap_or(200).clamp(1, 1000);

    let rows = sqlx::query_as::<_, AdminAuditLogItem>(
        "SELECT l.id,
                l.actor_user_id,
                u.username AS actor_username,
                l.actor_role,
                l.action,
                l.target_type,
                l.target_id,
                l.detail,
                l.created_at
         FROM audit_logs l
         LEFT JOIN users u ON u.id = l.actor_user_id
         WHERE ($1::text IS NULL OR l.action = $1)
           AND ($2::text IS NULL OR l.target_type = $2)
           AND ($3::uuid IS NULL OR l.actor_user_id = $3)
         ORDER BY l.created_at DESC
         LIMIT $4",
    )
    .bind(action_filter)
    .bind(target_type_filter)
    .bind(query.actor_user_id)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(rows))
}

async fn get_runtime_overview(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
) -> AppResult<Json<AdminRuntimeOverview>> {
    ensure_admin_or_judge(&current_user)?;

    let total_users = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await
        .map_err(AppError::internal)?;

    let total_teams = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM teams")
        .fetch_one(&state.db)
        .await
        .map_err(AppError::internal)?;

    let total_contests = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM contests")
        .fetch_one(&state.db)
        .await
        .map_err(AppError::internal)?;

    let running_contests =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM contests WHERE status = 'running'")
            .fetch_one(&state.db)
            .await
            .map_err(AppError::internal)?;

    let total_challenges = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM challenges")
        .fetch_one(&state.db)
        .await
        .map_err(AppError::internal)?;

    let total_submissions = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM submissions")
        .fetch_one(&state.db)
        .await
        .map_err(AppError::internal)?;

    let submissions_last_24h = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM submissions
         WHERE submitted_at >= NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(&state.db)
    .await
    .map_err(AppError::internal)?;

    let instances_total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM instances")
        .fetch_one(&state.db)
        .await
        .map_err(AppError::internal)?;

    let instances_running =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM instances WHERE status = 'running'")
            .fetch_one(&state.db)
            .await
            .map_err(AppError::internal)?;

    let instances_failed =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM instances WHERE status = 'failed'")
            .fetch_one(&state.db)
            .await
            .map_err(AppError::internal)?;

    let instances_expiring_within_30m = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM instances
         WHERE status = 'running'
           AND expires_at IS NOT NULL
           AND expires_at > NOW()
           AND expires_at <= NOW() + INTERVAL '30 minutes'",
    )
    .fetch_one(&state.db)
    .await
    .map_err(AppError::internal)?;

    let instances_expired_not_destroyed = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM instances
         WHERE status <> 'destroyed'
           AND expires_at IS NOT NULL
           AND expires_at <= NOW()",
    )
    .fetch_one(&state.db)
    .await
    .map_err(AppError::internal)?;

    let recent_failed_instances = sqlx::query_as::<_, AdminRuntimeInstanceAlertItem>(
        "SELECT i.id,
                i.contest_id,
                ct.title AS contest_title,
                i.challenge_id,
                c.title AS challenge_title,
                i.team_id,
                t.name AS team_name,
                i.status,
                i.expires_at,
                i.last_heartbeat_at,
                i.updated_at
         FROM instances i
         JOIN contests ct ON ct.id = i.contest_id
         JOIN challenges c ON c.id = i.challenge_id
         JOIN teams t ON t.id = i.team_id
         WHERE i.status = 'failed'
         ORDER BY i.updated_at DESC
         LIMIT 20",
    )
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(AdminRuntimeOverview {
        generated_at: Utc::now(),
        total_users,
        total_teams,
        total_contests,
        running_contests,
        total_challenges,
        total_submissions,
        submissions_last_24h,
        instances_total,
        instances_running,
        instances_failed,
        instances_expiring_within_30m,
        instances_expired_not_destroyed,
        recent_failed_instances,
    }))
}

async fn ensure_contest_exists(state: &AppState, contest_id: Uuid) -> AppResult<()> {
    let exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM contests WHERE id = $1)")
            .bind(contest_id)
            .fetch_one(&state.db)
            .await
            .map_err(AppError::internal)?;

    if exists {
        Ok(())
    } else {
        Err(AppError::BadRequest("contest not found".to_string()))
    }
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

fn normalize_optional_filter(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim().to_lowercase();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn validate_contest_time_window(
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
    freeze_at: Option<DateTime<Utc>>,
) -> AppResult<()> {
    if end_at <= start_at {
        return Err(AppError::BadRequest(
            "contest end_at must be later than start_at".to_string(),
        ));
    }

    if let Some(freeze) = freeze_at {
        if freeze < start_at || freeze > end_at {
            return Err(AppError::BadRequest(
                "freeze_at must be between start_at and end_at".to_string(),
            ));
        }
    }

    Ok(())
}

fn is_unique_violation(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db_err) => db_err.code().as_deref() == Some("23505"),
        _ => false,
    }
}

fn is_foreign_key_violation(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db_err) => db_err.code().as_deref() == Some("23503"),
        _ => false,
    }
}

async fn record_audit_log(
    state: &AppState,
    current_user: &AuthenticatedUser,
    action: &str,
    target_type: &str,
    target_id: Option<Uuid>,
    detail: Value,
) {
    if let Err(err) = sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, actor_role, action, target_type, target_id, detail)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(current_user.user_id)
    .bind(current_user.role.as_str())
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(detail)
    .execute(&state.db)
    .await
    {
        warn!(
            actor_user_id = %current_user.user_id,
            action,
            target_type,
            error = %err,
            "failed to write admin audit log"
        );
    }
}
