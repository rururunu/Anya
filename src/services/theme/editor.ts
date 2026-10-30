import type { CustomThemeConfig } from "@/types/setting";
import { harmonizeCustomTheme } from "./ThemeService";

export type ThemeField = {
  token: string;
  zh: string;
  en: string;
  property: string;
  fallback?: string;
};
export type ThemeRegion = { id: string; zh: string; en: string; fields: ThemeField[] };
const field = (
  token: string,
  zh: string,
  en: string,
  property = "color",
  fallback?: string,
): ThemeField => ({ token: `--${token}`, zh, en, property, fallback });
const appearance = (prefix: string): ThemeField[] => [
  field(`${prefix}-radius`, "圆角", "Radius", "border-radius", "12px"),
  field(`${prefix}-shadow`, "阴影", "Shadow", "box-shadow", "none"),
];
export const themeRegions: ThemeRegion[] = [
  {
    id: "global",
    zh: "整体",
    en: "Global",
    fields: [
      field("peek-bg", "窗口背景", "Window background"),
      field("peek-text", "正文", "Text"),
      field("peek-muted", "次要文字", "Muted text"),
      field("peek-faint", "弱化文字", "Faint text"),
      field("peek-placeholder", "占位文字", "Placeholder text"),
      field("peek-border", "分割线", "Borders"),
      field("peek-accent", "强调色", "Accent"),
      field("peek-accent-fg", "强调色上的文字", "Accent text"),
      field("peek-danger", "危险/完全访问色", "Danger / Full access"),
      field("peek-warning", "警告色", "Warning"),
      field("peek-success", "成功色", "Success"),
      field("peek-info", "信息色", "Info"),
      field(
        "peek-font-sans",
        "界面字体",
        "UI font family",
        "font-family",
        "var(--settings-font-sans)",
      ),
      field(
        "peek-font-mono",
        "代码字体",
        "Code font family",
        "font-family",
        "var(--settings-font-mono)",
      ),
      field("peek-font-md", "界面字号", "UI font size", "font-size", "14px"),
      field("peek-agent-font-size", "回复字号", "Reply font size", "font-size", "13px"),
      field("peek-radius-sm", "小圆角", "Small radius", "border-radius", "6px"),
      field("peek-radius-md", "中圆角", "Medium radius", "border-radius", "8px"),
      field("peek-radius-lg", "大圆角", "Large radius", "border-radius", "12px"),
      field("peek-shadow", "阴影基础色", "Shadow color"),
      field("peek-elev-sm", "浅层阴影", "Small elevation shadow", "box-shadow", "none"),
      field("peek-elev-md", "浮层阴影", "Medium elevation shadow", "box-shadow", "none"),
      field("peek-space-4", "消息间距", "Message spacing", "gap", "16px"),
    ],
  },
  {
    id: "sidebar",
    zh: "侧栏",
    en: "Sidebar",
    fields: [
      field("peek-sidebar", "背景", "Background"),
      field("peek-sidebar-text", "文字", "Text", "color", "var(--peek-text)"),
      field("peek-sidebar-muted", "次要文字", "Muted text", "color", "var(--peek-muted)"),
      field("peek-active-fg", "选中项文字", "Active item text", "color", "var(--peek-text)"),
    ],
  },
  {
    id: "header",
    zh: "顶栏",
    en: "Header",
    fields: [
      field("peek-header-bg", "背景", "Background", "color", "var(--peek-list-bg)"),
      field("peek-header-text", "标题文字", "Title text", "color", "var(--peek-text)"),
    ],
  },
  {
    id: "conversation",
    zh: "对话区",
    en: "Conversation",
    fields: [
      field("peek-list-bg", "背景", "Background"),
      field(
        "peek-conversation-text",
        "提示与标题文字",
        "Heading text",
        "color",
        "var(--peek-text)",
      ),
      field("peek-pane-radius", "面板圆角", "Pane radius", "border-radius", "20px"),
      field("peek-pane-shadow", "面板阴影", "Pane shadow", "box-shadow", "none"),
    ],
  },
  {
    id: "user",
    zh: "用户消息",
    en: "User message",
    fields: [
      field("peek-user-bubble-bg", "背景", "Background"),
      field("peek-user-bubble-text", "文字", "Text"),
      field("peek-user-bubble-border", "边框", "Border"),
      field(
        "peek-user-bubble-border-focus",
        "编辑边框",
        "Editing border",
        "color",
        "var(--peek-user-bubble-border)",
      ),
      ...appearance("peek-user-bubble"),
      field("peek-user-bubble-padding", "内边距", "Padding", "padding", "8px 36px 8px 12px"),
    ],
  },
  {
    id: "assistant",
    zh: "助手消息",
    en: "Assistant message",
    fields: [
      field("peek-assistant-bg", "背景", "Background", "color", "transparent"),
      field("peek-assistant-text", "文字", "Text", "color", "var(--peek-text)"),
      field("peek-assistant-border", "边框", "Border", "color", "transparent"),
      ...appearance("peek-assistant"),
      field("peek-assistant-padding", "内边距", "Padding", "padding", "0px"),
    ],
  },
  {
    id: "composer",
    zh: "输入框",
    en: "Composer",
    fields: [
      field("peek-composer-fill", "背景", "Background"),
      field("peek-composer-text", "文字", "Text"),
      field(
        "peek-composer-placeholder",
        "占位文字",
        "Placeholder text",
        "color",
        "var(--peek-placeholder)",
      ),
      field("peek-composer-border", "边框", "Border"),
      field("peek-composer-border-focus", "聚焦边框", "Focus border"),
      field("peek-radius-composer", "圆角", "Radius", "border-radius", "16px"),
      field("peek-composer-shadow", "阴影", "Shadow", "box-shadow", "none"),
      field(
        "peek-composer-shadow-focus",
        "聚焦阴影",
        "Focus shadow",
        "box-shadow",
        "var(--peek-composer-shadow)",
      ),
      field(
        "peek-composer-chip-text",
        "底部栏按钮文字",
        "Footer chip text",
        "color",
        "var(--peek-muted)",
      ),
      field(
        "peek-composer-chip-hover-text",
        "底部栏悬停文字",
        "Footer chip hover text",
        "color",
        "var(--peek-text)",
      ),
      field(
        "peek-composer-chip-bg",
        "底部栏按钮背景",
        "Footer chip background",
        "color",
        "transparent",
      ),
      field(
        "peek-composer-chip-radius",
        "底部栏按钮圆角",
        "Footer chip radius",
        "border-radius",
        "8px",
      ),
      field(
        "peek-composer-chip-shadow",
        "底部栏按钮阴影",
        "Footer chip shadow",
        "box-shadow",
        "none",
      ),
      field(
        "peek-composer-full-access-text",
        "完全访问状态文字",
        "Full access text",
        "color",
        "var(--peek-danger)",
      ),
      field(
        "peek-composer-meta-text",
        "Token 统计文字",
        "Token stats text",
        "color",
        "var(--peek-faint)",
      ),
    ],
  },
  {
    id: "cards",
    zh: "卡片与交互",
    en: "Cards & interactions",
    fields: [
      field("peek-surface", "面板背景", "Panel background"),
      field("peek-interaction-fill", "交互卡片背景", "Interaction background"),
      field("peek-input-bg", "表单背景", "Field background"),
      field("peek-card-text", "卡片文字", "Card text", "color", "var(--peek-text)"),
      field("peek-card-border", "卡片边框", "Card border", "color", "var(--peek-border)"),
      field("peek-card-radius", "卡片圆角", "Card radius", "border-radius", "12px"),
      field("peek-card-shadow", "卡片阴影", "Card shadow", "box-shadow", "none"),
    ],
  },
  {
    id: "code",
    zh: "代码块",
    en: "Code",
    fields: [
      field("peek-code-body-bg", "代码背景", "Code background"),
      field("peek-code-fg", "文字", "Text"),
      field("peek-code-toolbar-bg", "工具栏", "Toolbar"),
      field("peek-code-border", "边框", "Border"),
      field("peek-code-radius", "圆角", "Radius", "border-radius", "12px"),
      field("peek-code-shadow", "阴影", "Shadow", "box-shadow", "none"),
    ],
  },
  {
    id: "controls",
    zh: "按钮与图标",
    en: "Buttons & icons",
    fields: [
      field("peek-icon", "图标", "Icons"),
      field("peek-icon-muted", "次要图标", "Muted icons"),
      field("peek-icon-accent", "高亮图标", "Active icons"),
      field("peek-icon-glow", "图标发光滤镜", "Icon glow", "filter", "none"),
      field("peek-send-bg", "发送按钮", "Send button"),
      field("peek-send-fg", "按钮文字", "Button text"),
      field("peek-send-active-bg", "可发送背景", "Active send background"),
      field("peek-send-active-fg", "可发送文字", "Active send text"),
      field("peek-send-radius", "发送按钮圆角", "Send button radius", "border-radius", "999px"),
      field("peek-send-shadow", "发送按钮阴影", "Send button shadow", "box-shadow", "none"),
    ],
  },
];

