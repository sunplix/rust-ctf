<template>
  <section class="page-layout">
    <article class="surface stack">
      <header class="section-head">
        <div class="section-title">
          <h1>{{ tr("队伍协作", "Team Collaboration") }}</h1>
        </div>
        <button class="btn-line" type="button" @click="refreshAll" :disabled="refreshing">
          {{ refreshing ? tr("刷新中...", "Refreshing...") : tr("刷新全部", "Refresh all") }}
        </button>
      </header>
      <p v-if="pageError" class="error">{{ pageError }}</p>
    </article>

    <div class="teams-grid">
      <aside class="surface stack">
        <header class="row-between">
          <h2>{{ tr("队伍大厅", "Team Lobby") }}</h2>
          <button class="btn-line" type="button" @click="loadTeams" :disabled="loadingTeams">
            {{ loadingTeams ? tr("加载中...", "Loading...") : tr("刷新", "Refresh") }}
          </button>
        </header>

        <form class="row" @submit.prevent="loadTeams">
          <input v-model.trim="keyword" :placeholder="tr('按队伍名搜索', 'Search by team name')" />
          <button class="btn-line" type="submit" :disabled="loadingTeams">{{ tr("搜索", "Search") }}</button>
        </form>

        <p v-if="teamListError" class="error">{{ teamListError }}</p>

        <div class="list-board hall-list">
          <button
            v-for="team in teams"
            :key="team.id"
            class="select-item"
            :class="{ active: team.id === selectedHallTeamId }"
            type="button"
            @click="selectedHallTeamId = team.id"
          >
            <div class="row-between">
              <strong>{{ team.name }}</strong>
              <span class="badge">{{ team.member_count }}</span>
            </div>
            <p class="soft">{{ tr("队长", "Captain") }}: {{ team.captain_username ?? team.captain_user_id }}</p>
          </button>

          <p v-if="!loadingTeams && teams.length === 0" class="soft">{{ tr("暂无队伍数据。", "No team data.") }}</p>
        </div>

        <section v-if="selectedHallTeam" class="surface stack selected-team">
          <header class="row-between">
            <h3>{{ tr("选中队伍", "Selected Team") }}</h3>
            <span class="badge">{{ selectedHallTeamView?.members.length ?? selectedHallTeam.member_count }} {{ tr("人", "members") }}</span>
          </header>
          <p><strong>{{ selectedHallTeam.name }}</strong></p>
          <p class="muted">{{ selectedHallTeamView?.description || selectedHallTeam.description || tr("暂无队伍描述。", "No team description.") }}</p>
          <p class="soft mono">
            {{ tr("队长", "Captain") }}: {{ selectedHallTeamView?.captain_username ?? selectedHallTeam.captain_username ?? selectedHallTeam.captain_user_id }}
          </p>
          <p v-if="loadingSelectedHallTeam" class="soft">{{ tr("正在加载队伍详情...", "Loading team details...") }}</p>
          <p v-if="selectedHallTeamError" class="error">{{ selectedHallTeamError }}</p>
          <div class="context-menu" v-if="selectedHallTeamId">
            <button
              class="btn-solid"
              type="button"
              :disabled="!!myTeam || actionBusy"
              @click="handleJoinById(selectedHallTeam.id, selectedHallTeam.name)"
            >
              {{ actionBusy ? tr("处理中...", "Processing...") : myTeam ? tr("已在队伍中", "Already in a team") : tr("加入该队伍", "Join team") }}
            </button>
            <button class="btn-line" type="button" @click="copyTeamId(selectedHallTeam.id)">
              {{ tr("复制 Team ID", "Copy team ID") }}
            </button>
          </div>
        </section>
      </aside>

      <main class="surface stack">
        <header class="row-between">
          <h2>{{ tr("我的队伍", "My Team") }}</h2>
          <span class="soft mono" v-if="myTeam">{{ myTeam.id }}</span>
        </header>

        <p v-if="loadingMyTeam" class="soft">{{ tr("正在加载我的队伍...", "Loading my team...") }}</p>

        <template v-else-if="myTeam">
          <section class="surface stack selected-team">
            <div class="row-between">
              <div>
                <h3>{{ myTeam.name }}</h3>
                <p class="soft">{{ tr("队长", "Captain") }}: {{ myTeam.captain_username ?? myTeam.captain_user_id }}</p>
              </div>
              <span class="badge">{{ myTeam.members.length }} {{ tr("人", "members") }}</span>
            </div>
            <p class="muted">{{ myTeam.description || tr("暂无队伍描述。", "No team description.") }}</p>
            <div class="context-menu">
              <button class="btn-line" type="button" :disabled="leaveBusy" @click="handleLeaveTeam">
                {{ leaveBusy ? tr("处理中...", "Processing...") : tr("退出队伍", "Leave team") }}
              </button>
              <button v-if="isCaptain" class="btn-danger" type="button" :disabled="disbandBusy" @click="confirmDisband = !confirmDisband">
                {{ confirmDisband ? tr("取消解散", "Cancel disband") : tr("解散队伍", "Disband team") }}
              </button>
            </div>
            <p v-if="confirmDisband" class="warn">{{ tr("再次点击执行不可恢复的解散操作：", "Click again to run irreversible disband action:") }}</p>
            <button v-if="confirmDisband" class="btn-danger" type="button" :disabled="disbandBusy" @click="handleDisbandTeam">
              {{ disbandBusy ? tr("解散中...", "Disbanding...") : tr("确认解散", "Confirm disband") }}
            </button>
          </section>

          <section v-if="isCaptain" class="stack">
            <div class="split-line"></div>
            <h3>{{ tr("队伍设置", "Team Settings") }}</h3>
            <form class="form-grid" @submit.prevent="handleUpdateTeam">
              <label>
                <span>{{ tr("队伍名称", "Team name") }}</span>
                <input v-model.trim="teamEditForm.name" required maxlength="64" />
              </label>
              <label>
                <span>{{ tr("队伍描述", "Team description") }}</span>
                <textarea v-model.trim="teamEditForm.description" rows="3" maxlength="500" />
              </label>
              <button class="btn-solid" type="submit" :disabled="teamSettingBusy">
                {{ teamSettingBusy ? tr("保存中...", "Saving...") : tr("保存修改", "Save changes") }}
              </button>
            </form>
          </section>

          <section v-if="isCaptain" class="stack">
            <div class="split-line"></div>
            <h3>{{ tr("邀请成员", "Invite Members") }}</h3>
            <form class="form-grid" @submit.prevent="handleCreateInvitation">
              <label>
                <span>{{ tr("被邀请用户名", "Invitee username") }}</span>
                <input v-model.trim="inviteForm.username" required maxlength="32" />
              </label>
              <label>
                <span>{{ tr("邀请留言（可选）", "Message (optional)") }}</span>
                <input v-model.trim="inviteForm.message" maxlength="500" />
              </label>
              <button class="btn-solid" type="submit" :disabled="inviteBusy">
                {{ inviteBusy ? tr("发送中...", "Sending...") : tr("发送邀请", "Send invite") }}
              </button>
            </form>
          </section>

          <section class="stack">
            <div class="split-line"></div>
            <h3>{{ tr("成员管理", "Member Management") }}</h3>
            <div class="list-board">
              <button
                v-for="member in myTeam.members"
                :key="member.user_id"
                class="select-item"
                :class="{ active: member.user_id === selectedMemberUserId }"
                type="button"
                @click="selectedMemberUserId = member.user_id"
              >
                <div class="row-between">
                  <strong>{{ member.username }}</strong>
                  <span class="badge">{{ member.member_role }}</span>
                </div>
                <p class="soft mono">{{ tr("加入时间", "Joined at") }}: {{ formatTime(member.joined_at) }}</p>
              </button>
            </div>

            <div class="context-menu" v-if="selectedMember && isCaptain && selectedMember.member_role !== 'captain'">
              <button
                class="btn-danger"
                type="button"
                :disabled="memberActionUserId === selectedMember.user_id"
                @click="handleRemoveMember(selectedMember.user_id, selectedMember.username)"
              >
                {{ memberActionUserId === selectedMember.user_id ? tr("处理中...", "Processing...") : tr("移除成员", "Remove member") }}
              </button>
            </div>

            <div v-if="isCaptain && transferableMembers.length > 0" class="form-grid">
              <label>
                <span>{{ tr("转让队长", "Transfer captain") }}</span>
                <select v-model="transferCaptainUserId">
                  <option value="" disabled>{{ tr("选择新队长", "Select new captain") }}</option>
                  <option v-for="member in transferableMembers" :key="member.user_id" :value="member.user_id">
                    {{ member.username }}
                  </option>
                </select>
              </label>
              <button
                class="btn-line"
                type="button"
                :disabled="!transferCaptainUserId || transferBusy"
                @click="handleTransferCaptain"
              >
                {{ transferBusy ? tr("转让中...", "Transferring...") : tr("执行转让", "Transfer") }}
              </button>
            </div>
          </section>
        </template>

        <template v-else>
          <p class="muted">{{ tr("你当前不在任何队伍中。可以创建新队伍或按名称加入。", "You are not in a team. Create one or join by team name.") }}</p>

          <section class="stack">
            <h3>{{ tr("创建队伍", "Create Team") }}</h3>
            <form class="form-grid" @submit.prevent="handleCreateTeam">
              <label>
                <span>{{ tr("队伍名", "Team name") }}</span>
                <input v-model.trim="createForm.name" required maxlength="64" />
              </label>
              <label>
                <span>{{ tr("描述（可选）", "Description (optional)") }}</span>
                <textarea v-model.trim="createForm.description" rows="3" maxlength="500" />
              </label>
              <button class="btn-solid" type="submit" :disabled="actionBusy">
                {{ actionBusy ? tr("处理中...", "Processing...") : tr("创建队伍", "Create team") }}
              </button>
            </form>
          </section>

          <section class="stack">
            <h3>{{ tr("按名称加入", "Join by Name") }}</h3>
            <form class="form-grid" @submit.prevent="handleJoinByName">
              <label>
                <span>{{ tr("队伍名称", "Team name") }}</span>
                <input v-model.trim="joinTeamName" required maxlength="64" />
              </label>
              <button class="btn-line" type="submit" :disabled="actionBusy">
                {{ actionBusy ? tr("处理中...", "Processing...") : tr("加入队伍", "Join team") }}
              </button>
            </form>
          </section>
        </template>
      </main>

      <aside class="surface stack">
        <header class="row-between">
          <h2>{{ tr("邀请中心", "Invitations") }}</h2>
          <button class="btn-line" type="button" @click="loadReceivedInvitations" :disabled="refreshing">
            {{ tr("刷新", "Refresh") }}
          </button>
        </header>

        <section class="stack">
          <h3>{{ tr("收到的邀请", "Received Invitations") }}</h3>
          <div class="list-board">
            <button
              v-for="item in receivedInvitations"
              :key="item.id"
              class="select-item"
              :class="{ active: item.id === selectedReceivedInvitationId }"
              type="button"
              @click="selectedReceivedInvitationId = item.id"
            >
              <div class="row-between">
                <strong>{{ item.team_name }}</strong>
                <span class="badge">{{ item.status }}</span>
              </div>
              <p class="soft">{{ tr("邀请人", "Inviter") }}: {{ item.inviter_username ?? item.inviter_user_id }}</p>
              <p class="soft mono">{{ formatTime(item.created_at) }}</p>
            </button>
          </div>
          <p v-if="receivedInvitations.length === 0" class="soft">{{ tr("暂无邀请。", "No invitations.") }}</p>
          <div class="context-menu" v-if="selectedReceivedInvitation?.status === 'pending' && selectedReceivedInvitationId">
            <button
              class="btn-solid"
              type="button"
              :disabled="invitationActionId === selectedReceivedInvitation.id"
              @click="handleRespondInvitation(selectedReceivedInvitation.id, 'accept')"
            >
              {{ tr("接受", "Accept") }}
            </button>
            <button
              class="btn-line"
              type="button"
              :disabled="invitationActionId === selectedReceivedInvitation.id"
              @click="handleRespondInvitation(selectedReceivedInvitation.id, 'reject')"
            >
              {{ tr("拒绝", "Reject") }}
            </button>
          </div>
        </section>

        <div class="split-line"></div>

        <section class="stack" v-if="isCaptain">
          <h3>{{ tr("已发送邀请", "Sent Invitations") }}</h3>
          <div class="list-board">
            <button
              v-for="item in sentInvitations"
              :key="item.id"
              class="select-item"
              :class="{ active: item.id === selectedSentInvitationId }"
              type="button"
              @click="selectedSentInvitationId = item.id"
            >
              <div class="row-between">
                <strong>{{ item.invitee_username ?? item.invitee_user_id }}</strong>
                <span class="badge">{{ item.status }}</span>
              </div>
              <p class="soft mono">{{ formatTime(item.created_at) }}</p>
            </button>
          </div>
          <p v-if="sentInvitations.length === 0" class="soft">{{ tr("暂无已发送邀请。", "No sent invitations.") }}</p>
          <div class="context-menu" v-if="selectedSentInvitation?.status === 'pending' && selectedSentInvitationId">
            <button
              class="btn-danger"
              type="button"
              :disabled="cancelInvitationId === selectedSentInvitation.id"
              @click="handleCancelInvitation(selectedSentInvitation.id)"
            >
              {{ cancelInvitationId === selectedSentInvitation.id ? tr("取消中...", "Canceling...") : tr("取消邀请", "Cancel invite") }}
            </button>
          </div>
        </section>
      </aside>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";

