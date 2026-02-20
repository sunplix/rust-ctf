<template>
  <div class="app-shell">
    <div class="ambient-orb a"></div>
    <div class="ambient-orb b"></div>

    <header class="app-topbar">
      <div class="topbar-row">
        <RouterLink class="brand-link" to="/contests">
          <span class="brand-puck">RC</span>
          <span>RUST CTF</span>
        </RouterLink>

        <nav class="top-nav" v-if="authStore.isAuthenticated">
          <RouterLink class="top-nav-link" to="/contests">
            <svg class="line-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M4.5 7.5h15m-15 4.5h15m-15 4.5h9" />
            </svg>
            <span>比赛中心</span>
          </RouterLink>
          <RouterLink class="top-nav-link" to="/teams">
            <svg class="line-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M7.5 11.5a3 3 0 1 0 0-6 3 3 0 0 0 0 6Zm9 2a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5ZM3.5 19c.5-3 2.7-4.5 6-4.5s5.5 1.5 6 4.5m1-2.3c2.1.1 3.5 1 4 2.8" />
            </svg>
            <span>队伍中心</span>
          </RouterLink>
          <RouterLink class="top-nav-link" to="/profile">
            <svg class="line-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8Zm-6.5 8c.6-3.6 3-5.5 6.5-5.5s5.9 1.9 6.5 5.5" />
            </svg>
            <span>账户中心</span>
          </RouterLink>
          <RouterLink v-if="isAdminLike" class="top-nav-link" to="/admin">
            <svg class="line-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M12 3.8 4.5 7.2v4.8c0 4.2 2.8 7.4 7.5 8.8 4.7-1.4 7.5-4.6 7.5-8.8V7.2L12 3.8Z" />
              <path d="M9.5 12h5m-2.5-2.5v5" />
            </svg>
            <span>管理台</span>
          </RouterLink>
        </nav>

        <div class="top-right" v-if="authStore.isAuthenticated">
          <div class="user-pill">
            <span class="user-name">{{ authStore.user?.username }}</span>
            <span class="badge">{{ authStore.user?.role }}</span>
          </div>
          <button class="btn-ghost" type="button" @click="logout">退出</button>
        </div>
        <RouterLink v-else class="btn-ghost-link" to="/login">登录</RouterLink>
      </div>
    </header>

    <main class="page-wrap">
      <section class="page-shell">
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
            ×
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
