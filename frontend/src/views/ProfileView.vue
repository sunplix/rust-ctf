<template>
  <section class="page-block">
    <div class="row-between">
      <div>
        <h1>账户中心</h1>
        <p class="muted">维护个人资料、修改密码并查看最近登录记录。</p>
      </div>
      <RouterLink class="ghost-link" to="/contests">返回比赛中心</RouterLink>
    </div>

    <p v-if="pageError" class="error">{{ pageError }}</p>

    <div class="team-layout">
      <section class="panel team-panel">
        <h2>个人资料</h2>
        <form class="form-grid" @submit.prevent="handleUpdateProfile">
          <label>
            <span>用户名</span>
            <input v-model.trim="profileForm.username" maxlength="32" required />
          </label>
          <label>
            <span>邮箱</span>
            <input v-model.trim="profileForm.email" type="email" maxlength="128" required />
          </label>
          <button class="primary" type="submit" :disabled="updatingProfile">
            {{ updatingProfile ? "保存中..." : "保存资料" }}
          </button>
        </form>
      </section>

      <section class="panel team-panel">
        <h2>修改密码</h2>
        <form class="form-grid" @submit.prevent="handleChangePassword">
          <label>
            <span>当前密码</span>
            <input v-model="passwordForm.current_password" type="password" autocomplete="current-password" required />
          </label>
          <label>
            <span>新密码</span>
            <input v-model="passwordForm.new_password" type="password" autocomplete="new-password" minlength="8" required />
          </label>
          <button class="primary" type="submit" :disabled="changingPassword">
            {{ changingPassword ? "提交中..." : "修改密码" }}
          </button>
        </form>
      </section>
    </div>

    <section class="panel team-panel">
      <div class="row-between">
        <h2>登录历史</h2>
        <button class="ghost" type="button" @click="loadHistory" :disabled="loadingHistory">
          {{ loadingHistory ? "加载中..." : "刷新" }}
        </button>
      </div>

      <p v-if="historyError" class="error">{{ historyError }}</p>

      <table v-if="history.length > 0" class="scoreboard-table">
        <thead>
          <tr>
            <th>时间</th>
            <th>动作</th>
            <th>来源信息</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="item in history" :key="item.id">
            <td>{{ formatTime(item.created_at) }}</td>
            <td class="mono">{{ item.action }}</td>
            <td class="mono">{{ summarizeDetail(item.detail) }}</td>
          </tr>
        </tbody>
      </table>
      <p v-else class="muted">暂无登录历史记录。</p>
    </section>

    <section class="panel team-panel">
      <h2>危险操作</h2>
      <p class="muted">删除账号后将立即失效并退出登录，且无法恢复。</p>
      <button class="danger" type="button" @click="handleDeleteAccount" :disabled="deletingAccount">
        {{ deletingAccount ? "删除中..." : "删除我的账号" }}
      </button>
    </section>
  </section>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from "vue";
import { useRouter } from "vue-router";

import {
  ApiClientError,
  changePassword,
  deleteAccount,
  getLoginHistory,
  type LoginHistoryItem,
  updateProfile
} from "../api/client";
import { useAuthStore } from "../stores/auth";
import { useUiStore } from "../stores/ui";

const authStore = useAuthStore();
const uiStore = useUiStore();
const router = useRouter();

const pageError = ref("");

const updatingProfile = ref(false);
const changingPassword = ref(false);
const deletingAccount = ref(false);
const loadingHistory = ref(false);
const historyError = ref("");

const history = ref<LoginHistoryItem[]>([]);

const profileForm = reactive({
  username: authStore.user?.username ?? "",
  email: authStore.user?.email ?? ""
});

const passwordForm = reactive({
  current_password: "",
  new_password: ""
});

function requireAccessToken() {
  const token = authStore.accessToken;
  if (!token) {
    throw new ApiClientError("未登录或会话已失效", "unauthorized");
  }
  return token;
}

function formatTime(input: string) {
  return new Date(input).toLocaleString();
}

function summarizeDetail(detail: Record<string, unknown>) {
  const requestValue = detail.request;
  if (!requestValue || typeof requestValue !== "object") {
    return "-";
  }

  const request = requestValue as Record<string, unknown>;
  const ua = typeof request.user_agent === "string" ? request.user_agent : "-";
  const xff = typeof request.x_forwarded_for === "string" ? request.x_forwarded_for : "-";
  const xreal = typeof request.x_real_ip === "string" ? request.x_real_ip : "-";

  const text = `ip=${xff !== "-" ? xff : xreal} ua=${ua}`;
  return text.length > 180 ? `${text.slice(0, 180)}...` : text;
}

async function handleUpdateProfile() {
  updatingProfile.value = true;
  pageError.value = "";

  try {
    const token = requireAccessToken();
    const updated = await updateProfile(
      {
        username: profileForm.username,
        email: profileForm.email
      },
      token
    );

    authStore.user = updated;
    uiStore.success("资料已更新", "用户名和邮箱已保存。");
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : "更新资料失败";
    pageError.value = message;
    uiStore.error("更新资料失败", message);
  } finally {
    updatingProfile.value = false;
  }
}

async function handleChangePassword() {
  changingPassword.value = true;
  pageError.value = "";

  try {
    const token = requireAccessToken();
    const authResponse = await changePassword(
      {
        current_password: passwordForm.current_password,
        new_password: passwordForm.new_password
      },
      token
    );

    authStore.applyAuthResponse(authResponse);
    passwordForm.current_password = "";
    passwordForm.new_password = "";
    uiStore.success("密码已更新", "会话已刷新，请使用新密码登录。", 3200);
    await loadHistory();
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : "修改密码失败";
    pageError.value = message;
    uiStore.error("修改密码失败", message);
  } finally {
    changingPassword.value = false;
  }
}

async function loadHistory() {
  loadingHistory.value = true;
  historyError.value = "";

  try {
    const token = requireAccessToken();
    history.value = await getLoginHistory(token, { limit: 50 });
  } catch (err) {
    historyError.value = err instanceof ApiClientError ? err.message : "加载登录历史失败";
  } finally {
    loadingHistory.value = false;
  }
}

async function handleDeleteAccount() {
  if (!window.confirm("确认删除当前账号？该操作不可恢复。")) {
    return;
  }

  deletingAccount.value = true;
  pageError.value = "";

  try {
    const token = requireAccessToken();
    await deleteAccount(token);
    const username = authStore.user?.username ?? "当前账号";
    authStore.clearSession();
    uiStore.warning("账号已删除", `${username} 已被删除并退出登录。`, 3600);
    router.replace("/login");
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : "删除账号失败";
    pageError.value = message;
    uiStore.error("删除账号失败", message);
  } finally {
    deletingAccount.value = false;
  }
}

onMounted(async () => {
  profileForm.username = authStore.user?.username ?? "";
  profileForm.email = authStore.user?.email ?? "";
  await loadHistory();
});
</script>
