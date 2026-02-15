<template>
  <section class="page-block">
    <div class="row-between">
      <div>
        <h1>队伍中心</h1>
        <p class="muted">创建队伍、邀请成员、转让队长、离队/解散并管理成员。</p>
      </div>
      <RouterLink class="ghost-link" to="/contests">返回比赛中心</RouterLink>
    </div>

    <p v-if="pageError" class="error">{{ pageError }}</p>

    <div class="team-layout">
      <article class="panel team-panel">
        <div class="row-between">
          <h2>我的队伍</h2>
          <button class="ghost" type="button" @click="refreshAll" :disabled="refreshing">
            {{ refreshing ? "刷新中..." : "刷新" }}
          </button>
        </div>

        <div v-if="loadingMyTeam" class="muted">正在加载我的队伍...</div>

        <template v-else-if="myTeam">
          <div class="team-card">
            <div class="row-between">
              <strong>{{ myTeam.name }}</strong>
              <span class="badge">{{ myTeam.members.length }} 人</span>
            </div>
            <p class="muted">队长: {{ myTeam.captain_username ?? myTeam.captain_user_id }}</p>
            <p v-if="myTeam.description" class="muted">{{ myTeam.description }}</p>
            <p class="muted mono">team_id: {{ myTeam.id }}</p>
          </div>

          <div class="actions-row">
            <button class="ghost" type="button" :disabled="leaveBusy" @click="handleLeaveTeam">
              {{ leaveBusy ? "处理中..." : "退出队伍" }}
            </button>
            <button
              v-if="isCaptain"
              class="danger"
              type="button"
              :disabled="disbandBusy"
              @click="handleDisbandTeam"
            >
              {{ disbandBusy ? "解散中..." : "解散队伍" }}
            </button>
          </div>

          <section v-if="isCaptain" class="panel">
            <h3>队伍设置</h3>
            <form class="form-grid" @submit.prevent="handleUpdateTeam">
              <label>
                <span>队伍名称</span>
                <input v-model.trim="teamEditForm.name" maxlength="64" required />
              </label>
              <label>
                <span>队伍描述</span>
                <textarea v-model.trim="teamEditForm.description" rows="3" maxlength="500" />
              </label>
              <button class="primary" type="submit" :disabled="teamSettingBusy">
                {{ teamSettingBusy ? "保存中..." : "保存队伍信息" }}
              </button>
            </form>
          </section>

          <section v-if="isCaptain" class="panel">
            <h3>邀请成员</h3>
            <form class="form-grid" @submit.prevent="handleCreateInvitation">
              <label>
                <span>被邀请用户名</span>
                <input v-model.trim="inviteForm.username" maxlength="32" required />
              </label>
              <label>
                <span>邀请留言（可选）</span>
                <input v-model.trim="inviteForm.message" maxlength="500" />
              </label>
              <button class="primary" type="submit" :disabled="inviteBusy">
                {{ inviteBusy ? "发送中..." : "发送邀请" }}
              </button>
            </form>

            <h4>已发送邀请</h4>
            <table v-if="sentInvitations.length > 0" class="scoreboard-table">
              <thead>
                <tr>
                  <th>被邀请人</th>
                  <th>状态</th>
                  <th>时间</th>
                  <th>操作</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="item in sentInvitations" :key="item.id">
                  <td>{{ item.invitee_username ?? item.invitee_user_id }}</td>
                  <td>{{ item.status }}</td>
                  <td>{{ formatTime(item.created_at) }}</td>
                  <td>
                    <button
                      v-if="item.status === 'pending'"
                      class="ghost"
                      type="button"
                      :disabled="cancelInvitationId === item.id"
                      @click="handleCancelInvitation(item.id)"
                    >
                      {{ cancelInvitationId === item.id ? "取消中..." : "取消邀请" }}
                    </button>
                    <span v-else>-</span>
                  </td>
                </tr>
              </tbody>
            </table>
            <p v-else class="muted">暂无邀请记录。</p>
          </section>

          <section v-if="isCaptain" class="panel">
            <h3>队长转让</h3>
            <div class="actions-row">
              <select v-model="transferCaptainUserId">
                <option value="" disabled>选择新队长</option>
                <option v-for="member in transferableMembers" :key="member.user_id" :value="member.user_id">
                  {{ member.username }}
                </option>
              </select>
              <button
                class="ghost"
                type="button"
                :disabled="transferBusy || !transferCaptainUserId"
                @click="handleTransferCaptain"
              >
                {{ transferBusy ? "转让中..." : "转让队长" }}
              </button>
            </div>
          </section>

          <div class="team-members">
            <h3>成员列表</h3>
            <table class="scoreboard-table">
              <thead>
                <tr>
                  <th>用户名</th>
                  <th>角色</th>
                  <th>加入时间</th>
                  <th v-if="isCaptain">操作</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="member in myTeam.members" :key="member.user_id">
                  <td>{{ member.username }}</td>
                  <td>{{ member.member_role }}</td>
                  <td>{{ formatTime(member.joined_at) }}</td>
                  <td v-if="isCaptain">
                    <button
                      v-if="member.member_role !== 'captain'"
                      class="ghost"
                      type="button"
                      :disabled="memberActionUserId === member.user_id"
                      @click="handleRemoveMember(member.user_id, member.username)"
                    >
                      {{ memberActionUserId === member.user_id ? "处理中..." : "移除成员" }}
                    </button>
                    <span v-else>-</span>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </template>

        <template v-else>
          <p class="muted">你当前还没有队伍。先创建，或通过队伍名加入已有队伍。</p>

          <form class="form-grid" @submit.prevent="handleCreateTeam">
            <label>
              <span>队伍名称</span>
              <input v-model.trim="createForm.name" maxlength="64" placeholder="例如: TeamAlpha" required />
            </label>
            <label>
              <span>队伍描述（可选）</span>
              <textarea
                v-model.trim="createForm.description"
                rows="3"
                maxlength="500"
                placeholder="介绍队伍方向或成员信息"
              />
            </label>
            <button class="primary" type="submit" :disabled="actionBusy">
              {{ actionBusy ? "处理中..." : "创建队伍" }}
            </button>
          </form>

          <form class="form-grid" @submit.prevent="handleJoinByName">
            <label>
              <span>按队伍名加入</span>
              <input v-model.trim="joinTeamName" maxlength="64" placeholder="输入队伍名称" required />
            </label>
            <button class="ghost" type="submit" :disabled="actionBusy">
              {{ actionBusy ? "处理中..." : "加入队伍" }}
            </button>
          </form>

          <section class="panel">
            <h3>收到的邀请</h3>
            <table v-if="receivedInvitations.length > 0" class="scoreboard-table">
              <thead>
                <tr>
                  <th>队伍</th>
                  <th>邀请人</th>
                  <th>状态</th>
                  <th>时间</th>
                  <th>操作</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="item in receivedInvitations" :key="item.id">
                  <td>{{ item.team_name }}</td>
                  <td>{{ item.inviter_username ?? item.inviter_user_id }}</td>
                  <td>{{ item.status }}</td>
                  <td>{{ formatTime(item.created_at) }}</td>
                  <td>
                    <div v-if="item.status === 'pending'" class="actions-row">
                      <button
                        class="primary"
                        type="button"
                        :disabled="invitationActionId === item.id"
                        @click="handleRespondInvitation(item.id, 'accept')"
                      >
                        接受
                      </button>
                      <button
                        class="ghost"
                        type="button"
                        :disabled="invitationActionId === item.id"
                        @click="handleRespondInvitation(item.id, 'reject')"
                      >
                        拒绝
                      </button>
                    </div>
                    <span v-else>-</span>
                  </td>
                </tr>
              </tbody>
            </table>
            <p v-else class="muted">暂无待处理邀请。</p>
          </section>
        </template>
      </article>

      <article class="panel team-panel">
        <div class="row-between">
          <h2>队伍大厅</h2>
          <button class="ghost" type="button" @click="loadTeams" :disabled="loadingTeams">
            {{ loadingTeams ? "加载中..." : "刷新列表" }}
          </button>
        </div>

        <form class="team-search" @submit.prevent="loadTeams">
          <input v-model.trim="keyword" placeholder="搜索队伍名" />
          <button class="ghost" type="submit" :disabled="loadingTeams">搜索</button>
        </form>

        <p v-if="teamListError" class="error">{{ teamListError }}</p>

        <div v-if="loadingTeams && teams.length === 0" class="muted">正在加载队伍列表...</div>
        <div v-else-if="teams.length === 0" class="muted">暂无队伍数据。</div>

        <div v-else class="team-list stagger-list">
          <article v-for="team in teams" :key="team.id" class="admin-list-item team-list-item">
            <div class="row-between">
              <strong>{{ team.name }}</strong>
              <span class="badge">{{ team.member_count }} 人</span>
            </div>
            <p class="muted">队长: {{ team.captain_username ?? team.captain_user_id }}</p>
            <p v-if="team.description" class="muted">{{ team.description }}</p>
            <p class="muted mono">team_id: {{ team.id }}</p>

            <button
              class="primary"
              type="button"
              :disabled="!!myTeam || actionBusy"
              @click="handleJoinById(team.id, team.name)"
            >
              {{ actionBusy ? "处理中..." : "加入该队伍" }}
            </button>
          </article>
        </div>
      </article>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";

