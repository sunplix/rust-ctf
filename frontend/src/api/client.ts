import axios, { AxiosError, type AxiosRequestConfig } from "axios";

const RAW_API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? "http://localhost:8080";
export const API_BASE_URL = RAW_API_BASE_URL.replace(/\/+$/, "");

const api = axios.create({
  baseURL: `${API_BASE_URL}/api/v1`,
  timeout: 15000
});

type ApiErrorEnvelope = {
  error?: {
    code?: string;
    message?: string;
  };
};

export class ApiClientError extends Error {
  code: string;

  constructor(message: string, code = "request_failed") {
    super(message);
    this.name = "ApiClientError";
    this.code = code;
  }
}

function toApiClientError(error: unknown): ApiClientError {
  if (error instanceof ApiClientError) {
    return error;
  }

  if (axios.isAxiosError(error)) {
    const axiosError = error as AxiosError<ApiErrorEnvelope>;
    const message =
      axiosError.response?.data?.error?.message ??
      axiosError.message ??
      "request failed";
    const code = axiosError.response?.data?.error?.code ?? "request_failed";
    return new ApiClientError(message, code);
  }

  return new ApiClientError("request failed", "request_failed");
}

function authHeaders(accessToken?: string): AxiosRequestConfig {
  if (!accessToken) {
    return {};
  }

  return {
    headers: {
      Authorization: `Bearer ${accessToken}`
    }
  };
}

export function buildApiAssetUrl(path: string): string {
  return new URL(path, API_BASE_URL).toString();
}

export type AuthUser = {
  id: string;
  username: string;
  email: string;
  role: string;
  email_verified: boolean;
  email_verified_at: string | null;
  created_at: string;
};

export type AuthResponse = {
  access_token: string;
  refresh_token: string;
  token_type: string;
  access_expires_in_seconds: number;
  refresh_expires_in_seconds: number;
  user: AuthUser;
};

export type RegisterResponse = {
  requires_email_verification: boolean;
  message: string;
  auth: AuthResponse | null;
};

export type PasswordPolicySnapshot = {
  min_length: number;
  min_strength_score: number;
  require_lowercase: boolean;
  require_uppercase: boolean;
  require_digit: boolean;
  require_symbol: boolean;
  min_unique_chars: number;
  block_weak_patterns: boolean;
};

export type PasswordPolicyResponse = {
  policy: PasswordPolicySnapshot;
};

export type ActionMessageResponse = {
  message: string;
};

export type SiteSettings = {
  site_name: string;
  site_subtitle: string;
  home_title: string;
  home_tagline: string;
  home_signature: string;
  footer_text: string;
};

export type AdminSiteSettings = SiteSettings & {
  challenge_attachment_max_bytes: number;
  updated_by: string | null;
  updated_at: string;
};

export type LoginHistoryItem = {
  id: number;
  action: string;
  detail: Record<string, unknown>;
  created_at: string;
};

export type ContestListItem = {
  id: string;
  title: string;
  slug: string;
  description: string;
  poster_url: string | null;
  status: string;
  scoring_mode: string;
  dynamic_decay: number;
  latest_announcement_title: string | null;
  latest_announcement_content: string | null;
  latest_announcement_published_at: string | null;
  start_at: string;
  end_at: string;
};

export type ContestChallengeItem = {
  id: string;
  title: string;
  category: string;
  difficulty: string;
  challenge_type: string;
  static_score: number;
  release_at: string | null;
};

export type ContestAnnouncementItem = {
  id: string;
  title: string;
  content: string;
  is_pinned: boolean;
  published_at: string | null;
  created_at: string;
};

export type TeamListItem = {
  id: string;
  name: string;
  description: string;
  captain_user_id: string;
  captain_username: string | null;
  member_count: number;
  created_at: string;
  updated_at: string;
};

export type TeamMemberItem = {
  user_id: string;
  username: string;
  member_role: string;
  joined_at: string;
};

export type TeamProfile = {
  id: string;
  name: string;
  description: string;
  captain_user_id: string;
  captain_username: string | null;
  created_at: string;
  updated_at: string;
  members: TeamMemberItem[];
};

export type MyTeamResponse = {
  team: TeamProfile | null;
};

export type LeaveTeamResponse = {
  team_id: string;
  disbanded: boolean;
  message: string;
};

export type TeamInvitationItem = {
  id: string;
  team_id: string;
  team_name: string;
  inviter_user_id: string;
  inviter_username: string | null;
  invitee_user_id: string;
  invitee_username: string | null;
  status: string;
  message: string;
  created_at: string;
  updated_at: string;
  responded_at: string | null;
};

export type TeamInvitationRespondResponse = {
  invitation: TeamInvitationItem;
  team: TeamProfile | null;
};

export type SubmitFlagResponse = {
  verdict: string;
  score_awarded: number;
  total_score: number;
  message: string;
  submitted_at: string;
};

export type ScoreboardEntry = {
  rank: number;
  team_id: string;
  team_name: string;
  score: number;
  solved_count: number;
  last_submit_at: string | null;
};

export type ScoreboardPushPayload = {
  event: string;
  contest_id: string;
  entries: ScoreboardEntry[];
};

export type ScoreboardTimelineSnapshot = {
  trigger_submission_id: number;
  timestamp: string;
  entries: ScoreboardEntry[];
};

export type ScoreboardTimelineResponse = {
  contest_id: string;
  generated_at: string;
  snapshots: ScoreboardTimelineSnapshot[];
  latest_entries: ScoreboardEntry[];
};

