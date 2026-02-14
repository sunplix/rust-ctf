<template>
  <section class="page-block">
    <div class="row-between">
      <div>
        <p class="muted mono">contest_id: {{ contestId }}</p>
        <h1>题目与提交</h1>
      </div>
      <RouterLink class="ghost-link" to="/contests">返回比赛列表</RouterLink>
    </div>

    <p v-if="pageError" class="error">{{ pageError }}</p>

    <div class="layout-grid">
      <aside class="panel challenge-panel">
        <div class="row-between">
          <h2>题目</h2>
          <button class="ghost" type="button" @click="loadChallenges" :disabled="loadingChallenges">
            {{ loadingChallenges ? "刷新中..." : "刷新" }}
          </button>
        </div>

        <div v-if="challenges.length === 0" class="muted">暂无可见题目。</div>

        <button
          v-for="item in challenges"
          :key="item.id"
          class="challenge-item"
          :class="{ active: item.id === selectedChallengeId }"
          type="button"
          @click="selectedChallengeId = item.id"
        >
          <strong>{{ item.title }}</strong>
          <span>{{ item.category }} / {{ item.difficulty }}</span>
          <span>分值: {{ item.static_score }}</span>
        </button>
      </aside>

      <main class="panel detail-panel">
        <template v-if="selectedChallenge">
          <div class="row-between">
            <h2>{{ selectedChallenge.title }}</h2>
            <span class="badge">{{ selectedChallenge.challenge_type }}</span>
          </div>

          <p class="muted">
            分类: {{ selectedChallenge.category }} · 难度: {{ selectedChallenge.difficulty }} · 分值:
            {{ selectedChallenge.static_score }}
          </p>

          <form class="form-grid" @submit.prevent="handleSubmitFlag">
            <label>
              <span>Flag</span>
              <input v-model.trim="flagInput" placeholder="ctf{...}" required />
            </label>
            <button class="primary" type="submit" :disabled="submittingFlag">
              {{ submittingFlag ? "提交中..." : "提交 Flag" }}
            </button>
          </form>

          <p v-if="submitResult" class="message mono">
            verdict={{ submitResult.verdict }} score_awarded={{ submitResult.score_awarded }} total={{ submitResult.total_score }}
            message={{ submitResult.message }}
          </p>
          <p v-if="submitError" class="error">{{ submitError }}</p>

          <section v-if="canManageInstance" class="instance-block">
            <div class="row-between">
              <h3>实例环境</h3>
              <button class="ghost" type="button" @click="loadInstance" :disabled="loadingInstance">
                {{ loadingInstance ? "同步中..." : "同步状态" }}
              </button>
            </div>

            <div class="actions-row">
              <button class="primary" type="button" @click="handleInstanceAction('start')" :disabled="instanceBusy">
                启动
              </button>
              <button class="ghost" type="button" @click="handleInstanceAction('stop')" :disabled="instanceBusy">
                停止
              </button>
              <button class="ghost" type="button" @click="handleInstanceAction('reset')" :disabled="instanceBusy">
                重置
              </button>
              <button class="danger" type="button" @click="handleInstanceAction('destroy')" :disabled="instanceBusy">
                销毁
              </button>
            </div>

            <div v-if="instance" class="instance-meta mono">
              <p>status={{ instance.status }}</p>
              <p>subnet={{ instance.subnet }}</p>
              <p>entrypoint={{ instance.entrypoint_url || '-' }}</p>
              <p>expires_at={{ instance.expires_at || '-' }}</p>
              <p>message={{ instance.message }}</p>
            </div>
            <p v-else class="muted">尚未创建实例。</p>

            <p v-if="instanceError" class="error">{{ instanceError }}</p>
          </section>
        </template>

        <p v-else class="muted">请选择一个题目。</p>
      </main>

      <aside class="panel scoreboard-panel">
        <div class="row-between">
          <h2>实时榜单</h2>
          <button class="ghost" type="button" @click="loadScoreboard" :disabled="loadingScoreboard">
            {{ loadingScoreboard ? "刷新中..." : "刷新" }}
          </button>
        </div>
        <p class="muted mono">ws={{ wsState }}</p>

        <p v-if="scoreboardError" class="error">{{ scoreboardError }}</p>

        <table v-if="scoreboard.length > 0" class="scoreboard-table">
          <thead>
            <tr>
              <th>#</th>
              <th>队伍</th>
              <th>分数</th>
              <th>解题</th>
              <th>最后提交</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="entry in scoreboard" :key="entry.team_id">
              <td>{{ entry.rank }}</td>
              <td>{{ entry.team_name }}</td>
              <td>{{ entry.score }}</td>
              <td>{{ entry.solved_count }}</td>
              <td>{{ entry.last_submit_at ? formatTime(entry.last_submit_at) : "-" }}</td>
            </tr>
          </tbody>
        </table>
        <p v-else class="muted">暂无榜单数据。</p>
      </aside>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";

import {
  ApiClientError,
  buildScoreboardWsUrl,
  destroyInstance,
  getInstance,
  getScoreboard,
  listContestChallenges,
  type ContestChallengeItem,
  type InstanceResponse,
  type ScoreboardEntry,
  type ScoreboardPushPayload,
  resetInstance,
  startInstance,
  stopInstance,
  submitFlag,
  type SubmitFlagResponse
} from "../api/client";
import { useAuthStore } from "../stores/auth";

const props = defineProps<{
  contestId: string;
}>();

const authStore = useAuthStore();

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
const instanceError = ref("");

const scoreboard = ref<ScoreboardEntry[]>([]);
const loadingScoreboard = ref(false);
const scoreboardError = ref("");
const wsState = ref<"disconnected" | "connecting" | "connected" | "error">("disconnected");

