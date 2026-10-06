// @vitest-environment happy-dom
import { computed, nextTick, ref } from "vue";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";
import type ComposerEditable from "@/components/chat/ComposerEditable.vue";
import { useComposerKeyboard } from "./useComposerKeyboard";

afterEach(() => document.body.replaceChildren());

function keyboard(picker = false, suggestions = false, close = true, deferred = false) {
  const open = ref(picker);
  const suggested = ref(suggestions);
  const input = document.createElement("div");
  input.contentEditable = "true";
  input.tabIndex = 0;
  document.body.append(input);
  const focusInput = vi.fn(async () => {
    await nextTick();
    input.focus();
  });
  const routes = useComposerKeyboard({
    interactivePickerOpen: computed(() => open.value),
    showSuggestions: computed(() => suggested.value),
    composerRef: ref({ el: input } as InstanceType<typeof ComposerEditable>),
    focusInput,
    handleKeydown: (event) => {
      event.preventDefault();
      if (close) {
        const dismiss = () => {
          open.value = false;
          suggested.value = false;
        };
        if (deferred) void nextTick(dismiss);
        else dismiss();
      }
    },
  });
  return { ...routes, input, focusInput };
}

describe("composer picker cancellation", () => {
  it("restores focus after Escape closes a picker from a focused menu button", async () => {
    const route = keyboard(true);
    const button = document.createElement("button");
    document.body.append(button);
    button.focus();
    button.addEventListener("keydown", route.handleGlobalKeydown);
    button.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", cancelable: true }));
    await flushPromises();
    expect(document.activeElement).toBe(route.input);
  });

  it("restores focus after Escape dismisses slash suggestions", async () => {
    const route = keyboard(false, true);
    route.handleKeydown(new KeyboardEvent("keydown", { key: "Escape", cancelable: true }));
    await flushPromises();
    expect(document.activeElement).toBe(route.input);
  });

  it("waits for parent-owned history props to close before restoring focus", async () => {
    const route = keyboard(true, false, true, true);
    route.handleKeydown(new KeyboardEvent("keydown", { key: "Escape", cancelable: true }));
    expect(route.focusInput).not.toHaveBeenCalled();
    await flushPromises();
    expect(route.focusInput).toHaveBeenCalledOnce();
    expect(document.activeElement).toBe(route.input);
  });

  it("keeps focus routing in the picker when Escape returns to its parent menu", async () => {
    const route = keyboard(true, false, false);
    route.handleKeydown(new KeyboardEvent("keydown", { key: "Escape", cancelable: true }));
    await flushPromises();
    expect(route.focusInput).not.toHaveBeenCalled();
  });

  it("does not restore focus when Escape has no picker to dismiss", () => {
    const route = keyboard();
    route.handleKeydown(new KeyboardEvent("keydown", { key: "Escape", cancelable: true }));
    expect(route.focusInput).not.toHaveBeenCalled();
  });
});
