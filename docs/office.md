# Office document workflows

Office processing uses saved files, on-demand skills and a bundled JavaScript runtime. It does not attach to Microsoft Office through COM or collect live document/selection context. Save live changes or attach a saved copy before processing.

## Available workflows

- `load_skill documents`: Word creation, targeted OOXML editing, readable styles/tables and page validation.
- `load_skill spreadsheets`: Excel analysis, addressed data/formulas, batch editing and calculation checks.
- `load_skill presentations`: editable PPT creation, targeted existing-deck edits and slide validation.
- `read_office_file`: structured saved-file reading with `path`, `sheet`, `slide`, 1-based `offset` and `limit` (maximum 500 records). `read_file` also routes Office files through this reader; its Office offset counts records, not text lines.
- `prepare_office_runtime`: exact executable, import URL, CLI and API paths. Loading an Office skill prepares these automatically.

These three Office skills are always available. `generate_word` and `docx` remain compatibility shortcuts to `documents`. No `word_*`, `excel_*` or `ppt_*` live automation tools remain.

## Execution and packaging

Anya bundles Deno, an offline module containing docx, ExcelJS, PptxGenJS, JSZip and fast-xml-parser, and LibreOffice Kit 0.1.5 with its platform engine and private Node runtime. This follows the skill/script/verification approach; it does not redistribute Codex's `@oai/artifact-tool` or claim identical document-library coverage.

`pnpm build:office-runtime` produces `src-tauri/resources/office/`. Tauri dev/build commands run this automatically and package the result as resources, including third-party notices. The runtime locates installed resources first and materializes them under `{workspace}/.anya/office-runtime/v1/`, preserving task scripts. Library loading, reading and generation require no first-run npm downloads or global Node/Python installation.

The agent writes a reusable JavaScript task script, runs it using the returned Deno path and the existing shell lifecycle, validates the output package, reads back content and renders previews for inspection. Long commands use background jobs and actual exit polling. Script sources and outputs remain available for task recovery; application-restart continuation is not implied.

## Validation boundaries

- `validateOffice` checks ZIP CRC, XML syntax and required package parts. It does not prove complete OOXML schema compliance, relationship correctness or visual layout.
- `renderOffice` uses bundled LibreOffice Kit without system LibreOffice or Poppler. It uses fresh output directories and process timeouts, returning PDF and ordered PNG paths. The agent must inspect images; successful rendering alone is not visual verification.
- `recalculateWorkbook` uses bundled LibreOffice Kit to calculate a fresh `.xlsx` copy. Verify affected results and representative input changes. Excel-specific formulas/features may differ; ExcelJS itself does not calculate formulas, and setting `fullCalcOnLoad` is not calculation evidence.
- If a renderer/calculation dependency is unavailable, report the unverified scope explicitly. Do not claim that layout or formula results were checked.
- Body/slide text reading does not capture every image, header, comment, revision, note or animation. Inspect additional OOXML parts or previews when these matter.
- Preserve complex templates and unsupported objects through targeted package edits. ExcelJS must not silently rewrite VBA `.xlsm` or unsupported workbook features; PptxGenJS does not provide full-fidelity import of existing decks.
- Convert legacy `.doc`, `.xls` and `.ppt` copies with `convertOffice` using the bundled engine. Renaming an extension is not conversion.

Windows native rendering uses system fonts and requires the Microsoft VC++ v14 runtime. Missing fonts are reported in results. Engine failures are reported without switching to a system renderer.

## Checks

Run `pnpm build:office-runtime` followed by `pnpm test:office-runtime` for real DOCX/XLSX/PPTX round trips and offline bundled-Deno smoke checks. The Office eval fixtures (`--include-office --filter office`) exercise skill loading without requiring Microsoft Office.

## Delivering generated files

After generation and the required validation, call `present` for final DOCX/XLSX/PPTX, PDF or image files. Editable sources and reading copies can be delivered together. Deliver scripts and intermediates only when needed by the user. Conversion and rendering do not automatically create cards, and successful `present` does not establish layout or formula correctness. See [file delivery cards](./file-delivery.md).
