/**
 * Ask-user multi-question flow for ChatInputBar.
 * Keyboard navigation (`selectedIndex`) stays in the parent.
 */

import { computed, ref, watch, type Ref } from "vue";
import { tr } from "@/services/i18n";
import type { AskDisplayOption, AskUserQuestion } from "@/types/chat";
import type { AppLanguage } from "@/types/setting";
import { isPlanSwitchQuestion } from "@/services/chat/askUserAnswer";

export const ASK_SKIP_MARKER = "__user_supplement__";

export type AskUserSession = {
  requestId: string;
  questions: AskUserQuestion[];
};

function toAskSlug(label: string) {
  return label
    .trim()
    .toLowerCase()
    .replace(/[^\w\u4e00-\u9fff]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 32);
}

export function useAskUserFlow(options: {
  language: Ref<AppLanguage>;
  askUser: () => AskUserSession | null | undefined;
  selectedIndex: Ref<number>;
  emitAskUserComplete: (answer: string) => void;
  emitLayoutChange: () => void;
  syncPopupState: (open: boolean) => void | Promise<void>;
}) {
  const askQuestionIndex = ref(0);
  const askAnswers = ref<Record<number, string[]>>({});
  const askUserFinishing = ref(false);
  const askCustomInputOpen = ref(false);
  const askCustomText = ref("");

  const showAskUserPicker = computed(() => {
    const session = options.askUser();
    return Boolean(session && session.questions.length > 0 && !askUserFinishing.value);
  });

  const askQuestionCount = computed(() => options.askUser()?.questions.length ?? 0);

  const activeAskQuestion = computed(() => options.askUser()?.questions[askQuestionIndex.value]);

  const skipAskOption = computed<AskDisplayOption>(() => ({
    label: tr(options.language.value, "customAnswer"),
    slug: "custom",
    description: tr(options.language.value, "customAnswerDesc"),
    isSkip: true,
  }));

  const activeAskOptions = computed<AskDisplayOption[]>(() => {
    const questionOptions = (activeAskQuestion.value?.options ?? []).map((option) => ({
      label: option.label,
      slug: toAskSlug(option.label) || "option",
      description: option.description,
    }));
    if (isPlanSwitchQuestion(activeAskQuestion.value)) {
      return questionOptions;
    }
    return [...questionOptions, skipAskOption.value];
  });

  const askIsPlanSwitch = computed(() => isPlanSwitchQuestion(activeAskQuestion.value));

  const askConfirmRowIndex = computed(() =>
    activeAskQuestion.value?.multiSelect ? activeAskOptions.value.length : -1,
  );

  const askSelectedCount = computed(() => {
    if (!activeAskQuestion.value?.multiSelect) return 0;
    return (askAnswers.value[askQuestionIndex.value] ?? []).length;
  });

  const askPickerRowCount = computed(() => {
    if (!showAskUserPicker.value) {
      return 0;
    }
    const optionRows = activeAskOptions.value.length;
    const confirmRow = activeAskQuestion.value?.multiSelect ? 1 : 0;
    return 2 + optionRows + confirmRow;
  });

  function isAskOptionSelected(label: string) {
    if (label === tr(options.language.value, "customAnswer")) {
      return false;
    }
    const current = askAnswers.value[askQuestionIndex.value] ?? [];
    return current.includes(label);
  }

  function finishAskUser(answers: Record<number, string[]>, skipped = false) {
    const session = options.askUser();
    if (!session || askUserFinishing.value) {
      return;
    }

    askUserFinishing.value = true;

    const payload = {
      skipped,
      answers: session.questions.map((question, index) => {
        const selected = answers[index] ?? [];
        const userSupplement = selected.includes(ASK_SKIP_MARKER);
        return {
          header: question.header,
          question: question.question,
          kind: question.kind,
          selected: userSupplement ? [] : selected.filter((item) => item !== ASK_SKIP_MARKER),
          userSupplement,
        };
      }),
    };

    options.emitAskUserComplete(JSON.stringify(payload));
    options.emitLayoutChange();
  }

  function syncCustomInputDefaultState() {
    const question = activeAskQuestion.value;
    const hasPresetOptions = (question?.options?.length ?? 0) > 0;
    askCustomText.value = "";
    askCustomInputOpen.value = Boolean(
      question && !isPlanSwitchQuestion(question) && !hasPresetOptions,
    );
  }

  function advanceAskQuestion() {
    const session = options.askUser();
    if (!session) {
      return;
    }

    if (askQuestionIndex.value < session.questions.length - 1) {
      askQuestionIndex.value += 1;
      options.selectedIndex.value = 0;
      syncCustomInputDefaultState();
      options.emitLayoutChange();
      return;
    }

    finishAskUser(askAnswers.value);
  }

  function completeAskUserWithSkip() {
    const session = options.askUser();
    if (!session) {
      return;
    }

    const answers = { ...askAnswers.value };
    for (let index = askQuestionIndex.value; index < session.questions.length; index += 1) {
      answers[index] = [ASK_SKIP_MARKER];
    }
    finishAskUser(answers, true);
  }

  function openAskCustomInput() {
    const skipIdx = activeAskOptions.value.findIndex((item) => item.isSkip);
    if (skipIdx >= 0) {
      options.selectedIndex.value = skipIdx;
    }
    askCustomInputOpen.value = true;
    options.emitLayoutChange();
  }

  function closeAskCustomInput() {
    const hasPresetOptions = (activeAskQuestion.value?.options?.length ?? 0) > 0;
    if (!hasPresetOptions) {
      return;
    }
    if (!askCustomInputOpen.value) {
      return;
    }
    askCustomInputOpen.value = false;
    options.emitLayoutChange();
  }

  function submitCustomAskAnswer(rawText?: string): boolean {
    const text = (rawText ?? askCustomText.value).trim();
    if (!text) {
      return false;
    }
    if (!activeAskQuestion.value) {
      return false;
    }
    askAnswers.value[askQuestionIndex.value] = [text];
    askCustomInputOpen.value = false;
    askCustomText.value = "";
    advanceAskQuestion();
    return true;
  }

  function selectAskOption(option: AskDisplayOption) {
    if (option.isSkip) {
      openAskCustomInput();
      return;
    }

    const question = activeAskQuestion.value;
    if (!question) {
      return;
    }

    askCustomInputOpen.value = false;

    if (question.multiSelect) {
      const current = askAnswers.value[askQuestionIndex.value] ?? [];
      askAnswers.value[askQuestionIndex.value] = current.includes(option.label)
        ? current.filter((item) => item !== option.label)
        : [...current, option.label];
      return;
    }

    askAnswers.value[askQuestionIndex.value] = [option.label];
    advanceAskQuestion();
  }

  function confirmAskSelection() {
    const current = askAnswers.value[askQuestionIndex.value] ?? [];
    if (current.length === 0) {
      return;
    }
    askCustomInputOpen.value = false;
    advanceAskQuestion();
  }

  watch(
    () => options.askUser()?.requestId,
    (requestId) => {
      askUserFinishing.value = false;
      askQuestionIndex.value = 0;
      askAnswers.value = {};
      options.selectedIndex.value = 0;
      syncCustomInputDefaultState();
      // Callers may pass a binding that is initialized later in the same setup.
      queueMicrotask(() => {
        void options.syncPopupState(Boolean(requestId));
      });
      options.emitLayoutChange();
    },
    { immediate: true },
  );

  return {
    askQuestionIndex,
    askAnswers,
    askUserFinishing,
    askCustomInputOpen,
    askCustomText,
    showAskUserPicker,
    askQuestionCount,
    activeAskQuestion,
    skipAskOption,
    askIsPlanSwitch,
    activeAskOptions,
    askConfirmRowIndex,
    askSelectedCount,
    askPickerRowCount,
    isAskOptionSelected,
    selectAskOption,
    openAskCustomInput,
    closeAskCustomInput,
    submitCustomAskAnswer,
    confirmAskSelection,
    completeAskUserWithSkip,
    finishAskUser,
    advanceAskQuestion,
  };
}
