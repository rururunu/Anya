import type { AppLanguage } from "@/types/setting";
import { chatEn, chatLocales, chatSupplements, type ChatI18nKey } from "@/services/locales/chat";
import { mcpEn, mcpLocales, type McpI18nKey } from "@/services/locales/mcp";
import {
  onboardingEn,
  onboardingLocales,
  type OnboardingI18nKey,
} from "@/services/locales/onboarding";
import { settingsEn, settingsLocales, type SettingsI18nKey } from "@/services/locales/settings";
import { skillsEn, skillsLocales, type SkillsI18nKey } from "@/services/locales/skills";
import { chatInputEn, chatInputLocales, type ChatInputI18nKey } from "@/services/locales/chatInput";
import { workspaceEn, workspaceLocales, type WorkspaceI18nKey } from "@/services/locales/workspace";
import { slashEn, slashLocales, type SlashI18nKey } from "@/services/locales/slash";
import { uiEn, uiLocales, type UiI18nKey } from "@/services/locales/ui";
import { shellEn, shellLocales, type ShellI18nKey } from "@/services/locales/shell";

type ModuleEn = Record<string, string>;
type ModuleLocales = Record<AppLanguage, Partial<Record<string, string>>>;
type ModuleSupplements = Partial<Record<AppLanguage, Partial<Record<string, string>>>>;

const modules: Array<{
  en: ModuleEn;
  locales: ModuleLocales;
  supplements?: ModuleSupplements;
}> = [
  {
    en: chatEn as unknown as ModuleEn,
    locales: chatLocales as unknown as ModuleLocales,
    supplements: chatSupplements as unknown as ModuleSupplements,
  },
  {
    en: mcpEn as unknown as ModuleEn,
    locales: mcpLocales as unknown as ModuleLocales,
  },
  {
    en: onboardingEn as unknown as ModuleEn,
    locales: onboardingLocales as unknown as ModuleLocales,
  },
  {
    en: settingsEn as unknown as ModuleEn,
    locales: settingsLocales as unknown as ModuleLocales,
  },
  {
    en: skillsEn as unknown as ModuleEn,
    locales: skillsLocales as unknown as ModuleLocales,
  },
  {
    en: workspaceEn as unknown as ModuleEn,
    locales: workspaceLocales as unknown as ModuleLocales,
  },
  {
    en: chatInputEn as unknown as ModuleEn,
    locales: chatInputLocales as unknown as ModuleLocales,
  },
  {
    en: slashEn as unknown as ModuleEn,
    locales: slashLocales as unknown as ModuleLocales,
  },
  {
    en: uiEn as unknown as ModuleEn,
    locales: uiLocales as unknown as ModuleLocales,
  },
  {
    en: shellEn as unknown as ModuleEn,
    locales: shellLocales as unknown as ModuleLocales,
  },
];

function resolve(language: AppLanguage, key: string): string | undefined {
  for (const mod of modules) {
    const fromSupplement = mod.supplements?.[language]?.[key];
    if (fromSupplement != null) return fromSupplement;
    const fromLocale = mod.locales[language]?.[key];
    if (fromLocale != null) return fromLocale;
    if (key in mod.en) return mod.en[key];
  }
  return undefined;
}

// Widen as more modules are registered.
export type I18nKey =
  | ChatI18nKey
  | ChatInputI18nKey
  | McpI18nKey
  | OnboardingI18nKey
  | SettingsI18nKey
  | SkillsI18nKey
  | WorkspaceI18nKey
  | SlashI18nKey
  | UiI18nKey
  | ShellI18nKey;

export function tr(
  language: AppLanguage | undefined,
  key: I18nKey,
  values: Record<string, string | number> = {},
) {
  const locale = language ?? "en-US";
  let text = resolve(locale, key) ?? String(key);
  for (const [name, value] of Object.entries(values)) {
    text = text.split(`{${name}}`).join(String(value));
  }
  return text;
}
