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

    <article class="surface stack contests-shell">
      <header class="section-head">
        <div class="section-title">
          <h2>{{ tr("比赛列表", "Contest Directory") }}</h2>
          <p>{{ tr("筛选比赛并查看详情、公告和入口。", "Filter contests and inspect details, announcements, and entry actions.") }}</p>
        </div>
        <div class="context-menu contest-status-switch">
          <button
            class="btn-line btn-compact"
            :class="{ active: contestStatusFilter === 'all' }"
            type="button"
            @click="contestStatusFilter = 'all'"
          >
            {{ tr("全部", "All") }}
            <span class="status-count">{{ contests.length }}</span>
          </button>
          <button
            class="btn-line btn-compact"
            :class="{ active: contestStatusFilter === 'running' }"
            type="button"
            @click="contestStatusFilter = 'running'"
          >
            {{ tr("进行中", "Running") }}
            <span class="status-count">{{ running.length }}</span>
          </button>
          <button
            class="btn-line btn-compact"
            :class="{ active: contestStatusFilter === 'scheduled' }"
            type="button"
            @click="contestStatusFilter = 'scheduled'"
          >
            {{ tr("即将开始", "Scheduled") }}
            <span class="status-count">{{ scheduled.length }}</span>
          </button>
          <button
            class="btn-line btn-compact"
            :class="{ active: contestStatusFilter === 'ended' }"
            type="button"
            @click="contestStatusFilter = 'ended'"
          >
            {{ tr("已结束", "Ended") }}
            <span class="status-count">{{ ended.length }}</span>
          </button>
        </div>
      </header>

      <label class="search-field contest-search">
        <span>{{ tr("快速筛选", "Quick Filter") }}</span>
        <input
          v-model.trim="contestKeyword"
          :placeholder="tr('按标题、slug、状态检索', 'Filter by title, slug, or status')"
        />
      </label>

      <div v-if="loading && contests.length === 0" class="muted">
        {{ tr("正在加载比赛数据...", "Loading contests...") }}
      </div>

      <div v-else class="contest-workspace">
        <aside class="contest-rail">
          <div class="contest-rail-scroll list-board">
            <button
              v-for="contest in filteredContests"
              :key="contest.id"
              class="select-item contest-row"
              :class="{ active: contest.id === selectedContestId }"
              type="button"
              @click="selectedContestId = contest.id"
            >
              <div class="row-between">
                <strong>{{ contest.title }}</strong>
                <span class="badge">{{ statusLabel(contest.status) }}</span>
              </div>
              <p class="soft mono">{{ formatRange(contest.start_at, contest.end_at) }}</p>
              <p class="soft mono contest-row-meta">{{ contest.slug }} · {{ contest.scoring_mode }}</p>
            </button>
            <p v-if="filteredContests.length === 0" class="soft">
              {{ tr("没有匹配的比赛。", "No contests matched the current filter.") }}
            </p>
          </div>
        </aside>

        <main class="contest-focus stack">
          <template v-if="selectedContest">
            <header class="section-head contest-focus-head">
              <div class="section-title">
                <h2>{{ selectedContest.title }}</h2>
              </div>
              <span class="badge">{{ statusLabel(selectedContest.status) }}</span>
            </header>

            <div class="contest-focus-body">
              <div v-if="selectedContest.poster_url" class="poster-wrap contest-poster">
                <img :src="posterUrl(selectedContest)" :alt="`${selectedContest.title} poster`" />
              </div>
              <div v-else class="poster-fallback contest-poster mono">-</div>

              <div class="stack contest-meta">
                <p class="muted">{{ selectedContest.description || tr("暂无比赛描述。", "No contest description.") }}</p>
                <div class="contest-meta-grid">
                  <p class="soft mono">{{ formatRange(selectedContest.start_at, selectedContest.end_at) }}</p>
                  <p class="soft mono">ID · {{ selectedContest.id }}</p>
                  <p class="soft">{{ tr("计分模式", "Scoring") }} · {{ selectedContest.scoring_mode }}</p>
                  <p class="soft">{{ tr("动态衰减", "Dynamic Decay") }} · {{ selectedContest.dynamic_decay }}</p>
                </div>
                <article
                  v-if="selectedContest.latest_announcement_title || selectedContest.latest_announcement_content"
                  class="contest-announcement"
                >
                  <h3>{{ selectedContest.latest_announcement_title || tr("最新公告", "Latest Announcement") }}</h3>
                  <p class="muted">{{ summarize(selectedContest.latest_announcement_content) }}</p>
                </article>
              </div>
            </div>

            <div class="split-line"></div>

            <section class="stack">
              <h3>{{ tr("比赛操作", "Actions") }}</h3>
              <div class="context-menu contest-action-menu" v-if="selectedContestId">
                <button
                  class="btn-solid"
                  type="button"
                  :disabled="enteringContest"
                  @click="handleEnterContestWorkspace"
                >
                  {{ enteringContest ? tr("处理中...", "Processing...") : tr("进入比赛空间", "Open Contest Workspace") }}
                </button>
                <button class="btn-line" type="button" @click="copyContestId(selectedContest.id)">
                  {{ tr("复制比赛 ID", "Copy Contest ID") }}
                </button>
                <button class="btn-line" type="button" @click="loadContests" :disabled="loading">
                  {{ tr("同步最新数据", "Sync Latest Data") }}
                </button>
              </div>
            </section>
          </template>

          <p v-else class="muted">{{ tr("请选择一个比赛查看详情。", "Select a contest to view details.") }}</p>
        </main>
      </div>
    </article>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";

