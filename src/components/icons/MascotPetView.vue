<template>
  <div
    ref="rootRef"
    class="mascot-pet-view"
    :class="[
      `expr-${visualExpression}`,
      `action-${activeAction}`,
      `auto-${props.autonomousAction || 'none'}`,
      `tilt-${props.tiltDirection}`,
      {
        tilt: isTilt,
        pleased: props.pleased,
        'has-action': activeAction !== 'idle',
        'is-combo': props.isCombo,
      },
    ]"
    @click="handleClick"
  >
    <svg
      viewBox="0 0 512 512"
      xmlns="http://www.w3.org/2000/svg"
      focusable="false"
      class="mascot-pet-svg"
    >
      <defs>
        <clipPath id="pet-blob-clip">
          <path
            class="pet-blob-shape"
            d="M256 108C356 108 428 164 428 272C428 388 381 428 256 428C131 428 84 388 84 272C84 164 156 108 256 108Z"
          />
        </clipPath>
        <rect id="pet-pill" x="-17" y="-46" width="34" height="92" rx="17" />
        <rect id="pet-pill-s" x="-17" y="-38" width="34" height="76" rx="17" />
        <path
          id="pet-zed"
          d="M0 0H14L0 14H14"
          fill="none"
          stroke="currentColor"
          stroke-width="4"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        <g id="pet-gear">
          <circle
            r="15"
            fill="none"
            stroke="currentColor"
            stroke-width="9"
            stroke-dasharray="5.9 5.9"
          />
          <circle r="13" fill="currentColor" />
          <circle r="5.5" class="gear-hole" />
        </g>
        <!-- 3D 转身光影带渐变 -->
        <linearGradient
          id="pet-sweep-grad"
          gradientUnits="userSpaceOnUse"
          x1="116"
          y1="0"
          x2="396"
          y2="0"
        >
          <stop offset="0" stop-color="#000" stop-opacity="0" />
          <stop offset=".5" stop-color="#000" stop-opacity=".15" />
          <stop offset="1" stop-color="#000" stop-opacity="0" />
        </linearGradient>
      </defs>

      <!-- 吉祥物身体与面部整体包裹层 -->
      <g class="pet-body-group">
        <!-- 身体外轮廓（直接渲染 path 以支持 Chromium CSS d: path() 关键帧流体变形） -->
        <path
          class="pet-body pet-blob-shape"
          d="M256 108C356 108 428 164 428 272C428 388 381 428 256 428C131 428 84 388 84 272C84 164 156 108 256 108Z"
        />

        <!-- 扳手趣味道具（无聊时掏出来抛接玩耍） -->
        <g v-if="props.autonomousAction === 'wrench'" class="pet-wrench-prop">
          <g class="wrench-bob">
            <rect class="wrench-obj" x="-14" y="-40" width="28" height="150" rx="14" />
            <circle class="wrench-obj" cx="0" cy="-62" r="34" />
            <path class="wrench-cutout" d="M-12 -104h24v34a12 12 0 0 1-24 0z" />
          </g>
        </g>

        <!-- 3D 转身光影掠过层（受身体 clip 约束） -->
        <g clip-path="url(#pet-blob-clip)">
          <rect
            class="pet-sweep"
            x="-300"
            y="0"
            width="1112"
            height="512"
            fill="url(#pet-sweep-grad)"
            opacity="0"
          />
        </g>

        <!-- 面部区域（眼睛，受身体 clip 约束） -->
        <g clip-path="url(#pet-blob-clip)">
          <!-- 3D 球面转身包裹层：实现正面转至背面、背面转回正面的前后翻转效果 -->
          <g class="pet-face-turn">
            <!-- 待机 / 图标 / 思考 / 执行 / 说话时的胶囊眼睛 -->
            <g class="pet-eyes-wrap" :style="eyesTransformStyle">
              <!-- 待机与图标眼睛 -->
              <g v-if="visualExpression === 'idle' || visualExpression === 'icon'" class="eye-pair">
                <g transform="translate(208 290)">
                  <use href="#pet-pill" class="pet-eye" :class="{ blink: blinking }" />
                </g>
                <g transform="translate(304 290)">
                  <use href="#pet-pill" class="pet-eye" :class="{ blink: blinking }" />
                </g>
              </g>

              <!-- 思考中眼睛（位置上移） -->
              <g v-else-if="visualExpression === 'thinking'" class="eye-pair eye-thinking">
                <g transform="translate(226 272)">
                  <use href="#pet-pill-s" class="pet-eye" :class="{ blink: blinking }" />
                </g>
                <g transform="translate(322 272)">
                  <use href="#pet-pill-s" class="pet-eye" :class="{ blink: blinking }" />
                </g>
              </g>

              <!-- 执行中眼睛（位置下移，带左右扫描） -->
              <g v-else-if="visualExpression === 'working'" class="eye-pair eye-working">
                <g class="eye-scan">
                  <g transform="translate(208 302)">
                    <use href="#pet-pill-s" class="pet-eye" />
                  </g>
                  <g transform="translate(304 302)">
                    <use href="#pet-pill-s" class="pet-eye" />
                  </g>
                </g>
              </g>

              <!-- 说话中眼睛（缩放脉动） -->
              <g v-else-if="visualExpression === 'talking'" class="eye-pair eye-talking">
                <g transform="translate(208 290)">
                  <use href="#pet-pill" class="pet-eye eye-talk-1" />
                </g>
                <g transform="translate(304 290)">
                  <use href="#pet-pill" class="pet-eye eye-talk-2" />
                </g>
              </g>

              <!-- 等待输入（好奇大圆眼） -->
              <g v-else-if="visualExpression === 'waiting'" class="eye-pair eye-waiting">
                <g transform="translate(208 290)">
                  <circle cx="0" cy="0" r="25" class="pet-round-eye" />
                  <circle cx="-9" cy="-10" r="7" class="pet-round-glint" />
                </g>
                <g transform="translate(304 290)">
                  <circle cx="0" cy="0" r="25" class="pet-round-eye" />
                  <circle cx="-9" cy="-10" r="7" class="pet-round-glint" />
                </g>
              </g>

              <!-- 完成（笑眯眯弯眼 + 腮红） -->
              <g v-else-if="visualExpression === 'done'" class="eye-pair eye-done">
                <!-- 粉嫩腮红 -->
                <circle cx="168" cy="326" r="15" fill="#ffb4c2" opacity="0.9" />
                <circle cx="344" cy="326" r="15" fill="#ffb4c2" opacity="0.9" />
                <!-- 弯弯笑眼 -->
                <path class="pet-smile-eye" d="M182 306Q208 240 234 306" />
                <path class="pet-smile-eye" d="M278 306Q304 240 330 306" />
              </g>

              <!-- 出错（> < 痛苦眼） -->
              <g v-else-if="visualExpression === 'error'" class="eye-pair eye-error">
                <g class="eye-shake">
                  <path class="pet-ouch-eye" d="M194 272L224 290L194 308" />
                  <path class="pet-ouch-eye" d="M318 272L288 290L318 308" />
                </g>
              </g>

              <!-- 休眠（半闭睡眼） -->
              <g v-else-if="visualExpression === 'sleeping'" class="eye-pair eye-sleeping">
                <g transform="translate(208 290)">
                  <path d="M-17 -4H17V26A17 17 0 0 1 -17 26Z" class="pet-sleep-lid" />
                </g>
                <g transform="translate(304 290)">
                  <path d="M-17 -4H17V26A17 17 0 0 1 -17 26Z" class="pet-sleep-lid" />
                </g>
              </g>
            </g>
          </g>
        </g>

        <!-- 外部漂浮动画元素（不受身体裁切，在头顶或身体右上方绽放） -->
        <g class="pet-outer-decorations">
          <!-- 连击欢快粒子：小金星与彩光 -->
          <g v-if="props.showComboDecor" class="decor-combo">
            <g transform="translate(416 100)" class="decor-pop">
              <path d="M0 -15 L4 -4 L15 0 L4 4 L0 15 L-4 4 L-15 0 L-4 -4 Z" fill="#ffd60a" />
              <circle cx="15" cy="-10" r="3.5" fill="#ff9f0a" />
              <circle cx="-13" cy="-8" r="2.5" fill="#ff375f" />
            </g>
          </g>

          <!-- 讨好交互：冒出粉红爱心弹跳动效 -->
          <g v-if="props.pleased" class="decor-pleased">
            <g transform="translate(416 112)" class="decor-pop">
              <path d="M0 4 C-10 -10 -22 2 -14 14 L0 27 L14 14 C22 2 10 -10 0 4 Z" fill="#ff5c8a" />
              <circle cx="15" cy="-8" r="4.5" fill="#ff85a2" />
            </g>
          </g>

          <!-- 思考中：思绪水泡 -->
          <g v-else-if="visualExpression === 'thinking'" class="decor-thinking" fill="currentColor">
            <circle class="decor-dot dot-1" cx="392" cy="150" r="7" />
            <circle class="decor-dot dot-2" cx="411" cy="121" r="10" />
            <circle class="decor-dot dot-3" cx="436" cy="86" r="14" />
          </g>

          <!-- 执行中：旋转齿轮 -->
          <g v-else-if="visualExpression === 'working'" class="decor-working">
            <g transform="translate(412 118) scale(1.35)" class="decor-gear">
              <use href="#pet-gear" />
            </g>
          </g>

          <!-- 说话中：右侧声音扩散波纹 -->
          <g v-else-if="visualExpression === 'talking'" class="decor-talking">
            <path class="decor-wave wave-1" d="M446 262a22 22 0 0 1 0 40" />
            <path class="decor-wave wave-2" d="M462 250a38 38 0 0 1 0 64" />
          </g>

          <!-- 等待输入：好奇浮动问号 -->
          <g v-else-if="visualExpression === 'waiting'" class="decor-waiting">
            <g class="decor-bob">
              <path
                d="M403 100a13 13 0 1 1 18 12c-4 2-5 4-5 9"
                fill="none"
                stroke="currentColor"
                stroke-width="6"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <circle cx="416" cy="132" r="3.5" fill="currentColor" />
            </g>
          </g>

          <!-- 完成：绿色对勾弹跳徽章 -->
          <g v-else-if="visualExpression === 'done'" class="decor-done">
            <g class="decor-pop">
              <circle cx="416" cy="112" r="22" fill="#34c759" />
              <path
                d="M406 112l7 7l13 -14"
                fill="none"
                stroke="#ffffff"
                stroke-width="6"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </g>
          </g>

          <!-- 出错：红色惊叹号弹跳徽章 -->
          <g v-else-if="visualExpression === 'error'" class="decor-error">
            <g class="decor-pop">
              <circle cx="416" cy="112" r="22" fill="#ff453a" />
              <rect x="413" y="99" width="6" height="16" rx="3" fill="#ffffff" />
              <circle cx="416" cy="122" r="3.5" fill="#ffffff" />
            </g>
          </g>

          <!-- 休眠：缓缓升起的 Z z 💤 梦幻睡眠泡泡 -->
          <g v-else-if="visualExpression === 'sleeping'" class="decor-sleeping">
            <g class="decor-zz zz-1">
              <use href="#pet-zed" transform="translate(386 128)" />
            </g>
            <g class="decor-zz zz-2">
              <use href="#pet-zed" transform="translate(386 128) scale(1.35)" />
            </g>
            <g class="decor-zz zz-3">
              <use href="#pet-zed" transform="translate(386 128) scale(1.7)" />
            </g>
          </g>
        </g>
      </g>
    </svg>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useEventListener } from "@vueuse/core";
