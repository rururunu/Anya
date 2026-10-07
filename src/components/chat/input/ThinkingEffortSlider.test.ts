// @vitest-environment happy-dom
import { mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import ThinkingEffortSlider from "./ThinkingEffortSlider.vue";

const options = [
  { id: "low", label: "低" },
  { id: "medium", label: "中" },
  { id: "high", label: "高" },
];

describe("capsule thinking slider", () => {
  it("displays the model's actual levels and supports direct label selection and keyboard steps", async () => {
    const wrapper = mount(ThinkingEffortSlider, {
      props: { options, selectedId: "medium", title: "思考强度" },
    });
    expect(wrapper.get(".thinking-slider-heading").text()).toBe("思考强度");
    expect(wrapper.findAll(".thinking-slider-labels span").map((item) => item.text())).toEqual([
      "低",
      "中",
      "高",
    ]);
    await wrapper.findAll(".thinking-slider-labels span")[2].trigger("click");
    await wrapper.trigger("keydown", { key: "ArrowLeft" });
    expect(wrapper.emitted("select")).toEqual([["high"], ["low"]]);
    await wrapper.setProps({ selectedId: "high" });
    await wrapper.trigger("keydown", { key: "ArrowRight" });
    expect(wrapper.emitted("select")).toHaveLength(2);
    wrapper.unmount();
  });

  it("snaps dragging to the rail and commits the final level on release", async () => {
    const wrapper = mount(ThinkingEffortSlider, {
      props: { options, selectedId: "low", title: "思考强度" },
    });
    const hit = wrapper.get(".thinking-slider-hit").element as HTMLElement;
    hit.setPointerCapture = vi.fn();
    hit.hasPointerCapture = () => true;
    hit.releasePointerCapture = vi.fn();
    const positions = wrapper.get(".thinking-slider-positions").element;
    positions.getBoundingClientRect = () => ({ left: 30, width: 180 }) as DOMRect;
    await wrapper
      .get(".thinking-slider-hit")
      .trigger("pointerdown", { button: 0, pointerId: 1, clientX: 120 });
    expect(wrapper.emitted("select")).toBeUndefined();
    await wrapper
      .get(".thinking-slider-hit")
      .trigger("pointermove", { pointerId: 1, clientX: 240 });
    expect(wrapper.emitted("select")).toBeUndefined();
    await wrapper.get(".thinking-slider-hit").trigger("pointerup", { pointerId: 1 });
    expect(wrapper.emitted("select")).toEqual([["high"]]);
    expect(hit.releasePointerCapture).toHaveBeenCalledWith(1);
    wrapper.unmount();
  });
});