import {
  ApiClientError,
  applyContestRegistration,
  buildApiAssetUrl,
  getContestRegistrationStatus,
  getMyTeam,
  listContests,
  type ContestListItem
} from "../api/client";
import { useL10n } from "../composables/useL10n";
import { markdownToPlainText } from "../composables/useMarkdown";
import { useTimeFormat } from "../composables/useTimeFormat";
import { useAuthStore } from "../stores/auth";
import { useUiStore } from "../stores/ui";

const router = useRouter();
const authStore = useAuthStore();
const uiStore = useUiStore();
const { tr } = useL10n();
const { formatTime } = useTimeFormat();

const contests = ref<ContestListItem[]>([]);
const selectedContestId = ref("");
const loading = ref(false);
const error = ref("");
const enteringContest = ref(false);
const cacheVersion = ref(`${Date.now()}`);
const contestKeyword = ref("");
const contestStatusFilter = ref<"all" | "running" | "scheduled" | "ended">("all");

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

const filteredContests = computed(() => {
  const byStatus =
    contestStatusFilter.value === "all"
      ? sortedContests.value
      : sortedContests.value.filter((item) => item.status === contestStatusFilter.value);
  const keyword = contestKeyword.value.trim().toLowerCase();
  if (!keyword) {
    return byStatus;
  }
  return byStatus.filter((item) => {
    return (
      item.title.toLowerCase().includes(keyword) ||
      item.slug.toLowerCase().includes(keyword) ||
      item.status.toLowerCase().includes(keyword) ||
      item.scoring_mode.toLowerCase().includes(keyword)
    );
  });
});

const selectedContest = computed(() => {
  if (!selectedContestId.value) {
    return null;
  }
  return contests.value.find((contest) => contest.id === selectedContestId.value) ?? null;
});

watch(
  () => filteredContests.value,
  (next) => {
    if (next.length === 0) {
      selectedContestId.value = "";
      return;
    }

    if (!selectedContestId.value || !next.some((contest) => contest.id === selectedContestId.value)) {
      selectedContestId.value = next[0]?.id ?? "";
    }
  },
  { immediate: true }
);

function formatRange(startAt: string, endAt: string) {
  return `${formatTime(startAt)} ~ ${formatTime(endAt)}`;
}

