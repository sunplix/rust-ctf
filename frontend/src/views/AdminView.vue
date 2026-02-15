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

    <div class="actions-row module-tabs">
      <button class="ghost" type="button" :class="{ active: adminModule === 'challenges' }" @click="adminModule = 'challenges'">
        题目模块
      </button>
      <button class="ghost" type="button" :class="{ active: adminModule === 'contests' }" @click="adminModule = 'contests'">
        比赛模块
      </button>
      <button class="ghost" type="button" :class="{ active: adminModule === 'operations' }" @click="adminModule = 'operations'">
        运行监控
      </button>
      <button class="ghost" type="button" :class="{ active: adminModule === 'users' }" @click="adminModule = 'users'">
        用户管理
      </button>
      <button class="ghost" type="button" :class="{ active: adminModule === 'audit' }" @click="adminModule = 'audit'">
        审计日志
      </button>
    </div>

    <div v-if="adminModule === 'challenges'" class="actions-row module-tabs">
      <button
        class="ghost"
        type="button"
        :class="{ active: challengeSubTab === 'library' }"
        @click="challengeSubTab = 'library'"
      >
        题库配置
      </button>
      <button
        class="ghost"
        type="button"
        :class="{ active: challengeSubTab === 'versions' }"
        @click="challengeSubTab = 'versions'"
      >
        版本与附件
      </button>
    </div>

    <div v-if="adminModule === 'contests'" class="actions-row module-tabs">
      <button
        class="ghost"
        type="button"
        :class="{ active: contestSubTab === 'contests' }"
        @click="contestSubTab = 'contests'"
      >
        赛事配置
      </button>
      <button
        class="ghost"
        type="button"
        :class="{ active: contestSubTab === 'bindings' }"
        @click="contestSubTab = 'bindings'"
      >
        题目挂载
      </button>
      <button
        class="ghost"
        type="button"
        :class="{ active: contestSubTab === 'announcements' }"
        @click="contestSubTab = 'announcements'"
      >
        公告管理
      </button>
    </div>

    <div v-if="adminModule === 'operations'" class="actions-row module-tabs">
      <button
        class="ghost"
        type="button"
        :class="{ active: operationsSubTab === 'runtime' }"
        @click="operationsSubTab = 'runtime'"
      >
        运行概览
      </button>
      <button
        class="ghost"
        type="button"
        :class="{ active: operationsSubTab === 'instances' }"
        @click="operationsSubTab = 'instances'"
      >
        实例监控
      </button>
    </div>

    <div v-if="adminModule === 'challenges' || adminModule === 'contests'" class="admin-grid">
      <section v-if="adminModule === 'challenges'" class="panel">
        <div class="row-between">
          <h2>题目管理</h2>
          <span class="badge">{{ challenges.length }} 条</span>
        </div>

        <template v-if="challengeSubTab === 'library'">
          <div class="module-split challenge-split">
            <div class="module-column module-column-fill">
              <h3>创建题目</h3>
              <form class="form-grid compact-grid" @submit.prevent="handleCreateChallenge">
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
                  <span>难度</span>
                  <select v-model="newChallenge.difficulty">
                    <option value="easy">easy</option>
                    <option value="normal">normal</option>
                    <option value="hard">hard</option>
                    <option value="insane">insane</option>
                  </select>
                </label>
                <label>
                  <span>分值</span>
                  <input v-model.number="newChallenge.static_score" type="number" min="1" />
                </label>
                <label>
                  <span>状态</span>
                  <select v-model="newChallenge.status">
                    <option value="draft">draft</option>
                    <option value="published">published</option>
                    <option value="offline">offline</option>
                  </select>
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
                <label>
                  <span>标签（逗号分隔）</span>
                  <input v-model="newChallenge.tags_input" placeholder="web, sqli, internal" />
                </label>
                <label>
                  <span>题解可见策略</span>
                  <select v-model="newChallenge.writeup_visibility">
                    <option value="hidden">hidden</option>
                    <option value="after_solve">after_solve</option>
                    <option value="after_contest">after_contest</option>
                    <option value="public">public</option>
                  </select>
                </label>
                <label>
                  <span>描述（可选）</span>
                  <textarea v-model="newChallenge.description" rows="3" />
                </label>
                <label>
                  <span>题解内容（可选）</span>
                  <textarea v-model="newChallenge.writeup_content" rows="4" />
                </label>
                <label>
                  <span>版本备注（可选）</span>
                  <input v-model="newChallenge.change_note" />
                </label>

                <button class="primary" type="submit" :disabled="creatingChallenge">
                  {{ creatingChallenge ? "创建中..." : "创建题目" }}
                </button>
              </form>

              <p v-if="challengeError" class="error">{{ challengeError }}</p>
            </div>

            <div class="module-column">
              <div class="row-between">
                <h3>题库概览</h3>
                <span class="badge">{{ filteredChallenges.length }} / {{ challenges.length }}</span>
              </div>
              <label class="search-field">
                <span>快速筛选</span>
                <input v-model.trim="challengeKeyword" placeholder="按标题、slug、分类筛选" />
              </label>

              <div class="challenge-card-grid stagger-list">
                <article
                  v-for="item in filteredChallenges"
                  :key="item.id"
                  class="admin-list-item challenge-card"
                  :class="{ active: selectedChallengeId === item.id }"
                >
                  <div class="row-between">
                    <strong>{{ item.title }}</strong>
                    <span class="badge">{{ item.challenge_type }}</span>
                  </div>
                  <p class="muted mono">{{ item.slug }}</p>
                  <p class="muted">{{ item.category }} · {{ item.difficulty }} · {{ item.flag_mode }}</p>
                  <p class="muted">score={{ item.static_score }} · status={{ item.status }} · v{{ item.current_version }}</p>
                  <div class="actions-row compact-actions">
                    <button
                      class="ghost"
                      type="button"
                      @click="selectChallenge(item.id)"
                      :disabled="selectedChallengeId === item.id"
                    >
                      {{ selectedChallengeId === item.id ? "当前管理中" : "版本/附件" }}
                    </button>
                    <button
                      class="ghost"
                      type="button"
                      @click="updateChallengeStatus(item.id, 'published')"
                      :disabled="updatingChallengeId === item.id || item.status === 'published'"
                    >
                      发布
                    </button>
                    <button
                      class="ghost"
                      type="button"
                      @click="updateChallengeStatus(item.id, 'draft')"
                      :disabled="updatingChallengeId === item.id || item.status === 'draft'"
                    >
                      草稿
                    </button>
                    <button
                      class="ghost"
                      type="button"
                      @click="updateChallengeStatus(item.id, 'offline')"
                      :disabled="updatingChallengeId === item.id || item.status === 'offline'"
                    >
                      下线
                    </button>
                  </div>
                </article>
              </div>
              <p v-if="filteredChallenges.length === 0" class="muted">没有匹配的题目。</p>
            </div>
          </div>
        </template>

        <template v-if="challengeSubTab === 'versions' && selectedChallenge">
          <h3>版本与附件：{{ selectedChallenge.title }}</h3>
          <p v-if="challengeVersionError" class="error">{{ challengeVersionError }}</p>
          <p v-if="challengeAttachmentError" class="error">{{ challengeAttachmentError }}</p>

          <form class="form-grid" @submit.prevent="handleRollbackChallengeVersion">
            <label>
              <span>回滚版本号</span>
              <input v-model.number="rollbackForm.version_no" type="number" min="1" required />
            </label>
            <label>
              <span>回滚备注（可选）</span>
              <input v-model="rollbackForm.change_note" />
            </label>
            <button class="ghost" type="submit" :disabled="rollingBack">
              {{ rollingBack ? "回滚中..." : "执行回滚" }}
            </button>
          </form>

          <div class="admin-list stagger-list">
            <article v-for="version in challengeVersions" :key="version.id" class="admin-list-item">
              <div class="row-between">
                <strong>v{{ version.version_no }}</strong>
                <span class="muted">{{ formatTime(version.created_at) }}</span>
              </div>
              <p class="muted mono">
                by {{ version.created_by_username ?? version.created_by ?? "system" }} · {{ version.change_note }}
              </p>
              <div class="actions-row">
                <button
                  class="ghost"
                  type="button"
                  @click="rollbackToVersion(version.version_no)"
                  :disabled="rollingBack"
                >
                  回滚到该版本
                </button>
              </div>
            </article>
            <p v-if="challengeVersions.length === 0" class="muted">暂无版本记录。</p>
          </div>

          <form class="form-grid" @submit.prevent="handleUploadChallengeAttachment">
            <label>
              <span>上传附件</span>
              <input :key="attachmentInputKey" type="file" @change="onAttachmentFileChange" required />
            </label>
            <button class="ghost" type="submit" :disabled="uploadingAttachment || !selectedAttachmentFile">
              {{ uploadingAttachment ? "上传中..." : "上传附件" }}
            </button>
          </form>

          <div class="admin-list stagger-list">
            <article v-for="attachment in challengeAttachments" :key="attachment.id" class="admin-list-item">
              <div class="row-between">
                <strong>{{ attachment.filename }}</strong>
                <span class="muted">{{ formatTime(attachment.created_at) }}</span>
              </div>
              <p class="muted mono">
                {{ attachment.content_type }} · {{ formatSize(attachment.size_bytes) }} ·
                by {{ attachment.uploaded_by_username ?? attachment.uploaded_by ?? "system" }}
              </p>
              <div class="actions-row">
                <button
                  class="danger"
                  type="button"
                  @click="deleteChallengeAttachment(attachment.id)"
                  :disabled="deletingAttachmentId === attachment.id"
                >
                  {{ deletingAttachmentId === attachment.id ? "删除中..." : "删除附件" }}
                </button>
              </div>
            </article>
            <p v-if="challengeAttachments.length === 0" class="muted">暂无附件。</p>
          </div>
        </template>
        <p v-if="challengeSubTab === 'versions' && !selectedChallenge" class="muted">
          请先在“题库配置”中选择一个题目，再切换到版本管理。
        </p>
      </section>

      <section v-if="adminModule === 'contests' && contestSubTab === 'contests'" class="panel">
        <div class="row-between">
          <h2>比赛管理</h2>
          <span class="badge">{{ contests.length }} 场</span>
        </div>

        <div class="module-split contest-split">
          <div class="module-column module-column-fill">
            <h3>创建比赛</h3>
            <form class="form-grid compact-grid" @submit.prevent="handleCreateContest">
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
                <span>积分模式</span>
                <select v-model="newContest.scoring_mode">
                  <option value="static">static</option>
                  <option value="dynamic">dynamic</option>
                </select>
              </label>
              <label>
                <span>动态衰减参数</span>
                <input v-model.number="newContest.dynamic_decay" type="number" min="1" max="100000" />
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
          </div>

          <div class="module-column">
            <div class="row-between">
              <h3>赛事列表</h3>
              <span class="badge">{{ filteredContests.length }} / {{ contests.length }}</span>
            </div>
            <label class="search-field">
              <span>快速筛选</span>
              <input v-model.trim="contestKeyword" placeholder="按标题、slug、状态筛选" />
            </label>

            <div class="contest-browser">
              <aside class="contest-list-pane">
                <button
                  v-for="contest in filteredContests"
                  :key="contest.id"
                  class="contest-list-item"
                  :class="{ active: selectedContestId === contest.id }"
                  type="button"
                  @click="selectContest(contest.id)"
                >
                  <strong>{{ contest.title }}</strong>
                  <span class="muted mono">{{ contest.slug }}</span>
                  <span class="muted">{{ contest.status }} · {{ contest.visibility }}</span>
                </button>
                <p v-if="filteredContests.length === 0" class="muted">没有匹配的比赛。</p>
              </aside>

              <section v-if="selectedContest" class="contest-detail-pane">
                <div class="row-between">
                  <h4>{{ selectedContest.title }}</h4>
                  <span class="badge">{{ selectedContest.status }}</span>
                </div>
                <p class="muted mono">{{ selectedContest.slug }} · {{ selectedContest.visibility }}</p>
                <p class="muted">
                  scoring={{ selectedContest.scoring_mode }} · dynamic_decay={{ selectedContest.dynamic_decay }}
                </p>
                <p class="muted">
                  {{ formatTime(selectedContest.start_at) }} ~ {{ formatTime(selectedContest.end_at) }}
                </p>
                <div class="actions-row compact-actions">
                  <button
                    v-for="status in statusActions"
                    :key="status"
                    class="ghost"
                    type="button"
                    :disabled="updatingContestId === selectedContest.id || selectedContest.status === status"
                    @click="updateContestStatus(selectedContest.id, status)"
                  >
                    {{ status }}
                  </button>
                </div>
                <div class="actions-row compact-actions">
                  <button class="ghost" type="button" @click="contestSubTab = 'bindings'">
                    管理题目挂载
                  </button>
                  <button class="ghost" type="button" @click="contestSubTab = 'announcements'">
                    管理公告
                  </button>
                </div>
              </section>

              <section v-else class="contest-detail-pane">
                <p class="muted">从左侧选择一个比赛查看详情与状态操作。</p>
              </section>
            </div>
          </div>
        </div>
      </section>

      <section v-if="adminModule === 'contests' && contestSubTab === 'bindings'" class="panel">
        <div class="row-between">
          <h2>比赛题目挂载</h2>
          <span class="badge" v-if="selectedContest">{{ selectedContest.title }}</span>
        </div>

        <p class="muted" v-if="!selectedContest">请先在中间列选择一个比赛。</p>

        <template v-else>
          <p v-if="bindingError" class="error">{{ bindingError }}</p>

          <div class="contest-browser">
            <aside class="contest-list-pane">
              <h3>挂载/更新题目</h3>
              <form class="form-grid compact-grid" @submit.prevent="handleUpsertBinding">
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

              <h3>已挂载题目</h3>
              <button
                v-for="item in contestBindings"
                :key="item.challenge_id"
                class="contest-list-item"
                :class="{ active: selectedBindingChallengeId === item.challenge_id }"
                type="button"
                @click="selectBinding(item.challenge_id)"
              >
                <strong>{{ item.challenge_title }}</strong>
                <span class="muted mono">sort={{ item.sort_order }}</span>
                <span class="muted">{{ item.challenge_category }} · {{ item.challenge_difficulty }}</span>
              </button>
              <p v-if="contestBindings.length === 0" class="muted">当前比赛未挂载题目。</p>
            </aside>

            <section class="contest-detail-pane">
              <template v-if="selectedBinding">
                <div class="row-between">
                  <h4>{{ selectedBinding.challenge_title }}</h4>
                  <span class="badge">sort {{ selectedBinding.sort_order }}</span>
                </div>
                <p class="muted mono">
                  {{ selectedBinding.challenge_category }} · {{ selectedBinding.challenge_difficulty }}
                </p>
                <p class="muted">
                  release_at={{ selectedBinding.release_at ? formatTime(selectedBinding.release_at) : "-" }}
                </p>
                <div class="actions-row compact-actions">
                  <button
                    class="ghost"
                    type="button"
                    @click="loadBindingIntoForm(selectedBinding)"
                    :disabled="bindingBusy"
                  >
                    加载到左侧表单
                  </button>
                  <button
                    class="ghost"
                    type="button"
                    @click="quickAdjustSort(selectedBinding.challenge_id, selectedBinding.sort_order - 1)"
                    :disabled="bindingBusy"
                  >
                    上移
                  </button>
                  <button
                    class="ghost"
                    type="button"
                    @click="quickAdjustSort(selectedBinding.challenge_id, selectedBinding.sort_order + 1)"
                    :disabled="bindingBusy"
                  >
                    下移
                  </button>
                  <button
                    class="ghost"
                    type="button"
                    @click="clearBindingReleaseAt(selectedBinding.challenge_id)"
                    :disabled="bindingBusy"
                  >
                    清除发布时间
                  </button>
                  <button
                    class="danger"
                    type="button"
                    @click="removeBinding(selectedBinding.challenge_id)"
                    :disabled="bindingBusy"
                  >
                    移除挂载
                  </button>
                </div>
              </template>
              <p v-else class="muted">从左侧选择一个挂载题目查看详情。</p>
            </section>
          </div>
        </template>
      </section>

      <section v-if="adminModule === 'contests' && contestSubTab === 'announcements'" class="panel">
        <div class="row-between">
          <h2>比赛公告管理</h2>
          <span class="badge" v-if="selectedContest">{{ selectedContest.title }}</span>
        </div>

        <p class="muted" v-if="!selectedContest">请先在“赛事配置”中选择一个比赛。</p>

        <template v-else>
          <p v-if="announcementError" class="error">{{ announcementError }}</p>

          <div class="contest-browser">
            <aside class="contest-list-pane">
              <h3>创建公告</h3>
              <form class="form-grid compact-grid" @submit.prevent="handleCreateAnnouncement">
                <label>
                  <span>公告标题</span>
                  <input v-model.trim="announcementForm.title" required />
                </label>
                <label>
                  <span>公告内容</span>
                  <textarea v-model.trim="announcementForm.content" rows="4" required />
                </label>
                <label class="inline-check">
                  <input v-model="announcementForm.is_published" type="checkbox" />
                  <span>立即发布</span>
                </label>
                <label class="inline-check">
                  <input v-model="announcementForm.is_pinned" type="checkbox" />
                  <span>置顶公告</span>
                </label>
                <button class="primary" type="submit" :disabled="creatingAnnouncement">
                  {{ creatingAnnouncement ? "创建中..." : "创建公告" }}
                </button>
              </form>

              <h3>公告列表</h3>
              <button
                v-for="item in contestAnnouncements"
                :key="item.id"
                class="contest-list-item"
                :class="{ active: selectedAnnouncementId === item.id }"
                type="button"
                @click="selectAnnouncement(item.id)"
              >
                <strong>{{ item.title }}</strong>
                <span class="muted mono">{{ item.is_published ? "published" : "draft" }}</span>
                <span class="muted">
                  {{ item.published_at ? formatTime(item.published_at) : "未发布" }}
                </span>
              </button>
              <p v-if="contestAnnouncements.length === 0" class="muted">暂无公告。</p>
            </aside>

            <section class="contest-detail-pane">
              <template v-if="selectedAnnouncement">
                <div class="row-between">
                  <h4>{{ selectedAnnouncement.title }}</h4>
                  <span class="badge" v-if="selectedAnnouncement.is_pinned">置顶</span>
                </div>
                <p class="muted mono">
                  {{ selectedAnnouncement.is_published ? "published" : "draft" }} ·
                  {{ selectedAnnouncement.published_at ? formatTime(selectedAnnouncement.published_at) : "未发布" }}
                </p>
                <form
                  v-if="announcementDrafts[selectedAnnouncement.id]"
                  class="form-grid"
                  @submit.prevent="saveAnnouncementEdit(selectedAnnouncement)"
                >
                  <label>
                    <span>标题</span>
                    <input
                      v-model.trim="announcementDrafts[selectedAnnouncement.id].title"
                      required
                      :disabled="savingAnnouncementId === selectedAnnouncement.id"
                    />
                  </label>
                  <label>
                    <span>内容</span>
                    <textarea
                      v-model.trim="announcementDrafts[selectedAnnouncement.id].content"
                      rows="6"
                      required
                      :disabled="savingAnnouncementId === selectedAnnouncement.id"
                    />
                  </label>
                  <button class="ghost" type="submit" :disabled="savingAnnouncementId === selectedAnnouncement.id">
                    {{ savingAnnouncementId === selectedAnnouncement.id ? "保存中..." : "保存修改" }}
                  </button>
                </form>
                <p v-else class="muted">正在准备编辑器...</p>

                <div class="actions-row compact-actions">
                  <button
                    class="ghost"
                    type="button"
                    @click="toggleAnnouncementPublish(selectedAnnouncement)"
                    :disabled="
                      updatingAnnouncementId === selectedAnnouncement.id ||
                      deletingAnnouncementId === selectedAnnouncement.id ||
                      savingAnnouncementId === selectedAnnouncement.id
                    "
                  >
                    {{ selectedAnnouncement.is_published ? "撤回发布" : "发布" }}
                  </button>
                  <button
                    class="ghost"
                    type="button"
                    @click="toggleAnnouncementPin(selectedAnnouncement)"
                    :disabled="
                      updatingAnnouncementId === selectedAnnouncement.id ||
                      deletingAnnouncementId === selectedAnnouncement.id ||
                      savingAnnouncementId === selectedAnnouncement.id
                    "
                  >
                    {{ selectedAnnouncement.is_pinned ? "取消置顶" : "置顶" }}
                  </button>
                  <button
                    class="danger"
                    type="button"
                    @click="removeAnnouncement(selectedAnnouncement)"
                    :disabled="
                      deletingAnnouncementId === selectedAnnouncement.id ||
                      updatingAnnouncementId === selectedAnnouncement.id ||
                      savingAnnouncementId === selectedAnnouncement.id
                    "
                  >
                    {{ deletingAnnouncementId === selectedAnnouncement.id ? "删除中..." : "删除公告" }}
                  </button>
                </div>
              </template>
              <p v-else class="muted">从左侧选择一个公告查看详情。</p>
            </section>
          </div>
        </template>
      </section>
    </div>

    <section v-if="adminModule === 'operations' && operationsSubTab === 'runtime'" class="panel">
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

    <section v-if="adminModule === 'users'" class="panel">
      <div class="row-between">
        <h2>用户管理</h2>
        <button class="ghost" type="button" @click="loadUsers" :disabled="loadingUsers">
          {{ loadingUsers ? "加载中..." : "刷新用户" }}
        </button>
      </div>

      <div class="actions-row">
        <label>
          <span>关键词</span>
          <input v-model.trim="userKeyword" placeholder="用户名或邮箱" />
        </label>
        <label>
          <span>角色</span>
          <select v-model="userRoleFilter">
            <option value="">all</option>
            <option value="player">player</option>
            <option value="admin">admin</option>
            <option value="judge">judge</option>
          </select>
        </label>
        <label>
          <span>状态</span>
          <select v-model="userStatusFilter">
            <option value="">all</option>
            <option value="active">active</option>
            <option value="disabled">disabled</option>
          </select>
        </label>
        <label>
          <span>条数</span>
          <input v-model.number="userLimit" type="number" min="1" max="1000" />
        </label>
        <button class="ghost" type="button" @click="loadUsers" :disabled="loadingUsers">
          应用筛选
        </button>
      </div>

      <p v-if="userError" class="error">{{ userError }}</p>

      <table v-if="users.length > 0" class="scoreboard-table">
        <thead>
          <tr>
            <th>用户名</th>
            <th>邮箱</th>
            <th>角色</th>
            <th>状态</th>
            <th>创建时间</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="item in users" :key="item.id">
            <td>{{ item.username }}</td>
            <td class="mono">{{ item.email }}</td>
            <td>{{ item.role }}</td>
            <td>{{ item.status }}</td>
            <td>{{ formatTime(item.created_at) }}</td>
            <td>
              <div class="actions-row">
                <button
                  class="ghost"
                  type="button"
                  :disabled="
                    updatingUserId === item.id ||
                    resettingUserId === item.id ||
                    updatingUserRoleId === item.id
                  "
                  @click="toggleUserStatus(item)"
                >
                  {{ item.status === "active" ? "禁用" : "启用" }}
                </button>
                <select
                  v-model="roleDrafts[item.id]"
                  :disabled="
                    updatingUserId === item.id ||
                    resettingUserId === item.id ||
                    updatingUserRoleId === item.id
                  "
                >
                  <option value="player">player</option>
                  <option value="judge">judge</option>
                  <option value="admin">admin</option>
                </select>
                <button
                  class="ghost"
                  type="button"
                  :disabled="
                    updatingUserId === item.id ||
                    resettingUserId === item.id ||
                    updatingUserRoleId === item.id
                  "
                  @click="handleUpdateUserRole(item)"
                >
                  {{ updatingUserRoleId === item.id ? "更新中..." : "更新角色" }}
                </button>
                <input
                  v-model="resetPasswords[item.id]"
                  type="password"
                  minlength="8"
                  placeholder="新密码(>=8)"
                />
                <button
                  class="primary"
                  type="button"
                  :disabled="
                    updatingUserId === item.id ||
                    resettingUserId === item.id ||
                    updatingUserRoleId === item.id
                  "
                  @click="handleResetUserPassword(item)"
                >
                  {{ resettingUserId === item.id ? "重置中..." : "重置密码" }}
                </button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
      <p v-else class="muted">暂无用户记录。</p>
    </section>

    <section v-if="adminModule === 'operations' && operationsSubTab === 'instances'" class="panel">
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

    <section v-if="adminModule === 'audit'" class="panel">
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
import { computed, onMounted, onUnmounted, reactive, ref, watch } from "vue";

