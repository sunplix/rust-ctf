use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::HeaderMap,
    response::IntoResponse,
    routing::{get, patch, post},
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
    routes::contest_access::{ensure_user_contest_workspace_access, is_privileged_role},
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
    channels: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ScoreboardPushPayload {
    event: &'static str,
    contest_id: Uuid,
    channel_id: Option<Uuid>,
    entries: Vec<ScoreboardEntry>,
}

#[derive(Debug, Clone, Serialize)]
struct ScoreboardTimelineSnapshot {
    trigger_submission_id: i64,
    timestamp: DateTime<Utc>,
    entries: Vec<ScoreboardEntry>,
}

#[derive(Debug, Serialize)]
struct ScoreboardTimelineResponse {
    contest_id: Uuid,
    generated_at: DateTime<Utc>,
    snapshots: Vec<ScoreboardTimelineSnapshot>,
    latest_entries: Vec<ScoreboardEntry>,
}

#[derive(Debug, Clone, Serialize)]
struct ScoreboardRankingChallenge {
    challenge_id: Uuid,
    challenge_title: String,
    challenge_slug: String,
    marker: String,
    score_awarded: i32,
    submitted_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ScoreboardRankingCategory {
    category: String,
    solved_count: i64,
    challenges: Vec<ScoreboardRankingChallenge>,
}

#[derive(Debug, Serialize)]
struct ScoreboardRankingEntry {
    rank: usize,
    subject_id: Uuid,
    subject_name: String,
    total_score: i64,
    solved_count: i64,
    last_submit_at: Option<DateTime<Utc>>,
    channels: Vec<String>,
    categories: Vec<ScoreboardRankingCategory>,
}

#[derive(Debug, Serialize)]
struct ScoreboardCategoryChallengeItem {
    challenge_id: Uuid,
    challenge_title: String,
    challenge_slug: String,
}

#[derive(Debug, Serialize)]
struct ScoreboardCategoryItem {
    category: String,
    challenges: Vec<ScoreboardCategoryChallengeItem>,
}

#[derive(Debug, Serialize)]
struct ScoreboardRankingsResponse {
    contest_id: Uuid,
    channel_id: Option<Uuid>,
    generated_at: DateTime<Utc>,
    categories: Vec<ScoreboardCategoryItem>,
    team_rankings: Vec<ScoreboardRankingEntry>,
    player_rankings: Vec<ScoreboardRankingEntry>,
}

#[derive(Debug, Serialize)]
struct ScoreboardChannelItem {
    id: Uuid,
    name: String,
    description: String,
    is_active: bool,
    member_count: i64,
    my_joined: bool,
    invite_code: Option<String>,
}

#[derive(Debug, Serialize)]
struct ScoreboardChannelMembershipItem {
    id: Uuid,
    name: String,
    description: String,
    joined_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ScoreboardMyChannelResponse {
    contest_id: Uuid,
    channel: Option<ScoreboardChannelMembershipItem>,
}

#[derive(Debug, Serialize)]
struct ScoreboardChannelActionResponse {
    contest_id: Uuid,
    message: String,
    channel: Option<ScoreboardChannelMembershipItem>,
}

#[derive(Debug, FromRow)]
struct ScoreboardRow {
    team_id: Uuid,
    team_name: String,
    score: i64,
    solved_count: i64,
    last_submit_at: Option<DateTime<Utc>>,
    channels: Vec<String>,
}

#[derive(Debug, FromRow)]
struct ScoreboardTimelineEventRow {
    submission_id: i64,
    team_id: Uuid,
    team_name: String,
    score_awarded: i32,
    submitted_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct TeamRankingSolveEventRow {
    _submission_id: i64,
    team_id: Uuid,
    team_name: String,
    challenge_id: Uuid,
    challenge_title: String,
    challenge_slug: String,
    challenge_category: String,
    score_awarded: i32,
    submitted_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct PlayerRankingSolveEventRow {
    _submission_id: i64,
    user_id: Uuid,
    username: String,
    challenge_id: Uuid,
    challenge_title: String,
    challenge_slug: String,
    challenge_category: String,
    score_awarded: i32,
    submitted_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct ScoreboardChannelRow {
    id: Uuid,
    name: String,
    description: String,
    invite_code: String,
    is_active: bool,
    member_count: i64,
    my_joined: bool,
}

#[derive(Debug, FromRow)]
struct ScoreboardChannelMembershipRow {
    channel_id: Uuid,
    channel_name: String,
    channel_description: String,
    joined_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct ScoreboardChannelResolveRow {
    id: Uuid,
    name: String,
    description: String,
}

#[derive(Debug, FromRow)]
struct ContestChallengeCatalogRow {
    challenge_id: Uuid,
    challenge_title: String,
    challenge_slug: String,
    challenge_category: String,
}

#[derive(Debug, FromRow)]
struct TeamChannelsRow {
    team_id: Uuid,
    channels: Vec<String>,
}

#[derive(Debug, FromRow)]
struct UserChannelsRow {
    user_id: Uuid,
    channels: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ScoreboardWsAuthQuery {
    access_token: Option<String>,
    token: Option<String>,
    channel_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct ScoreboardScopeQuery {
    channel_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct ScoreboardTimelineQuery {
    max_snapshots: Option<i64>,
    top_n: Option<i64>,
    channel_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct JoinScoreboardChannelRequest {
    invite_code: String,
}

#[derive(Debug, Deserialize)]
struct CreateScoreboardChannelRequest {
    name: String,
    description: Option<String>,
    is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct UpdateScoreboardChannelRequest {
    name: Option<String>,
    description: Option<String>,
    is_active: Option<bool>,
    regenerate_invite_code: Option<bool>,
}

#[derive(Debug, Clone)]
struct RankingSubjectState {
    subject_name: String,
    total_score: i64,
    solved_count: i64,
    last_submit_at: Option<DateTime<Utc>>,
    categories: HashMap<String, Vec<ScoreboardRankingChallenge>>,
}

#[derive(Debug, Clone)]
struct TimelineTeamState {
    team_name: String,
    score: i64,
    solved_count: i64,
    last_submit_at: Option<DateTime<Utc>>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/contests/{contest_id}/scoreboard/channels",
            get(list_scoreboard_channels).post(create_scoreboard_channel),
        )
        .route(
            "/contests/{contest_id}/scoreboard/channels/{channel_id}",
            patch(update_scoreboard_channel),
        )
        .route(
            "/contests/{contest_id}/scoreboard/channels/me",
            get(get_my_scoreboard_channel),
        )
        .route(
            "/contests/{contest_id}/scoreboard/channels/join",
            post(join_scoreboard_channel),
        )
        .route(
            "/contests/{contest_id}/scoreboard/channels/leave",
            post(leave_scoreboard_channel),
        )
        .route("/contests/{contest_id}/scoreboard", get(get_scoreboard))
        .route(
            "/contests/{contest_id}/scoreboard/rankings",
            get(get_scoreboard_rankings),
        )
        .route(
            "/contests/{contest_id}/scoreboard/timeline",
            get(get_scoreboard_timeline),
        )
        .route("/contests/{contest_id}/scoreboard/ws", get(scoreboard_ws))
}

async fn list_scoreboard_channels(
    State(state): State<Arc<AppState>>,
    Path(contest_id): Path<Uuid>,
    current_user: AuthenticatedUser,
) -> AppResult<Json<Vec<ScoreboardChannelItem>>> {
    ensure_scoreboard_access(state.as_ref(), contest_id, &current_user).await?;

    let allow_invite_code = is_privileged_role(&current_user.role);

    let rows = sqlx::query_as::<_, ScoreboardChannelRow>(
        "SELECT c.id,
                c.name,
                c.description,
                c.invite_code,
                c.is_active,
                COUNT(cm.user_id)::bigint AS member_count,
                COALESCE(BOOL_OR(cm.user_id = $2), FALSE) AS my_joined
         FROM contest_scoreboard_channels c
         LEFT JOIN contest_scoreboard_channel_members cm
           ON cm.contest_id = c.contest_id
          AND cm.channel_id = c.id
         WHERE c.contest_id = $1
           AND (
               $3::boolean
               OR c.is_active = TRUE
               OR EXISTS (
                   SELECT 1
                   FROM contest_scoreboard_channel_members mine
                   WHERE mine.contest_id = c.contest_id
                     AND mine.channel_id = c.id
                     AND mine.user_id = $2
               )
           )
         GROUP BY c.id
         ORDER BY c.created_at ASC, c.name ASC",
    )
    .bind(contest_id)
    .bind(current_user.user_id)
    .bind(allow_invite_code)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(
        rows.into_iter()
            .map(|row| ScoreboardChannelItem {
                id: row.id,
                name: row.name,
                description: row.description,
                is_active: row.is_active,
                member_count: row.member_count,
                my_joined: row.my_joined,
                invite_code: if allow_invite_code {
                    Some(row.invite_code)
                } else {
                    None
                },
            })
            .collect(),
    ))
}

async fn get_my_scoreboard_channel(
    State(state): State<Arc<AppState>>,
    Path(contest_id): Path<Uuid>,
    current_user: AuthenticatedUser,
) -> AppResult<Json<ScoreboardMyChannelResponse>> {
    ensure_scoreboard_access(state.as_ref(), contest_id, &current_user).await?;

    let row = sqlx::query_as::<_, ScoreboardChannelMembershipRow>(
        "SELECT c.id AS channel_id,
                c.name AS channel_name,
                c.description AS channel_description,
                m.joined_at
         FROM contest_scoreboard_channel_members m
         JOIN contest_scoreboard_channels c
           ON c.id = m.channel_id
          AND c.contest_id = m.contest_id
         WHERE m.contest_id = $1
           AND m.user_id = $2
         LIMIT 1",
    )
    .bind(contest_id)
    .bind(current_user.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(ScoreboardMyChannelResponse {
        contest_id,
        channel: row.map(|item| ScoreboardChannelMembershipItem {
            id: item.channel_id,
            name: item.channel_name,
            description: item.channel_description,
            joined_at: item.joined_at,
        }),
    }))
}

async fn join_scoreboard_channel(
    State(state): State<Arc<AppState>>,
    Path(contest_id): Path<Uuid>,
    current_user: AuthenticatedUser,
    Json(req): Json<JoinScoreboardChannelRequest>,
) -> AppResult<Json<ScoreboardChannelActionResponse>> {
    ensure_scoreboard_access(state.as_ref(), contest_id, &current_user).await?;
    let invite_code = normalize_invite_code(&req.invite_code)?;

    let channel = sqlx::query_as::<_, ScoreboardChannelResolveRow>(
        "SELECT id, name, description
         FROM contest_scoreboard_channels
         WHERE contest_id = $1
           AND LOWER(invite_code) = LOWER($2)
           AND is_active = TRUE
         LIMIT 1",
    )
    .bind(contest_id)
    .bind(&invite_code)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or_else(|| AppError::BadRequest("invalid or inactive invite code".to_string()))?;

    let joined_at = sqlx::query_scalar::<_, DateTime<Utc>>(
        "INSERT INTO contest_scoreboard_channel_members (
             contest_id,
             channel_id,
             user_id,
             joined_by_invite_code,
             joined_at
         )
         VALUES ($1, $2, $3, $4, NOW())
         ON CONFLICT (contest_id, user_id)
         DO UPDATE
         SET channel_id = EXCLUDED.channel_id,
             joined_by_invite_code = EXCLUDED.joined_by_invite_code,
             joined_at = NOW()
         RETURNING joined_at",
    )
    .bind(contest_id)
    .bind(channel.id)
    .bind(current_user.user_id)
    .bind(invite_code)
    .fetch_one(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(ScoreboardChannelActionResponse {
        contest_id,
        message: "joined scoreboard channel".to_string(),
        channel: Some(ScoreboardChannelMembershipItem {
            id: channel.id,
            name: channel.name,
            description: channel.description,
            joined_at,
        }),
    }))
}

async fn leave_scoreboard_channel(
    State(state): State<Arc<AppState>>,
    Path(contest_id): Path<Uuid>,
    current_user: AuthenticatedUser,
) -> AppResult<Json<ScoreboardChannelActionResponse>> {
    ensure_scoreboard_access(state.as_ref(), contest_id, &current_user).await?;

    let affected = sqlx::query(
        "DELETE FROM contest_scoreboard_channel_members
         WHERE contest_id = $1
           AND user_id = $2",
    )
    .bind(contest_id)
    .bind(current_user.user_id)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?
    .rows_affected();

    Ok(Json(ScoreboardChannelActionResponse {
        contest_id,
        message: if affected > 0 {
            "left scoreboard channel".to_string()
        } else {
            "no scoreboard channel to leave".to_string()
        },
        channel: None,
    }))
}

async fn create_scoreboard_channel(
    State(state): State<Arc<AppState>>,
    Path(contest_id): Path<Uuid>,
    current_user: AuthenticatedUser,
    Json(req): Json<CreateScoreboardChannelRequest>,
) -> AppResult<Json<ScoreboardChannelItem>> {
    ensure_scoreboard_access(state.as_ref(), contest_id, &current_user).await?;
    if !is_privileged_role(&current_user.role) {
        return Err(AppError::Forbidden);
    }

    let name = trim_required(&req.name, "name")?;
    if name.chars().count() > 80 {
        return Err(AppError::BadRequest(
            "channel name must be at most 80 characters".to_string(),
        ));
    }

    let description = req.description.unwrap_or_default().trim().to_string();
    if description.chars().count() > 500 {
        return Err(AppError::BadRequest(
            "channel description must be at most 500 characters".to_string(),
        ));
    }

    let is_active = req.is_active.unwrap_or(true);

    let name_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
             SELECT 1
             FROM contest_scoreboard_channels
             WHERE contest_id = $1
               AND LOWER(name) = LOWER($2)
         )",
    )
    .bind(contest_id)
    .bind(&name)
    .fetch_one(&state.db)
    .await
    .map_err(AppError::internal)?;
    if name_exists {
        return Err(AppError::Conflict(
            "channel name already exists".to_string(),
        ));
    }

    let mut created: Option<ScoreboardChannelRow> = None;
    for _ in 0..16 {
        let invite_code = generate_invite_code();
        let result = sqlx::query_as::<_, ScoreboardChannelRow>(
            "INSERT INTO contest_scoreboard_channels (
                 contest_id,
                 name,
                 invite_code,
                 description,
                 is_active,
                 created_by
             )
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id,
                       name,
                       description,
                       invite_code,
                       is_active,
                       0::bigint AS member_count,
                       FALSE AS my_joined",
        )
        .bind(contest_id)
        .bind(&name)
        .bind(&invite_code)
        .bind(&description)
        .bind(is_active)
        .bind(current_user.user_id)
        .fetch_one(&state.db)
        .await;

        match result {
            Ok(row) => {
                created = Some(row);
                break;
            }
            Err(err) if is_unique_violation(&err) => continue,
            Err(err) => return Err(AppError::internal(err)),
        }
    }

    let created = created
        .ok_or_else(|| AppError::Conflict("failed to generate unique invite code".to_string()))?;

    Ok(Json(ScoreboardChannelItem {
        id: created.id,
        name: created.name,
        description: created.description,
        is_active: created.is_active,
        member_count: 0,
        my_joined: false,
        invite_code: Some(created.invite_code),
    }))
}

async fn update_scoreboard_channel(
    State(state): State<Arc<AppState>>,
    Path((contest_id, channel_id)): Path<(Uuid, Uuid)>,
    current_user: AuthenticatedUser,
    Json(req): Json<UpdateScoreboardChannelRequest>,
) -> AppResult<Json<ScoreboardChannelItem>> {
    ensure_scoreboard_access(state.as_ref(), contest_id, &current_user).await?;
    if !is_privileged_role(&current_user.role) {
        return Err(AppError::Forbidden);
    }

    let name = req
        .name
        .as_deref()
        .map(|value| trim_required(value, "name"))
        .transpose()?;
    if let Some(value) = &name {
        if value.chars().count() > 80 {
            return Err(AppError::BadRequest(
                "channel name must be at most 80 characters".to_string(),
            ));
        }
    }

    let description = req.description.map(|value| value.trim().to_string());
    if let Some(value) = &description {
        if value.chars().count() > 500 {
            return Err(AppError::BadRequest(
                "channel description must be at most 500 characters".to_string(),
            ));
        }
    }

    let regenerate_invite_code = req.regenerate_invite_code.unwrap_or(false);

    if name.is_none() && description.is_none() && req.is_active.is_none() && !regenerate_invite_code
    {
        return Err(AppError::BadRequest(
            "at least one field is required for update".to_string(),
        ));
    }

    let attempts = if regenerate_invite_code { 16 } else { 1 };
    let mut updated: Option<ScoreboardChannelRow> = None;

    for _ in 0..attempts {
        let invite_code = if regenerate_invite_code {
            Some(generate_invite_code())
        } else {
            None
        };

        let result = sqlx::query_as::<_, ScoreboardChannelRow>(
            "WITH updated AS (
                UPDATE contest_scoreboard_channels
                SET name = COALESCE($3, name),
                    invite_code = COALESCE($4, invite_code),
                    description = COALESCE($5, description),
                    is_active = COALESCE($6, is_active),
                    updated_at = NOW()
                WHERE contest_id = $1
                  AND id = $2
                RETURNING id,
                          contest_id,
                          name,
                          invite_code,
                          description,
                          is_active
             )
             SELECT u.id,
                    u.name,
                    u.description,
                    u.invite_code,
                    u.is_active,
                    COUNT(cm.user_id)::bigint AS member_count,
                    COALESCE(BOOL_OR(cm.user_id = $7), FALSE) AS my_joined
             FROM updated u
             LEFT JOIN contest_scoreboard_channel_members cm
               ON cm.contest_id = u.contest_id
              AND cm.channel_id = u.id
             GROUP BY u.id, u.name, u.description, u.invite_code, u.is_active",
        )
        .bind(contest_id)
        .bind(channel_id)
        .bind(name.as_deref())
        .bind(invite_code.as_deref())
        .bind(description.as_deref())
        .bind(req.is_active)
        .bind(current_user.user_id)
        .fetch_optional(&state.db)
        .await;

        match result {
            Ok(Some(row)) => {
                updated = Some(row);
                break;
            }
            Ok(None) => {
                return Err(AppError::BadRequest(
                    "scoreboard channel not found".to_string(),
                ));
            }
            Err(err) if regenerate_invite_code && is_unique_violation(&err) => continue,
            Err(err) if is_unique_violation(&err) => {
                return Err(AppError::Conflict(
                    "channel name or invite code already exists".to_string(),
                ));
            }
            Err(err) => return Err(AppError::internal(err)),
        }
    }

    let updated = updated
        .ok_or_else(|| AppError::Conflict("failed to generate unique invite code".to_string()))?;

    Ok(Json(ScoreboardChannelItem {
        id: updated.id,
        name: updated.name,
        description: updated.description,
        is_active: updated.is_active,
        member_count: updated.member_count,
        my_joined: updated.my_joined,
        invite_code: Some(updated.invite_code),
    }))
}

async fn get_scoreboard(
    State(state): State<Arc<AppState>>,
    Path(contest_id): Path<Uuid>,
    Query(query): Query<ScoreboardScopeQuery>,
    current_user: AuthenticatedUser,
) -> AppResult<Json<Vec<ScoreboardEntry>>> {
    ensure_scoreboard_access(state.as_ref(), contest_id, &current_user).await?;
    let channel_id = resolve_scoreboard_scope(state.as_ref(), contest_id, query.channel_id).await?;
    let entries = load_scoreboard_entries(state.as_ref(), contest_id, channel_id).await?;
    Ok(Json(entries))
}

async fn get_scoreboard_rankings(
    State(state): State<Arc<AppState>>,
    Path(contest_id): Path<Uuid>,
    Query(query): Query<ScoreboardScopeQuery>,
    current_user: AuthenticatedUser,
) -> AppResult<Json<ScoreboardRankingsResponse>> {
    ensure_scoreboard_access(state.as_ref(), contest_id, &current_user).await?;
    let channel_id = resolve_scoreboard_scope(state.as_ref(), contest_id, query.channel_id).await?;

    let (categories, team_rankings, player_rankings) =
        load_scoreboard_rankings(state.as_ref(), contest_id, channel_id).await?;

    Ok(Json(ScoreboardRankingsResponse {
        contest_id,
        channel_id,
        generated_at: Utc::now(),
        categories,
        team_rankings,
        player_rankings,
    }))
}

async fn get_scoreboard_timeline(
    State(state): State<Arc<AppState>>,
    Path(contest_id): Path<Uuid>,
    Query(query): Query<ScoreboardTimelineQuery>,
    current_user: AuthenticatedUser,
) -> AppResult<Json<ScoreboardTimelineResponse>> {
    ensure_scoreboard_access(state.as_ref(), contest_id, &current_user).await?;
    let channel_id = resolve_scoreboard_scope(state.as_ref(), contest_id, query.channel_id).await?;

    let max_snapshots = query.max_snapshots.unwrap_or(800).clamp(1, 5000) as usize;
    let top_n = query.top_n.unwrap_or(12).clamp(1, 200) as usize;

    let (snapshots, latest_entries) =
        load_scoreboard_timeline(state.as_ref(), contest_id, channel_id, max_snapshots, top_n)
            .await?;

    Ok(Json(ScoreboardTimelineResponse {
        contest_id,
        generated_at: Utc::now(),
        snapshots,
        latest_entries,
    }))
}

async fn scoreboard_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(contest_id): Path<Uuid>,
    headers: HeaderMap,
    Query(query): Query<ScoreboardWsAuthQuery>,
) -> AppResult<impl IntoResponse> {
    let current_user = resolve_ws_user(state.as_ref(), &headers, &query)?;
    ensure_scoreboard_access(state.as_ref(), contest_id, &current_user).await?;
    let channel_id = resolve_scoreboard_scope(state.as_ref(), contest_id, query.channel_id).await?;

    Ok(ws.on_upgrade(move |socket| {
        scoreboard_ws_loop(socket, state, contest_id, channel_id, current_user)
    }))
}

fn resolve_ws_user(
    state: &AppState,
    headers: &HeaderMap,
    query: &ScoreboardWsAuthQuery,
) -> AppResult<AuthenticatedUser> {
    let token_from_header = auth::extract_bearer_token(headers).ok().map(str::to_string);
    let token = token_from_header
        .or(query.access_token.clone())
        .or(query.token.clone())
        .ok_or(AppError::Unauthorized)?;

    auth::decode_access_token(&token, &state.config.jwt_secret)
}

async fn scoreboard_ws_loop(
    mut socket: WebSocket,
    state: Arc<AppState>,
    contest_id: Uuid,
    channel_id: Option<Uuid>,
    current_user: AuthenticatedUser,
) {
    if send_scoreboard_snapshot(&mut socket, state.as_ref(), contest_id, channel_id)
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

                if send_scoreboard_snapshot(&mut socket, state.as_ref(), contest_id, channel_id).await.is_err() {
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
    channel_id: Option<Uuid>,
) -> Result<(), ()> {
    let entries = load_scoreboard_entries(state, contest_id, channel_id)
        .await
        .map_err(|err| {
            warn!(contest_id = %contest_id, error = %err, "failed to build scoreboard snapshot");
        })?;

    let payload = serde_json::to_string(&ScoreboardPushPayload {
        event: "scoreboard_update",
        contest_id,
        channel_id,
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
    ensure_user_contest_workspace_access(state, contest_id, current_user).await?;
    Ok(())
}

async fn resolve_scoreboard_scope(
    state: &AppState,
    contest_id: Uuid,
    channel_id: Option<Uuid>,
) -> AppResult<Option<Uuid>> {
    let Some(channel_id) = channel_id else {
        return Ok(None);
    };

    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
             SELECT 1
             FROM contest_scoreboard_channels
             WHERE contest_id = $1
               AND id = $2
         )",
    )
    .bind(contest_id)
    .bind(channel_id)
    .fetch_one(&state.db)
    .await
    .map_err(AppError::internal)?;

    if !exists {
        return Err(AppError::BadRequest(
            "scoreboard channel not found for this contest".to_string(),
        ));
    }

    Ok(Some(channel_id))
}

fn trim_required(value: &str, field: &str) -> AppResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest(format!("{field} is required")));
    }
    Ok(trimmed.to_string())
}

fn normalize_invite_code(value: &str) -> AppResult<String> {
    let raw = value.trim();
    if raw.is_empty() {
        return Err(AppError::BadRequest("invite_code is required".to_string()));
    }
    if raw.chars().count() < 4 || raw.chars().count() > 48 {
        return Err(AppError::BadRequest(
            "invite_code length must be within 4..48".to_string(),
        ));
    }
    if !raw
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(AppError::BadRequest(
            "invite_code must contain only letters, numbers, '-' or '_'".to_string(),
        ));
    }
    Ok(raw.to_ascii_uppercase())
}

