/** Official Anya terminal: ConPTY via host `pty.*`. No static imports — workbench loads this as a blob. */

const PREFS_KEY = "anya.plugin.terminal.prefs";
const PREFS_TOPIC = "terminal.prefs";
const FONT_SIZE_MIN = 11;
const FONT_SIZE_MAX = 20;
const FONT_SIZE_DEFAULT = 13;

const LEGACY_FONT_IDS = {
  cascadia: "Cascadia Mono",
  consolas: "Consolas",
  jetbrains: "JetBrains Mono",
  maple: "Maple Mono",
  sarasa: "Sarasa Gothic SC",
  source: "Source Code Pro",
  fira: "Fira Code",
  courier: "Courier New",
};

const FALLBACK_FONT_PROBES = [
  "Cascadia Mono",
  "Cascadia Code",
  "Consolas",
  "Courier New",
  "Lucida Console",
  "Lucida Sans Typewriter",
  "MS Gothic",
  "NSimSun",
  "SimSun",
  "SimHei",
  "KaiTi",
  "FangSong",
  "Microsoft YaHei",
  "Microsoft YaHei UI",
  "Microsoft YaHei Mono",
  "DengXian",
  "Segoe UI",
  "Calibri",
  "Cambria",
  "Georgia",
  "Tahoma",
  "Verdana",
  "Arial",
  "Times New Roman",
  "JetBrains Mono",
  "Maple Mono",
  "Maple Mono NF",
  "Sarasa Gothic SC",
  "Sarasa Term SC",
  "Source Code Pro",
  "Fira Code",
  "Hack",
  "Iosevka",
  "Ubuntu Mono",
  "DejaVu Sans Mono",
  "Noto Sans Mono",
  "Noto Sans SC",
  "IBM Plex Mono",
  "Roboto Mono",
  "CaskaydiaCove Nerd Font",
  "JetBrainsMono Nerd Font",
];

