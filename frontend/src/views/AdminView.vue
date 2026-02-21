<template>
  <section class="page-block">
    <div class="row-between">
      <div>
        <h1>{{ tr("管理控制台", "Admin Console") }}</h1>
        <p class="muted">{{ tr("题目管理、比赛控制、实例监控、用户管理与审计日志。", "Challenge management, contests, instance monitoring, users, and audit logs.") }}</p>
      </div>
      <div class="actions-row">
        <RouterLink v-if="authStore.user?.role === 'admin'" class="ghost" to="/admin/site-settings">
          {{ tr("站点设置", "Site Settings") }}
        </RouterLink>
        <button class="ghost" type="button" @click="refreshAll" :disabled="refreshing">
          {{ refreshing ? tr("刷新中...", "Refreshing...") : tr("刷新全部", "Refresh all") }}
        </button>
      </div>
    </div>

    <p v-if="pageError" class="error">{{ pageError }}</p>

    <div class="admin-layout">
      <aside class="panel admin-side-nav">
        <div class="admin-side-group">
          <p class="nav-group-label">{{ tr("核心模块", "Core Modules") }}</p>
          <button
            class="side-nav-btn"
            type="button"
            :class="{ active: adminModule === 'challenges' }"
            @click="adminModule = 'challenges'"
          >
            {{ tr("题目模块", "Challenges") }}
          </button>
          <button
            class="side-nav-btn"
            type="button"
            :class="{ active: adminModule === 'contests' }"
            @click="adminModule = 'contests'"
          >
            {{ tr("比赛模块", "Contests") }}
          </button>
          <button
            class="side-nav-btn"
            type="button"
            :class="{ active: adminModule === 'operations' }"
            @click="adminModule = 'operations'"
          >
            {{ tr("运行监控", "Operations") }}
          </button>
          <button
            class="side-nav-btn"
            type="button"
            :class="{ active: adminModule === 'users' }"
            @click="adminModule = 'users'"
          >
            {{ tr("用户管理", "Users") }}
          </button>
          <button
            class="side-nav-btn"
            type="button"
            :class="{ active: adminModule === 'audit' }"
            @click="adminModule = 'audit'"
          >
            {{ tr("审计日志", "Audit Logs") }}
          </button>
        </div>

        <div v-if="adminModule === 'challenges'" class="admin-side-group">
          <p class="nav-group-label">{{ tr("题目子导航", "Challenge Tabs") }}</p>
          <button
            class="side-nav-btn side-sub-btn"
            type="button"
            :class="{ active: challengeSubTab === 'library' }"
            @click="challengeSubTab = 'library'"
          >
            {{ tr("题库配置", "Library") }}
          </button>
          <button
            class="side-nav-btn side-sub-btn"
            type="button"
            :class="{ active: challengeSubTab === 'versions' }"
            @click="challengeSubTab = 'versions'"
          >
            {{ tr("版本与附件", "Versions & Files") }}
          </button>
          <button
            class="side-nav-btn side-sub-btn"
            type="button"
            :class="{ active: challengeSubTab === 'lint' }"
            @click="challengeSubTab = 'lint'"
          >
            {{ tr("模板校验", "Template Lint") }}
          </button>
        </div>

        <div v-if="adminModule === 'contests'" class="admin-side-group">
          <p class="nav-group-label">{{ tr("比赛子导航", "Contest Tabs") }}</p>
          <button
            class="side-nav-btn side-sub-btn"
            type="button"
            :class="{ active: contestSubTab === 'contests' }"
            @click="contestSubTab = 'contests'"
          >
            {{ tr("赛事配置", "Config") }}
          </button>
          <button
            class="side-nav-btn side-sub-btn"
            type="button"
            :class="{ active: contestSubTab === 'bindings' }"
            @click="contestSubTab = 'bindings'"
          >
            {{ tr("题目挂载", "Bindings") }}
          </button>
          <button
            class="side-nav-btn side-sub-btn"
            type="button"
            :class="{ active: contestSubTab === 'announcements' }"
            @click="contestSubTab = 'announcements'"
          >
            {{ tr("公告管理", "Announcements") }}
          </button>
          <button
            class="side-nav-btn side-sub-btn"
            type="button"
            :class="{ active: contestSubTab === 'registrations' }"
            @click="contestSubTab = 'registrations'"
          >
            {{ tr("报名审核", "Registrations") }}
          </button>
        </div>

        <div v-if="adminModule === 'operations'" class="admin-side-group">
          <p class="nav-group-label">{{ tr("监控子导航", "Operations Tabs") }}</p>
          <button
            class="side-nav-btn side-sub-btn"
            type="button"
            :class="{ active: operationsSubTab === 'runtime' }"
            @click="operationsSubTab = 'runtime'"
          >
            {{ tr("运行概览", "Overview") }}
          </button>
          <button
            class="side-nav-btn side-sub-btn"
            type="button"
            :class="{ active: operationsSubTab === 'alerts' }"
            @click="operationsSubTab = 'alerts'"
          >
            {{ tr("运行告警", "Alerts") }}
          </button>
          <button
            class="side-nav-btn side-sub-btn"
            type="button"
            :class="{ active: operationsSubTab === 'instances' }"
            @click="operationsSubTab = 'instances'"
          >
            {{ tr("实例监控", "Instances") }}
          </button>
        </div>
      </aside>

      <div class="admin-main">
        <div v-if="adminModule === 'challenges' || adminModule === 'contests'" class="admin-grid">
      <section v-if="adminModule === 'challenges'" class="panel">
        <div class="row-between">
          <h2>{{ tr("题目管理", "Challenge Management") }}</h2>
          <span class="badge">{{ challenges.length }} {{ tr("条", "items") }}</span>
        </div>

        <template v-if="challengeSubTab === 'library'">
          <div class="row-between challenge-library-head">
            <div class="context-menu challenge-library-mode-switch">
              <button
                class="ghost"
                type="button"
                :class="{ active: challengeLibraryMode === 'catalog' }"
                @click="challengeLibraryMode = 'catalog'"
              >
                {{ tr("题库列表", "Challenge Library") }}
              </button>
              <button
                class="ghost"
                type="button"
                :class="{ active: challengeLibraryMode === 'editor' && !editingChallengeId }"
                @click="openCreateChallengeEditor"
              >
                {{ tr("创建题目", "Create challenge") }}
              </button>
              <button
                v-if="selectedChallengeId"
                class="ghost"
                type="button"
                :class="{ active: challengeLibraryMode === 'editor' && !!editingChallengeId }"
                @click="openSelectedChallengeEditor"
              >
                {{ tr("编辑选中", "Edit selected") }}
              </button>
            </div>
            <span class="muted">
              {{ challengeLibraryMode === "catalog" ? tr("浏览与筛选", "Browse & filter") : tr("配置与保存", "Configure & save") }}
            </span>
          </div>

          <div class="module-split challenge-library-shell">
            <div v-if="challengeLibraryMode === 'editor'" class="module-column module-column-fill challenge-editor-column">
              <div class="row-between">
                <h3>{{ challengeFormTitle }}</h3>
                <div class="actions-row compact-actions">
                  <button class="ghost" type="button" @click="challengeLibraryMode = 'catalog'">
                    {{ tr("返回题库", "Back to library") }}
                  </button>
                  <button
                    v-if="editingChallengeId"
                    class="ghost"
                    type="button"
                    @click="handleCancelChallengeEdit"
                  >{{ tl('取消编辑') }}</button>
                </div>
              </div>
              <details class="action-sheet">
                <summary>{{ tr("管理题目类别", "Manage challenge categories") }}</summary>
                <div class="action-sheet-body stack">
                  <form class="form-grid compact-grid" @submit.prevent="handleSaveChallengeCategory">
                    <label>
                      <span>slug</span>
                      <input v-model.trim="challengeCategoryDraft.slug" required maxlength="32" />
                    </label>
                    <label>
                      <span>{{ tr("显示名（可选）", "Display name (optional)") }}</span>
                      <input v-model.trim="challengeCategoryDraft.display_name" maxlength="64" />
                    </label>
                    <label>
                      <span>{{ tr("排序", "Sort") }}</span>
                      <input
                        v-model.number="challengeCategoryDraft.sort_order"
                        type="number"
                        min="-100000"
                        max="100000"
                      />
                    </label>
                    <div class="actions-row compact-actions category-actions">
                      <button class="ghost" type="submit" :disabled="savingChallengeCategory">
                        {{
                          savingChallengeCategory
                            ? tr("保存中...", "Saving...")
                            : editingChallengeCategoryId
                              ? tr("保存类别", "Save category")
                              : tr("创建类别", "Create category")
                        }}
                      </button>
                      <button
                        v-if="editingChallengeCategoryId"
                        class="ghost"
                        type="button"
                        @click="resetChallengeCategoryDraft"
                      >
                        {{ tr("取消编辑", "Cancel edit") }}
                      </button>
                    </div>
                  </form>
                  <p v-if="challengeCategoryError" class="error">{{ challengeCategoryError }}</p>
                  <div class="challenge-category-grid">
                    <article v-for="item in sortedChallengeCategories" :key="item.id" class="category-item">
                      <div class="row-between">
                        <strong>{{ item.display_name }}</strong>
                        <span class="badge">{{ item.sort_order }}</span>
                      </div>
                      <p class="mono muted">{{ item.slug }}</p>
                      <div class="actions-row compact-actions">
                        <button class="ghost" type="button" @click="loadChallengeCategoryIntoDraft(item)">
                          {{ tr("编辑", "Edit") }}
                        </button>
                        <button
                          class="danger"
                          type="button"
                          :disabled="item.is_builtin || deletingChallengeCategoryId === item.id"
                          @click="handleDeleteChallengeCategory(item)"
                        >
                          {{
                            deletingChallengeCategoryId === item.id
                              ? tr("删除中...", "Deleting...")
                              : tr("删除", "Delete")
                          }}
                        </button>
                      </div>
                    </article>
                  </div>
                </div>
              </details>
              <form class="form-grid challenge-create-form" @submit.prevent="handleCreateChallenge">
                <section class="challenge-form-block">
                  <header class="challenge-form-block-head">
                    <h4>{{ tr("基础信息", "Basics") }}</h4>
                    <p>{{ tr("定义题目身份、类别与分值。", "Define challenge identity, category, and score.") }}</p>
                  </header>
                  <div class="form-grid challenge-form-grid">
                    <label>
                      <span>{{ tl('标题') }}</span>
                      <input v-model.trim="newChallenge.title" required />
                    </label>
                    <label>
                      <span>slug</span>
                      <input v-model.trim="newChallenge.slug" required />
                    </label>
                    <label>
                      <span>{{ tl('分类') }}</span>
                      <select v-model="newChallenge.category" required>
                        <option value="" disabled>{{ tr("请选择类别", "Select category") }}</option>
                        <option
                          v-for="item in challengeCategoryOptions"
                          :key="item.id"
                          :value="item.slug"
                        >
                          {{ item.display_name }} ({{ item.slug }})
                        </option>
                      </select>
                    </label>
                    <label>
                      <span>{{ tl('难度') }}</span>
                      <select v-model="newChallenge.difficulty">
                        <option value="easy">easy</option>
                        <option value="normal">normal</option>
                        <option value="hard">hard</option>
                        <option value="insane">insane</option>
                      </select>
                    </label>
                    <label>
                      <span>{{ tl('分值') }}</span>
                      <input v-model.number="newChallenge.static_score" type="number" min="1" />
                    </label>
                    <label>
                      <span>{{ tl('状态') }}</span>
                      <select v-model="newChallenge.status">
                        <option value="draft">draft</option>
                        <option value="published">published</option>
                        <option value="offline">offline</option>
                      </select>
                    </label>
                  </div>
                </section>

                <section class="challenge-form-block">
                  <header class="challenge-form-block-head">
                    <h4>{{ tr("判题与部署", "Validation & Runtime") }}</h4>
                    <p>{{ tr("配置题型、flag 策略和运行环境。", "Configure challenge type, flag strategy, and runtime.") }}</p>
                  </header>
                  <div class="form-grid challenge-form-grid">
                    <label>
                      <span>{{ tl('题型') }}</span>
                      <select v-model="newChallenge.challenge_type">
                        <option value="static">static</option>
                        <option value="dynamic">dynamic</option>
                        <option value="internal">internal</option>
                      </select>
                      <small class="field-note">{{ challengeTypeDescription }}</small>
                    </label>
                    <label>
                      <span>{{ tl('flag 模式') }}</span>
                      <select v-model="newChallenge.flag_mode">
                        <option value="static">static</option>
                        <option value="dynamic">dynamic</option>
                        <option value="script">script</option>
                      </select>
                      <small class="field-note">{{ flagModeDescription }}</small>
                    </label>
                    <label class="field-span-2">
                      <span>{{ tl('flag/哈希') }}</span>
                      <input v-model="newChallenge.flag_hash" />
                    </label>
                    <label>
                      <span>{{ tl('运行模式') }}</span>
                      <select v-model="newChallenge.runtime_mode">
                        <option value="none">{{ tr("no_runtime（无需容器）", "no_runtime (no container runtime)") }}</option>
                        <option value="compose">{{ tl('compose（多容器）') }}</option>
                        <option value="single_image">{{ tl('single_image（单镜像）') }}</option>
                      </select>
                      <small class="field-note">{{ runtimeModeDescription }}</small>
                    </label>
                    <label v-if="newChallenge.runtime_mode === 'compose'">
                      <span>{{ tl('访问模式') }}</span>
                      <select v-model="newChallenge.runtime_access_mode">
                        <option value="ssh_bastion">{{ tl('ssh_bastion（默认）') }}</option>
                        <option value="wireguard">wireguard（VPN）</option>
                        <option value="direct">{{ tl('direct（直连入口）') }}</option>
                      </select>
                    </label>
                    <label v-if="newChallenge.runtime_mode === 'single_image'" class="field-span-2">
                      <span>{{ tl('镜像仓库地址') }}</span>
                      <input v-model.trim="newChallenge.runtime_image" />
                    </label>
                    <label v-if="newChallenge.runtime_mode === 'single_image'">
                      <span>{{ tl('内部端口') }}</span>
                      <input v-model.number="newChallenge.runtime_internal_port" type="number" min="1" max="65535" />
                    </label>
                    <label v-if="newChallenge.runtime_mode === 'single_image'">
                      <span>{{ tl('入口协议') }}</span>
                      <select v-model="newChallenge.runtime_protocol">
                        <option value="http">http</option>
                        <option value="https">https</option>
                        <option value="tcp">tcp</option>
                      </select>
                    </label>
                    <div v-if="newChallenge.runtime_mode === 'single_image'" class="image-test-block">
                      <div class="actions-row compact-actions">
                        <button
                          class="ghost"
                          type="button"
                          @click="handleTestChallengeRuntimeImage"
                          :disabled="testingChallengeRuntimeImage || !newChallenge.runtime_image.trim()"
                        >
                          {{ testingChallengeRuntimeImage ? tl('测试中...') : tl('测试镜像（拉取+构建探测）') }}
                        </button>
                      </div>
                      <p class="muted">
                        {{
                          tr(
                            "测试日志会实时显示（默认最长 300 秒），适合镜像拉取/构建较慢场景。",
                            "Live logs are streamed in real time (default timeout: 300s)."
                          )
                        }}
                      </p>
                      <pre
                        v-if="challengeRuntimeImageStreamOutput || testingChallengeRuntimeImage"
                        class="mono image-test-live-log"
                      >{{ challengeRuntimeImageStreamOutput || tr("等待日志输出...", "Waiting for stream output...") }}</pre>
                      <p v-if="challengeImageTestError" class="error">{{ challengeImageTestError }}</p>
                      <div
                        v-if="challengeRuntimeImageTestResult"
                        class="image-test-result"
                        :class="{ failed: !challengeRuntimeImageTestResult.succeeded }"
                      >
                        <p class="mono">
                          {{ tl('镜像') }} {{ challengeRuntimeImageTestResult.image }} ·
                          {{ tl('结果') }} {{ challengeRuntimeImageTestResult.succeeded ? "success" : "failed" }} ·
                          {{ tl('时间') }} {{ formatTime(challengeRuntimeImageTestResult.generated_at) }}
                        </p>
                        <details v-for="step in challengeRuntimeImageTestResult.steps" :key="step.step" class="image-test-step">
                          <summary>
                            {{ step.step }} · {{ step.success ? "ok" : "failed" }} · {{ step.duration_ms }}ms · exit={{ step.exit_code ?? "timeout" }}
                          </summary>
                          <pre class="mono">{{ step.output || "-" }}</pre>
                        </details>
                      </div>
                    </div>
                    <label v-if="newChallenge.runtime_mode === 'compose'" class="field-span-2">
                      <span>
                        {{
                          newChallenge.challenge_type === "static"
                            ? tl("compose 模板（可选）")
                            : tr("compose 模板（dynamic/internal 必填）", "compose template (required for dynamic/internal)")
                        }}
                      </span>
                      <textarea v-model="newChallenge.compose_template" rows="5" />
                    </label>
                  </div>
                </section>

                <section class="challenge-form-block">
                  <header class="challenge-form-block-head">
                    <h4>{{ tr("内容与题解", "Content & Writeup") }}</h4>
                    <p>{{ tr("配置标签、可见策略和内容文案。", "Configure tags, visibility policy, and writeup content.") }}</p>
                  </header>
                  <div class="form-grid challenge-form-grid">
                    <label>
                      <span>{{ tl('标签（逗号分隔）') }}</span>
                      <input v-model="newChallenge.tags_input" />
                    </label>
                    <label>
                      <span>{{ tl('题解可见策略') }}</span>
                      <select v-model="newChallenge.writeup_visibility">
                        <option value="hidden">hidden</option>
                        <option value="after_solve">after_solve</option>
                        <option value="after_contest">after_contest</option>
                        <option value="public">public</option>
                      </select>
                    </label>
                    <label class="field-span-2">
                      <span>{{ tl('描述（可选）') }}</span>
                      <textarea v-model="newChallenge.description" rows="4" />
                    </label>
                    <label class="field-span-2">
                      <span>{{ tr("提示（每行一条）", "Hints (one per line)") }}</span>
                      <textarea v-model="newChallenge.hints_input" rows="4" />
                    </label>
                    <label class="field-span-2">
                      <span>{{ tl('题解内容（可选）') }}</span>
                      <textarea v-model="newChallenge.writeup_content" rows="5" />
                    </label>
                    <label class="field-span-2">
                      <span>{{ tl('版本备注（可选）') }}</span>
                      <input v-model="newChallenge.change_note" />
                    </label>
                  </div>
                </section>

                <section class="challenge-form-block">
                  <header class="challenge-form-block-head">
                    <h4>{{ tr("附件（快捷）", "Attachments (Quick)") }}</h4>
                    <p>
                      {{
                        tr(
                          "在题库配置页直接管理附件，减少“编辑题目”与“版本与附件”来回切换。",
                          "Manage attachments directly here to reduce tab switching."
                        )
                      }}
                    </p>
                  </header>
                  <div v-if="editingChallengeId" class="stack">
                    <p v-if="challengeAttachmentError" class="error">{{ challengeAttachmentError }}</p>
                    <div class="form-grid compact-grid">
                      <label class="field-span-2">
                        <span>{{ tl('上传附件') }}</span>
                        <UploadField
                          v-model="selectedAttachmentFile"
                          :input-key="attachmentInputKey"
                          :button-label="tl('选择文件')"
                          :clear-label="tl('清除')"
                          :placeholder="tl('未选择文件')"
                          :hint="challengeAttachmentUploadHint"
                        />
                      </label>
                      <button
                        class="ghost"
                        type="button"
                        @click="handleUploadChallengeAttachment"
                        :disabled="uploadingAttachment || !selectedAttachmentFile"
                      >
                        {{ uploadingAttachment ? tl('上传中...') : tl('上传附件') }}
                      </button>
                      <button class="ghost" type="button" @click="openVersionsFromEditor">
                        {{ tr("打开完整版本与附件", "Open full versions & files") }}
                      </button>
                    </div>

                    <div class="admin-list stagger-list">
                      <article
                        v-for="attachment in challengeAttachments.slice(0, 5)"
                        :key="attachment.id"
                        class="admin-list-item"
                      >
                        <div class="row-between">
                          <strong>{{ attachment.filename }}</strong>
                          <span class="muted">{{ formatTime(attachment.created_at) }}</span>
                        </div>
                        <p class="muted mono">{{ attachment.content_type }} · {{ formatSize(attachment.size_bytes) }}</p>
                        <div class="actions-row compact-actions">
                          <button
                            class="danger"
                            type="button"
                            @click="deleteChallengeAttachment(attachment.id)"
                            :disabled="deletingAttachmentId === attachment.id"
                          >
                            {{ deletingAttachmentId === attachment.id ? tl('删除中...') : tl('删除附件') }}
                          </button>
                        </div>
                      </article>
                      <p v-if="challengeAttachments.length === 0" class="muted">{{ tl('暂无附件。') }}</p>
                      <p v-if="challengeAttachments.length > 5" class="muted">
                        {{ tr("仅显示最近 5 个附件，更多请切换到“版本与附件”。", "Showing latest 5 files. Switch to \"Versions & Files\" for all.") }}
                      </p>
                    </div>
                  </div>
                  <p v-else class="muted">
                    {{ tr("请先保存题目，再上传附件。", "Save this challenge first, then upload attachments.") }}
                  </p>
                </section>

                <div class="challenge-submit-row">
                  <button class="primary" type="submit" :disabled="creatingChallenge">
                    {{ challengeSubmitLabel }}
                  </button>
                </div>
              </form>

              <p v-if="challengeError" class="error">{{ challengeError }}</p>
            </div>

            <div v-else class="module-column challenge-library-column">
              <div class="row-between">
                <h3>{{ tl('题库概览') }}</h3>
                <span class="badge">{{ filteredChallenges.length }} / {{ challenges.length }}</span>
              </div>
              <label class="search-field">
                <span>{{ tl('快速筛选') }}</span>
                <input v-model.trim="challengeKeyword" :placeholder="tl('按标题、slug、分类筛选')" />
              </label>

              <div class="challenge-card-grid stagger-list">
                <article
                  v-for="item in filteredChallenges"
                  :key="item.id"
                  class="admin-list-item challenge-card"
                  :class="{ active: selectedChallengeId === item.id }"
                  @click="selectChallenge(item.id)"
                >
                  <div class="row-between">
                    <strong>{{ item.title }}</strong>
                    <span class="badge">{{ item.challenge_type }}</span>
                  </div>
                  <p class="muted mono">{{ item.slug }}</p>
                  <p class="muted">{{ item.category }} · {{ item.difficulty }} · {{ item.flag_mode }}</p>
                  <p class="muted">{{ tl('分值') }} {{ item.static_score }} {{ tl('· 状态') }} {{ item.status }} {{ tl('· 版本 v') }}{{ item.current_version }}</p>
                  <div class="actions-row compact-actions">
                    <button
                      class="ghost"
                      type="button"
                      @click.stop="selectChallenge(item.id)"
                      :disabled="selectedChallengeId === item.id"
                    >
                      {{ selectedChallengeId === item.id ? tl('已选中') : tl('选择并管理') }}
                    </button>
                  </div>
                  <details v-if="selectedChallengeId === item.id" class="action-sheet action-sheet-inverse">
                    <summary>{{ tl('显示题目操作菜单') }}</summary>
                    <div class="actions-row compact-actions action-sheet-body">
                      <button
                        class="ghost"
                        type="button"
                        @click.stop="handleLoadChallengeForEdit(item.id)"
                        :disabled="destroyingChallengeId === item.id"
                      >{{ tl('编辑') }}</button>
                      <button
                        class="ghost"
                        type="button"
                        @click.stop="
                          selectChallenge(item.id);
                          challengeSubTab = 'versions';
                        "
                      >{{ tl('版本/附件') }}</button>
                      <button
                        class="ghost"
                        type="button"
                        @click.stop="updateChallengeStatus(item.id, 'published')"
                        :disabled="updatingChallengeId === item.id || destroyingChallengeId === item.id || item.status === 'published'"
                      >{{ tl('发布') }}</button>
                      <button
                        class="ghost"
                        type="button"
                        @click.stop="updateChallengeStatus(item.id, 'draft')"
                        :disabled="updatingChallengeId === item.id || destroyingChallengeId === item.id || item.status === 'draft'"
                      >{{ tl('草稿') }}</button>
                      <button
                        class="ghost"
                        type="button"
                        @click.stop="updateChallengeStatus(item.id, 'offline')"
                        :disabled="updatingChallengeId === item.id || destroyingChallengeId === item.id || item.status === 'offline'"
                      >{{ tl('下线') }}</button>
                      <button
                        class="danger"
                        type="button"
                        @click.stop="handleDestroyChallenge(item)"
                        :disabled="updatingChallengeId === item.id || destroyingChallengeId === item.id"
                      >
                        {{ destroyingChallengeId === item.id ? tl('销毁中...') : tl('销毁') }}
                      </button>
                    </div>
                  </details>
                </article>
              </div>
              <p v-if="filteredChallenges.length === 0" class="muted">{{ tl('没有匹配的题目。') }}</p>
            </div>
          </div>
        </template>

        <template v-if="challengeSubTab === 'versions' && selectedChallenge">
          <h3>{{ tl('版本与附件：') }}{{ selectedChallenge.title }}</h3>
          <p v-if="challengeVersionError" class="error">{{ challengeVersionError }}</p>
          <p v-if="challengeAttachmentError" class="error">{{ challengeAttachmentError }}</p>

          <form class="form-grid" @submit.prevent="handleRollbackChallengeVersion">
            <label>
              <span>{{ tl('回滚版本号') }}</span>
              <input v-model.number="rollbackForm.version_no" type="number" min="1" required />
            </label>
            <label>
              <span>{{ tl('回滚备注（可选）') }}</span>
              <input v-model="rollbackForm.change_note" />
            </label>
            <button class="ghost" type="submit" :disabled="rollingBack">
              {{ rollingBack ? tl('回滚中...') : tl('执行回滚') }}
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
                  {{ tl('回滚到该版本') }}
                </button>
              </div>
            </article>
            <p v-if="challengeVersions.length === 0" class="muted">{{ tl('暂无版本记录。') }}</p>
          </div>

          <form class="form-grid" @submit.prevent="handleUploadChallengeAttachment">
            <label>
              <span>{{ tl('上传附件') }}</span>
              <UploadField
                v-model="selectedAttachmentFile"
                :input-key="attachmentInputKey"
                :button-label="tl('选择文件')"
                :clear-label="tl('清除')"
                :placeholder="tl('未选择文件')"
                :hint="challengeAttachmentUploadHint"
              />
            </label>
            <button class="ghost" type="submit" :disabled="uploadingAttachment || !selectedAttachmentFile">
              {{ uploadingAttachment ? tl('上传中...') : tl('上传附件') }}
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
                  {{ deletingAttachmentId === attachment.id ? tl('删除中...') : tl('删除附件') }}
                </button>
              </div>
            </article>
            <p v-if="challengeAttachments.length === 0" class="muted">{{ tl('暂无附件。') }}</p>
          </div>
        </template>
        <p v-if="challengeSubTab === 'versions' && !selectedChallenge" class="muted">
          {{ tl('请先在“题库配置”中选择一个题目，再切换到版本管理。') }}
        </p>

        <template v-if="challengeSubTab === 'lint'">
          <div class="row-between">
            <h3>{{ tl('运行模板校验') }}</h3>
            <button
              class="ghost"
              type="button"
              @click="loadChallengeRuntimeLint()"
              :disabled="loadingChallengeRuntimeLint"
            >
              {{ loadingChallengeRuntimeLint ? tl('扫描中...') : tl('刷新校验') }}
            </button>
          </div>

          <details class="filter-sheet">
            <summary>{{ tl('展开筛选条件') }}</summary>
            <div class="actions-row filter-sheet-body">
              <label>
                <span>{{ tl('题型') }}</span>
                <select v-model="challengeLintTypeFilter">
                  <option value="">all</option>
                  <option value="static">static</option>
                  <option value="dynamic">dynamic</option>
                  <option value="internal">internal</option>
                </select>
              </label>
              <label>
                <span>{{ tl('状态') }}</span>
                <select v-model="challengeLintStatusFilter">
                  <option value="">all</option>
                  <option value="draft">draft</option>
                  <option value="published">published</option>
                  <option value="offline">offline</option>
                </select>
              </label>
              <label>
                <span>{{ tl('关键词') }}</span>
                <input v-model.trim="challengeLintKeywordFilter" :placeholder="tl('标题或 slug')" />
              </label>
              <label>
                <span>{{ tl('条数') }}</span>
                <input v-model.number="challengeLintLimit" type="number" min="1" max="5000" />
              </label>
              <label class="inline-check">
                <span>{{ tl('仅错误') }}</span>
                <input v-model="challengeLintOnlyErrors" type="checkbox" />
              </label>
              <button
                class="ghost"
                type="button"
                @click="loadChallengeRuntimeLint()"
                :disabled="loadingChallengeRuntimeLint"
              >
                {{ tl('应用筛选') }}
              </button>
            </div>
          </details>

          <p v-if="challengeLintError" class="error">{{ challengeLintError }}</p>

          <div v-if="challengeRuntimeLint" class="challenge-lint-metrics">
            <article class="metric-card">
              <h4>{{ tl('扫描总数') }}</h4>
              <p>{{ challengeRuntimeLint.scanned_total }}</p>
            </article>
            <article class="metric-card">
              <h4>{{ tl('通过') }}</h4>
              <p>{{ challengeRuntimeLint.ok_count }}</p>
            </article>
            <article class="metric-card">
              <h4>{{ tl('错误') }}</h4>
              <p>{{ challengeRuntimeLint.error_count }}</p>
            </article>
            <article class="metric-card">
              <h4>{{ tl('更新时间') }}</h4>
              <p>{{ formatTime(challengeRuntimeLint.generated_at) }}</p>
            </article>
          </div>

          <table
            v-if="challengeLintItems.length > 0"
            class="scoreboard-table challenge-lint-table"
          >
            <thead>
              <tr>
                <th>{{ tl('题目') }}</th>
                <th>{{ tl('题型') }}</th>
                <th>{{ tl('状态') }}</th>
                <th>{{ tl('模板') }}</th>
                <th>{{ tl('校验') }}</th>
                <th>{{ tl('更新时间') }}</th>
                <th>{{ tl('信息') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="item in challengeLintItems" :key="item.id">
                <td>
                  <strong>{{ item.title }}</strong>
                  <p class="muted mono">{{ item.slug }}</p>
                </td>
                <td>{{ item.challenge_type }}</td>
                <td>{{ item.status }}</td>
                <td>{{ item.has_compose_template ? "yes" : "no" }}</td>
                <td>
                  <span
                    class="badge"
                    :class="item.lint_status === 'error' ? 'lint-badge-error' : 'lint-badge-ok'"
                  >
                    {{ item.lint_status }}
                  </span>
                </td>
                <td>{{ formatTime(item.updated_at) }}</td>
                <td class="mono audit-detail">{{ item.message ?? "-" }}</td>
              </tr>
            </tbody>
          </table>
          <p v-else class="muted">{{ tl('暂无匹配的模板校验记录。') }}</p>
        </template>
      </section>

      <section v-if="adminModule === 'contests' && contestSubTab === 'contests'" class="panel">
        <div class="row-between">
          <h2>{{ tl('比赛管理') }}</h2>
          <span class="badge">{{ contests.length }} {{ tl('场') }}</span>
        </div>

        <div class="row-between contest-manage-head">
          <div class="context-menu contest-manage-mode-switch">
            <button
              class="ghost"
              type="button"
              :class="{ active: contestManageMode === 'catalog' }"
              @click="contestManageMode = 'catalog'"
            >
              {{ tr("赛事列表", "Contest list") }}
            </button>
            <button
              class="ghost"
              type="button"
              :class="{ active: contestManageMode === 'editor' && !editingContestId }"
              @click="openCreateContestEditor"
            >
              {{ tr("创建比赛", "Create contest") }}
            </button>
            <button
              v-if="selectedContestId"
              class="ghost"
              type="button"
              :class="{ active: contestManageMode === 'editor' && !!editingContestId }"
              @click="openSelectedContestEditor"
            >
              {{ tr("编辑选中", "Edit selected") }}
            </button>
          </div>
          <span class="muted">
            {{ contestManageMode === "catalog" ? tr("浏览与运维", "Browse & operate") : tr("配置与保存", "Configure & save") }}
          </span>
        </div>

        <div class="module-split contest-management-shell">
          <div v-if="contestManageMode === 'editor'" class="module-column module-column-fill contest-editor-column">
            <div class="row-between">
              <h3>{{ contestFormTitle }}</h3>
              <div class="actions-row compact-actions">
                <button class="ghost" type="button" @click="contestManageMode = 'catalog'">
                  {{ tr("返回赛事列表", "Back to contest list") }}
                </button>
                <button
                  v-if="editingContestId"
                  class="ghost"
                  type="button"
                  @click="handleCancelContestEdit"
                >
                  {{ tl('取消编辑') }}
                </button>
              </div>
            </div>
            <form class="form-grid compact-grid" @submit.prevent="handleCreateContest">
              <label>
                <span>{{ tl('标题') }}</span>
                <input v-model.trim="newContest.title" required />
              </label>
              <label>
                <span>slug</span>
                <input v-model.trim="newContest.slug" required />
              </label>
              <label>
                <span>{{ tl('描述') }}</span>
                <input v-model="newContest.description" />
              </label>
              <label>
                <span>{{ tl('可见性') }}</span>
                <select v-model="newContest.visibility">
                  <option value="public">public</option>
                  <option value="private">private</option>
                </select>
              </label>
              <label>
                <span>{{ tl('初始状态') }}</span>
                <select v-model="newContest.status">
                  <option value="draft">draft</option>
                  <option value="scheduled">scheduled</option>
                  <option value="running">running</option>
                  <option value="ended">ended</option>
                  <option value="archived">archived</option>
                </select>
              </label>
              <label>
                <span>{{ tl('积分模式') }}</span>
                <select v-model="newContest.scoring_mode">
                  <option value="static">static</option>
                  <option value="dynamic">dynamic</option>
                </select>
              </label>
              <label>
                <span>{{ tl('动态衰减参数') }}</span>
                <input v-model.number="newContest.dynamic_decay" type="number" min="1" max="100000" />
              </label>
              <label>
                <span>{{ tr("一血加成(%)", "First blood bonus (%)") }}</span>
                <input
                  v-model.number="newContest.first_blood_bonus_percent"
                  type="number"
                  min="0"
                  max="500"
                />
              </label>
              <label>
                <span>{{ tr("二血加成(%)", "Second blood bonus (%)") }}</span>
                <input
                  v-model.number="newContest.second_blood_bonus_percent"
                  type="number"
                  min="0"
                  max="500"
                />
              </label>
              <label>
                <span>{{ tr("三血加成(%)", "Third blood bonus (%)") }}</span>
                <input
                  v-model.number="newContest.third_blood_bonus_percent"
                  type="number"
                  min="0"
                  max="500"
                />
              </label>
              <label class="inline-check">
                <input v-model="newContest.registration_requires_approval" type="checkbox" />
                <span>{{ tr("报名需管理员审批", "Registration requires admin approval") }}</span>
              </label>
              <label>
                <span>{{ tl('开始时间') }}</span>
                <input v-model="newContest.start_at" type="datetime-local" required />
              </label>
              <label>
                <span>{{ tl('结束时间') }}</span>
                <input v-model="newContest.end_at" type="datetime-local" required />
              </label>
              <label>
                <span>{{ tl('封榜时间（可选）') }}</span>
                <input v-model="newContest.freeze_at" type="datetime-local" />
              </label>

              <button class="primary" type="submit" :disabled="creatingContest">
                {{ contestSubmitLabel }}
              </button>
            </form>
          </div>

          <div v-else class="module-column contest-catalog-column">
            <div class="row-between">
              <h3>{{ tl('赛事列表') }}</h3>
              <span class="badge">{{ filteredContests.length }} / {{ contests.length }}</span>
            </div>
            <label class="search-field">
              <span>{{ tl('快速筛选') }}</span>
              <input v-model.trim="contestKeyword" :placeholder="tl('按标题、slug、状态筛选')" />
            </label>

            <div class="contest-catalog-workspace">
              <aside class="contest-list-pane contest-catalog-list">
                <button
                  v-for="contest in filteredContests"
                  :key="contest.id"
                  class="contest-list-item"
                  :class="{ active: selectedContestId === contest.id }"
                  type="button"
                  @click="selectContest(contest.id)"
                >
                  <div class="row-between">
                    <strong>{{ contest.title }}</strong>
                    <span class="badge">{{ contest.status }}</span>
                  </div>
                  <span class="muted mono">{{ contest.slug }}</span>
                  <span class="muted">{{ contest.visibility }} · {{ contest.scoring_mode }}</span>
                </button>
                <p v-if="filteredContests.length === 0" class="muted">{{ tl('没有匹配的比赛。') }}</p>
              </aside>

              <section v-if="selectedContest" class="contest-detail-pane contest-catalog-detail">
                <div class="row-between">
                  <h4>{{ selectedContest.title }}</h4>
                  <span class="badge">{{ selectedContest.status }}</span>
                </div>
                <p class="muted mono">{{ selectedContest.slug }} · {{ selectedContest.visibility }}</p>
                <p class="muted">
                  {{ tr("计分模式", "Scoring") }} {{ selectedContest.scoring_mode }} ·
                  {{ tr("动态衰减", "Dynamic decay") }} {{ selectedContest.dynamic_decay }}
                </p>
                <p class="muted">
                  {{ tr("一二三血加成", "Blood bonuses") }}
                  {{ selectedContest.first_blood_bonus_percent }}% /
                  {{ selectedContest.second_blood_bonus_percent }}% /
                  {{ selectedContest.third_blood_bonus_percent }}%
                </p>
                <p class="muted">
                  {{
                    selectedContest.registration_requires_approval
                      ? tr("报名模式：需管理员审批", "Registration: admin approval required")
                      : tr("报名模式：自动通过", "Registration: auto-approved")
                  }}
                </p>
                <p class="muted">
                  {{ formatTime(selectedContest.start_at) }} ~ {{ formatTime(selectedContest.end_at) }}
                </p>
                <p class="muted" v-if="selectedContest.description">{{ selectedContest.description }}</p>

                <img
                  v-if="canPreviewContestPoster(selectedContest)"
                  class="contest-poster-preview"
                  :src="contestPosterPreviewUrl(selectedContest)"
                  alt="contest poster preview"
                />
                <p v-else class="muted">
                  {{ selectedContest.poster_url ? tl('该海报当前在比赛中心不可预览（仅 public 且 scheduled/running/ended 可见）。') : tl('当前未设置海报。') }}
                </p>

                <form class="form-grid" @submit.prevent="handleUploadContestPoster">
                  <label>
                    <span>{{ tl('上传比赛海报') }}</span>
                    <UploadField
                      v-model="selectedContestPosterFile"
                      :input-key="contestPosterInputKey"
                      accept="image/*"
                      :button-label="tl('选择图片')"
                      :clear-label="tl('清除')"
                      :placeholder="tl('未选择图片文件')"
                      :hint="tl('支持 PNG、JPG、WebP 等格式')"
                    />
                  </label>
                  <div class="actions-row compact-actions">
                    <button
                      class="ghost"
                      type="submit"
                      :disabled="uploadingContestPoster || !selectedContestPosterFile"
                    >
                      {{ uploadingContestPoster ? tl('上传中...') : tl('上传海报') }}
                    </button>
                    <button
                      class="danger"
                      type="button"
                      @click="handleDeleteContestPoster(selectedContest)"
                      :disabled="deletingContestPosterId === selectedContest.id || !selectedContest.poster_url"
                    >
                      {{ deletingContestPosterId === selectedContest.id ? tl('删除中...') : tl('删除海报') }}
                    </button>
                  </div>
                </form>

                <details class="action-sheet">
                  <summary>{{ tl('显示比赛操作菜单') }}</summary>
                  <div class="action-sheet-body">
                    <div class="actions-row compact-actions">
                      <button
                        v-for="status in statusActions"
                        :key="status"
                        class="ghost"
                        type="button"
                        :disabled="updatingContestId === selectedContest.id || destroyingContestId === selectedContest.id || selectedContest.status === status"
                        @click="updateContestStatus(selectedContest.id, status)"
                      >
                        {{ status }}
                      </button>
                    </div>
                    <div class="actions-row compact-actions">
                      <button
                        class="ghost"
                        type="button"
                        @click="handleLoadContestForEdit(selectedContest)"
                      >
                        {{ tl('编辑比赛配置') }}
                      </button>
                      <button class="ghost" type="button" @click="contestSubTab = 'bindings'">
                        {{ tl('管理题目挂载') }}
                      </button>
                      <button class="ghost" type="button" @click="contestSubTab = 'announcements'">
                        {{ tl('管理公告') }}
                      </button>
                      <button class="ghost" type="button" @click="contestSubTab = 'registrations'">
                        {{ tr("管理报名审核", "Manage registrations") }}
                      </button>
                      <button
                        class="danger"
                        type="button"
                        @click="handleDestroyContest(selectedContest)"
                        :disabled="destroyingContestId === selectedContest.id || updatingContestId === selectedContest.id"
                      >
                        {{ destroyingContestId === selectedContest.id ? tl('销毁中...') : tl('销毁比赛') }}
                      </button>
                    </div>
                  </div>
                </details>
              </section>

              <section v-else class="contest-detail-pane contest-catalog-detail">
                <p class="muted">{{ tl('从左侧选择一个比赛查看详情与状态操作。') }}</p>
              </section>
            </div>
          </div>
        </div>
        <p v-if="contestError" class="error">{{ contestError }}</p>
      </section>

      <section v-if="adminModule === 'contests' && contestSubTab === 'bindings'" class="panel">
        <div class="row-between">
          <h2>{{ tl('比赛题目挂载') }}</h2>
          <span class="badge" v-if="selectedContest">{{ selectedContest.title }}</span>
        </div>

        <p class="muted" v-if="!selectedContest">{{ tl('请先在中间列选择一个比赛。') }}</p>

        <template v-else>
          <p v-if="bindingError" class="error">{{ bindingError }}</p>

          <div class="contest-browser">
            <aside class="contest-list-pane">
              <h3>{{ tl('挂载/更新题目') }}</h3>
              <form class="form-grid compact-grid" @submit.prevent="handleUpsertBinding">
                <label>
                  <span>{{ tl('选择题目') }}</span>
                  <select v-model="bindingForm.challenge_id" required>
                    <option value="" disabled>{{ tl('请选择题目') }}</option>
                    <option v-for="item in challenges" :key="item.id" :value="item.id">
                      {{ item.title }} ({{ item.category }})
                    </option>
                  </select>
                </label>
                <label>
                  <span>{{ tl('排序') }}</span>
                  <input v-model.number="bindingForm.sort_order" type="number" />
                </label>
                <label>
                  <span>{{ tl('发布时间（可选）') }}</span>
                  <input v-model="bindingForm.release_at" type="datetime-local" />
                </label>
                <button class="primary" type="submit" :disabled="bindingBusy">
                  {{ bindingBusy ? tl('处理中...') : tl('挂载或更新') }}
                </button>
              </form>

              <h3>{{ tl('已挂载题目') }}</h3>
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
              <p v-if="contestBindings.length === 0" class="muted">{{ tl('当前比赛未挂载题目。') }}</p>
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
                  {{ tr("发布时间", "Release at") }}:
                  {{ selectedBinding.release_at ? formatTime(selectedBinding.release_at) : "-" }}
                </p>
                <details class="action-sheet">
                  <summary>{{ tl('显示挂载操作菜单') }}</summary>
                  <div class="actions-row compact-actions action-sheet-body">
                    <button
                      class="ghost"
                      type="button"
                      @click="loadBindingIntoForm(selectedBinding)"
                      :disabled="bindingBusy"
                    >
                      {{ tl('加载到左侧表单') }}
                    </button>
                    <button
                      class="ghost"
                      type="button"
                      @click="quickAdjustSort(selectedBinding.challenge_id, selectedBinding.sort_order - 1)"
                      :disabled="bindingBusy"
                    >
                      {{ tl('上移') }}
                    </button>
                    <button
                      class="ghost"
                      type="button"
                      @click="quickAdjustSort(selectedBinding.challenge_id, selectedBinding.sort_order + 1)"
                      :disabled="bindingBusy"
                    >
                      {{ tl('下移') }}
                    </button>
                    <button
                      class="ghost"
                      type="button"
                      @click="clearBindingReleaseAt(selectedBinding.challenge_id)"
                      :disabled="bindingBusy"
                    >
                      {{ tl('清除发布时间') }}
                    </button>
                    <button
                      class="danger"
                      type="button"
                      @click="removeBinding(selectedBinding.challenge_id)"
                      :disabled="bindingBusy"
                    >
                      {{ tl('移除挂载') }}
                    </button>
                  </div>
                </details>
              </template>
              <p v-else class="muted">{{ tl('从左侧选择一个挂载题目查看详情。') }}</p>
            </section>
          </div>
        </template>
      </section>

      <section v-if="adminModule === 'contests' && contestSubTab === 'announcements'" class="panel">
        <div class="row-between">
          <h2>{{ tl('比赛公告管理') }}</h2>
          <span class="badge" v-if="selectedContest">{{ selectedContest.title }}</span>
        </div>

        <p class="muted" v-if="!selectedContest">{{ tl('请先在“赛事配置”中选择一个比赛。') }}</p>

        <template v-else>
          <p v-if="announcementError" class="error">{{ announcementError }}</p>

          <div class="contest-browser announcement-browser">
            <aside class="contest-list-pane announcement-pane">
              <section class="announcement-block announcement-create-block">
                <div class="row-between">
                  <h3>{{ tl('创建公告') }}</h3>
                  <div
                    class="context-menu announcement-mode-switch"
                    role="tablist"
                    :aria-label="tr('创建公告编辑模式', 'Create announcement editor mode')"
                  >
                    <button
                      class="ghost"
                      type="button"
                      :class="{ active: announcementCreateMode === 'edit' }"
                      :aria-pressed="announcementCreateMode === 'edit'"
                      @click="announcementCreateMode = 'edit'"
                    >
                      {{ tr("编辑", "Edit") }}
                    </button>
                    <button
                      class="ghost"
                      type="button"
                      :class="{ active: announcementCreateMode === 'preview' }"
                      :aria-pressed="announcementCreateMode === 'preview'"
                      @click="announcementCreateMode = 'preview'"
                    >
                      {{ tr("预览", "Preview") }}
                    </button>
                  </div>
                </div>

                <form class="form-grid announcement-create-form" @submit.prevent="handleCreateAnnouncement">
                  <label>
                    <span>{{ tl('公告标题') }}</span>
                    <input
                      v-model.trim="announcementForm.title"
                      maxlength="180"
                      required
                      :placeholder="tr('例如：比赛赛程更新', 'Example: Schedule update')"
                    />
                  </label>

                  <label class="announcement-editor-label">
                    <span>{{ tr("公告内容（Markdown）", "Announcement Content (Markdown)") }}</span>
                  </label>

                  <div class="announcement-toolbar">
                    <button class="ghost" type="button" @click="insertMarkdownSnippet('create', 'heading')">{{ tr("标题", "H2") }}</button>
                    <button class="ghost" type="button" @click="insertMarkdownSnippet('create', 'bold')">{{ tr("加粗", "Bold") }}</button>
                    <button class="ghost" type="button" @click="insertMarkdownSnippet('create', 'italic')">{{ tr("斜体", "Italic") }}</button>
                    <button class="ghost" type="button" @click="insertMarkdownSnippet('create', 'link')">{{ tr("链接", "Link") }}</button>
                    <button class="ghost" type="button" @click="insertMarkdownSnippet('create', 'code')">{{ tr("代码", "Code") }}</button>
                    <button class="ghost" type="button" @click="insertMarkdownSnippet('create', 'list')">{{ tr("列表", "List") }}</button>
                    <button class="ghost" type="button" @click="insertMarkdownSnippet('create', 'quote')">{{ tr("引用", "Quote") }}</button>
                  </div>

                  <div class="announcement-editor-shell">
                    <textarea
                      v-if="announcementCreateMode === 'edit'"
                      ref="announcementCreateTextareaRef"
                      v-model.trim="announcementForm.content"
                      rows="8"
                      class="announcement-editor-textarea"
                      :placeholder="tr('支持 Markdown，建议分段编写公告。', 'Markdown supported. Write the announcement in structured sections.')"
                    />
                    <article
                      v-else-if="announcementForm.content.trim()"
                      class="announcement-markdown-preview markdown-body"
                      v-html="renderAnnouncementMarkdown(announcementForm.content)"
                    ></article>
                    <p v-else class="muted">{{ tr("暂无预览内容。", "No preview content yet.") }}</p>
                  </div>

                  <p class="soft announcement-markdown-hint">
                    {{ tr("支持标题、列表、代码块、引用与链接。", "Supports headings, lists, code blocks, quotes and links.") }}
                  </p>

                  <div class="actions-row compact-actions">
                    <label class="inline-check">
                      <input v-model="announcementForm.is_published" type="checkbox" />
                      <span>{{ tl('立即发布') }}</span>
                    </label>
                    <label class="inline-check">
                      <input v-model="announcementForm.is_pinned" type="checkbox" />
                      <span>{{ tl('置顶公告') }}</span>
                    </label>
                  </div>

                  <button class="primary" type="submit" :disabled="creatingAnnouncement">
                    {{ creatingAnnouncement ? tl('创建中...') : tl('创建公告') }}
                  </button>
                </form>
              </section>

              <section class="announcement-block announcement-list-block">
                <div class="row-between">
                  <h3>{{ tl('公告列表') }}</h3>
                  <span class="badge">{{ contestAnnouncements.length }}</span>
                </div>
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
                    {{ item.published_at ? formatTime(item.published_at) : tl('未发布') }}
                  </span>
                </button>
                <p v-if="contestAnnouncements.length === 0" class="muted">{{ tl('暂无公告。') }}</p>
              </section>
            </aside>

            <section class="contest-detail-pane announcement-detail-pane">
              <template v-if="selectedAnnouncement">
                <header class="row-between">
                  <h4>{{ selectedAnnouncement.title }}</h4>
                  <span class="badge" v-if="selectedAnnouncement.is_pinned">{{ tl('置顶') }}</span>
                </header>
                <p class="muted mono">
                  {{ selectedAnnouncement.is_published ? "published" : "draft" }} ·
                  {{ selectedAnnouncement.published_at ? formatTime(selectedAnnouncement.published_at) : tl('未发布') }}
                </p>

                <form
                  v-if="announcementDrafts[selectedAnnouncement.id]"
                  class="form-grid announcement-edit-form"
                  @submit.prevent="saveAnnouncementEdit(selectedAnnouncement)"
                >
                  <label>
                    <span>{{ tl('标题') }}</span>
                    <input
                      v-model.trim="announcementDrafts[selectedAnnouncement.id].title"
                      required
                      maxlength="180"
                      :disabled="savingAnnouncementId === selectedAnnouncement.id"
                    />
                  </label>

                  <div class="row-between">
                    <span class="announcement-editor-title">{{ tr("内容（Markdown）", "Content (Markdown)") }}</span>
                    <div
                      class="context-menu announcement-mode-switch"
                      role="tablist"
                      :aria-label="tr('编辑公告内容模式', 'Edit announcement content mode')"
                    >
                      <button
                        class="ghost"
                        type="button"
                        :class="{ active: announcementEditMode === 'edit' }"
                        :aria-pressed="announcementEditMode === 'edit'"
                        @click="announcementEditMode = 'edit'"
                      >
                        {{ tr("编辑", "Edit") }}
                      </button>
                      <button
                        class="ghost"
                        type="button"
                        :class="{ active: announcementEditMode === 'preview' }"
                        :aria-pressed="announcementEditMode === 'preview'"
                        @click="announcementEditMode = 'preview'"
                      >
                        {{ tr("预览", "Preview") }}
                      </button>
                    </div>
                  </div>

                  <div class="announcement-toolbar">
                    <button class="ghost" type="button" @click="insertMarkdownSnippet('edit', 'heading')">{{ tr("标题", "H2") }}</button>
                    <button class="ghost" type="button" @click="insertMarkdownSnippet('edit', 'bold')">{{ tr("加粗", "Bold") }}</button>
                    <button class="ghost" type="button" @click="insertMarkdownSnippet('edit', 'italic')">{{ tr("斜体", "Italic") }}</button>
                    <button class="ghost" type="button" @click="insertMarkdownSnippet('edit', 'link')">{{ tr("链接", "Link") }}</button>
                    <button class="ghost" type="button" @click="insertMarkdownSnippet('edit', 'code')">{{ tr("代码", "Code") }}</button>
                    <button class="ghost" type="button" @click="insertMarkdownSnippet('edit', 'list')">{{ tr("列表", "List") }}</button>
                    <button class="ghost" type="button" @click="insertMarkdownSnippet('edit', 'quote')">{{ tr("引用", "Quote") }}</button>
                  </div>

                  <div class="announcement-editor-shell announcement-editor-shell-large">
                    <textarea
                      v-if="announcementEditMode === 'edit'"
                      ref="announcementEditTextareaRef"
                      v-model.trim="announcementDrafts[selectedAnnouncement.id].content"
                      rows="12"
                      class="announcement-editor-textarea announcement-editor-textarea-lg"
                      required
                      :disabled="savingAnnouncementId === selectedAnnouncement.id"
                    />
                    <article
                      v-else-if="currentAnnouncementDraftContent.trim()"
                      class="announcement-markdown-preview markdown-body"
                      v-html="renderAnnouncementMarkdown(currentAnnouncementDraftContent)"
                    ></article>
                    <p v-else class="muted">{{ tr("暂无预览内容。", "No preview content yet.") }}</p>
                  </div>

                  <div class="actions-row compact-actions">
                    <button class="ghost" type="submit" :disabled="savingAnnouncementId === selectedAnnouncement.id">
                      {{ savingAnnouncementId === selectedAnnouncement.id ? tl('保存中...') : tl('保存修改') }}
                    </button>
                  </div>
                </form>
                <p v-else class="muted">{{ tl('正在准备编辑器...') }}</p>

                <details class="action-sheet">
                  <summary>{{ tl('显示公告操作菜单') }}</summary>
                  <div class="actions-row compact-actions action-sheet-body">
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
                      {{ selectedAnnouncement.is_published ? tl('撤回发布') : tl('发布') }}
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
                      {{ selectedAnnouncement.is_pinned ? tl('取消置顶') : tl('置顶') }}
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
                      {{ deletingAnnouncementId === selectedAnnouncement.id ? tl('删除中...') : tl('删除公告') }}
                    </button>
                  </div>
                </details>
              </template>
              <p v-else class="muted">{{ tl('从左侧选择一个公告查看详情。') }}</p>
            </section>
          </div>
        </template>
      </section>

      <section v-if="adminModule === 'contests' && contestSubTab === 'registrations'" class="panel">
        <div class="row-between">
          <h2>{{ tr("报名审核", "Registration Review") }}</h2>
          <span class="badge" v-if="selectedContest">{{ selectedContest.title }}</span>
        </div>
        <p class="muted" v-if="!selectedContest">{{ tr("请先在“赛事配置”中选择一个比赛。", "Select a contest in config tab first.") }}</p>

        <template v-else>
          <div class="row compact-actions">
            <label>
              <span>{{ tr("状态筛选", "Status filter") }}</span>
              <select v-model="contestRegistrationStatusFilter">
                <option value="">{{ tr("全部", "All") }}</option>
                <option value="pending">pending</option>
                <option value="approved">approved</option>
                <option value="rejected">rejected</option>
              </select>
            </label>
            <button class="ghost" type="button" @click="loadContestRegistrations()" :disabled="updatingContestRegistrationId !== ''">
              {{ tr("刷新", "Refresh") }}
            </button>
          </div>

          <p v-if="contestRegistrationError" class="error">{{ contestRegistrationError }}</p>

          <table v-if="contestRegistrations.length > 0" class="scoreboard-table">
            <thead>
              <tr>
                <th>#</th>
                <th>{{ tr("队伍", "Team") }}</th>
                <th>{{ tr("状态", "Status") }}</th>
                <th>{{ tr("申请时间", "Requested at") }}</th>
                <th>{{ tr("审核时间", "Reviewed at") }}</th>
                <th>{{ tr("备注", "Note") }}</th>
                <th>{{ tr("操作", "Actions") }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(item, index) in contestRegistrations" :key="item.id">
                <td>{{ index + 1 }}</td>
                <td>{{ item.team_name }}</td>
                <td class="mono">{{ item.status }}</td>
                <td class="mono">{{ formatTime(item.requested_at) }}</td>
                <td class="mono">{{ item.reviewed_at ? formatTime(item.reviewed_at) : "-" }}</td>
                <td class="mono">{{ item.review_note || "-" }}</td>
                <td>
                  <div class="actions-row compact-actions">
                    <button
                      class="ghost"
                      type="button"
                      :disabled="updatingContestRegistrationId === item.id || item.status === 'approved'"
                      @click="updateContestRegistrationStatus(item, 'approved')"
                    >
                      {{ tr("批准", "Approve") }}
                    </button>
                    <button
                      class="ghost"
                      type="button"
                      :disabled="updatingContestRegistrationId === item.id || item.status === 'pending'"
                      @click="updateContestRegistrationStatus(item, 'pending')"
                    >
                      {{ tr("置为待审", "Set pending") }}
                    </button>
                    <button
                      class="danger"
                      type="button"
                      :disabled="updatingContestRegistrationId === item.id || item.status === 'rejected'"
                      @click="updateContestRegistrationStatus(item, 'rejected')"
                    >
                      {{ tr("拒绝", "Reject") }}
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
          <p v-else class="muted">{{ tr("当前没有报名记录。", "No registration records yet.") }}</p>
        </template>
      </section>
    </div>

    <section v-if="adminModule === 'operations' && operationsSubTab === 'runtime'" class="panel">
      <div class="row-between">
        <h2>{{ tl('运行概览') }}</h2>
        <div class="actions-row compact-actions">
          <span v-if="runtimeOverview" class="muted">{{ tl('更新于') }} {{ formatTime(runtimeOverview.generated_at) }}</span>
          <button
            class="ghost"
            type="button"
            @click="handleRunExpiredReaper"
            :disabled="runtimeReaperBusy !== ''"
          >
            {{ runtimeReaperBusy === "expired" ? tl('回收中...') : tl('执行过期回收') }}
          </button>
          <button
            class="ghost"
            type="button"
            @click="handleRunStaleReaper"
            :disabled="runtimeReaperBusy !== ''"
          >
            {{ runtimeReaperBusy === "stale" ? tl('回收中...') : tl('执行心跳超时回收') }}
          </button>
        </div>
      </div>

      <p v-if="runtimeError" class="error">{{ runtimeError }}</p>
      <p v-if="runtimeReaperError" class="error">{{ runtimeReaperError }}</p>
      <p v-if="runtimeReaperResult" class="muted mono">
        {{ tl('最近回收：mode=') }}{{ runtimeReaperResult.mode }} · scanned={{ runtimeReaperResult.scanned }} ·
        reaped={{ runtimeReaperResult.reaped }} · failed={{ runtimeReaperResult.failed }} ·
        updated_at={{ formatTime(runtimeReaperResult.generated_at) }}
      </p>

      <div v-if="runtimeOverview" class="runtime-metrics">
        <article class="metric-card">
          <h3>{{ tl('基础规模') }}</h3>
          <p>{{ tl('用户') }} {{ runtimeOverview.total_users }} {{ tl('· 队伍') }} {{ runtimeOverview.total_teams }}</p>
          <p>{{ tl('比赛') }} {{ runtimeOverview.total_contests }} {{ tl('· 题目') }} {{ runtimeOverview.total_challenges }}</p>
        </article>
        <article class="metric-card">
          <h3>{{ tl('比赛与提交') }}</h3>
          <p>{{ tl('运行中比赛') }} {{ runtimeOverview.running_contests }}</p>
          <p>{{ tl('总提交') }} {{ runtimeOverview.total_submissions }} {{ tl('· 24h 提交') }} {{ runtimeOverview.submissions_last_24h }}</p>
        </article>
        <article class="metric-card">
          <h3>{{ tl('实例健康') }}</h3>
          <p>{{ tl('总实例') }} {{ runtimeOverview.instances_total }} {{ tl('· 运行中') }} {{ runtimeOverview.instances_running }}</p>
          <p>{{ tl('失败') }} {{ runtimeOverview.instances_failed }} {{ tl('· 30 分钟内到期') }} {{ runtimeOverview.instances_expiring_within_30m }}</p>
          <p>{{ tl('已过期未销毁') }} {{ runtimeOverview.instances_expired_not_destroyed }}</p>
        </article>
      </div>

      <h3>{{ tl('最近失败实例') }}</h3>
      <table v-if="runtimeOverview && runtimeOverview.recent_failed_instances.length > 0" class="scoreboard-table">
        <thead>
          <tr>
            <th>{{ tl('更新时间') }}</th>
            <th>{{ tl('比赛') }}</th>
            <th>{{ tl('队伍') }}</th>
            <th>{{ tl('题目') }}</th>
            <th>{{ tl('状态') }}</th>
            <th>{{ tl('到期') }}</th>
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
      <p v-else class="muted">{{ tl('暂无失败实例。') }}</p>
    </section>

    <section v-if="adminModule === 'operations' && operationsSubTab === 'alerts'" class="panel">
      <div class="row-between">
        <h2>{{ tl('运行告警') }}</h2>
        <div class="actions-row compact-actions">
          <button class="ghost" type="button" @click="loadRuntimeAlerts()" :disabled="loadingRuntimeAlerts">
            {{ loadingRuntimeAlerts ? tl('加载中...') : tl('刷新告警') }}
          </button>
          <button class="primary" type="button" @click="handleScanRuntimeAlerts" :disabled="runtimeAlertScanBusy">
            {{ runtimeAlertScanBusy ? tl('扫描中...') : tl('触发扫描') }}
          </button>
        </div>
      </div>

      <details class="filter-sheet">
        <summary>{{ tl('展开筛选条件') }}</summary>
        <div class="actions-row filter-sheet-body">
          <label>
            <span>{{ tl('状态') }}</span>
            <select v-model="runtimeAlertStatusFilter">
              <option value="">all</option>
              <option value="open">open</option>
              <option value="acknowledged">acknowledged</option>
              <option value="resolved">resolved</option>
            </select>
          </label>
          <label>
            <span>{{ tl('级别') }}</span>
            <select v-model="runtimeAlertSeverityFilter">
              <option value="">all</option>
              <option value="info">info</option>
              <option value="warning">warning</option>
              <option value="critical">critical</option>
            </select>
          </label>
          <label>
            <span>{{ tl('告警类型') }}</span>
            <input v-model.trim="runtimeAlertTypeFilter" />
          </label>
          <label>
            <span>{{ tl('条数') }}</span>
            <input v-model.number="runtimeAlertLimit" type="number" min="1" max="500" />
          </label>
          <button class="ghost" type="button" @click="loadRuntimeAlerts()" :disabled="loadingRuntimeAlerts">
            {{ tl('应用筛选') }}
          </button>
        </div>
      </details>

      <p v-if="runtimeAlertError" class="error">{{ runtimeAlertError }}</p>

      <div class="runtime-alert-workspace">
        <aside class="runtime-alert-list-panel">
          <div class="row-between">
            <h3>{{ tr("告警列表", "Alert stream") }}</h3>
            <span class="badge">{{ runtimeAlerts.length }}</span>
          </div>
          <div class="runtime-alert-list">
            <button
              v-for="item in runtimeAlerts"
              :key="item.id"
              class="runtime-alert-item"
              :class="[
                { active: selectedRuntimeAlertId === item.id },
                `severity-${item.severity}`,
                `status-${item.status}`
              ]"
              type="button"
              @click="selectRuntimeAlert(item.id)"
            >
              <div class="row-between">
                <strong class="runtime-alert-title">{{ item.title }}</strong>
                <span class="badge">{{ item.severity }}</span>
              </div>
              <p class="muted mono runtime-alert-line">{{ item.alert_type }}</p>
              <p class="muted runtime-alert-line">{{ tl('状态') }} {{ item.status }} {{ tl('· 最近') }} {{ formatTime(item.last_seen_at) }}</p>
            </button>
            <p v-if="runtimeAlerts.length === 0" class="muted">{{ tl('暂无运行告警。') }}</p>
          </div>
        </aside>

        <section class="runtime-alert-detail-panel">
          <template v-if="selectedRuntimeAlert">
            <div class="row-between">
              <h3>{{ selectedRuntimeAlert.title }}</h3>
              <span class="badge">{{ selectedRuntimeAlert.status }}</span>
            </div>
            <p class="runtime-alert-message">{{ selectedRuntimeAlert.message }}</p>
            <div class="runtime-alert-tags">
              <span class="badge">{{ selectedRuntimeAlert.severity }}</span>
              <span class="badge mono">{{ selectedRuntimeAlert.alert_type }}</span>
              <span class="badge mono">{{ selectedRuntimeAlert.source_type }}</span>
            </div>
            <div class="runtime-alert-meta">
              <p>{{ tl('首次发现：') }}{{ formatTime(selectedRuntimeAlert.first_seen_at) }}</p>
              <p>{{ tl('最近发现：') }}{{ formatTime(selectedRuntimeAlert.last_seen_at) }}</p>
              <p>{{ tl('确认人：') }}{{ selectedRuntimeAlert.acknowledged_by_username ?? "-" }}</p>
              <p>{{ tl('恢复人：') }}{{ selectedRuntimeAlert.resolved_by_username ?? "-" }}</p>
            </div>

            <label>
              <span>{{ tl('处理备注（可选）') }}</span>
              <input
                v-model.trim="runtimeAlertActionNote"
                :placeholder="tl('用于 ack / resolve 审计记录')"
              />
            </label>

            <details class="action-sheet">
              <summary>{{ tl('显示告警操作菜单') }}</summary>
              <div class="actions-row action-sheet-body">
                <button
                  class="ghost"
                  type="button"
                  @click="handleAcknowledgeRuntimeAlert(selectedRuntimeAlert)"
                  :disabled="
                    runtimeAlertUpdatingId === selectedRuntimeAlert.id ||
                    selectedRuntimeAlert.status !== 'open'
                  "
                >
                  {{ runtimeAlertUpdatingId === selectedRuntimeAlert.id ? tl('处理中...') : tl('确认告警') }}
                </button>
                <button
                  class="primary"
                  type="button"
                  @click="handleResolveRuntimeAlert(selectedRuntimeAlert)"
                  :disabled="
                    runtimeAlertUpdatingId === selectedRuntimeAlert.id ||
                    selectedRuntimeAlert.status === 'resolved'
                  "
                >
                  {{ runtimeAlertUpdatingId === selectedRuntimeAlert.id ? tl('处理中...') : tl('标记恢复') }}
                </button>
              </div>
            </details>

            <details class="runtime-alert-detail-json">
              <summary>{{ tl('展示详细信息（JSON）') }}</summary>
              <pre class="mono">{{ formatJson(selectedRuntimeAlert.detail) }}</pre>
            </details>
          </template>
          <p v-else class="muted">{{ tl('从左侧选择一个告警查看详情。') }}</p>
        </section>
      </div>
    </section>

    <section v-if="adminModule === 'users'" class="panel">
      <div class="row-between">
        <h2>{{ tl('用户管理') }}</h2>
        <button class="ghost" type="button" @click="loadUsers" :disabled="loadingUsers">
          {{ loadingUsers ? tl('加载中...') : tl('刷新用户') }}
        </button>
      </div>

      <details class="filter-sheet">
        <summary>{{ tl('展开筛选条件') }}</summary>
        <div class="actions-row filter-sheet-body">
          <label>
            <span>{{ tl('关键词') }}</span>
            <input v-model.trim="userKeyword" :placeholder="tl('用户名或邮箱')" />
          </label>
          <label>
            <span>{{ tl('角色') }}</span>
            <select v-model="userRoleFilter">
              <option value="">all</option>
              <option value="player">player</option>
              <option value="admin">admin</option>
              <option value="judge">judge</option>
            </select>
          </label>
          <label>
            <span>{{ tl('状态') }}</span>
            <select v-model="userStatusFilter">
              <option value="">all</option>
              <option value="active">active</option>
              <option value="disabled">disabled</option>
            </select>
          </label>
          <label>
            <span>{{ tl('条数') }}</span>
            <input v-model.number="userLimit" type="number" min="1" max="1000" />
          </label>
          <button class="ghost" type="button" @click="loadUsers" :disabled="loadingUsers">
            {{ tl('应用筛选') }}
          </button>
        </div>
      </details>

      <p v-if="userError" class="error">{{ userError }}</p>

      <table v-if="users.length > 0" class="scoreboard-table">
        <thead>
          <tr>
            <th>{{ tl('用户名') }}</th>
            <th>{{ tl('邮箱') }}</th>
            <th>{{ tl('角色') }}</th>
            <th>{{ tl('状态') }}</th>
            <th>{{ tl('创建时间') }}</th>
            <th>{{ tl('操作') }}</th>
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
              <button
                class="ghost"
                type="button"
                :disabled="
                  updatingUserId === item.id ||
                  resettingUserId === item.id ||
                  updatingUserRoleId === item.id ||
                  deletingUserAccountId === item.id
                "
                @click="selectedUserId = selectedUserId === item.id ? '' : item.id"
              >
                {{ selectedUserId === item.id ? tl('收起操作') : tl('管理该账号') }}
              </button>
              <div v-if="selectedUserId === item.id" class="actions-row user-actions-menu">
                <button
                  class="ghost"
                  type="button"
                  :disabled="
                    updatingUserId === item.id ||
                    resettingUserId === item.id ||
                    updatingUserRoleId === item.id ||
                    deletingUserAccountId === item.id
                  "
                  @click="toggleUserStatus(item)"
                >
                  {{ item.status === "active" ? tl('禁用') : tl('启用') }}
                </button>
                <select
                  v-model="roleDrafts[item.id]"
                  :disabled="
                    updatingUserId === item.id ||
                    resettingUserId === item.id ||
                    updatingUserRoleId === item.id ||
                    deletingUserAccountId === item.id
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
                    updatingUserRoleId === item.id ||
                    deletingUserAccountId === item.id
                  "
                  @click="handleUpdateUserRole(item)"
                >
                  {{ updatingUserRoleId === item.id ? tl('更新中...') : tl('更新角色') }}
                </button>
                <input
                  v-model="resetPasswords[item.id]"
                  type="password"
                  minlength="8"
                  :placeholder="tl('新密码(>=8)')"
                />
                <button
                  class="primary"
                  type="button"
                  :disabled="
                    updatingUserId === item.id ||
                    resettingUserId === item.id ||
                    updatingUserRoleId === item.id ||
                    deletingUserAccountId === item.id
                  "
                  @click="handleResetUserPassword(item)"
                >
                  {{ resettingUserId === item.id ? tl('重置中...') : tl('重置密码') }}
                </button>
                <button
                  class="danger"
                  type="button"
                  :disabled="
                    updatingUserId === item.id ||
                    resettingUserId === item.id ||
                    updatingUserRoleId === item.id ||
                    deletingUserAccountId === item.id
                  "
                  @click="handleDeleteUserAccount(item)"
                >
                  {{ deletingUserAccountId === item.id ? tl('删除中...') : tl('删除账号') }}
                </button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
      <p v-else class="muted">{{ tl('暂无用户记录。') }}</p>
    </section>

    <section v-if="adminModule === 'operations' && operationsSubTab === 'instances'" class="panel">
      <div class="row-between">
        <h2>{{ tl('实例监控') }}</h2>
        <div class="actions-row">
          <label class="inline-check instance-filter-control">
            <span>{{ tl('状态过滤') }}</span>
            <select v-model="instanceFilter" :disabled="loadingInstances" @change="loadInstances">
              <option value="">all</option>
              <option value="creating">creating</option>
              <option value="running">running</option>
              <option value="stopped">stopped</option>
              <option value="destroyed">destroyed</option>
              <option value="failed">failed</option>
            </select>
          </label>
          <button class="ghost" type="button" :disabled="loadingInstances" @click="loadInstances">
            {{ loadingInstances ? tl('加载中...') : tr("刷新实例", "Refresh instances") }}
          </button>
        </div>
      </div>

      <p v-if="instanceError" class="error">{{ instanceError }}</p>

      <div class="instance-workspace">
        <aside class="instance-list-panel">
          <div class="row-between">
            <h3>{{ tr("实例列表", "Instance list") }}</h3>
            <span class="badge">{{ instances.length }}</span>
          </div>
          <div v-if="instances.length > 0" class="instance-list-body stagger-list">
            <article
              v-for="item in instances"
              :key="item.id"
              class="instance-list-item"
              :class="{ active: selectedInstanceId === item.id }"
              @click="loadInstanceRuntimeMetrics(item.id)"
            >
              <div class="row-between instance-list-item-head">
                <strong>{{ item.team_name }}</strong>
                <span class="badge instance-status-badge" :class="instanceStatusClass(item.status)">
                  {{ item.status }}
                </span>
              </div>
              <p class="muted">{{ item.contest_title }}</p>
              <p class="muted">{{ item.challenge_title }}</p>
              <p class="muted mono">
                {{ item.subnet }} · {{ tr("到期", "Expires") }} {{ item.expires_at ? formatTime(item.expires_at) : "-" }}
              </p>
              <div class="actions-row compact-actions">
                <button
                  class="ghost"
                  type="button"
                  @click.stop="loadInstanceRuntimeMetrics(item.id)"
                  :disabled="loadingInstanceRuntimeMetricsId === item.id"
                >
                  {{
                    loadingInstanceRuntimeMetricsId === item.id
                      ? tl('加载中...')
                      : selectedInstanceId === item.id
                        ? tr("刷新指标", "Refresh metrics")
                        : tr("查看指标", "View metrics")
                  }}
                </button>
                <a
                  v-if="item.entrypoint_url"
                  class="instance-entry-link mono"
                  :href="item.entrypoint_url"
                  target="_blank"
                  rel="noreferrer noopener"
                  @click.stop
                >
                  {{ tr("入口", "Entry") }}
                </a>
              </div>
            </article>
          </div>
          <p v-else class="muted">{{ tl('暂无实例记录。') }}</p>
        </aside>

        <section class="instance-detail-panel">
          <template v-if="selectedInstanceRuntimeMetrics">
            <div class="row-between">
              <h3>{{ tl('实例指标：') }}{{ selectedInstance?.team_name ?? selectedInstanceRuntimeMetrics.instance.team_name }}</h3>
              <span class="badge instance-status-badge" :class="instanceStatusClass(selectedInstanceRuntimeMetrics.instance.status)">
                {{ selectedInstanceRuntimeMetrics.instance.status }}
              </span>
            </div>

            <p class="muted mono">
              project={{ selectedInstanceRuntimeMetrics.instance.compose_project_name }} ·
              {{ tr("更新于", "Updated") }} {{ formatTime(selectedInstanceRuntimeMetrics.generated_at) }}
            </p>

            <div class="instance-meta-grid">
              <article class="metric-card">
                <h4>{{ tr("实例信息", "Instance overview") }}</h4>
                <p>{{ tr("比赛", "Contest") }} {{ selectedInstanceRuntimeMetrics.instance.contest_title }}</p>
                <p>{{ tr("队伍", "Team") }} {{ selectedInstanceRuntimeMetrics.instance.team_name }}</p>
                <p>{{ tr("题目", "Challenge") }} {{ selectedInstanceRuntimeMetrics.instance.challenge_title }}</p>
                <p class="mono">{{ tr("子网", "Subnet") }} {{ selectedInstanceRuntimeMetrics.instance.subnet }}</p>
              </article>
              <article class="metric-card">
                <h4>{{ tr("时间与入口", "Timing & entrypoint") }}</h4>
                <p>{{ tr("创建", "Created") }} {{ formatTime(selectedInstanceRuntimeMetrics.instance.created_at) }}</p>
                <p>
                  {{ tr("到期", "Expires") }}
                  {{ selectedInstanceRuntimeMetrics.instance.expires_at ? formatTime(selectedInstanceRuntimeMetrics.instance.expires_at) : "-" }}
                </p>
                <p>
                  {{ tr("心跳", "Heartbeat") }}
                  {{ selectedInstanceRuntimeMetrics.instance.last_heartbeat_at ? formatTime(selectedInstanceRuntimeMetrics.instance.last_heartbeat_at) : "-" }}
                </p>
                <a
                  v-if="selectedInstanceRuntimeMetrics.instance.entrypoint_url"
                  class="instance-entry-link mono"
                  :href="selectedInstanceRuntimeMetrics.instance.entrypoint_url"
                  target="_blank"
                  rel="noreferrer noopener"
                >
                  {{ selectedInstanceRuntimeMetrics.instance.entrypoint_url }}
                </a>
              </article>
            </div>

            <div class="runtime-metrics">
              <article class="metric-card">
                <h4>{{ tl('服务状态') }}</h4>
                <p>{{ tl('总服务') }} {{ selectedInstanceRuntimeMetrics.summary.services_total }}</p>
                <p>{{ tl('运行中') }} {{ selectedInstanceRuntimeMetrics.summary.running_services }}</p>
                <p>{{ tl('不健康') }} {{ selectedInstanceRuntimeMetrics.summary.unhealthy_services }}</p>
              </article>
              <article class="metric-card">
                <h4>{{ tl('资源汇总') }}</h4>
                <p>{{ tl('CPU 总计') }} {{ formatPercentValue(selectedInstanceRuntimeMetrics.summary.cpu_percent_total) }}</p>
                <p>
                  {{ tl('内存') }} {{ formatResourceBytes(selectedInstanceRuntimeMetrics.summary.memory_usage_bytes_total) }} /
                  {{ formatResourceBytes(selectedInstanceRuntimeMetrics.summary.memory_limit_bytes_total) }}
                </p>
                <p>{{ tl('重启中服务') }} {{ selectedInstanceRuntimeMetrics.summary.restarting_services }}</p>
              </article>
            </div>

            <div v-if="selectedInstanceRuntimeMetrics.warnings.length > 0" class="instance-warnings">
              <p
                v-for="warning in selectedInstanceRuntimeMetrics.warnings"
                :key="warning"
                class="instance-warning mono"
              >
                warning: {{ warning }}
              </p>
            </div>

            <div class="table-wrap">
              <table v-if="selectedInstanceRuntimeMetrics.services.length > 0" class="scoreboard-table instance-services-table">
                <thead>
                  <tr>
                    <th>{{ tl('服务') }}</th>
                    <th>{{ tl('状态') }}</th>
                    <th>CPU</th>
                    <th>{{ tl('内存') }}</th>
                    <th>{{ tl('网络 RX/TX') }}</th>
                    <th>IP</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="service in selectedInstanceRuntimeMetrics.services" :key="service.container_id">
                    <td>
                      <strong>{{ service.service_name ?? service.container_name }}</strong>
                      <p class="muted mono">{{ service.container_name }}</p>
                    </td>
                    <td>
                      <span class="mono">{{ service.state ?? "-" }}</span>
                      <p class="muted">health={{ service.health_status ?? "-" }} · restart={{ service.restart_count ?? 0 }}</p>
                    </td>
                    <td>{{ formatPercentValue(service.cpu_percent) }}</td>
                    <td>
                      {{ formatResourceBytes(service.memory_usage_bytes) }} /
                      {{ formatResourceBytes(service.memory_limit_bytes) }}
                      <p class="muted">{{ formatPercentValue(service.memory_percent) }}</p>
                    </td>
                    <td>
                      {{ formatResourceBytes(service.net_rx_bytes) }} /
                      {{ formatResourceBytes(service.net_tx_bytes) }}
                    </td>
                    <td class="mono">{{ service.ip_addresses.join(", ") || "-" }}</td>
                  </tr>
                </tbody>
              </table>
              <p v-else class="muted">{{ tr("该实例暂无服务指标。", "No service metrics for this instance yet.") }}</p>
            </div>
          </template>
          <p v-else class="muted">{{ tr("从左侧选择一个实例查看运行指标。", "Select an instance from the list to view runtime metrics.") }}</p>
        </section>
      </div>
    </section>

    <section v-if="adminModule === 'audit'" class="panel">
      <div class="row-between">
        <h2>{{ tl('审计日志') }}</h2>
        <button class="ghost" type="button" @click="loadAuditLogs" :disabled="auditLoading">
          {{ auditLoading ? tl('加载中...') : tl('刷新日志') }}
        </button>
      </div>

      <details class="filter-sheet">
        <summary>{{ tl('展开筛选条件') }}</summary>
        <div class="actions-row filter-sheet-body">
          <label>
            <span>action</span>
            <input v-model.trim="auditActionFilter" />
          </label>
          <label>
            <span>target_type</span>
            <input v-model.trim="auditTargetTypeFilter" />
          </label>
          <label>
            <span>{{ tl('条数') }}</span>
            <input v-model.number="auditLimit" type="number" min="1" max="1000" />
          </label>
          <button class="ghost" type="button" @click="loadAuditLogs" :disabled="auditLoading">
            {{ tl('应用筛选') }}
          </button>
        </div>
      </details>

      <p v-if="auditError" class="error">{{ auditError }}</p>

      <table v-if="auditLogs.length > 0" class="scoreboard-table">
        <thead>
          <tr>
            <th>{{ tl('时间') }}</th>
            <th>{{ tl('操作人') }}</th>
            <th>{{ tl('角色') }}</th>
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
      <p v-else class="muted">{{ tl('暂无审计记录。') }}</p>
    </section>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, watch } from "vue";

import {
  ApiClientError,
  buildApiAssetUrl,
  createAdminChallenge,
  createAdminChallengeCategory,
  createAdminContestAnnouncement,
  createAdminContest,
  acknowledgeAdminRuntimeAlert,
  deleteAdminChallenge,
  deleteAdminChallengeCategory,
  deleteAdminChallengeAttachment,
  deleteAdminContest,
  deleteAdminContestAnnouncement,
  deleteAdminContestPoster,
  deleteAdminContestChallenge,
  deleteAdminUser,
  getAdminInstanceRuntimeMetrics,
  getAdminRuntimeOverview,
  getAdminChallengeDetail,
  getAdminSiteSettings,
  listAdminChallengeCategories,
  listAdminRuntimeAlerts,
  listAdminChallengeRuntimeTemplateLint,
  listAdminChallengeAttachments,
  listAdminChallengeVersions,
  listAdminContestAnnouncements,
  listAdminContestRegistrations,
  listAdminUsers,
  listAdminAuditLogs,
  listAdminChallenges,
  listAdminContestChallenges,
  listAdminContests,
  listAdminInstances,
  resetAdminUserPassword,
  resolveAdminRuntimeAlert,
  rollbackAdminChallengeVersion,
  runAdminExpiredInstanceReaper,
  runAdminStaleInstanceReaper,
  scanAdminRuntimeAlerts,
  streamAdminChallengeRuntimeImageTest,
  uploadAdminContestPoster,
  uploadAdminChallengeAttachment,
  updateAdminContestAnnouncement,
  updateAdminContestRegistration,
  updateAdminUserRole,
  updateAdminUserStatus,
  type AdminChallengeAttachmentItem,
  type AdminChallengeCategoryItem,
  type AdminChallengeDetailItem,
  type AdminAuditLogItem,
  type AdminChallengeItem,
  type AdminChallengeRuntimeImageTestStreamEvent,
  type AdminChallengeRuntimeImageTestResponse,
  type AdminChallengeRuntimeLintItem,
  type AdminChallengeRuntimeLintResponse,
  type AdminChallengeVersionItem,
  type AdminContestAnnouncementItem,
  type AdminContestChallengeItem,
  type AdminContestRegistrationItem,
  type AdminContestItem,
  type AdminInstanceItem,
  type AdminInstanceReaperRunResponse,
  type AdminInstanceRuntimeMetricsResponse,
  type AdminRuntimeAlertItem,
  type AdminRuntimeOverview,
  type AdminUserItem,
  updateAdminChallenge,
  updateAdminChallengeCategory,
  updateAdminContest,
  updateAdminContestStatus,
  updateAdminContestChallenge,
  upsertAdminContestChallenge
} from "../api/client";
import UploadField from "../components/UploadField.vue";
import { useL10n } from "../composables/useL10n";
import { renderMarkdownToHtml } from "../composables/useMarkdown";
import { useAuthStore } from "../stores/auth";
import { useUiStore } from "../stores/ui";

const authStore = useAuthStore();
const uiStore = useUiStore();
const { locale, tr } = useL10n();

const adminTextMap: Record<string, string> = {
  "· 24h 提交": "· 24h submissions",
  "· 30 分钟内到期": "· expiring within 30m",
  "· 版本 v": "· version v",
  "· 队伍": "· teams",
  "· 题目": "· challenges",
  "· 运行中": "· running",
  "· 状态": "· status",
  "· 最近": "· latest",
  "按标题、slug、分类筛选": "Filter by title, slug, category",
  "按标题、slug、状态筛选": "Filter by title, slug, status",
  "版本/附件": "Version/Attachment",
  "版本备注（可选）": "Version notes (optional)",
  "版本号必须是大于等于 1 的整数": "Version number must be an integer greater than or equal to 1.",
  "版本号非法": "Invalid version number",
  "版本与附件：": "Versions & Files:",
  "保存比赛失败": "Failed to save contest",
  "保存题目失败": "Failed to save challenge",
  "保存修改": "Save changes",
  "保存中...": "Saving...",
  "比赛": "Contests",
  "比赛公告管理": "Contest Announcement Management",
  "比赛管理": "Contest Management",
  "比赛题目挂载": "Contest Challenge Bindings",
  "比赛已创建": "Contest created",
  "比赛已更新": "Contest updated",
  "比赛已销毁": "Contest deleted",
  "比赛与提交": "Contests & Submissions",
  "比赛状态已更新": "Contest status updated",
  "编辑": "edit",
  "编辑比赛配置": "Edit match configuration",
  "标记恢复": "mark recovery",
  "标签（逗号分隔）": "Tags (comma separated)",
  "标题": "title",
  "标题或 slug": "title or slug",
  "不健康": "Unhealthy",
  "操作": "Actions",
  "操作人": "Actor",
  "草稿": "Draft",
  "测试镜像（拉取+构建探测）": "Test image (pull + build probe)",
  "测试中...": "Testing...",
  "查看指标": "View metrics",
  "场": "contests",
  "撤回发布": "Withdraw publication",
  "初始状态": "initial state",
  "处理备注（可选）": "Process notes (optional)",
  "处理中...": "Processing...",
  "触发扫描": "trigger scan",
  "触发运行告警扫描失败": "Failed to run runtime alert scan",
  "创建公告": "Create announcement",
  "创建公告失败": "Failed to create announcement",
  "创建时间": "creation time",
  "创建中...": "Creating...",
  "从左侧选择一个比赛查看详情与状态操作。": "Select a contest on the left to view details and status actions.",
  "从左侧选择一个告警查看详情。": "Select an alert on the left to view details.",
  "从左侧选择一个公告查看详情。": "Select an announcement on the left to view details.",
  "从左侧选择一个挂载题目查看详情。": "Select a bound challenge on the left to view details.",
  "错误": "Error",
  "当前比赛未挂载题目。": "No challenges are bound to this contest.",
  "当前未设置海报。": "No posters are currently set.",
  "到期": "Expires",
  "动态衰减参数": "Dynamic attenuation parameters",
  "队伍": "Team",
  "发布": "Publish",
  "发布时间（可选）": "Release time (optional)",
  "发布时间已清除": "Release time cleared",
  "访问模式": "access mode",
  "分类": "Category",
  "分值": "Points",
  "封榜时间（可选）": "Closing time (optional)",
  "服务": "Service",
  "服务状态": "Service status",
  "附件已删除": "Attachment deleted",
  "附件已上传": "Attachment uploaded",
  "该海报当前在比赛中心不可预览（仅 public 且 scheduled/running/ended 可见）。": "This poster is currently not visible in contest center (only when visibility is public and status is scheduled/running/ended).",
  "该题将在比赛内即时可见。": "This challenge is now immediately visible in contest.",
  "告警类型": "Alarm type",
  "告警已恢复": "Alert resolved",
  "告警已确认": "Alert acknowledged",
  "更新比赛状态失败": "Failed to update contest status",
  "更新公告失败": "Failed to update announcement",
  "更新角色": "Update role",
  "更新排序失败": "Failed to update sort",
  "更新时间": "Update time",
  "更新题目失败": "Failed to update challenge",
  "更新用户角色失败": "Failed to update user role",
  "更新用户状态失败": "Failed to update user status",
  "更新于": "updated on",
  "更新中...": "Updating...",
  "公告标题": "Announcement title",
  "公告列表": "Announcement list",
  "公告内容": "Announcement content",
  "公告内容不完整": "Announcement content incomplete",
  "公告已保存。": "Announcement saved.",
  "公告已撤回": "Announcement unpublished",
  "公告已创建": "Announcement created",
  "公告已发布": "Announcement published",
  "公告已更新": "Announcement updated",
  "公告已取消置顶": "Announcement unpinned",
  "公告已删除": "Announcement deleted",
  "公告已置顶": "Announcement pinned",
  "公告置顶状态已更新": "Pin status updated",
  "公告状态已更新": "Announcement status updated",
  "挂载/更新题目": "Bind/Update Challenge",
  "挂载成功": "Binding updated",
  "挂载或更新": "Mount or update",
  "挂载失败": "Binding failed",
  "挂载已移除": "Binding removed",
  "关键词": "keywords",
  "管理该账号": "Manage this account",
  "管理公告": "Manage announcements",
  "管理台已刷新": "Admin console refreshed",
  "管理题目挂载": "Manage challenge bindings",
  "过期实例回收已执行": "Expired instance reaper executed",
  "海报已删除": "Poster deleted",
  "海报已上传": "Poster uploaded",
  "恢复人：": "Restoration Person:",
  "恢复运行告警失败": "Failed to resolve runtime alert",
  "回滚版本号": "Rollback version number",
  "回滚备注（可选）": "Rollback notes (optional)",
  "回滚到该版本": "Roll back to this version",
  "回滚题目版本失败": "Failed to rollback challenge version",
  "回滚中...": "Rolling back...",
  "回收中...": "Recycling...",
  "积分模式": "Points mode",
  "基础规模": "Base metrics",
  "级别": "level",
  "加载比赛失败": "Failed to load contests",
  "加载到左侧表单": "Load to left form",
  "加载公告失败": "Failed to load announcements",
  "加载挂载失败": "Failed to load bindings",
  "加载模板校验结果失败": "Failed to load template lint results",
  "加载审计日志失败": "Failed to load audit logs",
  "加载实例失败": "Failed to load instances",
  "加载实例运行指标失败": "Failed to load instance metrics",
  "加载题目版本失败": "Failed to load challenge versions",
  "加载题目附件失败": "Failed to load challenge attachments",
  "加载题目失败": "Failed to load challenges",
  "加载题目详情失败": "Failed to load challenge details",
  "加载用户列表失败": "Failed to load users",
  "加载运行概览失败": "Failed to load runtime overview",
  "加载运行告警失败": "Failed to load runtime alerts",
  "加载中...": "loading...",
  "角色": "Role",
  "角色非法": "Invalid role",
  "角色未变化": "Role unchanged",
  "角色已更新": "Role updated",
  "结果": "result",
  "结束时间": "end time",
  "仅错误": "only errors",
  "禁用": "disabled",
  "镜像": "Image",
  "镜像仓库地址": "Image registry URL",
  "镜像测试失败": "Image test failed",
  "镜像测试通过": "Image test passed",
  "镜像为空": "Image is empty",
  "开始时间": "start time",
  "可见性": "Visibility",
  "可以继续挂载题目并调整状态。": "You can continue binding challenges and adjusting status.",
  "可以继续管理版本、附件或挂载到比赛。": "You can continue with versions, attachments, or contest bindings.",
  "快速筛选": "Quick filter",
  "立即发布": "Publish immediately",
  "没有匹配的比赛。": "No matching contests.",
  "没有匹配的题目。": "No matching challenges.",
  "密码过短": "Password too short",
  "密码已重置": "Password reset",
  "描述": "Description",
  "描述（可选）": "Description (optional)",
  "模板": "template",
  "难度": "difficulty",
  "内部端口": "internal port",
  "内存": "Memory",
  "内容": "content",
  "排序": "sort",
  "排序已更新": "Sort updated",
  "启用": "enabled",
  "清除发布时间": "Clear publishing time",
  "清除发布时间失败": "Failed to clear release time",
  "请先填写镜像仓库地址": "Please provide image registry address first.",
  "请先选择海报文件": "Please select a poster file first.",
  "请先选择要管理的题目": "Please select a challenge to manage first.",
  "请先选择一个附件文件": "Please select an attachment file first.",
  "请先在“赛事配置”中选择一个比赛。": "Please select a competition in \"Event Configuration\" first.",
  "请先在“题库配置”中选择一个题目，再切换到版本管理。": "Please select a question in \"Question Bank Configuration\" first, and then switch to version management.",
  "请先在中间列选择一个比赛。": "Please select a match in the middle column first.",
  "请选择题目": "Select a challenge",
  "取消编辑": "Cancel edit",
  "取消置顶": "Unpin",
  "确认告警": "Confirm alarm",
  "确认人：": "Confirmed by:",
  "确认运行告警失败": "Failed to acknowledge runtime alert",
  "入口协议": "Entry protocol",
  "赛事列表": "Contest list",
  "扫描中...": "Scanning...",
  "扫描总数": "Total number of scans",
  "删除比赛海报失败": "Failed to delete contest poster",
  "删除附件": "Delete attachment",
  "删除公告": "Delete announcement",
  "删除公告失败": "Failed to delete announcement",
  "删除海报": "Delete poster",
  "删除题目附件失败": "Failed to delete challenge attachment",
  "删除账号": "Delete account",
  "删除账号失败": "Failed to delete account",
  "删除中...": "Deleting...",
  "上传比赛海报": "Upload contest poster",
  "上传比赛海报失败": "Failed to upload contest poster",
  "上传附件": "Upload attachment",
  "上传海报": "Upload poster",
  "上传题目附件失败": "Failed to upload challenge attachment",
  "上传中...": "Uploading...",
  "上移": "move up",
  "审计日志": "Audit log",
  "失败": "Failed",
  "时间": "time",
  "实例监控": "Instance monitoring",
  "实例健康": "Instance health",
  "实例指标：": "Instance metrics:",
  "收起操作": "Collapse operation",
  "首次发现：": "First discovered:",
  "刷新告警": "Refresh alerts",
  "刷新日志": "Refresh logs",
  "刷新失败": "Refresh failed",
  "刷新校验": "Refresh lint",
  "刷新用户": "Refresh users",
  "刷新指标": "Refresh metrics",
  "题解可见策略": "Visible strategies for solving problems",
  "题解内容（可选）": "Solution content (optional)",
  "题库概览": "Challenge Library Overview",
  "题目": "Challenge",
  "题目已创建": "Challenge created",
  "题目已从当前比赛移除。": "Challenge removed from current contest.",
  "题目已更新": "Challenge updated",
  "题目已挂载/更新到当前比赛。": "Challenge bound/updated for the current contest.",
  "题目已回滚": "Challenge rolled back",
  "题目已销毁": "Challenge deleted",
  "题目状态已更新": "Challenge status updated",
  "题型": "Challenge type",
  "条数": "Number of items",
  "通过": "Passed",
  "网络 RX/TX": "Network RX/TX",
  "未登录或会话过期": "Not signed in or session expired",
  "未发布": "Unpublished",
  "未选择比赛": "No contest selected",
  "未选择海报": "No poster selected",
  "未选择题目": "No challenge selected",
  "未选择文件": "No file selected",
  "无法测试镜像": "Cannot test image",
  "下线": "offline",
  "下移": "move down",
  "显示比赛操作菜单": "Show contest actions",
  "显示告警操作菜单": "Show alert actions",
  "显示公告操作菜单": "Show announcement actions",
  "显示挂载操作菜单": "Show binding actions",
  "显示题目操作菜单": "Show challenge actions",
  "销毁": "destroy",
  "销毁比赛": "Destroy the game",
  "销毁比赛失败": "Failed to delete contest",
  "销毁题目失败": "Failed to delete challenge",
  "销毁中...": "Destroying...",
  "校验": "check",
  "心跳超时实例回收已执行": "Stale-heartbeat reaper executed",
  "新密码(>=8)": "New password(>=8)",
  "新密码至少需要 8 位字符": "New password must contain at least 8 characters.",
  "信息": "information",
  "选择并管理": "Select and manage",
  "选择题目": "Select topic",
  "移除挂载": "Remove mount",
  "移除挂载失败": "Failed to remove binding",
  "已挂载题目": "Questions have been mounted",
  "已过期未销毁": "Expired but not destroyed",
  "已选中": "selected",
  "已载入比赛配置": "Contest config loaded",
  "已载入题目配置": "Challenge config loaded",
  "应用筛选": "Apply filters",
  "用户": "user",
  "用户管理": "User management",
  "用户名": "username",
  "用户名或邮箱": "Username or email",
  "用户状态已更新": "User status updated",
  "用于 ack / resolve 审计记录": "for ack / resolve audit records",
  "邮箱": "Mail",
  "运行概览": "Runtime Overview",
  "运行告警": "Runtime Alerts",
  "运行告警：过期实例未销毁": "Runtime alert: expired instance not destroyed",
  "运行告警：实例即将到期": "Runtime alert: instance expiring soon",
  "运行告警：实例失败": "Runtime alert: instance failure",
  "运行告警扫描完成": "Runtime alert scan completed",
  "运行模板校验": "Runtime template lint",
  "运行模式": "Runtime mode",
  "运行中": "Running",
  "运行中比赛": "Running contests",
  "暂无版本记录。": "There is no version record yet.",
  "暂无附件。": "There are no attachments yet.",
  "暂无公告。": "No announcement yet.",
  "暂无匹配的模板校验记录。": "There is currently no matching template verification record.",
  "暂无审计记录。": "There is no audit record yet.",
  "暂无失败实例。": "There are no failed instances yet.",
  "暂无实例记录。": "There is no instance record yet.",
  "暂无用户记录。": "There is no user record yet.",
  "暂无运行告警。": "There are no operational warnings yet.",
  "展开筛选条件": "Expand filters",
  "展示详细信息（JSON）": "Display details (JSON)",
  "账号已删除": "Account deleted",
  "正在准备编辑器...": "Preparing editor...",
  "执行过期回收": "Execute expiration recycling",
  "执行过期实例回收失败": "Failed to run expired instance reaper",
  "执行回滚": "perform rollback",
  "执行心跳超时回收": "Execute heartbeat timeout recycling",
  "执行心跳超时实例回收失败": "Failed to run stale-heartbeat reaper",
  "置顶": "pin to top",
  "置顶公告": "Pinned announcement",
  "重启中服务": "Restarting service",
  "重置密码": "reset password",
  "重置密码失败": "Failed to reset password",
  "重置中...": "Resetting...",
  "状态": "state",
  "状态过滤": "Status filtering",
  "资源汇总": "Resource summary",
  "子网": "subnet",
  "总服务": "Total services",
  "总实例": "Total instances",
  "总提交": "total commits",
  "最近发现：": "Recently discovered:",
  "最近回收：mode=": "Recent recycling: mode=",
  "最近失败实例": "Recent failed instances",
  "最新题目、比赛、实例、审计和运行概览已同步。": "Challenges, contests, instances, audit logs, and runtime overview are synced.",
  "compose 模板（可选）": "compose template (optional)",
  "compose（多容器）": "compose (multiple containers)",
  "CPU 总计": "Total CPU",
  "direct（直连入口）": "direct (direct entrance)",
  "dynamic/internal 题型在 compose 模式下必须提供 compose 模板": "dynamic/internal challenges in compose mode require a compose template.",
  "flag 模式": "flag mode",
  "flag/哈希": "flag/hash",
  "single_image 模式必须填写镜像仓库地址": "single_image mode requires an image registry address.",
  "single_image 模式仅支持 dynamic 或 internal 题型": "single_image mode only supports dynamic/internal challenge types.",
  "single_image 模式内部端口必须在 1~65535": "single_image internal port must be between 1 and 65535.",
  "single_image（单镜像）": "single_image (single image)",
  "ssh_bastion（默认）": "ssh_bastion (default)"
};;;

function tl(text: string): string {
  if (locale.value === "zh") {
    return text;
  }
  const exact = adminTextMap[text];
  if (exact) {
    return exact;
  }
  const editingMatch = text.match(/^正在编辑：(.+)$/);
  if (editingMatch) {
    return `Editing: ${editingMatch[1]}`;
  }
  const statusMatch = text.match(/^当前状态：(.+)$/);
  if (statusMatch) {
    return `Current status: ${statusMatch[1]}`;
  }
  const roleSetMatch = text.match(/^(.+) 已设为 (.+)。$/);
  if (roleSetMatch) {
    return `${roleSetMatch[1]} set to ${roleSetMatch[2]}.`;
  }
  const roleSameMatch = text.match(/^(.+) 当前角色仍为 (.+)。$/);
  if (roleSameMatch) {
    return `${roleSameMatch[1]} remains ${roleSameMatch[2]}.`;
  }
  const passwordResetMatch = text.match(/^(.+) 的密码已更新。$/);
  if (passwordResetMatch) {
    return `${passwordResetMatch[1]}'s password has been updated.`;
  }
  const rolledBackMatch = text.match(/^已回滚到版本 v(.+)\。$/);
  if (rolledBackMatch) {
    return `Rolled back to version v${rolledBackMatch[1]}.`;
  }
  const sortMatch = text.match(/^新排序值：(.+)$/);
  if (sortMatch) {
    return `New sort value: ${sortMatch[1]}`;
  }
  const deleteChallengeConfirm = text.match(
    /^确认销毁题目「(.+)」？该操作会删除题目、挂载关系、提交记录与运行实例。$/
  );
  if (deleteChallengeConfirm) {
    return `Delete challenge "${deleteChallengeConfirm[1]}"? This will remove the challenge, bindings, submissions, and instances.`;
  }
  const deleteContestPosterConfirm = text.match(/^确认删除比赛「(.+)」的海报？$/);
  if (deleteContestPosterConfirm) {
    return `Delete poster for contest "${deleteContestPosterConfirm[1]}"?`;
  }
  const deleteContestConfirm = text.match(
    /^确认销毁比赛「(.+)」？该操作将删除比赛、公告、挂载、提交与实例数据。$/
  );
  if (deleteContestConfirm) {
    return `Delete contest "${deleteContestConfirm[1]}"? This will remove contest, announcements, bindings, submissions, and instances.`;
  }
  const deleteUserConfirm = text.match(
    /^确认删除账号「(.+)」？该操作会禁用并匿名化该账号。$/
  );
  if (deleteUserConfirm) {
    return `Delete account "${deleteUserConfirm[1]}"? The account will be disabled and anonymized.`;
  }
  const runtimeScanSummary = text.match(/^新增\/更新 (.+)，自动恢复 (.+)，open (.+)。$/);
  if (runtimeScanSummary) {
    return `Upserted ${runtimeScanSummary[1]}, auto-resolved ${runtimeScanSummary[2]}, open ${runtimeScanSummary[3]}.`;
  }
  const reaperExpiredSummary = text.match(/^扫描 (.+)，回收 (.+)，失败 (.+)。$/);
  if (reaperExpiredSummary) {
    return `Scanned ${reaperExpiredSummary[1]}, reaped ${reaperExpiredSummary[2]}, failed ${reaperExpiredSummary[3]}.`;
  }
  const reaperStaleSummary = text.match(/^阈值 (.+) 秒，扫描 (.+)，回收 (.+)。$/);
  if (reaperStaleSummary) {
    return `Threshold ${reaperStaleSummary[1]}s, scanned ${reaperStaleSummary[2]}, reaped ${reaperStaleSummary[3]}.`;
  }
  const expiringSummary = text.match(
    /^当前 (.+) 个实例将在 30 分钟内到期（新增 (.+) 个）。$/
  );
  if (expiringSummary) {
    return `${expiringSummary[1]} instances expire within 30 minutes (${expiringSummary[2]} newly increased).`;
  }
  const expiredSummary = text.match(/^当前 (.+) 个已过期实例未销毁（新增 (.+) 个）。$/);
  if (expiredSummary) {
    return `${expiredSummary[1]} expired instances are not destroyed (${expiredSummary[2]} newly increased).`;
  }
  return text;
}

const notify = {
  success(title: string, message: string, durationMs?: number) {
    uiStore.success(tl(title), tl(message), durationMs);
  },
  error(title: string, message: string, durationMs?: number) {
    uiStore.error(tl(title), tl(message), durationMs);
  },
  info(title: string, message: string, durationMs?: number) {
    uiStore.info(tl(title), tl(message), durationMs);
  },
  warning(title: string, message: string, durationMs?: number) {
    uiStore.warning(tl(title), tl(message), durationMs);
  }
};

const challenges = ref<AdminChallengeItem[]>([]);
const challengeCategories = ref<AdminChallengeCategoryItem[]>([]);
const challengeVersions = ref<AdminChallengeVersionItem[]>([]);
const challengeAttachments = ref<AdminChallengeAttachmentItem[]>([]);
const challengeRuntimeLint = ref<AdminChallengeRuntimeLintResponse | null>(null);
const contests = ref<AdminContestItem[]>([]);
const contestBindings = ref<AdminContestChallengeItem[]>([]);
const contestAnnouncements = ref<AdminContestAnnouncementItem[]>([]);
const contestRegistrations = ref<AdminContestRegistrationItem[]>([]);
const instances = ref<AdminInstanceItem[]>([]);
const selectedInstanceRuntimeMetrics = ref<AdminInstanceRuntimeMetricsResponse | null>(null);
const users = ref<AdminUserItem[]>([]);
const auditLogs = ref<AdminAuditLogItem[]>([]);
const runtimeOverview = ref<AdminRuntimeOverview | null>(null);
const runtimeAlerts = ref<AdminRuntimeAlertItem[]>([]);

const selectedContestId = ref("");
const selectedChallengeId = ref("");
const selectedBindingChallengeId = ref("");
const selectedAnnouncementId = ref("");
const selectedContestRegistrationId = ref("");
const selectedRuntimeAlertId = ref("");
const selectedInstanceId = ref("");
const selectedUserId = ref("");
const editingChallengeId = ref("");
const editingChallengeCategoryId = ref("");
const editingContestId = ref("");
const adminModule = ref<"challenges" | "contests" | "operations" | "users" | "audit">("challenges");
const challengeSubTab = ref<"library" | "versions" | "lint">("library");
const challengeLibraryMode = ref<"catalog" | "editor">("catalog");
const contestSubTab = ref<"contests" | "bindings" | "announcements" | "registrations">("contests");
const contestManageMode = ref<"catalog" | "editor">("catalog");
const operationsSubTab = ref<"runtime" | "alerts" | "instances">("runtime");

const pageError = ref("");
const challengeError = ref("");
const challengeCategoryError = ref("");
const challengeVersionError = ref("");
const challengeAttachmentError = ref("");
const challengeLintError = ref("");
const challengeImageTestError = ref("");
const contestError = ref("");
const bindingError = ref("");
const announcementError = ref("");
const contestRegistrationError = ref("");
const instanceError = ref("");
const userError = ref("");
const auditError = ref("");
const runtimeError = ref("");
const runtimeAlertError = ref("");
const runtimeReaperError = ref("");

const refreshing = ref(false);
const creatingChallenge = ref(false);
const savingChallengeCategory = ref(false);
const deletingChallengeCategoryId = ref("");
const creatingContest = ref(false);
const updatingChallengeId = ref("");
const destroyingChallengeId = ref("");
const rollingBack = ref(false);
const uploadingAttachment = ref(false);
const deletingAttachmentId = ref("");
const loadingChallengeRuntimeLint = ref(false);
const testingChallengeRuntimeImage = ref(false);
const loadingInstances = ref(false);
const updatingContestId = ref("");
const destroyingContestId = ref("");
const uploadingContestPoster = ref(false);
const deletingContestPosterId = ref("");
const bindingBusy = ref(false);
const creatingAnnouncement = ref(false);
const updatingAnnouncementId = ref("");
const deletingAnnouncementId = ref("");
const savingAnnouncementId = ref("");
const updatingContestRegistrationId = ref("");
const loadingUsers = ref(false);
const auditLoading = ref(false);
const loadingRuntimeAlerts = ref(false);
const updatingUserId = ref("");
const deletingUserAccountId = ref("");
const resettingUserId = ref("");
const updatingUserRoleId = ref("");
const runtimeAlertScanBusy = ref(false);
const runtimeAlertUpdatingId = ref("");
const runtimeReaperBusy = ref<"" | "expired" | "stale">("");
const loadingInstanceRuntimeMetricsId = ref("");

const instanceFilter = ref("");
const challengeKeyword = ref("");
const challengeLintTypeFilter = ref("");
const challengeLintStatusFilter = ref("");
const challengeLintKeywordFilter = ref("");
const challengeLintOnlyErrors = ref(false);
const challengeLintLimit = ref(500);
const contestKeyword = ref("");
const contestRegistrationStatusFilter = ref("");
const contestRegistrationLimit = ref(200);
const runtimeAlertStatusFilter = ref("");
const runtimeAlertSeverityFilter = ref("");
const runtimeAlertTypeFilter = ref("");
const runtimeAlertActionNote = ref("");
const runtimeAlertLimit = ref(100);
const userKeyword = ref("");
const userRoleFilter = ref("");
const userStatusFilter = ref("");
const userLimit = ref(150);
const auditActionFilter = ref("");
const auditTargetTypeFilter = ref("");
const auditLimit = ref(200);
const statusActions = ["draft", "scheduled", "running", "ended", "archived"];
const RUNTIME_POLL_INTERVAL_MS = 20_000;
const CHALLENGE_IMAGE_TEST_TIMEOUT_SECONDS = 300;
const CHALLENGE_IMAGE_TEST_MAX_STREAM_LINES = 1200;
const DEFAULT_CHALLENGE_ATTACHMENT_MAX_BYTES = 20 * 1024 * 1024;
type AnnouncementEditorMode = "edit" | "preview";
type MarkdownSnippetPreset = "heading" | "bold" | "italic" | "link" | "code" | "list" | "quote";
const resetPasswords = reactive<Record<string, string>>({});
const roleDrafts = reactive<Record<string, string>>({});
const announcementDrafts = reactive<Record<string, { title: string; content: string }>>({});
const announcementCreateMode = ref<AnnouncementEditorMode>("edit");
const announcementEditMode = ref<AnnouncementEditorMode>("edit");
const announcementCreateTextareaRef = ref<HTMLTextAreaElement | null>(null);
const announcementEditTextareaRef = ref<HTMLTextAreaElement | null>(null);
const attachmentInputKey = ref(0);
const selectedAttachmentFile = ref<File | null>(null);
const challengeAttachmentMaxBytes = ref(DEFAULT_CHALLENGE_ATTACHMENT_MAX_BYTES);
const contestPosterInputKey = ref(0);
const selectedContestPosterFile = ref<File | null>(null);
const challengeRuntimeImageTestResult = ref<AdminChallengeRuntimeImageTestResponse | null>(null);
const challengeRuntimeImageStreamLines = ref<string[]>([]);
const runtimeReaperResult = ref<AdminInstanceReaperRunResponse | null>(null);

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
  hints_input: "",
  writeup_visibility: "hidden",
  writeup_content: "",
  change_note: "",
  compose_template: "",
  runtime_mode: "none",
  runtime_access_mode: "ssh_bastion",
  runtime_image: "",
  runtime_internal_port: 80,
  runtime_protocol: "http",
});

