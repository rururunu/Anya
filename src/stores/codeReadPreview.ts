import { defineStore } from "pinia";
import type { ReadCodePreview } from "@/services/chat/readCodePreview";

/** Local window preview; never changes the agent's context or edits the file. */
export const useCodeReadPreviewStore = defineStore("codeReadPreview", {
  state: () => ({ selection: null as ReadCodePreview | null }),
  actions: {
    open(selection: ReadCodePreview) {
      this.selection = { ...selection };
    },
    clear() {
      this.selection = null;
    },
  },
});
