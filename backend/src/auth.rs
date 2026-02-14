use std::{future::Future, sync::Arc};

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

const ACCESS_TOKEN_TTL_SECS: i64 = 60 * 60;
const REFRESH_TOKEN_TTL_SECS: i64 = 7 * 24 * 60 * 60;

#[derive(Debug, Serialize)]
pub struct TokenBundle {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: &'static str,
    pub access_expires_in_seconds: i64,
    pub refresh_expires_in_seconds: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TokenClaims {
    sub: String,
    role: String,
    sid: String,
    token_use: String,
    jti: Option<String>,
    iat: usize,
    exp: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct RefreshSession {
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub refresh_jti: Uuid,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub role: String,
    pub session_id: Uuid,
}

impl FromRequestParts<Arc<AppState>> for AuthenticatedUser {
    type Rejection = AppError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        let token = extract_bearer_token(&parts.headers).map(str::to_owned);
        let jwt_secret = state.config.jwt_secret.clone();

        async move {
            let token = token?;
            decode_access_identity(&token, &jwt_secret)
        }
    }
}

pub async fn issue_new_session_tokens(
    state: &AppState,
    user_id: Uuid,
    role: &str,
) -> AppResult<TokenBundle> {
    let session_id = Uuid::new_v4();
    let refresh_jti = Uuid::new_v4();

    store_refresh_session(state, user_id, session_id, refresh_jti).await?;
    build_token_bundle(state, user_id, role, session_id, refresh_jti)
}

pub async fn rotate_session_tokens(
    state: &AppState,
    user_id: Uuid,
    role: &str,
    session_id: Uuid,
    presented_refresh_jti: Uuid,
) -> AppResult<TokenBundle> {
    validate_refresh_session(state, user_id, session_id, presented_refresh_jti).await?;

    let next_refresh_jti = Uuid::new_v4();
    store_refresh_session(state, user_id, session_id, next_refresh_jti).await?;

    build_token_bundle(state, user_id, role, session_id, next_refresh_jti)
}

pub fn decode_refresh_session(token: &str, jwt_secret: &str) -> AppResult<RefreshSession> {
    let claims = decode_token(token, jwt_secret, "refresh")?;
    let user_id = parse_uuid_claim(&claims.sub)?;
    let session_id = parse_uuid_claim(&claims.sid)?;
    let refresh_jti = claims
        .jti
        .as_deref()
        .ok_or(AppError::Unauthorized)
        .and_then(parse_uuid_claim)?;

    Ok(RefreshSession {
        user_id,
        session_id,
        refresh_jti,
    })
}

pub fn extract_bearer_token(headers: &HeaderMap) -> AppResult<&str> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    value.strip_prefix("Bearer ").ok_or(AppError::Unauthorized)
}

fn decode_access_identity(token: &str, jwt_secret: &str) -> AppResult<AuthenticatedUser> {
    let claims = decode_token(token, jwt_secret, "access")?;

    Ok(AuthenticatedUser {
        user_id: parse_uuid_claim(&claims.sub)?,
        role: claims.role,
        session_id: parse_uuid_claim(&claims.sid)?,
    })
}

pub fn decode_access_token(token: &str, jwt_secret: &str) -> AppResult<AuthenticatedUser> {
    decode_access_identity(token, jwt_secret)
}

fn build_token_bundle(
    state: &AppState,
    user_id: Uuid,
    role: &str,
    session_id: Uuid,
    refresh_jti: Uuid,
) -> AppResult<TokenBundle> {
    let access_token = encode_token(state, user_id, role, session_id, "access", None)?;
    let refresh_token = encode_token(
        state,
        user_id,
        role,
        session_id,
        "refresh",
        Some(refresh_jti),
    )?;

    Ok(TokenBundle {
        access_token,
        refresh_token,
        token_type: "Bearer",
        access_expires_in_seconds: ACCESS_TOKEN_TTL_SECS,
        refresh_expires_in_seconds: REFRESH_TOKEN_TTL_SECS,
    })
}

fn encode_token(
    state: &AppState,
    user_id: Uuid,
    role: &str,
    session_id: Uuid,
    token_use: &str,
    refresh_jti: Option<Uuid>,
) -> AppResult<String> {
    let now = Utc::now();
    let ttl_seconds = if token_use == "refresh" {
        REFRESH_TOKEN_TTL_SECS
    } else {
        ACCESS_TOKEN_TTL_SECS
    };

    let claims = TokenClaims {
        sub: user_id.to_string(),
        role: role.to_string(),
        sid: session_id.to_string(),
        token_use: token_use.to_string(),
        jti: refresh_jti.map(|id| id.to_string()),
        iat: now.timestamp() as usize,
        exp: (now + Duration::seconds(ttl_seconds)).timestamp() as usize,
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(state.config.jwt_secret.as_bytes()),
    )
    .map_err(AppError::internal)
}

fn decode_token(token: &str, jwt_secret: &str, expected_use: &str) -> AppResult<TokenClaims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    let claims = decode::<TokenClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|_| AppError::Unauthorized)?
    .claims;

    if claims.token_use != expected_use {
        return Err(AppError::Unauthorized);
    }

    Ok(claims)
}

fn parse_uuid_claim(value: &str) -> AppResult<Uuid> {
    Uuid::parse_str(value).map_err(|_| AppError::Unauthorized)
}

async fn validate_refresh_session(
    state: &AppState,
    user_id: Uuid,
    session_id: Uuid,
    refresh_jti: Uuid,
) -> AppResult<()> {
    let mut redis_conn = state.redis.clone();
    let key = refresh_session_key(session_id);
    let stored: Option<String> = redis_conn.get(&key).await.map_err(AppError::internal)?;

    if stored == Some(refresh_session_value(user_id, refresh_jti)) {
        Ok(())
    } else {
        Err(AppError::Unauthorized)
    }
}

async fn store_refresh_session(
    state: &AppState,
    user_id: Uuid,
    session_id: Uuid,
    refresh_jti: Uuid,
) -> AppResult<()> {
    let mut redis_conn = state.redis.clone();
    let key = refresh_session_key(session_id);
    let value = refresh_session_value(user_id, refresh_jti);

    redis_conn
        .set_ex::<_, _, ()>(key, value, REFRESH_TOKEN_TTL_SECS as u64)
        .await
        .map_err(AppError::internal)
}

fn refresh_session_key(session_id: Uuid) -> String {
    format!("auth:session:{}", session_id)
}

fn refresh_session_value(user_id: Uuid, refresh_jti: Uuid) -> String {
    format!("{}:{}", user_id, refresh_jti)
}
