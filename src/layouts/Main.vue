<template>
  <main
    class="workbench"
    :class="{
      'is-glass': settingStore.chromeFrostedGlass,
      'navigation-closed': !navigationOpen,
      'navigation-resizing': navigationResizing,
      'is-settings': settingsOpen,
      'is-maximized': isMaximized,
    }"
    :data-theme="builtInTheme"
    @click="workspaceMenuId = ''"
  >
    <div class="glass-chrome" aria-hidden="true" />
    <AppConfirmDialog ref="confirmDialogRef" />
    <EditWorkspaceDialog ref="editWorkspaceDialogRef" />
    <RenameSessionDialog ref="renameSessionDialogRef" />
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
    <Transition name="workbench-ready">
      <WorkbenchLoading v-if="initializing" />
    </Transition>
    <header class="titlebar" :class="{ 'titlebar-compact': !settingsOpen }" data-tauri-drag-region>
      <template v-if="settingsOpen">
        <div class="titlebar-leading" data-tauri-drag-region="false">
          <button
            type="button"
            class="titlebar-back"
            :title="labels.backToChat"
            :aria-label="labels.backToChat"
            @click="closeSettings"
          >
            <ArrowLeft :size="15" />
            <span>{{ labels.backToChat }}</span>
          </button>
        </div>

        <div class="titlebar-context" data-tauri-drag-region>
          <span>{{ activeTitle }}</span>
        </div>
      </template>
      <template v-else>
        <div class="titlebar-leading" data-tauri-drag-region="false">
          <button
            type="button"
            class="icon-button nav-toggle"
            :title="labels.toggleNavigation"
            :aria-label="labels.toggleNavigation"
            :aria-pressed="navigationOpen"
            @click="navigationOpen = !navigationOpen"
          >
            <PanelLeft :size="15" />
          </button>
        </div>
        <div class="titlebar-drag" data-tauri-drag-region aria-hidden="true" />
      </template>

      <div class="titlebar-trailing" data-tauri-drag-region="false">
        <nav class="view-actions" :aria-label="labels.views">
          <button
            v-if="updaterStore.updateAvailable"
            type="button"
            class="icon-button update-button"
            :class="{ busy: updaterStore.isBusy }"
            :title="updaterCopy.titlebarAction"
            :aria-label="updaterCopy.titlebarAction"
            :disabled="updaterStore.isBusy"
            @click="promptInstallUpdate"
          >
            <ArrowUpCircle :size="15" />
            <span class="status-dot update-dot" />
          </button>
          <button
            type="button"
            class="icon-button"
            :class="{ active: settingsOpen }"
            :title="labels.settings"
            :aria-label="labels.settings"
            @click="toggleSettings"
          >
            <Settings :size="15" />
          </button>
        </nav>

        <div class="window-actions">
          <button
            type="button"
            class="window-button"
            :title="labels.minimize"
            @click="minimizeWindow"
          >
            <Minus :size="14" />
          </button>
          <button
            type="button"
            class="window-button"
            :title="tr(settingStore.language, isMaximized ? 'restoreWindow' : 'maximizeWindow')"
            :aria-label="
              tr(settingStore.language, isMaximized ? 'restoreWindow' : 'maximizeWindow')
            "
            @click="toggleMaximizeWindow"
          >
            <span v-if="isMaximized" class="windows-caption-icon" aria-hidden="true">&#xE923;</span>
            <span v-else class="windows-caption-icon" aria-hidden="true">&#xE922;</span>
          </button>
          <button
            type="button"
            class="window-button close"
            :title="labels.close"
            @click="hideWindow"
          >
            <X :size="14" />
          </button>
        </div>
      </div>
    </header>

    <div class="main-stage">
      <div
        class="workspace-grid"
        :class="{
          'navigation-closed': !navigationOpen,
          'review-open': reviewOpen,
          'is-covered': settingsOpen,
        }"
        :inert="settingsOpen"
        :aria-hidden="settingsOpen"
      >
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
                    @click="onToggleNavigationSection(workspaceSection.id)"
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
                      @click="onWorkspaceHeaderClick(workspace)"
                      @keydown.enter.self.prevent="onWorkspaceHeaderClick(workspace)"
                      @keydown.space.self.prevent="onWorkspaceHeaderClick(workspace)"
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
                          @click.stop="handleCreateWorkspaceConversation(workspace)"
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
                        :language="settingStore.language"
                        :untitled-label="labels.untitled"
                        :archive-label="labels.archiveConversation"
                        :archived-label="labels.conversationArchived"
                        :rename-label="tr(settingStore.language, 'session.renameTitle')"
                        :regenerate-title-label="
                          tr(settingStore.language, 'session.regenerateTitle')
                        "
                        :generating-title-label="
                          tr(settingStore.language, 'session.generatingTitle')
                        "
                        :delete-label="labels.deleteConversation"
                        :session-menu-label="tr(settingStore.language, 'session.sessionMenu')"
                        :archive-visual-state="archiveVisualBySessionId"
                        :dismiss-subagent-label="
                          tr(settingStore.language, 'subagent.hideFromSidebar')
                        "
                        :running-session-ids="runningSessionIds"
                        :title-generating-session-ids="titleGeneratingSessionIds"
                        :attention-session-ids="attentionSessionIds"
                        :unread-session-ids="unreadSessionIdList"
                        :draft-session-ids="draftSessionIds"
                        :dragged-session-id="draggedSessionId"
                        variant="workspace"
                        @select="handleSelectConversation"
                        @archive="archiveConversation"
                        @rename="renameConversation"
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
                    @click="onToggleNavigationSection('quick')"
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
                    @click="handleCreateQuickConversation"
                  >
                    <SquarePen :size="13" />
                  </button>
                </header>
                <NavCollapse :collapsed="collapsedNavigationSections.has('quick')">
                  <WorkbenchSessionList
                    :sessions="quickAskSessions"
                    :all-sessions="sessionsWithLiveTokens"
                    :active-session-id="activeSessionId"
                    :language="settingStore.language"
                    :untitled-label="labels.untitled"
                    :archive-label="labels.archiveConversation"
                    :archived-label="labels.conversationArchived"
                    :rename-label="tr(settingStore.language, 'session.renameTitle')"
                    :regenerate-title-label="tr(settingStore.language, 'session.regenerateTitle')"
                    :delete-label="labels.deleteConversation"
                    :generating-title-label="tr(settingStore.language, 'session.generatingTitle')"
                    :session-menu-label="tr(settingStore.language, 'session.sessionMenu')"
                    :archive-visual-state="archiveVisualBySessionId"
                    :dismiss-subagent-label="tr(settingStore.language, 'subagent.hideFromSidebar')"
                    :running-session-ids="runningSessionIds"
                    :title-generating-session-ids="titleGeneratingSessionIds"
                    :attention-session-ids="attentionSessionIds"
                    :unread-session-ids="unreadSessionIdList"
                    :draft-session-ids="draftSessionIds"
                    :dragged-session-id="draggedSessionId"
                    variant="quick"
                    @select="handleSelectConversation"
                    @archive="archiveConversation"
                    @rename="renameConversation"
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
            :aria-label="tr(settingStore.language, 'resizeNavigationSidebar')"
            :title="tr(settingStore.language, 'resizeNavigationSidebar')"
            :aria-valuemin="NAVIGATION_SIDEBAR_MIN_WIDTH"
            :aria-valuemax="NAVIGATION_SIDEBAR_MAX_WIDTH"
            :aria-valuenow="Math.round(navigationWidth)"
            tabindex="0"
            data-tauri-drag-region="false"
            @pointerdown="startNavigationResize"
            @keydown="handleNavigationResizeKey"
            @dblclick="resetNavigationWidth"
          />
        </div>

        <WorkbenchSearchPalette
          v-model:open="searchPaletteOpen"
          :sessions="sessionsWithLiveTokens"
          :workspaces="workspaces"
          :language="settingStore.language"
          @select-session="handleSelectConversation"
          @new-chat="handleCreateQuickConversation"
        />

        <section
          class="conversation-pane peek-pane"
          :class="{
            'empty-conversation': !extensionView && !hasConversationMessages,
            'extension-open': Boolean(extensionView),
          }"
          :style="composerOverlayStyle"
        >
          <header
            v-if="showConversationHeader"
            class="conversation-header"
            data-tauri-drag-region="false"
          >
            <span class="conversation-header-spacer" aria-hidden="true" />
            <p class="conversation-title" :title="activeTitle">{{ activeTitle }}</p>
            <button
              v-if="!reviewOpen"
              type="button"
              class="icon-button review-toggle"
              :class="{ active: reviewOpen }"
              :title="labels.views"
              :aria-label="labels.views"
              @click="toggleReviewSidebar"
            >
              <PanelRight :size="15" />
              <span v-if="runningSubagentCount" class="status-dot" />
            </button>
          </header>

          <div v-if="extensionView" class="extension-pane">
            <div class="extension-scroll peek-scrollbar">
              <div class="extension-panel">
                <PluginsPanel v-if="extensionView === 'plugins'" />
                <ConnectPhonePanel v-else-if="extensionView === 'phone'" />
              </div>
            </div>
          </div>
          <template v-else>
            <div v-if="contextNotice" class="context-notice" role="status">
              <CircleAlert :size="14" :stroke-width="1.8" aria-hidden="true" />
              <span>{{ contextNotice }}</span>
            </div>
            <Transition name="empty-hero">
              <div
                v-if="!viewingSubagent && !hasConversationMessages"
                class="empty-conversation-hero"
              >
                <div
                  class="empty-conversation-brand"
                  data-onboarding-logo-target
                  aria-hidden="true"
                >
                  <MascotFace
                    interactive
                    :tool="heroMascotTool"
                    :look-at="composerFocused ? composerWrapRef : null"
                  />
                </div>
                <p class="empty-conversation-prompt">
                  {{ emptyConversationPrompt }}
                </p>
              </div>
            </Transition>
            <AppErrorBoundary v-if="viewingSubagent" compact class="workbench-messages">
              <SubagentConversationPanel
                class="workbench-messages"
                :session-id="activeSessionId"
                :parent-messages="parentMessages"
                :language="settingStore.language"
                :show-reasoning="settingStore.showReasoning"
                :display-mode="settingStore.agentWorkDisplay"
              />
            </AppErrorBoundary>
            <AppErrorBoundary v-else compact class="workbench-messages">
              <MessageList
                class="workbench-messages"
                :messages="conversationMessages"
                :session-id="activeSessionId"
                :checkpoints="checkpoints"
                @rewound="handleRewound"
                @branch="handleBranchMessage"
                @review-changes="openReview('diff')"
                @review-file="openReviewFile"
                @inspect-subagent="openAgentReview"
                @preview-image="previewImage"
                @edit-from-image="handleEditFromImage"
              />
            </AppErrorBoundary>

            <div
              v-if="!viewingSubagent"
              ref="composerWrapRef"
              class="composer-wrap"
              :class="{ 'has-interaction-picker': Boolean(activePendingInteraction) }"
              @focusin="composerFocused = true"
              @focusout="composerFocused = false"
            >
              <div
                v-if="stagedMessages.length"
                class="staged-wrap peek-scrollbar"
                data-tauri-drag-region="false"
              >
                <div class="staged-list">
                  <div
                    v-for="(message, index) in stagedMessages"
                    :key="`${index}-${message}`"
                    class="staged-item"
                  >
                    <span class="staged-item-text">{{ message }}</span>
                    <span class="staged-item-actions">
                      <button
                        type="button"
                        class="staged-btn staged-btn-guide"
                        :title="labels.guideOneHint"
                        @click="guideStaged(index)"
                      >
                        <CornerDownLeft :size="13" />
                      </button>
                      <button
                        type="button"
                        class="staged-btn"
                        :title="labels.editStaged"
                        @click="startStagedEdit(index)"
                      >
                        <Pencil :size="13" />
                      </button>
                      <button
                        type="button"
                        class="staged-btn staged-btn-danger"
                        :title="labels.removeStaged"
                        @click="removeStaged(index)"
                      >
                        <Trash2 :size="13" />
                      </button>
                    </span>
                  </div>
                </div>
              </div>
              <ChatInputBar
                ref="inputRef"
                :sending="sending"
                :close-on-escape="false"
                appearance="workbench"
                overlay-pickers
                :context-ready="true"
                :session-id="activeSessionId"
                :ask-user="askUserSession"
                :path-permission="pathPermissionSession"
                :tool-approval="toolApprovalSession"
                @submit="submitMessage"
                @pause="pauseResponse"
                @ask-user-complete="completeAskUser"
                @path-permission-complete="completePathPermission"
                @tool-approval-complete="completeToolApproval"
                @preview-image="previewImage"
                @show-context="handleShowContext"
                @open-history="openSearchPalette"
                @close="handleCreateQuickConversation"
              />
            </div>
          </template>
        </section>

        <Transition name="review-panel">
          <div
            v-if="reviewOpen"
            class="review-shell"
            :style="{ '--review-pane-width': `${reviewWidth + REVIEW_RESIZE_HANDLE_WIDTH}px` }"
          >
            <div
              class="review-resize-handle"
              :class="{ active: reviewResizing }"
              role="separator"
              aria-orientation="vertical"
              :aria-label="tr(settingStore.language, 'resizeCodeChanges')"
              :title="tr(settingStore.language, 'resizeCodeChanges')"
              :aria-valuemin="REVIEW_SIDEBAR_MIN_WIDTH"
              :aria-valuemax="REVIEW_SIDEBAR_MAX_WIDTH"
              :aria-valuenow="Math.round(reviewWidth)"
              tabindex="0"
              data-tauri-drag-region="false"
              @pointerdown="startReviewResize"
              @keydown="handleReviewResizeKey"
              @dblclick="resetReviewWidth"
            />
            <aside class="review-pane">
              <header class="review-header">
                <div class="review-tabs" role="tablist" :aria-label="labels.views">
                  <button
                    v-for="view in reviewViews"
                    :key="view.id"
                    type="button"
                    :class="{ active: reviewView === view.id }"
                    @click="reviewView = view.id"
                  >
                    <component :is="view.icon" :size="14" />
                    <span>{{ view.label }}</span>
                  </button>
                </div>
                <button
                  type="button"
                  class="small-icon-button"
                  :title="labels.closePanel"
                  @click="reviewOpen = false"
                >
                  <PanelRightClose :size="15" />
                </button>
              </header>
              <CodeDiffSidebar
                v-show="reviewView === 'diff'"
                embedded
                :messages="messages"
                :width="reviewWidth"
                :focus-path="reviewFocusPath"
                :focus-at="reviewFocusAt"
              />
              <AgentDebugPanel v-show="reviewView === 'runtime'" embedded />
            </aside>
          </div>
        </Transition>
      </div>

      <div v-if="settingsOpen" class="embedded-settings">
        <SettingsPage embedded :category="settingsCategory" />
      </div>
    </div>

    <WelcomeOnboarding v-if="showOnboarding" @completed="showOnboarding = false" />

    <ImageLightbox
      :open="imageLightboxOpen"
      :sources="openedImageSources"
      :selected-source="selectedImageSource"
      @close="closeImageLightbox"
      @select="selectedImageSource = $event"
      @edit-from-image="handleEditFromImage"
    />

    <Transition name="shortcut-help">
      <div
        v-if="shortcutHelpOpen"
        class="shortcut-help-root"
        data-tauri-drag-region="false"
        @mousedown.self="shortcutHelpOpen = false"
      >
        <div
          class="shortcut-help-card"
          role="dialog"
          aria-modal="true"
          :aria-label="shortcutLabels.helpTitle"
        >
          <header>
            <strong>{{ shortcutLabels.helpTitle }}</strong>
            <button
              type="button"
              class="small-icon-button"
              :aria-label="labels.close"
              @click="shortcutHelpOpen = false"
            >
              <X :size="14" />
            </button>
          </header>
          <ul>
            <li v-for="item in shortcutHelpItems" :key="item.keys">
              <kbd>{{ item.keys }}</kbd>
              <span>{{ item.label }}</span>
            </li>
          </ul>
        </div>
      </div>
    </Transition>
  </main>
