//! Default files written by `manage_plugin` create.

use std::fs;
use std::path::Path;

use super::manifest::{
    PluginAgentContrib, PluginContributes, PluginHostSpec, PluginManifest, PluginUiSpec,
    PluginWindowSpec,
};
use crate::core::tools::error::ToolError;

pub fn default_manifest(id: &str, name: &str, description: &str) -> PluginManifest {
    PluginManifest {
        id: id.to_string(),
        name: name.to_string(),
        version: "0.1.0".into(),
        api_version: "1.0".into(),
        description: description.to_string(),
        icon: Some("ui/icon.svg".into()),
        about: None,
        entry: "ui/index.html".into(),
        ui: PluginUiSpec {
            entry: "ui/activate.js".into(),
        },
        host: PluginHostSpec {
            entry: Some("host/main.ts".into()),
        },
        contributes: PluginContributes {
            sidebar: false,
            view: false,
            composer: false,
            window: true,
            agent: PluginAgentContrib::default(),
        },
        permissions: vec!["storage".into(), "ask_anya".into(), "ui.workbench".into()],
        capabilities: Vec::new(),
        window: PluginWindowSpec::default(),
        role: String::new(),
    }
}

pub fn write_scaffold(dir: &Path, manifest: &PluginManifest) -> Result<(), ToolError> {
    fs::create_dir_all(dir.join("ui").join("src")).map_err(|e| ToolError::new(e.to_string()))?;
    fs::create_dir_all(dir.join("host")).map_err(|e| ToolError::new(e.to_string()))?;
    let json = serde_json::to_string_pretty(manifest).map_err(|e| ToolError::new(e.to_string()))?;
    fs::write(dir.join("plugin.json"), json).map_err(|e| ToolError::new(e.to_string()))?;
    fs::write(dir.join("ui").join("index.html"), scaffold_html(manifest))
        .map_err(|e| ToolError::new(e.to_string()))?;
    let activate = scaffold_activate(manifest);
    fs::write(dir.join("ui").join("src").join("activate.js"), &activate)
        .map_err(|e| ToolError::new(e.to_string()))?;
    fs::write(dir.join("ui").join("activate.js"), &activate)
        .map_err(|e| ToolError::new(e.to_string()))?;
    fs::write(dir.join("host").join("main.ts"), scaffold_host(manifest))
        .map_err(|e| ToolError::new(e.to_string()))?;
    fs::write(
        dir.join("ui").join("icon.svg"),
        icon_svg(&manifest.id, &manifest.name),
    )
    .map_err(|e| ToolError::new(e.to_string()))?;
    fs::write(
        dir.join("ui").join("src").join("anya-plugin.d.ts"),
        scaffold_dts(),
    )
    .map_err(|e| ToolError::new(e.to_string()))?;
    Ok(())
}

/// Letter-mark fallback. Agents should overwrite `ui/icon.svg` with a real mark.
pub fn icon_svg(id: &str, name: &str) -> String {
    let letter = name
        .chars()
        .find(|c| c.is_alphanumeric())
        .or_else(|| id.chars().find(|c| c.is_alphanumeric()))
        .unwrap_or('P')
        .to_uppercase()
        .next()
        .unwrap_or('P');
    let colors = [
        "#5b7cfa", "#5dba83", "#d4a017", "#e85d5d", "#6cb6ff", "#d2a8ff", "#ff7eb6",
    ];
    let mut hash: u32 = 2166136261;
    for b in id.as_bytes() {
        hash = hash.wrapping_mul(16777619) ^ u32::from(*b);
    }
    let fill = colors[(hash as usize) % colors.len()];
    format!(
        concat!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 48 48\">",
            "<rect width=\"48\" height=\"48\" rx=\"12\" fill=\"{fill}\"/>",
            "<text x=\"24\" y=\"32\" font-size=\"22\" text-anchor=\"middle\" fill=\"#fff\" ",
            "font-family=\"system-ui,sans-serif\">{letter}</text></svg>"
        ),
        fill = fill,
        letter = html_escape(&letter.to_string()),
    )
}