fn generate_invite_code() -> String {
    let raw = Uuid::new_v4().simple().to_string().to_ascii_uppercase();
    format!("CH{}", &raw[..10])
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(
        error,
        sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some("23505")
    )
}

async fn load_scoreboard_entries(
    state: &AppState,
    contest_id: Uuid,
    channel_id: Option<Uuid>,
) -> AppResult<Vec<ScoreboardEntry>> {
    let rows = sqlx::query_as::<_, ScoreboardRow>(
        "SELECT s.team_id,
                t.name AS team_name,
                COALESCE(SUM(s.score_awarded), 0) AS score,
                COUNT(*) FILTER (WHERE s.verdict = 'accepted' AND s.score_awarded > 0) AS solved_count,
                MAX(s.submitted_at) AS last_submit_at,
                COALESCE(tc.channels, ARRAY[]::text[]) AS channels
         FROM submissions s
         JOIN teams t ON t.id = s.team_id
         LEFT JOIN LATERAL (
             SELECT ARRAY_AGG(DISTINCT c.name ORDER BY c.name) AS channels
             FROM team_members tm
             JOIN contest_scoreboard_channel_members cm
               ON cm.user_id = tm.user_id
              AND cm.contest_id = $1
             JOIN contest_scoreboard_channels c
               ON c.id = cm.channel_id
              AND c.contest_id = cm.contest_id
             WHERE tm.team_id = s.team_id
         ) tc ON TRUE
         WHERE s.contest_id = $1
           AND (
               $2::uuid IS NULL
               OR EXISTS (
                   SELECT 1
                   FROM team_members tm
                   JOIN contest_scoreboard_channel_members cm
                     ON cm.user_id = tm.user_id
                    AND cm.contest_id = $1
                   WHERE tm.team_id = s.team_id
                     AND cm.channel_id = $2
               )
           )
         GROUP BY s.team_id, t.name, tc.channels
         ORDER BY score DESC, solved_count DESC, last_submit_at ASC",
    )
    .bind(contest_id)
    .bind(channel_id)
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
            channels: row.channels,
        });
    }

    Ok(scoreboard)
}