import {
  ApiClientError,
  createAdminChallenge,
  createAdminContestAnnouncement,
  createAdminContest,
  deleteAdminChallengeAttachment,
  deleteAdminContestAnnouncement,
  deleteAdminContestChallenge,
  getAdminRuntimeOverview,
  listAdminChallengeAttachments,
  listAdminChallengeVersions,
  listAdminContestAnnouncements,
  listAdminUsers,
  listAdminAuditLogs,
  listAdminChallenges,
  listAdminContestChallenges,
  listAdminContests,
  listAdminInstances,
  resetAdminUserPassword,
  rollbackAdminChallengeVersion,
  uploadAdminChallengeAttachment,
  updateAdminContestAnnouncement,
  updateAdminUserRole,
  updateAdminUserStatus,
  type AdminChallengeAttachmentItem,
  type AdminAuditLogItem,
  type AdminChallengeItem,
  type AdminChallengeVersionItem,
  type AdminContestAnnouncementItem,
  type AdminContestChallengeItem,
  type AdminContestItem,
  type AdminInstanceItem,
  type AdminRuntimeOverview,
  type AdminUserItem,
  updateAdminChallenge,
  updateAdminContestStatus,
  updateAdminContestChallenge,
  upsertAdminContestChallenge
} from "../api/client";
import { useAuthStore } from "../stores/auth";
import { useUiStore } from "../stores/ui";

