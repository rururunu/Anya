<template>
  <section class="settings-page is-wide usage-page">
    <DeepSeekBalanceCard
      :report="balance"
      :loading="balanceLoading"
      :error="balanceError"
      :language="settingStore.language"
      :copy="balanceCopy"
      @refresh="loadBalance"
    />

    <SettingsPageHeader :title="copy.title">
      <template #actions>
        <Select v-model="range" @update:model-value="applyRange">
          <SelectTrigger class="range-select"><SelectValue /></SelectTrigger>
          <SelectContent position="popper">
            <SelectItem v-for="item in rangeOptions" :key="item.value" :value="item.value">
              {{ item.label }}
            </SelectItem>
          </SelectContent>
        </Select>
        <button
          type="button"
          class="icon-button"
          :title="copy.refresh"
          :aria-label="copy.refresh"
          @click="load"
        >
          <RefreshCw :size="14" :class="{ spinning: loading }" />
        </button>
      </template>
    </SettingsPageHeader>

    <div class="filter-row">
      <div class="settings-seg" role="group">
        <button
          v-for="item in granularityOptions"
          :key="item.value"
          type="button"
          :class="{ on: granularity === item.value }"
          @click="setGranularity(item.value)"
        >
          {{ item.label }}
        </button>
      </div>
      <div v-if="range === 'custom'" class="custom-range">
        <label>
          <span>{{ copy.from }}</span>
          <input v-model="customFrom" type="date" @change="load" />
        </label>
        <label>
          <span>{{ copy.to }}</span>
          <input v-model="customTo" type="date" @change="load" />
        </label>
      </div>
    </div>

    <div v-if="error" class="settings-form-error">
      <CircleAlert :size="16" class="inline mr-1.5 align-text-bottom" />
      <span>{{ copy.error }}: {{ error }}</span>
    </div>
    <div v-else-if="loading" class="settings-empty">
      <span class="loader" />
      {{ copy.loading }}
    </div>

    <template v-else>
      <section class="settings-card summary-strip" aria-label="Token summary">
        <div class="summary-item total">
          <span>{{ copy.total }}</span>
          <strong>{{ format(report.total.totalTokens) }}</strong>
          <small>{{ copy.calls }}</small>
        </div>
        <div class="summary-item">
          <span>{{ copy.input }}</span>
          <strong>{{ format(report.total.inputTokens) }}</strong>
          <small>{{ share(report.total.inputTokens) }}</small>
        </div>
        <div class="summary-item">
          <span>{{ copy.output }}</span>
          <strong>{{ format(report.total.outputTokens) }}</strong>
          <small>{{ share(report.total.outputTokens) }}</small>
        </div>
        <div class="summary-item">
          <span>{{ copy.accuracy }}</span>
          <strong class="accuracy">{{ accuracyLabel }}</strong>
          <small>{{ copy.accuracyHint }}</small>
        </div>
      </section>

      <div v-if="report.modelCalls === 0" class="empty-state">
        <BarChart3 :size="25" />
        <strong>{{ copy.emptyTitle }}</strong>
        <span>{{ copy.emptyDescription }}</span>
      </div>

      <template v-else>
        <section class="data-section timeline-section">
          <header class="section-header">
            <div>
              <h2>{{ copy.timeline }}</h2>
              <p>{{ copy.timelineHint }}</p>
            </div>
          </header>
          <UsageTimelineChart
            :series="chartSeries"
            :timeline="report.timeline"
            :format="format"
            :total-label="copy.chartTotal"
            :chart-label="copy.timeline"
            :kind-labels="chartKindLabels"
          />
        </section>

        <div class="detail-grid">
          <section class="data-section">
            <header class="section-header">
              <div>
                <h2>{{ copy.models }}</h2>
                <p>{{ copy.modelsHint }}</p>
              </div>
            </header>
            <UsageShareChart
              :items="modelShareItems"
              :format="format"
              :total-label="copy.chartTotal"
              :empty-label="copy.emptyTitle"
              :chart-label="copy.models"
            />
          </section>

          <section class="data-section">
            <header class="section-header">
              <div>
                <h2>{{ copy.breakdown }}</h2>
                <p>{{ copy.breakdownHint }}</p>
              </div>
            </header>
            <UsageRadarChart
              :items="breakdownShareItems"
              :format="format"
              :empty-label="copy.emptyTitle"
              :chart-label="copy.breakdown"
            />
          </section>
        </div>
      </template>
    </template>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { BarChart3, CircleAlert, RefreshCw } from "@lucide/vue";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import SettingsPageHeader from "@/components/settings/SettingsPageHeader.vue";
