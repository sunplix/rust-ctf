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
