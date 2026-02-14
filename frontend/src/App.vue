<template>
  <div class="app-shell">
    <header class="topbar">
      <RouterLink class="brand" to="/contests">Rust-CTF</RouterLink>

      <nav class="nav" v-if="authStore.isAuthenticated">
        <RouterLink to="/contests">比赛</RouterLink>
      </nav>

      <div class="account" v-if="authStore.isAuthenticated">
        <span class="mono">{{ authStore.user?.username }} ({{ authStore.user?.role }})</span>
        <button class="ghost" type="button" @click="logout">退出</button>
      </div>

      <RouterLink v-else class="ghost-link" to="/login">登录</RouterLink>
    </header>

    <main class="content">
      <RouterView />
    </main>
  </div>
</template>

<script setup lang="ts">
import { useRouter } from "vue-router";

import { useAuthStore } from "./stores/auth";

const authStore = useAuthStore();
const router = useRouter();

authStore.hydrateFromStorage();

function logout() {
  authStore.clearSession();
  router.replace("/login");
}
</script>
