import { onBeforeUnmount, ref } from "vue";

export type AutonomousAction = "none" | "wrench" | "morph" | "look-around" | "stretch";

const IDLE_ACTION_POOL: AutonomousAction[] = ["wrench", "morph", "look-around", "stretch"];

const AUTONOMOUS_DURATIONS: Record<AutonomousAction, number> = {
  none: 0,
  wrench: 2400,
  morph: 2600,
  "look-around": 2400,
  stretch: 1800,
};

interface AutonomousOptions {
  onEnterSleep?: () => void;
  canTrigger?: () => boolean;
}

/**
 * 桌面宠物无聊自主趣味行为调度器：
 * 在用户未操作时，宠物自发拿出扳手玩、形变成几何多边形、左顾右盼或伸懒腰，并在深闲置时自然过渡至睡眠。
 */
export function useDesktopPetAutonomous(options: AutonomousOptions = {}) {
  const currentAutonomousAction = ref<AutonomousAction>("none");
  const autonomousGaze = ref<{ x: number; y: number } | null>(null);

  let nextActionTimer: ReturnType<typeof setTimeout> | null = null;
  let actionFinishTimer: ReturnType<typeof setTimeout> | null = null;
  let deepSleepTimer: ReturnType<typeof setTimeout> | null = null;
  let gazeStepTimers: ReturnType<typeof setTimeout>[] = [];
  let lastAction: AutonomousAction = "none";
  let isActive = true;

  /** 清除左顾右盼步骤计时器。 */
  function clearGazeTimers() {
    gazeStepTimers.forEach((t) => clearTimeout(t));
    gazeStepTimers = [];
  }

  let hasTriggeredFirst = false;

  /** 从动作池中随机挑选一个与上次不同的趣味微动作。 */
  function pickNextAction(): AutonomousAction {
    if (!hasTriggeredFirst) {
      hasTriggeredFirst = true;
      lastAction = "morph";
      return "morph";
    }
    const candidates = IDLE_ACTION_POOL.filter((a) => a !== lastAction);
    const chosen = candidates[Math.floor(Math.random() * candidates.length)] || IDLE_ACTION_POOL[0];
    lastAction = chosen;
    return chosen;
  }

  /** 调度执行左顾右盼视线轨迹。 */
  function scheduleLookAroundGaze(duration: number) {
    clearGazeTimers();
    // 阶段 1：探头好奇望向左上方
    gazeStepTimers.push(
      setTimeout(() => {
        autonomousGaze.value = { x: -26, y: -5 };
      }, 100),
    );
    // 阶段 2：转头扫向右上方
    gazeStepTimers.push(
      setTimeout(() => {
        autonomousGaze.value = { x: 28, y: -4 };
      }, 950),
    );
    // 阶段 3：视线回归中央，准备眨眼
    gazeStepTimers.push(
      setTimeout(() => {
        autonomousGaze.value = { x: 0, y: 0 };
      }, 1800),
    );
    // 阶段 4：动作结束清除视线接管
    gazeStepTimers.push(
      setTimeout(() => {
        autonomousGaze.value = null;
      }, duration),
    );
  }

  /** 执行选定的无聊自主行为。 */
  function executeAutonomousAction(action: AutonomousAction) {
    currentAutonomousAction.value = action;
    const duration = AUTONOMOUS_DURATIONS[action] || 2000;

    if (action === "look-around") {
      scheduleLookAroundGaze(duration);
    } else {
      autonomousGaze.value = null;
    }

    if (actionFinishTimer) clearTimeout(actionFinishTimer);
    actionFinishTimer = setTimeout(() => {
      currentAutonomousAction.value = "none";
      autonomousGaze.value = null;
      actionFinishTimer = null;
      scheduleNextAutonomous();
    }, duration);
  }

  /** 安排下一次自主微动作的发生时间（首次 3.5s ~ 5.5s，后续 7s ~ 13s）。 */
  function scheduleNextAutonomous() {
    if (!isActive) return;
    if (nextActionTimer) clearTimeout(nextActionTimer);

    const delay = hasTriggeredFirst
      ? 7000 + Math.floor(Math.random() * 6000)
      : 3500 + Math.floor(Math.random() * 2000);
    nextActionTimer = setTimeout(() => {
      if (!isActive) return;
      if (options.canTrigger && !options.canTrigger()) {
        scheduleNextAutonomous();
        return;
      }
      executeAutonomousAction(pickNextAction());
    }, delay);
  }

  /** 重置深闲置打瞌睡倒计时（累计无交互 50 秒进入睡眠）。 */
  function resetSleepTimer() {
    if (deepSleepTimer) clearTimeout(deepSleepTimer);
    deepSleepTimer = setTimeout(() => {
      if (isActive && (!options.canTrigger || options.canTrigger())) {
        interruptAutonomous();
        options.onEnterSleep?.();
      }
    }, 50000);
  }

  /** 重置所有闲置计时（在用户发生点击、拖拽或对话交互时调用）。 */
  function resetIdleTimer() {
    interruptAutonomous();
    resetSleepTimer();
    scheduleNextAutonomous();
  }

  /** 立即打断当前正在进行的自主动作并恢复初始状态。 */
  function interruptAutonomous() {
    if (actionFinishTimer) clearTimeout(actionFinishTimer);
    if (nextActionTimer) clearTimeout(nextActionTimer);
    clearGazeTimers();

    actionFinishTimer = null;
    nextActionTimer = null;
    currentAutonomousAction.value = "none";
    autonomousGaze.value = null;
  }

  /** 启动调度器。 */
  function start() {
    isActive = true;
    resetIdleTimer();
  }

  /** 停止调度器。 */
  function stop() {
    isActive = false;
    interruptAutonomous();
    if (deepSleepTimer) clearTimeout(deepSleepTimer);
    deepSleepTimer = null;
  }

  onBeforeUnmount(() => {
    stop();
  });

  return {
    currentAutonomousAction,
    autonomousGaze,
    start,
    stop,
    resetIdleTimer,
    interruptAutonomous,
    executeAutonomousAction,
  };
}