export type ScoreboardRankingChallenge = {
  challenge_id: string;
  challenge_title: string;
  challenge_slug: string;
  marker: "first_blood" | "second_blood" | "third_blood" | "solved";
  score_awarded: number;
  submitted_at: string;
};

export type ScoreboardRankingCategory = {
  category: string;
  solved_count: number;
  challenges: ScoreboardRankingChallenge[];
};

export type ScoreboardRankingEntry = {
  rank: number;
  subject_id: string;
  subject_name: string;
  total_score: number;
  solved_count: number;
  last_submit_at: string | null;
  categories: ScoreboardRankingCategory[];
};

export type ScoreboardCategoryChallengeItem = {
  challenge_id: string;
  challenge_title: string;
  challenge_slug: string;
};

export type ScoreboardCategoryItem = {
  category: string;
  challenges: ScoreboardCategoryChallengeItem[];
};

export type ScoreboardRankingsResponse = {
  contest_id: string;
  generated_at: string;
  categories: ScoreboardCategoryItem[];
  team_rankings: ScoreboardRankingEntry[];
  player_rankings: ScoreboardRankingEntry[];
};

export type InstanceNetworkAccess = {
  mode: string;
  host: string;
  port: number;
  username: string | null;
  password: string | null;
  download_url?: string | null;
  note: string;
};

export type InstanceResponse = {
  id: string;
  contest_id: string;
  challenge_id: string;
  team_id: string;
  status: string;
  subnet: string;
  compose_project_name: string;
  entrypoint_url: string;
  started_at: string | null;
  expires_at: string | null;
  destroyed_at: string | null;
  last_heartbeat_at: string | null;
  network_access?: InstanceNetworkAccess | null;
  message: string;
};

export type AdminChallengeItem = {
  id: string;
  title: string;
  slug: string;
  category: string;
  difficulty: string;
  static_score: number;
  challenge_type: string;
  flag_mode: string;
  status: string;
  is_visible: boolean;
  tags: string[];
  writeup_visibility: string;
  current_version: number;
  created_at: string;
  updated_at: string;
};

export type AdminChallengeDetailItem = {
  id: string;
  title: string;
  slug: string;
  category: string;
  difficulty: string;
  description: string;
  static_score: number;
  min_score: number;
  max_score: number;
  challenge_type: string;
  flag_mode: string;
  status: string;
  flag_hash: string;
  compose_template: string | null;
  metadata: Record<string, unknown>;
  is_visible: boolean;
  tags: string[];
  writeup_visibility: string;
  writeup_content: string;
  current_version: number;
  created_at: string;
  updated_at: string;
};

export type AdminChallengeCategoryItem = {
  id: string;
  slug: string;
  display_name: string;
  sort_order: number;
  is_builtin: boolean;
  created_at: string;
  updated_at: string;
};

export type AdminChallengeVersionItem = {
  id: string;
  challenge_id: string;
  version_no: number;
  change_note: string;
  created_by: string | null;
  created_by_username: string | null;
  created_at: string;
};

export type AdminChallengeAttachmentItem = {
  id: string;
  challenge_id: string;
  filename: string;
  content_type: string;
  storage_path: string;
  size_bytes: number;
  uploaded_by: string | null;
  uploaded_by_username: string | null;
  created_at: string;
};

export type AdminContestItem = {
  id: string;
  title: string;
  slug: string;
  description: string;
  poster_url: string | null;
  visibility: string;
  status: string;
  scoring_mode: string;
  dynamic_decay: number;
  first_blood_bonus_percent: number;
  second_blood_bonus_percent: number;
  third_blood_bonus_percent: number;
  start_at: string;
  end_at: string;
  freeze_at: string | null;
  created_at: string;
  updated_at: string;
};

export type AdminContestAnnouncementItem = {
  id: string;
  contest_id: string;
  title: string;
  content: string;
  is_published: boolean;
  is_pinned: boolean;
  published_at: string | null;
  created_by: string | null;
  created_by_username: string | null;
  updated_by: string | null;
  updated_by_username: string | null;
  created_at: string;
  updated_at: string;
};

export type AdminContestChallengeItem = {
  contest_id: string;
  challenge_id: string;
  challenge_title: string;
  challenge_category: string;
  challenge_difficulty: string;
  sort_order: number;
  release_at: string | null;
};

export type AdminUserItem = {
  id: string;
  username: string;
  email: string;
  role: string;
  status: string;
  created_at: string;
  updated_at: string;
};

export type AdminInstanceItem = {
  id: string;
  contest_id: string;
  contest_title: string;
  challenge_id: string;
  challenge_title: string;
  team_id: string;
  team_name: string;
  status: string;
  subnet: string;
  compose_project_name: string;
  entrypoint_url: string;
  started_at: string | null;
  expires_at: string | null;
  destroyed_at: string | null;
  last_heartbeat_at: string | null;
  created_at: string;
  updated_at: string;
};

export type AdminInstanceRuntimeMetricsService = {
  container_id: string;
  container_name: string;
  service_name: string | null;
  image: string | null;
  state: string | null;
  health_status: string | null;
  restart_count: number | null;
  started_at: string | null;
  finished_at: string | null;
  ip_addresses: string[];
  cpu_percent: number | null;
  memory_usage_bytes: number | null;
  memory_limit_bytes: number | null;
  memory_percent: number | null;
  net_rx_bytes: number | null;
  net_tx_bytes: number | null;
  block_read_bytes: number | null;
  block_write_bytes: number | null;
  pids: number | null;
};

export type AdminInstanceRuntimeMetricsSummary = {
  services_total: number;
  running_services: number;
  unhealthy_services: number;
  restarting_services: number;
  cpu_percent_total: number;
  memory_usage_bytes_total: number;
  memory_limit_bytes_total: number;
};

