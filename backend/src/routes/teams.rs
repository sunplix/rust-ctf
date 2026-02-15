use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    auth::AuthenticatedUser,
    error::{AppError, AppResult},
    state::AppState,
};

const INVITATION_STATUS_ALLOWED: &[&str] =
    &["pending", "accepted", "rejected", "canceled", "expired"];

#[derive(Debug, Serialize, FromRow)]
struct TeamListItem {
    id: Uuid,
    name: String,
    description: String,
    captain_user_id: Uuid,
    captain_username: Option<String>,
    member_count: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct TeamsQuery {
    keyword: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct MyTeamResponse {
    team: Option<TeamProfile>,
}

#[derive(Debug, Serialize)]
struct TeamProfile {
    id: Uuid,
    name: String,
    description: String,
    captain_user_id: Uuid,
    captain_username: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    members: Vec<TeamMemberItem>,
}

#[derive(Debug, Serialize, FromRow)]
struct TeamMemberItem {
    user_id: Uuid,
    username: String,
    member_role: String,
    joined_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct LeaveTeamResponse {
    team_id: Uuid,
    disbanded: bool,
    message: String,
}

#[derive(Debug, Serialize, FromRow)]
struct TeamInvitationItem {
    id: Uuid,
    team_id: Uuid,
    team_name: String,
    inviter_user_id: Uuid,
    inviter_username: Option<String>,
    invitee_user_id: Uuid,
    invitee_username: Option<String>,
    status: String,
    message: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    responded_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct InvitationRespondResponse {
    invitation: TeamInvitationItem,
    team: Option<TeamProfile>,
}

#[derive(Debug, Deserialize)]
struct CreateTeamRequest {
    name: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateTeamRequest {
    name: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JoinTeamRequest {
    team_id: Option<Uuid>,
    team_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TransferCaptainRequest {
    new_captain_user_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct CreateTeamInvitationRequest {
    invitee_user_id: Option<Uuid>,
    invitee_username: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TeamInvitationsQuery {
    status: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct RespondInvitationRequest {
    action: String,
}

#[derive(Debug, FromRow)]
struct TeamIdRow {
    team_id: Uuid,
}

#[derive(Debug, FromRow)]
struct TeamMembershipRow {
    team_id: Uuid,
    member_role: String,
}

#[derive(Debug, FromRow)]
struct TeamProfileRow {
    id: Uuid,
    name: String,
    description: String,
    captain_user_id: Uuid,
    captain_username: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct ResolvedTeamIdRow {
    id: Uuid,
}

#[derive(Debug, FromRow)]
struct ResolvedUserIdRow {
    id: Uuid,
}

#[derive(Debug, FromRow)]
struct MemberCountRow {
    count: i64,
}

#[derive(Debug, FromRow)]
struct InvitationGuardRow {
    team_id: Uuid,
    status: String,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/teams", get(list_teams).post(create_team))
        .route("/teams/me", get(get_my_team))
        .route("/teams/join", post(join_team))
        .route("/teams/leave", post(leave_team))
        .route("/teams/invitations", post(create_invitation))
        .route(
            "/teams/invitations/received",
            get(list_received_invitations),
        )
        .route("/teams/invitations/sent", get(list_sent_invitations))
        .route(
            "/teams/invitations/{invitation_id}/respond",
            post(respond_invitation),
        )
        .route(
            "/teams/invitations/{invitation_id}/cancel",
            post(cancel_invitation),
        )
        .route(
            "/teams/{team_id}",
            get(get_team_by_id).patch(update_team).delete(disband_team),
        )
        .route("/teams/{team_id}/transfer-captain", post(transfer_captain))
        .route(
            "/teams/{team_id}/members/{member_user_id}",
            delete(remove_team_member),
        )
}

async fn list_teams(
    State(state): State<Arc<AppState>>,
    _current_user: AuthenticatedUser,
    Query(query): Query<TeamsQuery>,
) -> AppResult<Json<Vec<TeamListItem>>> {
    let keyword = query.keyword.as_deref().and_then(normalize_optional_text);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let rows = sqlx::query_as::<_, TeamListItem>(
        "SELECT t.id,
                t.name,
                t.description,
                t.captain_user_id,
                u.username AS captain_username,
                COUNT(tm.user_id)::bigint AS member_count,
                t.created_at,
                t.updated_at
         FROM teams t
         LEFT JOIN users u ON u.id = t.captain_user_id
         LEFT JOIN team_members tm ON tm.team_id = t.id
         WHERE ($1::text IS NULL OR LOWER(t.name) LIKE '%' || LOWER($1) || '%')
         GROUP BY t.id, u.username
         ORDER BY t.created_at DESC
         LIMIT $2",
    )
    .bind(keyword)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(rows))
}

async fn get_my_team(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
) -> AppResult<Json<MyTeamResponse>> {
    let my_team_id = fetch_user_team_id(state.as_ref(), current_user.user_id).await?;
    let team = match my_team_id {
        Some(team_id) => Some(load_team_profile(state.as_ref(), team_id).await?),
        None => None,
    };

    Ok(Json(MyTeamResponse { team }))
}

async fn get_team_by_id(
    State(state): State<Arc<AppState>>,
    _current_user: AuthenticatedUser,
    Path(team_id): Path<Uuid>,
) -> AppResult<Json<TeamProfile>> {
    let team = load_team_profile(state.as_ref(), team_id).await?;
    Ok(Json(team))
}

async fn create_team(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Json(req): Json<CreateTeamRequest>,
) -> AppResult<Json<TeamProfile>> {
    let name = trim_required(&req.name, "name")?;
    let description = req.description.unwrap_or_default();

    if name.chars().count() > 64 {
        return Err(AppError::BadRequest(
            "team name must be at most 64 characters".to_string(),
        ));
    }

    if description.chars().count() > 500 {
        return Err(AppError::BadRequest(
            "team description must be at most 500 characters".to_string(),
        ));
    }

    if fetch_user_team_id(state.as_ref(), current_user.user_id)
        .await?
        .is_some()
    {
        warn!(
            user_id = %current_user.user_id,
            "team creation denied: user already in a team"
        );
        return Err(AppError::Conflict(
            "you are already in a team; leave current team before creating a new one".to_string(),
        ));
    }

    let mut tx = state.db.begin().await.map_err(AppError::internal)?;

    let team_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO teams (name, captain_user_id, description)
         VALUES ($1, $2, $3)
         RETURNING id",
    )
    .bind(&name)
    .bind(current_user.user_id)
    .bind(&description)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| {
        if is_unique_violation(&err) {
            AppError::Conflict("team name already exists".to_string())
        } else {
            AppError::internal(err)
        }
    })?;

    sqlx::query(
        "INSERT INTO team_members (team_id, user_id, member_role)
         VALUES ($1, $2, 'captain')",
    )
    .bind(team_id)
    .bind(current_user.user_id)
    .execute(&mut *tx)
    .await
    .map_err(|err| {
        if is_unique_violation(&err) {
            AppError::Conflict("you are already in a team".to_string())
        } else {
            AppError::internal(err)
        }
    })?;

    tx.commit().await.map_err(AppError::internal)?;

    info!(
        team_id = %team_id,
        team_name = %name,
        captain_user_id = %current_user.user_id,
        "team created"
    );

    let team = load_team_profile(state.as_ref(), team_id).await?;
    Ok(Json(team))
}

async fn join_team(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Json(req): Json<JoinTeamRequest>,
) -> AppResult<Json<TeamProfile>> {
    if fetch_user_team_id(state.as_ref(), current_user.user_id)
        .await?
        .is_some()
    {
        warn!(
            user_id = %current_user.user_id,
            "team join denied: user already in a team"
        );
        return Err(AppError::Conflict(
            "you are already in a team; leave current team before joining another one".to_string(),
        ));
    }

    let target_team_id = resolve_target_team_id(state.as_ref(), &req).await?;

    sqlx::query(
        "INSERT INTO team_members (team_id, user_id, member_role)
         VALUES ($1, $2, 'member')",
    )
    .bind(target_team_id)
    .bind(current_user.user_id)
    .execute(&state.db)
    .await
    .map_err(|err| {
        if is_unique_violation(&err) {
            AppError::Conflict("you are already in a team".to_string())
        } else if is_foreign_key_violation(&err) {
            AppError::BadRequest("target team does not exist".to_string())
        } else {
            AppError::internal(err)
        }
    })?;

    info!(
        team_id = %target_team_id,
        user_id = %current_user.user_id,
        "user joined team"
    );

    let team = load_team_profile(state.as_ref(), target_team_id).await?;
    Ok(Json(team))
}

async fn leave_team(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
) -> AppResult<Json<LeaveTeamResponse>> {
    let membership = fetch_user_team_membership(state.as_ref(), current_user.user_id)
        .await?
        .ok_or(AppError::BadRequest("you are not in any team".to_string()))?;

    if membership.member_role == "captain" {
        let member_count = sqlx::query_as::<_, MemberCountRow>(
            "SELECT COUNT(*)::bigint AS count
             FROM team_members
             WHERE team_id = $1",
        )
        .bind(membership.team_id)
        .fetch_one(&state.db)
        .await
        .map_err(AppError::internal)?
        .count;

        if member_count > 1 {
            return Err(AppError::Conflict(
                "captain cannot leave while team has other members; transfer captain or disband team"
                    .to_string(),
            ));
        }

        let result = sqlx::query("DELETE FROM teams WHERE id = $1 AND captain_user_id = $2")
            .bind(membership.team_id)
            .bind(current_user.user_id)
            .execute(&state.db)
            .await
            .map_err(AppError::internal)?;

        if result.rows_affected() == 0 {
            return Err(AppError::Forbidden);
        }

        info!(
            team_id = %membership.team_id,
            captain_user_id = %current_user.user_id,
            "captain left and team disbanded automatically"
        );

        return Ok(Json(LeaveTeamResponse {
            team_id: membership.team_id,
            disbanded: true,
            message: "team disbanded".to_string(),
        }));
    }

    let result = sqlx::query("DELETE FROM team_members WHERE team_id = $1 AND user_id = $2")
        .bind(membership.team_id)
        .bind(current_user.user_id)
        .execute(&state.db)
        .await
        .map_err(AppError::internal)?;

    if result.rows_affected() == 0 {
        return Err(AppError::BadRequest("you are not in this team".to_string()));
    }

    info!(
        team_id = %membership.team_id,
        user_id = %current_user.user_id,
        "user left team"
    );

    Ok(Json(LeaveTeamResponse {
        team_id: membership.team_id,
        disbanded: false,
        message: "left team".to_string(),
    }))
}

async fn update_team(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(team_id): Path<Uuid>,
    Json(req): Json<UpdateTeamRequest>,
) -> AppResult<Json<TeamProfile>> {
    ensure_captain_of_team(state.as_ref(), current_user.user_id, team_id).await?;

    let name = req
        .name
        .as_deref()
        .map(|value| trim_required(value, "name"))
        .transpose()?;
    let description = req.description.map(|value| value.trim().to_string());

    if name.is_none() && description.is_none() {
        return Err(AppError::BadRequest(
            "at least one field is required for update".to_string(),
        ));
    }

    if let Some(value) = &name {
        if value.chars().count() > 64 {
            return Err(AppError::BadRequest(
                "team name must be at most 64 characters".to_string(),
            ));
        }
    }

    if let Some(value) = &description {
        if value.chars().count() > 500 {
            return Err(AppError::BadRequest(
                "team description must be at most 500 characters".to_string(),
            ));
        }
    }

    let result = sqlx::query(
        "UPDATE teams
         SET name = COALESCE($2, name),
             description = COALESCE($3, description),
             updated_at = NOW()
         WHERE id = $1",
    )
    .bind(team_id)
    .bind(name)
    .bind(description)
    .execute(&state.db)
    .await
    .map_err(|err| {
        if is_unique_violation(&err) {
            AppError::Conflict("team name already exists".to_string())
        } else {
            AppError::internal(err)
        }
    })?;

    if result.rows_affected() == 0 {
        return Err(AppError::BadRequest("team not found".to_string()));
    }

    let team = load_team_profile(state.as_ref(), team_id).await?;
    Ok(Json(team))
}

async fn transfer_captain(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(team_id): Path<Uuid>,
    Json(req): Json<TransferCaptainRequest>,
) -> AppResult<Json<TeamProfile>> {
    ensure_captain_of_team(state.as_ref(), current_user.user_id, team_id).await?;

    if req.new_captain_user_id == current_user.user_id {
        return Err(AppError::BadRequest(
            "new captain must be different from current captain".to_string(),
        ));
    }

    let is_member = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1
            FROM team_members
            WHERE team_id = $1 AND user_id = $2
         )",
    )
    .bind(team_id)
    .bind(req.new_captain_user_id)
    .fetch_one(&state.db)
    .await
    .map_err(AppError::internal)?;

    if !is_member {
        return Err(AppError::BadRequest(
            "new captain must be a current team member".to_string(),
        ));
    }

    let mut tx = state.db.begin().await.map_err(AppError::internal)?;

    sqlx::query("UPDATE teams SET captain_user_id = $2, updated_at = NOW() WHERE id = $1")
        .bind(team_id)
        .bind(req.new_captain_user_id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::internal)?;

    sqlx::query(
        "UPDATE team_members
         SET member_role = 'member'
         WHERE team_id = $1 AND user_id = $2",
    )
    .bind(team_id)
    .bind(current_user.user_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::internal)?;

    sqlx::query(
        "UPDATE team_members
         SET member_role = 'captain'
         WHERE team_id = $1 AND user_id = $2",
    )
    .bind(team_id)
    .bind(req.new_captain_user_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::internal)?;

    tx.commit().await.map_err(AppError::internal)?;

    info!(
        team_id = %team_id,
        old_captain_user_id = %current_user.user_id,
        new_captain_user_id = %req.new_captain_user_id,
        "team captain transferred"
    );

    let team = load_team_profile(state.as_ref(), team_id).await?;
    Ok(Json(team))
}

async fn remove_team_member(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path((team_id, member_user_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<TeamProfile>> {
    ensure_captain_of_team(state.as_ref(), current_user.user_id, team_id).await?;

    if member_user_id == current_user.user_id {
        return Err(AppError::BadRequest(
            "captain cannot remove self; transfer captain or disband team".to_string(),
        ));
    }

    let result = sqlx::query(
        "DELETE FROM team_members
         WHERE team_id = $1
           AND user_id = $2
           AND member_role <> 'captain'",
    )
    .bind(team_id)
    .bind(member_user_id)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    if result.rows_affected() == 0 {
        return Err(AppError::BadRequest(
            "target member not found or cannot remove captain".to_string(),
        ));
    }

    info!(
        team_id = %team_id,
        captain_user_id = %current_user.user_id,
        member_user_id = %member_user_id,
        "team member removed by captain"
    );

    let team = load_team_profile(state.as_ref(), team_id).await?;
    Ok(Json(team))
}

async fn disband_team(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(team_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    ensure_captain_of_team(state.as_ref(), current_user.user_id, team_id).await?;

    let result = sqlx::query("DELETE FROM teams WHERE id = $1")
        .bind(team_id)
        .execute(&state.db)
        .await
        .map_err(AppError::internal)?;

    if result.rows_affected() == 0 {
        return Err(AppError::BadRequest("team not found".to_string()));
    }

    info!(
        team_id = %team_id,
        captain_user_id = %current_user.user_id,
        "team disbanded"
    );

    Ok(StatusCode::NO_CONTENT)
}

async fn create_invitation(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Json(req): Json<CreateTeamInvitationRequest>,
) -> AppResult<Json<TeamInvitationItem>> {
    let membership = fetch_user_team_membership(state.as_ref(), current_user.user_id)
        .await?
        .ok_or(AppError::Forbidden)?;
    if membership.member_role != "captain" {
        return Err(AppError::Forbidden);
    }

    let invitee_user_id = resolve_invitee_user_id(state.as_ref(), &req).await?;
    if invitee_user_id == current_user.user_id {
        return Err(AppError::BadRequest("cannot invite yourself".to_string()));
    }

    if fetch_user_team_id(state.as_ref(), invitee_user_id)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict(
            "invitee is already in a team".to_string(),
        ));
    }

    let message = req.message.unwrap_or_default().trim().to_string();
    if message.chars().count() > 500 {
        return Err(AppError::BadRequest(
            "invitation message must be at most 500 characters".to_string(),
        ));
    }

    let invitation_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO team_invitations (team_id, inviter_user_id, invitee_user_id, message)
         VALUES ($1, $2, $3, $4)
         RETURNING id",
    )
    .bind(membership.team_id)
    .bind(current_user.user_id)
    .bind(invitee_user_id)
    .bind(&message)
    .fetch_one(&state.db)
    .await
    .map_err(|err| {
        if is_unique_violation(&err) {
            AppError::Conflict(
                "a pending invitation already exists for this user in your team".to_string(),
            )
        } else if is_foreign_key_violation(&err) {
            AppError::BadRequest("team or user does not exist".to_string())
        } else {
            AppError::internal(err)
        }
    })?;

    let invitation = load_invitation_item(state.as_ref(), invitation_id).await?;

    info!(
        team_id = %invitation.team_id,
        invitation_id = %invitation.id,
        inviter_user_id = %current_user.user_id,
        invitee_user_id = %invitation.invitee_user_id,
        "team invitation created"
    );

    Ok(Json(invitation))
}

async fn list_received_invitations(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Query(query): Query<TeamInvitationsQuery>,
) -> AppResult<Json<Vec<TeamInvitationItem>>> {
    let status_filter = query
        .status
        .as_deref()
        .map(|value| normalize_with_allowed(value, INVITATION_STATUS_ALLOWED, "status"))
        .transpose()?;
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let rows = sqlx::query_as::<_, TeamInvitationItem>(
        "SELECT ti.id,
                ti.team_id,
                t.name AS team_name,
                ti.inviter_user_id,
                inviter.username AS inviter_username,
                ti.invitee_user_id,
                invitee.username AS invitee_username,
                ti.status,
                ti.message,
                ti.created_at,
                ti.updated_at,
                ti.responded_at
         FROM team_invitations ti
         JOIN teams t ON t.id = ti.team_id
         LEFT JOIN users inviter ON inviter.id = ti.inviter_user_id
         LEFT JOIN users invitee ON invitee.id = ti.invitee_user_id
         WHERE ti.invitee_user_id = $1
           AND ($2::text IS NULL OR ti.status = $2)
         ORDER BY ti.created_at DESC
         LIMIT $3",
    )
    .bind(current_user.user_id)
    .bind(status_filter)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(rows))
}

async fn list_sent_invitations(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Query(query): Query<TeamInvitationsQuery>,
) -> AppResult<Json<Vec<TeamInvitationItem>>> {
    let membership = fetch_user_team_membership(state.as_ref(), current_user.user_id)
        .await?
        .ok_or(AppError::Forbidden)?;
    if membership.member_role != "captain" {
        return Err(AppError::Forbidden);
    }

    let status_filter = query
        .status
        .as_deref()
        .map(|value| normalize_with_allowed(value, INVITATION_STATUS_ALLOWED, "status"))
        .transpose()?;
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let rows = sqlx::query_as::<_, TeamInvitationItem>(
        "SELECT ti.id,
                ti.team_id,
                t.name AS team_name,
                ti.inviter_user_id,
                inviter.username AS inviter_username,
                ti.invitee_user_id,
                invitee.username AS invitee_username,
                ti.status,
                ti.message,
                ti.created_at,
                ti.updated_at,
                ti.responded_at
         FROM team_invitations ti
         JOIN teams t ON t.id = ti.team_id
         LEFT JOIN users inviter ON inviter.id = ti.inviter_user_id
         LEFT JOIN users invitee ON invitee.id = ti.invitee_user_id
         WHERE ti.team_id = $1
           AND ($2::text IS NULL OR ti.status = $2)
         ORDER BY ti.created_at DESC
         LIMIT $3",
    )
    .bind(membership.team_id)
    .bind(status_filter)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(rows))
}

async fn respond_invitation(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(invitation_id): Path<Uuid>,
    Json(req): Json<RespondInvitationRequest>,
) -> AppResult<Json<InvitationRespondResponse>> {
    let action = req.action.trim().to_lowercase();
    let next_status = match action.as_str() {
        "accept" => "accepted",
        "reject" => "rejected",
        _ => {
            return Err(AppError::BadRequest(
                "action must be accept or reject".to_string(),
            ));
        }
    };

    let invitation_guard = sqlx::query_as::<_, InvitationGuardRow>(
        "SELECT team_id, status
         FROM team_invitations
         WHERE id = $1
           AND invitee_user_id = $2
         LIMIT 1",
    )
    .bind(invitation_id)
    .bind(current_user.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest(
        "invitation not found for current user".to_string(),
    ))?;

    if invitation_guard.status != "pending" {
        return Err(AppError::BadRequest(
            "invitation is already processed".to_string(),
        ));
    }

    if next_status == "accepted"
        && fetch_user_team_id(state.as_ref(), current_user.user_id)
            .await?
            .is_some()
    {
        return Err(AppError::Conflict("you are already in a team".to_string()));
    }

    let mut tx = state.db.begin().await.map_err(AppError::internal)?;

    let updated = sqlx::query(
        "UPDATE team_invitations
         SET status = $2,
             responded_at = NOW(),
             updated_at = NOW()
         WHERE id = $1
           AND status = 'pending'",
    )
    .bind(invitation_id)
    .bind(next_status)
    .execute(&mut *tx)
    .await
    .map_err(AppError::internal)?;

    if updated.rows_affected() == 0 {
        return Err(AppError::Conflict(
            "invitation is no longer pending".to_string(),
        ));
    }

    if next_status == "accepted" {
        sqlx::query(
            "INSERT INTO team_members (team_id, user_id, member_role)
             VALUES ($1, $2, 'member')",
        )
        .bind(invitation_guard.team_id)
        .bind(current_user.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| {
            if is_unique_violation(&err) {
                AppError::Conflict("you are already in a team".to_string())
            } else {
                AppError::internal(err)
            }
        })?;
    }

    tx.commit().await.map_err(AppError::internal)?;

    let invitation = load_invitation_item(state.as_ref(), invitation_id).await?;
    let team = if next_status == "accepted" {
        Some(load_team_profile(state.as_ref(), invitation.team_id).await?)
    } else {
        None
    };

    info!(
        invitation_id = %invitation.id,
        invitee_user_id = %current_user.user_id,
        status = %invitation.status,
        "team invitation responded"
    );

    Ok(Json(InvitationRespondResponse { invitation, team }))
}

async fn cancel_invitation(
    State(state): State<Arc<AppState>>,
    current_user: AuthenticatedUser,
    Path(invitation_id): Path<Uuid>,
) -> AppResult<Json<TeamInvitationItem>> {
    let invitation_guard = sqlx::query_as::<_, InvitationGuardRow>(
        "SELECT team_id, status
         FROM team_invitations
         WHERE id = $1
         LIMIT 1",
    )
    .bind(invitation_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("invitation not found".to_string()))?;

    ensure_captain_of_team(
        state.as_ref(),
        current_user.user_id,
        invitation_guard.team_id,
    )
    .await?;

    if invitation_guard.status != "pending" {
        return Err(AppError::BadRequest(
            "only pending invitation can be canceled".to_string(),
        ));
    }

    let result = sqlx::query(
        "UPDATE team_invitations
         SET status = 'canceled',
             responded_at = NOW(),
             updated_at = NOW()
         WHERE id = $1
           AND status = 'pending'",
    )
    .bind(invitation_id)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    if result.rows_affected() == 0 {
        return Err(AppError::Conflict(
            "invitation is no longer pending".to_string(),
        ));
    }

    let invitation = load_invitation_item(state.as_ref(), invitation_id).await?;
    Ok(Json(invitation))
}

async fn resolve_target_team_id(state: &AppState, req: &JoinTeamRequest) -> AppResult<Uuid> {
    if let Some(team_id) = req.team_id {
        let exists =
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM teams WHERE id = $1)")
                .bind(team_id)
                .fetch_one(&state.db)
                .await
                .map_err(AppError::internal)?;

        if exists {
            return Ok(team_id);
        }

        return Err(AppError::BadRequest(
            "target team does not exist".to_string(),
        ));
    }

    let team_name = req
        .team_name
        .as_deref()
        .and_then(normalize_optional_text)
        .ok_or(AppError::BadRequest(
            "team_id or team_name is required".to_string(),
        ))?;

    let resolved = sqlx::query_as::<_, ResolvedTeamIdRow>(
        "SELECT id
         FROM teams
         WHERE LOWER(name) = LOWER($1)
         LIMIT 1",
    )
    .bind(team_name)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest(
        "target team does not exist".to_string(),
    ))?;

    Ok(resolved.id)
}

async fn resolve_invitee_user_id(
    state: &AppState,
    req: &CreateTeamInvitationRequest,
) -> AppResult<Uuid> {
    if let Some(user_id) = req.invitee_user_id {
        let resolved = sqlx::query_as::<_, ResolvedUserIdRow>(
            "SELECT id
             FROM users
             WHERE id = $1
               AND status = 'active'
             LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(AppError::internal)?
        .ok_or(AppError::BadRequest(
            "invitee user not found or disabled".to_string(),
        ))?;

        return Ok(resolved.id);
    }

    let username = req
        .invitee_username
        .as_deref()
        .and_then(normalize_optional_text)
        .ok_or(AppError::BadRequest(
            "invitee_user_id or invitee_username is required".to_string(),
        ))?;

    let resolved = sqlx::query_as::<_, ResolvedUserIdRow>(
        "SELECT id
         FROM users
         WHERE LOWER(username) = LOWER($1)
           AND status = 'active'
         LIMIT 1",
    )
    .bind(username)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest(
        "invitee user not found or disabled".to_string(),
    ))?;

    Ok(resolved.id)
}

async fn load_invitation_item(
    state: &AppState,
    invitation_id: Uuid,
) -> AppResult<TeamInvitationItem> {
    let invitation = sqlx::query_as::<_, TeamInvitationItem>(
        "SELECT ti.id,
                ti.team_id,
                t.name AS team_name,
                ti.inviter_user_id,
                inviter.username AS inviter_username,
                ti.invitee_user_id,
                invitee.username AS invitee_username,
                ti.status,
                ti.message,
                ti.created_at,
                ti.updated_at,
                ti.responded_at
         FROM team_invitations ti
         JOIN teams t ON t.id = ti.team_id
         LEFT JOIN users inviter ON inviter.id = ti.inviter_user_id
         LEFT JOIN users invitee ON invitee.id = ti.invitee_user_id
         WHERE ti.id = $1
         LIMIT 1",
    )
    .bind(invitation_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("invitation not found".to_string()))?;

    Ok(invitation)
}

async fn ensure_captain_of_team(state: &AppState, user_id: Uuid, team_id: Uuid) -> AppResult<()> {
    let membership = sqlx::query_as::<_, TeamMembershipRow>(
        "SELECT team_id, member_role
         FROM team_members
         WHERE team_id = $1
           AND user_id = $2
         LIMIT 1",
    )
    .bind(team_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::Forbidden)?;

    if membership.member_role != "captain" {
        return Err(AppError::Forbidden);
    }

    Ok(())
}

async fn fetch_user_team_membership(
    state: &AppState,
    user_id: Uuid,
) -> AppResult<Option<TeamMembershipRow>> {
    let row = sqlx::query_as::<_, TeamMembershipRow>(
        "SELECT team_id, member_role
         FROM team_members
         WHERE user_id = $1
         LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(row)
}

async fn fetch_user_team_id(state: &AppState, user_id: Uuid) -> AppResult<Option<Uuid>> {
    let row = sqlx::query_as::<_, TeamIdRow>(
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

async fn load_team_profile(state: &AppState, team_id: Uuid) -> AppResult<TeamProfile> {
    let team = sqlx::query_as::<_, TeamProfileRow>(
        "SELECT t.id,
                t.name,
                t.description,
                t.captain_user_id,
                u.username AS captain_username,
                t.created_at,
                t.updated_at
         FROM teams t
         LEFT JOIN users u ON u.id = t.captain_user_id
         WHERE t.id = $1
         LIMIT 1",
    )
    .bind(team_id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or(AppError::BadRequest("team not found".to_string()))?;

    let members = sqlx::query_as::<_, TeamMemberItem>(
        "SELECT tm.user_id,
                u.username,
                tm.member_role,
                tm.joined_at
         FROM team_members tm
         JOIN users u ON u.id = tm.user_id
         WHERE tm.team_id = $1
         ORDER BY CASE WHEN tm.member_role = 'captain' THEN 0 ELSE 1 END, tm.joined_at ASC",
    )
    .bind(team_id)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(TeamProfile {
        id: team.id,
        name: team.name,
        description: team.description,
        captain_user_id: team.captain_user_id,
        captain_username: team.captain_username,
        created_at: team.created_at,
        updated_at: team.updated_at,
        members,
    })
}

fn trim_required(value: &str, field: &str) -> AppResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest(format!("{} is required", field)));
    }

    Ok(trimmed.to_string())
}

fn normalize_optional_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn normalize_with_allowed(value: &str, allowed: &[&str], field: &str) -> AppResult<String> {
    let lowered = value.trim().to_lowercase();
    if lowered.is_empty() {
        return Err(AppError::BadRequest(format!("{} is required", field)));
    }

    if allowed.contains(&lowered.as_str()) {
        Ok(lowered)
    } else {
        Err(AppError::BadRequest(format!(
            "{} is invalid, allowed: {}",
            field,
            allowed.join(",")
        )))
    }
}

fn is_unique_violation(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db_err) => db_err.code().as_deref() == Some("23505"),
        _ => false,
    }
}

fn is_foreign_key_violation(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db_err) => db_err.code().as_deref() == Some("23503"),
        _ => false,
    }
}
