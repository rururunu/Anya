<template>
  <ApprovalRequestPanel
    class="path-permission-list tool-approval-list"
    :header="header"
    :summary="summary"
    :detail="detail"
    :options="options"
    :selected-index="selectedIndex"
    :ariaLabel="ariaLabel"
    @hover="$emit('hover', $event)"
    @select="$emit('select', $event)"
  />
</template>

<script setup lang="ts">
import type { Component } from "vue";
import { computed } from "vue";
import ApprovalRequestPanel from "./ApprovalRequestPanel.vue";
import type { ToolApprovalDecision } from "@/types/chat";

const props = defineProps<{
  header: string;
  summary?: string;
  arguments?: Record<string, unknown>;
  options: Array<{
    slug: string;
    label: string;
    description: string;
    decision: ToolApprovalDecision;
    icon?: Component;
  }>;
  selectedIndex: number;
  ariaLabel: string;
}>();

defineEmits<{
  hover: [index: number];
  select: [decision: ToolApprovalDecision];
}>();
const detail = computed(() => {
  const args = props.arguments;
  if (!args) return "";
  const command = args.command ?? args.cmd;
  if (typeof command === "string") return command;
  const path = args.file_path ?? args.path;
  if (typeof path === "string") return path;
  return JSON.stringify(args, null, 2);
});
</script>
