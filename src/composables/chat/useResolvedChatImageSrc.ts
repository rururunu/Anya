import { ref, watch, type Ref } from "vue";
import { isLocalImagePath, resolveChatImageSrc } from "@/services/chat/localImageSrc";
import { loadImageSourceAsDataUrl } from "@/services/chat/imageEditReference";

/**
 * Resolve a chat image source (`path:`, data URL, http) into something `<img>` can load.
 * Local paths use plugin-fs → data URL so companion uploads outside assetProtocol still show.
 */
export function useResolvedChatImageSrc(source: Ref<string> | (() => string)) {
  const resolvedSource = ref("");
  let token = 0;

  watch(
    typeof source === "function" ? source : () => source.value,
    async (next) => {
      const value = (next ?? "").trim();
      const current = ++token;
      if (!value) {
        resolvedSource.value = "";
        return;
      }
      if (value.startsWith("data:") || /^https?:\/\//i.test(value)) {
        resolvedSource.value = value;
        return;
      }
      if (isLocalImagePath(value)) {
        resolvedSource.value = "";
        try {
          const dataUrl = await loadImageSourceAsDataUrl(value);
          if (current === token) resolvedSource.value = dataUrl;
        } catch {
          if (current === token) resolvedSource.value = resolveChatImageSrc(value);
        }
        return;
      }
      resolvedSource.value = resolveChatImageSrc(value);
    },
    { immediate: true },
  );

  return { resolvedSource };
}
