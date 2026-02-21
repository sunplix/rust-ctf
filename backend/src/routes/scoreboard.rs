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
    routes::contest_access::ensure_user_contest_workspace_access,
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
    generated_at: DateTime<Utc>,
    categories: Vec<ScoreboardCategoryItem>,
    team_rankings: Vec<ScoreboardRankingEntry>,
    player_rankings: Vec<ScoreboardRankingEntry>,
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
struct ScoreboardTimelineEventRow {
    submission_id: i64,
    team_id: Uuid,
    team_name: String,
    score_awarded: i32,
    submitted_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct RankingSolveEventRow {
    _submission_id: i64,
    team_id: Uuid,
    team_name: String,
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
struct ContestChallengeCatalogRow {
    challenge_id: Uuid,
    challenge_title: String,
    challenge_slug: String,
    challenge_category: String,
}

#[derive(Debug, Deserialize)]
struct ScoreboardWsAuthQuery {
    access_token: Option<String>,
    token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ScoreboardTimelineQuery {
    max_snapshots: Option<i64>,
    top_n: Option<i64>,
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

async fn get_scoreboard(
    State(state): State<Arc<AppState>>,
    Path(contest_id): Path<Uuid>,
    current_user: AuthenticatedUser,
) -> AppResult<Json<Vec<ScoreboardEntry>>> {
    ensure_scoreboard_access(state.as_ref(), contest_id, &current_user).await?;
    let entries = load_scoreboard_entries(state.as_ref(), contest_id).await?;
    Ok(Json(entries))
}

async fn get_scoreboard_rankings(
    State(state): State<Arc<AppState>>,
    Path(contest_id): Path<Uuid>,
    current_user: AuthenticatedUser,
) -> AppResult<Json<ScoreboardRankingsResponse>> {
    ensure_scoreboard_access(state.as_ref(), contest_id, &current_user).await?;

    let (categories, team_rankings, player_rankings) =
        load_scoreboard_rankings(state.as_ref(), contest_id).await?;

    Ok(Json(ScoreboardRankingsResponse {
        contest_id,
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

    let max_snapshots = query.max_snapshots.unwrap_or(800).clamp(1, 5000) as usize;
    let top_n = query.top_n.unwrap_or(12).clamp(1, 200) as usize;

    let (snapshots, latest_entries) =
        load_scoreboard_timeline(state.as_ref(), contest_id, max_snapshots, top_n).await?;

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
    ensure_user_contest_workspace_access(state, contest_id, current_user).await?;
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

async fn load_scoreboard_timeline(
    state: &AppState,
    contest_id: Uuid,
    max_snapshots: usize,
    top_n: usize,
) -> AppResult<(Vec<ScoreboardTimelineSnapshot>, Vec<ScoreboardEntry>)> {
    let latest_entries = load_scoreboard_entries(state, contest_id).await?;

    let events = sqlx::query_as::<_, ScoreboardTimelineEventRow>(
        "SELECT s.id AS submission_id,
                s.team_id,
                t.name AS team_name,
                s.score_awarded,
                s.submitted_at
         FROM submissions s
         JOIN teams t ON t.id = s.team_id
         WHERE s.contest_id = $1
           AND s.verdict = 'accepted'
           AND s.score_awarded > 0
         ORDER BY s.submitted_at ASC, s.id ASC",
    )
    .bind(contest_id)
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

    let events = sqlx::query_as::<_, RankingSolveEventRow>(
        "SELECT s.id AS _submission_id,
                s.team_id,
                t.name AS team_name,
                s.user_id,
                u.username,
                c.id AS challenge_id,
                c.title AS challenge_title,
                c.slug AS challenge_slug,
                c.category AS challenge_category,
                s.score_awarded,
                s.submitted_at
         FROM submissions s
         JOIN teams t ON t.id = s.team_id
         JOIN users u ON u.id = s.user_id
         JOIN challenges c ON c.id = s.challenge_id
         WHERE s.contest_id = $1
           AND s.verdict = 'accepted'
           AND s.score_awarded > 0
         ORDER BY s.submitted_at ASC, s.id ASC",
    )
    .bind(contest_id)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    let mut team_states: HashMap<Uuid, RankingSubjectState> = HashMap::new();
    let mut player_states: HashMap<Uuid, RankingSubjectState> = HashMap::new();
    let mut team_seen: HashSet<(Uuid, Uuid)> = HashSet::new();
    let mut player_seen: HashSet<(Uuid, Uuid)> = HashSet::new();
    let mut team_blood_order: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    let mut player_blood_order: HashMap<Uuid, Vec<Uuid>> = HashMap::new();

    for row in events {
        if team_seen.insert((row.team_id, row.challenge_id)) {
            let order = marker_order_for_subject(
                &mut team_blood_order,
                row.challenge_id,
                row.team_id,
            );
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

        if player_seen.insert((row.user_id, row.challenge_id)) {
            let order = marker_order_for_subject(
                &mut player_blood_order,
                row.challenge_id,
                row.user_id,
            );
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

    let team_rankings = build_ranking_entries(team_states, &category_order);
    let player_rankings = build_ranking_entries(player_states, &category_order);

    Ok((categories, team_rankings, player_rankings))
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
    let state = states.entry(subject_id).or_insert_with(|| RankingSubjectState {
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

fn build_ranking_entries(
    states: HashMap<Uuid, RankingSubjectState>,
    category_order: &[String],
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
            categories,
        });
    }

    entries
}
