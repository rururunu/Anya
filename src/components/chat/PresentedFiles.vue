<template>
  <div v-if="files.length" class="presented-files">
    <div class="delivery-grid">
      <div v-for="file in visible" :key="file.absolutePath" class="delivery-card">
        <button
          class="delivery-main"
          type="button"
          :title="file.absolutePath"
          :aria-label="`${t(isPreviewable(file.name) ? 'delivery.preview' : 'delivery.open')}: ${file.name}`"
          :disabled="busy.has(file.absolutePath)"
          @click="openCard(file)"
        >
          <span class="delivery-icon">
            <DeliveryFileIcon :name="file.name" />
          </span>
          <span class="delivery-body">
            <span class="delivery-name">{{ file.name }}</span>
            <span class="delivery-description">
              {{ file.description || extension(file.name) || t("delivery.file") }}
            </span>
            <span class="delivery-meta">{{ sizeLabel(file.size) }}</span>
          </span>
        </button>
        <button
          v-if="isPreviewable(file.name)"
          class="delivery-reveal"
          type="button"
          :title="t('delivery.open')"
          :aria-label="`${t('delivery.open')}: ${file.name}`"
          :disabled="busy.has(file.absolutePath)"
          @click="act(file, 'open')"
        >
          <ExternalLink :size="15" :stroke-width="1.75" />
        </button>
        <button
          class="delivery-reveal"
          type="button"
          :title="t('delivery.reveal')"
          :aria-label="`${t('delivery.reveal')}: ${file.name}`"
          :disabled="busy.has(file.absolutePath)"
          @click="act(file, 'reveal')"
        >
          <FolderOpen :size="15" :stroke-width="1.75" />
        </button>
        <span v-if="errors[file.absolutePath]" class="delivery-error" role="alert">
          {{ t("delivery.error") }}
        </span>
      </div>
    </div>
    <button
      v-if="files.length > 4"
      class="delivery-toggle"
      type="button"
      :aria-expanded="expanded"
      @click="expanded = !expanded"
    >
      {{ expanded ? t("delivery.collapse") : `${t("delivery.all")} (${files.length})` }}
      <component :is="expanded ? ChevronUp : ChevronDown" :size="13" />
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { FolderOpen, ExternalLink, ChevronDown, ChevronUp } from "@lucide/vue";
import DeliveryFileIcon from "./DeliveryFileIcon.vue";
import { presentedFiles, type PresentedFile } from "@/services/chat/presentedFiles";
import { openInDefaultApp, revealInExplorer } from "@/services/ipc";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";
import type { ChatMessage } from "@/types/chat";
import type { UiI18nKey } from "@/services/locales/ui";

const props = defineProps<{ message: ChatMessage }>();
const emit = defineEmits<{ previewImage: [source: string] }>();
const settingStore = useSettingStore();
const t = (key: UiI18nKey) => tr(settingStore.language, key);
const files = computed(() => presentedFiles(props.message));
const expanded = ref(false);
const visible = computed(() => (expanded.value ? files.value : files.value.slice(0, 4)));
const busy = ref(new Set<string>());
const errors = ref<Record<string, boolean>>({});
function extension(name: string) {
  return name.includes(".") ? name.split(".").pop()!.toUpperCase() : "";
}
function isPreviewable(name: string) {
  return ["PNG", "JPG", "JPEG", "WEBP", "GIF"].includes(extension(name));
}
function openCard(file: PresentedFile) {
  if (isPreviewable(file.name)) emit("previewImage", file.absolutePath);
  else void act(file, "open");
}
function sizeLabel(size: number) {
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
  return `${(size / (1024 * 1024)).toFixed(1)} MB`;
}
async function act(file: PresentedFile, action: "open" | "reveal") {
  if (busy.value.has(file.absolutePath)) return;
  busy.value.add(file.absolutePath);
  errors.value[file.absolutePath] = false;
  try {
    if (action === "open") await openInDefaultApp(file.absolutePath);
    else await revealInExplorer(file.absolutePath);
  } catch {
    errors.value[file.absolutePath] = true;
  } finally {
    busy.value.delete(file.absolutePath);
  }
}
</script>

<style scoped>
.presented-files {
  margin-top: 10px;
  width: 100%;
}
.delivery-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}
.delivery-card {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 6px;
  padding: 10px;
  border: 1px solid var(--peek-border);
  border-radius: 12px;
  background: color-mix(in srgb, var(--peek-text) 2%, transparent);
}
.delivery-main {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0;
  border: 0;
  background: transparent;
  text-align: left;
  color: var(--peek-text);
  cursor: pointer;
}
.delivery-icon {
  flex: none;
  display: grid;
  place-items: center;
  width: 36px;
  height: 40px;
  border-radius: 8px;
  background: color-mix(in srgb, var(--peek-text) 4%, transparent);
  color: var(--peek-muted);
}
.delivery-body {
  display: flex;
  flex-direction: column;
  min-width: 0;
  gap: 3px;
}
.delivery-name {
  font-size: 12px;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.delivery-description {
  font-size: 11px;
  color: var(--peek-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.delivery-meta {
  font-size: 10px;
  color: var(--peek-faint);
}
.delivery-reveal {
  flex: none;
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  padding: 0;
  border: 0;
  border-radius: 6px;
  color: var(--peek-muted);
  background: transparent;
  cursor: pointer;
}
.delivery-reveal:hover,
.delivery-main:hover .delivery-icon {
  background: color-mix(in srgb, var(--peek-text) 7%, transparent);
  color: var(--peek-text);
}
.delivery-main:focus-visible,
.delivery-reveal:focus-visible,
.delivery-toggle:focus-visible {
  outline: 2px solid var(--peek-muted);
  outline-offset: 3px;
}
.delivery-main:disabled,
.delivery-reveal:disabled {
  opacity: 0.55;
  cursor: wait;
}
.delivery-error {
  flex-basis: 100%;
  color: var(--destructive);
  font-size: 11px;
}
.delivery-toggle {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  margin-top: 6px;
  padding: 3px 0;
  border: 0;
  background: transparent;
  color: var(--peek-muted);
  font-size: 11px;
  cursor: pointer;
}
@media (max-width: 640px) {
  .delivery-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
