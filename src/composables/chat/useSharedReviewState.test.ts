// @vitest-environment happy-dom
import { beforeEach, expect, it } from "vitest";
import { defineComponent, h, ref } from "vue";
import { mount } from "@vue/test-utils";
import { useSharedReviewState } from "./useSharedReviewState";

beforeEach(() => localStorage.clear());
it("restores selected images and clears them when moving to a different session", () => {
  localStorage.setItem(
    "anya.shared-review.v1.parent",
    JSON.stringify({
      images: ["image.png"],
      selectedImage: "image.png",
      subagents: ["child"],
      selectedSubagent: "child",
    }),
  );
  const session = ref("parent"),
    images = ref<string[]>([]),
    selected = ref("");
  const wrapper = mount(
    defineComponent({
      setup() {
        useSharedReviewState(() => session.value, { images, selectedImage: selected });
        return () => h("div");
      },
    }),
  );
  expect(images.value).toEqual(["image.png"]);
  expect(selected.value).toBe("image.png");
  session.value = "different";
  expect(images.value).toEqual([]);
  expect(selected.value).toBe("");
  wrapper.unmount();
});
it("receives another window's update and preserves fields owned by that window", () => {
  const images = ref<string[]>([]),
    selected = ref("");
  const wrapper = mount(
    defineComponent({
      setup() {
        useSharedReviewState(() => "s", { images, selectedImage: selected });
        return () => h("div");
      },
    }),
  );
  const key = "anya.shared-review.v1.s";
  localStorage.setItem(
    key,
    JSON.stringify({ images: ["first.png"], selectedImage: "first.png", subagents: ["child"] }),
  );
  globalThis.dispatchEvent(new StorageEvent("storage", { key }));
  expect(selected.value).toBe("first.png");
  images.value = ["first.png", "second.png"];
  expect(JSON.parse(localStorage.getItem(key)!).subagents).toEqual(["child"]);
  wrapper.unmount();
});
