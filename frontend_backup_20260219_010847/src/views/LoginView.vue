<template>
  <section class="auth-stage">
    <aside class="panel auth-aside">
      <p class="soft mono">RUST CTF PLATFORM</p>
      <h1>统一登录入口</h1>
      <p class="muted">
        使用账号登录后可访问比赛、题目环境、队伍协作和实时榜单。
      </p>
      <ul class="auth-bullets">
        <li>单点登录与会话刷新</li>
        <li>选手与管理端统一入口</li>
        <li>操作结果统一 toast 提示</li>
      </ul>
    </aside>

    <article class="panel auth-card">
      <div class="row-between">
        <h2>{{ mode === "login" ? "登录" : "注册" }}</h2>
        <div class="switcher">
          <button type="button" :class="{ active: mode === 'login' }" @click="mode = 'login'">
            登录
          </button>
          <button type="button" :class="{ active: mode === 'register' }" @click="mode = 'register'">
            注册
          </button>
        </div>
      </div>

      <form class="form-grid" @submit.prevent="handleSubmit">
        <label v-if="mode === 'register'">
          <span>用户名</span>
          <input v-model.trim="registerForm.username" required minlength="3" maxlength="32" />
        </label>

        <label>
          <span>{{ mode === "login" ? "用户名或邮箱" : "邮箱" }}</span>
          <input
            v-model.trim="identifierOrEmail"
            :type="mode === 'login' ? 'text' : 'email'"
            required
          />
        </label>

        <label>
          <span>密码</span>
          <input
            v-model="password"
            type="password"
            autocomplete="current-password"
            required
            minlength="8"
          />
        </label>

        <button class="btn" type="submit" :disabled="submitting">
          {{ submitting ? "处理中..." : mode === "login" ? "登录" : "注册并登录" }}
        </button>
      </form>

      <p v-if="message" class="message">{{ message }}</p>
      <p v-if="error" class="error">{{ error }}</p>
    </article>
  </section>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from "vue";
import { useRoute, useRouter } from "vue-router";

import { ApiClientError } from "../api/client";
import { useAuthStore } from "../stores/auth";
import { useUiStore } from "../stores/ui";

const authStore = useAuthStore();
const uiStore = useUiStore();
const router = useRouter();
const route = useRoute();

const mode = ref<"login" | "register">("login");
const identifierOrEmail = ref("");
const password = ref("");
const submitting = ref(false);
const message = ref("");
const error = ref("");

const registerForm = reactive({
  username: ""
});

const redirectPath = computed(() => {
  const raw = route.query.redirect;
  if (typeof raw === "string" && raw.startsWith("/")) {
    return raw;
  }
  return "/contests";
});

function resetFeedback() {
  message.value = "";
  error.value = "";
}

async function handleSubmit() {
  resetFeedback();
  submitting.value = true;

  try {
    if (mode.value === "login") {
      await authStore.loginWithPassword({
        identifier: identifierOrEmail.value,
        password: password.value
      });
      message.value = "登录成功，正在跳转...";
      uiStore.success("登录成功", "欢迎回来，准备开始比赛。", 2200);
    } else {
      await authStore.registerWithPassword({
        username: registerForm.username,
        email: identifierOrEmail.value,
        password: password.value
      });
      message.value = "注册成功，正在跳转...";
      uiStore.success("注册成功", "账号已创建并自动登录。", 2200);
    }

    await router.replace(redirectPath.value);
  } catch (err) {
    error.value = err instanceof ApiClientError ? err.message : "请求失败";
    uiStore.error("认证失败", error.value);
  } finally {
    submitting.value = false;
  }
}
</script>

<style scoped>
.auth-stage {
  min-height: 68vh;
  display: grid;
  grid-template-columns: minmax(0, 0.9fr) minmax(0, 1.1fr);
  gap: 0.92rem;
  align-items: stretch;
}

.auth-aside,
.auth-card {
  display: grid;
  gap: 0.8rem;
  align-content: start;
}

.auth-aside h1 {
  font-size: clamp(1.45rem, 2.2vw, 2rem);
}

.auth-bullets {
  margin: 0;
  padding-left: 1.1rem;
  display: grid;
  gap: 0.3rem;
  color: var(--text-1);
}

.switcher {
  display: inline-flex;
  background: rgba(255, 255, 255, 0.38);
  border-radius: 999px;
  padding: 0.18rem;
  gap: 0.18rem;
}

.switcher button {
  border: 0;
  background: transparent;
  border-radius: 999px;
  padding: 0.36rem 0.72rem;
  color: var(--text-1);
}

.switcher button.active {
  background: rgba(17, 17, 17, 0.88);
  color: rgba(255, 255, 255, 0.95);
}

@media (max-width: 920px) {
  .auth-stage {
    grid-template-columns: 1fr;
  }
}
</style>
