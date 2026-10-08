<template>
  <ApprovalRequestPanel
    class="path-permission-list"
    :header="header"
    :summary="question"
    :detail="path"
    :options="options"
    :selected-index="selectedIndex"
    :ariaLabel="ariaLabel"
    @hover="$emit('hover', $event)"
    @select="$emit('select', $event)"
  />
</template>

<script setup lang="ts">
import type { Component } from "vue";
import ApprovalRequestPanel from "./ApprovalRequestPanel.vue";
import type { PathPermissionDecision } from "@/types/chat";

defineProps<{
  header: string;
  question: string;
  path?: string;
  options: Array<{
    slug: string;
    label: string;
    description: string;
    decision: PathPermissionDecision;
    icon?: Component;
  }>;
  selectedIndex: number;
  ariaLabel: string;
}>();

defineEmits<{
  hover: [index: number];
  select: [decision: PathPermissionDecision];
}>();
</script>
