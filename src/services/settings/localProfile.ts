import { ref } from "vue";
import { useEventListener } from "@vueuse/core";

export const LOCAL_PROFILE_KEY = "anya.local-profile.v1";
export const PROFILE_CHANGED_EVENT = "anya-local-profile-changed";

export function readLocalProfile() {
  try {
    const data = JSON.parse(localStorage.getItem(LOCAL_PROFILE_KEY) || "{}");
    return {
      displayName: typeof data.displayName === "string" ? data.displayName.slice(0, 60) : "",
      handle: typeof data.handle === "string" ? data.handle.slice(0, 40) : "local",
    };
  } catch {
    return { displayName: "", handle: "local" };
  }
}

export function saveLocalProfile(profile: { displayName: string; handle: string }) {
  localStorage.setItem(LOCAL_PROFILE_KEY, JSON.stringify(profile));
  window.dispatchEvent(new Event(PROFILE_CHANGED_EVENT));
}

export function useLocalProfile() {
  const profile = ref(readLocalProfile());
  const refresh = () => {
    profile.value = readLocalProfile();
  };
  useEventListener(window, PROFILE_CHANGED_EVENT, refresh);
  useEventListener(window, "storage", (event: StorageEvent) => {
    if (event.key === LOCAL_PROFILE_KEY || event.key === null) refresh();
  });
  return profile;
}
