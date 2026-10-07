<template>
  <Teleport to="body">
    <Transition name="search-palette">
      <div
        v-if="open"
        class="search-palette-root"
        data-tauri-drag-region="false"
        @mousedown.self="close"
      >
        <div
          class="search-palette"
          role="dialog"
          aria-modal="true"
          :aria-label="copy.title"
          @keydown="onKeydown"
        >
          <div class="search-input-row">
            <input
              ref="inputRef"
              v-model="query"
              class="search-input"
              type="text"
              :placeholder="copy.searchChatsPlaceholder"
              spellcheck="false"
              autocomplete="off"
            />
          </div>

          <div class="search-body peek-scrollbar">
            <section v-if="filteredSessions.length" class="search-section">
              <header class="search-section-label">{{ copy.chats }}</header>
              <ul class="search-list" role="listbox">
                <li
                  v-for="(item, index) in filteredSessions"
                  :key="item.sessionId"
                  class="search-item"
                  :class="{ active: selectedIndex === index }"
                  role="option"
                  :aria-selected="selectedIndex === index"
                  @mouseenter="selectedIndex = index"
                  @mousedown.prevent="selectSession(item.sessionId)"
                >
                  <span class="search-item-title">
                    {{ formatSessionPreview(item.preview || "") || copy.untitled }}
                  </span>
                  <span class="search-item-meta">{{ workspaceLabel(item.workspaceId) }}</span>
                  <kbd v-if="index < 9" class="search-shortcut">
                    {{ shortcutPrefix }}{{ index + 1 }}
                  </kbd>
                </li>
              </ul>
            </section>

            <section class="search-section">
              <header class="search-section-label">{{ copy.recommended }}</header>
              <ul class="search-list" role="listbox">
                <li
                  class="search-item action-item"
                  :class="{ active: selectedIndex === chatActionOffset }"
                  role="option"
                  :aria-selected="selectedIndex === chatActionOffset"
                  @mouseenter="selectedIndex = chatActionOffset"
                  @mousedown.prevent="createNewChat"
                >
                  <SquarePen :size="15" aria-hidden="true" />
                  <span class="search-item-title">{{ copy.newChat }}</span>
                  <kbd class="search-shortcut">{{ shortcutPrefix }}N</kbd>
                </li>
              </ul>
            </section>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { SquarePen } from "@lucide/vue";
import type { Workspace } from "@/commands/workspace";
import { formatSessionPreview } from "@/services/chat/sessionPreview";
import type { ChatSessionSummary } from "@/types/chat";
import type { AppLanguage } from "@/types/setting";
import { tr } from "@/services/i18n";

const props = defineProps<{
  open: boolean;
  sessions: ChatSessionSummary[];
  workspaces: Workspace[];
  language: AppLanguage;
}>();

const emit = defineEmits<{
  "update:open": [value: boolean];
  "select-session": [sessionId: string];
  "new-chat": [];
}>();

const inputRef = ref<HTMLInputElement | null>(null);
const query = ref("");
const selectedIndex = ref(0);

const shortcutPrefix = computed(() =>
  navigator.platform.toLowerCase().includes("mac") ? "⌘" : "Ctrl+",
);
const copy = computed(() => ({
  title: tr(props.language, "shell.search.title"),
  searchChatsPlaceholder: tr(props.language, "shell.search.chatsPlaceholder"),
  chats: tr(props.language, "shell.search.chats"),
  recommended: tr(props.language, "shell.search.recommended"),
  untitled: tr(props.language, "shell.search.untitled"),
  newChat: tr(props.language, "shell.search.newChat"),
  quickAsk: tr(props.language, "shell.search.quickAsk"),
}));

const filteredSessions = computed(() => {
  const needle = query.value.trim().toLowerCase();
  const sorted = [...props.sessions].sort((a, b) => b.updatedAt - a.updatedAt);
  if (!needle) return sorted.slice(0, 12);
  return sorted
    .filter((session) => {
      const preview = (session.preview || "").toLowerCase();
      const workspace = workspaceLabel(session.workspaceId).toLowerCase();
      return preview.includes(needle) || workspace.includes(needle);
    })
    .slice(0, 20);
});

const chatActionOffset = computed(() => filteredSessions.value.length);
const itemCount = computed(() => filteredSessions.value.length + 1);

watch(
  () => props.open,
  async (open) => {
    if (!open) return;
    query.value = "";
    selectedIndex.value = 0;
    await nextTick();
    inputRef.value?.focus();
  },
);

watch([query, itemCount], () => {
  if (selectedIndex.value >= itemCount.value) {
    selectedIndex.value = Math.max(0, itemCount.value - 1);
  }
});

