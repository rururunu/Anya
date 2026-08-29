<template>
  <AppConfirmDialog ref="confirmDialogRef" />
  <div class="settings-page is-wide history-page select-none">
    <SettingsPageHeader :title="historyText.title">
      <template #actions>
        <Button
          variant="ghost"
          size="sm"
          class="h-7 text-xs text-muted-foreground hover:text-foreground flex items-center gap-1.5"
          @click="toggleSelectAll"
        >
          <input
            type="checkbox"
            :checked="isAllSelected"
            class="settings-checkbox settings-checkbox-sm pointer-events-none"
          />
          <span>{{ historyText.selectAll }}</span>
        </Button>

        <Button
          v-if="selectedSessionIds.length > 0"
          variant="destructive"
          size="sm"
          class="h-7 text-xs flex items-center gap-1.5"
          @click="deleteSelectedSessions"
        >
          <Trash2 class="size-3" />
          <span>
            {{ historyText.deleteSelected.replace("{count}", String(selectedSessionIds.length)) }}
          </span>
        </Button>

        <Button
          variant="ghost"
          size="sm"
          class="h-7 text-xs text-destructive hover:bg-destructive/10 hover:text-destructive flex items-center gap-1.5"
          @click="clearAllSessions"
        >
          <AlertTriangle class="size-3" />
          <span>{{ historyText.clearAll }}</span>
        </Button>
      </template>
    </SettingsPageHeader>

    <SettingsFormError :message="pageError" />

    <div class="history-groups">
      <p v-if="historyGroups.length === 0" class="settings-empty">{{ historyText.empty }}</p>

      <section v-for="group in historyGroups" :key="group.id" class="history-group">
        <div class="group-header flex items-center gap-1">
          <button
            type="button"
            class="flex min-w-0 flex-1 items-center gap-2 py-1 text-left"
            @click="toggleHistoryGroup(group.id)"
          >
            <ChevronDown
              v-if="isHistoryGroupExpanded(group.id)"
              class="size-3.5 text-muted-foreground"
            />
            <ChevronRight v-else class="size-3.5 text-muted-foreground" />
            <Globe2 v-if="group.public" class="size-4 text-muted-foreground" />
            <Folder v-else class="size-4 text-primary" />
            <span class="min-w-0 flex-1">
              <strong class="block truncate text-xs font-semibold">{{ group.name }}</strong>
              <small v-if="group.root" class="block truncate text-[10px] text-muted-foreground">
                {{ group.root }}
              </small>
            </span>
          </button>
          <span class="text-[11px] tabular-nums text-muted-foreground">
            {{ group.sessions.length }}
          </span>
          <Button
            variant="ghost"
            size="sm"
            class="h-7 gap-1.5 px-2 text-xs text-muted-foreground hover:text-foreground"
            :disabled="group.sessions.length === 0"
            @click="toggleHistoryGroupSelection(group)"
          >
            <input
              type="checkbox"
              :checked="isHistoryGroupSelected(group)"
              :indeterminate="isHistoryGroupPartiallySelected(group)"
              class="settings-checkbox settings-checkbox-sm pointer-events-none"
            />
            {{ historyText.selectAll }}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            class="size-7 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
            :disabled="group.sessions.length === 0"
            :title="historyText.deleteGroup"
            :aria-label="historyText.deleteGroup"
            @click="deleteHistoryGroup(group)"
          >
            <Trash2 class="size-3.5" />
          </Button>
        </div>

        <article
          v-for="session in isHistoryGroupExpanded(group.id) ? group.sessions : []"
          :key="session.sessionId"
          class="settings-list-row history-session-row grid grid-cols-[minmax(0,1fr)_120px] items-center gap-4"
        >
          <div class="flex min-w-0 items-start gap-3">
            <input
              v-model="selectedSessionIds"
              type="checkbox"
              :value="session.sessionId"
              class="settings-checkbox settings-checkbox-md mt-1"
            />
            <div class="min-w-0 flex-1 space-y-1">
              <p class="text-[11px] text-muted-foreground">{{ formatTime(session.updatedAt) }}</p>
              <h3
                class="truncate text-sm font-medium cursor-pointer hover:text-primary"
                @click="openSession(session)"
              >
                {{ session.preview }}
              </h3>
              <p class="text-xs text-muted-foreground">
                {{ historyText.messages.replace("{count}", String(session.messageCount)) }}
                <span v-if="session.estimatedTokens">
                  · ≈{{ formatTokenCount(session.estimatedTokens, settingStore.language) }} tokens
                </span>
              </p>
            </div>
          </div>
          <div class="flex justify-end gap-2">
            <Button
              variant="ghost"
              size="icon"
              class="size-8 text-muted-foreground hover:text-foreground"
              :title="historyText.open"
              :aria-label="historyText.open"
              @click="openSession(session)"
            >
              <FolderOpen class="size-3.5" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              class="size-8 text-muted-foreground hover:text-foreground"
              :title="historyText.archiveLabel"
              :aria-label="historyText.archiveLabel"
              @click="archiveSingleSession(session.sessionId)"
            >
              <Archive class="size-3.5" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              class="size-8 text-muted-foreground hover:text-destructive"
              :title="historyText.deleteLabel"
              :aria-label="historyText.deleteLabel"
              @click="deleteSingleSession(session.sessionId)"
            >
              <Trash2 class="size-3.5" />
            </Button>
          </div>
        </article>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { emit as tauriEmit } from "@tauri-apps/api/event";
