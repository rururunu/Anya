<template>
  <DialogRoot
    :open="true"
    @update:open="
      (open) => {
        if (!open && !saving) emit('close');
      }
    "
  >
    <DialogPortal>
      <DialogOverlay class="theme-editor-overlay" />
      <DialogContent
        class="theme-editor"
        :aria-describedby="undefined"
        @escape-key-down="
          (event) => {
            if (saving) event.preventDefault();
          }
        "
      >
        <header class="editor-header">
          <div>
            <DialogTitle>{{ tr("编辑主题", "Theme editor") }}</DialogTitle>
            <p>
              {{
                tr(
                  "点选预览中的区域，独立调整外观。留空即可跟随默认值。",
                  "Select a preview region to customize it. Empty values inherit defaults.",
                )
              }}
            </p>
          </div>
          <button :disabled="saving" :aria-label="tr('关闭', 'Close')" @click="emit('close')">
            ×
          </button>
        </header>
        <div class="editor-meta">
          <label>
            {{ tr("名称", "Name") }}
            <input
              :value="draft.name"
              @change="setName(($event.target as HTMLInputElement).value)"
            />
          </label>
          <label>
            {{ tr("基础模式", "Base mode") }}
            <select
              :value="draft.mode"
              @change="setMode(($event.target as HTMLSelectElement).value)"
            >
              <option value="light">{{ tr("浅色", "Light") }}</option>
              <option value="dark">{{ tr("深色", "Dark") }}</option>
            </select>
          </label>
          <button :disabled="!past.length || saving" @click="undo">{{ tr("撤销", "Undo") }}</button>
          <button :disabled="!future.length || saving" @click="redo">
            {{ tr("重做", "Redo") }}
          </button>
          <label class="live-toggle">
            <input v-model="live" type="checkbox" :disabled="saving" />
            {{ tr("在应用中预览", "Preview in app") }}
          </label>
        </div>
        <div class="editor-body">
          <div class="preview-column">
            <div
              class="theme-preview"
              :style="previewStyle"
              :aria-label="tr('主题实时预览', 'Live theme preview')"
            >
              <button class="preview-sidebar" @click="select('sidebar')">
                Anya
                <br />
                <br />
                {{ tr("工作区", "Workspaces") }}
                <br />
                <br />
                {{ tr("随问", "Chats") }}
              </button>
              <div class="preview-main">
                <button class="preview-header" @click="select('header')">
                  {{ tr("主题预览", "Theme preview") }}
                </button>
                <div class="preview-conversation" @click.self="select('conversation')">
                  <img
                    v-if="backgroundSrc"
                    class="preview-wallpaper"
                    :src="backgroundSrc"
                    alt=""
                    :style="wallpaperStyle"
                  />
                  <button class="preview-user" @click="select('user')">
                    {{ tr("为我制作一个喜欢的主题", "Create a theme I love") }}
                  </button>
                  <button class="preview-assistant" @click="select('assistant')">
                    {{ tr("每个区域，都可以有自己的风格。", "Give every region its own style.") }}
                  </button>
                  <button class="preview-code" @click="select('code')">
                    <span>TypeScript</span>
                    <code>const theme = 'Anya';</code>
                  </button>
                  <button class="preview-card" @click="select('cards')">
                    {{ tr("工具执行 · 等待确认", "Tool action · Awaiting approval") }}
                  </button>
                  <button class="preview-composer" @click="select('composer')">
                    <span>{{ tr("输入消息…", "Write a message…") }}</span>
                    <span class="preview-send" @click.stop="select('controls')">↑</span>
                  </button>
                </div>
              </div>
            </div>
            <p class="preview-caption">
              {{
                tr(
                  "更改仅在保存后持久化。取消或关闭会移除临时预览。",
                  "Changes persist only when saved. Closing removes the temporary preview.",
                )
              }}
            </p>
            <div v-if="warnings.length" class="contrast-warning" role="status">
              <p>
                {{
                  tr(
                    "以下文字对比度偏低，颜色仍按你的设置保留：",
                    "Low text contrast. Your colors are preserved:",
                  )
                }}
              </p>
              <div v-for="warning in warnings" :key="warning.token">
                {{ warning.label }} · {{ warning.ratio.toFixed(1) }}:1
                <button @click="improveContrast(warning.token, warning.background)">
                  {{ tr("改善文字对比度", "Improve text contrast") }}
                </button>
              </div>
            </div>
          </div>
          <div class="inspector">
            <nav :aria-label="tr('主题区域', 'Theme regions')">
              <button
                v-for="entry in themeRegions"
                :key="entry.id"
                :class="{ selected: regionId === entry.id && !advanced }"
                :aria-pressed="regionId === entry.id && !advanced"
                @click="select(entry.id)"
              >
                {{ isZh ? entry.zh : entry.en }}
              </button>
              <button :class="{ selected: advanced }" @click="openAdvanced">JSON</button>
            </nav>
            <div v-if="advanced" class="advanced-panel">
              <label for="theme-json">
                {{ tr("高级颜色与样式参数", "Advanced color and style tokens") }}
              </label>
              <textarea id="theme-json" v-model="jsonDraft" spellcheck="false" />
              <button :disabled="saving" @click="applyJson">
                {{ tr("应用到预览", "Apply to preview") }}
              </button>
            </div>
            <div v-else class="region-fields">
              <div class="region-heading">
                <strong>{{ isZh ? region.zh : region.en }}</strong>
                <button :disabled="saving" @click="resetRegion">
                  {{ tr("重置此区域", "Reset region") }}
                </button>
              </div>
              <div v-for="entry in region.fields" :key="entry.token" class="token-field">
                <label :for="entry.token">{{ isZh ? entry.zh : entry.en }}</label>
                <div class="token-control">
                  <input
                    v-if="entry.property === 'color'"
                    type="color"
                    :aria-label="`${isZh ? entry.zh : entry.en} ${tr('取色', 'picker')}`"
                    :value="pickerValue(entry)"
                    :disabled="saving"
                    @input="setToken(entry, ($event.target as HTMLInputElement).value)"
                  />
                  <input
                    :id="entry.token"
                    :value="draft.tokens[entry.token] || ''"
                    :placeholder="resolvedValue(entry)"
                    :disabled="saving"
                    @change="setToken(entry, ($event.target as HTMLInputElement).value)"
                  />
                  <button
                    v-if="draft.tokens[entry.token]"
                    :disabled="saving"
                    :title="tr('跟随默认值', 'Use default')"
                    @click="setToken(entry, '')"
                  >
                    ↺
                  </button>
                </div>
                <small>
                  {{
                    draft.tokens[entry.token]
                      ? tr("单独设置", "Custom")
                      : tr("跟随默认值", "Inherited")
                  }}
                </small>
              </div>
              <fieldset v-if="regionId === 'global'" class="background-fields">
                <legend>{{ tr("背景图片", "Wallpaper") }}</legend>
                <input
                  :value="draft.background?.image || ''"
                  :placeholder="tr('本地路径或图片 URL', 'Local path or image URL')"
                  @change="setBackground('image', ($event.target as HTMLInputElement).value)"
                />
                <button @click="pickWallpaper">{{ tr("选择图片", "Choose image") }}</button>
                <label>
                  {{ tr("透明度", "Opacity") }}
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.05"
                    :value="draft.background?.opacity ?? 0.2"
                    @input="
                      setBackground('opacity', Number(($event.target as HTMLInputElement).value))
                    "
                  />
                </label>
                <label>
                  {{ tr("模糊", "Blur") }}
                  <input
                    type="range"
                    min="0"
                    max="30"
                    :value="draft.background?.blur ?? 0"
                    @input="
                      setBackground('blur', Number(($event.target as HTMLInputElement).value))
                    "
                  />
                </label>
                <select
                  :value="draft.background?.fit ?? 'cover'"
                  :aria-label="tr('背景缩放', 'Wallpaper fit')"
                  @change="setBackground('fit', ($event.target as HTMLSelectElement).value)"
                >
                  <option value="cover">cover</option>
                  <option value="contain">contain</option>
                  <option value="fill">fill</option>
                  <option value="center">center</option>
                </select>
              </fieldset>
            </div>
          </div>
        </div>
        <footer class="editor-footer">
          <p v-if="error" role="alert">{{ error }}</p>
          <div>
            <button :disabled="saving" @click="emit('close')">{{ tr("取消", "Cancel") }}</button>
            <button :disabled="saving" @click="save(true)">
              {{ tr("另存为新主题", "Save as new") }}
            </button>
            <button class="primary" :disabled="saving" @click="save(false)">
              {{ saving ? tr("保存中…", "Saving…") : tr("保存并应用", "Save & apply") }}
            </button>
          </div>
        </footer>
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
</template>