import type { PetAction, TiltDirection } from "@/composables/useDesktopPetMotion";
import type { AutonomousAction } from "@/composables/useDesktopPetAutonomous";

export type MascotExpression =
  "icon" | "idle" | "thinking" | "working" | "talking" | "waiting" | "done" | "error" | "sleeping";

const props = withDefaults(
  defineProps<{
    expression?: MascotExpression;
    interactive?: boolean;
    followPointer?: boolean;
    pleased?: boolean;
    bouncing?: boolean;
    action?: PetAction;
    tiltDirection?: TiltDirection;
    showComboDecor?: boolean;
    isCombo?: boolean;
    autonomousAction?: AutonomousAction;
    autonomousGaze?: { x: number; y: number } | null;
  }>(),
  {
    expression: "idle",
    interactive: true,
    followPointer: true,
    pleased: false,
    bouncing: false,
    action: "idle",
    tiltDirection: "center",
    showComboDecor: false,
    isCombo: false,
    autonomousAction: "none",
    autonomousGaze: null,
  },
);

const emit = defineEmits<{ click: [event?: MouseEvent] }>();

const rootRef = ref<HTMLElement | null>(null);
const pointer = ref<{ x: number; y: number } | null>(null);
const gaze = ref({ x: 0, y: 0 });
const blinking = ref(false);
const internalBouncing = ref(false);

