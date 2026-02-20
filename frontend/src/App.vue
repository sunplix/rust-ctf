<template>
  <div class="app-root">
    <div class="grain-layer" aria-hidden="true"></div>

    <header class="topbar-shell">
      <div class="topbar-line"></div>
      <div class="topbar-content">
        <RouterLink class="brand" :to="brandTo">
          <span class="brand-mark">RC</span>
          <span class="brand-text">
            <strong>{{ appStore.siteSettings.site_name }}</strong>
            <small>{{ appStore.siteSettings.site_subtitle }}</small>
          </span>
        </RouterLink>

        <nav v-if="authStore.isAuthenticated" class="top-nav">
          <RouterLink class="nav-link" to="/contests">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M4 7.5h16M4 12h10M4 16.5h16" />
            </svg>
            {{ tr("比赛", "Contests") }}
          </RouterLink>
          <RouterLink class="nav-link" to="/teams">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M7.5 11a3 3 0 1 0 0-6 3 3 0 0 0 0 6Zm9 2a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5ZM4 19c.6-3 2.9-4.5 6.5-4.5s5.9 1.5 6.5 4.5m.8-2.3c1.8.2 3.1 1.1 3.7 2.8" />
            </svg>
            {{ tr("队伍", "Teams") }}
          </RouterLink>
          <RouterLink class="nav-link" to="/profile">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8Zm-6.6 8c.6-3.5 3.1-5.4 6.6-5.4s6 1.9 6.6 5.4" />
            </svg>
            {{ tr("账户", "Profile") }}
          </RouterLink>
          <RouterLink v-if="isAdminLike" class="nav-link" to="/admin">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M12 3.8 4.8 7v4.8c0 4.2 2.7 7.3 7.2 8.7 4.5-1.4 7.2-4.5 7.2-8.7V7L12 3.8Z" />
              <path d="M9.7 12h4.6m-2.3-2.3v4.6" />
            </svg>
            {{ tr("管理", "Admin") }}
          </RouterLink>
          <RouterLink v-if="isAdminOnly" class="nav-link" to="/admin/site-settings">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <circle cx="12" cy="12" r="3.2" />
              <path d="M4.6 13.3v-2.6l2-.5a5.5 5.5 0 0 1 .7-1.7L6 6.7l1.9-1.9 1.8 1.3a5.5 5.5 0 0 1 1.7-.7l.5-2h2.6l.5 2a5.5 5.5 0 0 1 1.7.7l1.8-1.3L18 6.7l-1.3 1.8a5.5 5.5 0 0 1 .7 1.7l2 .5v2.6l-2 .5a5.5 5.5 0 0 1-.7 1.7l1.3 1.8-1.9 1.9-1.8-1.3a5.5 5.5 0 0 1-1.7.7l-.5 2h-2.6l-.5-2a5.5 5.5 0 0 1-1.7-.7l-1.8 1.3L6 17.3l1.3-1.8a5.5 5.5 0 0 1-.7-1.7l-2-.5Z" />
            </svg>
            {{ tr("站点", "Site") }}
          </RouterLink>
        </nav>

        <div class="top-actions">
          <div class="context-menu top-utility-menu">
            <button class="btn-line btn-compact" type="button" @click="appStore.toggleLocale()">
              {{ localeSwitchLabel }}
            </button>
            <button class="btn-line btn-compact" type="button" @click="appStore.toggleTheme()">
              {{ themeSwitchLabel }}
            </button>
          </div>
          <template v-if="authStore.isAuthenticated">
            <div class="user-chip">
              <span>{{ authStore.user?.username }}</span>
              <small>{{ roleLabel }}</small>
            </div>
            <button class="btn-line" type="button" @click="logout">{{ tr("退出", "Sign Out") }}</button>
          </template>
          <RouterLink v-else class="btn-line" :to="isAuthRoute ? '/home' : '/auth'">
            {{ isAuthRoute ? tr("首页", "Home") : tr("登录", "Sign In") }}
          </RouterLink>
        </div>
      </div>
    </header>

    <section v-if="uiStore.alerts.length" class="alert-stack" aria-live="assertive">
      <article
        v-for="alert in uiStore.alerts"
        :key="alert.id"
        class="alert-card"
        :class="`alert-${alert.type}`"
      >
        <div class="alert-head">
          <strong>{{ alert.title }}</strong>
          <button type="button" class="quiet-close" @click="uiStore.removeAlert(alert.id)">
            <span aria-hidden="true">×</span>
          </button>
        </div>
        <p>{{ alert.message }}</p>
      </article>
    </section>

    <main class="page-shell">
      <RouterView v-slot="{ Component }">
        <Transition name="fade-switch" mode="out-in">
          <component :is="Component" />
        </Transition>
      </RouterView>
    </main>

    <footer class="site-footer">
      <p>{{ appStore.siteSettings.footer_text }}</p>
    </footer>

    <TransitionGroup name="toast-anim" tag="section" class="toast-stack" aria-live="polite">
      <article
        v-for="toast in uiStore.toasts"
        :key="toast.id"
        class="toast-card"
        :class="`toast-${toast.type}`"
      >
        <div class="toast-head">
          <strong>{{ toast.title }}</strong>
          <button type="button" class="quiet-close" @click="uiStore.removeToast(toast.id)">
            <span aria-hidden="true">×</span>
          </button>
        </div>
        <p>{{ toast.message }}</p>
      </article>
    </TransitionGroup>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";

import { useL10n } from "./composables/useL10n";
import { useAppStore } from "./stores/app";
import { useAuthStore } from "./stores/auth";
import { useUiStore } from "./stores/ui";

const appStore = useAppStore();
const authStore = useAuthStore();
const uiStore = useUiStore();
const router = useRouter();
const route = useRoute();
const { tr } = useL10n();

appStore.hydrateFromStorage();
authStore.hydrateFromStorage();

const brandTo = computed(() => {
  return authStore.isAuthenticated ? "/contests" : "/home";
});

const isAuthRoute = computed(() => route.name === "auth");

const isAdminLike = computed(() => {
  const role = authStore.user?.role;
  return role === "admin" || role === "judge";
});

const isAdminOnly = computed(() => authStore.user?.role === "admin");

const roleLabel = computed(() => {
  const role = authStore.user?.role ?? "";
  if (role === "admin") {
    return tr("管理员", "Admin");
  }
  if (role === "judge") {
    return tr("裁判", "Judge");
  }
  if (role === "player") {
    return tr("选手", "Player");
  }
  return role;
});

const localeSwitchLabel = computed(() => {
  return appStore.locale === "zh" ? "EN" : "中文";
});

const themeSwitchLabel = computed(() => {
  return appStore.theme === "light" ? tr("深色", "Dark") : tr("浅色", "Light");
});

function logout() {
  const username = authStore.user?.username ?? "";
  authStore.clearSession();
  uiStore.info(
    tr("会话结束", "Session Ended"),
    username ? tr(`${username} 已退出登录。`, `${username} signed out.`) : tr("会话已清理。", "Session cleared.")
  );
  uiStore.clearAlerts();
  router.replace("/home");
}
</script>
