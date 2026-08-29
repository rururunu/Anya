<template>
  <AppConfirmDialog ref="confirmDialogRef" />
  <section class="settings-page is-wide archive-page">
    <SettingsPageHeader :title="copy.title" :description="copy.description">
      <template #actions>
        <Button
          variant="ghost"
          size="sm"
          class="h-7 text-xs text-muted-foreground hover:text-foreground flex items-center gap-1.5"
          :disabled="visibleCount === 0"
          @click="toggleSelectAll"
        >
          <input
            type="checkbox"
            :checked="isAllSelected"
            :indeterminate="isPartiallySelected"
            class="settings-checkbox settings-checkbox-sm pointer-events-none"
          />
          <span>{{ copy.selectAll }}</span>
        </Button>
        <Button
          v-if="selectedCount > 0"
          variant="ghost"
          size="sm"
          class="h-7 text-xs flex items-center gap-1.5"
          @click="restoreSelected"
        >
          <ArchiveRestore class="size-3" />
          <span>{{ copy.restoreSelected.replace("{count}", String(selectedCount)) }}</span>
        </Button>
        <Button
          v-if="selectedCount > 0"
          variant="destructive"
          size="sm"
          class="h-7 text-xs flex items-center gap-1.5"
          @click="deleteSelected"
        >
          <Trash2 class="size-3" />
          <span>{{ copy.deleteSelected.replace("{count}", String(selectedCount)) }}</span>
        </Button>
      </template>
    </SettingsPageHeader>
    <SettingsSearchField
      v-model="query"
      class="archive-search"
      :placeholder="copy.search"
      :submit-label="copy.searchSubmit"
      :clear-label="copy.searchClear"
    />
    <SettingsFormError :message="pageError" />

    <h3 class="settings-section-label">{{ copy.conversations }}</h3>
    <div class="settings-card">
      <p v-if="sessions.length === 0" class="settings-empty">{{ copy.emptyConversations }}</p>
      <p v-else-if="filteredSessions.length === 0" class="settings-empty">
        {{ copy.noMatchingConversations }}
      </p>
      <article
        v-for="session in filteredSessions"
        :key="session.sessionId"
        class="settings-list-row is-selectable archive-item-row"
        :class="{ 'is-selected': selectedSessionIds.includes(session.sessionId) }"
        @click="toggleSession(session.sessionId)"
      >
        <input
          v-model="selectedSessionIds"
          type="checkbox"
          :value="session.sessionId"
          class="settings-checkbox"
          @click.stop
        />
        <div class="archive-copy">
          <strong>{{ session.preview }}</strong>
          <small>{{ conversationMeta(session) }}</small>
        </div>
        <div class="archive-actions" @click.stop>
          <Button
            variant="ghost"
            size="sm"
            class="h-7 px-2 text-xs"
            @click="restoreSession(session.sessionId)"
          >
            {{ copy.restore }}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            class="size-7 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
            :title="copy.delete"
            :aria-label="copy.delete"
            @click="deleteSession(session.sessionId)"
          >
            <Trash2 class="size-3.5" />
          </Button>
        </div>
      </article>
    </div>

    <h3 class="settings-section-label">{{ copy.workspaces }}</h3>
    <div class="settings-card">
      <p v-if="workspaces.length === 0" class="settings-empty">{{ copy.emptyWorkspaces }}</p>
      <p v-else-if="filteredWorkspaces.length === 0" class="settings-empty">
        {{ copy.noMatchingWorkspaces }}
      </p>
      <article
        v-for="workspace in filteredWorkspaces"
        :key="workspace.id"
        class="settings-list-row is-selectable archive-item-row"
        :class="{ 'is-selected': selectedWorkspaceIds.includes(workspace.id) }"
        @click="toggleWorkspace(workspace.id)"
      >
        <input
          v-model="selectedWorkspaceIds"
          type="checkbox"
          :value="workspace.id"
          class="settings-checkbox"
          @click.stop
        />
        <div class="archive-copy">
          <strong>{{ workspace.name }}</strong>
          <small>{{ workspace.root }}</small>
        </div>
        <div class="archive-actions" @click.stop>
          <Button
            variant="ghost"
            size="sm"
            class="h-7 px-2 text-xs"
            @click="restoreWorkspace(workspace.id)"
          >
            {{ copy.restore }}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            class="size-7 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
            :title="copy.delete"
            :aria-label="copy.delete"
            @click="deleteArchivedWorkspace(workspace)"
          >
            <Trash2 class="size-3.5" />
          </Button>
        </div>
      </article>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { emit as tauriEmit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { ArchiveRestore, Trash2 } from "@lucide/vue";
import { Button } from "@/components/ui/button";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import SettingsPageHeader from "@/components/settings/SettingsPageHeader.vue";
import SettingsFormError from "@/components/settings/SettingsFormError.vue";
import SettingsSearchField from "@/components/settings/SettingsSearchField.vue";
import {
  deleteWorkspace,
  listArchivedWorkspaces,
  setWorkspaceArchived,
  type Workspace,
} from "@/commands/workspace";
import {
  deleteChatSession,
  listArchivedChatSessions,
  setChatSessionArchived,
} from "@/services/ipc";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";
import type { ChatSessionSummary } from "@/types/chat";

const settingStore = useSettingStore();
const sessions = ref<ChatSessionSummary[]>([]);
const workspaces = ref<Workspace[]>([]);
const selectedSessionIds = ref<string[]>([]);
const selectedWorkspaceIds = ref<string[]>([]);
const query = ref("");
const pageError = ref("");
const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);
let unlisten: UnlistenFn | null = null;

