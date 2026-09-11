# Slot Badge Demo

Reference plugin proving the generic primitives are enough to build a small,
opinionated feature without any Anya source change:

- `ctx.slots` — a badge in the composer accessory row ("cat ears" style spot)
- `ctx.assets` — overrides the mascot idle resource key with an inline SVG
- `ctx.bus` — publishes a heartbeat other plugins can subscribe to
- `ctx.i18n` — localizes the badge label
- `ctx.home.setSettingsView` — a settings tab on this plugin's own home page

Copy this folder into the user plugins directory and rename `id` to try it.