/// After `create`/`put_file`: keep `plugin.json` `icon` + file so list/nav never fall back to Puzzle.
pub fn ensure_plugin_icon(id: &str) -> Result<(), ToolError> {
    let dir = super::manifest::plugin_dir(id)?;
    let manifest_path = dir.join("plugin.json");
    let raw = fs::read_to_string(&manifest_path)
        .map_err(|e| ToolError::new(format!("read plugin.json: {e}")))?;
    let mut value: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| ToolError::new(format!("invalid plugin.json: {e}")))?;
    let declared = value
        .get("icon")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.replace('\\', "/"));
    let icon_rel = declared.clone().unwrap_or_else(|| "ui/icon.svg".into());
    let icon_file = dir.join(&icon_rel);
    if !icon_file.is_file() {
        if let Some(parent) = icon_file.parent() {
            fs::create_dir_all(parent).map_err(|e| ToolError::new(e.to_string()))?;
        }
        let name = value.get("name").and_then(|v| v.as_str()).unwrap_or(id);
        fs::write(&icon_file, icon_svg(id, name)).map_err(|e| ToolError::new(e.to_string()))?;
    }
    if declared.is_none() {
        value["icon"] = serde_json::Value::String(icon_rel);
        let pretty =
            serde_json::to_string_pretty(&value).map_err(|e| ToolError::new(e.to_string()))?;
        fs::write(&manifest_path, pretty).map_err(|e| ToolError::new(e.to_string()))?;
    }
    Ok(())
}

