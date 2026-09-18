<template>
  <div
    class="pet-companion"
    :class="[
      expressionClass,
      variant === 'pill' ? 'is-pill' : 'is-orb',
      glow === false ? 'no-glow' : '',
    ]"
    :style="rootStyle"
    aria-hidden="true"
  >
    <div class="pet-companion-halo" />
    <div class="pet-companion-body">
      <div class="pet-companion-core" />
      <div class="pet-companion-shine" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import {
  companionExpressionClass,
  normalizePetExpression,
  type PetCompanionConfig,
} from "@/services/pet/appearance";

const props = withDefaults(
  defineProps<{
    expression?: string;
    config?: PetCompanionConfig;
  }>(),
  {
    expression: "idle",
    config: undefined,
  },
);

const variant = computed(() => props.config?.variant ?? "orb");
const glow = computed(() => props.config?.glow ?? true);
const expressionClass = computed(() =>
  companionExpressionClass(normalizePetExpression(props.expression)),
);

const rootStyle = computed(() => {
  const accent = props.config?.accent?.trim();
  if (!accent) return undefined;
  return {
    "--pet-companion-accent": accent,
  };
});
</script>

<style scoped>
.pet-companion {
  --pet-companion-accent: color-mix(in srgb, var(--peek-accent, #5b8def) 72%, #ffffff);
  --pet-companion-depth: color-mix(in srgb, var(--pet-companion-accent) 55%, #0b1020);
  position: relative;
  width: 100%;
  height: 100%;
  display: grid;
  place-items: center;
  pointer-events: none;
}

.pet-companion-halo {
  position: absolute;
  inset: 8%;
  border-radius: 50%;
  background: radial-gradient(
    circle at 50% 50%,
    color-mix(in srgb, var(--pet-companion-accent) 34%, transparent) 0%,
    transparent 72%
  );
  filter: blur(10px);
  animation: companion-breathe 4.8s ease-in-out infinite;
}

.pet-companion-body {
  position: relative;
  width: 78%;
  height: 78%;
  border-radius: 50%;
  background: radial-gradient(
    circle at 35% 28%,
    color-mix(in srgb, #ffffff 78%, var(--pet-companion-accent)) 0%,
    var(--pet-companion-accent) 38%,
    var(--pet-companion-depth) 100%
  );
  box-shadow:
    inset 0 -10px 18px color-mix(in srgb, #000 18%, transparent),
    inset 0 8px 14px color-mix(in srgb, #fff 24%, transparent);
  animation: companion-breathe 4.8s ease-in-out infinite;
}

.pet-companion.is-pill .pet-companion-body {
  width: 62%;
  height: 88%;
  border-radius: 999px;
}

.pet-companion-core {
  position: absolute;
  inset: 18%;
  border-radius: inherit;
  background: radial-gradient(
    circle at 50% 62%,
    color-mix(in srgb, var(--pet-companion-accent) 18%, transparent) 0%,
    transparent 72%
  );
  opacity: 0.85;
}

.pet-companion-shine {
  position: absolute;
  top: 14%;
  left: 22%;
  width: 34%;
  height: 22%;
  border-radius: 50%;
  background: radial-gradient(circle, rgba(255, 255, 255, 0.92) 0%, transparent 72%);
  filter: blur(1px);
  transform: rotate(-18deg);
}

.pet-companion.no-glow .pet-companion-halo {
  opacity: 0;
}

.companion-thinking .pet-companion-body {
  animation:
    companion-spin 6s linear infinite,
    companion-breathe 4.8s ease-in-out infinite;
}

.companion-working .pet-companion-body {
  animation: companion-pulse 1.1s ease-in-out infinite;
}

.companion-talking .pet-companion-halo {
  animation: companion-talk 0.55s ease-in-out infinite;
}

.companion-waiting .pet-companion-body {
  animation: companion-breathe 2.4s ease-in-out infinite;
}

.companion-error .pet-companion-body {
  --pet-companion-accent: #ef6b6b;
  --pet-companion-depth: #5a1414;
}

.companion-sleeping .pet-companion-body {
  animation: none;
  opacity: 0.72;
  filter: saturate(0.75);
}

@keyframes companion-breathe {
  0%,
  100% {
    transform: scale(0.96);
  }
  50% {
    transform: scale(1.04);
  }
}

@keyframes companion-spin {
  from {
    transform: rotate(0deg) scale(1);
  }
  to {
    transform: rotate(360deg) scale(1);
  }
}

@keyframes companion-pulse {
  0%,
  100% {
    transform: scale(0.94);
  }
  50% {
    transform: scale(1.08);
  }
}

@keyframes companion-talk {
  0%,
  100% {
    transform: scale(0.92);
    opacity: 0.55;
  }
  50% {
    transform: scale(1.08);
    opacity: 0.95;
  }
}

@media (prefers-reduced-motion: reduce) {
  .pet-companion-halo,
  .pet-companion-body {
    animation: none !important;
  }
}
</style>
