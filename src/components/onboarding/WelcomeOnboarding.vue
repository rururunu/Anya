<template>
  <div
    ref="overlayRef"
    class="onboarding"
    :class="{
      'is-centered': step === 1 || step === 3,
      'is-revealing': revealing,
      'can-scroll peek-scrollbar': step === 2 && providerTab === 'custom',
    }"
    role="dialog"
    aria-modal="true"
    :aria-label="t('onboarding.welcomeTitle')"
  >
    <div class="onboarding-atmosphere" aria-hidden="true" />

    <div
      ref="logoWrapRef"
      class="onboarding-logo"
      :class="{ 'is-revealing': revealing, 'is-large': step === 1 && !revealing }"
    >
      <img :src="appIconAsset" alt="Anya" draggable="false" />
    </div>

    <div v-show="!revealing" class="onboarding-stage">
      <p class="onboarding-step">
        {{ t("onboarding.stepOf").replace("{current}", String(step)).replace("{total}", "3") }}
      </p>

      <Transition name="onboarding-page" mode="out-in">
        <section v-if="step === 1" key="welcome" class="onboarding-page welcome-page">
          <h1>{{ t("onboarding.welcomeTitle") }}</h1>
          <p>{{ t("onboarding.welcomeSubtitle") }}</p>
          <button type="button" class="primary-btn" @click="goToStep(2)">
            {{ t("onboarding.continue") }}
          </button>
        </section>

        <section v-else-if="step === 2" key="provider" class="onboarding-page provider-page">
          <header>
            <h1>{{ t("onboarding.providerTitle") }}</h1>
            <p>{{ t("onboarding.providerSubtitle") }}</p>
          </header>

          <div class="provider-panels">
            <article class="provider-panel" :class="{ open: providerTab === 'deepseek' }">
              <button type="button" class="provider-panel-head" @click="providerTab = 'deepseek'">
                <span class="provider-icon"><DeepSeekIcon :size="18" /></span>
                <span class="provider-copy">
                  <strong>DeepSeek</strong>
                  <small>{{ t("onboarding.providerDeepSeekHint") }}</small>
                </span>
                <span v-if="isDeepSeekConfigured" class="ready-pill">
                  {{ t("onboarding.providerConfigured") }}
                </span>
              </button>
              <div v-if="providerTab === 'deepseek'" class="provider-panel-body">
                <div class="field-row">
                  <label for="onboarding-deepseek-key">{{ t("settings.provider.apiKey") }}</label>
                  <div class="onboarding-secret">
                    <input
                      id="onboarding-deepseek-key"
                      v-model="deepseekKey"
                      class="onboarding-input is-mono"
                      :type="deepseekKeyVisible ? 'text' : 'password'"
                      placeholder="sk-..."
                      spellcheck="false"
                      autocomplete="off"
                      @input="handleDeepSeekInput"
                      @blur="saveDeepSeek"
                    />
                    <button
                      type="button"
                      class="onboarding-secret-toggle"
                      tabindex="-1"
                      :aria-label="deepseekKeyVisible ? t('hideSecret') : t('showSecret')"
                      @mousedown.prevent
                      @click="deepseekKeyVisible = !deepseekKeyVisible"
                    >
                      <EyeOff v-if="deepseekKeyVisible" :size="14" />
                      <Eye v-else :size="14" />
                    </button>
                  </div>
                </div>
              </div>
            </article>

            <article class="provider-panel" :class="{ open: providerTab === 'gemini' }">
              <button type="button" class="provider-panel-head" @click="providerTab = 'gemini'">
                <span class="provider-icon"><GeminiIcon :size="18" /></span>
                <span class="provider-copy">
                  <strong>Gemini</strong>
                  <small>{{ t("onboarding.providerGeminiHint") }}</small>
                </span>
                <span v-if="isGeminiConfigured" class="ready-pill">
                  {{ t("onboarding.providerConfigured") }}
                </span>
              </button>
              <div v-if="providerTab === 'gemini'" class="provider-panel-body gemini-body">
                <div class="oauth-status">
                  <p class="oauth-status-label">{{ t("settings.provider.geminiAccount") }}</p>
                  <p class="oauth-status-value">
                    {{
                      isGeminiConfigured
                        ? settingStore.geminiOauth.email || t("settings.provider.geminiSignedIn")
                        : t("settings.provider.geminiSignedOut")
                    }}
                  </p>
                  <p v-if="geminiError" class="gemini-error">{{ geminiError }}</p>
                </div>
                <div class="gemini-actions">
                  <button
                    v-if="!isGeminiConfigured && !geminiBusy"
                    type="button"
                    class="primary-btn compact"
                    @click="loginGemini"
                  >
                    {{ t("settings.provider.geminiLogin") }}
                  </button>
                  <template v-else-if="!isGeminiConfigured && geminiBusy">
                    <button type="button" class="primary-btn compact" disabled>
                      {{ t("settings.provider.geminiLoggingIn") }}
                    </button>
                    <button type="button" class="ghost-btn compact" @click="cancelGeminiLogin">
                      {{ t("settings.provider.geminiCancelLogin") }}
                    </button>
                  </template>
                  <button
                    v-else
                    type="button"
                    class="ghost-btn compact"
                    :disabled="geminiBusy"
                    @click="logoutGemini"
                  >
                    {{ t("settings.provider.geminiLogout") }}
                  </button>
                </div>
              </div>
            </article>

            <article class="provider-panel" :class="{ open: providerTab === 'custom' }">
              <button type="button" class="provider-panel-head" @click="providerTab = 'custom'">
                <span class="provider-icon">
                  <component :is="selectedPresetIcon" v-if="selectedPresetIcon" :size="18" />
                  <Globe2 v-else :size="18" />
                </span>
                <span class="provider-copy">
                  <strong>{{ customPanelTitle }}</strong>
                  <small>{{ t("onboarding.providerCustomHint") }}</small>
                </span>
                <span v-if="hasCustomProvider" class="ready-pill">
                  {{ t("onboarding.providerConfigured") }}
                </span>
              </button>
              <div v-if="providerTab === 'custom'" class="provider-panel-body custom-body">
                <div class="preset-grid" role="list">
                  <button
                    v-for="preset in providerPresets"
                    :key="preset.id"
                    type="button"
                    class="preset-option"
                    :class="{ active: customPresetId === preset.id }"
                    role="listitem"
                    @click="selectPreset(preset)"
                  >
                    <component
                      :is="presetIcon(preset.id)"
                      v-if="presetIcon(preset.id)"
                      :size="14"
                      class="preset-option-icon"
                    />
                    <span>{{ preset.name }}</span>
                  </button>
                  <button
                    type="button"
                    class="preset-option"
                    :class="{ active: customPresetId === undefined && customFormOpen }"
                    @click="selectBlankCustom"
                  >
                    <Globe2 :size="14" class="preset-option-icon" />
                    <span>{{ t("settings.provider.addBlank") }}</span>
                  </button>
                </div>

                <div v-if="customFormOpen" class="custom-fields">
                  <div v-if="!customPresetId" class="field-row">
                    <label for="onboarding-custom-name">{{ t("onboarding.customName") }}</label>
                    <input
                      id="onboarding-custom-name"
                      v-model="customName"
                      class="onboarding-input"
                      type="text"
                      :placeholder="t('settings.provider.namePlaceholder')"
                      spellcheck="false"
                      autocomplete="off"
                    />
                  </div>
                  <div v-if="!customPresetId" class="field-row">
                    <label for="onboarding-custom-url">{{ t("onboarding.customBaseUrl") }}</label>
                    <input
                      id="onboarding-custom-url"
                      v-model="customUrl"
                      class="onboarding-input is-mono"
                      type="url"
                      :placeholder="t('settings.provider.urlPlaceholder')"
                      spellcheck="false"
                      autocomplete="off"
                    />
                  </div>
                  <div class="field-row">
                    <label for="onboarding-custom-key">{{ t("onboarding.customApiKey") }}</label>
                    <div class="onboarding-secret">
                      <input
                        id="onboarding-custom-key"
                        v-model="customKey"
                        class="onboarding-input is-mono"
                        :type="customKeyVisible ? 'text' : 'password'"
                        placeholder="sk-..."
                        spellcheck="false"
                        autocomplete="off"
                      />
                      <button
                        type="button"
                        class="onboarding-secret-toggle"
                        tabindex="-1"
                        :aria-label="customKeyVisible ? t('hideSecret') : t('showSecret')"
                        @mousedown.prevent
                        @click="customKeyVisible = !customKeyVisible"
                      >
                        <EyeOff v-if="customKeyVisible" :size="14" />
                        <Eye v-else :size="14" />
                      </button>
                    </div>
                  </div>
                  <div class="field-row">
                    <label for="onboarding-custom-models">{{ t("onboarding.customModels") }}</label>
                    <input
                      id="onboarding-custom-models"
                      v-model="customModels"
                      class="onboarding-input is-mono"
                      type="text"
                      :placeholder="t('settings.provider.modelsPlaceholder')"
                      spellcheck="false"
                      autocomplete="off"
                    />
                  </div>
                  <button type="button" class="primary-btn compact" @click="saveCustom">
                    {{ t("onboarding.saveCustom") }}
                  </button>
                </div>
              </div>
            </article>
          </div>

          <p class="provider-later">{{ t("onboarding.providerLater") }}</p>

          <div class="onboarding-actions">
            <button type="button" class="ghost-btn" @click="goToStep(1)">
              {{ t("onboarding.back") }}
            </button>
            <button type="button" class="primary-btn" @click="goToStep(3)">
              {{ hasAnyProvider ? t("onboarding.continue") : t("onboarding.skip") }}
            </button>
          </div>
        </section>

        <section v-else key="hotkey" class="onboarding-page hotkey-page">
          <header>
            <h1>{{ t("onboarding.hotkeyTitle") }}</h1>
            <p>{{ t("onboarding.hotkeySubtitle") }}</p>
          </header>

          <div class="hotkey-demo" aria-hidden="true">
            <span class="hotkey-key">{{ t("onboarding.hotkeyGesture") }}</span>
            <span class="hotkey-dot" />
            <span class="hotkey-key pulse">{{ t("onboarding.hotkeyGesture") }}</span>
          </div>
          <p class="hotkey-caption">{{ t("onboarding.hotkeyHint") }}</p>

          <div class="onboarding-actions">
            <button type="button" class="ghost-btn" :disabled="revealing" @click="goToStep(2)">
              {{ t("onboarding.back") }}
            </button>
            <button type="button" class="primary-btn" :disabled="revealing" @click="finish">
              {{ t("onboarding.finish") }}
            </button>
          </div>
        </section>
      </Transition>
    </div>

    <button v-show="!revealing" type="button" class="skip-tour-btn" @click="skipTour">
      {{ t("onboarding.skipTour") }}
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { Eye, EyeOff, Globe2 } from "@lucide/vue";
import { useDebounceFn } from "@vueuse/core";
import { gsap, safeGsap } from "@/services/motion/gsapSafe";

