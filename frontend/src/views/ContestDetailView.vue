<template>
  <section class="page-layout">
    <article class="surface stack">
      <header class="section-head">
        <div class="section-title">
          <h1>{{ tr("比赛空间", "Contest Workspace") }}</h1>
        </div>
        <div class="row">
          <RouterLink class="btn-line" to="/contests">{{ tr("返回比赛列表", "Back to contests") }}</RouterLink>
        </div>
      </header>
      <p v-if="pageError" class="error">{{ pageError }}</p>
    </article>

    <div class="cols-3 workspace-grid">
      <aside class="surface stack">
        <div class="row-between">
          <h2>{{ tr("题目列表", "Challenges") }}</h2>
          <button class="btn-line" type="button" @click="loadChallenges" :disabled="loadingChallenges">
            {{ loadingChallenges ? tr("同步中...", "Syncing...") : tr("刷新", "Refresh") }}
          </button>
        </div>

        <p v-if="loadingChallenges && challenges.length === 0" class="soft">{{ tr("正在加载题目...", "Loading challenges...") }}</p>
        <p v-if="!loadingChallenges && challenges.length === 0" class="soft">{{ tr("暂无可见题目。", "No visible challenges.") }}</p>

        <div class="list-board">
          <section
            v-for="group in challengeGroups"
            :key="group.category"
            class="challenge-category-group stack"
          >
            <header class="row-between challenge-category-head">
              <h3>{{ group.category }}</h3>
              <span class="badge">{{ group.items.length }}</span>
            </header>
            <button
              v-for="challenge in group.items"
              :key="challenge.id"
              class="select-item"
              :class="{ active: challenge.id === selectedChallengeId }"
              type="button"
              @click="selectedChallengeId = challenge.id"
            >
              <div class="row-between">
                <strong>{{ challenge.title }}</strong>
                <span class="badge">{{ challenge.challenge_type }}</span>
              </div>
              <p class="soft">{{ challenge.category }} · {{ challenge.difficulty }}</p>
              <p class="soft mono">{{ tr("分值", "Score") }} {{ challenge.static_score }}</p>
            </button>
          </section>
        </div>
      </aside>

      <main class="surface stack">
        <template v-if="selectedChallenge">
          <header class="section-head">
            <div class="section-title">
              <h2>{{ selectedChallenge.title }}</h2>
            </div>
            <span class="badge">{{ selectedChallenge.challenge_type }}</span>
          </header>

          <p class="muted">
            {{ tr("分类", "Category") }} {{ selectedChallenge.category }} ·
            {{ tr("难度", "Difficulty") }} {{ selectedChallenge.difficulty }} ·
            {{ tr("分值", "Score") }} {{ selectedChallenge.static_score }}
          </p>
          <div
            class="muted markdown-body challenge-description"
            v-html="renderChallengeDescription(selectedChallenge.description || tr('暂无题目描述。', 'No challenge description.'))"
          ></div>

          <section class="stack">
            <header class="row-between challenge-attachments-head">
              <h3>{{ tr("题目附件", "Challenge files") }}</h3>
              <button class="btn-line" type="button" @click="loadChallengeAttachments" :disabled="loadingChallengeAttachments">
                {{ loadingChallengeAttachments ? tr("刷新中...", "Refreshing...") : tr("刷新附件", "Refresh files") }}
              </button>
            </header>
            <p v-if="loadingChallengeAttachments && challengeAttachments.length === 0" class="soft">
              {{ tr("正在加载附件...", "Loading files...") }}
            </p>
            <div v-else-if="challengeAttachments.length > 0" class="challenge-attachment-list">
              <article
                v-for="attachment in challengeAttachments"
                :key="attachment.id"
                class="surface stack challenge-attachment-item"
              >
                <div class="row-between">
                  <strong>{{ attachment.filename }}</strong>
                  <button
                    class="btn-line"
                    type="button"
                    :disabled="downloadingChallengeAttachmentId === attachment.id"
                    @click="downloadChallengeAttachment(attachment)"
                  >
                    {{
                      downloadingChallengeAttachmentId === attachment.id
                        ? tr("下载中...", "Downloading...")
                        : tr("下载附件", "Download")
                    }}
                  </button>
                </div>
                <p class="soft mono">
                  {{ attachment.content_type }} · {{ formatSize(attachment.size_bytes) }} · {{ formatTime(attachment.created_at) }}
                </p>
              </article>
            </div>
            <p v-else class="soft">{{ tr("当前题目暂无附件。", "No files attached to this challenge.") }}</p>
            <p v-if="challengeAttachmentError" class="error">{{ challengeAttachmentError }}</p>
          </section>

          <section class="stack">
            <h3>{{ tr("题目操作", "Actions") }}</h3>
            <div class="context-menu" v-if="selectedChallengeId">
              <button
                class="btn-line"
                type="button"
                @click="copyToClipboard(selectedChallenge.id, tr('题目 ID 已复制', 'Challenge ID copied'))"
              >
                {{ tr("复制题目 ID", "Copy challenge ID") }}
              </button>
              <button class="btn-line" type="button" @click="loadInstance" :disabled="loadingInstance || !canManageInstance">
                {{ tr("同步实例状态", "Sync instance status") }}
              </button>
              <button class="btn-line" type="button" @click="loadScoreboard" :disabled="loadingScoreboard">
                {{ tr("刷新榜单", "Refresh scoreboard") }}
              </button>
              <button
                v-if="selectedChallenge.hints.length > 0"
                class="btn-line"
                type="button"
                @click="openHintModal"
              >
                {{ tr("查看提示", "Show hints") }} ({{ selectedChallenge.hints.length }})
              </button>
            </div>
          </section>

          <div class="split-line"></div>

          <section class="stack">
            <h3>{{ tr("提交 Flag", "Submit Flag") }}</h3>
            <form class="form-grid" @submit.prevent="handleSubmitFlag">
              <label>
                <span>Flag</span>
                <input v-model.trim="flagInput" required :placeholder="tr('输入 Flag', 'Enter flag')" />
              </label>
              <button class="btn-solid" type="submit" :disabled="submittingFlag">
                {{ submittingFlag ? tr("提交中...", "Submitting...") : tr("提交", "Submit") }}
              </button>
            </form>
            <p v-if="submitResult" class="message mono">
              {{ tr("结果", "Result") }}: {{ submitResult.message || submitResult.verdict }} ·
              {{ tr("得分", "Score") }} {{ submitResult.score_awarded }} ·
              {{ tr("总分", "Total") }} {{ submitResult.total_score }}
            </p>
            <p v-if="submitError" class="error">{{ submitError }}</p>
          </section>

          <section v-if="canManageInstance" class="stack">
            <div class="split-line"></div>
            <header class="row-between">
              <h3>{{ tr("动态环境", "Runtime Instance") }}</h3>
              <div class="context-menu" v-if="selectedChallengeId">
                <button class="btn-solid" type="button" @click="handleInstanceAction('start')" :disabled="instanceBusy">
                  {{ tr("启动", "Start") }}
                </button>
                <button class="btn-line" type="button" @click="handleInstanceAction('stop')" :disabled="instanceBusy">
                  {{ tr("停止", "Stop") }}
                </button>
                <button class="btn-line" type="button" @click="handleInstanceAction('reset')" :disabled="instanceBusy">
                  {{ tr("重置", "Reset") }}
                </button>
                <button class="btn-danger" type="button" @click="handleInstanceAction('destroy')" :disabled="instanceBusy">
                  {{ tr("销毁", "Destroy") }}
                </button>
              </div>
            </header>

            <div v-if="instance" class="surface stack instance-panel">
              <p class="mono">{{ tr("状态", "Status") }}: {{ instance.status }}</p>
              <p class="soft mono">{{ tr("子网", "Subnet") }}: {{ instance.subnet }}</p>
              <p class="soft mono">{{ tr("入口", "Entrypoint") }}: {{ instance.entrypoint_url || "-" }}</p>
              <p class="soft mono">
                {{ tr("到期时间", "Expires at") }}: {{ instance.expires_at ? formatTime(instance.expires_at) : "-" }}
              </p>
              <p class="soft mono">{{ tr("消息", "Message") }}: {{ instance.message }}</p>

              <template v-if="instance.network_access?.mode === 'ssh_bastion'">
                <div class="split-line"></div>
                <p class="soft mono">ssh: {{ instanceSshCommand || "-" }}</p>
                <p class="soft mono">{{ tr("用户名", "Username") }}: {{ instance.network_access.username || "-" }}</p>
                <p class="soft mono">{{ tr("密码", "Password") }}: {{ instance.network_access.password || "-" }}</p>
                <div class="row">
                  <button class="btn-line" type="button" @click="copyNetworkAccessCommand" :disabled="!instanceSshCommand">
                    {{ tr("复制 SSH 命令", "Copy SSH command") }}
                  </button>
                  <button
                    class="btn-line"
                    type="button"
                    @click="copyNetworkAccessPassword"
                    :disabled="!instance.network_access.password"
                  >
                    {{ tr("复制 SSH 密码", "Copy SSH password") }}
                  </button>
                </div>
              </template>

              <template v-if="instance.network_access?.mode === 'wireguard'">
                <div class="split-line"></div>
                <p class="soft mono">{{ tr("端点", "Endpoint") }}: {{ instanceWireguardEndpoint || "-" }}</p>
                <p class="soft mono">{{ tr("配置路径", "Config path") }}: {{ instance.network_access.download_url || "-" }}</p>
                <div class="row">
                  <button
                    class="btn-line"
                    type="button"
                    @click="downloadWireguardConfig"
                    :disabled="downloadingWireguardConfig"
                  >
                    {{ downloadingWireguardConfig ? tr("下载中...", "Downloading...") : tr("下载配置", "Download config") }}
                  </button>
                  <button
                    class="btn-line"
                    type="button"
                    @click="copyToClipboard(instanceWireguardEndpoint, tr('WireGuard 端点已复制', 'WireGuard endpoint copied'))"
                    :disabled="!instanceWireguardEndpoint"
                  >
                    {{ tr("复制端点", "Copy endpoint") }}
                  </button>
                </div>
              </template>
            </div>
            <p v-else class="soft">{{ tr("尚未创建动态环境实例。", "No runtime instance yet.") }}</p>
            <p v-if="instanceError" class="error">{{ instanceError }}</p>
          </section>
        </template>

        <p v-else class="soft">{{ tr("请选择一个题目进入解题流程。", "Select a challenge to continue.") }}</p>
      </main>

      <aside class="surface stack">
        <section class="stack">
          <header class="row-between scoreboard-head">
            <h2>{{ tr("实时榜单", "Live Scoreboard") }}</h2>
            <div class="context-menu scoreboard-actions">
              <button class="btn-line" type="button" @click="loadScoreboard" :disabled="loadingScoreboard">
                {{ loadingScoreboard ? tr("刷新中...", "Refreshing...") : tr("刷新", "Refresh") }}
              </button>
              <button
                class="btn-line"
                type="button"
                @click="exportScoreboardTrendImage"
                :disabled="exportingTrendImage || scoreboardTimeline.length === 0"
              >
                {{ exportingTrendImage ? tr("导出中...", "Exporting...") : tr("导出折线图", "Export chart") }}
              </button>
              <button
                class="btn-line"
                type="button"
                @click="exportScoreboardTrendAnimation"
                :disabled="exportingTrendAnimation || scoreboardTimeline.length < 2"
              >
                {{
                  exportingTrendAnimation
                    ? tr("导出中...", "Exporting...")
                    : tr("导出实时动图(WebM)", "Export realtime animation (WebM)")
                }}
              </button>
            </div>
          </header>
          <div class="row-between trend-toolbar">
            <span class="soft">{{ tr("趋势维度", "Trend metric") }}</span>
            <div class="context-menu">
              <button
                class="btn-line"
                :class="{ active: scoreboardTrendMode === 'score' }"
                type="button"
                @click="openScoreboardWall('score')"
              >
                {{ tr("积分曲线", "Score curve") }}
              </button>
              <button
                class="btn-line"
                :class="{ active: scoreboardTrendMode === 'rank' }"
                type="button"
                @click="openScoreboardWall('rank')"
              >
                {{ tr("排名曲线", "Rank curve") }}
              </button>
            </div>
          </div>
          <div class="trend-canvas-wrap">
            <canvas ref="scoreboardTrendCanvasRef" width="920" height="380"></canvas>
          </div>
          <p class="soft mono">ws: {{ wsState }}</p>
          <p v-if="scoreboardError" class="error">{{ scoreboardError }}</p>

          <div v-if="scoreboard.length > 0" class="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>#</th>
                  <th>{{ tr("队伍", "Team") }}</th>
                  <th>{{ tr("分数", "Score") }}</th>
                  <th>{{ tr("解题", "Solved") }}</th>
                  <th>{{ tr("最后提交", "Last submission") }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="entry in scoreboard" :key="entry.team_id">
                  <td>{{ entry.rank }}</td>
                  <td>{{ entry.team_name }}</td>
                  <td>{{ entry.score }}</td>
                  <td>{{ entry.solved_count }}</td>
                  <td>{{ entry.last_submit_at ? formatTime(entry.last_submit_at) : '-' }}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <p v-else class="soft">{{ tr("暂无榜单数据。", "No scoreboard data.") }}</p>
        </section>

        <div class="split-line"></div>

        <section class="stack">
          <header class="row-between">
            <h3>{{ tr("比赛公告", "Announcements") }}</h3>
            <button class="btn-line" type="button" @click="loadAnnouncements" :disabled="loadingAnnouncements">
              {{ loadingAnnouncements ? tr("刷新中...", "Refreshing...") : tr("刷新", "Refresh") }}
            </button>
          </header>
          <p v-if="announcementError" class="error">{{ announcementError }}</p>

          <div class="list-board">
            <article
              v-for="announcement in contestAnnouncements"
              :key="announcement.id"
              class="surface stack notice-card notice-card-clickable"
              role="button"
              tabindex="0"
              @click="openAnnouncementModal(announcement)"
              @keydown.enter.prevent="openAnnouncementModal(announcement)"
              @keydown.space.prevent="openAnnouncementModal(announcement)"
            >
              <div class="row-between">
                <strong>{{ announcement.title }}</strong>
                <span v-if="announcement.is_pinned" class="badge">{{ tr("置顶", "Pinned") }}</span>
              </div>
              <p class="soft mono">
                {{ tr("发布时间", "Published at") }}:
                {{ announcement.published_at ? formatTime(announcement.published_at) : formatTime(announcement.created_at) }}
              </p>
              <div
                class="muted markdown-body announcement-content announcement-preview"
                v-html="renderAnnouncementContent(announcement.content)"
              ></div>
              <p class="soft announcement-preview-hint">
                {{ tr("点击查看完整公告", "Click to view full announcement") }}
              </p>
            </article>
          </div>
          <p v-if="!loadingAnnouncements && contestAnnouncements.length === 0" class="soft">
            {{ tr("暂无公告。", "No announcements.") }}
          </p>
        </section>
      </aside>
    </div>

    <div
      v-if="activeAnnouncement"
      class="announcement-modal"
      role="dialog"
      aria-modal="true"
      :aria-label="tr('公告详情', 'Announcement details')"
      @click.self="closeAnnouncementModal"
    >
      <article class="announcement-modal-card stack">
        <header class="row-between announcement-modal-head">
          <div class="stack gap-xs">
            <strong>{{ activeAnnouncement.title }}</strong>
            <p class="soft mono announcement-modal-meta">
              {{ tr("发布时间", "Published at") }}:
              {{
                activeAnnouncement.published_at
                  ? formatTime(activeAnnouncement.published_at)
                  : formatTime(activeAnnouncement.created_at)
              }}
            </p>
          </div>
          <div class="announcement-modal-actions">
            <span v-if="activeAnnouncement.is_pinned" class="badge">{{ tr("置顶", "Pinned") }}</span>
            <button class="btn-line" type="button" @click="closeAnnouncementModal">
              {{ tr("关闭", "Close") }}
            </button>
          </div>
        </header>
        <div
          class="markdown-body announcement-content announcement-modal-content"
          v-html="renderAnnouncementContent(activeAnnouncement.content)"
        ></div>
      </article>
    </div>

    <div
      v-if="activeHintChallenge"
      class="announcement-modal"
      role="dialog"
      aria-modal="true"
      :aria-label="tr('题目提示', 'Challenge hints')"
      @click.self="closeHintModal"
    >
      <article class="announcement-modal-card stack">
        <header class="row-between announcement-modal-head">
          <div class="stack gap-xs">
            <strong>{{ tr("题目提示", "Challenge hints") }}</strong>
            <p class="soft mono announcement-modal-meta">{{ activeHintChallenge.title }}</p>
          </div>
          <button class="btn-line" type="button" @click="closeHintModal">
            {{ tr("关闭", "Close") }}
          </button>
        </header>
        <ol class="hint-list">
          <li v-for="(hint, index) in activeHintChallenge.hints" :key="`${activeHintChallenge.id}-${index}`">
            {{ hint }}
          </li>
        </ol>
      </article>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useRouter } from "vue-router";

