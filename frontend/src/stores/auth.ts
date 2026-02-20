import { computed, ref } from "vue";
import { defineStore } from "pinia";

import {
  type AuthResponse,
  type AuthUser,
  type RegisterResponse,
  ApiClientError,
  login,
  me,
  refresh,
  register
} from "../api/client";

const AUTH_STORAGE_KEY = "rust-ctf.auth";

type PersistedAuth = {
  accessToken: string;
  refreshToken: string;
  user: AuthUser;
};

export const useAuthStore = defineStore("auth", () => {
  const accessToken = ref("");
  const refreshToken = ref("");
  const user = ref<AuthUser | null>(null);
  const hydrated = ref(false);

  const isAuthenticated = computed(() => !!accessToken.value && !!user.value);

  function persist() {
    if (!accessToken.value || !refreshToken.value || !user.value) {
      localStorage.removeItem(AUTH_STORAGE_KEY);
      return;
    }

    const payload: PersistedAuth = {
      accessToken: accessToken.value,
      refreshToken: refreshToken.value,
      user: user.value
    };
    localStorage.setItem(AUTH_STORAGE_KEY, JSON.stringify(payload));
  }

  function hydrateFromStorage() {
    if (hydrated.value) {
      return;
    }

    hydrated.value = true;
    const raw = localStorage.getItem(AUTH_STORAGE_KEY);
    if (!raw) {
      return;
    }

    try {
      const parsed = JSON.parse(raw) as PersistedAuth;
      if (parsed.accessToken && parsed.refreshToken && parsed.user) {
        if (typeof parsed.user.email_verified !== "boolean") {
          parsed.user.email_verified = true;
        }
        if (typeof parsed.user.email_verified_at === "undefined") {
          parsed.user.email_verified_at = null;
        }
        accessToken.value = parsed.accessToken;
        refreshToken.value = parsed.refreshToken;
        user.value = parsed.user;
      }
    } catch {
      localStorage.removeItem(AUTH_STORAGE_KEY);
    }
  }

  function applySession(response: AuthResponse) {
    accessToken.value = response.access_token;
    refreshToken.value = response.refresh_token;
    user.value = response.user;
    persist();
  }

  function applyAuthResponse(response: AuthResponse) {
    applySession(response);
  }

  function clearSession() {
    accessToken.value = "";
    refreshToken.value = "";
    user.value = null;
    persist();
  }

  async function registerWithPassword(payload: {
    username: string;
    email: string;
    password: string;
    password_confirm: string;
    captcha_token?: string;
  }): Promise<RegisterResponse> {
    const response = await register(payload);
    if (response.auth) {
      applySession(response.auth);
    } else {
      clearSession();
    }
    return response;
  }

  async function loginWithPassword(payload: {
    identifier: string;
    password: string;
    captcha_token?: string;
  }) {
    const response = await login(payload);
    applySession(response);
    return response.user;
  }

  async function syncCurrentUser() {
    if (!accessToken.value) {
      return null;
    }

    try {
      const currentUser = await me(accessToken.value);
      user.value = currentUser;
      persist();
      return currentUser;
    } catch (error) {
      const apiError = error as ApiClientError;
      if (apiError.code === "unauthorized" && refreshToken.value) {
        try {
          const refreshed = await refresh({ refresh_token: refreshToken.value });
          applySession(refreshed);
          return refreshed.user;
        } catch {
          clearSession();
          return null;
        }
      }

      throw error;
    }
  }

  return {
    accessToken,
    refreshToken,
    user,
    hydrated,
    isAuthenticated,
    hydrateFromStorage,
    registerWithPassword,
    loginWithPassword,
    applyAuthResponse,
    syncCurrentUser,
    clearSession
  };
});
