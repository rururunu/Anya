/**
 * Boot cache for the composer-relevant settings.
 *
 * `useSettingStore` starts from `defaultSettings` and only learns the persisted
 * values once `get_app_settings` answers, so the input bar painted `reasoningEffort:
 * "disabled"` (思考强度「关闭」) for the first frames even when the user had chosen
 * 「高」— colours already avoid this via `readCachedColorScheme()`. Cache just the
 * fields the composer renders on first paint; everything else keeps loading.
 */

import { normalizeReasoningEffort, type AppSettings } from "@/types/setting";

const COMPOSER_SETTINGS_CACHE_KEY = "aaa.composerSettings.v1";
const CACHE_VERSION = 1;

export type CachedComposerSettings = Pick<
  AppSettings,
  "reasoningEffort" | "chatModel" | "chatModelProvider"
>;

type CachedPayload = CachedComposerSettings & { version: number; savedAt: number };

/** Cached composer settings, or `null` when nothing trustworthy was stored. */
export function readCachedComposerSettings(): CachedComposerSettings | null {
  try {
    const cached = localStorage.getItem(COMPOSER_SETTINGS_CACHE_KEY);
    if (!cached) {
      return null;
    }
    const parsed = JSON.parse(cached) as Partial<CachedPayload> | null;
    if (!parsed || parsed.version !== CACHE_VERSION) {
      return null;
    }
    // Without a model the composer cannot render anything useful, so treat the
    // whole snapshot as unusable rather than seeding a half-empty state.
    if (typeof parsed.chatModel !== "string" || !parsed.chatModel.trim()) {
      return null;
    }
    return {
      reasoningEffort: normalizeReasoningEffort(parsed.reasoningEffort),
      chatModel: parsed.chatModel,
      chatModelProvider:
        typeof parsed.chatModelProvider === "string" ? parsed.chatModelProvider : "",
    };
  } catch {
    // Private mode / corrupted payload.
    return null;
  }
}

/** Persist the composer-relevant fields of a settings snapshot. */
export function writeCachedComposerSettings(settings: CachedComposerSettings): void {
  try {
    const payload: CachedPayload = {
      version: CACHE_VERSION,
      savedAt: Date.now(),
      reasoningEffort: normalizeReasoningEffort(settings.reasoningEffort),
      chatModel: settings.chatModel,
      chatModelProvider: settings.chatModelProvider,
    };
    localStorage.setItem(COMPOSER_SETTINGS_CACHE_KEY, JSON.stringify(payload));
  } catch {
    // Quota or storage unavailable — the in-memory settings are still valid.
  }
}
