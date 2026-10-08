import { onMounted, onUnmounted, watch, type Ref } from "vue";
import { rootSessionId } from "@/services/chat/subagentSession";

export interface SharedReviewState {
  images: string[];
  selectedImage: string;
  subagents: string[];
  selectedSubagent: string;
  view: string;
  imageOpen: boolean;
}
type Bindings = { [K in keyof SharedReviewState]?: Ref<SharedReviewState[K]> };
const prefix = "anya.shared-review.v1.";

/** Synchronize only bound fields; each window keeps its own dimensions/layout. */
export function useSharedReviewState(
  session: () => string,
  bindings: Bindings,
  restored?: (value: Partial<SharedReviewState>) => void,
) {
  let loading = false;
  const key = () => prefix + rootSessionId(session());
  function read(): Partial<SharedReviewState> {
    try {
      return JSON.parse(localStorage.getItem(key()) ?? "{}");
    } catch {
      return {};
    }
  }
  function load() {
    loading = true;
    const value = read();
    for (const [field, target] of Object.entries(bindings)) {
      const item = value[field as keyof SharedReviewState];
      if (
        item !== undefined &&
        typeof item === typeof target.value &&
        Array.isArray(item) === Array.isArray(target.value) &&
        (!Array.isArray(item) || item.every((entry) => typeof entry === "string"))
      ) {
        target.value = item;
      } else {
        target.value = Array.isArray(target.value)
          ? []
          : typeof target.value === "boolean"
            ? false
            : field === "view"
              ? "diff"
              : "";
      }
    }
    restored?.(value);
    loading = false;
  }
  watch(() => rootSessionId(session()), load, { immediate: true, flush: "sync" });
  watch(
    () =>
      Object.fromEntries(Object.entries(bindings).map(([field, target]) => [field, target.value])),
    (value) => {
      if (loading || !session()) return;
      const serialized = JSON.stringify({ ...read(), ...value });
      if (localStorage.getItem(key()) !== serialized) localStorage.setItem(key(), serialized);
    },
    { deep: true, flush: "sync" },
  );
  const storage = (event: StorageEvent) => {
    if (event.key === key()) load();
  };
  onMounted(() => {
    globalThis.addEventListener("storage", storage);
    globalThis.addEventListener("focus", load);
  });
  onUnmounted(() => {
    globalThis.removeEventListener("storage", storage);
    globalThis.removeEventListener("focus", load);
  });
  return { load };
}