function cssVar(name, fallback) {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

function monoStack() {
  const anya = cssVar(
    "--font-mono",
    '"JetBrains Mono Variable", "Cascadia Mono", Consolas, monospace',
  );
  return [
    '"Sarasa Term SC Nerd"',
    '"SarasaTermSC Nerd Font"',
    '"Maple Mono NF"',
    '"Maple Mono"',
    '"CaskaydiaCove Nerd Font"',
    '"JetBrainsMono Nerd Font"',
    '"Symbols Nerd Font Mono"',
    anya,
  ].join(",");
}

function canvasContext() {
  const canvas = document.createElement("canvas");
  return canvas.getContext("2d");
}

function fontInstalled(name) {
  const ctx = canvasContext();
  if (!ctx) return false;
  const sample = "mmmmmmmmlli@Ww";
  ctx.font = '16px "FontNameThatDoesNotExistXYZ", monospace';
  const fallback = ctx.measureText(sample).width;
  ctx.font = `16px "${name}", monospace`;
  return ctx.measureText(sample).width !== fallback;
}

function isMonospace(name) {
  const ctx = canvasContext();
  if (!ctx) return false;
  ctx.font = `16px "${name}"`;
  return Math.abs(ctx.measureText("i").width - ctx.measureText("M").width) < 0.6;
}

function quoteFamily(name) {
  return `"${String(name).replace(/["\\]/g, "")}"`;
}

function resolveFontFamily(id) {
  const name = String(id || "").trim();
  if (!name || name === "default") return monoStack();
  return `${quoteFamily(name)}, ${monoStack()}`;
}

function normalizeFontFamily(value) {
  const raw = String(value || "").trim();
  if (!raw || raw === "default") return "default";
  return LEGACY_FONT_IDS[raw] ?? raw;
}

let cachedFamilies = null;

async function listSystemFontFamilies() {
  if (cachedFamilies) return cachedFamilies;
  const names = new Set();
  try {
    if (typeof globalThis.queryLocalFonts === "function") {
      const fonts = await globalThis.queryLocalFonts();
      for (const font of fonts) {
        const family = String(font.family || "").trim();
        if (family) names.add(family);
      }
    }
  } catch {
    /* permission / unsupported */
  }
  if (!names.size) {
    for (const name of FALLBACK_FONT_PROBES) {
      if (fontInstalled(name)) names.add(name);
    }
  }
  cachedFamilies = [...names].sort((a, b) => a.localeCompare(b, undefined, { sensitivity: "base" }));
  return cachedFamilies;
}

function loadPrefs() {
  try {
    const raw = JSON.parse(localStorage.getItem(PREFS_KEY) || "null");
    if (!raw || typeof raw !== "object") {
      return { fontSize: FONT_SIZE_DEFAULT, fontFamily: "default" };
    }
    const fontSize = Math.min(
      FONT_SIZE_MAX,
      Math.max(FONT_SIZE_MIN, Number(raw.fontSize) || FONT_SIZE_DEFAULT),
    );
    return { fontSize, fontFamily: normalizeFontFamily(raw.fontFamily) };
  } catch {
    return { fontSize: FONT_SIZE_DEFAULT, fontFamily: "default" };
  }
}

function savePrefs(prefs) {
  localStorage.setItem(PREFS_KEY, JSON.stringify(prefs));
}

function anyaXtermTheme() {
  const bg = cssVar("--peek-code-bg", cssVar("--peek-list-bg", "#1c1c1c"));
  const fg = cssVar("--peek-code-fg", cssVar("--peek-text", "#e8e8e8"));
  return {
    background: bg,
    foreground: fg,
    cursor: cssVar("--peek-accent", fg),
    cursorAccent: bg,
    selectionBackground: cssVar("--peek-code-selection", "#264f78"),
    selectionForeground: fg,
    black: cssVar("--peek-faint", "#6e6e6e"),
    red: cssVar("--peek-danger", "#e85d5d"),
    green: cssVar("--peek-success", "#5dba83"),
    yellow: cssVar("--peek-warning", "#d4a017"),
    blue: cssVar("--peek-info", "#6cb6ff"),
    magenta: cssVar("--peek-syntax-function", "#d2a8ff"),
    cyan: cssVar("--peek-chart-6", "#3ec6d9"),
    white: fg,
    brightBlack: cssVar("--peek-muted", "#9a9a9a"),
    brightRed: cssVar("--peek-syntax-keyword", "#ff7b72"),
    brightGreen: cssVar("--peek-success", "#5dba83"),
    brightYellow: cssVar("--peek-syntax-type", "#ffa657"),
    brightBlue: cssVar("--peek-syntax-number", "#79c0ff"),
    brightMagenta: cssVar("--peek-syntax-function", "#d2a8ff"),
    brightCyan: cssVar("--peek-chart-6", "#3ec6d9"),
    brightWhite: cssVar("--peek-text", fg),
  };
}

function paintChrome(el, bar, status) {
  const bg = cssVar("--peek-code-bg", cssVar("--peek-list-bg", "#1c1c1c"));
  el.style.background = bg;
  el.style.color = cssVar("--peek-text", "#e8e8e8");
  bar.style.background = cssVar("--peek-code-toolbar-bg", cssVar("--peek-sidebar", "#101010"));
  bar.style.borderBottom = `1px solid ${cssVar("--peek-code-border", cssVar("--peek-border", "rgba(255,255,255,.1)"))}`;
  bar.style.color = cssVar("--peek-muted", "#9a9a9a");
  bar.style.fontFamily = cssVar("--peek-font-sans", "system-ui, sans-serif");
  status.style.color = cssVar("--peek-muted", "#9a9a9a");
}

function pluginAssetUrls(pluginId, rel) {
  const path = `${pluginId}/${rel.replace(/^\/+/, "")}`;
  return [`http://anya-plugin.localhost/${path}`, `anya-plugin://localhost/${path}`];
}

async function fetchPluginText(pluginId, rel) {
  let last = "";
  for (const url of pluginAssetUrls(pluginId, rel)) {
    try {
      const res = await fetch(url);
      if (res.ok) return await res.text();
      last = `${url} ${res.status}`;
    } catch (err) {
      last = String(err);
    }
  }
  throw new Error(last || `missing ${rel}`);
}

async function loadXterm(pluginId) {
  if (typeof globalThis.Terminal === "function") return globalThis.Terminal;
  const css = await fetchPluginText(pluginId, "ui/src/vendor/xterm.css");
  if (!document.getElementById("plugin-xterm-css")) {
    const style = document.createElement("style");
    style.id = "plugin-xterm-css";
    style.textContent = css;
    document.head.appendChild(style);
  }
  const js = await fetchPluginText(pluginId, "ui/src/vendor/xterm.js");
  const blob = URL.createObjectURL(new Blob([js], { type: "text/javascript" }));
  await new Promise((resolve, reject) => {
    const el = document.createElement("script");
    el.src = blob;
    el.onload = () => {
      URL.revokeObjectURL(blob);
      resolve();
    };
    el.onerror = () => {
      URL.revokeObjectURL(blob);
      reject(new Error("xterm script failed"));
    };
    document.head.appendChild(el);
  });
  const Terminal = globalThis.Terminal;
  if (typeof Terminal !== "function") throw new Error("xterm Terminal missing");
  return Terminal;
}

function invoke(cmd, args) {
  const fn = globalThis.__TAURI_INTERNALS__?.invoke;
  if (typeof fn !== "function") return Promise.reject(new Error("invoke unavailable"));
  return fn(cmd, args ?? {});
}

async function resolveCwd(ctx) {
  const chatId = typeof ctx?.agent?.sessionId === "function" ? ctx.agent.sessionId() : "";
  let workspaceId = "";
  try {
    workspaceId = String(ctx.stores?.chat?.sessionCompose?.[chatId]?.draftWorkspaceId ?? "").trim();
  } catch {
    /* pinia shape may vary */
  }
  if (!workspaceId) {
    try {
      const summaries =
        ctx.pinia?.state?.value?.chatSessions?.summaries ??
        (await invoke("list_chat_sessions"))?.sessions;
      const row = (summaries || []).find((item) => (item.sessionId || item.session_id) === chatId);
      workspaceId = String(row?.workspaceId || row?.workspace_id || "").trim();
    } catch {
      /* list next */
    }
  }
  if (workspaceId) {
    try {
      const list = await invoke("list_workspaces");
      const ws = (list || []).find((item) => item.id === workspaceId);
      const root = String(ws?.root ?? "").trim();
      if (root) return root;
    } catch {
      /* current workspace next */
    }
  }
  try {
    const current = await invoke("get_current_workspace");
    const root = String(current?.root ?? "").trim();
    if (root) return root;
  } catch {
    /* user dir next */
  }
  try {
    const dir = String((await invoke("get_anya_user_dir")) ?? "").trim();
    if (dir) return dir;
  } catch {
    return undefined;
  }
  return undefined;
}

function ensureSettingsCss() {
  if (document.getElementById("plugin-terminal-settings-css")) return;
  const style = document.createElement("style");
  style.id = "plugin-terminal-settings-css";
  style.textContent = `
.term-settings{display:flex;flex-direction:column;gap:18px;max-width:28rem;color:var(--peek-text);font-size:13px;line-height:1.45}
.term-settings h3{margin:0;font-size:12px;font-weight:650;letter-spacing:.02em;text-transform:uppercase;color:var(--peek-muted)}
.term-settings .row{display:flex;flex-direction:column;gap:8px}
.term-settings .row-head{display:flex;align-items:baseline;justify-content:space-between;gap:8px}
.term-settings .hint{margin:0;font-size:12px;color:var(--peek-muted)}
.term-settings select.font-select{width:100%;font:inherit;font-size:13px;padding:7px 10px;border-radius:8px;color:var(--peek-text);background:var(--peek-surface,rgba(255,255,255,.06));border:1px solid var(--peek-border,rgba(255,255,255,.12))}
.term-settings select.font-select option,.term-settings select.font-select optgroup{background:#fff;color:#111}
.term-settings input[type="range"]{width:100%;accent-color:var(--peek-accent,#ef4444)}
.term-settings .preview{margin:0;padding:12px 14px;border-radius:10px;border:1px solid var(--peek-border,rgba(255,255,255,.1));background:var(--peek-code-bg,var(--peek-list-bg,#1c1c1c));color:var(--peek-code-fg,var(--peek-text,#e8e8e8));white-space:pre-wrap;line-height:1.45}
`;
  document.head.appendChild(style);
}

function mountSettings(ctx) {
  ctx.home.setSettingsView((el) => {
    ensureSettingsCss();
    el.innerHTML = "";
    const wrap = document.createElement("div");
    wrap.className = "term-settings";
    const t = (key, en, zh) => ctx.i18n.t(key, en, { "zh-CN": zh, en });
    let prefs = loadPrefs();

    const appearance = document.createElement("h3");
    appearance.textContent = t("settings.appearance", "Appearance", "外观");

    const fontRow = document.createElement("div");
    fontRow.className = "row";
    const fontLabel = document.createElement("div");
    fontLabel.textContent = t("settings.font", "Font", "字体");
    const select = document.createElement("select");
    select.className = "font-select";
    const defaultOpt = document.createElement("option");
    defaultOpt.value = "default";
    defaultOpt.textContent = t("font.default", "Anya default", "Anya 默认");
    select.append(defaultOpt);
    select.value = "default";

    const sizeRow = document.createElement("div");
    sizeRow.className = "row";
    const sizeHead = document.createElement("div");
    sizeHead.className = "row-head";
    const sizeLabel = document.createElement("span");
    sizeLabel.textContent = t("settings.size", "Size", "字号");
    const sizeValue = document.createElement("span");
    sizeValue.style.color = "var(--peek-muted)";
    const slider = document.createElement("input");
    slider.type = "range";
    slider.min = String(FONT_SIZE_MIN);
    slider.max = String(FONT_SIZE_MAX);
    slider.step = "1";

    const preview = document.createElement("pre");
    preview.className = "preview";

    const paintPreview = () => {
      sizeValue.textContent = `${prefs.fontSize}px`;
      slider.value = String(prefs.fontSize);
      select.value = prefs.fontFamily;
      select.style.fontFamily = resolveFontFamily(prefs.fontFamily);
      preview.style.fontFamily = resolveFontFamily(prefs.fontFamily);
      preview.style.fontSize = `${prefs.fontSize}px`;
      preview.textContent = t(
        "settings.preview",
        "Anya> echo hello\nhello",
        "Anya> echo 你好\n你好",
      );
    };

    const commit = (next) => {
      prefs = next;
      savePrefs(prefs);
      ctx.bus.publish(PREFS_TOPIC, prefs);
      paintPreview();
    };

    const fontOption = (name) => {
      const opt = document.createElement("option");
      opt.value = name;
      opt.textContent = name;
      opt.style.fontFamily = quoteFamily(name);
      return opt;
    };

    void listSystemFontFamilies().then((families) => {
      const mono = [];
      const other = [];
      for (const name of families) {
        (isMonospace(name) ? mono : other).push(name);
      }
      if (mono.length) {
        const group = document.createElement("optgroup");
        group.label = t("settings.mono", "Monospace", "等宽");
        for (const name of mono) group.append(fontOption(name));
        select.append(group);
      }
      if (other.length) {
        const group = document.createElement("optgroup");
        group.label = t("settings.other", "Other", "其他");
        for (const name of other) group.append(fontOption(name));
        select.append(group);
      }
      if (prefs.fontFamily !== "default" && !families.includes(prefs.fontFamily)) {
        select.insertBefore(fontOption(prefs.fontFamily), select.children[1] || null);
      }
      paintPreview();
    });

    select.onchange = () => commit({ ...prefs, fontFamily: select.value });
    slider.oninput = () => commit({ ...prefs, fontSize: Number(slider.value) });

    const hint = document.createElement("p");
    hint.className = "hint";
    hint.textContent = t(
      "settings.hint",
      "Lists fonts installed on this computer. Changes apply immediately.",
      "列出本机已安装的字体。修改后立刻作用于已打开的侧栏终端。",
    );

    fontRow.append(fontLabel, select);
    sizeHead.append(sizeLabel, sizeValue);
    sizeRow.append(sizeHead, slider);
    wrap.append(appearance, fontRow, sizeRow, preview, hint);
    el.append(wrap);
    paintPreview();
    return () => {
      el.replaceChildren();
    };
  });
}

export async function activate(ctx) {
  mountSettings(ctx);
  ctx.sidebar.addTab({
    id: "terminal",
    title: "Terminal",
    icon: `anya-plugin://localhost/${ctx.pluginId}/ui/icon.svg`,
    surfaces: ["views"],
    mount(el) {
      el.innerHTML = "";
      el.style.cssText =
        "display:flex;flex-direction:column;flex:1;min-width:0;min-height:0;width:100%;height:100%;";
      const bar = document.createElement("div");
      bar.style.cssText =
        "display:flex;align-items:center;gap:8px;padding:6px 10px;flex:none;font-size:12px;line-height:1.4;";
      const status = document.createElement("span");
      status.style.cssText =
        "flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;";
      status.textContent = "Starting…";
      const refreshBtn = document.createElement("button");
      refreshBtn.type = "button";
      refreshBtn.className = "term-refresh";
      refreshBtn.innerHTML =
        '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.85" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M8 16H3v5"/></svg>';
      bar.append(status, refreshBtn);
      const screen = document.createElement("div");
      screen.style.cssText =
        "flex:1;min-height:0;min-width:0;padding:8px 10px 10px;position:relative;display:flex;flex-direction:column;";
      el.append(bar, screen);
      paintChrome(el, bar, status);

      const style = document.createElement("style");
      style.textContent =
        ".xterm,.xterm-viewport,.xterm-screen{background:transparent!important}" +
        ".xterm{padding:0;height:100%}" +
        ".xterm-viewport{overflow-y:auto}" +
        ".term-refresh{flex:none;display:grid;place-items:center;width:24px;height:24px;padding:0;border:none;border-radius:999px;background:transparent;color:inherit;opacity:.7;cursor:pointer}" +
        ".term-refresh:hover{opacity:1;background:var(--peek-surface-muted,rgba(255,255,255,.08))}" +
        ".term-refresh.is-busy{opacity:.4;pointer-events:none}" +
        ".term-shell{flex:1;min-height:0;min-width:0;display:none;flex-direction:column;height:100%}" +
        ".term-shell.is-on{display:flex}";
      el.appendChild(style);

      const t = (en, zh) => ctx.i18n.t("term.refresh", en, { "zh-CN": zh, en });
      refreshBtn.title = t("Reconnect in this chat's workspace", "按当前对话工作区重新连接");
      refreshBtn.setAttribute("aria-label", refreshBtn.title);

      let prefs = loadPrefs();
      let TerminalCtor = null;
      let activeChatId = "";
      let poll = null;
      const shells = new Map();
      const lru = [];
      const MAX_SHELLS = 12;

      const chatKey = () => {
        const id = typeof ctx.agent.sessionId === "function" ? ctx.agent.sessionId() : "";
        return id || "__none__";
      };

      const touch = (id) => {
        const i = lru.indexOf(id);
        if (i >= 0) lru.splice(i, 1);
        lru.push(id);
      };

      const applyFontAll = (next) => {
        prefs = next;
        const family = resolveFontFamily(prefs.fontFamily);
        for (const rec of shells.values()) {
          if (rec.term) {
            rec.term.options.fontFamily = family;
            rec.term.options.fontSize = prefs.fontSize;
          }
        }
        fitActive();
      };

      const cell = (term) => {
        const dims = term?._core?._renderService?.dimensions?.css?.cell;
        if (dims?.width > 1 && dims?.height > 1) return dims;
        return { width: 9, height: 18 };
      };

      const fitRec = (rec) => {
        if (!rec?.term || !rec.pty) return;
        const w = rec.wrap.clientWidth;
        const h = rec.wrap.clientHeight;
        if (w < 8 || h < 8) return;
        const { width, height } = cell(rec.term);
        const cols = Math.max(8, Math.floor(w / width));
        const rows = Math.max(2, Math.floor(h / height));
        rec.term.resize(cols, rows);
        void ctx.host.rpc("pty.resize", { session: rec.pty, cols, rows });
      };

      const fitActive = () => {
        const rec = shells.get(activeChatId);
        if (rec) fitRec(rec);
      };

      const setStatus = (text, danger) => {
        status.textContent = text;
        status.style.color = danger ? cssVar("--peek-danger", "#e85d5d") : cssVar("--peek-muted", "#9a9a9a");
      };

      const dropShell = async (id) => {
        const rec = shells.get(id);
        if (!rec) return;
        rec.observer?.disconnect?.();
        if (rec.pty) void ctx.host.rpc("pty.kill", { session: rec.pty });
        rec.term?.dispose?.();
        rec.wrap.remove();
        shells.delete(id);
        const i = lru.indexOf(id);
        if (i >= 0) lru.splice(i, 1);
      };

      const evictIfNeeded = async () => {
        while (shells.size >= MAX_SHELLS && lru.length) {
          const oldest = lru.find((id) => id !== activeChatId) ?? lru[0];
          if (!oldest || (oldest === activeChatId && shells.size < MAX_SHELLS)) break;
          if (oldest === activeChatId) break;
          await dropShell(oldest);
        }
      };

      const createShell = async (id) => {
        await evictIfNeeded();
        const wrap = document.createElement("div");
        wrap.className = "term-shell";
        screen.appendChild(wrap);
        const cwd = await resolveCwd(ctx);
        let pty = null;
        try {
          const opened = await ctx.host.rpc("pty.open", { cols: 80, rows: 24, cwd });
          pty = opened?.session ?? null;
        } catch (err) {
          setStatus(String(err), true);
        }
        let term = null;
        if (typeof TerminalCtor === "function") {
          term = new TerminalCtor({
            cursorBlink: true,
            fontSize: prefs.fontSize,
            lineHeight: 1.35,
            fontFamily: resolveFontFamily(prefs.fontFamily),
            theme: anyaXtermTheme(),
          });
          term.open(wrap);
          term.onData((data) => {
            if (pty) void ctx.host.rpc("pty.write", { session: pty, data });
          });
        }
        const observer = new ResizeObserver(() => fitRec(shells.get(id)));
        observer.observe(wrap);
        const rec = { pty, term, wrap, cwd, observer };
        shells.set(id, rec);
        touch(id);
        return rec;
      };

      const pendingByChat = new Map();
      const showChat = async (id, opts) => {
        const prev = pendingByChat.get(id) || Promise.resolve();
        const next = prev.catch(() => {}).then(() => showChatUnlocked(id, opts));
        pendingByChat.set(id, next);
        try {
          await next;
        } finally {
          if (pendingByChat.get(id) === next) pendingByChat.delete(id);
        }
      };

      const showChatUnlocked = async (id, { force } = {}) => {
        if (force) await dropShell(id);
        if (!shells.has(id)) {
          setStatus("Starting…");
          await createShell(id);
        }
        activeChatId = id;
        touch(id);
        for (const [key, rec] of shells) {
          rec.wrap.classList.toggle("is-on", key === id);
        }
        const rec = shells.get(id);
        setStatus(rec?.cwd || (rec?.pty ? "Connected" : "No session"), !rec?.pty);
        requestAnimationFrame(() => {
          fitRec(rec);
          rec?.term?.focus?.();
        });
      };

      const applyTheme = () => {
        paintChrome(el, bar, status);
        const theme = anyaXtermTheme();
        for (const rec of shells.values()) {
          if (rec.term) rec.term.options.theme = theme;
        }
      };

      refreshBtn.onclick = async () => {
        refreshBtn.classList.add("is-busy");
        try {
          await showChat(chatKey(), { force: true });
        } finally {
          refreshBtn.classList.remove("is-busy");
        }
      };

      const themeWatch = new MutationObserver(applyTheme);
      themeWatch.observe(document.documentElement, {
        attributes: true,
        attributeFilter: ["data-theme", "data-theme-mode", "style", "class"],
      });

      const offData = ctx.host.on("pty.data", (ev) => {
        if (!ev?.session || !ev.data) return;
        for (const rec of shells.values()) {
          if (rec.pty === ev.session) rec.term?.write(ev.data);
        }
      });
      const offPrefs = ctx.bus.subscribe(PREFS_TOPIC, (payload) => {
        if (!payload || typeof payload !== "object") return;
        applyFontAll(loadPrefs());
      });

      void (async () => {
        try {
          TerminalCtor = await loadXterm(ctx.pluginId);
        } catch (err) {
          setStatus(String(err), true);
        }
        await showChat(chatKey());
        poll = setInterval(() => {
          const id = chatKey();
          if (id !== activeChatId) void showChat(id);
        }, 300);
      })();

      const cleanup = () => {
        if (poll) clearInterval(poll);
        offData?.();
        offPrefs?.();
        themeWatch.disconnect();
        for (const id of [...shells.keys()]) void dropShell(id);
      };
      ctx.onDeactivate(cleanup);
      return cleanup;
    },
  });
}

export function deactivate() {}
