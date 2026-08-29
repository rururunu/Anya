<template>
  <span
    class="anya-logo"
    :role="alt ? 'img' : undefined"
    :aria-label="alt || undefined"
    :aria-hidden="alt ? undefined : true"
  />
</template>

<script setup lang="ts">
withDefaults(
  defineProps<{
    alt?: string;
  }>(),
  { alt: "" },
);
</script>

<style scoped>
.anya-logo {
  display: block;
  width: 100%;
  height: 100%;
  background-color: var(--peek-text);
  -webkit-mask: url("../../../src-tauri/icons/Anya-transparent.svg") center / contain no-repeat;
  mask: url("../../../src-tauri/icons/Anya-transparent.svg") center / contain no-repeat;
}
</style>
