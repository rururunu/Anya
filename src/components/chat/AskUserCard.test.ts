// @vitest-environment happy-dom
import { flushPromises, mount } from "@vue/test-utils";
import { expect, it, vi } from "vitest";
import AskUserCard from "./AskUserCard.vue";

it("collects multiple choices and keeps a custom answer available after a failed submission", async () => {
  const submitAnswer = vi
    .fn()
    .mockRejectedValueOnce(new Error("Offline"))
    .mockResolvedValueOnce(undefined);
  const card = mount(AskUserCard, {
    props: {
      language: "en-US",
      session: {
        requestId: "request-1",
        questions: [
          {
            header: "Scope",
            question: "Which parts should change?",
            multiSelect: true,
            options: [{ label: "Layout" }, { label: "Spacing" }],
          },
          { header: "Details", question: "Any constraints?", options: [] },
        ],
      },
      submitAnswer,
    },
  });
  try {
    const choices = card.findAll(".ask-choice");
    await choices[0].trigger("click");
    await choices[1].trigger("click");
    await card.get(".ask-footer .ask-submit").trigger("click");
    expect(card.text()).toContain("Any constraints?");
    await card.get("textarea").setValue("Keep the sidebar");
    await card.get("form").trigger("submit");
    await flushPromises();
    expect(card.get('[role="alert"]').text()).toContain("Offline");
    expect(card.get("textarea").element.value).toBe("Keep the sidebar");
    await card.get("form").trigger("submit");
    await flushPromises();
    expect(submitAnswer).toHaveBeenCalledTimes(2);
    expect(
      JSON.parse(submitAnswer.mock.calls[1][0]).answers.map(
        (answer: { selected: string[] }) => answer.selected,
      ),
    ).toEqual([["Layout", "Spacing"], ["Keep the sidebar"]]);
  } finally {
    card.unmount();
  }
});