const authStore = useAuthStore();
const uiStore = useUiStore();

const challenges = ref<AdminChallengeItem[]>([]);
const challengeVersions = ref<AdminChallengeVersionItem[]>([]);
const challengeAttachments = ref<AdminChallengeAttachmentItem[]>([]);
const contests = ref<AdminContestItem[]>([]);
const contestBindings = ref<AdminContestChallengeItem[]>([]);
const contestAnnouncements = ref<AdminContestAnnouncementItem[]>([]);
const instances = ref<AdminInstanceItem[]>([]);
const users = ref<AdminUserItem[]>([]);
const auditLogs = ref<AdminAuditLogItem[]>([]);
const runtimeOverview = ref<AdminRuntimeOverview | null>(null);

const selectedContestId = ref("");
const selectedChallengeId = ref("");
const selectedBindingChallengeId = ref("");
const selectedAnnouncementId = ref("");
const adminModule = ref<"challenges" | "contests" | "operations" | "users" | "audit">("challenges");
const challengeSubTab = ref<"library" | "versions">("library");
const contestSubTab = ref<"contests" | "bindings" | "announcements">("contests");
const operationsSubTab = ref<"runtime" | "instances">("runtime");

const pageError = ref("");
const challengeError = ref("");
const challengeVersionError = ref("");
const challengeAttachmentError = ref("");
const contestError = ref("");
const bindingError = ref("");
const announcementError = ref("");
const instanceError = ref("");
const userError = ref("");
const auditError = ref("");
const runtimeError = ref("");

