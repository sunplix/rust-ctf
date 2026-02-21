use std::{
    io::ErrorKind,
    net::{TcpListener, UdpSocket},
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
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
    routes::contest_access::ensure_team_contest_workspace_access,
    runtime_template::{
        build_single_image_compose_template, parse_runtime_metadata_options,
        render_compose_template_variables, validate_compose_template_schema, RuntimeAccessMode,
        RuntimeEndpointProtocol, RuntimeMode,
    },
    state::AppState,
};

const INSTANCE_TTL_HOURS: i64 = 2;
const SUBNET_SECOND_OCTET_START: u16 = 16;
const SUBNET_SECOND_OCTET_END: u16 = 223;
const COMPOSE_FILE_NAME: &str = "docker-compose.generated.yml";
const INSTANCE_HEARTBEAT_TOKEN_USE: &str = "instance_heartbeat";
const INSTANCE_HEARTBEAT_TOKEN_GRACE_SECONDS: i64 = 10 * 60;
const INSTANCE_HEARTBEAT_TOKEN_MAX_TTL_SECONDS: i64 = 24 * 60 * 60;
const INSTANCE_SSH_GATEWAY_PORT: u16 = 2222;
const INSTANCE_SSH_GATEWAY_IMAGE: &str = "linuxserver/openssh-server:latest";
const INSTANCE_SSH_GATEWAY_SERVICE_NAME: &str = "ctf_access_gateway";
const INSTANCE_SSH_GATEWAY_USERNAME: &str = "ctf";
const INSTANCE_SSH_GATEWAY_ENABLE_SUDO: &str = "true";
const INSTANCE_WIREGUARD_IMAGE: &str = "linuxserver/wireguard:latest";
const INSTANCE_WIREGUARD_SERVICE_NAME: &str = "ctf_access_wireguard";
const INSTANCE_WIREGUARD_SERVICE_PORT: u16 = 51820;
const INSTANCE_WIREGUARD_PEERS: &str = "1";
const INSTANCE_WIREGUARD_PEER_DNS: &str = "1.1.1.1";
const INSTANCE_WIREGUARD_CONFIG_SERVICE_NAME: &str = "ctf_access_wireguard_config_api";
const INSTANCE_WIREGUARD_CONFIG_SERVICE_IMAGE: &str = INSTANCE_WIREGUARD_IMAGE;
const INSTANCE_WIREGUARD_CONFIG_SERVICE_PORT: u16 = 8000;
const INSTANCE_WIREGUARD_CONFIG_VOLUME_NAME: &str = "ctf_access_wireguard_config";
const INSTANCE_WIREGUARD_ACCESS_META_FILE: &str = "wireguard-access.json";
const INSTANCE_WIREGUARD_CONFIG_FETCH_RETRIES: usize = 6;
const INSTANCE_WIREGUARD_CONFIG_FETCH_DELAY_MS: u64 = 1000;
const INSTANCE_PORT_ALLOCATE_RETRIES: usize = 64;

