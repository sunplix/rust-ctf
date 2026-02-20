<template>
  <section class="page-block">
    <header class="page-head">
      <div>
        <p class="soft mono">RUST CTF PLATFORM</p>
        <h1>平台概览</h1>
        <p class="muted">多容器靶场、实时榜单与统一竞赛工作流。</p>
      </div>
      <RouterLink class="btn-link" to="/contests">进入比赛中心</RouterLink>
    </header>

    <div class="overview-grid">
      <article class="panel overview-card">
        <div class="row-between">
          <h2>选手侧</h2>
          <svg class="line-icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8Zm-6.5 8c.6-3.6 3-5.5 6.5-5.5s5.9 1.9 6.5 5.5" />
          </svg>
        </div>
        <p class="muted">比赛浏览、题目提交、环境管理、公告接收、实时榜单。</p>
      </article>

      <article class="panel overview-card">
        <div class="row-between">
          <h2>管理侧</h2>
          <svg class="line-icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 3.8 4.5 7.2v4.8c0 4.2 2.8 7.4 7.5 8.8 4.7-1.4 7.5-4.6 7.5-8.8V7.2L12 3.8Z" />
            <path d="M9.5 12h5m-2.5-2.5v5" />
          </svg>
        </div>
        <p class="muted">题库版本、赛事配置、挂载管理、运行监控与审计追踪。</p>
      </article>

      <article class="panel overview-card">
        <div class="row-between">
          <h2>实时层</h2>
          <svg class="line-icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M4.5 12h3l2-4 3 8 2.5-5h4.5" />
          </svg>
        </div>
        <p class="muted">WebSocket 推送榜单变化，环境状态与告警集中反馈。</p>
      </article>
    </div>
  </section>
</template>

<script setup lang="ts"></script>

<style scoped>
.overview-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.86rem;
}

.overview-card {
  display: grid;
  gap: 0.5rem;
  align-content: start;
}

@media (max-width: 980px) {
  .overview-grid {
    grid-template-columns: 1fr;
  }
}
</style>
