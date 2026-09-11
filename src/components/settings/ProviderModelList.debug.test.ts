/** @vitest-environment jsdom */
import { describe, expect, it } from "vitest";
import { defineComponent } from "vue";
import * as Vue from "vue";
import { mount } from "@vue/test-utils";

const Mini = defineComponent({
  emits: ["remove"],
  template: `<button class="rm" @click="$emit('remove', 1)">x</button>`,
});

describe("debug emits", () => {
  it("probes devtools hook capture of component emits", async () => {
    console.log("vue version:", Vue.version);
    console.log("NODE_ENV:", process.env.NODE_ENV);
    const calls: unknown[][] = [];
    (Vue as unknown as { setDevtoolsHook: (h: unknown, t: unknown) => void }).setDevtoolsHook(
      {
        emit: (eventType: string, ...payload: unknown[]) => {
          calls.push([eventType, ...payload]);
        },
        on: () => () => {},
        off: () => {},
        once: () => () => {},
      },
      {},
    );
    const wrapper = mount(Mini);
    await wrapper.find(".rm").trigger("click");
    console.log("devtools calls:", JSON.stringify(calls.map((c) => c[0])));
    console.log("emitted:", JSON.stringify(wrapper.emitted()));
    expect(true).toBe(true);
  });
});