import DeepSeekBalanceCard from "@/components/settings/DeepSeekBalanceCard.vue";
import UsageRadarChart from "@/components/settings/UsageRadarChart.vue";
import UsageShareChart from "@/components/settings/UsageShareChart.vue";
import UsageTimelineChart from "@/components/settings/UsageTimelineChart.vue";
import { getDeepSeekBalance, getTokenUsageReport } from "@/services/ipc";
import { tr } from "@/services/i18n";
import { getProviderIcon } from "@/lib/providerIcons";
import {
  BREAKDOWN_COLORS,
  buildChartSeries,
  buildModelGroups,
  buildModelShareItems,
  type ShareDatum,
} from "@/services/usage/tokenUsageChart";
import { useSettingStore } from "@/stores/setting";
import type { DeepSeekBalanceReport, TokenUsageReport } from "@/types/tokenUsage";

type Granularity = "day" | "week" | "month";
const settingStore = useSettingStore();
const loading = ref(true);
const error = ref("");
const range = ref("30d");
const granularity = ref<Granularity>("day");
const customFrom = ref("");
const customTo = ref("");

const emptyUsage = (): TokenUsageReport => ({
  from: 0,
  to: 0,
  granularity: "day",
  modelCalls: 0,
  total: {
    inputTokens: 0,
    outputTokens: 0,
    systemTokens: 0,
    contextTokens: 0,
    toolCallTokens: 0,
    toolResultTokens: 0,
    memoryTokens: 0,
    totalTokens: 0,
    accuracy: "estimated",
  },
  byModel: [],
  timeline: [],
});
const report = ref<TokenUsageReport>(emptyUsage());
const balance = ref<DeepSeekBalanceReport | null>(null);
const balanceLoading = ref(false);
const balanceError = ref("");

const copy = computed(() => {
  const language = settingStore.language;
  return {
    title: tr(language, "usage.title"),
    description: tr(language, "usage.description"),
    from: tr(language, "usage.from"),
    to: tr(language, "usage.to"),
    refresh: tr(language, "usage.refresh"),
    loading: tr(language, "usage.loading"),
    error: tr(language, "usage.error"),
    emptyTitle: tr(language, "usage.empty.title"),
    emptyDescription: tr(language, "usage.empty.description"),
    total: tr(language, "usage.total"),
    calls: tr(language, "usage.calls", { count: report.value.modelCalls }),
    input: tr(language, "usage.input"),
    output: tr(language, "usage.output"),
    accuracy: tr(language, "usage.accuracy"),
    accuracyHint: tr(language, "usage.accuracyHint"),
    timeline: tr(language, "usage.timeline"),
    timelineHint: tr(language, "usage.timelineHint", {
      granularity: tr(language, `usage.granularity.${granularity.value}`),
    }),
    chartStacked: tr(language, "usage.chart.stacked"),
    chartLine: tr(language, "usage.chart.line"),
    chartArea: tr(language, "usage.chart.area"),
    models: tr(language, "usage.models"),
    modelsHint: tr(language, "usage.modelsHint"),
    otherModels: tr(language, "usage.otherModels"),
    chartTotal: tr(language, "usage.chartTotal"),
    breakdown: tr(language, "usage.breakdown"),
    breakdownHint: tr(language, "usage.breakdownHint"),
    tools: tr(language, "usage.tools"),
    context: tr(language, "usage.context"),
    system: tr(language, "usage.system"),
    memory: tr(language, "usage.memory"),
    reasoning: tr(language, "usage.reasoning"),
    cacheRead: tr(language, "usage.cacheRead"),
  };
});

const balanceCopy = computed(() => ({
  title: tr(settingStore.language, "usage.balance.title"),
  available: tr(settingStore.language, "usage.balance.available"),
  granted: tr(settingStore.language, "usage.balance.granted"),
  toppedUp: tr(settingStore.language, "usage.balance.toppedUp"),
  ready: tr(settingStore.language, "usage.balance.ready"),
  empty: tr(settingStore.language, "usage.balance.empty"),
  unconfigured: tr(settingStore.language, "usage.balance.unconfigured"),
  error: tr(settingStore.language, "usage.balance.error"),
  loading: tr(settingStore.language, "usage.balance.loading"),
  refresh: tr(settingStore.language, "usage.balance.refresh"),
}));

