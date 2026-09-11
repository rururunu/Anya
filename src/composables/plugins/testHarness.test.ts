import { describe, expect, it } from "vitest";
import { createFakePluginContext } from "@/composables/plugins/testHarness";

describe("createFakePluginContext", () => {
  it("records slot/asset/bus calls and runs deactivate hooks", async () => {
    const { ctx, calls, runDeactivateHooks } = createFakePluginContext("demo");

    ctx.slots.mount("composer.accessory", { id: "badge", mount: () => {} });
    ctx.assets.register("mascot.idle", { kind: "image", source: "x" });
    await ctx.agent.send("hi");
    expect(ctx.agent.sessionId()).toBe("test-session");
    const unsubscribe = ctx.bus.subscribe("demo.topic", () => {});
    ctx.onDeactivate(unsubscribe);

    expect(calls).toContain("slots.mount:composer.accessory:badge");
    expect(calls).toContain("assets.register:mascot.idle");
    expect(calls).toContain("agent.send:hi");
    expect(calls).toContain("agent.sessionId");
    expect(calls).toContain("bus.subscribe:demo.topic");

    runDeactivateHooks();
    expect(calls).toContain("bus.unsubscribe:demo.topic");
  });
});