import {
  ApiClientError,
  cancelTeamInvitation,
  createTeam,
  createTeamInvitation,
  disbandTeam,
  getTeamById,
  getMyTeam,
  joinTeam,
  leaveTeam,
  listReceivedTeamInvitations,
  listSentTeamInvitations,
  listTeams,
  removeTeamMember,
  respondTeamInvitation,
  transferTeamCaptain,
  updateTeam,
  type TeamInvitationItem,
  type TeamListItem,
  type TeamMemberItem,
  type TeamProfile
} from "../api/client";
import { useL10n } from "../composables/useL10n";
import { useAuthStore } from "../stores/auth";
import { useUiStore } from "../stores/ui";

const authStore = useAuthStore();
const uiStore = useUiStore();
const { locale, tr } = useL10n();

const myTeam = ref<TeamProfile | null>(null);
const loadingMyTeam = ref(false);
const teams = ref<TeamListItem[]>([]);
const loadingTeams = ref(false);

const receivedInvitations = ref<TeamInvitationItem[]>([]);
const sentInvitations = ref<TeamInvitationItem[]>([]);
const selectedHallTeamDetail = ref<TeamProfile | null>(null);
const loadingSelectedHallTeam = ref(false);
const selectedHallTeamError = ref("");

const selectedHallTeamId = ref("");
const selectedMemberUserId = ref("");
const selectedReceivedInvitationId = ref("");
const selectedSentInvitationId = ref("");