const rangeOptions = computed(() =>
  ["today", "7d", "30d", "month", "custom"].map((value) => ({
    value,
    label: tr(settingStore.language, `usage.range.${value}` as "usage.range.today"),
  })),
);
const granularityOptions = computed(() =>
  (["day", "week", "month"] as Granularity[]).map((value) => ({
    value,
    label: tr(settingStore.language, `usage.granularity.${value}`),
  })),
);
const accuracyLabel = computed(() =>
  tr(
    settingStore.language,
    report.value.total.accuracy === "exact"
      ? "runtime.accuracy.exact"
      : report.value.total.accuracy === "mixed"
        ? "runtime.accuracy.mixed"
        : "runtime.accuracy.estimated",
  ),
);

const bounds = computed(() => {
  const now = new Date();
  let from = new Date(now);
  if (range.value === "today") from.setHours(0, 0, 0, 0);
  else if (range.value === "7d") from.setDate(now.getDate() - 7);
  else if (range.value === "30d") from.setDate(now.getDate() - 30);
  else if (range.value === "month") from = new Date(now.getFullYear(), now.getMonth(), 1);
  else if (customFrom.value) from = new Date(`${customFrom.value}T00:00:00`);
  const to =
    range.value === "custom" && customTo.value ? new Date(`${customTo.value}T23:59:59`) : now;
  return { from: from.getTime(), to: to.getTime() + 1 };
});

function providerIcon(brand: string) {
  return getProviderIcon(brand === "other" ? "" : brand);
}

function modelCallsLabel(count: number) {
  return tr(settingStore.language, "usage.modelCalls", { count });
}

const chartKindLabels = computed(() => ({
  stacked: copy.value.chartStacked,
  line: copy.value.chartLine,
  area: copy.value.chartArea,
}));

const chartSeries = computed(() => buildChartSeries(report.value, copy.value.otherModels));

const modelGroups = computed(() => buildModelGroups(report.value, copy.value.otherModels));
const modelShareItems = computed(() => {
  const groups = modelGroups.value;
  return buildModelShareItems(groups).map((item) => {
    const group = groups.find((entry) => entry.id === item.id);
    if (item.children?.length) {
      return {
        ...item,
        icon: providerIcon(item.id) ?? undefined,
        children: item.children.map((child) => {
          const model = group?.models.find((row) => row.id === child.id);
          return { ...child, hint: model ? modelCallsLabel(model.calls) : undefined };
        }),
      };
    }
    const model = groups[0]?.models.find((row) => row.id === item.id);
    return { ...item, hint: model ? modelCallsLabel(model.calls) : undefined };
  });
});

const breakdownShareItems = computed((): ShareDatum[] => {
  const total = report.value.total;
  const items: ShareDatum[] = [
    {
      id: "input",
      label: copy.value.input,
      color: BREAKDOWN_COLORS.input,
      value: total.inputTokens,
    },
    {
      id: "output",
      label: copy.value.output,
      color: BREAKDOWN_COLORS.output,
      value: total.outputTokens,
    },
    {
      id: "tools",
      label: copy.value.tools,
      color: BREAKDOWN_COLORS.tools,
      value: total.toolCallTokens + total.toolResultTokens,
    },
    {
      id: "context",
      label: copy.value.context,
      color: BREAKDOWN_COLORS.context,
      value: total.contextTokens,
    },
    {
      id: "system",
      label: copy.value.system,
      color: BREAKDOWN_COLORS.system,
      value: total.systemTokens,
    },
    {
      id: "memory",
      label: copy.value.memory,
      color: BREAKDOWN_COLORS.memory,
      value: total.memoryTokens,
    },
  ];
  if ((total.reasoningTokens ?? 0) > 0) {
    items.push({
      id: "reasoning",
      label: copy.value.reasoning,
      color: BREAKDOWN_COLORS.reasoning,
      value: total.reasoningTokens ?? 0,
    });
  }
  if ((total.cacheReadTokens ?? 0) > 0) {
    items.push({
      id: "cache",
      label: copy.value.cacheRead,
      color: BREAKDOWN_COLORS.cache,
      value: total.cacheReadTokens ?? 0,
    });
  }
  return items;
});

