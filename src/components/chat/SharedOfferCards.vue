<template>
  <section v-if="files.length || urls.length" class="shared-offers">
    <button
      v-for="file in files"
      :key="file.offerId || file.path"
      type="button"
      class="shared-offer-card"
      :title="file.absolutePath || file.path"
      @click="openFile(file)"
    >
      <span class="shared-offer-icon" aria-hidden="true">
        <File :size="16" :stroke-width="1.8" />
      </span>
      <span class="shared-offer-body">
        <strong>{{ file.name }}</strong>
        <span class="shared-offer-meta">
          {{ fileMeta(file) }}
        </span>
      </span>
    </button>
    <button
      v-for="url in urls"
      :key="url.offerId || url.publicUrl"
      type="button"
      class="shared-offer-card"
      :title="url.publicUrl"
      @click="openUrlCard(url)"
    >
      <span class="shared-offer-icon" aria-hidden="true">
        <Globe :size="16" :stroke-width="1.8" />
      </span>
      <span class="shared-offer-body">
        <strong>{{ url.label || tr(settingStore.language, "sharedUrlCard") }}</strong>
        <span class="shared-offer-meta">{{ url.publicUrl }}</span>
      </span>
    </button>
  </section>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { File, Globe } from "@lucide/vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { revealInExplorer } from "@/services/ipc";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";
import type { ChatMessage, SharedFileOffer, SharedUrlOffer } from "@/types/chat";

const props = defineProps<{
  message: ChatMessage;
}>();

const settingStore = useSettingStore();

const files = computed(() => {
  if (props.message.sharedFiles?.length) {
    return props.message.sharedFiles;
  }
  return filesFromActivities(props.message);
});

const urls = computed(() => {
  if (props.message.sharedUrls?.length) {
    return props.message.sharedUrls;
  }
  return urlsFromActivities(props.message);
});

function fileMeta(file: SharedFileOffer): string {
  const parts = [fileTypeLabel(file), formatBytes(file.size)].filter(Boolean);
  return parts.join(" · ");
}

function fileTypeLabel(file: SharedFileOffer): string {
  if (file.mime && file.mime !== "application/octet-stream") {
    const subtype = file.mime.split("/")[1];
    if (subtype) return subtype.toUpperCase();
  }
  const ext = file.name.split(".").pop();
  return ext && ext !== file.name ? ext.toUpperCase() : tr(settingStore.language, "sharedFileCard");
}

function formatBytes(size: number): string {
  if (!size || size <= 0) return "";
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
  return `${(size / (1024 * 1024)).toFixed(1)} MB`;
}

async function openFile(file: SharedFileOffer) {
  const path = file.absolutePath || file.path;
  if (!path) return;
  try {
    await revealInExplorer(path);
  } catch {
    /* ignore */
  }
}

async function openUrlCard(url: SharedUrlOffer) {
  if (!url.publicUrl) return;
  try {
    await openUrl(url.publicUrl);
  } catch {
    /* ignore */
  }
}

function filesFromActivities(message: ChatMessage): SharedFileOffer[] {
  const offers: SharedFileOffer[] = [];
  for (const activity of message.toolActivities ?? []) {
    if (activity.toolName !== "share_to_companion" || activity.status !== "done") continue;
    const args = activity.arguments ?? {};
    const path = String(args.path ?? "");
    if (!path) continue;
    const name = String(args.label ?? "") || path.replace(/\\/g, "/").split("/").pop() || path;
    offers.push({
      offerId: activity.id,
      path,
      name,
      mime: "",
      size: 0,
    });
  }
  return offers;
}

function urlsFromActivities(message: ChatMessage): SharedUrlOffer[] {
  const offers: SharedUrlOffer[] = [];
  for (const activity of message.toolActivities ?? []) {
    if (activity.toolName !== "share_preview_url" || activity.status !== "done") continue;
    const args = activity.arguments ?? {};
    const result = activity.result ?? "";
    const publicUrl =
      result.match(/https?:\/\/\S+/)?.[0]?.replace(/[.,)]+$/, "") || String(args.url ?? "");
    if (!publicUrl) continue;
    offers.push({
      offerId: activity.id,
      label: String(args.label ?? "") || tr(settingStore.language, "sharedUrlCard"),
      originUrl: String(args.url ?? ""),
      publicUrl,
    });
  }
  return offers;
}
</script>

<style scoped>
.shared-offers {
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 100%;
}

.shared-offer-card {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  box-sizing: border-box;
  margin: 0;
  padding: 10px 12px;
  text-align: left;
  cursor: pointer;
  color: inherit;
  border: 1px solid color-mix(in srgb, var(--peek-border) 88%, transparent);
  border-radius: 10px;
  background: color-mix(in srgb, var(--peek-input-bg) 92%, #0b0d10);
}

.shared-offer-card:hover {
  border-color: color-mix(in srgb, var(--peek-accent) 45%, var(--peek-border));
}

.shared-offer-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  width: 28px;
  height: 28px;
  color: var(--peek-muted);
}

.shared-offer-body {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 2px;
}

.shared-offer-body strong {
  overflow: hidden;
  font-size: 13px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.shared-offer-meta {
  overflow: hidden;
  color: var(--peek-muted);
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
