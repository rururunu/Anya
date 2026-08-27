<template>
  <div
    class="peek-panel"
    :class="{
      chat: mode === 'chat',
      'sidebar-open': sidebarOpen,
      'minimize-preview': isMinimizePreview,
    }"
    :style="panelStyle"
    data-tauri-drag-region
    @mousedown="onWindowDragMouseDown"
  >
    <div
      v-show="isMinimizePreview"
      class="minimize-preview-screen"
      data-tauri-drag-region
      aria-hidden="true"
    >
      <span class="minimize-preview-title" data-tauri-drag-region>{{ chatTitle }}</span>
    </div>

    <div class="chat-main">
      <section
        v-if="mode === 'chat'"
        key="thread"
        class="thread-panel peek-surface"
        :class="{
          glass: isGlass,
          'has-messages': hasVisibleMessages,
        }"
        data-tauri-drag-region
      >
        <header class="thread-header" data-tauri-drag-region @mousedown="onWindowDragMouseDown">
          <div class="header-tools" data-tauri-drag-region="false">
            <button
              type="button"
              class="window-btn"
              :disabled="!activeSessionId"
              :aria-label="tr(settingStore.language, 'chat.openInWorkbench')"
              :title="tr(settingStore.language, 'chat.openInWorkbench')"
              data-tauri-drag-region="false"
              @mousedown.stop.prevent="openInWorkbench"
            >
              <AppWindow :size="13" :stroke-width="1.8" />
            </button>
            <button
              type="button"
              class="window-btn sidebar-toggle-btn"
              :class="{ active: sidebarOpen }"
              :aria-label="sidebarViewsLabel"
              :title="sidebarViewsLabel"
              data-tauri-drag-region="false"
              @mousedown.stop.prevent="toggleSidebar"
            >
              <PanelRight :size="13" :stroke-width="1.8" />
              <span v-if="runningSubagentCount" class="running-dot" aria-hidden="true" />
            </button>
          </div>
          <div class="window-controls" data-tauri-drag-region="false">
            <button
              type="button"
              class="window-btn btn-minimize"
              :aria-label="tr(settingStore.language, 'minimize')"
              data-tauri-drag-region="false"
              @mousedown.stop.prevent="minimize"
            >
              <Minus :size="12" />
            </button>
            <button
              type="button"
              class="window-btn"
              :class="{ active: isAlwaysOnTop }"
              :aria-pressed="isAlwaysOnTop"
              :aria-label="tr(settingStore.language, isAlwaysOnTop ? 'unpinWindow' : 'pinWindow')"
              :title="tr(settingStore.language, isAlwaysOnTop ? 'unpinWindow' : 'pinWindow')"
              data-tauri-drag-region="false"
              @mousedown.stop.prevent="toggleAlwaysOnTop"
            >
              <PinOff v-if="isAlwaysOnTop" :size="12" />
              <Pin v-else :size="12" />
            </button>
            <button
              type="button"
              class="window-btn close"
              :aria-label="tr(settingStore.language, 'close')"
              data-tauri-drag-region="false"
              @mousedown.stop.prevent="close"
            >
              <X :size="12" />
            </button>
          </div>
        </header>

        <div
          v-if="contextNotice"
          class="context-notice"
          role="status"
          data-tauri-drag-region="false"
        >
          <CircleAlert :size="14" :stroke-width="1.8" aria-hidden="true" />
          <span>{{ contextNotice }}</span>
        </div>

        <div class="thread-content">
          <AppErrorBoundary compact>
            <MessageList
              :messages="messages"
              :session-id="activeSessionId"
              :workspace-name="workspaceDisplayName"
              :checkpoints="checkpoints"
              @rewound="handleRewound"
              @branch="handleBranchMessage"
              @review-changes="openDiffSidebar"
              @review-file="openDiffSidebarFile"
              @inspect-subagent="openSubagentSidebar"
              @preview-image="handlePreviewImage"
              @edit-from-image="handleEditFromImage"
            />
          </AppErrorBoundary>
          <Transition name="workspace-sidebar" @after-leave="emitComposerLayout">
            <div
              v-show="sidebarOpen"
              class="workspace-sidebar-shell"
              :style="{ width: `${diffSidebarWidth + DIFF_RESIZE_HANDLE_WIDTH}px` }"
            >
              <div
                class="diff-resize-handle"
                :class="{ active: diffSidebarResizing }"
                role="separator"
                aria-orientation="vertical"
                :aria-label="tr(settingStore.language, 'resizeCodeChanges')"
                :title="tr(settingStore.language, 'resizeCodeChanges')"
                :aria-valuemin="DIFF_SIDEBAR_MIN_WIDTH"
                :aria-valuemax="DIFF_SIDEBAR_MAX_WIDTH"
                :aria-valuenow="Math.round(diffSidebarWidth)"
                tabindex="0"
                data-tauri-drag-region="false"
                @pointerdown="startDiffSidebarResize"
                @keydown="handleDiffSidebarResizeKey"
                @dblclick="resetDiffSidebarWidth"
              />
              <aside
                class="workspace-sidebar"
                :style="{ width: `${diffSidebarWidth}px` }"
                data-tauri-drag-region="false"
              >
                <nav class="workspace-sidebar-tabs peek-card-tabs" :aria-label="sidebarViewsLabel">
                  <button
                    type="button"
                    class="workspace-view-tab peek-card-tab"
                    :class="{ active: sidebarTab === 'diff' }"
                    :title="diffTabLabel"
                    @click="selectSidebarTab('diff')"
                  >
                    <FileDiff :size="13" />
                    <span>{{ diffTabLabel }}</span>
                  </button>
                  <button
                    type="button"
                    class="workspace-view-tab peek-card-tab"
                    :class="{ active: sidebarTab === 'subagents' }"
                    :title="subagentTabLabel"
                    @click="selectSidebarTab('subagents')"
                  >
                    <SubagentIcon :status="runningSubagentCount ? 'running' : 'idle'" :size="13" />
                    <span>{{ subagentTabLabel }}</span>
                  </button>
                  <button
                    v-if="openedImageSources.length"
                    type="button"
                    class="workspace-view-tab peek-card-tab"
                    :class="{ active: sidebarTab === 'image' }"
                    :title="imageTabLabel"
                    @click="selectSidebarTab('image')"
                  >
                    <ImageIcon :size="13" />
                    <span>{{ imageTabLabel }}</span>
                  </button>
                  <button
                    v-if="runtimeDebugEnabled"
                    type="button"
                    class="workspace-view-tab peek-card-tab"
                    :class="{ active: sidebarTab === 'runtime' }"
                    :title="runtimeTabLabel"
                    @click="selectSidebarTab('runtime')"
                  >
                    <Bug :size="13" />
                    <span>{{ runtimeTabLabel }}</span>
                  </button>
                  <button
                    type="button"
                    class="sidebar-close-button"
                    :aria-label="sidebarCloseLabel"
                    :title="sidebarCloseLabel"
                    @click="closeSidebar"
                  >
                    <X :size="13" />
                  </button>
                </nav>
                <div class="workspace-sidebar-content">
                  <CodeDiffSidebar
                    v-show="sidebarTab === 'diff'"
                    :messages="messages"
                    :width="diffSidebarWidth"
                    :focus-path="diffFocusPath"
                    :focus-at="diffFocusAt"
                    embedded
                  />
                  <SubagentSidebar
                    v-show="sidebarTab === 'subagents'"
                    :activities="subagentActivities"
                    :all-activities="allToolActivities"
                    :opened-entry-ids="openedSubagentIds"
                    :selected-entry-id="selectedSubagentId"
                    embedded
                    @close-entry="closeSubagentTab"
                  />
                  <AgentDebugPanel
                    v-if="runtimeDebugEnabled"
                    v-show="sidebarTab === 'runtime'"
                    embedded
                  />
                  <ImagePreviewSidebar
                    v-show="sidebarTab === 'image'"
                    :sources="openedImageSources"
                    :selected-source="selectedImageSource"
                    @select="selectedImageSource = $event"
                    @close="closeImageTab"
                  />
                </div>
              </aside>
            </div>
          </Transition>
        </div>
      </section>

      <div
        ref="dockRef"
        class="composer-dock peek-surface"
        :class="{ expanded: mode === 'chat', glass: isGlass && mode !== 'chat' }"
      >
        <p v-if="contextPreview" class="captured-context-preview" data-tauri-drag-region="false">
          {{ contextPreview }}
        </p>
        <ChatInputBar
          ref="inputRef"
          appearance="overlay"
          :sending="sending"
          :session-id="activeSessionId"
          :captured-context="capturedContext"
          :context-ready="contextReady"
          overlay-pickers
          :placeholder="
            tr(settingStore.language, mode === 'chat' ? 'continueQuestion' : 'askAnything')
          "
          :close-on-escape="mode === 'input'"
          :ask-user="askUserSession"
          :path-permission="pathPermissionSession"
          :tool-approval="toolApprovalSession"
          :history-sessions="historySessions"
          :show-workspace-button="mode === 'input'"
          :selection-lines="selectionLines"
          @submit="handleSubmit"
          @pause="handlePause"
          @close="emit('close')"
          @layout-change="handleLayoutChange"
          @ask-user-complete="handleAskUserComplete"
          @path-permission-complete="handlePathPermissionComplete"
          @tool-approval-complete="handleToolApprovalComplete"
          @open-history="handleOpenHistory"
          @history-select="handleHistorySelect"
          @history-close="handleHistoryClose"
          @remove-selection="emit('selectionRemoved')"
          @show-context="handleShowContext"
          @preview-image="handlePreviewImage"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { defineAsyncComponent, computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { storeToRefs } from "pinia";
import {
  AppWindow,
  Bug,
  CircleAlert,
  FileDiff,
  Image as ImageIcon,
  Minus,
  PanelRight,
  Pin,
  PinOff,
  X,
} from "@lucide/vue";
import AppErrorBoundary from "@/components/AppErrorBoundary.vue";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { listen } from "@tauri-apps/api/event";
import ChatInputBar, {
  type AskUserSession,
  type PathPermissionSession,
} from "@/components/chat/ChatInputBar.vue";
import CodeDiffSidebar from "@/components/chat/CodeDiffSidebar.vue";
import AgentDebugPanel from "@/components/chat/AgentDebugPanel.vue";
import SubagentSidebar from "@/components/chat/SubagentSidebar.vue";
import SubagentIcon from "@/components/chat/SubagentIcon.vue";
import ImagePreviewSidebar from "@/components/chat/ImagePreviewSidebar.vue";
import { gsapOverlayDockReveal } from "@/services/motion/gsapPresets";
import { onWindowDragMouseDown } from "@/services/overlay/windowDrag";
import { fetchChatSessions } from "@/commands/slash";
import {
  listenAskUser,
  listenInteractionResolved,
  listenPathPermission,
  listenToolApproval,
} from "@/services/ipc/events";
import {
  chatCancel,
  listCheckpoints,
  minimizeOverlay,
  openSessionInWorkbench,
  respondAskUser,
  respondPathPermission,
  respondToolApproval,
  setWindowSessionView,
  setOverlayPopupOpen,
  openImagePreview,
  branchChatSession,
} from "@/services/ipc";
import { useChatStore } from "@/stores/chat";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";
import { SUBAGENT_TOOLS } from "@/services/chat/subagentTools";
import type {
  AskUserAnswerItem,
  CapturedContext,
  ChatSessionSummary,
  CheckpointInfo,
  PathPermissionDecision,
  ToolApprovalDecision,
  ToolApprovalSession,
} from "@/types/chat";
import {
  attachSelection,
  parseSelectionAttachment,
  selectionLineCount,
} from "@/services/chat/selectionAttachment";

// Lazy: keeps Markdown/echarts out of the Alt+Alt input-mode boot path.
const MessageList = defineAsyncComponent(() => import("@/components/chat/MessageList.vue"));

const props = defineProps<{
  mode: "input" | "chat";
  sessionId: string;
  capturedContext?: CapturedContext | null;
  contextReady?: boolean;
}>();

const emit = defineEmits<{
  layoutChange: [
    payload: {
      showSuggestions: boolean;
      suggestionCount: number;
      showModelMenu: boolean;
      modelMenuHeight: number;
      askUserRowCount: number;
      pickerRowCount: number;
      pickerHeight?: number;
      hasContextPreview: boolean;
      mode: "input" | "chat";
      diffSidebarOpen: boolean;
      subagentSidebarOpen: boolean;
      runtimeSidebarOpen: boolean;
      imageSidebarOpen: boolean;
      sidebarWidth: number;
      hasImages?: boolean;
      hasFiles?: boolean;
    },
  ];
  close: [];
  enterChat: [sessionId: string];
  contextConsumed: [];
  selectionRemoved: [];
}>();

const chatStore = useChatStore();
const settingStore = useSettingStore();
const { sessions, overlayDraftSessionId, overlayContextNotice } = storeToRefs(chatStore);
// The runtime/debug sidebar tab is a development aid; hide it in packaged builds.
const runtimeDebugEnabled = import.meta.env.DEV;

const inputRef = ref<InstanceType<typeof ChatInputBar> | null>(null);
const dockRef = ref<HTMLElement | null>(null);
const panelVisible = ref(false);
const isMinimizePreview = ref(false);
const isAlwaysOnTop = ref(true);
const MINIMIZE_PREVIEW_MS = 64;
let minimizeTimer: ReturnType<typeof setTimeout> | null = null;
const askUserSession = ref<AskUserSession | null>(null);
const askUserSubmitting = ref(false);
const pathPermissionSession = ref<PathPermissionSession | null>(null);
const pathPermissionSubmitting = ref(false);
const toolApprovalSession = ref<ToolApprovalSession | null>(null);
const toolApprovalSubmitting = ref(false);
const checkpoints = ref<CheckpointInfo[]>([]);
const historySessions = ref<ChatSessionSummary[] | null>(null);
const PUBLIC_HISTORY_LIMIT = 10;
const diffSidebarOpen = ref(false);
const subagentSidebarOpen = ref(false);
const runtimeSidebarOpen = ref(false);
const imageSidebarOpen = ref(false);
const openedImageSources = ref<string[]>([]);
const selectedImageSource = ref("");
const openedSubagentIds = ref<string[]>([]);
const selectedSubagentId = ref("");
const diffFocusPath = ref("");
const diffFocusAt = ref(0);
type SidebarTab = "diff" | "subagents" | "runtime" | "image";
const sidebarTab = ref<SidebarTab>("diff");
const sidebarOpen = computed(
  () =>
    diffSidebarOpen.value ||
    subagentSidebarOpen.value ||
    runtimeSidebarOpen.value ||
    imageSidebarOpen.value,
);
const panelStyle = computed(() => ({
  "--workspace-sidebar-width": sidebarOpen.value
    ? String(diffSidebarWidth.value + DIFF_RESIZE_HANDLE_WIDTH) + "px"
    : "0px",
}));
const diffSidebarResizing = ref(false);
const DIFF_SIDEBAR_DEFAULT_WIDTH = 720;
const DIFF_SIDEBAR_MIN_WIDTH = 420;
const DIFF_SIDEBAR_MAX_WIDTH = 1000;
const CHAT_PANE_MIN_WIDTH = 540;
const DIFF_RESIZE_HANDLE_WIDTH = 7;
const DIFF_SIDEBAR_WIDTH_KEY = "anya.diffSidebarWidth.v3";
const storedDiffSidebarWidth = Number(localStorage.getItem(DIFF_SIDEBAR_WIDTH_KEY));
const diffSidebarWidth = ref(
  Number.isFinite(storedDiffSidebarWidth)
    ? Math.min(DIFF_SIDEBAR_MAX_WIDTH, Math.max(DIFF_SIDEBAR_MIN_WIDTH, storedDiffSidebarWidth))
    : DIFF_SIDEBAR_DEFAULT_WIDTH,
);
let diffResizeStartX = 0;
let diffResizeStartWidth = DIFF_SIDEBAR_DEFAULT_WIDTH;

const isGlass = computed(() => settingStore.opacity < 100);
const activeSessionId = computed(() => overlayDraftSessionId.value || props.sessionId);
const messages = computed(() => {
  const sessionId = activeSessionId.value;
  if (!sessionId) {
    return [];
  }
  return sessions.value[sessionId] ?? [];
});
watch(activeSessionId, () => {
  openedSubagentIds.value = [];
  selectedSubagentId.value = "";
  openedImageSources.value = [];
  selectedImageSource.value = "";
  if (sidebarTab.value === "subagents" || sidebarTab.value === "image") closeSidebar();
});
const allToolActivities = computed(() =>
  messages.value.flatMap((message) => message.toolActivities ?? []),
);
const subagentActivities = computed(() =>
  allToolActivities.value.filter((activity) => SUBAGENT_TOOLS.has(activity.toolName)),
);
const runningSubagentCount = computed(
  () => subagentActivities.value.filter((activity) => activity.status === "running").length,
);
const subagentTabLabel = computed(() => tr(settingStore.language, "sidebar.subagents"));
const imageTabLabel = computed(() => tr(settingStore.language, "sidebar.image"));
const diffTabLabel = computed(() => tr(settingStore.language, "sidebar.diff"));
const runtimeTabLabel = computed(() => tr(settingStore.language, "sidebar.runtime"));
const sidebarViewsLabel = computed(() => tr(settingStore.language, "sidebar.views"));
const sidebarCloseLabel = computed(() => tr(settingStore.language, "sidebar.close"));
const hasVisibleMessages = computed(() =>
  messages.value.some((message) => String(message.role).toLowerCase() !== "system"),
);
const sending = computed(() => {
  const sessionId = activeSessionId.value;
  if (!sessionId) {
    return false;
  }
  if (chatStore.sending[sessionId]) {
    return true;
  }
  return messages.value.some(
    (message) =>
      String(message.role).toLowerCase() === "assistant" &&
      (message.status === "pending" || message.status === "streaming"),
  );
});
const contextNotice = computed(() => overlayContextNotice.value);
const workspaceDisplayName = computed(() => props.capturedContext?.workspace?.name?.trim() || "");
const selectedText = computed(() => props.capturedContext?.selection?.trim() ?? "");
const selectionLines = computed(() => selectionLineCount(selectedText.value));
const contextPreview = computed(() => {
  const context = props.capturedContext;
  if (!context) {
    return "";
  }

  if (context.selectedFiles?.length) {
    const files = context.selectedFiles;
    const preview =
      files.length === 1
        ? files[0]
        : tr(settingStore.language, "selectedFiles", { file: files[0] ?? "", count: files.length });
    return `[Selected Files] ${preview}`;
  }

  if (context.selectedImages?.length) {
    const count = context.selectedImages.length;
    return count === 1
      ? tr(settingStore.language, "selectedImage")
      : tr(settingStore.language, "selectedImages", { count });
  }

  return "";
});
watch(
  [activeSessionId, panelVisible],
  ([sessionId, visible]) => void setWindowSessionView(visible ? sessionId : undefined),
  { immediate: true },
);
const chatTitle = computed(() => {
  const userMsg = messages.value.find((message) => String(message.role).toLowerCase() === "user");
  const text = userMsg ? parseSelectionAttachment(userMsg.content).message.trim() : "";
  return text || tr(settingStore.language, "newChat");
});

const composerLayout = ref({
  showSuggestions: false,
  suggestionCount: 0,
  showModelMenu: false,
  modelMenuHeight: 0,
  askUserRowCount: 0,
  pickerRowCount: 0,
  pickerHeight: 0,
  hasImages: false,
  hasFiles: false,
  inputBarHeight: undefined as number | undefined,
  /** True once `.composer-dock` was measured after paint (includes in-flow pickers). */
  dockMeasured: false,
});

function emitComposerLayout() {
  const dockMeasured = composerLayout.value.dockMeasured;
  emit("layoutChange", {
    ...composerLayout.value,
    hasContextPreview: dockMeasured ? false : Boolean(contextPreview.value),
    pickerHeight: dockMeasured ? 0 : composerLayout.value.pickerHeight,
    hasImages: dockMeasured ? false : composerLayout.value.hasImages,
    hasFiles: dockMeasured ? false : composerLayout.value.hasFiles,
    mode: props.mode,
    diffSidebarOpen: props.mode === "chat" && diffSidebarOpen.value,
    subagentSidebarOpen: props.mode === "chat" && subagentSidebarOpen.value,
    runtimeSidebarOpen: props.mode === "chat" && runtimeSidebarOpen.value,
    imageSidebarOpen: props.mode === "chat" && imageSidebarOpen.value,
    sidebarWidth:
      props.mode === "chat" && sidebarOpen.value
        ? diffSidebarWidth.value + DIFF_RESIZE_HANDLE_WIDTH
        : 0,
  });
}

function toggleSidebar() {
  if (sidebarOpen.value) {
    closeSidebar();
    return;
  }

  const selectedTab = sidebarTab.value;
  const unavailableDynamicTab =
    (selectedTab === "subagents" && !openedSubagentIds.value.length) ||
    (selectedTab === "image" && !openedImageSources.value.length);
  selectSidebarTab(unavailableDynamicTab ? "diff" : selectedTab);
}

function openDiffSidebar() {
  selectSidebarTab("diff");
}

function openDiffSidebarFile(path: string) {
  diffFocusPath.value = path;
  diffFocusAt.value += 1;
  selectSidebarTab("diff");
}

function handlePreviewImage(source: string) {
  if (props.mode === "chat") {
    if (!openedImageSources.value.includes(source)) {
      openedImageSources.value = [...openedImageSources.value, source];
    }
    selectedImageSource.value = source;
    selectSidebarTab("image");
    return;
  }
  void openImagePreview(source).catch((error) => {
    console.error("openImagePreview failed:", error);
  });
}

function handleEditFromImage(payload: { images: string[]; draftText?: string; region?: boolean }) {
  void inputRef.value?.attachImageEditReference?.(payload);
}

function closeImageTab(source: string) {
  const index = openedImageSources.value.indexOf(source);
  if (index < 0) return;
  const remaining = openedImageSources.value.filter((item) => item !== source);
  openedImageSources.value = remaining;
  if (selectedImageSource.value === source) {
    selectedImageSource.value = remaining[index] ?? remaining[index - 1] ?? "";
  }
  if (!remaining.length && sidebarTab.value === "image") {
    closeSidebar();
  }
}

function openSubagentSidebar(entryId?: string) {
  if (entryId) {
    if (!openedSubagentIds.value.includes(entryId)) {
      openedSubagentIds.value = [...openedSubagentIds.value, entryId];
    }
    selectedSubagentId.value = entryId;
  }
  if (!openedSubagentIds.value.length) return;
  selectSidebarTab("subagents");
}

function closeSubagentTab(entryId: string) {
  const index = openedSubagentIds.value.indexOf(entryId);
  if (index < 0) return;
  const remaining = openedSubagentIds.value.filter((id) => id !== entryId);
  openedSubagentIds.value = remaining;
  if (selectedSubagentId.value === entryId) {
    selectedSubagentId.value = remaining[index] ?? remaining[index - 1] ?? "";
  }
  if (!remaining.length && sidebarTab.value === "subagents") {
    closeSidebar();
  }
}

function selectSidebarTab(tab: SidebarTab) {
  if (tab === "runtime" && !runtimeDebugEnabled) return;
  if (!sidebarOpen.value && props.mode === "chat") {
    const currentWidth = document.documentElement.clientWidth;
    const halfWidth = currentWidth / 2;
    const equalPaneWidth = halfWidth >= CHAT_PANE_MIN_WIDTH ? halfWidth : currentWidth;
    diffSidebarWidth.value = Math.min(
      DIFF_SIDEBAR_MAX_WIDTH,
      Math.max(DIFF_SIDEBAR_MIN_WIDTH, equalPaneWidth - DIFF_RESIZE_HANDLE_WIDTH),
    );
  }
  sidebarTab.value = tab;
  diffSidebarOpen.value = tab === "diff";
  subagentSidebarOpen.value = tab === "subagents";
  runtimeSidebarOpen.value = tab === "runtime";
  imageSidebarOpen.value = tab === "image";
  emitComposerLayout();
}

function closeSidebar() {
  if (!sidebarOpen.value) return;
  diffSidebarOpen.value = false;
  subagentSidebarOpen.value = false;
  runtimeSidebarOpen.value = false;
  imageSidebarOpen.value = false;
}

function availableDiffSidebarWidth() {
  const contentWidth = document.documentElement.clientWidth;
  return Math.min(
    DIFF_SIDEBAR_MAX_WIDTH,
    Math.max(DIFF_SIDEBAR_MIN_WIDTH, contentWidth - CHAT_PANE_MIN_WIDTH - DIFF_RESIZE_HANDLE_WIDTH),
  );
}

function startDiffSidebarResize(event: PointerEvent) {
  if (event.button !== 0) return;
  event.preventDefault();
  event.stopPropagation();
  (event.currentTarget as HTMLElement | null)?.setPointerCapture?.(event.pointerId);
  diffResizeStartX = event.clientX;
  diffResizeStartWidth = diffSidebarWidth.value;
  diffSidebarResizing.value = true;
  window.addEventListener("pointermove", handleDiffSidebarResize);
  window.addEventListener("pointerup", stopDiffSidebarResize, { once: true });
  window.addEventListener("pointercancel", stopDiffSidebarResize, { once: true });
}

function handleDiffSidebarResize(event: PointerEvent) {
  event.preventDefault();
  const requested = diffResizeStartWidth + diffResizeStartX - event.clientX;
  diffSidebarWidth.value = Math.min(
    availableDiffSidebarWidth(),
    Math.max(DIFF_SIDEBAR_MIN_WIDTH, requested),
  );
}

function stopDiffSidebarResize() {
  diffSidebarResizing.value = false;
  window.removeEventListener("pointermove", handleDiffSidebarResize);
  window.removeEventListener("pointerup", stopDiffSidebarResize);
  window.removeEventListener("pointercancel", stopDiffSidebarResize);
  localStorage.setItem(DIFF_SIDEBAR_WIDTH_KEY, String(Math.round(diffSidebarWidth.value)));
}

function handleDiffSidebarResizeKey(event: KeyboardEvent) {
  if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
  event.preventDefault();
  const delta = event.key === "ArrowLeft" ? 16 : -16;
  const requested = diffSidebarWidth.value + delta;
  diffSidebarWidth.value = Math.min(
    availableDiffSidebarWidth(),
    Math.max(DIFF_SIDEBAR_MIN_WIDTH, requested),
  );
  localStorage.setItem(DIFF_SIDEBAR_WIDTH_KEY, String(Math.round(diffSidebarWidth.value)));
}

function resetDiffSidebarWidth() {
  diffSidebarWidth.value = Math.min(DIFF_SIDEBAR_DEFAULT_WIDTH, availableDiffSidebarWidth());
  localStorage.setItem(DIFF_SIDEBAR_WIDTH_KEY, String(diffSidebarWidth.value));
}

function handleLayoutChange(payload: {
  showSuggestions: boolean;
  suggestionCount: number;
  showModelMenu: boolean;
  modelMenuHeight: number;
  askUserRowCount: number;
  pickerRowCount: number;
  pickerHeight?: number;
  hasImages?: boolean;
  hasFiles?: boolean;
  inputBarHeight?: number;
  layoutReason?: "picker" | "chrome" | "other";
}) {
  composerLayout.value = {
    ...payload,
    pickerHeight: payload.pickerHeight ?? 0,
    hasImages: payload.hasImages ?? false,
    hasFiles: payload.hasFiles ?? false,
    inputBarHeight: payload.inputBarHeight,
    dockMeasured: false,
  };

  const shouldRemeasureDock =
    payload.layoutReason === "picker" ||
    payload.layoutReason === "chrome" ||
    payload.showSuggestions ||
    (payload.pickerRowCount ?? 0) > 0 ||
    payload.hasImages ||
    payload.hasFiles;
  if (shouldRemeasureDock) {
    // Single emit after paint — avoid shell-then-dock double resize flash.
    scheduleDockHeightMeasure();
    return;
  }
  emitComposerLayout();
}

/** Re-measure after picker transitions paint so the native window includes the list. */
let dockMeasureScheduled = false;
function scheduleDockHeightMeasure() {
  if (dockMeasureScheduled) return;
  dockMeasureScheduled = true;
  void nextTick(() => {
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        dockMeasureScheduled = false;
        const dock = dockRef.value;
        if (!dock) {
          emitComposerLayout();
          return;
        }
        const dockHeight = Math.ceil(dock.getBoundingClientRect().height);
        if (dockHeight > 0) {
          composerLayout.value = {
            ...composerLayout.value,
            dockMeasured: true,
            pickerHeight: 0,
            hasImages: false,
            hasFiles: false,
            inputBarHeight: dockHeight,
          };
        }
        emitComposerLayout();
      });
    });
  });
}

