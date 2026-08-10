<template>
  <div class="overlay-shell" data-tauri-drag-region>
    <PeekPanel
      :mode="mode"
      :session-id="sessionId"
      :captured-context="capturedContext"
      :context-ready="contextReady"
      @layout-change="handleLayoutChange"
      @enter-chat="enterChatMode"
      @context-consumed="capturedContext = null"
      @selection-removed="removeCapturedSelection"
      @close="close"
    />
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { LogicalPosition, LogicalSize } from "@tauri-apps/api/dpi";
import { currentMonitor } from "@tauri-apps/api/window";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import PeekPanel from "@/components/chat/PeekPanel.vue";
import { closeOverlay, setOverlayChatMode, takeOverlayContext } from "@/services/ipc";
import { createLogger } from "@/services/logger";
import { useChatStore } from "@/stores/chat";
import { useSettingStore } from "@/stores/setting";
import type { CapturedContext } from "@/types/chat";
import { IPC_EVENTS } from "@/types/ipc";

const log = createLogger("overlay");

const chatStore = useChatStore();
const settingStore = useSettingStore();

const capturedContext = ref<CapturedContext | null>(null);
const contextReady = ref(false);

function removeCapturedSelection() {
  if (!capturedContext.value) return;
  capturedContext.value = {
    ...capturedContext.value,
    selection: undefined,
  };
}

const PANEL_WIDTH = 640;
// Fallback when dock height is not measured yet: single-line input + footer + dock borders.
const INPUT_HEIGHT = 88;
const OVERLAY_MIN_HEIGHT_INPUT = INPUT_HEIGHT;
const CHAT_HEIGHT_PREFERRED = 520;
const CHAT_SCREEN_MARGIN = 48;
const OVERLAY_MIN_HEIGHT_CHAT = 240;
const SUGGESTION_ROW_HEIGHT = 30;
const SUGGESTION_PADDING = 9;
const PICKER_VISIBLE_ROWS = 8;
const CONTEXT_PREVIEW_HEIGHT = 30;
const INPUT_BAR_HEIGHT = INPUT_HEIGHT;

const mode = ref<"input" | "chat">("input");
const sessionId = ref("");
const lastComposerExtraHeight = ref(0);
const chatWindowInitialized = ref(false);
const diffSidebarOpen = ref(false);
const subagentSidebarOpen = ref(false);
const runtimeSidebarOpen = ref(false);
const imageSidebarOpen = ref(false);
let layoutResizeQueue = Promise.resolve();
let windowWidthBeforeSidebar = PANEL_WIDTH;
let windowWidthWithSidebar = PANEL_WIDTH;
/** Last applied input-mode design size — skip redundant Win32 setMinSize/setSize. */
let lastInputDesignWidth = 0;
let lastInputDesignHeight = 0;

// 获取当前窗口的 label，用于所有 IPC 调用
const windowLabel = getCurrentWebviewWindow().label;

function computePickerHeight(rowCount: number) {
  if (rowCount <= 0) {
    return 0;
  }
  // rowCount is the total visible rows (options + optional status/meta rows).
  const visibleRows = Math.min(rowCount, PICKER_VISIBLE_ROWS);
  return SUGGESTION_PADDING + visibleRows * SUGGESTION_ROW_HEIGHT;
}

async function applySizeConstraints(
  layout: "input" | "chat",
  minHeight: number,
  designWidth = PANEL_WIDTH,
) {
  const window = getCurrentWebviewWindow();
  // Native size and constraint changes restore a maximized window. Layout
  // updates still occur while a user is interacting with the chat, so they
  // must become no-ops until the user explicitly restores the window.
  if (await window.isMaximized()) {
    return;
  }
  const zoom = (settingStore.zoom || 100) / 100;

  const panelWidth = designWidth * zoom;
  const height = minHeight * zoom;
  try {
    await window.setMaxSize(null);
  } catch (error) {
    console.warn("Failed to clear overlay maximum size; continuing resize:", error);
  }
  await window.setMinSize(new LogicalSize(panelWidth, height));

  if (layout === "chat") {
    try {
      await window.setMaxSize(new LogicalSize(10000 * zoom, 10000 * zoom));
    } catch (error) {
      console.warn("Failed to set overlay maximum size; continuing resize:", error);
    }
  } else {
    try {
      await window.setMaxSize(null);
    } catch (error) {
      console.warn("Failed to clear overlay maximum size; continuing resize:", error);
    }
  }
}

