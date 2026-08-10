<template>
  <div
    ref="chatInputShellRef"
    class="chat-input-shell"
    :style="chipPickerStyle"
    :class="{
      'overlay-pickers': props.overlayPickers,
      'workbench-composer': props.appearance === 'workbench',
      'overlay-composer': props.appearance === 'overlay',
      'picker-open': interactivePickerOpen && !chipPickerOpen,
      'file-suggestion-open': showFileSuggestions,
      'hash-suggestion-open': showHashSuggestions,
      'command-suggestion-open': showCommandSuggestions,
      'attach-panel-open': attachPanelOpen,
      'chip-picker-open': chipPickerOpen,
      'interaction-request-open': interactionRequestOpen,
    }"
  >
    <Transition :css="false" mode="out-in" @enter="gsapPickerEnter" @leave="gsapPickerLeave">
      <WorkspacePickerPanel
        v-if="workspacePickerOpen"
        key="workspace-picker"
        :title="tr(language, 'chatInput.workspacePanelTitle')"
        :quick-select-only="workspaceQuickSelectOnly"
        :workspaces="workspaces"
        :current-workspace="currentWorkspace"
        :selected-index="selectedIndex"
        :saving="workspaceSaving"
        :error="workspaceError"
        :new-workspace-label="tr(language, 'chatInput.newWorkspace')"
        :no-previous-workspaces-label="tr(language, 'chatInput.noPreviousWorkspaces')"
        @add-new="addWorkspaceFromFolder"
        @select="chooseWorkspace"
      />

      <AttachResourcePanel
        v-else-if="attachPanelOpen"
        key="attach-resource-panel"
        :tab="attachPanelTab"
        :loading="hashCatalogLoading"
        :picking-files="attachPickingFiles"
        :skills="attachSkillItems"
        :mcp-servers="attachMcpItems"
        :selected-index="selectedIndex"
        :workspace-files="workspaceFiles"
        :files-loading="workspaceFilesLoading"
        :has-workspace="Boolean(currentWorkspace)"
        :ariaLabel="tr(language, 'chatInput.attachPanelTitle')"
        :skills-label="tr(language, 'chatInput.attachSkills')"
        :mcp-label="tr(language, 'chatInput.attachMcp')"
        :files-label="tr(language, 'chatInput.attachFiles')"
        :pick-files-label="tr(language, 'chatInput.attachPickFiles')"
        :files-loading-text="tr(language, 'chatInput.attachFilesLoading')"
        :no-workspace-text="tr(language, 'chatInput.attachNoWorkspace')"
        :empty-files-text="tr(language, 'chatInput.attachEmptyFiles')"
        :insert-file-title="tr(language, 'chatInput.attachInsertFile')"
        :insert-folder-title="tr(language, 'chatInput.attachInsertFolder')"
        :loading-text="tr(language, 'chatInput.attachLoading')"
        :empty-skills-text="tr(language, 'chatInput.attachEmptySkills')"
        :empty-mcp-text="tr(language, 'chatInput.attachEmptyMcp')"
        :expand-more-label="tr(language, 'chatInput.attachExpandMore')"
        :collapse-label="tr(language, 'chatInput.attachCollapse')"
        @tab-change="onAttachPanelTabChange"
        @pick-files="pickAttachFiles"
        @select-file="selectAttachWorkspaceFile"
        @hover="onAttachPanelHover"
        @select="selectAttachResource"
        @visible-count="onAttachVisibleCount"
      />

      <AskUserPicker
        v-else-if="showAskUserPicker"
        key="ask-user-list"
        :header="activeAskQuestion?.header"
        :question="activeAskQuestion?.question"
        :question-index="askQuestionIndex"
        :question-count="askQuestionCount"
        :options="activeAskOptions"
        :multi-select="activeAskQuestion?.multiSelect"
        :confirm-row-index="askConfirmRowIndex"
        :confirm-label="tr(language, 'confirmSelection')"
        :selected-count="askSelectedCount"
        :selected-count-label="tr(language, 'askSelectedCount', { count: askSelectedCount })"
        :selected-index="selectedIndex"
        :ariaLabel="tr(language, 'select')"
        :is-option-selected="isAskOptionSelected"
        @hover="selectedIndex = $event"
        @select="selectAskOption"
        @confirm="confirmAskSelection"
      />

      <PathPermissionPicker
        v-else-if="showPathPermissionPicker"
        key="path-permission-list"
        :header="pathPermissionHeader"
        :question="pathPermissionQuestion"
        :path="props.pathPermission?.path"
        :options="pathPermissionOptions"
        :selected-index="selectedIndex"
        :ariaLabel="tr(language, 'permissionRequest')"
        @hover="selectedIndex = $event"
        @select="selectPathPermission"
      />

      <ToolApprovalPicker
        v-else-if="showToolApprovalPicker"
        key="tool-approval-list"
        :header="toolApprovalHeader"
        :options="toolApprovalOptions"
        :selected-index="selectedIndex"
        :ariaLabel="tr(language, 'toolApprovalTitle')"
        @hover="selectedIndex = $event"
        @select="selectToolApproval"
      />

      <HistoryPicker
        v-else-if="showHistoryPicker"
        key="history-list"
        :items="historyItems"
        :selected-index="selectedIndex"
        :ariaLabel="tr(language, 'chatHistory')"
        :format-time="formatTime"
        @hover="selectedIndex = $event"
        @select="selectHistorySession"
      />

      <ModelPicker
        v-else-if="showModelPicker"
        key="model-list"
        :models="modelPickerModels"
        :selected-model-id="chatModel"
        :selected-provider="chatModelProvider"
        :selected-index="selectedIndex"
        :loading="chatModelStore.loading"
        :refreshing="chatModelStore.refreshing"
        :error="chatModelStore.error"
        :loading-text="modelStatusText.loading"
        :empty-text="modelPickerEmptyText"
        :refresh-text="tr(language, 'refreshModels')"
        :ariaLabel="tr(language, 'chooseModel')"
        @hover="selectedIndex = $event"
        @select="selectModel"
        @refresh="refreshModelList"
      />

      <OptionPicker
        v-else-if="showChatModePicker"
        key="chat-mode-list"
        compact
        :options="chatModePickerOptions"
        :selected-id="effectiveChatMode"
        :selected-index="selectedIndex"
        :ariaLabel="tr(language, 'chooseChatMode')"
        @hover="selectedIndex = $event"
        @select="selectChatMode"
      />

      <OptionPicker
        v-else-if="showThinkingTierList"
        key="thinking-tier-list"
        :options="thinkingTierPickerOptions"
        :selected-id="chatModel"
        :selected-index="selectedIndex"
        :ariaLabel="tr(language, 'chooseThinkingTier')"
        @hover="selectedIndex = $event"
        @select="selectThinkingTier"
      />

      <OptionPicker
        v-else-if="showApprovalPicker"
        key="approval-mode-list"
        :options="approvalPickerOptions"
        :selected-id="sessionToolApprovalMode"
        :selected-index="selectedIndex"
        :ariaLabel="tr(language, 'toolApprovalMode')"
        @hover="selectedIndex = $event"
        @select="selectApprovalMode"
      />

      <FileMentionPicker
        v-else-if="showFileSuggestions"
        key="file-suggestions"
        :loading="workspaceFilesLoading"
        :suggestions="fileSuggestions"
        :selected-index="selectedIndex"
        :loading-text="tr(language, 'loadingFiles')"
        :empty-text="tr(language, 'noMatchingFiles')"
        :ariaLabel="tr(language, 'workspace')"
        @hover="selectedIndex = $event"
        @select="selectWorkspaceFile"
      />

      <HashMentionPicker
        v-else-if="showHashSuggestions"
        key="hash-suggestions"
        :loading="hashCatalogLoading"
        :items="hashSuggestions"
        :selected-index="selectedIndex"
        :loading-text="tr(language, 'loadingHashMentions')"
        :empty-text="tr(language, 'noMatchingHashMentions')"
        :ariaLabel="tr(language, 'hashMentions')"
        :skill-label="tr(language, 'hashSkill')"
        :mcp-label="tr(language, 'hashMcp')"
        @hover="selectedIndex = $event"
        @select="selectHashMention"
      />

      <CommandSuggestions
        v-else-if="showCommandSuggestions"
        key="command-list"
        :commands="filteredCommands"
        :selected-index="selectedIndex"
        :appearance="props.appearance"
        :ariaLabel="tr(language, 'commandSuggestions')"
        @hover="selectedIndex = $event"
        @select="executeCommand"
      />
    </Transition>

    <div
      class="input-bar"
      :class="{
        'has-images': attachedImages.length > 0 || attachedFiles.length > 0,
        'drag-over': fileDragOver,
      }"
      data-tauri-drag-region="false"
      @mousedown="onInputBarMouseDown"
      @dragover.prevent="onFileDragOver"
      @dragleave="onFileDragLeave"
      @drop.prevent="onFileDrop"
    >
      <div
        v-if="attachedImages.length"
        class="input-images peek-scrollbar"
        data-tauri-drag-region="false"
      >
        <div
          v-for="(img, idx) in attachedImages"
          :key="idx"
          class="image-thumb-container"
          data-tauri-drag-region="false"
        >
          <img
            :src="img"
            class="image-thumb"
            draggable="false"
            data-no-drag
            @mousedown.stop
            @click.stop="previewImage(img)"
          />
          <button
            type="button"
            class="image-remove-btn"
            title="Remove image"
            @click="removeAttachedImage(idx)"
          >
            <X :size="10" />
          </button>
        </div>
      </div>

      <div
        v-if="attachedFiles.length"
        class="input-files peek-scrollbar"
        data-tauri-drag-region="false"
      >
        <div
          v-for="(file, idx) in attachedFiles"
          :key="`${file.path}-${idx}`"
          class="file-chip"
          :class="{ skipped: Boolean(file.skippedReason) }"
          data-tauri-drag-region="false"
          :title="file.skippedReason ? `${file.path} (${file.skippedReason})` : file.path"
        >
          <img
            v-if="fileIconForPath(file.path)"
            class="file-chip-icon-img"
            :src="fileIconForPath(file.path) || ''"
            alt=""
          />
          <File v-else :size="12" :stroke-width="1.75" class="file-chip-icon" aria-hidden="true" />
          <span class="file-chip-name">{{ file.name }}</span>
          <button
            type="button"
            class="file-chip-remove"
            :aria-label="tr(language, 'close')"
            @click.stop="removeAttachedFile(idx)"
          >
            <X :size="11" :stroke-width="2" />
          </button>
        </div>
      </div>

      <div
        class="input-content peek-scrollbar"
        :class="{
          'has-chips': hasComposerChips,
          'has-leading': composerSegments.length > 0,
          'is-multiline': composerInputMultiline,
        }"
        data-tauri-drag-region="false"
      >
        <template v-for="(seg, segIdx) in composerSegments" :key="`L-${segIdx}-${seg.kind}`">
          <span
            v-if="seg.kind === 'selection'"
            class="selection-tag"
            data-tauri-drag-region="false"
            :title="`Selected ${seg.lines} lines`"
          >
            <span>select-{{ seg.lines }}</span>
          </span>
        </template>

        <ComposerEditable
          ref="composerRef"
          v-model="message"
          :placeholder="inputPlaceholder"
          :multiline="composerInputMultiline"
          :empty="!message"
          :readonly="inputLockedForTyping"
          :aria-expanded="showSuggestions || interactivePickerOpen"
          :mcp-servers="settingStore.mcpServers ?? []"
          :skills="composerSkillMeta"
          :file-catalog="workspaceFiles"
          class="peek-scrollbar"
          @caret-change="onComposerCaretChange"
          @input="onComposerInput"
          @keydown="handleKeydown"
          @paste="handlePaste"
        />
      </div>

      <div class="input-footer">
        <div class="input-footer-primary">
          <button
            v-if="props.appearance !== 'overlay'"
            ref="attachButtonRef"
            type="button"
            class="attach-trigger-btn"
            data-picker-trigger
            data-tauri-drag-region="false"
            :class="{ open: attachPanelOpen }"
            :title="tr(language, 'chatInput.attachResources')"
            :aria-label="tr(language, 'chatInput.attachResources')"
            :aria-expanded="attachPanelOpen"
            aria-haspopup="dialog"
            @mousedown.stop
            @click.stop="toggleAttachPanel"
          >
            <Plus :size="15" :stroke-width="2.25" />
          </button>

          <div
            v-if="props.showWorkspaceButton"
            class="workspace-control"
            :class="{ active: Boolean(currentWorkspace), open: workspacePickerOpen }"
          >
            <button
              type="button"
              class="workspace-btn"
              data-picker-trigger
              data-tauri-drag-region="false"
              :title="workspaceTooltip"
              @click.stop="toggleWorkspacePicker"
            >
              <Folder :size="14" />
              <span v-if="currentWorkspace" class="workspace-name">
                {{ currentWorkspace.name }}
              </span>
              <span v-else class="workspace-name">{{ tr(language, "workspace") }}</span>
            </button>
            <button
              v-if="currentWorkspace"
              type="button"
              class="workspace-exit-btn"
              data-tauri-drag-region="false"
              :title="tr(language, 'chatInput.exitWorkspace')"
              :aria-label="tr(language, 'chatInput.exitWorkspace')"
              @click.stop="exitCurrentWorkspace"
            >
              <X :size="13" />
            </button>
          </div>

          <div class="model-picker">
            <button
              ref="chatModeButtonRef"
              type="button"
              class="model-badge footer-chip"
              data-picker-trigger
              data-tauri-drag-region="false"
              :class="{ open: chatModePickerOpen }"
              :title="chatModeBadgeTitle"
              :aria-label="chatModeBadgeTitle"
              aria-haspopup="listbox"
              :aria-expanded="chatModePickerOpen"
              @mousedown.stop
              @click.stop="toggleChatModeMenu"
            >
              <component :is="chatModeIcon" :size="13" class="footer-chip-icon" />
              <span class="model-name">{{ chatModeLabel }}</span>
              <ChevronDown :size="11" class="model-chevron" />
            </button>
          </div>

          <div class="model-picker">
            <button
              ref="modelButtonRef"
              type="button"
              class="model-badge footer-chip"
              data-picker-trigger
              data-tauri-drag-region="false"
              :class="{ open: modelPickerOpen, confirm: modelChipConfirm }"
              :title="modelBadgeTitle"
              :aria-label="modelBadgeTitle"
              aria-haspopup="listbox"
              :aria-expanded="modelPickerOpen"
              @mousedown.stop
              @click.stop="toggleModelMenu"
            >
              <span class="footer-chip-icon-slot" aria-hidden="true">
                <component
                  :is="currentModelProviderIcon"
                  v-if="currentModelProviderIcon"
                  :size="13"
                  class="footer-chip-icon"
                />
              </span>
              <span class="model-name" :key="currentModelDisplayName">
                {{ currentModelDisplayName }}
              </span>
              <ChevronDown :size="11" class="model-chevron" />
            </button>
          </div>

          <div
            class="model-picker thinking-tier-slot"
            :class="{ dormant: !showThinkingTierPicker }"
            :aria-hidden="!showThinkingTierPicker"
          >
            <button
              ref="thinkingTierButtonRef"
              type="button"
              class="model-badge footer-chip"
              data-picker-trigger
              data-tauri-drag-region="false"
              :class="{ open: thinkingTierPickerOpen }"
              :title="thinkingTierBadgeTitle"
              :aria-label="thinkingTierBadgeTitle"
              aria-haspopup="listbox"
              :aria-expanded="thinkingTierPickerOpen"
              :tabindex="showThinkingTierPicker ? 0 : -1"
              :disabled="!showThinkingTierPicker"
              @mousedown.stop
              @click.stop="toggleThinkingTierMenu"
            >
              <Brain :size="13" class="footer-chip-icon" />
              <span class="model-name">{{ currentThinkingTierLabel || "—" }}</span>
              <ChevronDown :size="11" class="model-chevron" />
            </button>
          </div>

          <div
            class="model-picker approval-slot"
            :class="{ dormant: effectiveChatMode === 'ask' }"
            :aria-hidden="effectiveChatMode === 'ask'"
          >
            <button
              ref="approvalButtonRef"
              type="button"
              class="model-badge footer-chip"
              data-picker-trigger
              data-tauri-drag-region="false"
              :class="{ open: approvalPickerOpen }"
              :title="approvalBadgeTitle"
              :aria-label="approvalBadgeTitle"
              aria-haspopup="listbox"
              :aria-expanded="approvalPickerOpen"
              :tabindex="effectiveChatMode === 'ask' ? -1 : 0"
              :disabled="effectiveChatMode === 'ask'"
              @mousedown.stop
              @click.stop="toggleApprovalMenu"
            >
              <component
                :is="getApprovalIcon(sessionToolApprovalMode)"
                :size="13"
                class="footer-chip-icon"
              />
              <span class="model-name">{{ approvalModeLabel }}</span>
              <ChevronDown :size="11" class="model-chevron" />
            </button>
          </div>
        </div>

        <div class="input-footer-actions">
          <slot name="actions" />

          <span
            v-if="conversationTokenCount"
            class="conversation-token-count"
            :title="conversationTokenTitle"
          >
            ≈ {{ formatTokenCount(conversationTokenCount, language) }} tokens
          </span>

          <ContextUsageRing
            v-if="contextUsage.contextWindowTokens > 0"
            :ratio="contextUsage.usageRatio"
            :tooltip="contextUsageTooltip"
          />

          <button
            v-if="sending && canSend"
            type="button"
            class="send-btn pause"
            data-tauri-drag-region="false"
            :aria-label="tr(language, 'pause')"
            :disabled="interactivePickerOpen"
            @click="emit('pause')"
          >
            <svg viewBox="0 0 16 16" fill="none" aria-hidden="true">
              <path
                d="M5.25 4.5V11.5M10.75 4.5V11.5"
                stroke="currentColor"
                stroke-width="1.6"
                stroke-linecap="round"
              />
            </svg>
          </button>

          <button
            type="button"
            class="send-btn"
            data-tauri-drag-region="false"
            :class="showPauseIcon ? 'pause' : canSend ? 'active' : ''"
            :aria-label="tr(language, showPauseIcon ? 'pause' : 'send')"
            :title="sending && canSend ? tr(language, 'attachInjectHint') : undefined"
            :disabled="interactivePickerOpen"
            @click="submit"
          >
            <svg v-if="!showPauseIcon" viewBox="0 0 16 16" fill="none" aria-hidden="true">
              <path
                d="M8 2.25L9.35 6.15L13.25 7.5L9.35 8.85L8 12.75L6.65 8.85L2.75 7.5L6.65 6.15L8 2.25Z"
                stroke="currentColor"
                stroke-width="1.35"
                stroke-linejoin="round"
              />
            </svg>
            <svg v-else viewBox="0 0 16 16" fill="none" aria-hidden="true">
              <path
                d="M5.25 4.5V11.5M10.75 4.5V11.5"
                stroke="currentColor"
                stroke-width="1.6"
                stroke-linecap="round"
              />
            </svg>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useDebounceFn, useEventListener } from "@vueuse/core";
