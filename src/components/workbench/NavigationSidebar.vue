<template>
  <Teleport to="body">
    <div
      v-if="sessionDragGhost"
      class="session-drag-ghost"
      :style="{
        transform: `translate(${sessionDragGhost.x - 10}px, ${sessionDragGhost.y - 16}px)`,
      }"
    >
      <MessageSquare :size="14" :stroke-width="1.75" />
      <span>{{ sessionDragGhost.title }}</span>
    </div>
    <div
      v-if="activeWorkspaceMenu"
      class="workspace-menu workspace-menu-floating"
      :style="workspaceMenuStyle"
      @click.stop
    >
      <button type="button" @click.stop="toggleWorkspacePinned(activeWorkspaceMenu)">
        <PinOff v-if="activeWorkspaceMenu.pinned" :size="13" />
        <Pin v-else :size="13" />
        <span>
          {{
            activeWorkspaceMenu.pinned
              ? navigationLabels.unpinWorkspace
              : navigationLabels.pinWorkspace
          }}
        </span>
      </button>
      <button type="button" @click.stop="editWorkspace(activeWorkspaceMenu)">
        <Pencil :size="13" />
        <span>{{ navigationLabels.editWorkspace }}</span>
      </button>
      <button type="button" @click.stop="openWorkspaceFolder(activeWorkspaceMenu)">
        <FolderOpen :size="13" />
        <span>{{ navigationLabels.openFolder }}</span>
      </button>
      <button type="button" @click.stop="openWorkspaceInTerminal(activeWorkspaceMenu)">
        <Terminal :size="13" />
        <span>{{ navigationLabels.openInTerminal }}</span>
      </button>
      <button type="button" @click.stop="archiveWorkspace(activeWorkspaceMenu)">
        <Archive :size="13" />
        <span>{{ navigationLabels.archiveWorkspace }}</span>
      </button>
      <button type="button" class="danger" @click.stop="removeWorkspace(activeWorkspaceMenu)">
        <Trash2 :size="13" />
        <span>{{ navigationLabels.deleteWorkspace }}</span>
      </button>
    </div>
  </Teleport>

  <div
    class="navigation-shell"
    :style="navigationShellStyle"
    :inert="!navigationOpen"
    :aria-hidden="!navigationOpen"
  >
    <aside class="navigation-pane">
      <div class="navigation-brand">
        <div class="navigation-brand-text">
          <strong>{{ appDisplayName }}</strong>
          <span v-if="isDevBuild" class="debug-badge">debug</span>
        </div>
        <button
          type="button"
          class="icon-button navigation-search-button"
          :title="searchLabels.open"
          :aria-label="searchLabels.open"
          @click.stop="openSearchPalette"
        >
          <Search :size="16" />
        </button>
      </div>

      <div class="navigation-menu-items">
        <button
          type="button"
          class="nav-shortcut-button"
          :class="{ active: extensionView === 'plugins' }"
          @click.stop="openExtensionView('plugins')"
        >
          <Puzzle :size="15" :stroke-width="1.75" />
          <span>{{ navigationLabels.plugins }}</span>
        </button>
        <button
          type="button"
          class="nav-shortcut-button"
          :class="{ active: extensionView === 'phone' }"
          @click.stop="openExtensionView('phone')"
        >
          <span class="nav-shortcut-icon">
            <Smartphone :size="15" :stroke-width="1.75" />
            <span
              v-if="remoteGatewayRunning"
              class="nav-status-dot"
              :title="navigationLabels.connectPhone"
              aria-hidden="true"
            />
          </span>
          <span>{{ navigationLabels.connectPhone }}</span>
        </button>
      </div>

      <nav
        class="session-list peek-scrollbar"
        :class="{ 'is-dragging': Boolean(draggedWorkspaceId || draggedSessionId) }"
        :aria-label="labels.conversations"
        @click.stop
      >
        <section
          v-for="workspaceSection in workspaceNavigationSections"
          :key="workspaceSection.id"
          class="navigation-section"
        >
          <header class="navigation-section-header">
            <button
              type="button"
              class="navigation-section-toggle"
              @click="toggleNavigationSection(workspaceSection.id)"
            >
              <ChevronRight
                :size="13"
                :class="{ expanded: !collapsedNavigationSections.has(workspaceSection.id) }"
              />
              <span>{{ workspaceSection.label }}</span>
              <small>{{ workspaceSection.items.length }}</small>
            </button>
            <button
              v-if="workspaceSection.id === 'workspaces'"
              type="button"
              class="section-action"
              :title="navigationLabels.addWorkspace"
              @click="addWorkspace"
            >
              <Plus :size="14" />
            </button>
          </header>

          <NavCollapse
            :collapsed="collapsedNavigationSections.has(workspaceSection.id)"
            class="navigation-section-body"
          >
            <section
              v-for="workspace in workspaceSection.items"
              :key="workspace.id"
              class="workspace-group"
              :data-workspace-id="workspace.id"
              :class="{
                dragging: draggedWorkspaceId === workspace.id,
                'workspace-menu-open': workspaceMenuId === workspace.id,
                'drop-before':
                  dragOverWorkspaceId === workspace.id && workspaceDropPosition === 'before',
                'drop-after':
                  dragOverWorkspaceId === workspace.id && workspaceDropPosition === 'after',
              }"
            >
              <div
                class="workspace-row"
                :class="{ 'session-drop-target': sessionDropWorkspaceId === workspace.id }"
                role="button"
                tabindex="0"
                :aria-expanded="!collapsedWorkspaceIds.has(workspace.id)"
                :aria-label="
                  collapsedWorkspaceIds.has(workspace.id)
                    ? navigationLabels.expandWorkspace
                    : navigationLabels.collapseWorkspace
                "
                @click="handleWorkspaceClick(workspace)"
                @keydown.enter.self.prevent="handleWorkspaceClick(workspace)"
                @keydown.space.self.prevent="handleWorkspaceClick(workspace)"
                @pointerdown="startWorkspacePointerDrag($event, workspace)"
                @dragstart.prevent
              >
                <span class="workspace-collapse" aria-hidden="true" />
                <span class="workspace-path-tip">
                  <TooltipProvider :delay-duration="280">
                    <Tooltip :disabled="Boolean(draggedWorkspaceId || draggedSessionId)">
                      <TooltipTrigger as-child>
                        <span class="workspace-group-header">
                          <Folder v-if="collapsedWorkspaceIds.has(workspace.id)" :size="14" />
                          <FolderOpen v-else :size="14" />
                          <span>{{ workspace.name }}</span>
                        </span>
                      </TooltipTrigger>
                      <TooltipContent
                        side="right"
                        :side-offset="8"
                        class="max-w-[320px] break-all font-mono text-[11px] leading-snug font-medium whitespace-normal"
                      >
                        {{ workspace.root }}
                      </TooltipContent>
                    </Tooltip>
                  </TooltipProvider>
                </span>
                <div class="workspace-actions">
                  <button
                    type="button"
                    :title="navigationLabels.more"
                    @click.stop="onWorkspaceMenuToggle(workspace.id, $event)"
                  >
                    <Ellipsis :size="14" />
                  </button>
                  <button
                    type="button"
                    :title="navigationLabels.newWorkspaceChat"
                    @click.stop="createWorkspaceConversation(workspace)"
                  >
                    <SquarePen :size="13" />
                  </button>
                </div>
              </div>
              <NavCollapse :collapsed="collapsedWorkspaceIds.has(workspace.id)">
                <WorkbenchSessionList
                  :sessions="sessionsForWorkspace(workspace.id)"
                  :all-sessions="sessionsWithLiveTokens"
                  :active-session-id="activeSessionId"
                  :language="language"
                  :untitled-label="labels.untitled"
                  :archive-label="labels.archiveConversation"
                  :archived-label="labels.conversationArchived"
                  :rename-label="tr(language, 'session.renameTitle')"
                  :regenerate-title-label="tr(language, 'session.regenerateTitle')"
                  :delete-label="labels.deleteConversation"
                  :generating-title-label="tr(language, 'session.generatingTitle')"
                  :session-menu-label="tr(language, 'session.sessionMenu')"
                  :archive-visual-state="archiveVisualBySessionId"
                  :dismiss-subagent-label="tr(language, 'subagent.hideFromSidebar')"
                  :running-session-ids="Array.from(runningSessionIds)"
                  :title-generating-session-ids="Array.from(titleGeneratingSessionIds)"
                  :attention-session-ids="Array.from(attentionSessionIds)"
                  :unread-session-ids="unreadSessionIds"
                  :draft-session-ids="Array.from(draftSessionIds)"
                  :dragged-session-id="draggedSessionId"
                  variant="workspace"
                  @select="(sessionId) => selectConversation(sessionId)"
                  @archive="archiveConversation"
                  @rename="(session) => renameConversation(session)"
                  @regenerate-title="regenerateConversationTitle"
                  @delete="removeConversation"
                  @dismiss="dismissSubagentSession"
                  @session-pointer-down="startSessionPointerDrag"
                />
              </NavCollapse>
            </section>
          </NavCollapse>
        </section>

        <section class="navigation-section quick-ask-section">
          <header class="navigation-section-header">
            <button
              type="button"
              class="navigation-section-toggle"
              @click="toggleNavigationSection('quick')"
            >
              <ChevronRight
                :size="13"
                :class="{ expanded: !collapsedNavigationSections.has('quick') }"
              />
              <span>{{ navigationLabels.quickAsk }}</span>
              <small>{{ quickAskSessions.length }}</small>
            </button>
            <button
              type="button"
              class="section-action"
              :title="navigationLabels.newQuickAsk"
              @click="createQuickConversation"
            >
              <SquarePen :size="13" />
            </button>
          </header>
          <NavCollapse :collapsed="collapsedNavigationSections.has('quick')">
            <WorkbenchSessionList
              :sessions="quickAskSessions"
              :all-sessions="sessionsWithLiveTokens"
              :active-session-id="activeSessionId"
              :language="language"
              :untitled-label="labels.untitled"
              :archive-label="labels.archiveConversation"
              :archived-label="labels.conversationArchived"
              :rename-label="tr(language, 'session.renameTitle')"
              :regenerate-title-label="tr(language, 'session.regenerateTitle')"
              :delete-label="labels.deleteConversation"
              :generating-title-label="tr(language, 'session.generatingTitle')"
              :session-menu-label="tr(language, 'session.sessionMenu')"
              :archive-visual-state="archiveVisualBySessionId"
              :dismiss-subagent-label="tr(language, 'subagent.hideFromSidebar')"
              :running-session-ids="Array.from(runningSessionIds)"
              :title-generating-session-ids="Array.from(titleGeneratingSessionIds)"
              :attention-session-ids="Array.from(attentionSessionIds)"
              :unread-session-ids="unreadSessionIds"
              :draft-session-ids="Array.from(draftSessionIds)"
              :dragged-session-id="draggedSessionId"
              variant="quick"
              @select="(sessionId) => selectConversation(sessionId)"
              @archive="archiveConversation"
              @rename="(session) => renameConversation(session)"
              @regenerate-title="regenerateConversationTitle"
              @delete="removeConversation"
              @dismiss="dismissSubagentSession"
              @session-pointer-down="startSessionPointerDrag"
            />
          </NavCollapse>
        </section>
      </nav>
    </aside>
    <div
      v-if="navigationOpen"
      class="navigation-resize-handle"
      :class="{ active: navigationResizing }"
      role="separator"
      aria-orientation="vertical"
      :aria-label="tr(language, 'resizeNavigationSidebar')"
      :title="tr(language, 'resizeNavigationSidebar')"
      :aria-valuemin="navigationMinWidth"
      :aria-valuemax="navigationMaxWidth"
      :aria-valuenow="Math.round(navigationWidth)"
      tabindex="0"
      data-tauri-drag-region="false"
      @pointerdown="startNavigationResize"
      @keydown="handleNavigationResizeKey"
      @dblclick="resetNavigationWidth"
    />
  </div>
