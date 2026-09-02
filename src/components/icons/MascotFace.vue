<template>
  <span
    ref="rootRef"
    class="mascot-face"
    :class="[`st-${visualState}`, { interactive, pleased, turning }]"
    :role="alt ? 'img' : undefined"
    :aria-label="alt || undefined"
    :aria-hidden="alt ? undefined : true"
    @click="handleClick"
  >
    <svg viewBox="64 76 384 384" xmlns="http://www.w3.org/2000/svg" focusable="false">
      <!-- No clip-path on purpose: the gaze range keeps the eyes well inside the body,
           and shared/hidden clipPath ids across instances can clip everything away. -->
      <g class="mascot-all" :class="{ wiggle }">
        <g class="mascot-turner">
          <!-- The item for the current mode is tucked behind the body and peeks out at the
             lower right, so it reads as "carried" without needing arms or hands. -->
          <Transition name="mascot-tool">
            <g v-if="tool" :key="tool" class="mascot-tool">
              <g class="mascot-tool-bob">
                <!-- ask: a ringing phone beside its head -->
                <template v-if="tool === 'phone'">
                  <g transform="translate(440 262) rotate(16)">
                    <rect class="mp-obj" x="-32" y="-60" width="64" height="120" rx="16" />
                    <rect class="mp-dot" x="-22" y="-44" width="44" height="72" rx="6" />
                    <circle class="mp-dot" cx="0" cy="44" r="5" />
                    <path
                      class="mp-line mp-ring"
                      d="M46 -30a34 34 0 0 1 0 60M60 -46a52 52 0 0 1 0 92"
                    />
                  </g>
                </template>

                <!-- agent: a chunky wrench poking out from behind -->
                <template v-else-if="tool === 'wrench'">
                  <g transform="translate(424 352) rotate(42)">
                    <rect class="mp-obj" x="-14" y="-40" width="28" height="150" rx="14" />
                    <circle class="mp-obj" cx="0" cy="-62" r="34" />
                    <path class="mp-dot" d="M-12 -104h24v34a12 12 0 0 1-24 0z" />
                  </g>
                </template>

                <!-- plan: a clipboard with a checklist -->
                <template v-else-if="tool === 'clipboard'">
                  <g transform="translate(438 366) rotate(-12)">
                    <rect class="mp-obj" x="-52" y="-66" width="104" height="132" rx="14" />
                    <rect class="mp-dot" x="-22" y="-78" width="44" height="26" rx="9" />
                    <rect class="mp-line" x="-32" y="-34" width="18" height="18" rx="5" />
                    <path class="mp-line" d="M-29 -25l5 5 9-10" />
                    <path class="mp-line" d="M-2 -25H32" />
                    <rect class="mp-line" x="-32" y="0" width="18" height="18" rx="5" />
                    <path class="mp-line" d="M-29 9l5 5 9-10" />
                    <path class="mp-line" d="M-2 9H32" />
                    <rect class="mp-line" x="-32" y="34" width="18" height="18" rx="5" />
                    <path class="mp-line" d="M-2 43H32" />
                  </g>
                </template>

                <!-- image: a fat paintbrush with a drop of paint -->
                <template v-else-if="tool === 'brush'">
                  <g transform="translate(432 336) rotate(44)">
                    <rect class="mp-obj" x="-12" y="-20" width="24" height="150" rx="12" />
                    <rect class="mp-obj" x="-17" y="-52" width="34" height="34" rx="6" />
                    <path class="mp-obj" d="M-17 -52L-24 -96Q0 -118 24 -96L17 -52Z" />
                    <path class="mp-dot" d="M-36 -112c0 -8 8 -22 8 -22s8 14 8 22a8 8 0 0 1-16 0z" />
                  </g>
                </template>
              </g>
            </g>
          </Transition>

          <path class="mascot-body" :d="BLOB_PATH" />
          <!-- Every eye variant is always in the DOM; the state class picks which one shows,
             so switching states cross-fades instead of re-mounting. The extra wrapper
             carries the "turn around" animation played when the state changes. -->
          <g class="mascot-face-turn">
            <g class="mascot-eyes" :style="eyesStyle">
              <g transform="translate(208 290)">
                <rect
                  class="mascot-eye eye-l"
                  :class="{ blink: blinking }"
                  x="-17"
                  y="-46"
                  width="34"
                  height="92"
                  rx="17"
                />
                <path class="mascot-smile-eye" d="M-26 10A26 26 0 0 1 26 10" />
                <g class="mascot-round-eye">
                  <circle cx="0" cy="0" r="25" />
                  <circle class="mascot-glint" cx="-9" cy="-10" r="7" />
                </g>
                <path class="mascot-ouch-eye" d="M-14 -18L14 0L-14 18" />
              </g>
              <g transform="translate(304 290)">
                <rect
                  class="mascot-eye eye-r"
                  :class="{ blink: blinking }"
                  x="-17"
                  y="-46"
                  width="34"
                  height="92"
                  rx="17"
                />
                <path class="mascot-smile-eye" d="M-26 10A26 26 0 0 1 26 10" />
                <g class="mascot-round-eye">
                  <circle cx="0" cy="0" r="25" />
                  <circle class="mascot-glint" cx="-9" cy="-10" r="7" />
                </g>
                <path class="mascot-ouch-eye" d="M14 -18L-14 0L14 18" />
              </g>
            </g>
          </g>
        </g>
      </g>
    </svg>
  </span>
