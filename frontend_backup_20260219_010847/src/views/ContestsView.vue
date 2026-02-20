<template>
  <section class="page-block contests-stage">
    <header class="page-head">
      <div>
        <p class="soft mono">CONTEST HUB</p>
        <h1>比赛中心</h1>
        <p class="muted">按时间优先展示进行中与即将开始比赛，并聚合海报与最新公告。</p>
      </div>
      <button class="btn-ghost" type="button" @click="loadContests" :disabled="loading">
        {{ loading ? "刷新中..." : "刷新比赛" }}
      </button>
    </header>

    <p v-if="error" class="error">{{ error }}</p>
    <div v-if="loading && contests.length === 0" class="panel muted">正在加载比赛数据...</div>

    <template v-else>
      <section v-if="focusContest" class="panel focus-board">
        <div class="focus-media" :class="{ empty: !focusContest.poster_url }">
          <img
            v-if="focusContest.poster_url"
            :src="contestPosterUrl(focusContest)"
            :alt="`${focusContest.title} poster`"
          />
          <div v-else class="focus-fallback mono">NO POSTER</div>
        </div>

        <div class="focus-body">
          <div class="row-between">
            <h2>{{ focusContest.title }}</h2>
            <span class="badge">{{ focusContest.status }}</span>
          </div>
          <p class="muted mono">{{ formatRange(focusContest.start_at, focusContest.end_at) }}</p>
          <p class="focus-desc">{{ focusContest.description || "暂无比赛描述。" }}</p>

          <article
            v-if="focusContest.latest_announcement_title || focusContest.latest_announcement_content"
            class="focus-notice"
          >
            <strong>{{ focusContest.latest_announcement_title || "最新公告" }}</strong>
            <p>{{ announcementSummary(focusContest.latest_announcement_content) }}</p>
            <span class="soft mono" v-if="focusContest.latest_announcement_published_at">
              {{ formatTime(focusContest.latest_announcement_published_at) }}
            </span>
          </article>

          <RouterLink class="btn-link" :to="`/contests/${focusContest.id}`">进入比赛</RouterLink>
        </div>
      </section>

      <section class="contest-column">
        <div class="row-between section-head">
          <h3>进行中</h3>
          <span class="badge">{{ runningContests.length }}</span>
        </div>

        <div v-if="runningContests.length === 0" class="panel muted">暂无进行中的比赛。</div>

        <div v-else class="card-grid">
          <article v-for="contest in runningContests" :key="contest.id" class="panel contest-card">
            <div class="contest-card-media" :class="{ empty: !contest.poster_url }">
              <img v-if="contest.poster_url" :src="contestPosterUrl(contest)" :alt="`${contest.title} poster`" />
              <div v-else class="focus-fallback mono">NO POSTER</div>
            </div>
            <div class="contest-card-body">
              <div class="row-between">
                <strong>{{ contest.title }}</strong>
                <span class="badge">running</span>
              </div>
              <p class="muted mono">{{ formatRange(contest.start_at, contest.end_at) }}</p>
              <p class="muted">{{ contest.description || "暂无比赛描述。" }}</p>
              <p class="muted" v-if="contest.latest_announcement_title || contest.latest_announcement_content">
                {{ contest.latest_announcement_title || "最新公告" }}：
                {{ announcementSummary(contest.latest_announcement_content) }}
              </p>
              <RouterLink class="btn-link" :to="`/contests/${contest.id}`">进入比赛</RouterLink>
            </div>
          </article>
        </div>
      </section>

      <section class="contest-column">
        <div class="row-between section-head">
          <h3>即将开始</h3>
          <span class="badge">{{ upcomingContests.length }}</span>
        </div>

        <div v-if="upcomingContests.length === 0" class="panel muted">暂无即将开始的比赛。</div>

        <div v-else class="card-grid">
          <article v-for="contest in upcomingContests" :key="contest.id" class="panel contest-card">
            <div class="contest-card-media" :class="{ empty: !contest.poster_url }">
              <img v-if="contest.poster_url" :src="contestPosterUrl(contest)" :alt="`${contest.title} poster`" />
              <div v-else class="focus-fallback mono">NO POSTER</div>
            </div>
            <div class="contest-card-body">
              <div class="row-between">
                <strong>{{ contest.title }}</strong>
                <span class="badge">scheduled</span>
              </div>
              <p class="muted mono">{{ formatRange(contest.start_at, contest.end_at) }}</p>
              <p class="muted">{{ contest.description || "暂无比赛描述。" }}</p>
              <RouterLink class="btn-ghost-link" :to="`/contests/${contest.id}`">查看详情</RouterLink>
            </div>
          </article>
        </div>
      </section>

      <section class="contest-column">
        <details class="panel ended-panel">
          <summary class="row-between">
            <span>已结束比赛</span>
            <span class="badge">{{ endedContests.length }}</span>
          </summary>
          <div v-if="endedContests.length === 0" class="muted">暂无已结束比赛。</div>
          <div v-else class="card-grid ended-grid">
            <article v-for="contest in endedContests" :key="contest.id" class="panel ended-card">
              <div class="row-between">
                <strong>{{ contest.title }}</strong>
                <span class="badge">ended</span>
              </div>
              <p class="muted mono">{{ formatRange(contest.start_at, contest.end_at) }}</p>
              <p class="muted">{{ contest.description || "暂无比赛描述。" }}</p>
              <RouterLink class="btn-ghost-link" :to="`/contests/${contest.id}`">进入回顾</RouterLink>
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
.contests-stage {
  gap: 0.95rem;
}

