<template>
  <section
    v-if="activeAskQuestion"
    class="ask-card"
    :aria-labelledby="questionId"
    @keydown="navigateOptions"
  >
    <header class="ask-heading">
      <span class="ask-icon">
        <ListChecks v-if="askIsPlanSwitch" :size="16" aria-hidden="true" />
        <CircleHelp v-else :size="16" aria-hidden="true" />
      </span>
      <span class="ask-heading-text">{{ activeAskQuestion.header }}</span>
      <span v-if="askQuestionCount > 1" class="ask-progress">
        {{ askQuestionIndex + 1 }} / {{ askQuestionCount }}
      </span>
      <span class="ask-status" role="status">{{ tr(language, "waitingAnswer") }}</span>
    </header>
    <p :id="questionId" ref="prompt" class="ask-question" tabindex="-1">
      {{ activeAskQuestion.question }}
    </p>
    <fieldset class="ask-controls" :disabled="askUserFinishing">
      <button
        v-for="(option, index) in presetOptions"
        :key="index"
        type="button"
        class="ask-choice"
        :aria-pressed="
          activeAskQuestion.multiSelect ? isAskOptionSelected(option.label) : undefined
        "
        @click="choose(option)"
      >
        <span class="ask-mark" :class="{ checkbox: activeAskQuestion.multiSelect }">
          <Check
            v-if="activeAskQuestion.multiSelect && isAskOptionSelected(option.label)"
            :size="12"
          />
        </span>
        <span class="ask-choice-copy">
          <strong>{{ option.label }}</strong>
          <span v-if="option.description && option.description !== option.label">
            {{ option.description }}
          </span>
        </span>
      </button>
      <form v-if="askCustomInputOpen" class="ask-custom" @submit.prevent="submitCustom">
        <textarea
          ref="customInput"
          v-model="askCustomText"
          rows="2"
          :aria-label="tr(language, 'typeYourAnswer')"
          :placeholder="tr(language, 'typeYourAnswer')"
          @keydown.enter.exact="submitCustomOnEnter"
        />
        <button type="submit" class="ask-submit" :disabled="!askCustomText.trim()">
          {{ tr(language, "send") }}
        </button>
      </form>
      <footer v-if="showCustomButton || activeAskQuestion.multiSelect" class="ask-footer">
        <button v-if="showCustomButton" type="button" class="ask-own-answer" @click="toggleCustom">
          <PenLine :size="14" aria-hidden="true" />
          {{ tr(language, "customAnswer") }}
        </button>
        <button
          v-if="activeAskQuestion.multiSelect"
          type="button"
          class="ask-submit"
          :disabled="!askSelectedCount"
          @click="confirmSelection"
        >
          {{ tr(language, "confirmSelection") }}
        </button>
      </footer>
    </fieldset>
    <p v-if="submitError" class="ask-error" role="alert">{{ submitError }}</p>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, toRef, useId } from "vue";
import { Check, CircleHelp, ListChecks, PenLine } from "@lucide/vue";
import { useAskUserFlow, type AskUserSession } from "@/composables/chat/useAskUserFlow";
import { tr } from "@/services/i18n";
import type { AskDisplayOption } from "@/types/chat";
import type { AppLanguage } from "@/types/setting";

const props = defineProps<{
  session: AskUserSession;
  language: AppLanguage;
  submitAnswer: (answer: string) => Promise<void>;
}>();

const questionId = useId();
const prompt = ref<HTMLElement | null>(null);
const customInput = ref<HTMLTextAreaElement | null>(null);
const submitError = ref("");
const {
  activeAskQuestion,
  activeAskOptions,
  askCustomInputOpen,
  askCustomText,
  askIsPlanSwitch,
  askQuestionCount,
  askQuestionIndex,
  askSelectedCount,
  askUserFinishing,
  closeAskCustomInput,
  confirmAskSelection,
  isAskOptionSelected,
  openAskCustomInput,
  selectAskOption,
  submitCustomAskAnswer,
} = useAskUserFlow({
  language: toRef(props, "language"),
  askUser: () => props.session,
  selectedIndex: ref(0),
  emitAskUserComplete: (answer) => void submit(answer),
  emitLayoutChange: () => {},
  syncPopupState: () => {},
});

const presetOptions = computed(() => activeAskOptions.value.filter((option) => !option.isSkip));
const showCustomButton = computed(() => !askIsPlanSwitch.value && presetOptions.value.length > 0);

async function submit(answer: string) {
  submitError.value = "";
  try {
    await props.submitAnswer(answer);
  } catch (error) {
    askUserFinishing.value = false;
    submitError.value = tr(props.language, "askSubmitFailed", { error: String(error) });
    await nextTick();
    prompt.value?.focus();
  }
}

async function advance(action: () => void) {
  const index = askQuestionIndex.value;
  action();
  if (index === askQuestionIndex.value) return;
  await nextTick();
  (customInput.value ?? prompt.value)?.focus();
}

function choose(option: AskDisplayOption) {
  void advance(() => selectAskOption(option));
}

function confirmSelection() {
  void advance(confirmAskSelection);
}

