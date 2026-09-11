import { onBeforeUnmount, ref, watch, type Ref } from "vue";
import { isLocalImagePath, loadLocalImageObjectUrl } from "@/services/chat/localImageSrc";

/**
 * Resolve a wallpaper / theme background source for CSS or `<img>`.
 * Local files are read with plugin-fs so they work outside assetProtocol scope.
 */
export function useResolvedBackgroundSrc(
  source: Ref<string | undefined> | (() => string | undefined),
) {
  const resolvedSource = ref("");
  let token = 0;
  let ownedBlob = "";

  function release() {
    if (!ownedBlob) return;
    URL.revokeObjectURL(ownedBlob);
    ownedBlob = "";
  }

  watch(
    typeof source === "function" ? source : () => source.value,
    async (next) => {
      const value = (next ?? "").trim();
      const current = ++token;
      release();
      if (!value) {
        resolvedSource.value = "";
        return;
      }
      if (value.startsWith("data:") || value.startsWith("blob:") || /^https?:\/\//i.test(value)) {
        resolvedSource.value = value;
        return;
      }
      if (!isLocalImagePath(value)) {
        resolvedSource.value = value;
        return;
      }
      resolvedSource.value = "";
      try {
        const url = await loadLocalImageObjectUrl(value);
        if (current !== token) {
          if (url.startsWith("blob:")) URL.revokeObjectURL(url);
          return;
        }
        ownedBlob = url.startsWith("blob:") ? url : "";
        resolvedSource.value = url;
      } catch {
        if (current === token) resolvedSource.value = "";
      }
    },
    { immediate: true },
  );

  onBeforeUnmount(release);

  return { resolvedSource };
}
