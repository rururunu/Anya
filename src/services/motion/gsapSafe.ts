/**
 * Single GSAP entry for the app: applies the WebView2 transform-property
 * patch first, then re-exports gsap. Prefer this over `import gsap from "gsap"`
 * so every call site is protected.
 */
import "./gsapWebViewPatch";
import gsap from "gsap";
import { ScrollToPlugin } from "gsap/ScrollToPlugin";

gsap.registerPlugin(ScrollToPlugin);

gsap.defaults({
  ease: "power2.out",
});

export { gsap };

/** Run a GSAP setup/tween block; never let WebView quirks crash the UI. */
export function safeGsap(label: string, run: () => void, fallback?: () => void): void {
  try {
    run();
  } catch (error) {
    console.warn(`[gsap:${label}] failed; using fallback`, error);
    try {
      fallback?.();
    } catch (fallbackError) {
      console.warn(`[gsap:${label}] fallback also failed`, fallbackError);
    }
  }
}

/** Clear inline styles GSAP may have left on an element. */
export function clearGsapProps(target: HTMLElement | HTMLElement[], props = "all"): void {
  try {
    gsap.set(target, { clearProps: props });
  } catch {
    const list = Array.isArray(target) ? target : [target];
    for (const el of list) {
      el.style.opacity = "";
      el.style.visibility = "";
      el.style.transform = "";
    }
  }
}