async function submitCustom() {
  const draft = askCustomText.value;
  await advance(() => {
    if (submitCustomAskAnswer() && askUserFinishing.value) {
      askCustomText.value = draft;
      askCustomInputOpen.value = true;
    }
  });
}

function submitCustomOnEnter(event: KeyboardEvent) {
  if (event.isComposing || event.keyCode === 229) return;
  event.preventDefault();
  submitCustom();
}

async function toggleCustom() {
  if (askCustomInputOpen.value) {
    closeAskCustomInput();
    return;
  }
  openAskCustomInput();
  await nextTick();
  customInput.value?.focus();
}

function navigateOptions(event: KeyboardEvent) {
  if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
  const section = event.currentTarget as HTMLElement;
  const choices = Array.from(section.querySelectorAll<HTMLButtonElement>(".ask-choice"));
  const index = choices.findIndex((choice) => choice === event.target);
  if (!choices.length || (index < 0 && event.target !== prompt.value)) return;
  const delta = event.key === "ArrowDown" ? 1 : -1;
  const next = index < 0 ? (delta > 0 ? 0 : choices.length - 1) : index + delta;
  event.preventDefault();
  event.stopPropagation();
  choices[(next + choices.length) % choices.length]?.focus();
}
</script>

<style scoped>
.ask-card {
  box-sizing: border-box;
  width: 100%;
  min-width: 0;
  margin: 8px 0 12px;
  padding: 14px;
  border: 1px solid var(--peek-border);
  border-radius: 16px;
  background: color-mix(in srgb, var(--peek-text) 3%, var(--peek-bg));
  color: var(--peek-text);
  scroll-margin: 64px 0 24px;
}
.ask-heading,
.ask-footer,
.ask-custom {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.ask-icon {
  display: grid;
  place-items: center;
  flex: none;
  width: 26px;
  height: 26px;
  border-radius: 8px;
  background: color-mix(in srgb, var(--peek-text) 8%, transparent);
}
.ask-heading-text {
  flex: 1;
  min-width: 0;
  font-size: 12px;
  font-weight: 650;
  overflow-wrap: anywhere;
}
.ask-progress,
.ask-status {
  font-size: 11px;
  color: var(--peek-muted);
}
.ask-question {
  margin: 9px 0 12px;
  font-size: 14px;
  font-weight: 600;
  line-height: 1.55;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
.ask-controls {
  display: grid;
  gap: 6px;
  min-width: 0;
  margin: 0;
  padding: 0;
  border: 0;
}
.ask-choice {
  display: flex;
  align-items: flex-start;
  gap: 9px;
  width: 100%;
  min-width: 0;
  padding: 8px 10px;
  border: 1px solid var(--peek-border);
  border-radius: 10px;
  background: var(--peek-bg);
  color: inherit;
  text-align: start;
  cursor: pointer;
}
.ask-choice:hover,
.ask-choice[aria-pressed="true"] {
  border-color: color-mix(in srgb, var(--peek-text) 22%, var(--peek-border));
  background: color-mix(in srgb, var(--peek-text) 7%, var(--peek-bg));
}
.ask-mark {
  display: grid;
  place-items: center;
  flex: none;
  box-sizing: border-box;
  width: 15px;
  height: 15px;
  margin-top: 2px;
  border: 1px solid var(--peek-muted);
  border-radius: 50%;
}
.ask-mark.checkbox {
  border-radius: 4px;
}
.ask-choice[aria-pressed="true"] .ask-mark {
  border-color: var(--peek-text);
  background: var(--peek-text);
  color: var(--peek-bg);
}
.ask-choice-copy {
  display: grid;
  gap: 2px;
  min-width: 0;
  font-size: 13px;
  line-height: 1.45;
  overflow-wrap: anywhere;
}
.ask-choice-copy strong {
  font-weight: 600;
}
.ask-choice-copy span {
  color: var(--peek-muted);
  font-size: 12px;
}
.ask-footer {
  margin-top: 3px;
  justify-content: space-between;
}
.ask-own-answer,
.ask-submit {
  display: inline-flex;
  gap: 6px;
  align-items: center;
  min-height: 30px;
  padding: 4px 9px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: inherit;
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}
.ask-own-answer:hover {
  background: var(--peek-hover-bg);
}
.ask-submit {
  margin-inline-start: auto;
  background: var(--peek-text);
  color: var(--peek-bg);
}
.ask-controls :disabled {
  opacity: 0.55;
  cursor: default;
}
.ask-custom textarea {
  box-sizing: border-box;
  flex: 1 1 180px;
  min-width: 0;
  padding: 7px 9px;
  border: 1px solid var(--peek-border);
  border-radius: 9px;
  resize: vertical;
  background: var(--peek-bg);
  color: inherit;
  font: inherit;
}
.ask-card :focus-visible {
  outline: 2px solid var(--peek-accent);
  outline-offset: 2px;
}
.ask-error {
  margin: 8px 0 0;
  color: var(--peek-danger, #bf3535);
  font-size: 12px;
  overflow-wrap: anywhere;
}
</style>
