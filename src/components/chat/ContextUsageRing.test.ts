// @vitest-environment happy-dom
import { mount } from "@vue/test-utils";
import { ref } from "vue";
import { describe, expect, it, vi } from "vitest";
import ContextUsageRing from "./ContextUsageRing.vue";

vi.mock("@/stores/setting", () => ({ useSettingStore: () => ({ language: ref("en") }) }));

describe("overlay context usage panel", () => {
  it("renders its full breakdown in the overlay picker host and closes on Escape", async () => {
    const host = document.createElement("div");
    document.body.append(host);
    const wrapper = mount(ContextUsageRing, {
      attachTo: document.body,
      props: {
        usage: {
          usageRatio: 0.1,
          estimatedTokens: 100,
          contextWindowTokens: 1000,
          systemPromptTokens: 60,
          messageTokens: 40,
        },
        panelTarget: host,
        open: true,
        panelStyle: { "--chip-picker-top": "80px", "--chip-picker-bottom": "auto" },
      },
    });
    const card = host.querySelector<HTMLElement>(".command-list.context-usage-card")!;
    expect(card).not.toBeNull();
    expect(card.querySelector("header")).not.toBeNull();
    expect(card.querySelectorAll(".context-usage-list li")).toHaveLength(2);
    expect(card.style.getPropertyValue("--chip-picker-top")).toBe("80px");
    const escape = new KeyboardEvent("keydown", { key: "Escape", cancelable: true });
    window.dispatchEvent(escape);
    expect(escape.defaultPrevented).toBe(true);
    expect(wrapper.emitted("openChange")?.[0]?.[0]).toBe(false);
    await wrapper.setProps({ open: false });
    expect(host.querySelector(".context-usage-card")).toBeNull();
    wrapper.unmount();
    host.remove();
  });
});