<script setup lang="ts">
import { computed, onUnmounted, ref, watch, type CSSProperties } from "vue";
import { DialogRoot, DialogPortal, DialogOverlay, DialogContent, DialogTitle } from "reka-ui";
import { useSettingStore } from "@/stores/setting";
import type { CustomThemeConfig, ThemeBackgroundConfig } from "@/types/setting";
import { themeColorContrast } from "@/services/theme/ThemeService";
import {
  cloneTheme,
  themeRegions,
  validateThemeTokens,
  resolveThemePreview,
  type ThemeField,
} from "@/services/theme/editor";
import { useResolvedBackgroundSrc } from "@/composables/theme/useResolvedBackgroundSrc";

const props = defineProps<{ theme: CustomThemeConfig }>();
const emit = defineEmits<{ close: [] }>();
const store = useSettingStore();
const isZh = computed(() => store.language === "zh-CN");
const tr = (zh: string, en: string) => (isZh.value ? zh : en);
const draft = ref(cloneTheme(props.theme));
const past = ref<CustomThemeConfig[]>([]);
const future = ref<CustomThemeConfig[]>([]);
const regionId = ref("global");
const advanced = ref(false);
const jsonDraft = ref("");
const error = ref("");
const invalidTokens = new Set<string>();
const saving = ref(false);
const live = ref(false);
const region = computed(() => themeRegions.find((entry) => entry.id === regionId.value)!);
const resolved = computed(() => resolveThemePreview(draft.value));
const previewStyle = computed(() => resolved.value as CSSProperties);
const { resolvedSource: backgroundSrc } = useResolvedBackgroundSrc(
  () => draft.value.background?.image,
);
const wallpaperStyle = computed<CSSProperties>(() => ({
  opacity: draft.value.background?.opacity ?? 0.2,
  filter: `blur(${draft.value.background?.blur ?? 0}px)`,
  objectFit:
    draft.value.background?.fit === "fill"
      ? "fill"
      : draft.value.background?.fit === "center"
        ? "none"
        : (draft.value.background?.fit ?? "cover"),
}));

