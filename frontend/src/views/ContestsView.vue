<template>
  <section class="page-block contests-page">
    <div class="page-head">
      <div>
        <h1>比赛中心</h1>
        <p class="muted">按时间聚焦进行中和即将开始的公开比赛。</p>
      </div>
      <button class="ghost" type="button" @click="loadContests" :disabled="loading">
        {{ loading ? "刷新中..." : "刷新" }}
      </button>
    </div>

    <p v-if="error" class="error">{{ error }}</p>

    <div v-if="loading && contests.length === 0" class="panel">正在加载比赛...</div>

    <template v-else>
      <section v-if="focusContest" class="panel focus-panel">
        <div class="focus-media" :class="{ empty: !focusContest.poster_url }">
          <img
            v-if="focusContest.poster_url"
            :src="contestPosterUrl(focusContest)"
            :alt="`${focusContest.title} poster`"
          />
          <div v-else class="focus-fallback">NO POSTER</div>
        </div>
        <div class="focus-content">
          <div class="row-between">
            <h2>{{ focusContest.title }}</h2>
            <span class="badge">{{ focusContest.status }}</span>
          </div>
          <p class="muted mono">{{ formatRange(focusContest.start_at, focusContest.end_at) }}</p>
          <p class="focus-description">{{ focusContest.description || "暂无比赛描述。" }}</p>
          <article
            v-if="focusContest.latest_announcement_title || focusContest.latest_announcement_content"
            class="focus-announcement"
          >
            <strong>{{ focusContest.latest_announcement_title || "最新公告" }}</strong>
            <p>{{ announcementSummary(focusContest.latest_announcement_content) }}</p>
            <span class="muted mono" v-if="focusContest.latest_announcement_published_at">
              {{ formatTime(focusContest.latest_announcement_published_at) }}
            </span>
          </article>
          <RouterLink class="primary-link" :to="`/contests/${focusContest.id}`">进入比赛</RouterLink>
        </div>
      </section>

      <section class="contest-section">
        <div class="row-between section-head">
          <h3>进行中</h3>
          <span class="badge">{{ runningContests.length }}</span>
        </div>
        <div v-if="runningContests.length === 0" class="panel muted">暂无进行中的比赛。</div>
        <div v-else class="contest-grid stagger-list">
          <article v-for="contest in runningContests" :key="contest.id" class="panel timeline-card">
            <div class="timeline-media" :class="{ empty: !contest.poster_url }">
              <img v-if="contest.poster_url" :src="contestPosterUrl(contest)" :alt="`${contest.title} poster`" />
              <div v-else class="timeline-fallback">NO POSTER</div>
            </div>
            <div class="timeline-content">
              <div class="row-between">
                <strong>{{ contest.title }}</strong>
                <span class="badge">running</span>
              </div>
              <p class="muted mono">{{ formatRange(contest.start_at, contest.end_at) }}</p>
              <p class="muted timeline-desc">{{ contest.description || "暂无比赛描述。" }}</p>
              <p class="timeline-announcement" v-if="contest.latest_announcement_title || contest.latest_announcement_content">
                {{ contest.latest_announcement_title || "最新公告" }}：
                {{ announcementSummary(contest.latest_announcement_content) }}
              </p>
              <RouterLink class="primary-link" :to="`/contests/${contest.id}`">进入比赛</RouterLink>
            </div>
          </article>
        </div>
      </section>

      <section class="contest-section">
        <div class="row-between section-head">
          <h3>即将开始</h3>
          <span class="badge">{{ upcomingContests.length }}</span>
        </div>
        <div v-if="upcomingContests.length === 0" class="panel muted">暂无即将开始的比赛。</div>
        <div v-else class="contest-grid stagger-list">
          <article v-for="contest in upcomingContests" :key="contest.id" class="panel timeline-card">
            <div class="timeline-media" :class="{ empty: !contest.poster_url }">
              <img v-if="contest.poster_url" :src="contestPosterUrl(contest)" :alt="`${contest.title} poster`" />
              <div v-else class="timeline-fallback">NO POSTER</div>
            </div>
            <div class="timeline-content">
              <div class="row-between">
                <strong>{{ contest.title }}</strong>
                <span class="badge">scheduled</span>
              </div>
              <p class="muted mono">{{ formatRange(contest.start_at, contest.end_at) }}</p>
              <p class="muted timeline-desc">{{ contest.description || "暂无比赛描述。" }}</p>
              <p class="timeline-announcement" v-if="contest.latest_announcement_title || contest.latest_announcement_content">
                {{ contest.latest_announcement_title || "最新公告" }}：
                {{ announcementSummary(contest.latest_announcement_content) }}
              </p>
              <RouterLink class="ghost-link" :to="`/contests/${contest.id}`">查看详情</RouterLink>
            </div>
          </article>
        </div>
      </section>

      <section class="contest-section">
        <details class="panel ended-section">
          <summary class="row-between">
            <span>已结束比赛</span>
            <span class="badge">{{ endedContests.length }}</span>
          </summary>
          <div v-if="endedContests.length === 0" class="muted">暂无已结束比赛。</div>
          <div v-else class="contest-grid stagger-list ended-grid">
            <article v-for="contest in endedContests" :key="contest.id" class="panel timeline-card ended-card">
              <div class="timeline-content">
                <div class="row-between">
                  <strong>{{ contest.title }}</strong>
                  <span class="badge">ended</span>
                </div>
                <p class="muted mono">{{ formatRange(contest.start_at, contest.end_at) }}</p>
                <p class="muted timeline-desc">{{ contest.description || "暂无比赛描述。" }}</p>
                <RouterLink class="ghost-link" :to="`/contests/${contest.id}`">进入回顾</RouterLink>
              </div>
            </article>
          </div>
        </details>
      </section>

      <div v-if="!hasContests" class="panel">暂无可见比赛。</div>
    </template>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";