function createSessionId() {
  return `session-${Date.now()}`;
}

async function handleSubmit(text: string) {
  const trimmed = text.trim();
  if (!trimmed) {
    return;
  }

  // Snapshot before enterChat/contextConsumed. Consuming capture context used
  // to clear the composer workspace and mis-file the turn as Quick Ask.
  const sendOptions = resolveOverlaySendOptions();

  if (props.mode === "chat") {
    await chatStore.send(trimmed, activeSessionId.value, sendOptions);
    return;
  }

  if (sending.value) {
    return;
  }

  const sessionId = createSessionId();
  const messageWithSelection = attachSelection(trimmed, selectedText.value);

  chatStore.setOverlayDraftSession(sessionId);
  chatStore.setComposeDraft(sessionId, "", {
    workspaceId: sendOptions.quickAsk ? null : (sendOptions.workspaceId ?? null),
  });
  chatStore.stageTurn(sessionId, messageWithSelection);
  emit("enterChat", sessionId);
  emit("contextConsumed");

  void chatStore.send(messageWithSelection, sessionId, {
    staged: true,
    ...sendOptions,
  });
}

function resolveOverlaySendOptions(): { workspaceId?: string; quickAsk?: boolean } {
  const selected = inputRef.value?.resolveSendWorkspaceOptions?.();
  if (selected) {
    return selected;
  }
  return { quickAsk: true };
}

