<template>
  <section class="settings-page pet-page">
    <header class="pet-page-header">
      <div class="pet-mascot" aria-hidden="true">
        <MascotFace interactive />
      </div>
      <div class="pet-header-copy">
        <h2>{{ copy.title }}</h2>
        <p>{{ copy.intro }}</p>
      </div>
    </header>

    <div class="settings-group">
      <div class="settings-card">
        <article class="settings-row">
          <div class="settings-row-copy">
            <h3>{{ copy.visibilityTitle }}</h3>
            <p>{{ copy.visibilityHint }}</p>
          </div>
          <div class="settings-row-control">
            <SettingsToggle :model-value="isPetVisible" @click.prevent="handleTogglePet" />
          </div>
        </article>

        <article class="settings-row">
          <div class="settings-row-copy">
            <h3>{{ copy.sizeTitle }}</h3>
            <p>{{ copy.sizeHint }}</p>
          </div>
          <div class="settings-row-control">
            <div class="pet-size-pills" role="radiogroup" :aria-label="copy.sizeTitle">
              <button
                v-for="option in SIZE_OPTIONS"
                :key="option.value"
                type="button"
                class="pet-size-pill"
                :class="{ active: petSize === option.value }"
                :aria-checked="petSize === option.value"
                role="radio"
                @click="handleSetPetSize(option.value)"
              >
                {{ option.label }}
              </button>
            </div>
          </div>
        </article>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import MascotFace from "@/components/icons/MascotFace.vue";
import SettingsToggle from "@/components/settings/SettingsToggle.vue";
import { getDesktopPetVisible, setDesktopPetSize, toggleDesktopPet } from "@/services/ipc/commands";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";
import { IPC_EVENTS } from "@/types/ipc";
import type { PetSize } from "@/composables/useDesktopPet";

const settingStore = useSettingStore();
const petSize = ref<PetSize>("medium");
const isPetVisible = ref(false);
let unlistenPet: UnlistenFn | undefined;

const SIZE_OPTIONS = computed<Array<{ value: PetSize; label: string }>>(() => [
  { value: "small", label: tr(settingStore.language, "settings.pet.sizeSmall") },
  { value: "medium", label: tr(settingStore.language, "settings.pet.sizeMedium") },
  { value: "large", label: tr(settingStore.language, "settings.pet.sizeLarge") },
]);

const copy = computed(() => ({
  title: tr(settingStore.language, "settings.categories.pet"),
  intro: tr(settingStore.language, "settings.pet.intro"),
  visibilityTitle: tr(settingStore.language, "settings.pet.visibilityTitle"),
  visibilityHint: tr(settingStore.language, "settings.pet.visibilityHint"),
  sizeTitle: tr(settingStore.language, "settings.pet.sizeTitle"),
  sizeHint: tr(settingStore.language, "settings.pet.sizeHint"),
}));

async function handleTogglePet() {
  const next = await toggleDesktopPet();
  isPetVisible.value = next;
}

async function handleSetPetSize(nextSize: PetSize) {
  petSize.value = nextSize;
  try {
    localStorage.setItem("anya.desktop-pet.size", nextSize);
    await setDesktopPetSize(nextSize);
  } catch (e) {
    console.warn("Failed to set pet size:", e);
  }
}

onMounted(async () => {
  const savedSize = localStorage.getItem("anya.desktop-pet.size") as PetSize | null;
  if (savedSize && (savedSize === "small" || savedSize === "medium" || savedSize === "large")) {
    petSize.value = savedSize;
  }
  try {
    isPetVisible.value = await getDesktopPetVisible();
  } catch (e) {
    console.warn("Failed to query pet visibility:", e);
  }
  unlistenPet = await listen<boolean>(IPC_EVENTS.desktopPetVisibilityChanged, (event) => {
    isPetVisible.value = event.payload;
  });
});

onBeforeUnmount(() => {
  if (unlistenPet) unlistenPet();
});
</script>

<style scoped>
.pet-page {
  width: min(100%, 44rem);
}

.pet-page-header {
  display: flex;
  align-items: center;
  gap: 16px;
  margin: 0 0 20px;
}

.pet-mascot {
  width: 104px;
  height: 104px;
  flex: none;
}

.pet-mascot :deep(.mascot-face) {
  opacity: 0.96;
  filter: drop-shadow(0 10px 24px color-mix(in srgb, var(--peek-text) 16%, transparent));
}

.pet-header-copy {
  min-width: 0;
}

.pet-header-copy h2 {
  margin: 0;
  color: var(--peek-text);
  font-size: 22px;
  font-weight: 700;
  letter-spacing: -0.02em;
  line-height: 1.25;
}

.pet-header-copy p {
  margin: 6px 0 0;
  color: var(--peek-muted);
  font-size: 12px;
  line-height: 1.5;
}

.pet-size-pills {
  display: flex;
  gap: 2px;
  padding: 2px;
  border: 1px solid var(--peek-border);
  border-radius: 8px;
  background: color-mix(in srgb, var(--peek-text) 4%, var(--peek-surface));
}

.pet-size-pill {
  padding: 3px 14px;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 500;
  border: none;
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
  transition: all 0.15s ease;
}

.pet-size-pill:hover {
  color: var(--peek-text);
  background: color-mix(in srgb, var(--peek-text) 6%, transparent);
}

.pet-size-pill.active {
  background: var(--peek-accent);
  color: var(--peek-accent-fg, #ffffff);
  font-weight: 600;
}
</style>
