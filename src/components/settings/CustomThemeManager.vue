<template>
  <CustomBackgroundCard :is-zh="isZh" />

  <section class="settings-group custom-theme-group">
    <AppConfirmDialog ref="confirmDialogRef" />

    <div class="settings-group-header">
      <h2 class="settings-group-title">
        {{ isZh ? "个性化主题库" : "Custom Themes" }}
      </h2>
      <div class="settings-group-actions">
        <span v-if="copiedPrompt" class="prompt-copied-badge">
          {{ isZh ? "已复制创作提示词" : "Prompt Copied" }}
        </span>
        <Button
          variant="ghost"
          size="sm"
          class="h-7 text-xs text-muted-foreground hover:text-foreground flex items-center gap-1.5"
          @click="showImportDialog = true"
        >
          <Upload class="size-3.5" />
          <span>{{ isZh ? "导入主题" : "Import" }}</span>
        </Button>
        <Button
          variant="ghost"
          size="sm"
          class="h-7 text-xs text-muted-foreground hover:text-foreground flex items-center gap-1.5"
          @click="summonAnyaThemeDesign"
        >
          <Sparkles class="size-3.5" />
          <span>{{ isZh ? "让 Anya 创作" : "Ask Anya" }}</span>
        </Button>
      </div>
    </div>

    <div class="settings-card theme-grid">
      <p v-if="settingStore.customThemes.length === 0" class="settings-empty">
        {{
          isZh
            ? "暂无自定义主题。可点击右上角让 Anya 为您设计，或导入主题 JSON 配置。"
            : "No custom themes yet. Ask Anya to design one, or import a theme JSON configuration."
        }}
      </p>

      <article
        v-for="theme in settingStore.customThemes"
        :key="theme.id"
        class="settings-row is-wide"
      >
        <div class="settings-row-copy">
          <div class="settings-theme-title-row">
            <h3>{{ theme.name }}</h3>
            <span v-if="isActiveTheme(theme.id)" class="settings-status-badge is-configured">
              {{ isZh ? "当前使用" : "Active" }}
            </span>
            <span class="settings-status-badge">
              {{
                theme.mode === "dark"
                  ? isZh
                    ? "深色基底"
                    : "Dark Base"
                  : isZh
                    ? "浅色基底"
                    : "Light Base"
              }}
            </span>
            <span v-if="theme.background?.image" class="settings-status-badge is-configured">
              {{ isZh ? "专属壁纸" : "Wallpaper" }}
            </span>
          </div>

          <p class="settings-theme-meta">
            <span class="settings-swatches" aria-label="Theme color palette">
              <span
                class="settings-swatch"
                :style="{ backgroundColor: getThemeToken(theme, '--peek-accent') }"
                :title="`${isZh ? '强调色' : 'Accent'}: ${getThemeToken(theme, '--peek-accent')}`"
              />
              <span
                class="settings-swatch"
                :style="{ backgroundColor: getThemeToken(theme, '--peek-bg') }"
                :title="`${isZh ? '背景色' : 'Background'}: ${getThemeToken(theme, '--peek-bg')}`"
              />
              <span
                class="settings-swatch"
                :style="{ backgroundColor: getThemeToken(theme, '--peek-surface') }"
                :title="`${isZh ? '面板色' : 'Surface'}: ${getThemeToken(theme, '--peek-surface')}`"
              />
              <span
                class="settings-swatch"
                :style="{ backgroundColor: getThemeToken(theme, '--peek-text') }"
                :title="`${isZh ? '前景色' : 'Text'}: ${getThemeToken(theme, '--peek-text')}`"
              />
              <span
                class="settings-swatch"
                :style="{ backgroundColor: getThemeToken(theme, '--peek-icon') }"
                :title="`${isZh ? '图标色' : 'Icon'}: ${getThemeToken(theme, '--peek-icon')}`"
              />
            </span>
            <span class="settings-theme-id">{{ theme.id }}</span>
          </p>
        </div>

        <div class="settings-row-control">
          <div class="settings-theme-actions">
            <Button
              v-if="!isActiveTheme(theme.id)"
              variant="outline"
              size="sm"
              class="h-7 px-2.5 text-xs"
              @click="applyTheme(theme.id)"
            >
              {{ isZh ? "应用" : "Apply" }}
            </Button>
            <Button
              variant="ghost"
              size="icon"
              class="size-7 text-muted-foreground hover:text-foreground"
              :title="isZh ? '复制主题 JSON' : 'Copy Theme JSON'"
              :aria-label="isZh ? '复制主题 JSON' : 'Copy Theme JSON'"
              @click="copyThemeJson(theme)"
            >
              <Check v-if="copiedThemeId === theme.id" class="size-3.5 text-primary" />
              <Copy v-else class="size-3.5" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              class="size-7 text-muted-foreground hover:text-destructive"
              :title="isZh ? '删除主题' : 'Delete'"
              :aria-label="isZh ? '删除主题' : 'Delete'"
              @click="deleteTheme(theme)"
            >
              <Trash2 class="size-3.5" />
            </Button>
          </div>
        </div>
      </article>
    </div>

    <!-- 导入主题弹窗 (Reka UI Dialog) -->
    <DialogRoot :open="showImportDialog" @update:open="(v) => (showImportDialog = v)">
      <DialogPortal>
        <DialogOverlay class="confirm-overlay" />
        <DialogContent class="confirm-dialog is-detail" :aria-describedby="undefined">
          <button
            type="button"
            class="confirm-close"
            :aria-label="isZh ? '关闭' : 'Close'"
            @click="showImportDialog = false"
          >
            <X :size="16" />
          </button>
          <div class="confirm-body">
            <span class="confirm-icon" aria-hidden="true">
              <Upload :size="16" />
            </span>
            <div class="confirm-copy">
              <DialogTitle class="confirm-title">
                {{ isZh ? "导入自定义主题" : "Import Custom Theme" }}
              </DialogTitle>
              <DialogDescription class="confirm-description">
                {{
                  isZh
                    ? "粘贴符合规范的主题 JSON 配置（包含 id、name、mode 与 tokens）。"
                    : "Paste valid custom theme JSON with id, name, mode, and tokens."
                }}
              </DialogDescription>
            </div>
          </div>

          <div class="theme-import-box">
            <textarea
              v-model="importJsonDraft"
              class="theme-import-textarea"
              rows="7"
              :placeholder="importPlaceholder"
              spellcheck="false"
            />
            <p v-if="importError" class="theme-import-error">{{ importError }}</p>
          </div>

          <div class="confirm-actions">
            <button type="button" class="confirm-button ghost" @click="showImportDialog = false">
              {{ isZh ? "取消" : "Cancel" }}
            </button>
            <button type="button" class="confirm-button primary" @click="confirmImport">
              {{ isZh ? "解析并导入" : "Import" }}
            </button>
          </div>
        </DialogContent>
      </DialogPortal>
    </DialogRoot>
  </section>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { Check, Copy, Sparkles, Trash2, Upload, X } from "@lucide/vue";