</template>

<script setup lang="ts">
import {
  computed,
  defineAsyncComponent,
  onMounted,
  onUnmounted,
  ref,
  watch,
  type CSSProperties,
} from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  ArrowLeft,
  ArrowUpCircle,
  Archive,
  CircleAlert,
  ChevronRight,
  CornerDownLeft,
  Ellipsis,
  Folder,
  FolderOpen,
  Minus,
  MessageSquare,
  PanelLeft,
  Pencil,
  PanelRight,
  PanelRightClose,
  Pin,
  PinOff,
  Plus,
  Puzzle,
  Search,
  Settings,
  Smartphone,
  SquarePen,
  Terminal,
  Trash2,
  X,
} from "@lucide/vue";
import AppErrorBoundary from "@/components/AppErrorBoundary.vue";
import AgentDebugPanel from "@/components/chat/AgentDebugPanel.vue";
import ChatInputBar from "@/components/chat/ChatInputBar.vue";
import CodeDiffSidebar from "@/components/chat/CodeDiffSidebar.vue";
import ImageLightbox from "@/components/chat/ImageLightbox.vue";
import MessageList from "@/components/chat/MessageList.vue";
import SubagentConversationPanel from "@/components/chat/SubagentConversationPanel.vue";
import WorkbenchSessionList from "@/components/workbench/WorkbenchSessionList.vue";
import NavCollapse from "@/components/workbench/NavCollapse.vue";
import WorkbenchSearchPalette from "@/components/workbench/WorkbenchSearchPalette.vue";
import WorkbenchLoading from "@/components/workbench/WorkbenchLoading.vue";
import WelcomeOnboarding from "@/components/onboarding/WelcomeOnboarding.vue";
import MascotFace, { type MascotTool } from "@/components/icons/MascotFace.vue";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import EditWorkspaceDialog from "@/components/workspace/EditWorkspaceDialog.vue";
import RenameSessionDialog from "@/components/workbench/RenameSessionDialog.vue";
import {
  useWorkbenchReview,
  REVIEW_RESIZE_HANDLE_WIDTH,
  REVIEW_SIDEBAR_MIN_WIDTH,
  REVIEW_SIDEBAR_MAX_WIDTH,
  NAVIGATION_RESIZE_HANDLE_WIDTH,
  NAVIGATION_SIDEBAR_MIN_WIDTH,
  NAVIGATION_SIDEBAR_MAX_WIDTH,
} from "@/composables/workbench/useWorkbenchReview";
import { useWorkbenchLabels } from "@/composables/workbench/useWorkbenchLabels";
import { useWorkbenchWindow } from "@/composables/workbench/useWorkbenchWindow";
import { useWorkbenchInteractions } from "@/composables/workbench/useWorkbenchInteractions";
import { useWorkbenchSessions } from "@/composables/workbench/useWorkbenchSessions";
import { useWorkbenchWorkspaces } from "@/composables/workbench/useWorkbenchWorkspaces";
import { useWorkbenchHotkeys } from "@/composables/workbench/useWorkbenchHotkeys";
import { useWorkbenchLifecycle } from "@/composables/workbench/useWorkbenchLifecycle";
import { storeToRefs } from "pinia";
import { tr } from "@/services/i18n";
import { formatSessionPreview } from "@/services/chat/sessionPreview";
import { isSubagentSessionId, rootSessionId } from "@/services/chat/subagentSession";
import { useChatStore } from "@/stores/chat";
import { useChatSessionsStore } from "@/stores/chatSessions";
import { useSubagentSessionStore } from "@/stores/subagentSessions";
import { useSettingStore, applyZoom, applyTheme } from "@/stores/setting";
import { normalizeChatMode, type ChatMode } from "@/types/setting";
import { useUpdaterStore } from "@/stores/updater";
import { remoteGatewayStatus, type GatewayStatus } from "@/commands/remote";
import type { Workspace } from "@/commands/workspace";
import type { CapturedContext } from "@/types/chat";