import { storeToRefs } from "pinia";
import { gsapPickerEnter, gsapPickerLeave } from "@/services/motion/gsapPresets";
import {
  ChevronDown,
  File,
  Folder,
  X,
  Zap,
  Bot,
  MessageCircle,
  Brain,
  ShieldQuestion,
  Shield,
  ShieldCheck,
  Unlock,
  Plus,
  ListChecks,
  Check,
  Ban,
} from "@lucide/vue";
import HistoryPicker from "./input/HistoryPicker.vue";
import ModelPicker from "./input/ModelPicker.vue";
import OptionPicker from "./input/OptionPicker.vue";
import AskUserPicker from "./input/AskUserPicker.vue";
import PathPermissionPicker from "./input/PathPermissionPicker.vue";
import ToolApprovalPicker from "./input/ToolApprovalPicker.vue";
import FileMentionPicker from "./input/FileMentionPicker.vue";
import HashMentionPicker from "./input/HashMentionPicker.vue";
import CommandSuggestions from "./input/CommandSuggestions.vue";
import WorkspacePickerPanel from "./input/WorkspacePickerPanel.vue";
import AttachResourcePanel from "./input/AttachResourcePanel.vue";
import ContextUsageRing from "./ContextUsageRing.vue";
import ComposerEditable from "./ComposerEditable.vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { executeSlashCommand, fetchEnvironmentContext, slashCommands } from "@/commands/slash";
import { listSkills } from "@/commands/skills";
import { peekInstallIcon, warmInstallIcons } from "@/services/iconCache";
import { getContextUsage, getPlanMode, setOverlayPopupOpen, setPlanMode } from "@/services/ipc";
import { createLogger } from "@/services/logger";
import { tr } from "@/services/i18n";
import {
  formatAttachedFilesForMessage,
  isImageFile,
  readAttachedFile,
  type AttachedFileChip,
} from "@/services/chat/attachFiles";
import { codeLanguageForPath } from "@/services/chat/codeLanguage";
import {
  formatMentionPath,
  formatResourceMention,
  normalizeMentionPath,
  parseComposerTextToSegments,
  serializeComposerSegments as serializeSegments,
  type ComposerSegment,
} from "@/services/chat/composerSegments";
import { createComposerUndoStack, type ComposerSnapshot } from "@/services/chat/composerUndo";
import {
  activeFilePathMention,
  activeHashMention,
  filterHashMentionItems,
  parseHashMentions,
  type HashMentionItem,
} from "@/services/chat/hashMentions";
import {
  recordResourceUsage,
  recordResourceUsages,
  sortByResourceUsage,
} from "@/services/usage/resourceUsage";
import { estimateMessageTokens, formatTokenCount } from "@/services/chat/tokenEstimate";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useChatModelStore } from "@/stores/chatModel";
import { useSettingStore } from "@/stores/setting";
import {
  getProviderIcon,
  getModelDisplayLabel,
  getModelDisplaySubtitle,
  groupModelsByProvider,
  resolveCustomPresetId,
} from "@/lib/providerIcons";
import {
  findModelEntry,
  getActiveThinkingVariant,
  getThinkingTierOptions,
  isKnownModelSelection,
  isModelEntrySelected,
  localizeThinkingTierLabel,
  modelHasThinkingVariants,
} from "@/lib/modelThinking";
import { useChatStore, type SessionCompose } from "@/stores/chat";
import {
  localizedOptionLabel,
  normalizeChatMode,
  toolApprovalModeOptions,
  type ChatMode,
  type ToolApprovalMode,
} from "@/types/setting";
import type {
  AskDisplayOption,
  AskUserQuestion,
  CapturedContext,
  ChatModelInfo,
  ContextUsageSnapshot,
  ChatSessionSummary,
  PathPermissionDecision,
  ToolApprovalDecision,
  ToolApprovalSession,
} from "@/types/chat";
import {
  clearCurrentWorkspace,
  createWorkspace,
  getCurrentWorkspace,
  listWorkspaces,
  listWorkspaceFiles,
  selectWorkspaceFolder,
  switchWorkspace,
  type Workspace,
} from "@/commands/workspace";
import { compressImageDataUrl } from "@/services/chat/compressImage";

export interface AskUserSession {
  requestId: string;
  questions: AskUserQuestion[];
}

export interface PathPermissionSession {
  requestId: string;
  path: string;
  operation: string;
  toolName: string;
}

const ASK_SKIP_MARKER = "__user_supplement__";
const log = createLogger("chat-input");

const props = withDefaults(
  defineProps<{
    sending?: boolean;
    placeholder?: string;
    enableCommands?: boolean;
    closeOnEscape?: boolean;
    askUser?: AskUserSession | null;
    pathPermission?: PathPermissionSession | null;
    toolApproval?: ToolApprovalSession | null;
    historySessions?: ChatSessionSummary[] | null;
    showWorkspaceButton?: boolean;
    selectionLines?: number;
    sessionId?: string;
    capturedContext?: CapturedContext | null;
    contextReady?: boolean;
    overlayPickers?: boolean;
    appearance?: "overlay" | "workbench";
  }>(),
  {
    sending: false,
    placeholder: "",
    enableCommands: true,
    closeOnEscape: true,
    showWorkspaceButton: false,
    selectionLines: 0,
    sessionId: "",
    capturedContext: null,
    contextReady: false,
    overlayPickers: false,
    appearance: "overlay",
    // An omitted history list means no picker is open. Without this default,
    // `undefined !== null` keeps the input locked in history navigation mode.
    historySessions: null,
  },
);

const emit = defineEmits<{
  submit: [message: string];
  pause: [];
  close: [];
  askUserComplete: [answer: string];
  pathPermissionComplete: [decision: PathPermissionDecision];
  toolApprovalComplete: [decision: ToolApprovalDecision];
  openHistory: [];
  historySelect: [sessionId: string];
  historyClose: [];
  removeSelection: [];
  showContext: [context: CapturedContext];
  previewImage: [source: string];
  layoutChange: [
    payload: {
      showSuggestions: boolean;
      suggestionCount: number;
      showModelMenu: boolean;
      modelMenuHeight: number;
      askUserRowCount: number;
      pickerRowCount: number;
      pickerHeight?: number;
      hasImages?: boolean;
      hasFiles?: boolean;
      isPreviewOpen?: boolean;
      /** Measured .input-bar height so Alt+Alt can grow with multi-line input. */
      inputBarHeight?: number;
      layoutReason?: "picker" | "chrome" | "other";
    },
  ];
  modelChange: [modelId: string];
}>();

const message = ref("");
/** Leading selection chips only — mentions/skills live as plain text in `message`. */
const composerSegments = ref<ComposerSegment[]>([]);
const composerUndo = createComposerUndoStack();
const composerInputMultiline = ref(false);
/** Kept as a no-op stub so stale HMR closures never throw ReferenceError. */
let composerShellResizeObserver: ResizeObserver | null = null;
const hasComposerChips = computed(() =>
  composerSegments.value.some((seg) => seg.kind === "selection"),
);
const attachedImages = ref<string[]>([]);
const attachedFiles = ref<AttachedFileChip[]>([]);
const fileDragOver = ref(false);
/** Esc closes @/# suggestion UI without deleting the typed token. */
const mentionSuggestSuppressed = ref<{ trigger: "@" | "#"; start: number } | null>(null);

function isMentionSuggestSuppressed(trigger: "@" | "#", start: number) {
  const suppressed = mentionSuggestSuppressed.value;
  return Boolean(suppressed && suppressed.trigger === trigger && suppressed.start === start);
}

function suppressMentionSuggestions(trigger: "@" | "#", start: number) {
  mentionSuggestSuppressed.value = { trigger, start };
  selectedIndex.value = 0;
  emitLayoutChange();
}

/** Flatten selection chips + live textarea into the outbound message body. */
function serializeComposerSegments() {
  return serializeSegments(composerSegments.value, message.value);
}

/** Capture current composer state for programmatic-edit undo. */
function captureComposerSnapshot(): ComposerSnapshot {
  return {
    message: message.value,
    segments: composerSegments.value.map((seg) => ({ ...seg })),
  };
}

/**
 * Restore the most recent snapshot; returns false when the stack is empty so
 * plain text edits can fall through to the textarea's native undo.
 */
function undoComposerSnapshot(): boolean {
  const snapshot = composerUndo.pop();
  if (!snapshot) return false;
  message.value = snapshot.message;
  composerSegments.value = snapshot.segments;
  void nextTick(() => {
    resizeComposerInput();
    focusInput();
    syncComposerCaret();
    emitLayoutChange();
  });
  return true;
}

function clearComposerSegments() {
  composerSegments.value = [];
}

/** Draft persistence is per-conversation: switching chats never loses input
 * and one conversation's draft is not shared with others. */
function persistDraft(sessionId = props.sessionId, draft = serializeComposerSegments()) {
  if (!sessionId) return;
  chatStore.setComposeDraft(sessionId, draft);
}

function loadDraft() {
  if (!props.sessionId) {
    clearComposerSegments();
    message.value = "";
    return;
  }
  const compose = chatStore.ensureCompose(props.sessionId);
  const parsed = parseComposerTextToSegments(compose.draft || "");
  message.value = serializeSegments(parsed.segments, parsed.liveMessage);
  composerSegments.value = [];
}

const persistDraftDebounced = useDebounceFn((sessionId: string) => {
  if (!sessionId || sessionId !== props.sessionId) return;
  persistDraft(sessionId);
}, 400);

watch(
  [message, composerSegments],
  () => {
    if (!props.sessionId) return;
    persistDraftDebounced(props.sessionId);
  },
  { deep: true },
);

function previewImage(url: string) {
  emit("previewImage", url);
}

function removeAttachedImage(index: number) {
  attachedImages.value.splice(index, 1);
  emitLayoutChange();
}

function removeAttachedFile(index: number) {
  attachedFiles.value.splice(index, 1);
  emitLayoutChange();
}

async function ingestDroppedOrPastedFiles(files: FileList | File[]) {
  const list = Array.from(files);
  if (list.length === 0) return;
  for (const file of list) {
    if (isImageFile(file)) {
      const dataUrl = await new Promise<string | null>((resolve) => {
        const reader = new FileReader();
        reader.onload = () => resolve(String(reader.result ?? "") || null);
        reader.onerror = () => resolve(null);
        reader.readAsDataURL(file);
      });
      if (!dataUrl) continue;
      const compressed = await compressImageDataUrl(dataUrl);
      attachedImages.value.push(compressed);
      continue;
    }
    const chip = await readAttachedFile(file);
    attachedFiles.value.push(chip);
  }
  emitLayoutChange();
}

function onFileDragOver(event: DragEvent) {
  if (!event.dataTransfer?.types?.includes("Files")) return;
  fileDragOver.value = true;
}

function onFileDragLeave() {
  fileDragOver.value = false;
}

async function onFileDrop(event: DragEvent) {
  fileDragOver.value = false;
  const files = event.dataTransfer?.files;
  if (!files?.length) return;
  await ingestDroppedOrPastedFiles(files);
}

async function applyCapturedImages(images?: string[]) {
  if (!images?.length) {
    return;
  }
  const compressed = await Promise.all(images.map((url) => compressImageDataUrl(url)));
  attachedImages.value = compressed;
  emitLayoutChange();
}

const composerRef = ref<InstanceType<typeof ComposerEditable> | null>(null);
const chatInputShellRef = ref<HTMLElement | null>(null);
const attachButtonRef = ref<HTMLButtonElement | null>(null);
const chatModeButtonRef = ref<HTMLButtonElement | null>(null);
const modelButtonRef = ref<HTMLButtonElement | null>(null);
const thinkingTierButtonRef = ref<HTMLButtonElement | null>(null);
const approvalButtonRef = ref<HTMLButtonElement | null>(null);
const chipPickerPosition = ref({ left: 8, bottom: 42, width: 280 });
const chipPickerStyle = computed(() => ({
  "--chip-picker-left": `${chipPickerPosition.value.left}px`,
  "--chip-picker-bottom": `${chipPickerPosition.value.bottom}px`,
  "--chip-picker-width": `${chipPickerPosition.value.width}px`,
}));
const selectedIndex = ref(0);

watch(selectedIndex, async () => {
  await nextTick();
  const activeEl = document.querySelector<HTMLElement>(".command-item.active");
  const list = activeEl?.closest<HTMLElement>(".command-list");
  if (!activeEl || !list) return;
  // Keep scrolling inside the picker only —never scrollIntoView on the
  // document / message list (that hides the picker header / question).
  const sticky = list.querySelector<HTMLElement>(".picker-sticky-head");
  const stickyHeight = sticky?.offsetHeight ?? 0;
  const itemTop = activeEl.offsetTop;
  const itemBottom = itemTop + activeEl.offsetHeight;
  const viewTop = list.scrollTop + stickyHeight;
  const viewBottom = list.scrollTop + list.clientHeight;
  if (itemTop < viewTop) {
    list.scrollTop = Math.max(0, itemTop - stickyHeight);
  } else if (itemBottom > viewBottom) {
    list.scrollTop = itemBottom - list.clientHeight;
  }
});

const modelPickerOpen = ref(false);
const modelChipConfirm = ref(false);
let modelChipConfirmTimer: ReturnType<typeof setTimeout> | null = null;
const approvalPickerOpen = ref(false);
const thinkingTierPickerOpen = ref(false);
const chatModePickerOpen = ref(false);
const askQuestionIndex = ref(0);
const askAnswers = ref<Record<number, string[]>>({});
const askUserFinishing = ref(false);

const settingStore = useSettingStore();
const chatStore = useChatStore();
const chatModelStore = useChatModelStore();
const { language } = storeToRefs(settingStore);
const { sessions } = storeToRefs(chatStore);

const conversationTokenCount = computed(() =>
  (sessions.value[props.sessionId] ?? []).reduce(
    (total, message) => total + estimateMessageTokens(message),
    0,
  ),
);

const conversationTokenTitle = computed(() =>
  tr(language.value, "tokens.sessionEstimated", {
    count: new Intl.NumberFormat(language.value).format(conversationTokenCount.value),
  }),
);

const contextUsage = ref<ContextUsageSnapshot>({
  usageRatio: 0,
  estimatedTokens: 0,
  contextWindowTokens: 64_000,
});

const contextUsageTooltip = computed(() => {
  const percent = Math.round(Math.max(0, Math.min(contextUsage.value.usageRatio, 1)) * 100);
  const used = formatTokenCount(contextUsage.value.estimatedTokens, language.value);
  const total = formatTokenCount(contextUsage.value.contextWindowTokens, language.value);
  return `${tr(language.value, "contextUsedPercent", { percent })} · ${tr(language.value, "contextUsageHint", { used, total })}`;
});

function emptyContextUsage(): ContextUsageSnapshot {
  return {
    usageRatio: 0,
    estimatedTokens: 0,
    contextWindowTokens: 64_000,
  };
}

function buildDraftMessage() {
  return serializeComposerSegments().trim();
}

let contextUsageRequestId = 0;

const loadContextUsage = useDebounceFn(async (requestId: number) => {
  const sessionId = props.sessionId;
  const draftMessage = buildDraftMessage();
  const hasConversation = (sessions.value[sessionId] ?? []).some(
    (item) => item.role === "user" || item.role === "assistant",
  );

  if (!hasConversation && !draftMessage) {
    contextUsage.value = emptyContextUsage();
    if (sessionId) {
      chatStore.setContextUsage(sessionId, contextUsage.value);
    }
    return;
  }

  try {
    const response = await getContextUsage({
      sessionId: sessionId || undefined,
      draftMessage: draftMessage || undefined,
      context: props.capturedContext ?? undefined,
      modelId: chatModel.value.trim() || undefined,
    });
    if (requestId !== contextUsageRequestId || sessionId !== props.sessionId) {
      return;
    }
    contextUsage.value = {
      usageRatio: response.usageRatio,
      estimatedTokens: response.estimatedTokens,
      contextWindowTokens: response.contextWindowTokens,
    };
    if (sessionId) {
      chatStore.setContextUsage(sessionId, contextUsage.value);
    }
  } catch (error) {
    console.error("Failed to load context usage:", error);
  }
}, 180);

function refreshContextUsage() {
  contextUsageRequestId += 1;
  return loadContextUsage(contextUsageRequestId);
}

function sessionMessagesFingerprint(sessionId: string) {
  const messages = sessions.value[sessionId] ?? [];
  let chars = 0;
  for (const item of messages) {
    chars += item.content.length + (item.reasoning?.length ?? 0);
  }
  return `${messages.length}:${chars}`;
}

watch(
  () =>
    [
      props.sessionId,
      props.capturedContext,
      settingStore.largeContextEnabled,
      props.sessionId ? sessionMessagesFingerprint(props.sessionId) : "",
    ] as const,
  () => {
    void refreshContextUsage();
  },
);

watch(
  [message, composerSegments],
  () => {
    void refreshContextUsage();
  },
  { deep: true },
);

watch(
  composerSegments,
  () => {
    void nextTick(resizeComposerInput);
    emitLayoutChange();
  },
  { deep: true },
);

watch(
  () => props.capturedContext?.selectedImages,
  (images) => {
    void applyCapturedImages(images);
  },
  { immediate: true, deep: true },
);

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
    return tr(language.value, "yesterday");
  }

  return `${date.getMonth() + 1}/${date.getDate()}`;
}

// Model selection is per-conversation (chatStore.sessionCompose). A session
// without its own choice inherits the last used conversation on first open.
const chatModel = ref("");
const chatModelProvider = ref("");

watch(chatModel, () => {
  void refreshContextUsage();
});

function syncComposeToModel() {
  if (!props.sessionId) {
    return;
  }
  const compose = chatStore.ensureCompose(props.sessionId);
  if (compose.chatModel !== chatModel.value) {
    chatModel.value = compose.chatModel;
  }
  if (compose.chatModelProvider !== chatModelProvider.value) {
    chatModelProvider.value = compose.chatModelProvider;
  }
}
syncComposeToModel();

watch(
  () => props.sessionId,
  (next, prev) => {
    if (prev && prev !== next) {
      persistDraft(prev, serializeComposerSegments());
    }
    syncComposeToModel();
    loadDraft();
  },
  { immediate: true },
);

const modelStatusText = computed(() => ({
  loading: tr(language.value, "loadingModels"),
  empty: tr(language.value, "noModels"),
}));

const availableModels = computed(() => {
  const models = [...chatModelStore.models];
  const current = chatModel.value.trim();

  if (
    current &&
    models.length > 0 &&
    !models.some((model) => isModelEntrySelected(model, current, chatModelProvider.value))
  ) {
    models.unshift({ id: current, ownedBy: "", provider: chatModelProvider.value });
  }

  return models;
});

/** Draft stashed while the model list uses the input as a filter query. */
const modelPickerDraft = ref<string | null>(null);

function beginModelFilterSession() {
  if (modelPickerDraft.value !== null) {
    return;
  }
  modelPickerDraft.value = message.value;
  message.value = "";
}

function endModelFilterSession() {
  if (modelPickerDraft.value === null) {
    return;
  }
  message.value = modelPickerDraft.value;
  modelPickerDraft.value = null;
}

function modelMatchesFilter(model: (typeof availableModels.value)[number], query: string) {
  const q = query.trim().toLowerCase();
  if (!q) {
    return true;
  }
  const haystack = [
    model.id,
    model.provider,
    model.ownedBy,
    getModelDisplayLabel(model),
    getModelDisplaySubtitle(model) ?? "",
  ]
    .join(" ")
    .toLowerCase();
  return haystack.includes(q);
}