import type { Component } from "vue";
import DeepSeekIcon from "@/components/icons/DeepSeekIcon.vue";
import GeminiIcon from "@/components/icons/GeminiIcon.vue";
import { getProviderIcon } from "@/lib/providerIcons";
import {
  CUSTOM_PROVIDER_PRESETS,
  type ProviderPreset,
  type ProviderPresetId,
  serializeProviderModels,
} from "@/lib/providerPresets";
import { tr } from "@/services/i18n";
import { geminiOauthCancelLogin, geminiOauthLogin, geminiOauthLogout } from "@/services/ipc";
import { gsapOnboardingReveal } from "@/services/motion/gsapPresets";
import { useChatModelStore } from "@/stores/chatModel";
import { useSettingStore } from "@/stores/setting";
import appIconAsset from "../../../src-tauri/icons/Anya-transparent.svg";

const emit = defineEmits<{
  completed: [];
}>();

const settingStore = useSettingStore();
const chatModelStore = useChatModelStore();

const step = ref(1);
const providerTab = ref<"deepseek" | "gemini" | "custom">("deepseek");
const revealing = ref(false);
const overlayRef = ref<HTMLElement | null>(null);
const logoWrapRef = ref<HTMLElement | null>(null);

const deepseekKey = ref(settingStore.deepseekApiKey);
const deepseekKeyDirty = ref(false);
const deepseekKeyVisible = ref(false);
const customKeyVisible = ref(false);
const geminiBusy = ref(false);
const geminiError = ref("");
const customName = ref("");
const customUrl = ref("");
const customKey = ref("");
const customModels = ref("");
const customPresetId = ref<ProviderPresetId | undefined>(undefined);
const customFormOpen = ref(false);
const providerPresets = CUSTOM_PROVIDER_PRESETS;

