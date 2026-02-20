<template>
  <section class="page-layout">
    <article class="surface stack">
      <header class="section-head">
        <div class="section-title">
          <h1>{{ tr("账户设置", "Account Settings") }}</h1>
        </div>
        <RouterLink class="btn-line" to="/contests">{{ tr("返回比赛", "Back to contests") }}</RouterLink>
      </header>
      <p v-if="pageError" class="error">{{ pageError }}</p>
    </article>

    <div class="cols-2">
      <section class="surface stack">
        <h2>{{ tr("个人资料", "Profile") }}</h2>
        <form class="form-grid" @submit.prevent="handleUpdateProfile">
          <label>
            <span>{{ tr("用户名", "Username") }}</span>
            <input v-model.trim="profileForm.username" maxlength="32" required />
          </label>
          <label>
            <span>{{ tr("邮箱", "Email") }}</span>
            <input v-model.trim="profileForm.email" type="email" maxlength="128" required />
          </label>
          <button class="btn-solid" type="submit" :disabled="updatingProfile">
            {{ updatingProfile ? tr("保存中...", "Saving...") : tr("保存资料", "Save profile") }}
          </button>
        </form>
      </section>

      <section class="surface stack">
        <h2>{{ tr("修改密码", "Change Password") }}</h2>
        <form class="form-grid" @submit.prevent="handleChangePassword">
          <label>
            <span>{{ tr("当前密码", "Current password") }}</span>
            <input v-model="passwordForm.current_password" type="password" autocomplete="current-password" required />
          </label>
          <label>
            <span>{{ tr("新密码", "New password") }}</span>
            <input
              v-model="passwordForm.new_password"
              type="password"
              autocomplete="new-password"
              minlength="8"
              required
            />
          </label>
          <button class="btn-solid" type="submit" :disabled="changingPassword">
            {{ changingPassword ? tr("提交中...", "Submitting...") : tr("更新密码", "Update password") }}
          </button>
        </form>
      </section>
    </div>

    <section class="surface stack">
      <header class="row-between">
        <h2>{{ tr("登录历史", "Login History") }}</h2>
        <button class="btn-line" type="button" @click="loadHistory" :disabled="loadingHistory">
          {{ loadingHistory ? tr("加载中...", "Loading...") : tr("刷新", "Refresh") }}
        </button>
      </header>
      <p v-if="historyError" class="error">{{ historyError }}</p>

      <div v-if="history.length > 0" class="table-wrap">
        <table>
          <thead>
            <tr>
              <th>{{ tr("时间", "Time") }}</th>
              <th>{{ tr("动作", "Action") }}</th>
              <th>{{ tr("来源信息", "Source") }}</th>
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
      </div>
      <p v-else class="soft">{{ tr("暂无登录历史记录。", "No login history records.") }}</p>
    </section>

    <section class="surface stack">
      <h2>{{ tr("危险操作", "Danger Zone") }}</h2>
      <p class="muted">{{ tr("删除账号后会立即撤销会话，且不可恢复。", "Deleting the account revokes all sessions and cannot be undone.") }}</p>
      <div class="context-menu">
        <button class="btn-danger" type="button" @click="confirmDelete = !confirmDelete">
          {{ confirmDelete ? tr("取消删除", "Cancel") : tr("删除我的账号", "Delete my account") }}
        </button>
        <button v-if="confirmDelete" class="btn-danger" type="button" :disabled="deletingAccount" @click="handleDeleteAccount">
          {{ deletingAccount ? tr("删除中...", "Deleting...") : tr("确认删除", "Confirm delete") }}
        </button>
      </div>
      <p v-if="confirmDelete" class="warn">{{ tr("确认后将立即退出并删除账号数据。", "This action will sign out and delete account data immediately.") }}</p>
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
  updateProfile,
  type LoginHistoryItem
} from "../api/client";
import { useL10n } from "../composables/useL10n";
import { useAuthStore } from "../stores/auth";
import { useUiStore } from "../stores/ui";

const authStore = useAuthStore();
const uiStore = useUiStore();
const { locale, tr } = useL10n();
const router = useRouter();

const pageError = ref("");

const updatingProfile = ref(false);
const changingPassword = ref(false);
const deletingAccount = ref(false);
const loadingHistory = ref(false);
const historyError = ref("");
const confirmDelete = ref(false);

const history = ref<LoginHistoryItem[]>([]);

const profileForm = reactive({
  username: authStore.user?.username ?? "",
  email: authStore.user?.email ?? ""
});

const passwordForm = reactive({
  current_password: "",
  new_password: ""
});

function accessTokenOrThrow() {
  const token = authStore.accessToken;
  if (!token) {
    throw new ApiClientError(tr("未登录或会话已失效", "Not signed in or session expired"), "unauthorized");
  }
  return token;
}

function formatTime(input: string) {
  const localeTag = locale.value === "en" ? "en-US" : "zh-CN";
  return new Date(input).toLocaleString(localeTag);
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
    const token = accessTokenOrThrow();
    const updated = await updateProfile(
      {
        username: profileForm.username,
        email: profileForm.email
      },
      token
    );

    authStore.user = updated;
    uiStore.success(tr("资料已更新", "Profile updated"), tr("用户名和邮箱已保存。", "Username and email saved."), 2200);
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : tr("更新资料失败", "Failed to update profile");
    pageError.value = message;
    uiStore.error(tr("更新资料失败", "Failed to update profile"), message);
    uiStore.alertError(tr("账户模块", "Account module"), message);
  } finally {
    updatingProfile.value = false;
  }
}

async function handleChangePassword() {
  changingPassword.value = true;
  pageError.value = "";

  try {
    const token = accessTokenOrThrow();
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
    uiStore.success(tr("密码已更新", "Password updated"), tr("会话已刷新。", "Session refreshed."), 2600);
    await loadHistory();
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : tr("修改密码失败", "Failed to change password");
    pageError.value = message;
    uiStore.error(tr("修改密码失败", "Failed to change password"), message);
  } finally {
    changingPassword.value = false;
  }
}

async function loadHistory() {
  loadingHistory.value = true;
  historyError.value = "";

  try {
    const token = accessTokenOrThrow();
    history.value = await getLoginHistory(token, { limit: 50 });
  } catch (err) {
    historyError.value = err instanceof ApiClientError ? err.message : tr("加载登录历史失败", "Failed to load login history");
  } finally {
    loadingHistory.value = false;
  }
}

async function handleDeleteAccount() {
  deletingAccount.value = true;
  pageError.value = "";

  try {
    const token = accessTokenOrThrow();
    await deleteAccount(token);
    const username = authStore.user?.username ?? tr("当前账号", "Current account");
    authStore.clearSession();
    confirmDelete.value = false;
    uiStore.warning(tr("账号已删除", "Account deleted"), tr(`${username} 已被删除并退出登录。`, `${username} was deleted and signed out.`), 3400);
    router.replace("/home");
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : tr("删除账号失败", "Failed to delete account");
    pageError.value = message;
    uiStore.error(tr("删除账号失败", "Failed to delete account"), message);
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
