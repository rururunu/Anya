<template>
  <section class="settings-page about-page">
    <header class="about-hero">
      <div class="about-logo" aria-hidden="true">
        <MascotFace interactive />
      </div>
      <h1>{{ name }}</h1>
      <p class="about-tagline">{{ copy.description }}</p>
      <p class="about-version">{{ copy.versionLabel }}</p>
    </header>

    <section class="about-update" :class="updateTone" aria-labelledby="about-updates-title">
      <div class="about-update-row">
        <div class="about-update-main">
          <div class="about-update-icon" aria-hidden="true">
            <RefreshCw v-if="updaterStore.isBusy" :size="16" class="spin" />
            <ArrowUpCircle v-else-if="updaterStore.updateAvailable" :size="16" />
            <CheckCircle2 v-else :size="16" />
          </div>
          <div class="about-update-copy">
            <h2 id="about-updates-title">{{ copy.updates }}</h2>
            <p>{{ updateDetail }}</p>
          </div>
        </div>
        <div class="about-actions">
          <button
            type="button"
            class="about-btn"
            :disabled="updaterStore.isBusy"
            @click="handleCheckUpdate"
          >
            {{ copy.check }}
          </button>
          <button
            v-if="updaterStore.updateAvailable"
            type="button"
            class="about-btn about-btn-accent"
            :disabled="updaterStore.isBusy"
            @click="handleInstallUpdate"
          >
            {{ copy.updateNow }}
          </button>
        </div>
      </div>
      <div
        v-if="showProgress"
        class="about-progress"
        role="progressbar"
        :aria-valuemin="0"
        :aria-valuemax="100"
        :aria-valuenow="progressPercent"
      >
        <div class="about-progress-track">
          <div class="about-progress-fill" :style="{ width: `${progressPercent}%` }" />
        </div>
      </div>
    </section>

    <section class="about-block" aria-labelledby="about-specs-title">
      <h2 id="about-specs-title">{{ copy.application }}</h2>
      <dl class="about-meta">
        <div>
          <dt>{{ copy.appName }}</dt>
          <dd>{{ name }}</dd>
        </div>
        <div>
          <dt>{{ copy.version }}</dt>
          <dd class="mono">{{ version || "—" }}</dd>
        </div>
        <div>
          <dt>{{ copy.identifier }}</dt>
          <dd>
            <button
              type="button"
              class="about-copy mono"
              :title="copied ? copy.copied : copy.copyId"
              @click="copyIdentifier"
            >
              {{ identifier }}
            </button>
          </dd>
        </div>
        <div>
          <dt>{{ copy.runtime }}</dt>
          <dd>{{ copy.runtimeValue }}</dd>
        </div>
      </dl>
    </section>

    <section class="about-privacy" aria-labelledby="about-privacy-title">
      <ShieldCheck :size="15" class="about-privacy-icon" aria-hidden="true" />
      <div>
        <h2 id="about-privacy-title">{{ copy.privacy }}</h2>
        <p>{{ copy.privacyDescription }}</p>
      </div>
    </section>
  </section>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { ArrowUpCircle, CheckCircle2, RefreshCw, ShieldCheck } from "@lucide/vue";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";
import { useUpdaterStore } from "@/stores/updater";
import MascotFace from "@/components/icons/MascotFace.vue";

const settingStore = useSettingStore();
const updaterStore = useUpdaterStore();
const props = defineProps<{ name: string; version: string; identifier: string }>();
const versionForCopy = computed(() => props.version || "-");
const copied = ref(false);
let copiedTimer: number | undefined;

const copy = computed(() => {
  const language = settingStore.language;
  return {
    description: tr(language, "about.description"),
    versionLabel: tr(language, "about.versionLabel", { version: versionForCopy.value }),
    application: tr(language, "about.application"),
    appName: tr(language, "about.appName"),
    version: tr(language, "about.version"),
    identifier: tr(language, "about.identifier"),
    runtime: tr(language, "about.runtime"),
    runtimeValue: tr(language, "about.runtimeValue"),
    privacy: tr(language, "about.privacy"),
    privacyDescription: tr(language, "about.privacyDescription"),
    copyId: tr(language, "about.copyIdentifier"),
    copied: tr(language, "about.copied"),
    updates: tr(language, "updater.sectionTitle"),
    updateAvailableDetail: tr(language, "updater.updateAvailableDetail", {
      version: updaterStore.latestVersion || "?",
    }),
    upToDateDetail: tr(language, "updater.upToDateDetail"),
    check: tr(language, "updater.check"),
    updateNow: tr(language, "updater.updateNow"),
    error: tr(language, "updater.error"),
  };
});

