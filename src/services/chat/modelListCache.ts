/**
 * Boot cache for the last successful `list_chat_models` reply.
 *
 * The picker used to open on an empty list until the host answered (provider
 * discovery is network-bound), so the panel flashed "loading" every launch.
 * Reading the previous reply synchronously at store creation keeps the list on
 * screen; `chatModel.softRefresh()` still replaces it with live data.
 */

import type { ChatModelInfo } from "@/types/chat";

const MODEL_LIST_CACHE_KEY = "aaa.chatModels.v1";
const CACHE_VERSION = 1;
/** A runaway provider list must not be able to blow the storage quota. */
const MAX_CACHED_MODELS = 2000;

type CachedModelList = {
  version: number;
  savedAt: number;
  models: ChatModelInfo[];
};

/** Keep only entries the picker can render; drop anything the host changed shape. */
function normalizeModel(value: unknown): ChatModelInfo | null {
  if (!value || typeof value !== "object") {
    return null;
  }
  const raw = value as Partial<ChatModelInfo>;
  if (typeof raw.id !== "string" || !raw.id.trim()) {
    return null;
  }
  if (typeof raw.provider !== "string") {
    return null;
  }
  return {
    ...raw,
    id: raw.id,
    ownedBy: typeof raw.ownedBy === "string" ? raw.ownedBy : raw.provider,
    provider: raw.provider,
  };
}

/** Last cached model list, or `[]` when there is nothing usable. */
export function readCachedModelList(): ChatModelInfo[] {
  try {
    const cached = localStorage.getItem(MODEL_LIST_CACHE_KEY);
    if (!cached) {
      return [];
    }
    const parsed = JSON.parse(cached) as Partial<CachedModelList> | null;
    if (!parsed || parsed.version !== CACHE_VERSION || !Array.isArray(parsed.models)) {
      return [];
    }
    return parsed.models
      .map(normalizeModel)
      .filter((model): model is ChatModelInfo => model != null)
      .slice(0, MAX_CACHED_MODELS);
  } catch {
    // Private mode / corrupted payload — fall back to an empty list.
    return [];
  }
}

/** Persist a host-confirmed model list for the next launch. */
export function writeCachedModelList(models: ChatModelInfo[]): void {
  try {
    const payload: CachedModelList = {
      version: CACHE_VERSION,
      savedAt: Date.now(),
      models: models.slice(0, MAX_CACHED_MODELS),
    };
    localStorage.setItem(MODEL_LIST_CACHE_KEY, JSON.stringify(payload));
  } catch {
    // Quota or storage unavailable — the in-memory list is still valid.
  }
}
