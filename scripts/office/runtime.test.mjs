import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, writeFile, readFile, rm, readdir, copyFile } from "node:fs/promises";
import { join, dirname } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath, pathToFileURL } from "node:url";
import { spawnSync } from "node:child_process";

const root = join(dirname(fileURLToPath(import.meta.url)), "../..");
const runtimePath = join(root, "src-tauri/resources/office/runtime.mjs");
const {
  docx,
  ExcelJS,
  pptxgen,
  JSZip,
  readOffice,
  validateOffice,
  saveDocx,
  saveWorkbook,
  savePresentation,
  renderOffice,
  recalculateWorkbook,
} = await import(pathToFileURL(runtimePath).href);
async function workspace(run) {
  const dir = await mkdtemp(join(tmpdir(), "anya-office-smoke-中文 "));
  try {
    await run(dir);
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
}

test("Word round trip preserves Chinese paragraphs and real table cells", () =>
  workspace(async (dir) => {
    const { Document, Paragraph, Table, TableRow, TableCell } = docx;
    const path = join(dir, "中文报告.docx");
    const document = new Document({
      sections: [
        {
          children: [
            new Paragraph("项目报告 & 检查"),
            new Table({
              rows: [
                new TableRow({
                  children: [
                    new TableCell({ children: [new Paragraph("任务")] }),
                    new TableCell({ children: [new Paragraph("完成")] }),
                  ],
                }),
              ],
            }),
          ],
        },
      ],
    });
    assert.equal((await saveDocx(document, path)).valid, true);
    const result = await readOffice(path);
    assert.equal(result.records[0].text, "项目报告 & 检查");
    assert.deepEqual(result.records[1].rows, [["任务", "完成"]]);
    assert.equal(result.total, 2);
    assert.equal((await readOffice(path, { offset: 2, limit: 1 })).records[0].kind, "table");
  }));

test("Excel reader resolves strings and preserves worksheet names, addresses and formulas", () =>
  workspace(async (dir) => {
    const path = join(dir, "数据.xlsx");
    const workbook = new ExcelJS.Workbook();
    const sheet = workbook.addWorksheet("中文数据");
    sheet.addRows([
      ["项目", "数量"],
      ["A", 2],
      ["B", 3],
    ]);
    sheet.getCell("B4").value = { formula: "SUM(B2:B3)", result: 5 };
    workbook.addWorksheet("空表");
    await saveWorkbook(workbook, path);
    const result = await readOffice(path, { sheet: "中文数据" });
    assert.deepEqual(result.sheets, ["中文数据", "空表"]);
    assert.equal(result.records.find((cell) => cell.address === "A1").value, "项目");
    assert.equal(result.records.find((cell) => cell.address === "B4").formula, "SUM(B2:B3)");
    assert.equal(result.records.find((cell) => cell.address === "B4").value, "5");
    assert.match(result.warnings.join(" "), /cached/);
    const page = await readOffice(path, { offset: 2, limit: 2 });
    assert.equal(page.nextOffset, 4);
    assert.equal(page.records.length, 2);
    await assert.rejects(readOffice(path, { sheet: "不存在" }), /Worksheet not found/);
    await assert.rejects(saveWorkbook(workbook, join(dir, "macro.xlsm")), /macro preservation/);
  }));

test("PPT reader follows presentation relationship order beyond slide 9", () =>
  workspace(async (dir) => {
    const path = join(dir, "slides.pptx");
    const presentation = new pptxgen();
    for (let index = 1; index <= 12; index++)
      presentation.addSlide().addText(`第 ${index} 页`, { x: 1, y: 1, w: 5, h: 1 });
    await savePresentation(presentation, path);
    const result = await readOffice(path);
    assert.equal(result.slideCount, 12);
    assert.equal(result.records[1].paragraphs[0], "第 2 页");
    assert.equal(result.records[9].paragraphs[0], "第 10 页");
    assert.equal((await readOffice(path, { slide: 12 })).records[0].slide, 12);
    // Change actual slide order independently of ZIP member filenames.
    const zip = await JSZip.loadAsync(await readFile(path));
    const xml = await zip.file("ppt/presentation.xml").async("string");
    const ids = xml.match(/<p:sldId\b[^>]*\/>/g);
    zip.file("ppt/presentation.xml", xml.replace(ids.join(""), [...ids].reverse().join("")));
    await writeFile(path, await zip.generateAsync({ type: "nodebuffer" }));
    assert.equal((await readOffice(path)).records[0].paragraphs[0], "第 12 页");
  }));

test("Malformed and incomplete Office packages fail instead of claiming valid", () =>
  workspace(async (dir) => {
    const path = join(dir, "broken.docx");
    const zip = new JSZip();
    zip.file("word/document.xml", "<document><broken></document>");
    await writeFile(path, await zip.generateAsync({ type: "nodebuffer" }));
    await assert.rejects(validateOffice(path), /Missing OOXML part/);
    await assert.rejects(readOffice(path), /Invalid XML/);
  }));

test("Rendering fails closed when the installed kit resources are missing", () =>
  workspace(async (dir) => {
    const copy = join(dir, "runtime.mjs");
    await copyFile(runtimePath, copy);
    const copiedRuntime = await import(pathToFileURL(copy).href);
    await assert.rejects(
      copiedRuntime.renderOffice(join(dir, "input.docx"), dir),
      /Bundled LibreOffice Kit is missing/,
    );
  }));

test("Bundled native engine renders DOCX and PPTX and recalculates XLSX with no system renderer", () =>
  workspace(async (dir) => {
    const word = join(dir, "中文报告.docx");
    await saveDocx(
      new docx.Document({ sections: [{ children: [new docx.Paragraph("内置引擎中文测试")] }] }),
      word,
    );
    const deck = new pptxgen();
    deck.addSlide().addText("内置幻灯片", { x: 1, y: 1, w: 6, h: 1, fontFace: "Microsoft YaHei" });
    const ppt = join(dir, "中文演示.pptx");
    await savePresentation(deck, ppt);
    const book = new ExcelJS.Workbook();
    const sheet = book.addWorksheet("数据");
    sheet.getCell("A1").value = 2;
    sheet.getCell("A2").value = 3;
    sheet.getCell("A3").value = { formula: "SUM(A1:A2)" };
    const xlsx = join(dir, "中文计算.xlsx");
    await saveWorkbook(book, xlsx);
    const savedPath = process.env.PATH;
    process.env.PATH = "";
    try {
      for (const input of [word, ppt]) {
        const preview = await renderOffice(input, join(dir, "preview"));
        assert.equal(preview.backend, "native");
        assert.ok((await readFile(preview.pdf)).subarray(0, 4).equals(Buffer.from("%PDF")));
        assert.ok(preview.pages.length > 0);
        assert.ok(
          (await readFile(preview.pages[0]))
            .subarray(0, 8)
            .equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])),
        );
      }
      const calculated = await recalculateWorkbook(xlsx, join(dir, "calculated"));
      const readBack = new ExcelJS.Workbook();
      await readBack.xlsx.readFile(calculated.path);
      assert.equal(readBack.getWorksheet("数据").getCell("A3").value.result, 5);
      assert.equal(sheet.getCell("A3").value.result, undefined);
    } finally {
      if (savedPath === undefined) delete process.env.PATH;
      else process.env.PATH = savedPath;
    }
  }));

