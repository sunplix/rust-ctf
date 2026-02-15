use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
};

use axum::{
    extract::{Path as AxumPath, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tokio::{
    fs,
    process::Command,
    time::{timeout, Duration as TokioDuration},
};
use tracing::warn;
use uuid::Uuid;

use crate::{
    auth::AuthenticatedUser,
    error::{AppError, AppResult},
    state::AppState,
};

const INSTANCE_TTL_HOURS: i64 = 2;
const SUBNET_SECOND_OCTET_START: u16 = 16;
const SUBNET_SECOND_OCTET_END: u16 = 223;
const COMPOSE_FILE_NAME: &str = "docker-compose.generated.yml";

#[derive(Debug, Deserialize)]
struct InstanceActionRequest {
    contest_id: Uuid,
    challenge_id: Uuid,
}

#[derive(Debug, Serialize)]
struct InstanceResponse {
    id: Uuid,
    contest_id: Uuid,
    challenge_id: Uuid,
    team_id: Uuid,
    status: String,
    subnet: String,
    compose_project_name: String,
    entrypoint_url: String,
    cpu_limit: Option<String>,
    memory_limit_mb: Option<i32>,
    started_at: Option<DateTime<Utc>>,
    expires_at: Option<DateTime<Utc>>,
    destroyed_at: Option<DateTime<Utc>>,
    last_heartbeat_at: Option<DateTime<Utc>>,
    message: String,
}

#[derive(Debug, FromRow)]
struct TeamMembershipRow {
    team_id: Uuid,
}

#[derive(Debug, FromRow)]
struct RuntimePolicyRow {
    contest_status: String,
    contest_visibility: String,
    challenge_type: String,
    flag_mode: String,
    compose_template: Option<String>,
    is_visible: bool,
    release_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow)]
struct ComposeTemplateRow {
    compose_template: Option<String>,
    flag_mode: String,
}

#[derive(Debug, FromRow)]
struct InstanceRow {
    id: Uuid,
    contest_id: Uuid,
    challenge_id: Uuid,
    team_id: Uuid,
    status: String,
    subnet: String,
    compose_project_name: String,
    entrypoint_url: String,
    cpu_limit: Option<String>,
    memory_limit_mb: Option<i32>,
    started_at: Option<DateTime<Utc>>,
    expires_at: Option<DateTime<Utc>>,
    destroyed_at: Option<DateTime<Utc>>,
    last_heartbeat_at: Option<DateTime<Utc>>,
}

#[derive(Debug)]
struct ComposeRenderSource {
    template: String,
    flag_mode: String,
}

#[derive(Debug)]
enum ComposeCommandError {
    SpawnNotFound,
    Spawn(String),
    Timeout,
    Failed(String),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct InstanceReaperSummary {
    pub scanned: i64,
    pub reaped: i64,
    pub failed: i64,
    pub skipped: i64,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/instances/start", post(start_instance))
        .route("/instances/stop", post(stop_instance))
        .route("/instances/reset", post(reset_instance))
        .route("/instances/destroy", post(destroy_instance))
        .route("/instances/heartbeat", post(heartbeat_instance))
        .route(
            "/instances/{contest_id}/{challenge_id}",
            get(get_instance_by_challenge),
        )
}

async fn start_instance(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Json(req): Json<InstanceActionRequest>,
) -> AppResult<Json<InstanceResponse>> {
    let team_id = fetch_user_team_id(state.as_ref(), current_user.user_id).await?;
    let policy = fetch_runtime_policy(state.as_ref(), req.contest_id, req.challenge_id).await?;

    validate_runtime_policy(&policy, &current_user.role, true)?;
    let compose_source = compose_source_from_policy(policy)?;

    let now = Utc::now();
    let expires_at = now + Duration::hours(INSTANCE_TTL_HOURS);

    if let Some(instance) =
        fetch_instance_row(state.as_ref(), req.contest_id, req.challenge_id, team_id).await?
    {
        if instance.status == "running" && !is_expired(&instance, now) {
            return Ok(Json(instance_to_response(
                instance,
                "instance is already running".to_string(),
            )));
        }
    }

    let pending = ensure_instance_pending(
        state.as_ref(),
        req.contest_id,
        req.challenge_id,
        team_id,
        now,
        expires_at,
    )
    .await?;

    let compose_file = persist_compose_file(state.as_ref(), &pending, &compose_source).await?;
    if let Err(err) = compose_up(
        state.as_ref(),
        &pending.compose_project_name,
        &compose_file,
        false,
    )
    .await
    {
        let _ = update_instance_status(state.as_ref(), pending.id, "failed").await;
        return Err(err);
    }

    let running = mark_instance_running(state.as_ref(), pending.id, now, expires_at).await?;
    Ok(Json(instance_to_response(
        running,
        "instance started".to_string(),
    )))
}

async fn stop_instance(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Json(req): Json<InstanceActionRequest>,
) -> AppResult<Json<InstanceResponse>> {
    let team_id = fetch_user_team_id(state.as_ref(), current_user.user_id).await?;

    let instance = fetch_instance_row(state.as_ref(), req.contest_id, req.challenge_id, team_id)
        .await?
        .ok_or(AppError::BadRequest("instance not found".to_string()))?;

    if instance.status == "destroyed" {
        return Err(AppError::BadRequest(
            "instance has already been destroyed".to_string(),
        ));
    }

    let compose_file = ensure_compose_file_for_existing(state.as_ref(), &instance).await?;
    if let Err(err) = compose_stop(
        state.as_ref(),
        &instance.compose_project_name,
        &compose_file,
    )
    .await
    {
        let _ = update_instance_status(state.as_ref(), instance.id, "failed").await;
        return Err(err);
    }

    let updated = update_instance_status(state.as_ref(), instance.id, "stopped").await?;
    Ok(Json(instance_to_response(
        updated,
        "instance stopped".to_string(),
    )))
}

async fn reset_instance(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Json(req): Json<InstanceActionRequest>,
) -> AppResult<Json<InstanceResponse>> {
    let team_id = fetch_user_team_id(state.as_ref(), current_user.user_id).await?;
    let policy = fetch_runtime_policy(state.as_ref(), req.contest_id, req.challenge_id).await?;

    validate_runtime_policy(&policy, &current_user.role, true)?;
    let compose_source = compose_source_from_policy(policy)?;

    let now = Utc::now();
    let expires_at = now + Duration::hours(INSTANCE_TTL_HOURS);

    let pending = ensure_instance_pending(
        state.as_ref(),
        req.contest_id,
        req.challenge_id,
        team_id,
        now,
        expires_at,
    )
    .await?;

    let compose_file = persist_compose_file(state.as_ref(), &pending, &compose_source).await?;

    if let Err(err) =
        compose_down(state.as_ref(), &pending.compose_project_name, &compose_file).await
    {
        warn!(
            instance_id = %pending.id,
            contest_id = %pending.contest_id,
            challenge_id = %pending.challenge_id,
            team_id = %pending.team_id,
            error = %err,
            "compose down during reset failed; continue to up"
        );
    }

    if let Err(err) = compose_up(
        state.as_ref(),
        &pending.compose_project_name,
        &compose_file,
        true,
    )
    .await
    {
        let _ = update_instance_status(state.as_ref(), pending.id, "failed").await;
        return Err(err);
    }

    let running = mark_instance_running(state.as_ref(), pending.id, now, expires_at).await?;
    Ok(Json(instance_to_response(
        running,
        "instance reset to running state".to_string(),
    )))
}

async fn destroy_instance(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Json(req): Json<InstanceActionRequest>,
) -> AppResult<Json<InstanceResponse>> {
    let team_id = fetch_user_team_id(state.as_ref(), current_user.user_id).await?;

    let instance = fetch_instance_row(state.as_ref(), req.contest_id, req.challenge_id, team_id)
        .await?
        .ok_or(AppError::BadRequest("instance not found".to_string()))?;

    if instance.status != "destroyed" {
        let compose_file = ensure_compose_file_for_existing(state.as_ref(), &instance).await?;
        if let Err(err) = compose_down(
            state.as_ref(),
            &instance.compose_project_name,
            &compose_file,
        )
        .await
        {
            let _ = update_instance_status(state.as_ref(), instance.id, "failed").await;
            return Err(err);
        }
    }

    let updated = sqlx::query_as::<_, InstanceRow>(
        "UPDATE instances
         SET status = 'destroyed',
             destroyed_at = NOW(),
             expires_at = NULL,
             updated_at = NOW()
         WHERE id = $1
         RETURNING id,
                   contest_id,
                   challenge_id,
                   team_id,
                   status,
                   subnet::text AS subnet,
                   compose_project_name,
                   entrypoint_url,
                   cpu_limit::text AS cpu_limit,
                   memory_limit_mb,
                   started_at,
                   expires_at,
                   destroyed_at,
                   last_heartbeat_at",
    )
    .bind(instance.id)
    .fetch_one(&state.db)
    .await
    .map_err(AppError::internal)?;

    cleanup_runtime_dir(state.as_ref(), &updated.compose_project_name).await;

    Ok(Json(instance_to_response(
        updated,
        "instance destroyed".to_string(),
    )))
}

async fn heartbeat_instance(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Json(req): Json<InstanceActionRequest>,
) -> AppResult<Json<InstanceResponse>> {
    let team_id = fetch_user_team_id(state.as_ref(), current_user.user_id).await?;

    let updated =
        touch_instance_heartbeat(state.as_ref(), req.contest_id, req.challenge_id, team_id)
            .await?
            .ok_or(AppError::BadRequest(
                "running instance not found".to_string(),
            ))?;

    Ok(Json(instance_to_response(
        updated,
        "instance heartbeat updated".to_string(),
    )))
}

async fn get_instance_by_challenge(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    AxumPath((contest_id, challenge_id)): AxumPath<(Uuid, Uuid)>,
) -> AppResult<Json<InstanceResponse>> {
    let team_id = fetch_user_team_id(state.as_ref(), current_user.user_id).await?;

    let instance = fetch_instance_row(state.as_ref(), contest_id, challenge_id, team_id)
        .await?
        .ok_or(AppError::BadRequest("instance not found".to_string()))?;

    Ok(Json(instance_to_response(
        instance,
        "instance query result".to_string(),
    )))
}

async fn fetch_user_team_id(state: &AppState, user_id: Uuid) -> AppResult<Uuid> {
    let team = sqlx::query_as::<_, TeamMembershipRow>(
        "SELECT team_id FROM team_members WHERE user_id = $1 LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::Forbidden)?;

    Ok(team.team_id)
}

async fn fetch_runtime_policy(
    state: &AppState,
    contest_id: Uuid,
    challenge_id: Uuid,
) -> AppResult<RuntimePolicyRow> {
    sqlx::query_as::<_, RuntimePolicyRow>(
        "SELECT ct.status AS contest_status,
                ct.visibility AS contest_visibility,
                c.challenge_type,
                c.flag_mode,
                c.compose_template,
                c.is_visible,
                cc.release_at
         FROM contest_challenges cc
         JOIN contests ct ON ct.id = cc.contest_id
         JOIN challenges c ON c.id = cc.challenge_id
         WHERE cc.contest_id = $1 AND cc.challenge_id = $2
         LIMIT 1",
    )
    .bind(contest_id)
    .bind(challenge_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest(
        "challenge is not available in this contest".to_string(),
    ))
}

async fn fetch_compose_template_row(
    state: &AppState,
    contest_id: Uuid,
    challenge_id: Uuid,
) -> AppResult<ComposeTemplateRow> {
    sqlx::query_as::<_, ComposeTemplateRow>(
        "SELECT c.compose_template,
                c.flag_mode
         FROM contest_challenges cc
         JOIN challenges c ON c.id = cc.challenge_id
         WHERE cc.contest_id = $1 AND cc.challenge_id = $2
         LIMIT 1",
    )
    .bind(contest_id)
    .bind(challenge_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest(
        "challenge runtime template is missing".to_string(),
    ))
}

fn validate_runtime_policy(
    policy: &RuntimePolicyRow,
    user_role: &str,
    require_running: bool,
) -> AppResult<()> {
    let now = Utc::now();

    if policy.contest_visibility == "private" && !is_privileged_role(user_role) {
        return Err(AppError::Forbidden);
    }

    if !policy.is_visible {
        return Err(AppError::BadRequest(
            "challenge runtime is not visible".to_string(),
        ));
    }

    if let Some(release_at) = policy.release_at {
        if now < release_at {
            return Err(AppError::BadRequest(
                "challenge runtime has not been released yet".to_string(),
            ));
        }
    }

    if require_running && policy.contest_status != "running" && !is_privileged_role(user_role) {
        return Err(AppError::BadRequest("contest is not running".to_string()));
    }

    if policy
        .compose_template
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        return Err(AppError::BadRequest(
            "challenge runtime template is missing".to_string(),
        ));
    }

    if policy.challenge_type != "dynamic" && policy.challenge_type != "internal" {
        return Err(AppError::BadRequest(
            "challenge type does not require runtime instance".to_string(),
        ));
    }

    Ok(())
}

fn compose_source_from_policy(policy: RuntimePolicyRow) -> AppResult<ComposeRenderSource> {
    let template = policy
        .compose_template
        .unwrap_or_default()
        .trim()
        .to_string();

    if template.is_empty() {
        return Err(AppError::BadRequest(
            "challenge runtime template is missing".to_string(),
        ));
    }

    Ok(ComposeRenderSource {
        template,
        flag_mode: policy.flag_mode,
    })
}

fn compose_source_from_row(row: ComposeTemplateRow) -> AppResult<ComposeRenderSource> {
    let template = row.compose_template.unwrap_or_default().trim().to_string();
    if template.is_empty() {
        return Err(AppError::BadRequest(
            "challenge runtime template is missing".to_string(),
        ));
    }

    Ok(ComposeRenderSource {
        template,
        flag_mode: row.flag_mode,
    })
}

fn is_privileged_role(role: &str) -> bool {
    role == "admin" || role == "judge"
}

async fn fetch_instance_row(
    state: &AppState,
    contest_id: Uuid,
    challenge_id: Uuid,
    team_id: Uuid,
) -> AppResult<Option<InstanceRow>> {
    sqlx::query_as::<_, InstanceRow>(
        "SELECT id,
                contest_id,
                challenge_id,
                team_id,
                status,
                subnet::text AS subnet,
                compose_project_name,
                entrypoint_url,
                cpu_limit::text AS cpu_limit,
                memory_limit_mb,
                started_at,
                expires_at,
                destroyed_at,
                last_heartbeat_at
         FROM instances
         WHERE contest_id = $1 AND challenge_id = $2 AND team_id = $3
         LIMIT 1",
    )
    .bind(contest_id)
    .bind(challenge_id)
    .bind(team_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)
}

async fn ensure_instance_pending(
    state: &AppState,
    contest_id: Uuid,
    challenge_id: Uuid,
    team_id: Uuid,
    now: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> AppResult<InstanceRow> {
    let cpu_limit = default_instance_cpu_limit_text(state);
    let memory_limit_mb = default_instance_memory_limit_mb(state);

    match fetch_instance_row(state, contest_id, challenge_id, team_id).await? {
        Some(existing) => {
            mark_instance_creating(
                state,
                existing.id,
                now,
                expires_at,
                cpu_limit.as_deref(),
                memory_limit_mb,
            )
            .await
        }
        None => {
            let subnet = allocate_subnet(state, contest_id, challenge_id, team_id).await?;
            let compose_project_name = compose_project_name(contest_id, challenge_id, team_id);
            let entrypoint_url = default_entrypoint_url(&subnet);

            insert_instance_row(
                state,
                contest_id,
                challenge_id,
                team_id,
                &subnet,
                &compose_project_name,
                &entrypoint_url,
                now,
                expires_at,
                cpu_limit.as_deref(),
                memory_limit_mb,
            )
            .await
        }
    }
}

async fn insert_instance_row(
    state: &AppState,
    contest_id: Uuid,
    challenge_id: Uuid,
    team_id: Uuid,
    subnet: &str,
    compose_project_name: &str,
    entrypoint_url: &str,
    now: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    cpu_limit: Option<&str>,
    memory_limit_mb: Option<i32>,
) -> AppResult<InstanceRow> {
    sqlx::query_as::<_, InstanceRow>(
        "INSERT INTO instances (
            contest_id,
            challenge_id,
            team_id,
            subnet,
            compose_project_name,
            status,
            entrypoint_url,
            cpu_limit,
            memory_limit_mb,
            started_at,
            expires_at,
            created_at,
            updated_at
         )
         VALUES ($1, $2, $3, $4::cidr, $5, 'creating', $6, $7::numeric, $8, $9, $10, NOW(), NOW())
         RETURNING id,
                   contest_id,
                   challenge_id,
                   team_id,
                   status,
                   subnet::text AS subnet,
                   compose_project_name,
                   entrypoint_url,
                   cpu_limit::text AS cpu_limit,
                   memory_limit_mb,
                   started_at,
                   expires_at,
                   destroyed_at,
                   last_heartbeat_at",
    )
    .bind(contest_id)
    .bind(challenge_id)
    .bind(team_id)
    .bind(subnet)
    .bind(compose_project_name)
    .bind(entrypoint_url)
    .bind(cpu_limit)
    .bind(memory_limit_mb)
    .bind(now)
    .bind(expires_at)
    .fetch_one(&state.db)
    .await
    .map_err(AppError::internal)
}

async fn mark_instance_creating(
    state: &AppState,
    instance_id: Uuid,
    now: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    cpu_limit: Option<&str>,
    memory_limit_mb: Option<i32>,
) -> AppResult<InstanceRow> {
    sqlx::query_as::<_, InstanceRow>(
        "UPDATE instances
         SET status = 'creating',
             started_at = $2,
             expires_at = $3,
             cpu_limit = $4::numeric,
             memory_limit_mb = $5,
             destroyed_at = NULL,
             updated_at = NOW()
         WHERE id = $1
         RETURNING id,
                   contest_id,
                   challenge_id,
                   team_id,
                   status,
                   subnet::text AS subnet,
                   compose_project_name,
                   entrypoint_url,
                   cpu_limit::text AS cpu_limit,
                   memory_limit_mb,
                   started_at,
                   expires_at,
                   destroyed_at,
                   last_heartbeat_at",
    )
    .bind(instance_id)
    .bind(now)
    .bind(expires_at)
    .bind(cpu_limit)
    .bind(memory_limit_mb)
    .fetch_one(&state.db)
    .await
    .map_err(AppError::internal)
}

async fn mark_instance_running(
    state: &AppState,
    instance_id: Uuid,
    now: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> AppResult<InstanceRow> {
    sqlx::query_as::<_, InstanceRow>(
        "UPDATE instances
         SET status = 'running',
             started_at = $2,
             expires_at = $3,
             destroyed_at = NULL,
             updated_at = NOW()
         WHERE id = $1
         RETURNING id,
                   contest_id,
                   challenge_id,
                   team_id,
                   status,
                   subnet::text AS subnet,
                   compose_project_name,
                   entrypoint_url,
                   cpu_limit::text AS cpu_limit,
                   memory_limit_mb,
                   started_at,
                   expires_at,
                   destroyed_at,
                   last_heartbeat_at",
    )
    .bind(instance_id)
    .bind(now)
    .bind(expires_at)
    .fetch_one(&state.db)
    .await
    .map_err(AppError::internal)
}

async fn update_instance_status(
    state: &AppState,
    instance_id: Uuid,
    next_status: &str,
) -> AppResult<InstanceRow> {
    sqlx::query_as::<_, InstanceRow>(
        "UPDATE instances
         SET status = $2,
             updated_at = NOW()
         WHERE id = $1
         RETURNING id,
                   contest_id,
                   challenge_id,
                   team_id,
                   status,
                   subnet::text AS subnet,
                   compose_project_name,
                   entrypoint_url,
                   cpu_limit::text AS cpu_limit,
                   memory_limit_mb,
                   started_at,
                   expires_at,
                   destroyed_at,
                   last_heartbeat_at",
    )
    .bind(instance_id)
    .bind(next_status)
    .fetch_one(&state.db)
    .await
    .map_err(AppError::internal)
}

async fn touch_instance_heartbeat(
    state: &AppState,
    contest_id: Uuid,
    challenge_id: Uuid,
    team_id: Uuid,
) -> AppResult<Option<InstanceRow>> {
    sqlx::query_as::<_, InstanceRow>(
        "UPDATE instances
         SET last_heartbeat_at = NOW(),
             updated_at = NOW()
         WHERE contest_id = $1
           AND challenge_id = $2
           AND team_id = $3
           AND status = 'running'
         RETURNING id,
                   contest_id,
                   challenge_id,
                   team_id,
                   status,
                   subnet::text AS subnet,
                   compose_project_name,
                   entrypoint_url,
                   cpu_limit::text AS cpu_limit,
                   memory_limit_mb,
                   started_at,
                   expires_at,
                   destroyed_at,
                   last_heartbeat_at",
    )
    .bind(contest_id)
    .bind(challenge_id)
    .bind(team_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)
}

async fn fetch_expired_instance_candidates(
    state: &AppState,
    limit: i64,
) -> AppResult<Vec<InstanceRow>> {
    sqlx::query_as::<_, InstanceRow>(
        "SELECT id,
                contest_id,
                challenge_id,
                team_id,
                status,
                subnet::text AS subnet,
                compose_project_name,
                entrypoint_url,
                cpu_limit::text AS cpu_limit,
                memory_limit_mb,
                started_at,
                expires_at,
                destroyed_at,
                last_heartbeat_at
         FROM instances
         WHERE status <> 'destroyed'
           AND expires_at IS NOT NULL
           AND expires_at <= NOW()
         ORDER BY expires_at ASC, updated_at ASC
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)
}

async fn fetch_stale_instance_candidates(
    state: &AppState,
    stale_after_seconds: i64,
    limit: i64,
) -> AppResult<Vec<InstanceRow>> {
    sqlx::query_as::<_, InstanceRow>(
        "SELECT id,
                contest_id,
                challenge_id,
                team_id,
                status,
                subnet::text AS subnet,
                compose_project_name,
                entrypoint_url,
                cpu_limit::text AS cpu_limit,
                memory_limit_mb,
                started_at,
                expires_at,
                destroyed_at,
                last_heartbeat_at
         FROM instances
         WHERE status = 'running'
           AND last_heartbeat_at IS NOT NULL
           AND last_heartbeat_at <= NOW() - ($1::bigint * INTERVAL '1 second')
           AND (expires_at IS NULL OR expires_at > NOW())
         ORDER BY last_heartbeat_at ASC
         LIMIT $2",
    )
    .bind(stale_after_seconds)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)
}

async fn mark_instance_destroyed_if_expired(
    state: &AppState,
    instance_id: Uuid,
) -> AppResult<Option<InstanceRow>> {
    sqlx::query_as::<_, InstanceRow>(
        "UPDATE instances
         SET status = 'destroyed',
             destroyed_at = NOW(),
             expires_at = NULL,
             updated_at = NOW()
         WHERE id = $1
           AND status <> 'destroyed'
           AND expires_at IS NOT NULL
           AND expires_at <= NOW()
         RETURNING id,
                   contest_id,
                   challenge_id,
                   team_id,
                   status,
                   subnet::text AS subnet,
                   compose_project_name,
                   entrypoint_url,
                   cpu_limit::text AS cpu_limit,
                   memory_limit_mb,
                   started_at,
                   expires_at,
                   destroyed_at,
                   last_heartbeat_at",
    )
    .bind(instance_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)
}

async fn mark_instance_destroyed_if_stale(
    state: &AppState,
    instance_id: Uuid,
    stale_after_seconds: i64,
) -> AppResult<Option<InstanceRow>> {
    sqlx::query_as::<_, InstanceRow>(
        "UPDATE instances
         SET status = 'destroyed',
             destroyed_at = NOW(),
             expires_at = NULL,
             updated_at = NOW()
         WHERE id = $1
           AND status = 'running'
           AND last_heartbeat_at IS NOT NULL
           AND last_heartbeat_at <= NOW() - ($2::bigint * INTERVAL '1 second')
         RETURNING id,
                   contest_id,
                   challenge_id,
                   team_id,
                   status,
                   subnet::text AS subnet,
                   compose_project_name,
                   entrypoint_url,
                   cpu_limit::text AS cpu_limit,
                   memory_limit_mb,
                   started_at,
                   expires_at,
                   destroyed_at,
                   last_heartbeat_at",
    )
    .bind(instance_id)
    .bind(stale_after_seconds)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)
}

pub(crate) async fn run_expired_instance_reaper(
    state: &AppState,
    batch_size: i64,
) -> AppResult<InstanceReaperSummary> {
    let limit = batch_size.clamp(1, 500);
    let candidates = fetch_expired_instance_candidates(state, limit).await?;

    let mut scanned = 0_i64;
    let mut reaped = 0_i64;
    let mut failed = 0_i64;
    let mut skipped = 0_i64;

    for instance in candidates {
        scanned += 1;

        let compose_file = match ensure_compose_file_for_existing(state, &instance).await {
            Ok(path) => path,
            Err(err) => {
                failed += 1;
                warn!(
                    instance_id = %instance.id,
                    contest_id = %instance.contest_id,
                    challenge_id = %instance.challenge_id,
                    team_id = %instance.team_id,
                    error = %err,
                    "instance reaper failed to prepare compose file"
                );
                let _ = update_instance_status(state, instance.id, "failed").await;
                continue;
            }
        };

        if let Err(err) = compose_down(state, &instance.compose_project_name, &compose_file).await {
            failed += 1;
            warn!(
                instance_id = %instance.id,
                contest_id = %instance.contest_id,
                challenge_id = %instance.challenge_id,
                team_id = %instance.team_id,
                compose_project_name = %instance.compose_project_name,
                error = %err,
                "instance reaper failed during compose down"
            );
            let _ = update_instance_status(state, instance.id, "failed").await;
            continue;
        }

        match mark_instance_destroyed_if_expired(state, instance.id).await? {
            Some(updated) => {
                cleanup_runtime_dir(state, &updated.compose_project_name).await;
                reaped += 1;
            }
            None => {
                skipped += 1;
            }
        }
    }

    Ok(InstanceReaperSummary {
        scanned,
        reaped,
        failed,
        skipped,
    })
}

pub(crate) async fn run_stale_instance_reaper(
    state: &AppState,
    stale_after_seconds: i64,
    batch_size: i64,
) -> AppResult<InstanceReaperSummary> {
    let stale_after_seconds = stale_after_seconds.clamp(60, 86_400);
    let limit = batch_size.clamp(1, 500);
    let candidates = fetch_stale_instance_candidates(state, stale_after_seconds, limit).await?;

    let mut scanned = 0_i64;
    let mut reaped = 0_i64;
    let mut failed = 0_i64;
    let mut skipped = 0_i64;

    for instance in candidates {
        scanned += 1;

        let compose_file = match ensure_compose_file_for_existing(state, &instance).await {
            Ok(path) => path,
            Err(err) => {
                failed += 1;
                warn!(
                    instance_id = %instance.id,
                    contest_id = %instance.contest_id,
                    challenge_id = %instance.challenge_id,
                    team_id = %instance.team_id,
                    error = %err,
                    "stale instance reaper failed to prepare compose file"
                );
                let _ = update_instance_status(state, instance.id, "failed").await;
                continue;
            }
        };

        if let Err(err) = compose_down(state, &instance.compose_project_name, &compose_file).await {
            failed += 1;
            warn!(
                instance_id = %instance.id,
                contest_id = %instance.contest_id,
                challenge_id = %instance.challenge_id,
                team_id = %instance.team_id,
                compose_project_name = %instance.compose_project_name,
                error = %err,
                "stale instance reaper failed during compose down"
            );
            let _ = update_instance_status(state, instance.id, "failed").await;
            continue;
        }

        match mark_instance_destroyed_if_stale(state, instance.id, stale_after_seconds).await? {
            Some(updated) => {
                cleanup_runtime_dir(state, &updated.compose_project_name).await;
                reaped += 1;
            }
            None => {
                skipped += 1;
            }
        }
    }

    Ok(InstanceReaperSummary {
        scanned,
        reaped,
        failed,
        skipped,
    })
}

async fn allocate_subnet(
    state: &AppState,
    contest_id: Uuid,
    challenge_id: Uuid,
    team_id: Uuid,
) -> AppResult<String> {
    let span = (SUBNET_SECOND_OCTET_END - SUBNET_SECOND_OCTET_START + 1) as usize * 256;
    let seed = subnet_seed(contest_id, challenge_id, team_id) as usize;

    for offset in 0..span {
        let idx = (seed + offset) % span;
        let second = SUBNET_SECOND_OCTET_START + (idx / 256) as u16;
        let third = (idx % 256) as u16;
        let candidate = format!("10.{}.{}.0/24", second, third);

        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM instances WHERE subnet = $1::cidr)",
        )
        .bind(&candidate)
        .fetch_one(&state.db)
        .await
        .map_err(AppError::internal)?;

        if !exists {
            return Ok(candidate);
        }
    }

    Err(AppError::internal(anyhow::anyhow!(
        "no available subnet in configured pool"
    )))
}

