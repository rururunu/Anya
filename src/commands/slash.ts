import { getEnvironmentContext, listChatSessions, openSettings } from "@/services/ipc";
import type { CapturedContext } from "@/types/chat";
import type { SlashI18nKey } from "@/services/locales/slash";

export interface SlashCommand {
  command: string;
  label: string;
  descriptionKey: SlashI18nKey;
}

export type SlashCommandAction =
  | "close"
  | "openHistory"
  | "openModel"
  | "openThinking"
  | "openWorkspace"
  | "clearInput"
  | "showContext"
  | null;

export const slashCommands: SlashCommand[] = [
  {
    command: "/history",
    label: "history",
    descriptionKey: "slash.history.description",
  },
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
    command: "/settings",
    label: "settings",
    descriptionKey: "slash.settings.description",
  },
  {
    command: "/work",
    label: "work",
    descriptionKey: "slash.work.description",
  },
  {
    command: "/exit",
    label: "exit",
    descriptionKey: "slash.exit.description",
  },
  {
    command: "/context",
    label: "context",
    descriptionKey: "slash.context.description",
  },
  {
    command: "/clear",
    label: "clear",
    descriptionKey: "slash.clear.description",
  },
];

export async function executeSlashCommand(command: string): Promise<SlashCommandAction> {
  switch (command) {
    case "/history":
      return "openHistory";
    case "/model":
      return "openModel";
    case "/thinking":
      return "openThinking";
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
    case "/context":
      return "showContext";
    default:
      return null;
  }
}

export async function fetchEnvironmentContext(): Promise<CapturedContext> {
  return Promise.race([
    getEnvironmentContext(),
    new Promise<never>((_, reject) => {
      window.setTimeout(() => reject(new Error("get_environment_context timed out")), 2500);
    }),
  ]);
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
