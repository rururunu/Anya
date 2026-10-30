// @vitest-environment jsdom
import { describe, expect, it, vi } from "vitest";
import { nextTick, ref } from "vue";
import { useAskUserFlow, type AskUserSession } from "./useAskUserFlow";
import type { AppLanguage } from "@/types/setting";

describe("useAskUserFlow", () => {
  it("handles custom input open and submitting custom answers", () => {
    const language = ref<AppLanguage>("zh-CN");
    const session = ref<AskUserSession | null>({
      requestId: "req-flow-1",
      questions: [
        {
          question: "你想要哪种布局？",
          header: "布局选择",
          options: [
            { label: "网格布局", description: "Grid" },
            { label: "流式布局", description: "Flex" },
          ],
        },
      ],
    });

    const selectedIndex = ref(0);
    const emitAskUserComplete = vi.fn();
    const emitLayoutChange = vi.fn();
    const syncPopupState = vi.fn();

    const flow = useAskUserFlow({
      language,
      askUser: () => session.value,
      selectedIndex,
      emitAskUserComplete,
      emitLayoutChange,
      syncPopupState,
    });

    expect(flow.showAskUserPicker.value).toBe(true);
    expect(flow.askCustomInputOpen.value).toBe(false);

    // activeAskOptions includes preset options and skip option
    const options = flow.activeAskOptions.value;
    expect(options.length).toBe(3);
    const skipOption = options[2];
    expect(skipOption.isSkip).toBe(true);

    // Selecting skip option activates custom input
    flow.selectAskOption(skipOption);
    expect(flow.askCustomInputOpen.value).toBe(true);
    expect(selectedIndex.value).toBe(2);

    // Submitting custom answer records the answer and finishes
    flow.askCustomText.value = "瀑布流布局";
    flow.submitCustomAskAnswer();

    expect(emitAskUserComplete).toHaveBeenCalledTimes(1);
    const payload = JSON.parse(emitAskUserComplete.mock.calls[0][0]);
    expect(payload.answers[0].selected).toEqual(["瀑布流布局"]);
  });

  it("syncs the popup after setup so a later binding is safe", async () => {
    const language = ref<AppLanguage>("zh-CN");
    const session = ref<AskUserSession | null>({
      requestId: "req-flow-2",
      questions: [{ question: "继续？", header: "确认", options: [{ label: "好" }] }],
    });
    const syncPopupState = vi.fn();

    useAskUserFlow({
      language,
      askUser: () => session.value,
      selectedIndex: ref(0),
      emitAskUserComplete: vi.fn(),
      emitLayoutChange: vi.fn(),
      syncPopupState,
    });

    expect(syncPopupState).not.toHaveBeenCalled();
    await Promise.resolve();
    expect(syncPopupState).toHaveBeenCalledWith(true);

    session.value = null;
    await nextTick();
    await Promise.resolve();
    expect(syncPopupState).toHaveBeenCalledWith(false);
  });
});
