import { describe, expect, it } from "vitest";
import { createMemoryHistory, createRouter } from "vue-router";
import { initialOverlayRoute } from "./initialRoute";

describe("conversation popup startup", () => {
  it("keeps the requested conversation through route normalization", async () => {
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [{ path: "/overlay", component: {} }],
    });
    await router.replace(initialOverlayRoute("#/overlay?session=session-123"));
    expect(router.currentRoute.value.query.session).toBe("session-123");
    expect(
      new URLSearchParams(router.currentRoute.value.fullPath.split("?")[1]).get("session"),
    ).toBe("session-123");
  });

  it("opens an ordinary summon without a stale conversation", () => {
    expect(initialOverlayRoute("#/overlay")).toEqual({ path: "/overlay", query: {} });
    expect(initialOverlayRoute("#/overlay?session=a%2Bb").query.session).toBe("a+b");
  });
});
