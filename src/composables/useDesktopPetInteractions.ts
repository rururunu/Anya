import { computed, onBeforeUnmount, onMounted, ref, type Ref } from "vue";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { currentMonitor } from "@tauri-apps/api/window";
import { LogicalSize, PhysicalPosition } from "@tauri-apps/api/dpi";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  listChatSessions,
  respondAskUser,
  respondPathPermission,
  respondToolApproval,
} from "@/services/ipc/commands";
import { listWorkspaces } from "@/commands/workspace";
import { IPC_EVENTS } from "@/types/ipc";
import type {
  AskUserEvent,
  InteractionResolvedEvent,
  PathPermissionDecision,
  PathPermissionEvent,
  ToolApprovalDecision,
  ToolApprovalEvent,
} from "@/types/chat";
import { isPluginAgentSessionId } from "@/services/chat/pluginSession";
import { PET_SIZES, type PetSize } from "./useDesktopPet";

export type PetInteraction =
  | {
      kind: "ask_user";
      requestId: string;
      sessionId: string;
      sessionTitle: string;
      workspaceName?: string;
      header?: string;
      question: string;
      options: Array<{ label: string; description?: string }>;
      multiSelect?: boolean;
    }
  | {
      kind: "path_permission";
      requestId: string;
      sessionId: string;
      sessionTitle: string;
      workspaceName?: string;
      path: string;
      operation: string;
      toolName: string;
    }
  | {
      kind: "tool_approval";
      requestId: string;
      sessionId: string;
      sessionTitle: string;
      workspaceName?: string;
      toolName: string;
      title: string;
      diffSummary?: string;
    };

export interface UseDesktopPetInteractionsOptions {
  petSize: Ref<PetSize>;
}

const EXPANDED_WIDTH = 376;
const EXPANDED_EXTRA_HEIGHT = 246;

/**
 * 管理桌面宠物的 Agent 提问与操作授权面板状态、跨窗口响应与自适应尺寸伸缩。
 */
