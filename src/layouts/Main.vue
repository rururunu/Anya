<template>
  <main
    class="workbench"
    :class="{
      'has-mica': hasMica && !hasCustomBg,
      'chat-under-titlebar':
        !settingsOpen &&
        settingStore.chatPageStyle === 'chat' &&
        !pluginMainView &&
        !viewingSubagent,
      'has-custom-bg': hasCustomBg,
      'navigation-closed': !navigationOpen,
      'navigation-resizing': navigationResizing,
      'is-settings': settingsOpen,
      'is-maximized': isMaximized,
    }"
    :data-theme="builtInTheme"
    :style="navigationShellStyle"
    @click="workspaceMenuId = ''"
  >
    <WorkbenchPluginBackdrop v-if="pluginBackdrop" :window-focused="windowFocused" />
    <div v-else-if="activeBackground?.image" class="workbench-custom-backdrop" aria-hidden="true">
      <div class="workbench-custom-backdrop-image" :style="backdropImageStyle" />
      <div class="workbench-custom-backdrop-overlay" />
    </div>
    <AppConfirmDialog ref="confirmDialogRef" />
    <EditWorkspaceDialog ref="editWorkspaceDialogRef" />
    <RenameSessionDialog ref="renameSessionDialogRef" />
    <WorkbenchUpdateDialog ref="updateDialogRef" />
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
    <header class="titlebar titlebar-compact" data-tauri-drag-region>
      <template v-if="settingsOpen">
        <div class="titlebar-leading titlebar-navigation" data-tauri-drag-region>
          <button
            type="button"
            class="titlebar-back"
            data-tauri-drag-region="false"
            :title="labels.backToChat"
            :aria-label="labels.backToChat"
            @click="closeSettings"
          >
            <ArrowLeft :size="15" />
            <span>{{ labels.backToChat }}</span>
          </button>
        </div>

        <div class="titlebar-context" data-tauri-drag-region>
          <span class="titlebar-title-group">{{ activeTitle }}</span>
        </div>
      </template>
      <template v-else>
        <div class="titlebar-leading titlebar-navigation" data-tauri-drag-region>
          <button
            type="button"
            class="icon-button nav-toggle"
            data-tauri-drag-region="false"
            :title="labels.toggleNavigation"
            :aria-label="labels.toggleNavigation"
            :aria-pressed="navigationOpen"
            @click="navigationOpen = !navigationOpen"
          >
            <PanelLeft :size="15" />
          </button>
          <button
            type="button"
            class="icon-button navigation-search-button"
            data-tauri-drag-region="false"
            :title="searchLabels.open"
            :aria-label="searchLabels.open"
            @click.stop="openSearchPalette"
          >
            <Search :size="16" />
          </button>
        </div>
        <div class="titlebar-conversation" data-tauri-drag-region>
          <div class="titlebar-message-mode">
            <ChatPageStyleSwitch />
          </div>
          <div class="titlebar-conversation-center" data-tauri-drag-region>
            <div class="titlebar-title-group" data-tauri-drag-region>
              <MascotFace
                :key="activeSessionId"
                class="titlebar-mascot"
                :state="activeMascotState"
                :alt="tr(settingStore.language, MASCOT_STATE_LABEL_KEYS[activeMascotState])"
                :follow-pointer="false"
                :turn-on-change="false"
                eyes-only
                idle-glance
              />
              <p class="conversation-title" :title="activeTitle" data-tauri-drag-region>
                {{ activeTitle }}
              </p>
            </div>
          </div>
          <div class="titlebar-conversation-actions" data-tauri-drag-region="false">
            <button
              v-for="tab in pluginHeaderTabs"
              :key="tab.id"
              type="button"
              class="icon-button"
              :class="{ active: !tabIsChromeAction(tab.surfaces) && isPluginNavActive(tab) }"
              :title="tab.title"
              :aria-label="tab.title"
              @click="onPluginHeaderClick(tab)"
            >
              <PluginSidebarIcon :name="tab.icon" :size="15" />
            </button>
            <button
              type="button"
              class="icon-button review-toggle"
              :title="reviewOpen ? labels.closePanel : labels.views"
              :aria-label="reviewOpen ? labels.closePanel : labels.views"
              :aria-expanded="reviewOpen"
              aria-controls="workbench-review-pane"
              @click="toggleReviewSidebar"
            >
              <X v-if="reviewOpen" :size="15" />
              <PanelRight v-else :size="15" />
              <span v-if="!reviewOpen && runningSubagentCount" class="status-dot" />
            </button>
          </div>
        </div>
      </template>

      <div class="titlebar-trailing" data-tauri-drag-region="false">
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
        <div class="navigation-shell" :inert="!navigationOpen" :aria-hidden="!navigationOpen">
          <aside class="navigation-pane">
            <div v-if="pluginNavTabs.length" class="navigation-menu-items">
              <button
                v-for="tab in pluginNavTabs"
                :key="tab.id"
                type="button"
                class="nav-shortcut-button"
                :class="{ active: isPluginNavActive(tab) }"
                @click.stop="togglePluginSidebar(tab.id)"
              >
                <PluginSidebarIcon :name="tab.icon" :size="15" />
                <span>{{ tab.title }}</span>
              </button>
            </div>

            <nav
              class="session-list peek-scrollbar"
              :class="{ 'is-dragging': Boolean(draggedSessionId) }"
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
                    :aria-expanded="!collapsedNavigationSections.has(workspaceSection.id)"
                    @click="onToggleNavigationSection(workspaceSection.id)"
                  >
                    <span>{{ workspaceSection.label }}</span>
                    <ChevronRight
                      :size="13"
                      aria-hidden="true"
                      :class="{ expanded: !collapsedNavigationSections.has(workspaceSection.id) }"
                    />
                  </button>
                  <button
                    v-if="workspaceSection.id === 'workspaces'"
                    type="button"
                    class="section-action"
                    data-tour="workspace"
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
                    :class="{ 'workspace-menu-open': workspaceMenuId === workspace.id }"
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
                      @dragstart.prevent
                    >
                      <span class="workspace-path-tip">
                        <TooltipProvider :delay-duration="280">
                          <Tooltip :disabled="Boolean(draggedSessionId)">
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
                        :running-session-states="runningSessionStates"
                        :title-generating-session-ids="titleGeneratingSessionIds"
                        :attention-session-ids="attentionSessionIds"
                        :pending-interactions="pendingInteractions"
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
                    :aria-expanded="!collapsedNavigationSections.has('quick')"
                    @click="onToggleNavigationSection('quick')"
                  >
                    <span>{{ navigationLabels.quickAsk }}</span>
                    <ChevronRight
                      :size="13"
                      aria-hidden="true"
                      :class="{ expanded: !collapsedNavigationSections.has('quick') }"
                    />
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
                    :running-session-states="runningSessionStates"
                    :title-generating-session-ids="titleGeneratingSessionIds"
                    :attention-session-ids="attentionSessionIds"
                    :pending-interactions="pendingInteractions"
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
            <SidebarUpdateButton @open="updateDialogRef?.open()" />
            <SidebarProfileButton :settings-label="labels.settings" @open="openSettings('profile')">
              <template #actions>
                <button
                  type="button"
                  class="icon-button sidebar-footer-action"
                  :class="{ active: extensionView === 'plugins' }"
                  :title="navigationLabels.plugins"
                  :aria-label="navigationLabels.plugins"
                  aria-haspopup="dialog"
                  :aria-expanded="extensionView === 'plugins'"
                  @click.stop="openExtensionDialog('plugins', $event)"
                >
                  <Puzzle :size="17" :stroke-width="1.75" />
                </button>
                <button
                  type="button"
                  class="icon-button sidebar-footer-action"
                  :class="{ active: extensionView === 'phone' }"
                  :title="navigationLabels.connectPhone"
                  :aria-label="navigationLabels.connectPhone"
                  aria-haspopup="dialog"
                  :aria-expanded="extensionView === 'phone'"
                  @click.stop="openExtensionDialog('phone', $event)"
                >
                  <Smartphone :size="17" :stroke-width="1.75" />
                  <span v-if="remoteGatewayRunning" class="nav-status-dot" aria-hidden="true" />
                </button>
              </template>
            </SidebarProfileButton>
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
            'empty-conversation':
              !pluginMainView &&
              !hasConversationMessages &&
              settingStore.chatPageStyle !== 'stream',
            'extension-open': Boolean(pluginMainView),
            'page-style-chat': settingStore.chatPageStyle === 'chat',
            'page-style-stream': settingStore.chatPageStyle === 'stream',
          }"
          :style="composerOverlayStyle"
        >
          <div v-show="pluginMainView" class="extension-pane">
            <div class="extension-scroll peek-scrollbar">
              <div class="extension-panel">
                <div v-if="pluginMainView" class="plugin-main-view">
                  <PluginSlotOutlet anchor-id="workbench.main" />
                </div>
              </div>
            </div>
          </div>
          <Teleport :disabled="!pluginChatMountEl" :to="pluginChatMountEl || 'body'">
            <div
              v-show="pluginChatMountEl || !pluginMainView"
              class="conversation-stack"
              :class="{ 'is-embedded': Boolean(pluginChatMountEl) }"
            >
              <div v-if="contextNotice" class="context-notice" role="status">
                <CircleAlert :size="14" :stroke-width="1.8" aria-hidden="true" />
                <span>{{ contextNotice }}</span>
              </div>
              <PluginSlotOutlet
                v-if="pluginsStore.conversationWorkspacePluginId"
                class="peek-scrollbar"
                anchor-id="conversation.materials"
                :plugin-id="pluginsStore.conversationWorkspacePluginId"
                fit="content"
                layout="rail"
              />
              <Transition name="empty-hero">
                <div
                  v-if="
                    !viewingSubagent &&
                    !hasConversationMessages &&
                    settingStore.chatPageStyle !== 'stream'
                  "
                  class="empty-conversation-hero is-guided"
                >
                  <EmptyConversationHero
                    :has-provider="hasProvider"
                    :prompt="emptyConversationPrompt"
                    :composer-focused="composerFocused"
                    :look-at="composerFocused ? composerWrapRef : null"
                    @open-providers="openProviderSettings"
                  />
                </div>
              </Transition>
              <AppErrorBoundary v-if="viewingSubagent" compact class="workbench-messages">
                <SubagentConversationPanel
                  class="workbench-messages"
                  :session-id="activeSessionId"
                  :parent-messages="parentMessages"
                  :language="settingStore.language"
                  :show-reasoning="settingStore.showReasoning"
                  :show-execution-process="settingStore.showExecutionProcess"
                  :display-mode="settingStore.agentWorkDisplay"
                />
              </AppErrorBoundary>
              <AppErrorBoundary v-else compact class="workbench-messages">
                <MessageList
                  :workspace-root="activeSessionWorkspaceId ?? undefined"
                  class="workbench-messages"
                  :messages="conversationMessages"
                  :session-id="activeSessionId"
                  :checkpoints="checkpoints"
                  :page-style="settingStore.chatPageStyle"
                  :inline-conversation="settingStore.chatPageStyle === 'stream'"
                  :ask-user="askUserSession"
                  :interaction-wait-times="interactionWaitTimes"
                  :submit-ask-answer="completeAskUser"
                  @rewound="handleRewound"
                  @branch="handleBranchMessage"
                  @review-changes="openReview('diff')"
                  @review-file="openReviewFile"
                  @review-plan="openReview('plan')"
                  @inspect-subagent="openAgentReview"
                  @preview-image="previewImage"
                  @edit-from-image="handleEditFromImage"
                >
                  <template #composer>
                    <div
                      v-if="settingStore.chatPageStyle === 'stream'"
                      ref="streamInputRowRef"
                      class="thread-input-row"
                    />
                  </template>
                </MessageList>
              </AppErrorBoundary>

              <div
                v-if="!viewingSubagent"
                ref="composerWrapRef"
                class="composer-wrap"
                :class="{
                  'has-interaction-picker': Boolean(pathPermissionSession || toolApprovalSession),
                }"
                @focusin="composerFocused = true"
                @focusout="composerFocused = false"
              >
                <div
                  v-if="stagedMessages.length && settingStore.chatPageStyle !== 'stream'"
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
                <PluginSlotOutlet
                  class="plugin-composer-slot"
                  anchor-id="composer.accessory"
                  fit="content"
                  layout="wrap"
                />
                <ChatInputBar
                  ref="inputRef"
                  :sending="sending"
                  :close-on-escape="false"
                  :appearance="settingStore.chatPageStyle === 'stream' ? 'overlay' : 'workbench'"
                  :inline-composer="settingStore.chatPageStyle === 'stream'"
                  :input-target="settingStore.chatPageStyle === 'stream' ? streamInputRowRef : null"
                  :picker-direction="settingStore.chatPageStyle === 'stream' ? 'up' : undefined"
                  :placeholder="
                    settingStore.chatPageStyle === 'stream'
                      ? tr(settingStore.language, 'continueQuestion')
                      : undefined
                  "
                  overlay-pickers
                  :context-ready="true"
                  :session-id="activeSessionId"
                  :path-permission="pathPermissionSession"
                  :tool-approval="toolApprovalSession"
                  @submit="submitFromComposer"
                  @configure-provider="focusProviderSetup"
                  @pause="pauseResponse"
                  @path-permission-complete="completePathPermission"
                  @tool-approval-complete="completeToolApproval"
                  @preview-image="previewImage"
                  @open-history="openSearchPalette"
                  @close="handleCreateQuickConversation"
                  @new-conversation="handleNewConversation"
                />
              </div>
            </div>
          </Teleport>
        </section>

        <Transition name="review-panel">
          <div
            v-show="reviewOpen"
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
            <aside id="workbench-review-pane" class="review-pane">
              <header class="review-header">
                <TooltipProvider :delay-duration="220">
                  <div class="review-tabs" role="tablist" :aria-label="labels.views">
                    <Tooltip v-for="view in reviewViews" :key="view.id">
                      <TooltipTrigger as-child>
                        <button
                          type="button"
                          :class="{ active: reviewView === view.id }"
                          :aria-label="view.label"
                          @click="selectReviewView(view.id)"
                        >
                          <component :is="view.icon" :size="14" />
                        </button>
                      </TooltipTrigger>
                      <TooltipContent side="bottom" :side-offset="6">
                        {{ view.label }}
                      </TooltipContent>
                    </Tooltip>
                    <Tooltip v-for="tab in pluginViewTabs" :key="tab.id">
                      <TooltipTrigger as-child>
                        <button
                          type="button"
                          :class="{ active: reviewView === tab.id }"
                          :aria-label="tab.title"
                          @click="selectReviewView(tab.id)"
                        >
                          <PluginSidebarIcon :name="tab.icon" :size="14" />
                        </button>
                      </TooltipTrigger>
                      <TooltipContent side="bottom" :side-offset="6">
                        {{ tab.title }}
                      </TooltipContent>
                    </Tooltip>
                  </div>
                </TooltipProvider>
              </header>
              <CodeDiffSidebar
                v-show="reviewView === 'diff'"
                embedded
                :active="reviewOpen && reviewView === 'diff'"
                :messages="messages"
                :width="reviewWidth"
                :focus-path="reviewFocusPath"
                :focus-at="reviewFocusAt"
              />
              <ReadCodeSidebar v-if="codePreview.selection" v-show="reviewView === 'code'" />
              <PlanPreviewSidebar
                v-show="reviewView === 'plan'"
                embedded
                :session-id="activeSessionId"
                :messages="messages"
                :width="reviewWidth"
              />
              <AgentDebugPanel v-show="reviewView === 'runtime'" embedded />
              <NativeTerminalPanel
                v-if="terminalVisited"
                v-show="terminalVisible"
                :visible="terminalVisible"
                :cwd="
                  workspaces.find((workspace) => workspace.id === activeSessionWorkspaceId)?.root
                "
              />
              <PluginSlotOutlet
                class="plugin-review-host"
                anchor-id="sidebar.tabs"
                exclude-main-pane
                :active-id="reviewView"
                :host-active="reviewOpen"
              />
            </aside>
          </div>
        </Transition>
      </div>

      <div v-if="settingsOpen" class="embedded-settings">
        <SettingsPage embedded :category="settingsCategory" />
      </div>
    </div>

    <DialogRoot :open="Boolean(extensionView)" @update:open="onExtensionDialogOpenChange">
      <DialogPortal>
        <DialogOverlay class="extension-dialog-overlay" />
        <DialogContent
          class="extension-dialog"
          :class="{ 'extension-dialog-phone': extensionView === 'phone' }"
          :aria-describedby="undefined"
          @close-auto-focus="restoreExtensionFocus"
        >
          <header class="extension-dialog-header">
            <DialogTitle class="extension-dialog-title">
              {{ extensionDialogTitle }}
            </DialogTitle>
            <DialogClose class="icon-button" :title="labels.close" :aria-label="labels.close">
              <X :size="18" />
            </DialogClose>
          </header>
          <div
            class="extension-dialog-body peek-scrollbar"
            :class="{ 'extension-dialog-body-tabs': extensionView === 'plugins' }"
          >
            <ConnectPhonePanel v-if="extensionView === 'phone'" />
            <PluginsPanel
              v-else-if="extensionView === 'plugins'"
              :plugins-label="navigationLabels.plugins"
            />
          </div>
        </DialogContent>
      </DialogPortal>
    </DialogRoot>

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
import ReadCodeSidebar from "@/components/chat/ReadCodeSidebar.vue";
import { invoke } from "@tauri-apps/api/core";
import { useCodeReadPreviewStore } from "@/stores/codeReadPreview";
import {
  computed,
  defineAsyncComponent,
  nextTick,
  onMounted,
  onUnmounted,
  ref,
  watch,
  type CSSProperties,
} from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  DialogClose,
  DialogContent,
  DialogOverlay,
  DialogPortal,
  DialogRoot,
  DialogTitle,
} from "reka-ui";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  ArrowLeft,
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
  Pin,
  PinOff,
  Plus,
  Puzzle,
  Search,
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
import PlanPreviewSidebar from "@/components/chat/PlanPreviewSidebar.vue";
import ImageLightbox from "@/components/chat/ImageLightbox.vue";
import MessageList from "@/components/chat/MessageList.vue";
import ChatPageStyleSwitch from "@/components/chat/ChatPageStyleSwitch.vue";
import MascotFace from "@/components/icons/MascotFace.vue";
import { MASCOT_STATE_LABEL_KEYS } from "@/services/chat/mascotState";
import SidebarProfileButton from "@/components/workbench/SidebarProfileButton.vue";
import SidebarUpdateButton from "@/components/workbench/SidebarUpdateButton.vue";
import WorkbenchUpdateDialog from "@/components/workbench/WorkbenchUpdateDialog.vue";
import SubagentConversationPanel from "@/components/chat/SubagentConversationPanel.vue";
import WorkbenchSessionList from "@/components/workbench/WorkbenchSessionList.vue";
import NavCollapse from "@/components/workbench/NavCollapse.vue";
import WorkbenchSearchPalette from "@/components/workbench/WorkbenchSearchPalette.vue";
import WorkbenchLoading from "@/components/workbench/WorkbenchLoading.vue";
import EmptyConversationHero from "@/components/onboarding/EmptyConversationHero.vue";
import { useProductTour } from "@/composables/useProductTour";
import { finishProductTour } from "@/services/onboarding/productTour";
import { hasConfiguredProvider } from "@/services/onboarding/providerReady";
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
import { usePluginsStore } from "@/stores/plugins";
import { remoteGatewayStatus, type GatewayStatus } from "@/commands/remote";
import { useResolvedBackgroundSrc } from "@/composables/theme/useResolvedBackgroundSrc";
import type { Workspace } from "@/commands/workspace";
import PluginSlotOutlet from "@/components/plugins/PluginSlotOutlet.vue";
import PluginSidebarIcon from "@/components/plugins/PluginSidebarIcon.vue";
import WorkbenchPluginBackdrop from "@/components/plugins/WorkbenchPluginBackdrop.vue";
import { getAssetOverride } from "@/composables/plugins/assetRegistry";
import {
  tabIsChromeAction,
  tabOpensMainPane,
  tabShowsOn,
  type PluginChromeSurface,
} from "@/composables/plugins/slotRegistry";
import {
  bindPluginAgentSession,
  bindPluginConversationSend,
  listenPluginAskAnya,
  pluginChatMountEl,
} from "@/composables/plugins/conversationSend";

