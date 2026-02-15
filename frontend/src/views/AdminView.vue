<template>
  <section class="page-block">
    <div class="row-between">
      <div>
        <h1>管理控制台（v2）</h1>
        <p class="muted">题目管理、比赛创建与状态控制、比赛题目挂载、实例监控。</p>
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
          <h2>比赛管理</h2>
          <span class="badge">{{ contests.length }} 场</span>
        </div>

        <form class="form-grid" @submit.prevent="handleCreateContest">
          <label>
            <span>标题</span>
            <input v-model.trim="newContest.title" required />
          </label>
          <label>
            <span>slug</span>
            <input v-model.trim="newContest.slug" required />
          </label>
          <label>
            <span>描述</span>
            <input v-model="newContest.description" />
          </label>
          <label>
            <span>可见性</span>
            <select v-model="newContest.visibility">
              <option value="public">public</option>
              <option value="private">private</option>
            </select>
          </label>
          <label>
            <span>初始状态</span>
            <select v-model="newContest.status">
              <option value="draft">draft</option>
              <option value="scheduled">scheduled</option>
              <option value="running">running</option>
              <option value="ended">ended</option>
              <option value="archived">archived</option>
            </select>
          </label>
          <label>
            <span>开始时间</span>
            <input v-model="newContest.start_at" type="datetime-local" required />
          </label>
          <label>
            <span>结束时间</span>
            <input v-model="newContest.end_at" type="datetime-local" required />
          </label>
          <label>
            <span>封榜时间（可选）</span>
            <input v-model="newContest.freeze_at" type="datetime-local" />
          </label>

          <button class="primary" type="submit" :disabled="creatingContest">
            {{ creatingContest ? "创建中..." : "创建比赛" }}
          </button>
        </form>

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
                class="ghost"
                type="button"
                @click="selectContest(contest.id)"
                :disabled="selectedContestId === contest.id"
              >
                {{ selectedContestId === contest.id ? "当前管理中" : "管理挂载" }}
              </button>
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
          <h2>比赛题目挂载</h2>
          <span class="badge" v-if="selectedContest">{{ selectedContest.title }}</span>
        </div>

        <p class="muted" v-if="!selectedContest">请先在中间列选择一个比赛。</p>

        <template v-else>
          <form class="form-grid" @submit.prevent="handleUpsertBinding">
            <label>
              <span>选择题目</span>
              <select v-model="bindingForm.challenge_id" required>
                <option value="" disabled>请选择题目</option>
                <option v-for="item in challenges" :key="item.id" :value="item.id">
                  {{ item.title }} ({{ item.category }})
                </option>
              </select>
            </label>
            <label>
              <span>排序</span>
              <input v-model.number="bindingForm.sort_order" type="number" />
            </label>
            <label>
              <span>发布时间（可选）</span>
              <input v-model="bindingForm.release_at" type="datetime-local" />
            </label>
            <button class="primary" type="submit" :disabled="bindingBusy">
              {{ bindingBusy ? "处理中..." : "挂载或更新" }}
            </button>
          </form>

          <p v-if="bindingError" class="error">{{ bindingError }}</p>

          <div class="admin-list">
            <article v-for="item in contestBindings" :key="item.challenge_id" class="admin-list-item">
              <div class="row-between">
                <strong>{{ item.challenge_title }}</strong>
                <span class="badge">sort {{ item.sort_order }}</span>
              </div>
              <p class="muted mono">{{ item.challenge_category }} · {{ item.challenge_difficulty }}</p>
              <p class="muted">release_at={{ item.release_at ? formatTime(item.release_at) : '-' }}</p>
              <div class="actions-row">
                <button
                  class="ghost"
                  type="button"
                  @click="quickAdjustSort(item.challenge_id, item.sort_order - 1)"
                  :disabled="bindingBusy"
                >
                  上移
                </button>
                <button
                  class="ghost"
                  type="button"
                  @click="quickAdjustSort(item.challenge_id, item.sort_order + 1)"
                  :disabled="bindingBusy"
                >
                  下移
                </button>
                <button
                  class="ghost"
                  type="button"
                  @click="clearBindingReleaseAt(item.challenge_id)"
                  :disabled="bindingBusy"
                >
                  清除发布时间
                </button>
                <button
                  class="danger"
                  type="button"
                  @click="removeBinding(item.challenge_id)"
                  :disabled="bindingBusy"
                >
                  移除
                </button>
              </div>
            </article>
            <p v-if="contestBindings.length === 0" class="muted">当前比赛未挂载题目。</p>
          </div>
        </template>
      </section>
    </div>

    <section class="panel">
      <div class="row-between">
        <h2>运行概览</h2>
        <span v-if="runtimeOverview" class="muted">更新于 {{ formatTime(runtimeOverview.generated_at) }}</span>
      </div>

      <p v-if="runtimeError" class="error">{{ runtimeError }}</p>

      <div v-if="runtimeOverview" class="runtime-metrics">
        <article class="metric-card">
          <h3>基础规模</h3>
          <p>用户 {{ runtimeOverview.total_users }} · 队伍 {{ runtimeOverview.total_teams }}</p>
          <p>比赛 {{ runtimeOverview.total_contests }} · 题目 {{ runtimeOverview.total_challenges }}</p>
        </article>
        <article class="metric-card">
          <h3>比赛与提交</h3>
          <p>运行中比赛 {{ runtimeOverview.running_contests }}</p>
          <p>总提交 {{ runtimeOverview.total_submissions }} · 24h 提交 {{ runtimeOverview.submissions_last_24h }}</p>
        </article>
        <article class="metric-card">
          <h3>实例健康</h3>
          <p>总实例 {{ runtimeOverview.instances_total }} · 运行中 {{ runtimeOverview.instances_running }}</p>
          <p>失败 {{ runtimeOverview.instances_failed }} · 30 分钟内到期 {{ runtimeOverview.instances_expiring_within_30m }}</p>
          <p>已过期未销毁 {{ runtimeOverview.instances_expired_not_destroyed }}</p>
        </article>
      </div>

      <h3>最近失败实例</h3>
      <table v-if="runtimeOverview && runtimeOverview.recent_failed_instances.length > 0" class="scoreboard-table">
        <thead>
          <tr>
            <th>更新时间</th>
            <th>比赛</th>
            <th>队伍</th>
            <th>题目</th>
            <th>状态</th>
            <th>到期</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="item in runtimeOverview.recent_failed_instances" :key="item.id">
            <td>{{ formatTime(item.updated_at) }}</td>
            <td>{{ item.contest_title }}</td>
            <td>{{ item.team_name }}</td>
            <td>{{ item.challenge_title }}</td>
            <td>{{ item.status }}</td>
            <td>{{ item.expires_at ? formatTime(item.expires_at) : "-" }}</td>
          </tr>
        </tbody>
      </table>
      <p v-else class="muted">暂无失败实例。</p>
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

    <section class="panel">
      <div class="row-between">
        <h2>审计日志</h2>
        <button class="ghost" type="button" @click="loadAuditLogs" :disabled="auditLoading">
          {{ auditLoading ? "加载中..." : "刷新日志" }}
        </button>
      </div>

      <div class="actions-row">
        <label>
          <span>action</span>
          <input v-model.trim="auditActionFilter" placeholder="如：admin.contest.create" />
        </label>
        <label>
          <span>target_type</span>
          <input v-model.trim="auditTargetTypeFilter" placeholder="如：contest" />
        </label>
        <label>
          <span>条数</span>
          <input v-model.number="auditLimit" type="number" min="1" max="1000" />
        </label>
        <button class="ghost" type="button" @click="loadAuditLogs" :disabled="auditLoading">
          应用筛选
        </button>
      </div>

      <p v-if="auditError" class="error">{{ auditError }}</p>

      <table v-if="auditLogs.length > 0" class="scoreboard-table">
        <thead>
          <tr>
            <th>时间</th>
            <th>操作人</th>
            <th>角色</th>
            <th>action</th>
            <th>target</th>
            <th>detail</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="item in auditLogs" :key="item.id">
            <td>{{ formatTime(item.created_at) }}</td>
            <td>{{ item.actor_username ?? item.actor_user_id ?? "system" }}</td>
            <td>{{ item.actor_role }}</td>
            <td class="mono">{{ item.action }}</td>
            <td class="mono">{{ item.target_type }}{{ item.target_id ? `:${item.target_id}` : "" }}</td>
            <td class="mono audit-detail">{{ formatAuditDetail(item.detail) }}</td>
          </tr>
        </tbody>
      </table>
      <p v-else class="muted">暂无审计记录。</p>
    </section>
  </section>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";

