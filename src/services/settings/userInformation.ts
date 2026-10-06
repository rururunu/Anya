import type { AppSettings, AppSettingsPatch } from "@/types/setting";
import { PROFILE_CHANGED_EVENT } from "./localProfile";
import {
  getUserAvatarStyle,
  getUserAvatarSeed,
  getUserAvatarOptions,
  userAvatarStyles,
  sanitizeAvatarOptions,
  type UserAvatarStyle,
  type UserAvatarOptions,
} from "@/services/chat/userAvatar";
import { isLocalImagePath } from "@/services/chat/localImageSrc";
import { loadImageSourceAsDataUrl } from "@/services/chat/imageEditReference";

export const userInformationKeys = [
  "anya.local-profile.v1",
  "anya.user-avatar-seed.v1",
  "anya.user-avatar-style.v1",
  "anya.user-avatar-options.v1",
  "anya.user-avatar-image.v1",
  "anya.token-usage-preferences.v1",
] as const;
export interface UsagePreferences {
  range: string;
  granularity: "day" | "week" | "month";
  customFrom: string;
  customTo: string;
}
export function usagePreferences(value: unknown): UsagePreferences {
  const data = value && typeof value === "object" ? (value as Record<string, unknown>) : {};
  return {
    range: ["today", "7d", "30d", "month", "custom"].includes(String(data.range))
      ? String(data.range)
      : "30d",
    granularity:
      data.granularity === "week" || data.granularity === "month" ? data.granularity : "day",
    customFrom:
      typeof data.customFrom === "string" && /^\d{4}-\d{2}-\d{2}$/.test(data.customFrom)
        ? data.customFrom
        : "",
    customTo:
      typeof data.customTo === "string" && /^\d{4}-\d{2}-\d{2}$/.test(data.customTo)
        ? data.customTo
        : "",
  };
}
export function readUsagePreferences(): UsagePreferences {
  try {
    return usagePreferences(JSON.parse(localStorage.getItem(userInformationKeys[5]) || "{}"));
  } catch {
    return usagePreferences({});
  }
}
export interface UserInformation {
  format: "anya-user-information";
  version: 1;
  exportedAt: string;
  settings: AppSettingsPatch;
  profile: {
    displayName: string;
    handle: string;
    avatar: {
      style: UserAvatarStyle;
      seed: string;
      options: UserAvatarOptions;
      image: string | null;
    };
  };
  tokenUsagePreferences: UsagePreferences;
}
export async function exportUserInformation(settings: AppSettings): Promise<UserInformation> {
  const portable = JSON.parse(JSON.stringify(settings)) as AppSettingsPatch;
  portable.customBackground ??= null;
  const backgrounds = [
    portable.customBackground,
    ...(portable.customThemes ?? []).map((theme) => theme.background),
  ];
  for (const background of backgrounds) {
    if (background?.image && isLocalImagePath(background.image))
      background.image = await loadImageSourceAsDataUrl(background.image);
  }
  const profile = JSON.parse(localStorage.getItem(userInformationKeys[0]) || "{}");
  return {
    format: "anya-user-information",
    version: 1,
    exportedAt: new Date().toISOString(),
    settings: portable,
    profile: {
      displayName: typeof profile.displayName === "string" ? profile.displayName : "",
      handle: typeof profile.handle === "string" ? profile.handle : "local",
      avatar: {
        style: getUserAvatarStyle(),
        seed: getUserAvatarSeed() || crypto.randomUUID(),
        options: getUserAvatarOptions(),
        image: localStorage.getItem(userInformationKeys[4]),
      },
    },
    tokenUsagePreferences: readUsagePreferences(),
  };
}
function object(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new Error("Invalid user information format");
  return value as Record<string, unknown>;
}
export function parseUserInformation(text: string, current: AppSettings): UserInformation {
  const data = object(JSON.parse(text));
  if (data.format !== "anya-user-information" || data.version !== 1)
    throw new Error("Unsupported user information file or version");
  const settings = object(data.settings);
  const patch: Record<string, unknown> = {};
  for (const key of [...Object.keys(current), "customBackground"]) {
    if (!(key in settings)) continue;
    const value = settings[key];
    const expected = current[key as keyof AppSettings];
    if (
      key !== "customBackground" &&
      expected != null &&
      (typeof value !== typeof expected || Array.isArray(value) !== Array.isArray(expected))
    )
      throw new Error(`Invalid setting: ${key}`);
    patch[key] = value;
  }
  if (!Object.keys(patch).length) throw new Error("No settings found in the file");
  const profile = object(data.profile);
  const avatar = object(profile.avatar);
  if (
    typeof profile.displayName !== "string" ||
    profile.displayName.length > 60 ||
    typeof profile.handle !== "string" ||
    profile.handle.length > 40 ||
    typeof avatar.seed !== "string" ||
    !avatar.seed.trim() ||
    avatar.seed.length > 128 ||
    !userAvatarStyles.includes(avatar.style as UserAvatarStyle)
  )
    throw new Error("Invalid profile or avatar");
  if (
    avatar.image != null &&
    (typeof avatar.image !== "string" ||
      avatar.image.length > 1500000 ||
      !/^data:image\/(png|jpeg|webp);base64,/.test(avatar.image))
  )
    throw new Error("Invalid avatar image");
  return {
    format: "anya-user-information",
    version: 1,
    exportedAt: String(data.exportedAt ?? ""),
    settings: patch as AppSettingsPatch,
    profile: {
      displayName: profile.displayName,
      handle: profile.handle,
      avatar: {
        style: avatar.style as UserAvatarStyle,
        seed: avatar.seed.trim(),
        options: sanitizeAvatarOptions(avatar.style as UserAvatarStyle, avatar.options),
        image: (avatar.image as string | null) ?? null,
      },
    },
    tokenUsagePreferences: usagePreferences(data.tokenUsagePreferences),
  };
}
export function informationStorage(data: UserInformation): (string | null)[] {
  return [
    JSON.stringify({ displayName: data.profile.displayName, handle: data.profile.handle }),
    data.profile.avatar.seed,
    data.profile.avatar.style,
    JSON.stringify(data.profile.avatar.options),
    data.profile.avatar.image,
    JSON.stringify(data.tokenUsagePreferences),
  ];
}
export function applyInformationStorage(values: (string | null)[]) {
  userInformationKeys.forEach((key, index) => {
    if (values[index] == null) localStorage.removeItem(key);
    else localStorage.setItem(key, values[index]!);
  });
  window.dispatchEvent(new Event("anya-user-avatar-changed"));
  window.dispatchEvent(new Event(PROFILE_CHANGED_EVENT));
}