const activeAction = computed<PetAction>(() => {
  if (props.action && props.action !== "idle") return props.action;
  if (props.bouncing || internalBouncing.value) return "bounce";
  return "idle";
});

const visualExpression = computed<MascotExpression>(() => {
  if (props.pleased) return "done";
  if (activeAction.value === "curious") return "waiting";
  if (activeAction.value === "dizzy") return "error";
  if (props.autonomousAction === "stretch") return "sleeping";
  if (props.autonomousAction === "morph") return "waiting";
  return props.expression;
});

const isTilt = computed(() => {
  return visualExpression.value === "icon";
});

const eyesTransformStyle = computed(() => {
  if (visualExpression.value === "sleeping" || activeAction.value === "dizzy") {
    return { transform: "translate(0px, 0px)" };
  }
  if (props.autonomousGaze) {
    return {
      transform: `translate(${props.autonomousGaze.x.toFixed(1)}px, ${props.autonomousGaze.y.toFixed(1)}px)`,
    };
  }
  return {
    transform: `translate(${gaze.value.x.toFixed(1)}px, ${gaze.value.y.toFixed(1)}px)`,
  };
});

let blinkTimer: ReturnType<typeof setTimeout> | null = null;
let bounceTimer: ReturnType<typeof setTimeout> | null = null;
let gazeFrame = 0;
let bounceFrame = 0;

function scheduleBlink(delay = 2600 + Math.random() * 2600) {
  blinkTimer = setTimeout(() => {
    blinking.value = true;
    blinkTimer = setTimeout(() => {
      blinking.value = false;
      scheduleBlink();
    }, 130);
  }, delay);
}

function updateGaze() {
  if (!rootRef.value || !pointer.value || !props.followPointer) {
    gaze.value = { x: 0, y: 0 };
    return;
  }
  const rect = rootRef.value.getBoundingClientRect();
  const cx = rect.left + rect.width / 2;
  const cy = rect.top + rect.height / 2;
  const dx = pointer.value.x - cx;
  const dy = pointer.value.y - cy;
  const dist = Math.hypot(dx, dy);
  if (dist < 1) {
    gaze.value = { x: 0, y: 0 };
    return;
  }
  const k = dist / (dist + 120);
  gaze.value = {
    x: (dx / dist) * k * 26,
    y: (dy / dist) * k * 18,
  };
}

function scheduleGaze() {
  if (gazeFrame) return;
  gazeFrame = requestAnimationFrame(() => {
    gazeFrame = 0;
    updateGaze();
  });
}

useEventListener(window, "mousemove", (e: MouseEvent) => {
  pointer.value = { x: e.clientX, y: e.clientY };
  scheduleGaze();
});

useEventListener(window, "mouseleave", () => {
  pointer.value = null;
  scheduleGaze();
});

function triggerBounce() {
  internalBouncing.value = false;
  if (bounceFrame) cancelAnimationFrame(bounceFrame);
  if (bounceTimer) clearTimeout(bounceTimer);

  // 弹跳瞬间伴随自然眨眼/挤眼，更具生命质感
  blinking.value = true;
  setTimeout(() => {
    blinking.value = false;
  }, 130);

  bounceFrame = requestAnimationFrame(() => {
    bounceFrame = 0;
    internalBouncing.value = true;
    bounceTimer = setTimeout(() => {
      internalBouncing.value = false;
      bounceTimer = null;
    }, 540);
  });
}

function handleClick() {
  emit("click");
  if (!props.interactive) return;
  triggerBounce();
}

watch(
  () => props.bouncing,
  (val) => {
    if (val) {
      blinking.value = true;
      setTimeout(() => {
        blinking.value = false;
      }, 130);
    }
  },
);

onMounted(() => {
  scheduleBlink();
});

onBeforeUnmount(() => {
  if (blinkTimer) clearTimeout(blinkTimer);
  if (bounceTimer) clearTimeout(bounceTimer);
  if (gazeFrame) cancelAnimationFrame(gazeFrame);
  if (bounceFrame) cancelAnimationFrame(bounceFrame);
});
</script>

