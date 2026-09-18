/** Agent tools for OpenCLI (executed in Anya Rust — host only describes schemas). */

const tools = [
  {
    name: "doctor",
    description:
      "Run `opencli doctor`. Check Node CLI + Browser Bridge extension. Call first when setup is unclear.",
    parameters: {
      type: "object",
      properties: {
        timeout_secs: { type: "integer", description: "Default 90, max 180" },
      },
    },
  },
  {
    name: "list",
    description: "List registered OpenCLI commands / site adapters (`opencli list --json`).",
    parameters: {
      type: "object",
      properties: {
        json: { type: "boolean", description: "Default true" },
        timeout_secs: { type: "integer" },
      },
    },
  },
  {
    name: "run",
    description:
      "Run a site adapter or OpenCLI command. Prefer argv[]. Examples: bilibili hot --limit 5 --json; hackernews top --limit 5.",
    parameters: {
      type: "object",
      properties: {
        argv: {
          type: "array",
          items: { type: "string" },
          description: 'e.g. ["bilibili","hot","--limit","5","--json"]',
        },
        command: {
          type: "string",
          description: "Alternative to argv: space-separated tokens (quoted values supported)",
        },
        timeout_secs: { type: "integer" },
      },
    },
  },
  {
    name: "browser",
    description:
      "Drive logged-in Chrome via OpenCLI Browser Bridge. session is required (e.g. work). Ops: open, state, click, type, fill, select, keys, wait, get, find, extract, scroll, back, eval, screenshot, network, tab, close.",
    parameters: {
      type: "object",
      properties: {
        session: { type: "string", description: "Session name, default work" },
        op: { type: "string", description: "open | state | click | type | fill | … | tab | close" },
        url: { type: "string", description: "For open / navigate" },
        selector: { type: "string", description: "CSS / bridge target for click/type/fill/…" },
        target: { type: "string", description: "Alias of selector" },
        text: { type: "string", description: "For type/fill/keys" },
        value: { type: "string", description: "Alias of text" },
        tab: { type: "string", description: "Pass --tab <targetId>" },
        tab_op: {
          type: "string",
          description: "When op=tab: list | new | select | close",
        },
        sub: { type: "string", description: "Alias of tab_op" },
        extra: {
          type: "array",
          items: { type: "string" },
          description: "Extra CLI args appended as-is",
        },
        timeout_secs: { type: "integer" },
      },
      required: ["op"],
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

write({ id: 0, event: "ready", pluginId: "opencli" });

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
        result: {
          content: "opencli tools are implemented by Anya (Rust runner), not the Deno host",
        },
      });
    } else {
      write({ id, error: "unknown method" });
    }
  } catch (err) {
    write({ id, error: String(err) });
  }
}
