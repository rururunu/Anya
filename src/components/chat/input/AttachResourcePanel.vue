<template>
  <div
    class="command-list attach-resource-panel peek-scrollbar"
    data-tauri-drag-region="false"
    role="dialog"
    :aria-label="ariaLabel"
  >
    <div class="attach-tabs" role="tablist">
      <button
        type="button"
        role="tab"
        class="attach-tab"
        data-picker-trigger
        :class="{ active: tab === 'skills' }"
        :aria-selected="tab === 'skills'"
        @mousedown.prevent="$emit('tab-change', 'skills')"
      >
        <Zap :size="12" />
        {{ skillsLabel }}
        <span v-if="!loading" class="attach-tab-count">{{ skills.length }}</span>
      </button>
      <button
        type="button"
        role="tab"
        class="attach-tab"
        data-picker-trigger
        :class="{ active: tab === 'mcp' }"
        :aria-selected="tab === 'mcp'"
        @mousedown.prevent="$emit('tab-change', 'mcp')"
      >
        <Bot :size="12" />
        {{ mcpLabel }}
        <span v-if="!loading" class="attach-tab-count">{{ mcpServers.length }}</span>
      </button>
      <button
        type="button"
        role="tab"
        class="attach-tab"
        data-picker-trigger
        :class="{ active: tab === 'files' }"
        :aria-selected="tab === 'files'"
        @mousedown.prevent="$emit('tab-change', 'files')"
      >
        <FolderOpen :size="12" />
        {{ filesLabel }}
      </button>
    </div>

    <div v-if="tab === 'files'" class="attach-files-pane">
      <div class="attach-tree-scroll peek-scrollbar">
        <p v-if="filesLoading" class="attach-empty">{{ filesLoadingText }}</p>
        <p v-else-if="!hasWorkspace" class="attach-empty">{{ noWorkspaceText }}</p>
        <p v-else-if="fileTree.length === 0" class="attach-empty">{{ emptyFilesText }}</p>
        <ul v-else class="attach-tree" role="tree">
          <TooltipProvider :delay-duration="280">
            <AttachFileTreeNodes
              :nodes="fileTree"
              :depth="0"
              :expanded="expandedDirs"
              :file-title="insertFileTitle"
              :folder-title="insertFolderTitle"
              @toggle="toggleDir"
              @select-file="(path, isDir) => $emit('select-file', path, isDir)"
            />
          </TooltipProvider>
        </ul>
      </div>

      <button
        type="button"
        class="attach-files-row"
        data-picker-trigger
        :disabled="pickingFiles"
        @mousedown.prevent="$emit('pick-files')"
      >
        <FolderOpen :size="14" />
        <span>{{ pickFilesLabel }}</span>
      </button>
    </div>

    <template v-else>
      <p v-if="loading" class="attach-empty">{{ loadingText }}</p>
      <p v-else-if="activeItems.length === 0" class="attach-empty">
        {{ tab === "skills" ? emptySkillsText : emptyMcpText }}
      </p>
      <div v-else class="attach-chip-grid">
        <button
          v-for="(item, index) in visibleItems"
          :key="`${item.kind}:${item.id}`"
          type="button"
          class="attach-chip"
          :class="{ active: selectedIndex === index }"
          :title="chipTitle(item)"
          @mouseenter="$emit('hover', { kind: tab === 'mcp' ? 'mcp' : 'skill', index })"
          @mousedown.prevent="$emit('select', item)"
        >
          <span class="attach-icon" aria-hidden="true">
            <img
              v-if="item.iconUrl && !brokenIcons[itemKey(item)]"
              :src="item.iconUrl"
              alt=""
              referrerpolicy="no-referrer"
              @error="markBroken(item)"
            />
            <span v-else class="attach-icon-fallback">{{ fallbackLetter(item) }}</span>
          </span>
          <span class="attach-chip-label">{{ item.title || item.id }}</span>
        </button>

        <button
          v-if="hiddenCount > 0 && !expanded"
          type="button"
          class="attach-chip attach-expand-chip"
          data-picker-trigger
          @mousedown.prevent="expanded = true"
        >
          <ChevronDown :size="12" />
          <span>{{ expandLabel(hiddenCount) }}</span>
        </button>
        <button
          v-else-if="expanded && activeItems.length > PREVIEW_COUNT"
          type="button"
          class="attach-chip attach-expand-chip"
          data-picker-trigger
          @mousedown.prevent="expanded = false"
        >
          <ChevronUp :size="12" />
          <span>{{ collapseLabel }}</span>
        </button>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { Bot, ChevronDown, ChevronUp, FolderOpen, Zap } from "@lucide/vue";
