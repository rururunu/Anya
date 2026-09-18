<template>
  <main
    class="desktop-pet-container"
    :class="{ 'is-locked': locked, 'has-interaction': Boolean(activeInteraction) }"
    @mouseenter="wakeUp"
    @contextmenu.prevent
  >
    <!-- 交互卡片层（当存在待回答/授权请求时在宠物上方浮动弹出） -->
    <div v-if="activeInteraction" class="pet-interaction-layer">
      <DesktopPetInteractionCard
        :interaction="activeInteraction"
        :total-count="interactionCount"
        :current-index="currentInteractionIndex"
        @prev="prevInteraction"
        @next="nextInteraction"
        @submit-ask="submitAskUserAnswer"
        @submit-path="submitPathPermission"
        @submit-tool="submitToolApproval"
        @dismiss="dismissInteraction"
      />
    </div>

    <!-- 吉祥物主体（多样化灵动点击交互，双击打开工作台） -->
    <div
      class="pet-avatar-wrapper"
      :style="avatarStyle"
      @mousedown="onMouseDown"
      @dblclick="onPetDoubleClick"
    >
      <MascotPetView
        v-if="appearance.mode === 'mascot'"
        :expression="expression"
        :action="currentAction"
        :autonomous-action="autonomousAction"
        :autonomous-gaze="autonomousGaze"
        :tilt-direction="tiltDirection"
        :show-combo-decor="showComboDecor"
        :is-combo="isCombo"
        :interactive="true"
        :follow-pointer="true"
        class="pet-mascot"
      />
      <DesktopPetCompanion
        v-else-if="appearance.mode === 'companion'"
        :expression="expression"
        :config="companionConfig"
        class="pet-mascot"
      />
      <DesktopPetSpritesheet
        v-else-if="appearance.mode === 'spritesheet' && appearance.source"
        :manifest-url="appearance.source"
        :expression="expression"
        :gesture="spriteGesture"
        class="pet-mascot"
      />
      <DesktopPetMediaStage v-else-if="appearance.mode === 'media'" :appearance="appearance" />
    </div>
  </main>
</template>

<script setup lang="ts">
import { computed, onMounted } from "vue";
import MascotPetView from "@/components/icons/MascotPetView.vue";
import DesktopPetInteractionCard from "@/components/pet/DesktopPetInteractionCard.vue";
import DesktopPetCompanion from "@/components/pet/DesktopPetCompanion.vue";
import DesktopPetMediaStage from "@/components/pet/DesktopPetMediaStage.vue";
import DesktopPetSpritesheet from "@/components/pet/DesktopPetSpritesheet.vue";
import { PET_SIZES, useDesktopPet } from "@/composables/useDesktopPet";
import type { PetCompanionConfig } from "@/services/pet/appearance";

onMounted(() => {
  if (typeof document !== "undefined") {
    document.documentElement.style.removeProperty("zoom");
    document.documentElement.style.setProperty("--ui-zoom", "1");
  }
});

const {
  expression,
  currentAction,
  autonomousAction,
  autonomousGaze,
  tiltDirection,
  showComboDecor,
  isCombo,
  size,
  locked,
  appearance,
  spriteGesture,
  activeInteraction,
  interactionCount,
  currentInteractionIndex,
  prevInteraction,
  nextInteraction,
  onMouseDown,
  onPetDoubleClick,
  wakeUp,
  submitAskUserAnswer,
  submitPathPermission,
  submitToolApproval,
  dismissInteraction,
} = useDesktopPet();

const companionConfig = computed(() => {
  if (appearance.value.mode !== "companion" || !appearance.value.config) return undefined;
  return appearance.value.config as PetCompanionConfig;
});

const avatarStyle = computed(() => {
  const conf = PET_SIZES[size.value];
  const padX = (conf.windowWidth - conf.sizePx) / 2;
  const padY = (conf.windowHeight - conf.sizePx) / 2;
  return {
    width: `${conf.sizePx}px`,
    height: `${conf.sizePx}px`,
    marginRight: `${padX}px`,
    marginBottom: `${padY}px`,
  };
});
</script>

<style scoped>
.desktop-pet-container {
  position: fixed;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  justify-content: flex-end;
  user-select: none;
  background: transparent !important;
  border: none !important;
  outline: none !important;
  box-shadow: none !important;
  overflow: visible;
  pointer-events: none;
  zoom: 1 !important;
}

/* 交互卡片层 */
.pet-interaction-layer {
  width: 100%;
  display: flex;
  justify-content: center;
  margin-bottom: 8px;
  pointer-events: auto;
}

/* 吉祥物包装层 */
.pet-avatar-wrapper {
  position: relative;
  display: grid;
  place-items: center;
  cursor: grab;
  background: transparent !important;
  border: none !important;
  outline: none !important;
  box-shadow: none !important;
  transform-origin: 50% 90%;
  transition: transform 0.16s cubic-bezier(0.34, 1.4, 0.64, 1);
  filter: drop-shadow(0 10px 24px rgba(0, 0, 0, 0.24)) drop-shadow(0 2px 6px rgba(0, 0, 0, 0.12));
  pointer-events: auto;
}

.pet-avatar-wrapper:active {
  cursor: grabbing;
  transform: scale(1.08, 0.9) translateY(4px);
  transition: transform 0.08s ease-out;
}

.is-locked .pet-avatar-wrapper {
  cursor: pointer;
}

.pet-mascot {
  width: 100%;
  height: 100%;
}
</style>
