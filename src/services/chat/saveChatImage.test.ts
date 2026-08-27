// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { saveChatImage, suggestedImageFilename, unwrapLocalImagePath } from "./saveChatImage";

const tauriMock = vi.hoisted(() => ({
  save: vi.fn(),
  writeFile: vi.fn(),
  copyFile: vi.fn(),
  readFile: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({ save: tauriMock.save }));
vi.mock("@tauri-apps/plugin-fs", () => ({
  writeFile: tauriMock.writeFile,
  copyFile: tauriMock.copyFile,
  readFile: tauriMock.readFile,
}));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (path: string) => `asset://localhost/${path}`,
}));

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
  tauriMock.copyFile.mockReset();
  tauriMock.readFile.mockReset();
  clearTauri();
  document.body.innerHTML = "";
  vi.unstubAllGlobals();
});

describe("unwrapLocalImagePath", () => {
  it("strips the path: wrapper", () => {
    expect(unwrapLocalImagePath("path:C:\\\\a\\\\cat.png")).toBe("C:\\\\a\\\\cat.png");
  });
});

describe("suggestedImageFilename", () => {
  it("keeps the original stem and extension from a local path", () => {
    expect(suggestedImageFilename("path:C:\\\\Users\\\\a\\\\generated\\\\cat.png")).toBe("cat.png");
    expect(suggestedImageFilename("/tmp/poster.webp")).toBe("poster.webp");
  });

  it("normalizes jpeg to jpg and sanitizes illegal characters", () => {
    expect(suggestedImageFilename('path:D:/tmp/Q1 "Report".jpeg')).toBe("Q1 _Report_.jpg");
  });

  it("uses the data URL mime subtype", () => {
    expect(suggestedImageFilename("data:image/webp;base64,abc", "shot")).toBe("shot.webp");
  });

  it("falls back to png when the source has no image extension", () => {
    expect(suggestedImageFilename("https://cdn.example/file", "art")).toBe("file.png");
  });
});

describe("saveChatImage", () => {
  it("copies a local file through the native save dialog in Tauri", async () => {
    setTauri();
    tauriMock.save.mockResolvedValue("C:\\\\Pictures\\\\cat.png");
    tauriMock.copyFile.mockResolvedValue(undefined);

    expect(await saveChatImage("path:C:\\\\Users\\\\a\\\\generated\\\\cat.png")).toBe("saved");
    expect(tauriMock.save).toHaveBeenCalledWith({
      defaultPath: "cat.png",
      filters: [{ name: "PNG Image", extensions: ["png"] }],
    });
    expect(tauriMock.copyFile).toHaveBeenCalledWith(
      "C:\\\\Users\\\\a\\\\generated\\\\cat.png",
      "C:\\\\Pictures\\\\cat.png",
    );
    expect(tauriMock.writeFile).not.toHaveBeenCalled();
  });

  it("treats a cancelled save dialog as cancelled, not failed", async () => {
    setTauri();
    tauriMock.save.mockResolvedValue(null);

    expect(await saveChatImage("path:C:\\\\a.png")).toBe("cancelled");
    expect(tauriMock.copyFile).not.toHaveBeenCalled();
  });

  it("writes decoded data URL bytes when copy is not applicable", async () => {
    setTauri();
    tauriMock.save.mockResolvedValue("C:\\\\Pictures\\\\shot.png");

    expect(await saveChatImage("data:image/png;base64,aGVsbG8=", "shot")).toBe("saved");
    expect(tauriMock.copyFile).not.toHaveBeenCalled();
    expect(tauriMock.writeFile).toHaveBeenCalledTimes(1);
    const [path, bytes] = tauriMock.writeFile.mock.calls[0] as [string, Uint8Array];
    expect(path).toBe("C:\\\\Pictures\\\\shot.png");
    expect(Array.from(bytes)).toEqual([104, 101, 108, 108, 111]);
  });

  it("falls back to writing fetched bytes when copyFile fails", async () => {
    setTauri();
    tauriMock.save.mockResolvedValue("D:\\\\out.png");
    tauriMock.copyFile.mockRejectedValue(new Error("denied"));
    tauriMock.readFile.mockRejectedValue(new Error("denied"));
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        arrayBuffer: async () => new Uint8Array([1, 2, 3]).buffer,
      }),
    );

    expect(await saveChatImage("path:C:\\\\a\\\\cat.png")).toBe("saved");
    expect(tauriMock.writeFile).toHaveBeenCalledTimes(1);
  });

  it("returns failed when writing throws", async () => {
    setTauri();
    tauriMock.save.mockResolvedValue("C:\\\\out.png");
    tauriMock.copyFile.mockRejectedValue(new Error("denied"));
    tauriMock.readFile.mockRejectedValue(new Error("denied"));
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("offline")));

    expect(await saveChatImage("path:C:\\\\a.png")).toBe("failed");
  });

  it("downloads via an anchor in a plain browser context", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        arrayBuffer: async () => new Uint8Array([9]).buffer,
      }),
    );

    expect(await saveChatImage("https://cdn.example/cat.png")).toBe("saved");
    expect(clickSpy).toHaveBeenCalledTimes(1);
    expect(tauriMock.save).not.toHaveBeenCalled();
  });
});
