# OpenCLI browser (Browser Bridge)

Drive **your logged-in Chrome** via OpenCLI — not Ghost CDP.

## Setup

1. Install CLI: `npm i -g @jackwener/opencli` (Node ≥ 20)
2. Install the **OpenCLI** Chrome extension (Chrome Web Store or release zip)
3. `plugin_opencli__doctor` until green
4. Optional: `opencli profile list` / rename when multiple Chrome profiles

Do **not** use Computer Use `browser`/`tab` on the same Chrome profile in the same turn.

## Session model

Every browser call needs a **session** name (default `work`):

1. `browser` `{session:"work", op:"open", url:"https://…"}` → note target id if returned
2. `browser` `{session:"work", op:"state"}` — structured DOM snapshot (prefer over screenshots)
3. Interact: `click` / `type` / `fill` / `select` / `keys` with `selector`
4. Read: `get` / `find` / `extract` / `eval`
5. Tabs: `op:"tab", tab_op:"list|new|select|close"`
6. Finish: `op:"close"` when done with the session

Pass `tab` to target a specific tab id (`--tab`).

## Speed

- Prefer **site adapters** (`run`) when the site is listed — zero exploration.
- For unknown pages: `state` once, then batch clicks/fills.
- Use `wait` before assuming the UI settled.
- Screenshot only when DOM state is insufficient.

## Failures

- Exit `69` → Browser Bridge down (extension / daemon)
- Exit `77` → auth required (log in in Chrome, retry)
- Exit `75` → timeout — raise `timeout_secs` or wait/retry
