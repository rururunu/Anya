<template>
  <header class="settings-page-header">
    <h2>{{ title }}</h2>
    <div v-if="$slots.actions" class="settings-page-header-actions">
      <slot name="actions" />
    </div>
  </header>
</template>

<script setup lang="ts">
defineProps<{
  title: string;
}>();
</script>
