use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::HeaderMap,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tracing::warn;
use uuid::Uuid;

use crate::{
    auth::{self, AuthenticatedUser},
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Debug, Clone, Serialize)]
struct ScoreboardEntry {
    rank: usize,
    team_id: Uuid,
    team_name: String,
    score: i64,
    solved_count: i64,
    last_submit_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct ScoreboardPushPayload {
    event: &'static str,
    contest_id: Uuid,
    entries: Vec<ScoreboardEntry>,
}

#[derive(Debug, FromRow)]
struct ScoreboardRow {
    team_id: Uuid,
    team_name: String,
    score: i64,
    solved_count: i64,
    last_submit_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow)]
struct ContestAccessRow {
    visibility: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct ScoreboardWsAuthQuery {
    access_token: Option<String>,
    token: Option<String>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/contests/{contest_id}/scoreboard", get(get_scoreboard))
        .route("/contests/{contest_id}/scoreboard/ws", get(scoreboard_ws))
}

async fn get_scoreboard(
    State(state): State<Arc<AppState>>,
    Path(contest_id): Path<Uuid>,
    current_user: AuthenticatedUser,
) -> AppResult<Json<Vec<ScoreboardEntry>>> {
    ensure_scoreboard_access(state.as_ref(), contest_id, &current_user).await?;
    let entries = load_scoreboard_entries(state.as_ref(), contest_id).await?;
    Ok(Json(entries))
}

async fn scoreboard_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(contest_id): Path<Uuid>,
    headers: HeaderMap,
    Query(query): Query<ScoreboardWsAuthQuery>,
) -> AppResult<impl IntoResponse> {
    let current_user = resolve_ws_user(state.as_ref(), &headers, query)?;
    ensure_scoreboard_access(state.as_ref(), contest_id, &current_user).await?;

    Ok(ws.on_upgrade(move |socket| scoreboard_ws_loop(socket, state, contest_id, current_user)))
}

fn resolve_ws_user(
    state: &AppState,
    headers: &HeaderMap,
    query: ScoreboardWsAuthQuery,
) -> AppResult<AuthenticatedUser> {
    let token_from_header = auth::extract_bearer_token(headers).ok().map(str::to_string);
    let token = token_from_header
        .or(query.access_token)
        .or(query.token)
        .ok_or(AppError::Unauthorized)?;

    auth::decode_access_token(&token, &state.config.jwt_secret)
}

async fn scoreboard_ws_loop(
    mut socket: WebSocket,
    state: Arc<AppState>,
    contest_id: Uuid,
    current_user: AuthenticatedUser,
) {
    if send_scoreboard_snapshot(&mut socket, state.as_ref(), contest_id)
        .await
        .is_err()
    {
        return;
    }

    let channel = format!("scoreboard:contest:{}", contest_id);

    let mut pubsub = match state.redis_client.get_async_pubsub().await {
        Ok(pubsub) => pubsub,
        Err(err) => {
            warn!(
                contest_id = %contest_id,
                user_id = %current_user.user_id,
                error = %err,
                "failed to create redis pubsub connection"
            );
            let _ = socket
                .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                    code: axum::extract::ws::close_code::ERROR,
                    reason: "pubsub init failed".into(),
                })))
                .await;
            return;
        }
    };

    if let Err(err) = pubsub.subscribe(&channel).await {
        warn!(
            contest_id = %contest_id,
            user_id = %current_user.user_id,
            error = %err,
            "failed to subscribe scoreboard channel"
        );
        let _ = socket
            .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                code: axum::extract::ws::close_code::ERROR,
                reason: "pubsub subscribe failed".into(),
            })))
            .await;
        return;
    }

    let mut pubsub_stream = pubsub.on_message();

    loop {
        tokio::select! {
            inbound = socket.recv() => {
                match inbound {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(payload))) => {
                        if socket.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(_)) => {}
                    Some(Err(err)) => {
                        warn!(
                            contest_id = %contest_id,
                            user_id = %current_user.user_id,
                            error = %err,
                            "websocket receive error"
                        );
                        break;
                    }
                }
            }
            update = pubsub_stream.next() => {
                if update.is_none() {
                    break;
                }

                if send_scoreboard_snapshot(&mut socket, state.as_ref(), contest_id).await.is_err() {
                    break;
                }
            }
        }
    }
}

async fn send_scoreboard_snapshot(
    socket: &mut WebSocket,
    state: &AppState,
    contest_id: Uuid,
) -> Result<(), ()> {
    let entries = load_scoreboard_entries(state, contest_id)
        .await
        .map_err(|err| {
            warn!(contest_id = %contest_id, error = %err, "failed to build scoreboard snapshot");
        })?;

    let payload = serde_json::to_string(&ScoreboardPushPayload {
        event: "scoreboard_update",
        contest_id,
        entries,
    })
    .map_err(|err| {
        warn!(contest_id = %contest_id, error = %err, "failed to serialize scoreboard payload");
    })?;

    socket.send(Message::Text(payload.into())).await.map_err(|err| {
        warn!(contest_id = %contest_id, error = %err, "failed to send websocket scoreboard payload");
    })
}

async fn ensure_scoreboard_access(
    state: &AppState,
    contest_id: Uuid,
    current_user: &AuthenticatedUser,
) -> AppResult<()> {
    let contest = sqlx::query_as::<_, ContestAccessRow>(
        "SELECT visibility, status
         FROM contests
         WHERE id = $1
         LIMIT 1",
    )
    .bind(contest_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("contest not found".to_string()))?;

    let is_privileged = current_user.role == "admin" || current_user.role == "judge";

    if contest.visibility == "private" && !is_privileged {
        return Err(AppError::Forbidden);
    }

    if (contest.status == "draft" || contest.status == "archived") && !is_privileged {
        return Err(AppError::Forbidden);
    }

    Ok(())
}

async fn load_scoreboard_entries(
    state: &AppState,
    contest_id: Uuid,
) -> AppResult<Vec<ScoreboardEntry>> {
    let rows = sqlx::query_as::<_, ScoreboardRow>(
        "SELECT s.team_id,
                t.name AS team_name,
                COALESCE(SUM(s.score_awarded), 0) AS score,
                COUNT(*) FILTER (WHERE s.verdict = 'accepted' AND s.score_awarded > 0) AS solved_count,
                MAX(s.submitted_at) AS last_submit_at
         FROM submissions s
         JOIN teams t ON t.id = s.team_id
         WHERE s.contest_id = $1
         GROUP BY s.team_id, t.name
         ORDER BY score DESC, solved_count DESC, last_submit_at ASC",
    )
    .bind(contest_id)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    let mut scoreboard = Vec::with_capacity(rows.len());
    let mut last_rank_score: Option<(i64, i64, Option<DateTime<Utc>>)> = None;
    let mut current_rank = 0_usize;

    for (index, row) in rows.into_iter().enumerate() {
        let key = (row.score, row.solved_count, row.last_submit_at);
        if last_rank_score.as_ref() != Some(&key) {
            current_rank = index + 1;
            last_rank_score = Some(key);
        }

        scoreboard.push(ScoreboardEntry {
            rank: current_rank,
            team_id: row.team_id,
            team_name: row.team_name,
            score: row.score,
            solved_count: row.solved_count,
            last_submit_at: row.last_submit_at,
        });
    }

    Ok(scoreboard)
}