import {
  ApiClientError,
  buildScoreboardWsUrl,
  downloadContestChallengeAttachment,
  destroyInstance,
  getInstance,
  getInstanceWireguardConfig,
  getScoreboard,
  getScoreboardTimeline,
  listContestChallengeAttachments,
  listContestAnnouncements,
  listContestChallenges,
  resetInstance,
  startInstance,
  stopInstance,
  submitFlag,
  type ContestChallengeAttachmentItem,
  type ContestAnnouncementItem,
  type ContestChallengeItem,
  type InstanceResponse,
  type ScoreboardEntry,
  type ScoreboardTimelineSnapshot,
  type ScoreboardPushPayload,
  type SubmitFlagResponse
} from "../api/client";
import { useL10n } from "../composables/useL10n";
import { renderMarkdownToHtml } from "../composables/useMarkdown";
import { useTimeFormat } from "../composables/useTimeFormat";
import { useAuthStore } from "../stores/auth";
import { useUiStore } from "../stores/ui";

const props = defineProps<{
  contestId: string;
}>();

const authStore = useAuthStore();
const uiStore = useUiStore();
const { tr } = useL10n();
const { formatTime, formatTimeOnly } = useTimeFormat();
const router = useRouter();

const challenges = ref<ContestChallengeItem[]>([]);
const selectedChallengeId = ref("");
const loadingChallenges = ref(false);
const pageError = ref("");