const refreshing = ref(false);
const creatingChallenge = ref(false);
const creatingContest = ref(false);
const updatingChallengeId = ref("");
const rollingBack = ref(false);
const uploadingAttachment = ref(false);
const deletingAttachmentId = ref("");
const updatingContestId = ref("");
const bindingBusy = ref(false);
const creatingAnnouncement = ref(false);
const updatingAnnouncementId = ref("");
const deletingAnnouncementId = ref("");
const savingAnnouncementId = ref("");
const loadingUsers = ref(false);
const auditLoading = ref(false);
const updatingUserId = ref("");
const resettingUserId = ref("");
const updatingUserRoleId = ref("");

const instanceFilter = ref("");
const challengeKeyword = ref("");
const contestKeyword = ref("");
const userKeyword = ref("");
const userRoleFilter = ref("");
const userStatusFilter = ref("");
const userLimit = ref(150);
const auditActionFilter = ref("");
const auditTargetTypeFilter = ref("");
const auditLimit = ref(200);
const statusActions = ["draft", "scheduled", "running", "ended", "archived"];
const RUNTIME_POLL_INTERVAL_MS = 20_000;
const resetPasswords = reactive<Record<string, string>>({});
const roleDrafts = reactive<Record<string, string>>({});
const announcementDrafts = reactive<Record<string, { title: string; content: string }>>({});
const attachmentInputKey = ref(0);
const selectedAttachmentFile = ref<File | null>(null);

const runtimeAlertPrimed = ref(false);
const seenRuntimeFailureKeys = new Set<string>();
const lastExpiringWithin30mCount = ref(0);
const lastExpiredNotDestroyedCount = ref(0);
let runtimePollTimer: number | null = null;

const newChallenge = reactive({
  title: "",
  slug: "",
  category: "web",
  description: "",
  difficulty: "normal",
  static_score: 100,
  status: "draft",
  challenge_type: "static",
  flag_mode: "static",
  flag_hash: "",
  tags_input: "",
  writeup_visibility: "hidden",
  writeup_content: "",
  change_note: "",
  compose_template: "",
});

const rollbackForm = reactive({
  version_no: 1,
  change_note: ""
});

function localInputValue(input: Date) {
  const normalized = new Date(input.getTime() - input.getTimezoneOffset() * 60_000);
  return normalized.toISOString().slice(0, 16);
}

function localInputToIso(value: string) {
  return new Date(value).toISOString();
}

function isoToLocalInput(value: string) {
  return localInputValue(new Date(value));
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
  scoring_mode: "static",
  dynamic_decay: 20,
  start_at: defaultStart,
  end_at: defaultEnd,
  freeze_at: ""
});

const bindingForm = reactive({
  challenge_id: "",
  sort_order: 0,
  release_at: ""
});

const announcementForm = reactive({
  title: "",
  content: "",
  is_published: false,
  is_pinned: false
});

const selectedContest = computed(() => {
  return contests.value.find((item) => item.id === selectedContestId.value) ?? null;
});