const SettingsPage = defineAsyncComponent(() => import("@/pages/Settings/index.vue"));
const NativeTerminalPanel = defineAsyncComponent(
  () => import("@/components/workbench/NativeTerminalPanel.vue"),
);
const PluginsPanel = defineAsyncComponent(() => import("@/components/workbench/PluginsPanel.vue"));
const ConnectPhonePanel = defineAsyncComponent(
  () => import("@/components/workbench/ConnectPhonePanel.vue"),
);

type ExtensionView = "plugins" | "phone";
const hasMica = ref(false);
const extensionView = ref<ExtensionView | null>(null);
let extensionTrigger: HTMLElement | null = null;
const remoteGatewayRunning = ref(false);
let remoteGatewayUnlisten: UnlistenFn | null = null;

const chatStore = useChatStore();
const sessionsStore = useChatSessionsStore();
const { summaries: sessions } = storeToRefs(sessionsStore);
const subagentSessionStore = useSubagentSessionStore();
const settingStore = useSettingStore();
const pluginsStore = usePluginsStore();
const pluginMainView = computed(() => pluginsStore.mainView);
const pluginSidebarTabs = computed(() => pluginsStore.sidebarTabs);
const pluginNavTabs = computed(() =>
  pluginSidebarTabs.value.filter((tab) => tabShowsOn(tab.surfaces, "nav")),
);
const pluginHeaderTabs = computed(() =>
  pluginSidebarTabs.value.filter((tab) => tabShowsOn(tab.surfaces, "header")),
);
const pluginViewTabs = computed(() =>
  pluginSidebarTabs.value.filter((tab) => tabShowsOn(tab.surfaces, "views")),
);
const updateDialogRef = ref<InstanceType<typeof WorkbenchUpdateDialog> | null>(null);
const appWindow = getCurrentWebviewWindow();
const inputRef = ref<InstanceType<typeof ChatInputBar> | null>(null);
const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);
const editWorkspaceDialogRef = ref<InstanceType<typeof EditWorkspaceDialog> | null>(null);
const renameSessionDialogRef = ref<InstanceType<typeof RenameSessionDialog> | null>(null);
const composerWrapRef = ref<HTMLElement | null>(null);
const streamInputRowRef = ref<HTMLElement | null>(null);
/** True while focus is inside the composer, so the mascot looks at the input. */
const composerFocused = ref(false);
const composerFootprint = ref(0);