fn subnet_seed(contest_id: Uuid, challenge_id: Uuid, team_id: Uuid) -> u32 {
    let mut seed = 0_u32;
    for byte in contest_id
        .as_bytes()
        .iter()
        .chain(challenge_id.as_bytes())
        .chain(team_id.as_bytes())
    {
        seed = seed.rotate_left(5) ^ (*byte as u32);
    }
    seed
}

fn compose_project_name(contest_id: Uuid, challenge_id: Uuid, team_id: Uuid) -> String {
    let contest = contest_id.as_simple().to_string();
    let challenge = challenge_id.as_simple().to_string();
    let team = team_id.as_simple().to_string();

    let mut name = format!("ctf_{}_{}_{}", &contest[..8], &challenge[..8], &team[..8]);

    if name.len() > 96 {
        name.truncate(96);
    }
    name
}

fn default_entrypoint_url(subnet: &str) -> String {
    match subnet_host_ip(subnet, 2) {
        Some(host) => format!("http://{}", host),
        None => "".to_string(),
    }
}

fn default_instance_cpu_limit_text(state: &AppState) -> Option<String> {
    let cpu_limit = state.config.instance_default_cpu_limit;
    if cpu_limit <= 0.0 {
        return None;
    }

    Some(format!("{:.2}", cpu_limit.clamp(0.10, 64.00)))
}

