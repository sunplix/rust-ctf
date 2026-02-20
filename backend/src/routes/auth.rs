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
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use tracing::warn;
use uuid::Uuid;

use crate::{
    auth::{self, AuthenticatedUser, RefreshSession, TokenBundle},
    error::{AppError, AppResult},
    mailer::{send_outbound_email, OutboundEmail},
    password_policy::{enforce_password_policy, PasswordContext, PasswordPolicySnapshot},
    state::AppState,
};

#[derive(Debug, Deserialize)]
struct RegisterRequest {
    username: String,
    email: String,
    password: String,
    password_confirm: String,
    captcha_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    identifier: String,
    password: String,
    captcha_token: Option<String>,
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
    new_password_confirm: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LoginHistoryQuery {
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct EmailActionRequest {
    email: String,
}

#[derive(Debug, Deserialize)]
struct TokenConfirmRequest {
    token: String,
}

#[derive(Debug, Deserialize)]
struct PasswordResetConfirmRequest {
    token: String,
    new_password: String,
    new_password_confirm: String,
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

#[derive(Debug, Serialize)]
struct RegisterResponse {
    requires_email_verification: bool,
    message: String,
    auth: Option<AuthResponse>,
}

#[derive(Debug, Serialize)]
struct ActionMessageResponse {
    message: String,
}

#[derive(Debug, Serialize)]
struct PasswordPolicyResponse {
    policy: PasswordPolicySnapshot,
}

#[derive(Debug, Serialize, FromRow)]
struct AuthUser {
    id: Uuid,
    username: String,
    email: String,
    role: String,
    email_verified: bool,
    email_verified_at: Option<DateTime<Utc>>,
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
    email_verified: bool,
    email_verified_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct TurnstileVerifyResponse {
    success: bool,
    #[serde(default, rename = "error-codes")]
    error_codes: Vec<String>,
    #[serde(default)]
    hostname: Option<String>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/auth/password-policy", get(get_password_policy))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
        .route("/auth/me", get(me))
        .route("/auth/profile", patch(update_profile))
        .route("/auth/change-password", post(change_password))
        .route(
            "/auth/email-verification/request",
            post(request_email_verification),
        )
        .route(
            "/auth/email-verification/confirm",
            post(confirm_email_verification),
        )
        .route("/auth/password-reset/request", post(request_password_reset))
        .route("/auth/password-reset/confirm", post(confirm_password_reset))
        .route("/auth/account", delete(delete_account))
        .route("/auth/login-history", get(login_history))
}

async fn get_password_policy(State(state): State<Arc<AppState>>) -> Json<PasswordPolicyResponse> {
    Json(PasswordPolicyResponse {
        policy: PasswordPolicySnapshot::from_config(&state.config),
    })
}

async fn register(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<RegisterRequest>,
) -> AppResult<(StatusCode, Json<RegisterResponse>)> {
    verify_human_verification(state.as_ref(), &headers, req.captcha_token.as_deref()).await?;

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

    if req.password != req.password_confirm {
        return Err(AppError::BadRequest(
            "password and password_confirm do not match".to_string(),
        ));
    }

    enforce_password_policy(
        &state.config,
        &req.password,
        PasswordContext {
            username: Some(&username),
            email: Some(&email),
        },
    )
    .map_err(AppError::BadRequest)?;

    let email_verification_enabled = state.config.auth_email_verification_enabled;
    let email_verification_required = is_email_verification_required(state.as_ref());
    let initial_email_verified = !email_verification_enabled;

    let password_hash = hash_password(&req.password)?;

    let user = sqlx::query_as::<_, UserAuthRow>(
        "INSERT INTO users (username, email, password_hash, role, status, email_verified, email_verified_at)
         VALUES ($1, $2, $3, 'player', 'active', $4, CASE WHEN $4 THEN NOW() ELSE NULL END)
         RETURNING id, username, email, password_hash, role, status, email_verified, email_verified_at, created_at",
    )
    .bind(&username)
    .bind(&email)
    .bind(password_hash)
    .bind(initial_email_verified)
    .fetch_one(&state.db)
    .await
    .map_err(|err| {
        if is_unique_violation(&err) {
            AppError::Conflict("username or email already exists".to_string())
        } else {
            AppError::internal(err)
        }
    })?;

    if email_verification_enabled && !user.email_verified {
        send_email_verification_flow(state.as_ref(), &user, &headers, "auth.register").await;
    }

    record_auth_audit_log(
        state.as_ref(),
        user.id,
        &user.role,
        "auth.register",
        json!({
            "username": username,
            "email": email,
            "email_verification_enabled": email_verification_enabled,
            "email_verification_required": email_verification_required,
            "request": request_meta(&headers)
        }),
    )
    .await;

    if email_verification_required {
        return Ok((
            StatusCode::ACCEPTED,
            Json(RegisterResponse {
                requires_email_verification: true,
                message: "registration succeeded, please verify your email before signing in"
                    .to_string(),
                auth: None,
            }),
        ));
    }

    let tokens = auth::issue_new_session_tokens(state.as_ref(), user.id, &user.role).await?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            requires_email_verification: false,
            message: if email_verification_enabled {
                "registration succeeded, verification email has been sent".to_string()
            } else {
                "registration succeeded".to_string()
            },
            auth: Some(build_auth_response(tokens, to_auth_user(&user))),
        }),
    ))
}