import {
  Archive,
  AlertTriangle,
  ChevronDown,
  ChevronRight,
  Folder,
  FolderOpen,
  Globe2,
  Trash2,
} from "@lucide/vue";
import { Button } from "@/components/ui/button";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import SettingsPageHeader from "@/components/settings/SettingsPageHeader.vue";
import SettingsFormError from "@/components/settings/SettingsFormError.vue";
import { listWorkspaces, switchWorkspace, type Workspace } from "@/commands/workspace";
import {
  listChatSessions,
  deleteChatSession,
  setChatSessionArchived,
  clearAllChatSessions,
  openSessionInOverlay,
} from "@/services/ipc";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";
import type { SettingsI18nKey } from "@/services/locales/settings";
import type { ChatSessionSummary } from "@/types/chat";
import { formatTokenCount } from "@/services/chat/tokenEstimate";

const props = defineProps<{
  expandedHistoryGroups: Record<string, boolean>;
}>();

const emit = defineEmits<{
  "toggle-history-group": [groupId: string];
}>();

const settingStore = useSettingStore();

const historyText = computed(() => {
  const language = settingStore.language;
  return {
    title: tr(language, "settings.history.title"),
    selectAll: tr(language, "settings.history.selectAll"),
    deleteSelected: tr(language, "settings.history.deleteSelected"),
    clearAll: tr(language, "settings.history.clearAll"),
    empty: tr(language, "settings.history.empty"),
    deleteGroup: tr(language, "settings.history.deleteGroup"),
    messages: tr(language, "settings.history.messages"),
    open: tr(language, "settings.history.open"),
    publicGroup: tr(language, "settings.history.publicGroup"),
    yesterday: tr(language, "settings.history.yesterday"),
    cancel: tr(language, "settings.history.cancel"),
    archiveLabel: tr(language, "settings.history.archiveLabel"),
    deleteLabel: tr(language, "settings.history.deleteLabel"),
    openError: tr(language, "history.openError"),
  };
});

function historyConfirm(
  key: Extract<SettingsI18nKey, `settings.historyConfirm.${string}`>,
  values: Record<string, string | number> = {},
) {
  return tr(settingStore.language, key, values);
}

const PUBLIC_HISTORY_GROUP = "__public__";
const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);
const historySessions = ref<ChatSessionSummary[]>([]);
const historyWorkspaces = ref<Workspace[]>([]);
const selectedSessionIds = ref<string[]>([]);
const pageError = ref("");

interface HistoryGroup {
  id: string;
  name: string;
  root?: string;
  public: boolean;
  sessions: ChatSessionSummary[];
}

function workspaceNameFromId(id: string) {
  const parts = id.replace(/\\/g, "/").split("/").filter(Boolean);
  return parts[parts.length - 1] ?? id;
}

const historyGroups = computed<HistoryGroup[]>(() => {
  const groups = new Map<string, HistoryGroup>();
  for (const workspace of historyWorkspaces.value) {
    groups.set(workspace.id, {
      id: workspace.id,
      name: workspace.name,
      root: workspace.root,
      public: false,
      sessions: [],
    });
  }
  groups.set(PUBLIC_HISTORY_GROUP, {
    id: PUBLIC_HISTORY_GROUP,
    name: historyText.value.publicGroup,
    public: true,
    sessions: [],
  });

  for (const session of historySessions.value) {
    const groupId = session.workspaceId ?? PUBLIC_HISTORY_GROUP;
    if (!groups.has(groupId)) {
      groups.set(groupId, {
        id: groupId,
        name: workspaceNameFromId(groupId),
        root: groupId,
        public: false,
        sessions: [],
      });
    }
    groups.get(groupId)?.sessions.push(session);
  }

  return [...groups.values()]
    .filter((group) => group.sessions.length > 0)
    .sort((left, right) => Number(left.public) - Number(right.public));
});

