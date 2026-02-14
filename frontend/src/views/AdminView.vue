<template>
  <section class="page-block">
    <div class="row-between">
      <div>
        <h1>管理控制台（v1）</h1>
        <p class="muted">题目管理、比赛状态控制、实例监控。</p>
      </div>
      <button class="ghost" type="button" @click="refreshAll" :disabled="refreshing">
        {{ refreshing ? "刷新中..." : "刷新全部" }}
      </button>
    </div>

    <p v-if="pageError" class="error">{{ pageError }}</p>

    <div class="admin-grid">
      <section class="panel">
        <div class="row-between">
          <h2>题目管理</h2>
          <span class="badge">{{ challenges.length }} 条</span>
        </div>

        <form class="form-grid" @submit.prevent="handleCreateChallenge">
          <label>
            <span>标题</span>
            <input v-model.trim="newChallenge.title" required />
          </label>
          <label>
            <span>slug</span>
            <input v-model.trim="newChallenge.slug" required />
          </label>
          <label>
            <span>分类</span>
            <input v-model.trim="newChallenge.category" required />
          </label>
          <label>
            <span>分值</span>
            <input v-model.number="newChallenge.static_score" type="number" min="1" />
          </label>
          <label>
            <span>题型</span>
            <select v-model="newChallenge.challenge_type">
              <option value="static">static</option>
              <option value="dynamic">dynamic</option>
              <option value="internal">internal</option>
            </select>
          </label>
          <label>
            <span>flag 模式</span>
            <select v-model="newChallenge.flag_mode">
              <option value="static">static</option>
              <option value="dynamic">dynamic</option>
              <option value="script">script</option>
            </select>
          </label>
          <label>
            <span>flag/哈希</span>
            <input v-model="newChallenge.flag_hash" />
          </label>
          <label>
            <span>compose 模板（可选）</span>
            <textarea v-model="newChallenge.compose_template" rows="4" />
          </label>
          <label class="inline-check">
            <input v-model="newChallenge.is_visible" type="checkbox" />
            <span>立即可见</span>
          </label>

          <button class="primary" type="submit" :disabled="creatingChallenge">
            {{ creatingChallenge ? "创建中..." : "创建题目" }}
          </button>
        </form>

        <p v-if="challengeError" class="error">{{ challengeError }}</p>

        <div class="admin-list">
          <article v-for="item in challenges" :key="item.id" class="admin-list-item">
            <div class="row-between">
              <strong>{{ item.title }}</strong>
              <span class="badge">{{ item.challenge_type }}</span>
            </div>
            <p class="muted mono">{{ item.slug }} · {{ item.category }} · {{ item.flag_mode }}</p>
            <p class="muted">score={{ item.static_score }} visible={{ item.is_visible }}</p>
            <div class="actions-row">
              <button
                class="ghost"
                type="button"
                @click="toggleChallengeVisibility(item.id, !item.is_visible)"
                :disabled="updatingChallengeId === item.id"
              >
                {{ item.is_visible ? "设为隐藏" : "设为可见" }}
              </button>
            </div>
          </article>
        </div>
      </section>

      <section class="panel">
        <div class="row-between">
          <h2>比赛状态控制</h2>
          <span class="badge">{{ contests.length }} 场</span>
        </div>

        <p v-if="contestError" class="error">{{ contestError }}</p>

        <div class="admin-list">
          <article v-for="contest in contests" :key="contest.id" class="admin-list-item">
            <div class="row-between">
              <strong>{{ contest.title }}</strong>
              <span class="badge">{{ contest.status }}</span>
            </div>
            <p class="muted mono">{{ contest.slug }} · {{ contest.visibility }}</p>
            <p class="muted">{{ formatTime(contest.start_at) }} ~ {{ formatTime(contest.end_at) }}</p>
            <div class="actions-row">
              <button
                v-for="status in statusActions"
                :key="status"
                class="ghost"
                type="button"
                :disabled="updatingContestId === contest.id || contest.status === status"
                @click="updateContestStatus(contest.id, status)"
              >
                {{ status }}
              </button>
            </div>
          </article>
        </div>
      </section>

      <section class="panel">
        <div class="row-between">
          <h2>实例监控</h2>
          <label class="inline-check">
            <span>状态过滤</span>
            <select v-model="instanceFilter" @change="loadInstances">
              <option value="">all</option>
              <option value="creating">creating</option>
              <option value="running">running</option>
              <option value="stopped">stopped</option>
              <option value="destroyed">destroyed</option>
              <option value="failed">failed</option>
            </select>
          </label>
        </div>

        <p v-if="instanceError" class="error">{{ instanceError }}</p>

        <table v-if="instances.length > 0" class="scoreboard-table">
          <thead>
            <tr>
              <th>比赛</th>
              <th>队伍</th>
              <th>题目</th>
              <th>状态</th>
              <th>子网</th>
              <th>到期</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="item in instances" :key="item.id">
              <td>{{ item.contest_title }}</td>
              <td>{{ item.team_name }}</td>
              <td>{{ item.challenge_title }}</td>
              <td>{{ item.status }}</td>
              <td class="mono">{{ item.subnet }}</td>
              <td>{{ item.expires_at ? formatTime(item.expires_at) : "-" }}</td>
            </tr>
          </tbody>
        </table>
        <p v-else class="muted">暂无实例记录。</p>
      </section>
    </div>
  </section>