const SettingsPage = defineAsyncComponent(() => import("@/pages/Settings/index.vue"));
const PluginsPanel = defineAsyncComponent(() => import("@/components/workbench/PluginsPanel.vue"));
const ConnectPhonePanel = defineAsyncComponent(
  () => import("@/components/workbench/ConnectPhonePanel.vue"),
);

type ExtensionView = "plugins" | "phone";
const extensionView = ref<ExtensionView | null>(null);
const remoteGatewayRunning = ref(false);
let remoteGatewayUnlisten: UnlistenFn | null = null;

const chatStore = useChatStore();
const sessionsStore = useChatSessionsStore();
const { summaries: sessions } = storeToRefs(sessionsStore);
const subagentSessionStore = useSubagentSessionStore();
const settingStore = useSettingStore();
const updaterStore = useUpdaterStore();
const appDisplayName = "Anya";
const isDevBuild = import.meta.env.DEV;
const appWindow = getCurrentWebviewWindow();
const inputRef = ref<InstanceType<typeof ChatInputBar> | null>(null);
const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);
const editWorkspaceDialogRef = ref<InstanceType<typeof EditWorkspaceDialog> | null>(null);
const renameSessionDialogRef = ref<InstanceType<typeof RenameSessionDialog> | null>(null);
const composerWrapRef = ref<HTMLElement | null>(null);
/** True while focus is inside the composer, so the mascot looks at the input. */
const composerFocused = ref(false);
const composerFootprint = ref(0);

function handleEditFromImage(payload: { images: string[]; draftText?: string; region?: boolean }) {
  void inputRef.value?.attachImageEditReference?.(payload);
}

/** Extra space between the last message and the composer. */
const COMPOSER_CLEARANCE_EXTRA = 12;
let composerResizeObserver: ResizeObserver | null = null;

const composerOverlayStyle = computed(() => {
  const height = composerFootprint.value;
  if (height <= 0) {
    return undefined;
  }
  const dock = height + 10;
  return {
    "--composer-fade-height": `${dock}px`,
    "--composer-list-clearance": `${dock + COMPOSER_CLEARANCE_EXTRA}px`,
  };
});

watch(
  composerWrapRef,
  (element) => {
    composerResizeObserver?.disconnect();
    composerResizeObserver = null;
    if (!element) {
      composerFootprint.value = 0;
      return;
    }
    const sync = () => {
      composerFootprint.value = element.offsetHeight;
    };
    composerResizeObserver = new ResizeObserver(sync);
    composerResizeObserver.observe(element);
    sync();
  },
  { flush: "post" },
);

// Cross-cutting state shared by several workbench composables below. Kept
// here (rather than owned by a single composable) to avoid a construction
// cycle: e.g. labels need the active workspace name, and the workspace list
// itself needs labels for its confirm dialogs.
const navigationOpen = ref(true);
const initializing = ref(true);
const workspaces = ref<Workspace[]>([]);
const activeSessionId = ref("");
const activeSessionWorkspaceId = ref<string | null>(null);

const showOnboarding = ref(!settingStore.onboardingCompleted);

const builtInTheme = computed(() => settingStore.colorScheme);
const messages = computed(() => chatStore.sessions[rootSessionId(activeSessionId.value)] ?? []);
const conversationMessages = computed(() => {
  if (isSubagentSessionId(activeSessionId.value)) return [];
  return chatStore.sessions[activeSessionId.value] ?? [];
});
const viewingSubagent = computed(() => isSubagentSessionId(activeSessionId.value));
const parentMessages = messages;