const statusCopy = computed(() => {
  const language = settingStore.language;
  if (updaterStore.progress.phase === "installing") {
    return tr(language, "updater.installing");
  }
  if (updaterStore.status === "downloading" || updaterStore.progress.phase === "downloading") {
    return tr(language, "updater.downloading");
  }
  return tr(language, "updater.checking");
});

const updateDetail = computed(() => {
  if (updaterStore.errorMessage && !updaterStore.isBusy) {
    return `${copy.value.error}: ${updaterStore.errorMessage}`;
  }
  if (updaterStore.status === "checking" || updaterStore.isBusy) {
    return statusCopy.value;
  }
  if (updaterStore.updateAvailable) return copy.value.updateAvailableDetail;
  if (updaterStore.status === "up-to-date") return copy.value.upToDateDetail;
  return copy.value.upToDateDetail;
});

const updateTone = computed(() => {
  if (updaterStore.errorMessage && !updaterStore.isBusy) return "is-error";
  if (updaterStore.updateAvailable) return "is-available";
  if (updaterStore.isBusy) return "is-busy";
  return "is-ready";
});

const showProgress = computed(
  () =>
    updaterStore.status === "downloading" ||
    updaterStore.progress.phase === "downloading" ||
    updaterStore.progress.phase === "installing",
);

const progressPercent = computed(() => {
  const { downloadedBytes, totalBytes, phase } = updaterStore.progress;
  if (phase === "installing") return 100;
  if (!totalBytes || totalBytes <= 0) return updaterStore.isBusy ? 10 : 0;
  return Math.max(4, Math.min(100, Math.round((downloadedBytes / totalBytes) * 100)));
});

async function handleCheckUpdate() {
  updaterStore.resetTransientError();
  await updaterStore.check();
}

async function handleInstallUpdate() {
  await updaterStore.install();
}

async function copyIdentifier() {
  const value = props.identifier?.trim();
  if (!value) return;
  try {
    await navigator.clipboard.writeText(value);
    copied.value = true;
    if (copiedTimer) window.clearTimeout(copiedTimer);
    copiedTimer = window.setTimeout(() => {
      copied.value = false;
    }, 1400);
  } catch (error) {
    console.error("copy identifier failed:", error);
  }
}
</script>

<style scoped>
.about-page {
  box-sizing: border-box;
  width: min(100%, 520px);
  margin: 0 auto;
  padding-top: var(--peek-space-5, 20px);
  padding-bottom: var(--peek-space-6, 24px);
  color: var(--peek-text);
}

.about-hero {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 4px 0 32px;
}

.about-logo {
  width: 156px;
  height: 156px;
  margin-bottom: 20px;
}

.about-logo :deep(.mascot-face) {
  opacity: 0.96;
  filter: drop-shadow(0 12px 28px color-mix(in srgb, var(--peek-text) 16%, transparent));
}

.about-hero h1 {
  margin: 0;
  font-size: 30px;
  font-weight: 700;
  letter-spacing: -0.03em;
  line-height: 1.1;
}

.about-tagline {
  max-width: 22em;
  margin: 12px 0 0;
  color: var(--peek-muted);
  font-size: 14px;
  line-height: 1.55;
  text-wrap: balance;
}

.about-version {
  margin: 12px 0 0;
  color: var(--peek-faint);
  font-size: 11px;
  font-weight: 550;
  letter-spacing: 0.02em;
}

.about-update {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin: 0 0 4px;
  padding: 12px 14px;
  border: 1px solid color-mix(in srgb, var(--peek-border) 88%, transparent);
  border-radius: var(--peek-radius-lg, 12px);
  background: var(--peek-list-bg);
}

.about-update.is-available {
  border-color: color-mix(in srgb, var(--peek-accent) 32%, var(--peek-border));
  background: color-mix(in srgb, var(--peek-accent) 6%, transparent);
}