const modelPickerModels = computed(() => {
  const models = availableModels.value;
  if (!modelPickerOpen.value) {
    return models;
  }
  return models.filter((model) => modelMatchesFilter(model, message.value));
});

const modelPickerEmptyText = computed(() => {
  if (modelPickerOpen.value && message.value.trim()) {
    return tr(language.value, "noMatchingModels");
  }
  return modelStatusText.value.empty;
});

const currentModelEntry = computed(() =>
  findModelEntry(availableModels.value, chatModel.value, chatModelProvider.value),
);

const showThinkingTierPicker = computed(() => {
  const entry = currentModelEntry.value;
  return entry ? modelHasThinkingVariants(entry) : false;
});

function thinkingTierIcon(label: string) {
  switch (label.trim().toLowerCase()) {
    case "low":
      return Zap;
    case "high":
      return Brain;
    case "agent":
      return Bot;
    case "default":
      return MessageCircle;
    default:
      return Brain;
  }
}

const thinkingTierPickerOptions = computed(() => {
  const entry = currentModelEntry.value;
  if (!entry) {
    return [];
  }
  return getThinkingTierOptions(entry).map((variant) => {
    const label = localizeThinkingTierLabel(variant.label, language.value);
    return {
      id: variant.id,
      label,
      icon: thinkingTierIcon(variant.label),
    };
  });
});

const currentThinkingTierLabel = computed(() => {
  const entry = currentModelEntry.value;
  if (!entry) {
    return "";
  }
  const active = getActiveThinkingVariant(entry, chatModel.value);
  return active ? localizeThinkingTierLabel(active.label, language.value) : "";
});

const thinkingTierBadgeTitle = computed(() =>
  tr(language.value, "currentThinkingTier", { tier: currentThinkingTierLabel.value }),
);

const currentModelProviderIcon = computed(() => {
  const provider = currentModelEntry.value?.provider;
  return getProviderIcon(provider, resolveCustomPresetId(provider, settingStore.customProviders));
});

const currentModelDisplayName = computed(() => {
  const current = chatModel.value.trim();
  if (!current || (chatModelStore.models.length === 0 && !chatModelStore.loading)) {
    return tr(language.value, "chooseModel");
  }
  const match = currentModelEntry.value;
  if (!match && chatModelStore.models.length === 0) {
    return tr(language.value, "chooseModel");
  }
  return getModelDisplayLabel(match ?? { id: current, provider: "", displayName: undefined });
});

const modelBadgeTitle = computed(() => {
  const current = chatModel.value.trim();
  if (!current || chatModelStore.models.length === 0) {
    return tr(language.value, "chooseModel");
  }
  const match = currentModelEntry.value;
  return tr(language.value, "currentModel", {
    model: getModelDisplayLabel(match ?? { id: current, provider: "", displayName: undefined }),
  });
});

/** Per-conversation mode/approval choices; sessionless overlay falls back to
 * global settings so its pickers keep working without a conversation. */
const sessionChatMode = computed(() => {
  if (!props.sessionId) {
    return normalizeChatMode(settingStore.chatMode);
  }
  // Read-only: never call ensureCompose inside a computed (it mutates the store
  // and can trip "Maximum recursive updates exceeded").
  const compose = chatStore.sessionCompose[props.sessionId];
  return normalizeChatMode(compose?.chatMode ?? settingStore.chatMode);
});
/** Session mode chip follows the user's picker choice only. */
const effectiveChatMode = computed<ChatMode>(() => sessionChatMode.value);
const sessionToolApprovalMode = computed(() => {
  if (!props.sessionId) {
    return settingStore.toolApprovalMode;
  }
  return (
    chatStore.sessionCompose[props.sessionId]?.toolApprovalMode ?? settingStore.toolApprovalMode
  );
});

const chatModeLabel = computed(() => {
  switch (effectiveChatMode.value) {
    case "ask":
      return tr(language.value, "chatModeAsk");
    case "plan":
      return tr(language.value, "chatModePlan");
    default:
      return tr(language.value, "chatModeAgent");
  }
});
const chatModeBadgeTitle = computed(() => {
  switch (effectiveChatMode.value) {
    case "ask":
      return tr(language.value, "currentChatModeAsk");
    case "plan":
      return tr(language.value, "currentChatModePlan");
    default:
      return tr(language.value, "currentChatModeAgent");
  }
});
const chatModeIcon = computed(() => {
  switch (effectiveChatMode.value) {
    case "ask":
      return MessageCircle;
    case "plan":
      return ListChecks;
    default:
      return Bot;
  }
});

async function syncPlanModeFromBackend(sessionId: string) {
  if (!sessionId) return;
  chatStore.ensureCompose(sessionId);
  try {
    const active = await getPlanMode(sessionId);
    chatStore.setSessionPlanMode(sessionId, Boolean(active));
  } catch (error) {
    log.warn("get_plan_mode failed", error);
  }
}

function updateCompose(
  patch: Partial<
    Pick<SessionCompose, "chatModel" | "chatModelProvider" | "chatMode" | "toolApprovalMode">
  >,
) {
  if (props.sessionId) {
    chatStore.ensureCompose(props.sessionId);
    chatStore.setCompose(props.sessionId, patch);
    return;
  }
  // Plan is session-scoped; never persist it as the app-wide default.
  const safe = patch.chatMode === "plan" ? { ...patch, chatMode: "agent" as const } : patch;
  void settingStore.update(safe as never);
}

function getApprovalIcon(mode: ToolApprovalMode) {
  switch (mode) {
    case "ask":
      // Ask before each tool —shield with question.
      return ShieldQuestion;
    case "auto":
      // Auto-run under policy —guarded shield.
      return Shield;
    case "alwaysAllow":
      // No prompts (dangerous shell still blocked) —unlocked.
      return Unlock;
  }
}

const chatModePickerOptions = computed(() => [
  {
    id: "ask",
    label: tr(language.value, "chatModeAsk"),
    icon: MessageCircle,
  },
  {
    id: "agent",
    label: tr(language.value, "chatModeAgent"),
    icon: Bot,
  },
  {
    id: "plan",
    label: tr(language.value, "chatModePlan"),
    icon: ListChecks,
  },
]);
const approvalPickerOptions = computed(() =>
  toolApprovalModeOptions.map((option) => ({
    id: option.value,
    label: localizedOptionLabel(option, language.value),
    icon: getApprovalIcon(option.value),
  })),
);
const approvalModeLabel = computed(() => {
  const current = approvalPickerOptions.value.find(
    (option) => option.id === sessionToolApprovalMode.value,
  );
  return current?.label ?? tr(language.value, "toolApprovalMode");
});
const approvalBadgeTitle = computed(() =>
  tr(language.value, "currentApprovalMode", { mode: approvalModeLabel.value }),
);

const showHistoryPicker = computed(() => Array.isArray(props.historySessions));

const historyItems = computed(() => props.historySessions ?? []);

const historyPickerRowCount = computed(() =>
  showHistoryPicker.value ? Math.max(historyItems.value.length, 1) : 0,
);

const showModelPicker = computed(() => modelPickerOpen.value);
const showChatModePicker = computed(() => chatModePickerOpen.value);
const showApprovalPicker = computed(() => approvalPickerOpen.value);
const showThinkingTierList = computed(() => thinkingTierPickerOpen.value);
const chipPickerOpen = computed(
  () =>
    showModelPicker.value ||
    showChatModePicker.value ||
    showApprovalPicker.value ||
    showThinkingTierList.value,
);

const modelPickerRowCount = computed(() => {
  if (!showModelPicker.value) {
    return 0;
  }
  const models = Math.max(modelPickerModels.value.length, 1);
  const groups = Math.max(
    groupModelsByProvider(modelPickerModels.value, settingStore.customProviders).length,
    1,
  );
  // group headers + model rows + refresh
  return groups + models + 1;
});

const chatModePickerRowCount = computed(() =>
  showChatModePicker.value ? chatModePickerOptions.value.length : 0,
);

const approvalPickerRowCount = computed(() =>
  showApprovalPicker.value ? approvalPickerOptions.value.length : 0,
);

const thinkingTierPickerRowCount = computed(() =>
  showThinkingTierList.value ? thinkingTierPickerOptions.value.length : 0,
);

// function historySlug(sessionId: string) {
//   const compact = sessionId.replace(/^session-/, "").slice(-8) || sessionId;
//   return compact.length > 12 ? `${compact.slice(0, 12)}…` : compact;
// }

const showPathPermissionPicker = computed(() => Boolean(props.pathPermission));

const pathPermissionHeader = computed(() => tr(language.value, "permissionRequest"));

const pathPermissionQuestion = computed(() => {
  const operation = props.pathPermission?.operation ?? "write";
  const tool = props.pathPermission?.toolName ?? "tool";
  return tr(language.value, "permissionQuestion", {
    operation: tr(language.value, operation === "write" ? "write" : "read"),
    tool,
  });
});

const pathPermissionOptions = computed(() => [
  {
    slug: "yes",
    label: tr(language.value, "allowOnce"),
    description: tr(language.value, "allowOnceDesc"),
    decision: "allow_once" as const,
    icon: Check,
  },
  {
    slug: "always",
    label: tr(language.value, "allowAlways"),
    description: tr(language.value, "allowAlwaysDesc"),
    decision: "allow_always" as const,
    icon: ShieldCheck,
  },
  {
    slug: "no",
    label: tr(language.value, "deny"),
    description: tr(language.value, "denyDesc"),
    decision: "deny" as const,
    icon: Ban,
  },
]);

const showToolApprovalPicker = computed(() => Boolean(props.toolApproval));

const toolApprovalHeader = computed(() => tr(language.value, "toolApprovalTitle"));

const toolApprovalOptions = computed(() => [
  {
    slug: "once",
    label: tr(language.value, "allowOnce"),
    description: tr(language.value, "allowOnceDesc"),
    decision: "allow_once" as const,
    icon: Check,
  },
  {
    slug: "session",
    label: tr(language.value, "allowSession"),
    description: tr(language.value, "allowSessionDesc"),
    decision: "allow_session" as const,
    icon: Shield,
  },
  {
    slug: "deny",
    label: tr(language.value, "deny"),
    description: tr(language.value, "denyDesc"),
    decision: "deny" as const,
    icon: Ban,
  },
]);

const showAskUserPicker = computed(() =>
  Boolean(props.askUser && props.askUser.questions.length > 0 && !askUserFinishing.value),
);

const askQuestionCount = computed(() => props.askUser?.questions.length ?? 0);

const activeAskQuestion = computed(() => props.askUser?.questions[askQuestionIndex.value]);

function toAskSlug(label: string) {
  return label
    .trim()
    .toLowerCase()
    .replace(/[^\w\u4e00-\u9fff]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 32);
}

const skipAskOption = computed<AskDisplayOption>(() => ({
  label: tr(language.value, "customAnswer"),
  slug: "custom",
  description: tr(language.value, "customAnswerDesc"),
  isSkip: true,
}));

const activeAskOptions = computed<AskDisplayOption[]>(() => {
  const options = (activeAskQuestion.value?.options ?? []).map((option) => ({
    label: option.label,
    slug: toAskSlug(option.label) || "option",
    description: option.description,
  }));
  return [...options, skipAskOption.value];
});

const askConfirmRowIndex = computed(() =>
  activeAskQuestion.value?.multiSelect ? activeAskOptions.value.length : -1,
);

const askSelectedCount = computed(() => {
  if (!activeAskQuestion.value?.multiSelect) return 0;
  return (askAnswers.value[askQuestionIndex.value] ?? []).length;
});

const pathPermissionPickerRowCount = computed(() =>
  showPathPermissionPicker.value ? 3 + pathPermissionOptions.value.length : 0,
);

const toolApprovalPickerRowCount = computed(() =>
  showToolApprovalPicker.value ? 1 + toolApprovalOptions.value.length : 0,
);

const interactivePickerOpen = computed(
  () =>
    showAskUserPicker.value ||
    showPathPermissionPicker.value ||
    showToolApprovalPicker.value ||
    showHistoryPicker.value ||
    showModelPicker.value ||
    showChatModePicker.value ||
    showApprovalPicker.value ||
    showThinkingTierList.value ||
    workspacePickerOpen.value ||
    attachPanelOpen.value,
);

/** Ask / path / tool-approval requests —these need reserved vertical room. */
const interactionRequestOpen = computed(
  () => showAskUserPicker.value || showPathPermissionPicker.value || showToolApprovalPicker.value,
);

/** Pickers that must keep the input read-only (model picker allows typing to filter). */
const inputLockedForTyping = computed(
  () =>
    showAskUserPicker.value ||
    showPathPermissionPicker.value ||
    showToolApprovalPicker.value ||
    showHistoryPicker.value ||
    showChatModePicker.value ||
    showApprovalPicker.value ||
    showThinkingTierList.value ||
    workspacePickerOpen.value ||
    attachPanelOpen.value,
);

const askPickerRowCount = computed(() => {
  if (!showAskUserPicker.value) {
    return 0;
  }
  const optionRows = activeAskOptions.value.length;
  const confirmRow = activeAskQuestion.value?.multiSelect ? 1 : 0;
  return 2 + optionRows + confirmRow;
});

const hasInlineAttachmentTags = computed(
  () =>
    composerSegments.value.some((seg) => seg.kind === "selection") ||
    attachedFiles.value.length > 0,
);

const inputPlaceholder = computed(() => {
  // Images sit above the text field —keep the hint when only images are attached.
  if (composerSegments.value.length > 0 || hasInlineAttachmentTags.value) {
    return "";
  }
  if (props.sending && !interactivePickerOpen.value) {
    return canSend.value
      ? tr(language.value, "attachInjectHint")
      : tr(language.value, "aiResponding");
  }
  if (showHistoryPicker.value) {
    return tr(language.value, "openChatHint");
  }
  if (showModelPicker.value) {
    return tr(language.value, "selectModelHint");
  }
  if (showChatModePicker.value || showApprovalPicker.value || showThinkingTierList.value) {
    return tr(language.value, "selectOptionHint");
  }
  if (showPathPermissionPicker.value || showToolApprovalPicker.value) {
    return tr(language.value, "permissionHint");
  }
  if (showAskUserPicker.value) {
    return tr(
      language.value,
      activeAskQuestion.value?.multiSelect ? "askMultiHint" : "askCustomHint",
    );
  }
  return props.placeholder || tr(language.value, "askAnything");
});

const canSend = computed(
  () =>
    serializeComposerSegments().trim().length > 0 ||
    attachedFiles.value.length > 0 ||
    attachedImages.value.length > 0,
);

const showPauseIcon = computed(() => props.sending && !canSend.value);

function removeTrailingAttachment(): boolean {
  if (attachedImages.value.length > 0) {
    attachedImages.value.pop();
    return true;
  }
  if (attachedFiles.value.length > 0) {
    attachedFiles.value.pop();
    return true;
  }
  const last = composerSegments.value[composerSegments.value.length - 1];
  if (last) {
    composerSegments.value.pop();
    if (last.kind === "selection") {
      emit("removeSelection");
    }
    return true;
  }
  if (props.selectionLines) {
    emit("removeSelection");
    return true;
  }
  return false;
}

let layoutChangeFlushScheduled = false;
let lastEmittedChromeHeight = 0;
/** Signature of picker/attachment chrome — only emit layout when this changes. */
let lastLayoutChromeSignature = "";

function layoutChromeSignature() {
  return [
    activePickerRowCount(),
    showSuggestions.value ? 1 : 0,
    attachPanelOpen.value ? 1 : 0,
    attachedImages.value.length,
    attachedFiles.value.length,
  ].join(":");
}

function emitLayoutChange(force = false) {
  if (layoutChangeFlushScheduled) {
    return;
  }
  layoutChangeFlushScheduled = true;
  void nextTick(() => {
    layoutChangeFlushScheduled = false;
    flushLayoutChange(force);
  });
}

/** Last measured picker list height —refined after paint so tall/desc rows fit. */
let measuredPickerHeight = 0;
let pickerMeasureScheduled = false;

function estimateActivePickerHeight(pickerRows: number): number {
  if (pickerRows <= 0) {
    return 0;
  }
  // Match component row metrics (padding + row). Prefer overestimate to avoid clipping.
  if (attachPanelOpen.value) {
    // Tabs + preview chips (+ optional expand). Prefer overestimate so overlay grows.
    return 170;
  }
  if (showChatModePicker.value) {
    return 10 + chatModePickerOptions.value.length * 36;
  }
  if (showApprovalPicker.value) {
    return 10 + approvalPickerOptions.value.length * 36;
  }
  if (showThinkingTierList.value) {
    return 10 + thinkingTierPickerOptions.value.length * 36;
  }
  if (showModelPicker.value) {
    const models = Math.max(modelPickerModels.value.length, 1);
    const groups = Math.max(
      groupModelsByProvider(modelPickerModels.value, settingStore.customProviders).length,
      1,
    );
    return 6 + groups * 24 + models * 32 + 34;
  }
  if (showHistoryPicker.value) {
    return 10 + Math.max(historyItems.value.length, 1) * 32;
  }
  if (showAskUserPicker.value) {
    const options = activeAskOptions.value.length + (activeAskQuestion.value?.multiSelect ? 1 : 0);
    return 10 + 26 + 48 + options * 30;
  }
  if (showPathPermissionPicker.value) {
    // Header + path chip + option cards (with descriptions) + card padding.
    return 24 + 40 + 52 + pathPermissionOptions.value.length * 52;
  }
  if (showToolApprovalPicker.value) {
    return 24 + 40 + toolApprovalOptions.value.length * 52;
  }
  if (workspacePickerOpen.value) {
    return 10 + pickerRows * 32;
  }
  if (showSuggestions.value) {
    return 9 + suggestionCount.value * 30;
  }
  return 9 + pickerRows * 32;
}

function activePickerRowCount(): number {
  if (workspacePickerOpen.value) return workspacePickerRowCount.value;
  // Attach is not a fixed row list, but it must reserve height so Overlay /
  // layout measure runs (otherwise the panel opens clipped / looks "stuck").
  if (attachPanelOpen.value) return 5;
  if (showAskUserPicker.value) return askPickerRowCount.value;
  if (showPathPermissionPicker.value) return pathPermissionPickerRowCount.value;
  if (showToolApprovalPicker.value) return toolApprovalPickerRowCount.value;
  if (showHistoryPicker.value) return historyPickerRowCount.value;
  if (showModelPicker.value) return modelPickerRowCount.value;
  if (showChatModePicker.value) return chatModePickerRowCount.value;
  if (showApprovalPicker.value) return approvalPickerRowCount.value;
  if (showThinkingTierList.value) return thinkingTierPickerRowCount.value;
  if (showSuggestions.value) return suggestionCount.value;
  return 0;
}

function schedulePickerHeightMeasure() {
  if (pickerMeasureScheduled) {
    return;
  }
  pickerMeasureScheduled = true;
  void nextTick(async () => {
    pickerMeasureScheduled = false;
    // Wait two frames so Transition/GSAP has mounted the list.
    await new Promise<void>((resolve) =>
      requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
    );
    updateInteractionPickerMaxHeight();
    if (activePickerRowCount() <= 0) {
      measuredPickerHeight = 0;
      return;
    }
    const list = document.querySelector(".chat-input-shell .command-list") as HTMLElement | null;
    if (list && interactionRequestOpen.value) {
      list.scrollTop = 0;
    }
    const height = list?.offsetHeight ?? 0;
    if (height <= 0) {
      return;
    }
    if (Math.abs(height - measuredPickerHeight) < 1) {
      return;
    }
    measuredPickerHeight = height;
    flushLayoutChange();
  });
}