/// Minimal editor type hints for `activate.js`. Not consumed at build time -- add a
/// `/// <reference path="./anya-plugin.d.ts" />` comment at the top of activate.js
/// (or a jsconfig.json) to get IntelliSense in editors that support it.
fn scaffold_dts() -> String {
    r#"export type PluginMount = (el: HTMLElement) => void | (() => void);

export interface AnchorDescriptor {
  id: string;
  surfaces: string[];
  multiplicity: "exclusive" | "stack" | "user-choice";
}

export interface PluginActivateContext {
  pluginId: string;
  slots: {
    list(): AnchorDescriptor[];
    mount(anchorId: string, view: { id: string; title?: string; icon?: string; mount: PluginMount; surfaces?: Array<"nav" | "header" | "views"> }): void;
    unmount(anchorId: string, id: string): void;
  };
  conversation: {
    mount(el: HTMLElement): () => void;
    unmount(): void;
    send(text: string): Promise<void>;
    run(prompt: string, options?: { sessionId?: string }): Promise<{ sessionId: string }>;
    sessionId(): string;
    addMaterials(item: { id: string; mount: PluginMount }): void;
    removeMaterials(id: string): void;
  };
  agent: {
    mount(el: HTMLElement): () => void;
    unmount(): void;
    send(text: string): Promise<void>;
    run(prompt: string, options?: { sessionId?: string }): Promise<{ sessionId: string }>;
    sessionId(): string;
  };
  workspace: {
    openInTerminal(id?: string): Promise<void>;
  };
  assets: {
    register(key: string, asset: { kind: "image" | "video" | "lottie"; source: string }): void;
    unregister(key: string): void;
  };
  bus: {
    publish(topic: string, payload: unknown): void;
    subscribe(topic: string, fn: (payload: unknown) => void): () => void;
  };
  i18n: {
    t(key: string, fallback: string, dict?: Record<string, string>): string;
  };
  sidebar: {
    addTab(tab: { id: string; title: string; icon?: string; mount?: PluginMount; onClick?: () => void | Promise<void>; surfaces?: Array<"nav" | "header" | "views"> }): void;
    removeTab(id: string): void;
  };
  host: {
    rpc(method: string, params?: Record<string, unknown>): Promise<unknown>;
    on(event: string, fn: (payload: Record<string, unknown>) => void): () => void;
  };
  home: {
    setSettingsView(mount: PluginMount): void;
    clearSettingsView(): void;
  };
  fs: {
    pick(options?: { multiple?: boolean; directory?: boolean; filters?: Array<{ name: string; extensions: string[] }> }): Promise<Array<{ path: string; name: string; url: string }> | null>;
  };
  onDeactivate(fn: () => void): void;
}
"#
    .into()
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn scaffold_html(manifest: &PluginManifest) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{name}</title>
  <style>
    :root {{ color-scheme: dark; }}
    body {{
      margin: 0; min-height: 100vh; font: 15px/1.5 system-ui, sans-serif;
      background: #12141a; color: #e8eaed; display: grid; place-items: center;
    }}
    main {{ max-width: 36rem; padding: 2rem; }}
    h1 {{ font-size: 1.35rem; margin: 0 0 .5rem; }}
    p {{ color: #9aa3b2; margin: 0 0 1rem; }}
    button {{
      background: #5b7cfa; color: #fff; border: 0; border-radius: 8px;
      padding: .55rem 1rem; cursor: pointer; font: inherit;
    }}
  </style>
</head>
<body>
  <main>
    <h1>{name}</h1>
    <p>{description}</p>
    <p>Window UI for this plugin. Workbench UI is bundled from <code>ui/src/activate.js</code> on Enable.</p>
    <button type="button" id="ping">Save a local record</button>
  </main>
  <script>
    document.getElementById("ping").onclick = async () => {{
      if (!window.AnyaPlugin) return;
      await window.AnyaPlugin.storageSet("hello", new Date().toISOString());
      const v = await window.AnyaPlugin.storageGet("hello");
      alert("storage ok: " + v);
    }};
  </script>
</body>
</html>
"#,
        name = html_escape(&manifest.name),
        description = html_escape(if manifest.description.is_empty() {
            "User plugin"
        } else {
            &manifest.description
        }),
    )
}

fn scaffold_activate(manifest: &PluginManifest) -> String {
    format!(
        r#"/** Bundled to ui/.anya/activate.js on Enable. Pure npm JS imports (lodash, zod) are OK; disk/PTY use ctx.host.rpc. */
export async function activate(ctx) {{
  ctx.onDeactivate(() => {{}});
  console.info("plugin {id} activated");
}}

export function deactivate() {{}}
"#,
        id = manifest.id
    )
}

fn scaffold_host(manifest: &PluginManifest) -> String {
    format!(
        r#"const PLUGIN_ID = "{id}";

const tools = [];
const hooks = [];

async function handleTool(_name: string, _args: Record<string, unknown>) {{
  return {{ content: "no tools registered" }};
}}

async function handleHook(hook: string, payload: Record<string, unknown>) {{
  return payload;
}}

async function readLine(): Promise<string | null> {{
  const buf = new Uint8Array(8 * 1024);
  const n = await Deno.stdin.read(buf);
  if (n === null) return null;
  return new TextDecoder().decode(buf.subarray(0, n));
}}

let leftover = "";
async function nextMessage(): Promise<any | null> {{
  while (true) {{
    const idx = leftover.indexOf("\n");
    if (idx >= 0) {{
      const line = leftover.slice(0, idx).trim();
      leftover = leftover.slice(idx + 1);
      if (!line) continue;
      return JSON.parse(line);
    }}
    const chunk = await readLine();
    if (chunk === null) {{
      if (!leftover.trim()) return null;
      const line = leftover;
      leftover = "";
      return JSON.parse(line);
    }}
    leftover += chunk;
  }}
}}

function write(msg: unknown) {{
  Deno.stdout.writeSync(new TextEncoder().encode(JSON.stringify(msg) + "\n"));
}}

write({{ id: 0, event: "ready", pluginId: PLUGIN_ID }});

while (true) {{
  const msg = await nextMessage();
  if (!msg) break;
  const id = msg.id;
  try {{
    if (msg.method === "describe") {{
      write({{ id, result: {{ tools, hooks }} }});
    }} else if (msg.method === "tool") {{
      const name = String(msg.params?.name ?? "");
      const args = msg.params?.args ?? {{}};
      const result = await handleTool(name, args);
      write({{ id, result }});
    }} else if (msg.method === "hook") {{
      const hook = String(msg.params?.hook ?? "");
      const payload = msg.params?.payload ?? {{}};
      const result = await handleHook(hook, payload);
      write({{ id, result }});
    }} else {{
      write({{ id, error: "unknown method" }});
    }}
  }} catch (err) {{
    write({{ id, error: String(err) }});
  }}
}}
"#,
        id = manifest.id
    )
}

#[cfg(test)]
mod tests {
    use super::icon_svg;

    #[test]
    fn icon_svg_uses_a_letter_and_fill() {
        let svg = icon_svg("bid-writer", "Bid writer");
        assert!(svg.contains("viewBox=\"0 0 48 48\""));
        assert!(svg.contains(">B</text>"));
        assert!(svg.contains("fill=\"#"));
    }
}
