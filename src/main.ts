import { createApp } from "vue";
import { createPinia } from "pinia";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import App from "./App.vue";
import router from "./router";
import { wireChatIpc } from "@/composables/chat/wireChatIpc";
import { hideBootSplash, waitForNextPaint } from "@/services/bootSplash";
import { markPeekWindow } from "@/services/overlay/appearance";
import { installBrowserGuards } from "@/services/browserGuards";
import { createLogger, rootLogger } from "@/services/logger";
import { warmInstalledResourceIcons } from "@/services/warmIcons";
import "@/services/motion/gsapSafe";
import { useChatStore } from "@/stores/chat";
import { applyTheme, bootstrapThemeHint, useSettingStore } from "@/stores/setting";
import "./styles/index.css";

installBrowserGuards();

const app = createApp(App);
const pinia = createPinia();
const bootLog = createLogger("bootstrap");

function formatErrorDetail(err: unknown): Record<string, unknown> {
  if (err instanceof Error) {
    return {
      name: err.name,
      message: err.message,
      stack: err.stack,
    };
  }
  if (typeof err === "string") {
    return { message: err };
  }
  if (err && typeof err === "object") {
    const record = err as { message?: unknown; name?: unknown; stack?: unknown };
    return {
      name: typeof record.name === "string" ? record.name : undefined,
      message:
        typeof record.message === "string" ? record.message : Object.prototype.toString.call(err),
      stack: typeof record.stack === "string" ? record.stack : undefined,
    };
  }
  return { message: String(err) };
}

app.config.errorHandler = (err, _instance, info) => {
  rootLogger.error("vue errorHandler", {
    info,
    err: formatErrorDetail(err),
  });
};

window.addEventListener("unhandledrejection", (event) => {
  rootLogger.error("unhandledrejection", {
    reason:
      event.reason instanceof Error
        ? { message: event.reason.message, stack: event.reason.stack }
        : event.reason,
  });
});

window.addEventListener("error", (event) => {
  rootLogger.error("window error", {
    message: event.message,
    filename: event.filename,
    lineno: event.lineno,
    colno: event.colno,
  });
});

app.use(pinia);
app.use(router);

const settingStore = useSettingStore();
const chatStore = useChatStore();

/**
 * Boot the correct window surface (workbench / overlay / preview),
 * wire chat IPC listeners, then drop the HTML splash once paint is ready.
 */
async function bootstrap() {
  const webviewWindow = getCurrentWebviewWindow();
  const windowLabel = webviewWindow.label;
  const isOverlay =
    (windowLabel === "overlay" || windowLabel.startsWith("overlay-")) &&
    !windowLabel.startsWith("overlay-preview-");

  // Resolve each interactive route before loading settings. Keep the HTML
  // boot splash up until the workbench loading layer has painted, so we never
  // cut to a blank frame between splash → Suspense → Main loading.
  if (windowLabel === "workbench") {
    void router.replace("/workbench");
    applyTheme(bootstrapThemeHint(settingStore.language));
    // Load persisted settings before Main mounts. Otherwise its first render
    // sees the default onboardingCompleted=false and opens the wizard before
    // the persisted value arrives.
    await settingStore.load();
    applyTheme({
      colorScheme: settingStore.colorScheme,
      language: settingStore.language,
    });
    void warmInstalledResourceIcons(settingStore.mcpServers);
    app.mount("#app");
    await router.isReady();
    await waitForNextPaint();
    hideBootSplash({ fadeMs: 220 });
  } else if (isOverlay) {
    markPeekWindow();
    hideBootSplash({ fadeMs: 0 });
    void router.replace("/overlay");
    applyTheme(bootstrapThemeHint(settingStore.language));
    await settingStore.load();
    applyTheme({
      colorScheme: settingStore.colorScheme,
      language: settingStore.language,
    });
    void warmInstalledResourceIcons(settingStore.mcpServers);
    app.mount("#app");
    await router.isReady();
    await waitForNextPaint();
  } else {
    applyTheme(bootstrapThemeHint(settingStore.language));
    await settingStore.load();
    applyTheme({
      colorScheme: settingStore.colorScheme,
      language: settingStore.language,
    });
    void warmInstalledResourceIcons(settingStore.mcpServers);
  }

  // Wire chat IPC events into the store: stream deltas, tool activity, plan
  // gate, file/url offers, and remote compose sync.
  await wireChatIpc({ chatStore, settingStore });

  if (windowLabel.startsWith("overlay-preview-")) {
    document.documentElement.classList.add("peek-window");
    await router.replace("/image-preview");
  } else if (isOverlay) {
    // The overlay route was mounted eagerly above.
  }

  await router.isReady();
  if (windowLabel !== "workbench" && !isOverlay) {
    app.mount("#app");
    await waitForNextPaint();
    hideBootSplash({ fadeMs: 180 });
  }

  bootLog.info("ready", { windowLabel });
}

void bootstrap().catch((err) => {
  bootLog.error("bootstrap failed", err);
});
