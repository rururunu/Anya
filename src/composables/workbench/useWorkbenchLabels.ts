import { computed, type Ref } from "vue";

import type { Workspace } from "@/commands/workspace";
import type { AppLanguage } from "@/types/setting";

export interface UseWorkbenchLabelsOptions {
  language: Ref<AppLanguage>;
  workspaces: Ref<Workspace[]>;
  activeSessionWorkspaceId: Ref<string | null>;
}

/**
 * Localized copy for the workbench chrome (titlebar, navigation, composer,
 * review tabs, shortcut help). Centralized here so every workbench composable
 * shares the same `isChinese` flag instead of re-deriving it.
 */
export function useWorkbenchLabels(options: UseWorkbenchLabelsOptions) {
  const { language, workspaces, activeSessionWorkspaceId } = options;

  const isChinese = computed(() => language.value === "zh-CN");

  const labels = computed(() =>
    isChinese.value
      ? {
          toggleNavigation: "切换会话栏",
          views: "工作区视图",
          settings: "设置",
          backToChat: "返回",
          minimize: "最小化",
          close: "关闭",
          newChat: "新对话",
          conversations: "对话",
          refresh: "刷新会话",
          untitled: "新对话",
          archiveConversation: "归档对话",
          deleteConversation: "删除对话",
          noConversations: "还没有对话",
          closePanel: "关闭审阅区",
          deleteConfirm: "确定删除这个对话吗？此操作无法撤销。",
          guideOneHint: "立即发送这条暂存消息给当前执行中的 AI 作为引导",
          editStaged: "编辑这条暂存消息",
          removeStaged: "删除这条暂存消息",
          stagedAutoHint: "本轮结束后自动发送",
          diff: "差异",
          agents: "子 Agent",
          runtime: "运行时",
        }
      : {
          toggleNavigation: "Toggle conversations",
          views: "Workspace views",
          settings: "Settings",
          backToChat: "Back",
          minimize: "Minimize",
          close: "Close",
          newChat: "New chat",
          conversations: "Conversations",
          refresh: "Refresh conversations",
          untitled: "New conversation",
          archiveConversation: "Archive conversation",
          deleteConversation: "Delete conversation",
          noConversations: "No conversations yet",
          closePanel: "Close review pane",
          deleteConfirm: "Delete this conversation? This cannot be undone.",
          guideOneHint: "Send this staged message to the running AI as guidance now",
          editStaged: "Edit this staged message",
          removeStaged: "Remove this staged message",
          stagedAutoHint: "Sent automatically when this turn finishes",
          diff: "Diff",
          agents: "Sub-agents",
          runtime: "Runtime",
        },
  );

  const navigationLabels = computed(() =>
    isChinese.value
      ? {
          pinned: "\u7f6e\u9876",
          workspaces: "\u5de5\u4f5c\u533a",
          quickAsk: "\u968f\u95ee",
          skills: "\u6280\u80fd",
          mcp: "MCP",
          connectPhone: "\u8fde\u63a5\u624b\u673a",
          extensions: "\u6269\u5c55",
          addWorkspace: "\u6dfb\u52a0\u5de5\u4f5c\u533a",
          pinWorkspace: "\u7f6e\u9876\u5de5\u4f5c\u533a",
          unpinWorkspace: "\u53d6\u6d88\u7f6e\u9876",
          editWorkspace: "\u7f16\u8f91\u5de5\u4f5c\u533a",
          newWorkspaceChat: "\u5728\u5de5\u4f5c\u533a\u65b0\u5efa\u4f1a\u8bdd",
          expandWorkspace: "\u5c55\u5f00\u5de5\u4f5c\u533a",
          collapseWorkspace: "\u6298\u53e0\u5de5\u4f5c\u533a",
          newQuickAsk: "\u65b0\u5efa\u968f\u95ee\u4f1a\u8bdd",
          more: "\u66f4\u591a\u9009\u9879",
          openFolder: "\u5728\u8d44\u6e90\u7ba1\u7406\u5668\u4e2d\u6253\u5f00",
          openInTerminal: "\u5728\u7ec8\u7aef\u4e2d\u6253\u5f00",
          archiveWorkspace: "\u5f52\u6863\u5de5\u4f5c\u533a",
          deleteWorkspace: "\u5220\u9664\u5de5\u4f5c\u533a",
          deleteWorkspaceConfirm:
            "\u5220\u9664\u8fd9\u4e2a\u5de5\u4f5c\u533a\uff1f\u5bf9\u8bdd\u4e0d\u4f1a\u88ab\u5220\u9664\u3002",
          cancel: "\u53d6\u6d88",
          continue: "\u7ee7\u7eed",
          confirmDelete: "\u5220\u9664",
          moveConversationTitle: "\u5c06\u5bf9\u8bdd\u79fb\u5230\u5de5\u4f5c\u533a\uff1f",
          moveConversationDescription:
            "\u6b64\u5bf9\u8bdd\u5c06\u83b7\u5f97\u8be5\u5de5\u4f5c\u533a\u7684\u8bbf\u95ee\u6743\u9650\uff1a",
        }
      : {
          pinned: "Pinned",
          workspaces: "Workspaces",
          quickAsk: "Quick Ask",
          skills: "Skills",
          mcp: "MCP",
          connectPhone: "Connect phone",
          extensions: "Extensions",
          addWorkspace: "Add workspace",
          pinWorkspace: "Pin workspace",
          unpinWorkspace: "Unpin workspace",
          editWorkspace: "Edit workspace",
          newWorkspaceChat: "New chat in workspace",
          expandWorkspace: "Expand workspace",
          collapseWorkspace: "Collapse workspace",
          newQuickAsk: "New quick ask",
          more: "More options",
          openFolder: "Open in File Explorer",
          openInTerminal: "Open in terminal",
          archiveWorkspace: "Archive workspace",
          deleteWorkspace: "Delete workspace",
          deleteWorkspaceConfirm: "Delete this workspace? Conversations will be kept.",
          cancel: "Cancel",
          continue: "Continue",
          confirmDelete: "Delete",
          moveConversationTitle: "Move conversation to this workspace?",
          moveConversationDescription: "This conversation will get access to this workspace:",
        },
  );

  const searchLabels = computed(() =>
    isChinese.value
      ? {
          open: "搜索聊天",
        }
      : {
          open: "Search chats",
        },
  );

  const shortcutMod = computed(() =>
    typeof navigator !== "undefined" && /mac|iphone|ipad|ipod/i.test(navigator.platform)
      ? "⌘"
      : "Ctrl",
  );

  const shortcutLabels = computed(() =>
    isChinese.value
      ? {
          helpTitle: "工作台快捷键",
        }
      : {
          helpTitle: "Workbench shortcuts",
        },
  );

  const shortcutHelpItems = computed(() => {
    const mod = shortcutMod.value;
    return isChinese.value
      ? [
          { keys: `${mod} + F`, label: "在对话中查找" },
          { keys: `${mod} + Shift + F`, label: "打开搜索" },
          { keys: `${mod} + K`, label: "打开搜索" },
          { keys: `${mod} + N`, label: "新建对话" },
          { keys: `${mod} + B`, label: "显示/隐藏左侧栏" },
          { keys: `${mod} + R`, label: "打开/关闭审阅面板" },
          { keys: `${mod} + ,`, label: "打开设置" },
          { keys: `${mod} + L`, label: "聚焦输入框" },
          { keys: `${mod} + /`, label: "显示此帮助" },
        ]
      : [
          { keys: `${mod} + F`, label: "Find in conversation" },
          { keys: `${mod} + Shift + F`, label: "Open search" },
          { keys: `${mod} + K`, label: "Open search" },
          { keys: `${mod} + N`, label: "New chat" },
          { keys: `${mod} + B`, label: "Toggle left sidebar" },
          { keys: `${mod} + R`, label: "Toggle review pane" },
          { keys: `${mod} + ,`, label: "Open settings" },
          { keys: `${mod} + L`, label: "Focus composer" },
          { keys: `${mod} + /`, label: "Show this help" },
        ];
  });

  const activeWorkspaceName = computed(
    () =>
      workspaces.value.find((workspace) => workspace.id === activeSessionWorkspaceId.value)?.name,
  );
  const emptyConversationPrompt = computed(() => {
    if (!activeWorkspaceName.value) {
      return isChinese.value ? "我能为您做什么？" : "What can I do for you?";
    }
    return isChinese.value
      ? `需要我在 ${activeWorkspaceName.value} 中帮助您完成什么？`
      : `What would you like me to help you accomplish in ${activeWorkspaceName.value}?`;
  });

  const tutorialButtonLabel = computed(() => (isChinese.value ? "查看教程" : "Tutorial"));

  return {
    isChinese,
    labels,
    navigationLabels,
    searchLabels,
    shortcutMod,
    shortcutLabels,
    shortcutHelpItems,
    emptyConversationPrompt,
    tutorialButtonLabel,
  };
}

export type WorkbenchLabels = ReturnType<typeof useWorkbenchLabels>;
