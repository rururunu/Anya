// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { nextTick } from "vue";
import SplitDiffStack from "./SplitDiffStack.vue";

class TestObserver {
  static current: TestObserver | undefined;
  readonly observe = vi.fn();
  readonly unobserve = vi.fn();
  readonly disconnect = vi.fn();
  private readonly callback: IntersectionObserverCallback;

  constructor(callback: IntersectionObserverCallback) {
    this.callback = callback;
    TestObserver.current = this;
  }

  emit(element: Element, isIntersecting: boolean) {
    this.callback(
      [{ target: element, isIntersecting } as IntersectionObserverEntry],
      this as unknown as IntersectionObserver,
    );
  }
}

afterEach(() => {
  vi.unstubAllGlobals();
  TestObserver.current = undefined;
});

it("releases diff editors outside the viewport and while the sidebar is hidden", async () => {
  vi.stubGlobal("IntersectionObserver", TestObserver);
  const wrapper = mount(SplitDiffStack, {
    props: {
      files: [
        {
          id: "first",
          path: "first.ts",
          displayPath: "first.ts",
          diff: "+changed",
          added: 1,
          removed: 0,
          language: { id: "typescript", badge: "TS", icon: "" },
        },
      ],
      activeId: "first",
      wrapLines: true,
      viewMode: "unified",
      active: true,
    },
    global: { stubs: { CodeDiffEditor: { template: '<div class="code-diff-editor" />' } } },
  });
  const article = wrapper.get(".split-file").element;
  expect(wrapper.find(".code-diff-editor").exists()).toBe(false);
  TestObserver.current!.emit(article, true);
  await nextTick();
  expect(wrapper.find(".code-diff-editor").exists()).toBe(true);
  Object.defineProperty(wrapper.get(".code-diff-editor").element, "offsetHeight", { value: 240 });
  TestObserver.current!.emit(article, false);
  await nextTick();
  expect(wrapper.find(".code-diff-editor").exists()).toBe(false);
  expect(wrapper.get(".split-file-pending").attributes("style")).toContain("240px");
  TestObserver.current!.emit(article, true);
  await nextTick();
  await wrapper.setProps({ active: false });
  expect(wrapper.find(".code-diff-editor").exists()).toBe(false);
  wrapper.unmount();
});
