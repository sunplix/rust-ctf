<template>
  <section class="page-layout home-shell">
    <article class="surface surface-dashed stack home-main">
      <header class="home-head">
        <p class="eyebrow">{{ tr("站点首页", "Home") }}</p>
        <h1>{{ appStore.siteSettings.home_title }}</h1>
      </header>

      <p class="lead">{{ appStore.siteSettings.home_tagline }}</p>

      <div class="home-chips">
        <span class="chip">{{ tr("赛事管理", "Contest Hub") }}</span>
        <span class="chip">{{ tr("队伍协作", "Team Space") }}</span>
        <span class="chip">{{ tr("实时看板", "Live Board") }}</span>
      </div>

      <section class="home-metrics">
        <div class="metric">
          <span>{{ tr("流程", "Flow") }}</span>
          <strong class="mono">{{ tr("登录 · 组队 · 参赛", "Sign In · Team · Play") }}</strong>
        </div>
        <div class="metric">
          <span>{{ tr("体验", "Experience") }}</span>
          <strong class="mono">{{ tr("轻量 · 稳定", "Clean · Stable") }}</strong>
        </div>
        <div class="metric">
          <span>{{ tr("节奏", "Rhythm") }}</span>
          <strong>{{ tr("专注解题", "Focus on Solves") }}</strong>
        </div>
      </section>
    </article>

    <article class="surface surface-dashed stack home-side">
      <header class="side-head">
        <h2>{{ tr("今日签名", "Signature") }}</h2>
      </header>

      <blockquote class="signature">
        “{{ appStore.siteSettings.home_signature }}”
      </blockquote>

      <p class="soft">
        {{ tr("准备好后，从右上角进入登录。", "Use the top-right entry when you're ready.") }}
      </p>

      <div class="side-actions">
        <RouterLink class="btn-solid" to="/auth">{{ tr("进入登录", "Sign In") }}</RouterLink>
      </div>
    </article>
  </section>
</template>

<script setup lang="ts">
import { useL10n } from "../composables/useL10n";
import { useAppStore } from "../stores/app";

const { tr } = useL10n();
const appStore = useAppStore();
</script>

<style scoped>
.home-shell {
  min-height: min(720px, calc(100vh - 176px));
  display: grid;
  gap: 0.9rem;
  grid-template-columns: minmax(0, 1.15fr) minmax(0, 0.85fr);
}

.home-main,
.home-side {
  min-height: 100%;
}

.home-main {
  display: grid;
  gap: 0.72rem;
}

.home-head {
  display: grid;
  gap: 0.2rem;
}

.eyebrow {
  font-size: 0.76rem;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--fg-2);
}

.home-head h1 {
  font-size: clamp(1.4rem, 2.2vw, 2rem);
}

.lead {
  font-size: 1rem;
  line-height: 1.65;
}

.home-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.42rem;
}

.chip {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 1.8rem;
  padding: 0.28rem 0.66rem;
  border-radius: 999px;
  background: var(--glass-mid);
  box-shadow: inset 0 -1px 0 var(--line-soft);
  font-size: 0.82rem;
}

.home-metrics {
  display: grid;
  gap: 0.52rem;
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

.metric {
  border-radius: var(--radius-md);
  background: var(--glass-mid);
  box-shadow: inset 0 -1px 0 var(--line-soft);
  padding: 0.6rem 0.66rem;
  display: grid;
  gap: 0.14rem;
}

.metric span {
  color: var(--fg-2);
  font-size: 0.72rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
}

.metric strong {
  font-size: 0.9rem;
}

.home-side {
  display: grid;
  align-content: start;
  gap: 0.72rem;
}

.side-head h2 {
  font-size: 1.05rem;
}

.signature {
  margin: 0;
  padding: 0.76rem 0.8rem;
  border-radius: var(--radius-md);
  background: var(--glass-mid);
  box-shadow: inset 0 -1px 0 var(--line-soft);
  font-size: 1rem;
  line-height: 1.6;
}

.side-actions {
  margin-top: auto;
}

.side-actions .btn-solid {
  width: 100%;
}

@media (max-width: 1080px) {
  .home-shell {
    min-height: auto;
    grid-template-columns: 1fr;
  }

  .home-metrics {
    grid-template-columns: 1fr;
  }
}
</style>
