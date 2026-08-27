import { inject, provide, watch, type InjectionKey, type Ref } from "vue";

export type ConversationFindContext = {
  active: Ref<boolean>;
  query: Ref<string>;
};

export const conversationFindKey: InjectionKey<ConversationFindContext> =
  Symbol("conversationFind");

export function provideConversationFind(context: ConversationFindContext) {
  provide(conversationFindKey, context);
}

/** Expand a collapsed block when the current find query matches its hidden text. */
export function useExpandForFind(
  matches: (query: string) => boolean,
  expand: () => void,
  extra?: () => unknown,
) {
  const find = inject(conversationFindKey, null);
  if (!find) return;
  watch(
    () => [find.active.value, find.query.value, extra?.()] as const,
    ([active, query]) => {
      if (!active || !query.trim()) return;
      if (matches(query)) expand();
    },
    { immediate: true },
  );
}