const filteredHistorySessions = computed(() =>
  historyGroups.value.flatMap((group) => group.sessions),
);

function isHistoryGroupExpanded(groupId: string) {
  return props.expandedHistoryGroups[groupId] !== false;
}

function toggleHistoryGroup(groupId: string) {
  emit("toggle-history-group", groupId);
}

function historyGroupSessionIds(group: HistoryGroup) {
  return historySessions.value
    .filter((session) => (session.workspaceId ?? PUBLIC_HISTORY_GROUP) === group.id)
    .map((session) => session.sessionId);
}

function isHistoryGroupSelected(group: HistoryGroup) {
  const groupIds = historyGroupSessionIds(group);
  return groupIds.length > 0 && groupIds.every((id) => selectedSessionIds.value.includes(id));
}

function isHistoryGroupPartiallySelected(group: HistoryGroup) {
  const selectedCount = historyGroupSessionIds(group).filter((id) =>
    selectedSessionIds.value.includes(id),
  ).length;
  return selectedCount > 0 && selectedCount < historyGroupSessionIds(group).length;
}

function toggleHistoryGroupSelection(group: HistoryGroup) {
  const groupIds = historyGroupSessionIds(group);
  if (isHistoryGroupSelected(group)) {
    const ids = new Set(groupIds);
    selectedSessionIds.value = selectedSessionIds.value.filter((id) => !ids.has(id));
    return;
  }
  selectedSessionIds.value = [...new Set([...selectedSessionIds.value, ...groupIds])];
}

const isAllSelected = computed(() => {
  return (
    filteredHistorySessions.value.length > 0 &&
    filteredHistorySessions.value.every((s) => selectedSessionIds.value.includes(s.sessionId))
  );
});

function toggleSelectAll() {
  if (isAllSelected.value) {
    selectedSessionIds.value = [];
  } else {
    selectedSessionIds.value = filteredHistorySessions.value.map((s) => s.sessionId);
  }
}

async function loadSessions() {
  const [sessionsList, workspaces] = await Promise.all([listChatSessions(), listWorkspaces()]);
  historySessions.value = sessionsList.sessions ?? [];
  historyWorkspaces.value = workspaces;
}

async function deleteSelectedSessions() {
  if (selectedSessionIds.value.length === 0) return;
  const confirmed = await confirmDialogRef.value?.ask({
    title: historyConfirm("settings.historyConfirm.deleteTitle"),
    description: historyConfirm("settings.historyConfirm.deleteSelectedDesc", {
      count: selectedSessionIds.value.length,
    }),
    confirmLabel: historyText.value.deleteLabel,
    cancelLabel: historyText.value.cancel,
  });
  if (!confirmed) return;
  try {
    await Promise.all(selectedSessionIds.value.map((id) => deleteChatSession(id)));
    selectedSessionIds.value = [];
    await loadSessions();
    await tauriEmit("history-updated");
  } catch (error) {
    console.error("Failed to delete sessions:", error);
  }
}

async function deleteHistoryGroup(group: HistoryGroup) {
  const sessionIds = historyGroupSessionIds(group);
  if (sessionIds.length === 0) return;
  const confirmed = await confirmDialogRef.value?.ask({
    title: historyConfirm("settings.historyConfirm.deleteGroupTitle"),
    description: historyConfirm("settings.historyConfirm.deleteGroupDesc", {
      name: group.name,
      count: sessionIds.length,
    }),
    confirmLabel: historyConfirm("settings.historyConfirm.deleteAllLabel"),
    cancelLabel: historyText.value.cancel,
  });
  if (!confirmed) return;
  try {
    await Promise.all(sessionIds.map((id) => deleteChatSession(id)));
    const deletedIds = new Set(sessionIds);
    selectedSessionIds.value = selectedSessionIds.value.filter((id) => !deletedIds.has(id));
    await loadSessions();
    await tauriEmit("history-updated");
  } catch (error) {
    console.error("Failed to delete history group:", error);
  }
}

