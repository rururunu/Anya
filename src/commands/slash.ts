import { listChatSessions, openSettings } from "@/services/ipc";
import type { SlashI18nKey } from "@/services/locales/slash";

export interface SlashCommand {
  command: string;
  label: string;
  descriptionKey: SlashI18nKey;
}

export type SlashCommandAction =
  | "close"
  | "newConversation"
  | "openHistory"
  | "openModel"
  | "openThinking"
  | "openMode"
  | "openSecurity"
  | "openPopup"
  | "openWorkbench"
  | "openWorkspace"
  | "clearInput"
  | null;

export const slashCommands: SlashCommand[] = [
  { command: "/new", label: "new", descriptionKey: "slash.new.description" },
  {
    command: "/model",
    label: "model",
    descriptionKey: "slash.model.description",
  },
  {
    command: "/thinking",
    label: "thinking",
    descriptionKey: "slash.thinking.description",
  },
  {
    command: "/mode",
    label: "mode",
    descriptionKey: "slash.mode.description",
  },
  {
    command: "/security",
    label: "security",
    descriptionKey: "slash.security.description",
  },
  {
    command: "/work",
    label: "work",
    descriptionKey: "slash.work.description",
  },
  {
    command: "/history",
    label: "history",
    descriptionKey: "slash.history.description",
  },
  {
    command: "/clear",
    label: "clear",
    descriptionKey: "slash.clear.description",
  },
  { command: "/popup", label: "popup", descriptionKey: "slash.popup.description" },
  { command: "/workbench", label: "workbench", descriptionKey: "slash.workbench.description" },
  {
    command: "/settings",
    label: "settings",
    descriptionKey: "slash.settings.description",
  },
  {
    command: "/exit",
    label: "exit",
    descriptionKey: "slash.exit.description",
  },
];

export async function executeSlashCommand(command: string): Promise<SlashCommandAction> {
  switch (command) {
    case "/new":
      return "newConversation";
    case "/history":
      return "openHistory";
    case "/model":
      return "openModel";
    case "/thinking":
      return "openThinking";
    case "/mode":
      return "openMode";
    case "/security":
      return "openSecurity";
    case "/popup":
      return "openPopup";
    case "/workbench":
      return "openWorkbench";
    case "/settings":
      try {
        await openSettings();
      } catch (error) {
        console.error("Failed to open settings:", error);
      }
      return null;
    case "/work":
      return "openWorkspace";
    case "/exit":
      return "close";
    case "/clear":
      return "clearInput";
    default:
      return null;
  }
}

export async function fetchChatSessions() {
  try {
    const response = await listChatSessions();
    return response.sessions ?? [];
  } catch (error) {
    console.error("list_chat_sessions failed:", error);
    return [];
  }
}