import {
  ApiClientError,
  createAdminChallenge,
  createAdminContest,
  deleteAdminContestChallenge,
  getAdminRuntimeOverview,
  listAdminAuditLogs,
  listAdminChallenges,
  listAdminContestChallenges,
  listAdminContests,
  listAdminInstances,
  type AdminAuditLogItem,
  type AdminChallengeItem,
  type AdminContestChallengeItem,
  type AdminContestItem,
  type AdminInstanceItem,
  type AdminRuntimeOverview,
  updateAdminChallenge,
  updateAdminContestStatus,
  updateAdminContestChallenge,
  upsertAdminContestChallenge
} from "../api/client";
import { useAuthStore } from "../stores/auth";

const authStore = useAuthStore();

const challenges = ref<AdminChallengeItem[]>([]);
const contests = ref<AdminContestItem[]>([]);
const contestBindings = ref<AdminContestChallengeItem[]>([]);
const instances = ref<AdminInstanceItem[]>([]);
const auditLogs = ref<AdminAuditLogItem[]>([]);
const runtimeOverview = ref<AdminRuntimeOverview | null>(null);

const selectedContestId = ref("");

const pageError = ref("");
const challengeError = ref("");
const contestError = ref("");
const bindingError = ref("");
const instanceError = ref("");
const auditError = ref("");
const runtimeError = ref("");

