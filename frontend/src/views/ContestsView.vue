<template>
  <section class="page-layout">
    <article class="surface stack">
      <header class="section-head">
        <div class="section-title">
          <h1>{{ tr("比赛总览", "Contest Overview") }}</h1>
        </div>
        <button class="btn-line" type="button" @click="loadContests" :disabled="loading">
          {{ loading ? tr("刷新中...", "Refreshing...") : tr("刷新", "Refresh") }}
        </button>
      </header>
      <p v-if="error" class="error">{{ error }}</p>
    </article>

    <div class="cols-2 contests-layout">
      <aside class="surface stack">
        <div class="row-between">
          <h2>{{ tr("比赛列表", "Contests") }}</h2>
          <span class="badge">{{ contests.length }}</span>
        </div>

        <div v-if="loading && contests.length === 0" class="muted">
          {{ tr("正在加载比赛数据...", "Loading contests...") }}
        </div>

        <template v-else>
          <section class="stack">
            <div class="row-between">
              <h3>{{ tr("进行中", "Running") }}</h3>
              <span class="soft mono">{{ running.length }}</span>
            </div>
            <div class="list-board">
              <button
                v-for="contest in running"
                :key="contest.id"
                class="select-item"
                :class="{ active: contest.id === selectedContestId }"
                type="button"
                @click="selectedContestId = contest.id"
              >
                <div class="row-between">
                  <strong>{{ contest.title }}</strong>
                  <span class="badge">{{ tr("进行中", "running") }}</span>
                </div>
                <p class="soft mono">{{ formatRange(contest.start_at, contest.end_at) }}</p>
              </button>
              <p v-if="running.length === 0" class="soft">{{ tr("暂无进行中比赛。", "No running contests.") }}</p>
            </div>
          </section>

          <div class="split-line"></div>

          <section class="stack">
            <div class="row-between">
              <h3>{{ tr("即将开始", "Scheduled") }}</h3>
              <span class="soft mono">{{ scheduled.length }}</span>
            </div>
            <div class="list-board">
              <button
                v-for="contest in scheduled"
                :key="contest.id"
                class="select-item"
                :class="{ active: contest.id === selectedContestId }"
                type="button"
                @click="selectedContestId = contest.id"
              >
                <div class="row-between">
                  <strong>{{ contest.title }}</strong>
                  <span class="badge">{{ tr("待开始", "scheduled") }}</span>
                </div>
                <p class="soft mono">{{ formatRange(contest.start_at, contest.end_at) }}</p>
              </button>
              <p v-if="scheduled.length === 0" class="soft">{{ tr("暂无即将开始比赛。", "No scheduled contests.") }}</p>
            </div>
          </section>

          <div class="split-line"></div>

          <section class="stack">
            <div class="row-between">
              <h3>{{ tr("已结束", "Ended") }}</h3>
              <span class="soft mono">{{ ended.length }}</span>
            </div>
            <div class="list-board">
              <button
                v-for="contest in ended"
                :key="contest.id"
                class="select-item"
                :class="{ active: contest.id === selectedContestId }"
                type="button"
                @click="selectedContestId = contest.id"
              >
                <div class="row-between">
                  <strong>{{ contest.title }}</strong>
                  <span class="badge">{{ tr("已结束", "ended") }}</span>
                </div>
                <p class="soft mono">{{ formatRange(contest.start_at, contest.end_at) }}</p>
              </button>
              <p v-if="ended.length === 0" class="soft">{{ tr("暂无已结束比赛。", "No ended contests.") }}</p>
            </div>
          </section>
        </template>
      </aside>

      <main class="surface stack">
        <template v-if="selectedContest">
          <header class="section-head">
            <div class="section-title">
              <h2>{{ selectedContest.title }}</h2>
            </div>
            <span class="badge">{{ selectedContest.status }}</span>
          </header>

          <div v-if="selectedContest.poster_url" class="poster-wrap">
            <img :src="posterUrl(selectedContest)" :alt="`${selectedContest.title} poster`" />
          </div>
          <div v-else class="poster-fallback mono">-</div>

          <p class="muted">{{ selectedContest.description || tr("暂无比赛描述。", "No contest description.") }}</p>
          <p class="soft mono">{{ formatRange(selectedContest.start_at, selectedContest.end_at) }}</p>
          <p
            v-if="selectedContest.latest_announcement_title || selectedContest.latest_announcement_content"
            class="muted"
          >
            <strong>{{ selectedContest.latest_announcement_title || tr("最新公告", "Latest announcement") }}：</strong>
            {{ summarize(selectedContest.latest_announcement_content) }}
          </p>

          <div class="split-line"></div>

          <section class="stack">
            <h3>{{ tr("比赛操作", "Actions") }}</h3>
            <div class="context-menu" v-if="selectedContestId">
              <RouterLink class="btn-solid" :to="`/contests/${selectedContest.id}`">
                {{ tr("进入比赛空间", "Open contest workspace") }}
              </RouterLink>
              <button class="btn-line" type="button" @click="copyContestId(selectedContest.id)">
                {{ tr("复制比赛 ID", "Copy contest ID") }}
              </button>
              <button class="btn-line" type="button" @click="loadContests" :disabled="loading">
                {{ tr("同步最新数据", "Sync latest data") }}
              </button>
            </div>
          </section>
        </template>

        <p v-else class="muted">{{ tr("请选择一个比赛查看详情。", "Select a contest to view details.") }}</p>
      </main>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";