export function cloneTheme(theme: CustomThemeConfig): CustomThemeConfig {
  return JSON.parse(JSON.stringify(theme)) as CustomThemeConfig;
}

/** Isolate the preview from tokens inherited from the currently active theme. */
export function resolveThemePreview(theme: CustomThemeConfig): Record<string, string> {
  const defaults = Object.fromEntries(
    themeRegions
      .flatMap((region) => region.fields)
      .filter((entry) => entry.fallback)
      .map((entry) => [entry.token, entry.fallback!]),
  );
  const dark = theme.mode === "dark";
  return {
    ...defaults,
    "--peek-user-bubble-radius": "18px",
    "--peek-assistant-radius": "0px",
    "--peek-code-body-bg": dark ? "#121212" : "#f6f8fa",
    "--peek-code-fg": dark ? "#e6e6e6" : "#1f2328",
    "--peek-code-toolbar-bg": dark ? "#202020" : "#f6f8fa",
    "--peek-code-border": dark ? "#ffffff14" : "#1b1f361f",
    ...harmonizeCustomTheme(theme),
  };
}

/** Theme values are CSS declaration values, never stylesheets or scripts. */
export function validateThemeTokens(raw: unknown): Record<string, string> {
  if (!raw || typeof raw !== "object" || Array.isArray(raw))
    throw new Error("Expected a token object");
  const tokens: Record<string, string> = {};
  for (const [key, value] of Object.entries(raw)) {
    if (!/^--[a-zA-Z][\w-]*$/.test(key) || typeof value !== "string" || /[;{}<>]/.test(value))
      throw new Error(`Invalid theme token: ${key}`);
    if (!value.trim()) continue;
    const known = themeRegions
      .flatMap((region) => region.fields)
      .find((entry) => entry.token === key);
    if (known && globalThis.CSS?.supports && !CSS.supports(known.property, value))
      throw new Error(`Invalid CSS value: ${key}`);
    tokens[key] = value.trim();
  }
  return tokens;
}
