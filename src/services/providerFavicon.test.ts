import { beforeEach, describe, expect, it, vi } from "vitest";

const iconCache = vi.hoisted(() => ({
  peekInstallIcon: vi.fn(() => null),
  lookupInstallIcon: vi.fn(async () => null),
  cacheInstallIcon: vi.fn<(...args: string[]) => Promise<string | null>>(),
}));

vi.mock("@/services/iconCache", () => iconCache);

import { ensureProviderFavicon, peekProviderFavicon } from "./providerFavicon";

describe("provider favicon", () => {
  beforeEach(() => {
    iconCache.peekInstallIcon.mockClear();
    iconCache.lookupInstallIcon.mockClear();
    iconCache.cacheInstallIcon.mockReset();
  });

  it("uses the official website and falls back to its favicon.ico", async () => {
    iconCache.cacheInstallIcon.mockResolvedValueOnce(null).mockResolvedValueOnce("asset://icon");

    ensureProviderFavicon(
      "commandcode",
      "https://api.commandcode.ai/provider/v1/",
      "https://commandcode.ai/",
    );

    await vi.waitFor(() => expect(peekProviderFavicon("commandcode")).toBe("asset://icon"));
    expect(iconCache.cacheInstallIcon.mock.calls.map((call) => call[2])).toEqual([
      "https://www.google.com/s2/favicons?sz=64&domain=commandcode.ai",
      "https://commandcode.ai/favicon.ico",
    ]);
  });

  it("can retry after a failed download", async () => {
    iconCache.cacheInstallIcon.mockResolvedValueOnce(null).mockResolvedValueOnce(null);
    ensureProviderFavicon("retry-provider", "https://api.example.com/v1", "https://example.com");
    await vi.waitFor(() => expect(iconCache.cacheInstallIcon).toHaveBeenCalledTimes(2));

    iconCache.cacheInstallIcon.mockResolvedValueOnce("asset://retry-icon");
    ensureProviderFavicon("retry-provider", "https://api.example.com/v1", "https://example.com");
    await vi.waitFor(() =>
      expect(peekProviderFavicon("retry-provider")).toBe("asset://retry-icon"),
    );
    expect(iconCache.cacheInstallIcon).toHaveBeenCalledTimes(3);
  });
});
