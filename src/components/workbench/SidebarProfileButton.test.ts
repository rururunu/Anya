// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { nextTick } from "vue";
import SidebarProfileButton from "./SidebarProfileButton.vue";
import { saveLocalProfile } from "@/services/settings/localProfile";
import { DEFAULT_SETTINGS_CATEGORY } from "@/types/setting";

vi.mock("@/stores/setting", () => ({ useSettingStore: () => ({ language: "zh-CN" }) }));
vi.mock("@/components/chat/UserAvatar.vue", () => ({
  default: { template: "<span class='test-avatar' />" },
}));
const wrappers: ReturnType<typeof mount>[] = [];
beforeEach(() => localStorage.clear());
afterEach(() => wrappers.splice(0).forEach((wrapper) => wrapper.unmount()));

describe("sidebar profile entry", () => {
  it("uses personal profile as the default settings page", () => {
    expect(DEFAULT_SETTINGS_CATEGORY).toBe("profile");
  });
  it("shows the avatar and nickname, updates after profile editing and opens settings", async () => {
    const wrapper = mount(SidebarProfileButton, { props: { settingsLabel: "设置" } });
    wrappers.push(wrapper);
    expect(wrapper.find(".test-avatar").exists()).toBe(true);
    expect(wrapper.get(".sidebar-profile-name").text()).toBe("本地用户");
    saveLocalProfile({ displayName: "新昵称", handle: "local" });
    await nextTick();
    expect(wrapper.get(".sidebar-profile-name").text()).toBe("新昵称");
    await wrapper.get("button").trigger("click");
    expect(wrapper.emitted("open")).toEqual([[]]);
  });
});
