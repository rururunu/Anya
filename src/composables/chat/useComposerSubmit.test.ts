import { describe, expect, it, vi } from "vitest";
import { ref } from "vue";
import { useComposerSubmit } from "./useComposerSubmit";
import type { Workspace } from "@/types/chat";

function createMockOptions(overrides: Partial<Parameters<typeof useComposerSubmit>[0]> = {}) {
  const overlayWorkspaceOverride = ref<Workspace | null>(null);
  const currentWorkspace = ref<Workspace | null>(null);
  const appearance = ref<"overlay" | "workbench" | "detached">("overlay");
  const overlayContextRoot = ref("");
  const knownWorkspaces: Workspace[] = [
    { id: "ws-1", name: "Workspace 1", root: "C:/Projects/App1" },
    { id: "ws-2", name: "Workspace 2", root: "C:/Projects/App2" },
  ];

  const defaultOptions: Parameters<typeof useComposerSubmit>[0] = {
    message: ref(""),
    composerRef: ref(null),
    composerUndo: { push: vi.fn() },
    captureComposerSnapshot: vi.fn(),
    clearComposerSegments: vi.fn(),
    serializeComposerSegments: () => "test message",
    persistDraft: vi.fn(),
    clearAttachedFiles: vi.fn(),
    clearAttachedImages: vi.fn(),
    clearMentionSuppression: vi.fn(),
    attachedFilesMessagePrefix: () => "",
    formatAttachedImagesForMessage: () => "",
    selectedIndex: ref(0),
    overlayWorkspaceOverride,
    currentWorkspace,
    appearance: () => appearance.value,
    capturedContext: () => null,
    overlayContextWorkspaceRoot: () => overlayContextRoot.value,
    matchKnownWorkspace: (root: string) =>
      knownWorkspaces.find((w) => w.root.toLowerCase() === root.toLowerCase()) ?? null,
    emitSubmit: vi.fn(),
    emitLayoutChange: vi.fn(),
    resetLayoutTracking: vi.fn(),
    resizeWorkbenchInput: vi.fn(),
    resizeComposerInput: vi.fn(),
    focusInput: vi.fn(),
    closeModelPicker: vi.fn(),
    closeApprovalMenu: vi.fn(),
    closeChatModeMenu: vi.fn(),
    closeThinkingTierMenu: vi.fn(),
    closeImageGenPicker: vi.fn(),
    workspacePickerOpen: ref(false),
    workspaceQuickSelectOnly: ref(false),
    attachPanelOpen: ref(false),
    resetWorkspaceFilesCache: vi.fn(),
  };

  return {
    options: { ...defaultOptions, ...overrides },
    overlayWorkspaceOverride,
    currentWorkspace,
    appearance,
    overlayContextRoot,
  };
}

describe("useComposerSubmit", () => {
  describe("resolveSendWorkspaceOptions", () => {
    it("returns quickAsk: true in overlay mode when no override or context root matches, ignoring stale currentWorkspace", () => {
      const { options, currentWorkspace } = createMockOptions();
      // Simulate stale currentWorkspace from previous conversation
      currentWorkspace.value = { id: "ws-stale", name: "Old Workspace", root: "C:/Projects/Old" };

      const { resolveSendWorkspaceOptions } = useComposerSubmit(options);
      const result = resolveSendWorkspaceOptions();

      expect(result).toEqual({ quickAsk: true });
    });

    it("returns explicit override workspace in overlay mode when user selected one", () => {
      const { options, overlayWorkspaceOverride } = createMockOptions();
      overlayWorkspaceOverride.value = {
        id: "ws-override",
        name: "Picked",
        root: "C:/Projects/Picked",
      };

      const { resolveSendWorkspaceOptions } = useComposerSubmit(options);
      const result = resolveSendWorkspaceOptions();

      expect(result).toEqual({ workspaceId: "ws-override", quickAsk: false });
    });

    it("returns matched workspace in overlay mode when IDE context matches a known folder", () => {
      const { options, overlayContextRoot } = createMockOptions();
      overlayContextRoot.value = "C:/Projects/App1";

      const { resolveSendWorkspaceOptions } = useComposerSubmit(options);
      const result = resolveSendWorkspaceOptions();

      expect(result).toEqual({ workspaceId: "ws-1", quickAsk: false });
    });

    it("uses currentWorkspace in workbench mode", () => {
      const { options, appearance, currentWorkspace } = createMockOptions();
      appearance.value = "workbench";
      currentWorkspace.value = { id: "ws-wb", name: "Workbench WS", root: "C:/Projects/WB" };

      const { resolveSendWorkspaceOptions } = useComposerSubmit(options);
      const result = resolveSendWorkspaceOptions();

      expect(result).toEqual({ workspaceId: "ws-wb", quickAsk: false });
    });
  });

  describe("reset", () => {
    it("clears overlayWorkspaceOverride and currentWorkspace in overlay mode", () => {
      const { options, overlayWorkspaceOverride, currentWorkspace } = createMockOptions();
      overlayWorkspaceOverride.value = { id: "ws-1", name: "WS 1", root: "C:/1" };
      currentWorkspace.value = { id: "ws-2", name: "WS 2", root: "C:/2" };

      const { reset } = useComposerSubmit(options);
      reset();

      expect(overlayWorkspaceOverride.value).toBeNull();
      expect(currentWorkspace.value).toBeNull();
    });
  });
});