fn default_instance_memory_limit_mb(state: &AppState) -> Option<i32> {
    let memory_limit_mb = state.config.instance_default_memory_limit_mb;
    if memory_limit_mb <= 0 {
        return None;
    }

    Some(memory_limit_mb.clamp(64, 1_048_576) as i32)
}

fn subnet_host_ip(subnet: &str, host_octet: u8) -> Option<String> {
    let base = subnet.split('/').next()?;
    let mut parts = base.split('.');

    let first = parts.next()?.parse::<u8>().ok()?;
    let second = parts.next()?.parse::<u8>().ok()?;
    let third = parts.next()?.parse::<u8>().ok()?;

    Some(format!("{}.{}.{}.{}", first, second, third, host_octet))
}

fn runtime_root_path(config_root: &str) -> PathBuf {
    let root = PathBuf::from(config_root);
    if root.is_absolute() {
        return root;
    }

    match std::env::current_dir() {
        Ok(cwd) => cwd.join(root),
        Err(_) => PathBuf::from(config_root),
    }
}

fn runtime_project_dir(state: &AppState, compose_project_name: &str) -> PathBuf {
    runtime_root_path(&state.config.instance_runtime_root).join(compose_project_name)
}

fn compose_file_path(state: &AppState, compose_project_name: &str) -> PathBuf {
    runtime_project_dir(state, compose_project_name).join(COMPOSE_FILE_NAME)
}

