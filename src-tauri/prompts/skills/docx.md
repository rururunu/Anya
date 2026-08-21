---
name: docx
description: Create, read, edit Word .docx/.dotx with docx-js, OOXML unzip/edit, tracked changes, comments, validation. Use for professional Word deliverables, editing existing docs, redlining. For md→docx bulk conversion prefer pandoc.
---

# DOCX creation, editing, and analysis

Playbook adapted from an open-source docx skill package. Helper scripts are materialized to `.anya/docx/scripts/` at skill run time.

## When to use this skill

Use `docx` when the task touches the **existing binary structure** of a Word file: tracked changes, comments, raw OOXML edits, or a new document that needs precise layout control (letterhead, TOC, custom styles) beyond what a simple script produces.

## When NOT to use this skill

| Need | Use instead |
|---|---|
| Markdown → Word/PDF/HTML format conversion | `pandoc` |
| Simple new doc, script-generated, no special layout | `generate_word` |

<example>
User: "把这份季度报告转成 Word，保留标题层级。"
Reasoning: the source is Markdown and the destination is a format conversion, not an edit of an existing binary docx or a layout-heavy new document.
Correct: use `pandoc`, not `docx` — reaching for OOXML surgery here would be solving a conversion problem with editing tools.
</example>

<example>
User: "这份合同第 3 条要加一条批注，说明需要法务确认。"
Reasoning: adding a comment to an existing .docx is exactly OOXML-level editing.
Correct: use `docx` (the comment scripts below), not `generate_word`, which only creates new documents from scratch.
</example>

> All `scripts/…` paths below mean `{SCRIPTS}` from the runtime preamble (`.anya/docx/scripts`).

A `.docx` is a ZIP archive of XML files. Choose your approach by task:

| Task | Approach |
|---|---|
| **Create** a new document | Write a `docx` (npm) script — see gotchas below |
| **Edit** an existing document | `unzip` → edit `word/document.xml` → `zip` (docx-js cannot open existing files) |
| **Read** content | `pandoc -t markdown file.docx` (or `#skill:pandoc`) |

## Creating with docx-js — gotchas

Try `require('docx')` first; only if it fails: `npm install docx` in the workspace. Footguns:

- **Page size defaults to A4.** For US Letter set `page: { size: { width: 12240, height: 15840 } }` (DXA; 1440 = 1″).
- **Landscape:** pass portrait dimensions and `orientation: PageOrientation.LANDSCAPE`.
- **Tables need dual widths:** `columnWidths` on the table AND `width` on every cell, both `WidthType.DXA`.
- **Table shading:** use `ShadingType.CLEAR`, never `SOLID`.
- **Lists:** use `numbering` with `LevelFormat.BULLET`, not literal `•`.
- **`ImageRun` requires `type:`** (`"png"`, `"jpg"`, …).
- **`PageBreak` must be inside a `Paragraph`.**
- **Never use `\n`** — separate `Paragraph` elements.
- **TOC:** headings need built-in `HeadingLevel.*` or `outlineLevel`.

## Verify the output

```bash
python {SCRIPTS}/office/soffice.py --headless --convert-to pdf output.docx
pdftoppm -jpeg -r 100 output.pdf page
# Read page-*.jpg to visually inspect
```

On Windows: install [LibreOffice](https://www.libreoffice.org/), [Poppler](https://github.com/oschwartz10612/poppler-windows/releases) (pdftoppm), add to PATH.

## Editing existing documents

Legacy `.doc` → `python {SCRIPTS}/office/soffice.py --headless --convert-to docx file.doc`

```bash
unzip -q doc.docx -d unpacked/
# Windows PowerShell: remove symlinks if any
python {SCRIPTS}/merge_runs.py unpacked/
# edit unpacked/word/document.xml in place — do NOT pretty-print
# repack (PowerShell Compress-Archive is NOT OOXML-safe; use zip or Python zipfile)
python {SCRIPTS}/office/validate.py out.docx --original doc.docx
```

**Tracked changes:** validate with `--author "…"` and `--original`. Use `<w:ins>` / `<w:del>`; inside `<w:del>` use `<w:delText>`.

Clean accept: `python {SCRIPTS}/accept_changes.py in.docx out.docx`

## Comments

```bash
python {SCRIPTS}/comment.py unpacked/ "Comment text"
python {SCRIPTS}/comment.py contract.docx "Comment" -o annotated.docx
```

Then place the printed range markers in `word/document.xml`.

## Dependencies

`docx` (npm) · `pandoc` · LibreOffice (`soffice`) · `pdftoppm` (Poppler)

Windows install hints: `winget install JohnMacFarlane.Pandoc` · LibreOffice MSI · Poppler zip on PATH.

## Output

Return the `.docx` path, what was changed/created, and verification steps taken (validate / preview).