const refreshing = ref(false);
const creatingChallenge = ref(false);
const creatingContest = ref(false);
const updatingChallengeId = ref("");
const updatingContestId = ref("");
const bindingBusy = ref(false);
const auditLoading = ref(false);

const instanceFilter = ref("");
const auditActionFilter = ref("");
const auditTargetTypeFilter = ref("");
const auditLimit = ref(200);
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

function localInputValue(input: Date) {
  const normalized = new Date(input.getTime() - input.getTimezoneOffset() * 60_000);
  return normalized.toISOString().slice(0, 16);
}

function localInputToIso(value: string) {
  return new Date(value).toISOString();
}

const now = new Date();
const defaultStart = localInputValue(new Date(now.getTime() + 30 * 60_000));
const defaultEnd = localInputValue(new Date(now.getTime() + 3 * 60 * 60_000));

const newContest = reactive({
  title: "",
  slug: "",
  description: "",
  visibility: "public",
  status: "draft",
  start_at: defaultStart,
  end_at: defaultEnd,
  freeze_at: ""
});

const bindingForm = reactive({
  challenge_id: "",
  sort_order: 0,
  release_at: ""
});

const selectedContest = computed(() => {
  return contests.value.find((item) => item.id === selectedContestId.value) ?? null;
});

function formatTime(input: string) {
  return new Date(input).toLocaleString();
}

function formatAuditDetail(detail: Record<string, unknown>) {
  const text = JSON.stringify(detail);
  if (!text) {
    return "{}";
  }

  if (text.length <= 180) {
    return text;
  }

  return `${text.slice(0, 180)}...`;
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
    if (!selectedContestId.value && contests.value.length > 0) {
      selectedContestId.value = contests.value[0].id;
    }
  } catch (err) {
    contestError.value = err instanceof ApiClientError ? err.message : "加载比赛失败";
  }
}