import {
  ApiClientError,
  buildApiAssetUrl,
  listContests,
  type ContestListItem
} from "../api/client";
import { useL10n } from "../composables/useL10n";
import { markdownToPlainText } from "../composables/useMarkdown";
import { useUiStore } from "../stores/ui";

const uiStore = useUiStore();
const { locale, tr } = useL10n();

const contests = ref<ContestListItem[]>([]);
const selectedContestId = ref("");
const loading = ref(false);
const error = ref("");
const cacheVersion = ref(`${Date.now()}`);

const sortedContests = computed(() => {
  return [...contests.value].sort((a, b) => {
    const rank = (status: string) => {
      if (status === "running") {
        return 0;
      }
      if (status === "scheduled") {
        return 1;
      }
      return 2;
    };

    const byStatus = rank(a.status) - rank(b.status);
    if (byStatus !== 0) {
      return byStatus;
    }

    if (a.status === "running") {
      return new Date(a.end_at).getTime() - new Date(b.end_at).getTime();
    }

    return new Date(a.start_at).getTime() - new Date(b.start_at).getTime();
  });
});

const running = computed(() => sortedContests.value.filter((item) => item.status === "running"));
const scheduled = computed(() => sortedContests.value.filter((item) => item.status === "scheduled"));
const ended = computed(() => sortedContests.value.filter((item) => item.status === "ended"));

const selectedContest = computed(() => {
  if (!selectedContestId.value) {
    return null;
  }
  return contests.value.find((contest) => contest.id === selectedContestId.value) ?? null;
});

watch(
  () => contests.value,
  (next) => {
    if (next.length === 0) {
      selectedContestId.value = "";
      return;
    }

    if (!selectedContestId.value || !next.some((contest) => contest.id === selectedContestId.value)) {
      selectedContestId.value = sortedContests.value[0]?.id ?? "";
    }
  },
  { immediate: true }
);

function formatTime(input: string) {
  const localeTag = locale.value === "en" ? "en-US" : "zh-CN";
  return new Date(input).toLocaleString(localeTag);
}

function formatRange(startAt: string, endAt: string) {
  return `${formatTime(startAt)} ~ ${formatTime(endAt)}`;
}

function posterUrl(contest: ContestListItem) {
  if (!contest.poster_url) {
    return "";
  }

  const url = new URL(buildApiAssetUrl(contest.poster_url));
  url.searchParams.set("v", cacheVersion.value);
  return url.toString();
}

function summarize(content: string | null) {
  const text = markdownToPlainText(content ?? "");
  if (!text) {
    return tr("暂无公告内容。", "No announcement content.");
  }
  if (text.length <= 100) {
    return text;
  }
  return `${text.slice(0, 100)}...`;
}

async function copyContestId(contestId: string) {
  try {
    await navigator.clipboard.writeText(contestId);
    uiStore.info(tr("已复制", "Copied"), tr("比赛 ID 已复制到剪贴板。", "Contest ID copied to clipboard."), 1800);
  } catch {
    uiStore.warning(
      tr("复制失败", "Copy failed"),
      tr("浏览器不允许写入剪贴板。", "Clipboard access is blocked by the browser."),
      2200
    );
  }
}

async function loadContests() {
  loading.value = true;
  error.value = "";

  try {
    contests.value = await listContests();
    cacheVersion.value = `${Date.now()}`;
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : tr("加载比赛失败", "Failed to load contests");
    error.value = message;
    uiStore.error(tr("加载失败", "Load failed"), message);
    uiStore.alertError(tr("比赛模块", "Contest module"), message);
  } finally {
    loading.value = false;
  }
}

onMounted(async () => {
  await loadContests();
});
</script>

<style scoped>
.poster-wrap {
  border-radius: var(--radius-md);
  overflow: hidden;
  background: rgba(255, 255, 255, 0.28);
  max-height: 280px;
}

.poster-wrap img {
  display: block;
  width: 100%;
  height: 100%;
  max-height: 280px;
  object-fit: cover;
}

.poster-fallback {
  min-height: 160px;
  border-radius: var(--radius-md);
  display: grid;
  place-items: center;
  background: rgba(255, 255, 255, 0.24);
  color: var(--ink-2);
}

.contests-layout {
  align-items: start;
}

@media (max-width: 1180px) {
  .contests-layout {
    grid-template-columns: 1fr;
  }
}
</style>
