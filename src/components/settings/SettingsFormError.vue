<template>
  <p v-if="message" class="settings-form-error" role="alert">
    {{ message }}
  </p>
</template>

<script setup lang="ts">
defineProps<{
  message?: string | null;
}>();
</script>