const selectedChallenge = computed(() => {
  return challenges.value.find((item) => item.id === selectedChallengeId.value) ?? null;
});

const selectedBinding = computed(() => {
  return contestBindings.value.find((item) => item.challenge_id === selectedBindingChallengeId.value) ?? null;
});

const selectedAnnouncement = computed(() => {
  return contestAnnouncements.value.find((item) => item.id === selectedAnnouncementId.value) ?? null;
});

const filteredChallenges = computed(() => {
  const keyword = challengeKeyword.value.trim().toLowerCase();
  if (!keyword) {
    return challenges.value;
  }

  return challenges.value.filter((item) => {
    return (
      item.title.toLowerCase().includes(keyword) ||
      item.slug.toLowerCase().includes(keyword) ||
      item.category.toLowerCase().includes(keyword) ||
      item.flag_mode.toLowerCase().includes(keyword) ||
      item.challenge_type.toLowerCase().includes(keyword)
    );
  });
});

const filteredContests = computed(() => {
  const keyword = contestKeyword.value.trim().toLowerCase();
  if (!keyword) {
    return contests.value;
  }

  return contests.value.filter((item) => {
    return (
      item.title.toLowerCase().includes(keyword) ||
      item.slug.toLowerCase().includes(keyword) ||
      item.status.toLowerCase().includes(keyword) ||
      item.visibility.toLowerCase().includes(keyword) ||
      item.scoring_mode.toLowerCase().includes(keyword)
    );
  });
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

function parseTagsInput(raw: string): string[] {
  return raw
    .split(/[,，\n]/g)
    .map((item) => item.trim())
    .filter((item) => item.length > 0);
}

function formatSize(sizeBytes: number): string {
  if (sizeBytes < 1024) {
    return `${sizeBytes} B`;
  }
  if (sizeBytes < 1024 * 1024) {
    return `${(sizeBytes / 1024).toFixed(1)} KB`;
  }
  return `${(sizeBytes / (1024 * 1024)).toFixed(1)} MB`;
}

function onAttachmentFileChange(event: Event) {
  const target = event.target as HTMLInputElement | null;
  selectedAttachmentFile.value = target?.files?.[0] ?? null;
}

function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result;
      if (typeof result !== "string") {
        reject(new Error("failed to read file"));
        return;
      }

      const parts = result.split(",", 2);
      resolve(parts.length === 2 ? parts[1] : result);
    };
    reader.onerror = () => reject(reader.error ?? new Error("failed to read file"));
    reader.readAsDataURL(file);
  });
}

function accessTokenOrThrow() {
  if (!authStore.accessToken) {
    throw new ApiClientError("未登录或会话过期", "unauthorized");
  }
  return authStore.accessToken;
}

function shrinkFailureAlertCache(maxSize: number) {
  while (seenRuntimeFailureKeys.size > maxSize) {
    const first = seenRuntimeFailureKeys.values().next().value as string | undefined;
    if (!first) {
      break;
    }
    seenRuntimeFailureKeys.delete(first);
  }
}

function primeRuntimeAlerts(overview: AdminRuntimeOverview) {
  runtimeAlertPrimed.value = true;
  lastExpiringWithin30mCount.value = overview.instances_expiring_within_30m;
  lastExpiredNotDestroyedCount.value = overview.instances_expired_not_destroyed;

  for (const item of overview.recent_failed_instances) {
    seenRuntimeFailureKeys.add(`${item.id}:${item.updated_at}`);
  }
  shrinkFailureAlertCache(2000);
}

function emitRuntimeAlerts(overview: AdminRuntimeOverview) {
  for (const item of overview.recent_failed_instances) {
    const key = `${item.id}:${item.updated_at}`;
    if (seenRuntimeFailureKeys.has(key)) {
      continue;
    }

    seenRuntimeFailureKeys.add(key);
    uiStore.error(
      "运行告警：实例失败",
      `${item.contest_title} / ${item.team_name} / ${item.challenge_title}（${item.status}）`,
      6500
    );
  }
  shrinkFailureAlertCache(2000);

  if (overview.instances_expiring_within_30m > lastExpiringWithin30mCount.value) {
    const increased = overview.instances_expiring_within_30m - lastExpiringWithin30mCount.value;
    uiStore.warning(
      "运行告警：实例即将到期",
      `当前 ${overview.instances_expiring_within_30m} 个实例将在 30 分钟内到期（新增 ${increased} 个）。`,
      5000
    );
  }

  if (overview.instances_expired_not_destroyed > lastExpiredNotDestroyedCount.value) {
    const increased =
      overview.instances_expired_not_destroyed - lastExpiredNotDestroyedCount.value;
    uiStore.warning(
      "运行告警：过期实例未销毁",
      `当前 ${overview.instances_expired_not_destroyed} 个已过期实例未销毁（新增 ${increased} 个）。`,
      5000
    );
  }

  lastExpiringWithin30mCount.value = overview.instances_expiring_within_30m;
  lastExpiredNotDestroyedCount.value = overview.instances_expired_not_destroyed;
}

async function loadChallenges() {
  challengeError.value = "";
  try {
    challenges.value = await listAdminChallenges(accessTokenOrThrow());
    if (selectedChallengeId.value) {
      const exists = challenges.value.some((item) => item.id === selectedChallengeId.value);
      if (!exists) {
        selectedChallengeId.value = "";
        challengeVersions.value = [];
        challengeAttachments.value = [];
      }
    }
  } catch (err) {
    challengeError.value = err instanceof ApiClientError ? err.message : "加载题目失败";
    uiStore.error("加载题目失败", challengeError.value);
  }
}

async function loadChallengeVersions() {
  challengeVersionError.value = "";
  if (!selectedChallengeId.value) {
    challengeVersions.value = [];
    return;
  }

  try {
    challengeVersions.value = await listAdminChallengeVersions(
      selectedChallengeId.value,
      accessTokenOrThrow(),
      { limit: 50 }
    );
  } catch (err) {
    challengeVersionError.value = err instanceof ApiClientError ? err.message : "加载题目版本失败";
    uiStore.error("加载题目版本失败", challengeVersionError.value);
  }
}

async function loadChallengeAttachments() {
  challengeAttachmentError.value = "";
  if (!selectedChallengeId.value) {
    challengeAttachments.value = [];
    return;
  }

  try {
    challengeAttachments.value = await listAdminChallengeAttachments(
      selectedChallengeId.value,
      accessTokenOrThrow(),
      { limit: 200 }
    );
  } catch (err) {
    challengeAttachmentError.value =
      err instanceof ApiClientError ? err.message : "加载题目附件失败";
    uiStore.error("加载题目附件失败", challengeAttachmentError.value);
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
    uiStore.error("加载比赛失败", contestError.value);
  }
}

async function loadContestBindings() {
  bindingError.value = "";

  if (!selectedContestId.value) {
    contestBindings.value = [];
    selectedBindingChallengeId.value = "";
    return;
  }

  try {
    const rows = await listAdminContestChallenges(
      selectedContestId.value,
      accessTokenOrThrow()
    );
    contestBindings.value = rows;

    if (rows.length === 0) {
      selectedBindingChallengeId.value = "";
      return;
    }

    if (!rows.some((item) => item.challenge_id === selectedBindingChallengeId.value)) {
      selectedBindingChallengeId.value = rows[0].challenge_id;
    }
  } catch (err) {
    bindingError.value = err instanceof ApiClientError ? err.message : "加载挂载失败";
    uiStore.error("加载挂载失败", bindingError.value);
  }
}

