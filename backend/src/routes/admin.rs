use std::{collections::HashSet, path::PathBuf, sync::Arc};

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, patch, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;
use tokio::fs;
use tracing::warn;
use uuid::Uuid;

use crate::{
    auth::{self, AuthenticatedUser},
    error::{AppError, AppResult},
    state::AppState,
};

const DIFFICULTY_ALLOWED: &[&str] = &["easy", "normal", "hard", "insane"];
const CHALLENGE_TYPE_ALLOWED: &[&str] = &["static", "dynamic", "internal"];
const FLAG_MODE_ALLOWED: &[&str] = &["static", "dynamic", "script"];
const CONTEST_STATUS_ALLOWED: &[&str] = &["draft", "scheduled", "running", "ended", "archived"];
const CONTEST_VISIBILITY_ALLOWED: &[&str] = &["public", "private"];
const CONTEST_SCORING_MODE_ALLOWED: &[&str] = &["static", "dynamic"];
const WRITEUP_VISIBILITY_ALLOWED: &[&str] = &["hidden", "after_solve", "after_contest", "public"];
const CHALLENGE_STATUS_ALLOWED: &[&str] = &["draft", "published", "offline"];
const USER_ROLE_ALLOWED: &[&str] = &["player", "admin", "judge"];
const USER_STATUS_ALLOWED: &[&str] = &["active", "disabled"];
const INSTANCE_STATUS_ALLOWED: &[&str] = &[
    "creating",
    "running",
    "stopped",
    "destroyed",
    "expired",
    "failed",
];
const RUNTIME_ALERT_STATUS_ALLOWED: &[&str] = &["open", "acknowledged", "resolved"];
const RUNTIME_ALERT_SEVERITY_ALLOWED: &[&str] = &["info", "warning", "critical"];
const RUNTIME_ALERT_SOURCE_INSTANCE: &str = "instance";
const RUNTIME_ALERT_TYPE_INSTANCE_FAILED: &str = "instance_failed";
const RUNTIME_ALERT_TYPE_INSTANCE_EXPIRING_SOON: &str = "instance_expiring_soon";
const RUNTIME_ALERT_TYPE_INSTANCE_EXPIRED_NOT_DESTROYED: &str =
    "instance_expired_not_destroyed";
