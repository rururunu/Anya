/** Agent tools for computer use. Actions run in Anya Rust (`computer` permission), not Deno. */

const tools = [
  {
    name: "screenshot",
    description:
      "Capture the foreground window (default) or the desktop, plus an interactive UI Automation control list with image-space rects. Click/move/scroll x,y must use pixels in THIS image. Prefer click_control index/name/id from the list. Do not call after every click.",
    parameters: {
      type: "object",
      properties: {
        scope: {
          type: "string",
          description: "foreground (default) or desktop",
        },
        title: {
          type: "string",
          description: "Substring of a window title to capture instead of the foreground window",
        },
      },
    },
  },
  {
    name: "screen_info",
    description: "Return virtual-screen origin and size without capturing an image.",
    parameters: { type: "object", properties: {} },
  },
  {
    name: "list_windows",
    description: "List visible top-level windows (title + short id). Read-only.",
    parameters: { type: "object", properties: {} },
  },
  {
    name: "focus_window",
    description: "Restore and focus a window by title substring or id from list_windows.",
    parameters: {
      type: "object",
      properties: {
        title: { type: "string" },
        id: { type: "string" },
      },
    },
  },
  {
    name: "find_control",
    description:
      "Search the foreground (or named) window for UI Automation controls whose Name or AutomationId contains the query. Returns up to 20 rows. Read-only.",
    parameters: {
      type: "object",
      properties: {
        name: { type: "string" },
        id: { type: "string", description: "UIA AutomationId substring" },
        windowTitle: { type: "string" },
      },
    },
  },
  {
    name: "click_control",
    description:
      "Click a control from the last screenshot/find_control by index, visible name, or AutomationId. Uses InvokePattern when possible; otherwise clicks the control center. Prefer this over pixel click.",
    parameters: {
      type: "object",
      properties: {
        name: { type: "string" },
        id: { type: "string", description: "UIA AutomationId" },
        index: { type: "integer", description: "0-based index from the last screenshot or find_control" },
      },
    },
  },
  {
    name: "set_value",
    description:
      "Set an edit/combo value via UI Automation ValuePattern (Level 3). Prefer this over click+type for named fields. name, id, or index from the last screenshot/find_control.",
    parameters: {
      type: "object",
      properties: {
        name: { type: "string" },
        id: { type: "string", description: "UIA AutomationId" },
        index: { type: "integer" },
        value: { type: "string" },
      },
      required: ["value"],
    },
  },
  {
    name: "launch",
    description:
      "Level 1: open an app, file, or URI with ShellExecute. Examples: mspaint, notepad, calc, winword.exe, a file path, ms-settings:display, https://…. Faster and more accurate than Start-menu clicking or Win+R.",
    parameters: {
      type: "object",
      properties: {
        target: { type: "string", description: "Executable name, file path, or URI" },
        args: { type: "string", description: "Optional arguments, e.g. a document path" },
        ms: { type: "integer", description: "Wait after launch, 0–2000. Default 200." },
      },
      required: ["target"],
    },
  },
  {
    name: "click",
    description:
      "Tap (press and release) at screenshot-image coordinates. button: left (default), right, middle. count: 2 for double-click. modifiers: ctrl/alt/shift/win (e.g. 'ctrl' or 'ctrl+shift'). Not for drawing: use drag.",
    parameters: {
      type: "object",
      properties: {
        x: { type: "number" },
        y: { type: "number" },
        button: { type: "string", description: "left (default), right, or middle" },
        count: { type: "integer", description: "1 or 2" },
        modifiers: {
          description: "Held during the click: 'ctrl', 'ctrl+shift', 'win', or ['ctrl','alt']",
        },
      },
      required: ["x", "y"],
    },
  },
  {
    name: "drag",
    description:
      "Hold the mouse button and move. x,y + x2,y2 is a STRAIGHT line only. For a curve pass path with many points (8+). For a circle/arc pass cx,cy,r (image pixels) and optional startDeg/endDeg (0–360, y-down). Do not fake curves with two-point drags.",
    parameters: {
      type: "object",
      properties: {
        x: { type: "number" },
        y: { type: "number" },
        x2: { type: "number" },
        y2: { type: "number" },
        path: {
          type: "array",
          items: {
            type: "object",
            properties: { x: { type: "number" }, y: { type: "number" } },
            required: ["x", "y"],
          },
        },
        button: { type: "string", description: "left (default), right, or middle" },
        modifiers: { description: "ctrl/alt/shift/win held while dragging" },
        cx: { type: "number", description: "Arc/circle center x in the screenshot" },
        cy: { type: "number", description: "Arc/circle center y in the screenshot" },
        r: { type: "number", description: "Arc/circle radius in screenshot pixels" },
        startDeg: { type: "number", description: "Arc start degrees, 0=right, 90=down. Default 0." },
        endDeg: { type: "number", description: "Arc end degrees. Default 360 (full circle)." },
      },
    },
  },
  {
    name: "move",
    description: "Move the pointer without clicking. Optional waitMs to hover after arriving.",
    parameters: {
      type: "object",
      properties: {
        x: { type: "number" },
        y: { type: "number" },
        waitMs: { type: "integer", description: "Dwell after moving, 0–4000. Default 0." },
      },
      required: ["x", "y"],
    },
  },
  {
    name: "hover",
    description: "Move to screenshot-image coordinates and stay there so tooltips/menus can open. Default waitMs 400.",
    parameters: {
      type: "object",
      properties: {
        x: { type: "number" },
        y: { type: "number" },
        waitMs: { type: "integer", description: "Dwell ms, 50–4000. Default 400." },
      },
      required: ["x", "y"],
    },
  },
  {
    name: "scroll",
    description: "Focus the window under x,y then scroll. dy>0 scrolls down. dx is horizontal.",
    parameters: {
      type: "object",
      properties: {
        x: { type: "number" },
        y: { type: "number" },
        dy: { type: "number" },
        dx: { type: "number" },
      },
      required: ["x", "y"],
    },
  },
  {
    name: "wait",
    description: "Pause so UI animations/tooltips can finish. Does not move the mouse.",
    parameters: {
      type: "object",
      properties: {
        ms: { type: "integer", description: "50–4000. Default 400." },
      },
    },
  },
  {
    name: "type",
    description: "Type unicode text into the focused window. Click a field first.",
    parameters: {
      type: "object",
      properties: { text: { type: "string" } },
      required: ["text"],
    },
  },
  {
    name: "key",
    description: "Press a key or chord: enter, tab, esc, win, win+e, ctrl+c, alt+tab, f5.",
    parameters: {
      type: "object",
      properties: { keys: { type: "string" } },
      required: ["keys"],
    },
  },
];

