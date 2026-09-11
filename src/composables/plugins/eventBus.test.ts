import { describe, expect, it } from "vitest";
import { publish, subscribe, unsubscribeAll } from "@/composables/plugins/eventBus";

describe("eventBus", () => {
  it("delivers published payloads to subscribers on the same topic", () => {
    const received: unknown[] = [];
    const off = subscribe("t1", "plugin-a", (payload) => received.push(payload));
    publish("t1", { hello: "world" });
    expect(received).toEqual([{ hello: "world" }]);
    off();
    publish("t1", { hello: "again" });
    expect(received).toHaveLength(1);
  });

  it("caps subscriptions per plugin", () => {
    const pluginId = `cap-test-${Math.random()}`;
    expect(() => {
      for (let i = 0; i < 201; i += 1) subscribe(`topic-${i}`, pluginId, () => {});
    }).toThrow();
    unsubscribeAll(pluginId);
  });

  it("isolates a subscriber error without breaking other subscribers", () => {
    const received: unknown[] = [];
    subscribe("t2", "plugin-bad", () => {
      throw new Error("boom");
    });
    subscribe("t2", "plugin-good", (payload) => received.push(payload));
    expect(() => publish("t2", 1)).not.toThrow();
    expect(received).toEqual([1]);
    unsubscribeAll("plugin-bad");
    unsubscribeAll("plugin-good");
  });
});