function change(update: (theme: CustomThemeConfig) => void) {
  if (saving.value) return;
  const next = cloneTheme(draft.value);
  update(next);
  if (JSON.stringify(next) === JSON.stringify(draft.value)) return;
  past.value = [...past.value.slice(-49), cloneTheme(draft.value)];
  future.value = [];
  draft.value = next;
  error.value = "";
}
function undo() {
  const previous = past.value.pop();
  if (!previous) return;
  future.value.push(cloneTheme(draft.value));
  draft.value = previous;
  invalidTokens.clear();
  error.value = "";
  if (advanced.value) jsonDraft.value = JSON.stringify(previous.tokens, null, 2);
}
function redo() {
  const next = future.value.pop();
  if (!next) return;
  past.value.push(cloneTheme(draft.value));
  draft.value = next;
  invalidTokens.clear();
  error.value = "";
  if (advanced.value) jsonDraft.value = JSON.stringify(next.tokens, null, 2);
}
function select(id: string) {
  regionId.value = id;
  advanced.value = false;
}
function setName(name: string) {
  change((theme) => {
    theme.name = name;
  });
}
function setMode(mode: string) {
  if (mode === "dark" || mode === "light")
    change((theme) => {
      theme.mode = mode;
    });
}
function resolvedValue(entry: ThemeField) {
  return (
    resolved.value[entry.token] ||
    entry.fallback ||
    (draft.value.mode === "dark" ? "#202020" : "#ffffff")
  );
}
function pickerValue(entry: ThemeField) {
  const value = resolvedValue(entry);
  return /^#[\da-f]{6}$/i.test(value) ? value : "#808080";
}
function setToken(entry: ThemeField, value: string) {
  try {
    const checked = validateThemeTokens({ [entry.token]: value });
    invalidTokens.delete(entry.token);
    change((theme) => {
      if (checked[entry.token]) theme.tokens[entry.token] = checked[entry.token];
      else delete theme.tokens[entry.token];
    });
  } catch {
    invalidTokens.add(entry.token);
    error.value = tr(
      "无效的样式值，请检查颜色或单位。",
      "Invalid style value. Check the color or units.",
    );
  }
}
function resetRegion() {
  change((theme) => {
    for (const entry of region.value.fields) {
      delete theme.tokens[entry.token];
      invalidTokens.delete(entry.token);
    }
    if (regionId.value === "global") delete theme.background;
  });
}
function openAdvanced() {
  jsonDraft.value = JSON.stringify(draft.value.tokens, null, 2);
  advanced.value = true;
}
function applyJson() {
  try {
    const tokens = validateThemeTokens(JSON.parse(jsonDraft.value));
    invalidTokens.clear();
    change((theme) => {
      theme.tokens = tokens;
    });
    error.value = "";
    return true;
  } catch {
    error.value = tr(
      "JSON 或样式值无效，请检查后重试。",
      "Invalid JSON or style values. Check and try again.",
    );
    return false;
  }
}
function setBackground(key: keyof ThemeBackgroundConfig, value: string | number) {
  if (key === "image" && typeof value === "string" && /^data:/i.test(value.trim())) {
    error.value = tr("请使用图片路径或 URL。", "Use an image path or URL.");
    return;
  }
  change((theme) => {
    theme.background = { ...theme.background, [key]: value };
    if (key === "image" && !value) delete theme.background;
  });
}
async function pickWallpaper() {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const path = await open({
      multiple: false,
      filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp", "gif"] }],
    });
    if (typeof path === "string") setBackground("image", path);
  } catch (reason) {
    error.value = String(reason);
  }
}
const warnings = computed(() =>
  [
    {
      token: "--peek-text",
      background: resolved.value["--peek-list-bg"],
      label: tr("正文", "Text"),
    },
    {
      token: "--peek-user-bubble-text",
      background: resolved.value["--peek-user-bubble-bg"],
      label: tr("用户消息", "User message"),
    },
    {
      token: "--peek-composer-text",
      background: resolved.value["--peek-composer-fill"],
      label: tr("输入框", "Composer"),
    },
  ].flatMap((pair) => {
    const ratio = themeColorContrast(resolved.value[pair.token], pair.background);
    return ratio !== null && ratio < 4.5 ? [{ ...pair, ratio }] : [];
  }),
);
function improveContrast(token: string, background: string) {
  const black = themeColorContrast("#000000", background) ?? 0;
  const white = themeColorContrast("#ffffff", background) ?? 0;
  change((theme) => {
    theme.tokens[token] = black >= white ? "#000000" : "#ffffff";
  });
}

