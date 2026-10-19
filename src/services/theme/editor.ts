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
      field("peek-border", "分割线", "Borders"),
      field("peek-accent", "强调色", "Accent"),
      field("peek-accent-fg", "强调色上的文字", "Accent text"),
      field("peek-font-md", "界面字号", "UI font size", "font-size", "14px"),
      field("peek-agent-font-size", "回复字号", "Reply font size", "font-size", "13px"),
      field("peek-space-4", "消息间距", "Message spacing", "gap", "16px"),
    ],
  },
  {
    id: "sidebar",
    zh: "侧栏",
    en: "Sidebar",
    fields: [field("peek-sidebar", "背景", "Background")],
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
    fields: [field("peek-list-bg", "背景", "Background")],
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
      field("peek-composer-border", "边框", "Border"),
      field("peek-composer-border-focus", "聚焦边框", "Focus border"),
      field("peek-radius-composer", "圆角", "Radius", "border-radius", "16px"),
      field("peek-composer-shadow", "阴影", "Shadow", "box-shadow", "none"),
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
    ],
  },
  {
    id: "controls",
    zh: "按钮与图标",
    en: "Buttons & icons",
    fields: [
      field("peek-icon", "图标", "Icons"),
      field("peek-icon-accent", "高亮图标", "Active icons"),
      field("peek-send-bg", "发送按钮", "Send button"),
      field("peek-send-fg", "按钮文字", "Button text"),
      field("peek-send-active-bg", "可发送背景", "Active send background"),
      field("peek-send-active-fg", "可发送文字", "Active send text"),
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
