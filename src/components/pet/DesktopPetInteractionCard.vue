<template>
  <section
    class="permission-card pet-workbench-card peek-scrollbar"
    data-tauri-drag-region="false"
    role="dialog"
    :aria-label="headerTitle"
  >
    <!-- 头部：图标、标题、所属工作区、关闭按钮 -->
    <header class="permission-card-header">
      <span
        class="permission-card-icon"
        :class="[interaction.kind, { 'plan-switch': isPlanSwitch }]"
        aria-hidden="true"
      >
        <ListChecks v-if="isPlanSwitch" :size="16" :stroke-width="2" />
        <CircleHelp v-else-if="interaction.kind === 'ask_user'" :size="16" :stroke-width="2" />
        <Shield v-else-if="interaction.kind === 'path_permission'" :size="16" :stroke-width="2" />
        <ShieldAlert v-else :size="16" :stroke-width="2" />
      </span>

      <div class="permission-card-copy">
        <div class="header-main-row">
          <strong class="header-title">{{ headerTitle }}</strong>

          <!-- 多会话待处理项切换导航 -->
          <div
            v-if="totalCount > 1"
            class="interaction-queue-nav"
            role="navigation"
            :aria-label="queueNavLabel"
          >
            <button
              type="button"
              class="queue-nav-btn"
              :title="prevRequestLabel"
              :aria-label="prevRequestLabel"
              @click.stop="$emit('prev')"
            >
              <ChevronLeft :size="11" />
            </button>
            <span class="queue-counter">{{ currentIndex + 1 }}/{{ totalCount }}</span>
            <button
              type="button"
              class="queue-nav-btn"
              :title="nextRequestLabel"
              :aria-label="nextRequestLabel"
              @click.stop="$emit('next')"
            >
              <ChevronRight :size="11" />
            </button>
          </div>

          <span
            v-if="interaction.workspaceName"
            class="workspace-chip"
            :title="`${workspaceLabel}: ${interaction.workspaceName}`"
          >
            <Folder :size="10.5" />
            <span class="workspace-chip-text">{{ interaction.workspaceName }}</span>
          </span>
          <button
            type="button"
            class="dismiss-btn"
            :title="closePanelLabel"
            :aria-label="closePanelLabel"
            @click="$emit('dismiss')"
          >
            <X :size="13" />
          </button>
        </div>
        <span class="header-session" :title="interaction.sessionTitle">
          {{ interaction.sessionTitle }}
        </span>
      </div>
    </header>

    <!-- 1. 用户提问模式 -->
    <template v-if="interaction.kind === 'ask_user'">
      <div class="ask-question-text">
        {{ interaction.question }}
      </div>

      <!-- 选项列表 -->
      <ul
        v-if="interaction.options && interaction.options.length > 0"
        class="permission-options"
        role="listbox"
      >
        <li
          v-for="(opt, index) in interaction.options"
          :key="`${opt.label}-${index}`"
          class="permission-option option-item"
          :class="{
            active: isOptionActive(opt.label),
            selected: interaction.multiSelect && isOptionSelected(opt.label),
          }"
          role="option"
          :aria-selected="
            interaction.multiSelect ? isOptionSelected(opt.label) : isOptionActive(opt.label)
          "
          @mouseenter="hoveredLabel = opt.label"
          @mouseleave="hoveredLabel = null"
          @mousedown.prevent="handleOptionClick(opt.label)"
          @click="handleOptionClick(opt.label)"
        >
          <span
            v-if="interaction.multiSelect"
            class="ask-checkbox"
            :class="{ checked: isOptionSelected(opt.label) }"
            aria-hidden="true"
          >
            <Check v-if="isOptionSelected(opt.label)" :size="11" :stroke-width="2.75" />
          </span>
          <span
            v-else-if="planSwitchOptionIcon(opt.label)"
            class="permission-option-icon"
            aria-hidden="true"
          >
            <component :is="planSwitchOptionIcon(opt.label)" :size="14" :stroke-width="2.25" />
          </span>

          <div class="permission-option-text">
            <span class="permission-option-label">{{ opt.label }}</span>
            <span
              v-if="opt.description && opt.description !== opt.label"
              class="permission-option-desc"
            >
              {{ opt.description }}
            </span>
          </div>
        </li>

        <!-- 多选确认按钮，遵循 AskUserPicker 的 confirm-item 规范 -->
        <li
          v-if="interaction.multiSelect"
          class="permission-option confirm-item"
          :class="{ active: hoveredLabel === '__confirm', disabled: selectedOptions.length === 0 }"
          role="option"
          :aria-disabled="selectedOptions.length === 0"
          @mouseenter="hoveredLabel = '__confirm'"
          @mouseleave="hoveredLabel = null"
          @mousedown.prevent="confirmMultiSelect"
          @click="confirmMultiSelect"
        >
          <span class="ask-leading confirm-mark" aria-hidden="true">
            <Check :size="13" :stroke-width="2.5" />
          </span>
          <div class="permission-option-text">
            <span class="permission-option-label">{{ confirmLabel }}</span>
            <span v-if="selectedOptions.length > 0" class="permission-option-desc">
              {{ selectedCountLabel }}
            </span>
          </div>
        </li>
      </ul>

      <!-- 无选项自由输入备选 -->
      <div v-else class="ask-input-row">
        <input
          v-model="customAnswer"
          type="text"
          class="ask-input-field"
          :placeholder="inputPlaceholder"
          @keydown.enter.prevent="submitCustomAnswer"
        />
        <button
          type="button"
          class="ask-submit-btn"
          :disabled="!customAnswer.trim()"
          @click="submitCustomAnswer"
        >
          {{ sendLabel }}
        </button>
      </div>
    </template>

    <!-- 2. 文件路径授权模式 -->
    <template v-else-if="interaction.kind === 'path_permission'">
      <div v-if="interaction.path" class="permission-path" :title="interaction.path">
        {{ interaction.path }}
      </div>

      <ul class="permission-options" role="listbox">
        <li
          v-for="opt in pathOptions"
          :key="opt.decision"
          class="permission-option"
          :class="{
            active: hoveredLabel === opt.decision,
            danger: opt.decision === 'deny',
          }"
          role="option"
          @mouseenter="hoveredLabel = opt.decision"
          @mouseleave="hoveredLabel = null"
          @mousedown.prevent="$emit('submit-path', opt.decision)"
          @click="$emit('submit-path', opt.decision)"
        >
          <span class="permission-option-icon" aria-hidden="true">
            <component :is="opt.icon" :size="14" :stroke-width="2.25" />
          </span>
          <div class="permission-option-text">
            <span class="permission-option-label">{{ opt.label }}</span>
            <span class="permission-option-desc">{{ opt.description }}</span>
          </div>
        </li>
      </ul>
    </template>

    <!-- 3. 工具执行审批模式 -->
    <template v-else-if="interaction.kind === 'tool_approval'">
      <div v-if="interaction.diffSummary" class="permission-path" :title="interaction.diffSummary">
        {{ interaction.diffSummary }}
      </div>

      <ul class="permission-options" role="listbox">
        <li
          v-for="opt in toolOptions"
          :key="opt.decision"
          class="permission-option"
          :class="{
            active: hoveredLabel === opt.decision,
            danger: opt.decision === 'deny',
          }"
          role="option"
          @mouseenter="hoveredLabel = opt.decision"
          @mouseleave="hoveredLabel = null"
          @mousedown.prevent="$emit('submit-tool', opt.decision)"
          @click="$emit('submit-tool', opt.decision)"
        >
          <span class="permission-option-icon" aria-hidden="true">
            <component :is="opt.icon" :size="14" :stroke-width="2.25" />
          </span>
          <div class="permission-option-text">
            <span class="permission-option-label">{{ opt.label }}</span>
            <span class="permission-option-desc">{{ opt.description }}</span>
          </div>
        </li>
      </ul>
    </template>
  </section>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { storeToRefs } from "pinia";