</template>

<script setup lang="ts">
import {
  Archive,
  ChevronRight,
  Ellipsis,
  Folder,
  FolderOpen,
  MessageSquare,
  Pencil,
  Pin,
  PinOff,
  Plus,
  Puzzle,
  Search,
  Smartphone,
  SquarePen,
  Terminal,
  Trash2,
} from "@lucide/vue";
import type { CSSProperties } from "vue";

import WorkbenchSessionList from "@/components/workbench/WorkbenchSessionList.vue";
import NavCollapse from "@/components/workbench/NavCollapse.vue";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import type { WorkbenchExtensionView } from "@/composables/workbench/useWorkbenchNavigation";
import type { ArchiveVisualState } from "@/stores/chatSessions";
import { tr } from "@/services/i18n";
import type { Workspace } from "@/commands/workspace";
import type { ChatSessionSummary } from "@/types/chat";
import type { AppLanguage } from "@/types/setting";
import type { WorkbenchLabels } from "@/composables/workbench/useWorkbenchLabels";

defineProps<{
  appDisplayName: string;
  isDevBuild: boolean;
  language: AppLanguage;
  labels: WorkbenchLabels["labels"]["value"];
  navigationLabels: WorkbenchLabels["navigationLabels"]["value"];
  searchLabels: WorkbenchLabels["searchLabels"]["value"];
  navigationOpen: boolean;
  navigationShellStyle: CSSProperties;
  navigationResizing: boolean;
  navigationWidth: number;
  navigationMinWidth: number;
  navigationMaxWidth: number;
  extensionView: WorkbenchExtensionView | null;
  remoteGatewayRunning: boolean;
  workspaceNavigationSections: Array<{ id: string; label: string; items: Workspace[] }>;
  collapsedNavigationSections: Set<string>;
  collapsedWorkspaceIds: Set<string>;
  draggedWorkspaceId: string;
  draggedSessionId: string;
  dragOverWorkspaceId: string;
  workspaceDropPosition: "before" | "after" | null;
  workspaceMenuId: string;
  sessionDropWorkspaceId: string;
  sessionDragGhost: { x: number; y: number; title: string } | null;
  activeWorkspaceMenu: Workspace | null;
  workspaceMenuStyle: CSSProperties;
  sessionsWithLiveTokens: ChatSessionSummary[];
  quickAskSessions: ChatSessionSummary[];
  activeSessionId: string;
  archiveVisualBySessionId: Record<string, ArchiveVisualState>;
  runningSessionIds: string[] | Set<string>;
  titleGeneratingSessionIds: string[] | Set<string>;
  attentionSessionIds: string[] | Set<string>;
  unreadSessionIds: string[];
  draftSessionIds: string[] | Set<string>;
  sessionsForWorkspace: (workspaceId: string) => ChatSessionSummary[];
  openSearchPalette: () => void;
  openExtensionView: (view: WorkbenchExtensionView) => void;
  toggleNavigationSection: (id: string) => void;
  addWorkspace: () => void;
  handleWorkspaceClick: (workspace: Workspace) => void;
  onWorkspaceMenuToggle: (id: string, event: Event) => void;
  startWorkspacePointerDrag: (event: PointerEvent, workspace: Workspace) => void;
  createWorkspaceConversation: (workspace: Workspace) => void;
  selectConversation: (sessionId: string) => void;
  archiveConversation: (sessionId: string) => void;
  renameConversation: (session: ChatSessionSummary) => void | Promise<void>;
  regenerateConversationTitle: (sessionId: string) => void;
  removeConversation: (sessionId: string) => void | Promise<void>;
  dismissSubagentSession: (sessionId: string) => void;
  startSessionPointerDrag: (event: PointerEvent, session: ChatSessionSummary) => void;
  createQuickConversation: () => void;
  startNavigationResize: (event: PointerEvent) => void;
  handleNavigationResizeKey: (event: KeyboardEvent) => void;
  resetNavigationWidth: () => void;
  toggleWorkspacePinned: (workspace: Workspace) => void;
  editWorkspace: (workspace: Workspace) => void;
  openWorkspaceFolder: (workspace: Workspace) => void;
  openWorkspaceInTerminal: (workspace: Workspace) => void;
  archiveWorkspace: (workspace: Workspace) => void;
  removeWorkspace: (workspace: Workspace) => void;
}>();
</script>

