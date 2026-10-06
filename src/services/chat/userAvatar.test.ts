// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { effectScope, nextTick } from "vue";

afterEach(() => {
  localStorage.removeItem("anya.user-avatar-seed.v1");
  localStorage.removeItem("anya.user-avatar-image.v1");
  localStorage.removeItem("anya.user-avatar-style.v1");
  localStorage.removeItem("anya.user-avatar-options.v1");
  vi.unstubAllGlobals();
});

describe("local user avatar", () => {
  it("uses real hair/skin controls and saves the exact preview configuration", async () => {
    const { setSeedUserAvatar, avatarStylePreview, userAvatarDataUri, getUserAvatarOptions } =
      await import("./userAvatar");
    const options = { hairColor: "#ab2a18", skinColor: "#f2d3b1", hairVariant: "long01" };
    const preview = avatarStylePreview("adventurer", "red-girl-01", options);
    expect(decodeURIComponent(preview)).toContain("#ab2a18");
    expect(decodeURIComponent(preview)).toContain("#f2d3b1");
    setSeedUserAvatar("adventurer", "red-girl-01", options);
    expect(userAvatarDataUri()).toBe(preview);
    expect(getUserAvatarOptions()).toEqual(options);
  });
  it("reproduces the same avatar from a manually entered seed", async () => {
    const { setSeedUserAvatar, getUserAvatarSeed, userAvatarDataUri } =
      await import("./userAvatar");
    setSeedUserAvatar("pixelArt", "my-avatar");
    const first = userAvatarDataUri();
    setSeedUserAvatar("pixelArt", "my-avatar");
    expect(userAvatarDataUri()).toBe(first);
    expect(getUserAvatarSeed()).toBe("my-avatar");
  });
  it("renders every style locally and persists the selected random style", async () => {
    const {
      userAvatarStyles,
      avatarStylePreview,
      randomizeUserAvatar,
      getUserAvatarStyle,
      userAvatarDataUri,
    } = await import("./userAvatar");
    for (const style of userAvatarStyles) {
      expect(decodeURIComponent(avatarStylePreview(style))).toContain("<svg");
    }
    randomizeUserAvatar("bottts");
    expect(getUserAvatarStyle()).toBe("bottts");
    expect(userAvatarDataUri()).toBe(
      avatarStylePreview("bottts", localStorage.getItem("anya.user-avatar-seed.v1")!),
    );
  });
  it("updates existing avatar views when an image is saved and when randomized", async () => {
    const { setUserAvatar, randomizeUserAvatar, useUserAvatar } = await import("./userAvatar");
    const scope = effectScope();
    const source = scope.run(() => useUserAvatar())!;
    await nextTick();
    const custom = "data:image/png;base64,aGVsbG8=";
    setUserAvatar(custom);
    expect(source.value).toBe(custom);
    expect(localStorage.getItem("anya.user-avatar-image.v1")).toBe(custom);
    randomizeUserAvatar();
    expect(source.value).toContain("data:image/svg+xml");
    expect(localStorage.getItem("anya.user-avatar-image.v1")).toBeNull();
    scope.stop();
  });
  it("rejects unsupported image sources without overwriting an avatar", async () => {
    const { setUserAvatar } = await import("./userAvatar");
    expect(() => setUserAvatar("https://example.com/avatar.png")).toThrow();
    expect(localStorage.getItem("anya.user-avatar-image.v1")).toBeNull();
  });
  it("keeps the same randomly seeded avatar after reopening a window", async () => {
    vi.resetModules();
    const fetch = vi.fn();
    vi.stubGlobal("fetch", fetch);
    const { userAvatarDataUri } = await import("./userAvatar");
    const first = userAvatarDataUri();
    const seed = localStorage.getItem("anya.user-avatar-seed.v1");
    expect(seed).toBeTruthy();
    expect(decodeURIComponent(first)).toContain("<svg");
    expect(userAvatarDataUri()).toBe(first);
    vi.resetModules();
    const reopened = await import("./userAvatar");
    expect(reopened.userAvatarDataUri()).toBe(first);
    expect(localStorage.getItem("anya.user-avatar-seed.v1")).toBe(seed);
    expect(fetch).not.toHaveBeenCalled();
  });
});