import {
  Ban,
  Check,
  ChevronLeft,
  ChevronRight,
  CircleHelp,
  Folder,
  ListChecks,
  Shield,
  ShieldAlert,
  ShieldCheck,
  Sparkle,
  X,
} from "@lucide/vue";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";
import type { PetInteraction } from "@/composables/useDesktopPetInteractions";
import type { PathPermissionDecision, ToolApprovalDecision } from "@/types/chat";
import {
  isPlanSwitchQuestion,
  PLAN_SWITCH_ACCEPT_LABEL,
  PLAN_SWITCH_DECLINE_LABEL,
} from "@/services/chat/askUserAnswer";

const props = withDefaults(
  defineProps<{
    interaction: PetInteraction;
    totalCount?: number;
    currentIndex?: number;
  }>(),
  {
    totalCount: 1,
    currentIndex: 0,
  },
);

const emit = defineEmits<{
  "submit-ask": [answer: string];
  "submit-path": [decision: PathPermissionDecision];
  "submit-tool": [decision: ToolApprovalDecision];
  prev: [];
  next: [];
  dismiss: [];
}>();

const settingStore = useSettingStore();
const { language } = storeToRefs(settingStore);

const selectedOptions = ref<string[]>([]);
const customAnswer = ref("");
const hoveredLabel = ref<string | null>(null);