const RUNTIME_ALERT_TYPE_INSTANCE_HEARTBEAT_STALE: &str = "instance_heartbeat_stale";
const RUNTIME_ALERT_SCANNER_TYPES: &[&str] = &[
    RUNTIME_ALERT_TYPE_INSTANCE_FAILED,
    RUNTIME_ALERT_TYPE_INSTANCE_EXPIRING_SOON,
    RUNTIME_ALERT_TYPE_INSTANCE_EXPIRED_NOT_DESTROYED,
    RUNTIME_ALERT_TYPE_INSTANCE_HEARTBEAT_STALE,
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
    status: String,
    is_visible: bool,
    tags: Vec<String>,
    writeup_visibility: String,
    current_version: i32,
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
    status: Option<String>,
    flag_hash: Option<String>,
    compose_template: Option<String>,
    metadata: Option<Value>,
    is_visible: Option<bool>,
    tags: Option<Vec<String>>,
    writeup_visibility: Option<String>,
    writeup_content: Option<String>,
    change_note: Option<String>,
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
    status: Option<String>,
    flag_hash: Option<String>,
    compose_template: Option<String>,
    metadata: Option<Value>,
    is_visible: Option<bool>,
    tags: Option<Vec<String>>,
    writeup_visibility: Option<String>,
    writeup_content: Option<String>,
    change_note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChallengeSnapshot {
    title: String,
    slug: String,
    category: String,
    difficulty: String,
    description: String,
    static_score: i32,
    min_score: i32,
    max_score: i32,
    challenge_type: String,
    flag_mode: String,
    #[serde(default = "default_challenge_status")]
    status: String,
    flag_hash: String,
    compose_template: Option<String>,
    metadata: Value,
    is_visible: bool,
    tags: Vec<String>,
    writeup_visibility: String,
    writeup_content: String,
}

#[derive(Debug, FromRow)]
struct ChallengeSnapshotRow {
    id: Uuid,
    title: String,
    slug: String,
    category: String,
    difficulty: String,
    description: String,
    static_score: i32,
    min_score: i32,
    max_score: i32,
    challenge_type: String,
    flag_mode: String,
    status: String,
    flag_hash: String,
    compose_template: Option<String>,
    metadata: Value,
    is_visible: bool,
    tags: Vec<String>,
    writeup_visibility: String,
    writeup_content: String,
    current_version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
struct AdminChallengeVersionItem {
    id: Uuid,
    challenge_id: Uuid,
    version_no: i32,
    change_note: String,
    created_by: Option<Uuid>,
    created_by_username: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct ChallengeVersionsQuery {
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct RollbackChallengeRequest {
    version_no: i32,
    change_note: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UploadChallengeAttachmentRequest {
    filename: String,
    content_base64: String,
    content_type: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
struct AdminChallengeAttachmentItem {
    id: Uuid,
    challenge_id: Uuid,
    filename: String,
    content_type: String,
    storage_path: String,
    size_bytes: i64,
    uploaded_by: Option<Uuid>,
    uploaded_by_username: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct ChallengeAttachmentsQuery {
    limit: Option<i64>,
}

#[derive(Debug, Serialize, FromRow)]
struct AdminContestItem {
    id: Uuid,
    title: String,
    slug: String,
    description: String,
    visibility: String,
    status: String,
    scoring_mode: String,
    dynamic_decay: i32,
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
    scoring_mode: Option<String>,
    dynamic_decay: Option<i32>,
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
    scoring_mode: Option<String>,
    dynamic_decay: Option<i32>,
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
struct AdminContestAnnouncementItem {
    id: Uuid,
    contest_id: Uuid,
    title: String,
    content: String,
    is_published: bool,
    is_pinned: bool,
    published_at: Option<DateTime<Utc>>,
    created_by: Option<Uuid>,
    created_by_username: Option<String>,
    updated_by: Option<Uuid>,
    updated_by_username: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct CreateContestAnnouncementRequest {
    title: String,
    content: String,
    is_published: Option<bool>,
    is_pinned: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct UpdateContestAnnouncementRequest {
    title: Option<String>,
    content: Option<String>,
    is_published: Option<bool>,
    is_pinned: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ContestAnnouncementsQuery {
    limit: Option<i64>,
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

#[derive(Debug, Deserialize)]
struct AdminRuntimeAlertsQuery {
    status: Option<String>,
    severity: Option<String>,
    alert_type: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct UpdateRuntimeAlertRequest {
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AdminUsersQuery {
    keyword: Option<String>,
    role: Option<String>,
    status: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct UpdateUserStatusRequest {
    status: String,
}

#[derive(Debug, Deserialize)]
struct UpdateUserRoleRequest {
    role: String,
}

#[derive(Debug, Deserialize)]
struct ResetUserPasswordRequest {
    new_password: String,
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
struct AdminRuntimeAlertItem {
    id: Uuid,
    alert_type: String,
    severity: String,
    status: String,
    source_type: String,
    source_id: Option<Uuid>,
    fingerprint: String,
    title: String,
    message: String,
    detail: Value,
    first_seen_at: DateTime<Utc>,
    last_seen_at: DateTime<Utc>,
    acknowledged_at: Option<DateTime<Utc>>,
    acknowledged_by: Option<Uuid>,
    acknowledged_by_username: Option<String>,
    resolved_at: Option<DateTime<Utc>>,
    resolved_by: Option<Uuid>,
    resolved_by_username: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
struct AdminUserItem {
    id: Uuid,
    username: String,
    email: String,
    role: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
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

#[derive(Debug, Serialize)]
struct AdminRuntimeAlertScanResponse {
    generated_at: DateTime<Utc>,
    upserted: i64,
    auto_resolved: i64,
    open_count: i64,
    acknowledged_count: i64,
    resolved_count: i64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RuntimeAlertScanSummary {
    pub upserted: i64,
    pub auto_resolved: i64,
    pub open_count: i64,
    pub acknowledged_count: i64,
    pub resolved_count: i64,
}

#[derive(Debug, FromRow)]
struct RuntimeAlertSignalInstanceRow {
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

#[derive(Debug)]
struct RuntimeAlertCandidate {
    alert_type: String,
    severity: String,
    source_type: String,
    source_id: Option<Uuid>,
    fingerprint: String,
    title: String,
    message: String,
    detail: Value,
}

#[derive(Debug, FromRow)]
struct RuntimeAlertCountsRow {
    open_count: i64,
    acknowledged_count: i64,
    resolved_count: i64,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/admin/users", get(list_users))
        .route("/admin/users/{user_id}/status", patch(update_user_status))
        .route("/admin/users/{user_id}/role", patch(update_user_role))
        .route(
            "/admin/users/{user_id}/reset-password",
            post(reset_user_password),
        )
        .route(
            "/admin/challenges",
            get(list_challenges).post(create_challenge),
        )
        .route("/admin/challenges/{challenge_id}", patch(update_challenge))
        .route(
            "/admin/challenges/{challenge_id}/versions",
            get(list_challenge_versions),
        )
        .route(
            "/admin/challenges/{challenge_id}/rollback",
            post(rollback_challenge_version),
        )
        .route(
            "/admin/challenges/{challenge_id}/attachments",
            get(list_challenge_attachments).post(upload_challenge_attachment),
        )
        .route(
            "/admin/challenges/{challenge_id}/attachments/{attachment_id}",
            axum::routing::delete(delete_challenge_attachment),
        )
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
        .route(
            "/admin/contests/{contest_id}/announcements",
            get(list_contest_announcements).post(create_contest_announcement),
        )
        .route(
            "/admin/contests/{contest_id}/announcements/{announcement_id}",
            patch(update_contest_announcement).delete(delete_contest_announcement),
        )
        .route("/admin/instances", get(list_instances))
        .route("/admin/audit-logs", get(list_audit_logs))
        .route("/admin/runtime/alerts", get(list_runtime_alerts))
        .route("/admin/runtime/alerts/scan", post(scan_runtime_alerts))
        .route(
            "/admin/runtime/alerts/{alert_id}/ack",
            post(acknowledge_runtime_alert),
        )
        .route(
            "/admin/runtime/alerts/{alert_id}/resolve",
            post(resolve_runtime_alert),
        )
        .route("/admin/runtime/overview", get(get_runtime_overview))
}

async fn list_users(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Query(query): Query<AdminUsersQuery>,
) -> AppResult<Json<Vec<AdminUserItem>>> {
    ensure_admin(&current_user)?;

    let keyword = normalize_optional_filter(query.keyword);
    let role_filter = query
        .role
        .as_deref()
        .map(|value| normalize_with_allowed(value, USER_ROLE_ALLOWED, "role"))
        .transpose()?;
    let status_filter = query
        .status
        .as_deref()
        .map(|value| normalize_with_allowed(value, USER_STATUS_ALLOWED, "status"))
        .transpose()?;
    let limit = query.limit.unwrap_or(200).clamp(1, 1000);

    let rows = sqlx::query_as::<_, AdminUserItem>(
        "SELECT id,
                username,
                email,
                role,
                status,
                created_at,
                updated_at
         FROM users
         WHERE ($1::text IS NULL OR LOWER(username) LIKE '%' || LOWER($1) || '%' OR LOWER(email) LIKE '%' || LOWER($1) || '%')
           AND ($2::text IS NULL OR role = $2)
           AND ($3::text IS NULL OR status = $3)
         ORDER BY created_at DESC
         LIMIT $4",
    )
    .bind(keyword)
    .bind(role_filter)
    .bind(status_filter)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(rows))
}

async fn update_user_status(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(user_id): Path<Uuid>,
    Json(req): Json<UpdateUserStatusRequest>,
) -> AppResult<Json<AdminUserItem>> {
    ensure_admin(&current_user)?;

    let status = normalize_with_allowed(&req.status, USER_STATUS_ALLOWED, "status")?;
    if current_user.user_id == user_id && status == "disabled" {
        return Err(AppError::BadRequest(
            "cannot disable current admin user".to_string(),
        ));
    }

    let row = sqlx::query_as::<_, AdminUserItem>(
        "UPDATE users
         SET status = $2,
             updated_at = NOW()
         WHERE id = $1
         RETURNING id,
                   username,
                   email,
                   role,
                   status,
                   created_at,
                   updated_at",
    )
    .bind(user_id)
    .bind(&status)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("user not found".to_string()))?;

    let mut sessions_revoked = false;
    if row.status == "disabled" {
        auth::revoke_all_user_sessions(state.as_ref(), row.id).await?;
        sessions_revoked = true;
    }

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.user.status.update",
        "user",
        Some(row.id),
        json!({
            "target_user_id": row.id,
            "target_username": &row.username,
            "status": &row.status,
            "sessions_revoked": sessions_revoked
        }),
    )
    .await;

    Ok(Json(row))
}

async fn update_user_role(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(user_id): Path<Uuid>,
    Json(req): Json<UpdateUserRoleRequest>,
) -> AppResult<Json<AdminUserItem>> {
    ensure_admin(&current_user)?;

    let role = normalize_with_allowed(&req.role, USER_ROLE_ALLOWED, "role")?;
    if current_user.user_id == user_id && role != "admin" {
        return Err(AppError::BadRequest(
            "cannot downgrade current admin user role".to_string(),
        ));
    }

    let row = sqlx::query_as::<_, AdminUserItem>(
        "UPDATE users
         SET role = $2,
             updated_at = NOW()
         WHERE id = $1
         RETURNING id,
                   username,
                   email,
                   role,
                   status,
                   created_at,
                   updated_at",
    )
    .bind(user_id)
    .bind(&role)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("user not found".to_string()))?;

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.user.role.update",
        "user",
        Some(row.id),
        json!({
            "target_user_id": row.id,
            "target_username": &row.username,
            "role": &row.role
        }),
    )
    .await;

    Ok(Json(row))
}

async fn reset_user_password(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(user_id): Path<Uuid>,
    Json(req): Json<ResetUserPasswordRequest>,
) -> AppResult<Json<AdminUserItem>> {
    ensure_admin(&current_user)?;

    if req.new_password.len() < 8 {
        return Err(AppError::BadRequest(
            "new_password must be at least 8 characters".to_string(),
        ));
    }

    let password_hash = hash_password(&req.new_password)?;
    let row = sqlx::query_as::<_, AdminUserItem>(
        "UPDATE users
         SET password_hash = $2,
             updated_at = NOW()
         WHERE id = $1
         RETURNING id,
                   username,
                   email,
                   role,
                   status,
                   created_at,
                   updated_at",
    )
    .bind(user_id)
    .bind(password_hash)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("user not found".to_string()))?;

    auth::revoke_all_user_sessions(state.as_ref(), row.id).await?;

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.user.password.reset",
        "user",
        Some(row.id),
        json!({
            "target_user_id": row.id,
            "target_username": &row.username,
            "sessions_revoked": true
        }),
    )
    .await;

    Ok(Json(row))
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
                status,
                is_visible,
                tags,
                writeup_visibility,
                current_version,
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
    let normalized_status = req
        .status
        .as_deref()
        .map(|value| normalize_with_allowed(value, CHALLENGE_STATUS_ALLOWED, "status"))
        .transpose()?;
    let (status, is_visible) = match (normalized_status, req.is_visible) {
        (Some(status), Some(explicit_visible)) => {
            let should_be_visible = status == "published";
            if explicit_visible != should_be_visible {
                return Err(AppError::BadRequest(
                    "is_visible conflicts with status".to_string(),
                ));
            }
            (status, explicit_visible)
        }
        (Some(status), None) => {
            let visible = status == "published";
            (status, visible)
        }
        (None, Some(explicit_visible)) => {
            let derived_status = if explicit_visible {
                "published".to_string()
            } else {
                "draft".to_string()
            };
            (derived_status, explicit_visible)
        }
        (None, None) => ("draft".to_string(), false),
    };
    let tags = normalize_tags(req.tags.unwrap_or_default())?;
    let writeup_visibility = normalize_with_allowed(
        req.writeup_visibility.as_deref().unwrap_or("hidden"),
        WRITEUP_VISIBILITY_ALLOWED,
        "writeup_visibility",
    )?;
    let writeup_content = req.writeup_content.unwrap_or_default();
    if writeup_content.chars().count() > 20_000 {
        return Err(AppError::BadRequest(
            "writeup_content must be at most 20000 characters".to_string(),
        ));
    }
    let change_note = req
        .change_note
        .as_deref()
        .and_then(normalize_optional_text)
        .map(str::to_string);

    let mut tx = state.db.begin().await.map_err(AppError::internal)?;

    let row = sqlx::query_as::<_, ChallengeSnapshotRow>(
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
            status,
            flag_hash,
            compose_template,
            metadata,
            is_visible,
            tags,
            writeup_visibility,
            writeup_content,
            created_by
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
         RETURNING id,
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
                   status,
                   flag_hash,
                   compose_template,
                   metadata,
                   is_visible,
                   tags,
                   writeup_visibility,
                   writeup_content,
                   current_version,
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
    .bind(status)
    .bind(flag_hash)
    .bind(compose_template)
    .bind(metadata)
    .bind(is_visible)
    .bind(tags)
    .bind(writeup_visibility)
    .bind(writeup_content)
    .bind(current_user.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| {
        if is_unique_violation(&err) {
            AppError::Conflict("challenge slug already exists".to_string())
        } else {
            AppError::internal(err)
        }
    })?;

    let snapshot = challenge_snapshot_to_value(&row);
    sqlx::query(
        "INSERT INTO challenge_versions (
            challenge_id,
            version_no,
            snapshot,
            change_note,
            created_by
         )
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(row.id)
    .bind(row.current_version)
    .bind(snapshot)
    .bind(
        change_note
            .as_deref()
            .unwrap_or("initial version"),
    )
    .bind(current_user.user_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::internal)?;

    tx.commit().await.map_err(AppError::internal)?;
    let item = challenge_item_from_snapshot_row(&row);

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.challenge.create",
        "challenge",
        Some(item.id),
        json!({
            "title": &item.title,
            "slug": &item.slug,
            "category": &item.category,
            "difficulty": &item.difficulty,
            "challenge_type": &item.challenge_type,
            "flag_mode": &item.flag_mode,
            "status": &item.status,
            "is_visible": item.is_visible,
            "tags": &item.tags,
            "writeup_visibility": &item.writeup_visibility,
            "current_version": item.current_version
        }),
    )
    .await;

    Ok(Json(item))
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
    let normalized_status = req
        .status
        .as_deref()
        .map(|value| normalize_with_allowed(value, CHALLENGE_STATUS_ALLOWED, "status"))
        .transpose()?;
    let normalized_writeup_visibility = req
        .writeup_visibility
        .as_deref()
        .map(|value| normalize_with_allowed(value, WRITEUP_VISIBILITY_ALLOWED, "writeup_visibility"))
        .transpose()?;
    let normalized_tags = req.tags.map(normalize_tags).transpose()?;
    let change_note = req
        .change_note
        .as_deref()
        .and_then(normalize_optional_text)
        .map(str::to_string);

    let resolved_status = match (normalized_status, req.is_visible) {
        (Some(status), Some(explicit_visible)) => {
            let should_be_visible = status == "published";
            if explicit_visible != should_be_visible {
                return Err(AppError::BadRequest(
                    "is_visible conflicts with status".to_string(),
                ));
            }
            Some(status)
        }
        (Some(status), None) => Some(status),
        (None, Some(true)) => Some("published".to_string()),
        (None, Some(false)) => Some("draft".to_string()),
        (None, None) => None,
    };
    let resolved_is_visible = resolved_status
        .as_ref()
        .map(|status| status == "published")
        .or(req.is_visible);

    if let Some(content) = req.writeup_content.as_deref() {
        if content.chars().count() > 20_000 {
            return Err(AppError::BadRequest(
                "writeup_content must be at most 20000 characters".to_string(),
            ));
        }
    }

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

    let mut tx = state.db.begin().await.map_err(AppError::internal)?;

    let row = sqlx::query_as::<_, ChallengeSnapshotRow>(
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
              tags = COALESCE($14, tags),
              writeup_visibility = COALESCE($15, writeup_visibility),
              writeup_content = COALESCE($16, writeup_content),
              status = COALESCE($17, status),
              current_version = current_version + 1,
              updated_at = NOW()
         WHERE id = $1
         RETURNING id,
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
                   status,
                   flag_hash,
                   compose_template,
                   metadata,
                   is_visible,
                   tags,
                   writeup_visibility,
                   writeup_content,
                   current_version,
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
    .bind(resolved_is_visible)
    .bind(normalized_tags)
    .bind(normalized_writeup_visibility)
    .bind(req.writeup_content)
    .bind(resolved_status)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| {
        if is_unique_violation(&err) {
            AppError::Conflict("challenge slug already exists".to_string())
        } else {
            AppError::internal(err)
        }
    })?
    .ok_or(AppError::BadRequest("challenge not found".to_string()))?;

    let snapshot = challenge_snapshot_to_value(&row);
    sqlx::query(
        "INSERT INTO challenge_versions (
            challenge_id,
            version_no,
            snapshot,
            change_note,
            created_by
         )
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(row.id)
    .bind(row.current_version)
    .bind(snapshot)
    .bind(change_note.as_deref().unwrap_or("content update"))
    .bind(current_user.user_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::internal)?;

    tx.commit().await.map_err(AppError::internal)?;
    let item = challenge_item_from_snapshot_row(&row);

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.challenge.update",
        "challenge",
        Some(item.id),
        json!({
            "title": &item.title,
            "slug": &item.slug,
            "category": &item.category,
            "difficulty": &item.difficulty,
            "challenge_type": &item.challenge_type,
            "flag_mode": &item.flag_mode,
            "status": &item.status,
            "is_visible": item.is_visible,
            "tags": &item.tags,
            "writeup_visibility": &item.writeup_visibility,
            "current_version": item.current_version
        }),
    )
    .await;

    Ok(Json(item))
}

async fn list_challenge_versions(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(challenge_id): Path<Uuid>,
    Query(query): Query<ChallengeVersionsQuery>,
) -> AppResult<Json<Vec<AdminChallengeVersionItem>>> {
    ensure_admin_or_judge(&current_user)?;
    ensure_challenge_exists(state.as_ref(), challenge_id).await?;

    let limit = query.limit.unwrap_or(30).clamp(1, 200);

    let rows = sqlx::query_as::<_, AdminChallengeVersionItem>(
        "SELECT v.id,
                v.challenge_id,
                v.version_no,
                v.change_note,
                v.created_by,
                u.username AS created_by_username,
                v.created_at
         FROM challenge_versions v
         LEFT JOIN users u ON u.id = v.created_by
         WHERE v.challenge_id = $1
         ORDER BY v.version_no DESC
         LIMIT $2",
    )
    .bind(challenge_id)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(rows))
}

async fn rollback_challenge_version(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(challenge_id): Path<Uuid>,
    Json(req): Json<RollbackChallengeRequest>,
) -> AppResult<Json<AdminChallengeItem>> {
    ensure_admin_or_judge(&current_user)?;
    ensure_challenge_exists(state.as_ref(), challenge_id).await?;

    if req.version_no < 1 {
        return Err(AppError::BadRequest(
            "version_no must be >= 1".to_string(),
        ));
    }

    let snapshot_value = sqlx::query_scalar::<_, Value>(
        "SELECT snapshot
         FROM challenge_versions
         WHERE challenge_id = $1
           AND version_no = $2
         LIMIT 1",
    )
    .bind(challenge_id)
    .bind(req.version_no)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("target version not found".to_string()))?;

    let target_snapshot: ChallengeSnapshot =
        serde_json::from_value(snapshot_value).map_err(AppError::internal)?;
    let rollback_status = normalize_with_allowed(
        &target_snapshot.status,
        CHALLENGE_STATUS_ALLOWED,
        "status",
    )?;
    let rollback_visible = rollback_status == "published";

    let mut tx = state.db.begin().await.map_err(AppError::internal)?;
    let row = sqlx::query_as::<_, ChallengeSnapshotRow>(
        "UPDATE challenges
         SET title = $2,
             slug = $3,
             category = $4,
             difficulty = $5,
             description = $6,
             static_score = $7,
             min_score = $8,
             max_score = $9,
             challenge_type = $10,
             flag_mode = $11,
             flag_hash = $12,
             compose_template = $13,
             metadata = $14,
             is_visible = $15,
             tags = $16,
             writeup_visibility = $17,
             writeup_content = $18,
             status = $19,
             current_version = current_version + 1,
             updated_at = NOW()
         WHERE id = $1
         RETURNING id,
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
                   status,
                   flag_hash,
                   compose_template,
                   metadata,
                   is_visible,
                   tags,
                   writeup_visibility,
                   writeup_content,
                   current_version,
                   created_at,
                   updated_at",
    )
    .bind(challenge_id)
    .bind(target_snapshot.title)
    .bind(target_snapshot.slug)
    .bind(target_snapshot.category)
    .bind(target_snapshot.difficulty)
    .bind(target_snapshot.description)
    .bind(target_snapshot.static_score)
    .bind(target_snapshot.min_score)
    .bind(target_snapshot.max_score)
    .bind(target_snapshot.challenge_type)
    .bind(target_snapshot.flag_mode)
    .bind(target_snapshot.flag_hash)
    .bind(target_snapshot.compose_template)
    .bind(target_snapshot.metadata)
    .bind(rollback_visible)
    .bind(target_snapshot.tags)
    .bind(target_snapshot.writeup_visibility)
    .bind(target_snapshot.writeup_content)
    .bind(rollback_status)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| {
        if is_unique_violation(&err) {
            AppError::Conflict("challenge slug already exists".to_string())
        } else {
            AppError::internal(err)
        }
    })?;

    let snapshot = challenge_snapshot_to_value(&row);
    let change_note = req
        .change_note
        .as_deref()
        .and_then(normalize_optional_text)
        .map(str::to_string)
        .unwrap_or_else(|| format!("rollback to version {}", req.version_no));

    sqlx::query(
        "INSERT INTO challenge_versions (
            challenge_id,
            version_no,
            snapshot,
            change_note,
            created_by
         )
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(row.id)
    .bind(row.current_version)
    .bind(snapshot)
    .bind(change_note)
    .bind(current_user.user_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::internal)?;

    tx.commit().await.map_err(AppError::internal)?;
    let item = challenge_item_from_snapshot_row(&row);

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.challenge.rollback",
        "challenge",
        Some(item.id),
        json!({
            "rollback_to_version": req.version_no,
            "current_version": item.current_version
        }),
    )
    .await;

    Ok(Json(item))
}

async fn upload_challenge_attachment(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(challenge_id): Path<Uuid>,
    Json(req): Json<UploadChallengeAttachmentRequest>,
) -> AppResult<Json<AdminChallengeAttachmentItem>> {
    ensure_admin_or_judge(&current_user)?;
    ensure_challenge_exists(state.as_ref(), challenge_id).await?;

    let filename = trim_required(&req.filename, "filename")?;
    if filename.chars().count() > 255 {
        return Err(AppError::BadRequest(
            "filename must be at most 255 characters".to_string(),
        ));
    }

    let decoded = {
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        STANDARD
            .decode(req.content_base64.trim())
            .map_err(|_| AppError::BadRequest("content_base64 is invalid".to_string()))?
    };

    if decoded.is_empty() {
        return Err(AppError::BadRequest(
            "attachment content is empty".to_string(),
        ));
    }
    if decoded.len() > 20 * 1024 * 1024 {
        return Err(AppError::BadRequest(
            "attachment size must be <= 20MB".to_string(),
        ));
    }

    let content_type = req
        .content_type
        .as_deref()
        .and_then(normalize_optional_text)
        .map(str::to_string)
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let safe_filename = sanitize_filename(&filename);
    let attachment_dir = challenge_attachments_dir(state.as_ref(), challenge_id);
    fs::create_dir_all(&attachment_dir)
        .await
        .map_err(AppError::internal)?;

    let stored_name = format!("{}-{}", Uuid::new_v4(), safe_filename);
    let stored_path = attachment_dir.join(stored_name);
    fs::write(&stored_path, &decoded)
        .await
        .map_err(AppError::internal)?;

    let attachment_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO challenge_attachments (
            challenge_id,
            filename,
            content_type,
            storage_path,
            size_bytes,
            uploaded_by
         )
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id",
    )
    .bind(challenge_id)
    .bind(&filename)
    .bind(&content_type)
    .bind(stored_path.to_string_lossy().to_string())
    .bind(decoded.len() as i64)
    .bind(current_user.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(AppError::internal)?;

    let item = load_challenge_attachment_item(state.as_ref(), attachment_id).await?;
    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.challenge.attachment.upload",
        "challenge_attachment",
        Some(item.id),
        json!({
            "challenge_id": challenge_id,
            "filename": &item.filename,
            "size_bytes": item.size_bytes
        }),
    )
    .await;

    Ok(Json(item))
}

async fn list_challenge_attachments(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(challenge_id): Path<Uuid>,
    Query(query): Query<ChallengeAttachmentsQuery>,
) -> AppResult<Json<Vec<AdminChallengeAttachmentItem>>> {
    ensure_admin_or_judge(&current_user)?;
    ensure_challenge_exists(state.as_ref(), challenge_id).await?;

    let limit = query.limit.unwrap_or(100).clamp(1, 500);
    let rows = sqlx::query_as::<_, AdminChallengeAttachmentItem>(
        "SELECT a.id,
                a.challenge_id,
                a.filename,
                a.content_type,
                a.storage_path,
                a.size_bytes,
                a.uploaded_by,
                u.username AS uploaded_by_username,
                a.created_at
         FROM challenge_attachments a
         LEFT JOIN users u ON u.id = a.uploaded_by
         WHERE a.challenge_id = $1
         ORDER BY a.created_at DESC
         LIMIT $2",
    )
    .bind(challenge_id)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(rows))
}

async fn delete_challenge_attachment(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path((challenge_id, attachment_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    ensure_admin_or_judge(&current_user)?;

    let row = sqlx::query_as::<_, AdminChallengeAttachmentItem>(
        "DELETE FROM challenge_attachments
         WHERE id = $1
           AND challenge_id = $2
         RETURNING id,
                   challenge_id,
                   filename,
                   content_type,
                   storage_path,
                   size_bytes,
                   uploaded_by,
                   NULL::text AS uploaded_by_username,
                   created_at",
    )
    .bind(attachment_id)
    .bind(challenge_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest(
        "challenge attachment not found".to_string(),
    ))?;

    let path = PathBuf::from(&row.storage_path);
    if let Err(err) = fs::remove_file(&path).await {
        if err.kind() != std::io::ErrorKind::NotFound {
            return Err(AppError::internal(err));
        }
    }

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.challenge.attachment.delete",
        "challenge_attachment",
        Some(row.id),
        json!({
            "challenge_id": row.challenge_id,
            "filename": row.filename
        }),
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
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
                scoring_mode,
                dynamic_decay,
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
    let scoring_mode = normalize_with_allowed(
        req.scoring_mode.as_deref().unwrap_or("static"),
        CONTEST_SCORING_MODE_ALLOWED,
        "scoring_mode",
    )?;
    let dynamic_decay = req.dynamic_decay.unwrap_or(20);
    if !(1..=100000).contains(&dynamic_decay) {
        return Err(AppError::BadRequest(
            "dynamic_decay must be between 1 and 100000".to_string(),
        ));
    }

    let row = sqlx::query_as::<_, AdminContestItem>(
        "INSERT INTO contests (
            title,
            slug,
            description,
            visibility,
            status,
            scoring_mode,
            dynamic_decay,
            start_at,
            end_at,
            freeze_at,
            created_by
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
         RETURNING id,
                   title,
                   slug,
                   description,
                   visibility,
                   status,
                   scoring_mode,
                   dynamic_decay,
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
    .bind(scoring_mode)
    .bind(dynamic_decay)
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
            "scoring_mode": &row.scoring_mode,
            "dynamic_decay": row.dynamic_decay,
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
                scoring_mode,
                dynamic_decay,
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
    let scoring_mode = match req.scoring_mode {
        Some(value) => normalize_with_allowed(&value, CONTEST_SCORING_MODE_ALLOWED, "scoring_mode")?,
        None => existing.scoring_mode,
    };
    let dynamic_decay = req.dynamic_decay.unwrap_or(existing.dynamic_decay);
    if !(1..=100000).contains(&dynamic_decay) {
        return Err(AppError::BadRequest(
            "dynamic_decay must be between 1 and 100000".to_string(),
        ));
    }

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
             scoring_mode = $7,
             dynamic_decay = $8,
             start_at = $9,
             end_at = $10,
             freeze_at = $11,
             updated_at = NOW()
         WHERE id = $1
         RETURNING id,
                   title,
                   slug,
                   description,
                   visibility,
                   status,
                   scoring_mode,
                   dynamic_decay,
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
    .bind(scoring_mode)
    .bind(dynamic_decay)
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
            "scoring_mode": &row.scoring_mode,
            "dynamic_decay": row.dynamic_decay,
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
                   scoring_mode,
                   dynamic_decay,
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

async fn list_contest_announcements(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(contest_id): Path<Uuid>,
    Query(query): Query<ContestAnnouncementsQuery>,
) -> AppResult<Json<Vec<AdminContestAnnouncementItem>>> {
    ensure_admin_or_judge(&current_user)?;
    ensure_contest_exists(state.as_ref(), contest_id).await?;

    let limit = query.limit.unwrap_or(200).clamp(1, 1000);
    let rows = sqlx::query_as::<_, AdminContestAnnouncementItem>(
        "SELECT a.id,
                a.contest_id,
                a.title,
                a.content,
                a.is_published,
                a.is_pinned,
                a.published_at,
                a.created_by,
                cu.username AS created_by_username,
                a.updated_by,
                uu.username AS updated_by_username,
                a.created_at,
                a.updated_at
         FROM contest_announcements a
         LEFT JOIN users cu ON cu.id = a.created_by
         LEFT JOIN users uu ON uu.id = a.updated_by
         WHERE a.contest_id = $1
         ORDER BY a.is_pinned DESC, COALESCE(a.published_at, a.created_at) DESC, a.created_at DESC
         LIMIT $2",
    )
    .bind(contest_id)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(rows))
}

async fn create_contest_announcement(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(contest_id): Path<Uuid>,
    Json(req): Json<CreateContestAnnouncementRequest>,
) -> AppResult<Json<AdminContestAnnouncementItem>> {
    ensure_admin_or_judge(&current_user)?;
    ensure_contest_exists(state.as_ref(), contest_id).await?;

    let title = trim_required(&req.title, "title")?;
    let content = trim_required(&req.content, "content")?;
    let is_published = req.is_published.unwrap_or(false);
    let is_pinned = req.is_pinned.unwrap_or(false);
    let published_at = if is_published { Some(Utc::now()) } else { None };

    let announcement_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO contest_announcements (
            contest_id,
            title,
            content,
            is_published,
            is_pinned,
            published_at,
            created_by,
            updated_by
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
         RETURNING id",
    )
    .bind(contest_id)
    .bind(title)
    .bind(content)
    .bind(is_published)
    .bind(is_pinned)
    .bind(published_at)
    .bind(current_user.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(AppError::internal)?;

    let item = load_contest_announcement_item(state.as_ref(), contest_id, announcement_id).await?;
    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.contest.announcement.create",
        "contest_announcement",
        Some(item.id),
        json!({
            "contest_id": item.contest_id,
            "title": &item.title,
            "is_published": item.is_published,
            "is_pinned": item.is_pinned
        }),
    )
    .await;

    Ok(Json(item))
}

async fn update_contest_announcement(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path((contest_id, announcement_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateContestAnnouncementRequest>,
) -> AppResult<Json<AdminContestAnnouncementItem>> {
    ensure_admin_or_judge(&current_user)?;
    ensure_contest_exists(state.as_ref(), contest_id).await?;

    let title = req
        .title
        .as_deref()
        .and_then(normalize_optional_text)
        .map(str::to_string);
    let content = req
        .content
        .as_deref()
        .and_then(normalize_optional_text)
        .map(str::to_string);

    if title.is_none()
        && content.is_none()
        && req.is_published.is_none()
        && req.is_pinned.is_none()
    {
        return Err(AppError::BadRequest(
            "at least one field is required for update".to_string(),
        ));
    }

    let row = sqlx::query_as::<_, AdminContestAnnouncementItem>(
        "WITH updated AS (
            UPDATE contest_announcements
            SET title = COALESCE($3, title),
                content = COALESCE($4, content),
                is_published = COALESCE($5, is_published),
                is_pinned = COALESCE($6, is_pinned),
                published_at = CASE
                    WHEN COALESCE($5, is_published) THEN COALESCE(published_at, NOW())
                    ELSE NULL
                END,
                updated_by = $7,
                updated_at = NOW()
            WHERE contest_id = $1
              AND id = $2
            RETURNING id,
                      contest_id,
                      title,
                      content,
                      is_published,
                      is_pinned,
                      published_at,
                      created_by,
                      updated_by,
                      created_at,
                      updated_at
         )
         SELECT u.id,
                u.contest_id,
                u.title,
                u.content,
                u.is_published,
                u.is_pinned,
                u.published_at,
                u.created_by,
                cu.username AS created_by_username,
                u.updated_by,
                uu.username AS updated_by_username,
                u.created_at,
                u.updated_at
         FROM updated u
         LEFT JOIN users cu ON cu.id = u.created_by
         LEFT JOIN users uu ON uu.id = u.updated_by",
    )
    .bind(contest_id)
    .bind(announcement_id)
    .bind(title)
    .bind(content)
    .bind(req.is_published)
    .bind(req.is_pinned)
    .bind(current_user.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest(
        "contest announcement not found".to_string(),
    ))?;

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.contest.announcement.update",
        "contest_announcement",
        Some(row.id),
        json!({
            "contest_id": row.contest_id,
            "is_published": row.is_published,
            "is_pinned": row.is_pinned
        }),
    )
    .await;

    Ok(Json(row))
}

async fn delete_contest_announcement(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path((contest_id, announcement_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    ensure_admin_or_judge(&current_user)?;
    ensure_contest_exists(state.as_ref(), contest_id).await?;

    let deleted = sqlx::query_as::<_, AdminContestAnnouncementItem>(
        "WITH deleted AS (
            DELETE FROM contest_announcements
            WHERE contest_id = $1
              AND id = $2
            RETURNING id,
                      contest_id,
                      title,
                      content,
                      is_published,
                      is_pinned,
                      published_at,
                      created_by,
                      updated_by,
                      created_at,
                      updated_at
         )
         SELECT d.id,
                d.contest_id,
                d.title,
                d.content,
                d.is_published,
                d.is_pinned,
                d.published_at,
                d.created_by,
                cu.username AS created_by_username,
                d.updated_by,
                uu.username AS updated_by_username,
                d.created_at,
                d.updated_at
         FROM deleted d
         LEFT JOIN users cu ON cu.id = d.created_by
         LEFT JOIN users uu ON uu.id = d.updated_by",
    )
    .bind(contest_id)
    .bind(announcement_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest(
        "contest announcement not found".to_string(),
    ))?;

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.contest.announcement.delete",
        "contest_announcement",
        Some(deleted.id),
        json!({
            "contest_id": deleted.contest_id,
            "title": deleted.title
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

async fn list_runtime_alerts(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Query(query): Query<AdminRuntimeAlertsQuery>,
) -> AppResult<Json<Vec<AdminRuntimeAlertItem>>> {
    ensure_admin_or_judge(&current_user)?;

    let status_filter = query
        .status
        .as_deref()
        .map(|value| normalize_with_allowed(value, RUNTIME_ALERT_STATUS_ALLOWED, "status"))
        .transpose()?;
    let severity_filter = query
        .severity
        .as_deref()
        .map(|value| normalize_with_allowed(value, RUNTIME_ALERT_SEVERITY_ALLOWED, "severity"))
        .transpose()?;
    let alert_type_filter = normalize_optional_filter(query.alert_type);
    let limit = query.limit.unwrap_or(100).clamp(1, 500);

    let rows = sqlx::query_as::<_, AdminRuntimeAlertItem>(
        "SELECT a.id,
                a.alert_type,
                a.severity,
                a.status,
                a.source_type,
                a.source_id,
                a.fingerprint,
                a.title,
                a.message,
                a.detail,
                a.first_seen_at,
                a.last_seen_at,
                a.acknowledged_at,
                a.acknowledged_by,
                ack_user.username AS acknowledged_by_username,
                a.resolved_at,
                a.resolved_by,
                resolved_user.username AS resolved_by_username,
                a.created_at,
                a.updated_at
         FROM runtime_alerts a
         LEFT JOIN users ack_user ON ack_user.id = a.acknowledged_by
         LEFT JOIN users resolved_user ON resolved_user.id = a.resolved_by
         WHERE ($1::text IS NULL OR a.status = $1)
           AND ($2::text IS NULL OR a.severity = $2)
           AND ($3::text IS NULL OR a.alert_type = $3)
         ORDER BY CASE a.status
                      WHEN 'open' THEN 0
                      WHEN 'acknowledged' THEN 1
                      ELSE 2
                  END,
                  a.last_seen_at DESC
         LIMIT $4",
    )
    .bind(status_filter)
    .bind(severity_filter)
    .bind(alert_type_filter)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(rows))
}

async fn scan_runtime_alerts(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
) -> AppResult<Json<AdminRuntimeAlertScanResponse>> {
    ensure_admin_or_judge(&current_user)?;

    let summary = run_runtime_alert_scan(state.as_ref()).await?;

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.runtime.alert.scan",
        "runtime_alert",
        None,
        json!({
            "upserted": summary.upserted,
            "auto_resolved": summary.auto_resolved,
            "open_count": summary.open_count,
            "acknowledged_count": summary.acknowledged_count,
            "resolved_count": summary.resolved_count
        }),
    )
    .await;

    Ok(Json(AdminRuntimeAlertScanResponse {
        generated_at: Utc::now(),
        upserted: summary.upserted,
        auto_resolved: summary.auto_resolved,
        open_count: summary.open_count,
        acknowledged_count: summary.acknowledged_count,
        resolved_count: summary.resolved_count,
    }))
}

async fn acknowledge_runtime_alert(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(alert_id): Path<Uuid>,
    payload: Option<Json<UpdateRuntimeAlertRequest>>,
) -> AppResult<Json<AdminRuntimeAlertItem>> {
    ensure_admin_or_judge(&current_user)?;

    let updated = sqlx::query_scalar::<_, Uuid>(
        "UPDATE runtime_alerts
         SET status = 'acknowledged',
             acknowledged_at = COALESCE(acknowledged_at, NOW()),
             acknowledged_by = $2,
             last_seen_at = NOW(),
             updated_at = NOW()
         WHERE id = $1
           AND status IN ('open', 'acknowledged')
         RETURNING id",
    )
    .bind(alert_id)
    .bind(current_user.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest(
        "runtime alert not found or already resolved".to_string(),
    ))?;

    let item = load_runtime_alert_item(state.as_ref(), updated).await?;
    let note = payload.as_ref().and_then(|Json(body)| {
        body.note
            .as_deref()
            .and_then(normalize_optional_text)
            .map(str::to_string)
    });
    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.runtime.alert.ack",
        "runtime_alert",
        Some(item.id),
        json!({
            "alert_type": item.alert_type,
            "severity": item.severity,
            "status": item.status,
            "note": note
        }),
    )
    .await;

    Ok(Json(item))
}

async fn resolve_runtime_alert(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(alert_id): Path<Uuid>,
    payload: Option<Json<UpdateRuntimeAlertRequest>>,
) -> AppResult<Json<AdminRuntimeAlertItem>> {
    ensure_admin_or_judge(&current_user)?;

    let updated = sqlx::query_scalar::<_, Uuid>(
        "UPDATE runtime_alerts
         SET status = 'resolved',
             resolved_at = COALESCE(resolved_at, NOW()),
             resolved_by = $2,
             updated_at = NOW()
         WHERE id = $1
           AND status IN ('open', 'acknowledged')
         RETURNING id",
    )
    .bind(alert_id)
    .bind(current_user.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest(
        "runtime alert not found or already resolved".to_string(),
    ))?;

    let item = load_runtime_alert_item(state.as_ref(), updated).await?;
    let note = payload.as_ref().and_then(|Json(body)| {
        body.note
            .as_deref()
            .and_then(normalize_optional_text)
            .map(str::to_string)
    });
    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.runtime.alert.resolve",
        "runtime_alert",
        Some(item.id),
        json!({
            "alert_type": item.alert_type,
            "severity": item.severity,
            "status": item.status,
            "note": note
        }),
    )
    .await;

    Ok(Json(item))
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

pub(crate) async fn run_runtime_alert_scan(state: &AppState) -> AppResult<RuntimeAlertScanSummary> {
    let (upserted, auto_resolved) = scan_runtime_alerts_internal(state).await?;
    let counts = load_runtime_alert_counts(state).await?;

    Ok(RuntimeAlertScanSummary {
        upserted,
        auto_resolved,
        open_count: counts.open_count,
        acknowledged_count: counts.acknowledged_count,
        resolved_count: counts.resolved_count,
    })
}

async fn scan_runtime_alerts_internal(state: &AppState) -> AppResult<(i64, i64)> {
    let candidates = collect_runtime_alert_candidates(state).await?;
    let mut fingerprints = HashSet::new();
    let mut upserted = 0_i64;

    for candidate in candidates {
        if upsert_runtime_alert_candidate(state, &candidate).await? {
            upserted += 1;
        }
        fingerprints.insert(candidate.fingerprint);
    }

    let active_fingerprints = fingerprints.into_iter().collect::<Vec<_>>();
    let auto_resolved = auto_resolve_scanner_runtime_alerts(state, &active_fingerprints).await?;

    Ok((upserted, auto_resolved))
}

async fn collect_runtime_alert_candidates(state: &AppState) -> AppResult<Vec<RuntimeAlertCandidate>> {
    let mut candidates = Vec::new();

    let failed_rows = sqlx::query_as::<_, RuntimeAlertSignalInstanceRow>(
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
         ORDER BY i.updated_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    for row in failed_rows {
        let message = format!(
            "实例 {} / {} / {} 处于 failed 状态，最后更新时间 {}",
            row.contest_title, row.challenge_title, row.team_name, row.updated_at
        );
        candidates.push(build_runtime_alert_candidate(
            RUNTIME_ALERT_TYPE_INSTANCE_FAILED,
            "critical",
            "实例运行失败",
            message,
            &row,
        ));
    }

    let expiring_rows = sqlx::query_as::<_, RuntimeAlertSignalInstanceRow>(
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
         WHERE i.status = 'running'
           AND i.expires_at IS NOT NULL
           AND i.expires_at > NOW()
           AND i.expires_at <= NOW() + INTERVAL '30 minutes'
         ORDER BY i.expires_at ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    for row in expiring_rows {
        if let Some(expires_at) = row.expires_at {
            let message = format!(
                "实例 {} / {} / {} 将在 {} 过期",
                row.contest_title, row.challenge_title, row.team_name, expires_at
            );
            candidates.push(build_runtime_alert_candidate(
                RUNTIME_ALERT_TYPE_INSTANCE_EXPIRING_SOON,
                "warning",
                "实例即将过期",
                message,
                &row,
            ));
        }
    }

    let expired_rows = sqlx::query_as::<_, RuntimeAlertSignalInstanceRow>(
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
         WHERE i.status <> 'destroyed'
           AND i.expires_at IS NOT NULL
           AND i.expires_at <= NOW()
         ORDER BY i.expires_at ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    for row in expired_rows {
        if let Some(expires_at) = row.expires_at {
            let message = format!(
                "实例 {} / {} / {} 已在 {} 过期但尚未销毁（当前状态：{}）",
                row.contest_title, row.challenge_title, row.team_name, expires_at, row.status
            );
            candidates.push(build_runtime_alert_candidate(
                RUNTIME_ALERT_TYPE_INSTANCE_EXPIRED_NOT_DESTROYED,
                "critical",
                "实例过期未回收",
                message,
                &row,
            ));
        }
    }

    let stale_heartbeat_rows = sqlx::query_as::<_, RuntimeAlertSignalInstanceRow>(
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
         WHERE i.status = 'running'
           AND i.last_heartbeat_at IS NOT NULL
           AND i.last_heartbeat_at <= NOW() - INTERVAL '5 minutes'
         ORDER BY i.last_heartbeat_at ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    for row in stale_heartbeat_rows {
        if let Some(last_heartbeat_at) = row.last_heartbeat_at {
            let message = format!(
                "实例 {} / {} / {} 心跳停留在 {}，可能存在异常",
                row.contest_title, row.challenge_title, row.team_name, last_heartbeat_at
            );
            candidates.push(build_runtime_alert_candidate(
                RUNTIME_ALERT_TYPE_INSTANCE_HEARTBEAT_STALE,
                "warning",
                "实例心跳超时",
                message,
                &row,
            ));
        }
    }

    Ok(candidates)
}

fn build_runtime_alert_candidate(
    alert_type: &str,
    severity: &str,
    title: &str,
    message: String,
    row: &RuntimeAlertSignalInstanceRow,
) -> RuntimeAlertCandidate {
    RuntimeAlertCandidate {
        alert_type: alert_type.to_string(),
        severity: severity.to_string(),
        source_type: RUNTIME_ALERT_SOURCE_INSTANCE.to_string(),
        source_id: Some(row.id),
        fingerprint: format!("{}:{}", alert_type, row.id),
        title: title.to_string(),
        message,
        detail: json!({
            "instance_id": row.id,
            "contest_id": row.contest_id,
            "contest_title": row.contest_title,
            "challenge_id": row.challenge_id,
            "challenge_title": row.challenge_title,
            "team_id": row.team_id,
            "team_name": row.team_name,
            "status": row.status,
            "expires_at": row.expires_at,
            "last_heartbeat_at": row.last_heartbeat_at,
            "updated_at": row.updated_at
        }),
    }
}

async fn upsert_runtime_alert_candidate(
    state: &AppState,
    candidate: &RuntimeAlertCandidate,
) -> AppResult<bool> {
    let existing_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id
         FROM runtime_alerts
         WHERE fingerprint = $1
           AND status IN ('open', 'acknowledged')
         LIMIT 1",
    )
    .bind(&candidate.fingerprint)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?;

    if let Some(alert_id) = existing_id {
        sqlx::query(
            "UPDATE runtime_alerts
             SET severity = $2,
                 source_type = $3,
                 source_id = $4,
                 title = $5,
                 message = $6,
                 detail = $7,
                 last_seen_at = NOW(),
                 updated_at = NOW()
             WHERE id = $1",
        )
        .bind(alert_id)
        .bind(&candidate.severity)
        .bind(&candidate.source_type)
        .bind(candidate.source_id)
        .bind(&candidate.title)
        .bind(&candidate.message)
        .bind(&candidate.detail)
        .execute(&state.db)
        .await
        .map_err(AppError::internal)?;

        Ok(false)
    } else {
        sqlx::query(
            "INSERT INTO runtime_alerts (
                alert_type,
                severity,
                status,
                source_type,
                source_id,
                fingerprint,
                title,
                message,
                detail,
                first_seen_at,
                last_seen_at
             )
             VALUES ($1, $2, 'open', $3, $4, $5, $6, $7, $8, NOW(), NOW())",
        )
        .bind(&candidate.alert_type)
        .bind(&candidate.severity)
        .bind(&candidate.source_type)
        .bind(candidate.source_id)
        .bind(&candidate.fingerprint)
        .bind(&candidate.title)
        .bind(&candidate.message)
        .bind(&candidate.detail)
        .execute(&state.db)
        .await
        .map_err(AppError::internal)?;

        Ok(true)
    }
}

async fn auto_resolve_scanner_runtime_alerts(
    state: &AppState,
    active_fingerprints: &[String],
) -> AppResult<i64> {
    let scanner_types = RUNTIME_ALERT_SCANNER_TYPES
        .iter()
        .map(|item| item.to_string())
        .collect::<Vec<_>>();

    let result = if active_fingerprints.is_empty() {
        sqlx::query(
            "UPDATE runtime_alerts
             SET status = 'resolved',
                 resolved_at = COALESCE(resolved_at, NOW()),
                 resolved_by = NULL,
                 updated_at = NOW()
             WHERE status IN ('open', 'acknowledged')
               AND alert_type = ANY($1::text[])",
        )
        .bind(&scanner_types)
        .execute(&state.db)
        .await
        .map_err(AppError::internal)?
    } else {
        sqlx::query(
            "UPDATE runtime_alerts
             SET status = 'resolved',
                 resolved_at = COALESCE(resolved_at, NOW()),
                 resolved_by = NULL,
                 updated_at = NOW()
             WHERE status IN ('open', 'acknowledged')
               AND alert_type = ANY($1::text[])
               AND NOT (fingerprint = ANY($2::text[]))",
        )
        .bind(&scanner_types)
        .bind(active_fingerprints)
        .execute(&state.db)
        .await
        .map_err(AppError::internal)?
    };

    Ok(result.rows_affected() as i64)
}

async fn load_runtime_alert_counts(state: &AppState) -> AppResult<RuntimeAlertCountsRow> {
    sqlx::query_as::<_, RuntimeAlertCountsRow>(
        "SELECT COUNT(*) FILTER (WHERE status = 'open')::bigint AS open_count,
                COUNT(*) FILTER (WHERE status = 'acknowledged')::bigint AS acknowledged_count,
                COUNT(*) FILTER (WHERE status = 'resolved')::bigint AS resolved_count
         FROM runtime_alerts",
    )
    .fetch_one(&state.db)
    .await
    .map_err(AppError::internal)
}

async fn load_runtime_alert_item(state: &AppState, alert_id: Uuid) -> AppResult<AdminRuntimeAlertItem> {
    sqlx::query_as::<_, AdminRuntimeAlertItem>(
        "SELECT a.id,
                a.alert_type,
                a.severity,
                a.status,
                a.source_type,
                a.source_id,
                a.fingerprint,
                a.title,
                a.message,
                a.detail,
                a.first_seen_at,
                a.last_seen_at,
                a.acknowledged_at,
                a.acknowledged_by,
                ack_user.username AS acknowledged_by_username,
                a.resolved_at,
                a.resolved_by,
                resolved_user.username AS resolved_by_username,
                a.created_at,
                a.updated_at
         FROM runtime_alerts a
         LEFT JOIN users ack_user ON ack_user.id = a.acknowledged_by
         LEFT JOIN users resolved_user ON resolved_user.id = a.resolved_by
         WHERE a.id = $1
         LIMIT 1",
    )
    .bind(alert_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("runtime alert not found".to_string()))
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

fn ensure_admin(user: &AuthenticatedUser) -> AppResult<()> {
    if user.role == "admin" {
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

fn normalize_optional_text(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn default_challenge_status() -> String {
    "draft".to_string()
}

fn normalize_tags(tags: Vec<String>) -> AppResult<Vec<String>> {
    if tags.len() > 32 {
        return Err(AppError::BadRequest(
            "tags must be at most 32 items".to_string(),
        ));
    }

    let mut out: Vec<String> = Vec::new();
    for tag in tags {
        let normalized = tag.trim().to_lowercase();
        if normalized.is_empty() {
            continue;
        }
        if normalized.chars().count() > 32 {
            return Err(AppError::BadRequest(
                "each tag must be at most 32 characters".to_string(),
            ));
        }
        if !out.iter().any(|item| item == &normalized) {
            out.push(normalized);
        }
    }

    Ok(out)
}

fn challenge_item_from_snapshot_row(row: &ChallengeSnapshotRow) -> AdminChallengeItem {
    AdminChallengeItem {
        id: row.id,
        title: row.title.clone(),
        slug: row.slug.clone(),
        category: row.category.clone(),
        difficulty: row.difficulty.clone(),
        static_score: row.static_score,
        challenge_type: row.challenge_type.clone(),
        flag_mode: row.flag_mode.clone(),
        status: row.status.clone(),
        is_visible: row.is_visible,
        tags: row.tags.clone(),
        writeup_visibility: row.writeup_visibility.clone(),
        current_version: row.current_version,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn challenge_snapshot_to_value(row: &ChallengeSnapshotRow) -> Value {
    serde_json::to_value(ChallengeSnapshot {
        title: row.title.clone(),
        slug: row.slug.clone(),
        category: row.category.clone(),
        difficulty: row.difficulty.clone(),
        description: row.description.clone(),
        static_score: row.static_score,
        min_score: row.min_score,
        max_score: row.max_score,
        challenge_type: row.challenge_type.clone(),
        flag_mode: row.flag_mode.clone(),
        status: row.status.clone(),
        flag_hash: row.flag_hash.clone(),
        compose_template: row.compose_template.clone(),
        metadata: row.metadata.clone(),
        is_visible: row.is_visible,
        tags: row.tags.clone(),
        writeup_visibility: row.writeup_visibility.clone(),
        writeup_content: row.writeup_content.clone(),
    })
    .unwrap_or_else(|_| json!({}))
}

async fn ensure_challenge_exists(state: &AppState, challenge_id: Uuid) -> AppResult<()> {
    let exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM challenges WHERE id = $1)")
            .bind(challenge_id)
            .fetch_one(&state.db)
            .await
            .map_err(AppError::internal)?;

    if exists {
        Ok(())
    } else {
        Err(AppError::BadRequest("challenge not found".to_string()))
    }
}

fn challenge_attachments_dir(state: &AppState, challenge_id: Uuid) -> PathBuf {
    PathBuf::from(&state.config.instance_runtime_root)
        .join("_challenge_files")
        .join(challenge_id.to_string())
}

fn sanitize_filename(filename: &str) -> String {
    let mut out = String::with_capacity(filename.len());
    for ch in filename.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }

    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "attachment.bin".to_string()
    } else {
        trimmed.to_string()
    }
}

async fn load_challenge_attachment_item(
    state: &AppState,
    attachment_id: Uuid,
) -> AppResult<AdminChallengeAttachmentItem> {
    let row = sqlx::query_as::<_, AdminChallengeAttachmentItem>(
        "SELECT a.id,
                a.challenge_id,
                a.filename,
                a.content_type,
                a.storage_path,
                a.size_bytes,
                a.uploaded_by,
                u.username AS uploaded_by_username,
                a.created_at
         FROM challenge_attachments a
         LEFT JOIN users u ON u.id = a.uploaded_by
         WHERE a.id = $1
         LIMIT 1",
    )
    .bind(attachment_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest(
        "challenge attachment not found".to_string(),
    ))?;

    Ok(row)
}

async fn load_contest_announcement_item(
    state: &AppState,
    contest_id: Uuid,
    announcement_id: Uuid,
) -> AppResult<AdminContestAnnouncementItem> {
    let row = sqlx::query_as::<_, AdminContestAnnouncementItem>(
        "SELECT a.id,
                a.contest_id,
                a.title,
                a.content,
                a.is_published,
                a.is_pinned,
                a.published_at,
                a.created_by,
                cu.username AS created_by_username,
                a.updated_by,
                uu.username AS updated_by_username,
                a.created_at,
                a.updated_at
         FROM contest_announcements a
         LEFT JOIN users cu ON cu.id = a.created_by
         LEFT JOIN users uu ON uu.id = a.updated_by
         WHERE a.contest_id = $1
           AND a.id = $2
         LIMIT 1",
    )
    .bind(contest_id)
    .bind(announcement_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest(
        "contest announcement not found".to_string(),
    ))?;

    Ok(row)
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

fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("failed to hash password")))?
        .to_string();
    Ok(hash)
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