async fn load_scoreboard_timeline(
    state: &AppState,
    contest_id: Uuid,
    channel_id: Option<Uuid>,
    max_snapshots: usize,
    top_n: usize,
) -> AppResult<(Vec<ScoreboardTimelineSnapshot>, Vec<ScoreboardEntry>)> {
    let latest_entries = load_scoreboard_entries(state, contest_id, channel_id).await?;

    let events = sqlx::query_as::<_, ScoreboardTimelineEventRow>(
        "SELECT s.id AS submission_id,
                s.team_id,
                t.name AS team_name,
                s.score_awarded,
                s.submitted_at
         FROM submissions s
         JOIN teams t ON t.id = s.team_id
         WHERE s.contest_id = $1
           AND (
               $2::uuid IS NULL
               OR EXISTS (
                   SELECT 1
                   FROM team_members tm
                   JOIN contest_scoreboard_channel_members cm
                     ON cm.user_id = tm.user_id
                    AND cm.contest_id = $1
                   WHERE tm.team_id = s.team_id
                     AND cm.channel_id = $2
               )
           )
           AND s.verdict = 'accepted'
           AND s.score_awarded > 0
         ORDER BY s.submitted_at ASC, s.id ASC",
    )
    .bind(contest_id)
    .bind(channel_id)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    if events.is_empty() {
        return Ok((Vec::new(), latest_entries));
    }

    let mut team_states: HashMap<Uuid, TimelineTeamState> = HashMap::new();
    let mut snapshots: Vec<ScoreboardTimelineSnapshot> = Vec::with_capacity(events.len());

    for event in events {
        let team_state = team_states
            .entry(event.team_id)
            .or_insert_with(|| TimelineTeamState {
                team_name: event.team_name.clone(),
                score: 0,
                solved_count: 0,
                last_submit_at: None,
            });

        team_state.team_name = event.team_name.clone();
        team_state.score += event.score_awarded as i64;
        team_state.solved_count += 1;
        team_state.last_submit_at = Some(event.submitted_at);

        let mut entries = build_ranked_entries_from_states(&team_states);
        if entries.len() > top_n {
            entries.truncate(top_n);
        }

        snapshots.push(ScoreboardTimelineSnapshot {
            trigger_submission_id: event.submission_id,
            timestamp: event.submitted_at,
            entries,
        });
    }

    let snapshots = downsample_timeline_snapshots(snapshots, max_snapshots);
    Ok((snapshots, latest_entries))
}