/** What the hero mascot holds, per chat mode: ask -> phone, agent -> wrench, plan -> plan, image -> brush. */
const MASCOT_TOOL_BY_MODE: Record<ChatMode, MascotTool> = {
  ask: "phone",
  agent: "wrench",
  plan: "clipboard",
  image: "brush",
};
const heroMascotTool = computed<MascotTool>(() => {
  // Mirrors the composer's mode chip: per-session choice, falling back to the global default.
  const compose = activeSessionId.value
    ? chatStore.sessionCompose[activeSessionId.value]
    : undefined;
  return MASCOT_TOOL_BY_MODE[normalizeChatMode(compose?.chatMode ?? settingStore.chatMode)];
});

const {
  isMaximized,
  settingsOpen,
  settingsCategory,
  minimizeWindow,
  syncMaximizedState,
  toggleMaximizeWindow,
  hideWindow,
  openSettings: openSettingsPanel,
  closeSettings,
  toggleSettings,
} = useWorkbenchWindow({ appWindow });

function openSettings(category?: Parameters<typeof openSettingsPanel>[0]) {
  extensionView.value = null;
  openSettingsPanel(category);
}

const {
  labels,
  navigationLabels,
  searchLabels,
  shortcutLabels,
  shortcutHelpItems,
  emptyConversationPrompt,
} = useWorkbenchLabels({
  language: computed(() => settingStore.language),
  workspaces,
  activeSessionWorkspaceId,
});

const updaterCopy = computed(() => {
  const language = settingStore.language;
  return {
    titlebarAction: tr(language, "updater.titlebarAction"),
    confirmTitle: tr(language, "updater.confirmTitle", {
      version: updaterStore.latestVersion || "?",
    }),
    confirmDescription: tr(language, "updater.confirmDescription"),
    confirmAction: tr(language, "updater.confirmAction"),
    cancelAction: tr(language, "updater.cancelAction"),
  };
});

async function promptInstallUpdate() {
  if (!updaterStore.updateAvailable || updaterStore.isBusy) return;

  const confirmed = await confirmDialogRef.value?.ask({
    title: updaterCopy.value.confirmTitle,
    description: updaterCopy.value.confirmDescription,
    confirmLabel: updaterCopy.value.confirmAction,
    cancelLabel: updaterCopy.value.cancelAction,
    tone: "default",
  });

  if (!confirmed) return;
  await updaterStore.install();
}

const {
  pendingInteractions,
  attentionSessionIds,
  unreadSessionIdList,
  activePendingInteraction,
  askUserSession,
  pathPermissionSession,
  toolApprovalSession,
  setPendingInteraction,
  removePendingInteraction,
  markSessionUnread,
  clearSessionUnread,
  isWorkbenchClosed,
  sessionDisplayName,
  showActionableWindowsNotification,
  notifyWhenNotViewed,
  dismissNotificationForInteraction,
  completeAskUser,
  completePathPermission,
  completeToolApproval,
} = useWorkbenchInteractions({ activeSessionId, sessions, settingsOpen, labels, appWindow });

const {
  reviewOpen,
  reviewView,
  reviewViews,
  reviewWidth,
  reviewResizing,
  startReviewResize,
  handleReviewResizeKey,
  resetReviewWidth,
  updateReviewWidth,
  navigationWidth,
  navigationResizing,
  startNavigationResize,
  handleNavigationResizeKey,
  resetNavigationWidth,
  updateNavigationWidth,
  runningSubagentCount,
  imageLightboxOpen,
  openedImageSources,
  selectedImageSource,
  reviewFocusPath,
  reviewFocusAt,
  openReview,
  openReviewFile,
  toggleReviewSidebar,
  openAgentReview,
  previewImage,
  closeImageLightbox,
} = useWorkbenchReview({
  navigationOpen,
  activeSessionId,
  messages,
  sessions,
  labels,
  clearSessionUnread,
  selectConversation: handleSelectConversation,
});

const navigationShellStyle = computed(() => ({
  "--nav-shell-width": navigationOpen.value
    ? `${navigationWidth.value + NAVIGATION_RESIZE_HANDLE_WIDTH}px`
    : "0px",
}));

const {
  checkpoints,
  sessionsWithLiveTokens,
  draftSessionIds,
  quickAskSessions,
  hasConversationMessages,
  sending,
  runningSessionIds,
  titleGeneratingSessionIds,
  stagedMessages,
  contextNotice,
  sessionsForWorkspace,
  refreshSessions,
  createConversation,
  createQuickConversation,
  refreshCheckpoints,
  selectConversation,
  dismissSubagentSession,
  moveSessionToWorkspace,
  archiveConversation,
  archiveVisualBySessionId,
  renameConversation,
  regenerateConversationTitle,
  removeConversation,
  branchConversation,
  guideStaged,
  startStagedEdit,
  removeStaged,
  submitMessage,
  pauseResponse,
  handleRewound,
} = useWorkbenchSessions({
  activeSessionId,
  activeSessionWorkspaceId,
  workspaces,
  messages,
  labels,
  navigationLabels,
  confirmDialogRef,
  renameSessionDialogRef,
  inputRef,
  reviewOpen,
  removePendingInteraction,
  clearSessionUnread,
});

function handleCreateQuickConversation() {
  extensionView.value = null;
  return createQuickConversation();
}

function handleShowContext(context: CapturedContext) {
  extensionView.value = null;
  let sessionId = activeSessionId.value;
  if (!sessionId) {
    createConversation(activeSessionWorkspaceId.value);
    sessionId = activeSessionId.value;
  }
  if (!sessionId) return;
  chatStore.upsertMessage({
    id: `local-context-${Date.now()}`,
    sessionId,
    role: "assistant",
    content: "",
    environmentContext: context,
    status: "done",
    timestamp: Date.now(),
  });
}

const {
  collapsedWorkspaceIds,
  collapsedNavigationSections,
  workspaceMenuId,
  draggedWorkspaceId,
  dragOverWorkspaceId,
  workspaceDropPosition,
  draggedSessionId,
  sessionDropWorkspaceId,
  sessionDragGhost,
  workspaceNavigationSections,
  toggleNavigationSection,
  toggleWorkspaceMenu,
  handleWorkspaceClick,
  startWorkspacePointerDrag,
  startSessionPointerDrag,
  consumeSuppressedSessionClick,
  moveWorkspacePointerDrag,
  finishWorkspacePointerDrag,
  cancelWorkspacePointerDrag,
  toggleWorkspacePinned,
  editWorkspace,
  addWorkspace,
  createWorkspaceConversation,
  openWorkspaceFolder,
  openWorkspaceInTerminal,
  archiveWorkspace,
  removeWorkspace,
} = useWorkbenchWorkspaces({
  workspaces,
  sessions,
  activeSessionWorkspaceId,
  navigationLabels,
  labels,
  confirmDialogRef,
  editWorkspaceDialogRef,
  refreshSessions,
  createConversation,
  moveSessionToWorkspace,
});

function onToggleNavigationSection(id: string) {
  toggleNavigationSection(id);
}

function onWorkspaceHeaderClick(workspace: Workspace) {
  handleWorkspaceClick(workspace);
}

const workspaceMenuStyle = ref<CSSProperties>({});
const activeWorkspaceMenu = computed(
  () => workspaces.value.find((workspace) => workspace.id === workspaceMenuId.value) ?? null,
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
  const opening = workspaceMenuId.value !== id;
  if (opening && event.currentTarget instanceof HTMLElement) {
    syncWorkspaceMenuPosition(event.currentTarget);
    toggleWorkspaceMenu(id);
    return;
  }
  workspaceMenuId.value = "";
}

function closeWorkspaceMenu() {
  workspaceMenuId.value = "";
}

let workspaceMenuScrollTarget: HTMLElement | null = null;

function onWorkspaceMenuScroll() {
  closeWorkspaceMenu();
}

watch(workspaceMenuId, (id) => {
  workspaceMenuScrollTarget?.removeEventListener("scroll", onWorkspaceMenuScroll);
  workspaceMenuScrollTarget = null;
  if (!id) return;
  workspaceMenuScrollTarget = document.querySelector<HTMLElement>(".session-list");
  workspaceMenuScrollTarget?.addEventListener("scroll", onWorkspaceMenuScroll, { passive: true });
});

