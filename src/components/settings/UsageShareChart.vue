<template>
  <div v-if="arcs.length" class="share-chart">
    <div class="donut-wrap">
      <svg
        :viewBox="`0 0 ${DONUT_SIZE} ${DONUT_SIZE}`"
        class="donut"
        role="img"
        :aria-label="chartLabel"
      >
        <path
          v-for="arc in arcs"
          :key="arc.id"
          :d="arc.path"
          :fill="arc.color"
          class="donut-seg"
          :class="{ dim: hoveredId != null && hoveredId !== arc.id }"
          @mouseenter="hoveredId = arc.id"
          @mouseleave="hoveredId = null"
        />
      </svg>
      <div class="donut-center">
        <strong>{{ centerValue }}</strong>
        <small>{{ centerLabel }}</small>
      </div>
    </div>
    <ul class="share-legend">
      <li
        v-for="item in arcs"
        :key="item.id"
        :class="{ on: hoveredId === item.id }"
        @mouseenter="hoveredId = item.id"
        @mouseleave="hoveredId = null"
      >
        <div class="legend-row">
          <component :is="item.icon" v-if="item.icon" :size="13" class="legend-icon" />
          <i v-else :style="{ background: item.color }" />
          <strong :title="item.hint">{{ item.label }}</strong>
          <span>{{ formatSharePercent(item.share) }}</span>
          <b>{{ format(item.value) }}</b>
        </div>
        <p v-if="item.hint && !item.children?.length" class="legend-hint">{{ item.hint }}</p>
        <div v-for="child in item.children" :key="child.id" class="legend-child">
          <i :style="{ background: child.color }" />
          <span :title="child.hint">{{ child.label }}</span>
          <small>{{ formatSharePercent(childShare(child.value)) }}</small>
          <b>{{ format(child.value) }}</b>
        </div>
      </li>
    </ul>
  </div>
  <p v-else class="share-empty">{{ emptyLabel }}</p>
</template>

<script setup lang="ts">
import { computed, ref, type Component } from "vue";
import {
  DONUT_SIZE,
  buildDonutArcs,
  formatSharePercent,
  type ShareDatum,
} from "@/services/usage/tokenUsageChart";

type ShareChartItem = ShareDatum & { icon?: Component };

const props = defineProps<{
  items: ShareChartItem[];
  format: (value: number) => string;
  totalLabel: string;
  emptyLabel: string;
  chartLabel: string;
}>();

const hoveredId = ref<string | null>(null);
const total = computed(() => props.items.reduce((sum, item) => sum + Math.max(0, item.value), 0));
const arcs = computed(() =>
  buildDonutArcs(props.items).map((arc) => ({
    ...arc,
    icon: props.items.find((item) => item.id === arc.id)?.icon,
  })),
);
const hovered = computed(() => arcs.value.find((item) => item.id === hoveredId.value) ?? null);
const centerValue = computed(() =>
  hovered.value ? formatSharePercent(hovered.value.share) : props.format(total.value),
);
const centerLabel = computed(() => hovered.value?.label ?? props.totalLabel);

function childShare(value: number) {
  return total.value > 0 ? value / total.value : 0;
}
</script>

<style scoped>
.share-chart {
  display: grid;
  grid-template-columns: 160px minmax(0, 1fr);
  gap: 16px 18px;
  align-items: start;
  padding-top: 4px;
}
.donut-wrap {
  position: relative;
  width: 160px;
  height: 160px;
}
.donut {
  width: 160px;
  height: 160px;
  display: block;
}
.donut-seg {
  cursor: pointer;
  transition: opacity 120ms ease;
}
.donut-seg.dim {
  opacity: 0.38;
}
.donut-center {
  position: absolute;
  inset: 46px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
  pointer-events: none;
  text-align: center;
}
.donut-center strong {
  max-width: 100%;
  overflow: hidden;
  font-size: 15px;
  font-weight: 650;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.donut-center small {
  max-width: 100%;
  overflow: hidden;
  color: var(--peek-muted);
  font-size: 9px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.share-legend {
  min-width: 0;
  margin: 0;
  padding: 0;
  list-style: none;
}
.share-legend li {
  padding: 6px 4px;
  border-radius: 6px;
}
.share-legend li.on {
  background: color-mix(in srgb, var(--peek-text) 6%, transparent);
}
.legend-row,
.legend-child {
  display: grid;
  grid-template-columns: 12px minmax(0, 1fr) auto auto;
  gap: 8px;
  align-items: center;
}
.legend-row i,
.legend-child i {
  width: 8px;
  height: 8px;
  justify-self: center;
  border-radius: 2px;
}
.legend-icon {
  justify-self: center;
  color: var(--peek-text);
}
.legend-row strong,
.legend-child span {
  min-width: 0;
  overflow: hidden;
  font-size: 11px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.legend-row span,
.legend-row b,
.legend-child small,
.legend-child b {
  color: var(--peek-muted);
  font-size: 10px;
  font-weight: 500;
  white-space: nowrap;
}
.legend-row b,
.legend-child b {
  color: var(--peek-text);
  font-weight: 650;
}
.legend-hint {
  margin: 2px 0 0 20px;
  color: var(--peek-faint);
  font-size: 9px;
}
.legend-child {
  margin-top: 4px;
  padding-left: 12px;
}
.legend-child span {
  font-weight: 500;
  color: var(--peek-muted);
}
.share-empty {
  margin: 12px 0 0;
  color: var(--peek-muted);
  font-size: 11px;
}
@media (max-width: 700px) {
  .share-chart {
    grid-template-columns: 1fr;
    justify-items: center;
  }
  .share-legend {
    width: 100%;
  }
}
</style>
