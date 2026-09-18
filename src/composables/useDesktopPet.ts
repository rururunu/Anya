import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { currentMonitor } from "@tauri-apps/api/window";
import { LogicalSize, PhysicalPosition } from "@tauri-apps/api/dpi";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { MascotExpression } from "@/components/icons/MascotPetView.vue";
import { useDesktopPetInteractions } from "./useDesktopPetInteractions";
import { useDesktopPetMotion } from "./useDesktopPetMotion";
export type { PetAction, TiltDirection } from "./useDesktopPetMotion";
import { useDesktopPetAutonomous } from "./useDesktopPetAutonomous";
export type { AutonomousAction } from "./useDesktopPetAutonomous";
import { useDesktopPetAgentSessions } from "./useDesktopPetAgentSessions";
import {
  showWorkbench,
  toggleDesktopPet,
  toggleOverlayFromPet,
  setDesktopPetSize,
} from "@/services/ipc/commands";
import { IPC_EVENTS } from "@/types/ipc";
import {
  DEFAULT_PET_APPEARANCE,
  normalizePetAppearance,
  type PetAppearance,
} from "@/services/pet/appearance";
import type { PetSpriteAnimationId } from "@/services/pet/spritesheet";

export type PetSize = "small" | "medium" | "large";

export type {
  PetAppearance,
  PetAppearanceMode,
  PetCompanionConfig,
  PetMediaKind,
} from "@/services/pet/appearance";

export interface PetSizeConfig {
  sizePx: number;
  windowWidth: number;
  windowHeight: number;
}

export const PET_SIZES: Record<PetSize, PetSizeConfig> = {
  small: { sizePx: 120, windowWidth: 170, windowHeight: 170 },
  medium: { sizePx: 155, windowWidth: 215, windowHeight: 215 },
  large: { sizePx: 190, windowWidth: 260, windowHeight: 260 },
};

const STORAGE_KEY_POS = "anya.desktop-pet.pos";
const STORAGE_KEY_SIZE = "anya.desktop-pet.size";
const STORAGE_KEY_LOCKED = "anya.desktop-pet.locked";

/**
 * 管理桌面宠物的状态响应、动作反馈、拖拽记忆与窗口配置。
 */
