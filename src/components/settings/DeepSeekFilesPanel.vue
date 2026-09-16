<template>
  <section class="files-panel" :aria-label="copy.title">
    <AppConfirmDialog ref="confirmDialogRef" />

    <header class="files-head">
      <div>
        <h3>{{ copy.title }}</h3>
        <p>{{ copy.hint }}</p>
      </div>
      <div class="files-actions">
        <Button
          variant="outline"
          size="sm"
          class="h-7 gap-1"
          :disabled="!configured || loading"
          @click="load(true)"
        >
          <RefreshCw :size="13" :class="{ spinning: loading }" />
          {{ copy.refresh }}
        </Button>
        <Button size="sm" class="h-7 gap-1" :disabled="!configured || uploading" @click="upload">
          <Upload :size="13" />
          {{ copy.upload }}
        </Button>
      </div>
    </header>

    <div v-if="!configured" class="files-empty">{{ copy.unconfigured }}</div>
    <div v-else-if="loading && files.length === 0" class="files-empty">
      <span class="loader" />
      {{ copy.loading }}
    </div>
    <p v-else-if="error" class="files-error">{{ copy.error }}: {{ error }}</p>
    <div v-else-if="files.length === 0" class="files-empty">{{ copy.empty }}</div>
    <ul v-else class="files-list">
      <li v-for="file in files" :key="file.id" class="files-row">
        <div class="files-copy">
          <strong>{{ file.filename }}</strong>
          <span>{{ formatBytes(file.bytes) }} · {{ formatTime(file.createdAt) }}</span>
          <span v-if="file.expiresAt" class="files-expires">
            {{ copy.expires.replace("{date}", formatTime(file.expiresAt)) }}
          </span>
          <code>{{ file.id }}</code>
        </div>
        <Button
          variant="ghost"
          size="icon-xs"
          class="files-delete"
          :title="copy.delete"
          :aria-label="copy.delete"
          :disabled="deletingId === file.id"
          @click="remove(file)"
        >
          <Trash2 :size="13" />
        </Button>
      </li>
    </ul>
    <Button
      v-if="hasMore"
      variant="outline"
      size="sm"
      class="h-7"
      :disabled="loading"
      @click="load(false)"
    >
      {{ copy.loadMore }}
    </Button>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { RefreshCw, Trash2, Upload } from "@lucide/vue";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import { Button } from "@/components/ui/button";
import { tr } from "@/services/i18n";
import { deleteDeepSeekFile, listDeepSeekFiles, uploadDeepSeekFile } from "@/services/ipc";
import { useSettingStore } from "@/stores/setting";
import type { DeepSeekFileObject } from "@/types/tokenUsage";

const settingStore = useSettingStore();
const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);
const files = ref<DeepSeekFileObject[]>([]);
const loading = ref(false);
const uploading = ref(false);
const error = ref("");
const hasMore = ref(false);
const deletingId = ref("");
const cursor = ref<string | null>(null);

const configured = computed(() => Boolean(settingStore.deepseekApiKey.trim()));
const copy = computed(() => ({
  title: tr(settingStore.language, "files.title"),
  hint: tr(settingStore.language, "files.hint"),
  refresh: tr(settingStore.language, "files.refresh"),
  upload: tr(settingStore.language, "files.upload"),
  loading: tr(settingStore.language, "files.loading"),
  empty: tr(settingStore.language, "files.empty"),
  unconfigured: tr(settingStore.language, "files.unconfigured"),
  error: tr(settingStore.language, "files.error"),
  delete: tr(settingStore.language, "files.delete"),
  deleteConfirm: tr(settingStore.language, "files.deleteConfirm"),
  cancel: tr(settingStore.language, "files.cancel"),
  expires: tr(settingStore.language, "files.expires"),
  loadMore: tr(settingStore.language, "files.loadMore"),
}));

/** Load DeepSeek Files API listing, resetting when `reset` is true. */
async function load(reset: boolean) {
  if (!configured.value) {
    files.value = [];
    hasMore.value = false;
    error.value = "";
    return;
  }
  loading.value = true;
  error.value = "";
  try {
    const list = await listDeepSeekFiles(reset ? undefined : (cursor.value ?? undefined), 50);
    files.value = reset ? list.data : [...files.value, ...list.data];
    cursor.value = list.lastId ?? null;
    hasMore.value = list.hasMore;
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : String(cause);
    if (reset) files.value = [];
  } finally {
    loading.value = false;
  }
}

/** Pick a local image and POST it to DeepSeek /files. */
async function upload() {
  if (!configured.value) return;
  const selected = await openFileDialog({
    multiple: false,
    filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp", "gif"] }],
  });
  const path = Array.isArray(selected) ? selected[0] : selected;
  if (!path) return;
  uploading.value = true;
  error.value = "";
  try {
    await uploadDeepSeekFile(path);
    await load(true);
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : String(cause);
  } finally {
    uploading.value = false;
  }
}

/** Confirm then DELETE /files/:id. */
async function remove(file: DeepSeekFileObject) {
  const confirmed = await confirmDialogRef.value?.ask({
    title: copy.value.delete,
    description: copy.value.deleteConfirm.replace("{name}", file.filename),
    confirmLabel: copy.value.delete,
    cancelLabel: copy.value.cancel,
  });
  if (!confirmed) return;
  deletingId.value = file.id;
  error.value = "";
  try {
    await deleteDeepSeekFile(file.id);
    files.value = files.value.filter((item) => item.id !== file.id);
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : String(cause);
  } finally {
    deletingId.value = "";
  }
}

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatTime(unixSeconds: number) {
  const date = new Date(unixSeconds * 1000);
  if (Number.isNaN(date.getTime())) return String(unixSeconds);
  return date.toLocaleString(settingStore.language);
}

watch(configured, (ready) => {
  if (ready) void load(true);
  else {
    files.value = [];
    error.value = "";
  }
});

onMounted(() => {
  void load(true);
});
</script>

<style scoped>
.files-panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding-top: 4px;
  border-top: 1px solid var(--border);
}

.files-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.files-head h3 {
  margin: 0;
  font-size: 12px;
  font-weight: 600;
}

.files-head p {
  margin: 4px 0 0;
  font-size: 10px;
  line-height: 1.4;
  color: var(--muted-foreground);
}

.files-actions {
  display: flex;
  flex-shrink: 0;
  gap: 6px;
}

.files-empty,
.files-error {
  margin: 0;
  font-size: 11px;
  color: var(--muted-foreground);
}

.files-error {
  color: var(--destructive);
}

.files-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.files-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 8px;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: color-mix(in srgb, var(--sidebar) 55%, transparent);
}

.files-copy {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 2px;
}

.files-copy strong {
  font-size: 12px;
  font-weight: 500;
}

.files-copy span,
.files-expires {
  font-size: 10px;
  color: var(--muted-foreground);
}

.files-copy code {
  font-size: 10px;
  color: var(--muted-foreground);
  word-break: break-all;
}

.files-delete {
  color: var(--muted-foreground);
}

.files-delete:hover {
  color: var(--destructive);
}

.loader {
  display: inline-block;
  width: 10px;
  height: 10px;
  margin-right: 6px;
  border: 1.5px solid color-mix(in srgb, var(--muted-foreground) 35%, transparent);
  border-top-color: var(--foreground);
  border-radius: 50%;
  animation: files-spin 0.7s linear infinite;
  vertical-align: -1px;
}

.spinning {
  animation: files-spin 0.7s linear infinite;
}

@keyframes files-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
