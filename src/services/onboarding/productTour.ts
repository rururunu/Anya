import { driver, type DriveStep, type Driver } from "driver.js";
import "driver.js/dist/driver.css";
import "@/styles/product-tour.css";

import { tr } from "@/services/i18n";
import type { AppLanguage } from "@/types/setting";

import {
  planProductTour,
  selectProductTourSteps,
  type ProductTourPlanStep,
  type ProductTourStepId,
} from "./productTourPlan";

const requests = new Set<() => void>();

let activeTour: Driver | null = null;
let suppressFinish = false;

/** 从设置里再次打开新手引导。 */
export function requestProductTour() {
  for (const listener of requests) listener();
}

/** 订阅「再看一次引导」。返回取消订阅。 */
export function onProductTourRequested(listener: () => void) {
  requests.add(listener);
  return () => {
    requests.delete(listener);
  };
}

/** 用户完成、跳过或自己发出第一条消息时结束引导。 */
export function finishProductTour() {
  if (!activeTour?.isActive()) return;
  activeTour.destroy();
}

/** 离开工作台时拆掉浮层，不记成已完成。 */
export function destroyProductTour() {
  if (!activeTour) return;
  suppressFinish = true;
  activeTour.destroy();
  activeTour = null;
  suppressFinish = false;
}

/** 密钥保存后，从「填写密钥」步进入下一步。 */
export function advanceProductTourAfterProvider() {
  if (!activeTour?.isActive()) return;
  if (activeTour.getActiveStep()?.data?.id !== "provider") return;
  if (activeTour.hasNextStep()) activeTour.moveNext();
}

/** 在当前工作台上启动 driver.js 引导。 */
export function startProductTour(options: {
  hasProvider: boolean;
  language: AppLanguage;
  onFinished: () => void;
}) {
  destroyProductTour();
  const copy = tourCopy(options.language);
  const steps = selectProductTourSteps(planProductTour(options.hasProvider), isVisible).map(
    (step) => toDriveStep(step, copy),
  );
  if (steps.length === 0) {
    options.onFinished();
    return;
  }
  const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const tour = driver({
    animate: !reduceMotion,
    smoothScroll: !reduceMotion,
    overlayColor: "#121212",
    overlayOpacity: 0.62,
    stagePadding: 8,
    stageRadius: 14,
    allowClose: true,
    allowKeyboardControl: true,
    overlayClickBehavior: "close",
    showButtons: ["next", "previous", "close"],
    showProgress: true,
    progressText: "{{current}} / {{total}}",
    nextBtnText: copy.next,
    prevBtnText: copy.back,
    doneBtnText: copy.done,
    closeBtnLabel: copy.close,
    popoverClass: "anya-product-tour",
    steps,
    skipMissingElement: true,
    onElementMissing: "stay",
    onDestroyed: () => {
      if (activeTour === tour) activeTour = null;
      if (!suppressFinish) options.onFinished();
    },
  });
  activeTour = tour;
  tour.drive();
}

function isVisible(selector: string) {
  const element = document.querySelector(selector);
  if (!(element instanceof HTMLElement)) return false;
  const rect = element.getBoundingClientRect();
  return rect.width > 0 && rect.height > 0;
}

function toDriveStep(step: ProductTourPlanStep, copy: ReturnType<typeof tourCopy>): DriveStep {
  const text = copy.steps[step.id];
  return {
    element: step.element,
    disableActiveInteraction: step.allowInteraction ? false : undefined,
    data: { id: step.id },
    popover: {
      title: text.title,
      description: text.description,
      side: step.side,
      align: step.align,
    },
  };
}

function tourCopy(language: AppLanguage) {
  const text = (id: ProductTourStepId) => ({
    title: tr(language, `onboarding.tour.${id}.title`),
    description: tr(language, `onboarding.tour.${id}.body`),
  });
  return {
    next: tr(language, "onboarding.tour.next"),
    back: tr(language, "onboarding.tour.back"),
    done: tr(language, "onboarding.tour.done"),
    close: tr(language, "onboarding.tour.close"),
    steps: {
      welcome: text("welcome"),
      provider: text("provider"),
      composer: text("composer"),
      hotkey: text("hotkey"),
      workspace: text("workspace"),
    },
  };
}