async function loadContestBindings() {
  bindingError.value = "";

  if (!selectedContestId.value) {
    contestBindings.value = [];
    return;
  }

  try {
    contestBindings.value = await listAdminContestChallenges(
      selectedContestId.value,
      accessTokenOrThrow()
    );
  } catch (err) {
    bindingError.value = err instanceof ApiClientError ? err.message : "加载挂载失败";
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

async function loadRuntimeOverview() {
  runtimeError.value = "";
  try {
    runtimeOverview.value = await getAdminRuntimeOverview(accessTokenOrThrow());
  } catch (err) {
    runtimeError.value = err instanceof ApiClientError ? err.message : "加载运行概览失败";
  }
}

async function loadAuditLogs() {
  auditLoading.value = true;
  auditError.value = "";

  try {
    auditLogs.value = await listAdminAuditLogs(accessTokenOrThrow(), {
      action: auditActionFilter.value || undefined,
      target_type: auditTargetTypeFilter.value || undefined,
      limit: Number.isFinite(auditLimit.value) ? Math.max(1, Math.min(1000, auditLimit.value)) : 200
    });
  } catch (err) {
    auditError.value = err instanceof ApiClientError ? err.message : "加载审计日志失败";
  } finally {
    auditLoading.value = false;
  }
}

async function refreshAll() {
  refreshing.value = true;
  pageError.value = "";

  try {
    await Promise.all([
      loadChallenges(),
      loadContests(),
      loadInstances(),
      loadRuntimeOverview(),
      loadAuditLogs()
    ]);
    await loadContestBindings();
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

async function handleCreateContest() {
  creatingContest.value = true;
  contestError.value = "";

  try {
    const created = await createAdminContest(
      {
        title: newContest.title,
        slug: newContest.slug,
        description: newContest.description || undefined,
        visibility: newContest.visibility,
        status: newContest.status,
        start_at: localInputToIso(newContest.start_at),
        end_at: localInputToIso(newContest.end_at),
        freeze_at: newContest.freeze_at ? localInputToIso(newContest.freeze_at) : undefined
      },
      accessTokenOrThrow()
    );

    newContest.title = "";
    newContest.slug = "";
    newContest.description = "";

    await loadContests();
    selectedContestId.value = created.id;
    await loadContestBindings();
  } catch (err) {
    contestError.value = err instanceof ApiClientError ? err.message : "创建比赛失败";
  } finally {
    creatingContest.value = false;
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

function selectContest(contestId: string) {
  selectedContestId.value = contestId;
}

async function handleUpsertBinding() {
  if (!selectedContestId.value) {
    bindingError.value = "请先选择比赛";
    return;
  }

  bindingBusy.value = true;
  bindingError.value = "";

  try {
    await upsertAdminContestChallenge(
      selectedContestId.value,
      {
        challenge_id: bindingForm.challenge_id,
        sort_order: bindingForm.sort_order,
        release_at: bindingForm.release_at ? localInputToIso(bindingForm.release_at) : undefined
      },
      accessTokenOrThrow()
    );

    await loadContestBindings();
  } catch (err) {
    bindingError.value = err instanceof ApiClientError ? err.message : "挂载失败";
  } finally {
    bindingBusy.value = false;
  }
}

async function quickAdjustSort(challengeId: string, nextSort: number) {
  if (!selectedContestId.value) {
    return;
  }

  bindingBusy.value = true;
  bindingError.value = "";

  try {
    await updateAdminContestChallenge(
      selectedContestId.value,
      challengeId,
      { sort_order: nextSort },
      accessTokenOrThrow()
    );
    await loadContestBindings();
  } catch (err) {
    bindingError.value = err instanceof ApiClientError ? err.message : "更新排序失败";
  } finally {
    bindingBusy.value = false;
  }
}

async function clearBindingReleaseAt(challengeId: string) {
  if (!selectedContestId.value) {
    return;
  }

  bindingBusy.value = true;
  bindingError.value = "";

  try {
    await updateAdminContestChallenge(
      selectedContestId.value,
      challengeId,
      { clear_release_at: true },
      accessTokenOrThrow()
    );
    await loadContestBindings();
  } catch (err) {
    bindingError.value = err instanceof ApiClientError ? err.message : "清除发布时间失败";
  } finally {
    bindingBusy.value = false;
  }
}

async function removeBinding(challengeId: string) {
  if (!selectedContestId.value) {
    return;
  }

  bindingBusy.value = true;
  bindingError.value = "";

  try {
    await deleteAdminContestChallenge(selectedContestId.value, challengeId, accessTokenOrThrow());
    await loadContestBindings();
  } catch (err) {
    bindingError.value = err instanceof ApiClientError ? err.message : "移除挂载失败";
  } finally {
    bindingBusy.value = false;
  }
}

watch(
  () => selectedContestId.value,
  () => {
    loadContestBindings();
  }
);

refreshAll();
</script>