function handleEditFromImage(payload: { images: string[]; draftText?: string; region?: boolean }) {
  void inputRef.value?.attachImageEditReference?.(payload);
}

/** Extra space between the last message and the composer. */
const COMPOSER_CLEARANCE_EXTRA = 12;
let composerResizeObserver: ResizeObserver | null = null;

/** Composer dock height plus the absolutely-positioned staged queue above it. */
function measureComposerFootprint(element: HTMLElement): number {
  const staged = element.querySelector(".staged-wrap") as HTMLElement | null;
  const stagedHeight = staged?.getBoundingClientRect().height ?? 0;
  return element.offsetHeight + stagedHeight;
}

const pluginBackdrop = computed(() => getAssetOverride("workbench.backdrop"));
const activeBackground = computed(() => {
  const currentThemeId = settingStore.colorScheme;
  const customTheme = settingStore.customThemes.find((t) => t.id === currentThemeId);
  if (customTheme?.background?.image) {
    return customTheme.background;
  }
  return settingStore.customBackground;
});
const hasCustomBg = computed(() => Boolean(pluginBackdrop.value || activeBackground.value?.image));

const { resolvedSource: backdropImageSrc } = useResolvedBackgroundSrc(
  () => activeBackground.value?.image,
);

const backdropImageStyle = computed<CSSProperties>(() => {
  const bg = activeBackground.value;
  if (!bg?.image || !backdropImageSrc.value) return {};
  const opacity = typeof bg.opacity === "number" ? bg.opacity : 0.2;
  const blur = typeof bg.blur === "number" ? bg.blur : 0;
  const fit = (bg.fit as CSSProperties["backgroundSize"]) || "cover";

  return {
    backgroundImage: `url(${JSON.stringify(backdropImageSrc.value)})`,
    backgroundSize: fit,
    backgroundPosition: "center",
    backgroundRepeat: "no-repeat",
    opacity,
    filter: blur > 0 ? `blur(${blur}px)` : "none",
  };
});