const flagInput = ref("");
const submittingFlag = ref(false);
const submitResult = ref<SubmitFlagResponse | null>(null);
const submitError = ref("");

const instance = ref<InstanceResponse | null>(null);
const loadingInstance = ref(false);
const mutatingInstance = ref(false);
const downloadingWireguardConfig = ref(false);
const instanceError = ref("");

const scoreboard = ref<ScoreboardEntry[]>([]);
const scoreboardTimeline = ref<ScoreboardTimelineSnapshot[]>([]);
const scoreboardTrendMode = ref<"score" | "rank">("score");
const loadingScoreboard = ref(false);
const scoreboardError = ref("");
const exportingTrendImage = ref(false);
const exportingTrendAnimation = ref(false);
const scoreboardTrendCanvasRef = ref<HTMLCanvasElement | null>(null);

const contestAnnouncements = ref<ContestAnnouncementItem[]>([]);
const loadingAnnouncements = ref(false);
const announcementError = ref("");
const activeAnnouncement = ref<ContestAnnouncementItem | null>(null);
const challengeAttachments = ref<ContestChallengeAttachmentItem[]>([]);
const loadingChallengeAttachments = ref(false);
const challengeAttachmentError = ref("");
const downloadingChallengeAttachmentId = ref("");
const activeHintChallengeId = ref("");

const wsState = ref("closed");
let scoreboardSocket: WebSocket | null = null;
let reconnectTimer: number | null = null;
let shouldReconnectScoreboard = true;
let trendRenderFrame: number | null = null;
let themeObserver: MutationObserver | null = null;
const HINT_DISMISSED_STORAGE_KEY = "rust-ctf.dismissed-hints";

const SCOREBOARD_TREND_MAX_SNAPSHOTS = 1200;
const SCOREBOARD_TREND_TOP_N = 12;

const selectedChallenge = computed(() => {
  if (!selectedChallengeId.value) {
    return null;
  }
  return challenges.value.find((item) => item.id === selectedChallengeId.value) ?? null;
});

const activeHintChallenge = computed(() => {
  if (!activeHintChallengeId.value) {
    return null;
  }
  return challenges.value.find((item) => item.id === activeHintChallengeId.value) ?? null;
});

