/**
 * Per-session compose cache (model / mode / draft / image-gen) + remote sync.
 * Module-level singletons stay process-wide — do not put these in Pinia state.
 */

import { normalizeChatMode } from "@/types/setting";
import {
  defaultImageGenCompose,
  normalizeImageGenCompose,
  type ImageGenCompose,
} from "@/services/chat/imageGenMode";

/** Per-conversation compose settings. Each conversation remembers its own
 * model / mode / approval choice and input draft; unopened sessions inherit
 * the previous session's values on first open, then stay independent. */
export interface SessionCompose {
  chatModel: string;
  chatModelProvider: string;
  chatMode: "ask" | "agent" | "plan" | "image";
  toolApprovalMode: "ask" | "auto" | "alwaysAllow";
  imageGen: ImageGenCompose;
  draft: string;
  /** Workspace binding for draft-only (not-yet-sent) sessions shown in the sidebar. */
  draftWorkspaceId?: string | null;
  draftUpdatedAt?: number;
}

export function defaultCompose(): SessionCompose {
  return {
    chatModel: "",
    chatModelProvider: "",
    chatMode: "agent",
    toolApprovalMode: "ask",
    imageGen: defaultImageGenCompose(),
    draft: "",
  };
}

export function sanitizeCompose(raw: Partial<SessionCompose> | null | undefined): SessionCompose {
  const base = defaultCompose();
  if (!raw || typeof raw !== "object") {
    return base;
  }
  const approval = raw.toolApprovalMode;
  return {
    chatModel: typeof raw.chatModel === "string" ? raw.chatModel : base.chatModel,
    chatModelProvider:
      typeof raw.chatModelProvider === "string" ? raw.chatModelProvider : base.chatModelProvider,
    chatMode: normalizeChatMode(raw.chatMode),
    toolApprovalMode:
      approval === "ask" || approval === "auto" || approval === "alwaysAllow"
        ? approval
        : base.toolApprovalMode,
    imageGen: normalizeImageGenCompose(raw.imageGen),
    draft: typeof raw.draft === "string" ? raw.draft : "",
    draftWorkspaceId: raw.draftWorkspaceId ?? null,
    draftUpdatedAt: typeof raw.draftUpdatedAt === "number" ? raw.draftUpdatedAt : undefined,
  };
}

const COMPOSE_STORAGE_KEY = "aaa.sessionCompose.v1";
export const REJECTED_PLAN_STORAGE_KEY = "aaa.rejectedPlanFingerprint.v1";

interface ComposeCache {
  entries: Record<string, SessionCompose>;
  last: string;
}

let composeCacheLoaded = false;
export const composeCache: ComposeCache = { entries: {}, last: "" };

export function loadComposeCache(): void {
  if (composeCacheLoaded) {
    return;
  }
  composeCacheLoaded = true;
  try {
    const raw = localStorage.getItem(COMPOSE_STORAGE_KEY);
    if (!raw) {
      return;
    }
    const parsed = JSON.parse(raw) as Partial<ComposeCache>;
    if (parsed && typeof parsed === "object") {
      const entries: Record<string, SessionCompose> = {};
      for (const [id, value] of Object.entries(parsed.entries ?? {})) {
        entries[id] = sanitizeCompose(value as Partial<SessionCompose>);
      }
      composeCache.entries = entries;
      composeCache.last = typeof parsed.last === "string" ? parsed.last : "";
    }
  } catch {
    // Corrupted cache — start fresh.
  }
}

export function persistComposeCache(): void {
  try {
    localStorage.setItem(COMPOSE_STORAGE_KEY, JSON.stringify(composeCache));
  } catch {
    // Storage unavailable — keep in-memory state only.
  }
}

/** Draft typing hits setComposeDraft often — coalesce localStorage writes. */
let persistComposeCacheTimer: ReturnType<typeof setTimeout> | null = null;
export function schedulePersistComposeCache(delayMs = 1000): void {
  if (persistComposeCacheTimer) {
    clearTimeout(persistComposeCacheTimer);
  }
  persistComposeCacheTimer = setTimeout(() => {
    persistComposeCacheTimer = null;
    persistComposeCache();
  }, delayMs);
}

export function flushPersistComposeCache(): void {
  if (persistComposeCacheTimer) {
    clearTimeout(persistComposeCacheTimer);
    persistComposeCacheTimer = null;
  }
  persistComposeCache();
}

/** Coalesce draft-only sidebar preview refreshes while typing a new chat. */
let draftListBumpTimer: ReturnType<typeof setTimeout> | null = null;
export function scheduleDraftListBump(bump: () => void): void {
  if (draftListBumpTimer) return;
  draftListBumpTimer = setTimeout(() => {
    draftListBumpTimer = null;
    bump();
  }, 1000);
}

export async function syncComposeToRemote(
  sessionId: string,
  compose: SessionCompose,
): Promise<void> {
  try {
    const { remoteSyncSessionCompose } = await import("@/commands/remote");
    const { useChatModelStore } = await import("@/stores/chatModel");
    const chatModelStore = useChatModelStore();
    const match = chatModelStore.models.find(
      (model) =>
        model.id === compose.chatModel &&
        (!compose.chatModelProvider || model.provider === compose.chatModelProvider),
    );
    await remoteSyncSessionCompose(sessionId, {
      chatMode: compose.chatMode,
      toolApprovalMode: compose.toolApprovalMode,
      chatModel: compose.chatModel,
      chatModelProvider: compose.chatModelProvider,
      chatModelLabel: match?.displayName ?? match?.id ?? null,
    });
  } catch {
    // Gateway may be stopped — compose still lives in Pinia.
  }
}

export function loadRejectedPlanFingerprints(): Record<string, string> {
  try {
    const raw = localStorage.getItem(REJECTED_PLAN_STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    if (!parsed || typeof parsed !== "object") return {};
    const entries: Record<string, string> = {};
    for (const [sessionId, value] of Object.entries(parsed)) {
      if (typeof value === "string" && value.trim()) {
        entries[sessionId] = value;
      }
    }
    return entries;
  } catch {
    return {};
  }
}

export function persistRejectedPlanFingerprints(entries: Record<string, string>): void {
  try {
    localStorage.setItem(REJECTED_PLAN_STORAGE_KEY, JSON.stringify(entries));
  } catch {
    // Storage unavailable — keep in-memory state only.
  }
}
