<template>
  <section class="page-layout scoreboard-wall-page">
    <article class="surface stack wall-header">
      <header class="section-head">
        <div class="section-title">
          <p>{{ tr("排行榜大屏", "Scoreboard Wall") }}</p>
          <h1>{{ tr("实时趋势与排名", "Live Trend & Rankings") }}</h1>
        </div>
        <div class="context-menu">
          <button
            class="btn-line"
            :class="{ active: trendMode === 'score' }"
            type="button"
            @click="setTrendMode('score')"
          >
            {{ tr("积分曲线", "Score curve") }}
          </button>
          <button
            class="btn-line"
            :class="{ active: trendMode === 'rank' }"
            type="button"
            @click="setTrendMode('rank')"
          >
            {{ tr("排名曲线", "Rank curve") }}
          </button>
          <button class="btn-line" type="button" @click="loadWallData" :disabled="loadingWallData">
            {{ loadingWallData ? tr("刷新中...", "Refreshing...") : tr("刷新", "Refresh") }}
          </button>
          <RouterLink class="btn-line" :to="{ name: 'contest-detail', params: { contestId: props.contestId } }">
            {{ tr("返回比赛", "Back to contest") }}
          </RouterLink>
        </div>
      </header>
      <p class="soft mono">ws: {{ wsState }}</p>
      <p v-if="pageError" class="error">{{ pageError }}</p>
    </article>

    <article class="surface stack wall-trend">
      <div class="row-between">
        <h2>
          {{
            trendMode === "score"
              ? tr("实时积分趋势", "Live score trend")
              : tr("实时排名趋势", "Live rank trend")
          }}
        </h2>
        <p class="soft mono">
          {{ trendTimeRangeLabel }}
        </p>
      </div>
      <div v-if="chartSeries.length > 0" class="trend-svg-wrap">
        <svg viewBox="0 0 1200 360" role="img" :aria-label="tr('积分/排名趋势', 'Score/rank trend')">
          <g class="trend-grid">
            <line v-for="item in gridLinesY" :key="`y-${item}`" :x1="80" :y1="item" :x2="1140" :y2="item" />
            <line v-for="item in gridLinesX" :key="`x-${item}`" :x1="item" y1="28" :x2="item" y2="316" />
          </g>
          <g v-for="series in chartSeries" :key="series.teamId" class="trend-series">
            <polyline :points="series.points" :stroke="series.color" />
            <circle :cx="series.lastX" :cy="series.lastY" r="4" :fill="series.color" />
            <text :x="series.lastX + 8" :y="series.lastY + 4" class="trend-series-label">
              {{ series.label }}
            </text>
          </g>
          <g class="trend-axis-labels">
            <text x="18" y="28">{{ yAxisMaxLabel }}</text>
            <text x="18" y="318">{{ yAxisMinLabel }}</text>
            <text x="80" y="344">{{ trendStartLabel }}</text>
            <text x="1040" y="344">{{ trendEndLabel }}</text>
          </g>
        </svg>
      </div>
      <p v-else class="soft">{{ tr("暂无趋势数据。", "No trend data yet.") }}</p>
    </article>

    <article class="surface stack wall-rankings">
      <div class="legend-row">
        <span class="legend-chip marker-first">① {{ tr("一血", "First blood") }}</span>
        <span class="legend-chip marker-second">② {{ tr("二血", "Second blood") }}</span>
        <span class="legend-chip marker-third">③ {{ tr("三血", "Third blood") }}</span>
        <span class="legend-chip marker-solved">✓ {{ tr("解出", "Solved") }}</span>
        <span class="legend-chip marker-empty">· {{ tr("未解", "Unsolved") }}</span>
      </div>

      <section class="rank-section">
        <h2>{{ tr("队伍排名", "Team ranking") }}</h2>
        <div class="table-wrap">
          <table class="matrix-table">
            <thead>
              <tr>
                <th rowspan="2" class="matrix-rank-col">{{ tr("排名", "Rank") }}</th>
                <th rowspan="2" class="matrix-name-col">{{ tr("名称", "Name") }}</th>
                <th
                  v-for="group in rankingMatrixGroups"
                  :key="`team-header-${group.category}`"
                  :colspan="group.columns.length"
                  class="matrix-category-head"
                >
                  {{ group.category }}
                </th>
                <th rowspan="2" class="matrix-score-col">{{ tr("总积分", "Total score") }}</th>
              </tr>
              <tr v-if="rankingMatrixGroups.length > 0">
                <template v-for="group in rankingMatrixGroups" :key="`team-sub-${group.category}`">
                  <th
                    v-for="column in group.columns"
                    :key="`team-col-${group.category}-${column.challenge_id}`"
                    class="matrix-challenge-head"
                    :title="column.challenge_title"
                  >
                    {{ challengeHeaderLabel(column.challenge_title, column.challenge_slug) }}
                  </th>
                </template>
              </tr>
            </thead>
            <tbody>
              <tr v-for="row in topTeamRankings" :key="`team-${row.subject_id}`">
                <td class="mono">{{ row.rank }}</td>
                <td class="matrix-name-col" :title="row.subject_name">{{ row.subject_name }}</td>
                <template v-for="group in rankingMatrixGroups" :key="`team-row-${row.subject_id}-${group.category}`">
                  <td
                    v-for="column in group.columns"
                    :key="`team-cell-${row.subject_id}-${column.challenge_id}`"
                    class="matrix-cell"
                    :title="column.challenge_title"
                  >
                    <span
                      v-if="!column.is_placeholder"
                      class="matrix-marker"
                      :class="markerClass(teamChallengeMarker(row.subject_id, column.challenge_id))"
                    >
                      {{ markerSymbol(teamChallengeMarker(row.subject_id, column.challenge_id)) }}
                    </span>
                    <span v-else class="matrix-placeholder">-</span>
                  </td>
                </template>
                <td class="matrix-score-col mono">{{ row.total_score }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <section class="rank-section">
        <h2>{{ tr("选手排名", "Player ranking") }}</h2>
        <div class="table-wrap">
          <table class="matrix-table">
            <thead>
              <tr>
                <th rowspan="2" class="matrix-rank-col">{{ tr("排名", "Rank") }}</th>
                <th rowspan="2" class="matrix-name-col">{{ tr("名称", "Name") }}</th>
                <th
                  v-for="group in rankingMatrixGroups"
                  :key="`player-header-${group.category}`"
                  :colspan="group.columns.length"
                  class="matrix-category-head"
                >
                  {{ group.category }}
                </th>
                <th rowspan="2" class="matrix-score-col">{{ tr("总积分", "Total score") }}</th>
              </tr>
              <tr v-if="rankingMatrixGroups.length > 0">
                <template v-for="group in rankingMatrixGroups" :key="`player-sub-${group.category}`">
                  <th
                    v-for="column in group.columns"
                    :key="`player-col-${group.category}-${column.challenge_id}`"
                    class="matrix-challenge-head"
                    :title="column.challenge_title"
                  >
                    {{ challengeHeaderLabel(column.challenge_title, column.challenge_slug) }}
                  </th>
                </template>
              </tr>
            </thead>
            <tbody>
              <tr v-for="row in topPlayerRankings" :key="`player-${row.subject_id}`">
                <td class="mono">{{ row.rank }}</td>
                <td class="matrix-name-col" :title="row.subject_name">{{ row.subject_name }}</td>
                <template
                  v-for="group in rankingMatrixGroups"
                  :key="`player-row-${row.subject_id}-${group.category}`"
                >
                  <td
                    v-for="column in group.columns"
                    :key="`player-cell-${row.subject_id}-${column.challenge_id}`"
                    class="matrix-cell"
                    :title="column.challenge_title"
                  >
                    <span
                      v-if="!column.is_placeholder"
                      class="matrix-marker"
                      :class="markerClass(playerChallengeMarker(row.subject_id, column.challenge_id))"
                    >
                      {{ markerSymbol(playerChallengeMarker(row.subject_id, column.challenge_id)) }}
                    </span>
                    <span v-else class="matrix-placeholder">-</span>
                  </td>
                </template>
                <td class="matrix-score-col mono">{{ row.total_score }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </article>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { RouterLink, useRoute, useRouter } from "vue-router";

import {
  ApiClientError,
  buildScoreboardWsUrl,
  getScoreboardRankings,
  getScoreboardTimeline,
  type ScoreboardCategoryItem,
  type ScoreboardRankingEntry,
  type ScoreboardTimelineSnapshot,
  type ScoreboardPushPayload
} from "../api/client";
import { useL10n } from "../composables/useL10n";
import { useAuthStore } from "../stores/auth";

const props = defineProps<{
  contestId: string;
}>();

const authStore = useAuthStore();
const { locale, tr } = useL10n();
const route = useRoute();
const router = useRouter();

const trendMode = ref<"score" | "rank">("score");
const timeline = ref<ScoreboardTimelineSnapshot[]>([]);
const rankingCategories = ref<ScoreboardCategoryItem[]>([]);
const teamRankings = ref<ScoreboardRankingEntry[]>([]);
const playerRankings = ref<ScoreboardRankingEntry[]>([]);
const loadingWallData = ref(false);
const pageError = ref("");
const wsState = ref("closed");

let scoreboardSocket: WebSocket | null = null;
let reconnectTimer: number | null = null;
let refreshTimer: number | null = null;
let pollTimer: number | null = null;
let shouldReconnect = true;

const topTeamRankings = computed(() => teamRankings.value.slice(0, 60));
const topPlayerRankings = computed(() => playerRankings.value.slice(0, 60));
const teamMarkerLookup = computed(() => buildMarkerLookup(teamRankings.value));
const playerMarkerLookup = computed(() => buildMarkerLookup(playerRankings.value));
const rankingMatrixGroups = computed(() =>
  rankingCategories.value.map((item, index) => {
    const columns =
      item.challenges.length > 0
        ? item.challenges.map((challenge) => ({
            challenge_id: challenge.challenge_id,
            challenge_title: challenge.challenge_title,
            challenge_slug: challenge.challenge_slug,
            is_placeholder: false
          }))
        : [
            {
              challenge_id: `empty-${index}`,
              challenge_title: "-",
              challenge_slug: "-",
              is_placeholder: true
            }
          ];
    return {
      category: item.category,
      columns
    };
  })
);

const normalizedTimeline = computed(() => {
  const rows = [...timeline.value];
  rows.sort((a, b) => {
    const diff = new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime();
    if (diff !== 0) {
      return diff;
    }
    return a.trigger_submission_id - b.trigger_submission_id;
  });
  return rows;
});

const trendStartLabel = computed(() => {
  const first = normalizedTimeline.value[0];
  return first ? formatTimeLabel(first.timestamp) : "-";
});

const trendEndLabel = computed(() => {
  const rows = normalizedTimeline.value;
  const last = rows[rows.length - 1];
  return last ? formatTimeLabel(last.timestamp) : "-";
});

const trendTimeRangeLabel = computed(() => `${trendStartLabel.value} ~ ${trendEndLabel.value}`);

const chartSeries = computed(() => {
  const snapshots = normalizedTimeline.value;
  if (snapshots.length === 0) {
    return [] as Array<{
      teamId: string;
      label: string;
      points: string;
      color: string;
      lastX: number;
      lastY: number;
      values: number[];
    }>;
  }

  const topTeams = teamRankings.value.slice(0, 8);
  if (topTeams.length === 0) {
    return [];
  }

  const palette = ["#111111", "#2f2f2f", "#4b4b4b", "#686868", "#848484", "#9e9e9e", "#b8b8b8", "#d2d2d2"];
  const left = 80;
  const right = 1140;
  const top = 28;
  const bottom = 316;
  const width = right - left;
  const height = bottom - top;

  const snapshotMaps = snapshots.map((item) => {
    const map = new Map<string, { rank: number; score: number }>();
    for (const entry of item.entries) {
      map.set(entry.team_id, {
        rank: entry.rank,
        score: entry.score
      });
    }
    return map;
  });

  const maxRank = Math.max(
    1,
    ...snapshots.flatMap((item) => item.entries.map((entry) => entry.rank)),
    topTeams.length
  );

  const series = topTeams.map((team, index) => {
    const values: number[] = [];
    let lastScore = 0;
    let lastRank = maxRank;

    for (const map of snapshotMaps) {
      const found = map.get(team.subject_id);
      if (found) {
        lastScore = found.score;
        lastRank = found.rank;
      }
      values.push(trendMode.value === "score" ? lastScore : lastRank);
    }

    return {
      teamId: team.subject_id,
      teamName: team.subject_name,
      values,
      color: palette[index % palette.length],
      rank: team.rank
    };
  });

  const minValue = trendMode.value === "score" ? 0 : 1;
  const rawMax = Math.max(...series.flatMap((item) => item.values));
  const maxValue = Math.max(minValue + 1, rawMax);

  return series.map((item) => {
    const coords = item.values.map((value, idx) => {
      const x = left + (snapshots.length === 1 ? 0 : (idx * width) / (snapshots.length - 1));
      const normalized = (value - minValue) / (maxValue - minValue);
      const y =
        trendMode.value === "rank"
          ? top + normalized * height
          : top + (1 - normalized) * height;
      return { x, y };
    });

    const last = coords[coords.length - 1] ?? { x: right, y: top };
    return {
      teamId: item.teamId,
      label: `#${item.rank} ${item.teamName}`,
      points: coords.map((point) => `${point.x},${point.y}`).join(" "),
      color: item.color,
      lastX: last.x,
      lastY: last.y,
      values: item.values
    };
  });
});

const yAxisMaxLabel = computed(() => {
  const series = chartSeries.value;
  if (series.length === 0) {
    return "-";
  }
  const maxValue = Math.max(...series.flatMap((item) => item.values));
  return trendMode.value === "score" ? `${Math.round(maxValue)}` : `#${Math.round(maxValue)}`;
});

const yAxisMinLabel = computed(() => {
  return trendMode.value === "score" ? "0" : "#1";
});

const gridLinesY = [28, 100, 172, 244, 316];
const gridLinesX = [80, 292, 504, 716, 928, 1140];

watch(
  () => route.query.mode,
  (value) => {
    trendMode.value = value === "rank" ? "rank" : "score";
  },
  { immediate: true }
);

function setTrendMode(mode: "score" | "rank") {
  trendMode.value = mode;
  const nextQuery = {
    ...route.query,
    mode
  };
  router.replace({ query: nextQuery }).catch(() => undefined);
}

function buildMarkerLookup(rows: ScoreboardRankingEntry[]) {
  const lookup = new Map<string, Map<string, string>>();
  for (const row of rows) {
    const solved = new Map<string, string>();
    for (const category of row.categories) {
      for (const challenge of category.challenges) {
        solved.set(challenge.challenge_id, challenge.marker);
      }
    }
    lookup.set(row.subject_id, solved);
  }
  return lookup;
}

function lookupChallengeMarker(
  lookup: Map<string, Map<string, string>>,
  subjectId: string,
  challengeId: string
) {
  return lookup.get(subjectId)?.get(challengeId) ?? "unsolved";
}

function teamChallengeMarker(subjectId: string, challengeId: string) {
  return lookupChallengeMarker(teamMarkerLookup.value, subjectId, challengeId);
}

function playerChallengeMarker(subjectId: string, challengeId: string) {
  return lookupChallengeMarker(playerMarkerLookup.value, subjectId, challengeId);
}

function markerSymbol(marker: string) {
  if (marker === "first_blood") {
    return "①";
  }
  if (marker === "second_blood") {
    return "②";
  }
  if (marker === "third_blood") {
    return "③";
  }
  if (marker === "unsolved") {
    return "·";
  }
  return "✓";
}

function challengeHeaderLabel(title: string, slug: string) {
  const raw = (title || slug || "-").trim();
  const limit = 14;
  if (raw.length <= limit) {
    return raw;
  }
  return `${raw.slice(0, limit - 1)}…`;
}

function markerClass(marker: string) {
  if (marker === "first_blood") {
    return "marker-first";
  }
  if (marker === "second_blood") {
    return "marker-second";
  }
  if (marker === "third_blood") {
    return "marker-third";
  }
  if (marker === "unsolved") {
    return "marker-empty";
  }
  return "marker-solved";
}

function formatTimeLabel(input: string) {
  const localeTag = locale.value === "en" ? "en-US" : "zh-CN";
  return new Date(input).toLocaleTimeString(localeTag, {
    hour: "2-digit",
    minute: "2-digit"
  });
}

function accessTokenOrThrow() {
  const token = authStore.accessToken;
  if (!token) {
    throw new ApiClientError(tr("未登录或会话已失效", "Not signed in or session expired"), "unauthorized");
  }
  return token;
}

async function loadWallData() {
  loadingWallData.value = true;
  pageError.value = "";

  try {
    const token = accessTokenOrThrow();
    const [timelineResponse, rankingResponse] = await Promise.all([
      getScoreboardTimeline(props.contestId, token, {
        max_snapshots: 1200,
        top_n: 12
      }),
      getScoreboardRankings(props.contestId, token)
    ]);

    timeline.value = timelineResponse.snapshots;
    rankingCategories.value = rankingResponse.categories;
    teamRankings.value = rankingResponse.team_rankings;
    playerRankings.value = rankingResponse.player_rankings;
  } catch (err) {
    pageError.value = err instanceof ApiClientError ? err.message : tr("加载排行榜失败", "Failed to load scoreboard wall");
  } finally {
    loadingWallData.value = false;
  }
}

function teardownSocket() {
  if (reconnectTimer !== null) {
    window.clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }

  const socket = scoreboardSocket;
  scoreboardSocket = null;

  if (socket) {
    socket.onopen = null;
    socket.onmessage = null;
    socket.onerror = null;
    socket.onclose = null;
    socket.close();
  }

  wsState.value = "closed";
}

function scheduleRefresh(delayMs = 380) {
  if (refreshTimer !== null) {
    window.clearTimeout(refreshTimer);
    refreshTimer = null;
  }
  refreshTimer = window.setTimeout(() => {
    refreshTimer = null;
    loadWallData();
  }, delayMs);
}

function openScoreboardSocket() {
  teardownSocket();

  let token = "";
  try {
    token = accessTokenOrThrow();
  } catch {
    return;
  }

  wsState.value = "connecting";
  const wsUrl = buildScoreboardWsUrl(props.contestId, token);
  scoreboardSocket = new WebSocket(wsUrl);

  scoreboardSocket.onopen = () => {
    wsState.value = "open";
  };

  scoreboardSocket.onmessage = (event) => {
    try {
      const payload = JSON.parse(event.data) as ScoreboardPushPayload;
      if (payload.contest_id === props.contestId) {
        scheduleRefresh(160);
      }
    } catch {
      // ignore malformed message
    }
  };

  scoreboardSocket.onerror = () => {
    wsState.value = "error";
  };

  scoreboardSocket.onclose = () => {
    wsState.value = "closed";
    if (!shouldReconnect) {
      return;
    }
    reconnectTimer = window.setTimeout(() => {
      openScoreboardSocket();
    }, 2400);
  };
}

onMounted(async () => {
  shouldReconnect = true;
  await loadWallData();
  openScoreboardSocket();
  pollTimer = window.setInterval(() => {
    loadWallData();
  }, 20000);
});

onUnmounted(() => {
  shouldReconnect = false;
  teardownSocket();
  if (refreshTimer !== null) {
    window.clearTimeout(refreshTimer);
    refreshTimer = null;
  }
  if (pollTimer !== null) {
    window.clearInterval(pollTimer);
    pollTimer = null;
  }
});
</script>

<style scoped>
.scoreboard-wall-page {
  gap: 0.86rem;
}

.wall-header .btn-line.active {
  background: var(--fg-0);
  color: var(--bg-0);
}

.wall-trend {
  min-height: 320px;
}

.trend-svg-wrap {
  border-radius: var(--radius-md);
  overflow: hidden;
  background: var(--glass-mid);
  box-shadow: inset 0 -1px 0 var(--line-soft);
}

.trend-svg-wrap svg {
  display: block;
  width: 100%;
  height: auto;
}

.trend-grid line {
  stroke: var(--line-mid);
  stroke-width: 1;
  stroke-dasharray: 2 6;
}

.trend-series polyline {
  fill: none;
  stroke-width: 2.2;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.trend-series-label {
  font-size: 11px;
  fill: var(--fg-0);
}

.trend-axis-labels text {
  font-size: 11px;
  fill: var(--fg-2);
}

.wall-rankings {
  gap: 0.9rem;
}

.rank-section {
  display: grid;
  gap: 0.46rem;
}

.legend-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.36rem;
}

.legend-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.26rem;
  border-radius: 999px;
  padding: 0.2rem 0.58rem;
  background: var(--glass-mid);
  font-size: 0.76rem;
}

.matrix-table {
  width: max-content;
  min-width: 100%;
  border-collapse: separate;
  border-spacing: 0;
  table-layout: fixed;
}

.matrix-table th,
.matrix-table td {
  padding: 0.4rem 0.42rem;
  text-align: center;
  vertical-align: middle;
  border-right: 1px solid color-mix(in srgb, var(--line-soft) 86%, transparent 14%);
}

.matrix-table thead th {
  background: color-mix(in srgb, var(--glass-mid) 86%, transparent 14%);
  font-size: 0.72rem;
  font-weight: 620;
  white-space: nowrap;
  text-transform: none;
  letter-spacing: 0.03em;
  border-bottom: 1px dashed var(--line-mid);
}

.matrix-table thead tr:first-child th {
  background: color-mix(in srgb, var(--glass-strong) 82%, transparent 18%);
  color: var(--fg-1);
}

.matrix-table .matrix-score-col {
  border-right: 0;
}

.matrix-table tbody td {
  border-bottom: 1px dashed var(--line-soft);
}

.matrix-table tbody tr:last-child td {
  border-bottom: 0;
}

.matrix-table tbody tr:hover {
  background: color-mix(in srgb, var(--glass-mid) 68%, transparent 32%);
}

.matrix-rank-col {
  min-width: 64px;
}

.matrix-name-col {
  min-width: 176px;
  max-width: 232px;
  text-align: left !important;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-weight: 620;
}

.matrix-score-col {
  min-width: 88px;
}

.matrix-category-head {
  letter-spacing: 0.06em;
  font-weight: 700;
}

.matrix-challenge-head {
  min-width: 82px;
  max-width: 110px;
  overflow: hidden;
  text-overflow: ellipsis;
  font-size: 0.68rem;
  color: var(--fg-2);
}

.matrix-cell {
  min-width: 52px;
}

.matrix-marker {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  width: 1.44rem;
  height: 1.44rem;
  font-size: 0.74rem;
  font-weight: 700;
  background: color-mix(in srgb, var(--glass-mid) 88%, transparent 12%);
  box-shadow: inset 0 0 0 1px var(--line-soft);
}

.matrix-placeholder {
  color: var(--fg-2);
}

.marker-first {
  background: rgba(18, 18, 18, 0.88);
  color: rgba(255, 255, 255, 0.96);
}

.marker-second {
  background: rgba(56, 56, 56, 0.82);
  color: rgba(255, 255, 255, 0.96);
}

.marker-third {
  background: rgba(90, 90, 90, 0.8);
  color: rgba(255, 255, 255, 0.96);
}

.marker-solved {
  background: var(--glass-strong);
  color: var(--fg-0);
}

.marker-empty {
  background: transparent;
  box-shadow: inset 0 0 0 1px var(--line-mid);
  color: var(--fg-2);
}

@media (max-width: 980px) {
  .wall-header .context-menu {
    width: 100%;
    justify-content: flex-start;
  }

  .trend-series-label {
    display: none;
  }
}
</style>
