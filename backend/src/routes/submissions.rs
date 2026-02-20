use std::{process::Stdio, sync::Arc};

use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
use axum::{extract::State, routing::post, Json, Router};
use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use tokio::{
    process::Command,
    time::{timeout, Duration as TokioDuration},
};
use tracing::{info, warn};
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
    contest_scoring_mode: String,
    contest_dynamic_decay: i32,
    contest_first_blood_bonus_percent: i32,
    contest_second_blood_bonus_percent: i32,
    contest_third_blood_bonus_percent: i32,
    flag_mode: String,
    flag_hash: String,
    static_score: i32,
    min_score: i32,
    max_score: i32,
    is_visible: bool,
    release_at: Option<DateTime<Utc>>,
    metadata: Value,
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
    info!(
        user_id = %current_user.user_id,
        user_role = %current_user.role,
        contest_id = %req.contest_id,
        challenge_id = %req.challenge_id,
        "submission request received"
    );

    let submitted_flag = req.flag.trim();
    if submitted_flag.is_empty() {
        warn!(
            user_id = %current_user.user_id,
            contest_id = %req.contest_id,
            challenge_id = %req.challenge_id,
            "submission rejected: empty flag"
        );
        return Err(AppError::BadRequest("flag is required".to_string()));
    }

    let membership = sqlx::query_as::<_, TeamMembershipRow>(
        "SELECT team_id FROM team_members WHERE user_id = $1 LIMIT 1",
    )
    .bind(current_user.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?;

    let membership = match membership {
        Some(row) => row,
        None => {
            warn!(
                user_id = %current_user.user_id,
                user_role = %current_user.role,
                contest_id = %req.contest_id,
                challenge_id = %req.challenge_id,
                "submission denied: user has no team membership"
            );
            return Err(AppError::Forbidden);
        }
    };

    info!(
        user_id = %current_user.user_id,
        team_id = %membership.team_id,
        contest_id = %req.contest_id,
        challenge_id = %req.challenge_id,
        "submission team membership resolved"
    );

    let judge_ctx = sqlx::query_as::<_, JudgeContextRow>(
        "SELECT ct.status AS contest_status,
                ct.start_at AS contest_start_at,
                ct.end_at AS contest_end_at,
                ct.scoring_mode AS contest_scoring_mode,
                ct.dynamic_decay AS contest_dynamic_decay,
                ct.first_blood_bonus_percent AS contest_first_blood_bonus_percent,
                ct.second_blood_bonus_percent AS contest_second_blood_bonus_percent,
                ct.third_blood_bonus_percent AS contest_third_blood_bonus_percent,
                c.flag_mode,
                c.flag_hash,
                c.static_score,
                c.min_score,
                c.max_score,
                c.is_visible,
                cc.release_at,
                c.metadata
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

    if let Err(err) = validate_submission_window(&judge_ctx) {
        warn!(
            user_id = %current_user.user_id,
            team_id = %membership.team_id,
            contest_id = %req.contest_id,
            challenge_id = %req.challenge_id,
            contest_status = %judge_ctx.contest_status,
            is_visible = judge_ctx.is_visible,
            release_at = ?judge_ctx.release_at,
            error = %err,
            "submission rejected by contest/challenge window validation"
        );
        return Err(err);
    }

    if let Err(err) =
        enforce_submission_rate_limit(state.as_ref(), current_user.user_id, req.contest_id).await
    {
        if let AppError::TooManyRequests(message) = err {
            warn!(
                user_id = %current_user.user_id,
                team_id = %membership.team_id,
                contest_id = %req.contest_id,
                challenge_id = %req.challenge_id,
                message = %message,
                "submission rate limited"
            );
            let submitted_at = insert_submission(
                state.as_ref(),
                SubmissionInsertParams {
                    contest_id: req.contest_id,
                    challenge_id: req.challenge_id,
                    team_id: membership.team_id,
                    user_id: current_user.user_id,
                    submitted_flag,
                    verdict: "rate_limited",
                    score_awarded: 0,
                    message: &message,
                },
            )
            .await?;

            let total_score =
                fetch_total_score(state.as_ref(), req.contest_id, membership.team_id).await?;
            publish_scoreboard_update(state.as_ref(), req.contest_id).await;

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
        SubmissionInsertParams {
            contest_id: req.contest_id,
            challenge_id: req.challenge_id,
            team_id: membership.team_id,
            user_id: current_user.user_id,
            submitted_flag,
            verdict: &verdict,
            score_awarded,
            message: &message,
        },
    )
    .await?;

    let total_score = fetch_total_score(state.as_ref(), req.contest_id, membership.team_id).await?;
    publish_scoreboard_update(state.as_ref(), req.contest_id).await;

    info!(
        user_id = %current_user.user_id,
        team_id = %membership.team_id,
        contest_id = %req.contest_id,
        challenge_id = %req.challenge_id,
        verdict = %verdict,
        score_awarded,
        total_score,
        "submission judged"
    );

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

struct SubmissionInsertParams<'a> {
    contest_id: Uuid,
    challenge_id: Uuid,
    team_id: Uuid,
    user_id: Uuid,
    submitted_flag: &'a str,
    verdict: &'a str,
    score_awarded: i32,
    message: &'a str,
}

async fn insert_submission(
    state: &AppState,
    params: SubmissionInsertParams<'_>,
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
    .bind(params.contest_id)
    .bind(params.challenge_id)
    .bind(params.team_id)
    .bind(params.user_id)
    .bind(params.submitted_flag)
    .bind(params.verdict)
    .bind(params.score_awarded)
    .bind(params.message)
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

async fn publish_scoreboard_update(state: &AppState, contest_id: Uuid) {
    let channel = format!("scoreboard:contest:{}", contest_id);
    let payload = Utc::now().to_rfc3339();
    let mut redis_conn = state.redis.clone();

    let publish_result: Result<usize, redis::RedisError> =
        redis_conn.publish(channel, payload).await;
    if let Err(err) = publish_result {
        warn!(contest_id = %contest_id, error = %err, "failed to publish scoreboard update event");
    }
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
        warn!(
            now = %now,
            contest_start_at = %ctx.contest_start_at,
            contest_end_at = %ctx.contest_end_at,
            "contest status is running but now is outside configured start/end window; allowing submission by status"
        );
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
    let decision = match ctx.flag_mode.as_str() {
        "static" => {
            if verify_static_flag(submitted_flag, &ctx.flag_hash)? {
                JudgeDecision::Correct("correct flag".to_string())
            } else {
                JudgeDecision::Wrong("incorrect flag".to_string())
            }
        }
        "dynamic" => {
            let mut redis_conn = state.redis.clone();
            let key = format!("flag:dynamic:{}:{}:{}", contest_id, challenge_id, team_id);
            let expected: Option<String> =
                redis_conn.get(&key).await.map_err(AppError::internal)?;

            match expected {
                Some(flag) => {
                    if submitted_flag == flag {
                        JudgeDecision::Correct("correct dynamic flag".to_string())
                    } else {
                        JudgeDecision::Wrong("incorrect flag".to_string())
                    }
                }
                None => JudgeDecision::Invalid("dynamic flag is not provisioned yet".to_string()),
            }
        }
        "script" => {
            run_script_verifier(ctx, contest_id, challenge_id, team_id, submitted_flag).await
        }
        other => JudgeDecision::Invalid(format!("unsupported flag mode '{}'", other)),
    };

    match decision {
        JudgeDecision::Correct(message) => {
            if already_solved {
                return Ok((
                    "accepted".to_string(),
                    0,
                    "correct flag, but this challenge is already solved by your team".to_string(),
                ));
            }

            let score_awarded =
                calculate_awarded_score(state, ctx, contest_id, challenge_id).await?;
            Ok(("accepted".to_string(), score_awarded, message))
        }
        JudgeDecision::Wrong(message) => Ok(("wrong".to_string(), 0, message)),
        JudgeDecision::Invalid(message) => Ok(("invalid".to_string(), 0, message)),
    }
}

async fn calculate_awarded_score(
    state: &AppState,
    ctx: &JudgeContextRow,
    contest_id: Uuid,
    challenge_id: Uuid,
) -> AppResult<i32> {
    let solved_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT team_id)
         FROM submissions
         WHERE contest_id = $1
           AND challenge_id = $2
           AND verdict = 'accepted'
           AND score_awarded > 0",
    )
    .bind(contest_id)
    .bind(challenge_id)
    .fetch_one(&state.db)
    .await
    .map_err(AppError::internal)?;

    let base_score = if ctx.contest_scoring_mode != "dynamic" {
        ctx.static_score.max(0)
    } else {
        let min_score = ctx.min_score.max(0);
        let max_score = ctx.max_score.max(min_score);
        if max_score == min_score {
            max_score
        } else {
            let decay = ctx.contest_dynamic_decay.max(1) as f64;
            let solves = solved_count.max(0) as f64;
            let raw = min_score as f64 + (max_score - min_score) as f64 * (decay / (decay + solves));
            let score = raw.round() as i32;
            score.clamp(min_score, max_score)
        }
    };

    let bonus_percent = match solved_count {
        0 => ctx.contest_first_blood_bonus_percent,
        1 => ctx.contest_second_blood_bonus_percent,
        2 => ctx.contest_third_blood_bonus_percent,
        _ => 0,
    }
    .clamp(0, 500);

    if bonus_percent == 0 || base_score <= 0 {
        return Ok(base_score);
    }

    let bonus = ((base_score as f64) * (bonus_percent as f64 / 100.0)).round() as i32;
    Ok(base_score.saturating_add(bonus.max(0)))
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

#[derive(Debug)]
enum JudgeDecision {
    Correct(String),
    Wrong(String),
    Invalid(String),
}

#[derive(Debug)]
struct ScriptVerifierConfig {
    program: String,
    args: Vec<String>,
    timeout_seconds: u64,
}

async fn run_script_verifier(
    ctx: &JudgeContextRow,
    contest_id: Uuid,
    challenge_id: Uuid,
    team_id: Uuid,
    submitted_flag: &str,
) -> JudgeDecision {
    let config = match parse_script_verifier_config(&ctx.metadata) {
        Ok(config) => config,
        Err(message) => return JudgeDecision::Invalid(message),
    };

    let mut command = Command::new(&config.program);
    command
        .args(&config.args)
        .env("SUBMITTED_FLAG", submitted_flag)
        .env("CONTEST_ID", contest_id.to_string())
        .env("CHALLENGE_ID", challenge_id.to_string())
        .env("TEAM_ID", team_id.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = match timeout(
        TokioDuration::from_secs(config.timeout_seconds),
        command.output(),
    )
    .await
    {
        Ok(Ok(output)) => output,
        Ok(Err(err)) => {
            return JudgeDecision::Invalid(format!(
                "failed to execute script verifier '{}': {}",
                config.program, err
            ));
        }
        Err(_) => {
            return JudgeDecision::Invalid(format!(
                "script verifier timed out after {} seconds",
                config.timeout_seconds
            ));
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    match output.status.code() {
        Some(0) => JudgeDecision::Correct(compact_message(
            &stdout,
            &stderr,
            "correct flag (verified by script)",
        )),
        Some(1) => JudgeDecision::Wrong(compact_message(&stderr, &stdout, "incorrect flag")),
        Some(code) => JudgeDecision::Invalid(compact_message(
            &stderr,
            &stdout,
            &format!("script verifier exited with status {}", code),
        )),
        None => JudgeDecision::Invalid("script verifier terminated by signal".to_string()),
    }
}

fn parse_script_verifier_config(metadata: &Value) -> Result<ScriptVerifierConfig, String> {
    let script_value = metadata.get("script_verifier").ok_or_else(|| {
        "challenge metadata.script_verifier is required for script mode".to_string()
    })?;

    if let Some(program) = script_value.as_str() {
        let trimmed = program.trim();
        if trimmed.is_empty() {
            return Err("metadata.script_verifier must not be empty".to_string());
        }

        return Ok(ScriptVerifierConfig {
            program: trimmed.to_string(),
            args: Vec::new(),
            timeout_seconds: 5,
        });
    }

    let obj = script_value
        .as_object()
        .ok_or_else(|| "metadata.script_verifier must be string or object".to_string())?;

    let program = obj
        .get("program")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "metadata.script_verifier.program is required".to_string())?
        .to_string();

    let args = obj
        .get("args")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let timeout_seconds = obj
        .get("timeout_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(5)
        .clamp(1, 30);

    Ok(ScriptVerifierConfig {
        program,
        args,
        timeout_seconds,
    })
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
