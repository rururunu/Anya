import type { AppLanguage } from "@/types/setting";

export const chatInputEn = {
  "chatInput.workspacePanelTitle": "Workspace",
  "chatInput.newWorkspace": "New Workspace",
  "chatInput.noPreviousWorkspaces": "No previous workspaces",
  "chatInput.exitWorkspace": "Exit current workspace",
  "chatInput.attachResources": "Add skills, MCP, or files",
  "chatInput.attachPanelTitle": "Add to message",
  "chatInput.attachFiles": "Files",
  "chatInput.attachPickFiles": "Choose files…",
  "chatInput.attachFilesLoading": "Loading workspace files…",
  "chatInput.attachNoWorkspace": "No workspace selected. Choose a workspace or pick files below.",
  "chatInput.attachEmptyFiles": "This workspace has no files to show.",
  "chatInput.attachInsertFile": "Click to insert file",
  "chatInput.attachInsertFolder": "Long-press to insert folder",
  "chatInput.attachSkills": "Skills",
  "chatInput.attachMcp": "MCP",
  "chatInput.attachEmptySkills":
    "No skills available. Enable built-ins or install from the sidebar.",
  "chatInput.attachEmptyMcp":
    "No enabled MCP servers. Install one from the sidebar MCP page, then turn it on.",
  "chatInput.attachLoading": "Loading…",
  "chatInput.attachExpandMore": "+{count} more",
  "chatInput.attachCollapse": "Show less",
} as const;

export type ChatInputI18nKey = keyof typeof chatInputEn;

export type ChatInputLocalePartial = Partial<Record<ChatInputI18nKey, string>>;

export const chatInputLocales: Record<AppLanguage, ChatInputLocalePartial> = {
  "en-US": chatInputEn,
  "zh-CN": {
    "chatInput.workspacePanelTitle": "工作区",
    "chatInput.newWorkspace": "新建工作区",
    "chatInput.noPreviousWorkspaces": "暂无历史工作区",
    "chatInput.exitWorkspace": "退出当前工作区",
    "chatInput.attachResources": "添加 Skill、MCP 或文件",
    "chatInput.attachPanelTitle": "添加到消息",
    "chatInput.attachPickFiles": "选择文件…",
    "chatInput.attachFiles": "文件",
    "chatInput.attachFilesLoading": "正在读取工作区文件…",
    "chatInput.attachNoWorkspace": "未选择工作区。请先选择工作区，或使用下方选择文件。",
    "chatInput.attachEmptyFiles": "当前工作区没有可展示的文件。",
    "chatInput.attachInsertFile": "点击引入文件",
    "chatInput.attachInsertFolder": "长按引入文件夹",
    "chatInput.attachSkills": "Skills",
    "chatInput.attachMcp": "MCP",
    "chatInput.attachEmptySkills": "暂无可用 Skill。可在侧栏技能页启用内置或安装。",
    "chatInput.attachEmptyMcp": "暂无已启用的 MCP。请到侧栏 MCP 页安装后打开开关。",
    "chatInput.attachLoading": "加载中…",
    "chatInput.attachExpandMore": "展开 +{count}",
    "chatInput.attachCollapse": "收起",
  },
  "ja-JP": {},
  "ru-RU": {},
  "de-DE": {},
  "fr-FR": {},
  "ko-KR": {},
};