import {
  ApiClientError,
  cancelTeamInvitation,
  createTeam,
  createTeamInvitation,
  disbandTeam,
  getMyTeam,
  joinTeam,
  leaveTeam,
  listReceivedTeamInvitations,
  listSentTeamInvitations,
  listTeams,
  removeTeamMember,
  respondTeamInvitation,
  transferTeamCaptain,
  type TeamInvitationItem,
  type TeamListItem,
  type TeamProfile,
  updateTeam
} from "../api/client";
import { useAuthStore } from "../stores/auth";
import { useUiStore } from "../stores/ui";

const authStore = useAuthStore();
const uiStore = useUiStore();

const myTeam = ref<TeamProfile | null>(null);
const loadingMyTeam = ref(false);
const teams = ref<TeamListItem[]>([]);
const loadingTeams = ref(false);

const receivedInvitations = ref<TeamInvitationItem[]>([]);
const sentInvitations = ref<TeamInvitationItem[]>([]);

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

async function loadMyTeam() {
  loadingMyTeam.value = true;
  pageError.value = "";

  try {
    const token = requireAccessToken();
    const data = await getMyTeam(token);
    myTeam.value = data.team;
    syncTeamForms(data.team);
  } catch (err) {
    pageError.value = err instanceof ApiClientError ? err.message : "加载我的队伍失败";
  } finally {
    loadingMyTeam.value = false;
  }
}