#[derive(Debug, Deserialize)]
struct InstanceActionRequest {
    contest_id: Uuid,
    challenge_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct InternalHeartbeatReportRequest {
    token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct InstanceHeartbeatTokenClaims {
    sub: String,
    token_use: String,
    iat: usize,
    exp: usize,
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
    network_access: Option<InstanceNetworkAccess>,
    message: String,
}

#[derive(Debug, Serialize)]
struct InstanceNetworkAccess {
    mode: String,
    host: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
    download_url: Option<String>,
    note: String,
}

#[derive(Debug, Serialize)]
struct WireguardConfigResponse {
    contest_id: Uuid,
    challenge_id: Uuid,
    team_id: Uuid,
    endpoint: String,
    filename: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct WireguardAccessMeta {
    config_host_port: u16,
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
    metadata: Value,
    is_visible: bool,
    release_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow)]
struct ComposeTemplateRow {
    compose_template: Option<String>,
    flag_mode: String,
    metadata: Value,
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
    metadata: Value,
    entrypoint_mode: RuntimeEntrypointMode,
    network_access_mode: RuntimeAccessMode,
}

#[derive(Debug)]
struct WireguardComposeInjected {
    compose: String,
    config_host_port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeEntrypointMode {
    InternalSubnet,
    HostMapped(RuntimeEndpointProtocol),
    SshBastion,
    Wireguard,
}

#[derive(Debug, Clone, Copy)]
enum HostPortProtocol {
    Tcp,
    Udp,
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
            "/instances/heartbeat/report",
            post(report_instance_heartbeat),
        )
        .route(
            "/instances/{contest_id}/{challenge_id}",
            get(get_instance_by_challenge),
        )
        .route(
            "/instances/{contest_id}/{challenge_id}/wireguard-config",
            get(get_instance_wireguard_config),
        )
}

async fn start_instance(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Json(req): Json<InstanceActionRequest>,
) -> AppResult<Json<InstanceResponse>> {
    let team_id = fetch_user_team_id(state.as_ref(), current_user.user_id).await?;
    ensure_team_contest_workspace_access(
        state.as_ref(),
        req.contest_id,
        team_id,
        &current_user,
    )
    .await?;
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
                state.as_ref(),
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
        compose_source.entrypoint_mode,
    )
    .await?;

    let compose_file = persist_compose_file(state.as_ref(), &pending, &compose_source).await?;
    if let Err(err) =
        compose_up_with_self_heal(state.as_ref(), &pending, &compose_file, false).await
    {
        let _ = update_instance_status(state.as_ref(), pending.id, "failed").await;
        return Err(err);
    }

    let running = mark_instance_running(state.as_ref(), pending.id, now, expires_at).await?;
    Ok(Json(instance_to_response(
        state.as_ref(),
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
    ensure_team_contest_workspace_access(
        state.as_ref(),
        req.contest_id,
        team_id,
        &current_user,
    )
    .await?;

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
        state.as_ref(),
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
    ensure_team_contest_workspace_access(
        state.as_ref(),
        req.contest_id,
        team_id,
        &current_user,
    )
    .await?;
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
        compose_source.entrypoint_mode,
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

    if let Err(err) = compose_up_with_self_heal(state.as_ref(), &pending, &compose_file, true).await
    {
        let _ = update_instance_status(state.as_ref(), pending.id, "failed").await;
        return Err(err);
    }

    let running = mark_instance_running(state.as_ref(), pending.id, now, expires_at).await?;
    Ok(Json(instance_to_response(
        state.as_ref(),
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
    ensure_team_contest_workspace_access(
        state.as_ref(),
        req.contest_id,
        team_id,
        &current_user,
    )
    .await?;

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
        state.as_ref(),
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
    ensure_team_contest_workspace_access(
        state.as_ref(),
        req.contest_id,
        team_id,
        &current_user,
    )
    .await?;

    let updated =
        touch_instance_heartbeat(state.as_ref(), req.contest_id, req.challenge_id, team_id)
            .await?
            .ok_or(AppError::BadRequest(
                "running instance not found".to_string(),
            ))?;

    Ok(Json(instance_to_response(
        state.as_ref(),
        updated,
        "instance heartbeat updated".to_string(),
    )))
}

async fn report_instance_heartbeat(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InternalHeartbeatReportRequest>,
) -> AppResult<Json<InstanceResponse>> {
    let token = req.token.trim();
    if token.is_empty() {
        return Err(AppError::BadRequest("token is required".to_string()));
    }

    let instance_id = decode_instance_heartbeat_token(token, &state.config.jwt_secret)?;
    let updated = touch_instance_heartbeat_by_id(state.as_ref(), instance_id)
        .await?
        .ok_or(AppError::BadRequest(
            "running instance not found".to_string(),
        ))?;

    Ok(Json(instance_to_response(
        state.as_ref(),
        updated,
        "instance heartbeat reported".to_string(),
    )))
}

async fn get_instance_by_challenge(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    AxumPath((contest_id, challenge_id)): AxumPath<(Uuid, Uuid)>,
) -> AppResult<Json<InstanceResponse>> {
    let team_id = fetch_user_team_id(state.as_ref(), current_user.user_id).await?;
    ensure_team_contest_workspace_access(
        state.as_ref(),
        contest_id,
        team_id,
        &current_user,
    )
    .await?;

    let instance = fetch_instance_row(state.as_ref(), contest_id, challenge_id, team_id)
        .await?
        .ok_or(AppError::BadRequest("instance not found".to_string()))?;

    Ok(Json(instance_to_response(
        state.as_ref(),
        instance,
        "instance query result".to_string(),
    )))
}

async fn get_instance_wireguard_config(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    AxumPath((contest_id, challenge_id)): AxumPath<(Uuid, Uuid)>,
) -> AppResult<Json<WireguardConfigResponse>> {
    let team_id = fetch_user_team_id(state.as_ref(), current_user.user_id).await?;
    ensure_team_contest_workspace_access(
        state.as_ref(),
        contest_id,
        team_id,
        &current_user,
    )
    .await?;

    let instance = fetch_instance_row(state.as_ref(), contest_id, challenge_id, team_id)
        .await?
        .ok_or(AppError::BadRequest("instance not found".to_string()))?;

    if instance.status == "destroyed" {
        return Err(AppError::BadRequest(
            "instance has already been destroyed".to_string(),
        ));
    }

    if !instance.entrypoint_url.starts_with("wg://") {
        return Err(AppError::BadRequest(
            "instance access mode is not wireguard".to_string(),
        ));
    }

    let content = read_instance_wireguard_config(state.as_ref(), &instance).await?;

    let filename = format!(
        "{}-{}-{}.conf",
        contest_id.as_simple(),
        challenge_id.as_simple(),
        team_id.as_simple()
    );

    Ok(Json(WireguardConfigResponse {
        contest_id,
        challenge_id,
        team_id,
        endpoint: instance.entrypoint_url,
        filename,
        content,
    }))
}

async fn fetch_user_team_id(state: &AppState, user_id: Uuid) -> AppResult<Uuid> {
    let team = sqlx::query_as::<_, TeamMembershipRow>(
        "SELECT team_id FROM team_members WHERE user_id = $1 LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest(
        "join or create a team before entering the contest".to_string(),
    ))?;

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
                c.metadata,
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
                c.flag_mode,
                c.metadata
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

    if policy.challenge_type != "dynamic" && policy.challenge_type != "internal" {
        return Err(AppError::BadRequest(
            "challenge type does not require runtime instance".to_string(),
        ));
    }

    let runtime_options =
        parse_runtime_metadata_options(&policy.metadata).map_err(AppError::BadRequest)?;
    if runtime_options.mode == RuntimeMode::Compose
        && policy
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

    Ok(())
}

fn compose_source_from_policy(policy: RuntimePolicyRow) -> AppResult<ComposeRenderSource> {
    let runtime_options =
        parse_runtime_metadata_options(&policy.metadata).map_err(AppError::BadRequest)?;

    match runtime_options.mode {
        RuntimeMode::SingleImage => {
            let single = runtime_options.single_image.ok_or(AppError::BadRequest(
                "metadata.runtime single_image config is missing".to_string(),
            ))?;
            let template =
                build_single_image_compose_template(single.image.as_str(), single.internal_port);
            validate_compose_template_schema(&template, &policy.metadata)
                .map_err(AppError::BadRequest)?;

            Ok(ComposeRenderSource {
                template,
                flag_mode: policy.flag_mode,
                metadata: policy.metadata,
                entrypoint_mode: RuntimeEntrypointMode::HostMapped(single.protocol),
                network_access_mode: RuntimeAccessMode::Direct,
            })
        }
        RuntimeMode::Compose => {
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

            validate_compose_template_schema(&template, &policy.metadata)
                .map_err(AppError::BadRequest)?;

            let entrypoint_mode = if runtime_options.access_mode == RuntimeAccessMode::SshBastion {
                RuntimeEntrypointMode::SshBastion
            } else if runtime_options.access_mode == RuntimeAccessMode::Wireguard {
                RuntimeEntrypointMode::Wireguard
            } else {
                RuntimeEntrypointMode::InternalSubnet
            };

            Ok(ComposeRenderSource {
                template,
                flag_mode: policy.flag_mode,
                metadata: policy.metadata,
                entrypoint_mode,
                network_access_mode: runtime_options.access_mode,
            })
        }
    }
}

fn compose_source_from_row(row: ComposeTemplateRow) -> AppResult<ComposeRenderSource> {
    let runtime_options =
        parse_runtime_metadata_options(&row.metadata).map_err(AppError::BadRequest)?;

    match runtime_options.mode {
        RuntimeMode::SingleImage => {
            let single = runtime_options.single_image.ok_or(AppError::BadRequest(
                "metadata.runtime single_image config is missing".to_string(),
            ))?;
            let template =
                build_single_image_compose_template(single.image.as_str(), single.internal_port);
            validate_compose_template_schema(&template, &row.metadata)
                .map_err(AppError::BadRequest)?;

            Ok(ComposeRenderSource {
                template,
                flag_mode: row.flag_mode,
                metadata: row.metadata,
                entrypoint_mode: RuntimeEntrypointMode::HostMapped(single.protocol),
                network_access_mode: RuntimeAccessMode::Direct,
            })
        }
        RuntimeMode::Compose => {
            let template = row.compose_template.unwrap_or_default().trim().to_string();
            if template.is_empty() {
                return Err(AppError::BadRequest(
                    "challenge runtime template is missing".to_string(),
                ));
            }

            validate_compose_template_schema(&template, &row.metadata)
                .map_err(AppError::BadRequest)?;

            let entrypoint_mode = if runtime_options.access_mode == RuntimeAccessMode::SshBastion {
                RuntimeEntrypointMode::SshBastion
            } else if runtime_options.access_mode == RuntimeAccessMode::Wireguard {
                RuntimeEntrypointMode::Wireguard
            } else {
                RuntimeEntrypointMode::InternalSubnet
            };

            Ok(ComposeRenderSource {
                template,
                flag_mode: row.flag_mode,
                metadata: row.metadata,
                entrypoint_mode,
                network_access_mode: runtime_options.access_mode,
            })
        }
    }
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
    entrypoint_mode: RuntimeEntrypointMode,
) -> AppResult<InstanceRow> {
    let cpu_limit = default_instance_cpu_limit_text(state);
    let memory_limit_mb = default_instance_memory_limit_mb(state);

    match fetch_instance_row(state, contest_id, challenge_id, team_id).await? {
        Some(existing) => {
            let entrypoint_url = resolve_entrypoint_url(
                state,
                entrypoint_mode,
                &existing.subnet,
                Some(&existing.entrypoint_url),
            )?;
            mark_instance_creating(
                state,
                existing.id,
                now,
                expires_at,
                cpu_limit.as_deref(),
                memory_limit_mb,
                &entrypoint_url,
            )
            .await
        }
        None => {
            let subnet = allocate_subnet(state, contest_id, challenge_id, team_id).await?;
            let compose_project_name = compose_project_name(contest_id, challenge_id, team_id);
            let entrypoint_url = resolve_entrypoint_url(state, entrypoint_mode, &subnet, None)?;

            insert_instance_row(
                state,
                InsertInstanceRowParams {
                    contest_id,
                    challenge_id,
                    team_id,
                    subnet: &subnet,
                    compose_project_name: &compose_project_name,
                    entrypoint_url: &entrypoint_url,
                    now,
                    expires_at,
                    cpu_limit: cpu_limit.as_deref(),
                    memory_limit_mb,
                },
            )
            .await
        }
    }
}

struct InsertInstanceRowParams<'a> {
    contest_id: Uuid,
    challenge_id: Uuid,
    team_id: Uuid,
    subnet: &'a str,
    compose_project_name: &'a str,
    entrypoint_url: &'a str,
    now: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    cpu_limit: Option<&'a str>,
    memory_limit_mb: Option<i32>,
}

async fn insert_instance_row(
    state: &AppState,
    params: InsertInstanceRowParams<'_>,
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
    .bind(params.contest_id)
    .bind(params.challenge_id)
    .bind(params.team_id)
    .bind(params.subnet)
    .bind(params.compose_project_name)
    .bind(params.entrypoint_url)
    .bind(params.cpu_limit)
    .bind(params.memory_limit_mb)
    .bind(params.now)
    .bind(params.expires_at)
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
    entrypoint_url: &str,
) -> AppResult<InstanceRow> {
    sqlx::query_as::<_, InstanceRow>(
        "UPDATE instances
         SET status = 'creating',
             started_at = $2,
             expires_at = $3,
             cpu_limit = $4::numeric,
             memory_limit_mb = $5,
             entrypoint_url = $6,
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
    .bind(entrypoint_url)
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

async fn mark_instance_reaper_failed(state: &AppState, instance_id: Uuid) -> AppResult<()> {
    sqlx::query(
        "UPDATE instances
         SET status = 'failed',
             expires_at = NULL,
             updated_at = NOW()
         WHERE id = $1
           AND status <> 'destroyed'",
    )
    .bind(instance_id)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(())
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

async fn touch_instance_heartbeat_by_id(
    state: &AppState,
    instance_id: Uuid,
) -> AppResult<Option<InstanceRow>> {
    sqlx::query_as::<_, InstanceRow>(
        "UPDATE instances
         SET last_heartbeat_at = NOW(),
             updated_at = NOW()
         WHERE id = $1
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
    .bind(instance_id)
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

async fn fetch_instance_candidates_for_contest(
    state: &AppState,
    contest_id: Uuid,
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
         WHERE contest_id = $1
         ORDER BY created_at DESC",
    )
    .bind(contest_id)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)
}

async fn fetch_instance_candidates_for_challenge(
    state: &AppState,
    challenge_id: Uuid,
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
         WHERE challenge_id = $1
         ORDER BY created_at DESC",
    )
    .bind(challenge_id)
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

async fn mark_instance_destroyed(
    state: &AppState,
    instance_id: Uuid,
) -> AppResult<Option<InstanceRow>> {
    sqlx::query_as::<_, InstanceRow>(
        "UPDATE instances
         SET status = 'destroyed',
             destroyed_at = COALESCE(destroyed_at, NOW()),
             expires_at = NULL,
             updated_at = NOW()
         WHERE id = $1
           AND status <> 'destroyed'
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
                if let Err(mark_err) = mark_instance_reaper_failed(state, instance.id).await {
                    warn!(
                        instance_id = %instance.id,
                        error = %mark_err,
                        "instance reaper failed to mark instance as failed"
                    );
                }
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
            if let Err(mark_err) = mark_instance_reaper_failed(state, instance.id).await {
                warn!(
                    instance_id = %instance.id,
                    error = %mark_err,
                    "instance reaper failed to mark instance as failed"
                );
            }
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

pub(crate) async fn destroy_instances_for_contest(
    state: &AppState,
    contest_id: Uuid,
) -> AppResult<InstanceReaperSummary> {
    let candidates = fetch_instance_candidates_for_contest(state, contest_id).await?;
    destroy_instance_candidates(state, candidates).await
}

pub(crate) async fn destroy_instances_for_challenge(
    state: &AppState,
    challenge_id: Uuid,
) -> AppResult<InstanceReaperSummary> {
    let candidates = fetch_instance_candidates_for_challenge(state, challenge_id).await?;
    destroy_instance_candidates(state, candidates).await
}

async fn destroy_instance_candidates(
    state: &AppState,
    candidates: Vec<InstanceRow>,
) -> AppResult<InstanceReaperSummary> {
    let mut scanned = 0_i64;
    let mut reaped = 0_i64;
    let mut failed = 0_i64;
    let mut skipped = 0_i64;

    for instance in candidates {
        scanned += 1;

        if instance.status == "destroyed" {
            cleanup_runtime_dir(state, &instance.compose_project_name).await;
            skipped += 1;
            continue;
        }

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
                    "failed to prepare compose file for force destroy"
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
                "failed to compose down during force destroy"
            );
            let _ = update_instance_status(state, instance.id, "failed").await;
            continue;
        }

        match mark_instance_destroyed(state, instance.id).await? {
            Some(updated) => {
                cleanup_runtime_dir(state, &updated.compose_project_name).await;
                reaped += 1;
            }
            None => {
                cleanup_runtime_dir(state, &instance.compose_project_name).await;
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

fn resolve_entrypoint_url(
    state: &AppState,
    mode: RuntimeEntrypointMode,
    subnet: &str,
    existing_entrypoint_url: Option<&str>,
) -> AppResult<String> {
    match mode {
        RuntimeEntrypointMode::InternalSubnet => Ok(default_entrypoint_url(subnet)),
        RuntimeEntrypointMode::HostMapped(protocol) => {
            let host = instance_public_host(state);
            let port = existing_entrypoint_url
                .and_then(parse_entrypoint_host_port)
                .map(|(_, port)| port)
                .unwrap_or(allocate_random_host_port(state, HostPortProtocol::Tcp)?);

            let scheme = match protocol {
                RuntimeEndpointProtocol::Http => "http",
                RuntimeEndpointProtocol::Https => "https",
                RuntimeEndpointProtocol::Tcp => "tcp",
            };
            Ok(format!("{scheme}://{host}:{port}"))
        }
        RuntimeEntrypointMode::SshBastion => {
            let host = instance_public_host(state);
            let port = existing_entrypoint_url
                .and_then(parse_entrypoint_host_port)
                .map(|(_, port)| port)
                .unwrap_or(allocate_random_host_port(state, HostPortProtocol::Tcp)?);
            Ok(format!("ssh://{host}:{port}"))
        }
        RuntimeEntrypointMode::Wireguard => {
            let host = instance_public_host(state);
            let port = existing_entrypoint_url
                .and_then(parse_entrypoint_host_port)
                .map(|(_, port)| port)
                .unwrap_or(allocate_random_host_port(state, HostPortProtocol::Udp)?);
            Ok(format!("wg://{host}:{port}"))
        }
    }
}

fn instance_public_host(state: &AppState) -> String {
    let host = state.config.instance_public_host.trim();
    if host.is_empty() {
        "127.0.0.1".to_string()
    } else {
        host.to_string()
    }
}

fn allocate_random_host_port(state: &AppState, protocol: HostPortProtocol) -> AppResult<u16> {
    let min = state.config.instance_host_port_min.max(1024);
    let max = state.config.instance_host_port_max.max(min);
    for _ in 0..INSTANCE_PORT_ALLOCATE_RETRIES {
        let port = match protocol {
            HostPortProtocol::Tcp => {
                let listener = TcpListener::bind(("0.0.0.0", 0)).map_err(AppError::internal)?;
                listener.local_addr().map_err(AppError::internal)?.port()
            }
            HostPortProtocol::Udp => {
                let socket = UdpSocket::bind(("0.0.0.0", 0)).map_err(AppError::internal)?;
                socket.local_addr().map_err(AppError::internal)?.port()
            }
        };
        if port >= min && port <= max {
            return Ok(port);
        }
    }

    Err(AppError::internal(anyhow::anyhow!(
        "failed to allocate random host port in configured range"
    )))
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

fn issue_instance_heartbeat_token(state: &AppState, instance: &InstanceRow) -> AppResult<String> {
    let now = Utc::now();
    let mut expires_at = instance
        .expires_at
        .map(|value| value + Duration::seconds(INSTANCE_HEARTBEAT_TOKEN_GRACE_SECONDS))
        .unwrap_or_else(|| now + Duration::hours(INSTANCE_TTL_HOURS));
    let max_expires_at = now + Duration::seconds(INSTANCE_HEARTBEAT_TOKEN_MAX_TTL_SECONDS);

    if expires_at > max_expires_at {
        expires_at = max_expires_at;
    }
    if expires_at <= now {
        expires_at = now + Duration::minutes(10);
    }

    let claims = InstanceHeartbeatTokenClaims {
        sub: instance.id.to_string(),
        token_use: INSTANCE_HEARTBEAT_TOKEN_USE.to_string(),
        iat: now.timestamp() as usize,
        exp: expires_at.timestamp() as usize,
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(state.config.jwt_secret.as_bytes()),
    )
    .map_err(AppError::internal)
}

fn decode_instance_heartbeat_token(token: &str, jwt_secret: &str) -> AppResult<Uuid> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    let claims = decode::<InstanceHeartbeatTokenClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|_| AppError::Unauthorized)?
    .claims;

    if claims.token_use != INSTANCE_HEARTBEAT_TOKEN_USE {
        return Err(AppError::Unauthorized);
    }

    Uuid::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)
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

fn wireguard_access_meta_path(state: &AppState, compose_project_name: &str) -> PathBuf {
    runtime_project_dir(state, compose_project_name).join(INSTANCE_WIREGUARD_ACCESS_META_FILE)
}

async fn write_wireguard_access_meta(
    state: &AppState,
    compose_project_name: &str,
    config_host_port: u16,
) -> AppResult<()> {
    let path = wireguard_access_meta_path(state, compose_project_name);
    let meta = WireguardAccessMeta { config_host_port };
    let data = serde_json::to_vec(&meta).map_err(AppError::internal)?;
    fs::write(path, data).await.map_err(AppError::internal)
}

async fn read_wireguard_access_meta(
    state: &AppState,
    compose_project_name: &str,
) -> AppResult<WireguardAccessMeta> {
    let path = wireguard_access_meta_path(state, compose_project_name);
    let data = fs::read(path).await.map_err(|err| {
        AppError::BadRequest(format!("wireguard access metadata is missing: {err}"))
    })?;
    serde_json::from_slice::<WireguardAccessMeta>(&data).map_err(AppError::internal)
}

async fn cleanup_wireguard_access_meta(state: &AppState, compose_project_name: &str) {
    let path = wireguard_access_meta_path(state, compose_project_name);
    if let Err(err) = fs::remove_file(path).await {
        if err.kind() != ErrorKind::NotFound {
            warn!(error = %err, compose_project_name, "failed to cleanup wireguard meta file");
        }
    }
}

fn render_compose_template(
    state: &AppState,
    source: &str,
    instance: &InstanceRow,
    dynamic_flag: Option<&str>,
    heartbeat_report_url: &str,
    heartbeat_report_token: &str,
    heartbeat_interval_seconds: u64,
) -> String {
    let network_name = format!("{}_net", instance.compose_project_name);
    let team_id = instance.team_id.to_string();
    let contest_id = instance.contest_id.to_string();
    let challenge_id = instance.challenge_id.to_string();
    let entrypoint_host = subnet_host_ip(&instance.subnet, 2).unwrap_or_default();
    let gateway_ip = subnet_host_ip(&instance.subnet, 1).unwrap_or_default();
    let public_host = parse_entrypoint_host_port(&instance.entrypoint_url)
        .map(|(host, _)| host)
        .unwrap_or_else(|| instance_public_host(state));
    let host_port = parse_entrypoint_host_port(&instance.entrypoint_url)
        .map(|(_, port)| port.to_string())
        .unwrap_or_default();
    let ssh_username = instance_ssh_gateway_username();
    let ssh_password = instance_ssh_gateway_password(state, instance);
    let cpu_limit = instance.cpu_limit.clone().unwrap_or_default();
    let memory_limit_mb = instance
        .memory_limit_mb
        .map(|value| value.to_string())
        .unwrap_or_default();
    let memory_limit = instance
        .memory_limit_mb
        .map(|value| format!("{}m", value))
        .unwrap_or_default();
    let heartbeat_interval_seconds = heartbeat_interval_seconds.to_string();

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
        ("{{PUBLIC_HOST}}", public_host.as_str()),
        ("{{HOST_PORT}}", host_port.as_str()),
        ("{{ACCESS_HOST_PORT}}", host_port.as_str()),
        ("{{ACCESS_USERNAME}}", ssh_username.as_str()),
        ("{{ACCESS_PASSWORD}}", ssh_password.as_str()),
        ("{{CPU_LIMIT}}", cpu_limit.as_str()),
        ("{{MEMORY_LIMIT_MB}}", memory_limit_mb.as_str()),
        ("{{MEMORY_LIMIT}}", memory_limit.as_str()),
        ("{{HEARTBEAT_REPORT_URL}}", heartbeat_report_url),
        ("{{HEARTBEAT_REPORT_TOKEN}}", heartbeat_report_token),
        (
            "{{HEARTBEAT_INTERVAL_SECONDS}}",
            heartbeat_interval_seconds.as_str(),
        ),
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

    let heartbeat_report_url = state.config.instance_heartbeat_report_url.trim();
    let heartbeat_report_interval_seconds = state
        .config
        .instance_heartbeat_report_interval_seconds
        .clamp(5, 3600);
    let heartbeat_report_token = issue_instance_heartbeat_token(state, instance)?;

    let rendered = render_compose_template(
        state,
        &source.template,
        instance,
        dynamic_flag.as_deref(),
        heartbeat_report_url,
        &heartbeat_report_token,
        heartbeat_report_interval_seconds,
    );
    let rendered = render_compose_template_variables(&rendered, &source.metadata)
        .map_err(AppError::BadRequest)?;
    let mut wireguard_config_host_port: Option<u16> = None;
    let rendered = match source.network_access_mode {
        RuntimeAccessMode::SshBastion => {
            let (_, host_port) = parse_entrypoint_host_port(&instance.entrypoint_url).ok_or(
                AppError::BadRequest("instance entrypoint host port is missing".to_string()),
            )?;
            let username = instance_ssh_gateway_username();
            let password = instance_ssh_gateway_password(state, instance);
            inject_ssh_gateway_service(&rendered, host_port, &username, &password)?
        }
        RuntimeAccessMode::Wireguard => {
            let (public_host, host_port) = parse_entrypoint_host_port(&instance.entrypoint_url)
                .ok_or(AppError::BadRequest(
                    "instance entrypoint host port is missing".to_string(),
                ))?;
            let injected =
                inject_wireguard_service(state, instance, &rendered, &public_host, host_port)?;
            wireguard_config_host_port = Some(injected.config_host_port);
            injected.compose
        }
        RuntimeAccessMode::Direct => rendered,
    };
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

    if let Some(config_host_port) = wireguard_config_host_port {
        write_wireguard_access_meta(state, &instance.compose_project_name, config_host_port)
            .await?;
    } else {
        cleanup_wireguard_access_meta(state, &instance.compose_project_name).await;
    }

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

async fn read_instance_wireguard_config(
    state: &AppState,
    instance: &InstanceRow,
) -> AppResult<String> {
    let meta = read_wireguard_access_meta(state, &instance.compose_project_name).await?;
    let url = format!(
        "http://host.docker.internal:{}/peer1/peer1.conf",
        meta.config_host_port
    );
    let mut last_error: Option<String> = None;

    for attempt in 0..INSTANCE_WIREGUARD_CONFIG_FETCH_RETRIES {
        match fetch_wireguard_config_via_http(state, &url).await {
            Ok(content) => {
                let normalized = content.replace("\r\n", "\n");
                if normalized.contains("[Interface]") && normalized.contains("[Peer]") {
                    return Ok(normalized);
                }
                last_error = Some("wireguard config is not ready yet".to_string());
            }
            Err(AppError::BadRequest(message)) => {
                last_error = Some(message);
            }
            Err(err) => return Err(err),
        }

        if attempt + 1 < INSTANCE_WIREGUARD_CONFIG_FETCH_RETRIES {
            tokio::time::sleep(TokioDuration::from_millis(
                INSTANCE_WIREGUARD_CONFIG_FETCH_DELAY_MS,
            ))
            .await;
        }
    }

    Err(AppError::BadRequest(format!(
        "wireguard config is not ready: {}",
        last_error.unwrap_or_else(|| "unknown error".to_string())
    )))
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

async fn compose_up_with_self_heal(
    state: &AppState,
    instance: &InstanceRow,
    compose_file: &Path,
    force_recreate: bool,
) -> AppResult<()> {
    if let Err(initial_err) = compose_up(
        state,
        &instance.compose_project_name,
        compose_file,
        force_recreate,
    )
    .await
    {
        warn!(
            instance_id = %instance.id,
            contest_id = %instance.contest_id,
            challenge_id = %instance.challenge_id,
            team_id = %instance.team_id,
            compose_project_name = %instance.compose_project_name,
            error = %initial_err,
            "instance compose up failed, starting self-heal retry"
        );

        if let Err(down_err) =
            compose_down(state, &instance.compose_project_name, compose_file).await
        {
            warn!(
                instance_id = %instance.id,
                contest_id = %instance.contest_id,
                challenge_id = %instance.challenge_id,
                team_id = %instance.team_id,
                compose_project_name = %instance.compose_project_name,
                error = %down_err,
                "instance self-heal cleanup failed before retry"
            );
        }

        match compose_up(state, &instance.compose_project_name, compose_file, true).await {
            Ok(()) => Ok(()),
            Err(retry_err) => Err(append_self_heal_failure_context(retry_err)),
        }
    } else {
        Ok(())
    }
}

fn append_self_heal_failure_context(err: AppError) -> AppError {
    match err {
        AppError::BadRequest(message) => {
            AppError::BadRequest(format!("{message}; self-heal retry also failed"))
        }
        other => other,
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
    run_single_compose_command_capture(state, program, args)
        .await
        .map(|_| ())
}

async fn run_single_compose_command_capture(
    state: &AppState,
    program: &str,
    args: &[String],
) -> Result<String, ComposeCommandError> {
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

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        let trimmed = stdout.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
        return Ok(stderr.trim().to_string());
    }

    Err(ComposeCommandError::Failed(compact_message(
        &stderr,
        &stdout,
        "compose command failed",
    )))
}

async fn fetch_wireguard_config_via_http(state: &AppState, url: &str) -> AppResult<String> {
    let timeout_secs = state.config.compose_command_timeout_seconds.clamp(5, 120);
    let curl_max_time = timeout_secs.min(2).to_string();

    let mut command = Command::new("curl");
    command
        .args(["-fsS", "--max-time", curl_max_time.as_str(), url])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = match timeout(TokioDuration::from_secs(timeout_secs), command.output()).await {
        Ok(Ok(output)) => output,
        Ok(Err(err)) if err.kind() == ErrorKind::NotFound => {
            return Err(AppError::BadRequest(
                "curl command is unavailable for wireguard config fetch".to_string(),
            ));
        }
        Ok(Err(err)) => {
            return Err(AppError::BadRequest(format!(
                "wireguard config fetch failed to start: {}",
                err
            )));
        }
        Err(_) => {
            return Err(AppError::BadRequest(format!(
                "wireguard config fetch timed out after {} seconds",
                timeout_secs
            )));
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        return Ok(stdout);
    }

    Err(AppError::BadRequest(compact_message(
        &stderr,
        &stdout,
        "wireguard config fetch failed",
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

    let mut message = source.replace(['\n', '\r'], " ");
    if message.chars().count() > 240 {
        message = message.chars().take(240).collect::<String>() + "...";
    }
    message
}

fn parse_entrypoint_host_port(url: &str) -> Option<(String, u16)> {
    let without_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    let authority = without_scheme.split('/').next()?.trim();
    let authority = authority.rsplit('@').next()?;
    let (host, port_raw) = authority.rsplit_once(':')?;
    if host.trim().is_empty() {
        return None;
    }
    let port = port_raw.trim().parse::<u16>().ok()?;
    Some((host.trim().to_string(), port))
}

fn instance_ssh_gateway_username() -> String {
    INSTANCE_SSH_GATEWAY_USERNAME.to_string()
}

fn instance_ssh_gateway_password(state: &AppState, instance: &InstanceRow) -> String {
    let seed = format!(
        "{}:{}:{}:{}",
        state.config.jwt_secret, instance.id, instance.team_id, instance.challenge_id
    );
    let digest = Uuid::new_v5(&Uuid::NAMESPACE_URL, seed.as_bytes())
        .as_simple()
        .to_string();
    format!("ctf{}", &digest[..12])
}

fn inject_ssh_gateway_service(
    compose_text: &str,
    host_port: u16,
    username: &str,
    password: &str,
) -> AppResult<String> {
    let mut value: serde_yaml::Value = serde_yaml::from_str(compose_text).map_err(|err| {
        AppError::BadRequest(format!("failed to parse rendered compose yaml: {err}"))
    })?;

    let Some(root_map) = value.as_mapping_mut() else {
        return Err(AppError::BadRequest(
            "rendered compose yaml root must be a mapping".to_string(),
        ));
    };

    let services_key = serde_yaml::Value::String("services".to_string());
    if !root_map.contains_key(&services_key) {
        root_map.insert(
            services_key.clone(),
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
        );
    }

    let primary_network_name = root_map
        .get(serde_yaml::Value::String("networks".to_string()))
        .and_then(serde_yaml::Value::as_mapping)
        .and_then(|networks| {
            networks
                .keys()
                .find_map(|key| key.as_str().map(|name| name.to_string()))
        });

    let Some(services_map) = root_map
        .get_mut(&services_key)
        .and_then(serde_yaml::Value::as_mapping_mut)
    else {
        return Err(AppError::BadRequest(
            "compose.services must be a mapping".to_string(),
        ));
    };

    let gateway_service_key =
        serde_yaml::Value::String(INSTANCE_SSH_GATEWAY_SERVICE_NAME.to_string());
    if services_map.contains_key(&gateway_service_key) {
        return Err(AppError::BadRequest(format!(
            "compose template reserves service name '{INSTANCE_SSH_GATEWAY_SERVICE_NAME}', please rename your service"
        )));
    }

    let mut env_map = serde_yaml::Mapping::new();
    env_map.insert(
        serde_yaml::Value::String("PUID".to_string()),
        serde_yaml::Value::String("1000".to_string()),
    );
    env_map.insert(
        serde_yaml::Value::String("PGID".to_string()),
        serde_yaml::Value::String("1000".to_string()),
    );
    env_map.insert(
        serde_yaml::Value::String("TZ".to_string()),
        serde_yaml::Value::String("UTC".to_string()),
    );
    env_map.insert(
        serde_yaml::Value::String("PASSWORD_ACCESS".to_string()),
        serde_yaml::Value::String("true".to_string()),
    );
    env_map.insert(
        serde_yaml::Value::String("SUDO_ACCESS".to_string()),
        serde_yaml::Value::String(INSTANCE_SSH_GATEWAY_ENABLE_SUDO.to_string()),
    );
    env_map.insert(
        serde_yaml::Value::String("USER_NAME".to_string()),
        serde_yaml::Value::String(username.to_string()),
    );
    env_map.insert(
        serde_yaml::Value::String("USER_PASSWORD".to_string()),
        serde_yaml::Value::String(password.to_string()),
    );

    let mut service_map = serde_yaml::Mapping::new();
    service_map.insert(
        serde_yaml::Value::String("image".to_string()),
        serde_yaml::Value::String(INSTANCE_SSH_GATEWAY_IMAGE.to_string()),
    );
    service_map.insert(
        serde_yaml::Value::String("environment".to_string()),
        serde_yaml::Value::Mapping(env_map),
    );
    service_map.insert(
        serde_yaml::Value::String("ports".to_string()),
        serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(format!(
            "{host_port}:{INSTANCE_SSH_GATEWAY_PORT}"
        ))]),
    );
    if let Some(network_name) = primary_network_name {
        service_map.insert(
            serde_yaml::Value::String("networks".to_string()),
            serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(network_name)]),
        );
    }

    services_map.insert(gateway_service_key, serde_yaml::Value::Mapping(service_map));

    serde_yaml::to_string(&value).map_err(|err| {
        AppError::BadRequest(format!(
            "failed to serialize compose yaml with ssh gateway: {err}"
        ))
    })
}

fn inject_wireguard_service(
    state: &AppState,
    instance: &InstanceRow,
    compose_text: &str,
    public_host: &str,
    host_port: u16,
) -> AppResult<WireguardComposeInjected> {
    let config_host_port = allocate_random_host_port(state, HostPortProtocol::Tcp)?;
    let mut value: serde_yaml::Value = serde_yaml::from_str(compose_text).map_err(|err| {
        AppError::BadRequest(format!("failed to parse rendered compose yaml: {err}"))
    })?;

    let Some(root_map) = value.as_mapping_mut() else {
        return Err(AppError::BadRequest(
            "rendered compose yaml root must be a mapping".to_string(),
        ));
    };

    let services_key = serde_yaml::Value::String("services".to_string());
    if !root_map.contains_key(&services_key) {
        root_map.insert(
            services_key.clone(),
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
        );
    }

    let primary_network_name = root_map
        .get(serde_yaml::Value::String("networks".to_string()))
        .and_then(serde_yaml::Value::as_mapping)
        .and_then(|networks| {
            networks
                .keys()
                .find_map(|key| key.as_str().map(|name| name.to_string()))
        });

    let Some(services_map) = root_map
        .get_mut(&services_key)
        .and_then(serde_yaml::Value::as_mapping_mut)
    else {
        return Err(AppError::BadRequest(
            "compose.services must be a mapping".to_string(),
        ));
    };

    let wireguard_service_key =
        serde_yaml::Value::String(INSTANCE_WIREGUARD_SERVICE_NAME.to_string());
    if services_map.contains_key(&wireguard_service_key) {
        return Err(AppError::BadRequest(format!(
            "compose template reserves service name '{INSTANCE_WIREGUARD_SERVICE_NAME}', please rename your service"
        )));
    }
    let config_service_key =
        serde_yaml::Value::String(INSTANCE_WIREGUARD_CONFIG_SERVICE_NAME.to_string());
    if services_map.contains_key(&config_service_key) {
        return Err(AppError::BadRequest(format!(
            "compose template reserves service name '{INSTANCE_WIREGUARD_CONFIG_SERVICE_NAME}', please rename your service"
        )));
    }

    let mut env_map = serde_yaml::Mapping::new();
    env_map.insert(
        serde_yaml::Value::String("PUID".to_string()),
        serde_yaml::Value::String("1000".to_string()),
    );
    env_map.insert(
        serde_yaml::Value::String("PGID".to_string()),
        serde_yaml::Value::String("1000".to_string()),
    );
    env_map.insert(
        serde_yaml::Value::String("TZ".to_string()),
        serde_yaml::Value::String("UTC".to_string()),
    );
    env_map.insert(
        serde_yaml::Value::String("SERVERURL".to_string()),
        serde_yaml::Value::String(public_host.to_string()),
    );
    env_map.insert(
        serde_yaml::Value::String("SERVERPORT".to_string()),
        serde_yaml::Value::String(host_port.to_string()),
    );
    env_map.insert(
        serde_yaml::Value::String("PEERS".to_string()),
        serde_yaml::Value::String(INSTANCE_WIREGUARD_PEERS.to_string()),
    );
    env_map.insert(
        serde_yaml::Value::String("PEERDNS".to_string()),
        serde_yaml::Value::String(INSTANCE_WIREGUARD_PEER_DNS.to_string()),
    );
    env_map.insert(
        serde_yaml::Value::String("ALLOWEDIPS".to_string()),
        serde_yaml::Value::String(instance.subnet.clone()),
    );
    env_map.insert(
        serde_yaml::Value::String("LOG_CONFS".to_string()),
        serde_yaml::Value::String("true".to_string()),
    );

    let mut service_map = serde_yaml::Mapping::new();
    service_map.insert(
        serde_yaml::Value::String("image".to_string()),
        serde_yaml::Value::String(INSTANCE_WIREGUARD_IMAGE.to_string()),
    );
    service_map.insert(
        serde_yaml::Value::String("environment".to_string()),
        serde_yaml::Value::Mapping(env_map),
    );
    service_map.insert(
        serde_yaml::Value::String("ports".to_string()),
        serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(format!(
            "{host_port}:{INSTANCE_WIREGUARD_SERVICE_PORT}/udp"
        ))]),
    );
    service_map.insert(
        serde_yaml::Value::String("cap_add".to_string()),
        serde_yaml::Value::Sequence(vec![serde_yaml::Value::String("NET_ADMIN".to_string())]),
    );
    service_map.insert(
        serde_yaml::Value::String("sysctls".to_string()),
        serde_yaml::Value::Mapping({
            let mut map = serde_yaml::Mapping::new();
            map.insert(
                serde_yaml::Value::String("net.ipv4.conf.all.src_valid_mark".to_string()),
                serde_yaml::Value::String("1".to_string()),
            );
            map
        }),
    );
    service_map.insert(
        serde_yaml::Value::String("volumes".to_string()),
        serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(format!(
            "{INSTANCE_WIREGUARD_CONFIG_VOLUME_NAME}:/config"
        ))]),
    );

    if let Some(network_name) = primary_network_name {
        service_map.insert(
            serde_yaml::Value::String("networks".to_string()),
            serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(network_name)]),
        );
    }

    services_map.insert(
        wireguard_service_key,
        serde_yaml::Value::Mapping(service_map),
    );

    let config_api_command = format!(
        "while true; do if [ -f /config/peer1/peer1.conf ]; then {{ printf 'HTTP/1.1 200 OK\\r\\nContent-Type: text/plain\\r\\nConnection: close\\r\\n\\r\\n'; cat /config/peer1/peer1.conf; }} | nc -l -p {port} -q 1; else printf 'HTTP/1.1 503 Service Unavailable\\r\\nConnection: close\\r\\n\\r\\nwireguard config not ready\\n' | nc -l -p {port} -q 1; fi; done",
        port = INSTANCE_WIREGUARD_CONFIG_SERVICE_PORT
    );

    let mut config_service_map = serde_yaml::Mapping::new();
    config_service_map.insert(
        serde_yaml::Value::String("image".to_string()),
        serde_yaml::Value::String(INSTANCE_WIREGUARD_CONFIG_SERVICE_IMAGE.to_string()),
    );
    config_service_map.insert(
        serde_yaml::Value::String("entrypoint".to_string()),
        serde_yaml::Value::Sequence(vec![
            serde_yaml::Value::String("sh".to_string()),
            serde_yaml::Value::String("-lc".to_string()),
            serde_yaml::Value::String(config_api_command),
        ]),
    );
    config_service_map.insert(
        serde_yaml::Value::String("depends_on".to_string()),
        serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(
            INSTANCE_WIREGUARD_SERVICE_NAME.to_string(),
        )]),
    );
    config_service_map.insert(
        serde_yaml::Value::String("ports".to_string()),
        serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(format!(
            "{}:{INSTANCE_WIREGUARD_CONFIG_SERVICE_PORT}",
            config_host_port
        ))]),
    );
    config_service_map.insert(
        serde_yaml::Value::String("volumes".to_string()),
        serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(format!(
            "{INSTANCE_WIREGUARD_CONFIG_VOLUME_NAME}:/config:ro"
        ))]),
    );

    services_map.insert(
        config_service_key,
        serde_yaml::Value::Mapping(config_service_map),
    );

    let volumes_key = serde_yaml::Value::String("volumes".to_string());
    if !root_map.contains_key(&volumes_key) {
        root_map.insert(
            volumes_key.clone(),
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
        );
    }
    let Some(volumes_map) = root_map
        .get_mut(&volumes_key)
        .and_then(serde_yaml::Value::as_mapping_mut)
    else {
        return Err(AppError::BadRequest(
            "compose.volumes must be a mapping".to_string(),
        ));
    };
    let volume_key = serde_yaml::Value::String(INSTANCE_WIREGUARD_CONFIG_VOLUME_NAME.to_string());
    if !volumes_map.contains_key(&volume_key) {
        volumes_map.insert(
            volume_key,
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
        );
    }

    let compose = serde_yaml::to_string(&value).map_err(|err| {
        AppError::BadRequest(format!(
            "failed to serialize compose yaml with wireguard: {err}"
        ))
    })?;

    Ok(WireguardComposeInjected {
        compose,
        config_host_port,
    })
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

fn instance_to_response(state: &AppState, row: InstanceRow, message: String) -> InstanceResponse {
    let network_access = if row.entrypoint_url.starts_with("ssh://") {
        parse_entrypoint_host_port(&row.entrypoint_url).map(|(host, port)| InstanceNetworkAccess {
            mode: "ssh_bastion".to_string(),
            host,
            port,
            username: Some(instance_ssh_gateway_username()),
            password: Some(instance_ssh_gateway_password(state, &row)),
            download_url: None,
            note: "Use SSH access box to scan your isolated 10.x.x.0/24 subnet; install extra tools when needed via sudo".to_string(),
        })
    } else if row.entrypoint_url.starts_with("wg://") {
        parse_entrypoint_host_port(&row.entrypoint_url).map(|(host, port)| InstanceNetworkAccess {
            mode: "wireguard".to_string(),
            host,
            port,
            username: None,
            password: None,
            download_url: Some(instance_wireguard_config_download_url(&row)),
            note:
                "Download WireGuard config, connect VPN, then scan your isolated 10.x.x.0/24 subnet"
                    .to_string(),
        })
    } else {
        None
    };

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
        network_access,
        message,
    }
}

fn instance_wireguard_config_download_url(instance: &InstanceRow) -> String {
    format!(
        "/api/v1/instances/{}/{}/wireguard-config",
        instance.contest_id, instance.challenge_id
    )
}
