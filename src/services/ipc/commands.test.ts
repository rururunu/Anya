import { afterEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { getAppSettings } from "./commands";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

afterEach(() => {
  vi.useRealTimers();
  vi.resetAllMocks();
});

describe("settings startup readiness", () => {
  it("waits for backend initialization and returns persisted themes", async () => {
    vi.useFakeTimers();
    const settings = {
      colorScheme: "macos-sonoma-light",
      customThemes: [{ id: "macos-sonoma-light", name: "macOS", mode: "light", tokens: {} }],
    };
    vi.mocked(invoke)
      .mockRejectedValueOnce("settings state is unavailable")
      .mockRejectedValueOnce("settings state is unavailable")
      .mockResolvedValueOnce(settings);
    const pending = getAppSettings();
    await vi.advanceTimersByTimeAsync(100);
    expect(await pending).toEqual(settings);
    expect(invoke).toHaveBeenCalledTimes(3);
  });

  it("propagates unrelated failures without retrying", async () => {
    vi.mocked(invoke).mockRejectedValueOnce("settings file is unreadable");
    await expect(getAppSettings()).rejects.toBe("settings file is unreadable");
    expect(invoke).toHaveBeenCalledTimes(1);
  });

  it("stops retrying when initialization never completes", async () => {
    vi.useFakeTimers();
    vi.mocked(invoke).mockRejectedValue("settings state is unavailable");
    const result = expect(getAppSettings()).rejects.toBe("settings state is unavailable");
    await vi.advanceTimersByTimeAsync(5000);
    await result;
    expect(invoke).toHaveBeenCalledTimes(101);
    expect(vi.getTimerCount()).toBe(0);
  });
});
