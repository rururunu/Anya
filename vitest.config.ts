import path from "node:path";
import { defineConfig } from "vitest/config";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
      "@/services": path.resolve(__dirname, "./src/services"),
      "@/stores": path.resolve(__dirname, "./src/stores"),
    },
  },
  test: {
    environment: "node",
    include: ["src/**/*.{test,spec}.ts"],
    globals: false,
    // A globally exported NODE_ENV=production would make Vue skip its dev-only
    // devtools emit reporting, silently breaking VTU `emitted()` assertions.
    env: {
      NODE_ENV: "test",
    },
  },
});
