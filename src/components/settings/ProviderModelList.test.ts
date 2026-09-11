/** @vitest-environment jsdom */
import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import ProviderModelList from "./ProviderModelList.vue";

describe("ProviderModelList.vue", () => {
  setActivePinia(createPinia());

  it("renders model list and emits toggleDisabled on toggle switch", async () => {
    const wrapper = mount(ProviderModelList, {
      props: {
        models: ["deepseek-chat", "deepseek-reasoner"],
        disabledModels: new Set(["deepseek-reasoner"]),
        canFetch: true,
      },
    });

    const items = wrapper.findAll(".model-item");
    expect(items.length).toBe(2);

    expect(items[0].text()).toContain("deepseek-chat");
    expect(items[0].classes()).not.toContain("is-disabled");

    expect(items[1].text()).toContain("deepseek-reasoner");
    expect(items[1].classes()).toContain("is-disabled");

    // Click toggle on the first model
    const toggle = items[0].findComponent({ name: "SettingsToggle" });
    await toggle.trigger("click");
    expect(wrapper.emitted("toggleDisabled")?.[0]).toEqual(["deepseek-chat"]);
  });

  it("emits add event when entering model name and clicking add", async () => {
    const wrapper = mount(ProviderModelList, {
      props: {
        models: ["deepseek-chat"],
        disabledModels: new Set(),
      },
    });

    const input = wrapper.find(".models-add-row input");
    await input.setValue("deepseek-coder");
    await wrapper.find(".models-add-row button").trigger("click");

    expect(wrapper.emitted("add")?.[0]).toEqual(["deepseek-coder"]);
  });

  it("emits remove event with model index when remove button is clicked", async () => {
    const wrapper = mount(ProviderModelList, {
      props: {
        models: ["deepseek-chat", "custom-model"],
        disabledModels: new Set(),
      },
    });

    const removeButtons = wrapper.findAll(".model-remove");
    expect(removeButtons.length).toBe(2);
    await removeButtons[1].trigger("click");

    expect(wrapper.emitted("remove")?.[0]).toEqual([1]);
  });

  it("emits fetch event when fetch button is clicked", async () => {
    const wrapper = mount(ProviderModelList, {
      props: {
        models: [],
        disabledModels: new Set(),
        canFetch: true,
        fetching: false,
      },
    });

    const fetchBtn = wrapper.find(".models-header-row button");
    expect(fetchBtn.attributes("disabled")).toBeUndefined();
    await fetchBtn.trigger("click");

    expect(wrapper.emitted("fetch")).toHaveLength(1);
  });

  it("shows protocol select only when supportsProtocolSelect is true", () => {
    const withoutProtocol = mount(ProviderModelList, {
      props: {
        models: ["deepseek-chat"],
        disabledModels: new Set(),
        supportsProtocolSelect: false,
      },
    });
    expect(withoutProtocol.find(".model-protocol-select").exists()).toBe(false);

    const withProtocol = mount(ProviderModelList, {
      props: {
        models: ["custom-model"],
        disabledModels: new Set(),
        supportsProtocolSelect: true,
      },
    });
    expect(withProtocol.find(".model-protocol-select").exists()).toBe(true);
  });
});