const t = (key: Parameters<typeof tr>[1]) => tr(settingStore.language, key);

const isDeepSeekConfigured = computed(() => deepseekKey.value.trim().length > 0);
const isGeminiConfigured = computed(() => {
  const oauth = settingStore.geminiOauth;
  return Boolean(oauth.accessToken?.trim() || oauth.refreshToken?.trim());
});
const hasCustomProvider = computed(() =>
  settingStore.customProviders.some((p) => p.baseUrl.trim() || p.apiKey.trim()),
);
const hasAnyProvider = computed(
  () => isDeepSeekConfigured.value || isGeminiConfigured.value || hasCustomProvider.value,
);
const selectedPresetIcon = computed(() =>
  customPresetId.value ? getProviderIcon(null, customPresetId.value) : null,
);
const customPanelTitle = computed(() => {
  if (customPresetId.value) {
    const preset = providerPresets.find((item) => item.id === customPresetId.value);
    if (preset) return preset.name;
  }
  return t("settings.provider.custom");
});

function presetIcon(presetId: string): Component | null {
  return getProviderIcon(null, presetId);
}

function selectPreset(preset: ProviderPreset) {
  customPresetId.value = preset.id;
  customName.value = preset.name;
  customUrl.value = preset.baseUrl;
  customModels.value = serializeProviderModels(preset.models);
  customFormOpen.value = true;
  providerTab.value = "custom";
}

