<template>
  <div class="app-shell">
    <div class="ambient ambient-a"></div>
    <div class="ambient ambient-b"></div>
    <div class="ambient ambient-c"></div>

    <header class="topbar">
      <div class="topbar-left">
        <RouterLink class="brand" to="/contests">
          <span class="brand-mark">RC</span>
          <span>Rust-CTF</span>
        </RouterLink>

        <nav class="nav" v-if="authStore.isAuthenticated">
          <RouterLink class="nav-link" to="/contests">比赛中心</RouterLink>
          <RouterLink class="nav-link" to="/teams">队伍中心</RouterLink>
          <RouterLink class="nav-link" to="/profile">账户中心</RouterLink>
          <RouterLink v-if="isAdminLike" class="nav-link" to="/admin">管理控制台</RouterLink>
        </nav>
      </div>

      <div class="topbar-right" v-if="authStore.isAuthenticated">
        <div class="account">
          <span class="account-name">{{ authStore.user?.username }}</span>
          <span class="badge">{{ authStore.user?.role }}</span>
        </div>
        <button class="ghost" type="button" @click="logout">退出</button>
      </div>
      <RouterLink v-else class="ghost-link" to="/login">登录</RouterLink>
    </header>

    <main class="content">
      <section class="content-shell">
        <RouterView v-slot="{ Component }">
          <Transition name="view-fade" mode="out-in">
            <component :is="Component" />
          </Transition>
        </RouterView>
      </section>
    </main>

    <TransitionGroup name="toast" tag="section" class="toast-stack">
      <article
        v-for="toast in uiStore.toasts"
        :key="toast.id"
        class="toast-item"
        :class="`toast-${toast.type}`"
      >
        <div class="row-between">
          <strong>{{ toast.title }}</strong>
          <button class="toast-close" type="button" @click="uiStore.removeToast(toast.id)">
            x
          </button>
        </div>
        <p>{{ toast.message }}</p>
      </article>
    </TransitionGroup>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useRouter } from "vue-router";

import { useAuthStore } from "./stores/auth";
import { useUiStore } from "./stores/ui";

const authStore = useAuthStore();
const uiStore = useUiStore();
const router = useRouter();

authStore.hydrateFromStorage();

const isAdminLike = computed(() => {
  const role = authStore.user?.role;
  return role === "admin" || role === "judge";
});

function logout() {
  const username = authStore.user?.username ?? "";
  authStore.clearSession();
  uiStore.info("已退出登录", username ? `${username} 已安全退出。` : "会话已清除。");
  router.replace("/login");
}
</script>