const challengeGroups = computed(() => {
  const groups = new Map<string, ContestChallengeItem[]>();
  for (const item of challenges.value) {
    const category = item.category?.trim() || tr("未分类", "Uncategorized");
    const bucket = groups.get(category);
    if (bucket) {
      bucket.push(item);
    } else {
      groups.set(category, [item]);
    }
  }

  return Array.from(groups.entries()).map(([category, items]) => ({
    category,
    items
  }));
});

const canManageInstance = computed(() => {
  const type = selectedChallenge.value?.challenge_type ?? "";
  return type === "dynamic" || type === "internal";
});

const instanceBusy = computed(() => loadingInstance.value || mutatingInstance.value);

const instanceSshCommand = computed(() => {
  const network = instance.value?.network_access;
  if (!network || network.mode !== "ssh_bastion") {
    return "";
  }
  if (!network.host || !network.port || !network.username) {
    return "";
  }
  return `ssh ${network.username}@${network.host} -p ${network.port}`;
});

const instanceWireguardEndpoint = computed(() => {
  const network = instance.value?.network_access;
  if (!network || network.mode !== "wireguard") {
    return "";
  }
  return `${network.host}:${network.port}`;
});

watch(
  () => challenges.value,
  (rows) => {
    if (rows.length === 0) {
      selectedChallengeId.value = "";
      return;
    }

    if (!selectedChallengeId.value || !rows.some((item) => item.id === selectedChallengeId.value)) {
      selectedChallengeId.value = rows[0].id;
    }
  },
  { immediate: true }
);

watch(
  () => selectedChallengeId.value,
  async () => {
    flagInput.value = "";
    submitResult.value = null;
    submitError.value = "";
    instance.value = null;
    instanceError.value = "";
    challengeAttachments.value = [];
    challengeAttachmentError.value = "";

    if (!selectedChallenge.value) {
      activeHintChallengeId.value = "";
      return;
    }

    await loadChallengeAttachments();
    maybeShowHintModal(selectedChallenge.value);
    if (canManageInstance.value) {
      await loadInstance();
    }
  }
);

watch(
  () => scoreboardTrendMode.value,
  () => {
    scheduleTrendRender();
  }
);

watch(
  () => scoreboardTimeline.value,
  () => {
    scheduleTrendRender();
  },
  { deep: true }
);

function formatSize(bytes: number) {
  const size = Number(bytes);
  if (!Number.isFinite(size) || size <= 0) {
    return "0 B";
  }
  const units = ["B", "KB", "MB", "GB"];
  let value = size;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  return `${value.toFixed(value >= 10 || unitIndex === 0 ? 0 : 1)} ${units[unitIndex]}`;
}

function renderAnnouncementContent(markdown: string) {
  return renderMarkdownToHtml(markdown);
}

function renderChallengeDescription(markdown: string) {
  return renderMarkdownToHtml(markdown || "");
}

function hintDismissKey(challengeId: string) {
  return `${props.contestId}:${challengeId}`;
}

function loadDismissedHints(): Record<string, boolean> {
  try {
    const raw = localStorage.getItem(HINT_DISMISSED_STORAGE_KEY);
    if (!raw) {
      return {};
    }
    const parsed = JSON.parse(raw) as Record<string, boolean>;
    if (parsed && typeof parsed === "object") {
      return parsed;
    }
  } catch {
    // Ignore storage parse failures.
  }
  return {};
}

function saveDismissedHints(next: Record<string, boolean>) {
  localStorage.setItem(HINT_DISMISSED_STORAGE_KEY, JSON.stringify(next));
}

function maybeShowHintModal(challenge: ContestChallengeItem) {
  if (!Array.isArray(challenge.hints) || challenge.hints.length === 0) {
    activeHintChallengeId.value = "";
    return;
  }

  const dismissed = loadDismissedHints();
  if (dismissed[hintDismissKey(challenge.id)]) {
    return;
  }
  activeHintChallengeId.value = challenge.id;
}

function openHintModal() {
  if (!selectedChallenge.value || selectedChallenge.value.hints.length === 0) {
    return;
  }
  activeHintChallengeId.value = selectedChallenge.value.id;
}

function closeHintModal() {
  const challenge = activeHintChallenge.value;
  if (challenge) {
    const dismissed = loadDismissedHints();
    dismissed[hintDismissKey(challenge.id)] = true;
    saveDismissedHints(dismissed);
  }
  activeHintChallengeId.value = "";
}

function openAnnouncementModal(announcement: ContestAnnouncementItem) {
  activeAnnouncement.value = announcement;
}

function closeAnnouncementModal() {
  activeAnnouncement.value = null;
}

function handleWindowKeydown(event: KeyboardEvent) {
  if (event.key !== "Escape") {
    return;
  }

  if (activeHintChallenge.value) {
    closeHintModal();
    return;
  }

  if (activeAnnouncement.value) {
    closeAnnouncementModal();
  }
}

function openScoreboardWall(mode: "score" | "rank") {
  scoreboardTrendMode.value = mode;

  const target = router.resolve({
    name: "scoreboard-wall",
    params: { contestId: props.contestId },
    query: { mode }
  });

  const opened = window.open(target.href, "_blank", "noopener,noreferrer");
  if (!opened) {
    router
      .push({
        name: "scoreboard-wall",
        params: { contestId: props.contestId },
        query: { mode }
      })
      .catch(() => undefined);
  }
}

function normalizeTimelineSnapshots(snapshots: ScoreboardTimelineSnapshot[]) {
  const rows = snapshots
    .map((item) => ({
      trigger_submission_id: item.trigger_submission_id,
      timestamp: item.timestamp,
      entries: [...item.entries].sort((a, b) => {
        if (a.rank !== b.rank) {
          return a.rank - b.rank;
        }
        return b.score - a.score;
      })
    }))
    .sort((a, b) => {
      const tsDiff = new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime();
      if (tsDiff !== 0) {
        return tsDiff;
      }
      return a.trigger_submission_id - b.trigger_submission_id;
    });

  const normalized: ScoreboardTimelineSnapshot[] = [];
  let lastKey = "";
  for (const item of rows) {
    const key = `${item.trigger_submission_id}:${item.timestamp}`;
    if (key === lastKey) {
      continue;
    }
    normalized.push(item);
    lastKey = key;
  }
  return normalized;
}

function sameScoreboardEntries(left: ScoreboardEntry[], right: ScoreboardEntry[]) {
  if (left.length !== right.length) {
    return false;
  }
  for (let index = 0; index < left.length; index += 1) {
    const lhs = left[index];
    const rhs = right[index];
    if (
      lhs.team_id !== rhs.team_id ||
      lhs.rank !== rhs.rank ||
      lhs.score !== rhs.score ||
      lhs.solved_count !== rhs.solved_count
    ) {
      return false;
    }
  }
  return true;
}

