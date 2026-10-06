import { Avatar, Style } from "@dicebear/core";
import notionists from "@dicebear/styles/notionists-neutral.json";
import adventurer from "@dicebear/styles/adventurer.json";
import pixelArt from "@dicebear/styles/pixel-art.json";
import bottts from "@dicebear/styles/bottts-neutral.json";
import emoji from "@dicebear/styles/fun-emoji.json";
import identicon from "@dicebear/styles/identicon.json";
import lorelei from "@dicebear/styles/lorelei.json";
import micah from "@dicebear/styles/micah.json";
import openPeeps from "@dicebear/styles/open-peeps.json";
import thumbs from "@dicebear/styles/thumbs.json";
import shapes from "@dicebear/styles/shapes.json";
import rings from "@dicebear/styles/rings.json";
import { ref } from "vue";
import { useEventListener } from "@vueuse/core";

const SEED_KEY = "anya.user-avatar-seed.v1";
const IMAGE_KEY = "anya.user-avatar-image.v1";
const STYLE_KEY = "anya.user-avatar-style.v1";
const OPTIONS_KEY = "anya.user-avatar-options.v1";
export type UserAvatarOptions = Record<string, string | number>;
const styles = {
  notionists,
  adventurer,
  pixelArt,
  bottts,
  emoji,
  identicon,
  lorelei,
  micah,
  openPeeps,
  thumbs,
  shapes,
  rings,
};
export type UserAvatarStyle = keyof typeof styles;
export const userAvatarStyles: UserAvatarStyle[] = [
  "notionists",
  "adventurer",
  "pixelArt",
  "bottts",
  "emoji",
  "identicon",
  "lorelei",
  "micah",
  "openPeeps",
  "thumbs",
  "shapes",
  "rings",
];
export function getUserAvatarStyle(): UserAvatarStyle {
  try {
    const stored = localStorage.getItem(STYLE_KEY);
    if (stored && userAvatarStyles.includes(stored as UserAvatarStyle))
      return stored as UserAvatarStyle;
  } catch {
    /* Keep the default style. */
  }
  return "notionists";
}
export function avatarStyleControls(style: UserAvatarStyle): Record<string, string[]> {
  const definition = styles[style] as {
    colors?: Record<string, unknown>;
    components?: Record<string, { variants?: Record<string, unknown> }>;
  };
  const result: Record<string, string[]> = { backgroundColor: ["six-digit hex color"] };
  for (const name of Object.keys(definition.colors ?? {}))
    result[`${name}Color`] = ["six-digit hex color"];
  for (const [name, component] of Object.entries(definition.components ?? {})) {
    if (component.variants) result[`${name}Variant`] = Object.keys(component.variants);
  }
  return result;
}
export function sanitizeAvatarOptions(style: UserAvatarStyle, value: unknown): UserAvatarOptions {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  const controls = avatarStyleControls(style);
  const result: UserAvatarOptions = {};
  for (const [key, option] of Object.entries(value)) {
    if (!controls[key] || typeof option !== "string") continue;
    if (key.endsWith("Color") && /^#?[0-9a-f]{6}$/i.test(option))
      result[key] = `#${option.replace(/^#/, "")}`;
    else if (key.endsWith("Variant") && controls[key].includes(option)) result[key] = option;
  }
  return result;
}
export function getUserAvatarOptions(): UserAvatarOptions {
  try {
    return sanitizeAvatarOptions(
      getUserAvatarStyle(),
      JSON.parse(localStorage.getItem(OPTIONS_KEY) || "{}"),
    );
  } catch {
    return {};
  }
}
export function avatarStylePreview(
  style: UserAvatarStyle,
  seed = "anya-preview",
  options: UserAvatarOptions = {},
): string {
  const avatar = new Avatar(new Style(styles[style]), {
    ...sanitizeAvatarOptions(style, options),
    seed,
    size: 64,
  });
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(avatar.toString())}`;
}
const CHANGE_EVENT = "anya-user-avatar-changed";
let cachedAvatar: string | undefined;

/** Generate the user's avatar locally and keep it consistent across windows. */
export function userAvatarDataUri(): string {
  try {
    const custom = localStorage.getItem(IMAGE_KEY);
    if (custom && /^data:image\/(png|jpeg|webp);base64,/.test(custom)) return custom;
  } catch {
    /* Use the generated avatar when storage is unavailable. */
  }
  if (cachedAvatar) return cachedAvatar;
  let seed: string;
  try {
    seed = localStorage.getItem(SEED_KEY) || crypto.randomUUID();
    localStorage.setItem(SEED_KEY, seed);
  } catch {
    seed = crypto.randomUUID();
  }
  cachedAvatar = avatarStylePreview(getUserAvatarStyle(), seed, getUserAvatarOptions());
  return cachedAvatar;
}

export function setUserAvatar(image: string): void {
  if (!/^data:image\/(png|jpeg|webp);base64,/.test(image) || image.length > 1500000) {
    throw new Error("Invalid avatar image");
  }
  localStorage.setItem(IMAGE_KEY, image);
  window.dispatchEvent(new Event(CHANGE_EVENT));
}

export function randomizeUserAvatar(style = getUserAvatarStyle()): void {
  setSeedUserAvatar(style, crypto.randomUUID());
}

export function getUserAvatarSeed(): string {
  try {
    return localStorage.getItem(SEED_KEY) ?? "";
  } catch {
    return "";
  }
}

export function setSeedUserAvatar(
  style: UserAvatarStyle,
  seed: string,
  options: UserAvatarOptions = {},
): void {
  if (!userAvatarStyles.includes(style)) throw new Error("Unknown avatar style");
  if (!seed.trim() || seed.trim().length > 128) throw new Error("Invalid avatar seed");
  localStorage.setItem(STYLE_KEY, style);
  localStorage.setItem(SEED_KEY, seed.trim());
  localStorage.setItem(OPTIONS_KEY, JSON.stringify(sanitizeAvatarOptions(style, options)));
  localStorage.removeItem(IMAGE_KEY);
  cachedAvatar = undefined;
  window.dispatchEvent(new Event(CHANGE_EVENT));
}

/** Keep profile and chat avatars in sync, including other application windows. */
export function useUserAvatar() {
  const source = ref(userAvatarDataUri());
  const refresh = () => {
    cachedAvatar = undefined;
    source.value = userAvatarDataUri();
  };
  useEventListener(window, CHANGE_EVENT, refresh);
  useEventListener(window, "storage", (event: StorageEvent) => {
    if (
      event.key === IMAGE_KEY ||
      event.key === SEED_KEY ||
      event.key === STYLE_KEY ||
      event.key === OPTIONS_KEY ||
      event.key === null
    )
      refresh();
  });
  return source;
}
