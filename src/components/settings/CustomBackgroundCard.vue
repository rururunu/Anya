<template>
  <section class="settings-group custom-bg-group">
    <div class="settings-group-header">
      <h2 class="settings-group-title">
        {{ isZh ? "自定义背景壁纸" : "Custom Background Wallpaper" }}
      </h2>
      <div class="settings-group-actions">
        <span v-if="copiedPrompt" class="prompt-copied-badge">
          {{ isZh ? "已复制创作提示词" : "Prompt Copied" }}
        </span>
        <Button
          variant="ghost"
          size="sm"
          class="h-7 text-xs text-muted-foreground hover:text-foreground flex items-center gap-1.5"
          @click="summonAnyaBackgroundDesign"
        >
          <Sparkles class="size-3.5" />
          <span>{{ isZh ? "让 Anya 绘图" : "Ask Anya to Paint" }}</span>
        </Button>
      </div>
    </div>

    <div class="settings-card bg-card-shell">
      <div v-if="activeBg?.image" class="bg-active-layout">
        <div class="bg-preview-box">
          <img
            v-if="previewSrc"
            :src="previewSrc"
            :alt="isZh ? '当前背景预览' : 'Current background preview'"
            class="bg-preview-img"
            :style="{
              filter:
                (activeBg.blur ?? 0) > 0 ? `blur(${Math.min(activeBg.blur ?? 0, 10)}px)` : 'none',
              opacity: Math.max(0.2, activeBg.opacity ?? 0.2),
            }"
          />
          <div class="bg-preview-badge">
            {{ activeThemeName }}
          </div>
        </div>

        <div class="bg-controls">
          <div class="bg-path-row" :title="activeBg.image">
            <span class="bg-path-label">{{ isZh ? "壁纸源" : "Source" }}:</span>
            <span class="bg-path-text">{{ activeBg.image }}</span>
          </div>

          <div class="bg-slider-row">
            <div class="bg-slider-label">
              <span>{{ isZh ? "不透明度" : "Opacity" }}</span>
              <span class="bg-slider-val">{{ Math.round((activeBg.opacity ?? 0.2) * 100) }}%</span>
            </div>
            <input
              type="range"
              min="5"
              max="100"
              step="5"
              :value="Math.round((activeBg.opacity ?? 0.2) * 100)"
              class="setting-slider h-1.5 w-full cursor-pointer appearance-none rounded-lg bg-border accent-primary focus:outline-none"
              @input="onOpacityInput"
            />
          </div>

          <div class="bg-slider-row">
            <div class="bg-slider-label">
              <span>{{ isZh ? "模糊度" : "Blur" }}</span>
              <span class="bg-slider-val">{{ activeBg.blur ?? 0 }}px</span>
            </div>
            <input
              type="range"
              min="0"
              max="20"
              step="1"
              :value="activeBg.blur ?? 0"
              class="setting-slider h-1.5 w-full cursor-pointer appearance-none rounded-lg bg-border accent-primary focus:outline-none"
              @input="onBlurInput"
            />
          </div>

          <div class="bg-actions-row">
            <Button
              variant="outline"
              size="sm"
              class="h-7 px-2.5 text-xs flex items-center gap-1.5"
              @click="pickLocalImage"
            >
              <FolderOpen class="size-3.5" />
              <span>{{ isZh ? "更换本地图片" : "Change Image" }}</span>
            </Button>
            <Button
              variant="ghost"
              size="sm"
              class="h-7 px-2 text-xs text-muted-foreground hover:text-destructive flex items-center gap-1.5"
              @click="clearBackground"
            >
              <Trash2 class="size-3.5" />
              <span>{{ isZh ? "移除背景" : "Remove" }}</span>
            </Button>
          </div>
        </div>
      </div>

      <div v-else class="bg-empty-layout">
        <div class="bg-empty-icon" aria-hidden="true">
          <Image class="size-7 text-muted-foreground" />
        </div>
        <div class="bg-empty-copy">
          <p class="text-xs text-muted-foreground">
            {{
              isZh
                ? "当前主题未设置背景壁纸。可上传本地图片作为背景，或让 Anya 绘图生成契合主题的宽屏壁纸并融入主题。"
                : "No custom background set for this theme. Choose a local image, or ask Anya to generate a tailored background."
            }}
          </p>
          <div class="bg-empty-buttons">
            <Button
              variant="outline"
              size="sm"
              class="h-7 px-3 text-xs flex items-center gap-1.5"
              @click="pickLocalImage"
            >
              <FolderOpen class="size-3.5" />
              <span>{{ isZh ? "选择本地图片" : "Choose Local Image" }}</span>
            </Button>
            <Button
              variant="ghost"
              size="sm"
              class="h-7 px-3 text-xs text-muted-foreground hover:text-foreground flex items-center gap-1.5"
              @click="summonAnyaBackgroundDesign"
            >
              <Sparkles class="size-3.5" />
              <span>{{ isZh ? "让 Anya 创作" : "Ask Anya" }}</span>
            </Button>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { FolderOpen, Image, Sparkles, Trash2 } from "@lucide/vue";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { useResolvedBackgroundSrc } from "@/composables/theme/useResolvedBackgroundSrc";
import { useSettingStore } from "@/stores/setting";
import type { ThemeBackgroundConfig } from "@/types/setting";

const props = defineProps<{
  isZh: boolean;
}>();

const settingStore = useSettingStore();
const copiedPrompt = ref(false);
let promptTimer: ReturnType<typeof setTimeout> | null = null;

const activeCustomTheme = computed(() => {
  return settingStore.customThemes.find((t) => t.id === settingStore.colorScheme);
});