async function archiveSingleSession(sessionId: string) {
  try {
    await setChatSessionArchived(sessionId, true);
    selectedSessionIds.value = selectedSessionIds.value.filter((id) => id !== sessionId);
    await loadSessions();
    await tauriEmit("history-updated");
  } catch (error) {
    console.error("Failed to archive session:", error);
  }
}

async function deleteSingleSession(sessionId: string) {
  const confirmed = await confirmDialogRef.value?.ask({
    title: historyConfirm("settings.historyConfirm.deleteTitle"),
    description: historyConfirm("settings.historyConfirm.deleteSingleDesc"),
    confirmLabel: historyText.value.deleteLabel,
    cancelLabel: historyText.value.cancel,
  });
  if (!confirmed) return;
  try {
    await deleteChatSession(sessionId);
    selectedSessionIds.value = selectedSessionIds.value.filter((id) => id !== sessionId);
    await loadSessions();
    await tauriEmit("history-updated");
  } catch (error) {
    console.error("Failed to delete session:", error);
  }
}

async function clearAllSessions() {
  const confirmMsg = historyConfirm("settings.historyConfirm.clearDesc");
  const confirmed = await confirmDialogRef.value?.ask({
    title: historyConfirm("settings.historyConfirm.clearTitle"),
    description: confirmMsg,
    confirmLabel: historyText.value.clearAll,
    cancelLabel: historyText.value.cancel,
  });
  if (!confirmed) return;
  try {
    await clearAllChatSessions();
    selectedSessionIds.value = [];
    await loadSessions();
    await tauriEmit("history-updated");
  } catch (error) {
    console.error("Failed to clear sessions:", error);
  }
}

async function openSession(session: ChatSessionSummary) {
  pageError.value = "";
  try {
    if (
      session.workspaceId &&
      historyWorkspaces.value.some((workspace) => workspace.id === session.workspaceId)
    ) {
      await switchWorkspace(session.workspaceId);
    }
    await openSessionInOverlay(session.sessionId);
  } catch (err) {
    pageError.value = `${historyText.value.openError}: ${String(err)}`;
  }
}

function formatTime(timestamp: number) {
  const date = new Date(timestamp);
  const now = new Date();

  const isToday = date.toDateString() === now.toDateString();
  if (isToday) {
    return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hour12: false });
  }

  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  const isYesterday = date.toDateString() === yesterday.toDateString();
  if (isYesterday) {
    return historyText.value.yesterday;
  }

  return `${date.getMonth() + 1}/${date.getDate()}`;
}

onMounted(() => {
  void loadSessions();
});
</script>

<style scoped>
.history-page {
  color: var(--peek-text);
}
.history-header {
  min-height: 49px;
  gap: 18px;
  padding: 0 0 15px;
  border-bottom: 1px solid var(--peek-border);
}
.history-groups {
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--peek-border) 88%, transparent);
  border-radius: var(--peek-radius-lg, 12px);
}
.history-group {
  border-bottom: 1px solid var(--peek-border);
}
.history-group:last-child {
  border-bottom: 0;
}
.group-header {
  min-height: 43px;
  padding: 4px 2px;
}
.group-header:hover {
  background: transparent;
}
.group-header > button:first-child {
  border-radius: 5px;
  padding: 5px;
}
.group-header > button:first-child:hover {
  background: var(--peek-hover-bg);
}
.group-header > button:first-child strong {
  color: var(--peek-text);
  font-size: 11px;
}
.group-header > button:first-child small {
  color: var(--peek-faint);
  font-size: 9px;
}
.group-header :deep([data-slot="button"]) {
  border-radius: 5px;
}
.history-session-row {
  min-height: 54px;
  padding: 7px 3px 7px 34px;
  border-top: 1px solid color-mix(in srgb, var(--peek-border) 72%, transparent);
}
.history-session-row h3 {
  color: var(--peek-text);
  font-size: 11px;
  font-weight: 600;
}
.history-session-row p {
  margin: 0;
  color: var(--peek-muted);
  font-size: 9px;
}
.history-session-row > div:last-child {
  gap: 3px;
}
.history-session-row :deep([data-slot="button"]) {
  height: 29px;
  border-radius: 5px;
  font-size: 10px;
}
@media (max-width: 640px) {
  .history-header {
    align-items: flex-start;
    flex-direction: column;
  }
  .history-session-row {
    grid-template-columns: minmax(0, 1fr);
    padding-left: 8px;
  }
  .history-session-row > div:last-child {
    justify-content: flex-start;
    padding-left: 27px;
  }
}
</style>