const teamListError = ref("");
const pageError = ref("");
const refreshing = ref(false);

const actionBusy = ref(false);
const leaveBusy = ref(false);
const disbandBusy = ref(false);
const teamSettingBusy = ref(false);
const inviteBusy = ref(false);
const transferBusy = ref(false);
const memberActionUserId = ref("");
const invitationActionId = ref("");
const cancelInvitationId = ref("");
const confirmDisband = ref(false);

const keyword = ref("");
const joinTeamName = ref("");
const transferCaptainUserId = ref("");

const createForm = reactive({
  name: "",
  description: ""
});

const teamEditForm = reactive({
  name: "",
  description: ""
});

const inviteForm = reactive({
  username: "",
  message: ""
});

const isCaptain = computed(() => {
  return !!myTeam.value && myTeam.value.captain_user_id === authStore.user?.id;
});

const transferableMembers = computed(() => {
  if (!myTeam.value) {
    return [];
  }
  return myTeam.value.members.filter((member) => member.member_role !== "captain");
});

const selectedHallTeam = computed(() => {
  return teams.value.find((team) => team.id === selectedHallTeamId.value) ?? null;
});

const selectedHallTeamView = computed(() => {
  return selectedHallTeamDetail.value;
});

const selectedMember = computed<TeamMemberItem | null>(() => {
  if (!myTeam.value) {
    return null;
  }
  return myTeam.value.members.find((member) => member.user_id === selectedMemberUserId.value) ?? null;
});

