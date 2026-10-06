import { computed, ref } from "vue";
import { normalizeChatMode, type ChatMode } from "@/types/setting";

/** Keep an explicit pre-chat choice without making Plan the app-wide default. */
export function useSessionlessChatMode(defaultMode: () => unknown) {
  const pending = ref<ChatMode | null>(null);
  const mode = computed(() => pending.value ?? normalizeChatMode(defaultMode()));
  function choose(next: ChatMode) {
    pending.value = next;
  }
  function reset() {
    pending.value = null;
  }
  return { mode, choose, reset };
}