function appendLiveTimelineSnapshot(entries: ScoreboardEntry[]) {
  if (entries.length === 0) {
    return;
  }
  const nextEntries = entries.map((item) => ({ ...item }));
  const snapshots = [...scoreboardTimeline.value];
  const last = snapshots[snapshots.length - 1];
  if (last && sameScoreboardEntries(last.entries, nextEntries)) {
    last.timestamp = new Date().toISOString();
    return;
  }

  snapshots.push({
    trigger_submission_id: -Date.now(),
    timestamp: new Date().toISOString(),
    entries: nextEntries
  });

  const normalized = normalizeTimelineSnapshots(snapshots);
  if (normalized.length > SCOREBOARD_TREND_MAX_SNAPSHOTS) {
    scoreboardTimeline.value = normalized.slice(
      normalized.length - SCOREBOARD_TREND_MAX_SNAPSHOTS
    );
  } else {
    scoreboardTimeline.value = normalized;
  }
}

function formatTrendTimestamp(input: string) {
  return formatTimeOnly(input, {
    hour: "2-digit",
    minute: "2-digit"
  });
}

function trendPalette() {
  const isDark = document.documentElement.dataset.theme === "dark";
  if (isDark) {
    return ["#f5f5f5", "#dcdcdc", "#c6c6c6", "#b0b0b0", "#9a9a9a", "#848484", "#6e6e6e", "#5a5a5a"];
  }
  return ["#111111", "#2a2a2a", "#434343", "#5b5b5b", "#747474", "#8d8d8d", "#a5a5a5", "#bebebe"];
}

function drawScoreboardTrendChart(
  canvas: HTMLCanvasElement,
  snapshotsInput: ScoreboardTimelineSnapshot[],
  mode: "score" | "rank",
  extra?: { frameLabel?: string }
) {
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    return;
  }

  const width = canvas.width;
  const height = canvas.height;
  ctx.clearRect(0, 0, width, height);

  const isDark = document.documentElement.dataset.theme === "dark";
  const fgColor = isDark ? "rgba(243,243,243,0.94)" : "rgba(20,20,20,0.94)";
  const softColor = isDark ? "rgba(225,225,225,0.5)" : "rgba(28,28,28,0.48)";
  const gridColor = isDark ? "rgba(255,255,255,0.15)" : "rgba(0,0,0,0.14)";
  ctx.fillStyle = isDark ? "rgba(20,20,20,0.28)" : "rgba(255,255,255,0.54)";
  ctx.fillRect(0, 0, width, height);

  const snapshots = normalizeTimelineSnapshots(snapshotsInput);
  if (snapshots.length === 0) {
    ctx.fillStyle = softColor;
    ctx.font = "500 16px system-ui";
    ctx.fillText(tr("暂无趋势数据。", "No trend data yet."), 20, 34);
    return;
  }

  const latest = snapshots[snapshots.length - 1];
  const teams = latest.entries.slice(0, Math.min(8, latest.entries.length));
  if (teams.length === 0) {
    ctx.fillStyle = softColor;
    ctx.font = "500 16px system-ui";
    ctx.fillText(tr("暂无可视队伍数据。", "No team data for trend view."), 20, 34);
    return;
  }

  const palette = trendPalette();
  const left = 52;
  const right = width - 28;
  const top = 26;
  const bottom = height - 44;
  const chartWidth = Math.max(1, right - left);
  const chartHeight = Math.max(1, bottom - top);
  const maxRank = Math.max(
    teams.length,
    ...snapshots.flatMap((snapshot) => snapshot.entries.map((entry) => entry.rank))
  );

  const snapshotMaps = snapshots.map((snapshot) => {
    const map = new Map<string, ScoreboardEntry>();
    for (const entry of snapshot.entries) {
      map.set(entry.team_id, entry);
    }
    return map;
  });

  const teamSeries = teams.map((team) => {
    const values: number[] = [];
    let lastScore = 0;
    let lastRank = maxRank;
    for (const map of snapshotMaps) {
      const entry = map.get(team.team_id);
      if (entry) {
        lastScore = entry.score;
        lastRank = entry.rank;
      }
      values.push(mode === "score" ? lastScore : lastRank);
    }
    return {
      team_id: team.team_id,
      team_name: team.team_name,
      final_rank: team.rank,
      values
    };
  });

  const allValues = teamSeries.flatMap((item) => item.values);
  const minValue = mode === "score" ? 0 : 1;
  const maxValue = Math.max(minValue + 1, ...allValues);

  ctx.strokeStyle = gridColor;
  ctx.lineWidth = 1;
  for (let step = 0; step <= 4; step += 1) {
    const y = top + (chartHeight * step) / 4;
    ctx.beginPath();
    ctx.moveTo(left, y);
    ctx.lineTo(right, y);
    ctx.stroke();
  }

  for (let step = 0; step <= 5; step += 1) {
    const x = left + (chartWidth * step) / 5;
    ctx.beginPath();
    ctx.moveTo(x, top);
    ctx.lineTo(x, bottom);
    ctx.stroke();
  }

  ctx.fillStyle = softColor;
  ctx.font = "500 11px \"JetBrains Mono\", monospace";
  for (let step = 0; step <= 4; step += 1) {
    const value = maxValue - ((maxValue - minValue) * step) / 4;
    const y = top + (chartHeight * step) / 4;
    const label =
      mode === "score"
        ? Math.round(value).toString()
        : `#${Math.round(value).toString()}`;
    ctx.fillText(label, 8, y + 4);
  }

  const xOf = (index: number) =>
    left + (snapshots.length === 1 ? 0 : (chartWidth * index) / (snapshots.length - 1));
  const yOf = (value: number) => {
    const normalized = (value - minValue) / (maxValue - minValue);
    const mapped = mode === "rank" ? normalized : 1 - normalized;
    return top + mapped * chartHeight;
  };

  teamSeries.forEach((team, index) => {
    const color = palette[index % palette.length];
    ctx.strokeStyle = color;
    ctx.lineWidth = 2;
    ctx.beginPath();
    team.values.forEach((value, pointIndex) => {
      const x = xOf(pointIndex);
      const y = yOf(value);
      if (pointIndex === 0) {
        ctx.moveTo(x, y);
      } else {
        const previousY = yOf(team.values[pointIndex - 1]);
        ctx.lineTo(x, previousY);
        ctx.lineTo(x, y);
      }
    });
    ctx.stroke();

    const endX = xOf(team.values.length - 1);
    const endY = yOf(team.values[team.values.length - 1]);
    ctx.fillStyle = color;
    ctx.beginPath();
    ctx.arc(endX, endY, 3, 0, Math.PI * 2);
    ctx.fill();
  });

  ctx.fillStyle = fgColor;
  ctx.font = "600 14px system-ui";
  ctx.fillText(
    mode === "score"
      ? tr("积分动态曲线", "Score trend")
      : tr("排名动态曲线", "Rank trend"),
    left,
    18
  );

  const start = snapshots[0];
  const end = snapshots[snapshots.length - 1];
  ctx.fillStyle = softColor;
  ctx.font = "500 11px \"JetBrains Mono\", monospace";
  ctx.fillText(formatTrendTimestamp(start.timestamp), left, height - 14);
  const endLabel = formatTrendTimestamp(end.timestamp);
  const endWidth = ctx.measureText(endLabel).width;
  ctx.fillText(endLabel, right - endWidth, height - 14);

  const legendStartY = top + 8;
  teamSeries.forEach((team, index) => {
    const y = legendStartY + index * 16;
    if (y > bottom - 4) {
      return;
    }
    const color = palette[index % palette.length];
    ctx.fillStyle = color;
    ctx.fillRect(right - 190, y - 8, 12, 2);
    ctx.fillStyle = fgColor;
    ctx.font = "500 11px system-ui";
    const name = team.team_name.length > 16 ? `${team.team_name.slice(0, 15)}…` : team.team_name;
    ctx.fillText(`#${team.final_rank} ${name}`, right - 172, y - 2);
  });

  if (extra?.frameLabel) {
    const frameLabel = extra.frameLabel;
    const frameWidth = ctx.measureText(frameLabel).width;
    ctx.fillStyle = fgColor;
    ctx.fillText(frameLabel, right - frameWidth, 20);
  }
}

