import { computed, ref } from "vue";
import { getPendingInteractions } from "@/services/ipc";
import type { PendingInteraction } from "@/composables/workbench/types";

/** Backend queue projection; resolved ids suppress late events and snapshots. */
export function usePendingInteractions() {
  const queues = ref<Record<string, PendingInteraction[]>>({});
  const resolved = new Set<string>();
  const revisions = new Map<string, number>();
  const pendingInteractions = computed(() =>
    Object.fromEntries(
      Object.entries(queues.value)
        .filter(([, items]) => items.length)
        .map(([session, items]) => [session, items[0]]),
    ),
  );
  function enqueue(session: string, interaction: PendingInteraction) {
    if (!session || resolved.has(interaction.value.requestId)) return;
    const items = [...(queues.value[session] ?? [])];
    const index = items.findIndex((item) => item.value.requestId === interaction.value.requestId);
    if (index < 0) items.push(interaction);
    else items[index] = interaction;
    queues.value = { ...queues.value, [session]: items };
  }
  function remove(session: string, id?: string) {
    const items = queues.value[session] ?? [];
    if (!id) {
      const next = { ...queues.value };
      delete next[session];
      queues.value = next;
      return items.length > 0;
    }
    const target = id ?? items[0]?.value.requestId;
    if (!target) return false;
    const rest = items.filter((item) => item.value.requestId !== target);
    queues.value = { ...queues.value, [session]: rest };
    return rest.length !== items.length;
  }
  function resolve(id: string, session?: string) {
    resolved.add(id);
    for (const key of session ? [session] : Object.keys(queues.value)) remove(key, id);
  }
  async function sync(session: string) {
    const revision = (revisions.get(session) ?? 0) + 1;
    const baseline = new Map(revisions);
    revisions.set(session, revision);
    const before = new Map(
      Object.entries(queues.value).map(([key, items]) => [
        key,
        new Set(items.map((item) => item.value.requestId)),
      ]),
    );
    const snapshot = await getPendingInteractions(session);
    if (revisions.get(session) !== revision) return;
    const events = [...snapshot.askUser, ...snapshot.pathPermission, ...snapshot.toolApproval];
    const targets = session
      ? [session]
      : [
          ...new Set([
            ...Object.keys(queues.value),
            ...events.map((item) => item.sessionId).filter(Boolean),
          ]),
        ];
    const order = new Map(
      events.map((item) => [item.requestId, item.sequence ?? Number.MAX_SAFE_INTEGER]),
    );
    for (const target of targets) {
      if (!session && (revisions.get(target) ?? 0) !== (baseline.get(target) ?? 0)) continue;
      const incoming: PendingInteraction[] = [
        ...snapshot.askUser
          .filter((item) => item.sessionId === target)
          .map((value): PendingInteraction => ({ kind: "ask_user", value })),
        ...snapshot.pathPermission
          .filter((item) => item.sessionId === target)
          .map((value): PendingInteraction => ({ kind: "path_permission", value })),
        ...snapshot.toolApproval
          .filter((item) => item.sessionId === target)
          .map((value): PendingInteraction => ({ kind: "tool_approval", value })),
      ].filter((item) => !resolved.has(item.value.requestId));
      const ids = new Set(incoming.map((item) => item.value.requestId));
      for (const id of before.get(target) ?? []) {
        if (!ids.has(id)) resolved.add(id);
      }
      queues.value = {
        ...queues.value,
        [target]: (queues.value[target] ?? []).filter(
          (item) =>
            !resolved.has(item.value.requestId) &&
            (!before.get(target)?.has(item.value.requestId) || ids.has(item.value.requestId)),
        ),
      };
      for (const item of incoming) enqueue(target, item);
      if (!session) revisions.set(target, (revisions.get(target) ?? 0) + 1);
      queues.value = {
        ...queues.value,
        [target]: [...(queues.value[target] ?? [])].sort(
          (left, right) =>
            (order.get(left.value.requestId) ?? Number.MAX_SAFE_INTEGER) -
            (order.get(right.value.requestId) ?? Number.MAX_SAFE_INTEGER),
        ),
      };
    }
  }
  return { queues, pendingInteractions, enqueue, remove, resolve, sync };
}