let ws: WebSocket | null = null;
let scoreboardPollTimer: number | null = null;

const selectedChallenge = computed(() =>
  challenges.value.find((item) => item.id === selectedChallengeId.value) ?? null
);

const canManageInstance = computed(() => {
  return (
    selectedChallenge.value?.challenge_type === "dynamic" ||
    selectedChallenge.value?.challenge_type === "internal"
  );
});

const instanceBusy = computed(() => loadingInstance.value || mutatingInstance.value);

function formatTime(input: string) {
  return new Date(input).toLocaleString();
}

function requireAccessToken() {
  const token = authStore.accessToken;
  if (!token) {
    throw new ApiClientError("未登录或会话已失效", "unauthorized");
  }
  return token;
}

async function loadChallenges() {
  loadingChallenges.value = true;
  pageError.value = "";

  try {
    const token = requireAccessToken();
    challenges.value = await listContestChallenges(props.contestId, token);

    if (!selectedChallengeId.value || !challenges.value.some((c) => c.id === selectedChallengeId.value)) {
      selectedChallengeId.value = challenges.value[0]?.id ?? "";
    }
  } catch (err) {
    pageError.value = err instanceof ApiClientError ? err.message : "加载题目失败";
  } finally {
    loadingChallenges.value = false;
  }
}

async function loadScoreboard() {
  loadingScoreboard.value = true;
  scoreboardError.value = "";

  try {
    const token = requireAccessToken();
    scoreboard.value = await getScoreboard(props.contestId, token);
  } catch (err) {
    scoreboardError.value = err instanceof ApiClientError ? err.message : "加载榜单失败";
  } finally {
    loadingScoreboard.value = false;
  }
}

function startScoreboardWs() {
  stopScoreboardWs();

  let token = "";
  try {
    token = requireAccessToken();
  } catch {
    wsState.value = "error";
    return;
  }

  wsState.value = "connecting";
  const wsUrl = buildScoreboardWsUrl(props.contestId, token);

  ws = new WebSocket(wsUrl);
  ws.onopen = () => {
    wsState.value = "connected";
  };

  ws.onmessage = (event) => {
    try {
      const payload = JSON.parse(event.data) as ScoreboardPushPayload;
      if (payload.event === "scoreboard_update" && Array.isArray(payload.entries)) {
        scoreboard.value = payload.entries;
      }
    } catch {
      // Ignore malformed push payload.
    }
  };

  ws.onerror = () => {
    wsState.value = "error";
  };

  ws.onclose = () => {
    if (wsState.value !== "error") {
      wsState.value = "disconnected";
    }
  };
}

function stopScoreboardWs() {
  if (ws) {
    ws.close();
    ws = null;
  }
}

function startScoreboardPolling() {
  stopScoreboardPolling();
  scoreboardPollTimer = window.setInterval(() => {
    loadScoreboard();
  }, 15000);
}

function stopScoreboardPolling() {
  if (scoreboardPollTimer) {
    window.clearInterval(scoreboardPollTimer);
    scoreboardPollTimer = null;
  }
}

async function handleSubmitFlag() {
  if (!selectedChallenge.value) {
    return;
  }

  submittingFlag.value = true;
  submitError.value = "";

  try {
    const token = requireAccessToken();
    submitResult.value = await submitFlag(
      {
        contest_id: props.contestId,
        challenge_id: selectedChallenge.value.id,
        flag: flagInput.value
      },
      token
    );

    flagInput.value = "";
    await loadScoreboard();
  } catch (err) {
    submitError.value = err instanceof ApiClientError ? err.message : "提交失败";
  } finally {
    submittingFlag.value = false;
  }
}

async function loadInstance() {
  if (!selectedChallenge.value || !canManageInstance.value) {
    instance.value = null;
    return;
  }

  loadingInstance.value = true;
  instanceError.value = "";

  try {
    const token = requireAccessToken();
    instance.value = await getInstance(props.contestId, selectedChallenge.value.id, token);
  } catch (err) {
    if (err instanceof ApiClientError) {
      if (err.code === "bad_request" && err.message.includes("instance not found")) {
        instance.value = null;
      } else {
        instanceError.value = err.message;
      }
    } else {
      instanceError.value = "查询实例失败";
    }
  } finally {
    loadingInstance.value = false;
  }
}

async function handleInstanceAction(action: "start" | "stop" | "reset" | "destroy") {
  if (!selectedChallenge.value) {
    return;
  }

  mutatingInstance.value = true;
  instanceError.value = "";

  try {
    const token = requireAccessToken();
    const payload = {
      contest_id: props.contestId,
      challenge_id: selectedChallenge.value.id
    };

    if (action === "start") {
      instance.value = await startInstance(payload, token);
    } else if (action === "stop") {
      instance.value = await stopInstance(payload, token);
    } else if (action === "reset") {
      instance.value = await resetInstance(payload, token);
    } else {
      instance.value = await destroyInstance(payload, token);
    }
  } catch (err) {
    instanceError.value = err instanceof ApiClientError ? err.message : "实例操作失败";
  } finally {
    mutatingInstance.value = false;
  }
}

watch(
  () => selectedChallengeId.value,
  () => {
    submitResult.value = null;
    submitError.value = "";
    loadInstance();
  }
);

onMounted(async () => {
  await loadChallenges();
  await loadScoreboard();
  startScoreboardWs();
  startScoreboardPolling();
  await loadInstance();
});

onUnmounted(() => {
  stopScoreboardWs();
  stopScoreboardPolling();
});
</script>