const selectedReceivedInvitation = computed(() => {
  return receivedInvitations.value.find((item) => item.id === selectedReceivedInvitationId.value) ?? null;
});

const selectedSentInvitation = computed(() => {
  return sentInvitations.value.find((item) => item.id === selectedSentInvitationId.value) ?? null;
});

watch(
  () => teams.value,
  (rows) => {
    if (rows.length === 0) {
      selectedHallTeamId.value = "";
      return;
    }
    if (!selectedHallTeamId.value || !rows.some((row) => row.id === selectedHallTeamId.value)) {
      selectedHallTeamId.value = rows[0].id;
    }
  },
  { immediate: true }
);

watch(
  () => selectedHallTeamId.value,
  async (teamId) => {
    selectedHallTeamDetail.value = null;
    selectedHallTeamError.value = "";
    if (!teamId) {
      return;
    }

    loadingSelectedHallTeam.value = true;
    try {
      const token = accessTokenOrThrow();
      selectedHallTeamDetail.value = await getTeamById(teamId, token);
    } catch (err) {
      selectedHallTeamError.value =
        err instanceof ApiClientError ? err.message : tr("加载队伍详情失败", "Failed to load team details");
    } finally {
      loadingSelectedHallTeam.value = false;
    }
  },
  { immediate: true }
);

