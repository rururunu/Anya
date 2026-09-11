/** Per-plugin subscription cap; a runaway plugin can't starve the shared bus. */
const MAX_SUBSCRIPTIONS_PER_PLUGIN = 200;

type Subscriber = { pluginId: string; fn: (payload: unknown) => void };

const topics = new Map<string, Set<Subscriber>>();
const subscriptionCounts = new Map<string, number>();

/**
 * Namespaced publish/subscribe usable by plugin-to-plugin or agent-to-agent
 * messaging. Anya only routes and rate-limits; topic protocols (e.g. an
 * "agent room" chat format) are defined by whoever uses the topic, not by
 * Anya adding a dedicated feature.
 */
export function publish(topic: string, payload: unknown): void {
  const subs = topics.get(topic);
  if (!subs) return;
  for (const sub of subs) {
    try {
      sub.fn(payload);
    } catch (error) {
      console.warn(`plugin bus subscriber failed on topic \`${topic}\``, sub.pluginId, error);
    }
  }
}

export function subscribe(
  topic: string,
  pluginId: string,
  fn: (payload: unknown) => void,
): () => void {
  const used = subscriptionCounts.get(pluginId) ?? 0;
  if (used >= MAX_SUBSCRIPTIONS_PER_PLUGIN) {
    throw new Error(
      `plugin \`${pluginId}\` exceeded ${MAX_SUBSCRIPTIONS_PER_PLUGIN} bus subscriptions`,
    );
  }
  const sub: Subscriber = { pluginId, fn };
  const set = topics.get(topic) ?? new Set<Subscriber>();
  set.add(sub);
  topics.set(topic, set);
  subscriptionCounts.set(pluginId, used + 1);
  return () => {
    set.delete(sub);
    subscriptionCounts.set(pluginId, Math.max(0, (subscriptionCounts.get(pluginId) ?? 1) - 1));
  };
}

export function unsubscribeAll(pluginId: string): void {
  for (const set of topics.values()) {
    for (const sub of Array.from(set)) {
      if (sub.pluginId === pluginId) set.delete(sub);
    }
  }
  subscriptionCounts.delete(pluginId);
}