</template>

<script lang="ts">
/** Items the mascot can hold; the caller maps app state (e.g. chat mode) onto these. */
export type MascotTool = "phone" | "wrench" | "clipboard" | "brush";

/**
 * Facial state, roughly mirroring an agent turn:
 * idle (blinks, follows) / thinking (eyes drift up) / working (eyes scan) /
 * talking (eyes pulse) / waiting (wide round eyes) / done (happy squint) / error (> < eyes).
 */
export type MascotState =
  "idle" | "thinking" | "working" | "talking" | "waiting" | "done" | "error";
</script>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useEventListener } from "@vueuse/core";

/**
 * The blob mascot's face: two pill eyes that follow the pointer while it is
 * inside the window, look at `lookAt` when given (e.g. the composer while the
 * user types), and rest centred + blinking otherwise.
 */
const props = withDefaults(
  defineProps<{
    alt?: string;
    /** Element or viewport point the eyes should look at; overrides the pointer. */
    lookAt?: HTMLElement | { x: number; y: number } | null;
    /** Follow the pointer while it is inside the window. */
    followPointer?: boolean;
    /** Clicking the face makes it beam (squinty happy eyes + head tilt + wiggle). */
    interactive?: boolean;
    /** Facial state; see MascotState. */
    state?: MascotState;
    /** Play the "turn around" flip when `state` changes (the new face appears mid-turn). */
    turnOnChange?: boolean;
    /** Shorthand for `state="working"` (eyes scan left and right). */
    busy?: boolean;
    /** Item carried behind the lower right of the body; null hides it. */
    tool?: MascotTool | null;
  }>(),
  {
    alt: "",
    lookAt: null,
    followPointer: true,
    interactive: false,
    state: "idle",
    turnOnChange: true,
    busy: false,
    tool: null,
  },
);

const emit = defineEmits<{ click: [] }>();

const BLOB_PATH =
  "M256 108C356 108 428 164 428 272C428 388 381 428 256 428C131 428 84 388 84 272C84 164 156 108 256 108Z";

/* How far (in SVG units) the eyes may travel from centre. */
const MAX_DX = 30;
const MAX_DY = 22;
/* Screen distance (px) at which the gaze is roughly half saturated. */
const GAZE_SOFTNESS = 140;

const rootRef = ref<HTMLElement | null>(null);
const pointer = ref<{ x: number; y: number } | null>(null);
const gaze = ref({ x: 0, y: 0 });
const blinking = ref(false);
const pleased = ref(false);
const wiggle = ref(false);