.about-update.is-error {
  border-color: color-mix(in srgb, var(--peek-danger, #ef4444) 30%, var(--peek-border));
}

.about-update-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.about-update-main {
  display: flex;
  gap: 10px;
  align-items: center;
  min-width: 0;
  flex: 1;
}

.about-update-icon {
  flex: none;
  width: 32px;
  height: 32px;
  display: grid;
  place-items: center;
  border-radius: 8px;
  background: color-mix(in srgb, var(--peek-text) 5%, transparent);
  color: var(--peek-muted);
}

.about-update.is-available .about-update-icon {
  background: color-mix(in srgb, var(--peek-accent) 14%, transparent);
  color: var(--peek-accent);
}

.about-update.is-ready .about-update-icon {
  color: color-mix(in srgb, var(--peek-accent) 65%, var(--peek-muted));
}

.about-update.is-error .about-update-icon {
  color: var(--peek-danger, #ef4444);
}

.about-update-copy {
  min-width: 0;
  flex: 1;
}

.about-update-copy h2 {
  margin: 0;
  color: var(--peek-text);
  font-size: 12.5px;
  font-weight: 600;
  letter-spacing: -0.01em;
}

.about-update-copy p {
  margin: 3px 0 0;
  color: var(--peek-muted);
  font-size: 12px;
  line-height: 1.45;
}

.about-update.is-available .about-update-copy p {
  color: var(--peek-accent);
}

.about-update.is-error .about-update-copy p {
  color: var(--peek-danger, #ef4444);
}

.about-progress {
  margin: 0;
}

.about-progress-track {
  height: 3px;
  overflow: hidden;
  border-radius: 999px;
  background: color-mix(in srgb, var(--peek-text) 8%, transparent);
}

.about-progress-fill {
  height: 100%;
  border-radius: inherit;
  background: var(--peek-accent);
  transition: width 200ms ease;
}

.about-actions {
  display: flex;
  flex: none;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 8px;
}

.about-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 28px;
  padding: 0 12px;
  border: 1px solid color-mix(in srgb, var(--peek-border) 90%, transparent);
  border-radius: 8px;
  background: transparent;
  color: var(--peek-text);
  font: inherit;
  font-size: 12px;
  font-weight: 550;
  cursor: pointer;
}

.about-btn:hover:not(:disabled) {
  background: color-mix(in srgb, var(--peek-text) 5%, transparent);
}

.about-btn-accent {
  border-color: transparent;
  background: var(--peek-accent);
  color: var(--peek-accent-fg, #fff);
}

.about-btn-accent:hover:not(:disabled) {
  filter: brightness(1.05);
  background: var(--peek-accent);
}

.about-btn:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.about-block {
  padding: 18px 0;
  border-top: 1px solid color-mix(in srgb, var(--peek-border) 85%, transparent);
}

.about-block h2,
.about-privacy h2 {
  margin: 0;
  font-size: 11px;
  font-weight: 650;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--peek-faint);
}

.about-meta {
  display: grid;
  gap: 0;
  margin: 10px 0 0;
}

.about-meta > div {
  display: grid;
  grid-template-columns: 112px minmax(0, 1fr);
  gap: 12px;
  align-items: baseline;
  padding: 10px 0;
  border-top: 1px solid color-mix(in srgb, var(--peek-border) 70%, transparent);
}

.about-meta > div:first-child {
  border-top: 0;
  padding-top: 2px;
}

dt {
  margin: 0;
  color: var(--peek-faint);
  font-size: 11px;
  font-weight: 550;
}

dd {
  min-width: 0;
  margin: 0;
  overflow-wrap: anywhere;
  color: var(--peek-text);
  font-size: 12.5px;
  line-height: 1.4;
}

.mono {
  font-family: var(--font-mono, ui-monospace, SFMono-Regular, Menlo, Consolas, monospace);
  font-size: 11.5px;
}

.about-copy {
  margin: 0;
  padding: 0;
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.about-copy:hover {
  color: var(--peek-accent);
}

.about-privacy {
  display: flex;
  gap: 10px;
  align-items: flex-start;
  padding: 18px 0 0;
  border-top: 1px solid color-mix(in srgb, var(--peek-border) 85%, transparent);
}

.about-privacy-icon {
  flex: none;
  margin-top: 1px;
  color: var(--peek-muted);
}

.about-privacy h2 {
  color: var(--peek-text);
  font-size: 12.5px;
  font-weight: 600;
  letter-spacing: -0.01em;
  text-transform: none;
}

.about-privacy p {
  margin: 4px 0 0;
  color: var(--peek-muted);
  font-size: 12px;
  line-height: 1.5;
}

.spin {
  animation: about-spin 0.9s linear infinite;
}

@keyframes about-spin {
  to {
    transform: rotate(360deg);
  }
}

@media (max-width: 480px) {
  .about-page {
    padding: 24px 16px 36px;
  }

  .about-update-row {
    flex-wrap: wrap;
    align-items: flex-start;
  }

  .about-actions {
    width: 100%;
    justify-content: flex-start;
  }

  .about-meta > div {
    grid-template-columns: 1fr;
    gap: 4px;
  }
}

@media (prefers-reduced-motion: reduce) {
  .spin {
    animation: none !important;
  }
}
</style>
