<template>
  <button
    v-if="text"
    type="button"
    class="settings-help"
    :data-ui-tooltip="text"
    :aria-label="text"
  >
    <CircleHelp class="settings-help-icon" aria-hidden="true" />
  </button>
</template>

<script setup lang="ts">
import { CircleHelp } from "@lucide/vue";

defineProps<{
  text: string;
}>();
</script>
