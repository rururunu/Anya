import { computed, onMounted, onUnmounted, ref, watch, type CSSProperties, type Ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import {
  NAVIGATION_RESIZE_HANDLE_WIDTH,
  readStoredNavigationSidebarWidth,
} from "@/composables/useNavigationSidebarResize";
import { remoteGatewayStatus, type GatewayStatus } from "@/commands/remote";
import type { Workspace } from "@/commands/workspace";

export interface UseNavigationSidebarOptions {
  workspaces: Ref<Workspace[]>;
  workspaceMenuId: Ref<string>;
  toggleWorkspaceMenu: (id: string) => void;
}

/**
 * Left navigation sidebar open state, shell width style, workspace menu, and remote gateway status.
 */
export function useNavigationSidebar(options: UseNavigationSidebarOptions) {
  const navigationOpen = ref(true);
  const navigationWidth = ref(readStoredNavigationSidebarWidth());

  const navigationShellStyle = computed(() => ({
    "--nav-shell-width": navigationOpen.value
      ? `${navigationWidth.value + NAVIGATION_RESIZE_HANDLE_WIDTH}px`
      : "0px",
  }));

  const workspaceMenuStyle = ref<CSSProperties>({});
  const activeWorkspaceMenu = computed(
    () =>
      options.workspaces.value.find(
        (workspace) => workspace.id === options.workspaceMenuId.value,
      ) ?? null,
  );

  function syncWorkspaceMenuPosition(anchor: HTMLElement) {
    const rect = anchor.getBoundingClientRect();
    const menuWidth = 188;
    workspaceMenuStyle.value = {
      position: "fixed",
      top: `${rect.bottom + 4}px`,
      left: `${Math.min(window.innerWidth - menuWidth - 8, Math.max(8, rect.right - menuWidth))}px`,
      zIndex: 80,
    };
  }

  function onWorkspaceMenuToggle(id: string, event: Event) {
    const opening = options.workspaceMenuId.value !== id;
    if (opening && event.currentTarget instanceof HTMLElement) {
      syncWorkspaceMenuPosition(event.currentTarget);
      options.toggleWorkspaceMenu(id);
      return;
    }
    options.workspaceMenuId.value = "";
  }

  function closeWorkspaceMenu() {
    options.workspaceMenuId.value = "";
  }

  let workspaceMenuScrollTarget: HTMLElement | null = null;

  function onWorkspaceMenuScroll() {
    closeWorkspaceMenu();
  }

  watch(options.workspaceMenuId, (id) => {
    workspaceMenuScrollTarget?.removeEventListener("scroll", onWorkspaceMenuScroll);
    workspaceMenuScrollTarget = null;
    if (!id) return;
    workspaceMenuScrollTarget = document.querySelector<HTMLElement>(".session-list");
    workspaceMenuScrollTarget?.addEventListener("scroll", onWorkspaceMenuScroll, { passive: true });
  });

  const remoteGatewayRunning = ref(false);
  let remoteGatewayUnlisten: UnlistenFn | null = null;

  async function refreshRemoteGatewayRunning() {
    try {
      const status = await remoteGatewayStatus();
      remoteGatewayRunning.value = status.running;
    } catch {
      remoteGatewayRunning.value = false;
    }
  }

  onMounted(async () => {
    window.addEventListener("resize", closeWorkspaceMenu);
    await refreshRemoteGatewayRunning();
    try {
      remoteGatewayUnlisten = await listen<GatewayStatus>("remote-gateway-status", (event) => {
        remoteGatewayRunning.value = Boolean(event.payload?.running);
      });
    } catch {
      /* event bridge unavailable in some shells */
    }
  });

  onUnmounted(() => {
    window.removeEventListener("resize", closeWorkspaceMenu);
    workspaceMenuScrollTarget?.removeEventListener("scroll", onWorkspaceMenuScroll);
    workspaceMenuScrollTarget = null;
    remoteGatewayUnlisten?.();
    remoteGatewayUnlisten = null;
  });

  return {
    navigationOpen,
    navigationWidth,
    navigationShellStyle,
    workspaceMenuStyle,
    activeWorkspaceMenu,
    onWorkspaceMenuToggle,
    closeWorkspaceMenu,
    remoteGatewayRunning,
  };
}