export type AdminInstanceRuntimeMetricsResponse = {
  generated_at: string;
  instance: AdminInstanceItem;
  summary: AdminInstanceRuntimeMetricsSummary;
  services: AdminInstanceRuntimeMetricsService[];
  warnings: string[];
};

export type AdminInstanceReaperRunResponse = {
  generated_at: string;
  mode: string;
  heartbeat_stale_seconds: number | null;
  scanned: number;
  reaped: number;
  failed: number;
  skipped: number;
};

export type AdminAuditLogItem = {
  id: number;
  actor_user_id: string | null;
  actor_username: string | null;
  actor_role: string;
  action: string;
  target_type: string;
  target_id: string | null;
  detail: Record<string, unknown>;
  created_at: string;
};

export type AdminRuntimeInstanceAlertItem = {
  id: string;
  contest_id: string;
  contest_title: string;
  challenge_id: string;
  challenge_title: string;
  team_id: string;
  team_name: string;
  status: string;
  expires_at: string | null;
  last_heartbeat_at: string | null;
  updated_at: string;
};

export type AdminRuntimeOverview = {
  generated_at: string;
  total_users: number;
  total_teams: number;
  total_contests: number;
  running_contests: number;
  total_challenges: number;
  total_submissions: number;
  submissions_last_24h: number;
  instances_total: number;
  instances_running: number;
  instances_failed: number;
  instances_expiring_within_30m: number;
  instances_expired_not_destroyed: number;
  recent_failed_instances: AdminRuntimeInstanceAlertItem[];
};

export type AdminRuntimeAlertItem = {
  id: string;
  alert_type: string;
  severity: string;
  status: string;
  source_type: string;
  source_id: string | null;
  fingerprint: string;
  title: string;
  message: string;
  detail: Record<string, unknown>;
  first_seen_at: string;
  last_seen_at: string;
  acknowledged_at: string | null;
  acknowledged_by: string | null;
  acknowledged_by_username: string | null;
  resolved_at: string | null;
  resolved_by: string | null;
  resolved_by_username: string | null;
  created_at: string;
  updated_at: string;
};

export type AdminRuntimeAlertScanResponse = {
  generated_at: string;
  upserted: number;
  auto_resolved: number;
  open_count: number;
  acknowledged_count: number;
  resolved_count: number;
};

export type AdminChallengeRuntimeLintItem = {
  id: string;
  title: string;
  slug: string;
  challenge_type: string;
  status: string;
  is_visible: boolean;
  has_compose_template: boolean;
  lint_status: string;
  message: string | null;
  updated_at: string;
};

export type AdminChallengeRuntimeLintResponse = {
  generated_at: string;
  scanned_total: number;
  returned_total: number;
  ok_count: number;
  error_count: number;
  items: AdminChallengeRuntimeLintItem[];
};

export type AdminChallengeRuntimeImageTestStep = {
  step: string;
  success: boolean;
  exit_code: number | null;
  duration_ms: number;
  output: string;
  truncated: boolean;
};

export type AdminChallengeRuntimeImageTestResponse = {
  image: string;
  force_pull: boolean;
  run_build_probe: boolean;
  succeeded: boolean;
  generated_at: string;
  steps: AdminChallengeRuntimeImageTestStep[];
};

export type AdminChallengeRuntimeImageTestStreamEvent =
  | {
      event: "start";
      image: string;
      force_pull: boolean;
      run_build_probe: boolean;
      timeout_seconds: number;
      generated_at: string;
    }
  | {
      event: "step_start";
      step: string;
      command: string;
      generated_at: string;
    }
  | {
      event: "step_log";
      step: string;
      stream: string;
      line: string;
      generated_at: string;
    }
  | {
      event: "step_finish";
      step: string;
      success: boolean;
      exit_code: number | null;
      duration_ms: number;
      truncated: boolean;
      generated_at: string;
    }
  | {
      event: "completed";
      result: AdminChallengeRuntimeImageTestResponse;
    }
  | {
      event: "error";
      message: string;
      step?: string | null;
      generated_at: string;
    };

export type WireguardConfigResponse = {
  contest_id: string;
  challenge_id: string;
  team_id: string;
  endpoint: string;
  filename: string;
  content: string;
};