watch(
  () => myTeam.value,
  (team) => {
    confirmDisband.value = false;
    if (!team || team.members.length === 0) {
      selectedMemberUserId.value = "";
      return;
    }
    if (!selectedMemberUserId.value || !team.members.some((m) => m.user_id === selectedMemberUserId.value)) {
      selectedMemberUserId.value = team.members[0].user_id;
    }
  },
  { immediate: true }
);

watch(
  () => receivedInvitations.value,
  (rows) => {
    if (rows.length === 0) {
      selectedReceivedInvitationId.value = "";
      return;
    }
    if (!selectedReceivedInvitationId.value || !rows.some((row) => row.id === selectedReceivedInvitationId.value)) {
      selectedReceivedInvitationId.value = rows[0].id;
    }
  },
  { immediate: true }
);

watch(
  () => sentInvitations.value,
  (rows) => {
    if (rows.length === 0) {
      selectedSentInvitationId.value = "";
      return;
    }
    if (!selectedSentInvitationId.value || !rows.some((row) => row.id === selectedSentInvitationId.value)) {
      selectedSentInvitationId.value = rows[0].id;
    }
  },
  { immediate: true }
);

function formatTime(input: string) {
  const localeTag = locale.value === "en" ? "en-US" : "zh-CN";
  return new Date(input).toLocaleString(localeTag);
}

function accessTokenOrThrow() {
  const token = authStore.accessToken;
  if (!token) {
    throw new ApiClientError(tr("未登录或会话已失效", "Not signed in or session expired"), "unauthorized");
  }
  return token;
}

function syncTeamForms(team: TeamProfile | null) {
  if (!team) {
    teamEditForm.name = "";
    teamEditForm.description = "";
    transferCaptainUserId.value = "";
    return;
  }

  teamEditForm.name = team.name;
  teamEditForm.description = team.description;
  transferCaptainUserId.value = "";
}

async function copyTeamId(teamId: string) {
  try {
    await navigator.clipboard.writeText(teamId);
    uiStore.info(tr("已复制", "Copied"), tr("队伍 ID 已复制。", "Team ID copied."), 1800);
  } catch {
    uiStore.warning(
      tr("复制失败", "Copy failed"),
      tr("浏览器不允许写入剪贴板。", "Clipboard access is blocked by the browser."),
      2200
    );
  }
}