function selectBlankCustom() {
  customPresetId.value = undefined;
  customName.value = "";
  customUrl.value = "";
  customModels.value = "";
  customFormOpen.value = true;
  providerTab.value = "custom";
}

function resetOnboardingScroll() {
  const el = overlayRef.value;
  if (!el) return;
  el.scrollTop = 0;
  el.scrollLeft = 0;
}

async function goToStep(next: number) {
  if (step.value === 2 && next !== 2) {
    await saveDeepSeek();
  }
  // Leaving step 2 / custom form: drop scroll mode so leftover scrollTop
  // cannot shift step 1/3 layout.
  if (next !== 2) {
    providerTab.value = "deepseek";
  }
  step.value = next;
  resetOnboardingScroll();
  await nextTick();
  resetOnboardingScroll();
}

watch(providerTab, async (tab) => {
  if (tab !== "custom") {
    resetOnboardingScroll();
    await nextTick();
    resetOnboardingScroll();
  }
});

onMounted(() => {
  const logo = logoWrapRef.value;
  if (!logo) return;
  safeGsap(
    "onboardingLogoEnter",
    () => {
      gsap.fromTo(
        logo,
        { autoAlpha: 0, y: 18, scale: 0.92 },
        { autoAlpha: 1, y: 0, scale: 1, duration: 0.7, ease: "power3.out" },
      );
    },
    () => {
      logo.style.opacity = "1";
      logo.style.visibility = "visible";
      logo.style.transform = "";
    },
  );
});

async function saveDeepSeek() {
  const next = deepseekKey.value.trim();
  const current = settingStore.deepseekApiKey.trim();
  if (next === current) return;
  // Do not let a stale pre-load draft erase a persisted credential.
  if (!deepseekKeyDirty.value && !next && current) {
    deepseekKey.value = settingStore.deepseekApiKey;
    return;
  }
  await settingStore.update({ deepseekApiKey: next });
  await chatModelStore.refresh();
  deepseekKeyDirty.value = false;
}

function handleDeepSeekInput() {
  deepseekKeyDirty.value = true;
  persistDeepSeekDebounced();
}

const persistDeepSeekDebounced = useDebounceFn(() => {
  void saveDeepSeek();
}, 400);

async function loginGemini() {
  geminiError.value = "";
  geminiBusy.value = true;
  try {
    await geminiOauthLogin();
    await settingStore.load();
    await chatModelStore.refresh();
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (!/sign-in was cancelled|sign-in was canceled|access_denied/i.test(message)) {
      geminiError.value = message;
    }
  } finally {
    geminiBusy.value = false;
  }
}

async function cancelGeminiLogin() {
  try {
    await geminiOauthCancelLogin();
  } catch {
    // await path surfaces cancel
  }
}

async function logoutGemini() {
  geminiBusy.value = true;
  try {
    await geminiOauthLogout();
    await settingStore.load();
    await chatModelStore.refresh();
  } catch (error) {
    geminiError.value = error instanceof Error ? error.message : String(error);
  } finally {
    geminiBusy.value = false;
  }
}

