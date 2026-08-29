<template>
  <p class="settings-empty">
    <component :is="icon" v-if="icon" class="size-5" aria-hidden="true" />
    <span>
      <slot>{{ message }}</slot>
    </span>
  </p>
</template>

<script setup lang="ts">
import type { Component } from "vue";

withDefaults(
  defineProps<{
    message?: string;
    icon?: Component;
  }>(),
  {
    message: "",
  },
);
</script>
