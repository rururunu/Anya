import { describe, expect, it } from "vitest";
import { isPluginAgentSessionId, pluginAgentSessionId } from "@/services/chat/pluginSession";

describe("pluginAgentSessionId", () => {
  it("keeps an already-prefixed id", () => {
    expect(pluginAgentSessionId("demo", "plugin:demo:room")).toBe("plugin:demo:room");
  });

  it("prefixes a stable suffix under the plugin id", () => {
    expect(pluginAgentSessionId("demo", "room")).toBe("plugin:demo:room");
  });

  it("mints a plugin-prefixed id when suffix is omitted", () => {
    const id = pluginAgentSessionId("demo");
    expect(isPluginAgentSessionId(id)).toBe(true);
    expect(id.startsWith("plugin:demo:")).toBe(true);
  });
});
