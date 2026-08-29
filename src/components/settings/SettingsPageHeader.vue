<template>
  <header class="settings-page-header" :class="{ 'is-actions-only': hideTitle }">
    <div v-if="!hideTitle" class="settings-page-header-main">
      <h2>{{ title }}</h2>
      <p v-if="description" class="settings-page-lede">{{ description }}</p>
    </div>
    <div v-if="$slots.actions" class="settings-page-header-actions">
      <slot name="actions" />
    </div>
  </header>
</template>

<script setup lang="ts">
defineProps<{
  title: string;
  description?: string;
  hideTitle?: boolean;
}>();
</script>

<style scoped>
.settings-page-header-main {
  min-width: 0;
  flex: 1;
}

.settings-page-header-main .settings-page-lede {
  margin: 6px 0 0;
}
</style>
