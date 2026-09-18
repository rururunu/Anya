# OpenCLI usage (site adapters)

Deterministic commands beat generic browser clicking for covered sites.

## Discover

- `list` — registered commands (JSON)
- `run` with `argv`: `["<site>","<command>", …]`
- Prefer `--json` when the adapter supports it

## Examples

```
run argv=["hackernews","top","--limit","5"]
run argv=["bilibili","hot","--limit","5","--json"]
run command="zhihu hot --limit 5"
```

## When to use what

| Need | Tool |
| --- | --- |
| Known site feed / search / status | `run` adapter |
| Logged-in page, no adapter | `browser` session |
| Desktop app / Paint / Explorer | Computer Use (Ghost), not OpenCLI |
| Setup broken | `doctor` |

## Authoring (optional)

Upstream can generate adapters (`opencli browser recon …`). In Anya, after a successful ad-hoc browser flow, summarize the stable path for the user; do not invent adapter files unless asked.

Upstream: https://github.com/jackwener/OpenCLI