</template>

<script setup lang="ts">
import { reactive, ref } from "vue";

import {
  ApiClientError,
  createAdminChallenge,
  listAdminChallenges,
  listAdminContests,
  listAdminInstances,
  type AdminChallengeItem,
  type AdminContestItem,
  type AdminInstanceItem,
  updateAdminChallenge,
  updateAdminContestStatus
} from "../api/client";
import { useAuthStore } from "../stores/auth";

const authStore = useAuthStore();

const challenges = ref<AdminChallengeItem[]>([]);
const contests = ref<AdminContestItem[]>([]);
const instances = ref<AdminInstanceItem[]>([]);

const pageError = ref("");
const challengeError = ref("");
const contestError = ref("");
const instanceError = ref("");

const refreshing = ref(false);
const creatingChallenge = ref(false);
const updatingChallengeId = ref("");
const updatingContestId = ref("");

const instanceFilter = ref("");
const statusActions = ["draft", "scheduled", "running", "ended", "archived"];

const newChallenge = reactive({
  title: "",
  slug: "",
  category: "web",
  static_score: 100,
  challenge_type: "static",
  flag_mode: "static",
  flag_hash: "",
  compose_template: "",
  is_visible: false
});

function formatTime(input: string) {
  return new Date(input).toLocaleString();
}

function accessTokenOrThrow() {
  if (!authStore.accessToken) {
    throw new ApiClientError("未登录或会话过期", "unauthorized");
  }
  return authStore.accessToken;
}

async function loadChallenges() {
  challengeError.value = "";
  try {
    challenges.value = await listAdminChallenges(accessTokenOrThrow());
  } catch (err) {
    challengeError.value = err instanceof ApiClientError ? err.message : "加载题目失败";
  }
}

async function loadContests() {
  contestError.value = "";
  try {
    contests.value = await listAdminContests(accessTokenOrThrow());
  } catch (err) {
    contestError.value = err instanceof ApiClientError ? err.message : "加载比赛失败";
  }
}

async function loadInstances() {
  instanceError.value = "";
  try {
    instances.value = await listAdminInstances(accessTokenOrThrow(), {
      status: instanceFilter.value || undefined,
      limit: 150
    });
  } catch (err) {
    instanceError.value = err instanceof ApiClientError ? err.message : "加载实例失败";
  }
}

async function refreshAll() {
  refreshing.value = true;
  pageError.value = "";

  try {
    await Promise.all([loadChallenges(), loadContests(), loadInstances()]);
  } catch (err) {
    pageError.value = err instanceof ApiClientError ? err.message : "刷新失败";
  } finally {
    refreshing.value = false;
  }
}

async function handleCreateChallenge() {
  creatingChallenge.value = true;
  challengeError.value = "";

  try {
    await createAdminChallenge(
      {
        title: newChallenge.title,
        slug: newChallenge.slug,
        category: newChallenge.category,
        static_score: newChallenge.static_score,
        challenge_type: newChallenge.challenge_type,
        flag_mode: newChallenge.flag_mode,
        flag_hash: newChallenge.flag_hash,
        is_visible: newChallenge.is_visible,
        compose_template: newChallenge.compose_template || undefined
      },
      accessTokenOrThrow()
    );

    newChallenge.title = "";
    newChallenge.slug = "";
    newChallenge.flag_hash = "";
    newChallenge.compose_template = "";

    await loadChallenges();
  } catch (err) {
    challengeError.value = err instanceof ApiClientError ? err.message : "创建题目失败";
  } finally {
    creatingChallenge.value = false;
  }
}

async function toggleChallengeVisibility(challengeId: string, visible: boolean) {
  updatingChallengeId.value = challengeId;
  challengeError.value = "";

  try {
    await updateAdminChallenge(challengeId, { is_visible: visible }, accessTokenOrThrow());
    await loadChallenges();
  } catch (err) {
    challengeError.value = err instanceof ApiClientError ? err.message : "更新题目失败";
  } finally {
    updatingChallengeId.value = "";
  }
}

async function updateContestStatus(contestId: string, status: string) {
  updatingContestId.value = contestId;
  contestError.value = "";

  try {
    await updateAdminContestStatus(contestId, status, accessTokenOrThrow());
    await loadContests();
  } catch (err) {
    contestError.value = err instanceof ApiClientError ? err.message : "更新比赛状态失败";
  } finally {
    updatingContestId.value = "";
  }
}

refreshAll();
</script>