/* State changes are not applied instantly: the face turns away (0.6s), the new
   expression is swapped in while it is on the back side, and it turns back.
   Agent turns flip state very quickly (tool start/stop, text chunks), so a change
   must hold for SETTLE_MS before it counts, and turns are spaced by MIN_GAP_MS,
   otherwise the face would flicker. */
const TURN_MS = 600;
const SETTLE_MS = 450;
const MIN_GAP_MS = 1400;
const shownState = ref<MascotState>(props.state);
const turning = ref(false);
let settleTimer: ReturnType<typeof setTimeout> | null = null;
let turnSwapTimer: ReturnType<typeof setTimeout> | null = null;
let turnEndTimer: ReturnType<typeof setTimeout> | null = null;
let lastTurnEnd = 0;

function reducedMotion() {
  return (
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true
  );
}

function clearTurnTimers() {
  if (settleTimer) clearTimeout(settleTimer);
  if (turnSwapTimer) clearTimeout(turnSwapTimer);
  if (turnEndTimer) clearTimeout(turnEndTimer);
  settleTimer = turnSwapTimer = turnEndTimer = null;
}

function startTurn() {
  turning.value = true;
  turnSwapTimer = setTimeout(() => {
    shownState.value = props.state;
  }, TURN_MS / 2);
  turnEndTimer = setTimeout(() => {
    shownState.value = props.state;
    turning.value = false;
    lastTurnEnd = Date.now();
    // The state moved on again while we were turning: queue another (settled) turn.
    if (props.state !== shownState.value) scheduleTurn();
  }, TURN_MS);
}

function scheduleTurn() {
  if (settleTimer) clearTimeout(settleTimer);
  const sinceLast = Date.now() - lastTurnEnd;
  const wait = Math.max(SETTLE_MS, MIN_GAP_MS - sinceLast);
  settleTimer = setTimeout(() => {
    settleTimer = null;
    if (props.state === shownState.value || turning.value) return;
    startTurn();
  }, wait);
}

watch(
  () => props.state,
  (next) => {
    if (!props.turnOnChange || reducedMotion()) {
      clearTurnTimers();
      shownState.value = next;
      return;
    }
    if (next === shownState.value) {
      // Flipped back before the change settled: nothing to do.
      if (settleTimer) clearTimeout(settleTimer);
      settleTimer = null;
      return;
    }
    if (turning.value) return; // the running turn will pick up the latest state
    scheduleTurn();
  },
);

/* A click-triggered beam wins over whatever state the caller set. */
const visualState = computed<MascotState>(() => {
  if (pleased.value) return "done";
  if (props.busy && shownState.value === "idle") return "working";
  return shownState.value;
});

const eyesStyle = computed(() => ({
  transform: `translate(${gaze.value.x.toFixed(1)}px, ${gaze.value.y.toFixed(1)}px)`,
}));

/* Clicking makes it beam for a moment: happy squint, head tilt and a little wiggle. */
const PLEASED_MS = 1800;
let pleasedTimer: ReturnType<typeof setTimeout> | null = null;
let wiggleFrame = 0;

function handleClick() {
  emit("click");
  if (!props.interactive) return;
  pleased.value = true;
  scheduleGaze();
  if (pleasedTimer) clearTimeout(pleasedTimer);
  pleasedTimer = setTimeout(() => {
    pleased.value = false;
    scheduleGaze();
  }, PLEASED_MS);
  // Restart the wiggle even when clicked repeatedly.
  wiggle.value = false;
  if (wiggleFrame) cancelAnimationFrame(wiggleFrame);
  wiggleFrame = requestAnimationFrame(() => {
    wiggleFrame = 0;
    wiggle.value = true;
  });
}