async function loadTeams() {
  loadingTeams.value = true;
  teamListError.value = "";

  try {
    const token = requireAccessToken();
    teams.value = await listTeams(token, {
      keyword: keyword.value || undefined,
      limit: 100
    });
  } catch (err) {
    teamListError.value = err instanceof ApiClientError ? err.message : "加载队伍列表失败";
  } finally {
    loadingTeams.value = false;
  }
}

async function loadReceivedInvitations() {
  try {
    const token = requireAccessToken();
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
    const token = requireAccessToken();
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
    uiStore.warning("队伍名为空", "请输入队伍名称后再提交。", 2600);
    return;
  }

  actionBusy.value = true;
  pageError.value = "";

  try {
    const token = requireAccessToken();
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
    uiStore.success("队伍创建成功", `已创建队伍 ${createdTeam.name}`);
    await Promise.all([loadTeams(), loadReceivedInvitations()]);
    await loadSentInvitations();
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : "创建队伍失败";
    pageError.value = message;
    uiStore.error("创建队伍失败", message);
  } finally {
    actionBusy.value = false;
  }
}

async function handleJoinByName() {
  const teamName = joinTeamName.value.trim();
  if (!teamName) {
    uiStore.warning("队伍名为空", "请输入队伍名称后再提交。", 2600);
    return;
  }

  actionBusy.value = true;
  pageError.value = "";

  try {
    const token = requireAccessToken();
    const joinedTeam = await joinTeam(
      {
        team_name: teamName
      },
      token
    );
    myTeam.value = joinedTeam;
    syncTeamForms(joinedTeam);
    joinTeamName.value = "";
    uiStore.success("加入成功", `你已加入队伍 ${joinedTeam.name}`);
    await Promise.all([loadTeams(), loadReceivedInvitations()]);
    await loadSentInvitations();
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : "加入队伍失败";
    pageError.value = message;
    uiStore.error("加入队伍失败", message);
  } finally {
    actionBusy.value = false;
  }
}

async function handleJoinById(teamId: string, teamName: string) {
  actionBusy.value = true;
  pageError.value = "";

  try {
    const token = requireAccessToken();
    const joinedTeam = await joinTeam(
      {
        team_id: teamId
      },
      token
    );
    myTeam.value = joinedTeam;
    syncTeamForms(joinedTeam);
    uiStore.success("加入成功", `你已加入队伍 ${teamName}`);
    await Promise.all([loadTeams(), loadReceivedInvitations()]);
    await loadSentInvitations();
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : "加入队伍失败";
    pageError.value = message;
    uiStore.error("加入队伍失败", message);
  } finally {
    actionBusy.value = false;
  }
}