/** Cap ask/approval panels so they never grow past the conversation pane
 *  (absolute overlays would otherwise clip the sticky question header). */
function updateInteractionPickerMaxHeight() {
  const shell = chatInputShellRef.value;
  if (!shell) return;
  if (!interactionRequestOpen.value) {
    shell.style.removeProperty("--interaction-picker-max-height");
    return;
  }

  const pane =
    shell.closest<HTMLElement>(".conversation-pane") ||
    shell.closest<HTMLElement>(".peek-panel") ||
    shell.closest<HTMLElement>(".composer-dock");
  const inputBar = shell.querySelector<HTMLElement>(".input-bar");
  const inputHeight = inputBar?.getBoundingClientRect().height ?? 96;
  const topReserve = 20;
  const bottomReserve = 12;

  let available = Math.floor(window.innerHeight * 0.48);
  if (pane) {
    const paneHeight = pane.getBoundingClientRect().height;
    available = Math.floor(paneHeight - inputHeight - topReserve - bottomReserve);
  }

  const capped = Math.max(180, Math.min(available, Math.floor(window.innerHeight * 0.62)));
  shell.style.setProperty("--interaction-picker-max-height", `${capped}px`);
}

function flushLayoutChange(force = false) {
  const pickerRows = activePickerRowCount();
  if (pickerRows <= 0) {
    measuredPickerHeight = 0;
  }

  const pickerHeight =
    pickerRows > 0 ? Math.max(measuredPickerHeight, estimateActivePickerHeight(pickerRows)) : 0;

  const shell = chatInputShellRef.value;
  const inputBar = shell?.querySelector<HTMLElement>(".input-bar");
  const chromeHeight =
    props.appearance === "overlay" && shell ? shell.offsetHeight : (inputBar?.offsetHeight ?? 0);

  const signature = layoutChromeSignature();
  const pickerStateChanged = signature !== lastLayoutChromeSignature;
  const chromeChanged = Math.abs(chromeHeight - lastEmittedChromeHeight) > 1;

  if (!force && !pickerStateChanged && !chromeChanged) {
    if (pickerRows > 0) schedulePickerHeightMeasure();
    return;
  }

  lastLayoutChromeSignature = signature;
  lastEmittedChromeHeight = chromeHeight;

  emit("layoutChange", {
    showSuggestions: showSuggestions.value,
    suggestionCount: suggestionCount.value,
    showModelMenu: false,
    modelMenuHeight: 0,
    askUserRowCount: showAskUserPicker.value ? askPickerRowCount.value : 0,
    pickerRowCount: pickerRows,
    pickerHeight,
    hasImages: attachedImages.value.length > 0,
    hasFiles: attachedFiles.value.length > 0,
    inputBarHeight: chromeHeight > 0 ? chromeHeight : undefined,
    layoutReason: pickerStateChanged ? "picker" : chromeChanged ? "chrome" : "other",
  });

  if (pickerRows > 0) {
    schedulePickerHeightMeasure();
  }
}

async function syncPopupState(open: boolean) {
  const windowLabel = getCurrentWebviewWindow().label;
  try {
    await Promise.race([
      setOverlayPopupOpen(windowLabel, open),
      new Promise<void>((resolve) => {
        window.setTimeout(resolve, 800);
      }),
    ]);
  } catch (error) {
    console.error("set_overlay_popup_open failed:", error);
  }
}

function closeChipPickers() {
  if (modelPickerOpen.value) {
    endModelFilterSession();
  }
  modelPickerOpen.value = false;
  approvalPickerOpen.value = false;
  chatModePickerOpen.value = false;
  thinkingTierPickerOpen.value = false;
}

function anyChipStillOpen() {
  return (
    modelPickerOpen.value ||
    approvalPickerOpen.value ||
    chatModePickerOpen.value ||
    thinkingTierPickerOpen.value
  );
}

function closeModelPicker() {
  if (!modelPickerOpen.value) {
    return;
  }
  endModelFilterSession();
  modelPickerOpen.value = false;
  if (!anyChipStillOpen()) {
    void syncPopupState(false);
  }
  emitLayoutChange();
}

function closeApprovalPicker() {
  if (!approvalPickerOpen.value) {
    return;
  }
  approvalPickerOpen.value = false;
  if (!anyChipStillOpen()) {
    void syncPopupState(false);
  }
  emitLayoutChange();
}

function closeChatModePicker() {
  if (!chatModePickerOpen.value) {
    return;
  }
  chatModePickerOpen.value = false;
  if (!anyChipStillOpen()) {
    void syncPopupState(false);
  }
  emitLayoutChange();
}

function closeThinkingTierPicker() {
  if (!thinkingTierPickerOpen.value) {
    return;
  }
  thinkingTierPickerOpen.value = false;
  if (!anyChipStillOpen()) {
    void syncPopupState(false);
  }
  emitLayoutChange();
}

/** Compatibility aliases used by shared close sites. */
function closeApprovalMenu(_immediate = false) {
  closeApprovalPicker();
}
function closeChatModeMenu(_immediate = false) {
  closeChatModePicker();
}
function closeThinkingTierMenu(_immediate = false) {
  closeThinkingTierPicker();
}

function dismissFloatingPickers() {
  const hadChip = chipPickerOpen.value;
  const hadWorkspace = workspacePickerOpen.value;
  const hadAttach = attachPanelOpen.value;
  if (!hadChip && !hadWorkspace && !hadAttach) return;
  closeChipPickers();
  if (hadWorkspace) {
    workspacePickerOpen.value = false;
    workspaceQuickSelectOnly.value = false;
  }
  if (hadAttach) {
    attachPanelOpen.value = false;
  }
  void syncPopupState(false);
  emitLayoutChange();
}

function handleDocumentPointerDown(event: PointerEvent) {
  if (event.button !== 0) return;
  if (!chipPickerOpen.value && !workspacePickerOpen.value && !attachPanelOpen.value) return;
  if (interactionRequestOpen.value) return;
  const target = event.target;
  if (!(target instanceof Element)) return;
  if (target.closest(".command-list")) return;
  if (target.closest("[data-picker-trigger]")) return;
  dismissFloatingPickers();
}

async function prepareChipPicker() {
  if (showHistoryPicker.value) {
    emit("historyClose");
  }
  workspacePickerOpen.value = false;
  workspaceQuickSelectOnly.value = false;
  attachPanelOpen.value = false;
  closeChipPickers();
}

async function positionChipPicker(button: HTMLButtonElement | null, preferredWidth: number) {
  if (props.appearance !== "workbench") return;
  await nextTick();
  const shell = chatInputShellRef.value;
  if (!shell || !button) return;
  const shellRect = shell.getBoundingClientRect();
  const buttonRect = button.getBoundingClientRect();
  const edge = 8;
  const width = Math.max(120, Math.min(preferredWidth, shellRect.width - edge * 2));
  const naturalLeft = buttonRect.left - shellRect.left;
  chipPickerPosition.value = {
    left: Math.min(shellRect.width - width - edge, Math.max(edge, naturalLeft)),
    bottom: shellRect.bottom - buttonRect.top + 4,
    width,
  };
}

async function openModelPicker() {
  await prepareChipPicker();
  beginModelFilterSession();
  const currentIdx = modelPickerModels.value.findIndex((model) =>
    isModelEntrySelected(model, chatModel.value, chatModelProvider.value),
  );
  selectedIndex.value = currentIdx >= 0 ? currentIdx : 0;
  modelPickerOpen.value = true;
  await positionChipPicker(modelButtonRef.value, 340);
  await syncPopupState(true);
  emitLayoutChange();
  void focusInput();

  if (chatModelStore.models.length === 0) {
    void chatModelStore.fetch().then(() => {
      if (modelPickerOpen.value) {
        emitLayoutChange();
      }
    });
  } else {
    void chatModelStore.softRefresh().then(() => {
      if (modelPickerOpen.value) {
        emitLayoutChange();
      }
    });
  }
}

async function openApprovalPicker() {
  if (effectiveChatMode.value === "ask") {
    return;
  }
  await prepareChipPicker();
  const idx = approvalPickerOptions.value.findIndex(
    (option) => option.id === sessionToolApprovalMode.value,
  );
  selectedIndex.value = idx >= 0 ? idx : 0;
  approvalPickerOpen.value = true;
  await positionChipPicker(approvalButtonRef.value, 340);
  await syncPopupState(true);
  emitLayoutChange();
  void focusInput();
}

async function openChatModePicker() {
  await prepareChipPicker();
  const idx = chatModePickerOptions.value.findIndex(
    (option) => option.id === effectiveChatMode.value,
  );
  selectedIndex.value = idx >= 0 ? idx : 0;
  chatModePickerOpen.value = true;
  await positionChipPicker(chatModeButtonRef.value, 132);
  await syncPopupState(true);
  emitLayoutChange();
  void focusInput();
}

async function openThinkingTierPicker() {
  if (!showThinkingTierPicker.value) {
    return;
  }
  await prepareChipPicker();
  const idx = thinkingTierPickerOptions.value.findIndex((option) => option.id === chatModel.value);
  selectedIndex.value = idx >= 0 ? idx : 0;
  thinkingTierPickerOpen.value = true;
  await positionChipPicker(thinkingTierButtonRef.value, 240);
  await syncPopupState(true);
  emitLayoutChange();
  void focusInput();
}

function toggleModelMenu() {
  if (modelPickerOpen.value) {
    closeModelPicker();
    return;
  }
  void openModelPicker();
}

function toggleApprovalMenu() {
  if (effectiveChatMode.value === "ask") {
    return;
  }
  if (approvalPickerOpen.value) {
    closeApprovalPicker();
    return;
  }
  void openApprovalPicker();
}

function toggleChatModeMenu() {
  if (chatModePickerOpen.value) {
    closeChatModePicker();
    return;
  }
  void openChatModePicker();
}

function toggleThinkingTierMenu() {
  if (!showThinkingTierPicker.value) {
    return;
  }
  if (thinkingTierPickerOpen.value) {
    closeThinkingTierPicker();
    return;
  }
  void openThinkingTierPicker();
}

function flashModelChipConfirm() {
  modelChipConfirm.value = true;
  if (modelChipConfirmTimer) {
    clearTimeout(modelChipConfirmTimer);
  }
  modelChipConfirmTimer = setTimeout(() => {
    modelChipConfirm.value = false;
    modelChipConfirmTimer = null;
  }, 120);
}

function selectModel(entry: ChatModelInfo) {
  closeModelPicker();
  const nextId = entry.id;
  const nextProvider = entry.provider;
  if (nextId === chatModel.value && nextProvider === chatModelProvider.value) {
    return;
  }
  chatModel.value = nextId;
  chatModelProvider.value = nextProvider;
  flashModelChipConfirm();
  updateCompose({ chatModel: nextId, chatModelProvider: nextProvider });
  emit("modelChange", nextId);
}

function applyFallbackModelIfNeeded() {
  if (chatModelStore.models.length === 0) return;
  if (
    chatModel.value.trim() &&
    isKnownModelSelection(chatModelStore.models, chatModel.value, chatModelProvider.value)
  ) {
    return;
  }
  const fallback = chatModelStore.models[0]!;
  selectModel(fallback);
  if (
    !settingStore.chatModel.trim() ||
    !isKnownModelSelection(
      chatModelStore.models,
      settingStore.chatModel,
      settingStore.chatModelProvider,
    )
  ) {
    void settingStore.update({
      chatModel: fallback.id,
      chatModelProvider: fallback.provider,
    });
  }
}

async function refreshModelList() {
  await chatModelStore.reload();
  if (modelPickerOpen.value) {
    emitLayoutChange();
  }
}

function selectApprovalMode(mode: string) {
  closeApprovalPicker();
  const next = mode as ToolApprovalMode;
  if (next === sessionToolApprovalMode.value) {
    return;
  }
  updateCompose({ toolApprovalMode: next });
}

function selectChatMode(mode: string) {
  closeChatModePicker();
  const next = mode as ChatMode;
  if (next === effectiveChatMode.value) {
    return;
  }
  if (next === "ask") {
    closeApprovalPicker();
  }
  updateCompose({ chatMode: next });
  if (!props.sessionId) {
    return;
  }
  // Entering Plan turns the writer gate on. Leaving Plan only updates the mode
  // preference — keep the pending plan/approval card until the user approves or
  // sends a follow-up (backend clears the gate on that send).
  if (next === "plan") {
    chatStore.setSessionRejectedPlanFingerprint(props.sessionId, null);
    void setPlanMode(props.sessionId, true, "manual")
      .then(() => {
        chatStore.setSessionPlanMode(props.sessionId!, true);
        chatStore.setSessionPlanTrigger(props.sessionId!, "manual");
      })
      .catch((error) => log.warn("sync plan mode on mode switch failed", error));
  }
}

function selectThinkingTier(variantId: string) {
  closeThinkingTierPicker();
  if (variantId === chatModel.value) {
    return;
  }
  chatModel.value = variantId;
  updateCompose({
    chatModel: variantId,
    chatModelProvider: chatModelProvider.value,
  });
  emit("modelChange", variantId);
}

onMounted(async () => {
  log.debug("slash command registration", {
    commands: slashCommands.map((item) => item.command),
    available: props.enableCommands && props.contextReady,
  });
  if (props.sessionId) {
    void syncPlanModeFromBackend(props.sessionId);
  }
  void ensureHashCatalog();
  await chatModelStore.fetch();
  if (chatModelStore.models.length === 0 && chatModel.value.trim() === "deepseek-chat") {
    chatModel.value = "";
    chatModelProvider.value = "";
    const composeFallback = props.sessionId
      ? chatStore.ensureCompose(props.sessionId).chatModel
      : settingStore.chatModel;
    if (composeFallback.trim() === "deepseek-chat") {
      updateCompose({ chatModel: "", chatModelProvider: "" });
    }
  } else {
    applyFallbackModelIfNeeded();
  }
  void refreshContextUsage();
  await loadWorkspaceState();
  unlistenWorkspaces = await listen("workspaces-changed", () => {
    void loadWorkspaceState();
  });
  unlistenChatFinished = await listen("chat-finished", () => {
    void refreshContextUsage();
  });
  unlistenChatStarted = await listen("chat-started", () => {
    void refreshContextUsage();
  });
  try {
    unlistenFocus = await getCurrentWebviewWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) {
        // Workspace files may change while the window is blurred (IDE save/new).
        workspaceFilesFetchedAt = 0;
        if (attachPanelOpen.value && attachPanelTab.value === "files") {
          void ensureWorkspaceFiles(true);
        }
        if (props.appearance === "overlay") {
          void focusInput();
        } else {
          restorePickerFocus();
        }
      }
    });
  } catch (error) {
    console.error("onFocusChanged failed:", error);
  }
});

watch(
  () => props.sessionId,
  (sessionId) => {
    if (sessionId) {
      chatStore.ensureCompose(sessionId);
      void syncPlanModeFromBackend(sessionId);
    }
  },
  { immediate: true },
);

watch(
  () => chatModelStore.models.map((model) => `${model.provider}:${model.id}`).join("|"),
  () => {
    applyFallbackModelIfNeeded();
  },
);

onUnmounted(() => {
  persistDraft();
  composerShellResizeObserver?.disconnect();
  composerShellResizeObserver = null;
  unlistenWorkspaces?.();
  unlistenChatFinished?.();
  unlistenChatStarted?.();
  unlistenFocus?.();
  void syncPopupState(false);
});

useEventListener(window, "keydown", handleGlobalKeydown);
useEventListener(window, "focus", restorePickerFocus);
useEventListener(window, "resize", () => {
  updateInteractionPickerMaxHeight();
});
useEventListener(document, "pointerdown", handleDocumentPointerDown);
useEventListener(document, "visibilitychange", () => {
  if (document.visibilityState === "visible") {
    restorePickerFocus();
  }
});

watch(
  interactionRequestOpen,
  (open) => {
    if (open) {
      void nextTick(() => {
        updateInteractionPickerMaxHeight();
        const list = chatInputShellRef.value?.querySelector<HTMLElement>(".command-list");
        if (list) list.scrollTop = 0;
      });
    } else {
      chatInputShellRef.value?.style.removeProperty("--interaction-picker-max-height");
    }
  },
  { immediate: true },
);

const isCommandMode = computed(
  () =>
    !interactivePickerOpen.value &&
    props.enableCommands &&
    props.contextReady &&
    message.value.startsWith("/") &&
    !message.value.includes(" "),
);

watch(
  () => [props.enableCommands, props.contextReady] as const,
  ([enabled, ready]) => {
    log.debug("slash command availability", {
      enabled,
      contextReady: ready,
      available: enabled && ready,
    });
  },
  { immediate: true },
);

const filteredCommands = computed(() => {
  if (!isCommandMode.value) {
    return [];
  }

  const query = message.value.toLowerCase();
  return slashCommands
    .filter(
      (item) =>
        item.command.toLowerCase().startsWith(query) &&
        (item.command !== "/work" || props.showWorkspaceButton),
    )
    .map((item) => ({
      ...item,
      description: tr(language.value, item.descriptionKey),
    }));
});

const showCommandSuggestions = computed(
  () => isCommandMode.value && filteredCommands.value.length > 0,
);

const workspaceFiles = ref<string[]>([]);
const workspaceFilesLoading = ref(false);
const workspaceFilesRoot = ref("");
/** Soft TTL so typing `@ab` does not rescan every keystroke, but opens refresh. */
const WORKSPACE_FILES_TTL_MS = 4_000;
let workspaceFilesFetchedAt = 0;
let workspaceFilesRequestId = 0;
/** Caret in the live input —`#` / `@` mentions resolve against this, not only EOF. */
const composerCaret = ref(0);

function syncComposerCaret() {
  const next = composerRef.value?.getSelection().start ?? message.value.length;
  if (composerCaret.value !== next) {
    composerCaret.value = next;
  }
}

function onComposerCaretChange(caret: number) {
  if (composerCaret.value !== caret) {
    composerCaret.value = caret;
  }
}

function onComposerInput() {
  syncComposerCaret();
  const heightChanged = resizeComposerInput();
  // Grow the Alt+Alt native window with wrapped lines (up to 4), not an
  // inner scrollbar while the dock is still single-line tall.
  if (props.appearance === "overlay" && heightChanged) {
    void nextTick(() => emitLayoutChange());
  }
}

watch(message, (value) => {
  if (composerCaret.value > value.length) {
    composerCaret.value = value.length;
  }
});

const activeFileMention = computed(() => {
  if (!currentWorkspace.value || interactivePickerOpen.value) {
    return null;
  }
  return activeFilePathMention(message.value, composerCaret.value);
});
const fileSuggestions = computed(() => {
  const mention = activeFileMention.value;
  if (!mention) return [];
  const query = mention.query.toLowerCase();
  const pool = workspaceFiles.value;
  const filtered = query ? pool.filter((path) => path.toLowerCase().includes(query)) : pool;
  return filtered
    .filter((path): path is string => typeof path === "string" && path.length > 0)
    .sort((left, right) => {
      const leftName = left.split("/").pop()?.toLowerCase() ?? left.toLowerCase();
      const rightName = right.split("/").pop()?.toLowerCase() ?? right.toLowerCase();
      const leftRank = query && leftName.startsWith(query) ? 0 : 1;
      const rightRank = query && rightName.startsWith(query) ? 0 : 1;
      return leftRank - rightRank || left.length - right.length || left.localeCompare(right);
    })
    .slice(0, 12);
});
const showFileSuggestions = computed(() => {
  const mention = activeFileMention.value;
  if (!mention) return false;
  return !isMentionSuggestSuppressed("@", mention.start);
});