<style scoped>
.mascot-pet-view {
  display: block;
  width: 100%;
  height: 100%;
  position: relative;
  user-select: none;
  cursor: grab;
  transform-origin: 50% 90%;
  transition: transform 0.22s cubic-bezier(0.34, 1.4, 0.64, 1);
}

.mascot-pet-view:active {
  cursor: grabbing;
}

.mascot-pet-view.tilt .pet-body-group {
  transform-origin: 256px 268px;
  transform: rotate(10deg);
  transition: transform 0.4s cubic-bezier(0.34, 1.4, 0.64, 1);
}

.mascot-pet-view.pleased .pet-body-group {
  transform: rotate(-6deg);
}

.mascot-pet-view.tilt-left {
  transform-origin: 40% 90%;
}

.mascot-pet-view.tilt-right {
  transform-origin: 60% 90%;
}

/* ---- 3D 球面转身（前后转：从正面转到背面，再从背面转回正面） ---- */
.pet-face-turn {
  transform-box: fill-box;
  transform-origin: 256px 290px;
}

.pet-sweep {
  pointer-events: none;
}

/* 单次点击：根据点击偏向触发向右或向左 3D 球面前后转身 */
.mascot-pet-view.action-twirl .pet-face-turn,
.mascot-pet-view.action-spin .pet-face-turn {
  animation: mascot-turn-face-right 640ms linear both;
}

.mascot-pet-view.action-twirl.tilt-left .pet-face-turn,
.mascot-pet-view.action-spin.tilt-left .pet-face-turn {
  animation-name: mascot-turn-face-left;
}

.mascot-pet-view.action-twirl .pet-sweep,
.mascot-pet-view.action-spin .pet-sweep {
  animation: mascot-sweep-right 640ms linear both;
}

.mascot-pet-view.action-twirl.tilt-left .pet-sweep,
.mascot-pet-view.action-spin.tilt-left .pet-sweep {
  animation-name: mascot-sweep-left;
}

.mascot-pet-view.action-twirl .pet-body-group,
.mascot-pet-view.action-spin .pet-body-group {
  transform-origin: 256px 428px;
  animation: mascot-turn-body 640ms cubic-bezier(0.25, 0.9, 0.35, 1) both;
}

.mascot-pet-view.action-twirl .pet-outer-decorations,
.mascot-pet-view.action-spin .pet-outer-decorations {
  animation: mascot-turn-outer 640ms linear both;
}

/* 连击（Combo）：极速 720° 前后连转两周 */
.mascot-pet-view.action-twirl.is-combo .pet-face-turn,
.mascot-pet-view.action-spin.is-combo .pet-face-turn {
  animation: mascot-turn-face-double-right 780ms linear both;
}

.mascot-pet-view.action-twirl.is-combo.tilt-left .pet-face-turn,
.mascot-pet-view.action-spin.is-combo.tilt-left .pet-face-turn {
  animation-name: mascot-turn-face-double-left;
}

.mascot-pet-view.action-twirl.is-combo .pet-sweep,
.mascot-pet-view.action-spin.is-combo .pet-sweep {
  animation: mascot-sweep-double-right 780ms linear both;
}

.mascot-pet-view.action-twirl.is-combo.tilt-left .pet-sweep,
.mascot-pet-view.action-spin.is-combo.tilt-left .pet-sweep {
  animation-name: mascot-sweep-double-left;
}

.mascot-pet-view.action-twirl.is-combo .pet-body-group,
.mascot-pet-view.action-spin.is-combo .pet-body-group {
  transform-origin: 256px 428px;
  animation: mascot-turn-body-double 780ms cubic-bezier(0.2, 0.9, 0.3, 1) both;
}

.mascot-pet-view.action-bounce,
.mascot-pet-view.bouncing {
  transform-origin: 50% 90%;
  animation: mascot-super-bounce 600ms cubic-bezier(0.22, 1, 0.36, 1);
}

.mascot-pet-view.action-jiggle {
  transform-origin: 50% 85%;
  animation: mascot-intense-jiggle 560ms cubic-bezier(0.36, 0.07, 0.19, 0.97);
}

.mascot-pet-view.action-curious {
  transform-origin: 30% 90%;
  animation: mascot-curious-peek 650ms cubic-bezier(0.34, 1.56, 0.64, 1);
}

.mascot-pet-view.action-double-hop {
  transform-origin: 50% 90%;
  animation: mascot-zigzag-hop 620ms cubic-bezier(0.28, 0.84, 0.42, 1);
}

.mascot-pet-view.action-dizzy {
  transform-origin: 50% 90%;
  animation: mascot-dizzy-stumble 750ms ease-in-out;
}

.mascot-pet-svg {
  display: block;
  width: 100%;
  height: 100%;
  overflow: visible;
}

