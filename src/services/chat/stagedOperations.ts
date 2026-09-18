// One IPC mutation at a time per session: a pop must never overtake a push.
const pending = new Map<string, Promise<unknown>>();

export function serializeStaged<T>(sessionId: string, operation: () => Promise<T>): Promise<T> {
  const result = (pending.get(sessionId) ?? Promise.resolve()).then(operation);
  const settled = result.catch(() => undefined);
  pending.set(sessionId, settled);
  void settled.then(() => {
    if (pending.get(sessionId) === settled) pending.delete(sessionId);
  });
  return result;
}