function targetPoint(): { x: number; y: number } | null {
  const target = props.lookAt;
  if (target) {
    if (target instanceof HTMLElement) {
      const rect = target.getBoundingClientRect();
      if (rect.width === 0 && rect.height === 0) return null;
      // Aim at the upper part of the box (where the text is), not its centre.
      return { x: rect.left + rect.width / 2, y: rect.top + Math.min(rect.height / 2, 40) };
    }
    return target;
  }
  return props.followPointer ? pointer.value : null;
}

function updateGaze() {
  const root = rootRef.value;
  // While beaming it looks straight at the user.
  const target = pleased.value ? null : targetPoint();
  if (!root || !target) {
    gaze.value = { x: 0, y: 0 };
    return;
  }
  const rect = root.getBoundingClientRect();
  const cx = rect.left + rect.width / 2;
  const cy = rect.top + rect.height / 2;
  const dx = target.x - cx;
  const dy = target.y - cy;
  const dist = Math.hypot(dx, dy);
  if (dist < 1) {
    gaze.value = { x: 0, y: 0 };
    return;
  }
  // Soft saturation: near targets move the eyes a little, far ones up to the max.
  const k = dist / (dist + GAZE_SOFTNESS);
  gaze.value = { x: (dx / dist) * k * MAX_DX, y: (dy / dist) * k * MAX_DY };
}

let gazeFrame = 0;
function scheduleGaze() {
  if (gazeFrame) return;
  gazeFrame = requestAnimationFrame(() => {
    gazeFrame = 0;
    updateGaze();
  });
}

useEventListener(document, "mousemove", (event: MouseEvent) => {
  pointer.value = { x: event.clientX, y: event.clientY };
  scheduleGaze();
});
useEventListener(document, "mouseleave", () => {
  pointer.value = null;
  scheduleGaze();
});
useEventListener(window, "blur", () => {
  pointer.value = null;
  scheduleGaze();
});
useEventListener(window, "resize", scheduleGaze);
useEventListener(window, "scroll", scheduleGaze, { capture: true, passive: true });

watch(() => props.lookAt, scheduleGaze);

/* Blinking: random 2.4s-5.2s gaps, occasionally a double blink. */
let blinkTimer: ReturnType<typeof setTimeout> | null = null;
function scheduleBlink(delay = 2400 + Math.random() * 2800) {
  blinkTimer = setTimeout(() => {
    blinking.value = true;
    blinkTimer = setTimeout(() => {
      blinking.value = false;
      if (Math.random() < 0.2) {
        blinkTimer = setTimeout(() => {
          blinking.value = true;
          blinkTimer = setTimeout(() => {
            blinking.value = false;
            scheduleBlink();
          }, 120);
        }, 160);
      } else {
        scheduleBlink();
      }
    }, 130);
  }, delay);
}

onMounted(() => {
  updateGaze();
  scheduleBlink(1200);
});
onBeforeUnmount(() => {
  if (blinkTimer) clearTimeout(blinkTimer);
  if (pleasedTimer) clearTimeout(pleasedTimer);
  clearTurnTimers();
  if (gazeFrame) cancelAnimationFrame(gazeFrame);
  if (wiggleFrame) cancelAnimationFrame(wiggleFrame);
});
</script>

