/**
 * 小型 ANSI 转义序列解析器：解析 SGR（颜色/粗体/斜体/下划线）序列，
 * 剥离其余控制序列（光标移动、清屏、OSC 标题等），供终端卡片渲染彩色输出。
 */

export interface AnsiSpan {
  text: string;
  color?: string;
  background?: string;
  bold?: boolean;
  dim?: boolean;
  italic?: boolean;
  underline?: boolean;
}

interface SgrState {
  color?: string;
  background?: string;
  bold?: boolean;
  dim?: boolean;
  italic?: boolean;
  underline?: boolean;
}

/** 深色终端底上的 16 色盘（GitHub Dark 风格，观感专业且对比度足够）。 */
const PALETTE_NORMAL = [
  "#6e7681", // black（提亮以便深底可见）
  "#ff7b72", // red
  "#3fb950", // green
  "#d29922", // yellow
  "#58a6ff", // blue
  "#bc8cff", // magenta
  "#39c5cf", // cyan
  "#b1bac4", // white
];

const PALETTE_BRIGHT = [
  "#8b949e",
  "#ffa198",
  "#56d364",
  "#e3b341",
  "#79c0ff",
  "#d2a8ff",
  "#56d4dd",
  "#f0f6fc",
];

/** CSI（\x1b[...X）、OSC（\x1b]...BEL/ST）与其他单字符转义。 */
// eslint-disable-next-line no-control-regex
const ANSI_TOKEN = /\x1b\[([0-9;:]*)([A-Za-z])|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)?|\x1b[@-Z\\-_]/g;

function color256(index: number): string {
  if (index < 8) return PALETTE_NORMAL[index]!;
  if (index < 16) return PALETTE_BRIGHT[index - 8]!;
  if (index < 232) {
    const value = index - 16;
    const steps = [0, 95, 135, 175, 215, 255];
    const r = steps[Math.floor(value / 36) % 6]!;
    const g = steps[Math.floor(value / 6) % 6]!;
    const b = steps[value % 6]!;
    return `rgb(${r},${g},${b})`;
  }
  const gray = 8 + (index - 232) * 10;
  return `rgb(${gray},${gray},${gray})`;
}

function applySgr(state: SgrState, params: number[]): void {
  if (params.length === 0) params = [0];
  let i = 0;
  while (i < params.length) {
    const code = params[i]!;
    switch (code) {
      case 0:
        delete state.color;
        delete state.background;
        state.bold = state.dim = state.italic = state.underline = false;
        break;
      case 1:
        state.bold = true;
        break;
      case 2:
        state.dim = true;
        break;
      case 3:
        state.italic = true;
        break;
      case 4:
        state.underline = true;
        break;
      case 22:
        state.bold = state.dim = false;
        break;
      case 23:
        state.italic = false;
        break;
      case 24:
        state.underline = false;
        break;
      case 39:
        delete state.color;
        break;
      case 49:
        delete state.background;
        break;
      case 38:
      case 48: {
        const target: "color" | "background" = code === 38 ? "color" : "background";
        const mode = params[i + 1];
        if (mode === 5 && params[i + 2] !== undefined) {
          state[target] = color256(params[i + 2]!);
          i += 2;
        } else if (mode === 2 && params[i + 4] !== undefined) {
          state[target] = `rgb(${params[i + 2]},${params[i + 3]},${params[i + 4]})`;
          i += 4;
        }
        break;
      }
      default:
        if (code >= 30 && code <= 37) state.color = PALETTE_NORMAL[code - 30];
        else if (code >= 90 && code <= 97) state.color = PALETTE_BRIGHT[code - 90];
        else if (code >= 40 && code <= 47) state.background = PALETTE_NORMAL[code - 40];
        else if (code >= 100 && code <= 107) state.background = PALETTE_BRIGHT[code - 100];
        break;
    }
    i += 1;
  }
}

function pushSpan(spans: AnsiSpan[], state: SgrState, text: string): void {
  if (!text) return;
  const last = spans[spans.length - 1];
  const span: AnsiSpan = { text };
  if (state.color) span.color = state.color;
  if (state.background) span.background = state.background;
  if (state.bold) span.bold = true;
  if (state.dim) span.dim = true;
  if (state.italic) span.italic = true;
  if (state.underline) span.underline = true;
  if (
    last &&
    last.color === span.color &&
    last.background === span.background &&
    last.bold === span.bold &&
    last.dim === span.dim &&
    last.italic === span.italic &&
    last.underline === span.underline
  ) {
    last.text += text;
    return;
  }
  spans.push(span);
}

/**
 * 预处理回车：`\r\n` 归一为 `\n`；行内孤立 `\r` 视为整行重写
 * （进度条式输出只保留最终帧）。
 */
function collapseCarriageReturns(input: string): string {
  return input
    .replace(/\r\n/g, "\n")
    .split("\n")
    .map((line) => {
      const idx = line.lastIndexOf("\r");
      return idx >= 0 ? line.slice(idx + 1) : line;
    })
    .join("\n");
}

/** 解析 ANSI 文本为带样式的 span 列表。非 SGR 的控制序列被剥离。 */
export function parseAnsi(input: string): AnsiSpan[] {
  const source = collapseCarriageReturns(input);
  const spans: AnsiSpan[] = [];
  const state: SgrState = {};
  let cursor = 0;
  ANSI_TOKEN.lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = ANSI_TOKEN.exec(source)) !== null) {
    pushSpan(spans, state, source.slice(cursor, match.index));
    cursor = match.index + match[0].length;
    if (match[2] === "m") {
      const params = (match[1] ?? "")
        .split(/[;:]/)
        .filter((part) => part !== "")
        .map((part) => Number.parseInt(part, 10))
        .filter((num) => Number.isFinite(num));
      applySgr(state, params);
    }
    // 其余序列（光标移动、清屏、OSC 等）直接丢弃。
  }
  pushSpan(spans, state, source.slice(cursor));
  return spans;
}

/** 移除所有 ANSI 转义序列，返回纯文本。 */
export function stripAnsi(input: string): string {
  return parseAnsi(input)
    .map((span) => span.text)
    .join("");
}

/** 判断文本中是否包含 ANSI 转义序列。 */
export function hasAnsi(input: string): boolean {
  // eslint-disable-next-line no-control-regex
  return /\x1b[[\]]/.test(input);
}