fn render_compose_template(
    source: &str,
    instance: &InstanceRow,
    dynamic_flag: Option<&str>,
) -> String {
    let network_name = format!("{}_net", instance.compose_project_name);
    let team_id = instance.team_id.to_string();
    let contest_id = instance.contest_id.to_string();
    let challenge_id = instance.challenge_id.to_string();
    let entrypoint_host = subnet_host_ip(&instance.subnet, 2).unwrap_or_default();
    let gateway_ip = subnet_host_ip(&instance.subnet, 1).unwrap_or_default();
    let cpu_limit = instance.cpu_limit.clone().unwrap_or_default();
    let memory_limit_mb = instance
        .memory_limit_mb
        .map(|value| value.to_string())
        .unwrap_or_default();
    let memory_limit = instance
        .memory_limit_mb
        .map(|value| format!("{}m", value))
        .unwrap_or_default();

    let mut rendered = source.to_string();
    let replacements = [
        ("{{PROJECT_NAME}}", instance.compose_project_name.as_str()),
        (
            "{{COMPOSE_PROJECT_NAME}}",
            instance.compose_project_name.as_str(),
        ),
        ("{{NETWORK_NAME}}", network_name.as_str()),
        ("{{SUBNET}}", instance.subnet.as_str()),
        ("{{SUBNET_CIDR}}", instance.subnet.as_str()),
        ("{{TEAM_ID}}", team_id.as_str()),
        ("{{CONTEST_ID}}", contest_id.as_str()),
        ("{{CHALLENGE_ID}}", challenge_id.as_str()),
        ("{{ENTRYPOINT_URL}}", instance.entrypoint_url.as_str()),
        ("{{ENTRYPOINT_HOST}}", entrypoint_host.as_str()),
        ("{{GATEWAY_IP}}", gateway_ip.as_str()),
        ("{{CPU_LIMIT}}", cpu_limit.as_str()),
        ("{{MEMORY_LIMIT_MB}}", memory_limit_mb.as_str()),
        ("{{MEMORY_LIMIT}}", memory_limit.as_str()),
    ];

    for (token, value) in replacements {
        rendered = rendered.replace(token, value);
    }

    rendered = rendered.replace("{{DYNAMIC_FLAG}}", dynamic_flag.unwrap_or(""));
    rendered = rendered.replace("{{FLAG}}", dynamic_flag.unwrap_or(""));

    rendered
}