/* 身体与眼睛配色 */
.pet-body {
  fill: var(--peek-text, #242424);
}

:global(html[data-theme="dark"]) .pet-body {
  fill: var(--peek-text, #e8e8e8);
}

.gear-hole {
  fill: var(--peek-text, #242424);
}

:global(html[data-theme="dark"]) .gear-hole {
  fill: var(--peek-text, #181818);
}

.pet-eye {
  fill: var(--peek-bg, #ffffff);
  transform-box: fill-box;
  transform-origin: center;
  transition: transform 70ms ease-out;
}

.pet-eye.blink {
  transform: scaleY(0.08);
}

.pet-round-eye {
  fill: var(--peek-bg, #ffffff);
}

.pet-round-glint {
  fill: var(--peek-text, #242424);
}

.pet-smile-eye {
  fill: none;
  stroke: var(--peek-bg, #ffffff);
  stroke-width: 14;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.pet-ouch-eye {
  fill: none;
  stroke: var(--peek-bg, #ffffff);
  stroke-width: 13;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.pet-sleep-lid {
  fill: var(--peek-bg, #ffffff);
}

.pet-eyes-wrap {
  transition: transform 160ms cubic-bezier(0.2, 0.7, 0.3, 1);
  will-change: transform;
}

/* 扫描动效 */
.eye-scan {
  animation: pet-scan 1.6s steps(1) infinite;
}

@keyframes pet-scan {
  0% {
    transform: translateX(0);
  }
  25% {
    transform: translateX(10px);
  }
  50% {
    transform: translateX(-8px);
  }
  75% {
    transform: translateX(4px);
  }
}

/* 说话脉动 */
.eye-talk-1 {
  animation: pet-talk 0.5s ease-in-out infinite;
}

.eye-talk-2 {
  animation: pet-talk 0.5s ease-in-out infinite;
  animation-delay: -0.25s;
}

@keyframes pet-talk {
  0%,
  100% {
    transform: scaleY(1);
  }
  35% {
    transform: scaleY(0.6);
  }
  65% {
    transform: scaleY(1.08);
  }
}

/* 错误微颤 */
.eye-shake {
  animation: pet-shake 0.12s linear infinite alternate;
}

@keyframes pet-shake {
  from {
    transform: translateX(-2.5px);
  }
  to {
    transform: translateX(2.5px);
  }
}

/* 外部装饰物样式与动画 */
.pet-outer-decorations {
  color: var(--peek-text, #242424);
}

:global(html[data-theme="dark"]) .pet-outer-decorations {
  color: #ffffff;
}

/* 思考思绪泡泡 */
.decor-dot {
  animation: pet-dot 1.3s ease-in-out infinite;
}
.dot-2 {
  animation-delay: 0.25s;
}
.dot-3 {
  animation-delay: 0.5s;
}

@keyframes pet-dot {
  0%,
  100% {
    opacity: 0.3;
    transform: scale(0.9);
  }
  50% {
    opacity: 1;
    transform: scale(1.1);
  }
}

/* 执行旋转齿轮 */
.decor-gear {
  transform-origin: 412px 118px;
  animation: pet-rotate 2.4s linear infinite;
}

@keyframes pet-rotate {
  from {
    transform: translate(412px, 118px) scale(1.35) rotate(0deg);
  }
  to {
    transform: translate(412px, 118px) scale(1.35) rotate(360deg);
  }
}

/* 声波弧线 */
.decor-wave {
  fill: none;
  stroke: currentColor;
  stroke-width: 5.5;
  stroke-linecap: round;
  animation: pet-wave 1.2s ease-out infinite;
}
.wave-2 {
  animation-delay: 0.4s;
}

@keyframes pet-wave {
  0% {
    opacity: 0;
    transform: translateX(-5px);
  }
  35% {
    opacity: 1;
  }
  100% {
    opacity: 0;
    transform: translateX(7px);
  }
}

/* 问号浮动 */
.decor-bob {
  animation: pet-bob 1.2s ease-in-out infinite alternate;
}

@keyframes pet-bob {
  from {
    transform: translateY(-3px) rotate(-4deg);
  }
  to {
    transform: translateY(4px) rotate(5deg);
  }
}

/* 徽章弹出效果 */
.decor-pop {
  transform-box: fill-box;
  transform-origin: center;
  animation: pet-pop 0.5s cubic-bezier(0.34, 1.56, 0.64, 1);
}

@keyframes pet-pop {
  0% {
    transform: scale(0.2);
    opacity: 0;
  }
  70% {
    transform: scale(1.15);
  }
  100% {
    transform: scale(1);
    opacity: 1;
  }
}

/* 睡眠 Z z 💤 */
.decor-zz {
  transform-box: fill-box;
  transform-origin: center;
  animation: pet-zz 2.4s ease-out infinite;
}
.zz-2 {
  animation-delay: 0.8s;
}
.zz-3 {
  animation-delay: 1.6s;
}

@keyframes pet-zz {
  0% {
    opacity: 0;
    transform: translate(0, 0) scale(0.6);
  }
  25% {
    opacity: 1;
  }
  100% {
    opacity: 0;
    transform: translate(22px, -45px) scale(1.3);
  }
}

@keyframes mascot-turn-body {
  0% {
    transform: translateY(0) scale(1, 1);
  }
  14% {
    /* 蓄力起跳下蹲 */
    transform: translateY(8px) scale(1.16, 0.85);
  }
  42% {
    /* 腾空跳起，身体转至背面 */
    transform: translateY(-24px) scale(0.93, 1.08);
  }
  74% {
    /* 落地缓冲压扁 */
    transform: translateY(6px) scale(1.14, 0.88);
  }
  88% {
    /* 微弹平复 */
    transform: translateY(-3px) scale(0.98, 1.02);
  }
  100% {
    transform: translateY(0) scale(1, 1);
  }
}

@keyframes mascot-turn-body-double {
  0% {
    transform: translateY(0) scale(1, 1);
  }
  10% {
    transform: translateY(8px) scale(1.18, 0.84);
  }
  28% {
    /* 第一跳最高点 */
    transform: translateY(-22px) scale(0.92, 1.1);
  }
  48% {
    /* 中间弹力触地蓄力 */
    transform: translateY(4px) scale(1.12, 0.9);
  }
  68% {
    /* 第二跳更高腾空 */
    transform: translateY(-28px) scale(0.9, 1.12);
  }
  86% {
    /* 落地缓冲 */
    transform: translateY(6px) scale(1.16, 0.86);
  }
  100% {
    transform: translateY(0) scale(1, 1);
  }
}

/* 球面采样：向右转身，转到背面（scaleX->0）再从左侧浮现 */
@keyframes mascot-turn-face-right {
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

/* 球面采样：向左转身，转到背面（scaleX->0）再从右侧浮现 */
@keyframes mascot-turn-face-left {
  0% {
    transform: translateX(0) scaleX(1);
    opacity: 1;
  }
  5.6% {
    transform: translateX(-39px) scaleX(0.966);
  }
  11.1% {
    transform: translateX(-75px) scaleX(0.866);
  }
  16.7% {
    transform: translateX(-106px) scaleX(0.707);
    opacity: 1;
  }
  22.2% {
    transform: translateX(-130px) scaleX(0.5);
    opacity: 0.55;
  }
  27.8% {
    transform: translateX(-145px) scaleX(0.259);
    opacity: 0;
  }
  33.3% {
    transform: translateX(-150px) scaleX(0);
    opacity: 0;
  }
  66.7% {
    transform: translateX(150px) scaleX(0);
    opacity: 0;
  }
  72.2% {
    transform: translateX(145px) scaleX(0.259);
    opacity: 0;
  }
  77.8% {
    transform: translateX(130px) scaleX(0.5);
    opacity: 0.55;
  }
  83.3% {
    transform: translateX(106px) scaleX(0.707);
    opacity: 1;
  }
  88.9% {
    transform: translateX(75px) scaleX(0.866);
  }
  94.4% {
    transform: translateX(39px) scaleX(0.966);
  }
  100% {
    transform: translateX(0) scaleX(1);
    opacity: 1;
  }
}

/* 连击两周极速前后连转：向右 */
@keyframes mascot-turn-face-double-right {
  0%,
  50%,
  100% {
    transform: translateX(0) scaleX(1);
    opacity: 1;
  }
  5.5% {
    transform: translateX(75px) scaleX(0.866);
    opacity: 1;
  }
  11% {
    transform: translateX(130px) scaleX(0.5);
    opacity: 0.55;
  }
  16.7% {
    transform: translateX(150px) scaleX(0);
    opacity: 0;
  }
  33.3% {
    transform: translateX(-150px) scaleX(0);
    opacity: 0;
  }
  39% {
    transform: translateX(-130px) scaleX(0.5);
    opacity: 0.55;
  }
  44.5% {
    transform: translateX(-75px) scaleX(0.866);
    opacity: 1;
  }
  55.5% {
    transform: translateX(75px) scaleX(0.866);
    opacity: 1;
  }
  61% {
    transform: translateX(130px) scaleX(0.5);
    opacity: 0.55;
  }
  66.7% {
    transform: translateX(150px) scaleX(0);
    opacity: 0;
  }
  83.3% {
    transform: translateX(-150px) scaleX(0);
    opacity: 0;
  }
  89% {
    transform: translateX(-130px) scaleX(0.5);
    opacity: 0.55;
  }
  94.5% {
    transform: translateX(-75px) scaleX(0.866);
    opacity: 1;
  }
}

/* 连击两周极速前后连转：向左 */
@keyframes mascot-turn-face-double-left {
  0%,
  50%,
  100% {
    transform: translateX(0) scaleX(1);
    opacity: 1;
  }
  5.5% {
    transform: translateX(-75px) scaleX(0.866);
    opacity: 1;
  }
  11% {
    transform: translateX(-130px) scaleX(0.5);
    opacity: 0.55;
  }
  16.7% {
    transform: translateX(-150px) scaleX(0);
    opacity: 0;
  }
  33.3% {
    transform: translateX(150px) scaleX(0);
    opacity: 0;
  }
  39% {
    transform: translateX(130px) scaleX(0.5);
    opacity: 0.55;
  }
  44.5% {
    transform: translateX(75px) scaleX(0.866);
    opacity: 1;
  }
  55.5% {
    transform: translateX(-75px) scaleX(0.866);
    opacity: 1;
  }
  61% {
    transform: translateX(-130px) scaleX(0.5);
    opacity: 0.55;
  }
  66.7% {
    transform: translateX(-150px) scaleX(0);
    opacity: 0;
  }
  83.3% {
    transform: translateX(150px) scaleX(0);
    opacity: 0;
  }
  89% {
    transform: translateX(130px) scaleX(0.5);
    opacity: 0.55;
  }
  94.5% {
    transform: translateX(75px) scaleX(0.866);
    opacity: 1;
  }
}

/* 3D 转身阴影扫过动效 */
@keyframes mascot-sweep-right {
  0% {
    transform: translateX(-340px);
    opacity: 0;
  }
  15%,
  85% {
    opacity: 1;
  }
  100% {
    transform: translateX(340px);
    opacity: 0;
  }
}

@keyframes mascot-sweep-left {
  0% {
    transform: translateX(340px);
    opacity: 0;
  }
  15%,
  85% {
    opacity: 1;
  }
  100% {
    transform: translateX(-340px);
    opacity: 0;
  }
}

@keyframes mascot-sweep-double-right {
  0% {
    transform: translateX(-340px);
    opacity: 0;
  }
  8%,
  42% {
    opacity: 1;
  }
  50% {
    transform: translateX(340px);
    opacity: 0;
  }
  51% {
    transform: translateX(-340px);
    opacity: 0;
  }
  58%,
  92% {
    opacity: 1;
  }
  100% {
    transform: translateX(340px);
    opacity: 0;
  }
}

@keyframes mascot-sweep-double-left {
  0% {
    transform: translateX(340px);
    opacity: 0;
  }
  8%,
  42% {
    opacity: 1;
  }
  50% {
    transform: translateX(-340px);
    opacity: 0;
  }
  51% {
    transform: translateX(340px);
    opacity: 0;
  }
  58%,
  92% {
    opacity: 1;
  }
  100% {
    transform: translateX(-340px);
    opacity: 0;
  }
}

@keyframes mascot-turn-outer {
  0%,
  100% {
    opacity: 1;
  }
  20%,
  80% {
    opacity: 0;
  }
}

@keyframes mascot-super-bounce {
  0% {
    transform: scale(1, 1) translateY(0);
  }
  12% {
    /* 极限深蹲蓄力压扁 */
    transform: scale(1.34, 0.66) translateY(12px);
  }
  32% {
    /* 火箭冲天大蹦高！大幅腾空，细长面条拉伸 */
    transform: scale(0.72, 1.38) translateY(-36px);
  }
  52% {
    /* 触地深度果冻二次缓冲 */
    transform: scale(1.22, 0.82) translateY(6px);
  }
  72% {
    /* 二次微跳 */
    transform: scale(0.92, 1.08) translateY(-8px);
  }
  88% {
    /* 触地轻微修正 */
    transform: scale(1.03, 0.98) translateY(1px);
  }
  100% {
    transform: scale(1, 1) translateY(0);
  }
}

@keyframes mascot-intense-jiggle {
  0% {
    transform: scale(1, 1) rotate(0deg);
  }
  14% {
    transform: scale(1.24, 0.82) rotate(-22deg) translateX(-10px);
  }
  28% {
    transform: scale(0.86, 1.18) rotate(20deg) translateX(10px);
  }
  44% {
    transform: scale(1.16, 0.88) rotate(-14deg) translateX(-6px);
  }
  60% {
    transform: scale(0.92, 1.08) rotate(10deg) translateX(4px);
  }
  76% {
    transform: scale(1.06, 0.96) rotate(-5deg) translateX(-2px);
  }
  90% {
    transform: scale(0.98, 1.02) rotate(2deg);
  }
  100% {
    transform: scale(1, 1) rotate(0deg);
  }
}

@keyframes mascot-curious-peek {
  0% {
    transform: scale(1, 1) rotate(0deg) translate(0, 0);
  }
  25% {
    /* 大幅度侧歪头 26°，身体探出 */
    transform: scale(1.08, 0.96) rotate(26deg) translate(16px, -18px);
  }
  65% {
    /* 探头定格凝视打量 */
    transform: scale(1.06, 0.98) rotate(24deg) translate(14px, -16px);
  }
  85% {
    /* 开始收回 */
    transform: scale(0.96, 1.04) rotate(-4deg) translate(-2px, 2px);
  }
  100% {
    transform: scale(1, 1) rotate(0deg) translate(0, 0);
  }
}

@keyframes mascot-zigzag-hop {
  0% {
    transform: scale(1, 1) translate(0, 0) rotate(0deg);
  }
  14% {
    transform: scale(1.18, 0.84) translateY(4px);
  }
  30% {
    /* 第一跳向左微倾 */
    transform: scale(0.86, 1.2) translate(-14px, -18px) rotate(-8deg);
  }
  46% {
    /* 第一跳触地缓冲 */
    transform: scale(1.14, 0.88) translate(-6px, 2px) rotate(-3deg);
  }
  64% {
    /* 第二跳爆发向右大幅拉高！ */
    transform: scale(0.82, 1.25) translate(14px, -28px) rotate(10deg);
  }
  82% {
    /* 落地归位缓冲 */
    transform: scale(1.16, 0.88) translate(0, 4px) rotate(0deg);
  }
  92% {
    transform: scale(0.96, 1.03) translateY(-3px);
  }
  100% {
    transform: scale(1, 1) translate(0, 0) rotate(0deg);
  }
}

@keyframes mascot-dizzy-stumble {
  0% {
    transform: scale(1, 1) rotate(0deg);
  }
  12% {
    transform: scale(1.12, 0.88) rotate(-26deg) translateX(-12px);
  }
  28% {
    transform: scale(0.92, 1.12) rotate(26deg) translateX(12px);
  }
  44% {
    transform: scale(1.1, 0.92) rotate(-18deg) translateX(-8px);
  }
  60% {
    transform: scale(0.94, 1.08) rotate(16deg) translateX(7px);
  }
  76% {
    transform: scale(1.05, 0.96) rotate(-8deg) translateX(-3px);
  }
  88% {
    transform: scale(0.98, 1.02) rotate(4deg);
  }
  100% {
    transform: scale(1, 1) rotate(0deg);
  }
}

/* ---- 自主趣味微动作（无聊时触发） ---- */

/* 1. 拿扳手玩耍 */
.pet-wrench-prop {
  transform-origin: 256px 268px;
  pointer-events: none;
}
.wrench-bob {
  transform-origin: 0px 0px;
  animation: mascot-wrench-toss 2400ms cubic-bezier(0.25, 1, 0.5, 1) both;
}
.wrench-obj {
  fill: var(--peek-text, #242424);
  stroke: var(--peek-bg, #ffffff);
  stroke-width: 8;
  stroke-linejoin: round;
  stroke-linecap: round;
  paint-order: stroke;
}
.wrench-cutout {
  fill: var(--peek-bg, #ffffff);
}
:global(html[data-theme="dark"]) .wrench-obj {
  fill: var(--peek-text, #e8e8e8);
  stroke: var(--peek-bg, #181818);
}
:global(html[data-theme="dark"]) .wrench-cutout {
  fill: var(--peek-bg, #181818);
}

.mascot-pet-view.auto-wrench .pet-eyes-wrap {
  animation: mascot-watch-wrench 2400ms ease-in-out both;
}

@keyframes mascot-wrench-toss {
  0% {
    transform: translate(430px, 380px) rotate(45deg) scale(0.3);
    opacity: 0;
  }
  12% {
    /* 掏出扳手，握在手中 */
    transform: translate(410px, 310px) rotate(20deg) scale(1);
    opacity: 1;
  }
  24% {
    /* 下压蓄力准备抛起 */
    transform: translate(414px, 326px) rotate(10deg) scale(0.96);
  }
  42% {
    /* 抛向空中！在头顶上方高高翻转 */
    transform: translate(360px, 90px) rotate(-220deg) scale(1.08);
  }
  56% {
    /* 达到最高点翻转并开始下落 */
    transform: translate(350px, 75px) rotate(-360deg) scale(1.1);
  }
  68% {
    /* 接住扳手！ */
    transform: translate(405px, 295px) rotate(-340deg) scale(1);
    opacity: 1;
  }
  78% {
    /* 开心地向上挥舞一下 */
    transform: translate(418px, 275px) rotate(-305deg) scale(1.05);
  }
  88% {
    /* 挥舞第二下 */
    transform: translate(408px, 298px) rotate(-335deg) scale(1);
    opacity: 1;
  }
  100% {
    /* 乖巧收回身后 */
    transform: translate(430px, 390px) rotate(-315deg) scale(0.2);
    opacity: 0;
  }
}

@keyframes mascot-watch-wrench {
  0%,
  100% {
    transform: translate(0, 0);
  }
  12%,
  24% {
    transform: translate(16px, 12px);
  }
  40%,
  58% {
    /* 视线崇拜注视着空中抛转的扳手！ */
    transform: translate(14px, -18px);
  }
  68%,
  88% {
    transform: translate(16px, 4px);
  }
}

/* 2. 果冻多面体流体变形（正方形、三角形、多边形再弹回） */
.mascot-pet-view.auto-morph .pet-blob-shape {
  animation: mascot-morph-shapes 2600ms cubic-bezier(0.34, 1.25, 0.64, 1) both;
}

.mascot-pet-view.auto-morph .pet-body-group {
  transform-origin: 256px 270px;
  animation: mascot-morph-squash 2600ms cubic-bezier(0.34, 1.25, 0.64, 1) both;
}

@keyframes mascot-morph-squash {
  0% {
    transform: scale(1, 1);
  }
  18% {
    /* 噗！蓄力变成方块 */
    transform: scale(1.16, 0.88);
  }
  28%,
  38% {
    /* 方形微晃定格 */
    transform: scale(0.96, 1.04) rotate(-3deg);
  }
  48%,
  58% {
    /* 三角形拉高探出 */
    transform: scale(0.88, 1.18) rotate(4deg);
  }
  70%,
  80% {
    /* 六边多边形微转定格 */
    transform: scale(1.15, 0.92) rotate(-8deg);
  }
  88% {
    /* 深蹲落地蓄力 */
    transform: scale(1.22, 0.78) rotate(0deg);
  }
  95% {
    /* 弹回微冲 */
    transform: scale(0.95, 1.05);
  }
  100% {
    transform: scale(1, 1);
  }
}

@keyframes mascot-morph-shapes {
  0% {
    d: path(
      "M256 108C356 108 428 164 428 272C428 388 381 428 256 428C131 428 84 388 84 272C84 164 156 108 256 108Z"
    );
  }
  24%,
  38% {
    /* 几何圆角正方形 (Square) */
    d: path(
      "M256 116C380 116 396 132 396 266C396 400 380 416 256 416C132 416 116 400 116 266C116 132 132 116 256 116Z"
    );
  }
  48%,
  62% {
    /* 几何三角体 (Triangle) */
    d: path(
      "M256 96C281 150 306 204 331 258C381 366 360 420 256 420C152 420 131 366 181 258C206 204 231 150 256 96Z"
    );
  }
  72%,
  84% {
    /* 几何多面多边形 (Diamond / Polygon) */
    d: path(
      "M256 90C312 149 368 209 424 268C368 324 312 380 256 436C200 380 144 324 88 268C144 209 200 149 256 90Z"
    );
  }
  92% {
    /* 超弹性落地回弹 */
    d: path(
      "M256 102C362 102 434 160 434 270C434 392 384 434 256 434C128 434 78 392 78 270C78 160 150 102 256 102Z"
    );
  }
  100% {
    /* 恢复圆润身体 */
    d: path(
      "M256 108C356 108 428 164 428 272C428 388 381 428 256 428C131 428 84 388 84 272C84 164 156 108 256 108Z"
    );
  }
}

/* 3. 左顾右盼张望 */
.mascot-pet-view.auto-look-around .pet-body-group {
  transform-origin: 256px 428px;
  animation: mascot-look-around-body 2400ms ease-in-out both;
}

@keyframes mascot-look-around-body {
  0%,
  100% {
    transform: rotate(0deg);
  }
  12%,
  38% {
    /* 身体微歪向左探头 */
    transform: rotate(-9deg) translateX(-5px);
  }
  48%,
  76% {
    /* 身体微歪向右张望 */
    transform: rotate(10deg) translateX(5px);
  }
  88% {
    transform: rotate(-2deg);
  }
}

/* 4. 伸懒腰打哈欠 */
.mascot-pet-view.auto-stretch .pet-body-group {
  transform-origin: 256px 428px;
  animation: mascot-stretch-body 1800ms ease-in-out both;
}

@keyframes mascot-stretch-body {
  0%,
  100% {
    transform: scale(1, 1) translateY(0);
  }
  20% {
    transform: scale(1.12, 0.9) translateY(4px);
  }
  45%,
  65% {
    /* 舒服地向上拉长身体伸懒腰 */
    transform: scale(0.88, 1.22) translateY(-18px);
  }
  82% {
    transform: scale(1.08, 0.94) translateY(3px);
  }
}
</style>