const hashCatalog = ref<HashMentionItem[]>([]);
const hashCatalogLoading = ref(false);
const hashCatalogReady = ref(false);
const composerSkillMeta = computed(() =>
  hashCatalog.value
    .filter((item) => item.kind === "skill")
    .map((item) => ({
      name: item.id,
      title: item.title,
      qualifiedName: item.vendor,
      iconUrl: item.iconUrl,
    })),
);

const activeHashQuery = computed(() => {
  if (
    interactivePickerOpen.value ||
    showFileSuggestions.value ||
    isCommandMode.value ||
    attachPanelOpen.value
  ) {
    return null;
  }
  return activeHashMention(message.value, composerCaret.value);
});

const hashSuggestions = computed(() => {
  const mention = activeHashQuery.value;
  if (!mention) return [];
  return filterHashMentionItems(hashCatalog.value, mention.query);
});

const showHashSuggestions = computed(() => {
  const mention = activeHashQuery.value;
  if (!mention) return false;
  return !isMentionSuggestSuppressed("#", mention.start);
});

const attachSkillItems = computed(() =>
  sortByResourceUsage(
    hashCatalog.value.filter((item) => item.kind === "skill"),
    "skill",
    (item) => item.id,
  ),
);

const attachMcpItems = computed(() =>
  sortByResourceUsage(
    hashCatalog.value.filter((item) => item.kind === "mcp"),
    "mcp",
    (item) => item.id,
  ),
);

const showSuggestions = computed(
  () => showFileSuggestions.value || showHashSuggestions.value || showCommandSuggestions.value,
);
const suggestionCount = computed(() => {
  if (showFileSuggestions.value) return Math.max(fileSuggestions.value.length, 1);
  if (showHashSuggestions.value) return Math.max(hashSuggestions.value.length, 1);
  return filteredCommands.value.length;
});

async function ensureHashCatalog(force = false) {
  if (!force && (hashCatalogReady.value || hashCatalogLoading.value)) return;
  hashCatalogLoading.value = true;
  try {
    const skills = await listSkills();
    const enabledBuiltins = new Set(settingStore.enabledBuiltinSkills ?? []);
    const skillItems: HashMentionItem[] = skills
      .filter((skill) => skill.source !== "builtin" || enabledBuiltins.has(skill.name))
      .map((skill) => ({
        kind: "skill" as const,
        id: skill.name,
        title: skill.title || skill.name,
        description: skill.description || undefined,
        iconUrl: skill.iconUrl ?? null,
        vendor:
          skill.qualifiedName?.trim() ||
          (skill.namespace && skill.slug ? `${skill.namespace}/${skill.slug}` : undefined) ||
          undefined,
      }));
    const mcpItems: HashMentionItem[] = (settingStore.mcpServers ?? [])
      .filter((server) => server.enabled !== false)
      .map((server) => ({
        kind: "mcp" as const,
        id: server.id,
        title: server.title || server.id,
        description: server.description || server.command || undefined,
        iconUrl: server.iconUrl ?? null,
        vendor: server.qualifiedName?.trim() || undefined,
      }));
    hashCatalog.value = [...skillItems, ...mcpItems];
    hashCatalogReady.value = true;
    void warmInstallIcons(
      hashCatalog.value.map((item) => ({
        kind: item.kind,
        cacheKey: item.id,
        url: item.iconUrl,
      })),
    ).then(() => {
      hashCatalog.value = hashCatalog.value.map((item) => {
        const local = peekInstallIcon(item.kind, item.id);
        return local ? { ...item, iconUrl: local } : item;
      });
    });
  } catch (error) {
    console.error("load hash mention catalog failed:", error);
    hashCatalog.value = [];
  } finally {
    hashCatalogLoading.value = false;
  }
}

function insertPlainToken(token: string) {
  const sel = composerRef.value?.getSelection() ?? {
    start: message.value.length,
    end: message.value.length,
  };
  const before = message.value.slice(0, sel.start);
  const after = message.value.slice(sel.end);
  const needsSpaceBefore = before.length > 0 && !/\s$/.test(before);
  // Always leave a trailing space so the caret exits the @/# token and suggestions close.
  const needsSpaceAfter = after.length === 0 || !/^\s/.test(after);
  const inserted = `${needsSpaceBefore ? " " : ""}${token}${needsSpaceAfter ? " " : ""}`;
  const next = `${before}${inserted}${after}`;
  const caret = before.length + inserted.length;
  mentionSuggestSuppressed.value = null;
  composerRef.value?.setText(next, caret);
  void nextTick(() => {
    composerRef.value?.focus({ preventScroll: true });
    resizeComposerInput();
    syncComposerCaret();
    emitLayoutChange();
  });
}

function selectHashMention(item: HashMentionItem) {
  const mention = activeHashQuery.value;
  if (!mention) return;
  selectedIndex.value = 0;
  const token = formatResourceMention(item.kind, item.id);
  const before = message.value.slice(0, mention.start);
  const after = message.value.slice(mention.end);
  const needsSpaceBefore = before.length > 0 && !/\s$/.test(before);
  const needsSpaceAfter = after.length === 0 || !/^\s/.test(after);
  const inserted = `${needsSpaceBefore ? " " : ""}${token}${needsSpaceAfter ? " " : ""}`;
  const next = `${before}${inserted}${after}`;
  const caret = before.length + inserted.length;
  mentionSuggestSuppressed.value = null;
  composerRef.value?.setText(next, caret);
  void nextTick(() => {
    resizeComposerInput();
    syncComposerCaret();
    emitLayoutChange();
  });
}

async function ensureWorkspaceFiles(force = false) {
  const root = currentWorkspace.value?.root ?? "";
  if (!root) {
    workspaceFiles.value = [];
    workspaceFilesRoot.value = "";
    workspaceFilesFetchedAt = 0;
    return;
  }

  const now = Date.now();
  const sameRoot = workspaceFilesRoot.value === root;
  const fresh =
    sameRoot &&
    workspaceFilesFetchedAt > 0 &&
    now - workspaceFilesFetchedAt < WORKSPACE_FILES_TTL_MS;
  // Same workspace was previously treated as a permanent cache — new/renamed
  // files never appeared in @ suggestions or the attach file tree.
  if (!force && fresh) return;
  if (!force && workspaceFilesLoading.value && sameRoot) return;

  const requestId = ++workspaceFilesRequestId;
  workspaceFilesLoading.value = true;
  try {
    const files = await listWorkspaceFiles();
    if (requestId !== workspaceFilesRequestId) return;
    workspaceFiles.value = files;
    workspaceFilesRoot.value = root;
    workspaceFilesFetchedAt = Date.now();
  } catch (error) {
    if (requestId !== workspaceFilesRequestId) return;
    console.error("list_workspace_files failed:", error);
    workspaceFiles.value = [];
    workspaceFilesRoot.value = root;
    workspaceFilesFetchedAt = 0;
  } finally {
    if (requestId === workspaceFilesRequestId) {
      workspaceFilesLoading.value = false;
    }
  }
}

function selectWorkspaceFile(path: string) {
  const mention = activeFileMention.value;
  if (!mention) return;
  selectedIndex.value = 0;
  const token = formatMentionPath(toWorkspaceRelativePath(path));
  const before = message.value.slice(0, mention.start);
  const after = message.value.slice(mention.end);
  const needsSpaceBefore = before.length > 0 && !/\s$/.test(before);
  const needsSpaceAfter = after.length === 0 || !/^\s/.test(after);
  const inserted = `${needsSpaceBefore ? " " : ""}${token}${needsSpaceAfter ? " " : ""}`;
  const next = `${before}${inserted}${after}`;
  const caret = before.length + inserted.length;
  mentionSuggestSuppressed.value = null;
  composerRef.value?.setText(next, caret);
  void nextTick(() => {
    resizeComposerInput();
    syncComposerCaret();
    emitLayoutChange();
  });
}

function toWorkspaceRelativePath(path: string): string {
  const normalized = normalizeMentionPath(path);
  const root = currentWorkspace.value?.root;
  if (!root) return normalized;
  const normRoot = normalizeMentionPath(root).toLowerCase();
  const lower = normalized.toLowerCase();
  if (lower === normRoot) return "";
  if (lower.startsWith(`${normRoot}/`)) {
    return normalized.slice(normRoot.length + 1);
  }
  return normalized;
}

function fileIconForPath(path: string) {
  return codeLanguageForPath(normalizeMentionPath(path)).icon;
}

async function focusInput() {
  await nextTick();
  composerRef.value?.focus({ preventScroll: true });
}

/** Click empty padding around the textarea — focus and place caret at end. */
function onInputBarMouseDown(event: MouseEvent) {
  const target = event.target;
  if (!(target instanceof Element)) return;
  if (
    target.closest(
      "button, a, textarea, input, label, [contenteditable='true'], .composer-editable, .input-footer, .input-images, .input-files, .command-list, .attach-resource-panel, .model-picker-list, .option-picker-list, .selection-tag",
    )
  ) {
    return;
  }
  event.preventDefault();
  void nextTick(() => {
    composerRef.value?.focus({ preventScroll: true });
    composerRef.value?.setSelection(message.value.length);
    syncComposerCaret();
  });
}

function resizeComposerInput(): boolean {
  const editor = composerRef.value;
  if (!editor) return false;
  const el = editor.el;
  if (!el) return false;
  const lineHeight = 24;
  const maxLines = props.appearance === "overlay" ? 4 : 8;
  const minHeight = lineHeight;
  const maxHeight = lineHeight * maxLines;
  const prevHeight = el.offsetHeight;

  // Auto-grow up to maxLines, then scroll. Measure via height:auto only when
  // applying — skip layout emit when the pixel height is unchanged.
  el.style.height = "auto";
  const contentHeight = el.scrollHeight;
  const nextHeight = Math.max(minHeight, Math.min(contentHeight, maxHeight));
  el.style.height = `${nextHeight}px`;
  el.style.overflowY = contentHeight > maxHeight ? "auto" : "hidden";

  composerInputMultiline.value = nextHeight > lineHeight + 4;
  return Math.abs(nextHeight - prevHeight) > 1;
}

/** @deprecated use resizeComposerInput — kept as alias for call-site churn */
function resizeWorkbenchInput() {
  resizeComposerInput();
}

/** Keyboard navigation for pickers must work even if the input lost focus (e.g. after Alt-Tab). */
function handleGlobalKeydown(event: KeyboardEvent) {
  if (!interactivePickerOpen.value) {
    return;
  }
  if (event.target instanceof Node && composerRef.value?.el?.contains(event.target)) {
    return;
  }
  handleKeydown(event);
}

function restorePickerFocus() {
  if (interactivePickerOpen.value) {
    void focusInput();
  }
}

async function executeCommand(command: string) {
  if (!props.enableCommands || !props.contextReady) {
    log.debug("slash command blocked", { command, contextReady: props.contextReady });
    return;
  }
  message.value = "";
  persistDraft();
  clearComposerSegments();
  selectedIndex.value = 0;
  emitLayoutChange();
  const action = await executeSlashCommand(command);
  if (action === "openHistory") {
    emit("openHistory");
    return;
  }
  if (action === "openModel") {
    void openModelPicker();
    return;
  }
  if (action === "openWorkspace") {
    await openWorkspaceQuickPicker();
    return;
  }
  if (action === "clearInput") {
    reset();
    return;
  }
  if (action === "showContext") {
    try {
      emit("showContext", await fetchEnvironmentContext());
    } catch (error) {
      console.error(
        "Failed to invoke get_environment_context; using resolved overlay snapshot:",
        error,
      );
      emit("showContext", props.capturedContext ?? {});
    }
    return;
  }
  if (action === "close") {
    emit("close");
  }
}

// Global workspace selection is available only before a conversation starts.
const workspaces = ref<Workspace[]>([]);
const currentWorkspace = ref<Workspace | null>(null);
const workspacePickerOpen = ref(false);
const attachPanelOpen = ref(false);
const attachPickingFiles = ref(false);
const attachPanelTab = ref<"skills" | "mcp" | "files">("skills");
const attachVisibleCount = ref(0);
const workspaceQuickSelectOnly = ref(false);
const workspaceSaving = ref(false);
const workspaceError = ref("");
/** Overlay-only: user explicitly picked a workspace for this summon session. */
const overlayWorkspaceOverride = ref<Workspace | null>(null);
let unlistenWorkspaces: UnlistenFn | null = null;
let unlistenChatFinished: UnlistenFn | null = null;
let unlistenChatStarted: UnlistenFn | null = null;
let unlistenFocus: UnlistenFn | null = null;

const workspaceTooltip = computed(() =>
  currentWorkspace.value
    ? `${currentWorkspace.value.name}\n${currentWorkspace.value.root}`
    : "Create a workspace before starting a conversation",
);

const workspacePickerRowCount = computed(() => {
  if (!workspacePickerOpen.value) return 0;
  return 2 + Math.max(workspaces.value.length, 1);
});

function normalizeWorkspaceRoot(root: string) {
  return root.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
}

function syncOverlayWorkspaceFromContext() {
  if (props.appearance !== "overlay" || overlayWorkspaceOverride.value) {
    return;
  }
  // Overlay should not inherit workbench workspace.
  // Only use an explicitly reported IDE workspace when present.
  const ideRoot = props.capturedContext?.ideContext?.workspace;
  if (!ideRoot) {
    currentWorkspace.value = null;
    workspaceFiles.value = [];
    workspaceFilesRoot.value = "";
    workspaceFilesFetchedAt = 0;
    return;
  }
  const normalized = normalizeWorkspaceRoot(ideRoot);
  currentWorkspace.value =
    workspaces.value.find((workspace) => normalizeWorkspaceRoot(workspace.root) === normalized) ??
    null;
}

async function loadWorkspaceState() {
  try {
    const items = await listWorkspaces();
    workspaces.value = items;
    if (props.appearance === "overlay") {
      syncOverlayWorkspaceFromContext();
      return;
    }
    const current = await getCurrentWorkspace();
    if (current?.root !== currentWorkspace.value?.root) {
      workspaceFiles.value = [];
      workspaceFilesRoot.value = "";
      workspaceFilesFetchedAt = 0;
    }
    currentWorkspace.value = current;
  } catch (error) {
    workspaceError.value = String(error);
  }
}

async function toggleWorkspacePicker() {
  if (workspacePickerOpen.value) {
    workspacePickerOpen.value = false;
    await syncPopupState(false);
    emitLayoutChange();
    return;
  }
  workspaceError.value = "";
  workspaceQuickSelectOnly.value = false;
  selectedIndex.value = 0;
  attachPanelOpen.value = false;
  closeModelPicker();
  closeApprovalMenu();
  closeChatModeMenu();
  closeThinkingTierMenu();
  await loadWorkspaceState();
  if (workspaces.value.length === 0) {
    await addWorkspaceFromFolder();
    return;
  }
  workspacePickerOpen.value = true;
  await syncPopupState(true);
  emitLayoutChange();
}

async function toggleAttachPanel() {
  if (props.appearance === "overlay") return;
  if (attachPanelOpen.value) {
    attachPanelOpen.value = false;
    await syncPopupState(false);
    emitLayoutChange();
    return;
  }
  if (showHistoryPicker.value) {
    emit("historyClose");
  }
  workspacePickerOpen.value = false;
  workspaceQuickSelectOnly.value = false;
  closeChipPickers();
  selectedIndex.value = 0;
  attachPanelTab.value = "skills";
  attachPanelOpen.value = true;
  await syncPopupState(true);
  emitLayoutChange();
  // Warm catalog in background; do not force-reload every open (sync list_skills
  // can stall the UI). Workspace files load only when the Files tab is opened.
  void ensureHashCatalog(false);
  void focusInput();
}

function onAttachPanelTabChange(tab: "skills" | "mcp" | "files") {
  attachPanelTab.value = tab;
  selectedIndex.value = 0;
  if (tab === "files") {
    void ensureWorkspaceFiles(true);
  }
}

function onAttachPanelHover(payload: { kind: "skill" | "mcp"; index: number }) {
  attachPanelTab.value = payload.kind === "mcp" ? "mcp" : "skills";
  selectedIndex.value = payload.index;
}

function onAttachVisibleCount(count: number) {
  attachVisibleCount.value = count;
  if (selectedIndex.value >= count) {
    selectedIndex.value = Math.max(0, count - 1);
  }
}

function selectAttachWorkspaceFile(path: string, isDir = false) {
  insertFileMention(path, { isDir });
  emitLayoutChange();
}

function selectAttachResource(item: HashMentionItem) {
  insertPlainToken(formatResourceMention(item.kind, item.id));
  recordResourceUsage(item.kind, item.id);
}

async function pickAttachFiles() {
  if (attachPickingFiles.value) return;
  attachPickingFiles.value = true;
  try {
    const selected = await openFileDialog({
      multiple: true,
      directory: false,
      title: tr(language.value, "chatInput.attachPickFiles"),
    });
    const paths = (Array.isArray(selected) ? selected : selected ? [selected] : [])
      .map((entry) => String(entry ?? "").trim())
      .filter(Boolean);
    if (paths.length === 0) return;
    const tokens = paths.map((path) => formatMentionPath(toWorkspaceRelativePath(path)));
    insertPlainToken(tokens.join(" "));
    attachPanelOpen.value = false;
    await syncPopupState(false);
    void nextTick(() => {
      resizeWorkbenchInput();
      focusInput();
      syncComposerCaret();
      emitLayoutChange();
    });
  } catch (error) {
    console.error("pick attach files failed:", error);
  } finally {
    attachPickingFiles.value = false;
  }
}

async function openWorkspaceQuickPicker() {
  workspaceError.value = "";
  workspaceQuickSelectOnly.value = true;
  selectedIndex.value = 0;
  attachPanelOpen.value = false;
  closeModelPicker();
  closeApprovalMenu();
  closeChatModeMenu();
  closeThinkingTierMenu();
  await loadWorkspaceState();
  workspacePickerOpen.value = true;
  await syncPopupState(true);
  emitLayoutChange();
}

async function addWorkspaceFromFolder() {
  if (workspaceSaving.value) return;
  workspaceSaving.value = true;
  workspaceError.value = "";
  await syncPopupState(true);
  try {
    const root = await selectWorkspaceFolder();
    if (!root) return;
    const workspace = await createWorkspace(root);
    if (props.appearance === "overlay") {
      overlayWorkspaceOverride.value = workspace;
      currentWorkspace.value = workspace;
    } else {
      currentWorkspace.value = await switchWorkspace(workspace.id);
    }
    await loadWorkspaceState();
    workspacePickerOpen.value = false;
    await syncPopupState(false);
  } catch (error) {
    workspaceError.value = String(error);
    workspacePickerOpen.value = true;
  } finally {
    workspaceSaving.value = false;
    if (!workspacePickerOpen.value) await syncPopupState(false);
    emitLayoutChange();
  }
}