async function saveCustom() {
  if (!customUrl.value.trim() || !customKey.value.trim()) return;
  const preset = customPresetId.value
    ? providerPresets.find((item) => item.id === customPresetId.value)
    : undefined;
  const existing = preset
    ? settingStore.customProviders.find((p) => p.presetId === preset.id)
    : undefined;
  const id = existing?.id ?? Math.random().toString(36).substring(2, 11);
  const next = {
    id,
    name: customName.value.trim() || preset?.name || `Custom - ${id}`,
    baseUrl: customUrl.value.trim(),
    apiKey: customKey.value.trim(),
    models: customModels.value.trim(),
    presetId: customPresetId.value,
  };
  const list = existing
    ? settingStore.customProviders.map((p) => (p.id === id ? next : p))
    : [...settingStore.customProviders, next];
  await settingStore.update({ customProviders: list });
  await chatModelStore.refresh();
  customKey.value = "";
}

async function skipTour() {
  if (revealing.value) return;
  // Same logo reveal as "进入工作区"; reset scroll first so the flight
  // path is measured from a centered / unscrolled overlay.
  resetOnboardingScroll();
  await nextTick();
  await finish();
}

async function finish() {
  if (revealing.value) return;

  const overlay = overlayRef.value;
  const logo = logoWrapRef.value;
  if (!overlay || !logo) {
    await completeOnboarding();
    return;
  }

  resetOnboardingScroll();
  await nextTick();

  // Measure both rects before toggling reveal layout, so the flight path
  // lands on the empty-conversation brand above the composer.
  const from = logo.getBoundingClientRect();
  const targetEl = document.querySelector<HTMLElement>("[data-onboarding-logo-target]");
  const target =
    targetEl?.getBoundingClientRect() ??
    new DOMRect(
      Math.round(window.innerWidth / 2 - 52),
      Math.round(window.innerHeight / 2 - 188),
      104,
      104,
    );

  if (targetEl) {
    targetEl.style.visibility = "hidden";
  }

  revealing.value = true;
  await nextTick();
  await saveDeepSeek();

  gsapOnboardingReveal({
    overlay,
    logo,
    from,
    target,
    onComplete: () => {
      if (targetEl) {
        targetEl.style.visibility = "";
      }
      void completeOnboarding();
    },
  });
}

async function completeOnboarding() {
  await chatModelStore.ensureDefault({ refresh: true });
  await settingStore.update({ onboardingCompleted: true });
  emit("completed");
}
</script>