async function loadContestAnnouncements() {
  announcementError.value = "";

  if (!selectedContestId.value) {
    contestAnnouncements.value = [];
    selectedAnnouncementId.value = "";
    return;
  }

  try {
    const rows = await listAdminContestAnnouncements(
      selectedContestId.value,
      accessTokenOrThrow(),
      { limit: 200 }
    );
    contestAnnouncements.value = rows;

    if (rows.length === 0) {
      selectedAnnouncementId.value = "";
    } else if (!rows.some((item) => item.id === selectedAnnouncementId.value)) {
      selectedAnnouncementId.value = rows[0].id;
    }

    for (const item of rows) {
      announcementDrafts[item.id] = {
        title: item.title,
        content: item.content
      };
    }

    for (const id of Object.keys(announcementDrafts)) {
      if (!rows.some((item) => item.id === id)) {
        delete announcementDrafts[id];
      }
    }
  } catch (err) {
    announcementError.value = err instanceof ApiClientError ? err.message : "加载公告失败";
    uiStore.error("加载公告失败", announcementError.value);
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
    uiStore.error("加载实例失败", instanceError.value);
  }
}

async function loadUsers() {
  loadingUsers.value = true;
  userError.value = "";

  try {
    users.value = await listAdminUsers(accessTokenOrThrow(), {
      keyword: userKeyword.value || undefined,
      role: userRoleFilter.value || undefined,
      status: userStatusFilter.value || undefined,
      limit: Number.isFinite(userLimit.value) ? Math.max(1, Math.min(1000, userLimit.value)) : 150
    });

    for (const item of users.value) {
      roleDrafts[item.id] = item.role;
    }
  } catch (err) {
    userError.value = err instanceof ApiClientError ? err.message : "加载用户列表失败";
    uiStore.error("加载用户列表失败", userError.value);
  } finally {
    loadingUsers.value = false;
  }
}

async function loadRuntimeOverview(options?: { silentError?: boolean }) {
  runtimeError.value = "";
  try {
    const overview = await getAdminRuntimeOverview(accessTokenOrThrow());

    if (!runtimeAlertPrimed.value) {
      primeRuntimeAlerts(overview);
    } else {
      emitRuntimeAlerts(overview);
    }

    runtimeOverview.value = overview;
  } catch (err) {
    runtimeError.value = err instanceof ApiClientError ? err.message : "加载运行概览失败";
    if (!options?.silentError) {
      uiStore.error("加载运行概览失败", runtimeError.value);
    }
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
    uiStore.error("加载审计日志失败", auditError.value);
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
      loadUsers(),
      loadRuntimeOverview(),
      loadAuditLogs()
    ]);
    await Promise.all([loadContestBindings(), loadContestAnnouncements()]);
    if (selectedChallengeId.value) {
      await Promise.all([loadChallengeVersions(), loadChallengeAttachments()]);
    }
    uiStore.success("管理台已刷新", "最新题目、比赛、实例、审计和运行概览已同步。", 2400);
  } catch (err) {
    pageError.value = err instanceof ApiClientError ? err.message : "刷新失败";
    uiStore.error("刷新失败", pageError.value);
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
        description: newChallenge.description || undefined,
        difficulty: newChallenge.difficulty,
        static_score: newChallenge.static_score,
        status: newChallenge.status,
        challenge_type: newChallenge.challenge_type,
        flag_mode: newChallenge.flag_mode,
        flag_hash: newChallenge.flag_hash,
        compose_template: newChallenge.compose_template || undefined,
        tags: parseTagsInput(newChallenge.tags_input),
        writeup_visibility: newChallenge.writeup_visibility,
        writeup_content: newChallenge.writeup_content || undefined,
        change_note: newChallenge.change_note || undefined
      },
      accessTokenOrThrow()
    );

    newChallenge.title = "";
    newChallenge.slug = "";
    newChallenge.description = "";
    newChallenge.difficulty = "normal";
    newChallenge.status = "draft";
    newChallenge.tags_input = "";
    newChallenge.writeup_visibility = "hidden";
    newChallenge.writeup_content = "";
    newChallenge.change_note = "";
    newChallenge.flag_hash = "";
    newChallenge.compose_template = "";

    await loadChallenges();
    uiStore.success("题目已创建", "可以继续管理版本、附件或挂载到比赛。");
  } catch (err) {
    challengeError.value = err instanceof ApiClientError ? err.message : "创建题目失败";
    uiStore.error("创建题目失败", challengeError.value);
  } finally {
    creatingChallenge.value = false;
  }
}

async function selectChallenge(challengeId: string) {
  selectedChallengeId.value = challengeId;
  rollbackForm.version_no = 1;
  rollbackForm.change_note = "";
  selectedAttachmentFile.value = null;
  challengeVersionError.value = "";
  challengeAttachmentError.value = "";
  await Promise.all([loadChallengeVersions(), loadChallengeAttachments()]);
}

async function updateChallengeStatus(challengeId: string, status: "draft" | "published" | "offline") {
  updatingChallengeId.value = challengeId;
  challengeError.value = "";

  try {
    await updateAdminChallenge(challengeId, { status }, accessTokenOrThrow());
    await loadChallenges();
    if (selectedChallengeId.value === challengeId) {
      await loadChallengeVersions();
    }
    uiStore.info("题目状态已更新", `当前状态：${status}`);
  } catch (err) {
    challengeError.value = err instanceof ApiClientError ? err.message : "更新题目失败";
    uiStore.error("更新题目失败", challengeError.value);
  } finally {
    updatingChallengeId.value = "";
  }
}

async function handleRollbackChallengeVersion() {
  if (!selectedChallengeId.value) {
    challengeVersionError.value = "请先选择要管理的题目";
    uiStore.warning("未选择题目", challengeVersionError.value);
    return;
  }

  if (!Number.isFinite(rollbackForm.version_no) || rollbackForm.version_no < 1) {
    challengeVersionError.value = "版本号必须是大于等于 1 的整数";
    uiStore.warning("版本号非法", challengeVersionError.value);
    return;
  }

  rollingBack.value = true;
  challengeVersionError.value = "";

  try {
    await rollbackAdminChallengeVersion(
      selectedChallengeId.value,
      {
        version_no: Math.floor(rollbackForm.version_no),
        change_note: rollbackForm.change_note || undefined
      },
      accessTokenOrThrow()
    );
    await Promise.all([loadChallenges(), loadChallengeVersions()]);
    uiStore.success("题目已回滚", `已回滚到版本 v${Math.floor(rollbackForm.version_no)}。`);
  } catch (err) {
    challengeVersionError.value =
      err instanceof ApiClientError ? err.message : "回滚题目版本失败";
    uiStore.error("回滚题目版本失败", challengeVersionError.value);
  } finally {
    rollingBack.value = false;
  }
}

async function rollbackToVersion(versionNo: number) {
  rollbackForm.version_no = versionNo;
  await handleRollbackChallengeVersion();
}

async function handleUploadChallengeAttachment() {
  if (!selectedChallengeId.value) {
    challengeAttachmentError.value = "请先选择要管理的题目";
    uiStore.warning("未选择题目", challengeAttachmentError.value);
    return;
  }

  if (!selectedAttachmentFile.value) {
    challengeAttachmentError.value = "请先选择一个附件文件";
    uiStore.warning("未选择文件", challengeAttachmentError.value);
    return;
  }

  uploadingAttachment.value = true;
  challengeAttachmentError.value = "";

  try {
    const file = selectedAttachmentFile.value;
    const contentBase64 = await fileToBase64(file);
    await uploadAdminChallengeAttachment(
      selectedChallengeId.value,
      {
        filename: file.name,
        content_type: file.type || undefined,
        content_base64: contentBase64
      },
      accessTokenOrThrow()
    );
    selectedAttachmentFile.value = null;
    attachmentInputKey.value += 1;
    await loadChallengeAttachments();
    uiStore.success("附件已上传", file.name);
  } catch (err) {
    challengeAttachmentError.value =
      err instanceof ApiClientError ? err.message : "上传题目附件失败";
    uiStore.error("上传题目附件失败", challengeAttachmentError.value);
  } finally {
    uploadingAttachment.value = false;
  }
}