async function scheduleOverlayInputFocus() {
  const window = getCurrentWebviewWindow();
  const attempt = async () => {
    await nextTick();
    try {
      await window.setFocus();
    } catch {
      // ignore focus errors during reveal
    }
    void inputRef.value?.focusInput();
  };
  await attempt();
  requestAnimationFrame(() => {
    void attempt();
    requestAnimationFrame(() => {
      void attempt();
      globalThis.setTimeout(() => void attempt(), 80);
    });
  });
}

function handleShowContext(context: CapturedContext) {
  const sessionId =
    props.mode === "chat" && activeSessionId.value ? activeSessionId.value : createSessionId();
  if (props.mode !== "chat") {
    chatStore.setOverlayDraftSession(sessionId);
    emit("enterChat", sessionId);
  }
  chatStore.upsertMessage({
    id: `local-context-${Date.now()}`,
    sessionId,
    role: "assistant",
    content: "",
    environmentContext: context,
    status: "done",
    timestamp: Date.now(),
  });
}

const activeAssistantMessageId = computed(() => {
  const last = [...messages.value]
    .reverse()
    .find(
      (message) =>
        String(message.role).toLowerCase() === "assistant" &&
        (message.status === "pending" || message.status === "streaming"),
    );
  return last?.id ?? "";
});