async function chooseWorkspace(workspace: Workspace) {
  if (props.appearance === "overlay") {
    overlayWorkspaceOverride.value = workspace;
    currentWorkspace.value = workspace;
  } else if (workspace.id !== currentWorkspace.value?.id) {
    try {
      currentWorkspace.value = await switchWorkspace(workspace.id);
    } catch (error) {
      workspaceError.value = String(error);
      return;
    }
  }
  workspacePickerOpen.value = false;
  workspaceQuickSelectOnly.value = false;
  await syncPopupState(false);
  emitLayoutChange();
  void focusInput();
}

async function exitCurrentWorkspace() {
  workspaceError.value = "";
  try {
    if (props.appearance !== "overlay") {
      await clearCurrentWorkspace();
    }
    overlayWorkspaceOverride.value = null;
    currentWorkspace.value = null;
    workspacePickerOpen.value = false;
    workspaceQuickSelectOnly.value = false;
    await syncPopupState(false);
    emitLayoutChange();
    void focusInput();
  } catch (error) {
    workspaceError.value = String(error);
  }
}

function selectHistorySession(sessionId: string) {
  emit("historySelect", sessionId);
}

function closeHistoryPicker() {
  emit("historyClose");
}

function resolveSendWorkspaceOptions(): { workspaceId?: string; quickAsk?: boolean } {
  // Overlay must only bind to an explicitly selected (or IDE-matched) workspace
  // shown in the composer. Never inherit inferred capture context —that often
  // falls back to the workbench's current workspace and mis-files Quick Ask.
  const active = overlayWorkspaceOverride.value ?? currentWorkspace.value;
  if (active) {
    return { workspaceId: active.id, quickAsk: false };
  }
  if (props.appearance === "overlay") {
    return { quickAsk: true };
  }
  const contextRoot = props.capturedContext?.workspace?.root;
  if (contextRoot) {
    return { workspaceId: contextRoot, quickAsk: false };
  }
  return { quickAsk: true };
}

async function submit() {
  if (workspacePickerOpen.value || attachPanelOpen.value) {
    return;
  }
  if (props.sending && !canSend.value && !interactivePickerOpen.value) {
    emit("pause");
    return;
  }

  if (showHistoryPicker.value) {
    const item = historyItems.value[selectedIndex.value];
    if (item) {
      selectHistorySession(item.sessionId);
    }
    return;
  }

  if (showModelPicker.value) {
    const models = modelPickerModels.value;
    if (selectedIndex.value < models.length) {
      const model = models[selectedIndex.value];
      if (model) selectModel(model);
    } else {
      void refreshModelList();
    }
    return;
  }

  if (showChatModePicker.value) {
    const option = chatModePickerOptions.value[selectedIndex.value];
    if (option) selectChatMode(option.id);
    return;
  }

  if (showApprovalPicker.value) {
    const option = approvalPickerOptions.value[selectedIndex.value];
    if (option) selectApprovalMode(option.id);
    return;
  }

  if (showThinkingTierList.value) {
    const option = thinkingTierPickerOptions.value[selectedIndex.value];
    if (option) selectThinkingTier(option.id);
    return;
  }

  if (showToolApprovalPicker.value) {
    const option = toolApprovalOptions.value[selectedIndex.value];
    if (option) {
      selectToolApproval(option.decision);
    }
    return;
  }

  if (showPathPermissionPicker.value) {
    const option = pathPermissionOptions.value[selectedIndex.value];
    if (option) {
      selectPathPermission(option.decision);
    }
    return;
  }

  if (showAskUserPicker.value) {
    if (activeAskQuestion.value?.multiSelect && selectedIndex.value === askConfirmRowIndex.value) {
      confirmAskSelection();
      return;
    }

    const option = activeAskOptions.value[selectedIndex.value];
    if (option) {
      selectAskOption(option);
    }
    return;
  }

  const text = serializeComposerSegments().trim();
  if (!text && attachedFiles.value.length === 0 && attachedImages.value.length === 0) {
    return;
  }

  if (showFileSuggestions.value) {
    const path = fileSuggestions.value[selectedIndex.value];
    if (path) selectWorkspaceFile(path);
    return;
  }

  if (showHashSuggestions.value) {
    const item = hashSuggestions.value[selectedIndex.value];
    if (item) selectHashMention(item);
    return;
  }

  if (showCommandSuggestions.value) {
    const command = filteredCommands.value[selectedIndex.value]?.command;
    if (command) {
      await executeCommand(command);
    }
    return;
  }

  if (
    props.enableCommands &&
    props.contextReady &&
    slashCommands.some((item) => item.command === text)
  ) {
    await executeCommand(text);
    return;
  }

  const attachedFileBlocks = formatAttachedFilesForMessage(attachedFiles.value);
  const imageTags = attachedImages.value.map((img) => `![image](${img})`).join("\n");
  const submittedText = [text, attachedFileBlocks, imageTags]
    .filter((part) => part.length > 0)
    .join("\n\n");
  recordResourceUsages(parseHashMentions(submittedText));
  emit("submit", submittedText);
  clearComposerSegments();
  message.value = "";
  persistDraft();
  attachedFiles.value = [];
  attachedImages.value = [];
  emitLayoutChange();
}

function handlePaste(event: ClipboardEvent) {
  const items = event.clipboardData?.items;
  if (items) {
    const files: File[] = [];
    for (const item of items) {
      if (item.kind !== "file") continue;
      const file = item.getAsFile();
      if (file) files.push(file);
    }
    if (files.length > 0) {
      event.preventDefault();
      void ingestDroppedOrPastedFiles(files);
      return;
    }
  }

  // Plain-text paste (including multiline) is handled by ComposerEditable.
  emitLayoutChange();
  void nextTick(resizeComposerInput);
}

function selectPathPermission(decision: PathPermissionDecision) {
  emit("pathPermissionComplete", decision);
}

function selectToolApproval(decision: ToolApprovalDecision) {
  emit("toolApprovalComplete", decision);
}

function handleKeydown(event: KeyboardEvent) {
  if (event.isComposing || event.keyCode === 229) {
    return;
  }

  // Ctrl/Cmd+Z: programmatic composer edits (chip removal, mention truncation,
  // setMessage) bypass the textarea's native undo stack, so replay them from the
  // snapshot stack; when empty, fall through and let the browser undo text edits.
  if ((event.ctrlKey || event.metaKey) && !event.shiftKey && event.key.toLowerCase() === "z") {
    if (undoComposerSnapshot()) {
      event.preventDefault();
    }
    return;
  }

  if ((event.key === "Backspace" || event.key === "Delete") && !showModelPicker.value) {
    const sel = composerRef.value?.getSelection();
    const caretAtStart = Boolean(sel) && sel!.start === 0 && sel!.end === 0;
    const empty = message.value.length === 0;
    const shouldRemoveTag =
      (event.key === "Backspace" && caretAtStart) || (event.key === "Delete" && empty);
    if (shouldRemoveTag) {
      const before = captureComposerSnapshot();
      if (removeTrailingAttachment()) {
        composerUndo.push(before);
        event.preventDefault();
        return;
      }
    }
  }

  if (event.key === "Enter" && event.shiftKey && !interactivePickerOpen.value) {
    // Shift+Enter inserts a newline (workbench + Alt+Alt).
    event.preventDefault();
    const sel = composerRef.value?.getSelection() ?? {
      start: message.value.length,
      end: message.value.length,
    };
    const next = `${message.value.slice(0, sel.start)}\n${message.value.slice(sel.end)}`;
    composerRef.value?.setText(next, sel.start + 1);
    void nextTick(() => {
      resizeComposerInput();
      syncComposerCaret();
      emitLayoutChange();
    });
    return;
  }

  if (props.sending && !interactivePickerOpen.value && event.key === "Enter") {
    if (canSend.value) {
      event.preventDefault();
      void submit();
      return;
    }
    // 回复中且无可发送内容：回车不暂停（点暂停按钮）
    event.preventDefault();
    return;
  }

  if (workspacePickerOpen.value) {
    if (event.key === "Escape") {
      event.preventDefault();
      workspacePickerOpen.value = false;
      workspaceQuickSelectOnly.value = false;
      void syncPopupState(false);
      emitLayoutChange();
      return;
    }
    const rows = workspaces.value.length + (workspaceQuickSelectOnly.value ? 0 : 1);
    if (rows === 0) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const delta = event.key === "ArrowDown" ? 1 : -1;
      selectedIndex.value = (selectedIndex.value + delta + rows) % rows;
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      if (!workspaceQuickSelectOnly.value && selectedIndex.value === 0) {
        void addWorkspaceFromFolder();
      } else {
        const workspaceIndex = workspaceQuickSelectOnly.value
          ? selectedIndex.value
          : selectedIndex.value - 1;
        const workspace = workspaces.value[workspaceIndex];
        if (workspace) void chooseWorkspace(workspace);
      }
      return;
    }
  }

  if (attachPanelOpen.value) {
    if (event.key === "Escape") {
      event.preventDefault();
      attachPanelOpen.value = false;
      void syncPopupState(false);
      emitLayoutChange();
      return;
    }
    if (attachPanelTab.value === "files") {
      if (event.key === "Enter") {
        event.preventDefault();
        void pickAttachFiles();
      }
      return;
    }
    const list = attachPanelTab.value === "mcp" ? attachMcpItems.value : attachSkillItems.value;
    const total = Math.min(list.length, Math.max(attachVisibleCount.value, 0));
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      if (total === 0) return;
      event.preventDefault();
      const delta = event.key === "ArrowDown" ? 1 : -1;
      selectedIndex.value = (selectedIndex.value + delta + total) % total;
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      const item = list[selectedIndex.value];
      if (item) selectAttachResource(item);
      return;
    }
  }

  if (showHistoryPicker.value) {
    if (historyItems.value.length === 0) {
      if (event.key === "Escape") {
        event.preventDefault();
        closeHistoryPicker();
      }
      return;
    }
    const totalRows = historyItems.value.length;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value + 1) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex.value =
        (selectedIndex.value - 1 + Math.max(totalRows, 1)) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      const item = historyItems.value[selectedIndex.value];
      if (item) {
        selectHistorySession(item.sessionId);
      }
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      closeHistoryPicker();
      return;
    }
  }

  if (showModelPicker.value) {
    const modelCount = modelPickerModels.value.length;
    const totalRows = modelCount + 1; // models + refresh
    if (event.key === "ArrowDown") {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value + 1) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex.value =
        (selectedIndex.value - 1 + Math.max(totalRows, 1)) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      if (selectedIndex.value < modelCount) {
        const model = modelPickerModels.value[selectedIndex.value];
        if (model) selectModel(model);
      } else {
        void refreshModelList();
      }
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      closeModelPicker();
      return;
    }
  }

  if (showToolApprovalPicker.value) {
    const totalRows = toolApprovalOptions.value.length;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value + 1) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex.value =
        (selectedIndex.value - 1 + Math.max(totalRows, 1)) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      const option = toolApprovalOptions.value[selectedIndex.value];
      if (option) {
        selectToolApproval(option.decision);
      }
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      return;
    }
  }

  if (showPathPermissionPicker.value) {
    const totalRows = pathPermissionOptions.value.length;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value + 1) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex.value =
        (selectedIndex.value - 1 + Math.max(totalRows, 1)) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      const option = pathPermissionOptions.value[selectedIndex.value];
      if (option) {
        selectPathPermission(option.decision);
      }
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      return;
    }
  }

  if (showAskUserPicker.value) {
    const totalRows =
      activeAskOptions.value.length + (activeAskQuestion.value?.multiSelect ? 1 : 0);

    if (event.key === "ArrowDown") {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value + 1) % Math.max(totalRows, 1);
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex.value =
        (selectedIndex.value - 1 + Math.max(totalRows, 1)) % Math.max(totalRows, 1);
      return;
    }

    if (event.key === "Enter") {
      event.preventDefault();
      if (
        activeAskQuestion.value?.multiSelect &&
        selectedIndex.value === askConfirmRowIndex.value
      ) {
        confirmAskSelection();
        return;
      }
      const option = activeAskOptions.value[selectedIndex.value];
      if (option) {
        selectAskOption(option);
      }
      return;
    }

    if (event.key === "Escape") {
      event.preventDefault();
      return;
    }
  }

  if (showChatModePicker.value) {
    const totalRows = chatModePickerOptions.value.length;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value + 1) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex.value =
        (selectedIndex.value - 1 + Math.max(totalRows, 1)) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      const option = chatModePickerOptions.value[selectedIndex.value];
      if (option) selectChatMode(option.id);
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      closeChatModePicker();
      return;
    }
  }

  if (showApprovalPicker.value) {
    const totalRows = approvalPickerOptions.value.length;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value + 1) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex.value =
        (selectedIndex.value - 1 + Math.max(totalRows, 1)) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      const option = approvalPickerOptions.value[selectedIndex.value];
      if (option) selectApprovalMode(option.id);
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      closeApprovalPicker();
      return;
    }
  }

  if (showThinkingTierList.value) {
    const totalRows = thinkingTierPickerOptions.value.length;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value + 1) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex.value =
        (selectedIndex.value - 1 + Math.max(totalRows, 1)) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      const option = thinkingTierPickerOptions.value[selectedIndex.value];
      if (option) selectThinkingTier(option.id);
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      closeThinkingTierPicker();
      return;
    }
  }

  if (showFileSuggestions.value) {
    const count = fileSuggestions.value.length;
    if (event.key === "ArrowDown" && count > 0) {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value + 1) % count;
      return;
    }
    if (event.key === "ArrowUp" && count > 0) {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value - 1 + count) % count;
      return;
    }
    if (event.key === "Tab" || event.key === "Enter") {
      event.preventDefault();
      const path = fileSuggestions.value[selectedIndex.value];
      if (path) selectWorkspaceFile(path);
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      const mention = activeFileMention.value;
      if (mention) suppressMentionSuggestions("@", mention.start);
      return;
    }
  }

  if (showHashSuggestions.value) {
    const count = hashSuggestions.value.length;
    if (event.key === "ArrowDown" && count > 0) {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value + 1) % count;
      return;
    }
    if (event.key === "ArrowUp" && count > 0) {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value - 1 + count) % count;
      return;
    }
    if (event.key === "Tab" || event.key === "Enter") {
      event.preventDefault();
      const item = hashSuggestions.value[selectedIndex.value];
      if (item) selectHashMention(item);
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      const mention = activeHashQuery.value;
      if (mention) suppressMentionSuggestions("#", mention.start);
      return;
    }
  }

  if (showCommandSuggestions.value) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value + 1) % filteredCommands.value.length;
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex.value =
        (selectedIndex.value - 1 + filteredCommands.value.length) % filteredCommands.value.length;
      return;
    }

    if (event.key === "Tab" || event.key === "Enter") {
      event.preventDefault();
      void submit();
      return;
    }

    if (event.key === "Escape") {
      event.preventDefault();
      composerUndo.push(captureComposerSnapshot());
      message.value = "";
      selectedIndex.value = 0;
      emitLayoutChange();
      return;
    }
  }

  if (event.key === "Enter") {
    event.preventDefault();
    void submit();
    return;
  }

  if (event.key === "Escape" && props.closeOnEscape) {
    event.preventDefault();
    emit("close");
  }
}

function reset() {
  clearComposerSegments();
  attachedFiles.value = [];
  attachedImages.value = [];
  mentionSuggestSuppressed.value = null;
  selectedIndex.value = 0;
  lastEmittedChromeHeight = 0;
  if (composerRef.value) {
    composerRef.value.setText("");
  } else {
    message.value = "";
  }
  closeModelPicker();
  closeApprovalMenu();
  closeChatModeMenu();
  closeThinkingTierMenu();
  workspacePickerOpen.value = false;
  workspaceQuickSelectOnly.value = false;
  attachPanelOpen.value = false;
  if (props.appearance === "overlay") {
    overlayWorkspaceOverride.value = null;
    syncOverlayWorkspaceFromContext();
  }
  emitLayoutChange();
  void nextTick(resizeWorkbenchInput);
}

function setMessage(text: string) {
  composerUndo.push(captureComposerSnapshot());
  clearComposerSegments();
  attachedFiles.value = [];
  attachedImages.value = [];
  mentionSuggestSuppressed.value = null;
  const parsed = parseComposerTextToSegments(text);
  const flat = serializeSegments(parsed.segments, parsed.liveMessage);
  if (composerRef.value) {
    composerRef.value.setText(flat);
  } else {
    message.value = flat;
  }
  emitLayoutChange();
  void nextTick(resizeComposerInput);
  void focusInput();
}

function isAskOptionSelected(label: string) {
  if (label === tr(language.value, "customAnswer")) {
    return false;
  }
  const current = askAnswers.value[askQuestionIndex.value] ?? [];
  return current.includes(label);
}

function selectAskOption(option: AskDisplayOption) {
  if (option.isSkip) {
    completeAskUserWithSkip();
    return;
  }

  const question = activeAskQuestion.value;
  if (!question) {
    return;
  }

  if (question.multiSelect) {
    const current = askAnswers.value[askQuestionIndex.value] ?? [];
    askAnswers.value[askQuestionIndex.value] = current.includes(option.label)
      ? current.filter((item) => item !== option.label)
      : [...current, option.label];
    return;
  }

  askAnswers.value[askQuestionIndex.value] = [option.label];
  advanceAskQuestion();
}

function confirmAskSelection() {
  const current = askAnswers.value[askQuestionIndex.value] ?? [];
  if (current.length === 0) {
    return;
  }
  advanceAskQuestion();
}

function completeAskUserWithSkip() {
  if (!props.askUser) {
    return;
  }

  const answers = { ...askAnswers.value };
  for (let index = askQuestionIndex.value; index < props.askUser.questions.length; index += 1) {
    answers[index] = [ASK_SKIP_MARKER];
  }
  finishAskUser(answers, true);
}

function finishAskUser(answers: Record<number, string[]>, skipped = false) {
  if (!props.askUser || askUserFinishing.value) {
    return;
  }

  askUserFinishing.value = true;

  const payload = {
    skipped,
    answers: props.askUser.questions.map((question, index) => {
      const selected = answers[index] ?? [];
      const userSupplement = selected.includes(ASK_SKIP_MARKER);
      return {
        header: question.header,
        question: question.question,
        selected: userSupplement ? [] : selected.filter((item) => item !== ASK_SKIP_MARKER),
        userSupplement,
      };
    }),
  };

  emit("askUserComplete", JSON.stringify(payload));
  emitLayoutChange();
}

function advanceAskQuestion() {
  if (!props.askUser) {
    return;
  }

  if (askQuestionIndex.value < props.askUser.questions.length - 1) {
    askQuestionIndex.value += 1;
    selectedIndex.value = 0;
    emitLayoutChange();
    return;
  }

  finishAskUser(askAnswers.value);
}

watch(
  () => props.askUser?.requestId,
  (requestId) => {
    askUserFinishing.value = false;
    askQuestionIndex.value = 0;
    askAnswers.value = {};
    selectedIndex.value = 0;
    void syncPopupState(Boolean(requestId));
    emitLayoutChange();
  },
);

watch(
  () => props.selectionLines,
  (lines, previous) => {
    if (lines && !previous) {
      if (!composerSegments.value.some((seg) => seg.kind === "selection")) {
        composerSegments.value.push({ kind: "selection", lines });
      }
      emitLayoutChange();
    }
    if (!lines) {
      composerSegments.value = composerSegments.value.filter((seg) => seg.kind !== "selection");
      emitLayoutChange();
    } else if (previous) {
      for (const seg of composerSegments.value) {
        if (seg.kind === "selection") {
          seg.lines = lines;
        }
      }
    }
  },
  { immediate: true },
);