function scheduleTrendRender() {
  if (trendRenderFrame !== null) {
    window.cancelAnimationFrame(trendRenderFrame);
  }
  trendRenderFrame = window.requestAnimationFrame(() => {
    trendRenderFrame = null;
    const canvas = scoreboardTrendCanvasRef.value;
    if (!canvas) {
      return;
    }
    drawScoreboardTrendChart(canvas, scoreboardTimeline.value, scoreboardTrendMode.value);
  });
}

function downloadBlob(blob: Blob, filename: string) {
  const url = window.URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  window.URL.revokeObjectURL(url);
}

async function exportScoreboardTrendImage() {
  const canvas = scoreboardTrendCanvasRef.value;
  if (!canvas || scoreboardTimeline.value.length === 0) {
    return;
  }

  exportingTrendImage.value = true;
  try {
    drawScoreboardTrendChart(canvas, scoreboardTimeline.value, scoreboardTrendMode.value);
    const blob = await new Promise<Blob | null>((resolve) => {
      canvas.toBlob((result) => resolve(result), "image/png");
    });
    if (!blob) {
      throw new Error("failed to export image");
    }
    downloadBlob(blob, `contest-${props.contestId}-scoreboard-${scoreboardTrendMode.value}.png`);
    uiStore.success(tr("导出成功", "Export completed"), tr("折线图已导出。", "Chart image exported."), 2200);
  } catch (err) {
    const message =
      err instanceof Error ? err.message : tr("导出折线图失败", "Failed to export chart image");
    uiStore.error(tr("导出失败", "Export failed"), message);
  } finally {
    exportingTrendImage.value = false;
  }
}

function sampleTimelineSnapshots(
  snapshots: ScoreboardTimelineSnapshot[],
  maxFrames: number
) {
  if (snapshots.length <= maxFrames) {
    return snapshots;
  }
  const result: ScoreboardTimelineSnapshot[] = [];
  for (let i = 0; i < maxFrames; i += 1) {
    const index = Math.round((i * (snapshots.length - 1)) / (maxFrames - 1));
    result.push(snapshots[index]);
  }
  return result;
}

function sleep(ms: number) {
  return new Promise<void>((resolve) => {
    window.setTimeout(resolve, ms);
  });
}

async function exportScoreboardTrendAnimation() {
  if (scoreboardTimeline.value.length < 2) {
    return;
  }
  if (typeof MediaRecorder === "undefined") {
    uiStore.warning(
      tr("浏览器不支持", "Unsupported browser"),
      tr("当前浏览器不支持动图导出。", "This browser does not support animated export.")
    );
    return;
  }

  exportingTrendAnimation.value = true;
  try {
    const snapshots = sampleTimelineSnapshots(scoreboardTimeline.value, 180);
    const canvas = document.createElement("canvas");
    canvas.width = 920;
    canvas.height = 380;

    const supportedMime =
      MediaRecorder.isTypeSupported("video/webm;codecs=vp9")
        ? "video/webm;codecs=vp9"
        : MediaRecorder.isTypeSupported("video/webm")
          ? "video/webm"
          : "";
    if (!supportedMime) {
      throw new Error(tr("浏览器不支持 WebM 导出。", "WebM export is not supported."));
    }

    const stream = canvas.captureStream(12);
    const chunks: BlobPart[] = [];
    let recorderError: Error | null = null;
    const recorder = new MediaRecorder(stream, {
      mimeType: supportedMime,
      videoBitsPerSecond: 2_500_000
    });
    recorder.ondataavailable = (event) => {
      if (event.data.size > 0) {
        chunks.push(event.data);
      }
    };
    recorder.onerror = () => {
      recorderError = new Error("failed to record animation");
    };

    const done = new Promise<void>((resolve) => {
      recorder.onstop = () => resolve();
    });

    recorder.start();
    for (let frame = 0; frame < snapshots.length; frame += 1) {
      const frameSnapshots = snapshots.slice(0, frame + 1);
      drawScoreboardTrendChart(canvas, frameSnapshots, scoreboardTrendMode.value, {
        frameLabel: formatTrendTimestamp(snapshots[frame].timestamp)
      });
      await sleep(90);
    }
    await sleep(260);
    recorder.stop();
    await done;
    stream.getTracks().forEach((track) => track.stop());

    if (recorderError) {
      throw recorderError;
    }
    if (chunks.length === 0) {
      throw new Error("no animation data generated");
    }

    const blob = new Blob(chunks, { type: supportedMime });
    downloadBlob(blob, `contest-${props.contestId}-scoreboard-trend.webm`);
    uiStore.success(
      tr("导出成功", "Export completed"),
      tr("实时变化动图已导出。", "Realtime animated chart exported."),
      2400
    );
  } catch (err) {
    const message =
      err instanceof Error ? err.message : tr("导出实时动图失败", "Failed to export animation");
    uiStore.error(tr("导出失败", "Export failed"), message);
  } finally {
    exportingTrendAnimation.value = false;
  }
}

function accessTokenOrThrow() {
  const token = authStore.accessToken;
  if (!token) {
    throw new ApiClientError(tr("未登录或会话已失效", "Not signed in or session expired"), "unauthorized");
  }
  return token;
}

async function copyToClipboard(value: string, successMessage: string) {
  if (!value) {
    return;
  }

  try {
    await navigator.clipboard.writeText(value);
    uiStore.info(tr("已复制", "Copied"), successMessage, 1800);
  } catch {
    uiStore.warning(
      tr("复制失败", "Copy failed"),
      tr("浏览器不允许写入剪贴板。", "Clipboard access is blocked by the browser."),
      2200
    );
  }
}

async function copyNetworkAccessCommand() {
  await copyToClipboard(instanceSshCommand.value, tr("SSH 命令已复制", "SSH command copied"));
}

async function copyNetworkAccessPassword() {
  const password = instance.value?.network_access?.password ?? "";
  await copyToClipboard(password, tr("SSH 密码已复制", "SSH password copied"));
}

