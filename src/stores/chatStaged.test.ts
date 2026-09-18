import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
const backend = vi.hoisted(() => ({ messages: [] as string[] }));

vi.mock("@/commands/remote", () => ({
  remotePushStaged: vi.fn(async (_sessionId: string, message: string) => {
    backend.messages.push(message);
    return [...backend.messages];
  }),
  remoteListStaged: vi.fn(async () => [...backend.messages]),
  remoteInsertStaged: vi.fn(async (_sessionId: string, index: number, message: string) => [
    message,
  ]),
  remoteRemoveStaged: vi.fn(async () => []),
  remoteClearStaged: vi.fn(async () => undefined),
  remotePopStaged: vi.fn(async () => backend.messages.shift() ?? null),
  remoteTakeStaged: vi.fn(
    async (_sessionId: string, index: number) => backend.messages.splice(index, 1)[0] ?? null,
  ),
}));

describe("staged queue mirror", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    backend.messages = [];
  });

  it("does not resurrect a just-flushed message from a stale remote snapshot", async () => {
    const { useChatStore } = await import("./chat");
    const store = useChatStore();
    const sessionId = "s-staged";

    store.setStagedLocal(sessionId, ["already sent"]);
    store.dropStagedFrontIf(sessionId, "already sent");
    expect(store.stagedMessages[sessionId]).toBeUndefined();

    // Late push/event still reports the pre-pop queue.
    await store.applyStagedFromRemote(sessionId, ["already sent"]);
    expect(store.stagedMessages[sessionId]).toBeUndefined();
  });

  it("keeps later queued items when filtering a stale dispatched head", async () => {
    const { useChatStore } = await import("./chat");
    const store = useChatStore();
    const sessionId = "s-staged-2";

    backend.messages = ["second"];
    await store.applyStagedFromRemote(sessionId, ["first", "second"]);
    expect(store.stagedMessages[sessionId]).toEqual(["second"]);
  });

  it("does not flush while an assistant turn is still live", async () => {
    const { remotePopStaged } = await import("@/commands/remote");
    const { useChatStore } = await import("./chat");
    const store = useChatStore();
    const sessionId = "s-staged-live";

    store.setSessionMessages(sessionId, [
      {
        id: "a1",
        sessionId,
        role: "assistant",
        content: "…",
        status: "streaming",
        timestamp: 1,
      },
    ]);
    store.setStagedLocal(sessionId, ["next"]);
    await store.flushStaged(sessionId);
    expect(remotePopStaged).not.toHaveBeenCalled();
    expect(store.stagedMessages[sessionId]).toEqual(["next"]);
  });

  it("waits for a delayed enqueue and sends it only once across three finish events", async () => {
    const remote = await import("@/commands/remote");
    const { useChatStore } = await import("./chat");
    const store = useChatStore();
    let release!: () => void;
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    vi.mocked(remote.remotePushStaged).mockImplementationOnce(async (_id, content) => {
      await gate;
      backend.messages.push(content);
      return [...backend.messages];
    });
    const send = vi.spyOn(store, "send").mockResolvedValue(true);
    const push = store.pushStagedMessage("delayed", "only once");
    const first = store.flushStaged("delayed");
    await Promise.resolve();
    expect(remote.remotePopStaged).not.toHaveBeenCalled();
    release();
    await push;
    await first;
    await store.applyStagedFromRemote("delayed", ["only once"]);
    await store.flushStaged("delayed");
    await store.flushStaged("delayed");
    expect(send).toHaveBeenCalledTimes(1);
    expect(backend.messages).toEqual([]);
    expect(store.stagedMessages.delayed).toBeUndefined();
  });

  it("does not send a local copy when dequeue acknowledgement fails", async () => {
    const remote = await import("@/commands/remote");
    const { useChatStore } = await import("./chat");
    const store = useChatStore();
    backend.messages = ["pending"];
    store.setStagedLocal("failed-pop", ["pending"]);
    vi.mocked(remote.remotePopStaged).mockRejectedValueOnce(new Error("IPC unavailable"));
    const send = vi.spyOn(store, "send").mockResolvedValue(true);
    await store.flushStaged("failed-pop");
    expect(send).not.toHaveBeenCalled();
    expect(store.stagedMessages["failed-pop"]).toEqual(["pending"]);
    expect(backend.messages).toEqual(["pending"]);
  });

  it("does not re-enqueue an ambiguous send failure", async () => {
    const remote = await import("@/commands/remote");
    const { useChatStore } = await import("./chat");
    const store = useChatStore();
    backend.messages = ["attempt"];
    const send = vi.spyOn(store, "send").mockResolvedValue(false);
    await store.flushStaged("send-failure");
    await store.flushStaged("send-failure");
    expect(send).toHaveBeenCalledTimes(1);
    expect(remote.remoteInsertStaged).not.toHaveBeenCalled();
    expect(store.stagedMessages["send-failure"]).toBeUndefined();
  });

  it("serializes repeated guide clicks and auto flush using the same dispatch lock", async () => {
    const remote = await import("@/commands/remote");
    const { useChatStore } = await import("./chat");
    const store = useChatStore();
    backend.messages = ["first", "second"];
    const send = vi.spyOn(store, "send").mockResolvedValue(true);
    await Promise.all([
      store.guideStagedMessage("guide", 0),
      store.guideStagedMessage("guide", 0),
      store.flushStaged("guide"),
    ]);
    expect(remote.remoteTakeStaged).toHaveBeenCalledTimes(1);
    expect(send).toHaveBeenCalledTimes(1);
    expect(store.stagedMessages.guide).toEqual(["second"]);
  });

  it("stageSoftInject does not attach timeline chrome to a finished assistant", async () => {
    const { useChatStore } = await import("./chat");
    const { useChatSessionsStore } = await import("./chatSessions");
    const store = useChatStore();
    const sessions = useChatSessionsStore();
    const sessionId = "s-staged-inject";

    store.setSessionMessages(sessionId, [
      {
        id: "a-done",
        sessionId,
        role: "assistant",
        content: "done",
        status: "done",
        timestamp: 1,
        workTimeline: [{ type: "content", id: "c1", content: "done" }],
      },
    ]);
    store.stageSoftInject(sessionId, "late nudge");
    const messages = sessions.sessions[sessionId] ?? [];
    const assistant = messages.find((item) => item.id === "a-done");
    expect(assistant?.workTimeline?.some((item) => item.type === "inject")).toBe(false);
    expect(messages.some((item) => item.injected && item.content === "late nudge")).toBe(true);
  });
});
