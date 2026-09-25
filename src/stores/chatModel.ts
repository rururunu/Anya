import { defineStore } from "pinia";

import { selectDefaultChatModel } from "@/services/chat/ensureDefaultModel";
import { readCachedModelList, writeCachedModelList } from "@/services/chat/modelListCache";
import { listChatModels } from "@/services/ipc";
import { createLogger } from "@/services/logger";
import type { ChatModelInfo } from "@/types/chat";
import { useSettingStore } from "./setting";

const log = createLogger("chat-model");

/** Split a provider's newline- or comma-separated model ID list. */
function splitModelIds(value: string | undefined): Set<string> {
  return new Set(
    (value ?? "")
      .split(/[,\n]/)
      .map((id) => id.trim())
      .filter(Boolean),
  );
}

export const useChatModelStore = defineStore("chatModel", {
  state: () => ({
    // Seeded from the boot cache so the picker opens on the last known list
    // instead of an empty panel; live discovery replaces it in the background.
    models: readCachedModelList(),
    loading: false,
    refreshing: false,
    error: null as string | null,
    /** `models` was filled from persisted settings before discovery finished. */
    primedFromSettings: false,
    /** One background discovery per session; see `queueBackgroundRefresh`. */
    backgroundRefreshStarted: false,
  }),
  actions: {
    /** Single writer for `models` — keeps the boot cache in step with the UI. */
    setModels(models: ChatModelInfo[]) {
      this.models = models;
      writeCachedModelList(models);
    },
    /**
     * Fill `models` from the persisted settings so the picker is usable before the
     * network-bound `list_chat_models` reply lands. Mirrors the host command's
     * non-network branches. Idempotent.
     */
    primeFromSettings(): boolean {
      if (this.primedFromSettings) {
        return false;
      }
      this.primedFromSettings = true;
      if (this.models.length > 0) {
        return false;
      }

      const settingStore = useSettingStore();
      const primed: ChatModelInfo[] = [];

      if (settingStore.deepseekApiKey.trim()) {
        for (const entry of settingStore.deepseekModels) {
          if (!entry.disabled) {
            primed.push({ id: entry.id, ownedBy: "deepseek", provider: "deepseek" });
          }
        }
      }

      for (const custom of settingStore.customProviders) {
        if (!custom.baseUrl.trim()) {
          continue;
        }
        const disabled = splitModelIds(custom.disabledModels);
        for (const id of splitModelIds(custom.models)) {
          if (!disabled.has(id)) {
            primed.push({ id, ownedBy: custom.name, provider: custom.id });
          }
        }
      }

      this.models = primed;
      return primed.length > 0;
    },
    /** Run the first background discovery of the session; later calls are no-ops. */
    queueBackgroundRefresh() {
      if (this.backgroundRefreshStarted) {
        return;
      }
      this.backgroundRefreshStarted = true;
      void this.softRefresh();
    },
    async fetch(force = false) {
      if (this.loading) {
        return;
      }

      this.primeFromSettings();

      if (!force && this.models.length > 0) {
        // Something is already on screen (persisted settings or an earlier load):
        // let provider discovery land in the background instead of blocking.
        this.queueBackgroundRefresh();
        return;
      }

      this.loading = true;
      this.error = null;

      try {
        this.setModels(await listChatModels());
      } catch (error) {
        log.error("list_chat_models failed", error);
        this.error = error instanceof Error ? error.message : "无法获取模型列表";
      } finally {
        this.loading = false;
      }
    },
    /** Refresh without clearing the list / flashing a loading state. */
    async softRefresh() {
      if (this.loading) {
        return;
      }

      try {
        this.setModels(await listChatModels());
        this.error = null;
      } catch (error) {
        log.error("list_chat_models failed", error);
        if (this.models.length === 0) {
          this.error = error instanceof Error ? error.message : "无法获取模型列表";
        }
      }
    },
    async refresh() {
      if (this.models.length > 0) {
        await this.reload();
        return;
      }
      await this.fetch(true);
    },
    /** Force reload from API while keeping the current list visible. */
    async reload() {
      if (this.loading || this.refreshing) {
        return;
      }

      this.refreshing = true;

      try {
        this.setModels(await listChatModels());
        this.error = null;
      } catch (error) {
        log.error("list_chat_models failed", error);
        this.error = error instanceof Error ? error.message : "无法获取模型列表";
      } finally {
        this.refreshing = false;
      }
    },
    /** Pick the first available model when none (or an invalid one) is selected. */
    async ensureDefault(options: { refresh?: boolean } = {}): Promise<ChatModelInfo | null> {
      const settingStore = useSettingStore();

      if (options.refresh) {
        await this.refresh();
      } else {
        // `fetch` primes from settings and only blocks when there is nothing to
        // show, so the persisted selection resolves without waiting on the network.
        await this.fetch();
      }

      const selected = selectDefaultChatModel(
        this.models,
        settingStore.chatModel,
        settingStore.chatModelProvider,
      );
      if (!selected) return null;

      if (selected.needsPersist) {
        await settingStore.update({
          chatModel: selected.model.id,
          chatModelProvider: selected.model.provider,
        });
      }
      return selected.model;
    },
  },
});
