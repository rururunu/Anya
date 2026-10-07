// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { reactive } from "vue";
import WorkbenchUpdateDialog from "./WorkbenchUpdateDialog.vue";

const updater = reactive({
  status: "available",
  updateAvailable: true,
  latestVersion: "0.2.27",
  releaseNotes: "Improved input and updater",
  errorMessage: "",
  progress: { phase: "idle", downloadedBytes: 0, totalBytes: 0 },
  install: vi.fn(),
});

vi.mock("@/stores/updater", () => ({ useUpdaterStore: () => updater }));
vi.mock("@/stores/setting", () => ({ useSettingStore: () => ({ language: "en" }) }));

describe("WorkbenchUpdateDialog", () => {
  afterEach(() => {
    document.body.innerHTML = "";
    vi.clearAllMocks();
  });

  it("shows release notes before confirmation and download progress afterward", async () => {
    const host = document.createElement("div");
    document.body.append(host);
    const wrapper = mount(WorkbenchUpdateDialog, { attachTo: host });
    wrapper.vm.open();
    await flushPromises();
    expect(document.body.textContent).toContain("Improved input and updater");

    const updateButton = [...document.querySelectorAll("button")].find((button) =>
      button.textContent?.includes("Update"),
    );
    updateButton?.click();
    expect(updater.install).toHaveBeenCalledTimes(1);

    updater.status = "downloading";
    updater.progress = { phase: "downloading", downloadedBytes: 25, totalBytes: 100 };
    await flushPromises();
    expect(document.body.textContent).toContain("25%");
    expect(document.querySelector('[role="progressbar"]')?.getAttribute("aria-valuenow")).toBe(
      "25",
    );
    wrapper.unmount();
  });
});