export function useDesktopPetInteractions(options: UseDesktopPetInteractionsOptions) {
  const { petSize } = options;
  const appWindow = getCurrentWebviewWindow();
  const pendingInteractions = ref<PetInteraction[]>([]);
  const currentIndex = ref(0);

  const activeInteraction = computed<PetInteraction | null>(() => {
    if (pendingInteractions.value.length === 0) return null;
    const safeIndex = Math.min(
      Math.max(0, currentIndex.value),
      pendingInteractions.value.length - 1,
    );
    return pendingInteractions.value[safeIndex] ?? null;
  });

  const interactionCount = computed(() => pendingInteractions.value.length);
  const currentInteractionIndex = computed(() => {
    if (pendingInteractions.value.length === 0) return 0;
    return Math.min(Math.max(0, currentIndex.value), pendingInteractions.value.length - 1);
  });

  const unlistenFns: UnlistenFn[] = [];

  let isExpanded = false;

  function addOrUpdateInteraction(interaction: PetInteraction) {
    const existingIdx = pendingInteractions.value.findIndex(
      (it) => it.requestId === interaction.requestId,
    );
    if (existingIdx >= 0) {
      const next = [...pendingInteractions.value];
      next[existingIdx] = interaction;
      pendingInteractions.value = next;
    } else {
      pendingInteractions.value = [...pendingInteractions.value, interaction];
    }
  }

  function removeInteractionByRequestId(requestId: string) {
    const next = pendingInteractions.value.filter((it) => it.requestId !== requestId);
    pendingInteractions.value = next;
    if (currentIndex.value >= next.length && next.length > 0) {
      currentIndex.value = next.length - 1;
    }
  }

  function nextInteraction() {
    if (pendingInteractions.value.length <= 1) return;
    currentIndex.value = (currentIndex.value + 1) % pendingInteractions.value.length;
  }

  function prevInteraction() {
    if (pendingInteractions.value.length <= 1) return;
    currentIndex.value =
      (currentIndex.value - 1 + pendingInteractions.value.length) %
      pendingInteractions.value.length;
  }

  /** 解析会话标题与所属工作区名称。 */
  async function resolveSessionMeta(
    sessionId: string,
  ): Promise<{ title: string; workspaceName?: string }> {
    try {
      const [sessionsRes, workspaces] = await Promise.all([
        listChatSessions().catch(() => ({ sessions: [] })),
        listWorkspaces().catch(() => []),
      ]);

      const session = sessionsRes.sessions.find((s) => s.sessionId === sessionId);
      const title = session?.preview?.trim() || "新对话";

      let workspaceName: string | undefined;
      if (session?.workspaceId) {
        const ws = workspaces.find((w) => w.id === session.workspaceId);
        if (ws?.name) {
          workspaceName = ws.name;
        }
      }

      return { title, workspaceName };
    } catch (err) {
      console.warn("resolveSessionMeta error:", err);
      return { title: "新对话" };
    }
  }

  /** 展开窗口以显示交互授权面板，并保持吉祥物在屏幕中的位置锚定。 */
  async function expandWindow() {
    if (isExpanded) return;
    isExpanded = true;
    const conf = PET_SIZES[petSize.value];
    const expandedHeight = conf.windowHeight + EXPANDED_EXTRA_HEIGHT;
    const deltaW = EXPANDED_WIDTH - conf.windowWidth;
    const deltaH = EXPANDED_EXTRA_HEIGHT;

    try {
      const pos = await appWindow.outerPosition();
      const monitor = await currentMonitor();
      const scale = monitor?.scaleFactor || 1;

      let newX = pos.x - Math.round(deltaW * scale);
      let newY = pos.y - Math.round(deltaH * scale);

      if (monitor) {
        newX = Math.max(0, Math.min(newX, monitor.size.width - Math.round(EXPANDED_WIDTH * scale)));
        newY = Math.max(
          0,
          Math.min(newY, monitor.size.height - Math.round(expandedHeight * scale)),
        );
      }

      await appWindow.setPosition(new PhysicalPosition(newX, newY));
      await appWindow.setSize(new LogicalSize(EXPANDED_WIDTH, expandedHeight));
    } catch (e) {
      console.warn("Failed to expand pet window for interaction:", e);
    }
  }

  /** 收缩窗口恢复常规紧凑桌面宠物大小。 */
  async function shrinkWindow() {
    if (!isExpanded) return;
    isExpanded = false;
    const conf = PET_SIZES[petSize.value];
    const deltaW = EXPANDED_WIDTH - conf.windowWidth;
    const deltaH = EXPANDED_EXTRA_HEIGHT;

    try {
      const pos = await appWindow.outerPosition();
      const monitor = await currentMonitor();
      const scale = monitor?.scaleFactor || 1;

      let newX = pos.x + Math.round(deltaW * scale);
      let newY = pos.y + Math.round(deltaH * scale);

      if (monitor) {
        newX = Math.max(
          0,
          Math.min(newX, monitor.size.width - Math.round(conf.windowWidth * scale)),
        );
        newY = Math.max(
          0,
          Math.min(newY, monitor.size.height - Math.round(conf.windowHeight * scale)),
        );
      }

      await appWindow.setSize(new LogicalSize(conf.windowWidth, conf.windowHeight));
      await appWindow.setPosition(new PhysicalPosition(newX, newY));
    } catch (e) {
      console.warn("Failed to shrink pet window after interaction:", e);
    }
  }

  /** 提交提问回答并关闭当前项或切换至下一项。 */
  async function submitAskUserAnswer(answer: string) {
    const current = activeInteraction.value;
    if (!current || current.kind !== "ask_user") return;
    const { requestId } = current;
    removeInteractionByRequestId(requestId);
    if (pendingInteractions.value.length === 0) {
      await shrinkWindow();
    }
    try {
      await respondAskUser({ requestId, answer });
    } catch (err) {
      console.warn("respondAskUser failed:", err);
    }
  }

  /** 提交文件路径权限决定并关闭当前项或切换至下一项。 */
  async function submitPathPermission(decision: PathPermissionDecision) {
    const current = activeInteraction.value;
    if (!current || current.kind !== "path_permission") return;
    const { requestId } = current;
    removeInteractionByRequestId(requestId);
    if (pendingInteractions.value.length === 0) {
      await shrinkWindow();
    }
    try {
      await respondPathPermission({ requestId, decision });
    } catch (err) {
      console.warn("respondPathPermission failed:", err);
    }
  }

  /** 提交工具执行审批决定并关闭当前项或切换至下一项。 */
  async function submitToolApproval(decision: ToolApprovalDecision) {
    const current = activeInteraction.value;
    if (!current || current.kind !== "tool_approval") return;
    const { requestId } = current;
    removeInteractionByRequestId(requestId);
    if (pendingInteractions.value.length === 0) {
      await shrinkWindow();
    }
    try {
      await respondToolApproval({ requestId, decision });
    } catch (err) {
      console.warn("respondToolApproval failed:", err);
    }
  }

  /** 关闭/忽略当前待处理项。 */
  async function dismissInteraction() {
    const current = activeInteraction.value;
    if (current) {
      removeInteractionByRequestId(current.requestId);
    }
    if (pendingInteractions.value.length === 0) {
      await shrinkWindow();
    }
  }

  onMounted(async () => {
    // 监听 ask-user 提问
    const unlistenAsk = await listen<AskUserEvent>(IPC_EVENTS.askUser, async (event) => {
      const payload = event.payload;
      if (isPluginAgentSessionId(payload.sessionId || "")) return;
      const firstQ = payload.questions?.[0];
      if (!firstQ) return;
      const { title, workspaceName } = await resolveSessionMeta(payload.sessionId);
      addOrUpdateInteraction({
        kind: "ask_user",
        requestId: payload.requestId,
        sessionId: payload.sessionId,
        sessionTitle: title,
        workspaceName,
        header: firstQ.header,
        question: firstQ.question,
        options: firstQ.options || [],
        multiSelect: firstQ.multiSelect,
      });
      await expandWindow();
    });
    unlistenFns.push(unlistenAsk);

    // 监听 path-permission 路径权限
    const unlistenPath = await listen<PathPermissionEvent>(
      IPC_EVENTS.pathPermission,
      async (event) => {
        const payload = event.payload;
        if (isPluginAgentSessionId(payload.sessionId || "")) return;
        const { title, workspaceName } = await resolveSessionMeta(payload.sessionId);
        addOrUpdateInteraction({
          kind: "path_permission",
          requestId: payload.requestId,
          sessionId: payload.sessionId,
          sessionTitle: title,
          workspaceName,
          path: payload.path,
          operation: payload.operation,
          toolName: payload.toolName,
        });
        await expandWindow();
      },
    );
    unlistenFns.push(unlistenPath);

    // 监听 tool-approval 工具审批
    const unlistenTool = await listen<ToolApprovalEvent>(IPC_EVENTS.toolApproval, async (event) => {
      const payload = event.payload;
      if (isPluginAgentSessionId(payload.sessionId || "")) return;
      const { title, workspaceName } = await resolveSessionMeta(payload.sessionId);
      addOrUpdateInteraction({
        kind: "tool_approval",
        requestId: payload.requestId,
        sessionId: payload.sessionId,
        sessionTitle: title,
        workspaceName,
        toolName: payload.toolName,
        title: payload.title,
        diffSummary: payload.preview?.path,
      });
      await expandWindow();
    });
    unlistenFns.push(unlistenTool);

    // 监听其它窗口（工作台/快捷提问）已解决交互事件
    const unlistenResolved = await listen<InteractionResolvedEvent>(
      IPC_EVENTS.interactionResolved,
      async (event) => {
        removeInteractionByRequestId(event.payload.requestId);
        if (pendingInteractions.value.length === 0) {
          await shrinkWindow();
        }
      },
    );
    unlistenFns.push(unlistenResolved);

    // 监听会话结束或取消事件，自动清理孤儿交互
    const unlistenChatFinished = await listen<{ sessionId?: string; session_id?: string }>(
      IPC_EVENTS.chatFinished,
      async (event) => {
        const sId = event.payload?.sessionId ?? event.payload?.session_id;
        if (sId) {
          const next = pendingInteractions.value.filter((it) => it.sessionId !== sId);
          if (next.length !== pendingInteractions.value.length) {
            pendingInteractions.value = next;
            if (currentIndex.value >= next.length && next.length > 0) {
              currentIndex.value = next.length - 1;
            }
            if (next.length === 0) {
              await shrinkWindow();
            }
          }
        }
      },
    );
    unlistenFns.push(unlistenChatFinished);
  });

  onBeforeUnmount(() => {
    for (const fn of unlistenFns) fn();
  });

  return {
    activeInteraction,
    pendingInteractions,
    interactionCount,
    currentInteractionIndex,
    nextInteraction,
    prevInteraction,
    submitAskUserAnswer,
    submitPathPermission,
    submitToolApproval,
    dismissInteraction,
    shrinkWindow,
  };
}