function statusLabel(status: string) {
  if (status === "running") {
    return tr("进行中", "Running");
  }
  if (status === "scheduled") {
    return tr("即将开始", "Scheduled");
  }
  if (status === "ended") {
    return tr("已结束", "Ended");
  }
  return status;
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

function accessTokenOrThrow() {
  const token = authStore.accessToken;
  if (!token) {
    throw new ApiClientError(tr("未登录或会话已失效", "Not signed in or session expired"), "unauthorized");
  }
  return token;
}

async function handleEnterContestWorkspace() {
  const contest = selectedContest.value;
  if (!contest) {
    return;
  }

  enteringContest.value = true;

  try {
    const token = accessTokenOrThrow();
    const role = authStore.user?.role ?? "";
    const isPrivileged = role === "admin" || role === "judge";

    if (!isPrivileged) {
      const myTeam = await getMyTeam(token);
      if (!myTeam.team) {
        uiStore.warning(
          tr("请先加入队伍", "Join a team first"),
          tr("你还不在队伍中，请先创建或加入队伍后再参赛。", "Create or join a team before entering contest workspace.")
        );
        await router.push({ name: "teams" });
        return;
      }
    }

    let registration = await getContestRegistrationStatus(contest.id, token);
    if (!registration.can_enter_workspace && registration.registration_status === "not_registered") {
      registration = await applyContestRegistration(contest.id, token);
    }

    if (registration.can_enter_workspace) {
      await router.push({ name: "contest-detail", params: { contestId: contest.id } });
      return;
    }

    if (registration.registration_status === "pending") {
      uiStore.info(
        tr("报名已提交", "Registration submitted"),
        tr("当前报名待管理员审核，请稍后再进入比赛空间。", "Registration is pending admin approval.")
      );
      return;
    }

    if (registration.registration_status === "rejected") {
      uiStore.error(
        tr("报名被拒绝", "Registration rejected"),
        registration.review_note || tr("请联系管理员处理报名审核。", "Contact admins for registration review.")
      );
      return;
    }

    if (registration.registration_status === "no_team") {
      uiStore.warning(
        tr("请先加入队伍", "Join a team first"),
        tr("你还不在队伍中，请先创建或加入队伍后再参赛。", "Create or join a team before entering contest workspace.")
      );
      await router.push({ name: "teams" });
      return;
    }

    uiStore.warning(
      tr("暂时无法进入", "Unable to enter"),
      tr("当前不满足参赛条件，请稍后重试。", "Contest entry requirements are not met yet.")
    );
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : tr("进入比赛失败", "Failed to enter contest workspace");
    uiStore.error(tr("进入比赛失败", "Failed to enter contest workspace"), message);
  } finally {
    enteringContest.value = false;
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
.contests-shell {
  gap: 0.8rem;
}

.contest-status-switch {
  row-gap: 0.36rem;
}

.contest-status-switch .btn-line.active {
  background: var(--fg-0);
  color: var(--bg-0);
}

.status-count {
  min-width: 1.5rem;
  padding: 0.08rem 0.32rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--glass-mid) 82%, transparent 18%);
  font-size: 0.7rem;
  line-height: 1.2;
}

.contest-status-switch .btn-line.active .status-count {
  background: rgba(255, 255, 255, 0.18);
  color: rgba(255, 255, 255, 0.94);
}

.contest-search {
  max-width: 560px;
}

.contest-workspace {
  display: grid;
  grid-template-columns: minmax(300px, 36%) minmax(0, 1fr);
  gap: 0.8rem;
  height: clamp(520px, 74vh, 860px);
  max-height: clamp(520px, 74vh, 860px);
}

.contest-rail,
.contest-focus {
  min-height: 0;
  border-radius: var(--radius-md);
  background: var(--glass-mid);
  box-shadow: inset 0 -1px 0 var(--line-soft);
}

.contest-rail {
  padding: 0.48rem;
  overflow: hidden;
}

.contest-rail-scroll {
  height: 100%;
  overflow-y: auto;
  padding-right: 0.18rem;
}

.contest-row {
  gap: 0.28rem;
}

.contest-row-meta {
  font-size: 0.78rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.contest-focus {
  padding: 0.78rem;
  overflow-y: auto;
  align-content: start;
}

.contest-focus-head {
  padding-bottom: 0.68rem;
}

.contest-focus-body {
  display: grid;
  grid-template-columns: minmax(230px, 0.82fr) minmax(0, 1fr);
  gap: 0.78rem;
  align-items: start;
}

.contest-poster {
  min-height: 220px;
  max-height: 340px;
}

.poster-wrap {
  border-radius: var(--radius-md);
  overflow: hidden;
  background: rgba(255, 255, 255, 0.28);
  box-shadow: inset 0 -1px 0 var(--line-soft);
}

.poster-wrap img {
  display: block;
  width: 100%;
  height: 100%;
  max-height: 340px;
  object-fit: cover;
}

.poster-fallback {
  min-height: 220px;
  border-radius: var(--radius-md);
  display: grid;
  place-items: center;
  background: rgba(255, 255, 255, 0.24);
  color: var(--ink-2);
  box-shadow: inset 0 -1px 0 var(--line-soft);
}

.contest-meta {
  align-content: start;
}

.contest-meta-grid {
  display: grid;
  gap: 0.26rem;
}

.contest-meta-grid p {
  margin: 0;
}

.contest-announcement {
  display: grid;
  gap: 0.32rem;
  padding: 0.62rem 0.68rem;
  border-radius: var(--radius-md);
  background: color-mix(in srgb, var(--glass-soft) 86%, transparent 14%);
  box-shadow: inset 0 -1px 0 var(--line-soft);
}

.contest-announcement h3,
.contest-announcement p {
  margin: 0;
}

.contest-announcement h3 {
  font-size: 0.92rem;
}

.contest-action-menu {
  width: fit-content;
}

@media (max-width: 1180px) {
  .contest-workspace {
    grid-template-columns: 1fr;
    height: auto;
    max-height: none;
  }

  .contest-rail {
    max-height: 46vh;
  }

  .contest-focus {
    min-height: 420px;
  }
}

@media (max-width: 760px) {
  .contest-focus-body {
    grid-template-columns: 1fr;
  }

  .contest-poster {
    min-height: 180px;
    max-height: 240px;
  }

  .contest-rail {
    max-height: 52vh;
  }

  .contest-action-menu {
    width: 100%;
  }
}
</style>