const queueNavLabel = computed(() => tr(language.value, "queueNav"));
const prevRequestLabel = computed(() => tr(language.value, "prevRequest"));
const nextRequestLabel = computed(() => tr(language.value, "nextRequest"));
const closePanelLabel = computed(() => tr(language.value, "closePanel"));
const inputPlaceholder = computed(() => tr(language.value, "typeYourAnswer"));
const sendLabel = computed(() => tr(language.value, "send"));
const workspaceLabel = computed(() => tr(language.value, "workspace"));

const headerTitle = computed(() => {
  if (props.interaction.kind === "ask_user") {
    return props.interaction.header || tr(language.value, "select");
  }
  if (props.interaction.kind === "path_permission") {
    return tr(language.value, "permissionRequest");
  }
  return props.interaction.title || tr(language.value, "toolApprovalTitle");
});

const confirmLabel = computed(() => tr(language.value, "confirmSelection"));

const isPlanSwitch = computed(
  () =>
    props.interaction.kind === "ask_user" &&
    isPlanSwitchQuestion({
      kind: props.interaction.questionKind,
      header: props.interaction.header,
    }),
);

function planSwitchOptionIcon(label: string) {
  if (!isPlanSwitch.value) return undefined;
  if (label === PLAN_SWITCH_ACCEPT_LABEL) return ListChecks;
  if (label === PLAN_SWITCH_DECLINE_LABEL) return Sparkle;
  return undefined;
}

const selectedCountLabel = computed(() =>
  tr(language.value, "askSelectedCount", { count: selectedOptions.value.length }),
);

const pathOptions = computed(() => [
  {
    decision: "allow_once" as const,
    label: tr(language.value, "allowOnce"),
    description: tr(language.value, "allowOnceDesc"),
    icon: Check,
  },
  {
    decision: "allow_always" as const,
    label: tr(language.value, "allowAlways"),
    description: tr(language.value, "allowAlwaysDesc"),
    icon: ShieldCheck,
  },
  {
    decision: "deny" as const,
    label: tr(language.value, "deny"),
    description: tr(language.value, "denyDesc"),
    icon: Ban,
  },
]);

const toolOptions = computed(() => [
  {
    decision: "allow_once" as const,
    label: tr(language.value, "allowOnce"),
    description: tr(language.value, "allowOnceDesc"),
    icon: Check,
  },
  {
    decision: "allow_session" as const,
    label: tr(language.value, "allowSession"),
    description: tr(language.value, "allowSessionDesc"),
    icon: Shield,
  },
  {
    decision: "deny" as const,
    label: tr(language.value, "deny"),
    description: tr(language.value, "denyDesc"),
    icon: Ban,
  },
]);

function isOptionSelected(label: string): boolean {
  return selectedOptions.value.includes(label);
}

function isOptionActive(label: string): boolean {
  return hoveredLabel.value === label;
}

function handleOptionClick(label: string) {
  if (props.interaction.kind !== "ask_user") return;
  if (props.interaction.multiSelect) {
    if (isOptionSelected(label)) {
      selectedOptions.value = selectedOptions.value.filter((l) => l !== label);
    } else {
      selectedOptions.value.push(label);
    }
  } else {
    emit("submit-ask", label);
  }
}

function confirmMultiSelect() {
  if (selectedOptions.value.length === 0) return;
  emit("submit-ask", selectedOptions.value.join(", "));
}

function submitCustomAnswer() {
  const text = customAnswer.value.trim();
  if (!text) return;
  emit("submit-ask", text);
  customAnswer.value = "";
}
</script>

