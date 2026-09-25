import { tr } from "@/services/i18n";
import { settingsEn } from "@/services/locales/settings";
import { uiEn } from "@/services/locales/ui";
import { workspaceEn } from "@/services/locales/workspace";
import type { AppLanguage } from "@/types/setting";

const standalonePrefixes: Record<string, readonly string[]> = {
  provider: ["settings.provider."],
  image: ["settings.image."],
  rag: ["settings.rag."],
  history: ["settings.history."],
  archive: ["settings.archive."],
  pet: ["settings.pet."],
  workspace: ["workspace."],
  usage: ["usage."],
  about: ["about."],
};

const standaloneKeys = Object.keys({ ...settingsEn, ...uiEn, ...workspaceEn });

export function matchesStandaloneSettingsCategory(
  category: string,
  language: AppLanguage,
  query: string,
): boolean {
  const normalizedQuery = query.trim().toLocaleLowerCase();
  if (!normalizedQuery) return true;
  const prefixes = standalonePrefixes[category];
  if (!prefixes) return false;
  return standaloneKeys.some(
    (key) =>
      prefixes.some((prefix) => key.startsWith(prefix)) &&
      tr(language, key as Parameters<typeof tr>[1])
        .toLocaleLowerCase()
        .includes(normalizedQuery),
  );
}
