---
name: documents
description: Read, create, edit and validate saved Word DOCX files using JavaScript, templates and OOXML; render and inspect pages before delivery.
---

# Word documents

Use this skill for saved Word files and new reports, forms or letters. Operate on saved files; no active-window or unsaved-document automation exists.

## Workflow
1. Use the document runtime paths supplied by `load_skill`. Read its `API.md` once before authoring. Use bundled Deno and the exported `docx` library; do not install npm/Python packages or assume a global runtime.
2. For an existing file, call `readOffice(input, {offset, limit})` from a small script. Read relevant paragraphs/tables and inspect additional OOXML parts for comments, revisions, headers, drawings or styles. Reading body text is not a complete document inspection.
3. Respect the user's template or reference. For a new file, plan heading styles, fonts (including Chinese), margins, page breaks and real editable tables. Never turn the whole document into screenshots.
4. Write a reusable `build-document.mjs` in the workspace. Import the absolute runtime module with a file URL. Use `docx.Document`, `Paragraph`, `TextRun`, `Table` and `saveDocx`. Preserve source files; edit an existing package using `JSZip` for targeted changes instead of recreating a complex document and losing its objects.
5. Validate the package with `validateOffice`, then read back relevant content. Validation checks ZIP/XML integrity, not all OOXML rules or layout.
6. Call `renderOffice(output, previewDir)` and inspect every returned PNG with the image-reading tool. Fix clipping, broken tables, missing glyphs, awkward pagination and overlaps, then render again.
7. Deliver the actual `.docx` path. Use the bundled LibreOffice Kit for previews. If the engine fails, state that visual validation was not performed. Do not claim the pages were checked merely because a file was generated.

## Editing rules
- Take a copy before editing. Use named paths and stable paragraph/table targets, not a guessed current selection.
- Preserve tracked changes and comments unless the task explicitly requests accepting/removing them. For redlines, edit the appropriate OOXML nodes and preserve authors, IDs and relationships.
- Keep headings semantic and table contents editable. Apply East Asian font mappings and consistent cell/paragraph formatting.
- Do not use COM, ActiveX, win32com or PowerShell Office automation.
- Use `run_shell` in the foreground for short commands; use background plus actual exit polling for long builders/renderers. Keep output compact and retain the script and intermediate files for recovery.