<style scoped>
.onboarding {
  position: absolute;
  inset: 0;
  z-index: 80;
  display: flex;
  flex-direction: column;
  align-items: center;
  overflow: hidden;
  padding: 24px 24px 56px;
  background:
    radial-gradient(120% 80% at 50% -10%, #ffffff 0%, transparent 55%),
    linear-gradient(180deg, #f7f5f1 0%, #efeae2 48%, #e8e2d8 100%);
  color: #1c1915;
  font-family: var(--peek-font-sans);
}

.onboarding.can-scroll {
  overflow-x: hidden;
  overflow-y: auto;
}

.onboarding.is-centered {
  justify-content: center;
}

.onboarding-atmosphere {
  position: absolute;
  inset: -20%;
  background:
    radial-gradient(circle at 18% 78%, rgba(180, 150, 110, 0.18), transparent 42%),
    radial-gradient(circle at 82% 22%, rgba(120, 150, 170, 0.14), transparent 40%);
  pointer-events: none;
}

.onboarding-logo {
  position: relative;
  flex: none;
  width: 112px;
  height: 112px;
  margin: 8px auto 0;
  z-index: 2;
  transition:
    width 0.28s ease,
    height 0.28s ease,
    margin 0.28s ease;
  will-change: transform;
}

.onboarding-logo.is-large {
  width: 220px;
  height: 220px;
  margin: 0 auto 8px;
}

.onboarding.is-centered .onboarding-logo {
  margin-top: 0;
}

.onboarding-logo img {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.onboarding-logo.is-revealing {
  pointer-events: none;
  transition: none;
}

.onboarding-stage {
  position: relative;
  z-index: 1;
  width: min(560px, 100%);
  margin-top: 18px;
  display: grid;
  gap: 16px;
  padding-bottom: 8px;
}

.onboarding.is-centered .onboarding-stage {
  margin-top: 20px;
}

.onboarding-step {
  margin: 0;
  color: rgba(28, 25, 21, 0.48);
  font-size: 12px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  text-align: center;
}

.onboarding-page {
  display: grid;
  gap: 18px;
  text-align: center;
}

.onboarding-page h1 {
  margin: 0;
  font-size: clamp(26px, 3.8vw, 38px);
  font-weight: 650;
  letter-spacing: -0.03em;
  line-height: 1.15;
}

.onboarding-page header {
  display: grid;
  gap: 8px;
}

.onboarding-page p,
.provider-later,
.hotkey-caption {
  margin: 0;
  color: rgba(28, 25, 21, 0.62);
  font-size: 14px;
  line-height: 1.6;
}

.welcome-page {
  gap: 22px;
  padding-top: 4px;
}

.provider-page {
  gap: 14px;
}

.provider-panels {
  display: grid;
  gap: 10px;
  text-align: left;
}

.provider-panel {
  border: 1px solid rgba(28, 25, 21, 0.1);
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.72);
  box-shadow: 0 10px 30px rgba(40, 30, 15, 0.04);
  overflow: hidden;
}

.provider-panel.open {
  border-color: rgba(28, 25, 21, 0.18);
}

.provider-panel-head {
  width: 100%;
  display: grid;
  grid-template-columns: auto 1fr auto;
  gap: 12px;
  align-items: center;
  padding: 14px;
  border: 0;
  background: transparent;
  color: inherit;
  cursor: pointer;
  text-align: left;
}

.provider-icon {
  display: grid;
  place-items: center;
  width: 36px;
  height: 36px;
  border-radius: 10px;
  background: rgba(28, 25, 21, 0.05);
  color: #1c1915;
}

.provider-copy {
  display: grid;
  gap: 2px;
  min-width: 0;
}

.provider-copy strong {
  font-size: 14px;
  font-weight: 600;
}

.provider-copy small {
  color: rgba(28, 25, 21, 0.55);
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ready-pill {
  padding: 4px 8px;
  border-radius: 999px;
  background: rgba(24, 121, 78, 0.12);
  color: #18794e;
  font-size: 11px;
  font-weight: 600;
}

.provider-panel-body {
  display: grid;
  gap: 12px;
  padding: 0 14px 14px;
}

.field-row {
  display: grid;
  gap: 6px;
}

.field-row > label {
  margin: 0;
  color: rgba(28, 25, 21, 0.58);
  font-size: 11px;
  font-weight: 550;
  letter-spacing: 0.02em;
}

.onboarding-input {
  box-sizing: border-box;
  width: 100%;
  height: 36px;
  margin: 0;
  padding: 0 12px;
  border: 1px solid rgba(28, 25, 21, 0.14);
  border-radius: 8px;
  background: #fff;
  color: #1c1915;
  font: inherit;
  font-size: 13px;
  line-height: 36px;
  outline: none;
  appearance: none;
  box-shadow: none;
}

.onboarding-input.is-mono {
  font-family: var(--font-mono, ui-monospace, Consolas, monospace);
  font-size: 12px;
}

.onboarding-input::placeholder {
  color: rgba(28, 25, 21, 0.38);
}

.onboarding-input:hover {
  border-color: rgba(28, 25, 21, 0.22);
}

.onboarding-input:focus {
  border-color: rgba(28, 25, 21, 0.4);
  box-shadow: none;
  outline: none;
}

.onboarding-secret {
  position: relative;
  width: 100%;
}

.onboarding-secret .onboarding-input {
  padding-right: 36px;
}

.onboarding-secret-toggle {
  position: absolute;
  top: 50%;
  right: 6px;
  width: 28px;
  height: 28px;
  display: inline-grid;
  place-items: center;
  padding: 0;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: rgba(28, 25, 21, 0.45);
  transform: translateY(-50%);
  cursor: pointer;
}

.onboarding-secret-toggle:hover {
  background: rgba(28, 25, 21, 0.06);
  color: rgba(28, 25, 21, 0.75);
}

.custom-body {
  gap: 12px;
}

.preset-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.preset-option {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-height: 34px;
  padding: 0 10px;
  border: 1px solid rgba(28, 25, 21, 0.14);
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.72);
  color: #1c1915;
  font: inherit;
  font-size: 12px;
  font-weight: 550;
  text-align: left;
  cursor: pointer;
}

.preset-option:hover {
  border-color: rgba(28, 25, 21, 0.28);
  background: #fff;
}

.preset-option.active {
  border-color: rgba(28, 25, 21, 0.45);
  background: #fff;
  box-shadow: inset 0 0 0 1px rgba(28, 25, 21, 0.12);
}

.preset-option-icon {
  flex: 0 0 auto;
  color: rgba(28, 25, 21, 0.72);
}

.custom-fields {
  display: grid;
  gap: 12px;
}

.oauth-status {
  display: grid;
  gap: 4px;
  padding: 10px 12px;
  border: 1px solid rgba(28, 25, 21, 0.1);
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.55);
}

.oauth-status-label {
  margin: 0;
  color: rgba(28, 25, 21, 0.5);
  font-size: 10px;
  font-weight: 550;
}

.oauth-status-value {
  margin: 0;
  color: #1c1915;
  font-size: 12px;
  font-weight: 550;
}

.gemini-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.gemini-error {
  margin: 0;
  color: #c42b1c;
  font-size: 12px;
}

.onboarding-actions {
  display: flex;
  justify-content: center;
  gap: 10px;
  flex-wrap: wrap;
}

.primary-btn,
.ghost-btn {
  min-height: 40px;
  padding: 0 18px;
  border-radius: 999px;
  border: 0;
  font: inherit;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
}

.primary-btn {
  background: #171411;
  color: #f7f5f1;
}

.primary-btn:disabled {
  opacity: 0.55;
  cursor: default;
}

.primary-btn.compact,
.ghost-btn.compact {
  min-height: 34px;
  padding: 0 14px;
  font-size: 13px;
}

.ghost-btn {
  background: transparent;
  color: rgba(28, 25, 21, 0.72);
  border: 1px solid rgba(28, 25, 21, 0.14);
}

.skip-tour-btn {
  position: absolute;
  right: 20px;
  bottom: 18px;
  z-index: 3;
  padding: 0;
  border: 0;
  background: transparent;
  color: rgba(28, 25, 21, 0.48);
  cursor: pointer;
  font: inherit;
  font-size: 13px;
  font-weight: 500;
  text-decoration: underline;
  text-underline-offset: 3px;
}

.skip-tour-btn:hover {
  color: rgba(28, 25, 21, 0.78);
}

.hotkey-demo {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 14px;
  min-height: 88px;
}

.hotkey-key {
  display: inline-grid;
  place-items: center;
  min-width: 72px;
  height: 56px;
  padding: 0 16px;
  border-radius: 12px;
  border: 1px solid rgba(28, 25, 21, 0.14);
  background: rgba(255, 255, 255, 0.85);
  box-shadow:
    0 1px 0 rgba(255, 255, 255, 0.8) inset,
    0 10px 24px rgba(40, 30, 15, 0.08);
  font-size: 18px;
  font-weight: 650;
  letter-spacing: 0.04em;
}

.hotkey-key.pulse {
  animation: hotkey-pulse 1.6s ease-in-out infinite;
}

.hotkey-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: rgba(28, 25, 21, 0.28);
}

.hotkey-plus {
  font-size: 20px;
  font-weight: 600;
  color: rgba(28, 25, 21, 0.42);
  user-select: none;
}

@keyframes hotkey-pulse {
  0%,
  100% {
    transform: translateY(0);
    box-shadow:
      0 1px 0 rgba(255, 255, 255, 0.8) inset,
      0 10px 24px rgba(40, 30, 15, 0.08);
  }
  45% {
    transform: translateY(3px) scale(0.97);
    box-shadow:
      0 1px 0 rgba(255, 255, 255, 0.8) inset,
      0 4px 10px rgba(40, 30, 15, 0.1);
  }
}

.onboarding-page-enter-active,
.onboarding-page-leave-active {
  transition:
    opacity 0.22s ease,
    transform 0.22s ease;
}

.onboarding-page-enter-from,
.onboarding-page-leave-to {
  opacity: 0;
  transform: translateY(10px);
}

@media (prefers-reduced-motion: reduce) {
  .hotkey-key.pulse {
    animation: none;
  }

  .onboarding-logo {
    transition: none;
  }

  .onboarding-page-enter-active,
  .onboarding-page-leave-active {
    transition: none;
  }
}
</style>