// Temporary stylesheet only: never save draft tokens into settings or the boot cache.
const previewSheet = document.createElement("style");
previewSheet.dataset.themeEditorPreview = "true";
document.head.append(previewSheet);
watch(
  [resolved, live],
  () => {
    if (!live.value) {
      previewSheet.textContent = "";
      return;
    }
    try {
      const tokens = validateThemeTokens(resolved.value);
      previewSheet.textContent = `html[data-theme] {${Object.entries(tokens)
        .map(([key, value]) => `${key}:${value}`)
        .join(";")}}`;
    } catch {
      previewSheet.textContent = "";
      error.value = tr(
        "部分样式值无效，请先修正后再预览。",
        "Some style values are invalid. Correct them before previewing.",
      );
    }
  },
  { immediate: true },
);
onUnmounted(() => previewSheet.remove());
async function save(copy: boolean) {
  if (saving.value || (advanced.value && !applyJson())) return;
  if (invalidTokens.size) {
    error.value = tr("请先修正无效的样式值。", "Correct invalid style values before saving.");
    return;
  }
  if (!draft.value.name.trim()) {
    error.value = tr("请输入主题名称。", "Enter a theme name.");
    return;
  }
  saving.value = true;
  try {
    const theme = cloneTheme(draft.value);
    theme.tokens = validateThemeTokens(theme.tokens);
    theme.name = theme.name.trim();
    if (copy) {
      theme.id = `custom-${crypto.randomUUID()}`;
      theme.name += tr(" · 副本", " · Copy");
    }
    theme.updatedAt = Date.now();
    await store.saveCustomTheme(theme);
    previewSheet.remove();
    emit("close");
  } catch (reason) {
    error.value = `${tr("保存失败：", "Save failed: ")}${String(reason)}`;
  } finally {
    saving.value = false;
  }
}
</script>

<style scoped src="./ThemeEditor.css"></style>
