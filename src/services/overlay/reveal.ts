/** Native Web Animations: compositor-only properties, no layout or per-frame IPC. */
export function revealOverlay(element: HTMLElement | null): () => void {
  if (
    !element ||
    typeof element.animate !== "function" ||
    globalThis.matchMedia?.("(prefers-reduced-motion: reduce)").matches
  )
    return () => {};
  const animation = element.animate(
    [
      { opacity: 0, transform: "scale(0.97)" },
      { opacity: 1, transform: "scale(1)" },
    ],
    // Restrained ease-out: keep the position stable and settle without overshoot.
    // These are app-tuned parameters, not an Apple-mandated duration or curve.
    { duration: 240, easing: "cubic-bezier(0.22, 1, 0.36, 1)" },
  );
  return () => animation.cancel();
}