async function loadMyTeam() {
  loadingMyTeam.value = true;
  pageError.value = "";

  try {
    const token = accessTokenOrThrow();
    const data = await getMyTeam(token);
    myTeam.value = data.team;
    syncTeamForms(data.team);
  } catch (err) {
    pageError.value = err instanceof ApiClientError ? err.message : tr("加载我的队伍失败", "Failed to load my team");
  } finally {
    loadingMyTeam.value = false;
  }
}

async function loadTeams() {
  loadingTeams.value = true;
  teamListError.value = "";

  try {
    const token = accessTokenOrThrow();
    teams.value = await listTeams(token, {
      keyword: keyword.value || undefined,
      limit: 100
    });
  } catch (err) {
    teamListError.value = err instanceof ApiClientError ? err.message : tr("加载队伍列表失败", "Failed to load team list");
  } finally {
    loadingTeams.value = false;
  }
}

async function loadReceivedInvitations() {
  try {
    const token = accessTokenOrThrow();
    receivedInvitations.value = await listReceivedTeamInvitations(token, {
      limit: 100
    });
  } catch {
    receivedInvitations.value = [];
  }
}

async function loadSentInvitations() {
  if (!isCaptain.value) {
    sentInvitations.value = [];
    return;
  }

  try {
    const token = accessTokenOrThrow();
    sentInvitations.value = await listSentTeamInvitations(token, {
      limit: 100
    });
  } catch {
    sentInvitations.value = [];
  }
}

async function refreshAll() {
  refreshing.value = true;
  await loadMyTeam();
  await Promise.all([loadTeams(), loadReceivedInvitations()]);
  await loadSentInvitations();
  refreshing.value = false;
}

async function handleCreateTeam() {
  const name = createForm.name.trim();
  if (!name) {
    uiStore.warning(tr("队伍名为空", "Team name required"), tr("请输入队伍名称后再提交。", "Please enter a team name."), 2600);
    return;
  }

  actionBusy.value = true;
  pageError.value = "";

  try {
    const token = accessTokenOrThrow();
    const createdTeam = await createTeam(
      {
        name,
        description: createForm.description.trim() || undefined
      },
      token
    );
    myTeam.value = createdTeam;
    syncTeamForms(createdTeam);
    createForm.name = "";
    createForm.description = "";
    uiStore.success(
      tr("队伍创建成功", "Team created"),
      tr(`已创建队伍 ${createdTeam.name}`, `Team ${createdTeam.name} created.`)
    );
    await Promise.all([loadTeams(), loadReceivedInvitations()]);
    await loadSentInvitations();
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : tr("创建队伍失败", "Failed to create team");
    pageError.value = message;
    uiStore.error(tr("创建队伍失败", "Failed to create team"), message);
  } finally {
    actionBusy.value = false;
  }
}

async function handleJoinByName() {
  const teamName = joinTeamName.value.trim();
  if (!teamName) {
    uiStore.warning(tr("队伍名为空", "Team name required"), tr("请输入队伍名称后再提交。", "Please enter a team name."), 2600);
    return;
  }

  actionBusy.value = true;
  pageError.value = "";

  try {
    const token = accessTokenOrThrow();
    const joinedTeam = await joinTeam(
      {
        team_name: teamName
      },
      token
    );
    myTeam.value = joinedTeam;
    syncTeamForms(joinedTeam);
    joinTeamName.value = "";
    uiStore.success(tr("加入成功", "Joined"), tr(`你已加入队伍 ${joinedTeam.name}`, `You joined ${joinedTeam.name}.`));
    await Promise.all([loadTeams(), loadReceivedInvitations()]);
    await loadSentInvitations();
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : tr("加入队伍失败", "Failed to join team");
    pageError.value = message;
    uiStore.error(tr("加入队伍失败", "Failed to join team"), message);
  } finally {
    actionBusy.value = false;
  }
}

