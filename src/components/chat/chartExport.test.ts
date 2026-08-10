// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { dataUrlToBytes, exportChartPng, isTauri, safeFilename } from "./chartExport";

const tauriMock = vi.hoisted(() => ({
  save: vi.fn(),
  writeFile: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({ save: tauriMock.save }));
vi.mock("@tauri-apps/plugin-fs", () => ({ writeFile: tauriMock.writeFile }));

const clickSpy = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => undefined);

function setTauri() {
  (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
}

function clearTauri() {
  delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
}

afterEach(() => {
  clickSpy.mockClear();
  tauriMock.save.mockReset();
  tauriMock.writeFile.mockReset();
  clearTauri();
  document.body.innerHTML = "";
});

describe("safeFilename", () => {
  it("replaces characters that are illegal in filenames", () => {
    expect(safeFilename('Q1 "Report": A/B*C?')).toBe("Q1 _Report__ A_B_C_");
  });

  it("falls back to a default name when empty", () => {
    expect(safeFilename("   ")).toBe("chart");
    expect(safeFilename("")).toBe("chart");
  });

  it("caps over-long names", () => {
    expect(safeFilename("x".repeat(300))).toHaveLength(120);
  });
});

describe("dataUrlToBytes", () => {
  it("decodes base64 png data urls into raw bytes", () => {
    const bytes = dataUrlToBytes("data:image/png;base64,aGVsbG8=");
    expect(Array.from(bytes)).toEqual([104, 101, 108, 108, 111]);
  });
});

describe("isTauri", () => {
  it("is false in a plain browser context", () => {
    expect(isTauri()).toBe(false);
  });

  it("is true when the Tauri bridge is present", () => {
    setTauri();
    expect(isTauri()).toBe(true);
  });
});

describe("exportChartPng", () => {
  function fakeChart(url: string) {
    return { getDataURL: vi.fn().mockReturnValue(url) };
  }

  it("downloads a PNG with the sanitized title as filename (browser fallback)", async () => {
    const chart = fakeChart("data:image/png;base64,abc");
    const ok = await exportChartPng(chart, '月度 "报表"', {
      pixelRatio: 2,
      backgroundColor: "#f8f8f8",
    });

    expect(ok).toBe(true);
    expect(chart.getDataURL).toHaveBeenCalledWith({
      type: "png",
      pixelRatio: 2,
      backgroundColor: "#f8f8f8",
    });
    expect(clickSpy).toHaveBeenCalledTimes(1);

    const clickedAnchor = clickSpy.mock.contexts[0] as HTMLAnchorElement;
    expect(clickedAnchor).toBeTruthy();
    expect(clickedAnchor.getAttribute("href")).toBe("data:image/png;base64,abc");
    expect(clickedAnchor.getAttribute("download")).toBe("月度 _报表_.png");
  });

  it("passes through an undefined pixelRatio (3D export path)", async () => {
    const chart = fakeChart("data:image/png;base64,abc");
    await exportChartPng(chart, "3d chart");
    expect(chart.getDataURL).toHaveBeenCalledWith(
      expect.objectContaining({ type: "png", pixelRatio: undefined }),
    );
  });

  it("returns false and does not click when getDataURL fails", async () => {
    const chart = {
      getDataURL: vi.fn(() => {
        throw new Error("canvas tainted");
      }),
    };
    expect(await exportChartPng(chart, "x")).toBe(false);
    expect(clickSpy).not.toHaveBeenCalled();
  });

  it("returns false when getDataURL returns an empty url", async () => {
    const chart = fakeChart("");
    expect(await exportChartPng(chart, "x")).toBe(false);
    expect(clickSpy).not.toHaveBeenCalled();
  });

  it("saves through the native dialog in Tauri instead of clicking an anchor", async () => {
    setTauri();
    tauriMock.save.mockResolvedValue("C:\\Pictures\\chart.png");
    const chart = fakeChart("data:image/png;base64,aGVsbG8=");

    const ok = await exportChartPng(chart, "sales");

    expect(ok).toBe(true);
    expect(clickSpy).not.toHaveBeenCalled();
    expect(tauriMock.save).toHaveBeenCalledWith({
      defaultPath: "sales.png",
      filters: [{ name: "PNG Image", extensions: ["png"] }],
    });
    expect(tauriMock.writeFile).toHaveBeenCalledTimes(1);
    const [path, bytes] = tauriMock.writeFile.mock.calls[0] as [string, Uint8Array];
    expect(path).toBe("C:\\Pictures\\chart.png");
    expect(Array.from(bytes)).toEqual([104, 101, 108, 108, 111]);
  });

  it("treats a cancelled save dialog as a success (no file written)", async () => {
    setTauri();
    tauriMock.save.mockResolvedValue(null);

    expect(await exportChartPng(fakeChart("data:image/png;base64,abc"), "x")).toBe(true);
    expect(tauriMock.writeFile).not.toHaveBeenCalled();
  });

  it("returns false when writing the file fails", async () => {
    setTauri();
    tauriMock.save.mockResolvedValue("C:\\Pictures\\chart.png");
    tauriMock.writeFile.mockRejectedValue(new Error("permission denied"));

    expect(await exportChartPng(fakeChart("data:image/png;base64,abc"), "x")).toBe(false);
    expect(clickSpy).not.toHaveBeenCalled();
  });
});