function openExtensionView(view: ExtensionView) {
  closeSettings();
  extensionView.value = extensionView.value === view ? null : view;
}

async function handleSelectConversation(sessionId: string) {
  if (consumeSuppressedSessionClick(sessionId)) return;
  extensionView.value = null;
  await selectConversation(sessionId);
}

function handleBranchMessage(messageId: string) {
  void branchConversation(activeSessionId.value, messageId);
}

function handleCreateWorkspaceConversation(workspace: Workspace) {
  extensionView.value = null;
  return createWorkspaceConversation(workspace);
}

const { searchPaletteOpen, shortcutHelpOpen, openSearchPalette, handleWorkbenchHotkey } =
  useWorkbenchHotkeys({
    settingsOpen,
    initializing,
    navigationOpen,
    inputRef,
    toggleReviewSidebar,
    openSettings,
    createQuickConversation: handleCreateQuickConversation,
  });

const showConversationHeader = computed(
  () => !settingsOpen.value && !reviewOpen.value && !extensionView.value,
);

const activeTitle = computed(() => {
  if (settingsOpen.value) return labels.value.settings;
  if (extensionView.value === "plugins") return navigationLabels.value.plugins;
  if (extensionView.value === "phone") return navigationLabels.value.connectPhone;
  if (isSubagentSessionId(activeSessionId.value)) {
    const badge = tr(settingStore.language, "subagent.badge");
    const preview =
      subagentSessionStore.records[activeSessionId.value]?.preview ||
      sessionsWithLiveTokens.value.find((session) => session.sessionId === activeSessionId.value)
        ?.preview ||
      "";
    const title = formatSessionPreview(preview) || labels.value.untitled;
    return title.startsWith(badge) ? title : `${badge} ${title}`;
  }
  const preview =
    sessionsWithLiveTokens.value.find((session) => session.sessionId === activeSessionId.value)
      ?.preview || "";
  return formatSessionPreview(preview) || labels.value.untitled;
});

useWorkbenchLifecycle({
  appWindow,
  activeSessionId,
  sessions,
  workspaces,
  activeSessionWorkspaceId,
  initializing,
  inputRef,
  settingsOpen,
  openSettings,
  syncMaximizedState,
  refreshSessions,
  selectConversation: handleSelectConversation,
  createQuickConversation: handleCreateQuickConversation,
  refreshCheckpoints,
  clearSessionUnread,
  markSessionUnread,
  isWorkbenchClosed,
  showActionableWindowsNotification,
  notifyWhenNotViewed,
  dismissNotificationForInteraction,
  pendingInteractions,
  setPendingInteraction,
  removePendingInteraction,
  sessionDisplayName,
  updateReviewWidth,
  updateNavigationWidth,
  handleWorkbenchHotkey,
  moveWorkspacePointerDrag,
  finishWorkspacePointerDrag,
  cancelWorkspacePointerDrag,
});

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
  composerResizeObserver?.disconnect();
  composerResizeObserver = null;
});

watch(
  () => settingStore.zoom,
  (zoom) => {
    applyZoom(zoom);
  },
  { immediate: true },
);

watch(
  () => [settingStore.colorScheme, settingStore.language] as const,
  ([colorScheme, language]) => {
    applyTheme({ colorScheme, language });
  },
  { immediate: true },
);

watch(settingsOpen, (open) => {
  if (!open) return;
  // Opening settings used to "magically" fix a stale theme after boot; re-sync
  // explicitly so the workbench never depends on that side effect.
  applyTheme({
    colorScheme: settingStore.colorScheme,
    language: settingStore.language,
  });
});
</script>

<style scoped>
.workbench {
  --workbench-chrome-bg: color-mix(in srgb, var(--peek-sidebar) 92%, var(--peek-bg));
  --nav-col: 258px;
  --titlebar-h: 32px;
  /*
   * Scale via transform (not CSS zoom): WebView2/Chromium zoom on a subtree
   * shrinks layout without reliably expanding paint into the leftover space,
   * which left empty chrome at 120%+. Inverse size + scale fills the window.
   */
  position: relative;
  box-sizing: border-box;
  width: calc(100% / var(--ui-zoom, 1));
  height: calc(100% / var(--ui-zoom, 1));
  transform: scale(var(--ui-zoom, 1));
  transform-origin: 0 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border-radius: 0;
  background: var(--workbench-chrome-bg);
  color: var(--peek-text);
  font-family: var(--font-sans);
  container-type: size;
  container-name: workbench;
}
.workbench.navigation-closed:not(.is-settings) {
  --nav-col: 42px;
}
.workbench-ready-leave-active {
  transition: opacity 280ms ease;
}
.workbench-ready-leave-to {
  opacity: 0;
}

.titlebar {
  flex: none;
  height: var(--titlebar-h);
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  background: var(--workbench-chrome-bg);
  user-select: none;
}
.titlebar.titlebar-compact {
  grid-template-columns: auto minmax(0, 1fr) auto;
}
.titlebar-drag {
  min-width: 0;
  align-self: stretch;
}

.glass-chrome {
  display: none;
}

.workbench.is-glass,
.workbench.is-glass .main-stage,
.workbench.is-glass .workspace-grid,
.workbench.is-glass .embedded-settings,
.workbench.is-glass .titlebar,
.workbench.is-glass .navigation-pane {
  background: transparent;
  box-shadow: none;
}
.workbench.is-glass .navigation-shell {
  background: transparent;
}

.workbench.is-glass .review-shell,
.workbench.is-glass .conversation-pane {
  background: var(--peek-list-bg);
}

.workbench.is-glass {
  transform: none;
  width: 100%;
  height: 100%;
}

.workbench.is-glass .glass-chrome {
  display: block;
  position: absolute;
  inset: 0;
  z-index: 0;
  pointer-events: none;
  background: var(--workbench-glass-fill);
}

/* Fullscreen / maximized: native blur is off (it ghosts icons). Use a
   smooth opaque tint so chrome does not fall through to black. */
.workbench.is-glass.is-maximized .glass-chrome {
  background: var(--workbench-glass-fill-covering);
}

.workbench.is-glass .titlebar,
.workbench.is-glass .main-stage {
  position: relative;
  z-index: 1;
}

.workbench.is-glass :deep(.settings-nav),
.workbench.is-glass :deep([data-slot="sidebar"]),
.workbench.is-glass :deep([data-slot="sidebar-wrapper"]) {
  background: transparent !important;
  box-shadow: none;
}

