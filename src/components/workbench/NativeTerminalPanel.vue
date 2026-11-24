<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { RotateCcw } from "@lucide/vue";
import { useSettingStore } from "@/stores/setting";
import "./terminal/xterm.css";
import xtermUrl from "./terminal/xterm.js?url";

type TerminalTheme = Record<string, string>;
type Terminal = {
  options: { fontFamily: string; theme: TerminalTheme };
  _core: {
    _charSizeService: { measure(): void };
    _renderService: { dimensions: { css: { cell: { width: number; height: number } } } };
    viewport?: { scrollBarWidth: number };
  };
  open(element: HTMLElement): void;
  write(data: string): void;
  reset(): void;
  focus(): void;
  dispose(): void;
  resize(cols: number, rows: number): void;
  onData(handler: (data: string) => void): { dispose(): void };
};
type TerminalOutput = { event: "pty.data" | "pty.exit"; session: string; data?: string };
const props = defineProps<{ cwd?: string; visible: boolean }>();
const settingStore = useSettingStore();
const zh = computed(() => settingStore.language === "zh-CN");
const surface = ref<HTMLElement>();
const error = ref("");
const starting = ref(true);
const exited = ref(false);
let active = true;
let term: Terminal | undefined;
let session: string | undefined;
let off: UnlistenFn | undefined;
let resize: ResizeObserver | undefined;
let themeObserver: MutationObserver | undefined;
let fitFrame = 0;
let size = { cols: 80, rows: 24 };
let terminalOperation = Promise.resolve();
let opening = false;
const early: TerminalOutput[] = [];

const send = <T,>(operation: string, args = {}) =>
  invoke<T>("workbench_terminal", { operation, ...args });

function fitTerminal() {
  const element = surface.value;
  if (!props.visible || !term || !element?.clientWidth || !element.clientHeight) return;
  // xterm's hidden measurement element contains multiple glyphs. Use the
  // renderer's measured dimensions for one cell, as its Fit addon does.
  const { width, height } = term._core._renderService.dimensions.css.cell;
  if (!width || !height) return;
  const scrollbarWidth = term._core.viewport?.scrollBarWidth ?? 15;
  const next = {
    cols: Math.max(8, Math.floor((element.clientWidth - scrollbarWidth) / width)),
    rows: Math.max(2, Math.floor(element.clientHeight / height)),
  };
  if (next.cols === size.cols && next.rows === size.rows) return;
  size = next;
  term.resize(size.cols, size.rows);
  queueOperation("resize", next);
}

function scheduleFit() {
  if (fitFrame) return;
  fitFrame = requestAnimationFrame(() => {
    fitFrame = 0;
    fitTerminal();
  });
}

function queueOperation(operation: "resize" | "write", args: object) {
  const id = session;
  if (!id || exited.value) return;
  terminalOperation = terminalOperation
    .then(async () => {
      if (active && session === id && !exited.value)
        await send(operation, { session: id, ...args });
    })
    .catch((err) => {
      if (active && session === id && !exited.value) error.value = String(err);
    });
}

function appearance() {
  const tokens = getComputedStyle(surface.value ?? document.documentElement);
  const color = (name: string) => tokens.getPropertyValue(`--peek-${name}`).trim();
  const fontFamily = color("font-mono") || "Consolas, monospace";
  const background = color("code-bg") || color("surface");
  const foreground = color("code-fg") || color("text");
  const theme: TerminalTheme = {
    background,
    foreground,
    cursor: foreground,
    cursorAccent: background,
    selectionBackground: color("code-selection"),
    red: color("danger"),
    green: color("success"),
    yellow: color("warning"),
    blue: color("info"),
    magenta: color("syntax-function"),
    cyan: color("chart-6"),
    brightBlack: color("muted"),
  };
  return { fontFamily, theme };
}

function refreshAppearance() {
  if (!term) return;
  const { fontFamily, theme } = appearance();
  term.options.fontFamily = fontFamily;
  term.options.theme = theme;
  scheduleFit();
}

function onFontsLoaded() {
  term?._core._charSizeService.measure();
  scheduleFit();
}

function onOutput(output: TerminalOutput) {
  if (!active) return;
  if (opening && !session) {
    early.push(output);
    return;
  }
  if (output.session !== session) return;
  if (output.event === "pty.exit") exited.value = true;
  else if (output.data) term?.write(output.data);
}

