<template>
  <section
    class="balance-card"
    :class="variant === 'row' ? 'is-row' : 'settings-card'"
    :aria-label="copy.title"
  >
    <header v-if="variant !== 'row'" class="balance-head">
      <div class="balance-head-main">
        <DeepSeekIcon :size="16" class="balance-icon" />
        <div>
          <h2>{{ copy.title }}</h2>
          <p v-if="statusText">{{ statusText }}</p>
        </div>
      </div>
      <button
        type="button"
        class="icon-button"
        :title="copy.refresh"
        :aria-label="copy.refresh"
        :disabled="loading"
        @click="emit('refresh')"
      >
        <RefreshCw :size="14" :class="{ spinning: loading }" />
      </button>
    </header>

    <div class="balance-body">
      <div v-if="loading && !report" class="balance-empty">
        <span class="loader" />
        {{ copy.loading }}
      </div>
      <p v-else-if="error" class="balance-error">{{ copy.error }}: {{ error }}</p>
      <p v-else-if="!report?.configured" class="balance-empty">{{ copy.unconfigured }}</p>
      <div v-else-if="rows.length" class="balance-rows">
        <article v-for="row in rows" :key="row.currency" class="balance-row">
          <span v-if="variant !== 'row'" class="balance-currency">{{ row.currency }}</span>
          <div class="balance-metrics">
            <div>
              <span>{{ copy.available }}</span>
              <strong>{{ row.total }}</strong>
            </div>
            <div>
              <span>{{ copy.granted }}</span>
              <strong>{{ row.granted }}</strong>
            </div>
            <div>
              <span>{{ copy.toppedUp }}</span>
              <strong>{{ row.toppedUp }}</strong>
            </div>
          </div>
        </article>
      </div>
      <p v-else class="balance-empty">{{ copy.empty }}</p>
      <button
        v-if="variant === 'row'"
        type="button"
        class="text-icon-button"
        :title="copy.refresh"
        :aria-label="copy.refresh"
        :disabled="loading"
        @click.stop="emit('refresh')"
      >
        <RefreshCw :size="14" :class="{ spinning: loading }" />
      </button>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { RefreshCw } from "@lucide/vue";
import DeepSeekIcon from "@/components/icons/DeepSeekIcon.vue";
import type { DeepSeekBalanceReport } from "@/types/tokenUsage";

const props = withDefaults(
  defineProps<{
    report: DeepSeekBalanceReport | null;
    loading: boolean;
    error: string;
    language: string;
    variant?: "card" | "row";
    copy: {
      title: string;
      available: string;
      granted: string;
      toppedUp: string;
      ready: string;
      empty: string;
      unconfigured: string;
      error: string;
      loading: string;
      refresh: string;
    };
  }>(),
  { variant: "card" },
);

const emit = defineEmits<{
  refresh: [];
}>();

const statusText = computed(() => {
  if (props.loading || props.error || !props.report?.configured) return "";
  return props.report.isAvailable ? props.copy.ready : props.copy.empty;
});

const rows = computed(() =>
  (props.report?.balances ?? []).map((item) => ({
    currency: item.currency,
    total: formatMoney(item.currency, item.totalBalance),
    granted: formatMoney(item.currency, item.grantedBalance),
    toppedUp: formatMoney(item.currency, item.toppedUpBalance),
  })),
);

function formatMoney(currency: string, value: string) {
  const amount = Number(value);
  if (!Number.isFinite(amount)) return `${currency} ${value}`;
  try {
    return new Intl.NumberFormat(props.language, {
      style: "currency",
      currency,
      minimumFractionDigits: 2,
    }).format(amount);
  } catch {
    return `${currency} ${value}`;
  }
}
</script>

<style scoped>
.balance-card {
  margin: 0 0 28px;
  padding: 14px 16px 12px;
}
.balance-card.is-row {
  margin: 0;
  padding: 10px 16px 12px 54px;
  border: 0;
  border-top: 1px solid color-mix(in srgb, var(--peek-border) 70%, transparent);
  border-radius: 0;
  background: transparent;
}
.balance-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 10px;
}
.balance-head-main {
  min-width: 0;
  display: flex;
  align-items: flex-start;
  gap: 8px;
}
.balance-icon {
  flex: none;
  margin-top: 1px;
}
.balance-body {
  min-width: 0;
}
.balance-card.is-row .balance-body {
  display: flex;
  align-items: center;
  gap: 10px;
}
.balance-card.is-row .balance-rows,
.balance-card.is-row .balance-empty,
.balance-card.is-row .balance-error {
  flex: 1;
  min-width: 0;
}
.icon-button {
  flex: none;
  width: 32px;
  height: 32px;
  display: grid;
  place-items: center;
  border: 1px solid var(--peek-border);
  border-radius: 6px;
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
}
.icon-button:hover:not(:disabled) {
  background: var(--peek-hover-bg);
  color: var(--peek-text);
}
.icon-button:disabled {
  cursor: default;
  opacity: 0.7;
}
.text-icon-button {
  flex: none;
  width: 22px;
  height: 22px;
  display: grid;
  place-items: center;
  margin: 0;
  padding: 0;
  border: 0;
  border-radius: 0;
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
}
.text-icon-button:hover:not(:disabled) {
  background: transparent;
  color: var(--peek-text);
}
.text-icon-button:disabled {
  cursor: default;
  opacity: 0.7;
}
.spinning {
  animation: spin 700ms linear infinite;
}
.balance-head h2 {
  margin: 0;
  font-size: 12px;
  font-weight: 650;
}
.balance-head p {
  margin: 3px 0 0;
  color: var(--peek-muted);
  font-size: 10px;
}
.balance-empty,
.balance-error {
  margin: 0;
  color: var(--peek-muted);
  font-size: 11px;
}
.balance-error {
  color: var(--peek-danger, #b42318);
}
.balance-rows {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.balance-row {
  min-width: 0;
}
.balance-currency {
  display: inline-block;
  margin-bottom: 6px;
  color: var(--peek-faint);
  font-size: 9px;
  letter-spacing: 0.04em;
}
.balance-metrics {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
}
.balance-card.is-row .balance-metrics {
  gap: 16px;
}
.balance-metrics div {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.balance-card.is-row .balance-metrics div {
  gap: 2px;
}
.balance-metrics span {
  color: var(--peek-muted);
  font-size: 10px;
}
.balance-metrics strong {
  overflow: hidden;
  font-size: 18px;
  font-weight: 650;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.balance-card.is-row .balance-metrics strong {
  font-size: 13px;
}
.loader {
  width: 13px;
  height: 13px;
  display: inline-block;
  margin-right: 6px;
  border: 2px solid var(--peek-border);
  border-top-color: var(--peek-text);
  border-radius: 50%;
  vertical-align: -2px;
  animation: spin 700ms linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
@media (max-width: 700px) {
  .balance-metrics {
    grid-template-columns: 1fr;
    gap: 8px;
  }
  .balance-card.is-row .balance-metrics {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
}
</style>