button {
  font: inherit;
}
.nav-shortcut-button,
.session-row,
.review-tabs button {
  border: 0;
  color: inherit;
  cursor: pointer;
}
.titlebar-leading {
  display: flex;
  align-items: center;
  align-self: stretch;
  box-sizing: border-box;
  min-width: 0;
  padding: 0 8px 0 10px;
}
.titlebar-back {
  height: var(--peek-control-icon);
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 0 8px 0 6px;
  border: 0;
  border-radius: var(--peek-radius-sm);
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
  font-size: var(--peek-font-sm);
}
.titlebar-back:hover {
  color: var(--peek-text);
  background: var(--peek-hover-bg);
}
.nav-toggle {
  flex: none;
  width: var(--peek-control-icon);
  height: var(--peek-control-icon);
}
.titlebar-context {
  justify-self: center;
  min-width: 0;
  max-width: min(420px, 46vw);
  overflow: hidden;
  padding: 0 12px;
  color: var(--peek-muted);
  font-size: 12px;
  text-align: center;
  white-space: nowrap;
}
.titlebar-context > span {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.titlebar-trailing {
  display: flex;
  align-items: center;
  justify-self: end;
  min-width: 0;
}
.view-actions,
.window-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}
.view-actions {
  padding-right: 6px;
}
.icon-button,
.small-icon-button,
.window-button,
.delete-session {
  display: inline-grid;
  place-items: center;
  padding: 0;
  border: 0;
  border-radius: var(--peek-radius-sm);
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
  transition:
    background-color var(--motion-fast, 110ms) var(--motion-ease-out, ease),
    color var(--motion-fast, 110ms) var(--motion-ease-out, ease),
    transform var(--motion-instant, 80ms) var(--motion-ease-out, ease);
}
.icon-button {
  position: relative;
  width: var(--peek-control-icon);
  height: var(--peek-control-icon);
}
.small-icon-button {
  width: var(--peek-control-icon);
  height: var(--peek-control-icon);
}
.window-button {
  width: var(--titlebar-h);
  height: var(--titlebar-h);
  border-radius: 0;
}
.windows-caption-icon {
  font-family: "Segoe Fluent Icons", "Segoe MDL2 Assets", sans-serif;
  font-size: 10px;
  line-height: 1;
}
.icon-button:hover,
.icon-button.active,
.small-icon-button:hover,
.window-button:hover {
  color: var(--peek-text);
  background: var(--peek-hover-bg);
}
.icon-button:active:not(:disabled),
.small-icon-button:active:not(:disabled) {
  background: var(--peek-press-bg);
  transform: scale(0.96);
}
.icon-button.active {
  color: var(--peek-accent);
}
.update-button {
  color: var(--peek-accent);
}
.update-button.busy {
  opacity: 0.7;
}
.update-dot {
  background: var(--peek-accent);
}
.window-button.close:hover {
  color: var(--peek-primary-foreground, #fff);
  background: var(--peek-danger);
}
.status-dot {
  position: absolute;
  right: 6px;
  bottom: 6px;
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--peek-accent);
}

.main-stage {
  flex: 1;
  min-width: 0;
  min-height: 0;
  position: relative;
  overflow: hidden;
}

.main-stage > .workspace-grid,
.main-stage > .embedded-settings {
  position: absolute;
  inset: 0;
}

.workspace-grid {
  min-width: 0;
  min-height: 0;
  display: grid;
  /* Both side columns are `auto`-sized — they just follow their pane's own
     (animated) width/mount state, rather than the grid track itself
     changing size. Grid tracks can't reliably interpolate between different
     track-sizing functions (e.g. minmax() <-> a plain length), so animating
     the pane's own width/padding is what makes the collapse feel smooth.
     The 3rd (review) column is always declared, even when no review-shell
     is mounted: toggling column *count* based on `review-open` caused it to
     briefly mismatch the review-shell's v-if mount/unmount timing during
     the enter/leave transition, so the panel got auto-placed onto a new row
     (appearing below the content) instead of into the missing 3rd column. */
  grid-template-columns: auto minmax(0, 1fr) minmax(0, auto);
  grid-template-rows: minmax(0, 1fr);
  overflow: hidden;
  background: var(--workbench-chrome-bg);
}
.workspace-grid > .navigation-shell {
  grid-column: 1;
  grid-row: 1;
}
.workspace-grid > .conversation-pane {
  grid-column: 2;
  grid-row: 1;
  min-width: 0;
}
.workspace-grid > .review-shell {
  grid-column: 3;
  grid-row: 1;
}
.workspace-grid.is-covered {
  visibility: hidden;
  pointer-events: none;
}

.embedded-settings {
  z-index: 1;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  background: var(--workbench-chrome-bg);
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
.workbench.navigation-resizing .navigation-shell {
  transition: opacity 160ms ease;
}
.workspace-grid.navigation-closed .navigation-shell {
  width: 0 !important;
  opacity: 0;
  pointer-events: none;
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
.workspace-grid.navigation-closed .navigation-pane {
  padding-left: 0;
  padding-right: 0;
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

.conversation-pane {
  --composer-dock-gap: 10px;
  --composer-fade-height: 120px;
  --composer-list-clearance: 132px;
  position: relative;
  z-index: 1;
  grid-column: 2;
  grid-row: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: visible;
  border: 1px solid color-mix(in srgb, var(--peek-border) 62%, transparent);
  border-right: 0;
  border-bottom: 0;
  border-radius: var(--peek-radius-lg, 12px) 0 0 0;
  background: var(--peek-list-bg);
  box-shadow: var(--peek-pane-shadow);
  container-type: size;
  container-name: conversation;
}
.conversation-pane.extension-open {
  background: var(--peek-list-bg);
}
.conversation-header {
  flex: none;
  display: flex;
  align-items: center;
  gap: 6px;
  min-height: 40px;
  padding: 6px 8px 4px 10px;
  border-bottom: 1px solid color-mix(in srgb, var(--peek-border) 42%, transparent);
  border-top-left-radius: inherit;
  background: var(--peek-list-bg);
}
.conversation-title {
  flex: 1;
  min-width: 0;
  margin: 0;
  overflow: hidden;
  color: var(--peek-text);
  font-size: 13px;
  font-weight: 550;
  line-height: 1.35;
  text-align: center;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.review-toggle {
  flex: none;
}
.review-toggle.active {
  color: var(--peek-accent);
}
.conversation-header-spacer {
  flex: none;
  width: var(--peek-control-icon);
  height: var(--peek-control-icon);
}
.extension-pane {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border-top-left-radius: inherit;
}
.extension-scroll {
  flex: 1;
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
  display: flex;
  flex-direction: column;
}
.extension-panel {
  flex: 1;
  min-height: 100%;
  width: 100%;
  display: flex;
  flex-direction: column;
  padding: 4px;
  box-sizing: border-box;
}
.extension-panel > :deep(.plugins-panel),
.extension-panel > :deep(.connect-phone) {
  flex: 1;
  min-height: 0;
  width: 100%;
}
.workbench-messages {
  box-sizing: border-box;
  flex: 1 1 0;
  width: 100%;
  min-height: 0;
  margin: 0;
  overflow: hidden;
  padding-top: 8px;
  padding-bottom: 0;
  contain: layout style;
  border-top-left-radius: inherit;
}
.workbench-messages :deep(.message-list) {
  padding: 18px clamp(12px, 4vw, max(40px, calc((100% - 900px) / 2))) var(--composer-list-clearance);
  gap: 20px;
  scroll-behavior: auto;
}
.workbench-messages :deep(.message-preview-rail) {
  right: 8px;
  bottom: var(--composer-fade-height);
}
.workbench-messages :deep(.scroll-to-bottom) {
  bottom: calc(var(--composer-fade-height) + 10px);
}
.workbench-messages :deep(.assistant-bubble) {
  max-width: 100%;
}
.workbench-messages :deep(.user-turn) {
  max-width: min(76%, 680px);
}
.composer-wrap {
  position: absolute;
  z-index: 8;
  left: 50%;
  top: calc(100% - var(--composer-dock-gap));
  bottom: auto;
  width: min(calc(100% - 48px), 820px);
  min-width: 0;
  min-height: 0;
  max-height: min(320px, calc(100% - 16px));
  margin: 0;
  transform: translate(-50%, -100%);
  display: flex;
  flex-direction: column;
  align-items: stretch;
  gap: 8px;
  overflow: visible;
  transition:
    top 280ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1)),
    width 280ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1)),
    max-height 280ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1)),
    transform 280ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1));
}
.workbench-messages :deep(.scroll-to-bottom) {
  border-color: color-mix(in srgb, var(--peek-border) 55%, transparent);
  background: color-mix(in srgb, var(--peek-list-bg) 92%, var(--peek-surface));
  box-shadow: var(--peek-elev-sm);
  transition:
    background-color var(--motion-fast, 110ms) ease,
    border-color var(--motion-fast, 110ms) ease,
    transform var(--motion-fast, 110ms) ease;
}
.workbench-messages :deep(.scroll-to-bottom:hover) {
  transform: translateY(-1px);
}
.composer-wrap.has-interaction-picker {
  /* Grow with ask / approval panels, but stay inside the conversation pane. */
  max-height: calc(100% - 16px);
}
.composer-wrap :deep(.chat-input-shell) {
  position: relative;
  z-index: 2;
  width: 100%;
  min-height: 0;
  max-height: 100%;
}
.composer-wrap :deep(.input-bar) {
  width: 100%;
  max-height: 100%;
  box-shadow: var(--peek-composer-shadow, var(--peek-elev-sm));
  transition:
    min-height 280ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1)),
    padding 280ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1)),
    border-radius 280ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1)),
    border-color 280ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1)),
    background 280ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1)),
    box-shadow 280ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1));
}
.composer-wrap :deep(.input-bar:focus-within) {
  box-shadow: var(--peek-composer-shadow-focus, var(--peek-composer-shadow, var(--peek-elev-sm)));
}
.composer-wrap :deep(.input-content),
.composer-wrap :deep(.composer-textarea),
.composer-wrap :deep(.footer-chip) {
  /* Do not animate min-height/line-height while typing — that reads as jitter. */
  transition:
    font-size 420ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1)),
    border-radius 420ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1)),
    letter-spacing 420ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1));
}
.staged-wrap {
  position: absolute;
  left: 0;
  right: 0;
  bottom: calc(100% - 1px);
  z-index: 1;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 0;
  pointer-events: none;
}
.staged-list {
  pointer-events: auto;
  box-sizing: border-box;
  width: min(calc(100% - 32px), 720px);
  display: flex;
  flex-direction: column;
  gap: 0;
  max-height: 184px;
  overflow-y: auto;
  padding: 6px 8px 0;
  border: 1px solid color-mix(in srgb, var(--peek-accent) 20%, transparent);
  border-bottom: 0;
  border-radius: 10px 10px 0 0;
  background: color-mix(in srgb, var(--peek-surface) 97%, transparent);
  box-shadow: 0 -10px 24px color-mix(in srgb, #000 16%, transparent);
}
.staged-item {
  box-sizing: border-box;
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 36px;
  padding: 5px 4px 5px 10px;
  border-bottom: 1px solid color-mix(in srgb, var(--peek-border) 42%, transparent);
  background: transparent;
}
.staged-item:last-child {
  border-bottom: 0;
}
.staged-item-text {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  color: var(--peek-text);
  font-size: 12px;
  line-height: 18px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.staged-item-actions {
  flex: none;
  display: inline-flex;
  align-items: center;
  gap: 2px;
  padding-top: 0;
}
.staged-btn {
  flex: none;
  width: 24px;
  height: 22px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: var(--peek-radius-sm);
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
}
.staged-btn:hover {
  background: color-mix(in srgb, var(--peek-text) 9%, transparent);
  color: var(--peek-text);
}
.staged-btn-guide {
  color: var(--peek-accent);
}
.staged-btn-guide:hover {
  background: color-mix(in srgb, var(--peek-accent) 15%, transparent);
}
.staged-btn-danger:hover {
  background: color-mix(in srgb, var(--peek-danger) 13%, transparent);
  color: var(--peek-danger);
}
.conversation-pane.empty-conversation .workbench-messages {
  visibility: hidden;
  pointer-events: none;
  padding-bottom: 18px;
}
.empty-conversation-hero {
  position: absolute;
  z-index: 1;
  left: 50%;
  bottom: calc(50% + 84px);
  width: min(calc(100% - 48px), 680px);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  transform: translateX(-50%);
  pointer-events: none;
  user-select: none;
}
.empty-hero-enter-active,
.empty-hero-leave-active {
  transition:
    opacity 320ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1)),
    transform 420ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1));
}
.empty-hero-enter-from,
.empty-hero-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(12px) scale(0.98);
}
.empty-conversation-brand {
  width: 104px;
  height: 104px;
  flex: none;
  /* The hero itself is click-through; the mascot alone reacts to clicks. */
  pointer-events: auto;
}
.empty-conversation-brand :deep(.mascot-face) {
  opacity: 0.94;
  filter: drop-shadow(0 10px 24px color-mix(in srgb, var(--peek-text) 18%, transparent));
}
.empty-conversation-prompt {
  margin: 0;
  max-width: 28em;
  color: var(--peek-text);
  font-size: clamp(20px, 2.4vw, 26px);
  font-weight: 560;
  letter-spacing: -0.025em;
  line-height: 1.35;
  text-align: center;
  text-wrap: balance;
}
.conversation-pane.empty-conversation .composer-wrap {
  /* Anchor the top edge: a growing composer must expand downward, never creep
     up over the hero title. Past the cap it scrolls internally. */
  top: calc(50% - 64px);
  width: min(calc(100% - 48px), 680px);
  transform: translate(-50%, 0);
  max-height: min(280px, calc(50% + 48px));
}