async fn login(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    verify_human_verification(state.as_ref(), &headers, req.captcha_token.as_deref()).await?;

    let identifier = req.identifier.trim();
    if identifier.is_empty() || req.password.is_empty() {
        return Err(AppError::BadRequest(
            "identifier and password are required".to_string(),
        ));
    }

    let user = sqlx::query_as::<_, UserAuthRow>(
        "SELECT id, username, email, password_hash, role, status, email_verified, email_verified_at, created_at
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

    if is_email_verification_required(state.as_ref()) && !user.email_verified {
        return Err(AppError::BadRequest(
            "email is not verified, please complete email verification first".to_string(),
        ));
    }

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
    if is_email_verification_required(state.as_ref()) && !user.email_verified {
        return Err(AppError::BadRequest(
            "email is not verified, please complete email verification first".to_string(),
        ));
    }

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
        "SELECT id, username, email, role, email_verified, email_verified_at, created_at
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
    let original_user = fetch_active_user_with_secret(state.as_ref(), current_user.user_id).await?;

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

    let email_verification_enabled = state.config.auth_email_verification_enabled;

    let updated = sqlx::query_as::<_, AuthUser>(
        "UPDATE users
         SET username = COALESCE($2, username),
             email = COALESCE($3, email),
             email_verified = CASE
                 WHEN $3::text IS NULL THEN email_verified
                 WHEN LOWER($3) = LOWER(email) THEN email_verified
                 WHEN $4 THEN FALSE
                 ELSE email_verified
             END,
             email_verified_at = CASE
                 WHEN $3::text IS NULL THEN email_verified_at
                 WHEN LOWER($3) = LOWER(email) THEN email_verified_at
                 WHEN $4 THEN NULL
                 ELSE email_verified_at
             END,
             updated_at = NOW()
         WHERE id = $1 AND status = 'active'
         RETURNING id, username, email, role, email_verified, email_verified_at, created_at",
    )
    .bind(current_user.user_id)
    .bind(&username)
    .bind(&email)
    .bind(email_verification_enabled)
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

    let email_changed = updated.email.to_ascii_lowercase() != original_user.email.to_ascii_lowercase();
    if email_verification_enabled && email_changed {
        let refreshed_user = fetch_active_user_with_secret(state.as_ref(), current_user.user_id).await?;
        send_email_verification_flow(
            state.as_ref(),
            &refreshed_user,
            &headers,
            "auth.profile.update",
        )
        .await;
    }

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
            "email_verification_reset": email_verification_enabled && email_changed,
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

    if let Some(new_password_confirm) = req.new_password_confirm.as_deref() {
        if req.new_password != new_password_confirm {
            return Err(AppError::BadRequest(
                "new_password and new_password_confirm do not match".to_string(),
            ));
        }
    }

    if req.current_password == req.new_password {
        return Err(AppError::BadRequest(
            "new_password must be different from current_password".to_string(),
        ));
    }

    let user = fetch_active_user_with_secret(state.as_ref(), current_user.user_id).await?;
    verify_password(&req.current_password, &user.password_hash)?;

    enforce_password_policy(
        &state.config,
        &req.new_password,
        PasswordContext {
            username: Some(&user.username),
            email: Some(&user.email),
        },
    )
    .map_err(AppError::BadRequest)?;

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

async fn request_email_verification(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<EmailActionRequest>,
) -> AppResult<Json<ActionMessageResponse>> {
    if !state.config.auth_email_verification_enabled {
        return Err(AppError::BadRequest(
            "email verification is disabled".to_string(),
        ));
    }

    let email = req.email.trim().to_lowercase();
    if !is_valid_email(&email) {
        return Err(AppError::BadRequest("invalid email format".to_string()));
    }

    let user = sqlx::query_as::<_, UserAuthRow>(
        "SELECT id, username, email, password_hash, role, status, email_verified, email_verified_at, created_at
         FROM users
         WHERE LOWER(email) = LOWER($1)
           AND status = 'active'
         LIMIT 1",
    )
    .bind(&email)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?;

    if let Some(user) = user {
        if !user.email_verified {
            send_email_verification_flow(
                state.as_ref(),
                &user,
                &headers,
                "auth.email.verification.request",
            )
            .await;
        }
    }

    Ok(Json(ActionMessageResponse {
        message: "if the account exists and verification is pending, a verification email has been sent"
            .to_string(),
    }))
}

async fn confirm_email_verification(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<TokenConfirmRequest>,
) -> AppResult<Json<ActionMessageResponse>> {
    if !state.config.auth_email_verification_enabled {
        return Err(AppError::BadRequest(
            "email verification is disabled".to_string(),
        ));
    }

    let token = req.token.trim();
    if token.is_empty() {
        return Err(AppError::BadRequest("token is required".to_string()));
    }

    let token_hash = hash_security_token(token);
    let user = sqlx::query_as::<_, UserAuthRow>(
        "SELECT u.id, u.username, u.email, u.password_hash, u.role, u.status, u.email_verified, u.email_verified_at, u.created_at
         FROM auth_email_verification_tokens t
         JOIN users u ON u.id = t.user_id
         WHERE t.token_hash = $1
           AND t.used_at IS NULL
           AND t.expires_at > NOW()
           AND u.status = 'active'
         LIMIT 1",
    )
    .bind(&token_hash)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or_else(|| AppError::BadRequest("invalid or expired verification token".to_string()))?;

    sqlx::query(
        "UPDATE auth_email_verification_tokens
         SET used_at = NOW()
         WHERE token_hash = $1
           AND used_at IS NULL",
    )
    .bind(&token_hash)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    sqlx::query(
        "UPDATE users
         SET email_verified = TRUE,
             email_verified_at = COALESCE(email_verified_at, NOW()),
             updated_at = NOW()
         WHERE id = $1",
    )
    .bind(user.id)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    record_auth_audit_log(
        state.as_ref(),
        user.id,
        &user.role,
        "auth.email.verify.confirm",
        json!({
            "email": user.email,
            "request": request_meta(&headers)
        }),
    )
    .await;

    Ok(Json(ActionMessageResponse {
        message: "email verification completed".to_string(),
    }))
}

async fn request_password_reset(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<EmailActionRequest>,
) -> AppResult<Json<ActionMessageResponse>> {
    if !state.config.auth_password_reset_enabled {
        return Err(AppError::BadRequest(
            "password reset is disabled".to_string(),
        ));
    }

    let email = req.email.trim().to_lowercase();
    if !is_valid_email(&email) {
        return Err(AppError::BadRequest("invalid email format".to_string()));
    }

    let user = sqlx::query_as::<_, UserAuthRow>(
        "SELECT id, username, email, password_hash, role, status, email_verified, email_verified_at, created_at
         FROM users
         WHERE LOWER(email) = LOWER($1)
           AND status = 'active'
         LIMIT 1",
    )
    .bind(&email)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?;

    if let Some(user) = user {
        send_password_reset_flow(state.as_ref(), &user, &headers, "auth.password.reset.request").await;
    }

    Ok(Json(ActionMessageResponse {
        message: "if the account exists, a password reset email has been sent".to_string(),
    }))
}

async fn confirm_password_reset(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<PasswordResetConfirmRequest>,
) -> AppResult<Json<ActionMessageResponse>> {
    if !state.config.auth_password_reset_enabled {
        return Err(AppError::BadRequest(
            "password reset is disabled".to_string(),
        ));
    }

    let token = req.token.trim();
    if token.is_empty() {
        return Err(AppError::BadRequest("token is required".to_string()));
    }

    if req.new_password != req.new_password_confirm {
        return Err(AppError::BadRequest(
            "new_password and new_password_confirm do not match".to_string(),
        ));
    }

    let token_hash = hash_security_token(token);
    let user = sqlx::query_as::<_, UserAuthRow>(
        "SELECT u.id, u.username, u.email, u.password_hash, u.role, u.status, u.email_verified, u.email_verified_at, u.created_at
         FROM auth_password_reset_tokens t
         JOIN users u ON u.id = t.user_id
         WHERE t.token_hash = $1
           AND t.used_at IS NULL
           AND t.expires_at > NOW()
           AND u.status = 'active'
         LIMIT 1",
    )
    .bind(&token_hash)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or_else(|| AppError::BadRequest("invalid or expired password reset token".to_string()))?;

    enforce_password_policy(
        &state.config,
        &req.new_password,
        PasswordContext {
            username: Some(&user.username),
            email: Some(&user.email),
        },
    )
    .map_err(AppError::BadRequest)?;

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

    sqlx::query(
        "UPDATE auth_password_reset_tokens
         SET used_at = NOW()
         WHERE token_hash = $1
           AND used_at IS NULL",
    )
    .bind(&token_hash)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    auth::revoke_all_user_sessions(state.as_ref(), user.id).await?;

    record_auth_audit_log(
        state.as_ref(),
        user.id,
        &user.role,
        "auth.password.reset.confirm",
        json!({
            "request": request_meta(&headers)
        }),
    )
    .await;

    Ok(Json(ActionMessageResponse {
        message: "password reset succeeded, please sign in with your new password".to_string(),
    }))
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
           AND action IN (
             'auth.register',
             'auth.login',
             'auth.refresh',
             'auth.password.change',
             'auth.email.verify.confirm',
             'auth.password.reset.confirm'
           )
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
             email_verified = FALSE,
             email_verified_at = NULL,
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
        email_verified: user.email_verified,
        email_verified_at: user.email_verified_at,
        created_at: user.created_at,
    }
}

async fn fetch_active_user_with_secret(state: &AppState, user_id: Uuid) -> AppResult<UserAuthRow> {
    let user = sqlx::query_as::<_, UserAuthRow>(
        "SELECT id, username, email, password_hash, role, status, email_verified, email_verified_at, created_at
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

    if let Some(value) = header_to_string(headers, "cf-connecting-ip") {
        map.insert("cf_connecting_ip".to_string(), Value::String(value));
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

async fn verify_human_verification(
    state: &AppState,
    headers: &HeaderMap,
    captcha_token: Option<&str>,
) -> AppResult<()> {
    if !state.config.auth_human_verification_enabled {
        return Ok(());
    }

    let secret = state.config.auth_turnstile_secret_key.trim();
    if secret.is_empty() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "AUTH_TURNSTILE_SECRET_KEY is empty while human verification is enabled"
        )));
    }

    let token = captcha_token
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::BadRequest("captcha_token is required".to_string()))?;

    let timeout_seconds = state
        .config
        .auth_human_verification_timeout_seconds
        .clamp(2, 20);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_seconds))
        .build()
        .map_err(AppError::internal)?;

    let mut form: Vec<(&str, String)> = vec![
        ("secret", secret.to_string()),
        ("response", token.to_string()),
    ];

    if let Some(remote_ip) = client_ip_from_headers(headers) {
        form.push(("remoteip", remote_ip));
    }

    let response = client
        .post(state.config.auth_turnstile_siteverify_url.trim())
        .form(&form)
        .send()
        .await
        .map_err(|err| AppError::BadRequest(format!("human verification request failed: {}", err)))?;

    let payload = response
        .json::<TurnstileVerifyResponse>()
        .await
        .map_err(|err| {
            AppError::BadRequest(format!(
                "human verification response parse failed: {}",
                err
            ))
        })?;

    if !payload.success {
        let detail = if payload.error_codes.is_empty() {
            "verification failed".to_string()
        } else {
            payload.error_codes.join(",")
        };
        return Err(AppError::BadRequest(format!(
            "human verification failed: {}",
            detail
        )));
    }

    let expected_hostname = state.config.auth_turnstile_expected_hostname.trim();
    if !expected_hostname.is_empty() {
        let actual_hostname = payload.hostname.as_deref().unwrap_or("");
        if !actual_hostname.eq_ignore_ascii_case(expected_hostname) {
            return Err(AppError::BadRequest(
                "human verification hostname mismatch".to_string(),
            ));
        }
    }

    Ok(())
}