async function handlePause() {
  if (!sending.value) {
    return;
  }

  const messageId = activeAssistantMessageId.value;
  const sessionId = activeSessionId.value;
  if (!messageId || !sessionId) {
    return;
  }

  // 乐观恢复发送：后端会再发 chat-finished（cancelled）来对齐状态
  chatStore.clearSending(sessionId);

  try {
    await chatCancel({ messageId });
  } catch (error) {
    console.error("chat_cancel failed:", error);
    // 无活跃任务时（例如异常退出后恢复），本地也要解除卡住的执行态
    chatStore.settleInterruptedSession(sessionId);
  }
  void chatStore.flushStaged(sessionId);
}

function close() {
  emit("close");
}

async function openInWorkbench() {
  const sessionId = activeSessionId.value;
  if (!sessionId) return;
  try {
    await openSessionInWorkbench(sessionId, getCurrentWebviewWindow().label);
  } catch (error) {
    console.error("Failed to open conversation in workbench:", error);
  }
}

async function toggleAlwaysOnTop() {
  const window = getCurrentWebviewWindow();
  const next = !isAlwaysOnTop.value;
  try {
    await window.setAlwaysOnTop(next);
    isAlwaysOnTop.value = next;
  } catch (error) {
    console.error("Failed to toggle window always-on-top state:", error);
  }
}