test("Installed bundle creates and reads all three formats with bundled Deno offline", () =>
  workspace(async (dir) => {
    const binaries = join(root, "src-tauri/binaries");
    const filename = (await readdir(binaries)).find(
      (name) =>
        name.startsWith("deno-") && name.endsWith(process.platform === "win32" ? ".exe" : ""),
    );
    assert.ok(filename, "Deno sidecar must be available for the installed-runtime smoke test");
    const script = join(dir, "builder.mjs");
    const installedRuntime = join(dir, "runtime.mjs");
    await copyFile(runtimePath, installedRuntime);
    await writeFile(
      join(dir, "kit-path.json"),
      JSON.stringify({ resourceDirectory: dirname(runtimePath) }),
    );
    await writeFile(
      script,
      `
    import { docx, ExcelJS, pptxgen, saveDocx, saveWorkbook, savePresentation, readOffice, renderOffice } from ${JSON.stringify(pathToFileURL(installedRuntime).href)};
    const document = new docx.Document({ sections:[{ children:[new docx.Paragraph("离线文档")] }] });
    await saveDocx(document, "output.docx");
    const workbook = new ExcelJS.Workbook(); workbook.addWorksheet("Sheet1").getCell("A1").value = "离线表格";
    await saveWorkbook(workbook, "output.xlsx");
    const presentation = new pptxgen(); presentation.addSlide().addText("离线幻灯片", {x:1,y:1,w:6,h:1});
    await savePresentation(presentation, "output.pptx");
    for (const file of ["output.docx", "output.xlsx", "output.pptx"]) console.log(JSON.stringify(await readOffice(file)));
    const preview = await renderOffice("output.docx", "preview");
    if (!preview.pages.length || preview.backend !== "native") throw new Error("Installed engine preview failed");
  `,
    );
    const result = spawnSync(
      join(binaries, filename),
      [
        "run",
        "--cached-only",
        "--no-prompt",
        "--allow-read",
        "--allow-write",
        "--allow-env",
        "--allow-run",
        script,
      ],
      {
        cwd: dir,
        encoding: "utf8",
        timeout: 60000,
        windowsHide: true,
        env: { ...process.env, DENO_DIR: join(dir, "empty-deno-cache") },
      },
    );
    assert.equal(result.status, 0, result.stderr || result.error?.message);
    assert.match(result.stdout, /离线文档/);
    assert.match(result.stdout, /离线表格/);
    assert.match(result.stdout, /离线幻灯片/);
  }));
