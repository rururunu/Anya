# Theme editor

Open **Settings → Appearance → Custom Themes**. Choose **New theme** or **Edit** on a theme card.

Click a region in the miniature preview, or choose a region tab, to edit its colors and available shape settings. The editor covers the window, sidebar, conversation header/body, user and assistant messages, composer, cards, code blocks, buttons and icons. Message regions include radius, shadow and padding controls. Global controls include UI/reply font size, message spacing, and wallpaper path, opacity, blur and fit.

An empty field uses the default for the base mode. Its placeholder shows the effective value; the reset arrow removes an individual override. **Reset region** clears that region's overrides. Undo/redo tracks draft changes. Light/dark selects fallback values and does not restrict explicit region colors. Explicit colors are preserved even when contrast is low; the advisory offers an optional text contrast improvement for supported solid colors.

The miniature preview updates immediately. **Preview in app** temporarily applies style tokens in the current window without writing settings or the startup cache. Wallpaper is previewed in the miniature; native window effects are not changed by this toggle. Closing or cancelling removes the temporary stylesheet. **Save & apply** persists the theme; **Save as new** generates a new ID and preserves the original. Save failures leave the draft open for retry.

The **JSON** tab edits the token map, not a stylesheet. Apply it to preview before switching back to visual controls. Known fields validate CSS values; malformed JSON and declaration-breaking values are rejected. Existing theme metadata, unknown valid tokens and wallpaper configuration are retained. Wallpaper selections store file paths, not image bytes. Theme import/export remains available in the theme library.

Implementation: `src/services/theme/editor.ts` defines regions; `ThemeService.ts` fills missing tokens without changing explicit choices; `ThemeEditor.vue` manages drafts and preview; `settingStore.saveCustomTheme` persists and applies a saved theme. The preview is illustrative: native window material, syntax highlighting details and every interaction state are not simulated.