fn apply_compose_resource_limits(
    compose_text: &str,
    cpu_limit: Option<&str>,
    memory_limit_mb: Option<i32>,
) -> String {
    if cpu_limit.is_none() && memory_limit_mb.is_none() {
        return compose_text.to_string();
    }

    let mut value: serde_yaml::Value = match serde_yaml::from_str(compose_text) {
        Ok(value) => value,
        Err(err) => {
            warn!(error = %err, "failed to parse compose yaml for resource limit injection");
            return compose_text.to_string();
        }
    };

    let Some(root_map) = value.as_mapping_mut() else {
        return compose_text.to_string();
    };

    let services_key = serde_yaml::Value::String("services".to_string());
    let Some(services_map) = root_map
        .get_mut(&services_key)
        .and_then(serde_yaml::Value::as_mapping_mut)
    else {
        return compose_text.to_string();
    };

    for (_, service_value) in services_map.iter_mut() {
        let Some(service_map) = service_value.as_mapping_mut() else {
            continue;
        };

        if let Some(cpu) = cpu_limit {
            service_map.insert(
                serde_yaml::Value::String("cpus".to_string()),
                serde_yaml::Value::String(cpu.to_string()),
            );
        }

        if let Some(memory_mb) = memory_limit_mb {
            service_map.insert(
                serde_yaml::Value::String("mem_limit".to_string()),
                serde_yaml::Value::String(format!("{}m", memory_mb)),
            );
        }
    }

    match serde_yaml::to_string(&value) {
        Ok(rendered) => rendered,
        Err(err) => {
            warn!(error = %err, "failed to serialize compose yaml after resource limit injection");
            compose_text.to_string()
        }
    }
}

