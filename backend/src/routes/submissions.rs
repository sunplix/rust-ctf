use std::sync::Arc;

use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
use axum::{extract::State, routing::post, Json, Router};
use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    auth::AuthenticatedUser,
    error::{AppError, AppResult},
    state::AppState,
};

const SUBMISSION_RATE_WINDOW_SECS: i64 = 30;
const SUBMISSION_RATE_MAX_COUNT: i64 = 10;

#[derive(Debug, Deserialize)]
struct SubmitFlagRequest {
    contest_id: Uuid,
    challenge_id: Uuid,
    flag: String,
}

#[derive(Debug, Serialize)]
struct SubmitFlagResponse {
    verdict: String,
    score_awarded: i32,
    total_score: i64,
    message: String,
    submitted_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct TeamMembershipRow {
    team_id: Uuid,
}

#[derive(Debug, FromRow)]
struct JudgeContextRow {
    contest_status: String,
    contest_start_at: DateTime<Utc>,
    contest_end_at: DateTime<Utc>,
    challenge_type: String,
    flag_mode: String,
    flag_hash: String,
    static_score: i32,
    is_visible: bool,
    release_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow)]
struct SubmittedAtRow {
    submitted_at: DateTime<Utc>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/submissions", post(submit_flag))
}

async fn submit_flag(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Json(req): Json<SubmitFlagRequest>,
) -> AppResult<Json<SubmitFlagResponse>> {
    let submitted_flag = req.flag.trim();
    if submitted_flag.is_empty() {
        return Err(AppError::BadRequest("flag is required".to_string()));
    }

    let membership = sqlx::query_as::<_, TeamMembershipRow>(
        "SELECT team_id FROM team_members WHERE user_id = $1 LIMIT 1",
    )
    .bind(current_user.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::Forbidden)?;

    let judge_ctx = sqlx::query_as::<_, JudgeContextRow>(
        "SELECT ct.status AS contest_status,
                ct.start_at AS contest_start_at,
                ct.end_at AS contest_end_at,
                c.challenge_type,
                c.flag_mode,
                c.flag_hash,
                c.static_score,
                c.is_visible,
                cc.release_at
         FROM contest_challenges cc
         JOIN contests ct ON ct.id = cc.contest_id
         JOIN challenges c ON c.id = cc.challenge_id
         WHERE cc.contest_id = $1 AND cc.challenge_id = $2
         LIMIT 1",
    )
    .bind(req.contest_id)
    .bind(req.challenge_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest(
        "challenge is not available in this contest".to_string(),
    ))?;

    validate_submission_window(&judge_ctx)?;

    if let Err(err) =
        enforce_submission_rate_limit(state.as_ref(), current_user.user_id, req.contest_id).await
    {
        if let AppError::TooManyRequests(message) = err {
            let submitted_at = insert_submission(
                state.as_ref(),
                req.contest_id,
                req.challenge_id,
                membership.team_id,
                current_user.user_id,
                submitted_flag,
                "rate_limited",
                0,
                &message,
            )
            .await?;

            let total_score =
                fetch_total_score(state.as_ref(), req.contest_id, membership.team_id).await?;

            return Ok(Json(SubmitFlagResponse {
                verdict: "rate_limited".to_string(),
                score_awarded: 0,
                total_score,
                message,
                submitted_at,
            }));
        }

        return Err(err);
    }

    let already_solved = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1 FROM submissions
            WHERE contest_id = $1
              AND challenge_id = $2
              AND team_id = $3
              AND verdict = 'accepted'
         )",
    )
    .bind(req.contest_id)
    .bind(req.challenge_id)
    .bind(membership.team_id)
    .fetch_one(&state.db)
    .await
    .map_err(AppError::internal)?;

    let (verdict, score_awarded, message) = judge_flag(
        state.as_ref(),
        &judge_ctx,
        req.contest_id,
        req.challenge_id,
        membership.team_id,
        submitted_flag,
        already_solved,
    )
    .await?;

    let submitted_at = insert_submission(
        state.as_ref(),
        req.contest_id,
        req.challenge_id,
        membership.team_id,
        current_user.user_id,
        submitted_flag,
        &verdict,
        score_awarded,
        &message,
    )
    .await?;

    let total_score = fetch_total_score(state.as_ref(), req.contest_id, membership.team_id).await?;

    Ok(Json(SubmitFlagResponse {
        verdict,
        score_awarded,
        total_score,
        message,
        submitted_at,
    }))
}

