use std::sync::Arc;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, SaltString},
    Argon2, PasswordVerifier,
};
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
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
}

async fn register(
    State(state): State<Arc<AppState>>,
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
    .bind(username)
    .bind(email)
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

    Ok((
        StatusCode::CREATED,
        Json(build_auth_response(tokens, to_auth_user(&user))),
    ))
}

async fn login(
    State(state): State<Arc<AppState>>,
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

    Ok(Json(build_auth_response(tokens, to_auth_user(&user))))
}

async fn refresh(
    State(state): State<Arc<AppState>>,
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
