import { describe, expect, it } from "vitest";
import { chatEn, chatLocales, chatSupplements } from "./locales/chat";
import { mcpEn, mcpLocales } from "./locales/mcp";
import { onboardingEn, onboardingLocales } from "./locales/onboarding";
import { settingsEn, settingsLocales } from "./locales/settings";
import { skillsEn, skillsLocales } from "./locales/skills";
import { workspaceEn, workspaceLocales } from "./locales/workspace";
import { chatInputEn, chatInputLocales } from "./locales/chatInput";
import { slashEn, slashLocales } from "./locales/slash";
import { uiEn, uiLocales } from "./locales/ui";
import { shellEn, shellLocales } from "./locales/shell";
import type { AppLanguage } from "@/types/setting";

const modules = [
  ["chat", chatEn, chatLocales, chatSupplements],
  ["mcp", mcpEn, mcpLocales],
  ["onboarding", onboardingEn, onboardingLocales],
  ["settings", settingsEn, settingsLocales],
  ["skills", skillsEn, skillsLocales],
  ["workspace", workspaceEn, workspaceLocales],
  ["chatInput", chatInputEn, chatInputLocales],
  ["slash", slashEn, slashLocales],
  ["ui", uiEn, uiLocales],
  ["shell", shellEn, shellLocales],
] as const;

describe("translation catalog", () => {
  it("keeps core catalogs complete and translated placeholders consistent", () => {
    const languages: AppLanguage[] = ["zh-CN", "ja-JP", "ru-RU", "de-DE", "fr-FR", "ko-KR"];
    const placeholderMismatches: string[] = [];
    for (const [name, english, locales, supplements] of modules) {
      const keys = Object.keys(english);
      for (const language of languages) {
        expect(locales[language], `${name}.${language}`).toBeDefined();
      }
      const chinese = { ...(locales["zh-CN"] || {}), ...(supplements?.["zh-CN"] || {}) } as Record<
        string,
        string
      >;
      const missingChinese = keys.filter((key) => chinese[key] == null);
      expect(missingChinese, `${name} missing zh-CN keys`).toEqual([]);
      if (["onboarding", "chatInput", "slash", "skills", "shell", "workspace"].includes(name)) {
        for (const language of languages) {
          const translated = locales[language] as Record<string, string>;
          expect(
            keys.filter((key) => translated[key] == null),
            `${name} missing ${language} keys`,
          ).toEqual([]);
        }
      }
      for (const language of languages) {
        const translated = {
          ...(locales[language] || {}),
          ...(supplements?.[language] || {}),
        } as Record<string, string>;
        for (const key of keys) {
          if (translated[key] == null) continue;
          const variables = (value: string) =>
            [...value.matchAll(/\{([a-zA-Z][a-zA-Z0-9]*)\}/g)].map((match) => match[1]).sort();
          if (
            variables(translated[key]).join() !==
            variables((english as Record<string, string>)[key]).join()
          ) {
            placeholderMismatches.push(`${name}.${language}.${key}`);
          }
        }
      }
    }
    expect(placeholderMismatches).toEqual([]);
  });
});