const challengeCategoryDraft = reactive({
  slug: "",
  display_name: "",
  sort_order: 100
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

function renderAnnouncementMarkdown(markdown: string) {
  return renderMarkdownToHtml(markdown);
}

const markdownSnippetMap: Record<
  MarkdownSnippetPreset,
  { before: string; after: string; placeholder: string }
> = {
  heading: { before: "## ", after: "", placeholder: "小标题" },
  bold: { before: "**", after: "**", placeholder: "重点内容" },
  italic: { before: "*", after: "*", placeholder: "强调内容" },
  link: { before: "[", after: "](https://example.com)", placeholder: "链接文本" },
  code: { before: "`", after: "`", placeholder: "code" },
  list: { before: "- ", after: "", placeholder: "列表项" },
  quote: { before: "> ", after: "", placeholder: "引用内容" }
};

function getAnnouncementEditorContent(target: "create" | "edit"): string {
  if (target === "create") {
    return announcementForm.content;
  }
  return currentAnnouncementDraft.value?.content ?? "";
}

function setAnnouncementEditorContent(target: "create" | "edit", value: string): boolean {
  if (target === "create") {
    announcementForm.content = value;
    return true;
  }

  const item = selectedAnnouncement.value;
  if (!item || !announcementDrafts[item.id]) {
    return false;
  }

  announcementDrafts[item.id].content = value;
  return true;
}

async function insertMarkdownSnippet(target: "create" | "edit", preset: MarkdownSnippetPreset) {
  if (target === "edit" && !currentAnnouncementDraft.value) {
    return;
  }

  const snippet = markdownSnippetMap[preset];
  if (!snippet) {
    return;
  }

  if (target === "create" && announcementCreateMode.value !== "edit") {
    announcementCreateMode.value = "edit";
    await nextTick();
  }
  if (target === "edit" && announcementEditMode.value !== "edit") {
    announcementEditMode.value = "edit";
    await nextTick();
  }

  const textarea =
    target === "create" ? announcementCreateTextareaRef.value : announcementEditTextareaRef.value;
  const source = getAnnouncementEditorContent(target);

  if (!textarea) {
    const appendPrefix = source && !source.endsWith("\n") ? "\n" : "";
    void setAnnouncementEditorContent(
      target,
      `${source}${appendPrefix}${snippet.before}${snippet.placeholder}${snippet.after}`
    );
    return;
  }

  const start = textarea.selectionStart ?? source.length;
  const end = textarea.selectionEnd ?? source.length;
  const selectedText = source.slice(start, end) || snippet.placeholder;
  const nextValue =
    source.slice(0, start) + snippet.before + selectedText + snippet.after + source.slice(end);

  const updated = setAnnouncementEditorContent(target, nextValue);
  if (!updated) {
    return;
  }

  await nextTick();
  const nextCursorStart = start + snippet.before.length;
  const nextCursorEnd = nextCursorStart + selectedText.length;
  const activeTextarea =
    target === "create" ? announcementCreateTextareaRef.value : announcementEditTextareaRef.value;
  activeTextarea?.focus();
  activeTextarea?.setSelectionRange(nextCursorStart, nextCursorEnd);
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
  first_blood_bonus_percent: 10,
  second_blood_bonus_percent: 5,
  third_blood_bonus_percent: 2,
  registration_requires_approval: true,
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

const currentAnnouncementDraft = computed(() => {
  const item = selectedAnnouncement.value;
  if (!item) {
    return null;
  }
  return announcementDrafts[item.id] ?? null;
});

const currentAnnouncementDraftContent = computed(() => currentAnnouncementDraft.value?.content ?? "");

const selectedRuntimeAlert = computed(() => {
  return runtimeAlerts.value.find((item) => item.id === selectedRuntimeAlertId.value) ?? null;
});

const selectedInstance = computed(() => {
  return instances.value.find((item) => item.id === selectedInstanceId.value) ?? null;
});

const challengeLintItems = computed<AdminChallengeRuntimeLintItem[]>(() => {
  return challengeRuntimeLint.value?.items ?? [];
});

const sortedChallengeCategories = computed(() => {
  const rows = [...challengeCategories.value];
  rows.sort((a, b) => {
    if (a.sort_order !== b.sort_order) {
      return a.sort_order - b.sort_order;
    }
    return a.slug.localeCompare(b.slug);
  });
  return rows;
});

const challengeCategoryOptions = computed(() => {
  const options = [...sortedChallengeCategories.value];
  if (
    newChallenge.category &&
    !options.some((item) => item.slug === newChallenge.category)
  ) {
    options.push({
      id: `legacy-${newChallenge.category}`,
      slug: newChallenge.category,
      display_name: newChallenge.category,
      sort_order: 999_999,
      is_builtin: false,
      created_at: "",
      updated_at: ""
    });
  }
  return options;
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

const challengeFormTitle = computed(() => {
  return editingChallengeId.value ? tr("编辑题目", "Edit Challenge") : tr("创建题目", "Create Challenge");
});

const challengeSubmitLabel = computed(() => {
  if (creatingChallenge.value) {
    return editingChallengeId.value ? tr("保存中...", "Saving...") : tr("创建中...", "Creating...");
  }
  return editingChallengeId.value ? tr("保存修改", "Save changes") : tr("创建题目", "Create challenge");
});

const challengeTypeDescription = computed(() => {
  if (newChallenge.challenge_type === "dynamic") {
    return tr(
      "dynamic：每队可创建独立运行实例，适合隔离容器题。",
      "dynamic: each team can start an isolated runtime instance."
    );
  }
  if (newChallenge.challenge_type === "internal") {
    return tr(
      "internal：题目运行在内部/裁判控制场景，通常不直接暴露给选手公网。",
      "internal: runtime is intended for internal/judge-controlled scenarios."
    );
  }
  return tr(
    "static：无需运行实例，提交 flag 即可判定。",
    "static: no runtime instance is required; only flag verification is used."
  );
});

const flagModeDescription = computed(() => {
  if (newChallenge.flag_mode === "dynamic") {
    return tr(
      "dynamic：按队伍下发动态 flag（通常来自实例启动流程）。",
      "dynamic: per-team flag issued from runtime flow."
    );
  }
  if (newChallenge.flag_mode === "script") {
    return tr(
      "script：调用脚本判定，可用于复杂答案校验。",
      "script: verify via script for complex validation."
    );
  }
  return tr(
    "static：固定 flag 或哈希比对。",
    "static: compare against fixed flag/hash."
  );
});

const runtimeModeDescription = computed(() => {
  if (newChallenge.runtime_mode === "single_image") {
    return tr(
      "single_image：单镜像快速部署，适合轻量动态题。",
      "single_image: single-image runtime for lightweight dynamic challenges."
    );
  }
  if (newChallenge.runtime_mode === "compose") {
    return tr(
      "compose：多服务编排，支持复杂依赖场景。",
      "compose: multi-service orchestration for complex runtimes."
    );
  }
  return tr(
    "no_runtime：不启动容器，适用于纯静态题。",
    "no_runtime: no container runtime, suited for static challenges."
  );
});

const challengeRuntimeImageStreamOutput = computed(() => {
  return challengeRuntimeImageStreamLines.value.join("\n");
});

const challengeAttachmentUploadHint = computed(() => {
  return tr(
    `支持任意附件格式，大小不超过 ${formatSize(challengeAttachmentMaxBytes.value)}`,
    `Any file type is supported, up to ${formatSize(challengeAttachmentMaxBytes.value)}.`
  );
});

const contestFormTitle = computed(() => {
  return editingContestId.value ? tr("编辑比赛", "Edit Contest") : tr("创建比赛", "Create Contest");
});

const contestSubmitLabel = computed(() => {
  if (creatingContest.value) {
    return editingContestId.value ? tr("保存中...", "Saving...") : tr("创建中...", "Creating...");
  }
  return editingContestId.value ? tr("保存修改", "Save changes") : tr("创建比赛", "Create contest");
});

function formatTime(input: string) {
  const localeTag = locale.value === "en" ? "en-US" : "zh-CN";
  return new Date(input).toLocaleString(localeTag);
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

function formatJson(value: unknown) {
  if (value === null || value === undefined) {
    return "{}";
  }

  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return "{}";
  }
}

function parseTagsInput(raw: string): string[] {
  return raw
    .split(/[,，\n]/g)
    .map((item) => item.trim())
    .filter((item) => item.length > 0);
}

function parseHintsInput(raw: string): string[] {
  return raw
    .split(/\r?\n/g)
    .map((item) => item.trim())
    .filter((item) => item.length > 0);
}

function appendChallengeRuntimeImageStreamLine(line: string) {
  const normalized = line.replace(/\r/g, "");
  challengeRuntimeImageStreamLines.value.push(normalized);
  if (challengeRuntimeImageStreamLines.value.length > CHALLENGE_IMAGE_TEST_MAX_STREAM_LINES) {
    challengeRuntimeImageStreamLines.value.splice(
      0,
      challengeRuntimeImageStreamLines.value.length - CHALLENGE_IMAGE_TEST_MAX_STREAM_LINES
    );
  }
}

function resetChallengeForm() {
  editingChallengeId.value = "";
  newChallenge.title = "";
  newChallenge.slug = "";
  newChallenge.category = sortedChallengeCategories.value[0]?.slug ?? "web";
  newChallenge.description = "";
  newChallenge.difficulty = "normal";
  newChallenge.static_score = 100;
  newChallenge.status = "draft";
  newChallenge.challenge_type = "static";
  newChallenge.flag_mode = "static";
  newChallenge.flag_hash = "";
  newChallenge.tags_input = "";
  newChallenge.hints_input = "";
  newChallenge.writeup_visibility = "hidden";
  newChallenge.writeup_content = "";
  newChallenge.change_note = "";
  newChallenge.compose_template = "";
  newChallenge.runtime_mode = "none";
  newChallenge.runtime_access_mode = "ssh_bastion";
  newChallenge.runtime_image = "";
  newChallenge.runtime_internal_port = 80;
  newChallenge.runtime_protocol = "http";
  challengeImageTestError.value = "";
  challengeRuntimeImageTestResult.value = null;
  challengeRuntimeImageStreamLines.value = [];
}

function resetContestForm() {
  editingContestId.value = "";
  newContest.title = "";
  newContest.slug = "";
  newContest.description = "";
  newContest.visibility = "public";
  newContest.status = "draft";
  newContest.scoring_mode = "static";
  newContest.dynamic_decay = 20;
  newContest.first_blood_bonus_percent = 10;
  newContest.second_blood_bonus_percent = 5;
  newContest.third_blood_bonus_percent = 2;
  newContest.registration_requires_approval = true;
  newContest.start_at = defaultStart;
  newContest.end_at = defaultEnd;
  newContest.freeze_at = "";
}

function applyChallengeDetailToForm(detail: AdminChallengeDetailItem) {
  const runtime = (detail.metadata?.runtime ?? {}) as Record<string, unknown>;
  const runtimeModeRaw = typeof runtime.mode === "string" ? runtime.mode.trim().toLowerCase() : "";
  const hasRuntimeMetadata =
    Object.keys(runtime).length > 0 ||
    typeof runtime.mode === "string" ||
    typeof runtime.image === "string" ||
    typeof runtime.internal_port === "number" ||
    typeof runtime.access_mode === "string";
  const hasComposeTemplate = !!detail.compose_template?.trim();
  const runtimeMode: "none" | "compose" | "single_image" =
    runtimeModeRaw === "single_image" || runtimeModeRaw === "single-image" || runtimeModeRaw === "image"
      ? "single_image"
      : runtimeModeRaw === "compose" || runtimeModeRaw === "compose_template" || hasRuntimeMetadata || hasComposeTemplate
        ? "compose"
        : detail.challenge_type === "static"
          ? "none"
          : "compose";
  const accessModeRaw =
    typeof runtime.access_mode === "string" ? runtime.access_mode.trim().toLowerCase() : "";
  const runtimeAccessMode =
    accessModeRaw === "wireguard"
      ? "wireguard"
      : accessModeRaw === "direct"
        ? "direct"
        : "ssh_bastion";
  const runtimeProtocolRaw = typeof runtime.protocol === "string" ? runtime.protocol.trim().toLowerCase() : "http";
  const runtimeProtocol =
    runtimeProtocolRaw === "https" || runtimeProtocolRaw === "tcp" ? runtimeProtocolRaw : "http";
  const runtimeInternalPortRaw =
    typeof runtime.internal_port === "number" ? runtime.internal_port : Number(runtime.internal_port ?? 80);
  const runtimeInternalPort =
    Number.isFinite(runtimeInternalPortRaw) && runtimeInternalPortRaw >= 1 && runtimeInternalPortRaw <= 65535
      ? Math.floor(runtimeInternalPortRaw)
      : 80;

  editingChallengeId.value = detail.id;
  newChallenge.title = detail.title;
  newChallenge.slug = detail.slug;
  newChallenge.category = detail.category;
  newChallenge.description = detail.description ?? "";
  newChallenge.difficulty = detail.difficulty;
  newChallenge.static_score = detail.static_score;
  newChallenge.status = detail.status;
  newChallenge.challenge_type = detail.challenge_type;
  newChallenge.flag_mode = detail.flag_mode;
  newChallenge.flag_hash = detail.flag_hash ?? "";
  newChallenge.compose_template = detail.compose_template ?? "";
  newChallenge.tags_input = (detail.tags ?? []).join(", ");
  newChallenge.hints_input = (detail.hints ?? []).join("\n");
  newChallenge.writeup_visibility = detail.writeup_visibility;
  newChallenge.writeup_content = detail.writeup_content ?? "";
  newChallenge.change_note = "";
  newChallenge.runtime_mode = runtimeMode;
  newChallenge.runtime_access_mode = runtimeAccessMode;
  newChallenge.runtime_image = typeof runtime.image === "string" ? runtime.image : "";
  newChallenge.runtime_internal_port = runtimeInternalPort;
  newChallenge.runtime_protocol = runtimeProtocol;
  challengeRuntimeImageStreamLines.value = [];
}

function applyContestToForm(item: AdminContestItem) {
  editingContestId.value = item.id;
  newContest.title = item.title;
  newContest.slug = item.slug;
  newContest.description = item.description ?? "";
  newContest.visibility = item.visibility;
  newContest.status = item.status;
  newContest.scoring_mode = item.scoring_mode;
  newContest.dynamic_decay = item.dynamic_decay;
  newContest.first_blood_bonus_percent = item.first_blood_bonus_percent;
  newContest.second_blood_bonus_percent = item.second_blood_bonus_percent;
  newContest.third_blood_bonus_percent = item.third_blood_bonus_percent;
  newContest.registration_requires_approval = item.registration_requires_approval;
  newContest.start_at = isoToLocalInput(item.start_at);
  newContest.end_at = isoToLocalInput(item.end_at);
  newContest.freeze_at = item.freeze_at ? isoToLocalInput(item.freeze_at) : "";
}

function buildChallengeRuntimeMetadata() {
  if (newChallenge.runtime_mode === "none") {
    return {};
  }

  const runtime: Record<string, unknown> = {
    mode: newChallenge.runtime_mode
  };

  if (newChallenge.runtime_mode === "single_image") {
    runtime.image = newChallenge.runtime_image.trim();
    runtime.internal_port = Number(newChallenge.runtime_internal_port);
    runtime.protocol = newChallenge.runtime_protocol;
    runtime.access_mode = "direct";
  } else {
    runtime.access_mode = newChallenge.runtime_access_mode;
  }

  return { runtime };
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

function formatResourceBytes(sizeBytes: number | null | undefined): string {
  if (sizeBytes == null || !Number.isFinite(sizeBytes)) {
    return "-";
  }
  if (sizeBytes < 1024) {
    return `${sizeBytes} B`;
  }
  if (sizeBytes < 1024 * 1024) {
    return `${(sizeBytes / 1024).toFixed(1)} KB`;
  }
  if (sizeBytes < 1024 * 1024 * 1024) {
    return `${(sizeBytes / (1024 * 1024)).toFixed(1)} MB`;
  }
  return `${(sizeBytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function formatPercentValue(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) {
    return "-";
  }
  return `${value.toFixed(2)}%`;
}

function instanceStatusClass(status: string): string {
  const normalized = status.trim().toLowerCase();
  if (normalized === "running") {
    return "status-running";
  }
  if (normalized === "creating") {
    return "status-creating";
  }
  if (normalized === "failed") {
    return "status-failed";
  }
  if (normalized === "stopped") {
    return "status-stopped";
  }
  if (normalized === "destroyed") {
    return "status-destroyed";
  }
  return "status-default";
}

function canPreviewContestPoster(item: AdminContestItem) {
  if (!item.poster_url) {
    return false;
  }

  if (item.visibility !== "public") {
    return false;
  }

  return item.status === "scheduled" || item.status === "running" || item.status === "ended";
}

function contestPosterPreviewUrl(item: AdminContestItem) {
  if (!item.poster_url) {
    return "";
  }

  const url = new URL(buildApiAssetUrl(item.poster_url));
  url.searchParams.set("v", item.updated_at);
  return url.toString();
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
    throw new ApiClientError(tl("未登录或会话过期"), "unauthorized");
  }
  return authStore.accessToken;
}

function resetChallengeCategoryDraft() {
  editingChallengeCategoryId.value = "";
  challengeCategoryDraft.slug = "";
  challengeCategoryDraft.display_name = "";
  challengeCategoryDraft.sort_order = 100;
}

function loadChallengeCategoryIntoDraft(item: AdminChallengeCategoryItem) {
  editingChallengeCategoryId.value = item.id;
  challengeCategoryDraft.slug = item.slug;
  challengeCategoryDraft.display_name = item.display_name;
  challengeCategoryDraft.sort_order = item.sort_order;
  challengeCategoryError.value = "";
}

async function loadChallengeCategories(options?: { silentError?: boolean }) {
  challengeCategoryError.value = "";
  try {
    challengeCategories.value = await listAdminChallengeCategories(accessTokenOrThrow());
    if (
      challengeCategories.value.length > 0 &&
      !challengeCategories.value.some((item) => item.slug === newChallenge.category)
    ) {
      newChallenge.category = sortedChallengeCategories.value[0].slug;
    }
  } catch (err) {
    challengeCategoryError.value =
      err instanceof ApiClientError ? err.message : tr("加载题目类别失败", "Failed to load challenge categories");
    if (!options?.silentError) {
      uiStore.error(tr("加载题目类别失败", "Failed to load challenge categories"), challengeCategoryError.value);
    }
  }
}

async function handleSaveChallengeCategory() {
  const slug = challengeCategoryDraft.slug.trim().toLowerCase();
  const displayName = challengeCategoryDraft.display_name.trim();
  const sortOrder = Number.isFinite(challengeCategoryDraft.sort_order)
    ? Math.floor(challengeCategoryDraft.sort_order)
    : 100;
  const previousSlug =
    editingChallengeCategoryId.value
      ? challengeCategories.value.find((item) => item.id === editingChallengeCategoryId.value)
          ?.slug ?? ""
      : "";
  if (!slug) {
    challengeCategoryError.value = tr("请填写类别 slug。", "Please provide a category slug.");
    uiStore.warning(tr("类别保存失败", "Category save failed"), challengeCategoryError.value);
    return;
  }

  savingChallengeCategory.value = true;
  challengeCategoryError.value = "";

  try {
    if (editingChallengeCategoryId.value) {
      const updated = await updateAdminChallengeCategory(
        editingChallengeCategoryId.value,
        {
          slug,
          display_name: displayName || undefined,
          sort_order: sortOrder
        },
        accessTokenOrThrow()
      );
      if (previousSlug && newChallenge.category === previousSlug) {
        newChallenge.category = updated.slug;
      }
      uiStore.success(tr("类别已更新", "Category updated"), updated.slug, 2000);
    } else {
      const created = await createAdminChallengeCategory(
        {
          slug,
          display_name: displayName || undefined,
          sort_order: sortOrder
        },
        accessTokenOrThrow()
      );
      newChallenge.category = created.slug;
      uiStore.success(tr("类别已创建", "Category created"), created.slug, 2000);
    }
    resetChallengeCategoryDraft();
    await loadChallengeCategories({ silentError: true });
  } catch (err) {
    challengeCategoryError.value =
      err instanceof ApiClientError ? err.message : tr("保存题目类别失败", "Failed to save challenge category");
    uiStore.error(tr("保存题目类别失败", "Failed to save challenge category"), challengeCategoryError.value);
  } finally {
    savingChallengeCategory.value = false;
  }
}

async function handleDeleteChallengeCategory(item: AdminChallengeCategoryItem) {
  const confirmed = await uiStore.confirm({
    title: tr("删除题目类别", "Delete challenge category"),
    message: tr(
      `确认删除题目类别「${item.display_name}」？`,
      `Delete challenge category "${item.display_name}"?`
    ),
    confirmLabel: tr("删除", "Delete"),
    cancelLabel: tr("取消", "Cancel"),
    intent: "danger"
  });
  if (!confirmed) {
    return;
  }

  deletingChallengeCategoryId.value = item.id;
  challengeCategoryError.value = "";

  try {
    await deleteAdminChallengeCategory(item.id, accessTokenOrThrow());
    if (newChallenge.category === item.slug) {
      const fallback = sortedChallengeCategories.value.find((category) => category.slug !== item.slug);
      newChallenge.category = fallback?.slug ?? "";
    }
    if (editingChallengeCategoryId.value === item.id) {
      resetChallengeCategoryDraft();
    }
    await loadChallengeCategories({ silentError: true });
    uiStore.warning(tr("类别已删除", "Category removed"), item.slug, 2000);
  } catch (err) {
    challengeCategoryError.value =
      err instanceof ApiClientError ? err.message : tr("删除题目类别失败", "Failed to delete challenge category");
    uiStore.error(tr("删除题目类别失败", "Failed to delete challenge category"), challengeCategoryError.value);
  } finally {
    deletingChallengeCategoryId.value = "";
  }
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
    notify.error(
      "运行告警：实例失败",
      `${item.contest_title} / ${item.team_name} / ${item.challenge_title}（${item.status}）`,
      6500
    );
  }
  shrinkFailureAlertCache(2000);

  if (overview.instances_expiring_within_30m > lastExpiringWithin30mCount.value) {
    const increased = overview.instances_expiring_within_30m - lastExpiringWithin30mCount.value;
    notify.warning(
      "运行告警：实例即将到期",
      `当前 ${overview.instances_expiring_within_30m} 个实例将在 30 分钟内到期（新增 ${increased} 个）。`,
      5000
    );
  }

  if (overview.instances_expired_not_destroyed > lastExpiredNotDestroyedCount.value) {
    const increased =
      overview.instances_expired_not_destroyed - lastExpiredNotDestroyedCount.value;
    notify.warning(
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
    challengeError.value = err instanceof ApiClientError ? err.message : tl("加载题目失败");
    notify.error("加载题目失败", challengeError.value);
  }
}

async function loadChallengeAttachmentLimit(options?: { silentError?: boolean }) {
  try {
    const settings = await getAdminSiteSettings(accessTokenOrThrow());
    const value = Number(settings.challenge_attachment_max_bytes);
    if (Number.isFinite(value) && value > 0) {
      challengeAttachmentMaxBytes.value = Math.floor(value);
    }
  } catch (err) {
    if (!options?.silentError) {
      const message = err instanceof ApiClientError ? err.message : tr("加载失败", "Load failed");
      notify.error(tr("加载站点设置失败", "Failed to load site settings"), message);
    }
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
    challengeVersionError.value = err instanceof ApiClientError ? err.message : tl("加载题目版本失败");
    notify.error("加载题目版本失败", challengeVersionError.value);
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
      err instanceof ApiClientError ? err.message : tl("加载题目附件失败");
    notify.error("加载题目附件失败", challengeAttachmentError.value);
  }
}

async function loadChallengeRuntimeLint(options?: { silentError?: boolean }) {
  loadingChallengeRuntimeLint.value = true;
  challengeLintError.value = "";

  try {
    challengeRuntimeLint.value = await listAdminChallengeRuntimeTemplateLint(
      accessTokenOrThrow(),
      {
        limit: Number.isFinite(challengeLintLimit.value)
          ? Math.max(1, Math.min(5000, challengeLintLimit.value))
          : 500,
        challenge_type: challengeLintTypeFilter.value || undefined,
        status: challengeLintStatusFilter.value || undefined,
        keyword: challengeLintKeywordFilter.value || undefined,
        only_errors: challengeLintOnlyErrors.value
      }
    );
  } catch (err) {
    challengeLintError.value = err instanceof ApiClientError ? err.message : tl("加载模板校验结果失败");
    if (!options?.silentError) {
      notify.error("加载模板校验结果失败", challengeLintError.value);
    }
  } finally {
    loadingChallengeRuntimeLint.value = false;
  }
}

async function loadContests() {
  contestError.value = "";
  try {
    contests.value = await listAdminContests(accessTokenOrThrow());
    if (
      selectedContestId.value &&
      !contests.value.some((item) => item.id === selectedContestId.value)
    ) {
      selectedContestId.value = "";
    }
    if (!selectedContestId.value && contests.value.length > 0) {
      selectedContestId.value = contests.value[0].id;
    }
  } catch (err) {
    contestError.value = err instanceof ApiClientError ? err.message : tl("加载比赛失败");
    notify.error("加载比赛失败", contestError.value);
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
    bindingError.value = err instanceof ApiClientError ? err.message : tl("加载挂载失败");
    notify.error("加载挂载失败", bindingError.value);
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
    announcementError.value = err instanceof ApiClientError ? err.message : tl("加载公告失败");
    notify.error("加载公告失败", announcementError.value);
  }
}

async function loadContestRegistrations(options?: { silentError?: boolean }) {
  contestRegistrationError.value = "";

  if (!selectedContestId.value) {
    contestRegistrations.value = [];
    selectedContestRegistrationId.value = "";
    return;
  }

  try {
    const rows = await listAdminContestRegistrations(
      selectedContestId.value,
      accessTokenOrThrow(),
      {
        status: contestRegistrationStatusFilter.value || undefined,
        limit: Number.isFinite(contestRegistrationLimit.value)
          ? Math.max(1, Math.min(1000, contestRegistrationLimit.value))
          : 200
      }
    );
    contestRegistrations.value = rows;

    if (rows.length === 0) {
      selectedContestRegistrationId.value = "";
      return;
    }

    if (!rows.some((item) => item.id === selectedContestRegistrationId.value)) {
      selectedContestRegistrationId.value = rows[0].id;
    }
  } catch (err) {
    contestRegistrationError.value =
      err instanceof ApiClientError ? err.message : tr("加载报名记录失败", "Failed to load contest registrations");
    if (!options?.silentError) {
      notify.error("加载报名记录失败", contestRegistrationError.value);
    }
  }
}

async function loadInstances() {
  loadingInstances.value = true;
  instanceError.value = "";
  try {
    instances.value = await listAdminInstances(accessTokenOrThrow(), {
      status: instanceFilter.value || undefined,
      limit: 150
    });

    if (
      selectedInstanceId.value &&
      !instances.value.some((item) => item.id === selectedInstanceId.value)
    ) {
      selectedInstanceId.value = "";
      selectedInstanceRuntimeMetrics.value = null;
    }

    if (
      selectedInstanceId.value &&
      instances.value.some((item) => item.id === selectedInstanceId.value)
    ) {
      await loadInstanceRuntimeMetrics(selectedInstanceId.value, { silentError: true });
    } else if (!selectedInstanceId.value && instances.value.length > 0) {
      await loadInstanceRuntimeMetrics(instances.value[0].id, { silentError: true });
    }
  } catch (err) {
    instanceError.value = err instanceof ApiClientError ? err.message : tl("加载实例失败");
    notify.error("加载实例失败", instanceError.value);
  } finally {
    loadingInstances.value = false;
  }
}

async function loadInstanceRuntimeMetrics(
  instanceId: string,
  options?: { silentError?: boolean }
) {
  loadingInstanceRuntimeMetricsId.value = instanceId;
  selectedInstanceId.value = instanceId;
  instanceError.value = "";

  try {
    selectedInstanceRuntimeMetrics.value = await getAdminInstanceRuntimeMetrics(
      instanceId,
      accessTokenOrThrow()
    );
  } catch (err) {
    const message = err instanceof ApiClientError ? err.message : tl("加载实例运行指标失败");
    if (!options?.silentError) {
      instanceError.value = message;
      notify.error("加载实例运行指标失败", message);
    }
  } finally {
    loadingInstanceRuntimeMetricsId.value = "";
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

    if (selectedUserId.value && !users.value.some((item) => item.id === selectedUserId.value)) {
      selectedUserId.value = "";
    }
  } catch (err) {
    userError.value = err instanceof ApiClientError ? err.message : tl("加载用户列表失败");
    notify.error("加载用户列表失败", userError.value);
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
    runtimeError.value = err instanceof ApiClientError ? err.message : tl("加载运行概览失败");
    if (!options?.silentError) {
      notify.error("加载运行概览失败", runtimeError.value);
    }
  }
}

async function loadRuntimeAlerts(options?: { silentError?: boolean; keepSelection?: boolean }) {
  loadingRuntimeAlerts.value = true;
  runtimeAlertError.value = "";

  try {
    const rows = await listAdminRuntimeAlerts(accessTokenOrThrow(), {
      status: runtimeAlertStatusFilter.value || undefined,
      severity: runtimeAlertSeverityFilter.value || undefined,
      alert_type: runtimeAlertTypeFilter.value || undefined,
      limit: Number.isFinite(runtimeAlertLimit.value)
        ? Math.max(1, Math.min(500, runtimeAlertLimit.value))
        : 100
    });
    runtimeAlerts.value = rows;

    if (rows.length === 0) {
      selectedRuntimeAlertId.value = "";
      return;
    }

    if (
      options?.keepSelection &&
      selectedRuntimeAlertId.value &&
      rows.some((item) => item.id === selectedRuntimeAlertId.value)
    ) {
      return;
    }

    selectedRuntimeAlertId.value = rows[0].id;
  } catch (err) {
    runtimeAlertError.value = err instanceof ApiClientError ? err.message : tl("加载运行告警失败");
    if (!options?.silentError) {
      notify.error("加载运行告警失败", runtimeAlertError.value);
    }
  } finally {
    loadingRuntimeAlerts.value = false;
  }
}

function selectRuntimeAlert(alertId: string) {
  selectedRuntimeAlertId.value = alertId;
}

function runtimeAlertNotePayload() {
  const note = runtimeAlertActionNote.value.trim();
  return note ? { note } : undefined;
}

async function handleScanRuntimeAlerts() {
  runtimeAlertScanBusy.value = true;
  runtimeAlertError.value = "";

  try {
    const summary = await scanAdminRuntimeAlerts(accessTokenOrThrow());
    await Promise.all([
      loadRuntimeAlerts({ silentError: true, keepSelection: true }),
      loadRuntimeOverview({ silentError: true })
    ]);
    notify.success(
      "运行告警扫描完成",
      `新增/更新 ${summary.upserted}，自动恢复 ${summary.auto_resolved}，open ${summary.open_count}。`,
      3200
    );
  } catch (err) {
    runtimeAlertError.value = err instanceof ApiClientError ? err.message : tl("触发运行告警扫描失败");
    notify.error("触发运行告警扫描失败", runtimeAlertError.value);
  } finally {
    runtimeAlertScanBusy.value = false;
  }
}

async function handleRunExpiredReaper() {
  runtimeReaperBusy.value = "expired";
  runtimeReaperError.value = "";

  try {
    const result = await runAdminExpiredInstanceReaper(accessTokenOrThrow());
    runtimeReaperResult.value = result;
    await Promise.all([
      loadInstances(),
      loadRuntimeOverview({ silentError: true }),
      loadRuntimeAlerts({ silentError: true, keepSelection: true })
    ]);
    notify.success(
      "过期实例回收已执行",
      `扫描 ${result.scanned}，回收 ${result.reaped}，失败 ${result.failed}。`,
      3600
    );
  } catch (err) {
    runtimeReaperError.value =
      err instanceof ApiClientError ? err.message : tl("执行过期实例回收失败");
    notify.error("执行过期实例回收失败", runtimeReaperError.value);
  } finally {
    runtimeReaperBusy.value = "";
  }
}

async function handleRunStaleReaper() {
  runtimeReaperBusy.value = "stale";
  runtimeReaperError.value = "";

  try {
    const result = await runAdminStaleInstanceReaper(accessTokenOrThrow());
    runtimeReaperResult.value = result;
    await Promise.all([
      loadInstances(),
      loadRuntimeOverview({ silentError: true }),
      loadRuntimeAlerts({ silentError: true, keepSelection: true })
    ]);
    notify.success(
      "心跳超时实例回收已执行",
      `阈值 ${result.heartbeat_stale_seconds ?? "-"} 秒，扫描 ${result.scanned}，回收 ${result.reaped}。`,
      3800
    );
  } catch (err) {
    runtimeReaperError.value =
      err instanceof ApiClientError ? err.message : tl("执行心跳超时实例回收失败");
    notify.error("执行心跳超时实例回收失败", runtimeReaperError.value);
  } finally {
    runtimeReaperBusy.value = "";
  }
}

async function handleAcknowledgeRuntimeAlert(item: AdminRuntimeAlertItem) {
  if (item.status !== "open") {
    return;
  }

  runtimeAlertUpdatingId.value = item.id;
  runtimeAlertError.value = "";

  try {
    await acknowledgeAdminRuntimeAlert(item.id, accessTokenOrThrow(), runtimeAlertNotePayload());
    await loadRuntimeAlerts({ keepSelection: true });
    runtimeAlertActionNote.value = "";
    notify.info("告警已确认", item.title);
  } catch (err) {
    runtimeAlertError.value = err instanceof ApiClientError ? err.message : tl("确认运行告警失败");
    notify.error("确认运行告警失败", runtimeAlertError.value);
  } finally {
    runtimeAlertUpdatingId.value = "";
  }
}

async function handleResolveRuntimeAlert(item: AdminRuntimeAlertItem) {
  if (item.status === "resolved") {
    return;
  }

  runtimeAlertUpdatingId.value = item.id;
  runtimeAlertError.value = "";

  try {
    await resolveAdminRuntimeAlert(item.id, accessTokenOrThrow(), runtimeAlertNotePayload());
    await loadRuntimeAlerts({ keepSelection: true });
    runtimeAlertActionNote.value = "";
    notify.success("告警已恢复", item.title);
  } catch (err) {
    runtimeAlertError.value = err instanceof ApiClientError ? err.message : tl("恢复运行告警失败");
    notify.error("恢复运行告警失败", runtimeAlertError.value);
  } finally {
    runtimeAlertUpdatingId.value = "";
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
    auditError.value = err instanceof ApiClientError ? err.message : tl("加载审计日志失败");
    notify.error("加载审计日志失败", auditError.value);
  } finally {
    auditLoading.value = false;
  }
}

async function refreshAll() {
  refreshing.value = true;
  pageError.value = "";

  try {
    await Promise.all([
      loadChallengeAttachmentLimit({ silentError: true }),
      loadChallengeCategories(),
      loadChallenges(),
      loadContests(),
      loadInstances(),
      loadUsers(),
      loadRuntimeOverview(),
      loadRuntimeAlerts(),
      loadAuditLogs()
    ]);
    await Promise.all([
      loadContestBindings(),
      loadContestAnnouncements(),
      loadContestRegistrations({ silentError: true })
    ]);
    if (selectedChallengeId.value) {
      await Promise.all([loadChallengeVersions(), loadChallengeAttachments()]);
    }
    notify.success("管理台已刷新", "最新题目、比赛、实例、审计和运行概览已同步。", 2400);
  } catch (err) {
    pageError.value = err instanceof ApiClientError ? err.message : tl("刷新失败");
    notify.error("刷新失败", pageError.value);
  } finally {
    refreshing.value = false;
  }
}

async function handleLoadChallengeForEdit(challengeId: string) {
  challengeError.value = "";
  challengeImageTestError.value = "";

  try {
    const detail = await getAdminChallengeDetail(challengeId, accessTokenOrThrow());
    applyChallengeDetailToForm(detail);
    selectedChallengeId.value = detail.id;
    challengeLibraryMode.value = "editor";
    challengeRuntimeImageTestResult.value = null;
    challengeRuntimeImageStreamLines.value = [];
    await Promise.all([loadChallengeVersions(), loadChallengeAttachments()]);
    notify.info("已载入题目配置", `正在编辑：${detail.title}`);
  } catch (err) {
    challengeError.value = err instanceof ApiClientError ? err.message : tl("加载题目详情失败");
    notify.error("加载题目详情失败", challengeError.value);
  }
}

function handleCancelChallengeEdit() {
  resetChallengeForm();
}

function openCreateChallengeEditor() {
  resetChallengeForm();
  challengeLibraryMode.value = "editor";
}

function openSelectedChallengeEditor() {
  if (!selectedChallengeId.value) {
    openCreateChallengeEditor();
    return;
  }
  void handleLoadChallengeForEdit(selectedChallengeId.value);
}

function openVersionsFromEditor() {
  if (!selectedChallengeId.value) {
    return;
  }
  challengeSubTab.value = "versions";
}

async function handleTestChallengeRuntimeImage() {
  challengeImageTestError.value = "";
  challengeRuntimeImageTestResult.value = null;
  challengeRuntimeImageStreamLines.value = [];

  if (newChallenge.runtime_mode !== "single_image") {
    challengeImageTestError.value = tl("仅 single_image 模式支持镜像测试");
    notify.warning("无法测试镜像", challengeImageTestError.value);
    return;
  }

  if (!newChallenge.runtime_image.trim()) {
    challengeImageTestError.value = tl("请先填写镜像仓库地址");
    notify.warning("镜像为空", challengeImageTestError.value);
    return;
  }

  testingChallengeRuntimeImage.value = true;
  try {
    const result = await streamAdminChallengeRuntimeImageTest(
      {
        image: newChallenge.runtime_image.trim(),
        force_pull: true,
        run_build_probe: true,
        timeout_seconds: CHALLENGE_IMAGE_TEST_TIMEOUT_SECONDS
      },
      accessTokenOrThrow(),
      {
        onEvent: (event: AdminChallengeRuntimeImageTestStreamEvent) => {
          if (event.event === "start") {
            appendChallengeRuntimeImageStreamLine(
              `[start] image=${event.image} timeout=${event.timeout_seconds}s`
            );
            return;
          }
          if (event.event === "step_start") {
            appendChallengeRuntimeImageStreamLine(`[${event.step}] $ ${event.command}`);
            return;
          }
          if (event.event === "step_log") {
            const prefix = event.stream === "stderr" ? "[stderr] " : "";
            appendChallengeRuntimeImageStreamLine(`${prefix}${event.line}`);
            return;
          }
          if (event.event === "step_finish") {
            appendChallengeRuntimeImageStreamLine(
              `[${event.step}] done success=${event.success ? "yes" : "no"} exit=${event.exit_code ?? "timeout"} duration=${event.duration_ms}ms`
            );
            return;
          }
          if (event.event === "completed") {
            appendChallengeRuntimeImageStreamLine(
              `[completed] ${event.result.succeeded ? "success" : "failed"}`
            );
            return;
          }
          if (event.event === "error") {
            appendChallengeRuntimeImageStreamLine(`[error] ${event.message}`);
          }
        }
      }
    );
    challengeRuntimeImageTestResult.value = result;
    if (result.succeeded) {
      notify.success("镜像测试通过", result.image);
    } else {
      notify.warning("镜像测试失败", result.image);
    }
  } catch (err) {
    challengeImageTestError.value = err instanceof ApiClientError ? err.message : tl("镜像测试失败");
    notify.error("镜像测试失败", challengeImageTestError.value);
  } finally {
    testingChallengeRuntimeImage.value = false;
  }
}

async function handleCreateChallenge() {
  creatingChallenge.value = true;
  challengeError.value = "";

  try {
    const isEditMode = !!editingChallengeId.value;
    if (newChallenge.runtime_mode === "none" && newChallenge.challenge_type !== "static") {
      challengeError.value = tr(
        "no_runtime 模式仅支持 static 题型",
        "no_runtime mode only supports static challenges."
      );
      return;
    }

    if (newChallenge.runtime_mode === "single_image") {
      if (!newChallenge.runtime_image.trim()) {
        challengeError.value = tl("single_image 模式必须填写镜像仓库地址");
        return;
      }
      if (
        !Number.isFinite(newChallenge.runtime_internal_port) ||
        newChallenge.runtime_internal_port < 1 ||
        newChallenge.runtime_internal_port > 65535
      ) {
        challengeError.value = tl("single_image 模式内部端口必须在 1~65535");
        return;
      }
      if (newChallenge.challenge_type === "static") {
        challengeError.value = tl("single_image 模式仅支持 dynamic 或 internal 题型");
        return;
      }
    }

    if (
      newChallenge.runtime_mode === "compose" &&
      (newChallenge.challenge_type === "dynamic" || newChallenge.challenge_type === "internal") &&
      !newChallenge.compose_template.trim()
    ) {
      challengeError.value = tl("dynamic/internal 题型在 compose 模式下必须提供 compose 模板");
      return;
    }

    const runtimeMetadata = buildChallengeRuntimeMetadata();
    const hasRuntimeMetadata = Object.keys(runtimeMetadata).length > 0;
    const selectedCategory = newChallenge.category.trim().toLowerCase();
    if (!selectedCategory) {
      challengeError.value = tr("请选择题目类别。", "Please select a challenge category.");
      return;
    }

    const payload = {
      title: newChallenge.title,
      slug: newChallenge.slug,
      category: selectedCategory,
      description: newChallenge.description || undefined,
      difficulty: newChallenge.difficulty,
      static_score: newChallenge.static_score,
      status: newChallenge.status,
      challenge_type: newChallenge.challenge_type,
      flag_mode: newChallenge.flag_mode,
      flag_hash: newChallenge.flag_hash,
      compose_template:
        newChallenge.runtime_mode === "compose"
          ? newChallenge.compose_template || undefined
          : undefined,
      metadata: hasRuntimeMetadata ? runtimeMetadata : undefined,
      tags: parseTagsInput(newChallenge.tags_input),
      hints: parseHintsInput(newChallenge.hints_input),
      writeup_visibility: newChallenge.writeup_visibility,
      writeup_content: newChallenge.writeup_content || undefined,
      change_note: newChallenge.change_note || undefined
    };

    if (isEditMode) {
      await updateAdminChallenge(editingChallengeId.value, payload, accessTokenOrThrow());
    } else {
      await createAdminChallenge(payload, accessTokenOrThrow());
    }

    resetChallengeForm();
    challengeLibraryMode.value = "catalog";

    await loadChallenges();
    notify.success(
      isEditMode ? "题目已更新" : "题目已创建",
      "可以继续管理版本、附件或挂载到比赛。"
    );
  } catch (err) {
    challengeError.value = err instanceof ApiClientError ? err.message : tl("保存题目失败");
    notify.error("保存题目失败", challengeError.value);
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
    notify.info("题目状态已更新", `当前状态：${status}`);
  } catch (err) {
    challengeError.value = err instanceof ApiClientError ? err.message : tl("更新题目失败");
    notify.error("更新题目失败", challengeError.value);
  } finally {
    updatingChallengeId.value = "";
  }
}

async function handleDestroyChallenge(item: AdminChallengeItem) {
  const confirmed = await uiStore.confirm({
    title: tr("销毁题目", "Destroy challenge"),
    message: tr(
      `确认销毁题目「${item.title}」？该操作会删除题目、挂载关系、提交记录与运行实例。`,
      `Destroy challenge "${item.title}"? This will remove challenge data, bindings, submissions, and instances.`
    ),
    confirmLabel: tr("销毁", "Destroy"),
    cancelLabel: tr("取消", "Cancel"),
    intent: "danger"
  });
  if (!confirmed) {
    return;
  }

  destroyingChallengeId.value = item.id;
  challengeError.value = "";

  try {
    await deleteAdminChallenge(item.id, accessTokenOrThrow());
    if (selectedChallengeId.value === item.id) {
      selectedChallengeId.value = "";
      challengeVersions.value = [];
      challengeAttachments.value = [];
    }
    await Promise.all([loadChallenges(), loadContests(), loadContestBindings()]);
    notify.warning("题目已销毁", item.title);
  } catch (err) {
    challengeError.value = err instanceof ApiClientError ? err.message : tl("销毁题目失败");
    notify.error("销毁题目失败", challengeError.value);
  } finally {
    destroyingChallengeId.value = "";
  }
}

async function handleRollbackChallengeVersion() {
  if (!selectedChallengeId.value) {
    challengeVersionError.value = tl("请先选择要管理的题目");
    notify.warning("未选择题目", challengeVersionError.value);
    return;
  }

  if (!Number.isFinite(rollbackForm.version_no) || rollbackForm.version_no < 1) {
    challengeVersionError.value = tl("版本号必须是大于等于 1 的整数");
    notify.warning("版本号非法", challengeVersionError.value);
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
    notify.success("题目已回滚", `已回滚到版本 v${Math.floor(rollbackForm.version_no)}。`);
  } catch (err) {
    challengeVersionError.value =
      err instanceof ApiClientError ? err.message : tl("回滚题目版本失败");
    notify.error("回滚题目版本失败", challengeVersionError.value);
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
    challengeAttachmentError.value = tl("请先选择要管理的题目");
    notify.warning("未选择题目", challengeAttachmentError.value);
    return;
  }

  if (!selectedAttachmentFile.value) {
    challengeAttachmentError.value = tl("请先选择一个附件文件");
    notify.warning("未选择文件", challengeAttachmentError.value);
    return;
  }

  if (selectedAttachmentFile.value.size > challengeAttachmentMaxBytes.value) {
    const limitText = formatSize(challengeAttachmentMaxBytes.value);
    challengeAttachmentError.value = tr(
      `附件超过大小限制（最大 ${limitText}）。`,
      `Attachment exceeds size limit (max ${limitText}).`
    );
    notify.warning(tr("附件过大", "Attachment too large"), challengeAttachmentError.value);
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
    notify.success("附件已上传", file.name);
  } catch (err) {
    challengeAttachmentError.value =
      err instanceof ApiClientError ? err.message : tl("上传题目附件失败");
    notify.error("上传题目附件失败", challengeAttachmentError.value);
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
    notify.warning("附件已删除", "已从当前题目移除附件。");
  } catch (err) {
    challengeAttachmentError.value =
      err instanceof ApiClientError ? err.message : tl("删除题目附件失败");
    notify.error("删除题目附件失败", challengeAttachmentError.value);
  } finally {
    deletingAttachmentId.value = "";
  }
}

function handleLoadContestForEdit(item: AdminContestItem) {
  applyContestToForm(item);
  selectedContestId.value = item.id;
  contestManageMode.value = "editor";
  contestError.value = "";
  notify.info("已载入比赛配置", `正在编辑：${item.title}`);
}

function handleCancelContestEdit() {
  resetContestForm();
  contestManageMode.value = "catalog";
}

function openCreateContestEditor() {
  resetContestForm();
  contestManageMode.value = "editor";
}

function openSelectedContestEditor() {
  const item = selectedContest.value;
  if (!item) {
    openCreateContestEditor();
    return;
  }
  handleLoadContestForEdit(item);
}

async function handleCreateContest() {
  creatingContest.value = true;
  contestError.value = "";

  try {
    const isEditMode = !!editingContestId.value;
    if (!Number.isFinite(newContest.dynamic_decay) || newContest.dynamic_decay < 1) {
      throw new ApiClientError(tl("dynamic_decay 必须为大于等于 1 的整数"), "bad_request");
    }
    const bloodBonuses = [
      newContest.first_blood_bonus_percent,
      newContest.second_blood_bonus_percent,
      newContest.third_blood_bonus_percent
    ];
    if (
      bloodBonuses.some(
        (value) =>
          !Number.isFinite(value) ||
          value < 0 ||
          value > 500 ||
          Math.floor(value) !== value
      )
    ) {
      throw new ApiClientError(
        tr(
          "一二三血加成必须为 0-500 的整数百分比。",
          "Blood bonuses must be integer percentages between 0 and 500."
        ),
        "bad_request"
      );
    }

    const payload = {
      title: newContest.title,
      slug: newContest.slug,
      description: newContest.description || undefined,
      visibility: newContest.visibility,
      status: newContest.status,
      scoring_mode: newContest.scoring_mode,
      dynamic_decay: Math.floor(newContest.dynamic_decay),
      first_blood_bonus_percent: Math.floor(newContest.first_blood_bonus_percent),
      second_blood_bonus_percent: Math.floor(newContest.second_blood_bonus_percent),
      third_blood_bonus_percent: Math.floor(newContest.third_blood_bonus_percent),
      registration_requires_approval: !!newContest.registration_requires_approval,
      start_at: localInputToIso(newContest.start_at),
      end_at: localInputToIso(newContest.end_at),
      freeze_at: newContest.freeze_at ? localInputToIso(newContest.freeze_at) : undefined
    };

    let targetContestId = "";
    if (isEditMode) {
      const updated = await updateAdminContest(
        editingContestId.value,
        {
          ...payload,
          clear_freeze_at: !newContest.freeze_at
        },
        accessTokenOrThrow()
      );
      targetContestId = updated.id;
    } else {
      const created = await createAdminContest(payload, accessTokenOrThrow());
      targetContestId = created.id;
    }

    resetContestForm();
    contestManageMode.value = "catalog";

    await loadContests();
    selectedContestId.value = targetContestId;
    await Promise.all([loadContestBindings(), loadContestAnnouncements()]);
    notify.success(
      isEditMode ? "比赛已更新" : "比赛已创建",
      "可以继续挂载题目并调整状态。"
    );
  } catch (err) {
    contestError.value = err instanceof ApiClientError ? err.message : tl("保存比赛失败");
    notify.error("保存比赛失败", contestError.value);
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
    notify.info("比赛状态已更新", `当前状态：${status}`);
  } catch (err) {
    contestError.value = err instanceof ApiClientError ? err.message : tl("更新比赛状态失败");
    notify.error("更新比赛状态失败", contestError.value);
  } finally {
    updatingContestId.value = "";
  }
}

async function updateContestRegistrationStatus(
  item: AdminContestRegistrationItem,
  status: "pending" | "approved" | "rejected"
) {
  if (!selectedContestId.value) {
    return;
  }

  updatingContestRegistrationId.value = item.id;
  contestRegistrationError.value = "";

  try {
    const reviewNote =
      status === "approved"
        ? tr("管理员批准参赛", "Approved by admin")
        : status === "rejected"
          ? tr("管理员拒绝报名", "Rejected by admin")
          : tr("已重置为待审核状态", "Reset to pending review");

    await updateAdminContestRegistration(
      selectedContestId.value,
      item.id,
      {
        status,
        review_note: reviewNote
      },
      accessTokenOrThrow()
    );
    await loadContestRegistrations({ silentError: true });
    notify.info(
      tr("报名状态已更新", "Registration status updated"),
      `${item.team_name} -> ${status}`
    );
  } catch (err) {
    contestRegistrationError.value =
      err instanceof ApiClientError ? err.message : tr("更新报名状态失败", "Failed to update registration status");
    notify.error("更新报名状态失败", contestRegistrationError.value);
  } finally {
    updatingContestRegistrationId.value = "";
  }
}

async function handleUploadContestPoster() {
  if (!selectedContestId.value) {
    contestError.value = tl("请先选择比赛");
    notify.warning("未选择比赛", contestError.value);
    return;
  }

  if (!selectedContestPosterFile.value) {
    contestError.value = tl("请先选择海报文件");
    notify.warning("未选择海报", contestError.value);
    return;
  }

  uploadingContestPoster.value = true;
  contestError.value = "";

  try {
    const file = selectedContestPosterFile.value;
    const contentBase64 = await fileToBase64(file);
    await uploadAdminContestPoster(
      selectedContestId.value,
      {
        filename: file.name,
        content_type: file.type || undefined,
        content_base64: contentBase64
      },
      accessTokenOrThrow()
    );
    selectedContestPosterFile.value = null;
    contestPosterInputKey.value += 1;
    await loadContests();
    notify.success("海报已上传", file.name);
  } catch (err) {
    contestError.value = err instanceof ApiClientError ? err.message : tl("上传比赛海报失败");
    notify.error("上传比赛海报失败", contestError.value);
  } finally {
    uploadingContestPoster.value = false;
  }
}

async function handleDeleteContestPoster(item: AdminContestItem) {
  if (!item.poster_url) {
    return;
  }

  const confirmed = await uiStore.confirm({
    title: tr("删除比赛海报", "Delete contest poster"),
    message: tr(
      `确认删除比赛「${item.title}」的海报？`,
      `Delete poster for contest "${item.title}"?`
    ),
    confirmLabel: tr("删除", "Delete"),
    cancelLabel: tr("取消", "Cancel"),
    intent: "danger"
  });
  if (!confirmed) {
    return;
  }

  deletingContestPosterId.value = item.id;
  contestError.value = "";

  try {
    await deleteAdminContestPoster(item.id, accessTokenOrThrow());
    await loadContests();
    notify.warning("海报已删除", item.title);
  } catch (err) {
    contestError.value = err instanceof ApiClientError ? err.message : tl("删除比赛海报失败");
    notify.error("删除比赛海报失败", contestError.value);
  } finally {
    deletingContestPosterId.value = "";
  }
}

async function handleDestroyContest(item: AdminContestItem) {
  const confirmed = await uiStore.confirm({
    title: tr("销毁比赛", "Destroy contest"),
    message: tr(
      `确认销毁比赛「${item.title}」？该操作将删除比赛、公告、挂载、提交与实例数据。`,
      `Destroy contest "${item.title}"? This will remove contest data, announcements, bindings, submissions, and instances.`
    ),
    confirmLabel: tr("销毁", "Destroy"),
    cancelLabel: tr("取消", "Cancel"),
    intent: "danger"
  });
  if (!confirmed) {
    return;
  }

  destroyingContestId.value = item.id;
  contestError.value = "";

  try {
    await deleteAdminContest(item.id, accessTokenOrThrow());
    if (editingContestId.value === item.id) {
      resetContestForm();
      contestManageMode.value = "catalog";
    }
    if (selectedContestId.value === item.id) {
      selectedContestId.value = "";
      contestBindings.value = [];
      contestAnnouncements.value = [];
      selectedBindingChallengeId.value = "";
      selectedAnnouncementId.value = "";
    }
    await Promise.all([loadContests(), loadContestBindings(), loadContestAnnouncements()]);
    notify.warning("比赛已销毁", item.title);
  } catch (err) {
    contestError.value = err instanceof ApiClientError ? err.message : tl("销毁比赛失败");
    notify.error("销毁比赛失败", contestError.value);
  } finally {
    destroyingContestId.value = "";
  }
}

async function toggleUserStatus(item: AdminUserItem) {
  updatingUserId.value = item.id;
  userError.value = "";

  const nextStatus = item.status === "active" ? "disabled" : "active";

  try {
    await updateAdminUserStatus(item.id, nextStatus, accessTokenOrThrow());
    await loadUsers();
    notify.info(
      "用户状态已更新",
      `${item.username} 已${nextStatus === "active" ? "启用" : "禁用"}。`
    );
  } catch (err) {
    userError.value = err instanceof ApiClientError ? err.message : tl("更新用户状态失败");
    notify.error("更新用户状态失败", userError.value);
  } finally {
    updatingUserId.value = "";
  }
}

async function handleResetUserPassword(item: AdminUserItem) {
  const nextPassword = (resetPasswords[item.id] ?? "").trim();
  if (nextPassword.length < 8) {
    userError.value = tl("新密码至少需要 8 位字符");
    notify.warning("密码过短", userError.value);
    return;
  }

  resettingUserId.value = item.id;
  userError.value = "";

  try {
    await resetAdminUserPassword(item.id, nextPassword, accessTokenOrThrow());
    resetPasswords[item.id] = "";
    await loadUsers();
    notify.success("密码已重置", `${item.username} 的密码已更新。`);
  } catch (err) {
    userError.value = err instanceof ApiClientError ? err.message : tl("重置密码失败");
    notify.error("重置密码失败", userError.value);
  } finally {
    resettingUserId.value = "";
  }
}

async function handleUpdateUserRole(item: AdminUserItem) {
  const nextRole = (roleDrafts[item.id] ?? "").trim().toLowerCase();
  if (!["player", "judge", "admin"].includes(nextRole)) {
    userError.value = tl("角色必须是 player / judge / admin");
    notify.warning("角色非法", userError.value);
    return;
  }

  if (nextRole === item.role) {
    notify.info("角色未变化", `${item.username} 当前角色仍为 ${item.role}。`);
    return;
  }

  updatingUserRoleId.value = item.id;
  userError.value = "";

  try {
    await updateAdminUserRole(item.id, nextRole, accessTokenOrThrow());
    await loadUsers();
    notify.success("角色已更新", `${item.username} 已设为 ${nextRole}。`);
  } catch (err) {
    userError.value = err instanceof ApiClientError ? err.message : tl("更新用户角色失败");
    notify.error("更新用户角色失败", userError.value);
  } finally {
    updatingUserRoleId.value = "";
  }
}

async function handleDeleteUserAccount(item: AdminUserItem) {
  const confirmed = await uiStore.confirm({
    title: tr("删除账号", "Delete account"),
    message: tr(
      `确认删除账号「${item.username}」？该操作会禁用并匿名化该账号。`,
      `Delete account "${item.username}"? This operation will disable and anonymize the account.`
    ),
    confirmLabel: tr("删除", "Delete"),
    cancelLabel: tr("取消", "Cancel"),
    intent: "danger"
  });
  if (!confirmed) {
    return;
  }

  deletingUserAccountId.value = item.id;
  userError.value = "";

  try {
    await deleteAdminUser(item.id, accessTokenOrThrow());
    await loadUsers();
    notify.warning("账号已删除", `${item.username} 已被禁用并匿名化。`);
  } catch (err) {
    userError.value = err instanceof ApiClientError ? err.message : tl("删除账号失败");
    notify.error("删除账号失败", userError.value);
  } finally {
    deletingUserAccountId.value = "";
  }
}

function selectContest(contestId: string) {
  selectedContestId.value = contestId;
  selectedContestPosterFile.value = null;
  contestPosterInputKey.value += 1;
}

function selectBinding(challengeId: string) {
  selectedBindingChallengeId.value = challengeId;
}

function selectAnnouncement(announcementId: string) {
  selectedAnnouncementId.value = announcementId;
  announcementEditMode.value = "edit";
}

function loadBindingIntoForm(item: AdminContestChallengeItem) {
  bindingForm.challenge_id = item.challenge_id;
  bindingForm.sort_order = item.sort_order;
  bindingForm.release_at = item.release_at ? isoToLocalInput(item.release_at) : "";
}

async function handleCreateAnnouncement() {
  if (!selectedContestId.value) {
    announcementError.value = tl("请先选择比赛");
    notify.warning("未选择比赛", announcementError.value);
    return;
  }

  const title = announcementForm.title.trim();
  const content = announcementForm.content.trim();
  if (!title || !content) {
    announcementError.value = tl("公告标题和内容不能为空");
    notify.warning("公告内容不完整", announcementError.value);
    return;
  }

  creatingAnnouncement.value = true;
  announcementError.value = "";

  try {
    const created = await createAdminContestAnnouncement(
      selectedContestId.value,
      {
        title,
        content,
        is_published: announcementForm.is_published,
        is_pinned: announcementForm.is_pinned
      },
      accessTokenOrThrow()
    );
    announcementForm.title = "";
    announcementForm.content = "";
    announcementForm.is_published = false;
    announcementForm.is_pinned = false;
    announcementCreateMode.value = "edit";
    selectedAnnouncementId.value = created.id;
    await loadContestAnnouncements();
    notify.success("公告已创建", "公告已保存。");
  } catch (err) {
    announcementError.value = err instanceof ApiClientError ? err.message : tl("创建公告失败");
    notify.error("创建公告失败", announcementError.value);
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
    notify.info(
      "公告状态已更新",
      !item.is_published ? "公告已发布" : "公告已撤回"
    );
  } catch (err) {
    announcementError.value = err instanceof ApiClientError ? err.message : tl("更新公告失败");
    notify.error("更新公告失败", announcementError.value);
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
    announcementError.value = tl("公告标题和内容不能为空");
    notify.warning("公告内容不完整", announcementError.value);
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
    notify.success("公告已更新", item.title);
  } catch (err) {
    announcementError.value = err instanceof ApiClientError ? err.message : tl("更新公告失败");
    notify.error("更新公告失败", announcementError.value);
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
    notify.info(
      "公告置顶状态已更新",
      !item.is_pinned ? "公告已置顶" : "公告已取消置顶"
    );
  } catch (err) {
    announcementError.value = err instanceof ApiClientError ? err.message : tl("更新公告失败");
    notify.error("更新公告失败", announcementError.value);
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
    notify.warning("公告已删除", item.title);
  } catch (err) {
    announcementError.value = err instanceof ApiClientError ? err.message : tl("删除公告失败");
    notify.error("删除公告失败", announcementError.value);
  } finally {
    deletingAnnouncementId.value = "";
  }
}

async function handleUpsertBinding() {
  if (!selectedContestId.value) {
    bindingError.value = tl("请先选择比赛");
    notify.warning("未选择比赛", bindingError.value);
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
    notify.success("挂载成功", "题目已挂载/更新到当前比赛。");
  } catch (err) {
    bindingError.value = err instanceof ApiClientError ? err.message : tl("挂载失败");
    notify.error("挂载失败", bindingError.value);
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
    notify.info("排序已更新", `新排序值：${nextSort}`);
  } catch (err) {
    bindingError.value = err instanceof ApiClientError ? err.message : tl("更新排序失败");
    notify.error("更新排序失败", bindingError.value);
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
    notify.info("发布时间已清除", "该题将在比赛内即时可见。");
  } catch (err) {
    bindingError.value = err instanceof ApiClientError ? err.message : tl("清除发布时间失败");
    notify.error("清除发布时间失败", bindingError.value);
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
    notify.warning("挂载已移除", "题目已从当前比赛移除。");
  } catch (err) {
    bindingError.value = err instanceof ApiClientError ? err.message : tl("移除挂载失败");
    notify.error("移除挂载失败", bindingError.value);
  } finally {
    bindingBusy.value = false;
  }
}

watch(
  () => selectedContestId.value,
  () => {
    selectedBindingChallengeId.value = "";
    selectedAnnouncementId.value = "";
    selectedContestRegistrationId.value = "";
    announcementCreateMode.value = "edit";
    announcementEditMode.value = "edit";
    selectedContestPosterFile.value = null;
    contestPosterInputKey.value += 1;
    bindingForm.challenge_id = "";
    bindingForm.sort_order = 0;
    bindingForm.release_at = "";
    loadContestBindings();
    loadContestAnnouncements();
    loadContestRegistrations({ silentError: true });
  }
);

watch(
  () => selectedAnnouncementId.value,
  () => {
    announcementEditMode.value = "edit";
  }
);

watch(
  () => contestRegistrationStatusFilter.value,
  () => {
    if (adminModule.value === "contests" && contestSubTab.value === "registrations") {
      loadContestRegistrations({ silentError: true });
    }
  }
);

watch(
  () => [adminModule.value, challengeSubTab.value] as const,
  ([module, subTab]) => {
    if (module !== "challenges" || subTab !== "lint") {
      return;
    }

    loadChallengeRuntimeLint({ silentError: true });
  }
);

watch(
  () => [adminModule.value, contestSubTab.value, selectedContestId.value] as const,
  ([module, subTab, contestId]) => {
    if (module !== "contests" || subTab !== "registrations" || !contestId) {
      return;
    }
    loadContestRegistrations({ silentError: true });
  }
);

function startRuntimePolling() {
  stopRuntimePolling();
  runtimePollTimer = window.setInterval(() => {
    loadRuntimeOverview({ silentError: true });
    loadRuntimeAlerts({ silentError: true, keepSelection: true });
    if (selectedInstanceId.value) {
      loadInstanceRuntimeMetrics(selectedInstanceId.value, { silentError: true });
    }
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
.admin-layout {
  display: grid;
  grid-template-columns: 260px minmax(0, 1fr);
  gap: 1rem;
  align-items: start;
}

.admin-side-nav {
  position: sticky;
  top: 5.3rem;
  max-height: calc(100vh - 6.4rem);
  overflow: auto;
  display: grid;
  gap: 0.85rem;
  align-content: start;
}

.admin-side-group {
  display: grid;
  gap: 0.42rem;
}

.nav-group-label {
  margin: 0;
  font-size: 0.72rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: rgba(18, 18, 18, 0.48);
}

.side-nav-btn {
  text-align: left;
  border: 1px solid transparent;
  border-radius: 11px;
  background: rgba(255, 255, 255, 0.32);
  color: rgba(20, 20, 20, 0.82);
  padding: 0.5rem 0.62rem;
  cursor: pointer;
  transition: background-color 150ms ease, transform 150ms ease, color 150ms ease;
}

.side-nav-btn:hover {
  background: rgba(255, 255, 255, 0.52);
  transform: translateY(-1px);
}

.side-nav-btn.active {
  background: rgba(14, 14, 14, 0.86);
  color: rgba(255, 255, 255, 0.94);
}

.side-sub-btn {
  font-size: 0.9rem;
}

.admin-main {
  display: grid;
  gap: 1rem;
  min-width: 0;
}

.admin-grid {
  grid-template-columns: minmax(0, 1fr);
}

.module-split {
  display: grid;
  gap: 0.95rem;
  margin-top: 0.74rem;
  align-items: stretch;
}

.challenge-split {
  grid-template-columns: minmax(0, 1.1fr) minmax(0, 1.4fr);
}

.contest-split {
  grid-template-columns: minmax(0, 1fr) minmax(0, 1.4fr);
}

.module-column {
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.36);
  padding: 0.85rem;
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
  min-height: 0;
  position: relative;
  overflow: hidden;
}

.module-column::before {
  content: "";
  position: absolute;
  inset: 0;
  border-radius: inherit;
  pointer-events: none;
  background:
    linear-gradient(rgba(12, 12, 12, 0.16), rgba(12, 12, 12, 0.16)) top / 100% 1px no-repeat,
    linear-gradient(rgba(12, 12, 12, 0.16), rgba(12, 12, 12, 0.16)) left / 1px 100% no-repeat,
    repeating-linear-gradient(90deg, transparent 0 9px, rgba(12, 12, 12, 0.24) 9px 14px) bottom / 100% 1px no-repeat;
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

.compact-grid .category-actions {
  grid-column: 1 / -1;
}

.challenge-library-head {
  margin-top: 0.7rem;
  margin-bottom: 0.12rem;
}

.challenge-library-mode-switch {
  gap: 0.18rem;
  padding: 0.2rem;
}

.challenge-library-mode-switch .ghost {
  min-height: 1.78rem;
  padding-inline: 0.62rem;
  min-width: 8.8rem;
}

.challenge-library-mode-switch .ghost.active {
  background: rgba(18, 18, 18, 0.86);
  color: rgba(255, 255, 255, 0.94);
}

.challenge-library-shell {
  margin-top: 0.3rem;
  grid-template-columns: minmax(0, 1fr);
}

.challenge-editor-column {
  width: 100%;
  max-width: none;
  justify-self: stretch;
}

.challenge-library-column .challenge-card-grid {
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  min-height: 560px;
  max-height: none;
}

.contest-manage-head {
  margin-top: 0.7rem;
  margin-bottom: 0.12rem;
}

.contest-manage-mode-switch {
  gap: 0.18rem;
  padding: 0.2rem;
}

.contest-manage-mode-switch .ghost {
  min-height: 1.78rem;
  padding-inline: 0.62rem;
}

.contest-manage-mode-switch .ghost.active {
  background: rgba(18, 18, 18, 0.86);
  color: rgba(255, 255, 255, 0.94);
}

.contest-management-shell {
  margin-top: 0.3rem;
  grid-template-columns: minmax(0, 1fr);
}

.contest-editor-column {
  width: 100%;
  max-width: none;
  justify-self: stretch;
}

.contest-catalog-column {
  min-height: 0;
}

.contest-catalog-workspace {
  display: grid;
  grid-template-columns: minmax(300px, 0.92fr) minmax(0, 1.5fr);
  gap: 0.88rem;
  min-height: 620px;
}

.contest-catalog-list {
  max-height: 72vh;
}

.contest-catalog-detail {
  min-height: 0;
}

.challenge-create-form {
  margin-top: 0.7rem;
  gap: 0.68rem;
}

.challenge-form-block {
  position: relative;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.44);
  padding: 0.64rem;
  display: grid;
  gap: 0.52rem;
}

.challenge-form-block::before {
  content: "";
  position: absolute;
  inset: 0;
  pointer-events: none;
  border-radius: inherit;
  background:
    linear-gradient(rgba(12, 12, 12, 0.14), rgba(12, 12, 12, 0.14)) top / 100% 1px no-repeat,
    linear-gradient(rgba(12, 12, 12, 0.14), rgba(12, 12, 12, 0.14)) left / 1px 100% no-repeat;
}

.challenge-form-block-head {
  display: grid;
  gap: 0.1rem;
  padding-bottom: 0.44rem;
  border-bottom: 1px dashed rgba(12, 12, 12, 0.24);
}

.challenge-form-block-head h4 {
  margin: 0;
  font-size: 1rem;
}

.challenge-form-block-head p {
  margin: 0;
  font-size: 0.8rem;
  color: rgba(18, 18, 18, 0.58);
}

.challenge-form-grid {
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.58rem 0.62rem;
}

.challenge-form-grid .field-span-2,
.challenge-form-grid .image-test-block {
  grid-column: 1 / -1;
}

.challenge-form-grid textarea {
  min-height: 7.1rem;
}

.challenge-form-grid .field-note {
  display: block;
  margin-top: 0.28rem;
  font-size: 0.75rem;
  color: rgba(18, 18, 18, 0.62);
  line-height: 1.45;
}

.challenge-submit-row {
  display: flex;
  justify-content: flex-end;
  margin-top: 0.1rem;
}

.challenge-submit-row .primary {
  min-width: min(100%, 220px);
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
  max-height: 650px;
  overflow: auto;
  padding-right: 0.2rem;
  align-content: start;
}

.challenge-category-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(170px, 1fr));
  gap: 0.46rem;
}

.category-item {
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.5);
  padding: 0.52rem;
  display: grid;
  gap: 0.3rem;
}

.category-actions {
  justify-content: flex-start;
}

.challenge-card {
  cursor: pointer;
}

.challenge-card.active {
  background: rgba(18, 18, 18, 0.84);
  color: rgba(255, 255, 255, 0.92);
}

.challenge-card.active .muted {
  color: rgba(255, 255, 255, 0.72);
}

.challenge-card.active .badge {
  background: rgba(255, 255, 255, 0.2);
}

.action-menu {
  border-top: 1px dashed rgba(255, 255, 255, 0.28);
  padding-top: 0.44rem;
}

.action-sheet {
  border-radius: 11px;
  background: rgba(255, 255, 255, 0.36);
  padding: 0.42rem 0.5rem;
  position: relative;
}

.action-sheet::before {
  content: "";
  position: absolute;
  inset: 0;
  pointer-events: none;
  border-radius: inherit;
  background:
    linear-gradient(rgba(12, 12, 12, 0.14), rgba(12, 12, 12, 0.14)) top / 100% 1px no-repeat,
    repeating-linear-gradient(90deg, transparent 0 8px, rgba(12, 12, 12, 0.22) 8px 13px) bottom / 100% 1px no-repeat;
}

.action-sheet > summary {
  font-size: 0.86rem;
  color: rgba(18, 18, 18, 0.8);
  padding: 0.2rem 0;
}

.action-sheet-body {
  margin-top: 0.44rem;
  padding-top: 0.44rem;
  border-top: 1px dashed rgba(12, 12, 12, 0.24);
}

.action-sheet-inverse {
  background: rgba(255, 255, 255, 0.16);
}

.action-sheet-inverse > summary {
  color: rgba(255, 255, 255, 0.86);
}

.action-sheet-inverse .action-sheet-body {
  border-top-color: rgba(255, 255, 255, 0.28);
}

.filter-sheet {
  border-radius: 11px;
  background: rgba(255, 255, 255, 0.38);
  padding: 0.44rem 0.52rem;
  margin-bottom: 0.62rem;
  position: relative;
}

.filter-sheet::before {
  content: "";
  position: absolute;
  inset: 0;
  pointer-events: none;
  border-radius: inherit;
  background:
    linear-gradient(rgba(12, 12, 12, 0.14), rgba(12, 12, 12, 0.14)) top / 100% 1px no-repeat,
    linear-gradient(rgba(12, 12, 12, 0.14), rgba(12, 12, 12, 0.14)) left / 1px 100% no-repeat;
}

.filter-sheet > summary {
  font-size: 0.86rem;
  color: rgba(18, 18, 18, 0.82);
  padding: 0.2rem 0;
}

.filter-sheet-body {
  margin-top: 0.44rem;
  padding-top: 0.44rem;
  border-top: 1px dashed rgba(12, 12, 12, 0.24);
  align-items: flex-end;
}

.filter-sheet-body > button.ghost {
  align-self: flex-end;
}

.compact-actions {
  margin-bottom: 0;
}

.contest-browser {
  display: grid;
  grid-template-columns: minmax(320px, 0.95fr) minmax(0, 1.45fr);
  gap: 0.92rem;
  flex: 1;
  min-height: 520px;
  min-width: 0;
}

.contest-list-pane {
  display: grid;
  gap: 0.64rem;
  min-height: 0;
  overflow: auto;
  padding-right: 0.2rem;
  align-content: start;
}

.announcement-pane {
  overflow: auto;
  padding-right: 0;
}

.announcement-block {
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.44);
  padding: 0.68rem;
  display: grid;
  gap: 0.56rem;
  align-content: start;
  position: relative;
}

.announcement-block::before {
  content: "";
  position: absolute;
  inset: 0;
  pointer-events: none;
  border-radius: inherit;
  background:
    linear-gradient(rgba(12, 12, 12, 0.16), rgba(12, 12, 12, 0.16)) top / 100% 1px no-repeat,
    repeating-linear-gradient(90deg, transparent 0 8px, rgba(12, 12, 12, 0.2) 8px 12px) bottom / 100% 1px no-repeat;
}

.announcement-create-form {
  grid-template-columns: 1fr;
  gap: 0.52rem;
}

.announcement-create-form > button.primary {
  width: 100%;
}

.announcement-editor-label {
  margin-bottom: -0.1rem;
}

.announcement-toolbar {
  display: flex;
  flex-wrap: wrap;
  gap: 0.32rem;
}

.announcement-toolbar .ghost {
  min-height: 1.74rem;
  padding: 0.22rem 0.5rem;
  font-size: 0.78rem;
}

.announcement-mode-switch {
  gap: 0.18rem;
  padding: 0.2rem;
}

.announcement-mode-switch .ghost {
  min-height: 1.7rem;
  padding-inline: 0.58rem;
}

.announcement-mode-switch .ghost.active {
  background: rgba(18, 18, 18, 0.84);
  color: rgba(255, 255, 255, 0.94);
}

.announcement-editor-title {
  font-size: 0.8rem;
  color: rgba(18, 18, 18, 0.74);
}

.announcement-editor-shell {
  min-height: 190px;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.54);
  padding: 0.44rem;
  display: grid;
  align-content: start;
  overflow: hidden;
}

.announcement-editor-shell-large {
  min-height: 320px;
}

.announcement-editor-textarea {
  width: 100%;
  min-height: 170px;
  resize: vertical;
  border: 1px solid rgba(12, 12, 12, 0.16);
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.68);
  padding: 0.58rem 0.62rem;
  line-height: 1.58;
}

.announcement-editor-textarea-lg {
  min-height: 290px;
}

.announcement-markdown-preview {
  min-height: 100%;
  max-height: 400px;
  overflow: auto;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.62);
  padding: 0.58rem 0.66rem;
}

.announcement-markdown-hint {
  margin: 0;
  font-size: 0.78rem;
}

.contest-list-item {
  text-align: left;
  border-radius: 10px;
  padding: 0.6rem 0.62rem;
  background: rgba(255, 255, 255, 0.44);
  display: grid;
  gap: 0.2rem;
  cursor: pointer;
  border: 1px solid transparent;
  transition: background-color 140ms ease, transform 140ms ease;
}

.contest-list-item:hover {
  transform: translateY(-1px);
  background: rgba(255, 255, 255, 0.58);
}

.contest-list-item.active {
  background: rgba(18, 18, 18, 0.84);
  color: rgba(255, 255, 255, 0.94);
}

.contest-list-item.active .muted {
  color: rgba(255, 255, 255, 0.72);
}

.contest-detail-pane {
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.44);
  padding: 0.76rem;
  min-height: 0;
  display: grid;
  align-content: start;
  gap: 0.62rem;
  overflow: auto;
  position: relative;
}

.contest-detail-pane::before {
  content: "";
  position: absolute;
  inset: 0;
  pointer-events: none;
  border-radius: inherit;
  background:
    linear-gradient(rgba(12, 12, 12, 0.16), rgba(12, 12, 12, 0.16)) top / 100% 1px no-repeat,
    linear-gradient(rgba(12, 12, 12, 0.16), rgba(12, 12, 12, 0.16)) right / 1px 100% no-repeat;
}

.contest-detail-pane h4 {
  margin: 0;
}

.announcement-edit-form {
  gap: 0.58rem;
}

.announcement-detail-pane .action-sheet {
  margin-top: 0.08rem;
}

.image-test-block {
  grid-column: 1 / -1;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.42);
  padding: 0.58rem;
}

.image-test-result {
  display: grid;
  gap: 0.45rem;
}

.image-test-live-log {
  margin: 0.4rem 0 0;
  max-height: 260px;
  overflow: auto;
  border-radius: 8px;
  padding: 0.5rem;
  white-space: pre-wrap;
  word-break: break-word;
  background: rgba(255, 255, 255, 0.5);
  border: 1px dashed rgba(12, 12, 12, 0.2);
}

.image-test-result.failed {
  border-left: 3px solid rgba(120, 24, 24, 0.72);
  padding-left: 0.45rem;
}

.image-test-step {
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.52);
  padding: 0.4rem 0.5rem;
}

.image-test-step summary {
  cursor: pointer;
  color: rgba(12, 12, 12, 0.9);
}

.image-test-step pre {
  margin: 0.45rem 0 0;
  max-height: 220px;
  overflow: auto;
  background: rgba(255, 255, 255, 0.4);
  border-radius: 8px;
  padding: 0.5rem;
  white-space: pre-wrap;
  word-break: break-word;
}

.contest-poster-preview {
  width: 100%;
  max-height: 220px;
  object-fit: cover;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.5);
}

.challenge-lint-metrics {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(170px, 1fr));
  gap: 0.65rem;
  margin-bottom: 0.65rem;
}

.challenge-lint-table td p {
  margin: 0.12rem 0 0;
}

.lint-badge-ok {
  background: rgba(28, 82, 28, 0.78);
  color: rgba(255, 255, 255, 0.94);
}

.lint-badge-error {
  background: rgba(120, 24, 24, 0.82);
  color: rgba(255, 255, 255, 0.94);
}

.runtime-alert-layout,
.runtime-alert-workspace {
  display: grid;
  grid-template-columns: minmax(260px, 0.9fr) minmax(0, 1.5fr);
  gap: 0.8rem;
  min-height: 420px;
}

.runtime-alert-list-panel,
.runtime-alert-detail-panel {
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.44);
  padding: 0.75rem;
  display: grid;
  gap: 0.62rem;
  min-height: 0;
  position: relative;
}

.runtime-alert-list-panel::before,
.runtime-alert-detail-panel::before {
  content: "";
  position: absolute;
  inset: 0;
  border-radius: inherit;
  pointer-events: none;
  background:
    linear-gradient(rgba(12, 12, 12, 0.16), rgba(12, 12, 12, 0.16)) top / 100% 1px no-repeat,
    linear-gradient(rgba(12, 12, 12, 0.16), rgba(12, 12, 12, 0.16)) left / 1px 100% no-repeat;
}

.instance-filter-control {
  min-width: 220px;
}

.instance-workspace {
  margin-top: 0.72rem;
  display: grid;
  grid-template-columns: minmax(320px, 0.92fr) minmax(0, 1.5fr);
  gap: 0.8rem;
  min-height: 560px;
}

.instance-list-panel,
.instance-detail-panel {
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.44);
  padding: 0.75rem;
  display: grid;
  gap: 0.65rem;
  min-height: 0;
  position: relative;
}

.instance-list-panel::before,
.instance-detail-panel::before {
  content: "";
  position: absolute;
  inset: 0;
  border-radius: inherit;
  pointer-events: none;
  background:
    linear-gradient(rgba(12, 12, 12, 0.16), rgba(12, 12, 12, 0.16)) top / 100% 1px no-repeat,
    linear-gradient(rgba(12, 12, 12, 0.16), rgba(12, 12, 12, 0.16)) left / 1px 100% no-repeat;
}

.instance-list-body {
  display: grid;
  gap: 0.45rem;
  align-content: start;
  overflow: auto;
  min-height: 0;
  max-height: 72vh;
  padding-right: 0.1rem;
}

.instance-list-item {
  text-align: left;
  border: 1px solid transparent;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.5);
  padding: 0.55rem 0.58rem;
  display: grid;
  gap: 0.18rem;
  cursor: pointer;
  transition: transform 130ms ease, background-color 130ms ease;
}

.instance-list-item:hover {
  transform: translateY(-1px);
  background: rgba(255, 255, 255, 0.62);
}

.instance-list-item.active {
  background: rgba(18, 18, 18, 0.84);
  color: rgba(255, 255, 255, 0.94);
}

.instance-list-item.active .muted {
  color: rgba(255, 255, 255, 0.72);
}

.instance-list-item.active .instance-entry-link {
  background: rgba(255, 255, 255, 0.2);
  color: rgba(255, 255, 255, 0.94);
}

.instance-list-item-head {
  align-items: flex-start;
}

.instance-status-badge {
  min-height: 1.32rem;
  padding: 0.1rem 0.48rem;
  font-size: 0.66rem;
}

.instance-status-badge.status-running {
  background: rgba(18, 18, 18, 0.9);
  color: rgba(255, 255, 255, 0.96);
}

.instance-status-badge.status-creating {
  background: rgba(58, 58, 58, 0.88);
  color: rgba(255, 255, 255, 0.95);
}

.instance-status-badge.status-stopped,
.instance-status-badge.status-destroyed {
  background: rgba(92, 92, 92, 0.84);
  color: rgba(255, 255, 255, 0.94);
}

.instance-status-badge.status-failed {
  background: rgba(120, 24, 24, 0.82);
  color: rgba(255, 255, 255, 0.96);
}

.instance-status-badge.status-default {
  background: rgba(62, 62, 62, 0.82);
  color: rgba(255, 255, 255, 0.94);
}

.instance-entry-link {
  display: inline-flex;
  align-items: center;
  border-radius: 999px;
  min-height: 1.78rem;
  padding: 0.24rem 0.62rem;
  background: rgba(255, 255, 255, 0.52);
  color: rgba(18, 18, 18, 0.9);
  text-decoration: none;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.instance-entry-link:hover {
  background: rgba(255, 255, 255, 0.66);
}

.instance-meta-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.62rem;
}

.instance-meta-grid .metric-card p {
  margin: 0.2rem 0;
}

.instance-warnings {
  display: grid;
  gap: 0.35rem;
}

.instance-warning {
  margin: 0;
  border-radius: 9px;
  background: rgba(255, 255, 255, 0.5);
  padding: 0.48rem 0.55rem;
}

.instance-services-table td p {
  margin: 0.12rem 0 0;
}

.runtime-alert-list {
  display: grid;
  gap: 0.45rem;
  overflow: auto;
  min-height: 0;
  max-height: 72vh;
  align-content: start;
  padding-right: 0.1rem;
}

.runtime-alert-item {
  text-align: left;
  border: 1px solid transparent;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.48);
  padding: 0.58rem 0.6rem;
  display: grid;
  gap: 0.2rem;
  cursor: pointer;
  transition: transform 130ms ease, background-color 130ms ease;
}

.runtime-alert-item:hover {
  background: rgba(255, 255, 255, 0.62);
  transform: translateY(-1px);
}

.runtime-alert-item.active {
  background: rgba(18, 18, 18, 0.84);
  color: rgba(255, 255, 255, 0.94);
}

.runtime-alert-item.active .muted {
  color: rgba(255, 255, 255, 0.74);
}

.runtime-alert-item.active .badge {
  background: rgba(255, 255, 255, 0.22);
}

.runtime-alert-item.severity-critical {
  border-left: 3px solid rgba(120, 24, 24, 0.82);
}

.runtime-alert-item.severity-warning {
  border-left: 3px solid rgba(92, 70, 18, 0.82);
}

.runtime-alert-item.severity-info {
  border-left: 3px solid rgba(20, 20, 20, 0.76);
}

.runtime-alert-title {
  margin-right: 0.5rem;
}

.runtime-alert-line {
  margin: 0;
}

.runtime-alert-detail,
.runtime-alert-detail-panel {
  min-height: 0;
  overflow: auto;
  display: grid;
  gap: 0.65rem;
  align-content: start;
}

.runtime-alert-detail h3,
.runtime-alert-detail-panel h3 {
  margin: 0;
}

.runtime-alert-message {
  margin: 0;
}

.runtime-alert-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 0.38rem;
}

.runtime-alert-meta {
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.46);
  padding: 0.5rem 0.55rem;
}

.runtime-alert-meta p {
  margin: 0.16rem 0;
}

.runtime-alert-detail-json summary {
  cursor: pointer;
  color: rgba(12, 12, 12, 0.9);
  font-weight: 600;
}

.runtime-alert-detail-json pre {
  margin: 0.5rem 0 0;
  max-height: 260px;
  overflow: auto;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.42);
  padding: 0.58rem;
  white-space: pre-wrap;
  word-break: break-word;
}

.user-actions-menu {
  margin-top: 0.5rem;
  padding-top: 0.5rem;
  border-top: 1px dashed rgba(12, 12, 12, 0.24);
}

:root[data-theme="dark"] .nav-group-label {
  color: var(--fg-2);
}

:root[data-theme="dark"] .side-nav-btn {
  background: var(--glass-soft);
  color: var(--fg-1);
}

:root[data-theme="dark"] .side-nav-btn:hover {
  background: var(--glass-mid);
}

:root[data-theme="dark"] .side-nav-btn.active {
  background: var(--fg-0);
  color: var(--bg-0);
}

:root[data-theme="dark"] .challenge-library-mode-switch .ghost.active {
  background: var(--fg-0);
  color: var(--bg-0);
}

:root[data-theme="dark"] .contest-manage-mode-switch .ghost.active {
  background: var(--fg-0);
  color: var(--bg-0);
}

:root[data-theme="dark"] .module-column,
:root[data-theme="dark"] .action-sheet,
:root[data-theme="dark"] .filter-sheet,
:root[data-theme="dark"] .challenge-form-block,
:root[data-theme="dark"] .runtime-alert-list-panel,
:root[data-theme="dark"] .runtime-alert-detail-panel,
:root[data-theme="dark"] .instance-list-panel,
:root[data-theme="dark"] .instance-detail-panel,
:root[data-theme="dark"] .instance-list-item,
:root[data-theme="dark"] .instance-entry-link,
:root[data-theme="dark"] .instance-warning,
:root[data-theme="dark"] .announcement-block,
:root[data-theme="dark"] .announcement-editor-shell,
:root[data-theme="dark"] .announcement-editor-textarea,
:root[data-theme="dark"] .announcement-markdown-preview,
:root[data-theme="dark"] .category-item,
:root[data-theme="dark"] .contest-list-item,
:root[data-theme="dark"] .contest-detail-pane,
:root[data-theme="dark"] .image-test-block,
:root[data-theme="dark"] .image-test-live-log,
:root[data-theme="dark"] .image-test-step,
:root[data-theme="dark"] .image-test-step pre,
:root[data-theme="dark"] .metric-card,
:root[data-theme="dark"] .runtime-alert-item,
:root[data-theme="dark"] .runtime-alert-detail,
:root[data-theme="dark"] .runtime-alert-detail-json pre {
  background: var(--glass-mid);
}

:root[data-theme="dark"] .module-column::before,
:root[data-theme="dark"] .action-sheet::before,
:root[data-theme="dark"] .filter-sheet::before,
:root[data-theme="dark"] .challenge-form-block::before,
:root[data-theme="dark"] .runtime-alert-list-panel::before,
:root[data-theme="dark"] .runtime-alert-detail-panel::before,
:root[data-theme="dark"] .instance-list-panel::before,
:root[data-theme="dark"] .instance-detail-panel::before,
:root[data-theme="dark"] .announcement-block::before,
:root[data-theme="dark"] .contest-detail-pane::before,
:root[data-theme="dark"] .runtime-alert-detail::before {
  background:
    linear-gradient(var(--line-soft), var(--line-soft)) top / 100% 1px no-repeat,
    linear-gradient(var(--line-soft), var(--line-soft)) left / 1px 100% no-repeat,
    repeating-linear-gradient(90deg, transparent 0 9px, var(--line-mid) 9px 14px) bottom / 100% 1px no-repeat;
}

:root[data-theme="dark"] .action-sheet > summary,
:root[data-theme="dark"] .filter-sheet > summary,
:root[data-theme="dark"] .image-test-step summary,
:root[data-theme="dark"] .runtime-alert-detail-json summary {
  color: var(--fg-1);
}

:root[data-theme="dark"] .action-sheet-body,
:root[data-theme="dark"] .filter-sheet-body,
:root[data-theme="dark"] .user-actions-menu {
  border-top-color: var(--line-mid);
}

:root[data-theme="dark"] .announcement-editor-title,
:root[data-theme="dark"] .announcement-markdown-hint {
  color: rgba(228, 228, 228, 0.72);
}

:root[data-theme="dark"] .challenge-form-grid .field-note {
  color: rgba(228, 228, 228, 0.7);
}

:root[data-theme="dark"] .instance-entry-link {
  color: rgba(244, 244, 244, 0.9);
}

:root[data-theme="dark"] .instance-list-item.active .instance-entry-link {
  background: rgba(255, 255, 255, 0.18);
}

:root[data-theme="dark"] .challenge-form-block-head p {
  color: rgba(228, 228, 228, 0.7);
}

:root[data-theme="dark"] .challenge-form-block-head {
  border-bottom-color: var(--line-mid);
}

:root[data-theme="dark"] .runtime-alert-item.status-open {
  border-left-color: color-mix(in srgb, var(--fg-0) 74%, transparent);
}

:root[data-theme="dark"] .runtime-alert-item.status-acknowledged {
  border-left-color: color-mix(in srgb, var(--warn) 80%, transparent);
}

:root[data-theme="dark"] .runtime-alert-item.status-resolved {
  border-left-color: color-mix(in srgb, var(--ok) 80%, transparent);
}

@media (max-width: 1220px) {
  .admin-layout {
    grid-template-columns: 1fr;
  }

  .admin-side-nav {
    position: static;
    max-height: none;
  }

  .challenge-split,
  .contest-split {
    grid-template-columns: 1fr;
  }

  .challenge-editor-column {
    max-width: none;
  }

  .contest-editor-column {
    max-width: none;
  }

  .contest-browser {
    grid-template-columns: 1fr;
  }

  .contest-catalog-workspace {
    grid-template-columns: 1fr;
    min-height: 0;
  }

  .contest-catalog-list {
    max-height: 340px;
  }

  .runtime-alert-layout,
  .runtime-alert-workspace {
    grid-template-columns: 1fr;
  }

  .instance-workspace {
    grid-template-columns: 1fr;
  }

  .instance-list-body {
    max-height: 340px;
  }
}

@media (max-width: 860px) {
  .compact-grid {
    grid-template-columns: 1fr;
  }

  .challenge-form-grid {
    grid-template-columns: 1fr;
  }

  .instance-meta-grid {
    grid-template-columns: 1fr;
  }

  .instance-filter-control {
    min-width: 0;
    width: 100%;
  }

  .contest-manage-head,
  .challenge-library-head {
    align-items: flex-start;
    gap: 0.42rem;
  }
}
</style>
