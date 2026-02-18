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
      <button
        class="ghost"
        type="button"
        :class="{ active: challengeSubTab === 'lint' }"
        @click="challengeSubTab = 'lint'"
      >
        模板校验
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
        :class="{ active: operationsSubTab === 'alerts' }"
        @click="operationsSubTab = 'alerts'"
      >
        运行告警
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
              <div class="row-between">
                <h3>{{ challengeFormTitle }}</h3>
                <button
                  v-if="editingChallengeId"
                  class="ghost"
                  type="button"
                  @click="handleCancelChallengeEdit"
                >
                  取消编辑
                </button>
              </div>
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
                  <span>运行模式</span>
                  <select v-model="newChallenge.runtime_mode">
                    <option value="compose">compose（多容器）</option>
                    <option value="single_image">single_image（单镜像）</option>
                  </select>
                </label>
                <label v-if="newChallenge.runtime_mode === 'compose'">
                  <span>访问模式</span>
                  <select v-model="newChallenge.runtime_access_mode">
                    <option value="ssh_bastion">ssh_bastion（默认）</option>
                    <option value="wireguard">wireguard（VPN）</option>
                    <option value="direct">direct（直连入口）</option>
                  </select>
                </label>
                <label v-if="newChallenge.runtime_mode === 'single_image'">
                  <span>镜像仓库地址</span>
                  <input v-model.trim="newChallenge.runtime_image" placeholder="nginx:alpine" />
                </label>
                <label v-if="newChallenge.runtime_mode === 'single_image'">
                  <span>内部端口</span>
                  <input v-model.number="newChallenge.runtime_internal_port" type="number" min="1" max="65535" />
                </label>
                <label v-if="newChallenge.runtime_mode === 'single_image'">
                  <span>入口协议</span>
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
                      {{ testingChallengeRuntimeImage ? "测试中..." : "测试镜像（拉取+构建探测）" }}
                    </button>
                  </div>
                  <p v-if="challengeImageTestError" class="error">{{ challengeImageTestError }}</p>
                  <div
                    v-if="challengeRuntimeImageTestResult"
                    class="image-test-result"
                    :class="{ failed: !challengeRuntimeImageTestResult.succeeded }"
                  >
                    <p class="mono">
                      image={{ challengeRuntimeImageTestResult.image }} ·
                      result={{ challengeRuntimeImageTestResult.succeeded ? "success" : "failed" }} ·
                      generated_at={{ formatTime(challengeRuntimeImageTestResult.generated_at) }}
                    </p>
                    <details v-for="step in challengeRuntimeImageTestResult.steps" :key="step.step" class="image-test-step">
                      <summary>
                        {{ step.step }} · {{ step.success ? "ok" : "failed" }} · {{ step.duration_ms }}ms · exit={{ step.exit_code ?? "timeout" }}
                      </summary>
                      <pre class="mono">{{ step.output || "-" }}</pre>
                    </details>
                  </div>
                </div>
                <label v-if="newChallenge.runtime_mode === 'compose'">
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
                  {{ challengeSubmitLabel }}
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
                      @click="handleLoadChallengeForEdit(item.id)"
                      :disabled="destroyingChallengeId === item.id"
                    >
                      编辑
                    </button>
                    <button
                      class="ghost"
                      type="button"
                      @click="updateChallengeStatus(item.id, 'published')"
                      :disabled="updatingChallengeId === item.id || destroyingChallengeId === item.id || item.status === 'published'"
                    >
                      发布
                    </button>
                    <button
                      class="ghost"
                      type="button"
                      @click="updateChallengeStatus(item.id, 'draft')"
                      :disabled="updatingChallengeId === item.id || destroyingChallengeId === item.id || item.status === 'draft'"
                    >
                      草稿
                    </button>
                    <button
                      class="ghost"
                      type="button"
                      @click="updateChallengeStatus(item.id, 'offline')"
                      :disabled="updatingChallengeId === item.id || destroyingChallengeId === item.id || item.status === 'offline'"
                    >
                      下线
                    </button>
                    <button
                      class="danger"
                      type="button"
                      @click="handleDestroyChallenge(item)"
                      :disabled="updatingChallengeId === item.id || destroyingChallengeId === item.id"
                    >
                      {{ destroyingChallengeId === item.id ? "销毁中..." : "销毁" }}
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

        <template v-if="challengeSubTab === 'lint'">
          <div class="row-between">
            <h3>运行模板校验</h3>
            <button
              class="ghost"
              type="button"
              @click="loadChallengeRuntimeLint()"
              :disabled="loadingChallengeRuntimeLint"
            >
              {{ loadingChallengeRuntimeLint ? "扫描中..." : "刷新校验" }}
            </button>
          </div>

          <div class="actions-row">
            <label>
              <span>题型</span>
              <select v-model="challengeLintTypeFilter">
                <option value="">all</option>
                <option value="static">static</option>
                <option value="dynamic">dynamic</option>
                <option value="internal">internal</option>
              </select>
            </label>
            <label>
              <span>状态</span>
              <select v-model="challengeLintStatusFilter">
                <option value="">all</option>
                <option value="draft">draft</option>
                <option value="published">published</option>
                <option value="offline">offline</option>
              </select>
            </label>
            <label>
              <span>关键词</span>
              <input v-model.trim="challengeLintKeywordFilter" placeholder="标题或 slug" />
            </label>
            <label>
              <span>条数</span>
              <input v-model.number="challengeLintLimit" type="number" min="1" max="5000" />
            </label>
            <label class="inline-check">
              <span>仅错误</span>
              <input v-model="challengeLintOnlyErrors" type="checkbox" />
            </label>
            <button
              class="ghost"
              type="button"
              @click="loadChallengeRuntimeLint()"
              :disabled="loadingChallengeRuntimeLint"
            >
              应用筛选
            </button>
          </div>

          <p v-if="challengeLintError" class="error">{{ challengeLintError }}</p>

          <div v-if="challengeRuntimeLint" class="challenge-lint-metrics">
            <article class="metric-card">
              <h4>扫描总数</h4>
              <p>{{ challengeRuntimeLint.scanned_total }}</p>
            </article>
            <article class="metric-card">
              <h4>通过</h4>
              <p>{{ challengeRuntimeLint.ok_count }}</p>
            </article>
            <article class="metric-card">
              <h4>错误</h4>
              <p>{{ challengeRuntimeLint.error_count }}</p>
            </article>
            <article class="metric-card">
              <h4>更新时间</h4>
              <p>{{ formatTime(challengeRuntimeLint.generated_at) }}</p>
            </article>
          </div>

          <table
            v-if="challengeLintItems.length > 0"
            class="scoreboard-table challenge-lint-table"
          >
            <thead>
              <tr>
                <th>题目</th>
                <th>题型</th>
                <th>状态</th>
                <th>模板</th>
                <th>校验</th>
                <th>更新时间</th>
                <th>信息</th>
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
          <p v-else class="muted">暂无匹配的模板校验记录。</p>
        </template>
      </section>

      <section v-if="adminModule === 'contests' && contestSubTab === 'contests'" class="panel">
        <div class="row-between">
          <h2>比赛管理</h2>
          <span class="badge">{{ contests.length }} 场</span>
        </div>

        <div class="module-split contest-split">
          <div class="module-column module-column-fill">
            <div class="row-between">
              <h3>{{ contestFormTitle }}</h3>
              <button
                v-if="editingContestId"
                class="ghost"
                type="button"
                @click="handleCancelContestEdit"
              >
                取消编辑
              </button>
            </div>
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
                {{ contestSubmitLabel }}
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
                <p class="muted" v-if="selectedContest.description">{{ selectedContest.description }}</p>

                <img
                  v-if="canPreviewContestPoster(selectedContest)"
                  class="contest-poster-preview"
                  :src="contestPosterPreviewUrl(selectedContest)"
                  alt="contest poster preview"
                />
                <p v-else class="muted">
                  {{ selectedContest.poster_url ? "该海报当前在比赛中心不可预览（仅 public 且 scheduled/running/ended 可见）。" : "当前未设置海报。" }}
                </p>

                <form class="form-grid" @submit.prevent="handleUploadContestPoster">
                  <label>
                    <span>上传比赛海报</span>
                    <input
                      :key="contestPosterInputKey"
                      type="file"
                      accept="image/*"
                      @change="onContestPosterFileChange"
                      required
                    />
                  </label>
                  <div class="actions-row compact-actions">
                    <button
                      class="ghost"
                      type="submit"
                      :disabled="uploadingContestPoster || !selectedContestPosterFile"
                    >
                      {{ uploadingContestPoster ? "上传中..." : "上传海报" }}
                    </button>
                    <button
                      class="danger"
                      type="button"
                      @click="handleDeleteContestPoster(selectedContest)"
                      :disabled="deletingContestPosterId === selectedContest.id || !selectedContest.poster_url"
                    >
                      {{ deletingContestPosterId === selectedContest.id ? "删除中..." : "删除海报" }}
                    </button>
                  </div>
                </form>

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
                    编辑比赛配置
                  </button>
                  <button class="ghost" type="button" @click="contestSubTab = 'bindings'">
                    管理题目挂载
                  </button>
                  <button class="ghost" type="button" @click="contestSubTab = 'announcements'">
                    管理公告
                  </button>
                  <button
                    class="danger"
                    type="button"
                    @click="handleDestroyContest(selectedContest)"
                    :disabled="destroyingContestId === selectedContest.id || updatingContestId === selectedContest.id"
                  >
                    {{ destroyingContestId === selectedContest.id ? "销毁中..." : "销毁比赛" }}
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
        <div class="actions-row compact-actions">
          <span v-if="runtimeOverview" class="muted">更新于 {{ formatTime(runtimeOverview.generated_at) }}</span>
          <button
            class="ghost"
            type="button"
            @click="handleRunExpiredReaper"
            :disabled="runtimeReaperBusy !== ''"
          >
            {{ runtimeReaperBusy === "expired" ? "回收中..." : "执行过期回收" }}
          </button>
          <button
            class="ghost"
            type="button"
            @click="handleRunStaleReaper"
            :disabled="runtimeReaperBusy !== ''"
          >
            {{ runtimeReaperBusy === "stale" ? "回收中..." : "执行心跳超时回收" }}
          </button>
        </div>
      </div>

      <p v-if="runtimeError" class="error">{{ runtimeError }}</p>
      <p v-if="runtimeReaperError" class="error">{{ runtimeReaperError }}</p>
      <p v-if="runtimeReaperResult" class="muted mono">
        最近回收：mode={{ runtimeReaperResult.mode }} · scanned={{ runtimeReaperResult.scanned }} ·
        reaped={{ runtimeReaperResult.reaped }} · failed={{ runtimeReaperResult.failed }} ·
        updated_at={{ formatTime(runtimeReaperResult.generated_at) }}
      </p>

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

    <section v-if="adminModule === 'operations' && operationsSubTab === 'alerts'" class="panel">
      <div class="row-between">
        <h2>运行告警</h2>
        <div class="actions-row compact-actions">
          <button class="ghost" type="button" @click="loadRuntimeAlerts()" :disabled="loadingRuntimeAlerts">
            {{ loadingRuntimeAlerts ? "加载中..." : "刷新告警" }}
          </button>
          <button class="primary" type="button" @click="handleScanRuntimeAlerts" :disabled="runtimeAlertScanBusy">
            {{ runtimeAlertScanBusy ? "扫描中..." : "触发扫描" }}
          </button>
        </div>
      </div>

      <div class="actions-row">
        <label>
          <span>状态</span>
          <select v-model="runtimeAlertStatusFilter">
            <option value="">all</option>
            <option value="open">open</option>
            <option value="acknowledged">acknowledged</option>
            <option value="resolved">resolved</option>
          </select>
        </label>
        <label>
          <span>级别</span>
          <select v-model="runtimeAlertSeverityFilter">
            <option value="">all</option>
            <option value="info">info</option>
            <option value="warning">warning</option>
            <option value="critical">critical</option>
          </select>
        </label>
        <label>
          <span>告警类型</span>
          <input v-model.trim="runtimeAlertTypeFilter" placeholder="instance_heartbeat_stale" />
        </label>
        <label>
          <span>条数</span>
          <input v-model.number="runtimeAlertLimit" type="number" min="1" max="500" />
        </label>
        <button class="ghost" type="button" @click="loadRuntimeAlerts()" :disabled="loadingRuntimeAlerts">
          应用筛选
        </button>
      </div>

      <p v-if="runtimeAlertError" class="error">{{ runtimeAlertError }}</p>

      <div class="runtime-alert-layout">
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
            <p class="muted runtime-alert-line">状态 {{ item.status }} · 最近 {{ formatTime(item.last_seen_at) }}</p>
          </button>
          <p v-if="runtimeAlerts.length === 0" class="muted">暂无运行告警。</p>
        </div>

        <div class="runtime-alert-detail">
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
              <p>首次发现：{{ formatTime(selectedRuntimeAlert.first_seen_at) }}</p>
              <p>最近发现：{{ formatTime(selectedRuntimeAlert.last_seen_at) }}</p>
              <p>确认人：{{ selectedRuntimeAlert.acknowledged_by_username ?? "-" }}</p>
              <p>恢复人：{{ selectedRuntimeAlert.resolved_by_username ?? "-" }}</p>
            </div>

            <label>
              <span>处理备注（可选）</span>
              <input
                v-model.trim="runtimeAlertActionNote"
                placeholder="用于 ack / resolve 审计记录"
              />
            </label>

            <div class="actions-row">
              <button
                class="ghost"
                type="button"
                @click="handleAcknowledgeRuntimeAlert(selectedRuntimeAlert)"
                :disabled="
                  runtimeAlertUpdatingId === selectedRuntimeAlert.id ||
                  selectedRuntimeAlert.status !== 'open'
                "
              >
                {{ runtimeAlertUpdatingId === selectedRuntimeAlert.id ? "处理中..." : "确认告警" }}
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
                {{ runtimeAlertUpdatingId === selectedRuntimeAlert.id ? "处理中..." : "标记恢复" }}
              </button>
            </div>

            <details class="runtime-alert-detail-json">
              <summary>展示详细信息（JSON）</summary>
              <pre class="mono">{{ formatJson(selectedRuntimeAlert.detail) }}</pre>
            </details>
          </template>
          <p v-else class="muted">从左侧选择一个告警查看详情。</p>
        </div>
      </div>
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
                    updatingUserRoleId === item.id ||
                    deletingUserAccountId === item.id
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
                    updatingUserRoleId === item.id ||
                    deletingUserAccountId === item.id
                  "
                  @click="handleResetUserPassword(item)"
                >
                  {{ resettingUserId === item.id ? "重置中..." : "重置密码" }}
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
                  {{ deletingUserAccountId === item.id ? "删除中..." : "删除账号" }}
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
            <th>操作</th>
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
            <td>
              <button
                class="ghost"
                type="button"
                @click="loadInstanceRuntimeMetrics(item.id)"
                :disabled="loadingInstanceRuntimeMetricsId === item.id"
              >
                {{
                  loadingInstanceRuntimeMetricsId === item.id
                    ? "加载中..."
                    : selectedInstanceId === item.id
                      ? "刷新指标"
                      : "查看指标"
                }}
              </button>
            </td>
          </tr>
        </tbody>
      </table>
      <p v-else class="muted">暂无实例记录。</p>

      <section v-if="selectedInstanceRuntimeMetrics" class="instance-metrics-panel">
        <div class="row-between">
          <h3>实例指标：{{ selectedInstance?.team_name ?? selectedInstanceRuntimeMetrics.instance.team_name }}</h3>
          <span class="muted">更新于 {{ formatTime(selectedInstanceRuntimeMetrics.generated_at) }}</span>
        </div>
        <p class="muted mono">
          project={{ selectedInstanceRuntimeMetrics.instance.compose_project_name }} ·
          status={{ selectedInstanceRuntimeMetrics.instance.status }}
        </p>

        <div class="runtime-metrics">
          <article class="metric-card">
            <h3>服务状态</h3>
            <p>总服务 {{ selectedInstanceRuntimeMetrics.summary.services_total }}</p>
            <p>运行中 {{ selectedInstanceRuntimeMetrics.summary.running_services }}</p>
            <p>不健康 {{ selectedInstanceRuntimeMetrics.summary.unhealthy_services }}</p>
          </article>
          <article class="metric-card">
            <h3>资源汇总</h3>
            <p>CPU 总计 {{ formatPercentValue(selectedInstanceRuntimeMetrics.summary.cpu_percent_total) }}</p>
            <p>
              内存 {{ formatResourceBytes(selectedInstanceRuntimeMetrics.summary.memory_usage_bytes_total) }} /
              {{ formatResourceBytes(selectedInstanceRuntimeMetrics.summary.memory_limit_bytes_total) }}
            </p>
            <p>重启中服务 {{ selectedInstanceRuntimeMetrics.summary.restarting_services }}</p>
          </article>
        </div>

        <p
          v-for="warning in selectedInstanceRuntimeMetrics.warnings"
          :key="warning"
          class="muted mono"
        >
          warning: {{ warning }}
        </p>

        <table v-if="selectedInstanceRuntimeMetrics.services.length > 0" class="scoreboard-table">
          <thead>
            <tr>
              <th>服务</th>
              <th>状态</th>
              <th>CPU</th>
              <th>内存</th>
              <th>网络 RX/TX</th>
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
      </section>
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
  buildApiAssetUrl,
  createAdminChallenge,
  createAdminContestAnnouncement,
  createAdminContest,
  acknowledgeAdminRuntimeAlert,
  deleteAdminChallenge,
  deleteAdminChallengeAttachment,
  deleteAdminContest,
  deleteAdminContestAnnouncement,
  deleteAdminContestPoster,
  deleteAdminContestChallenge,
  deleteAdminUser,
  getAdminInstanceRuntimeMetrics,
  getAdminRuntimeOverview,
  getAdminChallengeDetail,
  listAdminRuntimeAlerts,
  listAdminChallengeRuntimeTemplateLint,
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
  resolveAdminRuntimeAlert,
  rollbackAdminChallengeVersion,
  runAdminExpiredInstanceReaper,
  runAdminStaleInstanceReaper,
  scanAdminRuntimeAlerts,
  testAdminChallengeRuntimeImage,
  uploadAdminContestPoster,
  uploadAdminChallengeAttachment,
  updateAdminContestAnnouncement,
  updateAdminUserRole,
  updateAdminUserStatus,
  type AdminChallengeAttachmentItem,
  type AdminChallengeDetailItem,
  type AdminAuditLogItem,
  type AdminChallengeItem,
  type AdminChallengeRuntimeImageTestResponse,
  type AdminChallengeRuntimeLintItem,
  type AdminChallengeRuntimeLintResponse,
  type AdminChallengeVersionItem,
  type AdminContestAnnouncementItem,
  type AdminContestChallengeItem,
  type AdminContestItem,
  type AdminInstanceItem,
  type AdminInstanceReaperRunResponse,
  type AdminInstanceRuntimeMetricsResponse,
  type AdminRuntimeAlertItem,
  type AdminRuntimeOverview,
  type AdminUserItem,
  updateAdminChallenge,
  updateAdminContest,
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
const challengeRuntimeLint = ref<AdminChallengeRuntimeLintResponse | null>(null);
const contests = ref<AdminContestItem[]>([]);
const contestBindings = ref<AdminContestChallengeItem[]>([]);
const contestAnnouncements = ref<AdminContestAnnouncementItem[]>([]);
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
const selectedRuntimeAlertId = ref("");
const selectedInstanceId = ref("");
const editingChallengeId = ref("");
const editingContestId = ref("");
const adminModule = ref<"challenges" | "contests" | "operations" | "users" | "audit">("challenges");
const challengeSubTab = ref<"library" | "versions" | "lint">("library");
const contestSubTab = ref<"contests" | "bindings" | "announcements">("contests");
const operationsSubTab = ref<"runtime" | "alerts" | "instances">("runtime");

