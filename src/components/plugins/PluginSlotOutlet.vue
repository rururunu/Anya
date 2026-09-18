<template>
  <div
    v-if="items.length"
    class="plugin-slot-outlet"
    :class="{
      'is-fit-content': fit === 'content',
      'is-wrap': layout === 'wrap',
      'is-rail': layout === 'rail',
      'is-idle': idle,
    }"
  >
    <PluginHostPane
      v-for="item in items"
      :key="item.id"
      :mount-fn="item.mount"
      :plugin-id="item.pluginId"
      :fit="fit"
      :active="isActive(item.id)"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import PluginHostPane from "@/components/plugins/PluginHostPane.vue";
import {
  getSlotEntries,
  tabShowsOn,
  type PluginChromeSurface,
} from "@/composables/plugins/slotRegistry";

const props = withDefaults(
  defineProps<{
    anchorId: string;
    fit?: "fill" | "content";
    pluginId?: string;
    chrome?: PluginChromeSurface;
    /** Skip tabs that do not open the review strip (`views`). */
    excludeMainPane?: boolean;
    activeId?: string | null;
    hostActive?: boolean;
    layout?: "stack" | "wrap" | "rail";
  }>(),
  { fit: "fill", hostActive: true, layout: "stack", excludeMainPane: false },
);

const items = computed(() => {
  let list = getSlotEntries(props.anchorId);
  if (props.pluginId) list = list.filter((item) => item.pluginId === props.pluginId);
  if (props.chrome) {
    const chrome = props.chrome;
    list = list.filter((item) => tabShowsOn(item.chrome, chrome));
  }
  if (props.excludeMainPane) list = list.filter((item) => tabShowsOn(item.chrome, "views"));
  return list;
});

function isActive(id: string) {
  if (!props.hostActive) return false;
  if (props.activeId == null || props.activeId === "") return true;
  return id === props.activeId;
}

/** Hide when a sibling pane (diff/plan) is selected so flex:1 does not steal half the height. */
const idle = computed(
  () => items.value.length > 0 && !items.value.some((item) => isActive(item.id)),
);
</script>

<style scoped>
.plugin-slot-outlet {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  width: 100%;
}
.plugin-slot-outlet.is-fit-content {
  flex: none;
  min-height: 0;
}
.plugin-slot-outlet.is-wrap {
  flex-direction: row;
  flex-wrap: wrap;
  gap: 8px;
  width: auto;
}
.plugin-slot-outlet.is-rail {
  flex: 0 1 auto;
  max-height: min(38vh, 340px);
  overflow: auto;
  padding: 8px 12px 6px;
  border-bottom: 1px solid color-mix(in srgb, var(--peek-border) 42%, transparent);
}
.plugin-slot-outlet.is-rail :deep(.plugin-host-pane) {
  width: 100%;
}
.plugin-slot-outlet.is-idle {
  display: none;
}
</style>
