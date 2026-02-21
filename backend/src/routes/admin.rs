use std::{
    collections::HashSet, convert::Infallible, path::PathBuf, process::Stdio, sync::Arc,
    time::Instant,
};

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use axum::{
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{header, HeaderValue, StatusCode},
    response::Response,
    routing::{get, patch, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use futures_util::stream;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration as TokioDuration};
use tracing::warn;
use uuid::Uuid;

use crate::{
    auth::{self, AuthenticatedUser},
    error::{AppError, AppResult},
    password_policy::{enforce_password_policy, PasswordContext},
    routes::instances,
    runtime_template::{
        build_single_image_compose_template, parse_runtime_metadata_options,
        validate_compose_template_schema, RuntimeMode,
    },
    state::AppState,
};

const DIFFICULTY_ALLOWED: &[&str] = &["easy", "normal", "hard", "insane"];
const CHALLENGE_TYPE_ALLOWED: &[&str] = &["static", "dynamic", "internal"];
const FLAG_MODE_ALLOWED: &[&str] = &["static", "dynamic", "script"];
const CONTEST_STATUS_ALLOWED: &[&str] = &["draft", "scheduled", "running", "ended", "archived"];
const CONTEST_VISIBILITY_ALLOWED: &[&str] = &["public", "private"];
const CONTEST_SCORING_MODE_ALLOWED: &[&str] = &["static", "dynamic"];
const CONTEST_REGISTRATION_STATUS_ALLOWED: &[&str] = &["pending", "approved", "rejected"];
const TIME_DISPLAY_MODE_ALLOWED: &[&str] = &["local", "utc"];
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
const RUNTIME_ALERT_TYPE_INSTANCE_EXPIRED_NOT_DESTROYED: &str = "instance_expired_not_destroyed";
const RUNTIME_ALERT_TYPE_INSTANCE_HEARTBEAT_STALE: &str = "instance_heartbeat_stale";
const RUNTIME_ALERT_SCANNER_TYPES: &[&str] = &[
    RUNTIME_ALERT_TYPE_INSTANCE_FAILED,
    RUNTIME_ALERT_TYPE_INSTANCE_EXPIRING_SOON,
    RUNTIME_ALERT_TYPE_INSTANCE_EXPIRED_NOT_DESTROYED,
    RUNTIME_ALERT_TYPE_INSTANCE_HEARTBEAT_STALE,
];
const CONTEST_POSTER_MAX_BYTES: usize = 8 * 1024 * 1024;
const IMAGE_TEST_LOG_MAX_BYTES: usize = 256 * 1024;
const DEFAULT_CHALLENGE_ATTACHMENT_MAX_BYTES: i64 = 20 * 1024 * 1024;
const MIN_CHALLENGE_ATTACHMENT_MAX_BYTES: i64 = 1 * 1024 * 1024;
const MAX_CHALLENGE_ATTACHMENT_MAX_BYTES: i64 = 256 * 1024 * 1024;
const ATTACHMENT_UPLOAD_JSON_BODY_LIMIT_BYTES: usize = 384 * 1024 * 1024;

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

#[derive(Debug, Serialize, FromRow)]
struct AdminChallengeCategoryItem {
    id: Uuid,
    slug: String,
    display_name: String,
    sort_order: i32,
    is_builtin: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
struct AdminChallengeDetailItem {
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
    hints: Vec<String>,
    writeup_visibility: String,
    writeup_content: String,
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
    hints: Option<Vec<String>>,
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
    hints: Option<Vec<String>>,
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
    #[serde(default)]
    hints: Vec<String>,
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
    hints: Vec<String>,
    writeup_visibility: String,
    writeup_content: String,
    current_version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct ChallengeRuntimeConfigRow {
    challenge_type: String,
    compose_template: Option<String>,
    metadata: Value,
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
struct AdminChallengeRuntimeLintQuery {
    limit: Option<i64>,
    challenge_type: Option<String>,
    status: Option<String>,
    keyword: Option<String>,
    only_errors: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct TestChallengeRuntimeImageRequest {
    image: String,
    force_pull: Option<bool>,
    run_build_probe: Option<bool>,
    timeout_seconds: Option<u64>,
}

#[derive(Debug, Serialize)]
struct TestChallengeRuntimeImageStep {
    step: String,
    success: bool,
    exit_code: Option<i32>,
    duration_ms: i64,
    output: String,
    truncated: bool,
}

#[derive(Debug, Serialize)]
struct TestChallengeRuntimeImageResponse {
    image: String,
    force_pull: bool,
    run_build_probe: bool,
    succeeded: bool,
    generated_at: DateTime<Utc>,
    steps: Vec<TestChallengeRuntimeImageStep>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum TestChallengeRuntimeImageStreamEvent {
    Start {
        image: String,
        force_pull: bool,
        run_build_probe: bool,
        timeout_seconds: u64,
        generated_at: DateTime<Utc>,
    },
    StepStart {
        step: String,
        command: String,
        generated_at: DateTime<Utc>,
    },
    StepLog {
        step: String,
        stream: String,
        line: String,
        generated_at: DateTime<Utc>,
    },
    StepFinish {
        step: String,
        success: bool,
        exit_code: Option<i32>,
        duration_ms: i64,
        truncated: bool,
        generated_at: DateTime<Utc>,
    },
    Completed {
        result: TestChallengeRuntimeImageResponse,
    },
    Error {
        message: String,
        step: Option<String>,
        generated_at: DateTime<Utc>,
    },
}

#[derive(Debug, FromRow)]
struct ChallengeRuntimeLintSourceRow {
    id: Uuid,
    title: String,
    slug: String,
    challenge_type: String,
    status: String,
    is_visible: bool,
    compose_template: Option<String>,
    metadata: Value,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct AdminChallengeRuntimeLintItem {
    id: Uuid,
    title: String,
    slug: String,
    challenge_type: String,
    status: String,
    is_visible: bool,
    has_compose_template: bool,
    lint_status: String,
    message: Option<String>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct AdminChallengeRuntimeLintResponse {
    generated_at: DateTime<Utc>,
    scanned_total: i64,
    returned_total: i64,
    ok_count: i64,
    error_count: i64,
    items: Vec<AdminChallengeRuntimeLintItem>,
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

#[derive(Debug, Deserialize)]
struct CreateChallengeCategoryRequest {
    slug: String,
    display_name: Option<String>,
    sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct UpdateChallengeCategoryRequest {
    slug: Option<String>,
    display_name: Option<String>,
    sort_order: Option<i32>,
}

#[derive(Debug, Serialize, FromRow)]
struct AdminContestItem {
    id: Uuid,
    title: String,
    slug: String,
    description: String,
    poster_url: Option<String>,
    visibility: String,
    status: String,
    scoring_mode: String,
    dynamic_decay: i32,
    first_blood_bonus_percent: i32,
    second_blood_bonus_percent: i32,
    third_blood_bonus_percent: i32,
    registration_requires_approval: bool,
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
    first_blood_bonus_percent: Option<i32>,
    second_blood_bonus_percent: Option<i32>,
    third_blood_bonus_percent: Option<i32>,
    registration_requires_approval: Option<bool>,
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
    first_blood_bonus_percent: Option<i32>,
    second_blood_bonus_percent: Option<i32>,
    third_blood_bonus_percent: Option<i32>,
    registration_requires_approval: Option<bool>,
    start_at: Option<DateTime<Utc>>,
    end_at: Option<DateTime<Utc>>,
    freeze_at: Option<DateTime<Utc>>,
    clear_freeze_at: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct UpdateContestStatusRequest {
    status: String,
}

#[derive(Debug, Deserialize)]
struct UploadContestPosterRequest {
    filename: String,
    content_base64: String,
    content_type: Option<String>,
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

#[derive(Debug, Deserialize)]
struct UpdateSiteSettingsRequest {
    site_name: Option<String>,
    site_subtitle: Option<String>,
    home_title: Option<String>,
    home_tagline: Option<String>,
    home_signature: Option<String>,
    footer_text: Option<String>,
    challenge_attachment_max_bytes: Option<i64>,
    time_display_mode: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AdminContestRegistrationsQuery {
    status: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct UpdateContestRegistrationRequest {
    status: String,
    review_note: Option<String>,
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

#[derive(Debug, Serialize)]
struct AdminInstanceRuntimeMetricsService {
    container_id: String,
    container_name: String,
    service_name: Option<String>,
    image: Option<String>,
    state: Option<String>,
    health_status: Option<String>,
    restart_count: Option<i64>,
    started_at: Option<String>,
    finished_at: Option<String>,
    ip_addresses: Vec<String>,
    cpu_percent: Option<f64>,
    memory_usage_bytes: Option<i64>,
    memory_limit_bytes: Option<i64>,
    memory_percent: Option<f64>,
    net_rx_bytes: Option<i64>,
    net_tx_bytes: Option<i64>,
    block_read_bytes: Option<i64>,
    block_write_bytes: Option<i64>,
    pids: Option<i64>,
}

#[derive(Debug, Serialize)]
struct AdminInstanceRuntimeMetricsSummary {
    services_total: i64,
    running_services: i64,
    unhealthy_services: i64,
    restarting_services: i64,
    cpu_percent_total: f64,
    memory_usage_bytes_total: i64,
    memory_limit_bytes_total: i64,
}

#[derive(Debug, Serialize)]
struct AdminInstanceRuntimeMetricsResponse {
    generated_at: DateTime<Utc>,
    instance: AdminInstanceItem,
    summary: AdminInstanceRuntimeMetricsSummary,
    services: Vec<AdminInstanceRuntimeMetricsService>,
    warnings: Vec<String>,
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
struct AdminSiteSettingsItem {
    site_name: String,
    site_subtitle: String,
    home_title: String,
    home_tagline: String,
    home_signature: String,
    footer_text: String,
    challenge_attachment_max_bytes: i64,
    time_display_mode: String,
    updated_by: Option<Uuid>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
struct AdminContestRegistrationItem {
    id: Uuid,
    contest_id: Uuid,
    contest_title: String,
    team_id: Uuid,
    team_name: String,
    status: String,
    requested_by: Option<Uuid>,
    requested_by_username: Option<String>,
    requested_at: DateTime<Utc>,
    reviewed_by: Option<Uuid>,
    reviewed_by_username: Option<String>,
    reviewed_at: Option<DateTime<Utc>>,
    review_note: String,
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

#[derive(Debug)]
struct ProcessExecutionOutput {
    success: bool,
    exit_code: Option<i32>,
    duration_ms: i64,
    output: String,
    truncated: bool,
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

#[derive(Debug, Deserialize)]
struct InstanceStatsLine {
    #[serde(rename = "Container")]
    container: Option<String>,
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "CPUPerc")]
    cpu_perc: Option<String>,
    #[serde(rename = "MemUsage")]
    mem_usage: Option<String>,
    #[serde(rename = "MemPerc")]
    mem_perc: Option<String>,
    #[serde(rename = "NetIO")]
    net_io: Option<String>,
    #[serde(rename = "BlockIO")]
    block_io: Option<String>,
    #[serde(rename = "PIDs")]
    pids: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct ParsedInstanceStats {
    cpu_percent: Option<f64>,
    memory_usage_bytes: Option<i64>,
    memory_limit_bytes: Option<i64>,
    memory_percent: Option<f64>,
    net_rx_bytes: Option<i64>,
    net_tx_bytes: Option<i64>,
    block_read_bytes: Option<i64>,
    block_write_bytes: Option<i64>,
    pids: Option<i64>,
}

type InstanceStatsRow = (Option<String>, Option<String>, ParsedInstanceStats);
type InstanceStatsRows = Vec<InstanceStatsRow>;
type InstanceStatsParseOutput = (InstanceStatsRows, Vec<String>);

#[derive(Debug, Serialize)]
struct AdminInstanceReaperRunResponse {
    generated_at: DateTime<Utc>,
    mode: String,
    heartbeat_stale_seconds: Option<i64>,
    scanned: i64,
    reaped: i64,
    failed: i64,
    skipped: i64,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/admin/users", get(list_users))
        .route(
            "/admin/users/{user_id}",
            axum::routing::delete(delete_user_account),
        )
        .route("/admin/users/{user_id}/status", patch(update_user_status))
        .route("/admin/users/{user_id}/role", patch(update_user_role))
        .route(
            "/admin/users/{user_id}/reset-password",
            post(reset_user_password),
        )
        .route(
            "/admin/site-settings",
            get(get_site_settings).patch(update_site_settings),
        )
        .route(
            "/admin/challenge-categories",
            get(list_challenge_categories).post(create_challenge_category),
        )
        .route(
            "/admin/challenge-categories/{category_id}",
            patch(update_challenge_category).delete(delete_challenge_category),
        )
        .route(
            "/admin/challenges",
            get(list_challenges).post(create_challenge),
        )
        .route(
            "/admin/challenges/runtime-template/lint",
            get(lint_challenge_runtime_templates),
        )
        .route(
            "/admin/challenges/runtime-template/test-image",
            post(test_challenge_runtime_image),
        )
        .route(
            "/admin/challenges/runtime-template/test-image/stream",
            post(test_challenge_runtime_image_stream),
        )
        .route(
            "/admin/challenges/{challenge_id}",
            get(get_challenge_detail)
                .patch(update_challenge)
                .delete(delete_challenge),
        )
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
            get(list_challenge_attachments)
                .post(upload_challenge_attachment)
                .layer(DefaultBodyLimit::max(
                    ATTACHMENT_UPLOAD_JSON_BODY_LIMIT_BYTES,
                )),
        )
        .route(
            "/admin/challenges/{challenge_id}/attachments/{attachment_id}",
            axum::routing::delete(delete_challenge_attachment),
        )
        .route("/admin/contests", get(list_contests).post(create_contest))
        .route(
            "/admin/contests/{contest_id}",
            patch(update_contest).delete(delete_contest),
        )
        .route(
            "/admin/contests/{contest_id}/poster",
            post(upload_contest_poster).delete(delete_contest_poster),
        )
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
        .route(
            "/admin/contests/{contest_id}/registrations",
            get(list_contest_registrations),
        )
        .route(
            "/admin/contests/{contest_id}/registrations/{registration_id}",
            patch(update_contest_registration),
        )
        .route("/admin/instances", get(list_instances))
        .route(
            "/admin/instances/{instance_id}/runtime-metrics",
            get(get_instance_runtime_metrics),
        )
        .route("/admin/audit-logs", get(list_audit_logs))
        .route("/admin/runtime/alerts", get(list_runtime_alerts))
        .route("/admin/runtime/alerts/scan", post(scan_runtime_alerts))
        .route(
            "/admin/runtime/reaper/expired",
            post(run_expired_instance_reaper_now),
        )
        .route(
            "/admin/runtime/reaper/stale",
            post(run_stale_instance_reaper_now),
        )
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

async fn get_site_settings(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
) -> AppResult<Json<AdminSiteSettingsItem>> {
    ensure_admin(&current_user)?;

    let row = sqlx::query_as::<_, AdminSiteSettingsItem>(
        "SELECT site_name,
                site_subtitle,
                home_title,
                home_tagline,
                home_signature,
                footer_text,
                challenge_attachment_max_bytes,
                time_display_mode,
                updated_by,
                updated_at
         FROM site_settings
         WHERE id = TRUE
         LIMIT 1",
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or_else(|| AppError::Internal(anyhow::anyhow!("site settings not initialized")))?;

    Ok(Json(row))
}

async fn update_site_settings(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Json(req): Json<UpdateSiteSettingsRequest>,
) -> AppResult<Json<AdminSiteSettingsItem>> {
    ensure_admin(&current_user)?;

    let site_name = normalize_optional_setting(req.site_name.as_deref(), "site_name", 1, 80)?;
    let site_subtitle =
        normalize_optional_setting(req.site_subtitle.as_deref(), "site_subtitle", 0, 160)?;
    let home_title = normalize_optional_setting(req.home_title.as_deref(), "home_title", 1, 160)?;
    let home_tagline =
        normalize_optional_setting(req.home_tagline.as_deref(), "home_tagline", 0, 2000)?;
    let home_signature =
        normalize_optional_setting(req.home_signature.as_deref(), "home_signature", 0, 200)?;
    let footer_text =
        normalize_optional_setting(req.footer_text.as_deref(), "footer_text", 0, 240)?;
    let challenge_attachment_max_bytes = req
        .challenge_attachment_max_bytes
        .map(|value| {
            if !(MIN_CHALLENGE_ATTACHMENT_MAX_BYTES..=MAX_CHALLENGE_ATTACHMENT_MAX_BYTES)
                .contains(&value)
            {
                return Err(AppError::BadRequest(format!(
                    "challenge_attachment_max_bytes must be in {}..={}",
                    MIN_CHALLENGE_ATTACHMENT_MAX_BYTES, MAX_CHALLENGE_ATTACHMENT_MAX_BYTES
                )));
            }
            Ok(value)
        })
        .transpose()?;
    let time_display_mode = req
        .time_display_mode
        .as_deref()
        .map(|value| normalize_with_allowed(value, TIME_DISPLAY_MODE_ALLOWED, "time_display_mode"))
        .transpose()?;

    if site_name.is_none()
        && site_subtitle.is_none()
        && home_title.is_none()
        && home_tagline.is_none()
        && home_signature.is_none()
        && footer_text.is_none()
        && challenge_attachment_max_bytes.is_none()
        && time_display_mode.is_none()
    {
        return Err(AppError::BadRequest(
            "at least one site setting field is required".to_string(),
        ));
    }

    let row = sqlx::query_as::<_, AdminSiteSettingsItem>(
        "UPDATE site_settings
         SET site_name = COALESCE($1, site_name),
             site_subtitle = COALESCE($2, site_subtitle),
             home_title = COALESCE($3, home_title),
             home_tagline = COALESCE($4, home_tagline),
             home_signature = COALESCE($5, home_signature),
             footer_text = COALESCE($6, footer_text),
             challenge_attachment_max_bytes = COALESCE($7, challenge_attachment_max_bytes),
             time_display_mode = COALESCE($8, time_display_mode),
             updated_by = $9,
             updated_at = NOW()
         WHERE id = TRUE
         RETURNING site_name,
                   site_subtitle,
                   home_title,
                   home_tagline,
                   home_signature,
                   footer_text,
                   challenge_attachment_max_bytes,
                   time_display_mode,
                   updated_by,
                   updated_at",
    )
    .bind(site_name)
    .bind(site_subtitle)
    .bind(home_title)
    .bind(home_tagline)
    .bind(home_signature)
    .bind(footer_text)
    .bind(challenge_attachment_max_bytes)
    .bind(time_display_mode)
    .bind(current_user.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or_else(|| AppError::Internal(anyhow::anyhow!("site settings not initialized")))?;

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.site.settings.update",
        "site_settings",
        None,
        json!({
            "site_name": row.site_name,
            "site_subtitle": row.site_subtitle,
            "home_title": row.home_title,
            "footer_text": row.footer_text,
            "challenge_attachment_max_bytes": row.challenge_attachment_max_bytes,
            "time_display_mode": row.time_display_mode
        }),
    )
    .await;

    Ok(Json(row))
}

async fn list_challenge_categories(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
) -> AppResult<Json<Vec<AdminChallengeCategoryItem>>> {
    ensure_admin_or_judge(&current_user)?;

    let rows = sqlx::query_as::<_, AdminChallengeCategoryItem>(
        "SELECT id,
                slug,
                display_name,
                sort_order,
                is_builtin,
                created_at,
                updated_at
         FROM challenge_categories
         ORDER BY sort_order ASC, created_at ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(rows))
}

async fn create_challenge_category(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Json(req): Json<CreateChallengeCategoryRequest>,
) -> AppResult<Json<AdminChallengeCategoryItem>> {
    ensure_admin_or_judge(&current_user)?;

    let slug = normalize_challenge_category_slug(req.slug.as_str())?;
    let display_name = normalize_challenge_category_display_name(
        req.display_name.as_deref().unwrap_or(slug.as_str()),
    )?;
    let sort_order = req.sort_order.unwrap_or(100);
    if !(-100_000..=100_000).contains(&sort_order) {
        return Err(AppError::BadRequest(
            "sort_order must be between -100000 and 100000".to_string(),
        ));
    }

    let row = sqlx::query_as::<_, AdminChallengeCategoryItem>(
        "INSERT INTO challenge_categories (
            slug,
            display_name,
            sort_order,
            is_builtin
         )
         VALUES ($1, $2, $3, FALSE)
         RETURNING id,
                   slug,
                   display_name,
                   sort_order,
                   is_builtin,
                   created_at,
                   updated_at",
    )
    .bind(&slug)
    .bind(&display_name)
    .bind(sort_order)
    .fetch_one(&state.db)
    .await
    .map_err(|err| {
        if is_unique_violation(&err) {
            AppError::Conflict("challenge category slug already exists".to_string())
        } else {
            AppError::internal(err)
        }
    })?;

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.challenge_category.create",
        "challenge_category",
        Some(row.id),
        json!({
            "slug": &row.slug,
            "display_name": &row.display_name,
            "sort_order": row.sort_order
        }),
    )
    .await;

    Ok(Json(row))
}

async fn update_challenge_category(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(category_id): Path<Uuid>,
    Json(req): Json<UpdateChallengeCategoryRequest>,
) -> AppResult<Json<AdminChallengeCategoryItem>> {
    ensure_admin_or_judge(&current_user)?;

    let existing = sqlx::query_as::<_, AdminChallengeCategoryItem>(
        "SELECT id,
                slug,
                display_name,
                sort_order,
                is_builtin,
                created_at,
                updated_at
         FROM challenge_categories
         WHERE id = $1
         LIMIT 1",
    )
    .bind(category_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest(
        "challenge category not found".to_string(),
    ))?;

    let slug = match req.slug {
        Some(value) => normalize_challenge_category_slug(value.as_str())?,
        None => existing.slug.clone(),
    };
    if existing.is_builtin && slug != existing.slug {
        return Err(AppError::Conflict(
            "builtin challenge category slug cannot be changed".to_string(),
        ));
    }

    let display_name = match req.display_name {
        Some(value) => normalize_challenge_category_display_name(value.as_str())?,
        None => existing.display_name.clone(),
    };
    let sort_order = req.sort_order.unwrap_or(existing.sort_order);
    if !(-100_000..=100_000).contains(&sort_order) {
        return Err(AppError::BadRequest(
            "sort_order must be between -100000 and 100000".to_string(),
        ));
    }

    let row = sqlx::query_as::<_, AdminChallengeCategoryItem>(
        "UPDATE challenge_categories
         SET slug = $2,
             display_name = $3,
             sort_order = $4,
             updated_at = NOW()
         WHERE id = $1
         RETURNING id,
                   slug,
                   display_name,
                   sort_order,
                   is_builtin,
                   created_at,
                   updated_at",
    )
    .bind(category_id)
    .bind(&slug)
    .bind(&display_name)
    .bind(sort_order)
    .fetch_one(&state.db)
    .await
    .map_err(|err| {
        if is_unique_violation(&err) {
            AppError::Conflict("challenge category slug already exists".to_string())
        } else {
            AppError::internal(err)
        }
    })?;

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.challenge_category.update",
        "challenge_category",
        Some(row.id),
        json!({
            "slug": &row.slug,
            "display_name": &row.display_name,
            "sort_order": row.sort_order
        }),
    )
    .await;

    Ok(Json(row))
}

async fn delete_challenge_category(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(category_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    ensure_admin_or_judge(&current_user)?;

    let row = sqlx::query_as::<_, AdminChallengeCategoryItem>(
        "SELECT id,
                slug,
                display_name,
                sort_order,
                is_builtin,
                created_at,
                updated_at
         FROM challenge_categories
         WHERE id = $1
         LIMIT 1",
    )
    .bind(category_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest(
        "challenge category not found".to_string(),
    ))?;

    if row.is_builtin {
        return Err(AppError::Conflict(
            "builtin challenge category cannot be deleted".to_string(),
        ));
    }

    let usage_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1)
         FROM challenges
         WHERE LOWER(category) = LOWER($1)",
    )
    .bind(&row.slug)
    .fetch_one(&state.db)
    .await
    .map_err(AppError::internal)?;
    if usage_count > 0 {
        return Err(AppError::Conflict(
            "challenge category is in use by existing challenges".to_string(),
        ));
    }

    sqlx::query("DELETE FROM challenge_categories WHERE id = $1")
        .bind(category_id)
        .execute(&state.db)
        .await
        .map_err(AppError::internal)?;

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.challenge_category.delete",
        "challenge_category",
        Some(row.id),
        json!({
            "slug": &row.slug,
            "display_name": &row.display_name
        }),
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

async fn delete_user_account(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(user_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    ensure_admin(&current_user)?;

    if current_user.user_id == user_id {
        return Err(AppError::BadRequest(
            "cannot delete current admin user".to_string(),
        ));
    }

    let target = sqlx::query_as::<_, AdminUserItem>(
        "SELECT id,
                username,
                email,
                role,
                status,
                created_at,
                updated_at
         FROM users
         WHERE id = $1
         LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("user not found".to_string()))?;

    if target.role == "admin" && target.status == "active" {
        let remaining_active_admins = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(1) FROM users WHERE role = 'admin' AND status = 'active' AND id <> $1",
        )
        .bind(user_id)
        .fetch_one(&state.db)
        .await
        .map_err(AppError::internal)?;
        if remaining_active_admins <= 0 {
            return Err(AppError::Conflict(
                "cannot delete the last active admin account".to_string(),
            ));
        }
    }

    let user_simple = user_id.as_simple().to_string();
    let deleted_username = format!("deleted_{}", &user_simple[..12]);
    let deleted_email = format!("deleted+{}@deleted.local", user_simple);
    let deleted_password_hash = hash_password(Uuid::new_v4().to_string().as_str())?;

    sqlx::query(
        "UPDATE users
         SET username = $2,
             email = $3,
             password_hash = $4,
             role = 'player',
             status = 'disabled',
             updated_at = NOW()
         WHERE id = $1",
    )
    .bind(user_id)
    .bind(&deleted_username)
    .bind(&deleted_email)
    .bind(&deleted_password_hash)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    auth::revoke_all_user_sessions(state.as_ref(), user_id).await?;

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.user.delete",
        "user",
        Some(user_id),
        json!({
            "target_user_id": user_id,
            "target_username": target.username,
            "target_email": target.email,
            "target_role": target.role,
            "target_status": target.status
        }),
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
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

    let target = sqlx::query_as::<_, AdminUserItem>(
        "SELECT id,
                username,
                email,
                role,
                status,
                created_at,
                updated_at
         FROM users
         WHERE id = $1
         LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("user not found".to_string()))?;

    enforce_password_policy(
        &state.config,
        &req.new_password,
        PasswordContext {
            username: Some(&target.username),
            email: Some(&target.email),
        },
    )
    .map_err(AppError::BadRequest)?;

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

async fn get_challenge_detail(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(challenge_id): Path<Uuid>,
) -> AppResult<Json<AdminChallengeDetailItem>> {
    ensure_admin_or_judge(&current_user)?;

    let row = sqlx::query_as::<_, AdminChallengeDetailItem>(
        "SELECT id,
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
                hints,
                writeup_visibility,
                writeup_content,
                current_version,
                created_at,
                updated_at
         FROM challenges
         WHERE id = $1
         LIMIT 1",
    )
    .bind(challenge_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("challenge not found".to_string()))?;

    Ok(Json(row))
}

async fn lint_challenge_runtime_templates(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Query(query): Query<AdminChallengeRuntimeLintQuery>,
) -> AppResult<Json<AdminChallengeRuntimeLintResponse>> {
    ensure_admin_or_judge(&current_user)?;

    let challenge_type_filter = query
        .challenge_type
        .as_deref()
        .map(|value| normalize_with_allowed(value, CHALLENGE_TYPE_ALLOWED, "challenge_type"))
        .transpose()?;
    let status_filter = query
        .status
        .as_deref()
        .map(|value| normalize_with_allowed(value, CHALLENGE_STATUS_ALLOWED, "status"))
        .transpose()?;
    let keyword_filter = query
        .keyword
        .as_deref()
        .and_then(normalize_optional_text)
        .map(|value| format!("%{}%", value.to_lowercase()));
    let only_errors = query.only_errors.unwrap_or(false);
    let limit = query.limit.unwrap_or(500).clamp(1, 5000);

    let rows = sqlx::query_as::<_, ChallengeRuntimeLintSourceRow>(
        "SELECT id,
                title,
                slug,
                challenge_type,
                status,
                is_visible,
                compose_template,
                metadata,
                updated_at
         FROM challenges
         WHERE ($1::text IS NULL OR challenge_type = $1)
           AND ($2::text IS NULL OR status = $2)
           AND ($3::text IS NULL OR LOWER(title) LIKE $3 OR LOWER(slug) LIKE $3)
         ORDER BY updated_at DESC
         LIMIT $4",
    )
    .bind(challenge_type_filter)
    .bind(status_filter)
    .bind(keyword_filter)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    let mut ok_count = 0_i64;
    let mut error_count = 0_i64;
    let mut items: Vec<AdminChallengeRuntimeLintItem> = Vec::new();

    for row in rows {
        let has_compose_template = row
            .compose_template
            .as_deref()
            .and_then(normalize_optional_text)
            .is_some();

        match validate_compose_runtime_configuration(
            &row.challenge_type,
            row.compose_template.as_deref(),
            &row.metadata,
        ) {
            Ok(()) => {
                ok_count += 1;
                if !only_errors {
                    items.push(AdminChallengeRuntimeLintItem {
                        id: row.id,
                        title: row.title,
                        slug: row.slug,
                        challenge_type: row.challenge_type,
                        status: row.status,
                        is_visible: row.is_visible,
                        has_compose_template,
                        lint_status: "ok".to_string(),
                        message: None,
                        updated_at: row.updated_at,
                    });
                }
            }
            Err(AppError::BadRequest(message)) => {
                error_count += 1;
                items.push(AdminChallengeRuntimeLintItem {
                    id: row.id,
                    title: row.title,
                    slug: row.slug,
                    challenge_type: row.challenge_type,
                    status: row.status,
                    is_visible: row.is_visible,
                    has_compose_template,
                    lint_status: "error".to_string(),
                    message: Some(message),
                    updated_at: row.updated_at,
                });
            }
            Err(other) => return Err(other),
        }
    }

    Ok(Json(AdminChallengeRuntimeLintResponse {
        generated_at: Utc::now(),
        scanned_total: ok_count + error_count,
        returned_total: items.len() as i64,
        ok_count,
        error_count,
        items,
    }))
}

async fn test_challenge_runtime_image(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Json(req): Json<TestChallengeRuntimeImageRequest>,
) -> AppResult<Json<TestChallengeRuntimeImageResponse>> {
    ensure_admin_or_judge(&current_user)?;

    let image = validate_container_image_reference(req.image.as_str())?;
    let force_pull = req.force_pull.unwrap_or(true);
    let run_build_probe = req.run_build_probe.unwrap_or(true);
    let timeout_seconds = req
        .timeout_seconds
        .unwrap_or(state.config.compose_command_timeout_seconds)
        .clamp(10, 900);

    let mut steps: Vec<TestChallengeRuntimeImageStep> = Vec::new();

    let temp_runtime_context = std::env::temp_dir().join(format!(
        "rust-ctf-image-test-runtime-{}",
        Uuid::new_v4().as_simple()
    ));
    fs::create_dir_all(&temp_runtime_context)
        .await
        .map_err(AppError::internal)?;

    let runtime_compose_file = temp_runtime_context.join("docker-compose.yml");
    let runtime_compose_content =
        format!("version: \"3.9\"\nservices:\n  runtime_image_probe:\n    image: {image}\n");
    fs::write(&runtime_compose_file, runtime_compose_content)
        .await
        .map_err(AppError::internal)?;
    let runtime_compose_path = runtime_compose_file.to_string_lossy().to_string();
    let runtime_project = format!("imgtest{}", Uuid::new_v4().as_simple());

    if force_pull {
        let pull_args = vec![
            "-f".to_string(),
            runtime_compose_path.clone(),
            "-p".to_string(),
            runtime_project.clone(),
            "pull".to_string(),
            "runtime_image_probe".to_string(),
        ];

        let pull_output = run_compose_compatible_external_command(
            &pull_args,
            None,
            Some(&temp_runtime_context),
            timeout_seconds,
        )
        .await?;
        steps.push(TestChallengeRuntimeImageStep {
            step: "runtime_pull".to_string(),
            success: pull_output.success,
            exit_code: pull_output.exit_code,
            duration_ms: pull_output.duration_ms,
            output: pull_output.output,
            truncated: pull_output.truncated,
        });
    }

    let inspect_args = vec![
        "-f".to_string(),
        runtime_compose_path.clone(),
        "-p".to_string(),
        runtime_project,
        "config".to_string(),
    ];
    let inspect_output = run_compose_compatible_external_command(
        &inspect_args,
        None,
        Some(&temp_runtime_context),
        timeout_seconds,
    )
    .await?;
    steps.push(TestChallengeRuntimeImageStep {
        step: "runtime_config_validate".to_string(),
        success: inspect_output.success,
        exit_code: inspect_output.exit_code,
        duration_ms: inspect_output.duration_ms,
        output: inspect_output.output,
        truncated: inspect_output.truncated,
    });

    if let Err(err) = fs::remove_dir_all(&temp_runtime_context).await {
        if err.kind() != std::io::ErrorKind::NotFound {
            warn!(
                path = %temp_runtime_context.to_string_lossy(),
                error = %err,
                "failed to cleanup image test runtime context"
            );
        }
    }

    if run_build_probe {
        let probe_tag = format!("rust-ctf-image-probe:{}", Uuid::new_v4().as_simple());
        let dockerfile_content = format!("FROM {image}\nLABEL rust_ctf_image_probe=\"1\"\n");

        let temp_build_context = std::env::temp_dir().join(format!(
            "rust-ctf-image-test-build-{}",
            Uuid::new_v4().as_simple()
        ));
        fs::create_dir_all(&temp_build_context)
            .await
            .map_err(AppError::internal)?;

        let build_compose_file = temp_build_context.join("docker-compose.yml");
        let build_compose_content = format!(
            "version: \"3.9\"\nservices:\n  runtime_build_probe:\n    image: {probe_tag}\n    build:\n      context: .\n      dockerfile: Dockerfile\n"
        );

        fs::write(temp_build_context.join("Dockerfile"), dockerfile_content)
            .await
            .map_err(AppError::internal)?;
        fs::write(&build_compose_file, build_compose_content)
            .await
            .map_err(AppError::internal)?;

        let build_compose_path = build_compose_file.to_string_lossy().to_string();
        let build_project = format!("imgprobe{}", Uuid::new_v4().as_simple());

        let mut build_args = vec![
            "-f".to_string(),
            build_compose_path.clone(),
            "-p".to_string(),
            build_project.clone(),
            "build".to_string(),
        ];
        if force_pull {
            build_args.push("--pull".to_string());
        }
        build_args.push("runtime_build_probe".to_string());

        let build_output = run_compose_compatible_external_command(
            &build_args,
            None,
            Some(&temp_build_context),
            timeout_seconds,
        )
        .await?;
        steps.push(TestChallengeRuntimeImageStep {
            step: "runtime_build_probe".to_string(),
            success: build_output.success,
            exit_code: build_output.exit_code,
            duration_ms: build_output.duration_ms,
            output: build_output.output,
            truncated: build_output.truncated,
        });

        let cleanup_args = vec![
            "-f".to_string(),
            build_compose_path,
            "-p".to_string(),
            build_project,
            "down".to_string(),
            "--rmi".to_string(),
            "local".to_string(),
            "--volumes".to_string(),
            "--remove-orphans".to_string(),
        ];
        let cleanup_output = run_compose_compatible_external_command(
            &cleanup_args,
            None,
            Some(&temp_build_context),
            timeout_seconds,
        )
        .await?;
        steps.push(TestChallengeRuntimeImageStep {
            step: "runtime_cleanup_probe".to_string(),
            success: cleanup_output.success,
            exit_code: cleanup_output.exit_code,
            duration_ms: cleanup_output.duration_ms,
            output: cleanup_output.output,
            truncated: cleanup_output.truncated,
        });

        if let Err(err) = fs::remove_dir_all(&temp_build_context).await {
            if err.kind() != std::io::ErrorKind::NotFound {
                warn!(
                    path = %temp_build_context.to_string_lossy(),
                    error = %err,
                    "failed to cleanup image test build context"
                );
            }
        }
    }

    let succeeded = steps
        .iter()
        .filter(|item| item.step != "runtime_cleanup_probe")
        .all(|item| item.success);

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.challenge.runtime.image_test",
        "challenge_runtime",
        None,
        json!({
            "image": &image,
            "force_pull": force_pull,
            "run_build_probe": run_build_probe,
            "timeout_seconds": timeout_seconds,
            "succeeded": succeeded,
            "step_count": steps.len()
        }),
    )
    .await;

    Ok(Json(TestChallengeRuntimeImageResponse {
        image,
        force_pull,
        run_build_probe,
        succeeded,
        generated_at: Utc::now(),
        steps,
    }))
}

async fn test_challenge_runtime_image_stream(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Json(req): Json<TestChallengeRuntimeImageRequest>,
) -> AppResult<Response> {
    ensure_admin_or_judge(&current_user)?;

    let image = validate_container_image_reference(req.image.as_str())?;
    let force_pull = req.force_pull.unwrap_or(true);
    let run_build_probe = req.run_build_probe.unwrap_or(true);
    let timeout_seconds = req
        .timeout_seconds
        .unwrap_or(state.config.compose_command_timeout_seconds)
        .clamp(10, 900);

    let (sender, receiver) = mpsc::unbounded_channel::<Result<Bytes, Infallible>>();
    let state_clone = Arc::clone(&state);
    let current_user_clone = current_user.clone();

    tokio::spawn(async move {
        emit_test_challenge_runtime_image_stream_event(
            &sender,
            TestChallengeRuntimeImageStreamEvent::Start {
                image: image.clone(),
                force_pull,
                run_build_probe,
                timeout_seconds,
                generated_at: Utc::now(),
            },
        );

        match execute_test_challenge_runtime_image_stream(
            state_clone,
            &current_user_clone,
            &image,
            force_pull,
            run_build_probe,
            timeout_seconds,
            &sender,
        )
        .await
        {
            Ok(result) => {
                emit_test_challenge_runtime_image_stream_event(
                    &sender,
                    TestChallengeRuntimeImageStreamEvent::Completed { result },
                );
            }
            Err(err) => {
                emit_test_challenge_runtime_image_stream_event(
                    &sender,
                    TestChallengeRuntimeImageStreamEvent::Error {
                        message: image_test_stream_error_message(&err),
                        step: None,
                        generated_at: Utc::now(),
                    },
                );
            }
        }
    });

    let body_stream = stream::unfold(receiver, |mut receiver| async move {
        receiver.recv().await.map(|item| (item, receiver))
    });

    let mut response = Response::new(Body::from_stream(body_stream));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/x-ndjson; charset=utf-8"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));

    Ok(response)
}

async fn execute_test_challenge_runtime_image_stream(
    state: Arc<AppState>,
    current_user: &AuthenticatedUser,
    image: &str,
    force_pull: bool,
    run_build_probe: bool,
    timeout_seconds: u64,
    sender: &mpsc::UnboundedSender<Result<Bytes, Infallible>>,
) -> AppResult<TestChallengeRuntimeImageResponse> {
    let mut steps: Vec<TestChallengeRuntimeImageStep> = Vec::new();

    let temp_runtime_context = std::env::temp_dir().join(format!(
        "rust-ctf-image-test-runtime-{}",
        Uuid::new_v4().as_simple()
    ));
    fs::create_dir_all(&temp_runtime_context)
        .await
        .map_err(AppError::internal)?;

    let runtime_result = async {
        let runtime_compose_file = temp_runtime_context.join("docker-compose.yml");
        let runtime_compose_content =
            format!("version: \"3.9\"\nservices:\n  runtime_image_probe:\n    image: {image}\n");
        fs::write(&runtime_compose_file, runtime_compose_content)
            .await
            .map_err(AppError::internal)?;
        let runtime_compose_path = runtime_compose_file.to_string_lossy().to_string();
        let runtime_project = format!("imgtest{}", Uuid::new_v4().as_simple());

        if force_pull {
            let pull_args = vec![
                "-f".to_string(),
                runtime_compose_path.clone(),
                "-p".to_string(),
                runtime_project.clone(),
                "pull".to_string(),
                "runtime_image_probe".to_string(),
            ];
            let pull_step = run_runtime_image_test_step_stream(
                "runtime_pull",
                &pull_args,
                &temp_runtime_context,
                timeout_seconds,
                sender,
            )
            .await?;
            steps.push(pull_step);
        }

        let inspect_args = vec![
            "-f".to_string(),
            runtime_compose_path.clone(),
            "-p".to_string(),
            runtime_project,
            "config".to_string(),
        ];
        let inspect_step = run_runtime_image_test_step_stream(
            "runtime_config_validate",
            &inspect_args,
            &temp_runtime_context,
            timeout_seconds,
            sender,
        )
        .await?;
        steps.push(inspect_step);

        Ok::<(), AppError>(())
    }
    .await;

    if let Err(err) = fs::remove_dir_all(&temp_runtime_context).await {
        if err.kind() != std::io::ErrorKind::NotFound {
            warn!(
                path = %temp_runtime_context.to_string_lossy(),
                error = %err,
                "failed to cleanup image test runtime context"
            );
        }
    }
    runtime_result?;

    if run_build_probe {
        let probe_tag = format!("rust-ctf-image-probe:{}", Uuid::new_v4().as_simple());
        let dockerfile_content = format!("FROM {image}\nLABEL rust_ctf_image_probe=\"1\"\n");
        let temp_build_context = std::env::temp_dir().join(format!(
            "rust-ctf-image-test-build-{}",
            Uuid::new_v4().as_simple()
        ));
        fs::create_dir_all(&temp_build_context)
            .await
            .map_err(AppError::internal)?;

        let build_result = async {
            let build_compose_file = temp_build_context.join("docker-compose.yml");
            let build_compose_content = format!(
                "version: \"3.9\"\nservices:\n  runtime_build_probe:\n    image: {probe_tag}\n    build:\n      context: .\n      dockerfile: Dockerfile\n"
            );

            fs::write(temp_build_context.join("Dockerfile"), dockerfile_content)
                .await
                .map_err(AppError::internal)?;
            fs::write(&build_compose_file, build_compose_content)
                .await
                .map_err(AppError::internal)?;

            let build_compose_path = build_compose_file.to_string_lossy().to_string();
            let build_project = format!("imgprobe{}", Uuid::new_v4().as_simple());

            let mut build_args = vec![
                "-f".to_string(),
                build_compose_path.clone(),
                "-p".to_string(),
                build_project.clone(),
                "build".to_string(),
            ];
            if force_pull {
                build_args.push("--pull".to_string());
            }
            build_args.push("runtime_build_probe".to_string());

            let build_step = run_runtime_image_test_step_stream(
                "runtime_build_probe",
                &build_args,
                &temp_build_context,
                timeout_seconds,
                sender,
            )
            .await?;
            steps.push(build_step);

            let cleanup_args = vec![
                "-f".to_string(),
                build_compose_path,
                "-p".to_string(),
                build_project,
                "down".to_string(),
                "--rmi".to_string(),
                "local".to_string(),
                "--volumes".to_string(),
                "--remove-orphans".to_string(),
            ];
            let cleanup_step = run_runtime_image_test_step_stream(
                "runtime_cleanup_probe",
                &cleanup_args,
                &temp_build_context,
                timeout_seconds,
                sender,
            )
            .await?;
            steps.push(cleanup_step);

            Ok::<(), AppError>(())
        }
        .await;

        if let Err(err) = fs::remove_dir_all(&temp_build_context).await {
            if err.kind() != std::io::ErrorKind::NotFound {
                warn!(
                    path = %temp_build_context.to_string_lossy(),
                    error = %err,
                    "failed to cleanup image test build context"
                );
            }
        }

        build_result?;
    }

    let succeeded = steps
        .iter()
        .filter(|item| item.step != "runtime_cleanup_probe")
        .all(|item| item.success);

    record_audit_log(
        state.as_ref(),
        current_user,
        "admin.challenge.runtime.image_test",
        "challenge_runtime",
        None,
        json!({
            "image": image,
            "force_pull": force_pull,
            "run_build_probe": run_build_probe,
            "timeout_seconds": timeout_seconds,
            "succeeded": succeeded,
            "step_count": steps.len()
        }),
    )
    .await;

    Ok(TestChallengeRuntimeImageResponse {
        image: image.to_string(),
        force_pull,
        run_build_probe,
        succeeded,
        generated_at: Utc::now(),
        steps,
    })
}

async fn run_runtime_image_test_step_stream(
    step: &str,
    args: &[String],
    workdir: &std::path::Path,
    timeout_seconds: u64,
    sender: &mpsc::UnboundedSender<Result<Bytes, Infallible>>,
) -> AppResult<TestChallengeRuntimeImageStep> {
    let step_owned = step.to_string();
    emit_test_challenge_runtime_image_stream_event(
        sender,
        TestChallengeRuntimeImageStreamEvent::StepStart {
            step: step_owned.clone(),
            command: compose_command_preview(args),
            generated_at: Utc::now(),
        },
    );

    let output = run_compose_compatible_external_command_stream(
        args,
        None,
        Some(workdir),
        timeout_seconds,
        |stream_name, line| {
            emit_test_challenge_runtime_image_stream_event(
                sender,
                TestChallengeRuntimeImageStreamEvent::StepLog {
                    step: step_owned.clone(),
                    stream: stream_name.to_string(),
                    line: line.to_string(),
                    generated_at: Utc::now(),
                },
            );
        },
    )
    .await?;

    emit_test_challenge_runtime_image_stream_event(
        sender,
        TestChallengeRuntimeImageStreamEvent::StepFinish {
            step: step_owned.clone(),
            success: output.success,
            exit_code: output.exit_code,
            duration_ms: output.duration_ms,
            truncated: output.truncated,
            generated_at: Utc::now(),
        },
    );

    Ok(TestChallengeRuntimeImageStep {
        step: step_owned,
        success: output.success,
        exit_code: output.exit_code,
        duration_ms: output.duration_ms,
        output: output.output,
        truncated: output.truncated,
    })
}

fn emit_test_challenge_runtime_image_stream_event(
    sender: &mpsc::UnboundedSender<Result<Bytes, Infallible>>,
    event: TestChallengeRuntimeImageStreamEvent,
) {
    match serde_json::to_string(&event) {
        Ok(mut line) => {
            line.push('\n');
            let _ = sender.send(Ok(Bytes::from(line)));
        }
        Err(err) => warn!(error = %err, "failed to serialize image test stream event"),
    }
}

fn image_test_stream_error_message(err: &AppError) -> String {
    match err {
        AppError::BadRequest(message) => message.clone(),
        AppError::Unauthorized => "invalid credentials or token".to_string(),
        AppError::Forbidden => "permission denied".to_string(),
        AppError::TooManyRequests(message) => message.clone(),
        AppError::Conflict(message) => message.clone(),
        AppError::Internal(_) => "unexpected server error".to_string(),
    }
}

fn compose_command_preview(args: &[String]) -> String {
    if args.is_empty() {
        return "docker compose".to_string();
    }
    format!("docker compose {}", args.join(" "))
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
    ensure_challenge_category_exists(state.as_ref(), category.as_str()).await?;
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
    let compose_template = req.compose_template.and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });
    let metadata = req.metadata.unwrap_or(Value::Object(Default::default()));
    validate_compose_runtime_configuration(
        &challenge_type,
        compose_template.as_deref(),
        &metadata,
    )?;
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
    let hints = normalize_hints(req.hints.unwrap_or_default())?;
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
            hints,
            writeup_visibility,
            writeup_content,
            created_by
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
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
                   hints,
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
    .bind(hints)
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
    .bind(change_note.as_deref().unwrap_or("initial version"))
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
        .map(|value| {
            normalize_with_allowed(value, WRITEUP_VISIBILITY_ALLOWED, "writeup_visibility")
        })
        .transpose()?;
    let normalized_tags = req.tags.map(normalize_tags).transpose()?;
    let normalized_hints = req.hints.map(normalize_hints).transpose()?;
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
    if let Some(category_value) = category.as_deref() {
        ensure_challenge_category_exists(state.as_ref(), category_value).await?;
    }
    let compose_template_patch = req
        .compose_template
        .as_ref()
        .map(|value| value.trim().to_string());

    let existing_runtime = load_challenge_runtime_config(state.as_ref(), challenge_id).await?;
    let effective_challenge_type = normalized_challenge_type
        .as_deref()
        .unwrap_or(existing_runtime.challenge_type.as_str());
    let effective_template = compose_template_patch
        .as_deref()
        .and_then(|value| if value.is_empty() { None } else { Some(value) })
        .or_else(|| {
            existing_runtime
                .compose_template
                .as_deref()
                .and_then(normalize_optional_text)
        });
    let effective_metadata = req.metadata.as_ref().unwrap_or(&existing_runtime.metadata);

    validate_compose_runtime_configuration(
        effective_challenge_type,
        effective_template,
        effective_metadata,
    )?;

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
              hints = COALESCE($15, hints),
              writeup_visibility = COALESCE($16, writeup_visibility),
              writeup_content = COALESCE($17, writeup_content),
              status = COALESCE($18, status),
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
                   hints,
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
    .bind(compose_template_patch)
    .bind(req.metadata)
    .bind(resolved_is_visible)
    .bind(normalized_tags)
    .bind(normalized_hints)
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

async fn delete_challenge(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(challenge_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    ensure_admin_or_judge(&current_user)?;
    ensure_challenge_exists(state.as_ref(), challenge_id).await?;

    let instance_cleanup =
        instances::destroy_instances_for_challenge(state.as_ref(), challenge_id).await?;
    if instance_cleanup.failed > 0 {
        return Err(AppError::Conflict(format!(
            "failed to destroy {} runtime instances, resolve runtime errors and retry",
            instance_cleanup.failed
        )));
    }

    let attachment_paths = sqlx::query_scalar::<_, String>(
        "SELECT storage_path
         FROM challenge_attachments
         WHERE challenge_id = $1",
    )
    .bind(challenge_id)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    let deleted = sqlx::query_as::<_, (Uuid, String)>(
        "DELETE FROM challenges
         WHERE id = $1
         RETURNING id, title",
    )
    .bind(challenge_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("challenge not found".to_string()))?;

    for storage_path in &attachment_paths {
        let path = resolve_challenge_attachment_storage_path(
            state.as_ref(),
            challenge_id,
            storage_path,
        );
        if let Err(err) = fs::remove_file(&path).await {
            if err.kind() != std::io::ErrorKind::NotFound {
                return Err(AppError::internal(err));
            }
        }
    }

    let attachment_dir = challenge_attachments_dir(state.as_ref(), challenge_id);
    if let Err(err) = fs::remove_dir_all(&attachment_dir).await {
        if err.kind() != std::io::ErrorKind::NotFound {
            return Err(AppError::internal(err));
        }
    }

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.challenge.delete",
        "challenge",
        Some(deleted.0),
        json!({
            "title": deleted.1,
            "attachment_count": attachment_paths.len(),
            "instance_cleanup": {
                "scanned": instance_cleanup.scanned,
                "destroyed": instance_cleanup.reaped,
                "failed": instance_cleanup.failed,
                "skipped": instance_cleanup.skipped
            }
        }),
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
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
        return Err(AppError::BadRequest("version_no must be >= 1".to_string()));
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
    let rollback_status =
        normalize_with_allowed(&target_snapshot.status, CHALLENGE_STATUS_ALLOWED, "status")?;
    let rollback_visible = rollback_status == "published";
    validate_compose_runtime_configuration(
        &target_snapshot.challenge_type,
        target_snapshot.compose_template.as_deref(),
        &target_snapshot.metadata,
    )?;

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
             hints = $17,
             writeup_visibility = $18,
             writeup_content = $19,
             status = $20,
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
                   hints,
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
    .bind(target_snapshot.hints)
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
    let attachment_limit_bytes = load_challenge_attachment_limit_bytes(state.as_ref()).await?;

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
    if decoded.len() as i64 > attachment_limit_bytes {
        return Err(AppError::BadRequest(format!(
            "attachment size must be <= {}",
            format_bytes_for_message(attachment_limit_bytes)
        )));
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
    let stored_path = attachment_dir.join(&stored_name);
    let stored_rel_path = PathBuf::from("_challenge_files")
        .join(challenge_id.to_string())
        .join(&stored_name);
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
    .bind(stored_rel_path.to_string_lossy().to_string())
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

    let path = resolve_challenge_attachment_storage_path(
        state.as_ref(),
        row.challenge_id,
        &row.storage_path,
    );
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
                CASE
                    WHEN poster_storage_path IS NULL OR poster_storage_path = '' THEN NULL
                    ELSE '/api/v1/contests/' || id::text || '/poster'
                END AS poster_url,
                visibility,
                status,
                scoring_mode,
                dynamic_decay,
                first_blood_bonus_percent,
                second_blood_bonus_percent,
                third_blood_bonus_percent,
                registration_requires_approval,
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
    let first_blood_bonus_percent = validate_blood_bonus_percent(
        req.first_blood_bonus_percent.unwrap_or(10),
        "first_blood_bonus_percent",
    )?;
    let second_blood_bonus_percent = validate_blood_bonus_percent(
        req.second_blood_bonus_percent.unwrap_or(5),
        "second_blood_bonus_percent",
    )?;
    let third_blood_bonus_percent = validate_blood_bonus_percent(
        req.third_blood_bonus_percent.unwrap_or(2),
        "third_blood_bonus_percent",
    )?;
    let registration_requires_approval = req.registration_requires_approval.unwrap_or(true);

    let row = sqlx::query_as::<_, AdminContestItem>(
        "INSERT INTO contests (
            title,
            slug,
            description,
            visibility,
            status,
            scoring_mode,
            dynamic_decay,
            first_blood_bonus_percent,
            second_blood_bonus_percent,
            third_blood_bonus_percent,
            registration_requires_approval,
            start_at,
            end_at,
            freeze_at,
            created_by
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
         RETURNING id,
                   title,
                   slug,
                   description,
                   CASE
                       WHEN poster_storage_path IS NULL OR poster_storage_path = '' THEN NULL
                       ELSE '/api/v1/contests/' || id::text || '/poster'
                   END AS poster_url,
                   visibility,
                   status,
                   scoring_mode,
                   dynamic_decay,
                   first_blood_bonus_percent,
                   second_blood_bonus_percent,
                   third_blood_bonus_percent,
                   registration_requires_approval,
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
    .bind(first_blood_bonus_percent)
    .bind(second_blood_bonus_percent)
    .bind(third_blood_bonus_percent)
    .bind(registration_requires_approval)
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
            "first_blood_bonus_percent": row.first_blood_bonus_percent,
            "second_blood_bonus_percent": row.second_blood_bonus_percent,
            "third_blood_bonus_percent": row.third_blood_bonus_percent,
            "registration_requires_approval": row.registration_requires_approval,
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
                CASE
                    WHEN poster_storage_path IS NULL OR poster_storage_path = '' THEN NULL
                    ELSE '/api/v1/contests/' || id::text || '/poster'
                END AS poster_url,
                visibility,
                status,
                scoring_mode,
                dynamic_decay,
                first_blood_bonus_percent,
                second_blood_bonus_percent,
                third_blood_bonus_percent,
                registration_requires_approval,
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
        Some(value) => {
            normalize_with_allowed(&value, CONTEST_SCORING_MODE_ALLOWED, "scoring_mode")?
        }
        None => existing.scoring_mode,
    };
    let dynamic_decay = req.dynamic_decay.unwrap_or(existing.dynamic_decay);
    if !(1..=100000).contains(&dynamic_decay) {
        return Err(AppError::BadRequest(
            "dynamic_decay must be between 1 and 100000".to_string(),
        ));
    }
    let first_blood_bonus_percent = validate_blood_bonus_percent(
        req.first_blood_bonus_percent
            .unwrap_or(existing.first_blood_bonus_percent),
        "first_blood_bonus_percent",
    )?;
    let second_blood_bonus_percent = validate_blood_bonus_percent(
        req.second_blood_bonus_percent
            .unwrap_or(existing.second_blood_bonus_percent),
        "second_blood_bonus_percent",
    )?;
    let third_blood_bonus_percent = validate_blood_bonus_percent(
        req.third_blood_bonus_percent
            .unwrap_or(existing.third_blood_bonus_percent),
        "third_blood_bonus_percent",
    )?;
    let registration_requires_approval = req
        .registration_requires_approval
        .unwrap_or(existing.registration_requires_approval);

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
             first_blood_bonus_percent = $9,
             second_blood_bonus_percent = $10,
             third_blood_bonus_percent = $11,
             registration_requires_approval = $12,
             start_at = $13,
             end_at = $14,
             freeze_at = $15,
             updated_at = NOW()
         WHERE id = $1
         RETURNING id,
                   title,
                   slug,
                   description,
                   CASE
                       WHEN poster_storage_path IS NULL OR poster_storage_path = '' THEN NULL
                       ELSE '/api/v1/contests/' || id::text || '/poster'
                   END AS poster_url,
                   visibility,
                   status,
                   scoring_mode,
                   dynamic_decay,
                   first_blood_bonus_percent,
                   second_blood_bonus_percent,
                   third_blood_bonus_percent,
                   registration_requires_approval,
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
    .bind(first_blood_bonus_percent)
    .bind(second_blood_bonus_percent)
    .bind(third_blood_bonus_percent)
    .bind(registration_requires_approval)
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
            "first_blood_bonus_percent": row.first_blood_bonus_percent,
            "second_blood_bonus_percent": row.second_blood_bonus_percent,
            "third_blood_bonus_percent": row.third_blood_bonus_percent,
            "registration_requires_approval": row.registration_requires_approval,
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
                   CASE
                       WHEN poster_storage_path IS NULL OR poster_storage_path = '' THEN NULL
                       ELSE '/api/v1/contests/' || id::text || '/poster'
                   END AS poster_url,
                   visibility,
                   status,
                   scoring_mode,
                   dynamic_decay,
                   first_blood_bonus_percent,
                   second_blood_bonus_percent,
                   third_blood_bonus_percent,
                   registration_requires_approval,
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

async fn delete_contest(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(contest_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    ensure_admin_or_judge(&current_user)?;
    ensure_contest_exists(state.as_ref(), contest_id).await?;

    let instance_cleanup =
        instances::destroy_instances_for_contest(state.as_ref(), contest_id).await?;
    if instance_cleanup.failed > 0 {
        return Err(AppError::Conflict(format!(
            "failed to destroy {} runtime instances, resolve runtime errors and retry",
            instance_cleanup.failed
        )));
    }

    let deleted = sqlx::query_as::<_, (Uuid, String, Option<String>)>(
        "DELETE FROM contests
         WHERE id = $1
         RETURNING id, title, poster_storage_path",
    )
    .bind(contest_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("contest not found".to_string()))?;

    if let Some(path) = deleted.2.as_deref() {
        let poster_path = PathBuf::from(path);
        if let Err(err) = fs::remove_file(&poster_path).await {
            if err.kind() != std::io::ErrorKind::NotFound {
                return Err(AppError::internal(err));
            }
        }
    }

    let poster_dir = contest_posters_dir(state.as_ref(), contest_id);
    if let Err(err) = fs::remove_dir_all(&poster_dir).await {
        if err.kind() != std::io::ErrorKind::NotFound {
            return Err(AppError::internal(err));
        }
    }

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.contest.delete",
        "contest",
        Some(deleted.0),
        json!({
            "title": deleted.1,
            "instance_cleanup": {
                "scanned": instance_cleanup.scanned,
                "destroyed": instance_cleanup.reaped,
                "failed": instance_cleanup.failed,
                "skipped": instance_cleanup.skipped
            }
        }),
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

async fn upload_contest_poster(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(contest_id): Path<Uuid>,
    Json(req): Json<UploadContestPosterRequest>,
) -> AppResult<Json<AdminContestItem>> {
    ensure_admin_or_judge(&current_user)?;
    ensure_contest_exists(state.as_ref(), contest_id).await?;

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
        return Err(AppError::BadRequest("poster content is empty".to_string()));
    }
    if decoded.len() > CONTEST_POSTER_MAX_BYTES {
        return Err(AppError::BadRequest(format!(
            "poster size must be <= {}MB",
            CONTEST_POSTER_MAX_BYTES / (1024 * 1024)
        )));
    }

    let content_type = req
        .content_type
        .as_deref()
        .and_then(normalize_optional_text)
        .map(str::to_lowercase)
        .or_else(|| infer_image_content_type_from_filename(&filename).map(str::to_string))
        .unwrap_or_else(|| "image/png".to_string());
    if !content_type.starts_with("image/") {
        return Err(AppError::BadRequest(
            "content_type must be image/*".to_string(),
        ));
    }

    let old_poster_path = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT poster_storage_path
         FROM contests
         WHERE id = $1
         LIMIT 1",
    )
    .bind(contest_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("contest not found".to_string()))?
    .0;

    let safe_filename = sanitize_filename(&filename);
    let poster_dir = contest_posters_dir(state.as_ref(), contest_id);
    fs::create_dir_all(&poster_dir)
        .await
        .map_err(AppError::internal)?;

    let stored_name = format!("{}-{}", Uuid::new_v4(), safe_filename);
    let stored_path = poster_dir.join(stored_name);
    fs::write(&stored_path, &decoded)
        .await
        .map_err(AppError::internal)?;

    let updated = sqlx::query_as::<_, AdminContestItem>(
        "UPDATE contests
         SET poster_storage_path = $2,
             poster_content_type = $3,
             updated_at = NOW()
         WHERE id = $1
         RETURNING id,
                   title,
                   slug,
                   description,
                   CASE
                       WHEN poster_storage_path IS NULL OR poster_storage_path = '' THEN NULL
                       ELSE '/api/v1/contests/' || id::text || '/poster'
                   END AS poster_url,
                   visibility,
                   status,
                   scoring_mode,
                   dynamic_decay,
                   first_blood_bonus_percent,
                   second_blood_bonus_percent,
                   third_blood_bonus_percent,
                   registration_requires_approval,
                   start_at,
                   end_at,
                   freeze_at,
                   created_at,
                   updated_at",
    )
    .bind(contest_id)
    .bind(stored_path.to_string_lossy().to_string())
    .bind(&content_type)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("contest not found".to_string()))?;

    if let Some(old_path) = old_poster_path {
        let old_path = PathBuf::from(old_path);
        if old_path != stored_path {
            if let Err(err) = fs::remove_file(&old_path).await {
                if err.kind() != std::io::ErrorKind::NotFound {
                    return Err(AppError::internal(err));
                }
            }
        }
    }

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.contest.poster.upload",
        "contest",
        Some(updated.id),
        json!({
            "filename": filename,
            "content_type": content_type,
            "size_bytes": decoded.len()
        }),
    )
    .await;

    Ok(Json(updated))
}

async fn delete_contest_poster(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(contest_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    ensure_admin_or_judge(&current_user)?;

    let existing = sqlx::query_as::<_, (Uuid, String, Option<String>)>(
        "SELECT id, title, poster_storage_path
         FROM contests
         WHERE id = $1
         LIMIT 1",
    )
    .bind(contest_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("contest not found".to_string()))?;

    sqlx::query(
        "UPDATE contests
         SET poster_storage_path = NULL,
             poster_content_type = NULL,
             updated_at = NOW()
         WHERE id = $1",
    )
    .bind(contest_id)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    if let Some(path) = existing.2.as_deref() {
        let poster_path = PathBuf::from(path);
        if let Err(err) = fs::remove_file(&poster_path).await {
            if err.kind() != std::io::ErrorKind::NotFound {
                return Err(AppError::internal(err));
            }
        }
    }

    let poster_dir = contest_posters_dir(state.as_ref(), contest_id);
    if let Err(err) = fs::remove_dir_all(&poster_dir).await {
        if err.kind() != std::io::ErrorKind::NotFound {
            return Err(AppError::internal(err));
        }
    }

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.contest.poster.delete",
        "contest",
        Some(existing.0),
        json!({
            "title": existing.1
        }),
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
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

    if title.is_none() && content.is_none() && req.is_published.is_none() && req.is_pinned.is_none()
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

async fn list_contest_registrations(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(contest_id): Path<Uuid>,
    Query(query): Query<AdminContestRegistrationsQuery>,
) -> AppResult<Json<Vec<AdminContestRegistrationItem>>> {
    ensure_admin_or_judge(&current_user)?;
    ensure_contest_exists(state.as_ref(), contest_id).await?;

    let status_filter = query
        .status
        .as_deref()
        .map(|value| {
            normalize_with_allowed(
                value,
                CONTEST_REGISTRATION_STATUS_ALLOWED,
                "registration_status",
            )
        })
        .transpose()?;
    let limit = query.limit.unwrap_or(200).clamp(1, 1000);

    let rows = sqlx::query_as::<_, AdminContestRegistrationItem>(
        "SELECT r.id,
                r.contest_id,
                c.title AS contest_title,
                r.team_id,
                t.name AS team_name,
                r.status,
                r.requested_by,
                req_u.username AS requested_by_username,
                r.requested_at,
                r.reviewed_by,
                rev_u.username AS reviewed_by_username,
                r.reviewed_at,
                r.review_note,
                r.created_at,
                r.updated_at
         FROM contest_registrations r
         JOIN contests c ON c.id = r.contest_id
         JOIN teams t ON t.id = r.team_id
         LEFT JOIN users req_u ON req_u.id = r.requested_by
         LEFT JOIN users rev_u ON rev_u.id = r.reviewed_by
         WHERE r.contest_id = $1
           AND ($2::text IS NULL OR r.status = $2)
         ORDER BY r.requested_at DESC, r.created_at DESC
         LIMIT $3",
    )
    .bind(contest_id)
    .bind(status_filter)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(rows))
}

async fn update_contest_registration(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path((contest_id, registration_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateContestRegistrationRequest>,
) -> AppResult<Json<AdminContestRegistrationItem>> {
    ensure_admin_or_judge(&current_user)?;
    ensure_contest_exists(state.as_ref(), contest_id).await?;

    let status = normalize_with_allowed(
        req.status.as_str(),
        CONTEST_REGISTRATION_STATUS_ALLOWED,
        "registration_status",
    )?;
    let review_note = req
        .review_note
        .as_deref()
        .and_then(normalize_optional_text)
        .unwrap_or("")
        .to_string();
    if review_note.chars().count() > 1000 {
        return Err(AppError::BadRequest(
            "review_note must be at most 1000 characters".to_string(),
        ));
    }

    let row = sqlx::query_as::<_, AdminContestRegistrationItem>(
        "WITH updated AS (
            UPDATE contest_registrations
            SET status = $3,
                reviewed_by = CASE WHEN $3 = 'pending' THEN NULL ELSE $4 END,
                reviewed_at = CASE WHEN $3 = 'pending' THEN NULL ELSE NOW() END,
                review_note = $5,
                updated_at = NOW()
            WHERE contest_id = $1
              AND id = $2
            RETURNING id,
                      contest_id,
                      team_id,
                      status,
                      requested_by,
                      requested_at,
                      reviewed_by,
                      reviewed_at,
                      review_note,
                      created_at,
                      updated_at
         )
         SELECT u.id,
                u.contest_id,
                c.title AS contest_title,
                u.team_id,
                t.name AS team_name,
                u.status,
                u.requested_by,
                req_u.username AS requested_by_username,
                u.requested_at,
                u.reviewed_by,
                rev_u.username AS reviewed_by_username,
                u.reviewed_at,
                u.review_note,
                u.created_at,
                u.updated_at
         FROM updated u
         JOIN contests c ON c.id = u.contest_id
         JOIN teams t ON t.id = u.team_id
         LEFT JOIN users req_u ON req_u.id = u.requested_by
         LEFT JOIN users rev_u ON rev_u.id = u.reviewed_by",
    )
    .bind(contest_id)
    .bind(registration_id)
    .bind(&status)
    .bind(current_user.user_id)
    .bind(&review_note)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest(
        "contest registration not found".to_string(),
    ))?;

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.contest.registration.update",
        "contest_registration",
        Some(row.id),
        json!({
            "contest_id": row.contest_id,
            "team_id": row.team_id,
            "status": row.status,
            "review_note": row.review_note
        }),
    )
    .await;

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

async fn get_instance_runtime_metrics(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(instance_id): Path<Uuid>,
) -> AppResult<Json<AdminInstanceRuntimeMetricsResponse>> {
    ensure_admin_or_judge(&current_user)?;

    let instance = load_admin_instance_item(state.as_ref(), instance_id).await?;
    let timeout_seconds = state.config.compose_command_timeout_seconds.clamp(5, 120);
    let (services, warnings) =
        collect_instance_runtime_metrics(instance.compose_project_name.as_str(), timeout_seconds)
            .await?;
    let summary = summarize_instance_runtime_metrics(&services);

    Ok(Json(AdminInstanceRuntimeMetricsResponse {
        generated_at: Utc::now(),
        instance,
        summary,
        services,
        warnings,
    }))
}

async fn run_expired_instance_reaper_now(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
) -> AppResult<Json<AdminInstanceReaperRunResponse>> {
    ensure_admin_or_judge(&current_user)?;
    let batch_size = state.config.instance_reaper_batch_size.clamp(1, 500);
    let summary = instances::run_expired_instance_reaper(state.as_ref(), batch_size).await?;

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.runtime.reaper.expired.run",
        "instance",
        None,
        json!({
            "batch_size": batch_size,
            "scanned": summary.scanned,
            "reaped": summary.reaped,
            "failed": summary.failed,
            "skipped": summary.skipped
        }),
    )
    .await;

    Ok(Json(AdminInstanceReaperRunResponse {
        generated_at: Utc::now(),
        mode: "expired".to_string(),
        heartbeat_stale_seconds: None,
        scanned: summary.scanned,
        reaped: summary.reaped,
        failed: summary.failed,
        skipped: summary.skipped,
    }))
}

async fn run_stale_instance_reaper_now(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
) -> AppResult<Json<AdminInstanceReaperRunResponse>> {
    ensure_admin_or_judge(&current_user)?;
    let batch_size = state.config.instance_stale_reaper_batch_size.clamp(1, 500);
    let heartbeat_stale_seconds = state
        .config
        .instance_heartbeat_stale_seconds
        .clamp(60, 86_400) as i64;
    let summary =
        instances::run_stale_instance_reaper(state.as_ref(), heartbeat_stale_seconds, batch_size)
            .await?;

    record_audit_log(
        state.as_ref(),
        &current_user,
        "admin.runtime.reaper.stale.run",
        "instance",
        None,
        json!({
            "batch_size": batch_size,
            "heartbeat_stale_seconds": heartbeat_stale_seconds,
            "scanned": summary.scanned,
            "reaped": summary.reaped,
            "failed": summary.failed,
            "skipped": summary.skipped
        }),
    )
    .await;

    Ok(Json(AdminInstanceReaperRunResponse {
        generated_at: Utc::now(),
        mode: "stale".to_string(),
        heartbeat_stale_seconds: Some(heartbeat_stale_seconds),
        scanned: summary.scanned,
        reaped: summary.reaped,
        failed: summary.failed,
        skipped: summary.skipped,
    }))
}

async fn load_admin_instance_item(
    state: &AppState,
    instance_id: Uuid,
) -> AppResult<AdminInstanceItem> {
    let row = sqlx::query_as::<_, AdminInstanceItem>(
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
         WHERE i.id = $1
         LIMIT 1",
    )
    .bind(instance_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("instance not found".to_string()))?;

    Ok(row)
}

async fn collect_instance_runtime_metrics(
    compose_project_name: &str,
    timeout_seconds: u64,
) -> AppResult<(Vec<AdminInstanceRuntimeMetricsService>, Vec<String>)> {
    let project_name = compose_project_name.trim();
    if project_name.is_empty() {
        return Err(AppError::BadRequest(
            "instance compose project name is empty".to_string(),
        ));
    }

    let list_args = vec![
        "ps".to_string(),
        "-a".to_string(),
        "--filter".to_string(),
        format!("label=com.docker.compose.project={project_name}"),
        "--format".to_string(),
        "{{.ID}}".to_string(),
    ];
    let list_output =
        run_docker_command_or_error(&list_args, timeout_seconds, "docker ps for runtime metrics")
            .await?;
    let container_ids = list_output
        .output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();

    if container_ids.is_empty() {
        return Ok((
            Vec::new(),
            vec![format!(
                "no docker containers found for compose project '{project_name}'"
            )],
        ));
    }

    let mut inspect_args = vec!["inspect".to_string()];
    inspect_args.extend(container_ids.iter().cloned());
    let inspect_output =
        run_docker_command_or_error(&inspect_args, timeout_seconds, "docker inspect").await?;
    let inspect_rows =
        serde_json::from_str::<Vec<Value>>(&inspect_output.output).map_err(|err| {
            AppError::BadRequest(format!("failed to parse docker inspect output: {err}"))
        })?;

    let mut stats_args = vec![
        "stats".to_string(),
        "--no-stream".to_string(),
        "--format".to_string(),
        "{{json .}}".to_string(),
    ];
    stats_args.extend(container_ids.iter().cloned());

    let stats_output =
        run_docker_command_or_error(&stats_args, timeout_seconds, "docker stats").await?;
    let (stats_rows, mut warnings) = parse_instance_stats_output(&stats_output.output);

    let mut services = Vec::new();
    for row in inspect_rows {
        let container_id = row
            .get("Id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_default();
        if container_id.is_empty() {
            warnings.push(
                "skip one container entry: missing container id in docker inspect".to_string(),
            );
            continue;
        }

        let container_name = row
            .get("Name")
            .and_then(Value::as_str)
            .map(normalize_container_name)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| container_id.chars().take(12).collect());

        let labels = row
            .get("Config")
            .and_then(|value| value.get("Labels"))
            .and_then(Value::as_object);
        let service_name = labels
            .and_then(|map| map.get("com.docker.compose.service"))
            .and_then(Value::as_str)
            .map(str::to_string);
        let image = row
            .get("Config")
            .and_then(|value| value.get("Image"))
            .and_then(Value::as_str)
            .map(str::to_string);

        let state = row
            .get("State")
            .and_then(|value| value.get("Status"))
            .and_then(Value::as_str)
            .map(str::to_string);
        let health_status = row
            .get("State")
            .and_then(|value| value.get("Health"))
            .and_then(|value| value.get("Status"))
            .and_then(Value::as_str)
            .map(str::to_string);
        let restart_count = row.get("RestartCount").and_then(Value::as_i64).or_else(|| {
            row.get("State")
                .and_then(|value| value.get("RestartCount"))
                .and_then(Value::as_i64)
        });
        let started_at = row
            .get("State")
            .and_then(|value| value.get("StartedAt"))
            .and_then(Value::as_str)
            .and_then(normalize_container_time);
        let finished_at = row
            .get("State")
            .and_then(|value| value.get("FinishedAt"))
            .and_then(Value::as_str)
            .and_then(normalize_container_time);
        let ip_addresses = row
            .get("NetworkSettings")
            .and_then(|value| value.get("Networks"))
            .and_then(Value::as_object)
            .map(|networks| {
                let mut ips = Vec::new();
                for network in networks.values() {
                    if let Some(ip) = network.get("IPAddress").and_then(Value::as_str) {
                        let normalized = ip.trim();
                        if !normalized.is_empty() {
                            ips.push(normalized.to_string());
                        }
                    }
                }
                ips
            })
            .unwrap_or_default();

        let stats =
            find_instance_stats(&stats_rows, &container_id, &container_name).unwrap_or_default();

        services.push(AdminInstanceRuntimeMetricsService {
            container_id,
            container_name,
            service_name,
            image,
            state,
            health_status,
            restart_count,
            started_at,
            finished_at,
            ip_addresses,
            cpu_percent: stats.cpu_percent,
            memory_usage_bytes: stats.memory_usage_bytes,
            memory_limit_bytes: stats.memory_limit_bytes,
            memory_percent: stats.memory_percent,
            net_rx_bytes: stats.net_rx_bytes,
            net_tx_bytes: stats.net_tx_bytes,
            block_read_bytes: stats.block_read_bytes,
            block_write_bytes: stats.block_write_bytes,
            pids: stats.pids,
        });
    }

    services.sort_by(|left, right| left.container_name.cmp(&right.container_name));
    Ok((services, warnings))
}

async fn run_docker_command_or_error(
    args: &[String],
    timeout_seconds: u64,
    action_name: &str,
) -> AppResult<ProcessExecutionOutput> {
    let args_ref = args.iter().map(String::as_str).collect::<Vec<_>>();
    let output = run_external_command("docker", &args_ref, None, None, timeout_seconds).await?;
    if output.success {
        return Ok(output);
    }

    Err(AppError::BadRequest(format!(
        "{action_name} failed: {}",
        compact_runtime_monitor_message(&output.output)
    )))
}

fn parse_instance_stats_output(raw: &str) -> InstanceStatsParseOutput {
    let mut rows = Vec::new();
    let mut warnings = Vec::new();

    for (index, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let parsed = match serde_json::from_str::<InstanceStatsLine>(trimmed) {
            Ok(value) => value,
            Err(err) => {
                warnings.push(format!("skip stats line {}: {}", index + 1, err));
                continue;
            }
        };

        let (memory_usage_bytes, memory_limit_bytes) =
            parse_usage_limit_pair(parsed.mem_usage.as_deref());
        let (net_rx_bytes, net_tx_bytes) = parse_io_pair(parsed.net_io.as_deref());
        let (block_read_bytes, block_write_bytes) = parse_io_pair(parsed.block_io.as_deref());
        let stats = ParsedInstanceStats {
            cpu_percent: parse_percent_value(parsed.cpu_perc.as_deref()),
            memory_usage_bytes,
            memory_limit_bytes,
            memory_percent: parse_percent_value(parsed.mem_perc.as_deref()),
            net_rx_bytes,
            net_tx_bytes,
            block_read_bytes,
            block_write_bytes,
            pids: parse_i64_value(parsed.pids.as_deref()),
        };

        rows.push((
            parsed.container.map(|value| value.trim().to_string()),
            parsed
                .name
                .as_deref()
                .map(normalize_container_name)
                .filter(|value| !value.is_empty()),
            stats,
        ));
    }

    (rows, warnings)
}

fn find_instance_stats(
    rows: &[InstanceStatsRow],
    container_id: &str,
    container_name: &str,
) -> Option<ParsedInstanceStats> {
    if container_id.is_empty() && container_name.is_empty() {
        return None;
    }

    for (stats_container_id, stats_name, stats) in rows {
        if let Some(stats_id) = stats_container_id {
            if !stats_id.is_empty()
                && (container_id.starts_with(stats_id) || stats_id.starts_with(container_id))
            {
                return Some(stats.clone());
            }
        }
        if let Some(name) = stats_name {
            if name.eq_ignore_ascii_case(container_name) {
                return Some(stats.clone());
            }
        }
    }

    None
}

fn summarize_instance_runtime_metrics(
    services: &[AdminInstanceRuntimeMetricsService],
) -> AdminInstanceRuntimeMetricsSummary {
    let mut running_services = 0_i64;
    let mut unhealthy_services = 0_i64;
    let mut restarting_services = 0_i64;
    let mut cpu_percent_total = 0_f64;
    let mut memory_usage_bytes_total = 0_i64;
    let mut memory_limit_bytes_total = 0_i64;

    for service in services {
        if service.state.as_deref() == Some("running") {
            running_services += 1;
        }
        if service.health_status.as_deref() == Some("unhealthy") {
            unhealthy_services += 1;
        }
        if service.state.as_deref() == Some("restarting") {
            restarting_services += 1;
        }
        if let Some(cpu_percent) = service.cpu_percent {
            cpu_percent_total += cpu_percent;
        }
        if let Some(memory_usage_bytes) = service.memory_usage_bytes {
            memory_usage_bytes_total += memory_usage_bytes;
        }
        if let Some(memory_limit_bytes) = service.memory_limit_bytes {
            memory_limit_bytes_total += memory_limit_bytes;
        }
    }

    AdminInstanceRuntimeMetricsSummary {
        services_total: services.len() as i64,
        running_services,
        unhealthy_services,
        restarting_services,
        cpu_percent_total: (cpu_percent_total * 100.0).round() / 100.0,
        memory_usage_bytes_total,
        memory_limit_bytes_total,
    }
}

fn parse_percent_value(raw: Option<&str>) -> Option<f64> {
    let value = raw?.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("n/a") {
        return None;
    }
    let normalized = value.trim_end_matches('%').trim();
    normalized.parse::<f64>().ok()
}

fn parse_i64_value(raw: Option<&str>) -> Option<i64> {
    let value = raw?.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("n/a") {
        return None;
    }
    value.parse::<i64>().ok()
}

fn parse_usage_limit_pair(raw: Option<&str>) -> (Option<i64>, Option<i64>) {
    let Some(value) = raw else {
        return (None, None);
    };
    let Some((left, right)) = value.split_once('/') else {
        return (parse_size_to_bytes(value), None);
    };
    (parse_size_to_bytes(left), parse_size_to_bytes(right))
}

fn parse_io_pair(raw: Option<&str>) -> (Option<i64>, Option<i64>) {
    let Some(value) = raw else {
        return (None, None);
    };
    let Some((left, right)) = value.split_once('/') else {
        return (parse_size_to_bytes(value), None);
    };
    (parse_size_to_bytes(left), parse_size_to_bytes(right))
}

fn parse_size_to_bytes(raw: &str) -> Option<i64> {
    let value = raw.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("n/a") {
        return None;
    }

    let mut number_part = String::new();
    let mut unit_part = String::new();
    for ch in value.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            if unit_part.is_empty() {
                number_part.push(ch);
            }
        } else if !ch.is_ascii_whitespace() {
            unit_part.push(ch);
        }
    }

    if number_part.is_empty() {
        return None;
    }

    let number = number_part.parse::<f64>().ok()?;
    let factor = match unit_part.to_ascii_lowercase().as_str() {
        "" | "b" => 1_f64,
        "k" | "kb" => 1_000_f64,
        "kib" => 1_024_f64,
        "m" | "mb" => 1_000_000_f64,
        "mib" => 1_048_576_f64,
        "g" | "gb" => 1_000_000_000_f64,
        "gib" => 1_073_741_824_f64,
        "t" | "tb" => 1_000_000_000_000_f64,
        "tib" => 1_099_511_627_776_f64,
        _ => return None,
    };

    Some((number * factor).round() as i64)
}

fn normalize_container_name(raw: &str) -> String {
    raw.trim().trim_start_matches('/').to_string()
}

fn normalize_container_time(raw: &str) -> Option<String> {
    let value = raw.trim();
    if value.is_empty() || value.starts_with("0001-01-01") {
        return None;
    }
    Some(value.to_string())
}

fn compact_runtime_monitor_message(raw: &str) -> String {
    let source = raw.trim();
    if source.is_empty() {
        return "no diagnostic output".to_string();
    }

    let mut compact = source.replace(['\n', '\r'], " ");
    if compact.chars().count() > 240 {
        compact = compact.chars().take(240).collect::<String>() + "...";
    }
    compact
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

async fn collect_runtime_alert_candidates(
    state: &AppState,
) -> AppResult<Vec<RuntimeAlertCandidate>> {
    let mut candidates = Vec::new();
    let heartbeat_stale_seconds = state
        .config
        .instance_heartbeat_stale_seconds
        .clamp(60, 86_400);
    let heartbeat_stale_minutes = heartbeat_stale_seconds / 60;

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
           AND i.last_heartbeat_at <= NOW() - ($1::bigint * INTERVAL '1 second')
         ORDER BY i.last_heartbeat_at ASC",
    )
    .bind(heartbeat_stale_seconds as i64)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    for row in stale_heartbeat_rows {
        if let Some(last_heartbeat_at) = row.last_heartbeat_at {
            let message = format!(
                "实例 {} / {} / {} 心跳停留在 {}，超过 {} 分钟阈值，可能存在异常",
                row.contest_title,
                row.challenge_title,
                row.team_name,
                last_heartbeat_at,
                heartbeat_stale_minutes
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

async fn load_runtime_alert_item(
    state: &AppState,
    alert_id: Uuid,
) -> AppResult<AdminRuntimeAlertItem> {
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

fn normalize_optional_setting(
    value: Option<&str>,
    field: &str,
    min_len: usize,
    max_len: usize,
) -> AppResult<Option<String>> {
    let Some(raw) = value else {
        return Ok(None);
    };

    let trimmed = raw.trim();
    let len = trimmed.chars().count();

    if len < min_len {
        return Err(AppError::BadRequest(format!(
            "{} length must be >= {}",
            field, min_len
        )));
    }

    if len > max_len {
        return Err(AppError::BadRequest(format!(
            "{} length must be <= {}",
            field, max_len
        )));
    }

    Ok(Some(trimmed.to_string()))
}

fn normalize_optional_text(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn format_bytes_for_message(size_bytes: i64) -> String {
    if size_bytes <= 0 {
        return "0 B".to_string();
    }
    if size_bytes < 1024 {
        return format!("{size_bytes} B");
    }
    if size_bytes < 1024 * 1024 {
        return format!("{:.1} KB", size_bytes as f64 / 1024.0);
    }
    if size_bytes < 1024 * 1024 * 1024 {
        return format!("{:.1} MB", size_bytes as f64 / (1024.0 * 1024.0));
    }
    format!("{:.2} GB", size_bytes as f64 / (1024.0 * 1024.0 * 1024.0))
}

async fn load_challenge_attachment_limit_bytes(state: &AppState) -> AppResult<i64> {
    let configured = sqlx::query_scalar::<_, i64>(
        "SELECT challenge_attachment_max_bytes
         FROM site_settings
         WHERE id = TRUE
         LIMIT 1",
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?;

    let value = configured.unwrap_or(DEFAULT_CHALLENGE_ATTACHMENT_MAX_BYTES);
    Ok(value.clamp(
        MIN_CHALLENGE_ATTACHMENT_MAX_BYTES,
        MAX_CHALLENGE_ATTACHMENT_MAX_BYTES,
    ))
}

fn validate_compose_runtime_configuration(
    challenge_type: &str,
    compose_template: Option<&str>,
    metadata: &Value,
) -> AppResult<()> {
    let requires_runtime = challenge_type == "dynamic" || challenge_type == "internal";
    let runtime_options = parse_runtime_metadata_options(metadata).map_err(AppError::BadRequest)?;

    if runtime_options.mode == RuntimeMode::SingleImage {
        if !requires_runtime {
            return Err(AppError::BadRequest(
                "metadata.runtime.mode=single_image requires challenge_type=dynamic/internal"
                    .to_string(),
            ));
        }

        let single = runtime_options.single_image.ok_or(AppError::BadRequest(
            "metadata.runtime single_image config is missing".to_string(),
        ))?;
        let generated =
            build_single_image_compose_template(single.image.as_str(), single.internal_port);
        return validate_compose_template_schema(&generated, metadata)
            .map_err(AppError::BadRequest);
    }

    match compose_template.and_then(normalize_optional_text) {
        Some(template) => {
            validate_compose_template_schema(template, metadata).map_err(AppError::BadRequest)
        }
        None if requires_runtime => Err(AppError::BadRequest(
            "challenge runtime template is required for dynamic/internal challenge".to_string(),
        )),
        None => Ok(()),
    }
}

fn default_challenge_status() -> String {
    "draft".to_string()
}

fn normalize_challenge_category_slug(value: &str) -> AppResult<String> {
    let slug = trim_required(value, "slug")?.to_lowercase();
    if slug.chars().count() > 32 {
        return Err(AppError::BadRequest(
            "slug must be at most 32 characters".to_string(),
        ));
    }
    if !slug
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
    {
        return Err(AppError::BadRequest(
            "slug must contain only [a-z0-9_-]".to_string(),
        ));
    }
    Ok(slug)
}

fn normalize_challenge_category_display_name(value: &str) -> AppResult<String> {
    let display_name = trim_required(value, "display_name")?;
    if display_name.chars().count() > 64 {
        return Err(AppError::BadRequest(
            "display_name must be at most 64 characters".to_string(),
        ));
    }
    Ok(display_name.to_string())
}

async fn ensure_challenge_category_exists(state: &AppState, category: &str) -> AppResult<()> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1
            FROM challenge_categories
            WHERE LOWER(slug) = LOWER($1)
         )",
    )
    .bind(category)
    .fetch_one(&state.db)
    .await
    .map_err(AppError::internal)?;

    if exists {
        Ok(())
    } else {
        Err(AppError::BadRequest(
            "challenge category not found".to_string(),
        ))
    }
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

fn normalize_hints(hints: Vec<String>) -> AppResult<Vec<String>> {
    if hints.len() > 20 {
        return Err(AppError::BadRequest(
            "hints must be at most 20 items".to_string(),
        ));
    }

    let mut out: Vec<String> = Vec::new();
    for hint in hints {
        let normalized = hint.trim().to_string();
        if normalized.is_empty() {
            continue;
        }
        if normalized.chars().count() > 500 {
            return Err(AppError::BadRequest(
                "each hint must be at most 500 characters".to_string(),
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
        hints: row.hints.clone(),
        writeup_visibility: row.writeup_visibility.clone(),
        writeup_content: row.writeup_content.clone(),
    })
    .unwrap_or_else(|_| json!({}))
}

async fn load_challenge_runtime_config(
    state: &AppState,
    challenge_id: Uuid,
) -> AppResult<ChallengeRuntimeConfigRow> {
    sqlx::query_as::<_, ChallengeRuntimeConfigRow>(
        "SELECT challenge_type,
                compose_template,
                metadata
         FROM challenges
         WHERE id = $1
         LIMIT 1",
    )
    .bind(challenge_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("challenge not found".to_string()))
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

fn contest_posters_dir(state: &AppState, contest_id: Uuid) -> PathBuf {
    PathBuf::from(&state.config.instance_runtime_root)
        .join("_contest_posters")
        .join(contest_id.to_string())
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

fn infer_image_content_type_from_filename(filename: &str) -> Option<&'static str> {
    let lower = filename.trim().to_lowercase();
    if lower.ends_with(".png") {
        return Some("image/png");
    }
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        return Some("image/jpeg");
    }
    if lower.ends_with(".webp") {
        return Some("image/webp");
    }
    if lower.ends_with(".gif") {
        return Some("image/gif");
    }
    if lower.ends_with(".svg") {
        return Some("image/svg+xml");
    }

    None
}

fn validate_container_image_reference(raw: &str) -> AppResult<String> {
    let image = raw.trim();
    if image.is_empty() {
        return Err(AppError::BadRequest("image is required".to_string()));
    }
    if image.chars().count() > 255 {
        return Err(AppError::BadRequest(
            "image must be at most 255 characters".to_string(),
        ));
    }
    if image.chars().any(char::is_whitespace) {
        return Err(AppError::BadRequest(
            "image must not contain whitespace".to_string(),
        ));
    }
    if image.contains('\"') || image.contains('\'') {
        return Err(AppError::BadRequest(
            "image contains unsupported quote characters".to_string(),
        ));
    }

    Ok(image.to_string())
}

async fn run_compose_compatible_external_command(
    args: &[String],
    stdin_input: Option<&str>,
    workdir: Option<&std::path::Path>,
    timeout_seconds: u64,
) -> AppResult<ProcessExecutionOutput> {
    let mut docker_args: Vec<String> = Vec::with_capacity(args.len() + 1);
    docker_args.push("compose".to_string());
    docker_args.extend(args.iter().cloned());
    let docker_args_ref = docker_args.iter().map(String::as_str).collect::<Vec<_>>();

    let mut primary_output: Option<ProcessExecutionOutput> = None;
    match run_external_command(
        "docker",
        &docker_args_ref,
        stdin_input,
        workdir,
        timeout_seconds,
    )
    .await
    {
        Ok(output) => {
            if output.success || !should_fallback_to_legacy_compose_output(&output.output) {
                return Ok(output);
            }
            primary_output = Some(output);
        }
        Err(AppError::BadRequest(message)) if message == "docker command not found" => {}
        Err(err) => return Err(err),
    }

    let legacy_args_ref = args.iter().map(String::as_str).collect::<Vec<_>>();
    match run_external_command(
        "docker-compose",
        &legacy_args_ref,
        stdin_input,
        workdir,
        timeout_seconds,
    )
    .await
    {
        Ok(mut legacy_output) => {
            let prefix = "[executor=docker-compose]\n";
            legacy_output.output = if legacy_output.output.is_empty() {
                prefix.trim_end().to_string()
            } else {
                format!("{prefix}{}", legacy_output.output)
            };
            Ok(legacy_output)
        }
        Err(AppError::BadRequest(message)) if message == "docker-compose command not found" => {
            if let Some(output) = primary_output {
                Ok(output)
            } else {
                Err(AppError::BadRequest(
                    "docker compose command is unavailable (tried 'docker compose' and 'docker-compose')".to_string(),
                ))
            }
        }
        Err(err) => Err(err),
    }
}

#[derive(Debug)]
struct CommandLogLine {
    stream: &'static str,
    line: String,
}

async fn run_compose_compatible_external_command_stream<F>(
    args: &[String],
    stdin_input: Option<&str>,
    workdir: Option<&std::path::Path>,
    timeout_seconds: u64,
    mut on_log_line: F,
) -> AppResult<ProcessExecutionOutput>
where
    F: FnMut(&str, &str),
{
    let mut docker_args: Vec<String> = Vec::with_capacity(args.len() + 1);
    docker_args.push("compose".to_string());
    docker_args.extend(args.iter().cloned());
    let docker_args_ref = docker_args.iter().map(String::as_str).collect::<Vec<_>>();

    let mut primary_output: Option<ProcessExecutionOutput> = None;
    match run_external_command_stream(
        "docker",
        &docker_args_ref,
        stdin_input,
        workdir,
        timeout_seconds,
        |stream_name, line| on_log_line(stream_name, line),
    )
    .await
    {
        Ok(output) => {
            if output.success || !should_fallback_to_legacy_compose_output(&output.output) {
                return Ok(output);
            }
            primary_output = Some(output);
        }
        Err(AppError::BadRequest(message)) if message == "docker command not found" => {}
        Err(err) => return Err(err),
    }

    on_log_line("stderr", "[executor=fallback] switching to docker-compose");
    let legacy_args_ref = args.iter().map(String::as_str).collect::<Vec<_>>();
    match run_external_command_stream(
        "docker-compose",
        &legacy_args_ref,
        stdin_input,
        workdir,
        timeout_seconds,
        |stream_name, line| on_log_line(stream_name, line),
    )
    .await
    {
        Ok(mut legacy_output) => {
            let prefix = "[executor=docker-compose]\n";
            legacy_output.output = if legacy_output.output.is_empty() {
                prefix.trim_end().to_string()
            } else {
                format!("{prefix}{}", legacy_output.output)
            };
            Ok(legacy_output)
        }
        Err(AppError::BadRequest(message)) if message == "docker-compose command not found" => {
            if let Some(output) = primary_output {
                Ok(output)
            } else {
                Err(AppError::BadRequest(
                    "docker compose command is unavailable (tried 'docker compose' and 'docker-compose')".to_string(),
                ))
            }
        }
        Err(err) => Err(err),
    }
}

fn should_fallback_to_legacy_compose_output(raw: &str) -> bool {
    let lowered = raw.to_ascii_lowercase();
    lowered.contains("is not a docker command")
        || lowered.contains("unknown command \"compose\"")
        || lowered.contains("docker: 'compose' is not")
        || (lowered.contains("unknown shorthand flag")
            && (lowered.contains("in -f")
                || lowered.contains("in -p")
                || lowered.contains("see 'docker --help'")))
        || (lowered.contains("client version") && lowered.contains("minimum supported api version"))
}

async fn run_external_command_stream<F>(
    program: &str,
    args: &[&str],
    stdin_input: Option<&str>,
    workdir: Option<&std::path::Path>,
    timeout_seconds: u64,
    mut on_log_line: F,
) -> AppResult<ProcessExecutionOutput>
where
    F: FnMut(&str, &str),
{
    let mut command = Command::new(program);
    command.args(args);
    command.kill_on_drop(true);
    command.env_remove("DOCKER_API_VERSION");
    if let Some(dir) = workdir {
        command.current_dir(dir);
    }

    if stdin_input.is_some() {
        command.stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::null());
    }
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let start = Instant::now();
    let mut child = command.spawn().map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            AppError::BadRequest(format!("{program} command not found"))
        } else {
            AppError::internal(err)
        }
    })?;

    if let Some(input) = stdin_input {
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(input.as_bytes())
                .await
                .map_err(AppError::internal)?;
            stdin.shutdown().await.map_err(AppError::internal)?;
        }
    }

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::internal(anyhow::anyhow!("failed to capture child stdout")))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| AppError::internal(anyhow::anyhow!("failed to capture child stderr")))?;

    let (log_sender, mut log_receiver) = mpsc::unbounded_channel::<CommandLogLine>();
    let stdout_task = tokio::spawn(pump_command_output_lines(
        stdout,
        "stdout",
        log_sender.clone(),
    ));
    let stderr_task = tokio::spawn(pump_command_output_lines(stderr, "stderr", log_sender));

    let wait_deadline = tokio::time::sleep(TokioDuration::from_secs(timeout_seconds));
    tokio::pin!(wait_deadline);

    let mut exit_code = None;
    let mut merged: Vec<u8> = Vec::new();

    loop {
        tokio::select! {
            maybe_log = log_receiver.recv() => {
                match maybe_log {
                    Some(item) => {
                        append_command_output_line(&mut merged, item.line.as_str());
                        on_log_line(item.stream, item.line.as_str());
                    }
                    None => {
                        if exit_code.is_some() {
                            break;
                        }
                    }
                }
            }
            wait_result = child.wait(), if exit_code.is_none() => {
                let status = wait_result.map_err(AppError::internal)?;
                exit_code = status.code();
            }
            _ = &mut wait_deadline, if exit_code.is_none() => {
                return Ok(ProcessExecutionOutput {
                    success: false,
                    exit_code: None,
                    duration_ms: start.elapsed().as_millis() as i64,
                    output: format!("command timed out after {timeout_seconds} seconds"),
                    truncated: false,
                });
            }
        }
    }

    match stdout_task.await {
        Ok(Ok(())) => {}
        Ok(Err(err)) => {
            warn!(error = %err, "failed to read child stdout stream");
        }
        Err(err) => {
            warn!(error = %err, "failed to join child stdout reader task");
        }
    }
    match stderr_task.await {
        Ok(Ok(())) => {}
        Ok(Err(err)) => {
            warn!(error = %err, "failed to read child stderr stream");
        }
        Err(err) => {
            warn!(error = %err, "failed to join child stderr reader task");
        }
    }

    let (text, truncated) = truncate_log_bytes(merged);
    Ok(ProcessExecutionOutput {
        success: exit_code.unwrap_or(1) == 0,
        exit_code,
        duration_ms: start.elapsed().as_millis() as i64,
        output: text,
        truncated,
    })
}

async fn run_external_command(
    program: &str,
    args: &[&str],
    stdin_input: Option<&str>,
    workdir: Option<&std::path::Path>,
    timeout_seconds: u64,
) -> AppResult<ProcessExecutionOutput> {
    let mut command = Command::new(program);
    command.args(args);
    command.kill_on_drop(true);
    command.env_remove("DOCKER_API_VERSION");
    if let Some(dir) = workdir {
        command.current_dir(dir);
    }

    if stdin_input.is_some() {
        command.stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::null());
    }
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let start = Instant::now();
    let mut child = command.spawn().map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            AppError::BadRequest(format!("{program} command not found"))
        } else {
            AppError::internal(err)
        }
    })?;

    if let Some(input) = stdin_input {
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(input.as_bytes())
                .await
                .map_err(AppError::internal)?;
            stdin.shutdown().await.map_err(AppError::internal)?;
        }
    }

    let output = match timeout(
        TokioDuration::from_secs(timeout_seconds),
        child.wait_with_output(),
    )
    .await
    {
        Ok(output) => output.map_err(AppError::internal)?,
        Err(_) => {
            return Ok(ProcessExecutionOutput {
                success: false,
                exit_code: None,
                duration_ms: start.elapsed().as_millis() as i64,
                output: format!("command timed out after {timeout_seconds} seconds"),
                truncated: false,
            })
        }
    };

    let mut merged = output.stdout;
    if !output.stderr.is_empty() {
        if !merged.is_empty() {
            merged.extend_from_slice(b"\n");
        }
        merged.extend_from_slice(&output.stderr);
    }

    let (text, truncated) = truncate_log_bytes(merged);

    Ok(ProcessExecutionOutput {
        success: output.status.success(),
        exit_code: output.status.code(),
        duration_ms: start.elapsed().as_millis() as i64,
        output: text,
        truncated,
    })
}

fn append_command_output_line(target: &mut Vec<u8>, line: &str) {
    if !target.is_empty() {
        target.extend_from_slice(b"\n");
    }
    target.extend_from_slice(line.as_bytes());
}

async fn pump_command_output_lines<R>(
    reader: R,
    stream: &'static str,
    sender: mpsc::UnboundedSender<CommandLogLine>,
) -> std::io::Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut lines = BufReader::new(reader).lines();
    while let Some(line) = lines.next_line().await? {
        let _ = sender.send(CommandLogLine { stream, line });
    }
    Ok(())
}

fn truncate_log_bytes(bytes: Vec<u8>) -> (String, bool) {
    if bytes.len() <= IMAGE_TEST_LOG_MAX_BYTES {
        return (String::from_utf8_lossy(&bytes).to_string(), false);
    }

    let mut clipped = bytes[..IMAGE_TEST_LOG_MAX_BYTES].to_vec();
    while std::str::from_utf8(&clipped).is_err() && !clipped.is_empty() {
        clipped.pop();
    }
    let mut text = String::from_utf8_lossy(&clipped).to_string();
    text.push_str("\n... [log truncated]");

    (text, true)
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

fn validate_blood_bonus_percent(value: i32, field: &str) -> AppResult<i32> {
    if !(0..=500).contains(&value) {
        return Err(AppError::BadRequest(format!(
            "{} must be between 0 and 500",
            field
        )));
    }
    Ok(value)
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
