/** Own asynchronous subscriptions even when registration finishes after disposal. */
export function createAsyncCleanupScope() {
  let disposed = false;
  const cleanups = new Set<() => void>();
  return {
    get disposed() {
      return disposed;
    },
    async add(registration: Promise<() => void>) {
      const cleanup = await registration;
      if (disposed) cleanup();
      else cleanups.add(cleanup);
    },
    dispose() {
      disposed = true;
      for (const cleanup of cleanups) {
        cleanups.delete(cleanup);
        try {
          cleanup();
        } catch (error) {
          console.warn("Subscription cleanup failed:", error);
        }
      }
    },
  };
}
