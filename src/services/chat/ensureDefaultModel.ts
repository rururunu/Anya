import { findModelEntry, isKnownModelSelection } from "@/lib/modelThinking";
import type { ChatModelInfo } from "@/types/chat";

export const CONFIGURE_PROVIDER_MARKER = "<!--peek:configure-provider-->";

export function isConfigureProviderError(content: string): boolean {
  if (!content.trim()) return false;
  if (content.includes(CONFIGURE_PROVIDER_MARKER)) return true;
  return /No model selected|credentials are not configured|Model credentials are not configured/i.test(
    content,
  );
}

export function stripConfigureProviderMarker(content: string): string {
  return content.replace(CONFIGURE_PROVIDER_MARKER, "").trim();
}

/** Pure selection helper — store wiring lives in chatModel.ensureDefault. */
export function selectDefaultChatModel(
  models: ChatModelInfo[],
  currentModel: string,
  currentProvider: string,
): { model: ChatModelInfo; needsPersist: boolean } | null {
  if (models.length === 0) return null;

  const current = currentModel.trim();
  if (current && isKnownModelSelection(models, current, currentProvider)) {
    return {
      model: findModelEntry(models, current, currentProvider) ?? models[0]!,
      needsPersist: false,
    };
  }

  return { model: models[0]!, needsPersist: true };
}
