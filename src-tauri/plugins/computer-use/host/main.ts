/** Agent tools for computer use. Actions run in Anya Rust via GhostSession (`computer` permission). */

const tools = [
  {
    name: "see",
    description:
      "List UI controls (JSON: name/role/enabled/center). Default mode=fast; with window= auto-upgrades to full. Prefer mode=text to read. Always pass window= after launch/anchor.",
    parameters: {
      type: "object",
      properties: {
        window: { type: "string", description: "Window title substring; uses session anchor if omitted" },
        mode: { type: "string", description: "fast (default), full, or text — scoped window= upgrades fast→full" },
        name: { type: "string", description: "With mode=text: control name to read" },
        name_filter: { type: "string", description: "Substring filter on control names" },
        limit: { type: "integer", description: "Max rows. Default 150" },
      },
    },
  },
  {
    name: "find",
    description:
      "Locate one control by name/role (background UIA) or description (ground/OCR/VLM). Returns center/rect/source. Chromium windows with DevTools auto-use CDP.",
    parameters: {
      type: "object",
      properties: {
        window: { type: "string" },
        name: { type: "string" },
        role: { type: "string" },
        description: { type: "string", description: "Natural-language / vision target" },
        index: { type: "integer" },
        mode: {
          type: "string",
          description: "instant (default) | deliberate | instant_only — grounding cascade",
        },
      },
    },
  },
  {
    name: "act",
    description:
      "Click or type by name/role (background; CDP auto if Chromium+devtools). description= uses ground then real click. Returns verified + dispatch/route.",
    parameters: {
      type: "object",
      properties: {
        action: {
          type: "string",
          description: "click (default), type, double_click, right_click",
        },
        name: { type: "string" },
        role: { type: "string", description: "button, edit, …" },
        description: { type: "string", description: "Vision fallback description" },
        text_input: { type: "string", description: "Required when action=type" },
        text: { type: "string" },
        window: { type: "string", description: "Target window; anchors session" },
        index: { type: "integer", description: "Nth name/role match (0-based)" },
        mode: { type: "string", description: "For description=: instant|deliberate|instant_only" },
      },
    },
  },
  {
    name: "wait",
    description:
      "Wait for element/value/idle/event. Prefer this over sleeping. for=ms only as last resort.",
    parameters: {
      type: "object",
      properties: {
        for: {
          type: "string",
          description: "element | value | idle | event | ms",
        },
        name: { type: "string" },
        role: { type: "string" },
        window: { type: "string" },
        appears: { type: "boolean", description: "for=element: wait until appears (default true) or disappears" },
        pred: { type: "string", description: "for=value: equals (default)" },
        expected: { type: "string" },
        timeout_ms: { type: "integer", description: "Default 10000" },
        ms: { type: "integer", description: "for=ms only" },
        since_seq: { type: "integer", description: "for=event" },
      },
    },
  },
  {
    name: "assert",
    description: "Assert a control exists or its value equals expected.",
    parameters: {
      type: "object",
      properties: {
        kind: { type: "string", description: "exists (default) | value" },
        name: { type: "string" },
        role: { type: "string" },
        expected: { type: "string", description: "kind=value" },
      },
    },
  },
  {
    name: "window",
    description:
      "list / launch / anchor / focus / state / clear_anchor / policy. Prefer launch then act. focus anchors only (raise=true to steal focus). policy=background|prefer_background|foreground for canvas sessions.",
    parameters: {
      type: "object",
      properties: {
        op: {
          type: "string",
          description: "list | launch | focus | anchor | state | clear_anchor | policy",
        },
        name: { type: "string", description: "Title substring for focus/anchor/state; or policy name for op=policy" },
        exe: { type: "string", description: "Executable, path, or URI for launch" },
        target: { type: "string", description: "Alias of exe" },
        args: { type: "string", description: "Launch arguments" },
        raise: { type: "boolean", description: "focus only: briefly raise window (default false — anchor is enough)" },
        policy: {
          type: "string",
          description: "op=policy: background | prefer_background | foreground",
        },
        state: {
          type: "string",
          description: "maximize | minimize | restore | close",
        },
      },
    },
  },
  {
    name: "key",
    description:
      "Press a key or chord. With window=: single keys + Ctrl+C/X/V/A/Z stay background; other chords (ctrl+s, alt+f4) briefly raise the target. Always pass window=.",
    parameters: {
      type: "object",
      properties: {
        keys: { type: "string" },
        key: { type: "string" },
        window: { type: "string" },
      },
      required: ["keys"],
    },
  },
  {
    name: "hotkey",
    description: "Alias of key for chords like ctrl+shift+esc.",
    parameters: {
      type: "object",
      properties: {
        keys: { type: "string" },
        window: { type: "string" },
      },
      required: ["keys"],
    },
  },
  {
    name: "scroll",
    description:
      "Scroll in the anchored/window= app (background). Prefer direction+amount; optional until_name/until_role. dy/dx also accepted.",
    parameters: {
      type: "object",
      properties: {
        window: { type: "string" },
        x: { type: "number" },
        y: { type: "number" },
        direction: { type: "string", description: "up|down|left|right" },
        amount: { type: "integer" },
        dy: { type: "number" },
        dx: { type: "number" },
        until_name: { type: "string" },
        until_role: { type: "string" },
        max_scrolls: { type: "integer" },
      },
    },
  },
  {
    name: "drag",
    description: "Drag with real mouse on the anchored/window= app (Paint brush strokes). Auto-raises target then restores background. from_x/y → to_x/y.",
    parameters: {
      type: "object",
      properties: {
        window: { type: "string", description: "Target window; required unless session is already anchored" },
        from_x: { type: "number" },
        from_y: { type: "number" },
        to_x: { type: "number" },
        to_y: { type: "number" },
        x: { type: "number" },
        y: { type: "number" },
        x2: { type: "number" },
        y2: { type: "number" },
      },
    },
  },
  {
    name: "clipboard",
    description: "Clipboard get / set / paste.",
    parameters: {
      type: "object",
      properties: {
        op: { type: "string", description: "get | set | paste" },
        text: { type: "string" },
      },
    },
  },
  {
    name: "screenshot",
    description:
      "Fallback capture + short control list. Prefer see/act. scope=foreground (default) or desktop; optional title.",
    parameters: {
      type: "object",
      properties: {
        scope: { type: "string" },
        title: { type: "string" },
        window: { type: "string" },
      },
    },
  },
  {
    name: "screen_info",
    description:
      "Window count + Ghost focus policy/lock. Default policy is background (MCP parity). locked=false means Anya unlocked Ghost — never setx GHOST_FOCUS_LOCK.",
    parameters: { type: "object", properties: {} },
  },
  {
    name: "browser",
    description:
      "CDP browser lifecycle: launch | attach | tabs | close. Then drive pages with tab tools — not pixel clicks.",
    parameters: {
      type: "object",
      properties: {
        op: { type: "string", description: "launch | attach | tabs | close" },
        id: { type: "string", description: "Browser id. Default default" },
        mode: { type: "string", description: "windowed | headless" },
        which: { type: "string", description: "chrome | edge | brave | comet" },
        port: { type: "integer", description: "attach: remote-debugging-port" },
      },
    },
  },
  {
    name: "tab",
    description:
      "Browser tab: open | close | find | navigate | click | type | text | eval | wait (CSS selectors via CDP).",
    parameters: {
      type: "object",
      properties: {
        op: { type: "string" },
        browser: { type: "string", description: "Browser id. Default default" },
        url: { type: "string" },
        target_id: { type: "string" },
        needle: { type: "string", description: "find: URL/title substring" },
        selector: { type: "string" },
        text: { type: "string" },
        clear: { type: "boolean" },
        expression: { type: "string", description: "eval JS" },
        js: { type: "string" },
        timeout_ms: { type: "integer" },
      },
    },
  },
  {
    name: "playbook",
    description:
      "Learned procedures (Agent S2 + EchoPath style). lookup before exploring; save after novel success; fail when a step's verified is false; success to boost confidence. Steps are semantic (name/role/keys), not pixel coords.",
    parameters: {
      type: "object",
      properties: {
        op: {
          type: "string",
          description: "list | lookup | save | fail | success | delete",
        },
        id: { type: "string", description: "Playbook id for get/fail/success/delete/save-patch" },
        query: { type: "string", description: "lookup: natural-language intent" },
        intent: { type: "string", description: "save: short goal label; also accepted for lookup" },
        app: { type: "string", description: "Window/app title hint" },
        exe: { type: "string", description: "Executable name e.g. winword.exe" },
        version: { type: "string", description: "App version when known" },
        success_assert: { type: "string", description: "What proves success" },
        steps: {
          type: "array",
          description:
            "save: [{kind, action?, name?, role?, keys?, text?, window?, precondition?, note?}]",
          items: { type: "object" },
        },
        step: { type: "integer", description: "fail: 1-based step index that broke" },
        reason: { type: "string", description: "fail: short reason" },
      },
    },
  },
  // Aliases for older prompts
  {
    name: "list_windows",
    description: "Alias of window op=list.",
    parameters: { type: "object", properties: {} },
  },
  {
    name: "focus_window",
    description:
      "Alias of window op=focus. Best-effort raise; if it fails, window is still anchored — use act with window=title. Do not chase GHOST_FOCUS_LOCK.",
    parameters: {
      type: "object",
      properties: {
        title: { type: "string" },
        name: { type: "string" },
      },
    },
  },
  {
    name: "find_control",
    description: "Alias of see with name_filter.",
    parameters: {
      type: "object",
      properties: {
        name: { type: "string" },
        windowTitle: { type: "string" },
        window: { type: "string" },
      },
    },
  },
  {
    name: "click_control",
    description: "Alias of act action=click.",
    parameters: {
      type: "object",
      properties: {
        name: { type: "string" },
        role: { type: "string" },
        index: { type: "integer" },
        window: { type: "string" },
      },
    },
  },
  {
    name: "set_value",
    description: "Alias of act action=type with value→text_input.",
    parameters: {
      type: "object",
      properties: {
        name: { type: "string" },
        role: { type: "string" },
        value: { type: "string" },
        window: { type: "string" },
      },
      required: ["value"],
    },
  },
  {
    name: "launch",
    description: "Alias of window op=launch.",
    parameters: {
      type: "object",
      properties: {
        target: { type: "string" },
        exe: { type: "string" },
        args: { type: "string" },
      },
      required: ["target"],
    },
  },
  {
    name: "click",
    description:
      "Pixel click. Default posts to window= (UI). For Paint canvas marks use real=true (or prefer drag for strokes).",
    parameters: {
      type: "object",
      properties: {
        x: { type: "number" },
        y: { type: "number" },
        window: { type: "string" },
        real: { type: "boolean", description: "true = real mouse (canvas); default false = posted" },
      },
      required: ["x", "y"],
    },
  },
  {
    name: "type",
    description: "Alias of act action=type into a named control when possible.",
    parameters: {
      type: "object",
      properties: {
        text: { type: "string" },
        name: { type: "string" },
        window: { type: "string" },
      },
      required: ["text"],
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
        result: { content: "computer actions are implemented by Anya (Ghost engine), not the Deno host" },
      });
    } else {
      write({ id, error: "unknown method" });
    }
  } catch (err) {
    write({ id, error: String(err) });
  }
}