function write(msg: unknown) {
  Deno.stdout.writeSync(new TextEncoder().encode(JSON.stringify(msg) + "\n"));
}

let leftover = "";
async function nextMessage(): Promise<Record<string, unknown> | null> {
  const decoder = new TextDecoder();
  while (true) {
    const idx = leftover.indexOf("\n");
    if (idx >= 0) {
      const line = leftover.slice(0, idx).trim();
      leftover = leftover.slice(idx + 1);
      if (!line) continue;
      return JSON.parse(line);
    }
    const buf = new Uint8Array(8192);
    const n = await Deno.stdin.read(buf);
    if (n === null) {
      if (!leftover.trim()) return null;
      const line = leftover;
      leftover = "";
      return JSON.parse(line);
    }
    leftover += decoder.decode(buf.subarray(0, n));
  }
}

write({ id: 0, event: "ready", pluginId: "computer-use" });

while (true) {
  const msg = await nextMessage();
  if (!msg) break;
  const id = msg.id;
  try {
    const method = String(msg.method ?? "");
    if (method === "describe") {
      write({ id, result: { tools, hooks: [] } });
    } else if (method === "tool") {
      write({
        id,
        result: { content: "computer actions are implemented by Anya, not the Deno host" },
      });
    } else {
      write({ id, error: "unknown method" });
    }
  } catch (err) {
    write({ id, error: String(err) });
  }
}