async fn provision_dynamic_flag_if_needed(
    state: &AppState,
    flag_mode: &str,
    contest_id: Uuid,
    challenge_id: Uuid,
    team_id: Uuid,
) -> AppResult<Option<String>> {
    if flag_mode != "dynamic" {
        return Ok(None);
    }

    let key = format!("flag:dynamic:{}:{}:{}", contest_id, challenge_id, team_id);
    let mut redis_conn = state.redis.clone();

    let existing: Option<String> = redis_conn.get(&key).await.map_err(AppError::internal)?;
    if existing.is_some() {
        return Ok(existing);
    }

    let challenge_prefix = challenge_id.as_simple().to_string();
    let random_part = Uuid::new_v4().as_simple().to_string();
    let generated = format!("ctf{{{}-{}}}", &challenge_prefix[..8], &random_part[..12]);

    let _: () = redis_conn
        .set(&key, generated.as_str())
        .await
        .map_err(AppError::internal)?;

    Ok(Some(generated))
}

async fn persist_compose_file(
    state: &AppState,
    instance: &InstanceRow,
    source: &ComposeRenderSource,
) -> AppResult<PathBuf> {
    let dynamic_flag = provision_dynamic_flag_if_needed(
        state,
        &source.flag_mode,
        instance.contest_id,
        instance.challenge_id,
        instance.team_id,
    )
    .await?;

    let rendered = render_compose_template(&source.template, instance, dynamic_flag.as_deref());
    let rendered = apply_compose_resource_limits(
        &rendered,
        instance.cpu_limit.as_deref(),
        instance.memory_limit_mb,
    );
    let compose_file = compose_file_path(state, &instance.compose_project_name);

    if let Some(parent) = compose_file.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(AppError::internal)?;
    }

    fs::write(&compose_file, rendered)
        .await
        .map_err(AppError::internal)?;

    Ok(compose_file)
}