fn build_ranked_entries_from_states(
    team_states: &HashMap<Uuid, TimelineTeamState>,
) -> Vec<ScoreboardEntry> {
    let mut rows: Vec<(Uuid, TimelineTeamState)> = team_states
        .iter()
        .map(|(team_id, state)| (*team_id, state.clone()))
        .collect();

    rows.sort_by(|lhs, rhs| {
        rhs.1
            .score
            .cmp(&lhs.1.score)
            .then_with(|| rhs.1.solved_count.cmp(&lhs.1.solved_count))
            .then_with(|| lhs.1.last_submit_at.cmp(&rhs.1.last_submit_at))
            .then_with(|| lhs.1.team_name.cmp(&rhs.1.team_name))
    });

    let mut ranked: Vec<ScoreboardEntry> = Vec::with_capacity(rows.len());
    let mut last_rank_key: Option<(i64, i64, Option<DateTime<Utc>>)> = None;
    let mut current_rank = 0_usize;

    for (index, (team_id, state)) in rows.into_iter().enumerate() {
        let key = (state.score, state.solved_count, state.last_submit_at);
        if last_rank_key.as_ref() != Some(&key) {
            current_rank = index + 1;
            last_rank_key = Some(key);
        }

        ranked.push(ScoreboardEntry {
            rank: current_rank,
            team_id,
            team_name: state.team_name,
            score: state.score,
            solved_count: state.solved_count,
            last_submit_at: state.last_submit_at,
            channels: Vec::new(),
        });
    }

    ranked
}