async fn enforce_submission_rate_limit(
    state: &AppState,
    user_id: Uuid,
    contest_id: Uuid,
) -> AppResult<()> {
    let mut redis_conn = state.redis.clone();
    let key = format!("ratelimit:submit:{}:{}", contest_id, user_id);

    let count: i64 = redis_conn.incr(&key, 1).await.map_err(AppError::internal)?;
    if count == 1 {
        let _: bool = redis_conn
            .expire(&key, SUBMISSION_RATE_WINDOW_SECS)
            .await
            .map_err(AppError::internal)?;
    }

    if count > SUBMISSION_RATE_MAX_COUNT {
        return Err(AppError::TooManyRequests(format!(
            "submission rate limit exceeded: max {} submissions per {} seconds",
            SUBMISSION_RATE_MAX_COUNT, SUBMISSION_RATE_WINDOW_SECS
        )));
    }

    Ok(())
}

async fn insert_submission(
    state: &AppState,
    contest_id: Uuid,
    challenge_id: Uuid,
    team_id: Uuid,
    user_id: Uuid,
    submitted_flag: &str,
    verdict: &str,
    score_awarded: i32,
    message: &str,
) -> AppResult<DateTime<Utc>> {
    let submitted_at = sqlx::query_as::<_, SubmittedAtRow>(
        "INSERT INTO submissions (
            contest_id,
            challenge_id,
            team_id,
            user_id,
            submitted_flag,
            verdict,
            score_awarded,
            judger_message,
            judged_at
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
         RETURNING submitted_at",
    )
    .bind(contest_id)
    .bind(challenge_id)
    .bind(team_id)
    .bind(user_id)
    .bind(submitted_flag)
    .bind(verdict)
    .bind(score_awarded)
    .bind(message)
    .fetch_one(&state.db)
    .await
    .map_err(AppError::internal)?
    .submitted_at;

    Ok(submitted_at)
}

async fn fetch_total_score(state: &AppState, contest_id: Uuid, team_id: Uuid) -> AppResult<i64> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(score_awarded), 0)
         FROM submissions
         WHERE contest_id = $1 AND team_id = $2",
    )
    .bind(contest_id)
    .bind(team_id)
    .fetch_one(&state.db)
    .await
    .map_err(AppError::internal)
}

fn validate_submission_window(ctx: &JudgeContextRow) -> AppResult<()> {
    let now = Utc::now();

    if !ctx.is_visible {
        return Err(AppError::BadRequest("challenge is not visible".to_string()));
    }

    if let Some(release_at) = ctx.release_at {
        if now < release_at {
            return Err(AppError::BadRequest(
                "challenge has not been released yet".to_string(),
            ));
        }
    }

    if ctx.contest_status != "running" {
        return Err(AppError::BadRequest("contest is not running".to_string()));
    }

    if now < ctx.contest_start_at || now > ctx.contest_end_at {
        return Err(AppError::BadRequest(
            "outside contest submission window".to_string(),
        ));
    }

    Ok(())
}

async fn judge_flag(
    state: &AppState,
    ctx: &JudgeContextRow,
    contest_id: Uuid,
    challenge_id: Uuid,
    team_id: Uuid,
    submitted_flag: &str,
    already_solved: bool,
) -> AppResult<(String, i32, String)> {
    let is_correct = match ctx.flag_mode.as_str() {
        "static" => verify_static_flag(submitted_flag, &ctx.flag_hash)?,
        "dynamic" => {
            let mut redis_conn = state.redis.clone();
            let key = format!("flag:dynamic:{}:{}:{}", contest_id, challenge_id, team_id);
            let expected: Option<String> =
                redis_conn.get(&key).await.map_err(AppError::internal)?;

            match expected {
                Some(flag) => submitted_flag == flag,
                None => {
                    return Ok((
                        "invalid".to_string(),
                        0,
                        "dynamic flag is not provisioned yet".to_string(),
                    ));
                }
            }
        }
        "script" => {
            return Ok((
                "invalid".to_string(),
                0,
                format!(
                    "script flag verifier is not implemented yet for challenge type '{}'",
                    ctx.challenge_type
                ),
            ));
        }
        other => {
            return Ok((
                "invalid".to_string(),
                0,
                format!("unsupported flag mode '{}'", other),
            ));
        }
    };

    if is_correct {
        if already_solved {
            return Ok((
                "accepted".to_string(),
                0,
                "correct flag, but this challenge is already solved by your team".to_string(),
            ));
        }

        return Ok((
            "accepted".to_string(),
            ctx.static_score,
            "correct flag".to_string(),
        ));
    }

    Ok(("wrong".to_string(), 0, "incorrect flag".to_string()))
}

fn verify_static_flag(submitted_flag: &str, stored_flag: &str) -> AppResult<bool> {
    if stored_flag.starts_with("$argon2") {
        let parsed_hash = PasswordHash::new(stored_flag).map_err(|_| {
            AppError::BadRequest("challenge static flag hash is malformed".to_string())
        })?;

        let verified = Argon2::default()
            .verify_password(submitted_flag.as_bytes(), &parsed_hash)
            .is_ok();
        return Ok(verified);
    }

    Ok(submitted_flag == stored_flag)
}
