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

export type AuthUser = {
  id: string;
  username: string;
  email: string;
  role: string;
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

export type ContestListItem = {
  id: string;
  title: string;
  slug: string;
  status: string;
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
  is_visible: boolean;
  created_at: string;
  updated_at: string;
};

export type AdminContestItem = {
  id: string;
  title: string;
  slug: string;
  description: string;
  visibility: string;
  status: string;
  start_at: string;
  end_at: string;
  freeze_at: string | null;
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

export async function register(payload: {
  username: string;
  email: string;
  password: string;
}): Promise<AuthResponse> {
  try {
    const { data } = await api.post<AuthResponse>("/auth/register", payload);
    return data;
  } catch (error) {
    throw toApiClientError(error);
  }
}

export async function login(payload: {
  identifier: string;
  password: string;
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

export async function listAdminChallenges(accessToken: string): Promise<AdminChallengeItem[]> {
  try {
    const { data } = await api.get<AdminChallengeItem[]>("/admin/challenges", authHeaders(accessToken));
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
    difficulty?: string;
    static_score?: number;
    challenge_type?: string;
    flag_mode?: string;
    flag_hash?: string;
    is_visible?: boolean;
    compose_template?: string;
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
    difficulty?: string;
    static_score?: number;
    challenge_type?: string;
    flag_mode?: string;
    flag_hash?: string;
    is_visible?: boolean;
    compose_template?: string;
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

export async function listAdminContests(accessToken: string): Promise<AdminContestItem[]> {
  try {
    const { data } = await api.get<AdminContestItem[]>("/admin/contests", authHeaders(accessToken));
    return data;
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