fn downsample_timeline_snapshots(
    snapshots: Vec<ScoreboardTimelineSnapshot>,
    max_snapshots: usize,
) -> Vec<ScoreboardTimelineSnapshot> {
    if snapshots.len() <= max_snapshots {
        return snapshots;
    }

    if max_snapshots <= 1 {
        return snapshots
            .last()
            .cloned()
            .map(|item| vec![item])
            .unwrap_or_default();
    }

    if max_snapshots == 2 {
        return vec![snapshots[0].clone(), snapshots[snapshots.len() - 1].clone()];
    }

    let mut reduced = Vec::with_capacity(max_snapshots);
    reduced.push(snapshots[0].clone());

    let middle_slots = max_snapshots - 2;
    let available_middle = snapshots.len() - 2;
    for i in 0..middle_slots {
        let numerator = (i + 1) * available_middle;
        let denominator = middle_slots + 1;
        let idx = 1 + numerator / denominator;
        reduced.push(snapshots[idx].clone());
    }

    reduced.push(snapshots[snapshots.len() - 1].clone());
    reduced
}

async fn load_scoreboard_rankings(
    state: &AppState,
    contest_id: Uuid,
    channel_id: Option<Uuid>,
) -> AppResult<(
    Vec<ScoreboardCategoryItem>,
    Vec<ScoreboardRankingEntry>,
    Vec<ScoreboardRankingEntry>,
)> {
    let catalog_rows = sqlx::query_as::<_, ContestChallengeCatalogRow>(
        "SELECT c.id AS challenge_id,
                c.title AS challenge_title,
                c.slug AS challenge_slug,
                c.category AS challenge_category
         FROM contest_challenges cc
         JOIN challenges c ON c.id = cc.challenge_id
         WHERE cc.contest_id = $1
         ORDER BY c.category ASC, cc.sort_order ASC, c.title ASC",
    )
    .bind(contest_id)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    let mut category_order: Vec<String> = Vec::new();
    let mut category_map: HashMap<String, Vec<ScoreboardCategoryChallengeItem>> = HashMap::new();
    for row in catalog_rows {
        if !category_map.contains_key(&row.challenge_category) {
            category_order.push(row.challenge_category.clone());
        }
        category_map
            .entry(row.challenge_category)
            .or_default()
            .push(ScoreboardCategoryChallengeItem {
                challenge_id: row.challenge_id,
                challenge_title: row.challenge_title,
                challenge_slug: row.challenge_slug,
            });
    }

    let mut categories = Vec::with_capacity(category_order.len());
    for category in &category_order {
        let challenges = category_map.remove(category).unwrap_or_default();
        categories.push(ScoreboardCategoryItem {
            category: category.clone(),
            challenges,
        });
    }

    let team_events = load_team_ranking_events(state, contest_id, channel_id).await?;
    let player_events = load_player_ranking_events(state, contest_id, channel_id).await?;

    let mut team_states: HashMap<Uuid, RankingSubjectState> = HashMap::new();
    let mut player_states: HashMap<Uuid, RankingSubjectState> = HashMap::new();
    let mut team_seen: HashSet<(Uuid, Uuid)> = HashSet::new();
    let mut player_seen: HashSet<(Uuid, Uuid)> = HashSet::new();
    let mut team_blood_order: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    let mut player_blood_order: HashMap<Uuid, Vec<Uuid>> = HashMap::new();

    for row in team_events {
        if team_seen.insert((row.team_id, row.challenge_id)) {
            let order =
                marker_order_for_subject(&mut team_blood_order, row.challenge_id, row.team_id);
            let solve = ScoreboardRankingChallenge {
                challenge_id: row.challenge_id,
                challenge_title: row.challenge_title.clone(),
                challenge_slug: row.challenge_slug.clone(),
                marker: marker_name(order).to_string(),
                score_awarded: row.score_awarded,
                submitted_at: row.submitted_at,
            };
            push_subject_solve(
                &mut team_states,
                row.team_id,
                row.team_name.clone(),
                row.challenge_category.clone(),
                solve,
                row.score_awarded,
                row.submitted_at,
            );
        }
    }

    for row in player_events {
        if player_seen.insert((row.user_id, row.challenge_id)) {
            let order =
                marker_order_for_subject(&mut player_blood_order, row.challenge_id, row.user_id);
            let solve = ScoreboardRankingChallenge {
                challenge_id: row.challenge_id,
                challenge_title: row.challenge_title,
                challenge_slug: row.challenge_slug,
                marker: marker_name(order).to_string(),
                score_awarded: row.score_awarded,
                submitted_at: row.submitted_at,
            };
            push_subject_solve(
                &mut player_states,
                row.user_id,
                row.username,
                row.challenge_category,
                solve,
                row.score_awarded,
                row.submitted_at,
            );
        }
    }

    let team_subject_ids: Vec<Uuid> = team_states.keys().copied().collect();
    let player_subject_ids: Vec<Uuid> = player_states.keys().copied().collect();
    let team_channel_lookup =
        load_team_channels_lookup(state, contest_id, &team_subject_ids).await?;
    let player_channel_lookup =
        load_user_channels_lookup(state, contest_id, &player_subject_ids).await?;

    let team_rankings = build_ranking_entries(team_states, &category_order, &team_channel_lookup);
    let player_rankings =
        build_ranking_entries(player_states, &category_order, &player_channel_lookup);

    Ok((categories, team_rankings, player_rankings))
}

