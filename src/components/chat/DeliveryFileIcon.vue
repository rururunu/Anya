<template>
  <img class="delivery-file-icon" :src="iconUrl" width="28" height="28" alt="" aria-hidden="true" />
</template>

<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{ name: string }>();
const icons = import.meta.glob<string>("../../assets/material-file-icons/*.svg", {
  eager: true,
  query: "?url",
  import: "default",
});
const extensionIcons: Record<string, string> = {};
const groups: Record<string, string[]> = {
  powerpoint: ["ppt", "pptx", "odp", "key"],
  pdf: ["pdf"],
  word: ["doc", "docx", "odt", "rtf", "pages"],
  table: ["xls", "xlsx", "xlsm", "csv", "tsv", "ods", "numbers"],
  image: ["png", "jpg", "jpeg", "gif", "webp", "svg", "avif", "bmp", "heic"],
  video: ["mp4", "mov", "webm", "mkv", "avi"],
  audio: ["mp3", "wav", "ogg", "flac", "m4a"],
  zip: ["zip", "7z", "rar", "tar", "gz"],
  markdown: ["md", "mdx", "markdown"],
  javascript: ["js", "mjs", "cjs"],
  typescript: ["ts", "mts", "cts"],
  react: ["jsx", "tsx"],
  vue: ["vue"],
  python: ["py", "pyw"],
  rust: ["rs"],
  kotlin: ["kt", "kts"],
  java: ["java", "jar"],
  json: ["json", "jsonc", "json5"],
  html: ["html", "htm"],
  css: ["css"],
  console: ["sh", "bash", "zsh", "bat", "cmd"],
  powershell: ["ps1", "psm1", "psd1"],
};
for (const [icon, extensions] of Object.entries(groups)) {
  for (const extension of extensions) extensionIcons[extension] = icon;
}
const iconUrl = computed(() => {
  const basename = props.name.split(/[\\/]/).pop() ?? "";
  const dot = basename.lastIndexOf(".");
  const extension = dot >= 0 ? basename.slice(dot + 1).toLowerCase() : "";
  const icon = extensionIcons[extension] ?? "document";
  return icons[`../../assets/material-file-icons/${icon}.svg`];
});
</script>

<style scoped>
.delivery-file-icon {
  display: block;
  flex: none;
  object-fit: contain;
}
</style>
