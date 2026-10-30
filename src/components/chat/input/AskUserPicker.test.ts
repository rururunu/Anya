// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import AskUserPicker from "./AskUserPicker.vue";

describe("AskUserPicker", () => {
  const defaultProps = {
    header: "需要澄清",
    question: "请选择首选工具",
    questionIndex: 0,
    questionCount: 1,
    options: [
      { label: "Option A", slug: "opt-a", description: "Description A" },
      { label: "Option B", slug: "opt-b", description: "Description B" },
      {
        label: "不选择，由我补充",
        slug: "custom",
        description: "直接输入你的回答",
        isSkip: true,
      },
    ],
    confirmRowIndex: -1,
    confirmLabel: "确认",
    selectedCount: 0,
    selectedCountLabel: "已选 0 项",
    selectedIndex: 0,
    ariaLabel: "提问选择器",
    isOptionSelected: () => false,
  };

  it("renders questions and options list including custom skip option", () => {
    const wrapper = mount(AskUserPicker, {
      props: defaultProps,
    });

    expect(wrapper.text()).toContain("需要澄清");
    expect(wrapper.text()).toContain("请选择首选工具");
    expect(wrapper.text()).toContain("Option A");
    expect(wrapper.text()).toContain("Option B");
    expect(wrapper.text()).toContain("不选择，由我补充");
  });

  it("emits select when clicking an option", async () => {
    const wrapper = mount(AskUserPicker, {
      props: defaultProps,
    });

    const items = wrapper.findAll(".command-item");
    await items[0].trigger("mousedown");
    expect(wrapper.emitted("select")?.[0]).toEqual([defaultProps.options[0]]);

    await items[2].trigger("mousedown");
    expect(wrapper.emitted("select")?.[1]).toEqual([defaultProps.options[2]]);
  });

  it("renders custom input when customInputOpen is true and allows submitting custom text", async () => {
    const wrapper = mount(AskUserPicker, {
      props: {
        ...defaultProps,
        customInputOpen: true,
        customText: "我的自定义补充回答",
        sendLabel: "发送",
      },
    });

    const inputWrap = wrapper.find(".ask-custom-input-wrap");
    expect(inputWrap.exists()).toBe(true);

    const input = inputWrap.find("input.ask-custom-input");
    expect(input.exists()).toBe(true);
    expect((input.element as HTMLInputElement).value).toBe("我的自定义补充回答");

    const submitBtn = inputWrap.find(".ask-custom-submit-btn");
    expect(submitBtn.exists()).toBe(true);
    expect(submitBtn.attributes("disabled")).toBeUndefined();

    // Clicking submit emits submitCustom
    await submitBtn.trigger("click");
    expect(wrapper.emitted("submitCustom")).toBeTruthy();

    // Pressing Enter in input emits submitCustom
    await input.trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("submitCustom")?.length).toBe(2);

    // Pressing Escape emits cancelCustom
    await input.trigger("keydown", { key: "Escape" });
    expect(wrapper.emitted("cancelCustom")).toBeTruthy();

    // Pressing ArrowUp / ArrowDown emits navigateFromCustom
    await input.trigger("keydown", { key: "ArrowUp" });
    expect(wrapper.emitted("navigateFromCustom")?.[0]).toEqual([-1]);

    await input.trigger("keydown", { key: "ArrowDown" });
    expect(wrapper.emitted("navigateFromCustom")?.[1]).toEqual([1]);
  });
});
