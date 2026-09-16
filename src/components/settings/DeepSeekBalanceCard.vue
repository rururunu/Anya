<template>
  <section class="settings-card balance-card" :aria-label="copy.title">
    <header class="balance-head">
      <DeepSeekIcon :size="16" class="balance-icon" />
      <div>
        <h2>{{ copy.title }}</h2>
        <p v-if="statusText">{{ statusText }}</p>
      </div>
    </header>

    <div v-if="loading" class="balance-empty">
      <span class="loader" />
      {{ copy.loading }}
    </div>
    <p v-else-if="error" class="balance-error">{{ copy.error }}: {{ error }}</p>
    <p v-else-if="!report?.configured" class="balance-empty">{{ copy.unconfigured }}</p>
    <div v-else-if="rows.length" class="balance-rows">
      <article v-for="row in rows" :key="row.currency" class="balance-row">
        <span class="balance-currency">{{ row.currency }}</span>
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
  </section>
</template>

<script setup lang="ts">
import { computed } from "vue";
import DeepSeekIcon from "@/components/icons/DeepSeekIcon.vue";
import type { DeepSeekBalanceReport } from "@/types/tokenUsage";

const props = defineProps<{
  report: DeepSeekBalanceReport | null;
  loading: boolean;
  error: string;
  language: string;
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
  };
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
  margin: 0 0 18px;
  padding: 14px 16px 12px;
}
.balance-head {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  margin-bottom: 10px;
}
.balance-icon {
  flex: none;
  margin-top: 1px;
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
.balance-metrics div {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
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
}
</style>