<style scoped>
:global(.workbench.navigation-resizing) .navigation-shell {
  transition: opacity 160ms ease;
}
:global(.workspace-grid.navigation-closed) .navigation-shell {
  width: 0 !important;
  opacity: 0;
  pointer-events: none;
}
:global(.workspace-grid.navigation-closed) .navigation-pane {
  padding-left: 0;
  padding-right: 0;
}

.navigation-shell {
  min-width: 0;
  min-height: 0;
  width: var(--nav-shell-width, 265px);
  max-width: 100%;
  display: flex;
  overflow: hidden;
  transition:
    width 220ms cubic-bezier(0.2, 0.72, 0.25, 1),
    opacity 160ms ease;
}

.navigation-pane {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 0;
  padding: var(--peek-space-2, 8px) var(--peek-space-2, 8px) var(--peek-space-3, 12px);
  background: var(--peek-sidebar);
  overflow: hidden;
}
.navigation-resize-handle {
  position: relative;
  z-index: 4;
  flex: none;
  width: 7px;
  min-width: 7px;
  cursor: col-resize;
  outline: none;
  touch-action: none;
}
.navigation-resize-handle::after {
  content: "";
  position: absolute;
  top: calc(50% - 18px);
  right: 2px;
  width: 3px;
  height: 36px;
  border-radius: 2px;
  background: transparent;
  transition:
    background 100ms ease,
    transform 100ms ease;
}
.navigation-resize-handle:hover::after,
.navigation-resize-handle:focus-visible::after,
.navigation-resize-handle.active::after {
  background: color-mix(in srgb, var(--peek-accent) 68%, var(--peek-border));
  transform: scaleY(1.15);
}
.navigation-brand {
  display: flex;
  align-items: center;
  gap: var(--peek-space-2, 8px);
  min-height: 36px;
  margin: 0 2px var(--peek-space-2, 8px);
  padding: 2px 4px 2px 8px;
  color: var(--peek-text);
}
.navigation-brand-text {
  min-width: 0;
  flex: 1;
  display: flex;
  flex-direction: row;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
}
.navigation-brand strong {
  min-width: 0;
  overflow: hidden;
  font-size: var(--peek-font-lg, 15px);
  font-weight: 650;
  letter-spacing: -0.02em;
  line-height: 1.2;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.navigation-search-button {
  flex: none;
  margin-left: auto;
  color: var(--peek-muted);
}
.navigation-search-button:hover {
  color: var(--peek-text);
}
.debug-badge {
  flex: none;
  display: inline-flex;
  align-items: center;
  height: 14px;
  margin-left: 0;
  padding: 0 4px;
  border: 1px solid color-mix(in srgb, var(--peek-warning) 34%, var(--peek-border));
  border-radius: 999px;
  background: color-mix(in srgb, var(--peek-warning) 14%, transparent);
  color: color-mix(in srgb, var(--peek-warning) 82%, var(--peek-text));
  font-size: 9px;
  font-weight: 600;
  letter-spacing: 0.02em;
  line-height: 1;
  text-transform: lowercase;
}
.nav-shortcut-button {
  height: var(--peek-control-row);
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 0 10px;
  border-radius: var(--peek-radius-md);
  background: transparent;
  font-size: var(--peek-font-sm);
  font-weight: 550;
  transition:
    background-color var(--motion-fast, 110ms) var(--motion-ease-out, ease),
    color var(--motion-fast, 110ms) var(--motion-ease-out, ease),
    border-color var(--motion-fast, 110ms) var(--motion-ease-out, ease),
    box-shadow var(--motion-fast, 110ms) var(--motion-ease-out, ease),
    transform var(--motion-instant, 80ms) var(--motion-ease-out, ease);
}
.nav-shortcut-button:hover {
  background: var(--peek-row-hover);
}
.nav-shortcut-button:active {
  background: var(--peek-press-bg);
}
.navigation-menu-items {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin: 0 2px var(--peek-space-3, 12px);
}
.navigation-menu-items .nav-shortcut-button {
  width: calc(100% - 4px);
  margin: 0 2px;
}
.nav-shortcut-button {
  width: 100%;
  border: 0;
  color: inherit;
  cursor: pointer;
  text-align: left;
}
.nav-shortcut-button.active {
  background: var(--peek-row-active);
  color: var(--peek-text);
  font-weight: 600;
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--peek-accent) 10%, transparent);
}
.nav-shortcut-button > svg {
  flex: none;
  color: var(--peek-muted);
}
.nav-shortcut-icon {
  position: relative;
  display: inline-flex;
  flex: none;
  color: var(--peek-muted);
}
.nav-shortcut-icon > svg {
  display: block;
}
.nav-status-dot {
  position: absolute;
  right: -2px;
  bottom: -1px;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--peek-success);
  box-shadow: 0 0 0 2px var(--peek-bg, #fff);
}
.nav-shortcut-button:hover > svg,
.nav-shortcut-button.active > svg,
.nav-shortcut-button:hover .nav-shortcut-icon,
.nav-shortcut-button.active .nav-shortcut-icon {
  color: var(--peek-text);
}
.session-list {
  flex: 1;
  min-width: 0;
  min-height: 0;
  margin-top: 2px;
  padding-top: var(--peek-space-2, 8px);
  border-top: 1px solid color-mix(in srgb, var(--peek-border) 48%, transparent);
  overflow-x: clip;
  overflow-y: auto;
  user-select: none;
  -webkit-user-select: none;
}
.session-list.is-dragging,
.session-list.is-dragging * {
  cursor: grabbing !important;
  user-select: none !important;
  -webkit-user-select: none !important;
}
.navigation-section {
  min-width: 0;
  max-width: 100%;
  margin: 0 0 var(--peek-space-2, 8px);
}
.navigation-section-header {
  height: calc(var(--peek-control-row) - 2px);
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-radius: var(--peek-radius-sm);
  padding: 0 2px;
}
.navigation-section-toggle {
  min-width: 0;
  height: calc(var(--peek-control-row) - 2px);
  display: flex;
  align-items: center;
  gap: 5px;
  flex: 1;
  padding: 0 6px;
  border: 0;
  border-radius: var(--peek-radius-sm);
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
  font-size: var(--peek-font-sm);
  font-weight: 600;
  letter-spacing: 0;
  text-transform: none;
  text-align: left;
  transition:
    background-color var(--motion-fast, 110ms) ease,
    color var(--motion-fast, 110ms) ease;
}
.navigation-section-toggle:hover {
  background: var(--peek-row-hover);
  color: var(--peek-text);
}
.navigation-section-toggle > svg:first-child {
  flex: none;
  color: var(--peek-faint);
  opacity: 0.9;
  transition: transform 140ms ease;
}
.navigation-section-toggle > svg:first-child.expanded {
  transform: rotate(90deg);
}
.navigation-section-toggle span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.navigation-section-toggle small {
  margin-left: auto;
  padding: 1px 6px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--peek-text) 5%, transparent);
  color: var(--peek-faint);
  font-size: var(--peek-font-xs);
  font-weight: 500;
  font-variant-numeric: tabular-nums;
  line-height: 1.4;
}
.section-action,
.workspace-actions button {
  display: inline-grid;
  place-items: center;
  padding: 0;
  border: 0;
  border-radius: var(--peek-radius-sm);
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
}
.section-action {
  width: var(--peek-control-icon);
  height: var(--peek-control-icon);
}
.section-action:hover,
.workspace-actions button:hover {
  color: var(--peek-text);
  background: var(--peek-hover-bg);
}
.section-action.active {
  color: var(--peek-accent);
  background: color-mix(in srgb, var(--peek-accent) 12%, transparent);
}
.navigation-section-body {
  padding-top: 0;
}
.navigation-section-body :deep(.nav-collapse-inner) {
  padding-top: 3px;
}
.quick-ask-section :deep(.nav-collapse-inner) {
  padding-top: 3px;
}
.workspace-group {
  position: relative;
  margin: 0 0 4px;
  padding: 0 2px;
}
.workspace-group.drop-before::before,
.workspace-group.drop-after::after {
  content: "";
  position: absolute;
  z-index: 4;
  left: 5px;
  right: 5px;
  height: 2px;
  border-radius: 2px;
  background: var(--peek-accent);
  pointer-events: none;
}
.workspace-group.drop-before::before {
  top: -3px;
}
.workspace-group.drop-after::after {
  bottom: -3px;
}
.workspace-row {
  position: relative;
  display: flex;
  align-items: center;
  min-width: 0;
  height: var(--peek-control-row);
  padding-left: 4px;
  border-radius: var(--peek-radius-md);
  cursor: pointer;
  user-select: none;
  -webkit-user-select: none;
  touch-action: none;
  transition:
    background-color var(--motion-fast, 110ms) ease,
    box-shadow var(--motion-fast, 110ms) ease;
}
.workspace-row:hover {
  background: var(--peek-row-hover);
}
.workspace-row[aria-expanded="true"] {
  background: color-mix(in srgb, var(--peek-text) 3.5%, transparent);
}
.workspace-row.session-drop-target,
.workspace-row.session-drop-target:hover {
  background: color-mix(in srgb, var(--peek-accent) 12%, transparent);
  box-shadow: inset 0 0 0 1.5px var(--peek-accent);
}
.workspace-row.session-drop-target .workspace-actions {
  opacity: 0;
  pointer-events: none;
}
.workspace-group.dragging .workspace-row {
  cursor: grabbing;
  background: color-mix(in srgb, var(--peek-accent) 12%, transparent);
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--peek-accent) 24%, transparent);
}
.workspace-group.dragging .workspace-group-header {
  cursor: grabbing;
}
.workspace-collapse {
  display: none;
}
.workspace-path-tip {
  min-width: 0;
  flex: 1;
  display: flex;
}
.workspace-path-tip :deep([data-slot="tooltip"]) {
  min-width: 0;
  flex: 1;
  display: flex;
}
.workspace-group-header {
  min-width: 0;
  height: var(--peek-control-row);
  display: flex;
  align-items: center;
  flex: 1;
  gap: 8px;
  padding: 0 6px 0 2px;
  border: 0;
  border-radius: var(--peek-radius-sm);
  background: transparent;
  color: var(--peek-text);
  cursor: inherit;
  font-size: var(--peek-font-sm);
  text-align: left;
}
.workspace-group-header > svg {
  flex: none;
  color: color-mix(in srgb, var(--peek-accent) 55%, var(--peek-muted));
}
.workspace-group-header span {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
  user-select: none;
  -webkit-user-select: none;
}
.workspace-actions {
  display: flex;
  align-items: center;
  gap: 0;
  padding-right: 2px;
  opacity: 0;
  transition: opacity var(--motion-fast, 110ms) ease;
}
.workspace-row:hover .workspace-actions,
.workspace-row:focus-within .workspace-actions,
.workspace-group.workspace-menu-open .workspace-actions {
  opacity: 1;
}
.workspace-actions button {
  width: var(--peek-control-icon, 30px);
  height: calc(var(--peek-control-icon, 30px) - 2px);
  border-radius: var(--peek-radius-sm);
  transition:
    background-color var(--motion-fast, 110ms) ease,
    color var(--motion-fast, 110ms) ease;
}
.workspace-actions button:hover {
  color: var(--peek-text);
  background: var(--peek-hover-bg);
}
.workspace-menu {
  min-width: 188px;
  padding: 5px;
  border: 1px solid color-mix(in srgb, var(--peek-border) 75%, transparent);
  border-radius: var(--peek-radius-md);
  background: var(--peek-list-bg);
  box-shadow: var(--peek-elev-md);
  opacity: 1;
}
.workspace-menu-floating {
  position: fixed;
  z-index: 80;
}
.workspace-menu button {
  width: 100%;
  height: var(--peek-control-icon);
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 8px;
  border: 0;
  border-radius: var(--peek-radius-sm);
  background: transparent;
  color: var(--peek-text);
  cursor: pointer;
  font-size: var(--peek-font-sm);
  text-align: left;
}
.workspace-menu button:hover {
  background: var(--peek-hover-bg);
}
.workspace-menu button.danger {
  color: var(--peek-danger);
}

.session-drag-ghost {
  position: fixed;
  top: 0;
  left: 0;
  z-index: 40;
  display: flex;
  align-items: center;
  gap: 7px;
  max-width: 220px;
  height: 28px;
  padding: 0 10px 0 8px;
  border: 1px solid color-mix(in srgb, var(--peek-text) 8%, transparent);
  border-radius: 8px;
  background: var(--peek-surface, #fff);
  color: var(--peek-text);
  box-shadow: 0 6px 18px color-mix(in srgb, #000 16%, transparent);
  pointer-events: none;
  user-select: none;
  -webkit-user-select: none;
}
.session-drag-ghost > svg {
  flex: none;
  opacity: 0.72;
}
.session-drag-ghost span {
  min-width: 0;
  overflow: hidden;
  font-size: 12px;
  font-weight: 550;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
