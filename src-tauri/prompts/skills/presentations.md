---
name: presentations
description: Read, create and edit saved PowerPoint PPTX files using JavaScript, editable slides and templates; render and inspect slides before delivery.
---

# PowerPoint presentations

## Workflow
1. Read runtime `API.md`, then use bundled Deno and exported `pptxgen`. No global Node/Python/npm installation is required.
2. Read an existing deck using `readOffice(input, {slide, offset, limit})`. Slide order follows presentation relationships. Text extraction does not capture layout, images, charts, notes or animations; inspect those package parts or rendered slides when relevant.
3. Plan a concise outline and a consistent visual system based on the user's reference/template. Author one reusable JavaScript builder using `new pptxgen()` and editable native text, tables, charts and shapes. Images should remain assets within the deck; do not rasterize whole slides.
4. PptxGenJS generates new presentations; it does not load an existing deck for full-fidelity editing. For narrow edits to an existing deck, use `JSZip` for targeted OOXML changes and preserve master/layout/relationship/media parts. Never recreate a complex deck silently.
5. Save via `savePresentation`, then `validateOffice` and read back the expected slide count/content. XML validity alone does not prove relationships, visual fidelity or editability.
6. Render with `renderOffice`, inspect every slide image for overflow, overlap, font substitution, image cropping and chart/table readability. Revise and render again after changes.
7. Deliver the actual `.pptx`. If rendering is unavailable, disclose that visual validation is incomplete.

## Execution
- Use a separate output path and stable slide/shape targets; never rely on a current PowerPoint selection.
- Use the existing `run_shell` process lifecycle; background long exports/renderers and poll the actual exit code.
- No COM, ActiveX, PowerShell PowerPoint automation or current-window control.