<style scoped>
.mascot-face {
  display: block;
  width: 100%;
  height: 100%;
}
.mascot-face.interactive {
  cursor: pointer;
  user-select: none;
  -webkit-tap-highlight-color: transparent;
}
.mascot-face svg {
  display: block;
  width: 100%;
  height: 100%;
  overflow: visible;
  transform-origin: 50% 60%;
  transition: transform 320ms cubic-bezier(0.34, 1.4, 0.64, 1);
}
.mascot-face.pleased svg {
  transform: rotate(-6deg);
}
.mascot-all {
  transform-origin: 256px 300px;
}
.mascot-all.wiggle {
  animation: mascot-wiggle 720ms cubic-bezier(0.34, 1.3, 0.64, 1);
}
.mascot-body {
  fill: var(--peek-text);
}
.mascot-eyes {
  transition: transform 180ms cubic-bezier(0.2, 0.7, 0.3, 1);
  will-change: transform;
}
.mascot-eye {
  fill: var(--peek-bg);
  transform-box: fill-box;
  transform-origin: center;
  transition:
    transform 70ms ease-out,
    opacity 120ms ease-out;
}
/* thinking / working squint the pills so they differ from idle even when tiny */
.st-thinking .mascot-eye,
.st-working .mascot-eye {
  transform: scaleY(0.62);
}
.mascot-eye.blink {
  transform: scaleY(0.08);
}
.mascot-smile-eye,
.mascot-round-eye,
.mascot-ouch-eye {
  opacity: 0;
  transform-box: fill-box;
  transform-origin: center;
  transform: scale(0.6);
  transition:
    opacity 140ms ease-out,
    transform 260ms cubic-bezier(0.34, 1.56, 0.64, 1);
}
.mascot-smile-eye {
  fill: none;
  stroke: var(--peek-bg);
  stroke-width: 18;
  stroke-linecap: round;
}
.mascot-round-eye {
  fill: var(--peek-bg);
}
.mascot-glint {
  fill: var(--peek-text);
}
.mascot-ouch-eye {
  fill: none;
  stroke: var(--peek-bg);
  stroke-width: 12;
  stroke-linecap: round;
  stroke-linejoin: round;
}

/* ---- states ---- */
.st-thinking .mascot-eyes {
  animation: mascot-think 3.6s ease-in-out infinite;
}
.st-working .mascot-eyes {
  animation: mascot-scan 1.6s ease-in-out infinite;
}
.st-talking .mascot-eye {
  animation: mascot-talk 0.42s ease-in-out infinite;
}
.st-talking .mascot-eye.eye-r {
  animation-delay: -0.2s;
}
.st-waiting .mascot-eye,
.st-done .mascot-eye,
.st-error .mascot-eye {
  opacity: 0;
}
.st-waiting .mascot-round-eye,
.st-done .mascot-smile-eye,
.st-error .mascot-ouch-eye {
  opacity: 1;
  transform: scale(1);
}
.st-error .mascot-eyes {
  animation: mascot-shake 0.1s linear infinite alternate;
}

/* held item: pops in when the mode changes, then gently sways as if being waved */
.mascot-tool {
  transform-box: fill-box;
  transform-origin: center;
}
.mascot-tool-enter-active {
  transition:
    transform 340ms cubic-bezier(0.34, 1.56, 0.64, 1),
    opacity 160ms ease-out;
}
.mascot-tool-leave-active {
  transition:
    transform 160ms ease-in,
    opacity 120ms ease-in;
}
.mascot-tool-enter-from,
.mascot-tool-leave-to {
  transform: scale(0.2);
  opacity: 0;
}
.mascot-tool-bob {
  transform-box: fill-box;
  transform-origin: 20% 80%;
  animation: mascot-tool-sway 3.2s ease-in-out infinite;
}
/* prop palette: solid body-coloured silhouettes (same flat style as the character) with a
   page-coloured outline that separates them from the body where they overlap */
.mp-obj {
  fill: var(--peek-text);
  stroke: var(--peek-bg);
  stroke-width: 7;
  stroke-linejoin: round;
  stroke-linecap: round;
  paint-order: stroke;
}
.mp-dot {
  fill: var(--peek-bg);
}
.mp-line {
  fill: none;
  stroke: var(--peek-bg);
  stroke-width: 7;
  stroke-linecap: round;
  stroke-linejoin: round;
}
.mp-ring {
  stroke: var(--peek-text);
  stroke-width: 8;
}

