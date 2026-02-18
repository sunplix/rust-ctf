use std::sync::Arc;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, SaltString},
    Argon2, PasswordVerifier,
};
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, patch, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sqlx::FromRow;
use tracing::warn;
use uuid::Uuid;

use crate::{
    auth::{self, AuthenticatedUser, RefreshSession, TokenBundle},
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Debug, Deserialize)]
struct RegisterRequest {
    username: String,
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    identifier: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct UpdateProfileRequest {
    username: Option<String>,
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

#[derive(Debug, Deserialize)]
struct LoginHistoryQuery {
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct AuthResponse {
    access_token: String,
    refresh_token: String,
    token_type: &'static str,
    access_expires_in_seconds: i64,
    refresh_expires_in_seconds: i64,
    user: AuthUser,
}

#[derive(Debug, Serialize, FromRow)]
struct AuthUser {
    id: Uuid,
    username: String,
    email: String,
    role: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
struct LoginHistoryItem {
    id: i64,
    action: String,
    detail: Value,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct UserAuthRow {
    id: Uuid,
    username: String,
    email: String,
    password_hash: String,
    role: String,
    status: String,
    created_at: DateTime<Utc>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
        .route("/auth/me", get(me))
        .route("/auth/profile", patch(update_profile))
        .route("/auth/change-password", post(change_password))
        .route("/auth/account", delete(delete_account))
        .route("/auth/login-history", get(login_history))
}

async fn register(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<RegisterRequest>,
) -> AppResult<(StatusCode, Json<AuthResponse>)> {
    let username = req.username.trim().to_string();
    let email = req.email.trim().to_lowercase();

    if !is_valid_username(&username) {
        return Err(AppError::BadRequest(
            "username must be 3-32 chars and contain only letters, numbers, _ or -".to_string(),
        ));
    }

    if !is_valid_email(&email) {
        return Err(AppError::BadRequest("invalid email format".to_string()));
    }

    if req.password.len() < 8 {
        return Err(AppError::BadRequest(
            "password must be at least 8 characters".to_string(),
        ));
    }

    let password_hash = hash_password(&req.password)?;

    let user = sqlx::query_as::<_, UserAuthRow>(
        "INSERT INTO users (username, email, password_hash, role, status)
         VALUES ($1, $2, $3, 'player', 'active')
         RETURNING id, username, email, password_hash, role, status, created_at",
    )
    .bind(&username)
    .bind(&email)
    .bind(password_hash)
    .fetch_one(&state.db)
    .await
    .map_err(|err| {
        if is_unique_violation(&err) {
            AppError::Conflict("username or email already exists".to_string())
        } else {
            AppError::internal(err)
        }
    })?;

    let tokens = auth::issue_new_session_tokens(state.as_ref(), user.id, &user.role).await?;

    record_auth_audit_log(
        state.as_ref(),
        user.id,
        &user.role,
        "auth.register",
        json!({
            "username": username,
            "email": email,
            "request": request_meta(&headers)
        }),
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(build_auth_response(tokens, to_auth_user(&user))),
    ))
}

async fn login(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    let identifier = req.identifier.trim();
    if identifier.is_empty() || req.password.is_empty() {
        return Err(AppError::BadRequest(
            "identifier and password are required".to_string(),
        ));
    }

    let user = sqlx::query_as::<_, UserAuthRow>(
        "SELECT id, username, email, password_hash, role, status, created_at
         FROM users
         WHERE LOWER(username) = LOWER($1) OR LOWER(email) = LOWER($1)
         LIMIT 1",
    )
    .bind(identifier)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::Unauthorized)?;

    if user.status != "active" {
        return Err(AppError::Forbidden);
    }

    verify_password(&req.password, &user.password_hash)?;

    let tokens = auth::issue_new_session_tokens(state.as_ref(), user.id, &user.role).await?;

    record_auth_audit_log(
        state.as_ref(),
        user.id,
        &user.role,
        "auth.login",
        json!({
            "identifier": identifier,
            "request": request_meta(&headers)
        }),
    )
    .await;

    Ok(Json(build_auth_response(tokens, to_auth_user(&user))))
}

async fn refresh(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<RefreshRequest>,
) -> AppResult<Json<AuthResponse>> {
    let raw_refresh_token = req.refresh_token.trim();
    if raw_refresh_token.is_empty() {
        return Err(AppError::BadRequest(
            "refresh_token is required".to_string(),
        ));
    }

    let RefreshSession {
        user_id,
        session_id,
        refresh_jti,
    } = auth::decode_refresh_session(raw_refresh_token, &state.config.jwt_secret)?;

    let user = fetch_active_user_with_secret(&state, user_id).await?;
    let tokens =
        auth::rotate_session_tokens(state.as_ref(), user.id, &user.role, session_id, refresh_jti)
            .await?;

    record_auth_audit_log(
        state.as_ref(),
        user.id,
        &user.role,
        "auth.refresh",
        json!({
            "session_id": session_id,
            "request": request_meta(&headers)
        }),
    )
    .await;

    Ok(Json(build_auth_response(tokens, to_auth_user(&user))))
}

async fn me(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
) -> AppResult<Json<AuthUser>> {
    let _ = (&current_user.role, current_user.session_id);

    let user = sqlx::query_as::<_, AuthUser>(
        "SELECT id, username, email, role, created_at
         FROM users
         WHERE id = $1 AND status = 'active'
         LIMIT 1",
    )
    .bind(current_user.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::Unauthorized)?;

    Ok(Json(user))
}

async fn update_profile(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    current_user: AuthenticatedUser,
    Json(req): Json<UpdateProfileRequest>,
) -> AppResult<Json<AuthUser>> {
    let username = req
        .username
        .as_deref()
        .map(str::trim)
        .map(|value| {
            if value.is_empty() {
                return Err(AppError::BadRequest("username cannot be empty".to_string()));
            }
            if !is_valid_username(value) {
                return Err(AppError::BadRequest(
                    "username must be 3-32 chars and contain only letters, numbers, _ or -"
                        .to_string(),
                ));
            }
            Ok(value.to_string())
        })
        .transpose()?;

    let email = req
        .email
        .as_deref()
        .map(str::trim)
        .map(|value| {
            if value.is_empty() {
                return Err(AppError::BadRequest("email cannot be empty".to_string()));
            }
            let lowered = value.to_lowercase();
            if !is_valid_email(&lowered) {
                return Err(AppError::BadRequest("invalid email format".to_string()));
            }
            Ok(lowered)
        })
        .transpose()?;

    if username.is_none() && email.is_none() {
        return Err(AppError::BadRequest(
            "at least one of username or email is required".to_string(),
        ));
    }

    let updated = sqlx::query_as::<_, AuthUser>(
        "UPDATE users
         SET username = COALESCE($2, username),
             email = COALESCE($3, email),
             updated_at = NOW()
         WHERE id = $1 AND status = 'active'
         RETURNING id, username, email, role, created_at",
    )
    .bind(current_user.user_id)
    .bind(&username)
    .bind(&email)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| {
        if is_unique_violation(&err) {
            AppError::Conflict("username or email already exists".to_string())
        } else {
            AppError::internal(err)
        }
    })?
    .ok_or(AppError::Unauthorized)?;

    record_auth_audit_log(
        state.as_ref(),
        current_user.user_id,
        &updated.role,
        "auth.profile.update",
        json!({
            "updated_fields": {
                "username": username,
                "email": email
            },
            "request": request_meta(&headers)
        }),
    )
    .await;

    Ok(Json(updated))
}

async fn change_password(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    current_user: AuthenticatedUser,
    Json(req): Json<ChangePasswordRequest>,
) -> AppResult<Json<AuthResponse>> {
    if req.current_password.is_empty() || req.new_password.is_empty() {
        return Err(AppError::BadRequest(
            "current_password and new_password are required".to_string(),
        ));
    }

    if req.new_password.len() < 8 {
        return Err(AppError::BadRequest(
            "new_password must be at least 8 characters".to_string(),
        ));
    }

    if req.current_password == req.new_password {
        return Err(AppError::BadRequest(
            "new_password must be different from current_password".to_string(),
        ));
    }

    let user = fetch_active_user_with_secret(state.as_ref(), current_user.user_id).await?;
    verify_password(&req.current_password, &user.password_hash)?;

    let new_password_hash = hash_password(&req.new_password)?;

    sqlx::query(
        "UPDATE users
         SET password_hash = $2,
             updated_at = NOW()
         WHERE id = $1",
    )
    .bind(user.id)
    .bind(new_password_hash)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    auth::revoke_all_user_sessions(state.as_ref(), user.id).await?;

    let tokens = auth::issue_new_session_tokens(state.as_ref(), user.id, &user.role).await?;

    record_auth_audit_log(
        state.as_ref(),
        user.id,
        &user.role,
        "auth.password.change",
        json!({
            "session_rotated": true,
            "request": request_meta(&headers)
        }),
    )
    .await;

    Ok(Json(build_auth_response(tokens, to_auth_user(&user))))
}

async fn login_history(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Query(query): Query<LoginHistoryQuery>,
) -> AppResult<Json<Vec<LoginHistoryItem>>> {
    let limit = query.limit.unwrap_or(30).clamp(1, 200);

    let rows = sqlx::query_as::<_, LoginHistoryItem>(
        "SELECT id, action, detail, created_at
         FROM audit_logs
         WHERE actor_user_id = $1
           AND action IN ('auth.register', 'auth.login', 'auth.refresh', 'auth.password.change')
         ORDER BY created_at DESC
         LIMIT $2",
    )
    .bind(current_user.user_id)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(rows))
}

async fn delete_account(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    current_user: AuthenticatedUser,
) -> AppResult<StatusCode> {
    let user = fetch_active_user_with_secret(state.as_ref(), current_user.user_id).await?;

    if user.role == "admin" {
        let remaining_active_admins = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(1)
             FROM users
             WHERE role = 'admin'
               AND status = 'active'
               AND id <> $1",
        )
        .bind(user.id)
        .fetch_one(&state.db)
        .await
        .map_err(AppError::internal)?;
        if remaining_active_admins <= 0 {
            return Err(AppError::Conflict(
                "cannot delete the last active admin account".to_string(),
            ));
        }
    }