import {
  DialogContent,
  DialogDescription,
  DialogOverlay,
  DialogPortal,
  DialogRoot,
  DialogTitle,
} from "reka-ui";
import { Button } from "@/components/ui/button";
import AppConfirmDialog from "@/components/ui/confirm-dialog/AppConfirmDialog.vue";
import CustomBackgroundCard from "@/components/settings/CustomBackgroundCard.vue";
import { harmonizeCustomTheme } from "@/services/theme/ThemeService";
import { useSettingStore } from "@/stores/setting";
import type { CustomThemeConfig, ThemeId } from "@/types/setting";

const settingStore = useSettingStore();
const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);

const isZh = computed(() => settingStore.language === "zh-CN");
const showImportDialog = ref(false);
const importJsonDraft = ref("");
const importError = ref("");
const copiedThemeId = ref<string | null>(null);
const copiedPrompt = ref(false);

let copyTimer: ReturnType<typeof setTimeout> | null = null;
let promptTimer: ReturnType<typeof setTimeout> | null = null;

const importPlaceholder = `{\n  "id": "custom-my-theme",\n  "name": "我的主题",\n  "mode": "dark",\n  "background": {\n    "image": "path:C:/wallpaper.png",\n    "opacity": 0.2\n  },\n  "tokens": {\n    "--peek-bg": "#12131a",\n    "--peek-accent": "#00f0ff"\n  }\n}`;

function isActiveTheme(id: string): boolean {
  return settingStore.colorScheme === id;
}

function getThemeToken(theme: CustomThemeConfig, token: string): string {
  const harmonized = harmonizeCustomTheme(theme);
  return harmonized[token] || "#888888";
}

function applyTheme(id: string) {
  void settingStore.update({ colorScheme: id as ThemeId });
}

async function copyThemeJson(theme: CustomThemeConfig) {
  try {
    if (navigator.clipboard) {
      await navigator.clipboard.writeText(JSON.stringify(theme, null, 2));
      copiedThemeId.value = theme.id;
      if (copyTimer) clearTimeout(copyTimer);
      copyTimer = setTimeout(() => {
        copiedThemeId.value = null;
      }, 1500);
    }
  } catch (err) {
    console.error("Failed to copy custom theme json:", err);
  }
}

async function deleteTheme(theme: CustomThemeConfig) {
  const confirmed = await confirmDialogRef.value?.ask({
    title: isZh.value ? "删除自定义主题" : "Delete Custom Theme",
    description: isZh.value
      ? `确定要删除自定义主题「${theme.name}」吗？此操作无法撤销。`
      : `Are you sure you want to delete custom theme "${theme.name}"? This action cannot be undone.`,
    confirmLabel: isZh.value ? "删除" : "Delete",
    cancelLabel: isZh.value ? "取消" : "Cancel",
    tone: "danger",
  });
  if (!confirmed) return;
  void settingStore.deleteCustomTheme(theme.id);
}