import type { HashMentionItem } from "@/services/chat/hashMentions";
import { buildWorkspaceFileTree } from "@/services/chat/workspaceFileTree";
import AttachFileTreeNodes from "./AttachFileTreeNodes.vue";
import { TooltipProvider } from "@/components/ui/tooltip";

const PREVIEW_COUNT = 3;

const props = defineProps<{
  tab: "skills" | "mcp" | "files";
  loading: boolean;
  pickingFiles: boolean;
  skills: HashMentionItem[];
  mcpServers: HashMentionItem[];
  selectedIndex: number;
  workspaceFiles: string[];
  filesLoading: boolean;
  hasWorkspace: boolean;
  ariaLabel: string;
  skillsLabel: string;
  mcpLabel: string;
  filesLabel: string;
  pickFilesLabel: string;
  filesLoadingText: string;
  noWorkspaceText: string;
  emptyFilesText: string;
  insertFileTitle: string;
  insertFolderTitle: string;
  loadingText: string;
  emptySkillsText: string;
  emptyMcpText: string;
  expandMoreLabel: string;
  collapseLabel: string;
}>();

const emit = defineEmits<{
  "tab-change": [tab: "skills" | "mcp" | "files"];
  "pick-files": [];
  "select-file": [path: string, isDir?: boolean];
  hover: [payload: { kind: "skill" | "mcp"; index: number }];
  select: [item: HashMentionItem];
  "visible-count": [count: number];
}>();

const expanded = ref(false);
const expandedDirs = ref<Set<string>>(new Set());
const brokenIcons = reactive<Record<string, boolean>>({});

const activeItems = computed(() => (props.tab === "mcp" ? props.mcpServers : props.skills));

const visibleItems = computed(() =>
  expanded.value ? activeItems.value : activeItems.value.slice(0, PREVIEW_COUNT),
);

const hiddenCount = computed(() => Math.max(0, activeItems.value.length - PREVIEW_COUNT));

const fileTree = computed(() => buildWorkspaceFileTree(props.workspaceFiles));

watch(
  () => props.tab,
  () => {
    expanded.value = false;
  },
);

watch(
  fileTree,
  (tree) => {
    // Expand top-level directories by default for a usable first view.
    const next = new Set<string>();
    for (const node of tree) {
      if (node.kind === "dir") next.add(node.path);
    }
    expandedDirs.value = next;
  },
  { immediate: true },
);

watch(
  visibleItems,
  (items) => {
    emit("visible-count", items.length);
  },
  { immediate: true },
);

watch(
  () =>
    [...props.skills, ...props.mcpServers]
      .map((item) => `${item.kind}:${item.id}:${item.iconUrl ?? ""}`)
      .join("|"),
  () => {
    for (const key of Object.keys(brokenIcons)) {
      delete brokenIcons[key];
    }
  },
);

function toggleDir(path: string) {
  const next = new Set(expandedDirs.value);
  if (next.has(path)) next.delete(path);
  else next.add(path);
  expandedDirs.value = next;
}

function expandLabel(count: number) {
  return props.expandMoreLabel.replace("{count}", String(count));
}

function itemKey(item: HashMentionItem) {
  return `${item.kind}:${item.id}`;
}

