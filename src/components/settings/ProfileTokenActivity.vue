<script setup lang="ts">
import { computed, ref } from "vue";
import { RefreshCw } from "@lucide/vue";
import { activityDays, profileStats } from "@/services/usage/profileStats";
import type { TokenUsageReport } from "@/types/tokenUsage";
import type { AppLanguage } from "@/types/setting";
const props = defineProps<{
  report: TokenUsageReport | null;
  stats: ReturnType<typeof profileStats> | null;
  today: Date;
  loading: boolean;
  language: AppLanguage;
  labels: Record<string, string>;
}>();
const emit = defineEmits<{ refresh: [] }>();
const stats = computed(() => props.stats);
const days = activityDays(props.today);
const mode = ref<"daily" | "weekly" | "cumulative">("daily");
const exact = (value: number) => new Intl.NumberFormat(props.language).format(value);
const weekly = computed(() =>
  Array.from({ length: days.length / 7 }, (_, index) => {
    const slice = days.slice(index * 7, index * 7 + 7);
    return {
      key: slice[0].key,
      value: slice.reduce((sum, day) => sum + (stats.value?.daily.get(day.key) ?? 0), 0),
    };
  }),
);
const cumulative = computed(() => {
  let sum = [...(stats.value?.daily.entries() ?? [])].reduce(
    (total, [key, value]) => (key < days[0].key ? total + value : total),
    0,
  );
  return weekly.value.map((week) => ({ ...week, value: (sum += week.value) }));
});
const chart = computed(() => (mode.value === "weekly" ? weekly.value : cumulative.value));
const chartMax = computed(() => Math.max(1, ...chart.value.map((week) => week.value)));
const peak = computed(() =>
  Math.max(1, ...days.map((day) => stats.value?.daily.get(day.key) ?? 0)),
);
function level(value: number) {
  return value <= 0 ? 0 : Math.min(4, Math.max(1, Math.ceil((value / peak.value) * 4)));
}
const months = computed(() =>
  weekly.value.map((_, index) => {
    const date = days[index * 7].date;
    const prev = index ? days[(index - 1) * 7].date : null;
    return {
      index,
      label:
        !prev || prev.getMonth() !== date.getMonth()
          ? date.toLocaleDateString(props.language, { month: "short" })
          : "",
    };
  }),
);
</script>
<template>
  <section class="profile-activity">
    <div class="profile-section-head">
      <h2>{{ labels.activity }}</h2>
      <div class="activity-tabs" role="group" :aria-label="labels.activity">
        <button
          v-for="tab in ['daily', 'weekly', 'cumulative'] as const"
          :key="tab"
          type="button"
          :aria-pressed="mode === tab"
          @click="mode = tab"
        >
          {{ labels[tab] }}
        </button>
        <button
          type="button"
          :aria-label="labels.retry"
          :disabled="loading"
          @click="emit('refresh')"
        >
          <RefreshCw :size="12" />
        </button>
      </div>
    </div>
    <div class="activity-scroll">
      <div v-if="mode === 'daily'" class="activity-calendar" :style="{ '--weeks': weekly.length }">
        <span
          v-for="day in days"
          :key="day.key"
          class="activity-cell"
          :class="{ future: day.future }"
          :data-level="level(stats?.daily.get(day.key) ?? 0)"
          :title="`${day.date.toLocaleDateString(language)} · ${exact(stats?.daily.get(day.key) ?? 0)} tokens`"
        />
      </div>
      <div v-else class="activity-bars" :style="{ '--weeks': weekly.length }">
        <span
          v-for="week in chart"
          :key="week.key"
          :style="{ height: `${Math.max(2, (week.value / chartMax) * 100)}%` }"
          :title="`${week.key} · ${exact(week.value)} tokens`"
        />
      </div>
      <div class="activity-months" :style="{ '--weeks': weekly.length }">
        <span v-for="month in months" :key="month.index">{{ month.label }}</span>
      </div>
    </div>
    <div class="activity-legend">
      <span>{{ labels.less }}</span>
      <i v-for="value in [0, 1, 2, 3, 4]" :key="value" class="activity-cell" :data-level="value" />
      <span>{{ labels.more }}</span>
    </div>
    <p v-if="report && report.modelCalls === 0" class="profile-status">{{ labels.empty }}</p>
  </section>
</template>
<style scoped>
.profile-activity {
  margin-top: 46px;
}
h2 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}
.profile-section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 20px;
}
.activity-tabs {
  display: flex;
  gap: 12px;
  font-size: 12px;
}
.activity-scroll {
  overflow-x: auto;
  padding-bottom: 8px;
}
.activity-calendar {
  display: grid;
  grid-template-rows: repeat(7, 1fr);
  grid-auto-flow: column;
  grid-template-columns: repeat(var(--weeks), 1fr);
  gap: 3px;
  min-width: 620px;
}
.activity-cell {
  display: block;
  border-radius: 3px;
  aspect-ratio: 1;
  background: color-mix(in srgb, var(--peek-muted) 10%, transparent);
}
.activity-cell[data-level="1"] {
  background: color-mix(in srgb, var(--peek-accent) 22%, var(--peek-surface));
}
.activity-cell[data-level="2"] {
  background: color-mix(in srgb, var(--peek-accent) 42%, var(--peek-surface));
}
.activity-cell[data-level="3"] {
  background: color-mix(in srgb, var(--peek-accent) 68%, var(--peek-surface));
}
.activity-cell[data-level="4"] {
  background: var(--peek-accent);
}
.activity-cell.future {
  opacity: 0.3;
}
.activity-months {
  display: grid;
  grid-template-columns: repeat(var(--weeks), 1fr);
  min-width: 620px;
  margin-top: 9px;
  font-size: 10px;
  color: var(--peek-muted);
}
.activity-months span {
  white-space: nowrap;
}
.activity-bars {
  height: 100px;
  min-width: 620px;
  display: grid;
  grid-template-columns: repeat(var(--weeks), 1fr);
  align-items: end;
  gap: 3px;
}
.activity-bars span {
  min-height: 2px;
  background: color-mix(in srgb, var(--peek-accent) 70%, var(--peek-surface));
  border-radius: 3px 3px 0 0;
}
.activity-legend {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 4px;
  color: var(--peek-muted);
  font-size: 10px;
  margin-top: 8px;
}
.activity-legend i {
  width: 10px;
  height: 10px;
}
.profile-status {
  color: var(--peek-muted);
  font-size: 12px;
  line-height: 1.6;
}
@media (max-width: 700px) {
  .profile-section-head {
    align-items: flex-start;
    flex-direction: column;
  }
}
</style>