function clearMinimizePreview() {
  isMinimizePreview.value = false;
  if (minimizeTimer) {
    clearTimeout(minimizeTimer);
    minimizeTimer = null;
  }
}

function minimize() {
  if (isMinimizePreview.value) {
    return;
  }

  isMinimizePreview.value = true;
  void nextTick().then(() => {
    minimizeTimer = setTimeout(() => {
      minimizeTimer = null;
      void minimizeOverlay(getCurrentWebviewWindow().label);
    }, MINIMIZE_PREVIEW_MS);
  });
}

async function handleAskUserComplete(answer: string) {
  const session = askUserSession.value;
  if (!session || askUserSubmitting.value) {
    return;
  }
  askUserSubmitting.value = true;
  askUserSession.value = null;
  emitComposerLayout();

  // 用选择卡片展示回答，不显示 ask_user 原始 JSON
  try {
    const parsed = JSON.parse(answer) as {
      skipped?: boolean;
      answers?: Array<{
        header?: string;
        question?: string;
        selected?: string[];
        userSupplement?: boolean;
      }>;
    };

    const items: AskUserAnswerItem[] =
      parsed.answers
        ?.map((item) => ({
          header: String(item.header ?? "").trim() || undefined,
          selected: (item.selected ?? []).map((v) => String(v).trim()).filter(Boolean),
          userSupplement: Boolean(item.userSupplement),
        }))
        .filter((item) => item.userSupplement || item.selected.length > 0) ?? [];

    if (items.length > 0) {
      chatStore.stageAskUserAnswer(activeSessionId.value, items);
    }
  } catch {
    // ignore formatting errors
  }

  try {
    await respondAskUser({
      requestId: session.requestId,
      answer,
    });
    // 只有后端确认收到回答后，才把 ask_user 工具卡片标为完成/隐藏
    chatStore.completeAskUserToolActivities(activeSessionId.value, answer);
  } catch (error) {
    // 避免工具仍在等待但 UI 已“乐观结束”导致看起来 AI 不回复
    chatStore.stageUserMessage(
      activeSessionId.value,
      tr(settingStore.language, "askSubmitFailed", { error: String(error) }),
    );
    console.error("respond_ask_user failed:", error);
    if (!isAlreadyResolvedError(error) && !askUserSession.value) {
      askUserSession.value = session;
    }
    askUserSubmitting.value = false;
    return;
  }

  const label = getCurrentWebviewWindow().label;
  await setOverlayPopupOpen(label, false);
  emitComposerLayout();
  await nextTick();
  void inputRef.value?.focusInput();
  askUserSubmitting.value = false;
}

function closePathPermission() {
  pathPermissionSession.value = null;
}

async function handleOpenHistory() {
  closeAskUser();
  closePathPermission();
  historySessions.value = await loadScopedHistorySessions();
  const label = getCurrentWebviewWindow().label;
  await setOverlayPopupOpen(label, true);
  void inputRef.value?.focusInput();
}

async function loadScopedHistorySessions() {
  const allSessions = await fetchChatSessions();

  // Overlay history follows the same workspace scope as send:
  // 1) Composer-selected / IDE-matched workspace → that workspace's sessions
  // 2) otherwise Quick Ask sessions (no workspaceId)
  // Never fall back to the workbench's current workspace.
  const sendOptions = inputRef.value?.resolveSendWorkspaceOptions?.();
  if (sendOptions?.workspaceId && !sendOptions.quickAsk) {
    return allSessions.filter((session) => session.workspaceId === sendOptions.workspaceId);
  }

  return allSessions.filter((session) => !session.workspaceId).slice(0, PUBLIC_HISTORY_LIMIT);
}

