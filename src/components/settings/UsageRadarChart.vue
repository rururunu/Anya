<template>
  <div v-if="chart.axes.length" class="radar-chart">
    <svg
      :viewBox="`0 0 ${RADAR_SIZE} ${RADAR_SIZE}`"
      class="radar"
      role="img"
      :aria-label="chartLabel"
    >
      <polygon
        v-for="(ring, index) in chart.rings"
        :key="`ring:${index}`"
        :points="ring"
        class="radar-ring"
      />
      <line
        v-for="(spoke, index) in chart.spokes"
        :key="`spoke:${index}`"
        :x1="RADAR_CX"
        :y1="RADAR_CY"
        :x2="spoke.x"
        :y2="spoke.y"
        class="radar-spoke"
      />
      <polygon :points="chart.polygon" class="radar-fill" />
      <polygon :points="chart.polygon" class="radar-outline" />
      <g
        v-for="axis in chart.axes"
        :key="axis.id"
        class="radar-axis"
        :class="{ dim: hoveredId != null && hoveredId !== axis.id, on: hoveredId === axis.id }"
        @mouseenter="hoveredId = axis.id"
        @mouseleave="hoveredId = null"
      >
        <circle :cx="axis.x" :cy="axis.y" r="3.5" :fill="axis.color" />
        <text
          :x="axis.labelX"
          :y="axis.labelY"
          :text-anchor="axis.anchor"
          class="radar-label"
          dominant-baseline="middle"
        >
          {{ axis.label }}
        </text>
      </g>
    </svg>
    <ul class="radar-legend">
      <li
        v-for="axis in chart.axes"
        :key="axis.id"
        :class="{ on: hoveredId === axis.id }"
        @mouseenter="hoveredId = axis.id"
        @mouseleave="hoveredId = null"
      >
        <i :style="{ background: axis.color }" />
        <strong>{{ axis.label }}</strong>
        <span>{{ formatSharePercent(axis.share) }}</span>
        <b>{{ format(axis.value) }}</b>
      </li>
    </ul>
  </div>
  <p v-else class="radar-empty">{{ emptyLabel }}</p>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import {
  RADAR_CX,
  RADAR_CY,
  RADAR_SIZE,
  buildRadarChart,
  formatSharePercent,
  type ShareDatum,
} from "@/services/usage/tokenUsageChart";

const props = defineProps<{
  items: ShareDatum[];
  format: (value: number) => string;
  emptyLabel: string;
  chartLabel: string;
}>();

const hoveredId = ref<string | null>(null);
const chart = computed(() => buildRadarChart(props.items));
</script>

<style scoped>
.radar-chart {
  display: grid;
  grid-template-columns: 240px minmax(0, 1fr);
  gap: 8px 12px;
  align-items: center;
}
.radar {
  width: 240px;
  height: 240px;
  display: block;
  overflow: visible;
}
.radar-ring,
.radar-spoke {
  fill: none;
  stroke: color-mix(in srgb, var(--peek-border) 90%, transparent);
  stroke-width: 1;
}
.radar-fill {
  fill: color-mix(in srgb, var(--peek-accent) 22%, transparent);
}
.radar-outline {
  fill: none;
  stroke: var(--peek-accent);
  stroke-width: 1.5;
  stroke-linejoin: round;
}
.radar-axis {
  cursor: pointer;
}
.radar-axis.dim {
  opacity: 0.35;
}
.radar-axis.on circle {
  stroke: var(--peek-surface);
  stroke-width: 2;
}
.radar-label {
  fill: var(--peek-muted);
  font-size: 9px;
  paint-order: stroke;
  stroke: var(--peek-surface);
  stroke-width: 3px;
}
.radar-legend {
  min-width: 0;
  margin: 0;
  padding: 0;
  list-style: none;
}
.radar-legend li {
  display: grid;
  grid-template-columns: 10px minmax(0, 1fr) auto auto;
  gap: 8px;
  align-items: center;
  min-height: 28px;
  padding: 2px 4px;
  border-radius: 6px;
}
.radar-legend li.on {
  background: color-mix(in srgb, var(--peek-text) 6%, transparent);
}
.radar-legend i {
  width: 8px;
  height: 8px;
  justify-self: center;
  border-radius: 2px;
}
.radar-legend strong {
  min-width: 0;
  overflow: hidden;
  font-size: 11px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.radar-legend span,
.radar-legend b {
  color: var(--peek-muted);
  font-size: 10px;
  font-weight: 500;
  white-space: nowrap;
}
.radar-legend b {
  color: var(--peek-text);
  font-weight: 650;
}
.radar-empty {
  margin: 12px 0 0;
  color: var(--peek-muted);
  font-size: 11px;
}
@media (max-width: 700px) {
  .radar-chart {
    grid-template-columns: 1fr;
    justify-items: center;
  }
  .radar-legend {
    width: 100%;
  }
}
</style>