async function handleJoinById(teamId: string, teamName: string) {
  actionBusy.value = true;
  pageError.value = "";

  try {
    const token = accessTokenOrThrow();
    const joinedTeam = await joinTeam(
      {
        team_id: teamId
      },
      token
    );
    myTeam.value = joinedTeam;
    syncTeamForms(joinedTeam);
    uiStore.success(tr("加入成功", "Joined"), tr(`你已加入队伍 ${teamName}`, `You joined ${teamName}.`));
    await Promise.all([loadTeams(), loadReceivedInvitations()]);
    await loadSentInvitations();
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : tr("加入队伍失败", "Failed to join team");
    pageError.value = message;
    uiStore.error(tr("加入队伍失败", "Failed to join team"), message);
  } finally {
    actionBusy.value = false;
  }
}

async function handleLeaveTeam() {
  leaveBusy.value = true;
  pageError.value = "";

  try {
    const token = accessTokenOrThrow();
    const result = await leaveTeam(token);
    myTeam.value = null;
    syncTeamForms(null);
    sentInvitations.value = [];
    uiStore.info(
      tr("已退出队伍", "Left team"),
      result.disbanded ? tr("队伍已自动解散。", "The team was automatically disbanded.") : tr("你已退出当前队伍。", "You left the team.")
    );
    await Promise.all([loadTeams(), loadReceivedInvitations()]);
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : tr("退出队伍失败", "Failed to leave team");
    pageError.value = message;
    uiStore.error(tr("退出队伍失败", "Failed to leave team"), message);
  } finally {
    leaveBusy.value = false;
  }
}

async function handleDisbandTeam() {
  if (!myTeam.value) {
    return;
  }

  disbandBusy.value = true;
  pageError.value = "";

  try {
    const token = accessTokenOrThrow();
    await disbandTeam(myTeam.value.id, token);
    myTeam.value = null;
    syncTeamForms(null);
    sentInvitations.value = [];
    confirmDisband.value = false;
    uiStore.info(tr("队伍已解散", "Team disbanded"), tr("当前队伍已被删除。", "The team has been removed."), 2600);
    await Promise.all([loadTeams(), loadReceivedInvitations()]);
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : tr("解散队伍失败", "Failed to disband team");
    pageError.value = message;
    uiStore.error(tr("解散队伍失败", "Failed to disband team"), message);
  } finally {
    disbandBusy.value = false;
  }
}

async function handleUpdateTeam() {
  if (!myTeam.value) {
    return;
  }

  teamSettingBusy.value = true;
  pageError.value = "";

  try {
    const token = accessTokenOrThrow();
    const updatedTeam = await updateTeam(
      myTeam.value.id,
      {
        name: teamEditForm.name,
        description: teamEditForm.description
      },
      token
    );
    myTeam.value = updatedTeam;
    syncTeamForms(updatedTeam);
    uiStore.success(
      tr("队伍信息已更新", "Team updated"),
      tr("新的队伍信息已保存。", "Team changes saved."),
      2200
    );
    await loadTeams();
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : tr("更新队伍信息失败", "Failed to update team");
    pageError.value = message;
    uiStore.error(tr("更新队伍信息失败", "Failed to update team"), message);
  } finally {
    teamSettingBusy.value = false;
  }
}

async function handleCreateInvitation() {
  const username = inviteForm.username.trim();
  if (!username) {
    uiStore.warning(tr("用户名为空", "Username required"), tr("请输入被邀请用户名。", "Please enter invitee username."), 2400);
    return;
  }

  inviteBusy.value = true;
  pageError.value = "";

  try {
    const token = accessTokenOrThrow();
    await createTeamInvitation(
      {
        invitee_username: username,
        message: inviteForm.message.trim() || undefined
      },
      token
    );
    inviteForm.username = "";
    inviteForm.message = "";
    uiStore.success(
      tr("邀请已发送", "Invitation sent"),
      tr(`已向 ${username} 发送队伍邀请。`, `Invitation sent to ${username}.`),
      2200
    );
    await loadSentInvitations();
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : tr("发送邀请失败", "Failed to send invitation");
    pageError.value = message;
    uiStore.error(tr("发送邀请失败", "Failed to send invitation"), message);
  } finally {
    inviteBusy.value = false;
  }
}

