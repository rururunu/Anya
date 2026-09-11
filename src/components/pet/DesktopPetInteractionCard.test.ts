// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import DesktopPetInteractionCard from "./DesktopPetInteractionCard.vue";
import { useSettingStore } from "@/stores/setting";
import type { PetInteraction } from "@/composables/useDesktopPetInteractions";

describe("DesktopPetInteractionCard", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("renders ask_user mode with session title and workspace name", async () => {
    const interaction: PetInteraction = {
      kind: "ask_user",
      requestId: "req-1",
      sessionId: "session-1",
      sessionTitle: "测试会话",
      workspaceName: "AnyaCore",
      header: "需要澄清",
      question: "您希望使用哪个模型？",
      options: [
        { label: "Claude 3.5 Sonnet", description: "推荐用于代码编写" },
        { label: "GPT-4o", description: "通用全能模型" },
      ],
      multiSelect: false,
    };

    const wrapper = mount(DesktopPetInteractionCard, {
      props: { interaction },
    });

    expect(wrapper.text()).toContain("测试会话");
    expect(wrapper.text()).toContain("AnyaCore");
    expect(wrapper.text()).toContain("需要澄清");
    expect(wrapper.text()).toContain("您希望使用哪个模型？");
    expect(wrapper.text()).toContain("Claude 3.5 Sonnet");

    // 单击单选选项，直接触发 submit-ask
    const optionBtns = wrapper.findAll(".option-item");
    expect(optionBtns.length).toBe(2);
    await optionBtns[0].trigger("click");
    expect(wrapper.emitted("submit-ask")).toEqual([["Claude 3.5 Sonnet"]]);

    // 点击关闭按钮触发 dismiss
    const dismissBtn = wrapper.find(".dismiss-btn");
    await dismissBtn.trigger("click");
    expect(wrapper.emitted("dismiss")).toBeTruthy();
  });

  it("does not render workspace badge when workspaceName is undefined", () => {
    const interaction: PetInteraction = {
      kind: "ask_user",
      requestId: "req-2",
      sessionId: "session-2",
      sessionTitle: "无工作区对话",
      question: "请选择是否继续？",
      options: [{ label: "是" }, { label: "否" }],
    };

    const wrapper = mount(DesktopPetInteractionCard, {
      props: { interaction },
    });

    expect(wrapper.text()).toContain("无工作区对话");
    expect(wrapper.find(".workspace-chip").exists()).toBe(false);
  });

  it("supports multi-select options with confirm button", async () => {
    const interaction: PetInteraction = {
      kind: "ask_user",
      requestId: "req-3",
      sessionId: "session-3",
      sessionTitle: "多选测试",
      question: "请勾选所需功能",
      options: [{ label: "代码补全" }, { label: "单元测试" }, { label: "文档生成" }],
      multiSelect: true,
    };

    const wrapper = mount(DesktopPetInteractionCard, {
      props: { interaction },
    });

    const confirmBtn = wrapper.find(".confirm-item");
    expect(confirmBtn.classes()).toContain("disabled");

    const optionBtns = wrapper.findAll(".option-item");
    await optionBtns[0].trigger("mousedown");
    await optionBtns[2].trigger("mousedown");

    expect(confirmBtn.text()).toContain("已选 2 项");
    await confirmBtn.trigger("mousedown");
    expect(wrapper.emitted("submit-ask")).toEqual([["代码补全, 文档生成"]]);
  });

  it("renders path_permission mode and emits corresponding decisions", async () => {
    const interaction: PetInteraction = {
      kind: "path_permission",
      requestId: "req-4",
      sessionId: "session-4",
      sessionTitle: "读取文件",
      workspaceName: "MyProject",
      path: "C:/Projects/secret.key",
      operation: "read",
      toolName: "read_file",
    };

    const wrapper = mount(DesktopPetInteractionCard, {
      props: { interaction },
    });

    expect(wrapper.text()).toContain("权限申请");
    expect(wrapper.text()).toContain("C:/Projects/secret.key");

    const optionItems = wrapper.findAll(".permission-option");
    expect(optionItems.length).toBe(3);

    // 允许一次 (allow_once)
    await optionItems[0].trigger("mousedown");
    expect(wrapper.emitted("submit-path")?.[0]).toEqual(["allow_once"]);

    // 总是允许 (allow_always)
    await optionItems[1].trigger("mousedown");
    expect(wrapper.emitted("submit-path")?.[1]).toEqual(["allow_always"]);

    // 拒绝 (deny)
    await optionItems[2].trigger("mousedown");
    expect(wrapper.emitted("submit-path")?.[2]).toEqual(["deny"]);
  });

  it("renders tool_approval mode and emits corresponding decisions", async () => {
    const interaction: PetInteraction = {
      kind: "tool_approval",
      requestId: "req-5",
      sessionId: "session-5",
      sessionTitle: "执行命令",
      toolName: "run_terminal",
      title: "执行 npm run build",
      diffSummary: "package.json",
    };

    const wrapper = mount(DesktopPetInteractionCard, {
      props: { interaction },
    });

    expect(wrapper.text()).toContain("执行 npm run build");
    expect(wrapper.text()).toContain("package.json");

    const optionItems = wrapper.findAll(".permission-option");
    expect(optionItems.length).toBe(3);

    // 允许一次 (allow_once)
    await optionItems[0].trigger("mousedown");
    expect(wrapper.emitted("submit-tool")?.[0]).toEqual(["allow_once"]);

    // 会话允许 (allow_session)
    await optionItems[1].trigger("mousedown");
    expect(wrapper.emitted("submit-tool")?.[1]).toEqual(["allow_session"]);

    // 拒绝 (deny)
    await optionItems[2].trigger("mousedown");
    expect(wrapper.emitted("submit-tool")?.[2]).toEqual(["deny"]);
  });

  it("renders queue navigation when totalCount > 1 and emits prev/next", async () => {
    const interaction: PetInteraction = {
      kind: "ask_user",
      requestId: "req-queue",
      sessionId: "session-queue",
      sessionTitle: "多请求会话",
      question: "处理第 1 个请求？",
      options: [{ label: "OK" }],
    };

    const wrapper = mount(DesktopPetInteractionCard, {
      props: {
        interaction,
        totalCount: 3,
        currentIndex: 0,
      },
    });

    const queueNav = wrapper.find(".interaction-queue-nav");
    expect(queueNav.exists()).toBe(true);
    expect(wrapper.find(".queue-counter").text()).toBe("1/3");

    const navBtns = wrapper.findAll(".queue-nav-btn");
    expect(navBtns.length).toBe(2);

    await navBtns[0].trigger("click");
    expect(wrapper.emitted("prev")).toBeTruthy();

    await navBtns[1].trigger("click");
    expect(wrapper.emitted("next")).toBeTruthy();
  });

  it("renders localized strings according to user language", () => {
    const settingStore = useSettingStore();
    settingStore.language = "en-US";

    const interaction: PetInteraction = {
      kind: "path_permission",
      requestId: "req-i18n",
      sessionId: "session-i18n",
      sessionTitle: "English Session",
      path: "C:/Projects/test",
    };

    const wrapper = mount(DesktopPetInteractionCard, {
      props: {
        interaction,
        totalCount: 2,
        currentIndex: 0,
      },
    });

    expect(wrapper.find(".header-title").text()).toBe("Permission request");
    expect(wrapper.find(".queue-nav-btn").attributes("title")).toBe("Previous request");
    expect(wrapper.find(".dismiss-btn").attributes("title")).toBe("Close panel");
    expect(wrapper.text()).toContain("Allow once");
    expect(wrapper.text()).toContain("Always allow");
    expect(wrapper.text()).toContain("Deny");
  });
});
