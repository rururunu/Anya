<template>
  <section class="settings-page">
    <AppConfirmDialog ref="confirmDialogRef" />
    <SettingsPageHeader :title="copy.title">
      <template #actions>
        <Button size="sm" class="h-8 gap-1.5" :disabled="saving" @click="addWorkspace">
          <Plus class="size-3.5" />
          {{ copy.newWorkspace }}
        </Button>
      </template>
    </SettingsPageHeader>

    <p v-if="error" class="form-error">{{ error }}</p>

    <div class="settings-card workspace-list">
      <p v-if="workspaces.length === 0" class="empty">
        <FolderOpen class="size-5" />
        <span>{{ copy.empty }}</span>
      </p>
      <article
        v-for="workspace in workspaces"
        :key="workspace.id"
        :class="{ current: workspace.id === current?.id }"
      >
        <button type="button" class="workspace-select" @click="select(workspace)">
          <span class="folder-icon"><Folder class="size-4" /></span>
          <span class="copy">
            <span class="workspace-title">
              <strong>{{ workspace.name }}</strong>
              <span v-if="workspaceSourceLabel(workspace.source)" class="workspace-source">
                {{ workspaceSourceLabel(workspace.source) }}
              </span>
            </span>
            <span>{{ workspace.root }}</span>
          </span>
          <span v-if="workspace.id === current?.id" class="current-label">
            <Check class="size-3" />
            {{ copy.current }}
          </span>
        </button>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          class="size-8 shrink-0 text-muted-foreground hover:text-foreground"
          :title="copy.archiveWorkspace"
          :aria-label="copy.archiveWorkspace"
          @click="archive(workspace)"
        >
          <Archive class="size-3.5" />
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          class="size-8 shrink-0 text-muted-foreground hover:text-destructive"
          :title="copy.deleteWorkspace"
          :aria-label="copy.deleteWorkspace"
          @click="remove(workspace)"
        >
          <Trash2 class="size-3.5" />
        </Button>
      </article>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Archive, Check, Folder, FolderOpen, Plus, Trash2 } from "@lucide/vue";
import { Button } from "@/components/ui/button";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import SettingsPageHeader from "@/components/settings/SettingsPageHeader.vue";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";
import {
  createWorkspace,
  deleteWorkspace,
  getCurrentWorkspace,
  listWorkspaces,
  selectWorkspaceFolder,
  setWorkspaceArchived,
  switchWorkspace,
  workspaceSourceLabel,
  type Workspace,
} from "@/commands/workspace";

const settingStore = useSettingStore();
const workspaces = ref<Workspace[]>([]);
const current = ref<Workspace | null>(null);
const saving = ref(false);
const error = ref("");
const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);
let unlisten: UnlistenFn | null = null;

const copy = computed(() => {
  const language = settingStore.language;
  return {
    title: tr(language, "workspace.title"),
    newWorkspace: tr(language, "workspace.newWorkspace"),
    empty: tr(language, "workspace.empty"),
    current: tr(language, "workspace.current"),
    archiveWorkspace: tr(language, "workspace.archiveWorkspace"),
    deleteWorkspace: tr(language, "workspace.deleteWorkspace"),
    cancel: tr(language, "workspace.cancel"),
    confirmDelete: tr(language, "workspace.confirmDelete"),
    deleteConfirm: (name: string) => tr(language, "workspace.deleteConfirm", { name }),
  };
});

async function load() {
  [workspaces.value, current.value] = await Promise.all([listWorkspaces(), getCurrentWorkspace()]);
}

async function addWorkspace() {
  if (saving.value) return;
  saving.value = true;
  error.value = "";
  try {
    const root = await selectWorkspaceFolder();
    if (!root) return;
    const workspace = await createWorkspace(root);
    current.value = await switchWorkspace(workspace.id);
    await load();
  } catch (cause) {
    error.value = String(cause);
  } finally {
    saving.value = false;
  }
}

async function select(workspace: Workspace) {
  if (workspace.id !== current.value?.id) {
    current.value = await switchWorkspace(workspace.id);
  }
}

async function archive(workspace: Workspace) {
  await setWorkspaceArchived(workspace.id, true);
  await load();
}

async function remove(workspace: Workspace) {
  const confirmed = await confirmDialogRef.value?.ask({
    title: copy.value.deleteWorkspace,
    description: copy.value.deleteConfirm(workspace.name),
    confirmLabel: copy.value.confirmDelete,
    cancelLabel: copy.value.cancel,
  });
  if (!confirmed) return;
  await deleteWorkspace(workspace.id);
  await load();
}

onMounted(async () => {
  await load();
  unlisten = await listen("workspaces-changed", () => void load());
});

onUnmounted(() => unlisten?.());
</script>

<style scoped>
.form-error {
  margin: 0 0 12px;
  padding: 9px 10px;
  border-radius: 6px;
  background: color-mix(in srgb, var(--destructive) 8%, transparent);
  color: var(--destructive);
  font-size: 11px;
}
.empty {
  min-height: 220px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-direction: column;
  gap: 8px;
  margin: 0;
  color: var(--peek-muted);
  font-size: 12px;
}
.empty svg {
  color: var(--peek-faint);
}
.workspace-list article {
  min-height: 58px;
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 0 6px 0 2px;
  border-bottom: 1px solid color-mix(in srgb, var(--peek-border) 70%, transparent);
}
.workspace-list article:last-child {
  border-bottom: 0;
}
.workspace-list article:hover {
  background: color-mix(in srgb, var(--peek-text) 3%, transparent);
}
.workspace-select {
  min-width: 0;
  display: flex;
  flex: 1;
  align-items: center;
  gap: 10px;
  padding: 9px 5px;
  border: 0;
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: pointer;
}
.folder-icon {
  width: 25px;
  height: 25px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
  color: var(--peek-muted);
}
.current .folder-icon {
  color: var(--peek-accent);
}
.copy {
  min-width: 0;
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 2px;
}
.workspace-title {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 7px;
}
.copy strong {
  min-width: 0;
  overflow: hidden;
  color: var(--peek-text);
  font-size: 12px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.copy span {
  overflow: hidden;
  color: var(--peek-muted);
  font-size: 10px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.copy .workspace-source {
  flex: none;
  padding: 1px 6px;
  border: 1px solid color-mix(in srgb, var(--primary) 35%, var(--border));
  border-radius: 999px;
  color: var(--primary);
  font-size: 9px;
  font-weight: 600;
  line-height: 1.35;
}
.current-label {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  flex: none;
  color: var(--peek-accent);
  font-size: 10px;
  font-weight: 600;
}
.workspace-list :deep([data-slot="button"]) {
  border-radius: 5px;
}
@media (max-width: 620px) {
  .current-label {
    font-size: 0;
  }
  .current-label svg {
    width: 14px;
    height: 14px;
  }
}
</style>
