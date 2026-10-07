<template>
  <div ref="root" class="pet-svg-root" @pointermove="onPointerMove" @pointerleave="onPointerLeave">
    <svg
      viewBox="0 0 512 512"
      xmlns="http://www.w3.org/2000/svg"
      role="img"
      :aria-label="petAriaLabel"
    >
      <path class="pet-body" :d="bodyPath" />
      <g class="pet-face" :style="faceStyle">
        <g class="eye-pose" :style="leftEyeStyle">
          <g class="eye-blink" :class="{ 'is-blinking': blinking || winking }">
            <rect class="pet-eye" x="-17" y="-46" width="34" height="92" rx="17" />
          </g>
        </g>
        <g class="eye-pose" :style="rightEyeStyle">
          <g class="eye-blink" :class="{ 'is-blinking': blinking }">
            <rect class="pet-eye" x="-17" y="-46" width="34" height="92" rx="17" />
          </g>
        </g>
      </g>
    </svg>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { MascotExpression } from "@/components/icons/MascotPetView.vue";
import type { PetAction, TiltDirection } from "@/composables/useDesktopPetMotion";
import type { AutonomousAction } from "@/composables/useDesktopPetAutonomous";
import { tr } from "@/services/i18n";
import type { AppLanguage } from "@/types/setting";
import {
  interpolatePetOutline,
  petOutline,
  petOutlinePath,
  petPressureEase,
  squeezePetOutline,
  type PetShape,
} from "./petSvgGeometry";

const props = withDefaults(
  defineProps<{
    expression?: MascotExpression;
    action?: PetAction;
    autonomousAction?: AutonomousAction;
    autonomousGaze?: { x: number; y: number } | null;
    tiltDirection?: TiltDirection;
    showComboDecor?: boolean;
    isCombo?: boolean;
    interactive?: boolean;
    followPointer?: boolean;
    edgePressure?: { x: number; y: number };
    language?: AppLanguage;
  }>(),
  {
    expression: "idle",
    action: "idle",
    autonomousAction: "none",
    autonomousGaze: null,
    tiltDirection: "center",
    showComboDecor: false,
    isCombo: false,
    interactive: true,
    followPointer: true,
    edgePressure: () => ({ x: 0, y: 0 }),
    language: "en-US",
  },
);
const petAriaLabel = computed(() => tr(props.language, "shell.pet.aria"));

const root = ref<HTMLElement | null>(null);
const outline = ref<number[]>([...petOutline("rest")]);
const pointer = ref({ x: 0, y: 0 });
const hoverShape = ref<PetShape>("rest");
const blinking = ref(false);
const winking = ref(false);
const lingeringMood = ref<"happy" | "playful" | "surprised" | null>(null);
const petting = ref(false);
const renderedPressure = ref({ ...props.edgePressure });
let frame = 0;
let pressureFrame = 0;
let blinkTimer: ReturnType<typeof setTimeout> | null = null;
let blinkEndTimer: ReturnType<typeof setTimeout> | null = null;
let winkTimer: ReturnType<typeof setTimeout> | null = null;
let moodTimer: ReturnType<typeof setTimeout> | null = null;
let pettingTimer: ReturnType<typeof setTimeout> | null = null;
let lastHoverX = 0;
let lastHoverTime = 0;
let strokeTravel = 0;

function scheduleBlink() {
  blinkTimer = setTimeout(
    () => {
      if (props.expression !== "sleeping") {
        blinking.value = true;
        blinkEndTimer = setTimeout(() => {
          blinking.value = false;
        }, 125);
      }
      scheduleBlink();
    },
    2300 + Math.random() * 2600,
  );
}
onMounted(scheduleBlink);

watch(
  () => props.action,
  (action) => {
    if (action === "idle") return;
    lingeringMood.value =
      action === "curious"
        ? "surprised"
        : action === "jiggle" || action === "twirl"
          ? "playful"
          : "happy";
    if (moodTimer) clearTimeout(moodTimer);
    moodTimer = setTimeout(() => {
      lingeringMood.value = null;
    }, 1200);
    if (action === "jiggle" || action === "twirl") {
      winking.value = true;
      if (winkTimer) clearTimeout(winkTimer);
      winkTimer = setTimeout(() => {
        winking.value = false;
      }, 290);
    }
  },
);