export async function register(payload: {
  username: string;
  email: string;
  password: string;
  password_confirm: string;
  captcha_token?: string;
}): Promise<RegisterResponse> {
  try {
    const { data } = await api.post<RegisterResponse>("/auth/register", payload);
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function login(payload: {
  identifier: string;
  password: string;
  captcha_token?: string;
}): Promise<AuthResponse> {
  try {
    const { data } = await api.post<AuthResponse>("/auth/login", payload);
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function refresh(payload: { refresh_token: string }): Promise<AuthResponse> {
  try {
    const { data } = await api.post<AuthResponse>("/auth/refresh", payload);
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function me(accessToken: string): Promise<AuthUser> {
  try {
    const { data } = await api.get<AuthUser>("/auth/me", authHeaders(accessToken));
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function getPasswordPolicy(): Promise<PasswordPolicyResponse> {
  try {
    const { data } = await api.get<PasswordPolicyResponse>("/auth/password-policy");
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function getPublicSiteSettings(): Promise<SiteSettings> {
  try {
    const { data } = await api.get<SiteSettings>("/site/settings");
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function requestEmailVerification(payload: {
  email: string;
}): Promise<ActionMessageResponse> {
  try {
    const { data } = await api.post<ActionMessageResponse>("/auth/email-verification/request", payload);
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function confirmEmailVerification(payload: {
  token: string;
}): Promise<ActionMessageResponse> {
  try {
    const { data } = await api.post<ActionMessageResponse>("/auth/email-verification/confirm", payload);
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function requestPasswordReset(payload: {
  email: string;
}): Promise<ActionMessageResponse> {
  try {
    const { data } = await api.post<ActionMessageResponse>("/auth/password-reset/request", payload);
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function confirmPasswordReset(payload: {
  token: string;
  new_password: string;
  new_password_confirm: string;
}): Promise<ActionMessageResponse> {
  try {
    const { data } = await api.post<ActionMessageResponse>("/auth/password-reset/confirm", payload);
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function updateProfile(
  payload: {
    username?: string;
    email?: string;
  },
  accessToken: string
): Promise<AuthUser> {
  try {
    const { data } = await api.patch<AuthUser>("/auth/profile", payload, authHeaders(accessToken));
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function changePassword(
  payload: {
    current_password: string;
    new_password: string;
  },
  accessToken: string
): Promise<AuthResponse> {
  try {
    const { data } = await api.post<AuthResponse>("/auth/change-password", payload, authHeaders(accessToken));
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function getLoginHistory(
  accessToken: string,
  query?: { limit?: number }
): Promise<LoginHistoryItem[]> {
  try {
    const { data } = await api.get<LoginHistoryItem[]>("/auth/login-history", {
      ...authHeaders(accessToken),
      params: query
    });
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function deleteAccount(accessToken: string): Promise<void> {
  try {
    await api.delete("/auth/account", authHeaders(accessToken));
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function listContests(): Promise<ContestListItem[]> {
  try {
    const { data } = await api.get<ContestListItem[]>("/contests");
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function listContestChallenges(
  contestId: string,
  accessToken: string
): Promise<ContestChallengeItem[]> {
  try {
    const { data } = await api.get<ContestChallengeItem[]>(
      `/contests/${contestId}/challenges`,
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function listContestAnnouncements(
  contestId: string,
  accessToken: string
): Promise<ContestAnnouncementItem[]> {
  try {
    const { data } = await api.get<ContestAnnouncementItem[]>(
      `/contests/${contestId}/announcements`,
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function listTeams(
  accessToken: string,
  query?: { keyword?: string; limit?: number }
): Promise<TeamListItem[]> {
  try {
    const { data } = await api.get<TeamListItem[]>("/teams", {
      ...authHeaders(accessToken),
      params: query
    });
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function getMyTeam(accessToken: string): Promise<MyTeamResponse> {
  try {
    const { data } = await api.get<MyTeamResponse>("/teams/me", authHeaders(accessToken));
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function createTeam(
  payload: { name: string; description?: string },
  accessToken: string
): Promise<TeamProfile> {
  try {
    const { data } = await api.post<TeamProfile>("/teams", payload, authHeaders(accessToken));
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function joinTeam(
  payload: { team_id?: string; team_name?: string },
  accessToken: string
): Promise<TeamProfile> {
  try {
    const { data } = await api.post<TeamProfile>("/teams/join", payload, authHeaders(accessToken));
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function getTeamById(teamId: string, accessToken: string): Promise<TeamProfile> {
  try {
    const { data } = await api.get<TeamProfile>(`/teams/${teamId}`, authHeaders(accessToken));
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function updateTeam(
  teamId: string,
  payload: { name?: string; description?: string },
  accessToken: string
): Promise<TeamProfile> {
  try {
    const { data } = await api.patch<TeamProfile>(`/teams/${teamId}`, payload, authHeaders(accessToken));
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function transferTeamCaptain(
  teamId: string,
  newCaptainUserId: string,
  accessToken: string
): Promise<TeamProfile> {
  try {
    const { data } = await api.post<TeamProfile>(
      `/teams/${teamId}/transfer-captain`,
      { new_captain_user_id: newCaptainUserId },
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function removeTeamMember(
  teamId: string,
  memberUserId: string,
  accessToken: string
): Promise<TeamProfile> {
  try {
    const { data } = await api.delete<TeamProfile>(`/teams/${teamId}/members/${memberUserId}`, authHeaders(accessToken));
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function disbandTeam(teamId: string, accessToken: string): Promise<void> {
  try {
    await api.delete(`/teams/${teamId}`, authHeaders(accessToken));
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function leaveTeam(accessToken: string): Promise<LeaveTeamResponse> {
  try {
    const { data } = await api.post<LeaveTeamResponse>("/teams/leave", {}, authHeaders(accessToken));
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function createTeamInvitation(
  payload: { invitee_user_id?: string; invitee_username?: string; message?: string },
  accessToken: string
): Promise<TeamInvitationItem> {
  try {
    const { data } = await api.post<TeamInvitationItem>("/teams/invitations", payload, authHeaders(accessToken));
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function listReceivedTeamInvitations(
  accessToken: string,
  query?: { status?: string; limit?: number }
): Promise<TeamInvitationItem[]> {
  try {
    const { data } = await api.get<TeamInvitationItem[]>("/teams/invitations/received", {
      ...authHeaders(accessToken),
      params: query
    });
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function listSentTeamInvitations(
  accessToken: string,
  query?: { status?: string; limit?: number }
): Promise<TeamInvitationItem[]> {
  try {
    const { data } = await api.get<TeamInvitationItem[]>("/teams/invitations/sent", {
      ...authHeaders(accessToken),
      params: query
    });
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function respondTeamInvitation(
  invitationId: string,
  action: "accept" | "reject",
  accessToken: string
): Promise<TeamInvitationRespondResponse> {
  try {
    const { data } = await api.post<TeamInvitationRespondResponse>(
      `/teams/invitations/${invitationId}/respond`,
      { action },
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function cancelTeamInvitation(
  invitationId: string,
  accessToken: string
): Promise<TeamInvitationItem> {
  try {
    const { data } = await api.post<TeamInvitationItem>(
      `/teams/invitations/${invitationId}/cancel`,
      {},
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function submitFlag(
  payload: {
    contest_id: string;
    challenge_id: string;
    flag: string;
  },
  accessToken: string
): Promise<SubmitFlagResponse> {
  try {
    const { data } = await api.post<SubmitFlagResponse>("/submissions", payload, authHeaders(accessToken));
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function getScoreboard(
  contestId: string,
  accessToken: string
): Promise<ScoreboardEntry[]> {
  try {
    const { data } = await api.get<ScoreboardEntry[]>(
      `/contests/${contestId}/scoreboard`,
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function getScoreboardTimeline(
  contestId: string,
  accessToken: string,
  query?: { max_snapshots?: number; top_n?: number }
): Promise<ScoreboardTimelineResponse> {
  try {
    const { data } = await api.get<ScoreboardTimelineResponse>(
      `/contests/${contestId}/scoreboard/timeline`,
      {
        ...authHeaders(accessToken),
        params: query
      }
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function getScoreboardRankings(
  contestId: string,
  accessToken: string
): Promise<ScoreboardRankingsResponse> {
  try {
    const { data } = await api.get<ScoreboardRankingsResponse>(
      `/contests/${contestId}/scoreboard/rankings`,
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export function buildScoreboardWsUrl(contestId: string, accessToken: string): string {
  const url = new URL(API_BASE_URL);
  const protocol = url.protocol === "https:" ? "wss:" : "ws:";
  return `${protocol}//${url.host}/api/v1/contests/${contestId}/scoreboard/ws?access_token=${encodeURIComponent(accessToken)}`;
}

export async function startInstance(
  payload: { contest_id: string; challenge_id: string },
  accessToken: string
): Promise<InstanceResponse> {
  try {
    const { data } = await api.post<InstanceResponse>("/instances/start", payload, authHeaders(accessToken));
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function stopInstance(
  payload: { contest_id: string; challenge_id: string },
  accessToken: string
): Promise<InstanceResponse> {
  try {
    const { data } = await api.post<InstanceResponse>("/instances/stop", payload, authHeaders(accessToken));
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function resetInstance(
  payload: { contest_id: string; challenge_id: string },
  accessToken: string
): Promise<InstanceResponse> {
  try {
    const { data } = await api.post<InstanceResponse>("/instances/reset", payload, authHeaders(accessToken));
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function destroyInstance(
  payload: { contest_id: string; challenge_id: string },
  accessToken: string
): Promise<InstanceResponse> {
  try {
    const { data } = await api.post<InstanceResponse>("/instances/destroy", payload, authHeaders(accessToken));
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function getInstance(
  contestId: string,
  challengeId: string,
  accessToken: string
): Promise<InstanceResponse> {
  try {
    const { data } = await api.get<InstanceResponse>(
      `/instances/${contestId}/${challengeId}`,
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function getInstanceWireguardConfig(
  contestId: string,
  challengeId: string,
  accessToken: string
): Promise<WireguardConfigResponse> {
  try {
    const { data } = await api.get<WireguardConfigResponse>(
      `/instances/${contestId}/${challengeId}/wireguard-config`,
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function listAdminChallengeCategories(
  accessToken: string
): Promise<AdminChallengeCategoryItem[]> {
  try {
    const { data } = await api.get<AdminChallengeCategoryItem[]>(
      "/admin/challenge-categories",
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function createAdminChallengeCategory(
  payload: {
    slug: string;
    display_name?: string;
    sort_order?: number;
  },
  accessToken: string
): Promise<AdminChallengeCategoryItem> {
  try {
    const { data } = await api.post<AdminChallengeCategoryItem>(
      "/admin/challenge-categories",
      payload,
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function updateAdminChallengeCategory(
  categoryId: string,
  payload: {
    slug?: string;
    display_name?: string;
    sort_order?: number;
  },
  accessToken: string
): Promise<AdminChallengeCategoryItem> {
  try {
    const { data } = await api.patch<AdminChallengeCategoryItem>(
      `/admin/challenge-categories/${categoryId}`,
      payload,
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function deleteAdminChallengeCategory(
  categoryId: string,
  accessToken: string
): Promise<void> {
  try {
    await api.delete(`/admin/challenge-categories/${categoryId}`, authHeaders(accessToken));
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function listAdminChallenges(accessToken: string): Promise<AdminChallengeItem[]> {
  try {
    const { data } = await api.get<AdminChallengeItem[]>("/admin/challenges", authHeaders(accessToken));
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function getAdminChallengeDetail(
  challengeId: string,
  accessToken: string
): Promise<AdminChallengeDetailItem> {
  try {
    const { data } = await api.get<AdminChallengeDetailItem>(
      `/admin/challenges/${challengeId}`,
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function createAdminChallenge(
  payload: {
    title: string;
    slug: string;
    category: string;
    description?: string;
    difficulty?: string;
    static_score?: number;
    challenge_type?: string;
    flag_mode?: string;
    status?: string;
    flag_hash?: string;
    is_visible?: boolean;
    compose_template?: string;
    tags?: string[];
    writeup_visibility?: string;
    writeup_content?: string;
    change_note?: string;
    metadata?: Record<string, unknown>;
  },
  accessToken: string
): Promise<AdminChallengeItem> {
  try {
    const { data } = await api.post<AdminChallengeItem>("/admin/challenges", payload, authHeaders(accessToken));
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function updateAdminChallenge(
  challengeId: string,
  payload: {
    title?: string;
    slug?: string;
    category?: string;
    description?: string;
    difficulty?: string;
    static_score?: number;
    challenge_type?: string;
    flag_mode?: string;
    status?: string;
    flag_hash?: string;
    is_visible?: boolean;
    compose_template?: string;
    tags?: string[];
    writeup_visibility?: string;
    writeup_content?: string;
    change_note?: string;
    metadata?: Record<string, unknown>;
  },
  accessToken: string
): Promise<AdminChallengeItem> {
  try {
    const { data } = await api.patch<AdminChallengeItem>(
      `/admin/challenges/${challengeId}`,
      payload,
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function deleteAdminChallenge(
  challengeId: string,
  accessToken: string
): Promise<void> {
  try {
    await api.delete(`/admin/challenges/${challengeId}`, authHeaders(accessToken));
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function listAdminChallengeVersions(
  challengeId: string,
  accessToken: string,
  query?: { limit?: number }
): Promise<AdminChallengeVersionItem[]> {
  try {
    const { data } = await api.get<AdminChallengeVersionItem[]>(
      `/admin/challenges/${challengeId}/versions`,
      {
        ...authHeaders(accessToken),
        params: query
      }
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function rollbackAdminChallengeVersion(
  challengeId: string,
  payload: { version_no: number; change_note?: string },
  accessToken: string
): Promise<AdminChallengeItem> {
  try {
    const { data } = await api.post<AdminChallengeItem>(
      `/admin/challenges/${challengeId}/rollback`,
      payload,
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function listAdminChallengeAttachments(
  challengeId: string,
  accessToken: string,
  query?: { limit?: number }
): Promise<AdminChallengeAttachmentItem[]> {
  try {
    const { data } = await api.get<AdminChallengeAttachmentItem[]>(
      `/admin/challenges/${challengeId}/attachments`,
      {
        ...authHeaders(accessToken),
        params: query
      }
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function uploadAdminChallengeAttachment(
  challengeId: string,
  payload: { filename: string; content_base64: string; content_type?: string },
  accessToken: string
): Promise<AdminChallengeAttachmentItem> {
  try {
    const { data } = await api.post<AdminChallengeAttachmentItem>(
      `/admin/challenges/${challengeId}/attachments`,
      payload,
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function deleteAdminChallengeAttachment(
  challengeId: string,
  attachmentId: string,
  accessToken: string
): Promise<void> {
  try {
    await api.delete(
      `/admin/challenges/${challengeId}/attachments/${attachmentId}`,
      authHeaders(accessToken)
    );
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function listAdminContests(accessToken: string): Promise<AdminContestItem[]> {
  try {
    const { data } = await api.get<AdminContestItem[]>("/admin/contests", authHeaders(accessToken));
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function listAdminUsers(
  accessToken: string,
  query?: {
    keyword?: string;
    role?: string;
    status?: string;
    limit?: number;
  }
): Promise<AdminUserItem[]> {
  try {
    const { data } = await api.get<AdminUserItem[]>("/admin/users", {
      ...authHeaders(accessToken),
      params: query
    });
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function getAdminSiteSettings(accessToken: string): Promise<AdminSiteSettings> {
  try {
    const { data } = await api.get<AdminSiteSettings>("/admin/site-settings", authHeaders(accessToken));
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function updateAdminSiteSettings(
  payload: {
    site_name?: string;
    site_subtitle?: string;
    home_title?: string;
    home_tagline?: string;
    home_signature?: string;
    footer_text?: string;
    challenge_attachment_max_bytes?: number;
  },
  accessToken: string
): Promise<AdminSiteSettings> {
  try {
    const { data } = await api.patch<AdminSiteSettings>(
      "/admin/site-settings",
      payload,
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function updateAdminUserStatus(
  userId: string,
  status: string,
  accessToken: string
): Promise<AdminUserItem> {
  try {
    const { data } = await api.patch<AdminUserItem>(
      `/admin/users/${userId}/status`,
      { status },
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function updateAdminUserRole(
  userId: string,
  role: string,
  accessToken: string
): Promise<AdminUserItem> {
  try {
    const { data } = await api.patch<AdminUserItem>(
      `/admin/users/${userId}/role`,
      { role },
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function resetAdminUserPassword(
  userId: string,
  newPassword: string,
  accessToken: string
): Promise<AdminUserItem> {
  try {
    const { data } = await api.post<AdminUserItem>(
      `/admin/users/${userId}/reset-password`,
      { new_password: newPassword },
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function deleteAdminUser(
  userId: string,
  accessToken: string
): Promise<void> {
  try {
    await api.delete(`/admin/users/${userId}`, authHeaders(accessToken));
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function createAdminContest(
  payload: {
    title: string;
    slug: string;
    description?: string;
    visibility?: string;
    status?: string;
    scoring_mode?: string;
    dynamic_decay?: number;
    first_blood_bonus_percent?: number;
    second_blood_bonus_percent?: number;
    third_blood_bonus_percent?: number;
    start_at: string;
    end_at: string;
    freeze_at?: string | null;
  },
  accessToken: string
): Promise<AdminContestItem> {
  try {
    const { data } = await api.post<AdminContestItem>("/admin/contests", payload, authHeaders(accessToken));
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function updateAdminContest(
  contestId: string,
  payload: {
    title?: string;
    slug?: string;
    description?: string;
    visibility?: string;
    status?: string;
    scoring_mode?: string;
    dynamic_decay?: number;
    first_blood_bonus_percent?: number;
    second_blood_bonus_percent?: number;
    third_blood_bonus_percent?: number;
    start_at?: string;
    end_at?: string;
    freeze_at?: string | null;
    clear_freeze_at?: boolean;
  },
  accessToken: string
): Promise<AdminContestItem> {
  try {
    const { data } = await api.patch<AdminContestItem>(
      `/admin/contests/${contestId}`,
      payload,
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function updateAdminContestStatus(
  contestId: string,
  status: string,
  accessToken: string
): Promise<AdminContestItem> {
  try {
    const { data } = await api.patch<AdminContestItem>(
      `/admin/contests/${contestId}/status`,
      { status },
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function deleteAdminContest(
  contestId: string,
  accessToken: string
): Promise<void> {
  try {
    await api.delete(`/admin/contests/${contestId}`, authHeaders(accessToken));
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function uploadAdminContestPoster(
  contestId: string,
  payload: { filename: string; content_base64: string; content_type?: string },
  accessToken: string
): Promise<AdminContestItem> {
  try {
    const { data } = await api.post<AdminContestItem>(
      `/admin/contests/${contestId}/poster`,
      payload,
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function deleteAdminContestPoster(
  contestId: string,
  accessToken: string
): Promise<void> {
  try {
    await api.delete(`/admin/contests/${contestId}/poster`, authHeaders(accessToken));
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function listAdminContestAnnouncements(
  contestId: string,
  accessToken: string,
  query?: { limit?: number }
): Promise<AdminContestAnnouncementItem[]> {
  try {
    const { data } = await api.get<AdminContestAnnouncementItem[]>(
      `/admin/contests/${contestId}/announcements`,
      {
        ...authHeaders(accessToken),
        params: query
      }
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function createAdminContestAnnouncement(
  contestId: string,
  payload: {
    title: string;
    content: string;
    is_published?: boolean;
    is_pinned?: boolean;
  },
  accessToken: string
): Promise<AdminContestAnnouncementItem> {
  try {
    const { data } = await api.post<AdminContestAnnouncementItem>(
      `/admin/contests/${contestId}/announcements`,
      payload,
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function updateAdminContestAnnouncement(
  contestId: string,
  announcementId: string,
  payload: {
    title?: string;
    content?: string;
    is_published?: boolean;
    is_pinned?: boolean;
  },
  accessToken: string
): Promise<AdminContestAnnouncementItem> {
  try {
    const { data } = await api.patch<AdminContestAnnouncementItem>(
      `/admin/contests/${contestId}/announcements/${announcementId}`,
      payload,
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function deleteAdminContestAnnouncement(
  contestId: string,
  announcementId: string,
  accessToken: string
): Promise<void> {
  try {
    await api.delete(
      `/admin/contests/${contestId}/announcements/${announcementId}`,
      authHeaders(accessToken)
    );
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function listAdminInstances(
  accessToken: string,
  query?: { status?: string; limit?: number }
): Promise<AdminInstanceItem[]> {
  try {
    const { data } = await api.get<AdminInstanceItem[]>("/admin/instances", {
      ...authHeaders(accessToken),
      params: query
    });
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function getAdminInstanceRuntimeMetrics(
  instanceId: string,
  accessToken: string
): Promise<AdminInstanceRuntimeMetricsResponse> {
  try {
    const { data } = await api.get<AdminInstanceRuntimeMetricsResponse>(
      `/admin/instances/${instanceId}/runtime-metrics`,
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function listAdminAuditLogs(
  accessToken: string,
  query?: {
    action?: string;
    target_type?: string;
    actor_user_id?: string;
    limit?: number;
  }
): Promise<AdminAuditLogItem[]> {
  try {
    const { data } = await api.get<AdminAuditLogItem[]>("/admin/audit-logs", {
      ...authHeaders(accessToken),
      params: query
    });
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function getAdminRuntimeOverview(accessToken: string): Promise<AdminRuntimeOverview> {
  try {
    const { data } = await api.get<AdminRuntimeOverview>("/admin/runtime/overview", authHeaders(accessToken));
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function listAdminRuntimeAlerts(
  accessToken: string,
  query?: {
    status?: string;
    severity?: string;
    alert_type?: string;
    limit?: number;
  }
): Promise<AdminRuntimeAlertItem[]> {
  try {
    const { data } = await api.get<AdminRuntimeAlertItem[]>("/admin/runtime/alerts", {
      ...authHeaders(accessToken),
      params: query
    });
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function scanAdminRuntimeAlerts(
  accessToken: string
): Promise<AdminRuntimeAlertScanResponse> {
  try {
    const { data } = await api.post<AdminRuntimeAlertScanResponse>(
      "/admin/runtime/alerts/scan",
      {},
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function runAdminExpiredInstanceReaper(
  accessToken: string
): Promise<AdminInstanceReaperRunResponse> {
  try {
    const { data } = await api.post<AdminInstanceReaperRunResponse>(
      "/admin/runtime/reaper/expired",
      {},
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function runAdminStaleInstanceReaper(
  accessToken: string
): Promise<AdminInstanceReaperRunResponse> {
  try {
    const { data } = await api.post<AdminInstanceReaperRunResponse>(
      "/admin/runtime/reaper/stale",
      {},
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function acknowledgeAdminRuntimeAlert(
  alertId: string,
  accessToken: string,
  payload?: { note?: string }
): Promise<AdminRuntimeAlertItem> {
  try {
    const { data } = await api.post<AdminRuntimeAlertItem>(
      `/admin/runtime/alerts/${alertId}/ack`,
      payload ?? {},
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function resolveAdminRuntimeAlert(
  alertId: string,
  accessToken: string,
  payload?: { note?: string }
): Promise<AdminRuntimeAlertItem> {
  try {
    const { data } = await api.post<AdminRuntimeAlertItem>(
      `/admin/runtime/alerts/${alertId}/resolve`,
      payload ?? {},
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function listAdminChallengeRuntimeTemplateLint(
  accessToken: string,
  query?: {
    limit?: number;
    challenge_type?: string;
    status?: string;
    keyword?: string;
    only_errors?: boolean;
  }
): Promise<AdminChallengeRuntimeLintResponse> {
  try {
    const { data } = await api.get<AdminChallengeRuntimeLintResponse>(
      "/admin/challenges/runtime-template/lint",
      {
        ...authHeaders(accessToken),
        params: query
      }
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function testAdminChallengeRuntimeImage(
  payload: {
    image: string;
    force_pull?: boolean;
    run_build_probe?: boolean;
    timeout_seconds?: number;
  },
  accessToken: string
): Promise<AdminChallengeRuntimeImageTestResponse> {
  try {
    const { data } = await api.post<AdminChallengeRuntimeImageTestResponse>(
      "/admin/challenges/runtime-template/test-image",
      payload,
      {
        ...authHeaders(accessToken),
        timeout: 10 * 60 * 1000
      }
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function streamAdminChallengeRuntimeImageTest(
  payload: {
    image: string;
    force_pull?: boolean;
    run_build_probe?: boolean;
    timeout_seconds?: number;
  },
  accessToken: string,
  options?: {
    onEvent?: (event: AdminChallengeRuntimeImageTestStreamEvent) => void;
  }
): Promise<AdminChallengeRuntimeImageTestResponse> {
  try {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/admin/challenges/runtime-template/test-image/stream`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${accessToken}`
        },
        body: JSON.stringify(payload)
      }
    );

    if (!response.ok) {
      let message = `request failed (${response.status})`;
      let code = "request_failed";
      try {
        const data = (await response.json()) as ApiErrorEnvelope;
        message = data.error?.message ?? message;
        code = data.error?.code ?? code;
      } catch {
        try {
          const text = await response.text();
          if (text.trim()) {
            message = text.trim();
          }
        } catch {
          // ignore fallback parsing failures
        }
      }
      throw new ApiClientError(message, code);
    }

    if (!response.body) {
      throw new ApiClientError("stream response body is empty", "request_failed");
    }

    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let buffer = "";
    let completed: AdminChallengeRuntimeImageTestResponse | null = null;

    const emit = (event: AdminChallengeRuntimeImageTestStreamEvent) => {
      options?.onEvent?.(event);
      if (event.event === "completed") {
        completed = event.result;
        return;
      }
      if (event.event === "error") {
        throw new ApiClientError(event.message || "image test stream failed", "request_failed");
      }
    };

    const flushLines = (flushTail: boolean) => {
      while (true) {
        const newlineIndex = buffer.indexOf("\n");
        if (newlineIndex < 0) {
          break;
        }
        const raw = buffer.slice(0, newlineIndex);
        buffer = buffer.slice(newlineIndex + 1);
        const line = raw.trim();
        if (!line) {
          continue;
        }
        let event: AdminChallengeRuntimeImageTestStreamEvent;
        try {
          event = JSON.parse(line) as AdminChallengeRuntimeImageTestStreamEvent;
        } catch {
          throw new ApiClientError("invalid image test stream event payload", "request_failed");
        }
        emit(event);
      }

      if (flushTail) {
        const tail = buffer.trim();
        buffer = "";
        if (!tail) {
          return;
        }
        let event: AdminChallengeRuntimeImageTestStreamEvent;
        try {
          event = JSON.parse(tail) as AdminChallengeRuntimeImageTestStreamEvent;
        } catch {
          throw new ApiClientError("invalid image test stream tail payload", "request_failed");
        }
        emit(event);
      }
    };

    while (true) {
      const { done, value } = await reader.read();
      if (done) {
        buffer += decoder.decode();
        flushLines(true);
        break;
      }
      buffer += decoder.decode(value, { stream: true });
      flushLines(false);
    }

    if (!completed) {
      throw new ApiClientError(
        "image test stream finished without completion event",
        "request_failed"
      );
    }
    return completed;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function listAdminContestChallenges(
  contestId: string,
  accessToken: string
): Promise<AdminContestChallengeItem[]> {
  try {
    const { data } = await api.get<AdminContestChallengeItem[]>(
      `/admin/contests/${contestId}/challenges`,
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function upsertAdminContestChallenge(
  contestId: string,
  payload: {
    challenge_id: string;
    sort_order?: number;
    release_at?: string | null;
  },
  accessToken: string
): Promise<AdminContestChallengeItem> {
  try {
    const { data } = await api.post<AdminContestChallengeItem>(
      `/admin/contests/${contestId}/challenges`,
      payload,
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function updateAdminContestChallenge(
  contestId: string,
  challengeId: string,
  payload: {
    sort_order?: number;
    release_at?: string | null;
    clear_release_at?: boolean;
  },
  accessToken: string
): Promise<AdminContestChallengeItem> {
  try {
    const { data } = await api.patch<AdminContestChallengeItem>(
      `/admin/contests/${contestId}/challenges/${challengeId}`,
      payload,
      authHeaders(accessToken)
    );
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function deleteAdminContestChallenge(
  contestId: string,
  challengeId: string,
  accessToken: string
): Promise<void> {
  try {
    await api.delete(`/admin/contests/${contestId}/challenges/${challengeId}`, authHeaders(accessToken));
  } catch (error) {
    throw toApiClientError(error);
  }
}