async function handleCancelInvitation(invitationId: string) {
  cancelInvitationId.value = invitationId;
  pageError.value = "";

  try {
    const token = accessTokenOrThrow();
    await cancelTeamInvitation(invitationId, token);
    uiStore.info(tr("邀请已取消", "Invitation canceled"), tr("该邀请已失效。", "The invitation is now invalid."), 2200);
    await loadSentInvitations();
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : tr("取消邀请失败", "Failed to cancel invitation");
    pageError.value = message;
    uiStore.error(tr("取消邀请失败", "Failed to cancel invitation"), message);
  } finally {
    cancelInvitationId.value = "";
  }
}

async function handleRespondInvitation(invitationId: string, action: "accept" | "reject") {
  invitationActionId.value = invitationId;
  pageError.value = "";

  try {
    const token = accessTokenOrThrow();
    const result = await respondTeamInvitation(invitationId, action, token);
    if (result.team) {
      myTeam.value = result.team;
      syncTeamForms(result.team);
      uiStore.success(
        tr("已加入队伍", "Joined team"),
        tr(`你已加入 ${result.team.name}`, `You joined ${result.team.name}.`),
        2200
      );
      await loadSentInvitations();
    } else {
      uiStore.info(tr("邀请已处理", "Invitation processed"), tr("你已拒绝该邀请。", "You rejected the invitation."), 2200);
    }
    await Promise.all([loadReceivedInvitations(), loadTeams()]);
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : tr("处理邀请失败", "Failed to process invitation");
    pageError.value = message;
    uiStore.error(tr("处理邀请失败", "Failed to process invitation"), message);
  } finally {
    invitationActionId.value = "";
  }
}

async function handleTransferCaptain() {
  if (!myTeam.value || !transferCaptainUserId.value) {
    return;
  }

  transferBusy.value = true;
  pageError.value = "";

  try {
    const token = accessTokenOrThrow();
    const updatedTeam = await transferTeamCaptain(myTeam.value.id, transferCaptainUserId.value, token);
    myTeam.value = updatedTeam;
    syncTeamForms(updatedTeam);
    uiStore.success(tr("队长已转让", "Captain transferred"), tr("队伍队长角色已更新。", "Team captain role updated."), 2200);
    await Promise.all([loadTeams(), loadReceivedInvitations()]);
    await loadSentInvitations();
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : tr("转让队长失败", "Failed to transfer captain");
    pageError.value = message;
    uiStore.error(tr("转让队长失败", "Failed to transfer captain"), message);
  } finally {
    transferBusy.value = false;
  }
}

async function handleRemoveMember(memberUserId: string, username: string) {
  if (!myTeam.value) {
    return;
  }

  memberActionUserId.value = memberUserId;
  pageError.value = "";

  try {
    const token = accessTokenOrThrow();
    const updatedTeam = await removeTeamMember(myTeam.value.id, memberUserId, token);
    myTeam.value = updatedTeam;
    syncTeamForms(updatedTeam);
    uiStore.info(tr("成员已移除", "Member removed"), tr(`${username} 已移出队伍。`, `${username} was removed from team.`), 2200);
    await loadTeams();
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : tr("移除成员失败", "Failed to remove member");
    pageError.value = message;
    uiStore.error(tr("移除成员失败", "Failed to remove member"), message);
  } finally {
    memberActionUserId.value = "";
  }
}

onMounted(async () => {
  await refreshAll();
});
</script>

<style scoped>
.teams-grid {
  display: grid;
  gap: 0.82rem;
  grid-template-columns: minmax(0, 0.94fr) minmax(0, 1.2fr) minmax(0, 0.9fr);
  align-items: start;
}

.hall-list {
  max-height: 42vh;
  overflow: auto;
  padding-right: 0.16rem;
}

.selected-team {
  background: rgba(255, 255, 255, 0.24);
}

@media (max-width: 1320px) {
  .teams-grid {
    grid-template-columns: 1fr;
  }

  .hall-list {
    max-height: none;
  }
}
</style>