/* Ask / approval panels stack above the input, so keep that variant centered. */
.conversation-pane.empty-conversation .composer-wrap.has-interaction-picker {
  top: 50%;
  transform: translate(-50%, -50%);
  max-height: calc(100% - 16px);
}

.conversation-pane.empty-conversation .composer-wrap :deep(.input-bar) {
  min-height: 128px;
  padding: 16px 16px 12px;
  border-radius: var(--peek-radius-composer);
  border: 1px solid
    var(--peek-composer-border, color-mix(in srgb, var(--peek-text) 16%, transparent));
  background: var(--peek-composer-fill);
  box-shadow: var(--peek-composer-shadow, var(--peek-elev-sm));
}

/* Ask / permission panels should merge into the composer, not stack as a second card. */
.conversation-pane.empty-conversation
  .composer-wrap:has(:deep(.interaction-request-open))
  :deep(.input-bar) {
  border-top-left-radius: 0;
  border-top-right-radius: 0;
  box-shadow: none;
}

.conversation-pane.empty-conversation
  .composer-wrap:has(:deep(.interaction-request-open))
  :deep(.ask-user-list),
.conversation-pane.empty-conversation
  .composer-wrap:has(:deep(.interaction-request-open))
  :deep(.path-permission-list),
.conversation-pane.empty-conversation
  .composer-wrap:has(:deep(.interaction-request-open))
  :deep(.tool-approval-list) {
  margin: 0;
}