watch(
  hasCustomBg,
  (hasBg) => {
    if (typeof document !== "undefined") {
      document.documentElement.classList.toggle("has-custom-bg", hasBg);
    }
  },
  { immediate: true },
);

onUnmounted(() => {
  if (typeof document !== "undefined") {
    document.documentElement.classList.remove("has-custom-bg");
  }
});

const composerOverlayStyle = computed(() => {
  const height = composerFootprint.value;
  if (height <= 0) {
    return undefined;
  }
  return {
    "--composer-fade-height": `calc(${height}px + var(--composer-dock-gap))`,
    "--composer-list-clearance": `calc(var(--composer-fade-height) + ${COMPOSER_CLEARANCE_EXTRA}px)`,
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
      composerFootprint.value = measureComposerFootprint(element);
      const staged = element.querySelector(".staged-wrap");
      if (staged instanceof HTMLElement && composerResizeObserver) {
        composerResizeObserver.observe(staged);
      }
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

const hasProvider = computed(() => hasConfiguredProvider(settingStore));

const builtInTheme = computed(() => settingStore.colorScheme);
const messages = computed(() => chatStore.sessions[rootSessionId(activeSessionId.value)] ?? []);
const conversationMessages = computed(() => {
  if (isSubagentSessionId(activeSessionId.value)) return [];
  return chatStore.sessions[activeSessionId.value] ?? [];
});
const viewingSubagent = computed(() => isSubagentSessionId(activeSessionId.value));
const parentMessages = messages;

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
} = useWorkbenchWindow({ appWindow });

function openSettings(category?: Parameters<typeof openSettingsPanel>[0]) {
  extensionView.value = null;
  pluginsStore.activeSidebarTabId = null;
  pluginsStore.conversationWorkspacePluginId = null;
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

const {
  pendingInteractions,
  attentionSessionIds,
  interactionWaitTimes,
  unreadSessionIdList,
  askUserSession,
  pathPermissionSession,
  toolApprovalSession,
  setPendingInteraction,
  removePendingInteraction,
  syncPendingInteractions,
  resolvePendingInteraction,
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

const terminalVisible = computed(
  () => !settingsOpen.value && reviewOpen.value && reviewView.value === "terminal",
);
const terminalVisited = ref(terminalVisible.value);
watch(terminalVisible, (visible) => {
  if (visible) terminalVisited.value = true;
});

const navigationShellStyle = computed(() => ({
  "--nav-shell-width": navigationOpen.value
    ? `${navigationWidth.value + NAVIGATION_RESIZE_HANDLE_WIDTH}px`
    : "0px",
  "--settings-nav-width": `${navigationWidth.value + NAVIGATION_RESIZE_HANDLE_WIDTH}px`,
  "--nav-resize-width": `${NAVIGATION_RESIZE_HANDLE_WIDTH}px`,
}));

const {
  checkpoints,
  sessionsWithLiveTokens,
  draftSessionIds,
  quickAskSessions,
  hasConversationMessages,
  sending,
  runningSessionStates,
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

watch(
  stagedMessages,
  () => {
    const element = composerWrapRef.value;
    if (!element) return;
    nextTick(() => {
      composerFootprint.value = measureComposerFootprint(element);
      const staged = element.querySelector(".staged-wrap");
      if (staged instanceof HTMLElement && composerResizeObserver) {
        composerResizeObserver.observe(staged);
      }
    });
  },
  { flush: "post" },
);

function handleCreateQuickConversation() {
  extensionView.value = null;
  pluginsStore.clearMainView();
  pluginsStore.activeSidebarTabId = null;
  return createQuickConversation();
}

function handleNewConversation() {
  extensionView.value = null;
  pluginsStore.clearMainView();
  pluginsStore.activeSidebarTabId = null;
  createConversation(activeSessionWorkspaceId.value);
}

const {
  collapsedWorkspaceIds,
  collapsedNavigationSections,
  workspaceMenuId,
  draggedSessionId,
  sessionDropWorkspaceId,
  sessionDragGhost,
  workspaceNavigationSections,
  toggleNavigationSection,
  toggleWorkspaceMenu,
  handleWorkspaceClick,
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

const extensionDialogTitle = computed(() => {
  switch (extensionView.value) {
    case "phone":
      return navigationLabels.value.connectPhone;
    default:
      return navigationLabels.value.plugins;
  }
});
function openExtensionDialog(view: ExtensionView, event: MouseEvent) {
  extensionTrigger = event.currentTarget as HTMLElement;
  extensionView.value = view;
}

function onExtensionDialogOpenChange(open: boolean) {
  if (!open) extensionView.value = null;
}

function restoreExtensionFocus(event: Event) {
  event.preventDefault();
  extensionTrigger?.focus({ preventScroll: true });
  extensionTrigger = null;
}

function selectReviewView(id: string) {
  reviewView.value = id;
  const isPlugin = pluginsStore.sidebarTabs.some((tab) => tab.id === id);
  pluginsStore.activeSidebarTabId = isPlugin ? id : null;
  if (!reviewOpen.value) {
    reviewOpen.value = true;
    updateReviewWidth();
    updateNavigationWidth();
  }
}

function onPluginHeaderClick(tab: {
  id: string;
  pluginId: string;
  surfaces: PluginChromeSurface[];
  onClick?: () => void | Promise<void>;
}) {
  if (tabIsChromeAction(tab.surfaces)) {
    void Promise.resolve(tab.onClick?.()).catch((error) => {
      pluginsStore.recordError({
        pluginId: tab.pluginId,
        phase: "mount",
        message: error instanceof Error ? error.message : String(error),
      });
    });
    return;
  }
  togglePluginSidebar(tab.id);
}

function togglePluginSidebar(tabId: string) {
  closeSettings();
  const tab = pluginSidebarTabs.value.find((item) => item.id === tabId);
  if (tab && tabOpensMainPane(tab.surfaces)) {
    reviewOpen.value = false;
    pluginsStore.activeSidebarTabId = null;
    pluginsStore.conversationWorkspacePluginId = null;
    extensionView.value = null;
    if (pluginsStore.mainView?.id === tab.id) {
      pluginsStore.clearMainView();
      return;
    }
    pluginsStore.clearMainView();
    pluginsStore.setMainView({
      id: tab.id,
      pluginId: tab.pluginId,
      title: tab.title,
      mount: tab.mount,
    });
    return;
  }
  pluginsStore.clearMainView();
  extensionView.value = null;
  if (reviewOpen.value && reviewView.value === tabId) {
    reviewOpen.value = false;
    pluginsStore.activeSidebarTabId = null;
    pluginsStore.conversationWorkspacePluginId = null;
    return;
  }
  if (tab) pluginsStore.conversationWorkspacePluginId = tab.pluginId;
  selectReviewView(tabId);
}

function isPluginNavActive(tab: { id: string; pluginId: string; surfaces: PluginChromeSurface[] }) {
  if (tabIsChromeAction(tab.surfaces)) return false;
  if (tabOpensMainPane(tab.surfaces)) return pluginMainView.value?.id === tab.id;
  if (pluginsStore.conversationWorkspacePluginId === tab.pluginId) return true;
  return reviewOpen.value && reviewView.value === tab.id;
}

async function handleSelectConversation(sessionId: string) {
  if (consumeSuppressedSessionClick(sessionId)) return;
  extensionView.value = null;
  pluginsStore.clearMainView();
  pluginsStore.activeSidebarTabId = null;
  await selectConversation(sessionId);
}

function handleBranchMessage(messageId: string) {
  void branchConversation(activeSessionId.value, messageId);
}

function handleCreateWorkspaceConversation(workspace: Workspace) {
  extensionView.value = null;
  pluginsStore.clearMainView();
  pluginsStore.activeSidebarTabId = null;
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

const activeTitle = computed(() => {
  if (settingsOpen.value) return labels.value.settings;
  if (pluginMainView.value) return pluginMainView.value.title;
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

const activeMascotState = computed(() => {
  const sessionId = activeSessionId.value;
  if (pendingInteractions.value[sessionId]) return "waiting";
  return runningSessionStates.value.get(sessionId) ?? "idle";
});

const windowFocused = ref(true);

useWorkbenchLifecycle({
  appWindow,
  activeSessionId,
  sessions,
  workspaces,
  activeSessionWorkspaceId,
  initializing,
  inputRef,
  settingsOpen,
  windowFocused,
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
  syncPendingInteractions,
  resolvePendingInteraction,
  sessionDisplayName,
  updateReviewWidth,
  updateNavigationWidth,
  handleWorkbenchHotkey: (event) => {
    if (!extensionView.value) handleWorkbenchHotkey(event);
  },
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

function submitFromComposer(text: string) {
  finishProductTour();
  void submitMessage(text);
}

function focusProviderSetup() {
  const input = document.querySelector<HTMLInputElement>("[data-tour='provider-setup'] input");
  if (input) {
    input.focus();
    return;
  }
  openSettings("provider");
}

function openProviderSettings() {
  finishProductTour();
  openSettings("provider");
}

useProductTour({
  blocked: initializing,
  hasProvider,
  prepare: () => {
    if (settingsOpen.value) closeSettings();
  },
});

onMounted(async () => {
  void invoke<boolean>("apply_workbench_mica")
    .then((enabled) => {
      hasMica.value = enabled;
    })
    .catch((error) => console.debug("Workbench backdrop unavailable", error));
  bindPluginConversationSend((text) => submitMessage(text));
  bindPluginAgentSession(() => activeSessionId.value);
  window.addEventListener("resize", closeWorkspaceMenu);
  await refreshRemoteGatewayRunning();
  try {
    await listenPluginAskAnya();
  } catch {
    /* event bridge unavailable in some shells */
  }
  try {
    remoteGatewayUnlisten = await listen<GatewayStatus>("remote-gateway-status", (event) => {
      remoteGatewayRunning.value = Boolean(event.payload?.running);
    });
  } catch {
    /* event bridge unavailable in some shells */
  }
});

onUnmounted(() => {
  bindPluginConversationSend(null);
  bindPluginAgentSession(null);
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
  () =>
    [
      settingStore.colorScheme,
      settingStore.language,
      settingStore.chromeFrostedGlass,
      settingStore.customThemes,
    ] as const,
  ([colorScheme, language, chromeFrostedGlass, customThemes]) => {
    applyTheme({ colorScheme, language, chromeFrostedGlass, customThemes });
  },
  { immediate: true },
);

watch(settingsOpen, (open) => {
  if (!open) return;
  applyTheme({
    colorScheme: settingStore.colorScheme,
    language: settingStore.language,
    chromeFrostedGlass: settingStore.chromeFrostedGlass,
    customThemes: settingStore.customThemes,
  });
});
const codePreview = useCodeReadPreviewStore();
watch(
  () => codePreview.selection,
  (selection) => {
    if (selection) {
      reviewView.value = "code";
      reviewOpen.value = true;
      updateReviewWidth();
    }
  },
);
watch(activeSessionId, () => {
  codePreview.clear();
  if (reviewView.value === "code") {
    reviewView.value = "diff";
    reviewOpen.value = false;
  }
});
</script>

<style scoped>
@media (max-height: 650px) {
  .empty-conversation-brand {
    display: none;
  }
}
.workbench {
  --workbench-navigation-selected: color-mix(in srgb, var(--peek-text) 3.5%, transparent);
  --workbench-chrome-bg: color-mix(in srgb, var(--peek-sidebar) 92%, var(--peek-bg));
  --workbench-navigation-bg: var(--peek-sidebar);
  --nav-col: 258px;
  --titlebar-h: 32px;
  --titlebar-top-inset: 8px;
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
.workbench[data-theme="light"]:not(.has-custom-bg, .has-mica) {
  --workbench-chrome-bg: #f1f4f4;
  --workbench-navigation-bg: #eef1f1;
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
  --titlebar-control-bg: color-mix(in srgb, var(--peek-text) 5%, transparent);
  --titlebar-control-radius: 14px;
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
  box-sizing: border-box;
  height: calc(var(--titlebar-h) + var(--titlebar-top-inset));
  padding-top: var(--titlebar-top-inset);
}

.workbench-custom-backdrop {
  position: absolute;
  inset: 0;
  z-index: 0;
  pointer-events: none;
  overflow: hidden;
}

.workbench-custom-backdrop-image {
  position: absolute;
  inset: 0;
  transition:
    opacity 280ms ease,
    filter 280ms ease;
  transform: translateZ(0);
}

.workbench-custom-backdrop-overlay {
  position: absolute;
  inset: 0;
  background: color-mix(in srgb, var(--peek-bg) 8%, transparent);
}

.workbench.has-custom-bg {
  background: transparent;
}

.workbench.has-custom-bg .main-stage {
  position: relative;
  z-index: 1;
  background: transparent;
}

.workbench.has-custom-bg .workspace-grid {
  background: color-mix(in srgb, var(--workbench-chrome-bg) 82%, transparent) !important;
  backdrop-filter: none !important;
  -webkit-backdrop-filter: none !important;
  box-shadow: none;
}

.workbench.has-custom-bg .navigation-shell {
  background: transparent !important;
  backdrop-filter: none !important;
  -webkit-backdrop-filter: none !important;
}

.workbench.has-custom-bg .navigation-pane {
  background: transparent !important;
  backdrop-filter: none !important;
  -webkit-backdrop-filter: none !important;
  border-right: 0 !important;
}

.workbench.has-custom-bg .titlebar {
  position: relative;
  z-index: 1;
  background: color-mix(in srgb, var(--workbench-chrome-bg) 82%, transparent) !important;
  backdrop-filter: none !important;
  -webkit-backdrop-filter: none !important;
  border-bottom: 0 !important;
}

.workbench.has-custom-bg.is-settings .titlebar {
  background: color-mix(in srgb, var(--workbench-chrome-bg) 82%, transparent) !important;
  border-bottom: 0 !important;
}

.workbench.has-custom-bg .conversation-pane {
  background: color-mix(in srgb, var(--peek-list-bg) 40%, transparent) !important;
  backdrop-filter: none !important;
  -webkit-backdrop-filter: none !important;
  border: 1px solid color-mix(in srgb, var(--peek-border) 60%, transparent) !important;
  border-right: 0 !important;
  border-bottom: 0 !important;
  border-radius: 20px 0 0 0 !important;
  box-shadow: none !important;
}

.workbench.has-custom-bg .conversation-pane.extension-open {
  background: color-mix(in srgb, var(--peek-list-bg) 40%, transparent) !important;
}

.workbench.has-custom-bg .review-shell {
  background: color-mix(in srgb, var(--peek-list-bg) 85%, transparent) !important;
  backdrop-filter: none !important;
  -webkit-backdrop-filter: none !important;
  border-top: 1px solid color-mix(in srgb, var(--peek-border) 60%, transparent) !important;
}

.workbench.has-custom-bg .review-header {
  background: transparent !important;
  border-bottom: 1px solid color-mix(in srgb, var(--peek-border) 30%, transparent);
}

.workbench.has-custom-bg .embedded-settings {
  background: color-mix(in srgb, var(--workbench-chrome-bg) 82%, transparent) !important;
  backdrop-filter: none !important;
  -webkit-backdrop-filter: none !important;
}

.workbench.has-custom-bg .composer-wrap :deep(.input-bar) {
  background: color-mix(
    in srgb,
    var(--peek-composer-fill, var(--peek-surface)) 90%,
    transparent
  ) !important;
  backdrop-filter: none !important;
  -webkit-backdrop-filter: none !important;
}

.workbench.has-custom-bg .composer-wrap :deep(.ask-user-list),
.workbench.has-custom-bg .composer-wrap :deep(.path-permission-list),
.workbench.has-custom-bg .composer-wrap :deep(.tool-approval-list) {
  background: color-mix(
    in srgb,
    var(--peek-interaction-fill, var(--peek-composer-fill, var(--peek-surface))) 90%,
    transparent
  ) !important;
}

.workbench.has-custom-bg .conversation-pane.empty-conversation .composer-wrap :deep(.input-bar) {
  background: color-mix(
    in srgb,
    var(--peek-composer-fill, var(--peek-surface)) 92%,
    transparent
  ) !important;
  backdrop-filter: none !important;
  -webkit-backdrop-filter: none !important;
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
.titlebar-navigation {
  width: max(80px, var(--titlebar-navigation-width, var(--nav-shell-width)));
  gap: 4px;
  transition: width 220ms cubic-bezier(0.2, 0.72, 0.25, 1);
}
.workbench.navigation-resizing .titlebar-navigation {
  transition: none;
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
.window-actions {
  display: flex;
  align-items: center;
  gap: 2px;
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
  background: transparent;
}

.navigation-shell {
  position: relative;
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
  padding: 10px 10px 14px;
  background: var(--workbench-navigation-bg);
  color: var(--peek-sidebar-text, var(--peek-text));
  overflow: hidden;
}
.navigation-resize-handle {
  position: absolute;
  inset-block: 0;
  inset-inline-end: 0;
  z-index: 4;
  width: var(--nav-resize-width);
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
.navigation-search-button {
  flex: none;
  color: var(--peek-sidebar-muted, var(--peek-muted));
}
.navigation-search-button:hover {
  color: var(--peek-sidebar-text, var(--peek-text));
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
  gap: 4px;
  margin: 2px 2px 16px;
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
  background: var(--workbench-navigation-selected);
  color: var(--peek-active-fg, var(--peek-sidebar-text, var(--peek-text)));
  font-weight: 600;
}
.nav-shortcut-button > svg {
  flex: none;
  color: var(--peek-sidebar-muted, var(--peek-muted));
}
.sidebar-footer-action {
  flex: none;
}
.nav-status-dot {
  position: absolute;
  right: 4px;
  bottom: 5px;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--peek-success);
  box-shadow: 0 0 0 2px var(--peek-bg, #fff);
}
.nav-shortcut-button:hover > svg,
.nav-shortcut-button.active > svg {
  color: var(--peek-sidebar-text, var(--peek-text));
}
.session-list {
  flex: 1;
  min-width: 0;
  min-height: 0;
  margin-top: 2px;
  padding-top: var(--peek-space-2, 8px);
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
  padding: 0 6px 0 0;
  border: 0;
  border-radius: var(--peek-radius-sm);
  background: transparent;
  color: color-mix(in srgb, var(--peek-muted) 60%, var(--peek-faint));
  cursor: pointer;
  font-size: var(--peek-font-xs, 12px);
  font-weight: 400;
  letter-spacing: 0;
  text-transform: none;
  text-align: left;
  transition: color var(--motion-fast, 110ms) ease;
}
.navigation-section-toggle:hover {
  color: var(--peek-muted);
}
.navigation-section-toggle:focus-visible {
  outline: 2px solid var(--peek-focus);
  outline-offset: -2px;
}
.navigation-section-toggle > svg {
  flex: none;
  color: var(--peek-faint);
  opacity: 0.9;
  transition: transform 140ms ease;
}
.navigation-section-toggle > svg.expanded {
  transform: rotate(90deg);
}
.navigation-section-toggle span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.section-action,
.workspace-actions button {
  display: inline-grid;
  place-items: center;
  padding: 0;
  border: 0;
  border-radius: var(--peek-radius-sm);
  background: transparent;
  color: var(--peek-sidebar-muted, var(--peek-muted));
  cursor: pointer;
}
.section-action {
  width: var(--peek-control-icon);
  height: var(--peek-control-icon);
}
.section-action:hover,
.workspace-actions button:hover {
  color: var(--peek-sidebar-text, var(--peek-text));
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
.workspace-row:is(:hover, :focus-within),
.workspace-menu-open .workspace-row {
  background: var(--peek-row-hover);
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
  color: var(--peek-sidebar-text, var(--peek-text));
  cursor: inherit;
  font-size: var(--peek-font-sm);
  text-align: left;
}
.workspace-group-header > svg {
  flex: none;
  color: color-mix(in srgb, var(--peek-accent) 55%, var(--peek-sidebar-muted, var(--peek-muted)));
}
.workspace-group-header span {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  font-weight: 450;
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
  --chat-column-max: 820px;
  --chat-column-gutter: 48px;
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
  border-radius: var(--peek-pane-radius, 20px) 0 0 0;
  background: var(--peek-list-bg);
  box-shadow: var(--peek-pane-shadow, none);
  container-type: size;
  container-name: conversation;
}
.conversation-pane.extension-open {
  background: var(--peek-list-bg);
}

/* Keep the existing geometry; separate the light composer from its canvas. */
.workbench[data-theme="light"]:not(.has-custom-bg) .conversation-pane:not(.extension-open) {
  background: #ffffff;
}

.workbench[data-theme="light"] .composer-wrap {
  --peek-placeholder: #989d9f;
  --peek-composer-fill: #fffeff;
}

.titlebar-conversation {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  align-self: stretch;
  min-width: 0;
  gap: 6px;
  padding-inline: 16px 8px;
}
.titlebar-message-mode {
  justify-self: start;
}
.titlebar-conversation-center {
  min-width: 0;
  align-self: stretch;
  display: grid;
  place-items: center;
}
.titlebar-title-group {
  display: flex;
  align-items: center;
  gap: 5px;
  max-width: 100%;
  min-width: 0;
  padding: 3px 10px 3px 5px;
  border-radius: var(--titlebar-control-radius);
  background: var(--titlebar-control-bg);
}
.titlebar-title-group .titlebar-mascot {
  flex: none;
  width: 22px;
  height: 22px;
  pointer-events: none;
}
.titlebar-conversation-actions {
  display: flex;
  gap: 2px;
}
.titlebar-conversation-actions .review-toggle {
  border-radius: var(--titlebar-control-radius);
  background: var(--titlebar-control-bg);
}
.titlebar-conversation-actions .review-toggle:hover {
  background: var(--peek-hover-bg);
}
.conversation-title {
  max-width: 100%;
  min-width: 0;
  margin: 0;
  overflow: hidden;
  color: var(--peek-header-text, var(--peek-text));
  font-size: 13px;
  font-weight: 650;
  line-height: 1.35;
  text-align: center;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.conversation-pane.page-style-chat .workbench-messages :deep(.message-item.user) {
  justify-content: flex-end;
  padding-right: 18px;
}
.conversation-pane.page-style-chat .workbench-messages :deep(.user-turn) {
  width: auto;
  max-width: min(78%, 560px);
}
.conversation-pane.page-style-chat .workbench-messages :deep(.user-turn.is-editing) {
  width: min(100%, 640px);
  min-width: min(100%, 420px);
  max-width: min(100%, 640px);
}
.conversation-pane.page-style-chat
  .workbench-messages
  :deep(.user-turn.is-editing .user-message-footer),
.conversation-pane.page-style-chat
  .workbench-messages
  :deep(.user-turn.is-editing .input-footer-primary) {
  flex-wrap: nowrap;
}
.conversation-pane.page-style-chat .workbench-messages :deep(.assistant-bubble) {
  width: 100%;
  max-width: none;
  margin-left: 0;
  padding: 0;
  border: 0;
  border-radius: 0;
  background: transparent;
  box-shadow: none;
}
.conversation-pane.page-style-chat
  .workbench-messages
  :deep(.assistant-bubble:has(.generated-image-card)),
.conversation-pane.page-style-chat .workbench-messages :deep(.assistant-bubble:has(.agent-work)),
.conversation-pane.page-style-chat
  .workbench-messages
  :deep(.assistant-bubble:has(.provider-setup-card)) {
  max-width: none;
}
.conversation-pane.page-style-chat .workbench-messages :deep(.chat-turn-head),
.conversation-pane.page-style-stream .workbench-messages :deep(.chat-turn-head) {
  position: static;
  background: transparent;
}
.conversation-pane.page-style-chat .workbench-messages :deep(.chat-turn-head.is-stuck::before),
.conversation-pane.page-style-stream .workbench-messages :deep(.chat-turn-head.is-stuck::before) {
  content: none;
}
.conversation-pane.page-style-stream {
  --chat-column-max: 100cqw;
}
.conversation-pane.page-style-stream .thread-input-row {
  flex: none;
  min-height: 40px;
  margin: 0;
  padding-bottom: 8px;
}
.conversation-pane.page-style-stream .thread-input-row :deep(.inline-composer-body) {
  padding-left: 40px;
}
.conversation-pane.page-style-stream .thread-input-row :deep(.input-user-avatar) {
  left: 0;
}
.conversation-pane.page-style-stream .composer-wrap {
  gap: 0;
}
.conversation-pane.page-style-stream .composer-wrap :deep(.input-bar),
.conversation-pane.page-style-stream .composer-wrap :deep(.input-bar:focus-within) {
  border: none;
  border-radius: 0;
  background: var(--peek-list-bg);
  box-shadow: none;
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
.extension-scroll:has(.plugin-main-view) {
  overflow: hidden;
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
.extension-dialog-overlay,
.extension-dialog {
  position: fixed;
}
.extension-dialog-overlay {
  inset: 0;
  z-index: 44;
  background: rgb(15 16 20 / 28%);
}
.extension-dialog {
  z-index: 45;
  inset: 50% auto auto 50%;
  transform: translate(-50%, -50%);
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  width: min(960px, calc(100vw - 32px));
  height: min(700px, calc(100dvh - 32px));
  overflow: hidden;
  border: 1px solid var(--peek-border);
  border-radius: 18px;
  background: var(--peek-list-bg);
  box-shadow: 0 20px 64px rgb(0 0 0 / 22%);
  color: var(--peek-text);
}
.extension-dialog-phone {
  width: min(500px, calc(100vw - 32px));
  height: auto;
  max-height: calc(100dvh - 32px);
}
.extension-dialog-header {
  display: flex;
  flex: none;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 16px 12px 22px;
  border-bottom: 1px solid color-mix(in srgb, var(--peek-border) 65%, transparent);
}
.extension-dialog-title {
  margin: 0;
  font-size: 16px;
  font-weight: 650;
}
.extension-dialog-header .icon-button {
  flex: none;
  width: 32px;
  height: 32px;
}
.extension-dialog-body {
  flex: 1 1 auto;
  min-height: 0;
  overflow: auto;
  overscroll-behavior: contain;
}
.extension-dialog-body-tabs {
  display: flex;
  overflow: hidden;
}
.plugin-main-view {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  width: 100%;
  height: 100%;
}
.plugin-main-view :deep(.plugin-slot-outlet),
.plugin-main-view :deep(.plugin-host-pane) {
  flex: 1;
  min-height: 0;
}
.conversation-stack {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  min-width: 0;
}
.conversation-stack.is-embedded {
  width: 100%;
  height: 100%;
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
  --code-block-sticky-top: -18px;
  --chat-sticky-top: -16px;
  padding: 18px 0 var(--composer-list-clearance)
    max(calc(var(--chat-column-gutter) / 2), calc((100cqw - var(--chat-column-max)) / 2));
  gap: 20px;
  scroll-behavior: auto;
}
.workbench-messages :deep(.message-list-content) {
  width: min(calc(100cqw - var(--chat-column-gutter)), var(--chat-column-max));
  max-width: 100%;
}
.workbench-messages :deep(.chat-turn.is-first-turn) {
  --chat-sticky-top: 0px;
}
.workbench-messages :deep(.message-preview-rail) {
  right: 8px;
  bottom: var(--composer-fade-height);
}
.workbench-messages :deep(.scroll-to-bottom) {
  bottom: calc(var(--composer-fade-height) + 10px);
}
.workbench-messages :deep(.assistant-bubble) {
  width: calc(100% - 2 * var(--chat-reply-inset, var(--peek-radius-composer, 16px)));
  max-width: 100%;
  margin-left: var(--chat-reply-inset, var(--peek-radius-composer, 16px));
}
.workbench-messages :deep(.user-turn) {
  max-width: 100%;
}
.composer-wrap {
  position: absolute;
  z-index: 8;
  left: 50%;
  top: auto;
  bottom: var(--composer-dock-gap);
  width: min(calc(100% - var(--chat-column-gutter)), var(--chat-column-max));
  min-width: 0;
  min-height: 0;
  max-height: min(320px, calc(100% - 16px));
  margin: 0;
  transform: translateX(-50%);
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
  transition:
    min-height 280ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1)),
    padding 280ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1)),
    border-radius 280ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1)),
    border-color 280ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1)),
    background 280ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1)),
    box-shadow 280ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1));
}
.composer-wrap :deep(.input-content),
.composer-wrap :deep(.composer-textarea) {
  /* Do not animate min-height/line-height while typing — that reads as jitter. */
  transition:
    font-size 160ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1)),
    border-radius 160ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1)),
    letter-spacing 160ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1));
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
  box-shadow: 0 -2px 8px color-mix(in srgb, #000 6%, transparent);
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
.empty-conversation-hero.is-guided {
  top: 8px;
  bottom: auto;
  height: calc(50% - 80px);
  overflow: auto;
  justify-content: safe flex-end;
  pointer-events: auto;
  user-select: auto;
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
  width: 168px;
  height: 124px;
  flex: none;
  /* The hero itself is click-through; the mascot alone reacts to clicks. */
  pointer-events: auto;
}
.empty-conversation-brand :deep(.mascot-face) {
  opacity: 1;
}
.empty-conversation-prompt {
  margin: 0;
  max-width: 28em;
  color: var(--peek-conversation-text, var(--peek-text));
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
  bottom: auto;
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

.conversation-pane.empty-conversation .composer-wrap {
  --workbench-composer-min-height: 108px;
}

/* Ask / permission panels should merge into the composer, not stack as a second card. */
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

.conversation-pane.empty-conversation .composer-wrap :deep(.composer-textarea) {
  min-height: 24px;
  font-size: 15px;
  line-height: 24px;
  letter-spacing: -0.01em;
}
.conversation-pane.empty-conversation .composer-wrap :deep(.composer-textarea::placeholder) {
  color: var(--peek-composer-placeholder, var(--peek-placeholder));
  letter-spacing: 0;
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
  box-sizing: border-box;
  min-width: 0;
  min-height: 0;
  width: var(--review-pane-width, 527px);
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
.review-tabs :deep([data-slot="tooltip"]) {
  display: contents;
}
.review-tabs button {
  height: var(--peek-control-icon);
  width: var(--peek-control-icon);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0;
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
.plugin-review-host {
  flex: 1;
  min-width: 0;
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
  .workbench:has(.workspace-grid.review-open) {
    --titlebar-navigation-width: 0px;
  }
}

@media (max-height: 700px) {
  .conversation-pane {
    --composer-dock-gap: 8px;
    --composer-fade-height: 96px;
    --composer-list-clearance: 108px;
    --chat-column-gutter: 28px;
  }
  .composer-wrap {
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
  .workbench:has(.workspace-grid.review-open) .titlebar-compact {
    --titlebar-navigation-width: 0px;
  }
}

@container workbench (max-height: 560px) {
  .conversation-pane {
    --composer-dock-gap: 8px;
    --composer-fade-height: 96px;
    --composer-list-clearance: 108px;
    --chat-column-gutter: 20px;
  }
  .composer-wrap {
    max-height: min(46cqh, calc(100% - 12px));
  }
}

@container conversation (max-width: 560px) {
  .conversation-pane {
    --chat-column-gutter: 16px;
  }
  .workbench-messages :deep(.message-list) {
    gap: 16px;
  }
  .composer-wrap {
    max-height: min(360px, calc(100% - 12px));
  }
  .composer-wrap :deep(.workbench-composer .input-footer) {
    flex-wrap: wrap;
    row-gap: 6px;
    align-items: center;
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
}

@container conversation (max-width: 400px) {
  .conversation-pane {
    --chat-column-gutter: 20px;
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

/* Cohesive workbench pass. These rules keep the existing interaction model
   while giving every surface the same spacing, elevation and state language. */
.workbench {
  --nav-col: 272px;
  --titlebar-h: 38px;
  --chat-column-gutter: 48px;
  --peek-radius-sm: var(--ui-radius-sm);
  --peek-radius-md: var(--ui-radius-md);
  --peek-radius-lg: var(--ui-radius-lg);
  --peek-radius-composer: 18px;
}

.titlebar-compact {
  background: linear-gradient(
    to right,
    var(--workbench-navigation-bg) var(--titlebar-navigation-width, var(--nav-shell-width)),
    var(--peek-list-bg) var(--titlebar-navigation-width, var(--nav-shell-width))
  );
}

.workbench.is-settings .titlebar-compact {
  --titlebar-navigation-width: min(var(--settings-nav-width), 46cqw);
}

.workbench.is-settings .titlebar-context > span {
  padding: 5px 12px;
  font-size: 13px;
  font-weight: 650;
}

:global(html:has(.workbench.has-mica)),
:global(body:has(.workbench.has-mica)),
:global(#app:has(.workbench.has-mica)),
.workbench.has-mica,
.workbench.has-mica .main-stage,
.workbench.has-mica .workspace-grid,
.workbench.has-mica .navigation-shell,
.workbench.has-mica .navigation-pane {
  background: transparent;
}

.workbench.has-mica {
  --workbench-navigation-bg: transparent;
}

.workbench.has-mica .titlebar:not(.titlebar-compact) {
  background: transparent;
}

.workbench.chat-under-titlebar {
  --chat-titlebar-overlay-height: calc(var(--titlebar-h) + var(--titlebar-top-inset));
}

.workbench.chat-under-titlebar .main-stage {
  margin-top: calc(-1 * var(--chat-titlebar-overlay-height));
}

.workbench.chat-under-titlebar :is(.navigation-shell, .review-shell) {
  padding-top: var(--chat-titlebar-overlay-height);
}

.workbench.chat-under-titlebar .navigation-shell {
  background: var(--workbench-navigation-bg);
}

.workbench.chat-under-titlebar .navigation-pane {
  background: transparent;
}

.workbench.chat-under-titlebar .titlebar-compact {
  position: relative;
  z-index: 10;
  background: transparent !important;
}

.workbench.chat-under-titlebar .titlebar-conversation {
  --titlebar-control-bg: color-mix(in srgb, var(--peek-list-bg) 88%, var(--peek-text));
}

.workbench.chat-under-titlebar .conversation-pane .workbench-messages {
  padding-top: 0;
}

.workbench.chat-under-titlebar .conversation-pane .workbench-messages :deep(.message-list) {
  padding-top: var(--chat-titlebar-overlay-height);
  scroll-padding-top: var(--chat-titlebar-overlay-height);
}

.workbench.chat-under-titlebar
  .conversation-pane
  .workbench-messages
  :deep(.conversation-find-bar) {
  top: calc(var(--chat-titlebar-overlay-height) + 8px);
}

.window-actions {
  gap: 0;
}

.titlebar-compact .titlebar-trailing {
  margin-top: calc(-1 * var(--titlebar-top-inset));
}

.window-button {
  width: 42px;
  height: calc(var(--titlebar-h) + var(--titlebar-top-inset));
  padding-top: var(--titlebar-top-inset);
}

.main-stage {
  background: var(--peek-bg);
}

.workspace-grid {
  background: var(--peek-bg);
}

.navigation-shell {
  width: var(--nav-shell-width, 272px);
}

.nav-shortcut-button {
  height: 36px;
  padding-inline: 12px;
  border-radius: var(--ui-radius-sm);
  font-size: 13px;
}

.navigation-menu-items {
  gap: 3px;
  margin-bottom: 18px;
}

.session-list {
  padding-top: 4px;
}

.navigation-section {
  margin-bottom: 14px;
}

.navigation-section-header {
  padding-inline: 4px;
}

.conversation-pane {
  border: 0;
  border-radius: 0;
  background: var(--peek-list-bg);
  box-shadow: none;
}

.workbench.has-custom-bg .conversation-pane {
  border: 0 !important;
  border-radius: 0 !important;
}

.conversation-pane.page-style-chat {
  --chat-column-max: 1040px;
}

.workbench-messages :deep(.message-list) {
  gap: 26px;
  padding-top: 24px;
}

.workbench-messages :deep(.chat-turn) {
  animation: chat-turn-enter 180ms var(--motion-ease-out) both;
}

.workbench-messages :deep(.assistant-bubble) {
  margin-left: 0;
  padding: 2px 0;
  border: 0;
  background: transparent;
  box-shadow: none;
}

.workbench-messages :deep(.page-style-work .assistant-bubble) {
  margin-inline: var(--chat-reply-inset, var(--peek-radius-composer, 16px));
}

.workbench-messages :deep(.user-turn) {
  max-width: min(78%, 620px);
}

.workbench-messages :deep(.page-style-work .chat-turn-head .user-turn),
.conversation-pane.page-style-stream .workbench-messages :deep(.user-turn) {
  max-width: 100%;
}

.composer-wrap {
  --peek-radius-composer: 22px;
  --peek-composer-border-focus: color-mix(
    in srgb,
    var(--peek-composer-border) 92%,
    var(--peek-text)
  );
  --peek-composer-shadow: 0 3px 12px color-mix(in srgb, var(--peek-shadow) 18%, transparent);
  --peek-composer-shadow-focus: var(--peek-composer-shadow);
}

.review-shell {
  border: 0;
  border-inline-start: 1px solid var(--ui-border-subtle);
}

@keyframes chat-turn-enter {
  from {
    opacity: 0;
    transform: translateY(4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@media (prefers-reduced-motion: reduce) {
  .workbench-messages :deep(.chat-turn) {
    animation: none;
  }
}
</style>