function confirmImport() {
  importError.value = "";
  if (!importJsonDraft.value.trim()) {
    importError.value = isZh.value ? "请输入主题 JSON" : "Please input theme JSON";
    return;
  }
  try {
    const parsed = JSON.parse(importJsonDraft.value) as Partial<CustomThemeConfig>;
    if (!parsed.id || typeof parsed.id !== "string") {
      throw new Error(isZh.value ? "缺少必须的 id 字段" : "Missing required 'id' field");
    }
    if (!parsed.name || typeof parsed.name !== "string") {
      throw new Error(isZh.value ? "缺少必须的 name 字段" : "Missing required 'name' field");
    }
    const mode = parsed.mode === "dark" || parsed.mode === "light" ? parsed.mode : "dark";
    const theme: CustomThemeConfig = {
      id: parsed.id.trim(),
      name: parsed.name.trim(),
      mode,
      description: parsed.description ? String(parsed.description) : undefined,
      tokens: parsed.tokens && typeof parsed.tokens === "object" ? parsed.tokens : {},
      background: parsed.background,
      updatedAt: Date.now(),
    };
    void settingStore.saveCustomTheme(theme);
    showImportDialog.value = false;
    importJsonDraft.value = "";
  } catch (err) {
    importError.value = err instanceof Error ? err.message : String(err);
  }
}

function summonAnyaThemeDesign() {
  const prompt = isZh.value
    ? "我想制作一款新的 Anya 个性化主题，请根据我喜欢的风格（例如：赛博朋克霓虹、深海蔚蓝、复古暖阳等）为我设计一套配色方案，并使用 manage_custom_theme 工具为我创建并实时应用。"
    : "I'd like to create a new custom theme for Anya. Please design a distinctive color scheme for me and use the manage_custom_theme tool to create and apply it.";

  if (navigator.clipboard) {
    void navigator.clipboard.writeText(prompt);
  }
  copiedPrompt.value = true;
  if (promptTimer) clearTimeout(promptTimer);
  promptTimer = setTimeout(() => {
    copiedPrompt.value = false;
  }, 2200);
}
</script>

<style scoped>
.custom-theme-group {
  margin-bottom: 26px;
}

.theme-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
  overflow: visible;
  padding: 0;
  border: 0;
  background: transparent;
}

.theme-grid .settings-row {
  display: flex;
  min-width: 0;
  min-height: 108px;
  flex-direction: column;
  align-items: stretch;
  gap: 8px;
  padding: 14px;
  border: 1px solid color-mix(in srgb, var(--peek-border) 55%, transparent);
  border-radius: 14px;
  background: var(--peek-list-bg);
}

.theme-grid .settings-row-copy {
  min-width: 0;
}

.theme-grid .settings-row-control {
  width: 100%;
  margin-top: auto;
  justify-content: flex-end;
}

.theme-grid .settings-empty {
  grid-column: 1 / -1;
  border: 1px solid color-mix(in srgb, var(--peek-border) 55%, transparent);
  border-radius: 14px;
  background: var(--peek-list-bg);
}

.settings-group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin: 0 0 8px;
  padding: 0 2px;
}

.settings-group-header .settings-group-title {
  margin: 0;
}

.settings-group-actions {
  display: flex;
  align-items: center;
  gap: 6px;
}

.prompt-copied-badge {
  font-size: 11px;
  color: var(--primary, var(--peek-accent));
  padding: 2px 6px;
  border-radius: 4px;
  background: color-mix(in srgb, var(--primary, var(--peek-accent)) 10%, transparent);
  animation: fade-in 150ms ease;
}

.settings-theme-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.settings-theme-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 6px;
}

.settings-swatches {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.settings-swatch {
  display: inline-block;
  width: 13px;
  height: 13px;
  border-radius: 50%;
  border: 1px solid color-mix(in srgb, var(--peek-border) 80%, transparent);
  box-shadow: 0 1px 2px rgb(0 0 0 / 10%);
}

.settings-theme-id {
  font-family: var(--font-mono, ui-monospace, monospace);
  font-size: 11px;
  color: var(--peek-muted);
}

.settings-theme-actions {
  display: flex;
  align-items: center;
  gap: 6px;
}

@container settings-page (max-width: 760px) {
  .theme-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}

.theme-import-box {
  margin-top: 14px;
}

.theme-import-textarea {
  box-sizing: border-box;
  width: 100%;
  padding: 8px 10px;
  border: 1px solid var(--peek-border);
  border-radius: var(--peek-radius-sm, 6px);
  background: var(--peek-surface);
  color: var(--peek-text);
  font-family: var(--font-mono, ui-monospace, monospace);
  font-size: 11px;
  line-height: 1.45;
  resize: vertical;
}

.theme-import-textarea:focus {
  outline: none;
  border-color: var(--peek-accent);
}

.theme-import-error {
  margin: 6px 0 0;
  color: var(--peek-danger);
  font-size: 11px;
}
</style>