.conversation-pane.empty-conversation
  .composer-wrap:has(:deep(.attach-panel-open))
  :deep(.chat-input-shell.attach-panel-open.overlay-pickers .attach-resource-panel) {
  position: absolute;
  right: 0;
  bottom: calc(100% + 10px);
  left: 0;
  max-height: min(320px, 42vh);
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--peek-text) 12%, transparent);
  border-radius: 14px;
  box-shadow:
    0 12px 32px color-mix(in srgb, #000 18%, transparent),
    0 1px 0 color-mix(in srgb, #fff 4%, transparent) inset;
}

.conversation-pane.empty-conversation .composer-wrap :deep(.input-bar:focus-within) {
  border-color: var(
    --peek-composer-border-focus,
    color-mix(in srgb, var(--peek-text) 28%, transparent)
  );
  box-shadow: var(--peek-composer-shadow-focus, var(--peek-composer-shadow, var(--peek-elev-sm)));
}
.conversation-pane.empty-conversation .composer-wrap :deep(.input-content) {
  min-height: 56px;
}
.conversation-pane.empty-conversation .composer-wrap :deep(.composer-textarea) {
  min-height: 24px;
  font-size: 15px;
  line-height: 24px;
  letter-spacing: -0.01em;
}
.conversation-pane.empty-conversation .composer-wrap :deep(.composer-textarea::placeholder) {
  color: var(--peek-placeholder);
  letter-spacing: 0;
}
.conversation-pane.empty-conversation .composer-wrap :deep(.footer-chip) {
  height: 30px;
  border-radius: 8px;
  font-size: 12px;
}
.conversation-pane.empty-conversation .composer-wrap :deep(.model-picker-list) {
  max-height: max(96px, calc(50vh - 96px));
}
.conversation-pane.empty-conversation .composer-wrap :deep(.command-list) {
  max-height: min(260px, calc(50cqh - 96px));
  overflow-x: hidden;
  overflow-y: auto;
}
/* Detached @/# cards sit above a mid-screen composer — keep them below the titlebar. */
.conversation-pane.empty-conversation .composer-wrap :deep(.file-suggestion-list),
.conversation-pane.empty-conversation .composer-wrap :deep(.hash-suggestion-list) {
  max-height: max(96px, min(240px, calc(50vh - 200px)));
}

.conversation-pane.empty-conversation
  .composer-wrap:has(:deep(.attach-panel-open))
  :deep(.attach-tree-scroll) {
  max-height: min(200px, 32vh);
}

@media (prefers-reduced-motion: reduce) {
  .composer-wrap,
  .composer-wrap :deep(.input-bar),
  .composer-wrap :deep(.input-content),
  .composer-wrap :deep(.composer-textarea),
  .composer-wrap :deep(.footer-chip),
  .workbench-messages,
  .empty-hero-enter-active,
  .empty-hero-leave-active {
    transition: none !important;
  }
}

.context-notice {
  position: absolute;
  z-index: 9;
  top: 12px;
  left: 50%;
  box-sizing: border-box;
  width: min(calc(100% - 80px), 720px);
  min-height: 34px;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 11px;
  border: 1px solid color-mix(in srgb, var(--peek-warning) 24%, var(--peek-border));
  border-radius: 9px;
  background: color-mix(in srgb, var(--peek-warning) 8%, var(--peek-surface));
  color: var(--peek-text);
  box-shadow: 0 8px 22px color-mix(in srgb, #000 13%, transparent);
  font-size: 11px;
  line-height: 1.45;
  transform: translateX(-50%);
}
.context-notice > svg {
  flex: none;
  color: var(--peek-warning);
}
.context-notice > span {
  min-width: 0;
  overflow-wrap: anywhere;
}

.review-shell {
  grid-column: 3;
  grid-row: 1;
  min-width: 0;
  min-height: 0;
  width: min(var(--review-pane-width, 527px), 48cqw);
  max-width: 100%;
  display: flex;
  overflow: hidden;
  border-top: 1px solid color-mix(in srgb, var(--peek-border) 62%, transparent);
  background: var(--peek-list-bg);
}
.review-panel-enter-active,
.review-panel-leave-active {
  transition:
    width 220ms cubic-bezier(0.2, 0.72, 0.25, 1),
    opacity 160ms ease;
}
.review-panel-enter-from,
.review-panel-leave-to {
  width: 0 !important;
  opacity: 0;
}
.review-resize-handle {
  position: relative;
  z-index: 4;
  flex: none;
  width: 7px;
  min-width: 7px;
  cursor: col-resize;
  outline: none;
  touch-action: none;
}
.review-resize-handle::after {
  content: "";
  position: absolute;
  top: calc(50% - 18px);
  left: 2px;
  width: 3px;
  height: 36px;
  border-radius: 2px;
  background: transparent;
  transition:
    background 100ms ease,
    transform 100ms ease;
}
.review-resize-handle:hover::after,
.review-resize-handle:focus-visible::after,
.review-resize-handle.active::after {
  background: color-mix(in srgb, var(--peek-accent) 68%, var(--peek-border));
  transform: scaleY(1.15);
}
.review-pane {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  container: workspace-sidebar / inline-size;
}
.review-header {
  flex: none;
  height: 38px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 8px;
}
.review-tabs {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 2px;
}
.review-tabs button {
  height: var(--peek-control-icon);
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 0 9px;
  border-radius: var(--peek-radius-sm);
  background: transparent;
  color: var(--peek-muted);
  font-size: var(--peek-font-xs);
}
.review-tabs button:hover {
  color: var(--peek-text);
  background: var(--peek-hover-bg);
}
.review-tabs button.active {
  color: var(--peek-active-fg);
  background: color-mix(in srgb, var(--peek-accent) 13%, transparent);
}
.review-pane > :deep(aside) {
  flex: 1;
  min-height: 0;
}
.spinning {
  animation: spin 700ms linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

@media (max-width: 1120px) {
  .titlebar:not(.titlebar-compact) {
    grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
  }
  .workspace-grid.review-open .navigation-shell {
    width: 0 !important;
    opacity: 0;
    pointer-events: none;
  }
}

@media (max-height: 700px) {
  .conversation-pane {
    --composer-dock-gap: 8px;
    --composer-fade-height: 96px;
    --composer-list-clearance: 108px;
  }
  .composer-wrap {
    width: min(calc(100% - 28px), 820px);
    max-height: calc(100% - 12px);
  }
}

/* Prefer container queries so compact layout tracks zoom-compensated design size. */
@container workbench (max-width: 900px) {
  .titlebar:not(.titlebar-compact) {
    grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
  }
  .workspace-grid.review-open .navigation-shell {
    width: 0 !important;
    opacity: 0;
    pointer-events: none;
  }
}

@container workbench (max-height: 560px) {
  .conversation-pane {
    --composer-dock-gap: 8px;
    --composer-fade-height: 96px;
    --composer-list-clearance: 108px;
  }
  .composer-wrap {
    width: min(calc(100% - 20px), 820px);
    max-height: min(46cqh, calc(100% - 12px));
  }
}

@container conversation (max-width: 560px) {
  .workbench-messages :deep(.message-list) {
    padding-left: 14px;
    padding-right: 14px;
    gap: 16px;
  }
  .workbench-messages :deep(.user-turn) {
    max-width: 92%;
  }
  .composer-wrap {
    width: calc(100% - 16px);
    max-height: min(360px, calc(100% - 12px));
  }
  .composer-wrap :deep(.workbench-composer .input-footer) {
    flex-wrap: wrap;
    row-gap: 6px;
    align-items: flex-end;
  }
  .composer-wrap :deep(.workbench-composer .input-footer-primary) {
    flex: 1 1 auto;
    min-width: 0;
  }
  .composer-wrap :deep(.workbench-composer .input-footer-actions) {
    flex-wrap: wrap;
    margin-left: auto;
  }
  .composer-wrap :deep(.workbench-composer .conversation-token-meta) {
    display: none;
  }
  .composer-wrap :deep(.workbench-composer .model-badge) {
    max-width: 132px;
  }
  .composer-wrap :deep(.workbench-composer .model-name) {
    max-width: 76px;
  }
}

@container conversation (max-width: 400px) {
  .workbench-messages :deep(.message-list) {
    padding-left: 10px;
    padding-right: 10px;
  }
  .composer-wrap {
    width: calc(100% - 10px);
  }
  .composer-wrap :deep(.workbench-composer .model-badge) {
    max-width: 108px;
  }
  .composer-wrap :deep(.workbench-composer .model-name) {
    max-width: 56px;
  }
}

@container conversation (max-height: 640px) {
  .composer-wrap {
    max-height: min(42cqh, calc(100% - 24px));
  }
}

@container conversation (max-height: 480px) {
  .composer-wrap {
    max-height: min(48cqh, calc(100% - 8px));
  }
}

.shortcut-help-root {
  position: fixed;
  inset: 0;
  z-index: 75;
  display: grid;
  place-items: center;
  padding: 24px;
  background: color-mix(in srgb, #000 42%, transparent);
}
.shortcut-help-card {
  box-sizing: border-box;
  width: min(420px, 100%);
  padding: 14px;
  border: 1px solid var(--peek-border);
  border-radius: 12px;
  background: var(--peek-surface);
  color: var(--peek-text);
  box-shadow: 0 18px 48px var(--peek-shadow);
}
.shortcut-help-card header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
}
.shortcut-help-card ul {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 6px;
}
.shortcut-help-card li {
  display: flex;
  align-items: center;
  gap: 12px;
  min-height: 28px;
}
.shortcut-help-card kbd {
  flex: none;
  min-width: 88px;
  color: var(--peek-muted);
  font: 550 11px/1.2 var(--font-mono, ui-monospace, monospace);
}
.shortcut-help-enter-active,
.shortcut-help-leave-active {
  transition: opacity 140ms ease;
}
.shortcut-help-enter-from,
.shortcut-help-leave-to {
  opacity: 0;
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
  color: var(--peek-muted);
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
