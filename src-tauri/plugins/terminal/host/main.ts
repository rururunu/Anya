/** Agent `run` tool (needs `run` grant). Interactive terminal in the UI uses host `pty.*`, not this process. */

const tools = [
  {
    name: "run",
    description: "Run a shell command and return combined output (cwd optional).",
    parameters: {
      type: "object",
      properties: {
        command: { type: "string", description: "Command to run" },
        cwd: { type: "string", description: "Working directory" },
      },
      required: ["command"],
    },
  },
];
const hooks = ["on_turn_end"];

async function handleTool(name: string, args: Record<string, unknown>) {
  if (name !== "run") return { content: `unknown tool ${name}` };
  const command = String(args.command ?? "").trim();
  if (!command) return { content: "command is required" };
  const cwd = typeof args.cwd === "string" && args.cwd.trim() ? args.cwd : undefined;
  const shell = Deno.build.os === "windows" ? "powershell.exe" : "/bin/bash";
  const shellArgs = Deno.build.os === "windows" ? ["-NoProfile", "-Command", command] : ["-lc", command];
  const proc = new Deno.Command(shell, {
    args: shellArgs,
    cwd,
    stdout: "piped",
    stderr: "piped",
  });
  const out = await proc.output();
  const decoder = new TextDecoder();
  const text = `${decoder.decode(out.stdout)}${decoder.decode(out.stderr)}`.trim();
  const code = out.code ?? 1;
  return { content: `exit ${code}\n${text || "(no output)"}` };
}

async function handleHook(_hook: string, payload: Record<string, unknown>) {
  return payload;
}

let leftover = "";
async function nextMessage(): Promise<any | null> {
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

function write(msg: unknown) {
  Deno.stdout.writeSync(new TextEncoder().encode(JSON.stringify(msg) + "\n"));
}

write({ id: 0, event: "ready", pluginId: "terminal" });

while (true) {
  const msg = await nextMessage();
  if (!msg) break;
  const id = msg.id;
  try {
    if (msg.method === "describe") {
      write({ id, result: { tools, hooks } });
    } else if (msg.method === "tool") {
      const result = await handleTool(String(msg.params?.name ?? ""), msg.params?.args ?? {});
      write({ id, result });
    } else if (msg.method === "hook") {
      const result = await handleHook(String(msg.params?.hook ?? ""), msg.params?.payload ?? {});
      write({ id, result });
    } else {
      write({ id, error: "unknown method" });
    }
  } catch (err) {
    write({ id, error: String(err) });
  }
}
