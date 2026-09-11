import { computed, onBeforeUnmount, ref } from "vue";

export type PetAction =
  "idle" | "twirl" | "spin" | "bounce" | "jiggle" | "curious" | "double-hop" | "dizzy";

export type TiltDirection = "left" | "right" | "center";

const SINGLE_ACTIONS: PetAction[] = ["twirl", "bounce", "curious", "jiggle", "double-hop"];
const COMBO_ACTIONS: PetAction[] = ["twirl", "double-hop", "bounce"];

const ACTION_DURATIONS: Record<PetAction, number> = {
  idle: 0,
  twirl: 660,
  spin: 660,
  bounce: 600,
  jiggle: 560,
  curious: 650,
  "double-hop": 620,
  dizzy: 750,
};

/**
 * 管理桌面宠物丰富的点击动作库、连击计数与物理受力响应。
 */
export function useDesktopPetMotion() {
  const currentAction = ref<PetAction>("idle");
  const tiltDirection = ref<TiltDirection>("center");
  const showComboDecor = ref(false);
  const comboCount = ref(0);

  let lastAction: PetAction = "idle";
  let lastClickTimestamp = 0;
  let actionTimer: ReturnType<typeof setTimeout> | null = null;
  let comboResetTimer: ReturnType<typeof setTimeout> | null = null;
  let actionFrame = 0;

  /** 从候选动作中挑选一个，尽量避免连续相同动作。 */
  function pickNextAction(pool: PetAction[]): PetAction {
    const available = pool.length > 1 ? pool.filter((a) => a !== lastAction) : pool;
    const index = Math.floor(Math.random() * available.length);
    return available[index] || pool[0];
  }

  /** 计算鼠标点击偏向（左偏/右偏/居中），用于产生轻微受力回弹。 */
  function calculateTiltDirection(e?: MouseEvent): TiltDirection {
    if (!e || !(e.currentTarget instanceof HTMLElement)) {
      return "center";
    }
    const rect = e.currentTarget.getBoundingClientRect();
    const centerX = rect.left + rect.width / 2;
    const diffX = e.clientX - centerX;
    const threshold = rect.width * 0.18;
    if (diffX < -threshold) return "left";
    if (diffX > threshold) return "right";
    return "center";
  }

  /** 触发点击交互动作，结合连击机制与多样动作。 */
  function triggerClickReaction(e?: MouseEvent) {
    const now = Date.now();
    if (now - lastClickTimestamp < 900) {
      comboCount.value++;
    } else {
      comboCount.value = 1;
    }
    lastClickTimestamp = now;

    if (comboResetTimer) clearTimeout(comboResetTimer);
    comboResetTimer = setTimeout(() => {
      comboCount.value = 0;
      comboResetTimer = null;
    }, 1200);

    tiltDirection.value = calculateTiltDirection(e);

    let nextAction: PetAction;
    if (comboCount.value >= 5) {
      nextAction = "dizzy";
      comboCount.value = 0;
    } else if (comboCount.value >= 2) {
      nextAction = pickNextAction(COMBO_ACTIONS);
    } else {
      nextAction = pickNextAction(SINGLE_ACTIONS);
    }

    lastAction = nextAction;
    showComboDecor.value = comboCount.value >= 3;

    // 重启当前动作动画
    currentAction.value = "idle";
    if (actionFrame) cancelAnimationFrame(actionFrame);
    if (actionTimer) clearTimeout(actionTimer);

    actionFrame = requestAnimationFrame(() => {
      actionFrame = 0;
      currentAction.value = nextAction;
      const duration =
        (nextAction === "twirl" || nextAction === "spin") && comboCount.value >= 2
          ? 780
          : ACTION_DURATIONS[nextAction] || 540;
      actionTimer = setTimeout(() => {
        currentAction.value = "idle";
        tiltDirection.value = "center";
        showComboDecor.value = false;
        actionTimer = null;
      }, duration);
    });
  }

  /** 重置并清除当前所有动作与计时器。 */
  function resetMotion() {
    if (actionTimer) clearTimeout(actionTimer);
    if (comboResetTimer) clearTimeout(comboResetTimer);
    if (actionFrame) cancelAnimationFrame(actionFrame);
    actionTimer = null;
    comboResetTimer = null;
    actionFrame = 0;
    currentAction.value = "idle";
    tiltDirection.value = "center";
    showComboDecor.value = false;
    comboCount.value = 0;
  }

  const isCombo = computed(() => comboCount.value >= 2);

  onBeforeUnmount(() => {
    resetMotion();
  });

  return {
    currentAction,
    tiltDirection,
    showComboDecor,
    comboCount,
    isCombo,
    triggerClickReaction,
    resetMotion,
  };
}