async function start() {
  starting.value = true;
  error.value = "";
  exited.value = false;
  opening = true;
  try {
    if (session) {
      await send("close", { session });
      session = undefined;
    }
    term?.reset();
    fitTerminal();
    const initialSize = size;
    const id = await send<string>("open", { cwd: props.cwd, ...initialSize });
    if (!active) {
      await send("close", { session: id });
      return;
    }
    session = id;
    for (const output of early.splice(0)) onOutput(output);
    if (initialSize !== size) queueOperation("resize", size);
    if (props.visible) term?.focus();
  } catch (err) {
    if (active) error.value = String(err);
  } finally {
    starting.value = false;
    opening = false;
    early.length = 0;
  }
}

function loadTerminal() {
  type Constructor = new (options: object) => Terminal;
  const globals = globalThis as typeof globalThis & { Terminal?: Constructor };
  if (globals.Terminal) return Promise.resolve(globals.Terminal);
  return new Promise<Constructor>((resolve, reject) => {
    const script = document.createElement("script");
    script.src = xtermUrl;
    script.onload = () => {
      script.remove();
      if (globals.Terminal) resolve(globals.Terminal);
      else reject(new Error("Terminal renderer unavailable"));
    };
    script.onerror = () => {
      script.remove();
      reject(new Error("Unable to load terminal renderer"));
    };
    document.head.append(script);
  });
}

async function initialize() {
  try {
    const Terminal = await loadTerminal();
    if (!active || !surface.value) return;
    term = new Terminal({
      cursorBlink: true,
      fontSize: 13,
      windowsPty: { backend: "conpty" },
      ...appearance(),
    });
    term.open(surface.value);
    term.onData((data) => queueOperation("write", { data }));
    resize = new ResizeObserver(scheduleFit);
    resize.observe(surface.value);
    themeObserver = new MutationObserver(refreshAppearance);
    themeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme", "data-theme-mode", "style"],
    });
    document.fonts.addEventListener("loadingdone", onFontsLoaded);
    const unlisten = await listen<TerminalOutput>("terminal-output", (event) =>
      onOutput(event.payload),
    );
    if (!active) {
      unlisten();
      return;
    }
    off = unlisten;
    await start();
  } catch (err) {
    if (active) {
      error.value = String(err);
      starting.value = false;
    }
  }
}
onMounted(initialize);
watch(
  () => props.visible,
  (visible) => {
    if (visible) scheduleFit();
  },
  { flush: "post" },
);

onBeforeUnmount(() => {
  active = false;
  off?.();
  resize?.disconnect();
  themeObserver?.disconnect();
  document.fonts.removeEventListener("loadingdone", onFontsLoaded);
  cancelAnimationFrame(fitFrame);
  term?.dispose();
  if (session) void send("close", { session }).catch(() => {});
});
</script>

<template>
  <div class="terminal-panel">
    <div class="terminal-toolbar">
      <span>{{ zh ? "本地 PowerShell" : "Local PowerShell" }}</span>
      <span v-if="starting || exited" class="terminal-status" role="status">
        {{ starting ? (zh ? "启动中…" : "Starting…") : zh ? "已退出" : "Exited" }}
      </span>
      <button
        class="icon-button"
        type="button"
        :disabled="starting"
        :title="zh ? '重启终端' : 'Restart terminal'"
        :aria-label="zh ? '重启终端' : 'Restart terminal'"
        @click="term ? start() : initialize()"
      >
        <RotateCcw :size="16" />
      </button>
    </div>
    <p v-if="error" class="terminal-error" role="alert">{{ error }}</p>
    <div ref="surface" class="terminal-surface" @click="term?.focus()" />
  </div>
</template>

<style scoped>
.terminal-panel {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
  margin: 0 8px 8px;
  overflow: hidden;
  border: 1px solid var(--peek-code-border, var(--peek-border));
  border-radius: var(--peek-radius-sm);
  background: var(--peek-code-bg, var(--peek-surface));
  color: var(--peek-code-fg, var(--peek-text));
}
.terminal-toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 36px;
  flex: none;
  padding-inline: 10px 4px;
  background: var(--peek-code-toolbar-bg, var(--peek-surface));
  border-bottom: 1px solid var(--peek-code-border, var(--peek-border));
  font-size: var(--peek-font-xs);
}
.terminal-status {
  color: var(--peek-muted);
}
.terminal-toolbar button {
  margin-left: auto;
  color: var(--peek-muted);
}
.terminal-surface {
  flex: 1 1 0;
  min-height: 0;
  margin: 8px 3px 6px 10px;
  overflow: hidden;
  cursor: text;
}
.terminal-error {
  margin: 8px 10px 0;
  color: var(--peek-danger);
  overflow-wrap: anywhere;
}
</style>