async function handleHistorySelect(sessionId: string) {
  const label = getCurrentWebviewWindow().label;
  historySessions.value = null;
  await setOverlayPopupOpen(label, false);

  await chatStore.loadHistory(sessionId);
  chatStore.setOverlayDraftSession(sessionId);

  if (props.mode !== "chat") {
    emit("enterChat", sessionId);
  }

  emitComposerLayout();
  await nextTick();
  void inputRef.value?.focusInput();
}

function handleHistoryClose() {
  historySessions.value = null;
  void setOverlayPopupOpen(getCurrentWebviewWindow().label, false);
  emitComposerLayout();
}

async function handlePathPermissionComplete(decision: PathPermissionDecision) {
  const session = pathPermissionSession.value;
  if (!session || pathPermissionSubmitting.value) {
    return;
  }
  pathPermissionSubmitting.value = true;
  pathPermissionSession.value = null;
  emitComposerLayout();
  try {
    await respondPathPermission({
      requestId: session.requestId,
      decision,
    });
  } catch (error) {
    console.error("respond_path_permission failed:", error);
    if (!isAlreadyResolvedError(error) && !pathPermissionSession.value) {
      pathPermissionSession.value = session;
    }
    pathPermissionSubmitting.value = false;
    return;
  }

  const label = getCurrentWebviewWindow().label;
  await setOverlayPopupOpen(label, false);
  pathPermissionSubmitting.value = false;
  emitComposerLayout();
  await nextTick();
  void inputRef.value?.focusInput();
}

function closeToolApproval() {
  toolApprovalSession.value = null;
}

async function handleToolApprovalComplete(decision: ToolApprovalDecision) {
  const session = toolApprovalSession.value;
  if (!session || toolApprovalSubmitting.value) {
    return;
  }
  toolApprovalSubmitting.value = true;
  toolApprovalSession.value = null;
  emitComposerLayout();
  try {
    await respondToolApproval({
      requestId: session.requestId,
      decision,
    });
  } catch (error) {
    console.error("respond_tool_approval failed:", error);
    if (!isAlreadyResolvedError(error) && !toolApprovalSession.value) {
      toolApprovalSession.value = session;
    }
    toolApprovalSubmitting.value = false;
    return;
  }
  const label = getCurrentWebviewWindow().label;
  await setOverlayPopupOpen(label, false);
  toolApprovalSubmitting.value = false;
  emitComposerLayout();
  await nextTick();
  void inputRef.value?.focusInput();
}

function isAlreadyResolvedError(error: unknown) {
  return String(error).includes("already completed") || String(error).includes("not found");
}

async function refreshCheckpoints() {
  const sessionId = activeSessionId.value;
  if (!sessionId) {
    checkpoints.value = [];
    return;
  }
  try {
    checkpoints.value = await listCheckpoints(sessionId);
  } catch {
    checkpoints.value = [];
  }
}

async function handleRewound(payload: { text: string }) {
  await chatStore.loadHistory(activeSessionId.value);
  await refreshCheckpoints();
  if (payload.text) {
    inputRef.value?.setMessage(payload.text);
  } else {
    void inputRef.value?.focusInput();
  }
}

async function handleBranchMessage(messageId: string) {
  const sessionId = activeSessionId.value;
  if (!sessionId || !messageId) return;
  try {
    const summary = await branchChatSession(sessionId, messageId);
    const sourceCompose = chatStore.sessionCompose[sessionId];
    if (sourceCompose) {
      chatStore.setCompose(summary.sessionId, {
        chatModel: sourceCompose.chatModel,
        chatModelProvider: sourceCompose.chatModelProvider,
        chatMode: sourceCompose.chatMode === "plan" ? "agent" : sourceCompose.chatMode,
        toolApprovalMode: sourceCompose.toolApprovalMode,
        imageGen: sourceCompose.imageGen,
      });
    } else {
      chatStore.ensureCompose(summary.sessionId);
    }
    chatStore.setComposeDraft(summary.sessionId, "", {
      workspaceId: summary.workspaceId ?? null,
    });
    await chatStore.loadHistory(summary.sessionId);
    chatStore.setOverlayDraftSession(summary.sessionId);
    emit("enterChat", summary.sessionId);
    await refreshCheckpoints();
    await nextTick();
    void inputRef.value?.focusInput();
  } catch (error) {
    console.error("branch_chat_session failed:", error);
  }
}

function closeAskUser() {
  askUserSession.value = null;
  void setOverlayPopupOpen(getCurrentWebviewWindow().label, false);
}

watch(
  () => activeSessionId.value,
  () => {
    void refreshCheckpoints();
  },
);

watch(
  () =>
    messages.value
      .map((message) => `${message.id}:${message.status}:${message.toolActivities?.length ?? 0}`)
      .join("|"),
  () => {
    void refreshCheckpoints();
  },
);

watch(
  () => [contextPreview.value, props.mode, askUserSession.value] as const,
  () => {
    emitComposerLayout();
  },
);

watch(
  () => props.mode,
  (mode) => {
    if (mode === "chat") {
      void inputRef.value?.focusInput();
    }
    emitComposerLayout();
  },
);

watch(panelVisible, async (visible) => {
  // Keep the dock fully painted. Native cloak/show owns window visibility;
  // opacity hide/reveal here flashes on every Alt+Alt summon.
  if (!visible) {
    return;
  }
  await nextTick();
  void scheduleOverlayInputFocus();
});

