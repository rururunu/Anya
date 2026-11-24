import type { MascotState } from "@/types/mascot";
import { isAskUserTool } from "@/services/chat/askUserAnswer";
import type { I18nKey } from "@/services/i18n";
import type { ChatMessage } from "@/types/chat";

export const MASCOT_STATE_LABEL_KEYS: Record<MascotState, I18nKey> = {
  idle: "mascotState.idle",
  thinking: "mascotState.thinking",
  working: "mascotState.working",
  talking: "mascotState.talking",
  waiting: "mascotState.waiting",
  done: "mascotState.done",
  error: "mascotState.error",
};

export function isWaitingForAskUser(message: ChatMessage) {
  return (message.toolActivities ?? []).some(
    (activity) => isAskUserTool(activity.toolName) && activity.status === "running",
  );
}

export function turnMascotState(message: ChatMessage): MascotState {
  switch (message.status) {
    case "error":
      return "error";
    case "cancelled":
      return "idle";
    case "done":
      return "done";
  }
  if (isWaitingForAskUser(message)) return "waiting";
  if ((message.toolActivities ?? []).some((activity) => activity.status === "running")) {
    return "working";
  }
  return message.content.trim() ? "talking" : "thinking";
}
