// @vitest-environment happy-dom
import { mount } from "@vue/test-utils";
import { expect, it } from "vitest";
import type { MascotState } from "@/components/icons/MascotFace.vue";
import WorkbenchSessionList from "./WorkbenchSessionList.vue";

it("shows each active session's mascot, including background sessions awaiting input", async () => {
  const wrapper = mount(WorkbenchSessionList, {
    props: {
      sessions: ["foreground", "background", "idle"].map((sessionId) => ({
        sessionId,
        preview: sessionId,
        updatedAt: 0,
        messageCount: 0,
        turnCount: 0,
        estimatedTokens: 0,
      })),
      activeSessionId: "foreground",
      language: "en-US",
      untitledLabel: "Untitled",
      archiveLabel: "Archive",
      archivedLabel: "Archived",
      renameLabel: "Rename",
      regenerateTitleLabel: "Regenerate title",
      deleteLabel: "Delete",
      generatingTitleLabel: "Generating title",
      sessionMenuLabel: "Session menu",
      runningSessionStates: new Map<string, MascotState>([
        ["foreground", "thinking"],
        ["background", "working"],
      ]),
      attentionSessionIds: [],
      unreadSessionIds: [],
    },
  });

  try {
    const [foreground, background, idle] = wrapper.findAll(".session-row");
    expect(foreground.find(".mascot-face.st-thinking").exists()).toBe(true);
    expect(background.find(".mascot-face.st-working").exists()).toBe(true);
    expect(idle.find(".mascot-face").exists()).toBe(false);

    await wrapper.setProps({
      runningSessionStates: new Map([["foreground", "talking"]]),
      attentionSessionIds: ["background"],
    });
    expect(foreground.find(".mascot-face.st-talking").exists()).toBe(true);
    expect(background.find(".mascot-face.st-waiting").exists()).toBe(true);

    await wrapper.setProps({ runningSessionStates: new Map(), attentionSessionIds: [] });
    expect(wrapper.findAll(".mascot-face")).toHaveLength(0);
  } finally {
    wrapper.unmount();
  }
});