async fn load_team_ranking_events(
    state: &AppState,
    contest_id: Uuid,
    channel_id: Option<Uuid>,
) -> AppResult<Vec<TeamRankingSolveEventRow>> {
    sqlx::query_as::<_, TeamRankingSolveEventRow>(
        "SELECT s.id AS _submission_id,
                s.team_id,
                t.name AS team_name,
                c.id AS challenge_id,
                c.title AS challenge_title,
                c.slug AS challenge_slug,
                c.category AS challenge_category,
                s.score_awarded,
                s.submitted_at
         FROM submissions s
         JOIN teams t ON t.id = s.team_id
         JOIN challenges c ON c.id = s.challenge_id
         WHERE s.contest_id = $1
           AND (
               $2::uuid IS NULL
               OR EXISTS (
                   SELECT 1
                   FROM team_members tm
                   JOIN contest_scoreboard_channel_members cm
                     ON cm.user_id = tm.user_id
                    AND cm.contest_id = $1
                   WHERE tm.team_id = s.team_id
                     AND cm.channel_id = $2
               )
           )
           AND s.verdict = 'accepted'
           AND s.score_awarded > 0
         ORDER BY s.submitted_at ASC, s.id ASC",
    )
    .bind(contest_id)
    .bind(channel_id)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)
}