<style scoped>
/* 遵循 Anya 工作台 PermissionCard / AskUserPicker 统一设计系统 */
.pet-workbench-card {
  --permission-row-height: 32px;
  width: 356px;
  max-height: 236px;
  margin: 0 auto;
  padding: 10px 13px 10px;
  display: flex;
  flex-direction: column;
  gap: 7px;
  box-sizing: border-box;
  background: color-mix(in srgb, var(--peek-surface, #ffffff) 94%, transparent);
  border: 1px solid var(--peek-border, rgba(0, 0, 0, 0.16));
  border-radius: 14px;
  box-shadow:
    0 16px 36px rgba(0, 0, 0, 0.28),
    0 2px 8px rgba(0, 0, 0, 0.1);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
  color: var(--peek-text, #242424);
  user-select: none;
  pointer-events: auto;
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
  animation: card-pop 0.24s cubic-bezier(0.34, 1.4, 0.64, 1);
}

@keyframes card-pop {
  from {
    opacity: 0;
    transform: translateY(8px) scale(0.97);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

/* 头部 Header */
.permission-card-header {
  display: flex;
  gap: 8px;
  align-items: flex-start;
  flex-shrink: 0;
}

.permission-card-icon {
  flex: none;
  width: 26px;
  height: 26px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 7px;
  background: color-mix(in srgb, var(--peek-accent, #111111) 12%, transparent);
  color: var(--peek-accent, #111111);
}

.permission-card-icon.path_permission,
.permission-card-icon.tool_approval {
  background: color-mix(in srgb, var(--peek-warning, #8a6500) 14%, transparent);
  color: var(--peek-warning, #8a6500);
}

.permission-card-copy {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}

.header-main-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.header-title {
  font-size: 12.5px;
  font-weight: 650;
  line-height: 16px;
  color: var(--peek-text, #242424);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex: 1;
}

.interaction-queue-nav {
  display: inline-flex;
  align-items: center;
  gap: 1px;
  padding: 1px 3px;
  background: color-mix(in srgb, var(--peek-text, #242424) 8%, transparent);
  border: 1px solid var(--peek-border, rgba(0, 0, 0, 0.12));
  border-radius: 9999px;
  font-size: 10px;
  color: var(--peek-muted, #5a5a5a);
  flex-shrink: 0;
}

.queue-nav-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  padding: 0;
  border: none;
  background: transparent;
  color: var(--peek-muted, #5a5a5a);
  border-radius: 50%;
  cursor: pointer;
  transition: all 120ms ease;
}

.queue-nav-btn:hover {
  background: color-mix(in srgb, var(--peek-text, #242424) 12%, transparent);
  color: var(--peek-text, #242424);
}

.queue-counter {
  font-size: 10px;
  font-weight: 600;
  line-height: 1;
  padding: 0 2px;
}

.workspace-chip {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 10px;
  line-height: 14px;
  padding: 1px 5px;
  border-radius: 4px;
  background: color-mix(in srgb, var(--peek-text, #242424) 7%, transparent);
  color: var(--peek-muted, #5a5a5a);
  max-width: 85px;
  flex-shrink: 0;
}

.workspace-chip-text {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.dismiss-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border-radius: 4px;
  border: none;
  background: transparent;
  color: var(--peek-faint, #767676);
  cursor: pointer;
  padding: 0;
  flex-shrink: 0;
  transition:
    background 120ms ease,
    color 120ms ease;
}

.dismiss-btn:hover {
  background: color-mix(in srgb, var(--peek-text, #242424) 8%, transparent);
  color: var(--peek-text, #242424);
}

.header-session {
  font-size: 11px;
  line-height: 15px;
  color: var(--peek-muted, #5a5a5a);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 提问文本 */
.ask-question-text {
  font-size: 12.5px;
  font-weight: 550;
  line-height: 1.45;
  color: var(--peek-text, #242424);
  overflow-wrap: anywhere;
  word-break: break-word;
  flex-shrink: 0;
  padding: 1px 2px 2px;
}

/* 路径展示框（与工作台 permission-path 风格完全一致） */
.permission-path {
  margin: 0;
  padding: 5px 9px;
  border: 1px solid color-mix(in srgb, var(--peek-text, #242424) 10%, transparent);
  border-radius: 7px;
  background: color-mix(in srgb, var(--peek-input-bg, #ffffff) 88%, var(--peek-surface, #ffffff));
  color: var(--peek-text, #242424);
  font-family: var(
    --font-mono,
    ui-monospace,
    SFMono-Regular,
    "JetBrains Mono",
    Menlo,
    Consolas,
    monospace
  );
  font-size: 11px;
  line-height: 1.4;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  direction: rtl;
  text-align: left;
  flex-shrink: 0;
}

/* 选项列表（与工作台 permission-options 风格完全一致） */
.permission-options {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 3.5px;
}

.permission-option {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: var(--permission-row-height);
  margin: 0;
  padding: 5px 10px;
  border: 1px solid color-mix(in srgb, var(--peek-text, #242424) 8%, transparent);
  border-radius: 7px;
  background: color-mix(in srgb, var(--peek-surface, #ffffff) 70%, transparent);
  color: var(--peek-text, #242424);
  cursor: pointer;
  transition:
    background 120ms ease,
    border-color 120ms ease;
  text-align: left;
}

.permission-option:hover,
.permission-option.active {
  border-color: color-mix(in srgb, var(--peek-accent, #111111) 24%, transparent);
  background: color-mix(in srgb, var(--peek-accent, #111111) 9%, var(--peek-surface, #ffffff));
}

.permission-option.selected {
  border-color: color-mix(in srgb, var(--peek-accent, #111111) 32%, transparent);
  background: color-mix(in srgb, var(--peek-accent, #111111) 14%, var(--peek-surface, #ffffff));
}

.permission-option.danger:hover,
.permission-option.danger.active {
  border-color: color-mix(in srgb, var(--peek-danger, #c42b1c) 28%, transparent);
  background: color-mix(in srgb, var(--peek-danger, #c42b1c) 8%, var(--peek-surface, #ffffff));
}

.permission-option.danger:hover .permission-option-label,
.permission-option.danger.active .permission-option-label {
  color: var(--peek-danger, #c42b1c);
}

.permission-option-icon {
  flex: none;
  width: 16px;
  height: 16px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--peek-muted, #5a5a5a);
}

.permission-option.active .permission-option-icon {
  color: var(--peek-accent, #111111);
}

.permission-option.danger.active .permission-option-icon {
  color: var(--peek-danger, #c42b1c);
}

.permission-option-text {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}

.permission-option-label {
  font-size: 12px;
  font-weight: 600;
  line-height: 16px;
  color: var(--peek-text, #242424);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.permission-option-desc {
  font-size: 10.5px;
  line-height: 14px;
  color: var(--peek-muted, #5a5a5a);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 多选复选框（与工作台 ask-checkbox 完全一致） */
.ask-checkbox {
  flex: none;
  box-sizing: border-box;
  width: 15px;
  height: 15px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1.5px solid color-mix(in srgb, var(--peek-text, #242424) 28%, transparent);
  border-radius: 3px;
  background: transparent;
  color: transparent;
  transition: all 120ms ease;
}

.ask-checkbox.checked {
  border-color: var(--peek-accent, #111111);
  background: var(--peek-accent, #111111);
  color: var(--peek-surface, #ffffff);
}

/* 多选确认项（与工作台 confirm-item 一致） */
.ask-leading {
  flex: none;
  width: 15px;
  height: 15px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--peek-muted, #5a5a5a);
}

.ask-leading.confirm-mark {
  color: var(--peek-accent, #111111);
}

.confirm-item.disabled {
  opacity: 0.45;
  pointer-events: none;
}

.confirm-item .permission-option-label {
  color: var(--peek-accent, #111111);
}

/* 自由输入行 */
.ask-input-row {
  display: flex;
  gap: 6px;
  margin-top: 2px;
}

.ask-input-field {
  flex: 1;
  height: 28px;
  padding: 0 8px;
  border-radius: 6px;
  border: 1px solid var(--peek-border, rgba(0, 0, 0, 0.16));
  background: var(--peek-input-bg, #ffffff);
  color: var(--peek-text, #242424);
  font-size: 11.5px;
  outline: none;
  box-sizing: border-box;
}

.ask-input-field:focus {
  border-color: var(--peek-accent, #111111);
}

.ask-submit-btn {
  height: 28px;
  padding: 0 10px;
  border-radius: 6px;
  border: none;
  background: var(--peek-accent, #111111);
  color: var(--peek-surface, #ffffff);
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
  transition: opacity 120ms ease;
}

.ask-submit-btn:hover:not(:disabled) {
  opacity: 0.9;
}

.ask-submit-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
</style>
