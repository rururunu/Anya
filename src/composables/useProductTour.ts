import { nextTick, onMounted, onUnmounted, watch, type ComputedRef, type Ref } from "vue";

import {
  advanceProductTourAfterProvider,
  destroyProductTour,
  onProductTourRequested,
  startProductTour,
} from "@/services/onboarding/productTour";
import { useSettingStore } from "@/stores/setting";

/** 首次进入工作台时启动引导，并响应「再看一次」。 */
export function useProductTour(options: {
  blocked: Ref<boolean>;
  hasProvider: ComputedRef<boolean>;
  prepare: () => void;
}) {
  const settingStore = useSettingStore();
  let stopRequest: (() => void) | null = null;
  let autoStarted = false;

  async function openTour() {
    options.prepare();
    await nextTick();
    await waitFrames(2);
    startProductTour({
      hasProvider: options.hasProvider.value,
      language: settingStore.language,
      onFinished: () => {
        if (!settingStore.onboardingCompleted) {
          void settingStore.update({ onboardingCompleted: true });
        }
      },
    });
  }

  function startIfNeeded() {
    if (autoStarted || options.blocked.value || settingStore.onboardingCompleted) return;
    autoStarted = true;
    void openTour();
  }

  watch(
    () => options.blocked.value,
    () => startIfNeeded(),
    { immediate: true },
  );

  watch(options.hasProvider, (ready, previous) => {
    if (ready && previous === false) advanceProductTourAfterProvider();
  });

  onMounted(() => {
    stopRequest = onProductTourRequested(() => {
      void openTour();
    });
  });

  onUnmounted(() => {
    stopRequest?.();
    stopRequest = null;
    destroyProductTour();
  });
}

function waitFrames(count: number) {
  return new Promise<void>((resolve) => {
    const step = (left: number) => {
      if (left <= 0) {
        resolve();
        return;
      }
      requestAnimationFrame(() => step(left - 1));
    };
    step(count);
  });
}
