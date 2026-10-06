import { ref } from "vue";
import { describe, expect, it } from "vitest";
import { useSessionlessChatMode } from "./useSessionlessChatMode";

describe("pre-chat mode choice", () => {
  it("retains Plan for the new conversation without changing global defaults", () => {
    const defaultMode = ref("agent");
    const draft = useSessionlessChatMode(() => defaultMode.value);
    draft.choose("plan");
    expect(draft.mode.value).toBe("plan");
    expect(defaultMode.value).toBe("agent");
    defaultMode.value = "ask";
    expect(draft.mode.value).toBe("plan");
    draft.reset();
    expect(draft.mode.value).toBe("ask");
  });

  it("can leave Plan before sending instead of carrying a stale Plan choice", () => {
    const draft = useSessionlessChatMode(() => "agent");
    draft.choose("plan");
    draft.choose("image");
    expect(draft.mode.value).toBe("image");
  });
});