const copy = computed(() => {
  const language = settingStore.language;
  return {
    title: tr(language, "settings.archive.title"),
    description: tr(language, "settings.archive.description"),
    conversations: tr(language, "settings.archive.conversations"),
    workspaces: tr(language, "settings.archive.workspaces"),
    emptyConversations: tr(language, "settings.archive.emptyConversations"),
    emptyWorkspaces: tr(language, "settings.archive.emptyWorkspaces"),
    noMatchingConversations: tr(language, "settings.archive.noMatchingConversations"),
    noMatchingWorkspaces: tr(language, "settings.archive.noMatchingWorkspaces"),
    search: tr(language, "settings.archive.search"),
    searchSubmit: tr(language, "settings.archive.searchSubmit"),
    searchClear: tr(language, "settings.archive.searchClear"),
    selectAll: tr(language, "settings.archive.selectAll"),
    restoreSelected: tr(language, "settings.archive.restoreSelected"),
    deleteSelected: tr(language, "settings.archive.deleteSelected"),
    restore: tr(language, "settings.archive.restore"),
    delete: tr(language, "settings.archive.delete"),
    deleteConversation: tr(language, "settings.archive.deleteConversation"),
    deleteWorkspace: tr(language, "settings.archive.deleteWorkspace"),
    deleteSelectedConversations: tr(language, "settings.archive.deleteSelectedConversations"),
    deleteSelectedWorkspaces: tr(language, "settings.archive.deleteSelectedWorkspaces"),
    deleteSelectedMixed: tr(language, "settings.archive.deleteSelectedMixed"),
    cancel: tr(language, "settings.history.cancel"),
  };
});

function normalizeQuery(value: string) {
  return value.trim().toLowerCase();
}

function matchesQuery(haystack: string, needle: string) {
  return haystack.toLowerCase().includes(needle);
}

const filteredSessions = computed(() => {
  const needle = normalizeQuery(query.value);
  if (!needle) return sessions.value;
  return sessions.value.filter(
    (session) =>
      matchesQuery(session.preview, needle) || matchesQuery(conversationMeta(session), needle),
  );
});

const filteredWorkspaces = computed(() => {
  const needle = normalizeQuery(query.value);
  if (!needle) return workspaces.value;
  return workspaces.value.filter(
    (workspace) => matchesQuery(workspace.name, needle) || matchesQuery(workspace.root, needle),
  );
});

const visibleCount = computed(
  () => filteredSessions.value.length + filteredWorkspaces.value.length,
);

const selectedCount = computed(
  () => selectedSessionIds.value.length + selectedWorkspaceIds.value.length,
);

const isAllSelected = computed(() => {
  return (
    visibleCount.value > 0 &&
    filteredSessions.value.every((session) =>
      selectedSessionIds.value.includes(session.sessionId),
    ) &&
    filteredWorkspaces.value.every((workspace) => selectedWorkspaceIds.value.includes(workspace.id))
  );
});

const isPartiallySelected = computed(() => selectedCount.value > 0 && !isAllSelected.value);

function conversationMeta(session: ChatSessionSummary) {
  const when = new Intl.DateTimeFormat(settingStore.language, {
    year: "numeric",
    month: "short",
    day: "numeric",
  }).format(new Date(session.updatedAt));
  return `${when} · ${tr(settingStore.language, "settings.history.messages", {
    count: session.messageCount,
  })}`;
}

function toggleId(list: string[], id: string) {
  return list.includes(id) ? list.filter((item) => item !== id) : [...list, id];
}

function toggleSession(sessionId: string) {
  selectedSessionIds.value = toggleId(selectedSessionIds.value, sessionId);
}

function toggleWorkspace(workspaceId: string) {
  selectedWorkspaceIds.value = toggleId(selectedWorkspaceIds.value, workspaceId);
}

function toggleSelectAll() {
  if (isAllSelected.value) {
    const hiddenSessionIds = new Set(filteredSessions.value.map((session) => session.sessionId));
    const hiddenWorkspaceIds = new Set(filteredWorkspaces.value.map((workspace) => workspace.id));
    selectedSessionIds.value = selectedSessionIds.value.filter((id) => !hiddenSessionIds.has(id));
    selectedWorkspaceIds.value = selectedWorkspaceIds.value.filter(
      (id) => !hiddenWorkspaceIds.has(id),
    );
    return;
  }
  selectedSessionIds.value = [
    ...new Set([
      ...selectedSessionIds.value,
      ...filteredSessions.value.map((session) => session.sessionId),
    ]),
  ];
  selectedWorkspaceIds.value = [
    ...new Set([
      ...selectedWorkspaceIds.value,
      ...filteredWorkspaces.value.map((workspace) => workspace.id),
    ]),
  ];
}