watch(filteredCommands, () => {
  selectedIndex.value = 0;
});

watch(
  () => activeFileMention.value,
  (mention, previous) => {
    selectedIndex.value = 0;
    if (
      mentionSuggestSuppressed.value?.trigger === "@" &&
      (!mention || mention.start !== mentionSuggestSuppressed.value.start)
    ) {
      mentionSuggestSuppressed.value = null;
    }
    if (!mention) return;
    // Force a fresh scan when `@` is newly opened; reuse a short TTL while typing.
    void ensureWorkspaceFiles(!previous);
  },
);

watch(fileSuggestions, () => {
  if (selectedIndex.value >= fileSuggestions.value.length) {
    selectedIndex.value = 0;
  }
});

watch(
  () => activeHashQuery.value,
  (mention) => {
    selectedIndex.value = 0;
    if (
      mentionSuggestSuppressed.value?.trigger === "#" &&
      (!mention || mention.start !== mentionSuggestSuppressed.value.start)
    ) {
      mentionSuggestSuppressed.value = null;
    }
    if (mention) void ensureHashCatalog();
  },
);

watch(hashSuggestions, () => {
  if (selectedIndex.value >= hashSuggestions.value.length) {
    selectedIndex.value = 0;
  }
});

watch(
  () => [settingStore.mcpServers, settingStore.enabledBuiltinSkills],
  () => {
    hashCatalogReady.value = false;
  },
  { deep: true },
);

watch([() => chatModelStore.loading, () => chatModelStore.error, modelPickerModels], () => {
  if (!modelPickerOpen.value) {
    return;
  }
  const maxIndex = modelPickerModels.value.length; // refresh row
  if (selectedIndex.value > maxIndex) {
    selectedIndex.value = 0;
  }
  emitLayoutChange();
});

watch(
  () => message.value,
  () => {
    if (!modelPickerOpen.value) {
      return;
    }
    const models = modelPickerModels.value;
    const currentIdx = models.findIndex((model) =>
      isModelEntrySelected(model, chatModel.value, chatModelProvider.value),
    );
    selectedIndex.value = currentIdx >= 0 ? currentIdx : 0;
    emitLayoutChange();
  },
);

watch(
  showSuggestions,
  () => {
    if (showSuggestions.value && modelPickerOpen.value) {
      closeModelPicker();
    }
    if (showSuggestions.value && approvalPickerOpen.value) {
      closeApprovalMenu();
    }
    if (showSuggestions.value && chatModePickerOpen.value) {
      closeChatModeMenu();
    }
    if (showSuggestions.value && thinkingTierPickerOpen.value) {
      closeThinkingTierMenu();
    }
    emitLayoutChange(true);
  },
  { immediate: true },
);

watch(showAskUserPicker, async (open) => {
  if (open && modelPickerOpen.value) {
    closeModelPicker();
  }
  if (open && approvalPickerOpen.value) {
    closeApprovalMenu();
  }
  if (open && chatModePickerOpen.value) {
    closeChatModeMenu();
  }
  if (open && thinkingTierPickerOpen.value) {
    closeThinkingTierMenu();
  }
  if (open) {
    await nextTick();
    document.querySelectorAll<HTMLElement>(".ask-user-list").forEach((list) => {
      list.scrollTop = 0;
    });
  }
  emitLayoutChange();
});

watch(
  () => props.pathPermission,
  async (session) => {
    if (session) {
      selectedIndex.value = 0;
      if (modelPickerOpen.value) {
        closeModelPicker();
      }
      if (approvalPickerOpen.value) {
        closeApprovalMenu();
      }
      if (chatModePickerOpen.value) {
        closeChatModeMenu();
      }
      if (thinkingTierPickerOpen.value) {
        closeThinkingTierMenu();
      }
      await syncPopupState(true);
      await nextTick();
      document.querySelectorAll<HTMLElement>(".path-permission-list").forEach((list) => {
        list.scrollTop = 0;
      });
      void focusInput();
    }
    emitLayoutChange();
  },
);

watch(
  () => props.toolApproval,
  async (session) => {
    if (session) {
      selectedIndex.value = 0;
      if (modelPickerOpen.value) {
        closeModelPicker();
      }
      if (approvalPickerOpen.value) {
        closeApprovalMenu();
      }
      if (chatModePickerOpen.value) {
        closeChatModeMenu();
      }
      if (thinkingTierPickerOpen.value) {
        closeThinkingTierMenu();
      }
      await syncPopupState(true);
      await nextTick();
      document.querySelectorAll<HTMLElement>(".tool-approval-list").forEach((list) => {
        list.scrollTop = 0;
      });
      void focusInput();
    }
    emitLayoutChange();
  },
);

watch(
  () => props.historySessions,
  async (sessions) => {
    if (Array.isArray(sessions)) {
      selectedIndex.value = 0;
      if (modelPickerOpen.value) {
        closeModelPicker();
      }
      if (approvalPickerOpen.value) {
        closeApprovalMenu();
      }
      if (chatModePickerOpen.value) {
        closeChatModeMenu();
      }
      if (thinkingTierPickerOpen.value) {
        closeThinkingTierMenu();
      }
      await syncPopupState(true);
      void focusInput();
    }
    emitLayoutChange();
  },
);
function insertFileMention(path: string, options?: { isDir?: boolean }) {
  const relative = toWorkspaceRelativePath(path);
  if (!relative && !options?.isDir) return;
  const storage = relative || normalizeMentionPath(path);
  insertPlainToken(formatMentionPath(storage, options?.isDir));
}

watch(
  () => props.capturedContext?.ideContext?.workspace,
  () => {
    if (props.appearance === "overlay") {
      void loadWorkspaceState();
    }
  },
);

defineExpose({ focusInput, reset, setMessage, insertFileMention, resolveSendWorkspaceOptions });
</script>

<style scoped>
.chat-input-shell {
  position: relative;
  display: flex;
  flex-direction: column;
}

/* Pickers: workbench floats them; Alt+Alt overlay keeps them in document flow
   so input-mode window growth / chat-mode composer height actually wraps the list.
   Absolute `bottom: 100%` was clipped by overflow:hidden ancestors (and chat
   mode intentionally skipped native resize), so "/" and option menus vanished. */
