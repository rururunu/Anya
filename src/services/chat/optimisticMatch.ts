import { isPlanExecutePrompt } from "@/services/chat/planProposal";

/** True when a `local-user-*` bubble should be replaced by this ChatStarted user message. */
export function optimisticUserMatchesServer(
  localContent: string,
  serverContent: string,
  resumePlan = false,
): boolean {
  if (localContent === serverContent) return true;
  // Server may append model-only context (legacy checklist) while keeping the prefix.
  if (serverContent.startsWith(localContent)) return true;
  if (resumePlan && isPlanExecutePrompt(localContent) && isPlanExecutePrompt(serverContent)) {
    return true;
  }
  return false;
}