async fn load_player_ranking_events(
    state: &AppState,
    contest_id: Uuid,
    channel_id: Option<Uuid>,
) -> AppResult<Vec<PlayerRankingSolveEventRow>> {
    sqlx::query_as::<_, PlayerRankingSolveEventRow>(
        "SELECT s.id AS _submission_id,
                s.user_id,
                u.username,
                c.id AS challenge_id,
                c.title AS challenge_title,
                c.slug AS challenge_slug,
                c.category AS challenge_category,
                s.score_awarded,
                s.submitted_at
         FROM submissions s
         JOIN users u ON u.id = s.user_id
         JOIN challenges c ON c.id = s.challenge_id
         WHERE s.contest_id = $1
           AND (
               $2::uuid IS NULL
               OR EXISTS (
                   SELECT 1
                   FROM contest_scoreboard_channel_members cm
                   WHERE cm.contest_id = $1
                     AND cm.channel_id = $2
                     AND cm.user_id = s.user_id
               )
           )
           AND s.verdict = 'accepted'
           AND s.score_awarded > 0
         ORDER BY s.submitted_at ASC, s.id ASC",
    )
    .bind(contest_id)
    .bind(channel_id)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)
}

fn marker_order_for_subject(
    order_map: &mut HashMap<Uuid, Vec<Uuid>>,
    challenge_id: Uuid,
    subject_id: Uuid,
) -> usize {
    let subjects = order_map.entry(challenge_id).or_default();
    if let Some(index) = subjects.iter().position(|item| *item == subject_id) {
        index
    } else {
        subjects.push(subject_id);
        subjects.len() - 1
    }
}

