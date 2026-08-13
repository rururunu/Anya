<template>
  <ul
    class="command-list hash-suggestion-list peek-scrollbar"
    data-tauri-drag-region="false"
    role="listbox"
    :aria-label="ariaLabel"
  >
    <li v-if="loading" class="picker-meta question">{{ loadingText }}</li>
    <li v-else-if="items.length === 0" class="picker-meta question">{{ emptyText }}</li>
    <template v-else>
      <template v-for="section in sections" :key="section.kind">
        <li v-if="showSectionHeaders" class="hash-section-head" role="presentation">
          {{ section.kind === "skill" ? skillLabel : mcpLabel }}
        </li>
        <li
          v-for="row in section.rows"
          :key="`${row.item.kind}:${row.item.id}`"
          class="command-item hash-suggestion-item"
          :class="{ active: row.index === selectedIndex }"
          role="option"
          :aria-selected="row.index === selectedIndex"
          :title="rowTitle(row.item)"
          @mouseenter="$emit('hover', row.index)"
          @mousedown.prevent="$emit('select', row.item)"
        >
          <span class="hash-icon" aria-hidden="true">
            <img
              v-if="row.item.iconUrl && !brokenIcons[itemKey(row.item)]"
              :src="row.item.iconUrl"
              alt=""
              referrerpolicy="no-referrer"
              @error="markBroken(row.item)"
            />
            <span v-else class="hash-icon-fallback">{{ fallbackLetter(row.item) }}</span>
          </span>
          <span class="hash-title">{{ row.item.title || row.item.id }}</span>
          <span v-if="row.item.vendor" class="hash-vendor">{{ row.item.vendor }}</span>
          <span class="hash-kind-pill" :data-kind="row.item.kind">
            {{ kindLabel(row.item.kind) }}
          </span>
        </li>
      </template>
    </template>
  </ul>
</template>

<script setup lang="ts">
import { computed, reactive, watch } from "vue";
import type { HashMentionItem, HashResourceKind } from "@/services/chat/hashMentions";

const props = defineProps<{
  loading: boolean;
  items: HashMentionItem[];
  selectedIndex: number;
  loadingText: string;
  emptyText: string;
  ariaLabel: string;
  skillLabel: string;
  mcpLabel: string;
}>();

defineEmits<{
  hover: [index: number];
  select: [item: HashMentionItem];
}>();

const brokenIcons = reactive<Record<string, boolean>>({});

const showSectionHeaders = computed(() => {
  const kinds = new Set(props.items.map((item) => item.kind));
  return kinds.has("skill") && kinds.has("mcp");
});

const sections = computed(() => {
  const skillRows: Array<{ item: HashMentionItem; index: number }> = [];
  const mcpRows: Array<{ item: HashMentionItem; index: number }> = [];
  props.items.forEach((item, index) => {
    const row = { item, index };
    if (item.kind === "mcp") mcpRows.push(row);
    else skillRows.push(row);
  });
  const out: Array<{ kind: HashResourceKind; rows: typeof skillRows }> = [];
  if (skillRows.length) out.push({ kind: "skill", rows: skillRows });
  if (mcpRows.length) out.push({ kind: "mcp", rows: mcpRows });
  return out;
});

watch(
  () => props.items.map((item) => `${item.kind}:${item.id}:${item.iconUrl ?? ""}`).join("|"),
  () => {
    for (const key of Object.keys(brokenIcons)) {
      delete brokenIcons[key];
    }
  },
);

function itemKey(item: HashMentionItem) {
  return `${item.kind}:${item.id}`;
}

function markBroken(item: HashMentionItem) {
  brokenIcons[itemKey(item)] = true;
}

function kindLabel(kind: HashResourceKind): string {
  return kind === "skill" ? props.skillLabel : props.mcpLabel;
}

function fallbackLetter(item: HashMentionItem): string {
  const source = item.title || item.id || "?";
  return source.trim().charAt(0).toUpperCase() || "?";
}

function rowTitle(item: HashMentionItem): string {
  const kind = kindLabel(item.kind);
  const vendor = item.vendor?.trim();
  return vendor ? `${kind} · ${vendor}` : `${kind} · ${item.id}`;
}
</script>

<style scoped>
.command-list {
  --command-row-height: 30px;
  --command-list-padding: 6px;
  --picker-meta-row-height: 28px;
  --command-list-visible-rows: 10;
  list-style: none;
  margin: 0;
  padding: 3px 0;
  border-bottom: 1px solid var(--peek-border);
  background: var(--peek-list-bg);
  flex: none;
  max-height: min(
    calc(
      var(--command-row-height) * var(--command-list-visible-rows) + var(--command-list-padding)
    ),
    72vh
  );
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
}

.command-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 10px;
  height: var(--command-row-height);
  cursor: default;
  box-sizing: border-box;
}

.command-item.active {
  background: color-mix(in srgb, var(--peek-accent) 14%, var(--peek-list-bg));
  color: var(--peek-accent);
}

.hash-section-head {
  padding: 8px 10px 4px;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--peek-muted);
}

.hash-icon {
  flex: none;
  width: 16px;
  height: 16px;
  border-radius: 0;
  overflow: visible;
  display: grid;
  place-items: center;
  background: transparent;
}

.hash-icon img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  display: block;
  background: transparent;
}

.hash-icon-fallback {
  font-size: 9px;
  font-weight: 700;
  color: var(--peek-muted);
  line-height: 1;
}

.hash-title {
  flex: none;
  max-width: 42%;
  font-size: 12px;
  font-weight: 600;
  color: var(--peek-fg, inherit);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.hash-vendor {
  flex: 1;
  min-width: 0;
  font-family: var(--font-mono, ui-monospace, Consolas, monospace);
  font-size: 11px;
  font-weight: 500;
  color: var(--peek-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.command-item.active .hash-vendor {
  color: color-mix(in srgb, var(--peek-accent) 50%, var(--peek-muted));
}

.hash-kind-pill {
  flex: none;
  margin-left: auto;
  padding: 1px 5px;
  border-radius: 4px;
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.03em;
  text-transform: uppercase;
  line-height: 1.35;
}

.hash-kind-pill[data-kind="skill"] {
  background: color-mix(in srgb, var(--peek-accent) 16%, transparent);
  color: var(--peek-accent);
}

.hash-kind-pill[data-kind="mcp"] {
  background: color-mix(in srgb, #3b82f6 18%, transparent);
  color: #60a5fa;
}

.picker-meta {
  padding: 6px 10px;
  font-size: 12px;
  color: var(--peek-muted);
}
</style>