async fn ensure_compose_file_for_existing(
    state: &AppState,
    instance: &InstanceRow,
) -> AppResult<PathBuf> {
    let compose_file = compose_file_path(state, &instance.compose_project_name);
    if fs::metadata(&compose_file).await.is_ok() {
        return Ok(compose_file);
    }

    let row = fetch_compose_template_row(state, instance.contest_id, instance.challenge_id).await?;
    let source = compose_source_from_row(row)?;
    persist_compose_file(state, instance, &source).await
}

async fn compose_up(
    state: &AppState,
    project_name: &str,
    compose_file: &Path,
    force_recreate: bool,
) -> AppResult<()> {
    if force_recreate {
        run_compose_command(
            state,
            project_name,
            compose_file,
            &["up", "-d", "--force-recreate", "--remove-orphans"],
            "instance start",
        )
        .await
    } else {
        run_compose_command(
            state,
            project_name,
            compose_file,
            &["up", "-d", "--remove-orphans"],
            "instance start",
        )
        .await
    }
}

async fn compose_stop(state: &AppState, project_name: &str, compose_file: &Path) -> AppResult<()> {
    run_compose_command(
        state,
        project_name,
        compose_file,
        &["stop"],
        "instance stop",
    )
    .await
}

async fn compose_down(state: &AppState, project_name: &str, compose_file: &Path) -> AppResult<()> {
    run_compose_command(
        state,
        project_name,
        compose_file,
        &["down", "--volumes", "--remove-orphans"],
        "instance destroy",
    )
    .await
}