onMounted(async () => {
  const window = getCurrentWebviewWindow();
  isAlwaysOnTop.value = await window.isAlwaysOnTop().catch(() => true);
  // Ensure dock is visible even if a prior session left inline styles behind.
  gsapOverlayDockReveal(dockRef.value, true);

  void listenAskUser(async (payload) => {
    if (payload.sessionId && payload.sessionId !== activeSessionId.value) {
      return;
    }
    pathPermissionSession.value = null;
    toolApprovalSession.value = null;
    askUserSession.value = {
      requestId: payload.requestId,
      questions: payload.questions,
    };
    const label = getCurrentWebviewWindow().label;
    await setOverlayPopupOpen(label, true);
    void inputRef.value?.focusInput();
  });

  void listenPathPermission(async (payload) => {
    if (payload.sessionId && payload.sessionId !== activeSessionId.value) {
      return;
    }
    askUserSession.value = null;
    toolApprovalSession.value = null;
    pathPermissionSession.value = {
      requestId: payload.requestId,
      path: payload.path,
      operation: payload.operation,
      toolName: payload.toolName,
    };
    const label = getCurrentWebviewWindow().label;
    await setOverlayPopupOpen(label, true);
    void inputRef.value?.focusInput();
  });

  void listenToolApproval(async (payload) => {
    if (payload.sessionId && payload.sessionId !== activeSessionId.value) {
      return;
    }
    askUserSession.value = null;
    pathPermissionSession.value = null;
    toolApprovalSession.value = {
      requestId: payload.requestId,
      toolName: payload.toolName,
      title: payload.title,
      preview: payload.preview ?? null,
    };
    chatStore.attachToolApprovalPreview(
      payload.sessionId || activeSessionId.value,
      payload.toolName,
      payload.preview ?? null,
      activeSessionId.value,
    );
    const label = getCurrentWebviewWindow().label;
    await setOverlayPopupOpen(label, true);
    void inputRef.value?.focusInput();
  });

  void listenInteractionResolved(async (payload) => {
    let matched = false;
    if (askUserSession.value?.requestId === payload.requestId) {
      askUserSession.value = null;
      matched = true;
    }
    if (pathPermissionSession.value?.requestId === payload.requestId) {
      pathPermissionSession.value = null;
      matched = true;
    }
    if (toolApprovalSession.value?.requestId === payload.requestId) {
      toolApprovalSession.value = null;
      matched = true;
    }
    if (!matched) return;
    emitComposerLayout();
    await setOverlayPopupOpen(getCurrentWebviewWindow().label, false);
    void inputRef.value?.focusInput();
  });

  await window.listen("overlay-shown", () => {
    clearMinimizePreview();
    // Do not call refreshOverlayWindowBackground here — clearEffects/setShadow
    // on every summon forces a Win32 non-client refresh that flashes the window.
    // Rust configure_overlay_window already applied shadow/toolwindow on show.
    panelVisible.value = true;
    void scheduleOverlayInputFocus();
  });

  await window.listen("overlay-hidden", () => {
    clearMinimizePreview();
    panelVisible.value = false;
    inputRef.value?.reset();
    emit("layoutChange", {
      showSuggestions: false,
      suggestionCount: 0,
      showModelMenu: false,
      modelMenuHeight: 0,
      askUserRowCount: 0,
      pickerRowCount: 0,
      pickerHeight: 0,
      hasContextPreview: false,
      mode: props.mode,
      diffSidebarOpen: false,
      subagentSidebarOpen: false,
      runtimeSidebarOpen: false,
      imageSidebarOpen: false,
      sidebarWidth: 0,
    });
    diffSidebarOpen.value = false;
    subagentSidebarOpen.value = false;
    runtimeSidebarOpen.value = false;
    imageSidebarOpen.value = false;
    openedImageSources.value = [];
    selectedImageSource.value = "";
    closeAskUser();
    closePathPermission();
    closeToolApproval();
    historySessions.value = null;
  });

  void refreshCheckpoints();

  if (await window.isVisible()) {
    panelVisible.value = true;
    // 动态新建窗口时，overlay-shown 在 Vue 挂载前就发出了，
    // 这里补做相同的初始化：聚焦输入框（背景已由 Rust configure 处理）
    void scheduleOverlayInputFocus();
  } else {
    void scheduleOverlayInputFocus();
  }

  void listen<string>("open-session", async (event) => {
    const targetSessionId = event.payload;
    await handleHistorySelect(targetSessionId);
  });

  void listen("history-updated", async () => {
    if (historySessions.value !== null) {
      historySessions.value = await loadScopedHistorySessions();
    }
  });

  void listen<{ sessionId?: string; command?: string; args?: string }>("slash-command", (event) => {
    const command = (event.payload?.command ?? "").replace(/^\//, "").toLowerCase();
    if (
      props.sessionId &&
      event.payload?.sessionId &&
      event.payload.sessionId !== props.sessionId
    ) {
      return;
    }
    switch (command) {
      case "history":
        historySessions.value = historySessions.value ?? [];
        void loadScopedHistorySessions().then((sessions) => {
          historySessions.value = sessions;
        });
        break;
      case "work":
        void inputRef.value?.focusInput();
        break;
      case "exit":
        emit("close");
        break;
      case "clear":
        inputRef.value?.reset();
        break;
      default:
        break;
    }
  });
});

onUnmounted(() => {
  void setWindowSessionView();
  clearMinimizePreview();
  stopDiffSidebarResize();
});
</script>

<style scoped>
.peek-panel {
  box-sizing: border-box;
  width: 100%;
  height: 100%;
  position: relative;
  display: flex;
  flex-direction: column;
  justify-content: flex-start;
  align-items: stretch;
  background: transparent;
  color: var(--peek-text);
  overflow: hidden;
  border: none;
  outline: none;
  --thread-side-gap: 14px;
  --peek-panel-outline: var(
    --peek-strong-border,
    color-mix(in srgb, var(--peek-text) 20%, transparent)
  );
  --peek-panel-highlight: color-mix(in srgb, var(--peek-text) 5%, transparent);
  --peek-panel-shadow: transparent;
}

.chat-main {
  flex: 1 1 auto;
  min-width: 0;
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  overflow: hidden;
}

.thread-content {
  flex: 1;
  min-height: 0;
  min-width: 0;
  display: flex;
  overflow: hidden;
}

.thread-content :deep(.message-list-shell) {
  flex: 1;
  min-width: 540px;
}
.thread-content :deep(.error-boundary-root.failed) {
  flex: 1;
  min-width: 540px;
}

.workspace-sidebar-shell {
  flex: none;
  min-width: 0;
  height: 100%;
  display: flex;
  overflow: hidden;
  transform: translateX(0);
  transform-origin: right center;
  opacity: 1;
}

.workspace-sidebar-enter-active,
.workspace-sidebar-leave-active {
  overflow: hidden;
  transition:
    width 180ms cubic-bezier(0.2, 0.72, 0.25, 1),
    opacity 130ms ease,
    transform 180ms cubic-bezier(0.2, 0.72, 0.25, 1);
}

.workspace-sidebar-enter-from,
.workspace-sidebar-leave-to {
  width: 0 !important;
  opacity: 0;
  transform: translateX(12px);
}

.workspace-sidebar {
  flex: none;
  box-sizing: border-box;
  min-width: 420px;
  max-width: 1000px;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding-top: 34px;
  background: transparent;
  container: workspace-sidebar / inline-size;
}

.workspace-sidebar-tabs {
  flex: none;
  min-height: 40px;
  padding: 5px 7px;
  gap: 2px;
  background: color-mix(in srgb, var(--peek-text) 1.5%, transparent);
}

.workspace-sidebar-tabs .workspace-view-tab {
  flex: 0 1 auto;
  gap: 6px;
  min-width: 68px;
  height: 30px;
  padding: 0 9px;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--peek-muted);
  font-size: 11px;
  font-weight: 500;
  box-shadow: none;
}
.workspace-sidebar-tabs .workspace-view-tab:hover {
  background: color-mix(in srgb, var(--peek-text) 5%, transparent);
  color: var(--peek-text);
}
.workspace-sidebar-tabs .workspace-view-tab.active {
  background: color-mix(in srgb, var(--peek-text) 8%, transparent);
  color: var(--peek-text);
  box-shadow: 0 3px 10px color-mix(in srgb, #000 9%, transparent);
}
.workspace-sidebar-tabs .workspace-view-tab > svg {
  flex: none;
}
.workspace-sidebar-tabs .workspace-view-tab > span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.workspace-sidebar-tabs .sidebar-close-button {
  flex: none;
  min-width: 28px;
  width: 28px;
  height: 28px;
  display: grid;
  place-items: center;
  margin: 1px 0 0 auto;
  padding: 0;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
}
.workspace-sidebar-tabs .sidebar-close-button:hover {
  color: var(--peek-text);
  background: color-mix(in srgb, var(--peek-text) 7%, transparent);
}
.workspace-sidebar-content {
  flex: 1;
  min-height: 0;
  display: flex;
  overflow: hidden;
}

@container workspace-sidebar (max-width: 560px) {
  .workspace-sidebar-tabs .workspace-view-tab {
    flex: none;
    min-width: 32px;
    width: 32px;
    padding: 0;
    justify-content: center;
  }

  .workspace-sidebar-tabs .workspace-view-tab > span {
    display: none;
  }
}

.header-tools {
  display: flex;
  align-items: center;
  gap: 2px;
}

.running-dot {
  position: absolute;
  top: 4px;
  right: 3px;
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--peek-accent);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--peek-sidebar) 88%, transparent);
}