    let is_team_captain = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1
            FROM teams
            WHERE captain_user_id = $1
         )",
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await
    .map_err(AppError::internal)?;
    if is_team_captain {
        return Err(AppError::Conflict(
            "captain account cannot be deleted, transfer captain or disband team first".to_string(),
        ));
    }

    sqlx::query("DELETE FROM team_members WHERE user_id = $1")
        .bind(user.id)
        .execute(&state.db)
        .await
        .map_err(AppError::internal)?;

    let user_simple = user.id.as_simple().to_string();
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
    .bind(user.id)
    .bind(&deleted_username)
    .bind(&deleted_email)
    .bind(&deleted_password_hash)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    auth::revoke_all_user_sessions(state.as_ref(), user.id).await?;

    record_auth_audit_log(
        state.as_ref(),
        user.id,
        &user.role,
        "auth.account.delete",
        json!({
            "username": user.username,
            "email": user.email,
            "request": request_meta(&headers)
        }),
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

fn build_auth_response(tokens: TokenBundle, user: AuthUser) -> AuthResponse {
    AuthResponse {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        token_type: tokens.token_type,
        access_expires_in_seconds: tokens.access_expires_in_seconds,
        refresh_expires_in_seconds: tokens.refresh_expires_in_seconds,
        user,
    }
}

fn to_auth_user(user: &UserAuthRow) -> AuthUser {
    AuthUser {
        id: user.id,
        username: user.username.clone(),
        email: user.email.clone(),
        role: user.role.clone(),
        created_at: user.created_at,
    }
}

async fn fetch_active_user_with_secret(state: &AppState, user_id: Uuid) -> AppResult<UserAuthRow> {
    let user = sqlx::query_as::<_, UserAuthRow>(
        "SELECT id, username, email, password_hash, role, status, created_at
         FROM users
         WHERE id = $1 AND status = 'active'
         LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::Unauthorized)?;

    Ok(user)
}

async fn record_auth_audit_log(
    state: &AppState,
    actor_user_id: Uuid,
    actor_role: &str,
    action: &str,
    detail: Value,
) {
    let result = sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, actor_role, action, target_type, target_id, detail)
         VALUES ($1, $2, $3, 'user', $1, $4)",
    )
    .bind(actor_user_id)
    .bind(actor_role)
    .bind(action)
    .bind(detail)
    .execute(&state.db)
    .await;

    if let Err(err) = result {
        warn!(
            actor_user_id = %actor_user_id,
            action = %action,
            error = %err,
            "failed to record auth audit log"
        );
    }
}