function waitForNextFrame(count = 2): Promise<void> {
  return new Promise((resolve) => {
    let remaining = count;
    const tick = () => {
      remaining -= 1;
      if (remaining <= 0) {
        resolve();
        return;
      }
      requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  });
}

async function resizeWindow(
  width: number,
  height: number,
  resizable: boolean,
  skipPositionCorrection = false,
  verticalAnchor: "top" | "bottom" = "top",
) {
  const window = getCurrentWebviewWindow();
  if (await window.isMaximized()) {
    return;
  }
  const scaleFactor = await window.scaleFactor();
  const physicalPosition = await window.outerPosition();
  const physicalSize = await window.outerSize();

  const logicalPos = physicalPosition.toLogical(scaleFactor);
  const logicalSize = physicalSize.toLogical(scaleFactor);

  const zoom = (settingStore.zoom || 100) / 100;
  const scaledWidth = width * zoom;
  const scaledHeight = height * zoom;

  const currentHeight = logicalSize.height;
  const currentWidth = logicalSize.width;

  const delta = scaledHeight - currentHeight;
  const deltaWidth = scaledWidth - currentWidth;

  // Skip no-op resizes — setSize/setResizable on every keystroke causes jitter.
  if (Math.abs(delta) < 0.5 && Math.abs(deltaWidth) < 0.5) {
    return;
  }

  await window.setResizable(resizable);
  // Keep maximizable off for the borderless overlay — toggling it forces a
  // Win32 non-client style refresh that flashes on every double-Alt summon.
  await window.setMaximizable(false);

  await window.setSize(new LogicalSize(scaledWidth, scaledHeight));

  if (!skipPositionCorrection) {
    // 水平：居中扩展，并 clamp 到当前显示器左右边界
    let newX = logicalPos.x - deltaWidth / 2;

    const monitor = await currentMonitor();
    if (monitor) {
      const monitorPosition = monitor.position.toLogical(scaleFactor);
      const monitorSize = monitor.size.toLogical(scaleFactor);
      const monitorRight = monitorPosition.x + monitorSize.width;
      newX = Math.max(monitorPosition.x, newX);
      newX = Math.min(newX, monitorRight - scaledWidth);
    } else {
      newX = Math.max(0, newX);
    }

    // 输入模式以底边为锚点，建议列表只向上展开，输入框保持原位。
    let newY = verticalAnchor === "bottom" ? logicalPos.y - delta : logicalPos.y;

    if (verticalAnchor === "top") {
      if (monitor) {
        const monitorPosition = monitor.position.toLogical(scaleFactor);
        const monitorSize = monitor.size.toLogical(scaleFactor);
        const monitorBottom = monitorPosition.y + monitorSize.height;
        if (newY + scaledHeight > monitorBottom) {
          newY = Math.max(monitorPosition.y, monitorBottom - scaledHeight);
        }
      }
    }

    await window.setPosition(new LogicalPosition(newX, newY));
  }
}

function queueLayoutResize(operation: () => Promise<void>) {
  layoutResizeQueue = layoutResizeQueue
    .then(operation)
    .catch((error) => console.error("Failed to resize overlay:", error));
}

function handleLayoutChange(payload: {
  showSuggestions: boolean;
  suggestionCount: number;
  showModelMenu: boolean;
  modelMenuHeight: number;
  askUserRowCount?: number;
  pickerRowCount?: number;
  /** Measured/estimated picker height in design px — preferred over rowCount. */
  pickerHeight?: number;
  hasContextPreview?: boolean;
  mode?: "input" | "chat";
  hasImages?: boolean;
  hasFiles?: boolean;
  /** Measured composer .input-bar height (multi-line text, chips, images). */
  inputBarHeight?: number;
  diffSidebarOpen?: boolean;
  subagentSidebarOpen?: boolean;
  runtimeSidebarOpen?: boolean;
  imageSidebarOpen?: boolean;
  sidebarWidth?: number;
}) {
  const modeValue = payload.mode ?? mode.value;
  const pickerHeight =
    (payload.pickerHeight ?? 0) > 0
      ? payload.pickerHeight!
      : (payload.pickerRowCount ?? 0) > 0
        ? computePickerHeight(payload.pickerRowCount ?? 0)
        : payload.showSuggestions
          ? SUGGESTION_PADDING + payload.suggestionCount * SUGGESTION_ROW_HEIGHT
          : 0;
  const wasSidebarOpen =
    diffSidebarOpen.value ||
    subagentSidebarOpen.value ||
    runtimeSidebarOpen.value ||
    imageSidebarOpen.value;
  diffSidebarOpen.value = modeValue === "chat" && Boolean(payload.diffSidebarOpen);
  subagentSidebarOpen.value = modeValue === "chat" && Boolean(payload.subagentSidebarOpen);
  runtimeSidebarOpen.value = modeValue === "chat" && Boolean(payload.runtimeSidebarOpen);
  imageSidebarOpen.value = modeValue === "chat" && Boolean(payload.imageSidebarOpen);
  const willSidebarOpen =
    diffSidebarOpen.value ||
    subagentSidebarOpen.value ||
    runtimeSidebarOpen.value ||
    imageSidebarOpen.value;
  const sidebarOpening = !wasSidebarOpen && willSidebarOpen;
  const sidebarClosing = wasSidebarOpen && !willSidebarOpen;
  const sidebarWidth = willSidebarOpen ? Math.max(0, payload.sidebarWidth ?? 0) : 0;
  if (sidebarOpening) {
    windowWidthWithSidebar = Math.max(windowWidthBeforeSidebar, sidebarWidth * 2);
  }
  const initialDesignWidth =
    modeValue === "chat" && willSidebarOpen ? windowWidthWithSidebar : PANEL_WIDTH;
  // Chip menus are in-panel pickers now; height comes from pickerHeight.
  // Keep modelMenuHeight only for any leftover floating chrome in input mode.
  const modelMenuHeight =
    modeValue === "input" && payload.showModelMenu ? payload.modelMenuHeight : 0;
  const contextHeight = payload.hasContextPreview ? CONTEXT_PREVIEW_HEIGHT : 0;
  // Prefer measured composer dock height so multi-line Alt+Alt input grows the
  // window and the 1px top/bottom dock borders are never clipped.
  const measuredBarHeight =
    typeof payload.inputBarHeight === "number" && payload.inputBarHeight > 0
      ? payload.inputBarHeight
      : 0;
  const imagesHeight = measuredBarHeight > 0 ? 0 : payload.hasImages ? 60 : 0;
  const filesHeight = measuredBarHeight > 0 ? 0 : payload.hasFiles ? 34 : 0;
  const inputBarHeight = measuredBarHeight > 0 ? measuredBarHeight : INPUT_BAR_HEIGHT;
  // Input mode: grow the native window around in-flow pickers.
  // Chat mode: pickers stay in-flow inside the composer (thread shrinks) so they
  // are never clipped by absolute positioning against overflow:hidden ancestors.
  // Do not resize the chat window for picker chrome.
  const extraHeight =
    (modeValue === "chat" ? 0 : pickerHeight) +
    modelMenuHeight +
    contextHeight +
    imagesHeight +
    filesHeight;

  if (modeValue === "input") {
    chatWindowInitialized.value = false;
    lastComposerExtraHeight.value = 0;
    const nextHeight = inputBarHeight + extraHeight;
    if (
      Math.abs(nextHeight - lastInputDesignHeight) < 1 &&
      Math.abs(PANEL_WIDTH - lastInputDesignWidth) < 1
    ) {
      return;
    }
    lastInputDesignWidth = PANEL_WIDTH;
    lastInputDesignHeight = nextHeight;
    queueLayoutResize(async () => {
      await applySizeConstraints("input", OVERLAY_MIN_HEIGHT_INPUT);
      await resizeWindow(PANEL_WIDTH, nextHeight, false, false, "bottom");
    });
    return;
  }

  if (modeValue === "chat") {
    const deltaExtra = extraHeight - lastComposerExtraHeight.value;

    if (!chatWindowInitialized.value) {
      chatWindowInitialized.value = true;
      lastComposerExtraHeight.value = extraHeight;
      queueLayoutResize(async () => {
        await applySizeConstraints("chat", OVERLAY_MIN_HEIGHT_CHAT, initialDesignWidth);
        await resizeWindow(initialDesignWidth, await preferredChatHeight(extraHeight), true);
      });
      return;
    }

    lastComposerExtraHeight.value = extraHeight;
    queueLayoutResize(async () => {
      const window = getCurrentWebviewWindow();
      const scaleFactor = await window.scaleFactor();
      const logicalSize = (await window.outerSize()).toLogical(scaleFactor);
      const zoom = (settingStore.zoom || 100) / 100;
      const currentDesignHeight = logicalSize.height / zoom;
      const currentDesignWidth = logicalSize.width / zoom;

      if (sidebarOpening) {
        windowWidthBeforeSidebar = Math.max(PANEL_WIDTH, currentDesignWidth);
        windowWidthWithSidebar = Math.max(windowWidthBeforeSidebar, sidebarWidth * 2);
      }

      const targetDesignWidth = willSidebarOpen
        ? windowWidthWithSidebar
        : sidebarClosing
          ? windowWidthBeforeSidebar
          : currentDesignWidth;
      const minimumDesignWidth = willSidebarOpen ? targetDesignWidth : PANEL_WIDTH;

      // Opening/closing changes the resize constraints even when the current
      // window is already wide enough and no physical resize is necessary.
      if (sidebarOpening || sidebarClosing) {
        await applySizeConstraints("chat", OVERLAY_MIN_HEIGHT_CHAT, minimumDesignWidth);
      }
      if (Math.abs(deltaExtra) < 0.5 && Math.abs(currentDesignWidth - targetDesignWidth) < 0.5) {
        return;
      }
      if (!sidebarOpening && !sidebarClosing) {
        await applySizeConstraints("chat", OVERLAY_MIN_HEIGHT_CHAT, minimumDesignWidth);
      }
      const nextHeight = await constrainedChatHeight(currentDesignHeight + deltaExtra);
      await resizeWindow(targetDesignWidth, nextHeight, true);
    });
  }
}

async function monitorHeightLimit() {
  const window = getCurrentWebviewWindow();
  const monitor = await currentMonitor();
  if (!monitor) return null;
  const scaleFactor = await window.scaleFactor();
  const monitorSize = monitor.size.toLogical(scaleFactor);
  const zoom = (settingStore.zoom || 100) / 100;
  return Math.max(OVERLAY_MIN_HEIGHT_CHAT, (monitorSize.height - CHAT_SCREEN_MARGIN) / zoom);
}

async function preferredChatHeight(extraHeight = 0) {
  const limit = await monitorHeightLimit();
  const preferred = CHAT_HEIGHT_PREFERRED + extraHeight;
  return limit == null ? preferred : Math.min(preferred, limit);
}

async function constrainedChatHeight(height: number) {
  const limit = await monitorHeightLimit();
  return limit == null ? height : Math.min(height, limit);
}

async function enterChatMode(nextSessionId: string) {
  sessionId.value = nextSessionId;
  chatStore.setOverlayDraftSession(nextSessionId);
  mode.value = "chat";
  chatWindowInitialized.value = false;
  lastComposerExtraHeight.value = 0;
  diffSidebarOpen.value = false;
  subagentSidebarOpen.value = false;
  runtimeSidebarOpen.value = false;
  imageSidebarOpen.value = false;
  windowWidthBeforeSidebar = PANEL_WIDTH;
  windowWidthWithSidebar = PANEL_WIDTH;
  await setOverlayChatMode(windowLabel, true);
  // 先切换 UI、等一帧绘制，再以底边为锚点展开，让输入框保持原位
  await waitForNextFrame();
  await applySizeConstraints("chat", OVERLAY_MIN_HEIGHT_CHAT);
  await resizeWindow(PANEL_WIDTH, await preferredChatHeight(), true, false, "bottom");
  chatWindowInitialized.value = true;
}

async function resetToInputMode() {
  mode.value = "input";
  sessionId.value = "";
  chatWindowInitialized.value = false;
  lastComposerExtraHeight.value = 0;
  diffSidebarOpen.value = false;
  subagentSidebarOpen.value = false;
  runtimeSidebarOpen.value = false;
  imageSidebarOpen.value = false;
  windowWidthBeforeSidebar = PANEL_WIDTH;
  windowWidthWithSidebar = PANEL_WIDTH;
  chatStore.setOverlayDraftSession("");
  await setOverlayChatMode(windowLabel, false);
  lastInputDesignWidth = PANEL_WIDTH;
  lastInputDesignHeight = INPUT_HEIGHT;
  await applySizeConstraints("input", OVERLAY_MIN_HEIGHT_INPUT);
  await resizeWindow(PANEL_WIDTH, INPUT_HEIGHT, false, false, "bottom");
}

async function close() {
  // 直接通知 Rust 关闭/销毁，由 Rust 侧负责清理状态
  // 不能先 resetToInputMode()，否则会提前清除 chat mode 导致竞态
  await closeOverlay(windowLabel);
}

onMounted(async () => {
  const window = getCurrentWebviewWindow();
  void window.setMaximizable(false);
  // Sync native size with design px + UI zoom before the first paint settles.
  // tauri.conf / window.rs create 640×84, but zoomed shells still need a
  // matching LogicalSize or the first Alt+Alt frame looks clipped.
  if (mode.value === "input") {
    queueLayoutResize(async () => {
      lastInputDesignWidth = PANEL_WIDTH;
      lastInputDesignHeight = INPUT_HEIGHT;
      await applySizeConstraints("input", OVERLAY_MIN_HEIGHT_INPUT);
      await resizeWindow(PANEL_WIDTH, INPUT_HEIGHT, false, false, "bottom");
    });
  }
  void window.listen<CapturedContext>(IPC_EVENTS.contextCaptured, (event) => {
    if (mode.value === "chat") {
      return;
    }
    capturedContext.value = event.payload;
    contextReady.value = true;
    log.debug("overlay interactive ready", { windowLabel, source: "context-captured" });
  });
  const pendingContext = await takeOverlayContext(windowLabel);
  if (pendingContext && mode.value === "input") {
    capturedContext.value = pendingContext;
    contextReady.value = true;
    log.debug("overlay interactive ready", { windowLabel, source: "pending-context" });
  }
  // 基础 overlay 窗口：监听 overlay-hidden 重置 UI
  // 动态窗口（overlay-N）即将被销毁，不需要 reset UI/resize
  const isBaseOverlay = windowLabel === "overlay";
  if (isBaseOverlay) {
    void window.listen("overlay-hidden", () => {
      capturedContext.value = null;
      contextReady.value = false;
      void resetToInputMode();
    });
  }
});

watch(
  () => settingStore.zoom,
  async () => {
    if (mode.value === "chat") {
      const sidebarOpen =
        diffSidebarOpen.value ||
        subagentSidebarOpen.value ||
        runtimeSidebarOpen.value ||
        imageSidebarOpen.value;
      const designWidth = sidebarOpen ? windowWidthWithSidebar : windowWidthBeforeSidebar;
      await applySizeConstraints("chat", OVERLAY_MIN_HEIGHT_CHAT, designWidth);
      const window = getCurrentWebviewWindow();
      const scaleFactor = await window.scaleFactor();
      const logicalSize = (await window.outerSize()).toLogical(scaleFactor);
      const zoom = (settingStore.zoom || 100) / 100;
      await resizeWindow(designWidth, logicalSize.height / zoom, true);
    } else {
      await applySizeConstraints("input", OVERLAY_MIN_HEIGHT_INPUT);
      await resizeWindow(PANEL_WIDTH, INPUT_HEIGHT, false, false, "bottom");
    }
  },
);
</script>

<style scoped>
.overlay-shell {
  box-sizing: border-box;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: transparent;
}
</style>