fn marker_name(order: usize) -> &'static str {
    match order {
        0 => "first_blood",
        1 => "second_blood",
        2 => "third_blood",
        _ => "solved",
    }
}

fn push_subject_solve(
    states: &mut HashMap<Uuid, RankingSubjectState>,
    subject_id: Uuid,
    subject_name: String,
    category: String,
    solve: ScoreboardRankingChallenge,
    score_awarded: i32,
    submitted_at: DateTime<Utc>,
) {
    let state = states
        .entry(subject_id)
        .or_insert_with(|| RankingSubjectState {
            subject_name,
            total_score: 0,
            solved_count: 0,
            last_submit_at: None,
            categories: HashMap::new(),
        });

    state.total_score += score_awarded as i64;
    state.solved_count += 1;
    state.last_submit_at = Some(submitted_at);
    state.categories.entry(category).or_default().push(solve);
}

async fn load_team_channels_lookup(
    state: &AppState,
    contest_id: Uuid,
    team_ids: &[Uuid],
) -> AppResult<HashMap<Uuid, Vec<String>>> {
    if team_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = sqlx::query_as::<_, TeamChannelsRow>(
        "SELECT tm.team_id,
                ARRAY_AGG(DISTINCT c.name ORDER BY c.name) AS channels
         FROM team_members tm
         JOIN contest_scoreboard_channel_members cm
           ON cm.user_id = tm.user_id
          AND cm.contest_id = $1
         JOIN contest_scoreboard_channels c
           ON c.id = cm.channel_id
          AND c.contest_id = cm.contest_id
         WHERE tm.team_id = ANY($2::uuid[])
         GROUP BY tm.team_id",
    )
    .bind(contest_id)
    .bind(team_ids)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    let mut lookup = HashMap::with_capacity(rows.len());
    for row in rows {
        lookup.insert(row.team_id, row.channels);
    }
    Ok(lookup)
}

async fn load_user_channels_lookup(
    state: &AppState,
    contest_id: Uuid,
    user_ids: &[Uuid],
) -> AppResult<HashMap<Uuid, Vec<String>>> {
    if user_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = sqlx::query_as::<_, UserChannelsRow>(
        "SELECT cm.user_id,
                ARRAY_AGG(DISTINCT c.name ORDER BY c.name) AS channels
         FROM contest_scoreboard_channel_members cm
         JOIN contest_scoreboard_channels c
           ON c.id = cm.channel_id
          AND c.contest_id = cm.contest_id
         WHERE cm.contest_id = $1
           AND cm.user_id = ANY($2::uuid[])
         GROUP BY cm.user_id",
    )
    .bind(contest_id)
    .bind(user_ids)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    let mut lookup = HashMap::with_capacity(rows.len());
    for row in rows {
        lookup.insert(row.user_id, row.channels);
    }
    Ok(lookup)
}

fn build_ranking_entries(
    states: HashMap<Uuid, RankingSubjectState>,
    category_order: &[String],
    channels_lookup: &HashMap<Uuid, Vec<String>>,
) -> Vec<ScoreboardRankingEntry> {
    let mut rows: Vec<(Uuid, RankingSubjectState)> = states.into_iter().collect();
    rows.sort_by(|lhs, rhs| {
        rhs.1
            .total_score
            .cmp(&lhs.1.total_score)
            .then_with(|| rhs.1.solved_count.cmp(&lhs.1.solved_count))
            .then_with(|| lhs.1.last_submit_at.cmp(&rhs.1.last_submit_at))
            .then_with(|| lhs.1.subject_name.cmp(&rhs.1.subject_name))
    });

    let category_pos: HashMap<&str, usize> = category_order
        .iter()
        .enumerate()
        .map(|(idx, category)| (category.as_str(), idx))
        .collect();

    let mut entries = Vec::with_capacity(rows.len());
    let mut current_rank = 0_usize;
    let mut last_key: Option<(i64, i64, Option<DateTime<Utc>>)> = None;

    for (index, (subject_id, state)) in rows.into_iter().enumerate() {
        let key = (state.total_score, state.solved_count, state.last_submit_at);
        if last_key.as_ref() != Some(&key) {
            current_rank = index + 1;
            last_key = Some(key);
        }

        let mut categories: Vec<ScoreboardRankingCategory> = state
            .categories
            .into_iter()
            .map(|(category, mut challenges)| {
                challenges.sort_by(|lhs, rhs| {
                    lhs.submitted_at
                        .cmp(&rhs.submitted_at)
                        .then_with(|| lhs.challenge_title.cmp(&rhs.challenge_title))
                });
                ScoreboardRankingCategory {
                    category,
                    solved_count: challenges.len() as i64,
                    challenges,
                }
            })
            .collect();

        categories.sort_by(|lhs, rhs| {
            let left = category_pos
                .get(lhs.category.as_str())
                .copied()
                .unwrap_or(usize::MAX);
            let right = category_pos
                .get(rhs.category.as_str())
                .copied()
                .unwrap_or(usize::MAX);
            left.cmp(&right)
                .then_with(|| lhs.category.cmp(&rhs.category))
        });

        entries.push(ScoreboardRankingEntry {
            rank: current_rank,
            subject_id,
            subject_name: state.subject_name,
            total_score: state.total_score,
            solved_count: state.solved_count,
            last_submit_at: state.last_submit_at,
            channels: channels_lookup.get(&subject_id).cloned().unwrap_or_default(),
            categories,
        });
    }

    entries
}
