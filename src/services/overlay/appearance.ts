import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { Effect, EffectState } from "@tauri-apps/api/window";

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
  document.documentElement.style.setProperty("--peek-opacity", String(opacity / 100));
  document.documentElement.classList.toggle("frosted-glass", opacity < 100);
  await refreshOverlayWindowBackground();
}

async function applyWorkbenchGlassEffect(window: ReturnType<typeof getCurrentWebviewWindow>) {
  if (/Mac/i.test(navigator.userAgent)) {
    await window.setEffects({
      effects: [Effect.Sidebar],
      state: EffectState.Active,
    });
    return;
  }

  let lastError: unknown;
  for (const effect of [Effect.Acrylic, Effect.Mica, Effect.Blur]) {
    try {
      await window.setEffects({ effects: [effect] });
      return;
    } catch (error) {
      lastError = error;
    }
  }
  throw lastError;
}

export async function applyChromeFrostedGlass(enabled: boolean) {
  const window = getCurrentWebviewWindow();
  if (window.label !== "workbench") {
    document.documentElement.classList.remove("chrome-frosted-glass");
    return;
  }

  const theme = document.documentElement.dataset.theme === "dark" ? "dark" : "light";

  if (enabled) {
    document.documentElement.classList.add("chrome-frosted-glass");
    document.documentElement.style.colorScheme = "normal";
    document.documentElement.style.background = "transparent";
    document.body.style.colorScheme = theme;
    document.body.style.background = "transparent";
    // Windows acrylic is applied from Rust. Calling setEffects here would
    // switch the window to Win11 SYSTEMBACKDROP (too faint behind WebView2).
    if (!/Windows/i.test(navigator.userAgent)) {
      try {
        await applyWorkbenchGlassEffect(window);
      } catch (error) {
        console.error("workbench frosted glass failed:", error);
      }
    }
    return;
  }

  try {
    await window.clearEffects();
  } catch (error) {
    console.error("clear workbench window effects failed:", error);
  }
  document.documentElement.classList.remove("chrome-frosted-glass");
  document.documentElement.style.colorScheme = theme;
  document.documentElement.style.removeProperty("background");
  document.body.style.removeProperty("color-scheme");
  document.body.style.removeProperty("background");
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
