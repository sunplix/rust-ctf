<template>
  <section class="page-block">
    <div class="page-head">
      <h1>比赛列表</h1>
      <button class="ghost" type="button" @click="loadContests" :disabled="loading">
        {{ loading ? "刷新中..." : "刷新" }}
      </button>
    </div>

    <p class="muted">仅展示公开且状态为 scheduled/running/ended 的比赛。</p>

    <p v-if="error" class="error">{{ error }}</p>

    <div v-if="loading && contests.length === 0" class="panel">正在加载比赛...</div>

    <div v-else-if="contests.length === 0" class="panel">暂无可见比赛。</div>

    <div v-else class="contest-grid">
      <article v-for="contest in contests" :key="contest.id" class="panel contest-card">
        <div class="row-between">
          <h2>{{ contest.title }}</h2>
          <span class="badge">{{ contest.status }}</span>
        </div>
        <p class="muted mono">slug: {{ contest.slug }}</p>
        <p class="muted">开始: {{ formatTime(contest.start_at) }}</p>
        <p class="muted">结束: {{ formatTime(contest.end_at) }}</p>

        <RouterLink class="primary-link" :to="`/contests/${contest.id}`">
          进入比赛
        </RouterLink>
      </article>
    </div>
  </section>
</template>

<script setup lang="ts">
import { onMounted, ref } from "vue";

import { ApiClientError, listContests, type ContestListItem } from "../api/client";

const contests = ref<ContestListItem[]>([]);
const loading = ref(false);
const error = ref("");

function formatTime(input: string) {
  return new Date(input).toLocaleString();
}

async function loadContests() {
  loading.value = true;
  error.value = "";

  try {
    contests.value = await listContests();
  } catch (err) {
    error.value = err instanceof ApiClientError ? err.message : "加载比赛失败";
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  loadContests();
});
</script>
