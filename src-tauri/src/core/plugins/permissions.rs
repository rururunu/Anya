//! Declared plugin permission ids.

pub const KNOWN_PERMISSIONS: &[&str] = &[
    "storage",
    "ask_anya",
    "pty",
    "run",
    "fs.workspace",
    "fs.pick",
    "net",
    "agent.tools",
    "agent.hooks",
    "agent.prompt",
    "ui.workbench",
    "computer",
];

pub fn is_known_permission(perm: &str) -> bool {
    KNOWN_PERMISSIONS.contains(&perm)
}

pub fn permission_label(perm: &str) -> &'static str {
    match perm {
        "storage" => "Save small key-value data inside the plugin folder",
        "ask_anya" => {
            "Call Anya's agent (ctx.agent.run on a plugin session, or send into the current chat)"
        }
        "pty" => "Open a real terminal (ConPTY) on this computer",
        "run" => "Start programs from the plugin host (Deno --allow-run)",
        "fs.workspace" => "Read and write the current workspace",
        "fs.pick" => "Open a native file dialog (ctx.fs.pick / AnyaPlugin.pick)",
        "net" => "Make network requests from the plugin host",
        "agent.tools" => "Register tools the chat agent can call",
        "agent.hooks" => "Run hooks during an agent turn",
        "agent.prompt" => "Append instructions to the agent system prompt",
        "ui.workbench" => "Load UI into the workbench (same page as Anya, including stores)",
        "computer" => "See the desktop (screenshot) and control mouse/keyboard",
        _ => "Custom permission",
    }
}
