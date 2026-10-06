// @vitest-environment happy-dom
import { beforeEach, describe, expect, it } from "vitest";
import { computed } from "vue";
import { createPinia, setActivePinia } from "pinia";
import { useChatStore } from "./chat";
import { useChatSessionsStore } from "./chatSessions";
import type { ChatMessage } from "@/types/chat";

beforeEach(() => setActivePinia(createPinia()));

describe("live message updates", () => {
  it("keeps history and inactive messages stable across 500 chunks while rendering live text", () => {
    const store = useChatStore();
    const sessions = useChatSessionsStore();
    const history: ChatMessage[] = Array.from({ length: 1000 }, (_, i) => ({
      id: `message-${i}`,
      sessionId: "s",
      role: "assistant",
      content: "completed",
      status: "done",
      timestamp: i,
    }));
    history.push({
      id: "active",
      sessionId: "s",
      role: "assistant",
      content: "",
      status: "streaming",
      timestamp: 1001,
    });
    store.setSessionMessages("s", history);
    const before = sessions.sessions.s;
    const completed = before[0];
    let listBuilds = 0;
    const ids = computed(() => {
      listBuilds++;
      return sessions.sessions.s.map((message) => message.id);
    });
    const text = computed(() => sessions.sessions.s.at(-1)!.content);
    expect(ids.value.length).toBe(1001);
    for (let i = 0; i < 500; i++) {
      store.applyStreamDeltas([{ sessionId: "s", messageId: "active", contentDelta: "x" }]);
      expect(text.value.length).toBe(i + 1);
      expect(ids.value.length).toBe(1001);
    }
    expect(listBuilds).toBe(1);
    expect(sessions.sessions.s).toBe(before);
    expect(sessions.sessions.s[0]).toBe(completed);
    expect(sessions.sessions.s.at(-1)?.workTimeline).toHaveLength(1);
  });

  it("keeps reasoning, tools and injected-message boundaries and restores retry snapshots", () => {
    const store = useChatStore();
    const sessions = useChatSessionsStore();
    store.setSessionMessages("s", [
      {
        id: "a",
        sessionId: "s",
        role: "assistant",
        content: "saved",
        status: "streaming",
        timestamp: 1,
        workTimeline: [
          { id: "before", type: "content", content: "saved" },
          { id: "inject", type: "inject", content: "change direction" },
        ],
      },
    ]);
    store.applyStreamDeltas([
      { sessionId: "s", messageId: "a", reasoningDelta: "think", contentDelta: " next" },
    ]);
    expect(sessions.sessions.s[0].workTimeline?.map((item) => item.type)).toEqual([
      "content",
      "inject",
      "reasoning",
      "content",
    ]);
    expect(sessions.sessions.s[0].content).toBe("saved next");
    expect(sessions.sessions.s[0].reasoning).toBe("think");
    const snapshot: ChatMessage = {
      id: "a",
      sessionId: "s",
      role: "assistant",
      content: "saved",
      status: "done",
      timestamp: 1,
      workTimeline: [{ id: "before", type: "content", content: "saved" }],
    };
    store.setActivityStatus("s", "a", "stream_retry:1", undefined, snapshot);
    store.applyStreamDeltas([{ sessionId: "s", messageId: "a", contentDelta: " retry" }]);
    expect(snapshot.workTimeline?.[0]).toMatchObject({ content: "saved" });
    store.setActivityStatus("s", "a", "stream_retry:2", undefined, snapshot);
    expect(sessions.sessions.s[0].content).toBe("saved");
    expect(sessions.sessions.s[0].workTimeline?.[0]).toMatchObject({ content: "saved" });
  });
});
