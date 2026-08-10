/**
 * GSAP 3.15 CSSPlugin reads individual CSS transform properties
 * (`translate` / `rotate` / `scale`) from getComputedStyle and calls `.split`
 * whenever the value is not the literal `"none"`.
 *
 * On engines that lack those properties (Chromium < 104 / some WebView2
 * runtimes), getComputedStyle returns `undefined`. `"none"` is still truthy
 * for `translate`, so GSAP enters the bake-in branch and crashes with:
 *   Cannot read properties of undefined (reading 'split')
 *
 * Normalize missing values to `"none"` before any GSAP tween runs. Import this
 * module for its side effect before `import gsap from "gsap"`.
 */

const TRANSFORM_INDEPENDENTS = new Set(["scale", "rotate", "translate"]);

let installed = false;

export function installGsapWebViewPatch(): void {
  if (installed) return;
  if (typeof window === "undefined" || typeof window.getComputedStyle !== "function") {
    return;
  }
  installed = true;

  const original = window.getComputedStyle.bind(window);
  window.getComputedStyle = ((elt: Element, pseudoElt?: string | null) => {
    const cs = original(elt, pseudoElt ?? undefined);
    // Fast path: modern engines already return "none".
    if (
      cs &&
      typeof (cs as CSSStyleDeclaration).scale === "string" &&
      typeof (cs as CSSStyleDeclaration).rotate === "string" &&
      typeof (cs as CSSStyleDeclaration).translate === "string"
    ) {
      return cs;
    }
    return new Proxy(cs, {
      get(target, prop, receiver) {
        const value = Reflect.get(target, prop, receiver);
        if (
          typeof prop === "string" &&
          TRANSFORM_INDEPENDENTS.has(prop) &&
          (value == null || value === "")
        ) {
          return "none";
        }
        return typeof value === "function" ? value.bind(target) : value;
      },
    });
  }) as typeof window.getComputedStyle;
}

installGsapWebViewPatch();
