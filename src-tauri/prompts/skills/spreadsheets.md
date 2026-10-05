---
name: spreadsheets
description: Read, analyze, create and edit saved Excel spreadsheets with addressable cells, formulas, formats and verification using the bundled JavaScript runtime.
---

# Excel spreadsheets

## Workflow
1. Read the runtime `API.md` and use the exact bundled Deno executable and runtime module supplied by `load_skill`. The exported `ExcelJS` creates/edits `.xlsx`; no separate npm installation is needed.
2. Inspect saved workbooks using `readOffice(input, {sheet, offset, limit})`. Records retain worksheet names, cell addresses, shared strings, formulas and cached values. Dates may be serial numbers; cached formula values may be stale. Summarize large data with a script instead of dumping every row into model context.
3. Write one reusable builder/analysis script. Use explicit worksheet names and range addresses, batch row/cell operations, preserve numbers versus strings, distinguish empty from zero, and use formulas for derived values. Keep inputs editable and preserve the supplied template.
4. Use `ExcelJS.Workbook`, worksheet APIs and `saveWorkbook`. Save a separate `.xlsx` output. ExcelJS cannot calculate formulas. `fullCalcOnLoad` is a request to spreadsheet software, not proof of recalculation.
5. Read back affected cells; inspect missing/duplicate keys, formula references, totals and error values. For formula tasks, use `recalculateWorkbook` with bundled LibreOffice Kit to calculate a copy and verify representative changed inputs. If it is unavailable, disclose that formulas were written but not recalculated; do not fabricate cached results or claim a PASS from file existence.
6. Render output with `renderOffice` and inspect relevant page images. Set print areas and scaling deliberately; a spreadsheet may span many pages. Check widths, wrapped text, units, number formats and charts.
7. Deliver the workbook path and any material validation limitation.

## Preservation boundaries
- `.xlsm`/VBA, external connections, pivot features and unsupported objects must not be silently rewritten with ExcelJS. Preserve them via targeted OOXML edits using `JSZip`, or explain the unsupported operation.
- Legacy `.xls` is not OOXML; convert a copy through an available file-conversion tool before reading. Never rename its extension to pretend conversion succeeded.
- Do not overwrite a source file that another app may have unsaved changes to. Ask for a saved copy when required information only exists in a live session.
- Use no COM, ActiveX or PowerShell Excel automation.
