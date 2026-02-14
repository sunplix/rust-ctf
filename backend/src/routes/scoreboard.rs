use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Debug, Serialize)]
struct ScoreboardEntry {
    rank: usize,
    team_id: Uuid,
    team_name: String,
    score: i64,
    solved_count: i64,
    last_submit_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow)]
struct ScoreboardRow {
    team_id: Uuid,
    team_name: String,
    score: i64,
    solved_count: i64,
    last_submit_at: Option<DateTime<Utc>>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/contests/{contest_id}/scoreboard", get(get_scoreboard))
}

async fn get_scoreboard(
    State(state): State<Arc<AppState>>,
    Path(contest_id): Path<Uuid>,
) -> AppResult<Json<Vec<ScoreboardEntry>>> {
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

    Ok(Json(scoreboard))
}