.chat-input-shell.overlay-pickers :deep(.command-list) {
  position: absolute;
  z-index: 30;
  right: 0;
  bottom: 100%;
  left: 0;
  border: 1px solid var(--peek-border);
  border-bottom: 0;
  border-radius: 8px 8px 0 0;
  box-shadow: 0 -10px 28px color-mix(in srgb, #000 24%, transparent);
}

.chat-input-shell.overlay-composer.overlay-pickers :deep(.command-list) {
  position: relative;
  z-index: 1;
  right: auto;
  bottom: auto;
  left: auto;
  width: 100%;
  box-shadow: none;
}

/* Slash / @ / # pickers: fuse with the composer dock (same surface, no gray cap). */
.chat-input-shell.overlay-composer.command-suggestion-open :deep(.command-list),
.chat-input-shell.overlay-composer.file-suggestion-open :deep(.file-suggestion-list),
.chat-input-shell.overlay-composer.hash-suggestion-open :deep(.hash-suggestion-list) {
  --command-list-visible-rows: 5;
  margin: 0;
  padding: 4px 0 2px;
  border: none;
  border-bottom: 1px solid color-mix(in srgb, var(--peek-text) 10%, transparent);
  border-radius: 0;
  background: transparent;
  box-shadow: none;
}

.chat-input-shell.overlay-composer.command-suggestion-open :deep(.command-item),
.chat-input-shell.overlay-composer.file-suggestion-open :deep(.command-item),
.chat-input-shell.overlay-composer.hash-suggestion-open :deep(.command-item) {
  border-radius: 6px;
}

.chat-input-shell.overlay-composer.command-suggestion-open :deep(.command-item.active),
.chat-input-shell.overlay-composer.file-suggestion-open :deep(.command-item.active),
.chat-input-shell.overlay-composer.hash-suggestion-open :deep(.command-item.active) {
  background: color-mix(in srgb, var(--peek-text) 8%, var(--peek-surface));
}

.chat-input-shell.overlay-composer.command-suggestion-open :deep(.command-name) {
  color: var(--peek-text);
  font-weight: 600;
}

.chat-input-shell.overlay-composer.picker-open.command-suggestion-open .input-bar,
.chat-input-shell.overlay-composer.picker-open.file-suggestion-open .input-bar,
.chat-input-shell.overlay-composer.picker-open.hash-suggestion-open .input-bar {
  border-top: 0;
}

/* File / hash mention lists float above the input with a visible gap, and keep
   full border so they don't visually merge with (or cover) the input frame. */
.chat-input-shell.overlay-pickers :deep(.file-suggestion-list),
.chat-input-shell.overlay-pickers :deep(.hash-suggestion-list) {
  bottom: calc(100% + 6px);
  max-height: min(320px, 42vh);
  border-bottom: 1px solid var(--peek-border);
  border-radius: 8px;
}

.chat-input-shell.overlay-composer.overlay-pickers :deep(.file-suggestion-list),
.chat-input-shell.overlay-composer.overlay-pickers :deep(.hash-suggestion-list) {
  bottom: auto;
  max-height: min(320px, 42vh);
  border-bottom: 0;
  border-radius: 8px 8px 0 0;
}

.chat-input-shell.overlay-pickers :deep(.attach-resource-panel) {
  bottom: 100%;
  max-height: min(420px, 52vh);
}

/* Keep the attach panel in document flow so it cannot clip above the window /
   conversation viewport (same idea as ask/permission pickers). Absolute float
   made "+" look stuck when height was not reserved. */
.chat-input-shell.overlay-pickers.attach-panel-open :deep(.attach-resource-panel) {
  position: relative;
  z-index: 1;
  right: 0;
  bottom: auto;
  left: 0;
  width: 100%;
  max-height: min(360px, 46vh);
  border-bottom: 0;
  border-radius: 10px 10px 0 0;
  box-shadow: none;
}

/* Ask / permission / approval: fused with the input bar (top cap only). */
.chat-input-shell.overlay-pickers.interaction-request-open :deep(.ask-user-list),
.chat-input-shell.overlay-pickers.interaction-request-open :deep(.path-permission-list),
.chat-input-shell.overlay-pickers.interaction-request-open :deep(.tool-approval-list) {
  position: relative;
  z-index: 1;
  right: 0;
  bottom: auto;
  left: 0;
  width: 100%;
  max-height: var(--interaction-picker-max-height, min(420px, 48vh));
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
  margin: 0;
  /* Match .input-bar frame so fused sides read as one stroke. */
  border: 1px solid var(--peek-border);
  border-bottom: 0;
  border-radius: 14px 14px 0 0;
  background: var(--peek-surface);
  box-shadow: none;
}

.chat-input-shell.overlay-pickers.interaction-request-open :deep(.ask-user-list) {
  padding: 8px 10px 6px;
}

.chat-input-shell.overlay-pickers.interaction-request-open :deep(.ask-user-list .command-item) {
  border-radius: 8px;
}

.chat-input-shell.overlay-pickers.interaction-request-open
  :deep(.ask-user-list .picker-sticky-head),
.chat-input-shell.overlay-pickers.interaction-request-open :deep(.ask-user-list .picker-meta) {
  background: transparent;
}

.chat-input-shell.interaction-request-open.picker-open .input-bar {
  border-top-left-radius: 0;
  border-top-right-radius: 0;
  border-color: var(--peek-border);
  border-top-color: var(--peek-border);
}

.plan-mode-banner {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-items: center;
  justify-content: space-between;
  margin: 0 0 8px;
  padding: 10px 12px;
  border: 1px solid color-mix(in srgb, var(--peek-accent, #5b8def) 35%, var(--peek-border));
  border-radius: 10px;
  background: color-mix(in srgb, var(--peek-accent, #5b8def) 12%, var(--peek-panel, transparent));
}

.plan-mode-copy {
  display: flex;
  gap: 8px;
  align-items: flex-start;
  min-width: 0;
  flex: 1 1 220px;
}

.plan-mode-icon {
  flex-shrink: 0;
  margin-top: 1px;
  color: var(--peek-accent, #5b8def);
}

.plan-mode-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.plan-mode-text strong {
  font-size: 12px;
  font-weight: 600;
  line-height: 1.3;
  color: var(--peek-fg, inherit);
}

.plan-mode-text span {
  font-size: 11px;
  line-height: 1.35;
  color: var(--peek-muted, color-mix(in srgb, currentColor 65%, transparent));
}

.plan-mode-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  justify-content: flex-end;
}

.plan-mode-btn {
  height: 28px;
  padding: 0 10px;
  border-radius: 7px;
  border: 1px solid var(--peek-border, color-mix(in srgb, currentColor 18%, transparent));
  background: transparent;
  color: inherit;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
}

.plan-mode-btn:disabled {
  opacity: 0.55;
  cursor: default;
}

.plan-mode-btn.ghost:hover:not(:disabled) {
  background: color-mix(in srgb, currentColor 6%, transparent);
}

.plan-mode-btn.primary {
  border-color: color-mix(
    in srgb,
    var(--peek-accent, #5b8def) 50%,
    var(--peek-border, transparent)
  );
  background: color-mix(in srgb, var(--peek-accent, #5b8def) 22%, transparent);
}

.plan-mode-btn.primary:hover:not(:disabled) {
  background: color-mix(in srgb, var(--peek-accent, #5b8def) 32%, transparent);
}

.input-bar {
  box-sizing: border-box;
  flex: none;
  min-height: 82px;
  padding: 10px 10px 8px 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.input-footer,
.input-footer-primary,
.input-footer-actions {
  display: flex;
  align-items: center;
}

.input-content {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  align-content: flex-start;
  gap: 2px 4px;
  width: 100%;
  min-width: 0;
  min-height: 28px;
  line-height: 26px;
  overflow-x: clip;
  overflow-y: visible;
  cursor: text;
}

.input-footer {
  width: 100%;
  min-height: 28px;
  justify-content: space-between;
  gap: 8px;
  padding-top: 1px;
}

.input-footer-primary {
  min-width: 0;
  gap: 4px;
  flex-wrap: nowrap;
}

.input-footer-actions {
  min-width: 0;
  gap: 6px;
  flex-wrap: nowrap;
  align-items: center;
}

.conversation-token-count {
  flex: none;
  color: var(--peek-faint);
  font-size: 10px;
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
  user-select: none;
}

.chat-input {
  /* Fill leftover space on the current flex line so the empty region after
     typed text (and after chips) is still part of the focusable field. */
  flex: 1 1 6rem;
  display: block;
  box-sizing: border-box;
  width: 0;
  min-width: 4rem;
  max-width: 100%;
  height: 24px;
  margin: 0;
  padding: 0;
  border: 0;
  outline: none;
  background: transparent;
  color: var(--peek-text);
  font-family: var(--peek-font-sans);
  font-size: 14px;
  line-height: 24px;
  caret-color: var(--peek-accent);
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  word-break: break-word;
  overflow-x: clip;
}

/* Empty field with no selection chip: take the full row for a usable placeholder. */
.input-content:not(.has-chips) .chat-input.is-empty {
  flex: 1 1 100%;
  min-width: 100%;
  width: auto;
}

.chat-input::placeholder {
  color: var(--peek-placeholder);
  transition:
    color 160ms ease,
    opacity 160ms ease;
}

.workbench-composer .input-bar {
  min-height: 116px;
  max-height: min(280px, 100%);
  padding: 13px 13px 10px;
  gap: 9px;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--peek-text) 16%, transparent);
  border-radius: 16px;
  background: color-mix(in srgb, var(--peek-text) 7%, var(--peek-surface));
  box-shadow: 0 14px 36px color-mix(in srgb, #000 18%, transparent);
}

.workbench-composer .input-bar:focus-within {
  border-color: color-mix(in srgb, var(--peek-text) 28%, transparent);
  box-shadow: 0 16px 40px color-mix(in srgb, #000 22%, transparent);
}

.workbench-composer {
  --composer-line-height: 24px;
}

.workbench-composer .input-content {
  min-height: 28px;
  /* Hard cap so long input scrolls in place instead of growing the composer
     over whatever sits above it. */
  max-height: min(calc(var(--composer-line-height) * 8), 34vh);
  align-items: flex-start;
  overflow-x: clip;
  overflow-y: auto;
  overscroll-behavior: contain;
}

.chat-input-shell.overlay-composer {
  --composer-line-height: 24px;
  --composer-max-lines: 4;
  --composer-max-height: calc(var(--composer-line-height) * var(--composer-max-lines));
}

.chat-input-shell.overlay-composer .input-bar {
  min-height: 82px;
  padding: 10px 10px 8px 12px;
  box-sizing: border-box;
}

.overlay-composer .input-content {
  align-items: flex-start;
  align-content: flex-start;
  flex: 1 1 auto;
  min-height: var(--composer-line-height);
  max-height: var(--composer-max-height);
  overflow: visible;
}

.overlay-composer .composer-editable,
.overlay-composer .composer-textarea {
  flex: 1 1 100%;
  width: 100%;
  min-width: 100%;
  min-height: var(--composer-line-height);
  height: auto;
  max-height: var(--composer-max-height);
  overflow-x: clip;
  overflow-y: hidden;
  align-self: flex-start;
}

.workbench-composer .composer-textarea,
.workbench-composer .composer-editable.composer-textarea {
  flex: 1 1 6rem;
  display: block;
  box-sizing: border-box;
  /* width:0 + flex-grow fills the remainder of the line (clickable empty zone). */
  width: 0;
  min-width: 4rem;
  max-width: 100%;
  min-height: var(--composer-line-height);
  height: auto;
  max-height: calc(var(--composer-line-height) * 8);
  margin: 0;
  padding: 0;
  overflow-x: clip;
  overflow-y: auto;
  resize: none;
  font-size: 14px;
  line-height: 24px;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  word-break: break-word;
  field-sizing: fixed;
  align-self: center;
}

/* Selection chip precedes textarea, or field is multi-line — take a full row. */
.input-content.has-leading .composer-textarea,
.input-content.has-leading .composer-editable,
.input-content .composer-textarea.is-multiline,
.input-content .composer-editable.is-multiline {
  flex: 1 1 100%;
  min-width: 100%;
  width: auto;
  field-sizing: fixed;
}

.input-content:not(.has-chips):not(.has-leading) .composer-textarea.is-empty,
.input-content:not(.has-chips):not(.has-leading) .composer-editable.is-empty {
  flex: 1 1 100%;
  min-width: 100%;
  width: auto;
  field-sizing: fixed;
}

.workbench-composer .input-footer {
  min-height: 30px;
  align-items: flex-end;
  padding-top: 2px;
}

.workbench-composer .input-footer-primary {
  flex: 1;
  gap: 3px;
  flex-wrap: wrap;
  overflow: visible;
}

.workbench-composer .input-footer-actions {
  flex: none;
  gap: 8px;
}

.workbench-composer .footer-chip {
  height: 28px;
  padding-right: 8px;
  padding-left: 8px;
  border-radius: 6px;
}

.workbench-composer .model-badge {
  max-width: 164px;
}

.workbench-composer.overlay-pickers :deep(.command-list) {
  right: 12px;
  bottom: calc(100% + 8px);
  left: 12px;
  width: auto;
  max-height: none;
  padding: 6px;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--peek-text) 12%, transparent);
  border-radius: 12px;
  background: color-mix(in srgb, var(--peek-surface) 94%, transparent);
  box-shadow:
    0 10px 28px color-mix(in srgb, #000 16%, transparent),
    0 1px 0 color-mix(in srgb, #fff 4%, transparent) inset;
  backdrop-filter: blur(12px);
}

.workbench-composer.overlay-pickers :deep(.file-suggestion-list),
.workbench-composer.overlay-pickers :deep(.hash-suggestion-list) {
  bottom: calc(100% + 6px);
  max-height: min(320px, 42vh);
  border-bottom: 1px solid color-mix(in srgb, var(--peek-text) 7%, transparent);
  border-radius: 10px;
  box-shadow: 0 10px 28px color-mix(in srgb, #000 14%, transparent);
}

.workbench-composer.overlay-pickers.attach-panel-open :deep(.attach-resource-panel) {
  position: absolute;
  right: 0;
  bottom: calc(100% + 8px);
  left: 0;
  width: auto;
  max-height: min(340px, 44vh);
  overflow-x: hidden;
  overflow-y: auto;
  border: 1px solid color-mix(in srgb, var(--peek-text) 12%, transparent);
  border-radius: 14px;
  background: color-mix(in srgb, var(--peek-surface) 94%, transparent);
  box-shadow:
    0 10px 28px color-mix(in srgb, #000 16%, transparent),
    0 1px 0 color-mix(in srgb, #fff 4%, transparent) inset;
  backdrop-filter: blur(12px);
}

/* Interaction requests: fused top cap above the composer (in-flow). */
.workbench-composer.overlay-pickers.interaction-request-open :deep(.ask-user-list),
.workbench-composer.overlay-pickers.interaction-request-open :deep(.path-permission-list),
.workbench-composer.overlay-pickers.interaction-request-open :deep(.tool-approval-list) {
  right: 0;
  left: 0;
  width: 100%;
  max-height: var(--interaction-picker-max-height, min(420px, 48vh));
  margin: 0;
  /* Same stroke as .workbench-composer .input-bar */
  border: 1px solid color-mix(in srgb, var(--peek-text) 16%, transparent);
  border-bottom: 0;
  border-radius: 16px 16px 0 0;
  background: color-mix(in srgb, var(--peek-text) 7%, var(--peek-surface));
  box-shadow: none;
}

.workbench-composer.overlay-pickers.interaction-request-open :deep(.ask-user-list) {
  padding: 8px 10px 6px;
}

.workbench-composer.overlay-pickers :deep(.ask-user-list .picker-sticky-head),
.workbench-composer.overlay-pickers :deep(.ask-user-list .picker-meta) {
  background: transparent;
}

.workbench-composer.picker-open:not(.file-suggestion-open):not(.hash-suggestion-open):not(
    .command-suggestion-open
  ):not(.attach-panel-open)
  .input-bar {
  border-color: color-mix(in srgb, var(--peek-text) 16%, transparent);
  border-top-color: transparent;
  border-radius: 0 0 16px 16px;
  box-shadow: none;
}

.workbench-composer.attach-panel-open.picker-open .input-bar {
  /* Attach panel floats above — keep the composer frame intact. */
  border-top-color: color-mix(in srgb, var(--peek-text) 16%, transparent);
  border-radius: 16px;
}

.workbench-composer.interaction-request-open.picker-open .input-bar {
  border-top-left-radius: 0;
  border-top-right-radius: 0;
  /* Keep outer frame continuous; middle seam uses the same stroke. */
  border-color: color-mix(in srgb, var(--peek-text) 16%, transparent);
  border-top-color: color-mix(in srgb, var(--peek-text) 16%, transparent);
  box-shadow: 0 14px 36px color-mix(in srgb, #000 18%, transparent);
}

.workbench-composer.overlay-pickers :deep(.command-item) {
  border-radius: 6px;
}

.workbench-composer.interaction-request-open.overlay-pickers :deep(.ask-user-list .command-item) {
  border-radius: 8px;
}

.workbench-composer.overlay-pickers :deep(.command-item.active) {
  background: color-mix(in srgb, var(--peek-accent) 10%, var(--peek-surface));
}

.workbench-composer.overlay-pickers :deep(.command-item.current:not(.active)) {
  background: transparent;
}

.workbench-composer.chip-picker-open :deep(.command-list) {
  position: absolute;
  z-index: 40;
  right: auto;
  bottom: var(--chip-picker-bottom);
  left: var(--chip-picker-left);
  width: min(var(--chip-picker-width), calc(100% - 16px));
  max-height: min(280px, calc(100vh - 32px));
  padding: 5px;
  overflow-y: auto;
  border: 1px solid color-mix(in srgb, var(--peek-text) 12%, transparent);
  border-radius: 8px;
  background: var(--peek-surface);
  box-shadow: 0 12px 30px color-mix(in srgb, #000 20%, transparent);
}

.workbench-composer.chip-picker-open :deep(.command-item) {
  border-radius: 5px;
}

/* The overlay keeps its compact, composer-attached picker independent from
   the anchored workbench menus. Input mode stays in normal document flow so
   the native Alt+Alt window can grow upward around the list. */
.overlay-composer :deep(.model-picker-list) {
  --command-list-visible-rows: 8;
  max-height: min(
    calc(
      var(--command-row-height) * var(--command-list-visible-rows) + var(--command-list-padding) +
        34px
    ),
    72vh
  );
}

.overlay-composer.overlay-pickers.chip-picker-open :deep(.command-list) {
  position: relative;
  right: auto;
  bottom: auto;
  left: auto;
  width: 100%;
  padding: 4px 0;
  border: 1px solid var(--peek-border);
  border-bottom: 0;
  border-radius: 8px 8px 0 0;
  background: var(--peek-list-bg);
  box-shadow: none;
}

.overlay-composer.overlay-pickers.interaction-request-open :deep(.ask-user-list),
.overlay-composer.overlay-pickers.interaction-request-open :deep(.path-permission-list),
.overlay-composer.overlay-pickers.interaction-request-open :deep(.tool-approval-list) {
  margin: 0;
  border: 1px solid var(--peek-border);
  border-bottom: 0;
  border-radius: 14px 14px 0 0;
  background: var(--peek-surface);
}

.overlay-composer.overlay-pickers.interaction-request-open :deep(.ask-user-list) {
  padding: 8px 10px 6px;
}

.overlay-composer.overlay-pickers :deep(.ask-user-list .picker-sticky-head),
.overlay-composer.overlay-pickers :deep(.ask-user-list .picker-meta) {
  background: transparent;
}

.overlay-composer.interaction-request-open.picker-open .input-bar {
  border-top-left-radius: 0;
  border-top-right-radius: 0;
  border-color: var(--peek-border);
  border-top-color: var(--peek-border);
}

.overlay-composer.interaction-request-open.overlay-pickers :deep(.ask-user-list .command-item) {
  border-radius: 8px;
}

@media (max-width: 760px) {
  .workbench-composer .input-footer {
    align-items: flex-end;
  }
  .workbench-composer .model-name {
    max-width: 88px;
  }
}

@media (max-height: 700px) {
  .workbench-composer .input-bar {
    min-height: 88px;
    padding-top: 9px;
  }
  .workbench-composer .input-content {
    min-height: 30px;
  }
  .workbench-composer .composer-textarea {
    min-height: 28px;
    max-height: 64px;
  }
}

@media (max-height: 420px) {
  .workbench-composer .input-bar {
    min-height: 76px;
    padding: 7px 10px 6px;
    gap: 5px;
  }
  .workbench-composer .input-content {
    min-height: 24px;
  }
  .workbench-composer .composer-textarea {
    min-height: 24px;
    max-height: 44px;
    line-height: 20px;
  }
  .workbench-composer .input-footer {
    min-height: 28px;
    padding-top: 0;
  }
}

.model-picker {
  flex: none;
  min-width: 0;
}

.thinking-tier-slot {
  flex: none;
  min-width: 0;
}

.thinking-tier-slot.dormant {
  display: none;
}

.approval-slot {
  flex: none;
  min-width: 0;
}

.approval-slot.dormant {
  display: none;
}

.footer-chip-icon-slot {
  flex: none;
  width: 13px;
  height: 13px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

/* Shared ghost-chip language for footer controls */
.footer-chip {
  height: 26px;
  border-radius: 7px;
  border: 1px solid transparent;
  background: transparent;
  color: var(--peek-muted);
  font-family: var(--peek-font-sans);
  font-size: 11px;
  font-weight: 500;
  letter-spacing: 0.01em;
  line-height: 16px;
  transition:
    border-color 160ms ease,
    color 160ms ease,
    background 160ms ease,
    box-shadow 160ms ease;
}

.footer-chip-icon {
  flex: none;
  opacity: 0.78;
  transition:
    opacity 140ms ease,
    color 140ms ease;
}

.footer-chip:hover .footer-chip-icon,
.footer-chip.open .footer-chip-icon,
.footer-chip.active .footer-chip-icon {
  opacity: 1;
}

.footer-chip.active {
  border-color: color-mix(in srgb, var(--peek-accent, currentColor) 32%, transparent);
}

.footer-chip-icon-only {
  width: 26px;
  padding: 0;
  justify-content: center;
  max-width: none;
}

.workspace-control {
  position: relative;
  flex: none;
  height: 26px;
  max-width: 108px;
  min-width: 0;
  display: inline-flex;
  align-items: center;
  border-radius: 6px;
  background: transparent;
  color: var(--peek-muted);
  transition:
    background 120ms ease,
    color 120ms ease;
}

.input-bar.drag-over {
  outline: 1px dashed color-mix(in srgb, var(--peek-accent) 55%, transparent);
  outline-offset: -2px;
}

.input-files {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  width: 100%;
  max-height: 64px;
  overflow-x: hidden;
  overflow-y: auto;
  padding: 0;
}

.file-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  max-width: min(220px, 100%);
  height: 26px;
  padding: 0 4px 0 8px;
  border: 1px solid var(--peek-border);
  border-radius: 7px;
  background: color-mix(in srgb, var(--peek-input-bg) 72%, var(--peek-surface));
  color: var(--peek-text);
  font-size: 12px;
  line-height: 18px;
  transition:
    border-color 120ms ease,
    background 120ms ease,
    opacity 120ms ease;
}

.file-chip:hover {
  border-color: color-mix(in srgb, var(--peek-accent) 28%, var(--peek-border));
  background: color-mix(in srgb, var(--peek-accent) 8%, var(--peek-input-bg));
}

.file-chip.skipped {
  opacity: 0.55;
}

.file-chip-icon {
  flex: none;
  color: var(--peek-muted);
}

.file-chip-icon-img {
  flex: none;
  width: 14px;
  height: 14px;
  object-fit: contain;
  display: block;
}

.file-chip-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 500;
  letter-spacing: 0.01em;
  line-height: 18px;
  padding: 1px 0;
}

.file-chip-remove {
  display: inline-flex;
  flex: none;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  margin: 0;
  padding: 0;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
  opacity: 0.55;
  transition:
    opacity 120ms ease,
    background 120ms ease,
    color 120ms ease;
}

.file-chip:hover .file-chip-remove {
  opacity: 0.9;
}

.file-chip-remove:hover {
  opacity: 1;
  color: var(--peek-text);
  background: color-mix(in srgb, var(--peek-muted) 16%, transparent);
}

.selection-tag {
  display: inline-flex;
  flex: 0 0 auto;
  align-items: center;
  height: 24px;
  margin: 1px 0;
  padding: 0 8px;
  border: 1px solid color-mix(in srgb, var(--peek-accent) 28%, var(--peek-border));
  border-radius: 6px;
  background: color-mix(in srgb, var(--peek-accent) 10%, transparent);
  color: var(--peek-accent);
  font-size: 12px;
  font-weight: 550;
  line-height: 24px;
  white-space: nowrap;
  vertical-align: middle;
}

.workspace-control:hover {
  background: color-mix(in srgb, var(--peek-accent) 12%, transparent);
  color: var(--peek-text);
}

.workspace-control.active {
  background: color-mix(in srgb, var(--peek-accent) 18%, transparent);
  color: var(--peek-accent);
}

.workspace-btn {
  min-width: 0;
  max-width: 100%;
  height: 26px;
  border: 0;
  border-radius: inherit;
  background: transparent;
  color: inherit;
  padding: 0 6px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  overflow: hidden;
}

.workspace-name {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 11px;
  font-weight: 500;
  line-height: 16px;
}

.workspace-exit-btn {
  position: absolute;
  top: -6px;
  right: -6px;
  z-index: 2;
  width: 17px;
  height: 17px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: 1px solid var(--peek-border);
  border-radius: 50%;
  background: var(--peek-surface);
  color: var(--peek-muted);
  cursor: pointer;
  opacity: 0;
  pointer-events: none;
  transform: scale(0.72);
  transition:
    background 120ms ease,
    border-color 120ms ease,
    color 120ms ease,
    opacity 120ms ease,
    transform 120ms ease;
}

.workspace-control:hover .workspace-exit-btn,
.workspace-control:focus-within .workspace-exit-btn {
  opacity: 1;
  pointer-events: auto;
  transform: scale(1);
}

.workspace-exit-btn:hover {
  border-color: var(--destructive);
  background: var(--destructive);
  color: white;
}

.model-badge {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  max-width: 148px;
  min-width: 0;
  margin: 0;
  padding: 0 7px;
  user-select: none;
  cursor: pointer;
  appearance: none;
}

.model-badge > svg,
.model-badge .footer-chip-icon {
  flex: none;
}

.model-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  line-height: 16px;
  animation: model-name-fade 160ms ease;
}

@keyframes model-name-fade {
  from {
    opacity: 0.45;
  }
  to {
    opacity: 1;
  }
}

.model-badge.confirm {
  border-color: color-mix(in srgb, var(--peek-accent) 45%, transparent);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--peek-accent) 22%, transparent);
  color: var(--peek-text);
}

.model-chevron {
  flex: none;
  opacity: 0.45;
  transition:
    transform 160ms ease,
    opacity 140ms ease;
}

.model-badge:hover .model-chevron,
.model-badge.open .model-chevron {
  opacity: 0.8;
}

.model-badge.open .model-chevron {
  transform: rotate(180deg);
}

.model-badge:hover,
.model-badge.open {
  border-color: color-mix(in srgb, var(--peek-border) 80%, transparent);
  color: var(--peek-text);
  background: color-mix(in srgb, var(--peek-text) 5%, transparent);
}

.model-badge.open {
  border-color: color-mix(in srgb, var(--peek-accent) 28%, transparent);
  background: color-mix(in srgb, var(--peek-accent) 10%, transparent);
  box-shadow: inset 0 0 0 0.5px color-mix(in srgb, var(--peek-accent) 12%, transparent);
}

.context-label {
  flex: none;
  height: 26px;
  display: inline-flex;
  align-items: center;
  padding: 0 7px;
  border-radius: 7px;
  background: color-mix(in srgb, var(--peek-text) 4%, transparent);
  color: var(--peek-muted);
  font-family: var(--peek-font-sans);
  font-size: 10px;
  font-weight: 500;
  letter-spacing: 0.02em;
  line-height: 16px;
  user-select: none;
}

.send-btn {
  flex: none;
  width: 26px;
  height: 26px;
  border: 0;
  border-radius: 50%;
  background: var(--peek-send-bg);
  color: var(--peek-send-fg);
  padding: 0;
  cursor: default;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transform: translateZ(0);
  transition:
    background 120ms ease,
    color 120ms ease,
    transform 140ms cubic-bezier(0.22, 1, 0.36, 1);
}

.attach-trigger-btn {
  flex: none;
  width: 26px;
  height: 26px;
  border: 0;
  border-radius: 50%;
  background: var(--peek-send-bg);
  color: var(--peek-send-fg);
  padding: 0;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transform: translateZ(0);
  transition:
    background 120ms ease,
    color 120ms ease,
    transform 140ms cubic-bezier(0.22, 1, 0.36, 1);
}

.attach-trigger-btn:hover {
  transform: scale(1.03);
}

.attach-trigger-btn:active {
  transform: scale(0.97);
}

.attach-trigger-btn.open {
  background: color-mix(in srgb, var(--peek-accent) 18%, var(--peek-send-bg));
  color: var(--peek-accent);
  transform: rotate(45deg);
}

.attach-trigger-btn.open:hover {
  transform: rotate(45deg) scale(1.03);
}

.attach-trigger-btn.open:active {
  transform: rotate(45deg) scale(0.97);
}

@media (prefers-reduced-motion: reduce) {
  .attach-trigger-btn,
  .attach-trigger-btn:hover,
  .attach-trigger-btn:active,
  .attach-trigger-btn.open,
  .attach-trigger-btn.open:hover,
  .attach-trigger-btn.open:active {
    transition: none;
    transform: none;
  }

  .attach-trigger-btn.open {
    transform: none;
  }
}

.send-btn svg {
  width: 18px;
  height: 18px;
}

.send-btn.active {
  background: var(--peek-send-active-bg);
  color: var(--peek-send-active-fg);
  cursor: pointer;
}

.send-btn.pause {
  background: color-mix(in srgb, var(--destructive) 18%, var(--peek-send-bg));
  color: color-mix(in srgb, var(--destructive) 85%, var(--peek-send-fg));
  cursor: pointer;
}

.send-btn.active:hover:not(:disabled),
.send-btn.pause:hover:not(:disabled) {
  transform: scale(1.03);
}

.send-btn.active:active:not(:disabled),
.send-btn.pause:active:not(:disabled) {
  transform: scale(0.97);
}

@media (prefers-reduced-motion: reduce) {
  .send-btn,
  .send-btn.active:hover:not(:disabled),
  .send-btn.pause:hover:not(:disabled),
  .send-btn.active:active:not(:disabled),
  .send-btn.pause:active:not(:disabled) {
    transition: none;
    transform: none;
  }
}
</style>
<style>
/* Image thumbnail area in input bar */
.input-images {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin: 0;
  width: 100%;
  padding: 1px 0 2px;
  max-height: 84px;
  overflow-y: auto;
}

.image-thumb-container {
  position: relative;
  flex: none;
  width: 52px;
  height: 52px;
  border-radius: 10px;
  overflow: hidden;
  border: 1px solid var(--peek-border);
  background: color-mix(in srgb, var(--peek-surface) 70%, transparent);
  /* WebView2: force content to clip to radius */
  transform: translateZ(0);
  transition: border-color 140ms ease;
}

.image-thumb-container:hover {
  border-color: color-mix(in srgb, var(--peek-accent) 55%, var(--peek-border));
}

.image-thumb {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
  border-radius: inherit;
  cursor: zoom-in;
}

.image-remove-btn {
  position: absolute;
  top: 2px;
  right: 2px;
  width: 15px;
  height: 15px;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.55);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  border: none;
  padding: 0;
  opacity: 0.8;
  transition:
    opacity 120ms ease,
    background 120ms ease;
}

.image-thumb-container:hover .image-remove-btn {
  opacity: 1;
  background: rgba(0, 0, 0, 0.75);
}

.image-remove-btn:hover {
  background: rgba(239, 68, 68, 0.9) !important; /* soft red on hover */
}
</style>