fn client_ip_from_headers(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = header_to_string(headers, "cf-connecting-ip") {
        return Some(value);
    }

    if let Some(value) = header_to_string(headers, "x-real-ip") {
        return Some(value);
    }

    let xff = header_to_string(headers, "x-forwarded-for")?;
    xff.split(',')
        .next()
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

fn is_email_verification_required(state: &AppState) -> bool {
    state.config.auth_email_verification_enabled && state.config.auth_email_verification_required
}

fn generate_security_token() -> String {
    let mut raw = [0_u8; 32];
    use argon2::password_hash::rand_core::RngCore;
    OsRng.fill_bytes(&mut raw);
    URL_SAFE_NO_PAD.encode(raw)
}

fn hash_security_token(raw_token: &str) -> String {
    let digest = Sha256::digest(raw_token.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

async fn issue_email_verification_token(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
) -> AppResult<String> {
    let ttl_minutes = state
        .config
        .auth_email_verification_token_ttl_minutes
        .clamp(5, 60 * 24 * 7);

    let token = generate_security_token();
    let token_hash = hash_security_token(&token);
    let expires_at = Utc::now() + Duration::minutes(ttl_minutes);
    let request_ip = client_ip_from_headers(headers);
    let user_agent = header_to_string(headers, "user-agent");

    sqlx::query(
        "UPDATE auth_email_verification_tokens
         SET used_at = NOW()
         WHERE user_id = $1
           AND used_at IS NULL",
    )
    .bind(user_id)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    sqlx::query(
        "INSERT INTO auth_email_verification_tokens (user_id, token_hash, expires_at, request_ip, user_agent)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .bind(request_ip)
    .bind(user_agent)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(token)
}

async fn issue_password_reset_token(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
) -> AppResult<String> {
    let ttl_minutes = state
        .config
        .auth_password_reset_token_ttl_minutes
        .clamp(5, 60 * 24 * 7);

    let token = generate_security_token();
    let token_hash = hash_security_token(&token);
    let expires_at = Utc::now() + Duration::minutes(ttl_minutes);
    let request_ip = client_ip_from_headers(headers);
    let user_agent = header_to_string(headers, "user-agent");

    sqlx::query(
        "UPDATE auth_password_reset_tokens
         SET used_at = NOW()
         WHERE user_id = $1
           AND used_at IS NULL",
    )
    .bind(user_id)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    sqlx::query(
        "INSERT INTO auth_password_reset_tokens (user_id, token_hash, expires_at, request_ip, user_agent)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .bind(request_ip)
    .bind(user_agent)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(token)
}

async fn send_email_verification_flow(
    state: &AppState,
    user: &UserAuthRow,
    headers: &HeaderMap,
    source: &str,
) {
    match issue_email_verification_token(state, user.id, headers).await {
        Ok(raw_token) => {
            if let Err(err) = send_email_verification_message(state, user, &raw_token).await {
                warn!(
                    user_id = %user.id,
                    email = %user.email,
                    source,
                    error = %err,
                    "failed to send email verification message"
                );
            }
        }
        Err(err) => {
            warn!(
                user_id = %user.id,
                email = %user.email,
                source,
                error = %err,
                "failed to issue email verification token"
            );
        }
    }
}

async fn send_password_reset_flow(
    state: &AppState,
    user: &UserAuthRow,
    headers: &HeaderMap,
    source: &str,
) {
    match issue_password_reset_token(state, user.id, headers).await {
        Ok(raw_token) => {
            if let Err(err) = send_password_reset_message(state, user, &raw_token).await {
                warn!(
                    user_id = %user.id,
                    email = %user.email,
                    source,
                    error = %err,
                    "failed to send password reset message"
                );
            }
        }
        Err(err) => {
            warn!(
                user_id = %user.id,
                email = %user.email,
                source,
                error = %err,
                "failed to issue password reset token"
            );
        }
    }
}

async fn send_email_verification_message(
    state: &AppState,
    user: &UserAuthRow,
    raw_token: &str,
) -> anyhow::Result<()> {
    let verify_url = build_frontend_login_url(
        &state.config.auth_email_base_url,
        &format!("verify_token={raw_token}"),
    );

    let body = format!(
        "Hello {username},\n\nTo verify your Rust-CTF account email, open the link below:\n{verify_url}\n\nIf you did not request this action, you can ignore this message.",
        username = user.username
    );

    send_outbound_email(
        &state.config,
        OutboundEmail {
            to: user.email.clone(),
            subject: "[Rust-CTF] Verify your email".to_string(),
            text_body: body,
        },
    )
    .await
}

async fn send_password_reset_message(
    state: &AppState,
    user: &UserAuthRow,
    raw_token: &str,
) -> anyhow::Result<()> {
    let reset_url = build_frontend_login_url(
        &state.config.auth_email_base_url,
        &format!("reset_token={raw_token}"),
    );

    let body = format!(
        "Hello {username},\n\nTo reset your Rust-CTF account password, open the link below:\n{reset_url}\n\nIf you did not request this action, you can ignore this message.",
        username = user.username
    );

    send_outbound_email(
        &state.config,
        OutboundEmail {
            to: user.email.clone(),
            subject: "[Rust-CTF] Password reset".to_string(),
            text_body: body,
        },
    )
    .await
}

fn build_frontend_login_url(base_url: &str, query: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    let fallback = "http://127.0.0.1:5173";
    let root = if base.is_empty() { fallback } else { base };
    format!("{root}/login?{query}")
}