fn request_meta(headers: &HeaderMap) -> Value {
    let mut map = Map::new();

    if let Some(value) = header_to_string(headers, "x-forwarded-for") {
        map.insert("x_forwarded_for".to_string(), Value::String(value));
    }

    if let Some(value) = header_to_string(headers, "x-real-ip") {
        map.insert("x_real_ip".to_string(), Value::String(value));
    }

    if let Some(value) = header_to_string(headers, "user-agent") {
        map.insert("user_agent".to_string(), Value::String(value));
    }

    Value::Object(map)
}

fn header_to_string(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("failed to hash password")))?
        .to_string();
    Ok(hash)
}

fn verify_password(password: &str, password_hash: &str) -> AppResult<()> {
    let parsed_hash = PasswordHash::new(password_hash).map_err(|_| AppError::Unauthorized)?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::Unauthorized)
}

fn is_unique_violation(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db_err) => db_err.code().as_deref() == Some("23505"),
        _ => false,
    }
}

fn is_valid_username(username: &str) -> bool {
    let len = username.chars().count();
    if !(3..=32).contains(&len) {
        return false;
    }

    username
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

fn is_valid_email(email: &str) -> bool {
    let mut parts = email.split('@');
    let local = parts.next().unwrap_or_default();
    let domain = parts.next().unwrap_or_default();

    !local.is_empty()
        && !domain.is_empty()
        && parts.next().is_none()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}