const shape = computed<PetShape>(() => {
  if (props.action === "bounce") return "hop";
  if (props.action === "double-hop") return "doubleHop";
  if (props.action === "jiggle") return "squash";
  if (props.action === "dizzy") return "dizzy";
  if (props.action === "curious") return "curious";
  if (props.action === "twirl" || props.action === "spin") return "twirl";
  if (props.autonomousAction === "stretch") return "stretch";
  if (props.autonomousAction === "morph") return "morph";
  if (props.autonomousAction === "wrench") return "curious";
  if (props.tiltDirection === "left") return "left";
  if (props.tiltDirection === "right") return "right";
  return hoverShape.value;
});

watch(
  shape,
  (next) => {
    if (frame) cancelAnimationFrame(frame);
    const from = [...outline.value];
    const to = petOutline(next);
    if (
      typeof window === "undefined" ||
      window.matchMedia?.("(prefers-reduced-motion: reduce)").matches
    ) {
      outline.value = [...to];
      return;
    }
    const start = performance.now();
    const duration = next === "rest" ? 360 : 240;
    const tick = (time: number) => {
      const t = Math.min(1, (time - start) / duration);
      const eased = 1 - Math.pow(1 - t, 3);
      outline.value = interpolatePetOutline(from, to, eased);
      frame = t < 1 ? requestAnimationFrame(tick) : 0;
    };
    frame = requestAnimationFrame(tick);
  },
  { immediate: true },
);

watch(
  () => [props.edgePressure.x, props.edgePressure.y] as const,
  ([x, y]) => {
    if (pressureFrame) cancelAnimationFrame(pressureFrame);
    const from = { ...renderedPressure.value };
    const to = { x: Math.max(-1, Math.min(1, x)), y: Math.max(-1, Math.min(1, y)) };
    if (
      typeof window === "undefined" ||
      window.matchMedia?.("(prefers-reduced-motion: reduce)").matches
    ) {
      renderedPressure.value = to;
      return;
    }
    const releasing = to.x === 0 && to.y === 0;
    const duration = releasing ? 340 : 135;
    const start = performance.now();
    const tick = (time: number) => {
      const t = Math.min(1, (time - start) / duration);
      const ease = petPressureEase(t, releasing);
      renderedPressure.value = {
        x: from.x + (to.x - from.x) * ease,
        y: from.y + (to.y - from.y) * ease,
      };
      pressureFrame = t < 1 ? requestAnimationFrame(tick) : 0;
    };
    pressureFrame = requestAnimationFrame(tick);
  },
);

onBeforeUnmount(() => {
  if (frame) cancelAnimationFrame(frame);
  if (pressureFrame) cancelAnimationFrame(pressureFrame);
  if (blinkTimer) clearTimeout(blinkTimer);
  if (blinkEndTimer) clearTimeout(blinkEndTimer);
  if (winkTimer) clearTimeout(winkTimer);
  if (moodTimer) clearTimeout(moodTimer);
  if (pettingTimer) clearTimeout(pettingTimer);
});

const bodyPath = computed(() =>
  petOutlinePath(squeezePetOutline(outline.value, renderedPressure.value)),
);
const isPressedAtEdge = computed(
  () => Math.max(Math.abs(renderedPressure.value.x), Math.abs(renderedPressure.value.y)) > 0.12,
);
type FaceMood =
  "neutral" | "happy" | "playful" | "surprised" | "pressed" | "worried" | "sleepy" | "thinking";