.focus-board {
  display: grid;
  grid-template-columns: minmax(280px, 0.86fr) minmax(0, 1.14fr);
  gap: 0.86rem;
  align-items: stretch;
}

.focus-media {
  border-radius: 16px;
  background: rgba(255, 255, 255, 0.42);
  min-height: 240px;
  overflow: hidden;
  position: relative;
}

.focus-media::before {
  content: "";
  position: absolute;
  inset: 0;
  pointer-events: none;
  border-radius: inherit;
  background:
    linear-gradient(rgba(17, 17, 17, 0.16), rgba(17, 17, 17, 0.16)) top / 100% 1px no-repeat,
    linear-gradient(rgba(17, 17, 17, 0.16), rgba(17, 17, 17, 0.16)) left / 1px 100% no-repeat;
}

.focus-media img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.focus-media.empty,
.contest-card-media.empty {
  display: grid;
  place-items: center;
}

.focus-fallback {
  font-size: 0.78rem;
  color: var(--text-2);
  letter-spacing: 0.06em;
}

.focus-body {
  display: grid;
  gap: 0.5rem;
  align-content: start;
}

.focus-desc {
  white-space: pre-wrap;
}

.focus-notice {
  display: grid;
  gap: 0.24rem;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.42);
  padding: 0.66rem;
  position: relative;
}

.focus-notice::before {
  content: "";
  position: absolute;
  inset: 0;
  pointer-events: none;
  border-radius: inherit;
  background:
    linear-gradient(rgba(17, 17, 17, 0.16), rgba(17, 17, 17, 0.16)) left / 1px 100% no-repeat,
    repeating-linear-gradient(90deg, transparent 0 8px, rgba(17, 17, 17, 0.24) 8px 13px) bottom / 100% 1px no-repeat;
}

.focus-notice p {
  margin: 0;
}

.contest-column {
  display: grid;
  gap: 0.62rem;
}

.section-head h3 {
  font-size: 1rem;
}

.card-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 0.7rem;
}

.contest-card {
  display: grid;
  grid-template-columns: 140px minmax(0, 1fr);
  gap: 0.62rem;
}

.contest-card-media {
  border-radius: 12px;
  overflow: hidden;
  min-height: 110px;
  background: rgba(255, 255, 255, 0.48);
}

.contest-card-media img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.contest-card-body {
  display: grid;
  gap: 0.32rem;
  align-content: start;
}

.ended-panel {
  display: grid;
  gap: 0.8rem;
}

.ended-panel summary {
  list-style: none;
  cursor: pointer;
}

.ended-panel summary::-webkit-details-marker {
  display: none;
}

.ended-grid {
  margin-top: 0.7rem;
}

.ended-card {
  display: grid;
  gap: 0.32rem;
}

@media (max-width: 980px) {
  .focus-board {
    grid-template-columns: 1fr;
  }

  .contest-card {
    grid-template-columns: 1fr;
  }

  .contest-card-media {
    min-height: 150px;
  }
}
</style>
