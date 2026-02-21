use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    auth::AuthenticatedUser,
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Debug, Clone, FromRow)]
pub(crate) struct ContestGateRow {
    pub visibility: String,
    pub status: String,
    pub registration_requires_approval: bool,
}

#[derive(Debug, Clone, FromRow)]
struct TeamMembershipRow {
    team_id: Uuid,
}

#[derive(Debug, Clone, FromRow)]
pub(crate) struct ContestRegistrationRow {
    pub status: String,
    pub review_note: String,
    pub requested_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
}

pub(crate) fn is_privileged_role(role: &str) -> bool {
    role == "admin" || role == "judge"
}

pub(crate) fn ensure_contest_visibility(
    contest: &ContestGateRow,
    current_user: &AuthenticatedUser,
) -> AppResult<()> {
    if is_privileged_role(&current_user.role) {
        return Ok(());
    }

    if contest.visibility == "private" {
        return Err(AppError::Forbidden);
    }

    if contest.status == "draft" || contest.status == "archived" {
        return Err(AppError::Forbidden);
    }

    Ok(())
}

pub(crate) async fn load_contest_gate(state: &AppState, contest_id: Uuid) -> AppResult<ContestGateRow> {
    sqlx::query_as::<_, ContestGateRow>(
        "SELECT visibility,
                status,
                registration_requires_approval
         FROM contests
         WHERE id = $1
         LIMIT 1",
    )
    .bind(contest_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("contest not found".to_string()))
}

pub(crate) async fn get_user_team_id_optional(state: &AppState, user_id: Uuid) -> AppResult<Option<Uuid>> {
    let row = sqlx::query_as::<_, TeamMembershipRow>(
        "SELECT team_id
         FROM team_members
         WHERE user_id = $1
         LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(row.map(|item| item.team_id))
}

pub(crate) async fn ensure_user_has_team(state: &AppState, user_id: Uuid) -> AppResult<Uuid> {
    get_user_team_id_optional(state, user_id)
        .await?
        .ok_or(AppError::BadRequest(
            "join or create a team before entering the contest".to_string(),
        ))
}

pub(crate) async fn load_contest_registration(
    state: &AppState,
    contest_id: Uuid,
    team_id: Uuid,
) -> AppResult<Option<ContestRegistrationRow>> {
    sqlx::query_as::<_, ContestRegistrationRow>(
        "SELECT status,
                review_note,
                requested_at,
                reviewed_at
         FROM contest_registrations
         WHERE contest_id = $1
           AND team_id = $2
         LIMIT 1",
    )
    .bind(contest_id)
    .bind(team_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)
}

pub(crate) fn ensure_registration_status(
    registration: Option<&ContestRegistrationRow>,
    registration_requires_approval: bool,
) -> AppResult<()> {
    let row = registration.ok_or(AppError::BadRequest(
        "contest registration required before entering workspace".to_string(),
    ))?;

    match row.status.as_str() {
        "approved" => Ok(()),
        "pending" if !registration_requires_approval => Ok(()),
        "pending" => Err(AppError::BadRequest(
            "contest registration is pending admin approval".to_string(),
        )),
        "rejected" => Err(AppError::BadRequest(
            "contest registration was rejected by admin".to_string(),
        )),
        _ => Err(AppError::BadRequest(
            "contest registration status is invalid".to_string(),
        )),
    }
}

pub(crate) async fn ensure_user_contest_workspace_access(
    state: &AppState,
    contest_id: Uuid,
    current_user: &AuthenticatedUser,
) -> AppResult<Option<Uuid>> {
    let contest = load_contest_gate(state, contest_id).await?;
    ensure_contest_visibility(&contest, current_user)?;

    if is_privileged_role(&current_user.role) {
        return Ok(None);
    }

    let team_id = ensure_user_has_team(state, current_user.user_id).await?;
    let registration = load_contest_registration(state, contest_id, team_id).await?;
    ensure_registration_status(registration.as_ref(), contest.registration_requires_approval)?;
    Ok(Some(team_id))
}

pub(crate) async fn ensure_team_contest_workspace_access(
    state: &AppState,
    contest_id: Uuid,
    team_id: Uuid,
    current_user: &AuthenticatedUser,
) -> AppResult<()> {
    let contest = load_contest_gate(state, contest_id).await?;
    ensure_contest_visibility(&contest, current_user)?;

    if is_privileged_role(&current_user.role) {
        return Ok(());
    }

    let registration = load_contest_registration(state, contest_id, team_id).await?;
    ensure_registration_status(registration.as_ref(), contest.registration_requires_approval)
}
