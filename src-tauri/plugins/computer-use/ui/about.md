# Computer Use

Agent plugin. After you **Enable** it (grant `computer`), ask Anya in chat to look at the screen or click/type. There is no sidebar.

The agent can:

1. **Launch** an app, file, or URI (`launch`) — faster than clicking Start
2. List and focus windows (`list_windows`, `focus_window`)
3. See a **screenshot + UI Automation control list** together, then Invoke/`click_control` / `set_value` by name, AutomationId, or index
4. Click, **drag**, hover, scroll, or type when there is no named control (canvas, snip region)
5. Type text and press keys (`enter`, `tab`, `ctrl+c`, `win+e`, …)

Prefer launch → keyboard → named UIA controls over guessing pixels. Do not screenshot after every click.

When enabled, a **Windows apps playbook** is attached to the agent prompt.

Windows only. Mutating actions go through Anya's existing tool-approval picker. For speed, choose **Allow for session**. The plugin ships **disabled**.

Do not use this on a machine you do not control.
