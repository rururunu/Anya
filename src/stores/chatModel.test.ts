import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useChatModelStore } from "./chatModel";

const saved = vi.hoisted(() => ({
  chatModel: "chosen",
  chatModelProvider: "chosen-provider",
  update: vi.fn(),
}));
vi.mock("./setting", () => ({ useSettingStore: () => saved }));
vi.mock("@/services/chat/modelListCache", () => ({
  readCachedModelList: () => [{ id: "old-cache-model", provider: "other-provider" }],
  writeCachedModelList: vi.fn(),
}));
vi.mock("@/services/ipc", () => ({
  listChatModels: vi.fn(async () => [{ id: "old-cache-model", provider: "other-provider" }]),
}));
beforeEach(() => {
  setActivePinia(createPinia());
  saved.update.mockClear();
});
describe("default model startup persistence", () => {
  it("keeps the saved selection across startup and refresh with an incomplete cached model list", async () => {
    const store = useChatModelStore();
    await store.ensureDefault();
    await store.ensureDefault({ refresh: true });
    expect(saved.update).not.toHaveBeenCalled();
    expect(saved.chatModel).toBe("chosen");
    expect(saved.chatModelProvider).toBe("chosen-provider");
  });
});
