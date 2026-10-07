// @vitest-environment happy-dom
import { defineComponent, nextTick, ref } from "vue";
import { flushPromises, mount } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useComposerPickerFocus } from "./useComposerPickerFocus";

const wrappers: ReturnType<typeof mount>[] = [];
afterEach(() => {
  wrappers.splice(0).forEach((wrapper) => wrapper.unmount());
  document.body.replaceChildren();
});

function fixture() {
  const open = ref(false);
  const kind = ref("model");
  const editor = ref<HTMLElement | null>(null);
  const focusEditor = vi.fn(async () => {
    editor.value?.focus();
  });
  const wrapper = mount(
    defineComponent({
      setup() {
        useComposerPickerFocus({ open, editor: () => editor.value, focusEditor });
        return { open, kind, editor };
      },
      template: `<div>
      <div ref="editor" contenteditable="true" tabindex="0" />
      <button data-picker-trigger @click="open = true">Open</button>
      <input class="outside" />
      <Teleport to="body">
        <div v-if="open" class="picker-content" :key="kind"><input class="search" /></div>
      </Teleport>
    </div>`,
    }),
    { attachTo: document.body },
  );
  wrappers.push(wrapper);
  return { wrapper, open, kind, editor, focusEditor };
}

describe("footer picker focus", () => {
  it("restores the editor immediately after a selection closes a teleported menu", async () => {
    const f = fixture();
    f.editor.value!.focus();
    f.open.value = true;
    await nextTick();
    document.querySelector<HTMLInputElement>(".search")!.focus();
    f.open.value = false;
    await flushPromises();
    expect(document.activeElement).toBe(f.editor.value);
    expect(f.focusEditor).toHaveBeenCalledOnce();
  });

  it("preserves the original focus across rapid menu replacement without stealing search focus", async () => {
    const f = fixture();
    f.editor.value!.focus();
    f.open.value = true;
    await nextTick();
    for (const kind of ["thinking", "mode", "approval", "model"]) {
      f.kind.value = kind;
      await nextTick();
      const search = document.querySelector<HTMLInputElement>(".search")!;
      search.focus();
      await flushPromises();
      expect(document.activeElement).toBe(search);
    }
    expect(f.focusEditor).not.toHaveBeenCalled();
    f.open.value = false;
    await flushPromises();
    expect(document.activeElement).toBe(f.editor.value);
  });

  it("captures focus before a mouse click moves it to the trigger", async () => {
    const f = fixture();
    f.editor.value!.focus();
    await f.wrapper.get("button").trigger("pointerdown");
    (f.wrapper.get("button").element as HTMLElement).focus();
    await f.wrapper.get("button").trigger("click");
    f.open.value = false;
    await flushPromises();
    expect(document.activeElement).toBe(f.editor.value);
  });

  it("does not steal focus from a deliberately clicked outside input", async () => {
    const f = fixture();
    f.editor.value!.focus();
    f.open.value = true;
    await nextTick();
    const outside = f.wrapper.get(".outside").element as HTMLInputElement;
    outside.focus();
    f.open.value = false;
    await flushPromises();
    expect(document.activeElement).toBe(outside);
    expect(f.focusEditor).not.toHaveBeenCalled();
  });

  it("does not focus the editor if it was not focused before opening", async () => {
    const f = fixture();
    f.open.value = true;
    await nextTick();
    f.open.value = false;
    await flushPromises();
    expect(f.focusEditor).not.toHaveBeenCalled();
  });
});
