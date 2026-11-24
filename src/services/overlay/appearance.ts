import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

function isPeekWindow(label: string) {
  return label === "overlay" || label.startsWith("overlay-");
}

export async function refreshOverlayWindowBackground() {
  const window = getCurrentWebviewWindow();
  if (!isPeekWindow(window.label)) {
    return;
  }

  try {
    await window.clearEffects();
    await window.setShadow(false);
    // Do NOT call webview.setBackgroundColor() here, even with alpha 0.
    // WebView2's explicit background color takes a different composition
    // path than the default (unset) background, and on this transparent/
    // layered window that path stops blending correctly with the desktop —
    // it paints solid black instead of true transparency. The window's own
    // `transparent: true` config is sufficient; leave the webview background
    // untouched.
  } catch (error) {
    console.error("overlay window background failed:", error);
  }
}

export async function applyOpacity(opacity: number) {
  const norm = Math.max(0.1, Math.min(1, opacity / 100));
  document.documentElement.style.setProperty("--peek-opacity", String(norm));
  const glassPercent = Math.round(25 + norm * 57);
  document.documentElement.style.setProperty("--workbench-glass-opacity", `${glassPercent}%`);
  document.documentElement.classList.toggle("frosted-glass", opacity < 100);
  await refreshOverlayWindowBackground();
}

export async function reloadAppearanceWindow() {
  await refreshOverlayWindowBackground();
}

export function markPeekWindow() {
  document.documentElement.classList.add("peek-window");
  // Drop the solid HTML splash immediately — it looks like a Win32 popup flash
  // when the overlay window is first shown.
  const splash = document.getElementById("boot-splash");
  if (splash) {
    splash.hidden = true;
    splash.setAttribute("aria-busy", "false");
  }
  document.documentElement.style.background = "transparent";
  document.body.style.background = "transparent";
  void refreshOverlayWindowBackground();
}