const faceMood = computed<FaceMood>(() => {
  if (isPressedAtEdge.value) return "pressed";
  if (props.action === "dizzy" || props.expression === "error") return "worried";
  if (props.expression === "sleeping" || props.autonomousAction === "stretch") return "sleepy";
  if (props.expression === "done") return "happy";
  if (props.expression === "thinking") return "thinking";
  if (
    props.expression === "waiting" ||
    props.action === "curious" ||
    props.autonomousAction === "morph"
  )
    return "surprised";
  if (props.action === "jiggle" || props.action === "twirl") return "playful";
  if (props.action !== "idle") return "happy";
  if (petting.value) return "happy";
  if (props.isCombo || props.showComboDecor) return "playful";
  if (hoverShape.value === "curious") return "surprised";
  if (hoverShape.value !== "rest") return "playful";
  return lingeringMood.value || "neutral";
});
function eyePose(side: "left" | "right") {
  const x = side === "left" ? 208 : 304;
  const glance = Math.max(-1, Math.min(1, pointer.value.x / 17));
  const expression = faceMood.value;
  const y = props.expression === "thinking" ? 273 : props.expression === "working" ? 301 : 290;
  let sx = 1,
    sy = props.expression === "talking" ? 0.83 : 1,
    angle = side === "left" ? -2 : 2;
  if (expression === "happy") {
    sy = 0.22;
    sx = 1.22;
    angle = side === "left" ? -13 : 13;
  }
  if (expression === "playful") {
    sy = side === "left" ? 0.56 : 0.87;
    angle = side === "left" ? -12 : 11;
  }
  if (expression === "surprised") {
    sy = 0.69;
    sx = side === "left" ? 1.35 : 1.22;
    angle = side === "left" ? -7 : 7;
  }
  if (expression === "worried") {
    sy = 0.78;
    angle = side === "left" ? -8 : 8;
  }
  if (expression === "pressed") {
    const force = Math.min(
      1,
      Math.max(Math.abs(renderedPressure.value.x), Math.abs(renderedPressure.value.y)),
    );
    sy = 1 - force * 0.18;
    angle = renderedPressure.value.x * 5;
  }
  if (expression === "sleepy") {
    sy = 0.12;
    angle = side === "left" ? -9 : 9;
  }
  if (expression === "thinking") {
    sy = 0.82;
    angle = side === "left" ? -9 : 6;
  }
  return {
    transform: `translate(${x + glance * (side === "left" ? 3 : 5)}px, ${y}px) rotate(${angle}deg) scale(${sx}, ${sy})`,
  };
}
const leftEyeStyle = computed(() => eyePose("left"));
const rightEyeStyle = computed(() => eyePose("right"));
const faceStyle = computed(() => {
  const impact =
    props.action !== "idle"
      ? props.tiltDirection === "left"
        ? -9
        : props.tiltDirection === "right"
          ? 9
          : 0
      : 0;
  const force = Math.min(
    1,
    Math.max(Math.abs(renderedPressure.value.x), Math.abs(renderedPressure.value.y)),
  );
  const x =
    (props.autonomousGaze?.x ?? pointer.value.x) * (1 - force) +
    impact +
    renderedPressure.value.x * 13;
  const y =
    (props.autonomousGaze?.y ?? pointer.value.y) * (1 - force) + renderedPressure.value.y * 9;
  return { transform: `translate(${x.toFixed(1)}px, ${y.toFixed(1)}px)` };
});

function onPointerMove(event: PointerEvent) {
  if (!props.interactive || !props.followPointer || !root.value) return;
  const rect = root.value.getBoundingClientRect();
  pointer.value = {
    x: Math.max(-17, Math.min(17, ((event.clientX - rect.left) / rect.width - 0.5) * 34)),
    y: Math.max(-12, Math.min(12, ((event.clientY - rect.top) / rect.height - 0.5) * 24)),
  };
  const horizontal = (event.clientX - rect.left) / rect.width;
  const vertical = (event.clientY - rect.top) / rect.height;
  const now = performance.now();
  if (!event.buttons && vertical < 0.46 && lastHoverTime > 0 && now - lastHoverTime < 450) {
    strokeTravel += Math.abs(event.clientX - lastHoverX);
    if (strokeTravel > Math.max(38, rect.width * 0.32)) {
      petting.value = true;
      strokeTravel = 0;
      if (pettingTimer) clearTimeout(pettingTimer);
      pettingTimer = setTimeout(() => {
        petting.value = false;
      }, 1100);
    }
  } else if (now - lastHoverTime >= 450) {
    strokeTravel = 0;
  }
  lastHoverX = event.clientX;
  lastHoverTime = now;
  hoverShape.value =
    vertical < 0.3 ? "curious" : horizontal < 0.32 ? "left" : horizontal > 0.68 ? "right" : "rest";
}
function onPointerLeave() {
  pointer.value = { x: 0, y: 0 };
  hoverShape.value = "rest";
  strokeTravel = 0;
}
</script>

<style scoped>
.pet-svg-root,
svg {
  display: block;
  width: 100%;
  height: 100%;
  overflow: visible;
}
.pet-body {
  fill: var(--peek-text, #242424);
}
.pet-eye {
  fill: var(--peek-bg, #fff);
}
.eye-pose {
  transition: transform 230ms cubic-bezier(0.2, 1.18, 0.4, 1);
}
.eye-blink {
  transition: transform 85ms ease-in-out;
}
.eye-blink.is-blinking {
  transform: scaleY(0.06);
}
.pet-face {
  transition: transform 120ms ease-out;
}
@media (prefers-reduced-motion: reduce) {
  .pet-face {
    transition: none;
  }
  .eye-pose,
  .eye-blink {
    transition: none;
  }
}
</style>