const pageError = ref("");
const challengeError = ref("");
const challengeVersionError = ref("");
const challengeAttachmentError = ref("");
const challengeLintError = ref("");
const challengeImageTestError = ref("");
const contestError = ref("");
const bindingError = ref("");
const announcementError = ref("");
const instanceError = ref("");
const userError = ref("");
const auditError = ref("");
const runtimeError = ref("");
const runtimeAlertError = ref("");
const runtimeReaperError = ref("");

const refreshing = ref(false);
const creatingChallenge = ref(false);
const creatingContest = ref(false);
const updatingChallengeId = ref("");
const destroyingChallengeId = ref("");
const rollingBack = ref(false);
const uploadingAttachment = ref(false);
const deletingAttachmentId = ref("");
const loadingChallengeRuntimeLint = ref(false);
const testingChallengeRuntimeImage = ref(false);
const updatingContestId = ref("");
const destroyingContestId = ref("");
const uploadingContestPoster = ref(false);
const deletingContestPosterId = ref("");
const bindingBusy = ref(false);
const creatingAnnouncement = ref(false);
const updatingAnnouncementId = ref("");
const deletingAnnouncementId = ref("");
const savingAnnouncementId = ref("");
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
const resetPasswords = reactive<Record<string, string>>({});
const roleDrafts = reactive<Record<string, string>>({});
const announcementDrafts = reactive<Record<string, { title: string; content: string }>>({});
const attachmentInputKey = ref(0);
const selectedAttachmentFile = ref<File | null>(null);
const contestPosterInputKey = ref(0);
const selectedContestPosterFile = ref<File | null>(null);
const challengeRuntimeImageTestResult = ref<AdminChallengeRuntimeImageTestResponse | null>(null);
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
  writeup_visibility: "hidden",
  writeup_content: "",
  change_note: "",
  compose_template: "",
  runtime_mode: "compose",
  runtime_access_mode: "ssh_bastion",
  runtime_image: "",
  runtime_internal_port: 80,
  runtime_protocol: "http",
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