async function downloadWireguardConfig() {
  const challenge = selectedChallenge.value;
  if (!challenge) {
    return;
  }

  downloadingWireguardConfig.value = true;

  try {
    const token = accessTokenOrThrow();
    const response = await getInstanceWireguardConfig(props.contestId, challenge.id, token);
    const blob = new Blob([response.content], { type: "text/plain;charset=utf-8" });
    const url = window.URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = response.filename || `${challenge.id}.conf`;
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
    window.URL.revokeObjectURL(url);
    uiStore.success(tr("下载成功", "Download succeeded"), tr("WireGuard 配置已生成。", "WireGuard config generated."), 2200);
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : tr("下载配置失败", "Failed to download config");
    uiStore.error(tr("下载失败", "Download failed"), message);
  } finally {
    downloadingWireguardConfig.value = false;
  }
}

async function loadChallengeAttachments() {
  const challenge = selectedChallenge.value;
  if (!challenge) {
    challengeAttachments.value = [];
    return;
  }

  loadingChallengeAttachments.value = true;
  challengeAttachmentError.value = "";

  try {
    const token = accessTokenOrThrow();
    challengeAttachments.value = await listContestChallengeAttachments(
      props.contestId,
      challenge.id,
      token
    );
  } catch (err) {
    challengeAttachments.value = [];
    challengeAttachmentError.value =
      err instanceof ApiClientError
        ? err.message
        : tr("加载题目附件失败", "Failed to load challenge files");
  } finally {
    loadingChallengeAttachments.value = false;
  }
}

async function downloadChallengeAttachment(attachment: ContestChallengeAttachmentItem) {
  const challenge = selectedChallenge.value;
  if (!challenge) {
    return;
  }

  downloadingChallengeAttachmentId.value = attachment.id;

  try {
    const token = accessTokenOrThrow();
    const blob = await downloadContestChallengeAttachment(
      props.contestId,
      challenge.id,
      attachment.id,
      token
    );
    downloadBlob(blob, attachment.filename || `${attachment.id}.bin`);
    uiStore.success(tr("下载成功", "Download succeeded"), attachment.filename, 1800);
  } catch (err) {
    const message =
      err instanceof ApiClientError
        ? err.message
        : tr("下载附件失败", "Failed to download challenge file");
    uiStore.error(tr("下载失败", "Download failed"), message);
  } finally {
    downloadingChallengeAttachmentId.value = "";
  }
}

async function loadChallenges() {
  loadingChallenges.value = true;
  pageError.value = "";

  try {
    const token = accessTokenOrThrow();
    challenges.value = await listContestChallenges(props.contestId, token);
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : tr("加载题目失败", "Failed to load challenges");
    pageError.value = message;
    uiStore.error(tr("题目加载失败", "Failed to load challenges"), message);
    uiStore.alertError(tr("比赛空间", "Contest workspace"), message);
  } finally {
    loadingChallenges.value = false;
  }
}

async function loadAnnouncements() {
  loadingAnnouncements.value = true;
  announcementError.value = "";

  try {
    const token = accessTokenOrThrow();
    contestAnnouncements.value = await listContestAnnouncements(props.contestId, token);
  } catch (err) {
    announcementError.value = err instanceof ApiClientError ? err.message : tr("加载公告失败", "Failed to load announcements");
  } finally {
    loadingAnnouncements.value = false;
  }
}

async function loadScoreboard() {
  loadingScoreboard.value = true;
  scoreboardError.value = "";

  try {
    const token = accessTokenOrThrow();
    const [entries, timeline] = await Promise.all([
      getScoreboard(props.contestId, token),
      getScoreboardTimeline(props.contestId, token, {
        max_snapshots: SCOREBOARD_TREND_MAX_SNAPSHOTS,
        top_n: SCOREBOARD_TREND_TOP_N
      })
    ]);
    scoreboard.value = timeline.latest_entries.length > 0 ? timeline.latest_entries : entries;
    scoreboardTimeline.value = normalizeTimelineSnapshots(timeline.snapshots);
    scheduleTrendRender();
  } catch (err) {
    scoreboardError.value = err instanceof ApiClientError ? err.message : tr("加载榜单失败", "Failed to load scoreboard");
  } finally {
    loadingScoreboard.value = false;
  }
}

function teardownSocket() {
  if (reconnectTimer !== null) {
    window.clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }

  const socket = scoreboardSocket;
  scoreboardSocket = null;

  if (socket) {
    socket.onopen = null;
    socket.onmessage = null;
    socket.onerror = null;
    socket.onclose = null;
    socket.close();
  }

  wsState.value = "closed";
}

function openScoreboardSocket() {
  teardownSocket();

  let token = "";
  try {
    token = accessTokenOrThrow();
  } catch {
    return;
  }

  wsState.value = "connecting";
  const wsUrl = buildScoreboardWsUrl(props.contestId, token);
  scoreboardSocket = new WebSocket(wsUrl);

  scoreboardSocket.onopen = () => {
    wsState.value = "open";
  };

  scoreboardSocket.onmessage = (event) => {
    try {
      const payload = JSON.parse(event.data) as ScoreboardPushPayload;
      if (payload.contest_id === props.contestId && Array.isArray(payload.entries)) {
        scoreboard.value = payload.entries;
        appendLiveTimelineSnapshot(payload.entries);
        scheduleTrendRender();
      }
    } catch {
      // Ignore malformed payloads from intermediate services.
    }
  };

  scoreboardSocket.onerror = () => {
    wsState.value = "error";
  };

  scoreboardSocket.onclose = () => {
    wsState.value = "closed";
    if (!shouldReconnectScoreboard) {
      return;
    }
    reconnectTimer = window.setTimeout(() => {
      openScoreboardSocket();
    }, 2600);
  };
}

async function handleSubmitFlag() {
  const challenge = selectedChallenge.value;
  if (!challenge) {
    return;
  }

  submittingFlag.value = true;
  submitError.value = "";

  try {
    const token = accessTokenOrThrow();
    submitResult.value = await submitFlag(
      {
        contest_id: props.contestId,
        challenge_id: challenge.id,
        flag: flagInput.value
      },
      token
    );

    const verdict = submitResult.value.verdict;
    if (verdict === "accepted" && submitResult.value.score_awarded > 0) {
      uiStore.success(tr("提交正确", "Accepted"), `+${submitResult.value.score_awarded} ${tr("分", "pts")}`);
    } else if (verdict === "rate_limited" || verdict === "too_fast") {
      uiStore.warning(tr("提交过快", "Rate limited"), submitResult.value.message);
    } else {
      uiStore.info(tr("提交结果", "Submission result"), submitResult.value.message || tr("提交完成", "Submitted"));
    }

    flagInput.value = "";
    await loadScoreboard();
  } catch (err) {
    submitError.value = err instanceof ApiClientError ? err.message : tr("提交失败", "Submission failed");
    uiStore.error(tr("提交失败", "Submission failed"), submitError.value);
  } finally {
    submittingFlag.value = false;
  }
}