async fn run_compose_command(
    state: &AppState,
    project_name: &str,
    compose_file: &Path,
    action_args: &[&str],
    action_name: &str,
) -> AppResult<()> {
    let compose_file = compose_file
        .to_str()
        .ok_or_else(|| AppError::internal(anyhow::anyhow!("compose path is not utf-8")))?
        .to_string();

    let mut primary_args = vec![
        "compose".to_string(),
        "-f".to_string(),
        compose_file.clone(),
        "-p".to_string(),
        project_name.to_string(),
    ];
    primary_args.extend(action_args.iter().map(|arg| arg.to_string()));

    match run_single_compose_command(state, "docker", &primary_args).await {
        Ok(()) => return Ok(()),
        Err(ComposeCommandError::Timeout) => {
            return Err(AppError::BadRequest(format!(
                "{} timed out after {} seconds",
                action_name, state.config.compose_command_timeout_seconds
            )));
        }
        Err(ComposeCommandError::Spawn(message)) => {
            return Err(AppError::BadRequest(format!(
                "{} failed to start compose process: {}",
                action_name, message
            )));
        }
        Err(ComposeCommandError::Failed(message)) if !should_fallback_to_legacy(&message) => {
            return Err(AppError::BadRequest(format!(
                "{} failed: {}",
                action_name, message
            )));
        }
        Err(ComposeCommandError::Failed(_)) | Err(ComposeCommandError::SpawnNotFound) => {}
    }

    let mut legacy_args = vec![
        "-f".to_string(),
        compose_file,
        "-p".to_string(),
        project_name.to_string(),
    ];
    legacy_args.extend(action_args.iter().map(|arg| arg.to_string()));

    match run_single_compose_command(state, "docker-compose", &legacy_args).await {
        Ok(()) => Ok(()),
        Err(ComposeCommandError::Timeout) => Err(AppError::BadRequest(format!(
            "{} timed out after {} seconds",
            action_name, state.config.compose_command_timeout_seconds
        ))),
        Err(ComposeCommandError::SpawnNotFound) => Err(AppError::BadRequest(
            "docker compose command is unavailable (tried 'docker compose' and 'docker-compose')"
                .to_string(),
        )),
        Err(ComposeCommandError::Spawn(message)) => Err(AppError::BadRequest(format!(
            "{} failed to start compose process: {}",
            action_name, message
        ))),
        Err(ComposeCommandError::Failed(message)) => Err(AppError::BadRequest(format!(
            "{} failed: {}",
            action_name, message
        ))),
    }
}

async fn run_single_compose_command(
    state: &AppState,
    program: &str,
    args: &[String],
) -> Result<(), ComposeCommandError> {
    let timeout_secs = state.config.compose_command_timeout_seconds.clamp(5, 600);

    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = match timeout(TokioDuration::from_secs(timeout_secs), command.output()).await {
        Ok(Ok(output)) => output,
        Ok(Err(err)) if err.kind() == ErrorKind::NotFound => {
            return Err(ComposeCommandError::SpawnNotFound);
        }
        Ok(Err(err)) => return Err(ComposeCommandError::Spawn(err.to_string())),
        Err(_) => return Err(ComposeCommandError::Timeout),
    };

    if output.status.success() {
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(ComposeCommandError::Failed(compact_message(
        &stderr,
        &stdout,
        "compose command failed",
    )))
}

fn should_fallback_to_legacy(message: &str) -> bool {
    let lowered = message.to_ascii_lowercase();
    lowered.contains("is not a docker command")
        || lowered.contains("unknown command \"compose\"")
        || lowered.contains("docker: 'compose' is not")
        || (lowered.contains("unknown shorthand flag")
            && (lowered.contains("in -f")
                || lowered.contains("in -p")
                || lowered.contains("see 'docker --help'")))
}

fn compact_message(primary: &str, secondary: &str, fallback: &str) -> String {
    let source = if !primary.trim().is_empty() {
        primary.trim()
    } else if !secondary.trim().is_empty() {
        secondary.trim()
    } else {
        fallback
    };

    let mut message = source.replace('\n', " ").replace('\r', " ");
    if message.chars().count() > 240 {
        message = message.chars().take(240).collect::<String>() + "...";
    }
    message
}

async fn cleanup_runtime_dir(state: &AppState, compose_project_name: &str) {
    let runtime_dir = runtime_project_dir(state, compose_project_name);

    if let Err(err) = fs::remove_dir_all(&runtime_dir).await {
        if err.kind() != ErrorKind::NotFound {
            warn!(
                compose_project_name,
                runtime_dir = %runtime_dir.display(),
                error = %err,
                "failed to cleanup runtime directory"
            );
        }
    }
}

fn is_expired(instance: &InstanceRow, now: DateTime<Utc>) -> bool {
    instance
        .expires_at
        .is_some_and(|expires_at| now > expires_at)
}

fn instance_to_response(row: InstanceRow, message: String) -> InstanceResponse {
    InstanceResponse {
        id: row.id,
        contest_id: row.contest_id,
        challenge_id: row.challenge_id,
        team_id: row.team_id,
        status: row.status,
        subnet: row.subnet,
        compose_project_name: row.compose_project_name,
        entrypoint_url: row.entrypoint_url,
        cpu_limit: row.cpu_limit,
        memory_limit_mb: row.memory_limit_mb,
        started_at: row.started_at,
        expires_at: row.expires_at,
        destroyed_at: row.destroyed_at,
        last_heartbeat_at: row.last_heartbeat_at,
        message,
    }
}