async function deleteChallengeAttachment(attachmentId: string) {
  if (!selectedChallengeId.value) {
    return;
  }

  deletingAttachmentId.value = attachmentId;
  challengeAttachmentError.value = "";

  try {
    await deleteAdminChallengeAttachment(
      selectedChallengeId.value,
      attachmentId,
      accessTokenOrThrow()
    );
    await loadChallengeAttachments();
    uiStore.warning("附件已删除", "已从当前题目移除附件。");
  } catch (err) {
    challengeAttachmentError.value =
      err instanceof ApiClientError ? err.message : "删除题目附件失败";
    uiStore.error("删除题目附件失败", challengeAttachmentError.value);
  } finally {
    deletingAttachmentId.value = "";
  }
}

async function handleCreateContest() {
  creatingContest.value = true;
  contestError.value = "";

  try {
    if (!Number.isFinite(newContest.dynamic_decay) || newContest.dynamic_decay < 1) {
      throw new ApiClientError("dynamic_decay 必须为大于等于 1 的整数", "bad_request");
    }

    const created = await createAdminContest(
      {
        title: newContest.title,
        slug: newContest.slug,
        description: newContest.description || undefined,
        visibility: newContest.visibility,
        status: newContest.status,
        scoring_mode: newContest.scoring_mode,
        dynamic_decay: Math.floor(newContest.dynamic_decay),
        start_at: localInputToIso(newContest.start_at),
        end_at: localInputToIso(newContest.end_at),
        freeze_at: newContest.freeze_at ? localInputToIso(newContest.freeze_at) : undefined
      },
      accessTokenOrThrow()
    );

    newContest.title = "";
    newContest.slug = "";
    newContest.description = "";
    newContest.scoring_mode = "static";
    newContest.dynamic_decay = 20;

    await loadContests();
    selectedContestId.value = created.id;
    await Promise.all([loadContestBindings(), loadContestAnnouncements()]);
    uiStore.success("比赛已创建", "可以继续挂载题目并调整状态。");
  } catch (err) {
    contestError.value = err instanceof ApiClientError ? err.message : "创建比赛失败";
    uiStore.error("创建比赛失败", contestError.value);
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
    uiStore.info("比赛状态已更新", `当前状态：${status}`);
  } catch (err) {
    contestError.value = err instanceof ApiClientError ? err.message : "更新比赛状态失败";
    uiStore.error("更新比赛状态失败", contestError.value);
  } finally {
    updatingContestId.value = "";
  }
}

async function toggleUserStatus(item: AdminUserItem) {
  updatingUserId.value = item.id;
  userError.value = "";

  const nextStatus = item.status === "active" ? "disabled" : "active";

  try {
    await updateAdminUserStatus(item.id, nextStatus, accessTokenOrThrow());
    await loadUsers();
    uiStore.info(
      "用户状态已更新",
      `${item.username} 已${nextStatus === "active" ? "启用" : "禁用"}。`
    );
  } catch (err) {
    userError.value = err instanceof ApiClientError ? err.message : "更新用户状态失败";
    uiStore.error("更新用户状态失败", userError.value);
  } finally {
    updatingUserId.value = "";
  }
}

async function handleResetUserPassword(item: AdminUserItem) {
  const nextPassword = (resetPasswords[item.id] ?? "").trim();
  if (nextPassword.length < 8) {
    userError.value = "新密码至少需要 8 位字符";
    uiStore.warning("密码过短", userError.value);
    return;
  }

  resettingUserId.value = item.id;
  userError.value = "";

  try {
    await resetAdminUserPassword(item.id, nextPassword, accessTokenOrThrow());
    resetPasswords[item.id] = "";
    await loadUsers();
    uiStore.success("密码已重置", `${item.username} 的密码已更新。`);
  } catch (err) {
    userError.value = err instanceof ApiClientError ? err.message : "重置密码失败";
    uiStore.error("重置密码失败", userError.value);
  } finally {
    resettingUserId.value = "";
  }
}

async function handleUpdateUserRole(item: AdminUserItem) {
  const nextRole = (roleDrafts[item.id] ?? "").trim().toLowerCase();
  if (!["player", "judge", "admin"].includes(nextRole)) {
    userError.value = "角色必须是 player / judge / admin";
    uiStore.warning("角色非法", userError.value);
    return;
  }

  if (nextRole === item.role) {
    uiStore.info("角色未变化", `${item.username} 当前角色仍为 ${item.role}。`);
    return;
  }

  updatingUserRoleId.value = item.id;
  userError.value = "";

  try {
    await updateAdminUserRole(item.id, nextRole, accessTokenOrThrow());
    await loadUsers();
    uiStore.success("角色已更新", `${item.username} 已设为 ${nextRole}。`);
  } catch (err) {
    userError.value = err instanceof ApiClientError ? err.message : "更新用户角色失败";
    uiStore.error("更新用户角色失败", userError.value);
  } finally {
    updatingUserRoleId.value = "";
  }
}

function selectContest(contestId: string) {
  selectedContestId.value = contestId;
}

function selectBinding(challengeId: string) {
  selectedBindingChallengeId.value = challengeId;
}

function selectAnnouncement(announcementId: string) {
  selectedAnnouncementId.value = announcementId;
}

function loadBindingIntoForm(item: AdminContestChallengeItem) {
  bindingForm.challenge_id = item.challenge_id;
  bindingForm.sort_order = item.sort_order;
  bindingForm.release_at = item.release_at ? isoToLocalInput(item.release_at) : "";
}

async function handleCreateAnnouncement() {
  if (!selectedContestId.value) {
    announcementError.value = "请先选择比赛";
    uiStore.warning("未选择比赛", announcementError.value);
    return;
  }

  creatingAnnouncement.value = true;
  announcementError.value = "";

  try {
    const created = await createAdminContestAnnouncement(
      selectedContestId.value,
      {
        title: announcementForm.title,
        content: announcementForm.content,
        is_published: announcementForm.is_published,
        is_pinned: announcementForm.is_pinned
      },
      accessTokenOrThrow()
    );
    announcementForm.title = "";
    announcementForm.content = "";
    announcementForm.is_published = false;
    announcementForm.is_pinned = false;
    selectedAnnouncementId.value = created.id;
    await loadContestAnnouncements();
    uiStore.success("公告已创建", "公告已保存。");
  } catch (err) {
    announcementError.value = err instanceof ApiClientError ? err.message : "创建公告失败";
    uiStore.error("创建公告失败", announcementError.value);
  } finally {
    creatingAnnouncement.value = false;
  }
}

async function toggleAnnouncementPublish(item: AdminContestAnnouncementItem) {
  if (!selectedContestId.value) {
    return;
  }

  updatingAnnouncementId.value = item.id;
  announcementError.value = "";

  try {
    await updateAdminContestAnnouncement(
      selectedContestId.value,
      item.id,
      { is_published: !item.is_published },
      accessTokenOrThrow()
    );
    await loadContestAnnouncements();
    uiStore.info(
      "公告状态已更新",
      !item.is_published ? "公告已发布" : "公告已撤回"
    );
  } catch (err) {
    announcementError.value = err instanceof ApiClientError ? err.message : "更新公告失败";
    uiStore.error("更新公告失败", announcementError.value);
  } finally {
    updatingAnnouncementId.value = "";
  }
}