const activeThemeName = computed(() => {
  if (activeCustomTheme.value) return activeCustomTheme.value.name;
  if (settingStore.colorScheme === "dark") return props.isZh ? "深色默认" : "Dark";
  return props.isZh ? "浅色默认" : "Light";
});

const activeBg = computed<ThemeBackgroundConfig | undefined>(() => {
  if (activeCustomTheme.value?.background?.image) {
    return activeCustomTheme.value.background;
  }
  return settingStore.customBackground;
});

const { resolvedSource: previewSrc } = useResolvedBackgroundSrc(() => activeBg.value?.image);

async function pickLocalImage() {
  const selected = await openFileDialog({
    multiple: false,
    filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp", "gif"] }],
  });
  const path = Array.isArray(selected) ? selected[0] : selected;
  if (!path) return;

  const current = activeBg.value;
  const config: ThemeBackgroundConfig = {
    image: path,
    opacity: current?.opacity ?? 0.2,
    blur: current?.blur ?? 0,
    fit: current?.fit ?? "cover",
  };

  if (activeCustomTheme.value) {
    await settingStore.saveThemeBackground(activeCustomTheme.value.id, config);
  } else {
    await settingStore.update({ customBackground: config });
  }
}

async function onOpacityInput(event: Event) {
  const target = event.target as HTMLInputElement | null;
  if (!target) return;
  const percent = Number(target.value);
  const opacity = Math.max(0.05, Math.min(1, percent / 100));

  const current = activeBg.value;
  if (!current?.image) return;

  const config: ThemeBackgroundConfig = {
    ...current,
    opacity,
  };

  if (activeCustomTheme.value) {
    await settingStore.saveThemeBackground(activeCustomTheme.value.id, config);
  } else {
    await settingStore.update({ customBackground: config });
  }
}

async function onBlurInput(event: Event) {
  const target = event.target as HTMLInputElement | null;
  if (!target) return;
  const blur = Math.max(0, Math.min(20, Number(target.value)));

  const current = activeBg.value;
  if (!current?.image) return;

  const config: ThemeBackgroundConfig = {
    ...current,
    blur,
  };

  if (activeCustomTheme.value) {
    await settingStore.saveThemeBackground(activeCustomTheme.value.id, config);
  } else {
    await settingStore.update({ customBackground: config });
  }
}

async function clearBackground() {
  if (activeCustomTheme.value) {
    await settingStore.saveThemeBackground(activeCustomTheme.value.id, null);
  } else {
    await settingStore.update({ customBackground: null });
  }
}

function summonAnyaBackgroundDesign() {
  const themeName = activeThemeName.value;
  const prompt = props.isZh
    ? `请为我当前的 Anya 主题（${themeName}）量身设计并生成一张契合其色彩氛围的高清宽屏背景壁纸（使用 generate_image 工具，建议尺寸 1536x1024）。生成完成后，使用 manage_custom_theme 工具将图片路径以合适的不透明度（如 0.2）融入主题背景中。`
    : `Please design and generate an atmospheric wide-aspect background wallpaper matching my current Anya theme (${themeName}) using the generate_image tool (suggested size 1536x1024). Then use manage_custom_theme to integrate the resulting image into the theme background with appropriate opacity (e.g. 0.2).`;

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
.custom-bg-group {
  margin-bottom: var(--peek-space-5, 20px);
}

.settings-group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin: 0 0 8px;
  padding: 0 2px;
}

.settings-group-title {
  margin: 0;
  font-size: 14px;
  font-weight: 650;
  color: var(--peek-text);
}

.settings-group-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.prompt-copied-badge {
  font-size: 11px;
  font-weight: 500;
  color: var(--peek-success, #22c55e);
  background: color-mix(in srgb, var(--peek-success, #22c55e) 12%, transparent);
  padding: 2px 7px;
  border-radius: 4px;
}

.settings-card {
  padding: 16px 18px;
  border-radius: 14px;
  border: 1px solid color-mix(in srgb, var(--peek-border) 55%, transparent);
  background: var(--peek-list-bg);
}

.bg-active-layout {
  display: flex;
  gap: 16px;
  align-items: center;
}

.bg-preview-box {
  position: relative;
  flex: none;
  width: 140px;
  height: 88px;
  border-radius: 6px;
  overflow: hidden;
  border: 1px solid var(--peek-border);
  background: var(--peek-bg);
}

.bg-preview-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  transition: all 200ms ease;
}

.bg-preview-badge {
  position: absolute;
  bottom: 4px;
  left: 4px;
  right: 4px;
  font-size: 10px;
  font-weight: 550;
  padding: 2px 6px;
  border-radius: 4px;
  background: rgba(0, 0, 0, 0.68);
  color: #ffffff;
  text-overflow: ellipsis;
  overflow: hidden;
  white-space: nowrap;
}

.bg-controls {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.bg-path-row {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--peek-muted);
}

.bg-path-label {
  flex: none;
  font-weight: 500;
}

.bg-path-text {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: monospace;
  font-size: 10.5px;
}

.bg-slider-row {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.bg-slider-label {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  color: var(--peek-muted);
}

.bg-slider-val {
  font-weight: 550;
  color: var(--peek-text);
  font-variant-numeric: tabular-nums;
}

.bg-actions-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 2px;
}

.bg-empty-layout {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 4px 2px;
}

.bg-empty-icon {
  flex: none;
  width: 52px;
  height: 52px;
  border-radius: 8px;
  border: 1px dashed var(--peek-border);
  display: grid;
  place-items: center;
  background: color-mix(in srgb, var(--peek-muted) 8%, transparent);
}

.bg-empty-copy {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.bg-empty-buttons {
  display: flex;
  align-items: center;
  gap: 8px;
}
</style>