const selectedRuntimeAlert = computed(() => {
  return runtimeAlerts.value.find((item) => item.id === selectedRuntimeAlertId.value) ?? null;
});

const selectedInstance = computed(() => {
  return instances.value.find((item) => item.id === selectedInstanceId.value) ?? null;
});

const challengeLintItems = computed<AdminChallengeRuntimeLintItem[]>(() => {
  return challengeRuntimeLint.value?.items ?? [];
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
  return editingChallengeId.value ? "编辑题目" : "创建题目";
});

const challengeSubmitLabel = computed(() => {
  if (creatingChallenge.value) {
    return editingChallengeId.value ? "保存中..." : "创建中...";
  }
  return editingChallengeId.value ? "保存修改" : "创建题目";
});

const contestFormTitle = computed(() => {
  return editingContestId.value ? "编辑比赛" : "创建比赛";
});

const contestSubmitLabel = computed(() => {
  if (creatingContest.value) {
    return editingContestId.value ? "保存中..." : "创建中...";
  }
  return editingContestId.value ? "保存修改" : "创建比赛";
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

function resetChallengeForm() {
  editingChallengeId.value = "";
  newChallenge.title = "";
  newChallenge.slug = "";
  newChallenge.category = "web";
  newChallenge.description = "";
  newChallenge.difficulty = "normal";
  newChallenge.static_score = 100;
  newChallenge.status = "draft";
  newChallenge.challenge_type = "static";
  newChallenge.flag_mode = "static";
  newChallenge.flag_hash = "";
  newChallenge.tags_input = "";
  newChallenge.writeup_visibility = "hidden";
  newChallenge.writeup_content = "";
  newChallenge.change_note = "";
  newChallenge.compose_template = "";
  newChallenge.runtime_mode = "compose";
  newChallenge.runtime_access_mode = "ssh_bastion";
  newChallenge.runtime_image = "";
  newChallenge.runtime_internal_port = 80;
  newChallenge.runtime_protocol = "http";
  challengeImageTestError.value = "";
  challengeRuntimeImageTestResult.value = null;
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
  newContest.start_at = defaultStart;
  newContest.end_at = defaultEnd;
  newContest.freeze_at = "";
}

function applyChallengeDetailToForm(detail: AdminChallengeDetailItem) {
  const runtime = (detail.metadata?.runtime ?? {}) as Record<string, unknown>;
  const runtimeModeRaw = typeof runtime.mode === "string" ? runtime.mode.trim().toLowerCase() : "";
  const runtimeMode = runtimeModeRaw === "single_image" || runtimeModeRaw === "single-image" || runtimeModeRaw === "image"
    ? "single_image"
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
  newChallenge.writeup_visibility = detail.writeup_visibility;
  newChallenge.writeup_content = detail.writeup_content ?? "";
  newChallenge.change_note = "";
  newChallenge.runtime_mode = runtimeMode;
  newChallenge.runtime_access_mode = runtimeAccessMode;
  newChallenge.runtime_image = typeof runtime.image === "string" ? runtime.image : "";
  newChallenge.runtime_internal_port = runtimeInternalPort;
  newChallenge.runtime_protocol = runtimeProtocol;
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
  newContest.start_at = isoToLocalInput(item.start_at);
  newContest.end_at = isoToLocalInput(item.end_at);
  newContest.freeze_at = item.freeze_at ? isoToLocalInput(item.freeze_at) : "";
}

function buildChallengeRuntimeMetadata() {
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

function onAttachmentFileChange(event: Event) {
  const target = event.target as HTMLInputElement | null;
  selectedAttachmentFile.value = target?.files?.[0] ?? null;
}

function onContestPosterFileChange(event: Event) {
  const target = event.target as HTMLInputElement | null;
  selectedContestPosterFile.value = target?.files?.[0] ?? null;
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
    challengeLintError.value = err instanceof ApiClientError ? err.message : "加载模板校验结果失败";
    if (!options?.silentError) {
      uiStore.error("加载模板校验结果失败", challengeLintError.value);
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
    }
  } catch (err) {
    instanceError.value = err instanceof ApiClientError ? err.message : "加载实例失败";
    uiStore.error("加载实例失败", instanceError.value);
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
    const message = err instanceof ApiClientError ? err.message : "加载实例运行指标失败";
    if (!options?.silentError) {
      instanceError.value = message;
      uiStore.error("加载实例运行指标失败", message);
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
    runtimeAlertError.value = err instanceof ApiClientError ? err.message : "加载运行告警失败";
    if (!options?.silentError) {
      uiStore.error("加载运行告警失败", runtimeAlertError.value);
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
    uiStore.success(
      "运行告警扫描完成",
      `新增/更新 ${summary.upserted}，自动恢复 ${summary.auto_resolved}，open ${summary.open_count}。`,
      3200
    );
  } catch (err) {
    runtimeAlertError.value = err instanceof ApiClientError ? err.message : "触发运行告警扫描失败";
    uiStore.error("触发运行告警扫描失败", runtimeAlertError.value);
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
    uiStore.success(
      "过期实例回收已执行",
      `扫描 ${result.scanned}，回收 ${result.reaped}，失败 ${result.failed}。`,
      3600
    );
  } catch (err) {
    runtimeReaperError.value =
      err instanceof ApiClientError ? err.message : "执行过期实例回收失败";
    uiStore.error("执行过期实例回收失败", runtimeReaperError.value);
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
    uiStore.success(
      "心跳超时实例回收已执行",
      `阈值 ${result.heartbeat_stale_seconds ?? "-"} 秒，扫描 ${result.scanned}，回收 ${result.reaped}。`,
      3800
    );
  } catch (err) {
    runtimeReaperError.value =
      err instanceof ApiClientError ? err.message : "执行心跳超时实例回收失败";
    uiStore.error("执行心跳超时实例回收失败", runtimeReaperError.value);
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
    uiStore.info("告警已确认", item.title);
  } catch (err) {
    runtimeAlertError.value = err instanceof ApiClientError ? err.message : "确认运行告警失败";
    uiStore.error("确认运行告警失败", runtimeAlertError.value);
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
    uiStore.success("告警已恢复", item.title);
  } catch (err) {
    runtimeAlertError.value = err instanceof ApiClientError ? err.message : "恢复运行告警失败";
    uiStore.error("恢复运行告警失败", runtimeAlertError.value);
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
      loadRuntimeAlerts(),
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

async function handleLoadChallengeForEdit(challengeId: string) {
  challengeError.value = "";
  challengeImageTestError.value = "";

  try {
    const detail = await getAdminChallengeDetail(challengeId, accessTokenOrThrow());
    applyChallengeDetailToForm(detail);
    challengeRuntimeImageTestResult.value = null;
    uiStore.info("已载入题目配置", `正在编辑：${detail.title}`);
  } catch (err) {
    challengeError.value = err instanceof ApiClientError ? err.message : "加载题目详情失败";
    uiStore.error("加载题目详情失败", challengeError.value);
  }
}

function handleCancelChallengeEdit() {
  resetChallengeForm();
}

async function handleTestChallengeRuntimeImage() {
  challengeImageTestError.value = "";
  challengeRuntimeImageTestResult.value = null;

  if (newChallenge.runtime_mode !== "single_image") {
    challengeImageTestError.value = "仅 single_image 模式支持镜像测试";
    uiStore.warning("无法测试镜像", challengeImageTestError.value);
    return;
  }

  if (!newChallenge.runtime_image.trim()) {
    challengeImageTestError.value = "请先填写镜像仓库地址";
    uiStore.warning("镜像为空", challengeImageTestError.value);
    return;
  }

  testingChallengeRuntimeImage.value = true;
  try {
    const result = await testAdminChallengeRuntimeImage(
      {
        image: newChallenge.runtime_image.trim(),
        force_pull: true,
        run_build_probe: true
      },
      accessTokenOrThrow()
    );
    challengeRuntimeImageTestResult.value = result;
    if (result.succeeded) {
      uiStore.success("镜像测试通过", result.image);
    } else {
      uiStore.warning("镜像测试失败", result.image);
    }
  } catch (err) {
    challengeImageTestError.value = err instanceof ApiClientError ? err.message : "镜像测试失败";
    uiStore.error("镜像测试失败", challengeImageTestError.value);
  } finally {
    testingChallengeRuntimeImage.value = false;
  }
}

async function handleCreateChallenge() {
  creatingChallenge.value = true;
  challengeError.value = "";

  try {
    const isEditMode = !!editingChallengeId.value;
    if (newChallenge.runtime_mode === "single_image") {
      if (!newChallenge.runtime_image.trim()) {
        challengeError.value = "single_image 模式必须填写镜像仓库地址";
        return;
      }
      if (
        !Number.isFinite(newChallenge.runtime_internal_port) ||
        newChallenge.runtime_internal_port < 1 ||
        newChallenge.runtime_internal_port > 65535
      ) {
        challengeError.value = "single_image 模式内部端口必须在 1~65535";
        return;
      }
      if (newChallenge.challenge_type === "static") {
        challengeError.value = "single_image 模式仅支持 dynamic 或 internal 题型";
        return;
      }
    }

    if (
      newChallenge.runtime_mode === "compose" &&
      (newChallenge.challenge_type === "dynamic" || newChallenge.challenge_type === "internal") &&
      !newChallenge.compose_template.trim()
    ) {
      challengeError.value = "dynamic/internal 题型在 compose 模式下必须提供 compose 模板";
      return;
    }

    const runtimeMetadata = buildChallengeRuntimeMetadata();

    const payload = {
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
      compose_template:
        newChallenge.runtime_mode === "compose"
          ? newChallenge.compose_template || undefined
          : undefined,
      metadata: runtimeMetadata,
      tags: parseTagsInput(newChallenge.tags_input),
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

    await loadChallenges();
    uiStore.success(
      isEditMode ? "题目已更新" : "题目已创建",
      "可以继续管理版本、附件或挂载到比赛。"
    );
  } catch (err) {
    challengeError.value = err instanceof ApiClientError ? err.message : "保存题目失败";
    uiStore.error("保存题目失败", challengeError.value);
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

async function handleDestroyChallenge(item: AdminChallengeItem) {
  if (!window.confirm(`确认销毁题目「${item.title}」？该操作会删除题目、挂载关系、提交记录与运行实例。`)) {
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
    uiStore.warning("题目已销毁", item.title);
  } catch (err) {
    challengeError.value = err instanceof ApiClientError ? err.message : "销毁题目失败";
    uiStore.error("销毁题目失败", challengeError.value);
  } finally {
    destroyingChallengeId.value = "";
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

function handleLoadContestForEdit(item: AdminContestItem) {
  applyContestToForm(item);
  contestError.value = "";
  uiStore.info("已载入比赛配置", `正在编辑：${item.title}`);
}

function handleCancelContestEdit() {
  resetContestForm();
}

async function handleCreateContest() {
  creatingContest.value = true;
  contestError.value = "";

  try {
    const isEditMode = !!editingContestId.value;
    if (!Number.isFinite(newContest.dynamic_decay) || newContest.dynamic_decay < 1) {
      throw new ApiClientError("dynamic_decay 必须为大于等于 1 的整数", "bad_request");
    }

    const payload = {
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

    await loadContests();
    selectedContestId.value = targetContestId;
    await Promise.all([loadContestBindings(), loadContestAnnouncements()]);
    uiStore.success(
      isEditMode ? "比赛已更新" : "比赛已创建",
      "可以继续挂载题目并调整状态。"
    );
  } catch (err) {
    contestError.value = err instanceof ApiClientError ? err.message : "保存比赛失败";
    uiStore.error("保存比赛失败", contestError.value);
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

async function handleUploadContestPoster() {
  if (!selectedContestId.value) {
    contestError.value = "请先选择比赛";
    uiStore.warning("未选择比赛", contestError.value);
    return;
  }

  if (!selectedContestPosterFile.value) {
    contestError.value = "请先选择海报文件";
    uiStore.warning("未选择海报", contestError.value);
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
    uiStore.success("海报已上传", file.name);
  } catch (err) {
    contestError.value = err instanceof ApiClientError ? err.message : "上传比赛海报失败";
    uiStore.error("上传比赛海报失败", contestError.value);
  } finally {
    uploadingContestPoster.value = false;
  }
}

async function handleDeleteContestPoster(item: AdminContestItem) {
  if (!item.poster_url) {
    return;
  }

  if (!window.confirm(`确认删除比赛「${item.title}」的海报？`)) {
    return;
  }

  deletingContestPosterId.value = item.id;
  contestError.value = "";

  try {
    await deleteAdminContestPoster(item.id, accessTokenOrThrow());
    await loadContests();
    uiStore.warning("海报已删除", item.title);
  } catch (err) {
    contestError.value = err instanceof ApiClientError ? err.message : "删除比赛海报失败";
    uiStore.error("删除比赛海报失败", contestError.value);
  } finally {
    deletingContestPosterId.value = "";
  }
}

async function handleDestroyContest(item: AdminContestItem) {
  if (!window.confirm(`确认销毁比赛「${item.title}」？该操作将删除比赛、公告、挂载、提交与实例数据。`)) {
    return;
  }

  destroyingContestId.value = item.id;
  contestError.value = "";

  try {
    await deleteAdminContest(item.id, accessTokenOrThrow());
    if (selectedContestId.value === item.id) {
      selectedContestId.value = "";
      contestBindings.value = [];
      contestAnnouncements.value = [];
      selectedBindingChallengeId.value = "";
      selectedAnnouncementId.value = "";
    }
    await Promise.all([loadContests(), loadContestBindings(), loadContestAnnouncements()]);
    uiStore.warning("比赛已销毁", item.title);
  } catch (err) {
    contestError.value = err instanceof ApiClientError ? err.message : "销毁比赛失败";
    uiStore.error("销毁比赛失败", contestError.value);
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

async function handleDeleteUserAccount(item: AdminUserItem) {
  if (!window.confirm(`确认删除账号「${item.username}」？该操作会禁用并匿名化该账号。`)) {
    return;
  }

  deletingUserAccountId.value = item.id;
  userError.value = "";

  try {
    await deleteAdminUser(item.id, accessTokenOrThrow());
    await loadUsers();
    uiStore.warning("账号已删除", `${item.username} 已被禁用并匿名化。`);
  } catch (err) {
    userError.value = err instanceof ApiClientError ? err.message : "删除账号失败";
    uiStore.error("删除账号失败", userError.value);
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
    selectedContestPosterFile.value = null;
    contestPosterInputKey.value += 1;
    bindingForm.challenge_id = "";
    bindingForm.sort_order = 0;
    bindingForm.release_at = "";
    loadContestBindings();
    loadContestAnnouncements();
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
.admin-grid {
  grid-template-columns: minmax(0, 1fr);
}

.module-tabs {
  margin-top: 0.75rem;
}

.module-tabs .ghost.active {
  border-color: var(--brand-strong);
  color: var(--brand-strong);
  background: rgba(64, 132, 255, 0.12);
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
  border: 1px solid rgba(96, 120, 160, 0.24);
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.58);
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
  border-color: var(--brand-strong);
  box-shadow: 0 0 0 2px rgba(45, 107, 255, 0.16);
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
  border: 1px solid rgba(96, 120, 160, 0.28);
  border-radius: 10px;
  padding: 0.6rem 0.62rem;
  background: rgba(255, 255, 255, 0.82);
  display: grid;
  gap: 0.2rem;
  cursor: pointer;
  transition: border-color 140ms ease, background-color 140ms ease;
}

.contest-list-item.active {
  border-color: var(--brand-strong);
  background: rgba(64, 132, 255, 0.12);
}

.contest-detail-pane {
  border: 1px solid rgba(96, 120, 160, 0.28);
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.82);
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

.image-test-block {
  grid-column: 1 / -1;
  border: 1px solid rgba(96, 120, 160, 0.24);
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.58);
  padding: 0.58rem;
}

.image-test-result {
  display: grid;
  gap: 0.45rem;
}

.image-test-result.failed {
  border-left: 3px solid rgba(185, 28, 28, 0.6);
  padding-left: 0.45rem;
}

.image-test-step {
  border: 1px solid rgba(96, 120, 160, 0.24);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.82);
  padding: 0.4rem 0.5rem;
}

.image-test-step summary {
  cursor: pointer;
  color: var(--brand-strong);
}

.image-test-step pre {
  margin: 0.45rem 0 0;
  max-height: 220px;
  overflow: auto;
  background: rgba(255, 255, 255, 0.58);
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
  border: 1px solid rgba(96, 120, 160, 0.28);
  background: rgba(255, 255, 255, 0.58);
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
  border-color: rgba(22, 163, 74, 0.4);
  color: #166534;
  background: rgba(22, 163, 74, 0.08);
}

.lint-badge-error {
  border-color: rgba(185, 28, 28, 0.4);
  color: #991b1b;
  background: rgba(185, 28, 28, 0.08);
}

.runtime-alert-layout {
  display: grid;
  grid-template-columns: minmax(260px, 0.9fr) minmax(0, 1.5fr);
  gap: 0.8rem;
  min-height: 420px;
}

.instance-metrics-panel {
  margin-top: 0.8rem;
  border: 1px solid rgba(96, 120, 160, 0.28);
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.82);
  padding: 0.75rem;
  display: grid;
  gap: 0.65rem;
}

.runtime-alert-list {
  border: 1px solid rgba(96, 120, 160, 0.28);
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.58);
  padding: 0.6rem;
  display: grid;
  gap: 0.45rem;
  overflow: auto;
  min-height: 0;
  align-content: start;
}

.runtime-alert-item {
  text-align: left;
  border: 1px solid rgba(96, 120, 160, 0.28);
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.82);
  padding: 0.58rem 0.6rem;
  display: grid;
  gap: 0.2rem;
  cursor: pointer;
  transition: border-color 130ms ease, transform 130ms ease;
}

.runtime-alert-item:hover {
  border-color: var(--brand-strong);
  transform: translateY(-1px);
}

.runtime-alert-item.active {
  border-color: var(--brand-strong);
  box-shadow: 0 0 0 2px rgba(45, 107, 255, 0.16);
}

.runtime-alert-item.severity-critical {
  border-left: 3px solid var(--danger);
}

.runtime-alert-item.severity-warning {
  border-left: 3px solid var(--warning);
}

.runtime-alert-item.severity-info {
  border-left: 3px solid var(--brand-strong);
}

.runtime-alert-title {
  margin-right: 0.5rem;
}

.runtime-alert-line {
  margin: 0;
}

.runtime-alert-detail {
  border: 1px solid rgba(96, 120, 160, 0.28);
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.82);
  padding: 0.8rem;
  min-height: 0;
  overflow: auto;
  display: grid;
  gap: 0.65rem;
  align-content: start;
}

.runtime-alert-detail h3 {
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
  border: 1px solid rgba(96, 120, 160, 0.24);
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.58);
  padding: 0.5rem 0.55rem;
}

.runtime-alert-meta p {
  margin: 0.16rem 0;
}

.runtime-alert-detail-json summary {
  cursor: pointer;
  color: var(--brand-strong);
  font-weight: 600;
}

.runtime-alert-detail-json pre {
  margin: 0.5rem 0 0;
  max-height: 260px;
  overflow: auto;
  border: 1px solid rgba(96, 120, 160, 0.24);
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.58);
  padding: 0.58rem;
  white-space: pre-wrap;
  word-break: break-word;
}

@media (max-width: 1220px) {
  .challenge-split,
  .contest-split {
    grid-template-columns: 1fr;
  }

  .contest-browser {
    grid-template-columns: 1fr;
  }

  .runtime-alert-layout {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 860px) {
  .compact-grid {
    grid-template-columns: 1fr;
  }
}
</style>
