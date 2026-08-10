import path from "node:path";
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [vue(), tailwindcss()],

  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
      "@/services": path.resolve(__dirname, "./src/services"),
      "@/stores": path.resolve(__dirname, "./src/stores"),
    },
  },

  build: {
    // ECharts core alone exceeds Vite's default 500 kB once minified. ChartCard
    // (and chartEchartsExtra / echarts-gl) are already dynamic-imported off the
    // boot path — raise the warn so intentional chart chunks do not fail CI noise.
    chunkSizeWarningLimit: 1000,
    rollupOptions: {
      onwarn(warning, warn) {
        const id = warning.id?.replaceAll("\\", "/") ?? "";
        if (warning.code === "INVALID_ANNOTATION" && id.includes("/node_modules/@vueuse/core/")) {
          return;
        }
        warn(warning);
      },
      output: {
        // Keep vendor splits explicit: MessageList pulls Markdown + FileDiff +
        // CodeDiffEditor, and without this the CodeMirror language packs alone
        // land in a ~1.4 MB app chunk and trip Vite's 500 kB warning.
        onlyExplicitManualChunks: true,
        manualChunks(id) {
          const normalized = id.replaceAll("\\", "/");
          const modulePath = normalized.split("/node_modules/").at(-1);
          if (!modulePath) return undefined;

          const parts = modulePath.split("/");
          const packageName = parts[0]?.startsWith("@") ? `${parts[0]}/${parts[1]}` : parts[0];
          if (!packageName) return undefined;

          if (packageName === "katex") return "vendor-katex";
          if (packageName === "highlight.js") return "vendor-highlight";
          if (["marked", "marked-katex-extension", "dompurify"].includes(packageName)) {
            return "vendor-markdown";
          }
          // Language grammars are dynamically imported by CodeDiffEditor — leave
          // them (and their @lezer/* parsers) out of the shared vendor chunk.
          if (packageName.startsWith("@codemirror/lang-")) return undefined;
          if (
            packageName === "@codemirror/state" ||
            packageName === "@codemirror/view" ||
            packageName === "@codemirror/language" ||
            packageName === "@codemirror/commands" ||
            packageName === "@lezer/highlight" ||
            packageName === "@lezer/common" ||
            packageName === "@lezer/lr"
          ) {
            return "vendor-codemirror";
          }
          if (packageName.startsWith("@codemirror/") || packageName.startsWith("@lezer/")) {
            return undefined;
          }
          if (packageName === "gsap") return "vendor-gsap";
          if (packageName === "vue" || packageName === "vue-router" || packageName === "pinia") {
            return "vendor-vue";
          }
          if (
            packageName === "reka-ui" ||
            packageName === "@vueuse/core" ||
            packageName === "@lucide/vue"
          ) {
            return "vendor-ui";
          }
          return undefined;
        },
      },
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 13330,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 13331,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
