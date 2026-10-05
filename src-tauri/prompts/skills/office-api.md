# Bundled Office runtime API

Run task scripts with the bundled Deno: `run --no-prompt --allow-read --allow-write --allow-env <script.mjs>`.
On PowerShell use `& '<absolute deno.exe>' ...`. Rendering, conversion and recalculation require `--allow-run` for the exact bundled Office Node path returned by `prepare_office_runtime`; on Windows include `taskkill.exe` for process-tree timeout cleanup. Scripts and file paths must respect the user's workspace and existing shell permissions.

Import the runtime with an absolute file URL, e.g.:

```js
import { docx, ExcelJS, pptxgen, JSZip, readOffice, validateOffice,
  saveDocx, saveWorkbook, savePresentation, renderOffice, recalculateWorkbook, convertOffice } from
  "file:///C:/work/.anya/office-runtime/v1/runtime.mjs";
```

Use the actual module path returned by `load_skill`/`prepare_office_runtime`.

## Read and validate

```js
console.log(JSON.stringify(await readOffice("input.xlsx", {
  sheet: "Data", offset: 1, limit: 100,
}), null, 2));
console.log(await validateOffice("output.xlsx"));
```

`readOffice(path, options)` supports `.docx`, `.dotx`, `.xlsx`, `.xlsm`, `.pptx`.
`offset` is 1-based; `limit` defaults to 100 and is capped at 500. `sheet` filters a worksheet; `slide` filters a 1-based slide. `nextOffset` indicates more records.
Word returns paragraph/table records, Excel returns addressed cells plus formulas/cached values, PPT returns slide text in presentation order. Additional package parts may need inspecting for images, comments, revisions, notes and styles.

`validateOffice(path)` checks ZIP CRC, XML syntax and required package parts. It is not an OOXML schema validator, formula engine or visual checker.

The CLI accepts `read <path> [optionsJSON]`, `validate <path>`, `render <path> <outputDir>`. Prefer a short wrapper script for options to avoid shell JSON quoting problems.

## Generate Word

```js
const { Document, Paragraph, TextRun, HeadingLevel } = docx;
const document = new Document({
  styles: { default: { document: { run: { font: "Microsoft YaHei", size: 22 } } } },
  sections: [{ children: [
    new Paragraph({ text: "项目报告", heading: HeadingLevel.TITLE }),
    new Paragraph({ children: [new TextRun("正文")] }),
  ] }],
});
console.log(await saveDocx(document, "output.docx"));
```

`docx` exposes the library's full public API for tables, styles, numbering, headers/footers and page layout. Use a real `Table` for tabular content. `saveDocx` writes and validates the package.

## Generate or edit Excel

```js
const workbook = new ExcelJS.Workbook();
const sheet = workbook.addWorksheet("Data");
sheet.addRows([["项目", "数量"], ["A", 2], ["B", 3]]);
sheet.getCell("B4").value = { formula: "SUM(B2:B3)" };
sheet.columns = [{ width: 24 }, { width: 14 }];
console.log(await saveWorkbook(workbook, "output.xlsx"));
```

For ordinary existing `.xlsx`, use `await workbook.xlsx.readFile(input)`, then edit specific cells and save a copy. ExcelJS does not calculate formulas; `fullCalcOnLoad` is only a recalculation request. Complex/unsupported/VBA workbooks require targeted OOXML preservation, not a whole-package rewrite.

## Generate PPT

```js
const presentation = new pptxgen();
presentation.layout = "LAYOUT_WIDE";
const slide = presentation.addSlide();
slide.addText("项目进展", { x: 0.6, y: 0.5, w: 12, h: 0.6,
  fontFace: "Microsoft YaHei", fontSize: 28 });
slide.addTable([["任务", "状态"], ["检查", "完成"]],
  { x: 0.6, y: 1.6, w: 8, fontFace: "Microsoft YaHei", fontSize: 18 });
console.log(await savePresentation(presentation, "output.pptx"));
```

PptxGenJS creates new decks. Use targeted ZIP/XML editing for an existing deck rather than claiming it supports full-fidelity import.

## Render and inspect

```js
console.log(await renderOffice("output.docx", ".anya/previews/report"));
```

Uses the packaged LibreOffice Kit engine and bundled Node; system LibreOffice, Poppler and Microsoft Office are not required. The helper uses a fresh output directory and engine deadlines (120 seconds per operation by default). It returns PDF, ordered page PNG paths, backend and missing-font diagnostics. Inspect images with the image-reading tool; execution success is not visual QA. If the packaged engine fails, report the limitation rather than switching to a system executable. Windows requires the matching Microsoft Visual C++ v14 Redistributable. Fonts are read from the system; unavailable fonts can change layout.

## Recalculate a workbook copy

`await recalculateWorkbook("output.xlsx", ".anya/calculation")` uses the bundled engine to write a freshly calculated copy in an isolated directory. Read back its results with `readOffice`; test representative input changes. Excel-specific formulas may behave differently, so retain the original and disclose engine limitations.

## Convert a saved document

`await convertOffice("legacy.doc", "converted.docx")` exports to a distinct fresh output path. Supported families include DOC/DOCX/ODT, XLS/XLSX/ODS and PPT/PPTX/ODP. Keep the source; renamed RTF/HTML files and WPS are unsupported. The output extension selects the format. Existing output files are rejected.