function pruneSelection() {
  const sessionIds = new Set(sessions.value.map((session) => session.sessionId));
  const workspaceIds = new Set(workspaces.value.map((workspace) => workspace.id));
  selectedSessionIds.value = selectedSessionIds.value.filter((id) => sessionIds.has(id));
  selectedWorkspaceIds.value = selectedWorkspaceIds.value.filter((id) => workspaceIds.has(id));
}

async function load() {
  pageError.value = "";
  try {
    const [sessionResponse, archivedWorkspaces] = await Promise.all([
      listArchivedChatSessions(),
      listArchivedWorkspaces(),
    ]);
    sessions.value = sessionResponse.sessions ?? [];
    workspaces.value = archivedWorkspaces;
    pruneSelection();
  } catch (error) {
    pageError.value = String(error);
  }
}

async function restoreSession(sessionId: string) {
  await setChatSessionArchived(sessionId, false);
  await load();
  await tauriEmit("history-updated");
}

async function deleteSession(sessionId: string) {
  const confirmed = await confirmDialogRef.value?.ask({
    title: copy.value.delete,
    description: copy.value.deleteConversation,
    confirmLabel: copy.value.delete,
    cancelLabel: copy.value.cancel,
  });
  if (!confirmed) return;
  await deleteChatSession(sessionId);
  await load();
  await tauriEmit("history-updated");
}

async function restoreWorkspace(id: string) {
  await setWorkspaceArchived(id, false);
  await load();
  await tauriEmit("history-updated");
}

async function restoreSelected() {
  if (selectedCount.value === 0) return;
  try {
    await Promise.all([
      ...selectedSessionIds.value.map((id) => setChatSessionArchived(id, false)),
      ...selectedWorkspaceIds.value.map((id) => setWorkspaceArchived(id, false)),
    ]);
    selectedSessionIds.value = [];
    selectedWorkspaceIds.value = [];
    await load();
    await tauriEmit("history-updated");
  } catch (error) {
    pageError.value = String(error);
  }
}

async function deleteArchivedWorkspace(workspace: Workspace) {
  const confirmed = await confirmDialogRef.value?.ask({
    title: copy.value.delete,
    description: copy.value.deleteWorkspace.replace("{name}", workspace.name),
    confirmLabel: copy.value.delete,
    cancelLabel: copy.value.cancel,
  });
  if (!confirmed) return;
  await deleteWorkspace(workspace.id);
  await load();
  await tauriEmit("history-updated");
}

function selectedDeleteDescription() {
  const sessionCount = selectedSessionIds.value.length;
  const workspaceCount = selectedWorkspaceIds.value.length;
  if (sessionCount > 0 && workspaceCount > 0) {
    return copy.value.deleteSelectedMixed
      .replace("{sessionCount}", String(sessionCount))
      .replace("{workspaceCount}", String(workspaceCount));
  }
  if (workspaceCount > 0) {
    return copy.value.deleteSelectedWorkspaces.replace("{count}", String(workspaceCount));
  }
  return copy.value.deleteSelectedConversations.replace("{count}", String(sessionCount));
}

async function deleteSelected() {
  if (selectedCount.value === 0) return;
  const confirmed = await confirmDialogRef.value?.ask({
    title: copy.value.delete,
    description: selectedDeleteDescription(),
    confirmLabel: copy.value.delete,
    cancelLabel: copy.value.cancel,
  });
  if (!confirmed) return;
  try {
    await Promise.all([
      ...selectedSessionIds.value.map((id) => deleteChatSession(id)),
      ...selectedWorkspaceIds.value.map((id) => deleteWorkspace(id)),
    ]);
    selectedSessionIds.value = [];
    selectedWorkspaceIds.value = [];
    await load();
    await tauriEmit("history-updated");
  } catch (error) {
    pageError.value = String(error);
  }
}

onMounted(async () => {
  await load();
  unlisten = await listen("workspaces-changed", () => void load());
});

onUnmounted(() => unlisten?.());
</script>

<style scoped>
.archive-search {
  margin: 0 0 16px;
}
.settings-section-label {
  margin-bottom: 8px;
}
.settings-card {
  margin-bottom: 20px;
}
.archive-item-row {
  min-height: 54px;
  padding: 8px 10px 8px 12px;
}
.archive-copy {
  min-width: 0;
  display: grid;
  flex: 1;
  gap: 2px;
}
.archive-copy strong {
  overflow: hidden;
  color: var(--peek-text);
  font-size: 12px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.archive-copy small {
  overflow: hidden;
  color: var(--peek-muted);
  font-size: 10px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.archive-actions {
  display: flex;
  flex: none;
  align-items: center;
  gap: 2px;
}
</style>