export function useDesktopPet() {
  const appWindow = getCurrentWebviewWindow();

  const isSleeping = ref(false);
  const manualExpression = ref<MascotExpression | null>(null);
  const pluginExpression = ref<MascotExpression | null>(null);
  const size = ref<PetSize>("medium");
  const locked = ref(false);
  const appearance = ref<PetAppearance>(DEFAULT_PET_APPEARANCE);
  const spriteGesture = ref<PetSpriteAnimationId | null>(null);
  let spriteGestureTimer: ReturnType<typeof setTimeout> | null = null;

  function clearSpriteGesture() {
    if (spriteGestureTimer) {
      clearTimeout(spriteGestureTimer);
      spriteGestureTimer = null;
    }
    spriteGesture.value = null;
  }

  function playSpriteGesture(id: PetSpriteAnimationId, durationMs = 700) {
    if (appearance.value.mode !== "spritesheet") return;
    clearSpriteGesture();
    spriteGesture.value = id;
    spriteGestureTimer = setTimeout(() => {
      spriteGesture.value = null;
      spriteGestureTimer = null;
    }, durationMs);
  }

  const {
    activeInteraction,
    interactionCount,
    currentInteractionIndex,
    prevInteraction,
    nextInteraction,
    submitAskUserAnswer,
    submitPathPermission,
    submitToolApproval,
    dismissInteraction,
  } = useDesktopPetInteractions({ petSize: size });

  const {
    currentAction,
    tiltDirection,
    showComboDecor,
    comboCount,
    isCombo,
    triggerClickReaction,
    resetMotion,
  } = useDesktopPetMotion();

  const agentSessions = useDesktopPetAgentSessions({
    onActivity: () => {
      interruptAutonomous();
      wakeUp();
    },
  });

  const expression = computed<MascotExpression>({
    get() {
      if (manualExpression.value) {
        return manualExpression.value;
      }
      if (pluginExpression.value) {
        return pluginExpression.value;
      }
      if (activeInteraction.value) {
        return "waiting";
      }
      const agentExp = agentSessions.agentExpression.value;
      if (agentExp !== "idle") {
        return agentExp;
      }
      if (isSleeping.value) {
        return "sleeping";
      }
      return "idle";
    },
    set(val) {
      if (val === "sleeping") {
        isSleeping.value = true;
        manualExpression.value = null;
      } else if (val === "idle") {
        isSleeping.value = false;
        manualExpression.value = null;
      } else {
        isSleeping.value = false;
        manualExpression.value = val;
      }
    },
  });

  const {
    currentAutonomousAction,
    autonomousGaze,
    start: startAutonomous,
    stop: stopAutonomous,
    resetIdleTimer,
    interruptAutonomous,
  } = useDesktopPetAutonomous({
    canTrigger: () =>
      expression.value === "idle" && currentAction.value === "idle" && !activeInteraction.value,
    onEnterSleep: () => {
      if (
        expression.value === "idle" &&
        currentAction.value === "idle" &&
        !activeInteraction.value
      ) {
        isSleeping.value = true;
      }
    },
  });

  const isBouncing = computed(() => currentAction.value !== "idle");

  const unlistenFns: UnlistenFn[] = [];

  watch(activeInteraction, (curr) => {
    if (curr) {
      interruptAutonomous();
      wakeUp();
    } else {
      resetIdleTimer();
    }
  });

  /** 如果正在打瞌睡，唤醒宠物回到待机。 */
  function wakeUp() {
    resetIdleTimer();
    isSleeping.value = false;
    manualExpression.value = null;
  }

  /** 触发 Q 弹跳跃反馈动画。 */
  function triggerBounce(e?: MouseEvent) {
    interruptAutonomous();
    if (appearance.value.mode === "spritesheet") {
      playSpriteGesture("wave", 700);
      return;
    }
    triggerClickReaction(e);
  }

  /** 单击唤醒并触发丰富交互动作。 */
  function onPetClick(e?: MouseEvent) {
    interruptAutonomous();
    wakeUp();
    if (appearance.value.mode === "spritesheet") {
      playSpriteGesture("wave", 700);
      return;
    }
    triggerClickReaction(e);
  }

  /** 双击直接唤起并聚焦 Anya 主工作台。 */
  async function onPetDoubleClick() {
    wakeUp();
    await showWorkbench();
  }

  /** 切换或设置宠物尺寸并同步窗口大小。 */
  async function setPetSize(nextSize: PetSize, syncRust = false) {
    size.value = nextSize;
    try {
      localStorage.setItem(STORAGE_KEY_SIZE, nextSize);
      const conf = PET_SIZES[nextSize];
      await appWindow.setSize(new LogicalSize(conf.windowWidth, conf.windowHeight));
      if (syncRust) {
        await setDesktopPetSize(nextSize);
      }
    } catch (e) {
      console.warn("Failed to set pet window size:", e);
    }
  }

  /** 切换锁定拖拽状态。 */
  function toggleLocked() {
    locked.value = !locked.value;
    try {
      localStorage.setItem(STORAGE_KEY_LOCKED, String(locked.value));
    } catch (e) {
      console.warn("Failed to persist locked state:", e);
    }
  }

  /** 显示并聚焦 Anya 主工作台。 */
  async function openWorkbench() {
    wakeUp();
    await showWorkbench();
  }

  /** 从宠物窗口直接调出 Overlay 快捷提问。 */
  async function openOverlay() {
    wakeUp();
    await toggleOverlayFromPet();
  }

  /** 隐藏桌面宠物。 */
  async function hidePet() {
    await toggleDesktopPet(false);
  }

  /** 恢复持久化的屏幕坐标，或者默认放置在屏幕右下角。 */
  async function restorePosition() {
    try {
      const savedPos = localStorage.getItem(STORAGE_KEY_POS);
      if (savedPos) {
        const { x, y } = JSON.parse(savedPos);
        if (typeof x === "number" && typeof y === "number") {
          await appWindow.setPosition(new PhysicalPosition(x, y));
          return;
        }
      }

      // 初次加载时，计算右下角默认安全位置
      const monitor = await currentMonitor();
      if (monitor) {
        const screenW = monitor.size.width;
        const screenH = monitor.size.height;
        const scale = monitor.scaleFactor || 1;
        const conf = PET_SIZES[size.value];
        const winW = conf.windowWidth * scale;
        const winH = conf.windowHeight * scale;

        // 放置在右下角，距离边框各留一些间隙
        const posX = Math.round(screenW - winW - 30 * scale);
        const posY = Math.round(screenH - winH - 60 * scale);
        await appWindow.setPosition(new PhysicalPosition(posX, posY));
      }
    } catch (e) {
      console.warn("Failed to restore pet position:", e);
    }
  }

  /** 保存当前窗口物理坐标。 */
  async function persistPosition() {
    try {
      const pos = await appWindow.outerPosition();
      localStorage.setItem(STORAGE_KEY_POS, JSON.stringify({ x: pos.x, y: pos.y }));
    } catch (e) {
      console.warn("Failed to persist pet position:", e);
    }
  }

  /** 处理鼠标按下与拖拽判定。 */
  let dragStartX = 0;
  let dragStartY = 0;
  let dragStartTime = 0;
  let hasDragged = false;

  function onMouseDown(e: MouseEvent) {
    if (e.button !== 0) return;
    wakeUp();
    dragStartX = e.screenX;
    dragStartY = e.screenY;
    dragStartTime = Date.now();
    hasDragged = false;

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp, { once: true });
  }

  async function onMouseMove(e: MouseEvent) {
    if (hasDragged || locked.value) return;
    const dx = Math.abs(e.screenX - dragStartX);
    const dy = Math.abs(e.screenY - dragStartY);
    if (dx > 4 || dy > 4) {
      hasDragged = true;
      window.removeEventListener("mousemove", onMouseMove);
      if (appearance.value.mode === "spritesheet") {
        const dir = e.screenX >= dragStartX ? "runRight" : "runLeft";
        playSpriteGesture(dir, 1200);
      }
      try {
        await appWindow.startDragging();
        // 拖拽完成后记录新位置
        await persistPosition();
        if (appearance.value.mode === "spritesheet") {
          clearSpriteGesture();
        }
      } catch (err) {
        console.warn("startDragging failed:", err);
      }
    }
  }

  function onMouseUp(e: MouseEvent) {
    window.removeEventListener("mousemove", onMouseMove);
    if (!hasDragged && e.button === 0) {
      const elapsed = Date.now() - dragStartTime;
      if (elapsed < 350) {
        onPetClick(e);
      }
    }
  }

  /** 监听宠物窗口配置与系统级事件。 */
  async function setupEventSubscriptions() {
    const unlistenSizeChanged = await listen<PetSize>(
      IPC_EVENTS.desktopPetSizeChanged,
      async (event) => {
        const nextSize = event.payload;
        if (nextSize && PET_SIZES[nextSize]) {
          await setPetSize(nextSize);
        }
      },
    );
    unlistenFns.push(unlistenSizeChanged);

    const applyAppearance = (raw: unknown) => {
      appearance.value = normalizePetAppearance(raw) ?? DEFAULT_PET_APPEARANCE;
    };

    const unlistenAppearance = await listen<unknown>(
      IPC_EVENTS.desktopPetAppearanceChanged,
      (event) => {
        applyAppearance(event.payload);
      },
    );
    unlistenFns.push(unlistenAppearance);

    const unlistenSkin = await listen<unknown>(IPC_EVENTS.desktopPetSkinChanged, (event) => {
      applyAppearance(event.payload);
    });
    unlistenFns.push(unlistenSkin);

    const knownExpressions = new Set<MascotExpression>([
      "idle",
      "thinking",
      "working",
      "talking",
      "waiting",
      "done",
      "error",
      "sleeping",
    ]);
    const unlistenExpression = await listen<string | null>(
      IPC_EVENTS.desktopPetExpressionChanged,
      (event) => {
        const next = event.payload;
        if (typeof next === "string" && knownExpressions.has(next as MascotExpression)) {
          pluginExpression.value = next as MascotExpression;
          if (next !== "sleeping") {
            isSleeping.value = false;
          }
          return;
        }
        pluginExpression.value = null;
      },
    );
    unlistenFns.push(unlistenExpression);
  }

  onMounted(async () => {
    // 恢复尺寸设置
    const savedSize = localStorage.getItem(STORAGE_KEY_SIZE) as PetSize | null;
    if (savedSize && PET_SIZES[savedSize]) {
      size.value = savedSize;
      const conf = PET_SIZES[savedSize];
      await appWindow.setSize(new LogicalSize(conf.windowWidth, conf.windowHeight));
    }
    // 恢复锁定状态
    locked.value = localStorage.getItem(STORAGE_KEY_LOCKED) === "true";

    await restorePosition();
    await setupEventSubscriptions();
    startAutonomous();
  });

  onBeforeUnmount(() => {
    clearSpriteGesture();
    resetMotion();
    stopAutonomous();
    for (const fn of unlistenFns) fn();
  });

  return {
    expression,
    currentAction,
    tiltDirection,
    showComboDecor,
    comboCount,
    isCombo,
    isBouncing,
    autonomousAction: currentAutonomousAction,
    autonomousGaze,
    size,
    locked,
    appearance,
    spriteGesture,
    activeInteraction,
    interactionCount,
    currentInteractionIndex,
    prevInteraction,
    nextInteraction,
    onMouseDown,
    onPetDoubleClick,
    onPetClick,
    triggerBounce,
    setPetSize,
    toggleLocked,
    openWorkbench,
    openOverlay,
    hidePet,
    wakeUp,
    submitAskUserAnswer,
    submitPathPermission,
    submitToolApproval,
    dismissInteraction,
  };
}