async function handleLeaveTeam() {
  leaveBusy.value = true;
  pageError.value = "";

  try {
    const token = requireAccessToken();
    const result = await leaveTeam(token);
    myTeam.value = null;
    syncTeamForms(null);
    sentInvitations.value = [];
    uiStore.info("已退出队伍", result.disbanded ? "队伍已自动解散。" : "你已退出当前队伍。");
    await Promise.all([loadTeams(), loadReceivedInvitations()]);
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : "退出队伍失败";
    pageError.value = message;
    uiStore.error("退出队伍失败", message);
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
    const token = requireAccessToken();
    await disbandTeam(myTeam.value.id, token);
    myTeam.value = null;
    syncTeamForms(null);
    sentInvitations.value = [];
    uiStore.info("队伍已解散", "当前队伍已被删除。");
    await Promise.all([loadTeams(), loadReceivedInvitations()]);
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : "解散队伍失败";
    pageError.value = message;
    uiStore.error("解散队伍失败", message);
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
    const token = requireAccessToken();
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
    uiStore.success("队伍信息已更新", "新的队伍信息已保存。");
    await loadTeams();
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : "更新队伍信息失败";
    pageError.value = message;
    uiStore.error("更新队伍信息失败", message);
  } finally {
    teamSettingBusy.value = false;
  }
}

async function handleCreateInvitation() {
  const username = inviteForm.username.trim();
  if (!username) {
    uiStore.warning("用户名为空", "请输入被邀请用户名。");
    return;
  }

  inviteBusy.value = true;
  pageError.value = "";

  try {
    const token = requireAccessToken();
    await createTeamInvitation(
      {
        invitee_username: username,
        message: inviteForm.message.trim() || undefined
      },
      token
    );
    inviteForm.username = "";
    inviteForm.message = "";
    uiStore.success("邀请已发送", `已向 ${username} 发送队伍邀请。`);
    await loadSentInvitations();
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : "发送邀请失败";
    pageError.value = message;
    uiStore.error("发送邀请失败", message);
  } finally {
    inviteBusy.value = false;
  }
}

async function handleCancelInvitation(invitationId: string) {
  cancelInvitationId.value = invitationId;
  pageError.value = "";

  try {
    const token = requireAccessToken();
    await cancelTeamInvitation(invitationId, token);
    uiStore.info("邀请已取消", "该邀请已失效。");
    await loadSentInvitations();
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : "取消邀请失败";
    pageError.value = message;
    uiStore.error("取消邀请失败", message);
  } finally {
    cancelInvitationId.value = "";
  }
}

async function handleRespondInvitation(invitationId: string, action: "accept" | "reject") {
  invitationActionId.value = invitationId;
  pageError.value = "";

  try {
    const token = requireAccessToken();
    const result = await respondTeamInvitation(invitationId, action, token);
    if (result.team) {
      myTeam.value = result.team;
      syncTeamForms(result.team);
      uiStore.success("已加入队伍", `你已加入 ${result.team.name}`);
      await loadSentInvitations();
    } else {
      uiStore.info("邀请已处理", "你已拒绝该邀请。");
    }
    await Promise.all([loadReceivedInvitations(), loadTeams()]);
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : "处理邀请失败";
    pageError.value = message;
    uiStore.error("处理邀请失败", message);
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
    const token = requireAccessToken();
    const updatedTeam = await transferTeamCaptain(
      myTeam.value.id,
      transferCaptainUserId.value,
      token
    );
    myTeam.value = updatedTeam;
    syncTeamForms(updatedTeam);
    uiStore.success("队长已转让", "队伍队长角色已更新。");
    await Promise.all([loadTeams(), loadReceivedInvitations()]);
    await loadSentInvitations();
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : "转让队长失败";
    pageError.value = message;
    uiStore.error("转让队长失败", message);
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
    const token = requireAccessToken();
    const updatedTeam = await removeTeamMember(myTeam.value.id, memberUserId, token);
    myTeam.value = updatedTeam;
    syncTeamForms(updatedTeam);
    uiStore.info("成员已移除", `${username} 已移出队伍。`);
    await loadTeams();
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : "移除成员失败";
    pageError.value = message;
    uiStore.error("移除成员失败", message);
  } finally {
    memberActionUserId.value = "";
  }
}

onMounted(async () => {
  await refreshAll();
});
</script>