/** Resolve a workspace id to its display name, falling back to Quick Ask. */
function workspaceLabel(workspaceId?: string) {
  if (!workspaceId) return copy.value.quickAsk;
  return (
    props.workspaces.find((workspace) => workspace.id === workspaceId)?.name || copy.value.quickAsk
  );
}

function close() {
  emit("update:open", false);
}

function selectSession(sessionId: string) {
  emit("select-session", sessionId);
  close();
}

function createNewChat() {
  emit("new-chat");
  close();
}

/** Activate the currently highlighted list row (session or new-chat action). */
function activateSelected() {
  if (selectedIndex.value < filteredSessions.value.length) {
    const session = filteredSessions.value[selectedIndex.value];
    if (session) selectSession(session.sessionId);
    return;
  }
  createNewChat();
}

/** Keyboard navigation and shortcut activation for the palette dialog. */
function onKeydown(event: KeyboardEvent) {
  const mod = event.ctrlKey || event.metaKey;
  if (event.key === "Escape") {
    event.preventDefault();
    close();
    return;
  }
  if (event.key === "ArrowDown") {
    event.preventDefault();
    if (!itemCount.value) return;
    selectedIndex.value = (selectedIndex.value + 1) % itemCount.value;
    return;
  }
  if (event.key === "ArrowUp") {
    event.preventDefault();
    if (!itemCount.value) return;
    selectedIndex.value = (selectedIndex.value - 1 + itemCount.value) % itemCount.value;
    return;
  }
  if (event.key === "Enter") {
    event.preventDefault();
    activateSelected();
    return;
  }
  if (!mod) return;
  const key = event.key.toLowerCase();
  if (key >= "1" && key <= "9") {
    const index = Number(key) - 1;
    const session = filteredSessions.value[index];
    if (session) {
      event.preventDefault();
      selectSession(session.sessionId);
    }
    return;
  }
  if (key === "n") {
    event.preventDefault();
    createNewChat();
  }
}
</script>

<style scoped>
.search-palette-root {
  position: fixed;
  inset: 0;
  z-index: 80;
  display: grid;
  place-items: start center;
  padding: 12vh 16px 24px;
  background: color-mix(in srgb, #000 42%, transparent);
  backdrop-filter: blur(2px);
}
.search-palette {
  box-sizing: border-box;
  width: min(520px, 100%);
  max-height: min(480px, 70vh);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid var(--peek-border);
  border-radius: 12px;
  background: var(--peek-surface);
  box-shadow: 0 18px 48px var(--peek-shadow);
  color: var(--peek-text);
  font-family: var(--peek-font-sans, inherit);
}
.search-input-row {
  flex: none;
  padding: 14px 16px 12px;
  border-bottom: 1px solid var(--peek-border);
  background: var(--peek-bg);
}
.search-input {
  width: 100%;
  border: 0;
  outline: none;
  background: transparent;
  color: var(--peek-text);
  caret-color: var(--peek-text);
  font: 500 15px/1.45 var(--peek-font-sans, inherit);
}
.search-input::placeholder {
  color: var(--peek-placeholder);
  font-weight: 450;
}
.search-body {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 6px 0 10px;
  background: var(--peek-surface);
}
.search-section {
  margin: 2px 0 8px;
}
.search-section-label {
  padding: 6px 16px 5px;
  color: var(--peek-faint);
  font-size: 11px;
  font-weight: 600;
}
.search-list {
  list-style: none;
  margin: 0;
  padding: 0;
}
.search-item {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 34px;
  padding: 0 16px;
  cursor: default;
}
.search-item.active {
  background: var(--peek-list-active);
}
.search-item > svg {
  flex: none;
  color: var(--peek-muted);
}
.search-item-title {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  font-size: 13px;
  font-weight: 500;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.search-item-meta {
  flex: none;
  max-width: 28%;
  overflow: hidden;
  color: var(--peek-faint);
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.search-shortcut {
  flex: none;
  min-width: 40px;
  color: var(--peek-faint);
  font: 500 11px/1 var(--font-mono, ui-monospace, monospace);
  text-align: right;
}
.search-palette-enter-active,
.search-palette-leave-active {
  transition: opacity 130ms ease;
}
.search-palette-enter-active .search-palette,
.search-palette-leave-active .search-palette {
  transition:
    transform 150ms ease,
    opacity 130ms ease;
}
.search-palette-enter-from,
.search-palette-leave-to {
  opacity: 0;
}
.search-palette-enter-from .search-palette,
.search-palette-leave-to .search-palette {
  opacity: 0;
  transform: translateY(-8px);
}
@media (prefers-reduced-motion: reduce) {
  .search-palette-enter-active,
  .search-palette-leave-active,
  .search-palette-enter-active .search-palette,
  .search-palette-leave-active .search-palette {
    transition: none;
  }
}
</style>