/* ---- turn around on state change ---- */
.mascot-turner {
  transform-origin: 256px 268px;
}
.mascot-face-turn {
  transform-origin: 256px 290px;
}
.turning .mascot-turner {
  animation: mascot-turn-body 600ms ease-in-out both;
}
.turning .mascot-face-turn {
  animation: mascot-turn-face 600ms linear both;
}
@keyframes mascot-turn-body {
  0%,
  100% {
    transform: translateY(0) scale(1);
  }
  30%,
  70% {
    transform: translateY(-8px) scale(0.97, 1.02);
  }
  50% {
    transform: translateY(-11px) scale(1);
  }
}
/* face = point on a sphere sampled every 15deg: x = R*sin(a), width = cos(a) */
@keyframes mascot-turn-face {
  0% {
    transform: translateX(0) scaleX(1);
    opacity: 1;
  }
  5.6% {
    transform: translateX(39px) scaleX(0.966);
  }
  11.1% {
    transform: translateX(75px) scaleX(0.866);
  }
  16.7% {
    transform: translateX(106px) scaleX(0.707);
    opacity: 1;
  }
  22.2% {
    transform: translateX(130px) scaleX(0.5);
    opacity: 0.55;
  }
  27.8% {
    transform: translateX(145px) scaleX(0.259);
    opacity: 0;
  }
  33.3% {
    transform: translateX(150px) scaleX(0);
    opacity: 0;
  }
  66.7% {
    transform: translateX(-150px) scaleX(0);
    opacity: 0;
  }
  72.2% {
    transform: translateX(-145px) scaleX(0.259);
    opacity: 0;
  }
  77.8% {
    transform: translateX(-130px) scaleX(0.5);
    opacity: 0.55;
  }
  83.3% {
    transform: translateX(-106px) scaleX(0.707);
    opacity: 1;
  }
  88.9% {
    transform: translateX(-75px) scaleX(0.866);
  }
  94.4% {
    transform: translateX(-39px) scaleX(0.966);
  }
  100% {
    transform: translateX(0) scaleX(1);
    opacity: 1;
  }
}
@keyframes mascot-tool-sway {
  0%,
  100% {
    transform: rotate(-4deg);
  }
  50% {
    transform: rotate(5deg);
  }
}
@keyframes mascot-wiggle {
  0% {
    transform: rotate(0deg) scale(1);
  }
  18% {
    transform: rotate(-7deg) scale(1.05, 0.95);
  }
  42% {
    transform: rotate(6deg) scale(0.97, 1.03);
  }
  68% {
    transform: rotate(-3deg) scale(1.01, 0.99);
  }
  100% {
    transform: rotate(0deg) scale(1);
  }
}
/* working: eyes low and darting side to side, like reading output */
@keyframes mascot-scan {
  0%,
  100% {
    transform: translate(-34px, 14px);
  }
  50% {
    transform: translate(34px, 14px);
  }
}
/* thinking: eyes rolled up to the corner, drifting slowly */
@keyframes mascot-think {
  0%,
  100% {
    transform: translate(28px, -30px);
  }
  50% {
    transform: translate(36px, -22px);
  }
}
/* talking: strong equaliser-like pulsing */
@keyframes mascot-talk {
  0%,
  100% {
    transform: scaleY(1);
  }
  30% {
    transform: scaleY(0.42);
  }
  55% {
    transform: scaleY(1.12);
  }
  80% {
    transform: scaleY(0.7);
  }
}
@keyframes mascot-shake {
  from {
    transform: translateX(-2px);
  }
  to {
    transform: translateX(2px);
  }
}
@media (prefers-reduced-motion: reduce) {
  .mascot-face svg,
  .mascot-eyes,
  .mascot-eye,
  .mascot-smile-eye,
  .mascot-round-eye,
  .mascot-ouch-eye {
    transition: none;
  }
  .mascot-all.wiggle,
  .st-thinking .mascot-eyes,
  .st-working .mascot-eyes,
  .st-talking .mascot-eye,
  .st-error .mascot-eyes,
  .turning .mascot-turner,
  .turning .mascot-face-turn,
  .mascot-tool-bob {
    animation: none;
  }
  .mascot-tool-enter-active,
  .mascot-tool-leave-active {
    transition: none;
  }
}
</style>