import {
  ApiClientError,
  buildApiAssetUrl,
  listContests,
  type ContestListItem
} from "../api/client";
import { useUiStore } from "../stores/ui";

const contests = ref<ContestListItem[]>([]);
const loading = ref(false);
const error = ref("");
const posterCacheBuster = ref(`${Date.now()}`);
const uiStore = useUiStore();

const orderedContests = computed(() => {
  return [...contests.value].sort((a, b) => {
    const statusWeight = (status: string) => {
      if (status === "running") {
        return 0;
      }
      if (status === "scheduled") {
        return 1;
      }
      return 2;
    };

    const weightDelta = statusWeight(a.status) - statusWeight(b.status);
    if (weightDelta !== 0) {
      return weightDelta;
    }

    if (a.status === "running") {
      return new Date(a.end_at).getTime() - new Date(b.end_at).getTime();
    }

    return new Date(a.start_at).getTime() - new Date(b.start_at).getTime();
  });
});

const runningContests = computed(() => orderedContests.value.filter((item) => item.status === "running"));
const upcomingContests = computed(() => orderedContests.value.filter((item) => item.status === "scheduled"));
const endedContests = computed(() => orderedContests.value.filter((item) => item.status === "ended"));

const focusContest = computed(() => {
  if (runningContests.value.length > 0) {
    return runningContests.value[0];
  }
  if (upcomingContests.value.length > 0) {
    return upcomingContests.value[0];
  }
  if (endedContests.value.length > 0) {
    return endedContests.value[0];
  }
  return null;
});

const hasContests = computed(() => contests.value.length > 0);

function formatTime(input: string) {
  return new Date(input).toLocaleString();
}

function formatRange(startAt: string, endAt: string) {
  return `${formatTime(startAt)} ~ ${formatTime(endAt)}`;
}

function contestPosterUrl(contest: ContestListItem) {
  if (!contest.poster_url) {
    return "";
  }

  const url = new URL(buildApiAssetUrl(contest.poster_url));
  url.searchParams.set("v", posterCacheBuster.value);
  return url.toString();
}

function announcementSummary(content: string | null) {
  const text = (content ?? "").trim();
  if (!text) {
    return "暂无公告内容。";
  }

  if (text.length <= 120) {
    return text;
  }

  return `${text.slice(0, 120)}...`;
}

async function loadContests() {
  loading.value = true;
  error.value = "";

  try {
    contests.value = await listContests();
    posterCacheBuster.value = `${Date.now()}`;
  } catch (err) {
    error.value = err instanceof ApiClientError ? err.message : "加载比赛失败";
    uiStore.error("加载比赛失败", error.value);
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  loadContests();
});
</script>

<style scoped>
.contests-page {
  gap: 1rem;
}

.focus-panel {
  display: grid;
  grid-template-columns: minmax(260px, 0.85fr) minmax(0, 1.15fr);
  gap: 0.9rem;
  align-items: stretch;
}

.focus-media {
  border: 1px solid rgba(20, 33, 61, 0.14);
  border-radius: 14px;
  overflow: hidden;
  background: rgba(250, 253, 255, 0.9);
  min-height: 240px;
}

.focus-media img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.focus-media.empty,
.timeline-media.empty {
  display: grid;
  place-items: center;
}

.focus-fallback,
.timeline-fallback {
  font-size: 0.78rem;
  letter-spacing: 0.06em;
  color: rgba(20, 33, 61, 0.52);
}

.focus-content {
  display: grid;
  gap: 0.5rem;
  align-content: start;
}

.focus-content h2 {
  margin: 0;
}

.focus-description {
  margin: 0;
  white-space: pre-wrap;
}

.focus-announcement {
  border: 1px solid rgba(10, 147, 150, 0.22);
  border-radius: 12px;
  background: rgba(236, 254, 255, 0.64);
  padding: 0.62rem;
  display: grid;
  gap: 0.24rem;
}

.focus-announcement p {
  margin: 0;
}

.contest-section {
  display: grid;
  gap: 0.65rem;
}

.section-head h3 {
  margin: 0;
}

.timeline-card {
  display: grid;
  grid-template-columns: 132px minmax(0, 1fr);
  gap: 0.62rem;
  align-items: stretch;
}

.timeline-media {
  border-radius: 10px;
  overflow: hidden;
  border: 1px solid rgba(20, 33, 61, 0.12);
  background: rgba(246, 251, 255, 0.86);
  min-height: 110px;
}

.timeline-media img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.timeline-content {
  display: grid;
  gap: 0.32rem;
  align-content: start;
}

.timeline-content p {
  margin: 0;
}

.timeline-desc {
  line-height: 1.45;
}

.timeline-announcement {
  margin: 0;
  color: rgba(20, 33, 61, 0.9);
}

.ended-section {
  display: grid;
  gap: 0.8rem;
}

.ended-section summary {
  cursor: pointer;
  list-style: none;
}

.ended-section summary::-webkit-details-marker {
  display: none;
}

.ended-grid {
  margin-top: 0.75rem;
}

.ended-card {
  grid-template-columns: minmax(0, 1fr);
}

@media (max-width: 980px) {
  .focus-panel {
    grid-template-columns: 1fr;
  }

  .timeline-card {
    grid-template-columns: 1fr;
  }

  .timeline-media {
    min-height: 150px;
  }
}
</style>