.diff-resize-handle {
  position: relative;
  z-index: 4;
  flex: none;
  width: 7px;
  min-width: 7px;
  cursor: col-resize;
  outline: none;
  touch-action: none;
}

.diff-resize-handle::after {
  content: "";
  position: absolute;
  top: calc(50% - 18px);
  left: 2px;
  width: 3px;
  height: 36px;
  border-radius: 2px;
  background: transparent;
  transition:
    background 100ms ease,
    transform 100ms ease;
}

.diff-resize-handle:hover::after,
.diff-resize-handle:focus-visible::after,
.diff-resize-handle.active::after {
  background: color-mix(in srgb, var(--peek-accent) 68%, var(--peek-border));
  transform: scaleY(1.15);
}

@media (prefers-reduced-motion: reduce) {
  .workspace-sidebar-enter-active,
  .workspace-sidebar-leave-active {
    transition-duration: 1ms;
  }
}

.peek-panel.chat .thread-panel {
  /* Keep room for at least one user bubble while the window is still expanding. */
  min-height: 120px;
}

.thread-panel {
  flex: 1;
  min-height: 0;
  min-width: 0;
  width: calc(100% - (2 * var(--thread-side-gap)));
  margin: 0 auto calc(-1 * var(--composer-overlap, 12px));
  display: flex;
  flex-direction: column;
  border: 1px solid var(--peek-panel-outline);
  border-radius: 8px 8px 0 0;
  background: color-mix(in srgb, var(--peek-list-bg) 92%, transparent);
  overflow: hidden;
  position: relative;
  z-index: 1;
  isolation: isolate;
  box-shadow: inset 0 1px 0 var(--peek-panel-highlight);
}

.thread-panel.glass {
  background: color-mix(in srgb, var(--peek-list-bg) 76%, transparent);
  backdrop-filter: blur(20px) saturate(1.15);
  -webkit-backdrop-filter: blur(20px) saturate(1.15);
}

.peek-panel.chat :deep(.message-list) {
  position: relative;
  z-index: 2;
  padding-top: 42px;
  scroll-padding-top: 42px;
  padding-bottom: calc(var(--composer-overlap, 12px) + var(--composer-clearance, 90px));
}

.thread-header {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  z-index: 5;
  height: 34px;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: 0 8px;
  border-bottom: 1px solid transparent;
  background: transparent;
  opacity: 0.42;
  pointer-events: auto;
  transition:
    opacity 160ms ease,
    border-color 160ms ease,
    background 160ms ease;
  cursor: grab;
}

.sidebar-toggle-btn {
  position: relative;
  margin-right: auto;
}

.sidebar-toggle-btn.active {
  border-color: color-mix(in srgb, var(--peek-accent) 38%, transparent);
  background: color-mix(in srgb, var(--peek-accent) 13%, transparent);
  color: var(--peek-accent);
}

.thread-header:hover {
  opacity: 1;
  background: color-mix(in srgb, var(--peek-sidebar) 88%, transparent);
}

.thread-panel.glass .thread-header:hover {
  backdrop-filter: blur(16px) saturate(1.1);
  -webkit-backdrop-filter: blur(16px) saturate(1.1);
}

.thread-header:active {
  cursor: grabbing;
}

/* 红色：全宽消息框底座 */
.composer-dock {
  flex: none;
  box-sizing: border-box;
  width: 100%;
  border: 1px solid var(--peek-panel-outline);
  border-radius: 8px;
  background: var(--peek-surface);
  position: relative;
  z-index: 2;
  display: flex;
  flex-direction: column;
  gap: 8px;
  overflow: hidden;
  isolation: isolate;
  will-change: transform, opacity;
  box-shadow: inset 0 1px 0 var(--peek-panel-highlight);
}

.composer-dock :deep(.chat-input-shell) {
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.composer-dock :deep(.chat-input-shell.interaction-request-open .ask-user-list),
.composer-dock :deep(.chat-input-shell.interaction-request-open .path-permission-list),
.composer-dock :deep(.chat-input-shell.interaction-request-open .tool-approval-list) {
  margin: 0;
}

.composer-dock.expanded {
  width: calc(100% - 2px);
  margin: 0 1px 1px;
  border-radius: 8px;
  background: linear-gradient(
    180deg,
    color-mix(in srgb, var(--peek-surface) 88%, var(--peek-list-bg)) 0%,
    var(--peek-surface) 22%,
    var(--peek-surface) 100%
  );
}

/* Chat-mode composer grows with in-flow pickers; keep overflow clipped so
   dock corners stay clean (lists no longer float into the thread). */
.peek-panel.chat .composer-dock {
  overflow: hidden;
}

.peek-panel.chat :deep(.input-footer-primary) {
  flex-wrap: nowrap;
  min-width: 0;
}

.window-controls {
  flex: none;
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.window-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  margin: 0;
  padding: 0;
  border: 1px solid transparent;
  border-radius: 50%;
  background: transparent;
  color: var(--peek-muted);
  cursor: default;
  transition:
    background 120ms ease,
    color 120ms ease,
    border-color 120ms ease;
}

.window-btn:hover {
  background: var(--peek-list-active);
  color: var(--peek-text);
  border-color: color-mix(in srgb, var(--peek-accent) 24%, var(--peek-border));
}

.window-btn.active {
  color: var(--peek-accent);
  background: color-mix(in srgb, var(--peek-accent) 14%, transparent);
}

.window-btn:disabled {
  opacity: 0.42;
  cursor: default;
}

.window-btn.close:hover {
  background: var(--destructive);
  border-color: var(--destructive);
  color: #fff;
}

.context-notice {
  position: absolute;
  z-index: 6;
  top: 40px;
  left: 50%;
  box-sizing: border-box;
  width: min(calc(100% - 48px), 720px);
  min-height: 34px;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 11px;
  border: 1px solid color-mix(in srgb, var(--peek-warning) 24%, var(--peek-border));
  border-radius: 9px;
  background: color-mix(in srgb, var(--peek-warning) 8%, var(--peek-surface));
  color: var(--peek-text);
  box-shadow: 0 8px 22px color-mix(in srgb, #000 13%, transparent);
  font-size: 11px;
  line-height: 1.45;
  transform: translateX(-50%);
}
.context-notice > svg {
  flex: none;
  color: var(--peek-warning);
}
.context-notice > span {
  min-width: 0;
  overflow-wrap: anywhere;
}

.captured-context-preview {
  flex: none;
  margin: 0;
  padding: 8px 12px 4px;
  font-size: 11px;
  line-height: 1.45;
  color: var(--peek-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  border-bottom: 1px solid color-mix(in srgb, var(--peek-border) 70%, transparent);
}

.peek-panel.minimize-preview .thread-panel,
.peek-panel.minimize-preview .composer-dock {
  visibility: hidden;
  pointer-events: none;
}

.minimize-preview-screen {
  position: absolute;
  inset: 0;
  z-index: 30;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 20px;
  background: var(--peek-bg);
  color: var(--peek-text);
  box-sizing: border-box;
  user-select: none;
}

.minimize-preview-title {
  width: 100%;
  min-width: 0;
  font-size: 17px;
  font-weight: 600;
  line-height: 1.4;
  text-align: center;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
