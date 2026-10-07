/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { nextTick } from "vue";
import DesktopPetSvg from "./DesktopPetSvg.vue";

describe("desktop SVG pet expressions", () => {
  beforeEach(() => {
    vi.stubGlobal("matchMedia", vi.fn().mockReturnValue({ matches: true }));
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });
  it("reacts to a click with a brief wink while retaining only two eyes", async () => {
    const pet = mount(DesktopPetSvg);
    expect(pet.findAll(".pet-eye")).toHaveLength(2);
    await pet.setProps({ action: "jiggle" });
    expect(pet.findAll(".pet-eye")).toHaveLength(2);
    expect(pet.findAll(".eye-blink.is-blinking")).toHaveLength(1);
    expect(pet.findAll("svg path")).toHaveLength(1);
    pet.unmount();
  });

  it("shows a worried face when pressed against a screen edge", async () => {
    const pet = mount(DesktopPetSvg, { props: { edgePressure: { x: 1, y: 0 } } });
    const pressedPath = pet.find(".pet-body").attributes("d");
    expect(pet.find(".eye-pose").attributes("style")).toContain("5deg");
    await pet.setProps({ edgePressure: { x: 0, y: 0 } });
    expect(pet.find(".pet-body").attributes("d")).not.toBe(pressedPath);
    expect(pet.findAll("svg path")).toHaveLength(1);
    pet.unmount();
  });

  it("responds to a stroke across its head", async () => {
    const pet = mount(DesktopPetSvg);
    Object.defineProperty(pet.element, "getBoundingClientRect", {
      value: () => ({ left: 0, top: 0, width: 155, height: 155 }),
    });
    pet.element.dispatchEvent(
      new MouseEvent("pointermove", { clientX: 35, clientY: 45, buttons: 0 }),
    );
    pet.element.dispatchEvent(
      new MouseEvent("pointermove", { clientX: 105, clientY: 45, buttons: 0 }),
    );
    await nextTick();
    expect(pet.find(".eye-pose").attributes("style")).toContain("scale(1.22, 0.22)");
    expect(pet.findAll("svg path")).toHaveLength(1);
    pet.unmount();
  });
});
