<template>
  <template v-for="node in nodes" :key="node.path">
    <li
      class="tree-row"
      :style="{ paddingLeft: `${8 + depth * 14}px` }"
      role="treeitem"
      :aria-expanded="node.kind === 'dir' ? expanded.has(node.path) : undefined"
    >
      <Tooltip v-if="node.kind === 'dir'">
        <TooltipTrigger as-child>
          <button
            type="button"
            class="tree-item tree-dir"
            data-picker-trigger
            :class="{ pressing: pressPath === node.path }"
            :aria-label="`${folderTitle}: ${node.path}`"
            @pointerdown="onDirPointerDown($event, node.path)"
            @pointerup="onDirPointerUp($event, node.path)"
            @pointerleave="onDirPointerCancel(node.path)"
            @pointercancel="onDirPointerCancel(node.path)"
            @contextmenu.prevent
          >
            <ChevronRight
              :size="12"
              class="tree-chevron"
              :class="{ open: expanded.has(node.path) }"
            />
            <Folder :size="13" class="tree-icon" />
            <span class="tree-label">{{ node.name }}</span>
          </button>
        </TooltipTrigger>
        <TooltipContent side="top" :side-offset="6" class="attach-tree-tooltip">
          <span class="tip-action">{{ folderTitle }}</span>
          <span class="tip-path">{{ node.path }}</span>
        </TooltipContent>
      </Tooltip>

      <Tooltip v-else>
        <TooltipTrigger as-child>
          <button
            type="button"
            class="tree-item tree-file"
            :aria-label="`${fileTitle}: ${node.path}`"
            @mousedown.prevent="$emit('select-file', node.path, false)"
          >
            <span class="tree-chevron-spacer" aria-hidden="true" />
            <img
              v-if="iconForPath(node.path)"
              class="tree-file-icon"
              :src="iconForPath(node.path) || ''"
              alt=""
            />
            <File v-else :size="13" class="tree-icon" />
            <span class="tree-label">{{ node.name }}</span>
          </button>
        </TooltipTrigger>
        <TooltipContent side="top" :side-offset="6" class="attach-tree-tooltip">
          <span class="tip-action">{{ fileTitle }}</span>
          <span class="tip-path">{{ node.path }}</span>
        </TooltipContent>
      </Tooltip>
    </li>
    <AttachFileTreeNodes
      v-if="node.kind === 'dir' && expanded.has(node.path) && node.children?.length"
      :nodes="node.children"
      :depth="depth + 1"
      :expanded="expanded"
      :file-title="fileTitle"
      :folder-title="folderTitle"
      @toggle="$emit('toggle', $event)"
      @select-file="(path, isDir) => $emit('select-file', path, isDir)"
    />
  </template>
</template>

<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";
import { ChevronRight, File, Folder } from "@lucide/vue";
import { codeLanguageForPath } from "@/services/chat/codeLanguage";
import type { WorkspaceFileTreeNode } from "@/services/chat/workspaceFileTree";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

const LONG_PRESS_MS = 500;

defineProps<{
  nodes: WorkspaceFileTreeNode[];
  depth: number;
  expanded: Set<string>;
  fileTitle: string;
  folderTitle: string;
}>();

const emit = defineEmits<{
  toggle: [path: string];
  "select-file": [path: string, isDir?: boolean];
}>();

const pressPath = ref<string | null>(null);
const longPressed = ref(false);
let pressTimer: ReturnType<typeof setTimeout> | null = null;

function clearDirPress() {
  if (pressTimer != null) {
    globalThis.clearTimeout(pressTimer);
    pressTimer = null;
  }
  pressPath.value = null;
}

function onDirPointerDown(event: PointerEvent, path: string) {
  if (event.button !== 0) return;
  event.preventDefault();
  clearDirPress();
  longPressed.value = false;
  pressPath.value = path;
  pressTimer = globalThis.setTimeout(() => {
    pressTimer = null;
    if (pressPath.value !== path) return;
    longPressed.value = true;
    emit("select-file", path, true);
    pressPath.value = null;
  }, LONG_PRESS_MS);
}

function onDirPointerUp(event: PointerEvent, path: string) {
  if (event.button !== 0) return;
  event.preventDefault();
  const wasLong = longPressed.value;
  const active = pressPath.value === path || wasLong;
  clearDirPress();
  longPressed.value = false;
  if (active && !wasLong) {
    emit("toggle", path);
  }
}

function onDirPointerCancel(path: string) {
  if (pressPath.value !== path && !longPressed.value) return;
  clearDirPress();
  longPressed.value = false;
}

function iconForPath(path: string) {
  return codeLanguageForPath(path).icon;
}

onBeforeUnmount(() => {
  clearDirPress();
});
</script>

<style scoped>
.tree-row {
  list-style: none;
}

.tree-item {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  min-height: 26px;
  padding: 0 6px 0 2px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: inherit;
  font: inherit;
  font-size: 12px;
  text-align: left;
  cursor: pointer;
  touch-action: manipulation;
  user-select: none;
}

.tree-item:hover {
  background: color-mix(in srgb, var(--peek-accent) 12%, transparent);
  color: var(--peek-accent);
}

.tree-dir.pressing {
  background: color-mix(in srgb, var(--peek-accent) 18%, transparent);
  color: var(--peek-accent);
}

.tree-chevron {
  flex: none;
  color: var(--peek-muted);
  transition: transform 120ms ease;
}

.tree-chevron.open {
  transform: rotate(90deg);
}

.tree-chevron-spacer {
  flex: none;
  width: 12px;
  height: 12px;
}

.tree-icon {
  flex: none;
  color: var(--peek-muted);
}

.tree-file-icon {
  flex: none;
  width: 13px;
  height: 13px;
  object-fit: contain;
}

.tree-label {
  min-width: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tree-dir .tree-label {
  font-weight: 600;
}
</style>

<style>
/* Teleported tooltip — must not be scoped. */
.attach-tree-tooltip[data-slot="tooltip-content"],
.attach-tree-tooltip {
  display: flex !important;
  flex-direction: column;
  align-items: stretch !important;
  gap: 4px;
  box-sizing: border-box;
  width: max-content !important;
  max-width: min(260px, 70vw) !important;
  padding: 8px 10px !important;
  white-space: normal !important;
  text-align: left;
  line-height: 1.4;
}

.attach-tree-tooltip .tip-action {
  color: color-mix(in srgb, var(--peek-muted, #6b7280) 90%, transparent);
  font-size: 11px;
  font-weight: 600;
}

.attach-tree-tooltip .tip-path {
  overflow-wrap: anywhere;
  word-break: break-word;
  white-space: normal;
  font-size: 12px;
  font-weight: 500;
  color: inherit;
}
</style>
