// @vitest-environment happy-dom
import { beforeEach, describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { nextTick, reactive } from "vue";
import AboutSettings from "./AboutSettings.vue";

const updater = reactive({
  status: "available",
  updateAvailable: true,
  isBusy: false,
  latestVersion: "0.2.27",
  releaseNotes: "Preview notes",
  errorMessage: "",
  progress: { phase: "idle", downloadedBytes: 0, totalBytes: 0 },
  check: vi.fn(),
  resetTransientError: vi.fn(),
});

vi.mock("@/stores/updater", () => ({ useUpdaterStore: () => updater }));
vi.mock("@/stores/setting", () => ({ useSettingStore: () => ({ language: "en" }) }));

describe("AboutSettings updater", () => {
  beforeEach(() => {
    updater.status = "available";
    updater.isBusy = false;
    updater.progress = { phase: "idle", downloadedBytes: 0, totalBytes: 0 };
  });

  it("shows actual download progress in the About update section", async () => {
    const wrapper = mount(AboutSettings, {
      props: { name: "Anya", version: "0.2.26", identifier: "test" },
      global: { stubs: { WorkbenchUpdateDialog: true, MascotFace: true } },
    });
    expect(wrapper.find('.about-update [role="progressbar"]').exists()).toBe(false);

    updater.status = "downloading";
    updater.isBusy = true;
    updater.progress = { phase: "downloading", downloadedBytes: 25, totalBytes: 100 };
    await nextTick();
    expect(wrapper.find('.about-update [role="progressbar"]').attributes("aria-valuenow")).toBe(
      "25",
    );

    updater.progress = { phase: "installing", downloadedBytes: 100, totalBytes: 100 };
    await nextTick();
    expect(wrapper.find('.about-update [role="progressbar"]').attributes("aria-valuenow")).toBe(
      "100",
    );
    wrapper.unmount();
  });
});