async function saveAnnouncementEdit(item: AdminContestAnnouncementItem) {
  if (!selectedContestId.value) {
    return;
  }

  const draft = announcementDrafts[item.id];
  const title = draft?.title?.trim() ?? "";
  const content = draft?.content?.trim() ?? "";

  if (!title || !content) {
    announcementError.value = "公告标题和内容不能为空";
    uiStore.warning("公告内容不完整", announcementError.value);
    return;
  }

  savingAnnouncementId.value = item.id;
  announcementError.value = "";

  try {
    await updateAdminContestAnnouncement(
      selectedContestId.value,
      item.id,
      { title, content },
      accessTokenOrThrow()
    );
    await loadContestAnnouncements();
    uiStore.success("公告已更新", item.title);
  } catch (err) {
    announcementError.value = err instanceof ApiClientError ? err.message : "更新公告失败";
    uiStore.error("更新公告失败", announcementError.value);
  } finally {
    savingAnnouncementId.value = "";
  }
}

async function toggleAnnouncementPin(item: AdminContestAnnouncementItem) {
  if (!selectedContestId.value) {
    return;
  }

  updatingAnnouncementId.value = item.id;
  announcementError.value = "";

  try {
    await updateAdminContestAnnouncement(
      selectedContestId.value,
      item.id,
      { is_pinned: !item.is_pinned },
      accessTokenOrThrow()
    );
    await loadContestAnnouncements();
    uiStore.info(
      "公告置顶状态已更新",
      !item.is_pinned ? "公告已置顶" : "公告已取消置顶"
    );
  } catch (err) {
    announcementError.value = err instanceof ApiClientError ? err.message : "更新公告失败";
    uiStore.error("更新公告失败", announcementError.value);
  } finally {
    updatingAnnouncementId.value = "";
  }
}

async function removeAnnouncement(item: AdminContestAnnouncementItem) {
  if (!selectedContestId.value) {
    return;
  }

  deletingAnnouncementId.value = item.id;
  announcementError.value = "";

  try {
    await deleteAdminContestAnnouncement(
      selectedContestId.value,
      item.id,
      accessTokenOrThrow()
    );
    await loadContestAnnouncements();
    uiStore.warning("公告已删除", item.title);
  } catch (err) {
    announcementError.value = err instanceof ApiClientError ? err.message : "删除公告失败";
    uiStore.error("删除公告失败", announcementError.value);
  } finally {
    deletingAnnouncementId.value = "";
  }
}

async function handleUpsertBinding() {
  if (!selectedContestId.value) {
    bindingError.value = "请先选择比赛";
    uiStore.warning("未选择比赛", bindingError.value);
    return;
  }

  bindingBusy.value = true;
  bindingError.value = "";

  try {
    const targetChallengeId = bindingForm.challenge_id;
    await upsertAdminContestChallenge(
      selectedContestId.value,
      {
        challenge_id: bindingForm.challenge_id,
        sort_order: bindingForm.sort_order,
        release_at: bindingForm.release_at ? localInputToIso(bindingForm.release_at) : undefined
      },
      accessTokenOrThrow()
    );

    selectedBindingChallengeId.value = targetChallengeId;
    await loadContestBindings();
    uiStore.success("挂载成功", "题目已挂载/更新到当前比赛。");
  } catch (err) {
    bindingError.value = err instanceof ApiClientError ? err.message : "挂载失败";
    uiStore.error("挂载失败", bindingError.value);
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
    uiStore.info("排序已更新", `新排序值：${nextSort}`);
  } catch (err) {
    bindingError.value = err instanceof ApiClientError ? err.message : "更新排序失败";
    uiStore.error("更新排序失败", bindingError.value);
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
    uiStore.info("发布时间已清除", "该题将在比赛内即时可见。");
  } catch (err) {
    bindingError.value = err instanceof ApiClientError ? err.message : "清除发布时间失败";
    uiStore.error("清除发布时间失败", bindingError.value);
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
    uiStore.warning("挂载已移除", "题目已从当前比赛移除。");
  } catch (err) {
    bindingError.value = err instanceof ApiClientError ? err.message : "移除挂载失败";
    uiStore.error("移除挂载失败", bindingError.value);
  } finally {
    bindingBusy.value = false;
  }
}

watch(
  () => selectedContestId.value,
  () => {
    selectedBindingChallengeId.value = "";
    selectedAnnouncementId.value = "";
    bindingForm.challenge_id = "";
    bindingForm.sort_order = 0;
    bindingForm.release_at = "";
    loadContestBindings();
    loadContestAnnouncements();
  }
);

function startRuntimePolling() {
  stopRuntimePolling();
  runtimePollTimer = window.setInterval(() => {
    loadRuntimeOverview({ silentError: true });
  }, RUNTIME_POLL_INTERVAL_MS);
}

function stopRuntimePolling() {
  if (runtimePollTimer) {
    window.clearInterval(runtimePollTimer);
    runtimePollTimer = null;
  }
}

onMounted(() => {
  refreshAll();
  startRuntimePolling();
});

onUnmounted(() => {
  stopRuntimePolling();
});
</script>

<style scoped>
.admin-grid {
  grid-template-columns: minmax(0, 1fr);
}

.module-tabs {
  margin-top: 0.75rem;
}

.module-tabs .ghost.active {
  border-color: #0f766e;
  color: #0f766e;
  background: #ecfeff;
}

.module-split {
  display: grid;
  gap: 0.95rem;
  margin-top: 0.8rem;
  align-items: stretch;
}

.challenge-split {
  grid-template-columns: minmax(0, 1.1fr) minmax(0, 1.4fr);
}

.contest-split {
  grid-template-columns: minmax(0, 1fr) minmax(0, 1.4fr);
}

.module-column {
  border: 1px solid #d8e4f2;
  border-radius: 14px;
  background: #f8fbff;
  padding: 0.85rem;
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
  min-height: 0;
}

.module-column-fill {
  min-height: 0;
}

.module-column h3 {
  margin: 0;
}

.compact-grid {
  margin-top: 0.7rem;
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.compact-grid label:nth-last-of-type(1),
.compact-grid button {
  grid-column: 1 / -1;
}

.search-field {
  display: grid;
  gap: 0.35rem;
  margin-top: 0.65rem;
}

.challenge-card-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 0.65rem;
  flex: 1;
  min-height: 320px;
  overflow: auto;
  padding-right: 0.2rem;
  align-content: start;
}

.challenge-card.active {
  border-color: #0f766e;
  box-shadow: 0 0 0 2px rgba(15, 118, 110, 0.16);
}

.compact-actions {
  margin-bottom: 0;
}

.contest-browser {
  display: grid;
  grid-template-columns: 260px minmax(0, 1fr);
  gap: 0.8rem;
  flex: 1;
  min-height: 420px;
  min-width: 0;
}

.contest-list-pane {
  display: grid;
  gap: 0.5rem;
  min-height: 0;
  overflow: auto;
  padding-right: 0.2rem;
  align-content: start;
}

.contest-list-item {
  text-align: left;
  border: 1px solid #d1deec;
  border-radius: 10px;
  padding: 0.6rem 0.62rem;
  background: #ffffff;
  display: grid;
  gap: 0.2rem;
  cursor: pointer;
  transition: border-color 140ms ease, background-color 140ms ease;
}

.contest-list-item.active {
  border-color: #0f766e;
  background: #ecfeff;
}

.contest-detail-pane {
  border: 1px solid #d1deec;
  border-radius: 12px;
  background: #ffffff;
  padding: 0.8rem;
  min-height: 0;
  display: grid;
  align-content: start;
  gap: 0.6rem;
  overflow: auto;
}

.contest-detail-pane h4 {
  margin: 0;
}

@media (max-width: 1220px) {
  .challenge-split,
  .contest-split {
    grid-template-columns: 1fr;
  }

  .contest-browser {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 860px) {
  .compact-grid {
    grid-template-columns: 1fr;
  }
}
</style>
