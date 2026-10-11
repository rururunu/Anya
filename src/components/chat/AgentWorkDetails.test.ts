// @vitest-environment happy-dom
import { expect, it } from "vitest";
import { shallowMount } from "@vue/test-utils";
import { nextTick } from "vue";
import { createPinia } from "pinia";
import AgentWorkDetails from "./AgentWorkDetails.vue";

it("does not retain completed work segment views while their fold is closed", async () => {
  const wrapper = shallowMount(AgentWorkDetails, {
    props: {
      showReasoning: true,
      message: {
        id: "assistant-1",
        sessionId: "session-1",
        role: "assistant",
        status: "done",
        content: "Final answer",
        reasoning: "A completed reasoning segment",
        timestamp: 1,
      },
    },
    global: { plugins: [createPinia()] },
  });
  expect(wrapper.find(".agent-work-fold").exists()).toBe(true);
  expect(wrapper.find(".agent-work-fold-body").exists()).toBe(false);
  const details = wrapper.get(".agent-work-fold").element as HTMLDetailsElement;
  details.open = true;
  details.dispatchEvent(new Event("toggle"));
  await nextTick();
  expect(wrapper.find(".agent-work-fold-body").exists()).toBe(true);
  details.open = false;
  details.dispatchEvent(new Event("toggle"));
  await nextTick();
  expect(wrapper.find(".agent-work-fold-body").exists()).toBe(false);
  wrapper.unmount();
});

it("keeps overlay guidance out of the assistant timeline without copying the message", () => {
  const message = {
    id: "assistant-2",
    sessionId: "session-1",
    role: "assistant" as const,
    status: "streaming" as const,
    content: "",
    timestamp: 2,
    workTimeline: [{ type: "inject" as const, id: "guidance-1", content: "Follow-up" }],
  };
  const wrapper = shallowMount(AgentWorkDetails, {
    props: { message, hideTimelineInject: true },
    global: { plugins: [createPinia()] },
  });
  expect(wrapper.find(".agent-work").exists()).toBe(false);
  wrapper.unmount();
});
