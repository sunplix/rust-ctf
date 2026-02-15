<template>
  <section class="auth-page">
    <div class="panel auth-panel">
      <h1>Rust-CTF 选手登录</h1>
      <p class="muted">
        登录后可访问比赛、题目列表、Flag 提交与实时榜单。
      </p>

      <div class="segmented">
        <button
          type="button"
          :class="{ active: mode === 'login' }"
          @click="mode = 'login'"
        >
          登录
        </button>
        <button
          type="button"
          :class="{ active: mode === 'register' }"
          @click="mode = 'register'"
        >
          注册
        </button>
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

        <button class="primary" type="submit" :disabled="submitting">
          {{ submitting ? "处理中..." : mode === "login" ? "登录" : "注册并登录" }}
        </button>
      </form>

      <p v-if="message" class="message">{{ message }}</p>
      <p v-if="error" class="error">{{ error }}</p>
    </div>
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
      uiStore.success("登录成功", "欢迎回来，准备开始比赛。");
    } else {
      await authStore.registerWithPassword({
        username: registerForm.username,
        email: identifierOrEmail.value,
        password: password.value
      });
      message.value = "注册成功，正在跳转...";
      uiStore.success("注册成功", "账号已创建并自动登录。");
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