const format = (value: number) => new Intl.NumberFormat(settingStore.language).format(value);
const share = (value: number) =>
  tr(settingStore.language, "usage.share", {
    value: report.value.total.totalTokens
      ? Math.round((value / report.value.total.totalTokens) * 100)
      : 0,
  });

async function loadBalance() {
  balanceLoading.value = true;
  balanceError.value = "";
  try {
    balance.value = await getDeepSeekBalance();
  } catch (cause) {
    balanceError.value = String(cause);
  } finally {
    balanceLoading.value = false;
  }
}

async function load() {
  loading.value = true;
  error.value = "";
  try {
    report.value = await getTokenUsageReport({
      from: bounds.value.from,
      to: bounds.value.to,
      granularity: granularity.value,
    });
  } catch (cause) {
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
}
function applyRange() {
  void load();
}
function setGranularity(value: Granularity) {
  granularity.value = value;
  void load();
}
onMounted(() => {
  void loadBalance();
  void load();
});
</script>

<style scoped>
.usage-page {
  color: var(--peek-text);
}
.section-header p {
  margin: 4px 0 0;
  color: var(--peek-muted);
  font-size: 11px;
  line-height: 17px;
}
.range-select {
  min-width: 132px;
}
.icon-button {
  width: 32px;
  height: 32px;
  display: grid;
  place-items: center;
  border: 1px solid var(--peek-border);
  border-radius: 6px;
  background: transparent;
  color: var(--peek-muted);
}
.icon-button:hover {
  background: var(--peek-hover-bg);
  color: var(--peek-text);
}
.filter-row,
.custom-range {
  display: flex;
  align-items: center;
}
.filter-row {
  min-height: 48px;
  justify-content: space-between;
  gap: 12px;
  border-bottom: 1px solid var(--peek-border);
}
.filter-row .settings-seg {
  width: auto;
}
.custom-range {
  gap: 8px;
}
.custom-range label {
  display: flex;
  align-items: center;
  gap: 5px;
  color: var(--peek-muted);
  font-size: 10px;
}
.custom-range input {
  height: 28px;
  padding: 0 6px;
  border: 1px solid var(--peek-border);
  border-radius: 5px;
  background: transparent;
  color: var(--peek-text);
  font-size: 10px;
}
.loader {
  width: 15px;
  height: 15px;
  border: 2px solid var(--peek-border);
  border-top-color: var(--peek-text);
  border-radius: 50%;
  animation: spin 700ms linear infinite;
}
.summary-strip {
  display: grid;
  grid-template-columns: 1.35fr repeat(3, 1fr);
  margin-bottom: 18px;
  overflow: hidden;
}
.summary-item {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 5px;
  padding: 15px 14px;
  border-right: 1px solid var(--peek-border);
}
.summary-item:last-child {
  border-right: 0;
}
.summary-item > span,
.summary-item small {
  color: var(--peek-muted);
  font-size: 10px;
}
.summary-item strong {
  overflow: hidden;
  font-size: 19px;
  font-weight: 650;
  text-overflow: ellipsis;
}
.summary-item.total strong {
  font-size: 23px;
}
.summary-item .accuracy {
  font-size: 14px;
  text-transform: capitalize;
}
.empty-state {
  min-height: 280px;
  flex-direction: column;
  gap: 7px;
}
.empty-state svg {
  margin-bottom: 5px;
  color: var(--peek-faint);
}
.empty-state strong {
  color: var(--peek-text);
  font-size: 13px;
}
.data-section {
  min-width: 0;
  padding-top: 18px;
}
.timeline-section {
  border-bottom: 1px solid var(--peek-border);
}
.section-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 12px;
}
.section-header h2 {
  margin: 0;
  font-size: 12px;
  font-weight: 650;
}
.detail-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 24px;
}
.spinning {
  animation: spin 700ms linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
@media (max-width: 700px) {
  .summary-strip {
    grid-template-columns: 1fr 1fr;
  }
  .summary-item:nth-child(2) {
    border-right: 0;
  }
  .summary-item:nth-child(-n + 2) {
    border-bottom: 1px solid var(--peek-border);
  }
  .detail-grid {
    grid-template-columns: 1fr;
    gap: 0;
  }
  .filter-row {
    align-items: flex-start;
    flex-direction: column;
    padding: 9px 0;
  }
  .custom-range {
    flex-wrap: wrap;
  }
}
</style>
