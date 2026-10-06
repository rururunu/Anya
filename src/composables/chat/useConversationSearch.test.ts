// @vitest-environment happy-dom
import { afterEach, describe, expect, it } from "vitest";
import { computed, defineComponent, h, ref } from "vue";
import { flushPromises, mount } from "@vue/test-utils";
import { useConversationSearch } from "./useConversationSearch";

const wrappers: ReturnType<typeof mount>[] = [];
afterEach(() => wrappers.splice(0).forEach((wrapper) => wrapper.unmount()));

function searchHost() {
  const sessionId = ref("first");
  let search!: ReturnType<typeof useConversationSearch>;
  const wrapper = mount(
    defineComponent({
      setup() {
        search = useConversationSearch({
          listRef: ref(null),
          stickToBottom: ref(true),
          sessionId: computed(() => sessionId.value),
          messages: computed(() => []),
          language: computed(() => "en-US"),
        });
        return () => (search.findOpen.value ? h("input", { ref: search.findInputRef }) : h("div"));
      },
    }),
    { attachTo: document.body },
  );
  wrappers.push(wrapper);
  return { wrapper, search, sessionId };
}

describe("conversation search lifecycle", () => {
  it("opens with Ctrl+F, focuses the search field and closes with Escape", async () => {
    const { wrapper, search } = searchHost();
    const open = new KeyboardEvent("keydown", { key: "f", ctrlKey: true, cancelable: true });
    globalThis.dispatchEvent(open);
    await flushPromises();
    expect(open.defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(wrapper.get("input").element);
    globalThis.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    expect(search.findOpen.value).toBe(false);
  });

  it("clears search when switching conversations and removes shortcuts on unmount", async () => {
    const { wrapper, search, sessionId } = searchHost();
    search.openFind();
    search.findQuery.value = "previous conversation";
    sessionId.value = "second";
    await flushPromises();
    expect(search.findOpen.value).toBe(false);
    expect(search.findQuery.value).toBe("");
    wrapper.unmount();
    wrappers.splice(wrappers.indexOf(wrapper), 1);
    const open = new KeyboardEvent("keydown", { key: "f", ctrlKey: true, cancelable: true });
    globalThis.dispatchEvent(open);
    expect(open.defaultPrevented).toBe(false);
    expect(search.findOpen.value).toBe(false);
  });
});
