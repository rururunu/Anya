import { defineStore } from "pinia";

export type SubagentSessionRecord = {
  sessionId: string;
  parentSessionId: string;
  entryId: string;
  preview: string;
  workspaceId?: string | null;
  visible: boolean;
};

export const useSubagentSessionStore = defineStore("subagentSessions", {
  state: () => ({
    records: {} as Record<string, SubagentSessionRecord>,
  }),
  getters: {
    visibleRecords: (state) => Object.values(state.records).filter((record) => record.visible),
  },
  actions: {
    upsert(record: Omit<SubagentSessionRecord, "visible"> & { visible?: boolean }) {
      const existing = this.records[record.sessionId];
      this.records[record.sessionId] = {
        ...record,
        visible: record.visible ?? existing?.visible ?? true,
        preview: record.preview || existing?.preview || "",
      };
    },
    hide(sessionId: string) {
      const record = this.records[sessionId];
      if (!record) return;
      record.visible = false;
    },
    show(sessionId: string) {
      const record = this.records[sessionId];
      if (!record) return;
      record.visible = true;
    },
    remove(sessionId: string) {
      delete this.records[sessionId];
    },
    visibleForParent(parentSessionId: string) {
      return this.visibleRecords.filter((record) => record.parentSessionId === parentSessionId);
    },
  },
});
