<template>
  <div class="model-provider-tip">
    <span class="tip-icon" aria-hidden="true">
      <component :is="brandIcon" v-if="brandIcon" :size="16" />
      <img v-else-if="favicon" :src="favicon" alt="" class="tip-favicon" />
      <Globe2 v-else :size="16" />
    </span>
    <span class="tip-copy">
      <span class="tip-name">{{ name }}</span>
      <span v-if="detail" class="tip-detail">{{ detail }}</span>
    </span>
  </div>
</template>

<script setup lang="ts">
import type { Component } from "vue";
import { Globe2 } from "@lucide/vue";

defineProps<{
  name: string;
  detail?: string | null;
  brandIcon?: Component | null;
  favicon?: string | null;
}>();
</script>

<style>
/* Teleported tooltip — must not be scoped. */
.model-provider-tip {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  max-width: 240px;
}

.model-provider-tip .tip-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  flex: none;
  color: var(--popover-foreground, var(--foreground));
}

.model-provider-tip .tip-favicon {
  width: 16px;
  height: 16px;
  border-radius: 3px;
  object-fit: contain;
}

.model-provider-tip .tip-copy {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 1px;
}

.model-provider-tip .tip-name {
  font-size: 12px;
  font-weight: 600;
  line-height: 16px;
  color: var(--popover-foreground, var(--foreground));
}

.model-provider-tip .tip-detail {
  font-family: var(--font-mono, ui-monospace, monospace);
  font-size: 10px;
  line-height: 13px;
  color: var(--muted-foreground);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
