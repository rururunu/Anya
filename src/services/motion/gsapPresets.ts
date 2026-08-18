import { clearGsapProps, gsap, safeGsap } from "./gsapSafe";

function prefersReducedMotion() {
  return (
    typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

const COMPOSER_ENTER = 0.14;
const COMPOSER_LEAVE = 0.09;
const OVERLAY_ENTER = 0.17;
const OVERLAY_LEAVE = 0.1;

/** Vue Transition `done` must always fire — otherwise mode="out-in" freezes forever. */
function onceDone(done: () => void): () => void {
  let finished = false;
  return () => {
    if (finished) return;
    finished = true;
    done();
  };
}

/**
 * Scroll a specific overflow container to a child element.
 * Prefer this over element.scrollIntoView — that can scroll the wrong
 * ancestor (window / overlay) in Peek's nested scroll layout.
 */
export function gsapScrollContainerTo(
  container: HTMLElement,
  target: HTMLElement,
  opts?: { offsetY?: number; duration?: number },
) {
  const offsetY = opts?.offsetY ?? 12;
  const snap = () => {
    const next =
      container.scrollTop +
      (target.getBoundingClientRect().top - container.getBoundingClientRect().top) -
      offsetY;
    container.scrollTop = Math.max(0, next);
  };

  safeGsap(
    "scrollContainerTo",
    () => {
      gsap.killTweensOf(container);
      if (prefersReducedMotion()) {
        snap();
        return;
      }
      gsap.to(container, {
        duration: opts?.duration ?? 0.22,
        scrollTo: { y: target, offsetY },
      });
    },
    snap,
  );
}

/**
 * Ask / approval / permission picker panel.
 * Do NOT tween container height — Overlay resizes the window for that.
 * Height tweens here fight the window resize and flash the message panel.
 *
 * Composer floating menus: autoAlpha + x only, never drive window resize.
 */
export function gsapPickerEnter(el: Element, done: () => void) {
  const target = el as HTMLElement;
  const finish = onceDone(done);

  safeGsap(
    "pickerEnter",
    () => {
      if (prefersReducedMotion()) {
        clearGsapProps(target);
        finish();
        return;
      }

      const items = target.querySelectorAll<HTMLElement>(
        ".command-item, .workspace-option, .workspace-new-row, .attach-chip, .attach-tab",
      );
      gsap.killTweensOf([target, ...items]);

      // Safety: if a tween is killed/interrupted mid-flight, still unlock Transition.
      const safety = gsap.delayedCall(Math.max(COMPOSER_ENTER, 0.11) + 0.35, finish);

      const tl = gsap.timeline({
        onComplete: () => {
          safety.kill();
          finish();
        },
      });
      tl.fromTo(target, { autoAlpha: 0 }, { autoAlpha: 1, duration: COMPOSER_ENTER });

      if (items.length) {
        tl.fromTo(
          items,
          { autoAlpha: 0, x: -4 },
          {
            autoAlpha: 1,
            x: 0,
            duration: 0.11,
            stagger: 0.012,
            clearProps: "transform",
          },
          0.04,
        );
      }
    },
    () => {
      clearGsapProps(target);
      target.style.opacity = "1";
      target.style.visibility = "visible";
      finish();
    },
  );
}

export function gsapPickerLeave(el: Element, done: () => void) {
  const target = el as HTMLElement;
  const finish = onceDone(done);

  safeGsap(
    "pickerLeave",
    () => {
      if (target.matches(".model-picker-list, .option-picker-list")) {
        const items = target.querySelectorAll<HTMLElement>(".command-item");
        gsap.killTweensOf([target, ...items]);
        finish();
        return;
      }
      if (prefersReducedMotion()) {
        finish();
        return;
      }

      gsap.killTweensOf(target);
      const safety = gsap.delayedCall(COMPOSER_LEAVE + 0.35, finish);
      gsap.to(target, {
        autoAlpha: 0,
        duration: COMPOSER_LEAVE,
        ease: "power2.in",
        onComplete: () => {
          safety.kill();
          finish();
        },
      });
    },
    () => {
      target.style.opacity = "0";
      finish();
    },
  );
}

/** Floating model / approval mode menu (call after position is applied). */
export function gsapMenuPrepare(el: Element) {
  const target = el as HTMLElement;
  safeGsap(
    "menuPrepare",
    () => {
      gsap.set(target, { autoAlpha: 0 });
    },
    () => {
      target.style.opacity = "0";
      target.style.visibility = "hidden";
    },
  );
}

export function gsapMenuEnter(el: Element, done?: () => void) {
  const target = el as HTMLElement;
  const finish = onceDone(() => done?.());

  safeGsap(
    "menuEnter",
    () => {
      if (prefersReducedMotion()) {
        gsap.set(target, { autoAlpha: 1, clearProps: "transform" });
        finish();
        return;
      }

      gsap.killTweensOf(target);
      gsap.fromTo(
        target,
        { autoAlpha: 0, y: 3 },
        {
          autoAlpha: 1,
          y: 0,
          duration: COMPOSER_ENTER,
          clearProps: "transform",
          onComplete: finish,
        },
      );
    },
    () => {
      target.style.opacity = "1";
      target.style.visibility = "visible";
      target.style.transform = "";
      finish();
    },
  );
}

export function gsapMenuLeave(el: Element, done: () => void) {
  const target = el as HTMLElement;
  const finish = onceDone(done);

  safeGsap(
    "menuLeave",
    () => {
      if (prefersReducedMotion()) {
        finish();
        return;
      }

      gsap.killTweensOf(target);
      // Opacity-only leave — scale fights window resize and feels toy-like.
      gsap.to(target, {
        autoAlpha: 0,
        duration: COMPOSER_LEAVE,
        ease: "power2.in",
        onComplete: finish,
      });
    },
    () => {
      target.style.opacity = "0";
      finish();
    },
  );
}

/**
 * Chat thread panel enter/leave.
 * Prefer opacity + y — never scaleY (fights overlay window resize).
 * Do not use autoAlpha: visibility:hidden at enter start hides the first
 * messages if the tween is interrupted by window resize.
 */
export function gsapOverlayThreadEnter(el: Element, done: () => void) {
  const target = el as HTMLElement;
  const finish = onceDone(done);

  safeGsap(
    "overlayThreadEnter",
    () => {
      if (prefersReducedMotion()) {
        clearGsapProps(target);
        finish();
        return;
      }

      gsap.killTweensOf(target);
      gsap.set(target, { visibility: "visible", opacity: 0, y: 10 });
      gsap.to(target, {
        opacity: 1,
        y: 0,
        duration: OVERLAY_ENTER,
        ease: "power3.out",
        clearProps: "transform,opacity",
        onComplete: finish,
        onInterrupt: () => {
          clearGsapProps(target, "transform,opacity,visibility");
          finish();
        },
      });
    },
    () => {
      target.style.opacity = "1";
      target.style.visibility = "visible";
      target.style.transform = "";
      finish();
    },
  );
}

export function gsapOverlayThreadLeave(el: Element, done: () => void) {
  const target = el as HTMLElement;
  const finish = onceDone(done);

  safeGsap(
    "overlayThreadLeave",
    () => {
      if (prefersReducedMotion()) {
        finish();
        return;
      }

      gsap.killTweensOf(target);
      gsap.to(target, {
        opacity: 0,
        y: 8,
        duration: OVERLAY_LEAVE,
        ease: "power2.in",
        onComplete: finish,
        onInterrupt: finish,
      });
    },
    () => {
      target.style.opacity = "0";
      finish();
    },
  );
}

/** Composer dock show/hide when overlay visibility flips — instant, no tween. */
export function gsapOverlayDockReveal(el: Element | null, visible: boolean) {
  if (!el) return;
  const target = el as HTMLElement;

  const apply = () => {
    target.style.opacity = visible ? "1" : "0";
    target.style.visibility = visible ? "visible" : "hidden";
    target.style.transform = "";
  };

  safeGsap(
    "overlayDockReveal",
    () => {
      gsap.killTweensOf(target);
      gsap.set(target, {
        opacity: visible ? 1 : 0,
        visibility: visible ? "visible" : "hidden",
        y: 0,
        clearProps: visible ? "opacity,transform,visibility" : undefined,
      });
    },
    apply,
  );
}

/** Settings category panel swap (right pane). */
export function gsapSettingsPanelEnter(el: Element, done: () => void) {
  // Keep enter/leave synchronous. Animated hooks + Vue mode="out-in" blank the
  // panel forever when GSAP onComplete never runs — reproduced as “only Model
  // works; every other settings page white-screens/freezes” on some WebView2 setups.
  const finish = onceDone(done);
  safeGsap(
    "settingsPanelEnter",
    () => {
      gsap.killTweensOf(el as HTMLElement);
      clearGsapProps(el as HTMLElement);
      finish();
    },
    finish,
  );
}

export function gsapSettingsPanelLeave(el: Element, done: () => void) {
  const finish = onceDone(done);
  safeGsap(
    "settingsPanelLeave",
    () => {
      gsap.killTweensOf(el as HTMLElement);
      finish();
    },
    finish,
  );
}

/** Settings sidebar items on first open. */
export function gsapSettingsNavMount(root: Element) {
  if (prefersReducedMotion()) return;
  const items = [...root.querySelectorAll<HTMLElement>(".settings-nav-item")];
  if (!items.length) return;

  safeGsap(
    "settingsNavMount",
    () => {
      gsap.killTweensOf(items);
      // Opacity only — autoAlpha:0 can leave the nav permanently hidden if the
      // tween never completes on stalled WebView2/GPU tickers.
      gsap.fromTo(
        items,
        { opacity: 0.2, x: -5 },
        {
          opacity: 1,
          x: 0,
          duration: 0.16,
          stagger: 0.018,
          clearProps: "opacity,transform",
        },
      );
    },
    () => {
      for (const item of items) {
        item.style.opacity = "1";
        item.style.transform = "";
      }
    },
  );
}

export function gsapSettingsNavUnmount(root?: Element | null) {
  if (!root) return;
  const items = [...root.querySelectorAll<HTMLElement>(".settings-nav-item")];
  gsap.killTweensOf(items);
  for (const item of items) {
    item.style.opacity = "";
    item.style.transform = "";
  }
}

/**
 * First-run welcome: move the floating logo onto the empty-conversation brand,
 * then circular-reveal the workspace by shrinking the overlay mask.
 */
export function gsapOnboardingReveal(opts: {
  overlay: HTMLElement;
  logo: HTMLElement;
  from: DOMRect;
  target: DOMRect;
  onComplete: () => void;
}) {
  const { overlay, logo, from, target, onComplete } = opts;
  const finish = onceDone(onComplete);
  const originX = target.left + target.width / 2;
  const originY = target.top + target.height / 2;

  safeGsap(
    "onboardingReveal",
    () => {
      gsap.killTweensOf([overlay, logo]);

      // Freeze the logo in viewport space so CSS layout changes cannot skew the path.
      gsap.set(logo, {
        position: "fixed",
        left: from.left,
        top: from.top,
        width: from.width,
        height: from.height,
        margin: 0,
        x: 0,
        y: 0,
        scale: 1,
        transformOrigin: "50% 50%",
        zIndex: 3,
      });

      if (prefersReducedMotion()) {
        overlay.style.clipPath = "";
        finish();
        return;
      }

      const maxRadius = Math.hypot(
        Math.max(originX, window.innerWidth - originX),
        Math.max(originY, window.innerHeight - originY),
      );

      const state = { radius: maxRadius * 1.2 };
      overlay.style.clipPath = `circle(${state.radius}px at ${originX}px ${originY}px)`;

      const tl = gsap.timeline({
        onComplete: () => {
          overlay.style.clipPath = "";
          finish();
        },
      });

      tl.to(logo, {
        left: target.left,
        top: target.top,
        width: target.width,
        height: target.height,
        duration: 1.05,
        ease: "power3.inOut",
      });

      tl.to(
        state,
        {
          radius: 0,
          duration: 0.95,
          ease: "power2.inOut",
          onUpdate: () => {
            overlay.style.clipPath = `circle(${state.radius}px at ${originX}px ${originY}px)`;
          },
        },
        "-=0.28",
      );
    },
    () => {
      overlay.style.clipPath = "";
      finish();
    },
  );
}
