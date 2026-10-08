// @vitest-environment happy-dom
import { expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia } from "pinia";
import FileReadCard from "./FileReadCard.vue";
import { useCodeReadPreviewStore } from "@/stores/codeReadPreview";

it("opens the recorded file/range and expands into a code editor card", async () => {
  const pinia = createPinia();
  const preview = {
    activityId: "read-1",
    path: "src/sample.yml",
    startLine: 368,
    endLine: 370,
    content: "chatZkConfig:\n  enabled: true\n",
  };
  const wrapper = mount(FileReadCard, {
    props: { preview },
    global: {
      plugins: [pinia],
      stubs: {
        ReadCodeEditor: {
          props: ["content", "path", "firstLine"],
          template: '<div class="code-editor-stub">{{ content }}</div>',
        },
      },
    },
  });
  expect(wrapper.text()).toContain("368–370");
  await wrapper.get(".read-file-link").trigger("click");
  expect(useCodeReadPreviewStore(pinia).selection).toEqual(preview);
  await wrapper.get(".read-expand").trigger("click");
  expect(wrapper.find(".code-editor-stub").exists()).toBe(true);
  wrapper.unmount();
});