async function loadInstance() {
  const challenge = selectedChallenge.value;
  if (!challenge || !canManageInstance.value) {
    return;
  }

  loadingInstance.value = true;
  instanceError.value = "";

  try {
    const token = accessTokenOrThrow();
    instance.value = await getInstance(props.contestId, challenge.id, token);
  } catch (err) {
    instance.value = null;
    instanceError.value = err instanceof ApiClientError ? err.message : tr("加载实例失败", "Failed to load instance");
  } finally {
    loadingInstance.value = false;
  }
}

async function handleInstanceAction(action: "start" | "stop" | "reset" | "destroy") {
  const challenge = selectedChallenge.value;
  if (!challenge) {
    return;
  }

  mutatingInstance.value = true;
  instanceError.value = "";

  try {
    const token = accessTokenOrThrow();
    const payload = {
      contest_id: props.contestId,
      challenge_id: challenge.id
    };

    if (action === "start") {
      instance.value = await startInstance(payload, token);
      uiStore.success(tr("实例已启动", "Instance started"), tr("动态环境已进入运行状态。", "Runtime instance is running."), 2200);
    }

    if (action === "stop") {
      instance.value = await stopInstance(payload, token);
      uiStore.info(tr("实例已停止", "Instance stopped"), tr("动态环境已停止。", "Runtime instance stopped."), 2200);
    }

    if (action === "reset") {
      instance.value = await resetInstance(payload, token);
      uiStore.warning(tr("实例已重置", "Instance reset"), tr("动态环境已重新初始化。", "Runtime instance reinitialized."), 2200);
    }

    if (action === "destroy") {
      instance.value = await destroyInstance(payload, token);
      uiStore.warning(tr("实例已销毁", "Instance destroyed"), tr("动态环境已销毁。", "Runtime instance destroyed."), 2200);
    }
  } catch (err) {
    instanceError.value = err instanceof ApiClientError ? err.message : tr("实例操作失败", "Instance operation failed");
    uiStore.error(tr("实例操作失败", "Instance operation failed"), instanceError.value);
  } finally {
    mutatingInstance.value = false;
  }
}

onMounted(async () => {
  shouldReconnectScoreboard = true;
  await Promise.all([loadChallenges(), loadAnnouncements(), loadScoreboard()]);
  openScoreboardSocket();
  scheduleTrendRender();
  window.addEventListener("keydown", handleWindowKeydown);
  themeObserver = new MutationObserver(() => {
    scheduleTrendRender();
  });
  themeObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-theme"]
  });
});

onUnmounted(() => {
  shouldReconnectScoreboard = false;
  teardownSocket();
  window.removeEventListener("keydown", handleWindowKeydown);
  if (themeObserver) {
    themeObserver.disconnect();
    themeObserver = null;
  }
  if (trendRenderFrame !== null) {
    window.cancelAnimationFrame(trendRenderFrame);
    trendRenderFrame = null;
  }
});
</script>

<style scoped>
.workspace-grid {
  align-items: start;
}

.challenge-category-group {
  gap: 0.38rem;
}

.challenge-category-head {
  padding: 0.12rem 0.08rem;
}

.challenge-category-head h3 {
  margin: 0;
  font-size: 0.88rem;
  letter-spacing: 0.01em;
}

.challenge-attachments-head {
  align-items: center;
}

.challenge-description {
  margin: 0;
  padding: 0.62rem 0.68rem;
  border-radius: var(--radius-md);
  background: rgba(255, 255, 255, 0.22);
}

.challenge-attachments-head h3 {
  margin: 0;
}

.challenge-attachment-list {
  display: grid;
  gap: 0.42rem;
}

.challenge-attachment-item {
  gap: 0.28rem;
  padding: 0.56rem 0.62rem;
  background: rgba(255, 255, 255, 0.2);
}

.challenge-attachment-item p {
  margin: 0;
}

.scoreboard-head {
  align-items: flex-start;
  gap: 0.48rem;
  flex-wrap: wrap;
}

.scoreboard-actions {
  margin-left: auto;
  max-width: 100%;
  justify-content: flex-end;
  row-gap: 0.35rem;
  flex-wrap: wrap;
}

.instance-panel {
  background: rgba(255, 255, 255, 0.24);
}

.notice-card {
  background: rgba(255, 255, 255, 0.24);
}

.notice-card-clickable {
  cursor: pointer;
  transition: transform 0.16s ease, box-shadow 0.16s ease;
}

.notice-card-clickable:hover {
  transform: translateY(-1px);
  box-shadow: var(--shadow-soft);
}

.notice-card-clickable:focus-visible {
  outline: 2px solid var(--fg-0);
  outline-offset: 2px;
}

.announcement-content {
  margin: 0;
}

.announcement-preview {
  height: 10.5rem;
  overflow: hidden;
  position: relative;
}

.announcement-preview::after {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  height: 2.6rem;
  pointer-events: none;
  background: linear-gradient(to bottom, rgba(255, 255, 255, 0), rgba(250, 250, 250, 0.96));
}

.announcement-preview-hint {
  margin: 0;
  font-size: 0.82rem;
}

.announcement-modal {
  position: fixed;
  inset: 0;
  z-index: 94;
  padding: 1rem;
  display: grid;
  place-items: center;
  background: rgba(8, 8, 8, 0.42);
  backdrop-filter: blur(4px);
  -webkit-backdrop-filter: blur(4px);
}

.announcement-modal-card {
  width: min(780px, 96vw);
  max-height: min(84vh, 840px);
  padding: 0.88rem 0.94rem;
  border-radius: var(--radius-lg);
  background: var(--glass-strong);
  box-shadow: var(--shadow-strong);
  border: 1px solid var(--line-mid);
  display: grid;
  grid-template-rows: auto 1fr;
  gap: 0.72rem;
}

.announcement-modal-head {
  align-items: flex-start;
  gap: 0.8rem;
}

.announcement-modal-meta {
  margin: 0;
}

.announcement-modal-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.announcement-modal-content {
  overflow: auto;
  padding-right: 0.18rem;
}

.hint-list {
  margin: 0;
  padding-left: 1.2rem;
  display: grid;
  gap: 0.42rem;
}

.trend-toolbar {
  align-items: center;
  gap: 0.45rem;
  flex-wrap: wrap;
}

.trend-toolbar .context-menu {
  margin-left: auto;
  max-width: 100%;
}

.trend-toolbar .btn-line.active {
  background: var(--fg-0);
  color: var(--bg-0);
}

.trend-canvas-wrap {
  border-radius: var(--radius-md);
  overflow: hidden;
  background: rgba(255, 255, 255, 0.3);
  box-shadow: inset 0 -1px 0 var(--line-soft);
}

.trend-canvas-wrap canvas {
  display: block;
  width: 100%;
  height: auto;
}

:root[data-theme="dark"] .trend-canvas-wrap {
  background: var(--glass-mid);
}

:root[data-theme="dark"] .announcement-preview::after {
  background: linear-gradient(to bottom, rgba(10, 10, 10, 0), rgba(20, 20, 20, 0.96));
}

@media (max-width: 1180px) {
  .workspace-grid {
    grid-template-columns: 1fr;
  }
}
</style>