function markBroken(item: HashMentionItem) {
  brokenIcons[itemKey(item)] = true;
}

function fallbackLetter(item: HashMentionItem): string {
  const source = item.title || item.id || "?";
  return source.trim().charAt(0).toUpperCase() || "?";
}

function chipTitle(item: HashMentionItem): string {
  const vendor = item.vendor?.trim();
  return vendor ? `${item.title || item.id} · ${vendor}` : item.title || item.id;
}
</script>

<style scoped>
.attach-resource-panel {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin: 0;
  padding: 10px;
  border-bottom: 1px solid var(--peek-border);
  background: var(--peek-list-bg);
  flex: none;
  max-height: min(420px, 72vh);
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
}

.attach-tabs {
  flex: none;
  display: flex;
  gap: 4px;
  padding: 2px;
  border-radius: 10px;
  background: color-mix(in srgb, var(--peek-text) 4%, transparent);
}

.attach-tab {
  flex: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  min-height: 30px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: var(--peek-muted);
  font: inherit;
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.02em;
  cursor: pointer;
  transition:
    background 120ms ease,
    color 120ms ease;
}

.attach-tab.active {
  background: var(--peek-list-bg);
  color: var(--peek-text);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--peek-border) 70%, transparent);
}

.attach-tab-count {
  font-variant-numeric: tabular-nums;
  font-weight: 600;
  opacity: 0.7;
}

.attach-files-pane {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 8px;
  min-height: 0;
}

.attach-tree-scroll {
  flex: 1;
  min-height: 120px;
  max-height: min(260px, 48vh);
  overflow: auto;
  overscroll-behavior: contain;
  border-radius: 8px;
  background: color-mix(in srgb, var(--peek-text) 2.5%, transparent);
  padding: 4px;
}

.attach-tree {
  list-style: none;
  margin: 0;
  padding: 0;
}

.attach-files-row {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  width: 100%;
  min-height: 36px;
  border: 1px dashed color-mix(in srgb, var(--peek-border) 80%, transparent);
  border-radius: 10px;
  background: color-mix(in srgb, var(--peek-text) 3.5%, transparent);
  color: var(--peek-text);
  font: inherit;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition:
    background 120ms ease,
    border-color 120ms ease,
    color 120ms ease;
}

.attach-files-row:hover:not(:disabled) {
  background: color-mix(in srgb, var(--peek-accent) 10%, transparent);
  border-color: color-mix(in srgb, var(--peek-accent) 35%, var(--peek-border));
  color: var(--peek-accent);
}

.attach-files-row:disabled {
  opacity: 0.6;
  cursor: default;
}

.attach-empty {
  margin: 0;
  padding: 14px 6px;
  color: var(--peek-muted);
  font-size: 12px;
}

.attach-chip-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding: 2px 0 4px;
}

.attach-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  max-width: 100%;
  min-height: 28px;
  padding: 0 10px 0 8px;
  border: 0;
  border-radius: 999px;
  background: color-mix(in srgb, var(--peek-text) 5%, transparent);
  color: var(--peek-text);
  font: inherit;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  box-sizing: border-box;
  transition:
    background 120ms ease,
    color 120ms ease;
}

.attach-chip:hover,
.attach-chip.active {
  background: color-mix(in srgb, var(--peek-accent) 14%, transparent);
  color: var(--peek-accent);
}

.attach-expand-chip {
  color: var(--peek-muted);
  font-weight: 600;
}

.attach-expand-chip:hover,
.attach-expand-chip.active {
  color: var(--peek-accent);
}

.attach-icon {
  flex: none;
  width: 14px;
  height: 14px;
  display: grid;
  place-items: center;
  overflow: hidden;
}

.attach-icon img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  display: block;
}

.attach-icon-fallback {
  font-size: 9px;
  font-weight: 700;
  color: var(--peek-muted);
  line-height: 1;
}

.attach-chip-label {
  min-width: 0;
  max-width: 9.5rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
