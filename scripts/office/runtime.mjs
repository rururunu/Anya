// Build-time imports are bundled; installed apps require only the Deno sidecar.
import * as docx from "docx";
import ExcelJS from "exceljs";
import pptxgen from "pptxgenjs";
import JSZip from "jszip";
import { XMLParser, XMLValidator } from "fast-xml-parser";
import { mkdir, readFile, writeFile, stat, readdir, access } from "node:fs/promises";
import { resolve, dirname, basename, extname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";

export { docx, ExcelJS, pptxgen, JSZip };
export const runtimeVersion = "1";
const MAX_FILE_BYTES = 128 * 1024 * 1024;
const MAX_XML_BYTES = 32 * 1024 * 1024;
const MAX_TOTAL_XML_BYTES = 128 * 1024 * 1024;
const parser = new XMLParser({ ignoreAttributes: false, preserveOrder: true, trimValues: false });

function tree(xml) {
  const convert = (items) =>
    items.flatMap((item) => {
      if ("#text" in item) return [{ name: "#text", text: String(item["#text"]) }];
      const name = Object.keys(item).find((key) => key !== ":@");
      return name ? [{ name, attrs: item[":@"] ?? {}, children: convert(item[name] ?? []) }] : [];
    });
  return { name: "root", children: convert(parser.parse(xml)) };
}
const local = (name) => name.split(":").at(-1);
function descendants(node, name) {
  const result = [];
  for (const child of node.children ?? []) {
    if (local(child.name) === name) result.push(child);
    result.push(...descendants(child, name));
  }
  return result;
}
const attr = (node, name) =>
  Object.entries(node?.attrs ?? {}).find(([key]) => local(key.replace(/^@_/, "")) === name)?.[1];
function text(node) {
  if (node.name === "#text") return node.text;
  if (local(node.name) === "tab") return "\t";
  if (["br", "cr"].includes(local(node.name))) return "\n";
  return (node.children ?? []).map(text).join("");
}
async function loadZip(path) {
  const size = (await stat(path)).size;
  if (size > MAX_FILE_BYTES)
    throw new Error("Office file exceeds 128 MiB; split or use a streaming reader.");
  const zip = await JSZip.loadAsync(await readFile(path));
  let total = 0;
  for (const entry of Object.values(zip.files)) {
    const bytes = entry._data?.uncompressedSize ?? 0;
    if (bytes > MAX_XML_BYTES) throw new Error(`Office ZIP member exceeds 32 MiB: ${entry.name}`);
    total += bytes;
    if (total > MAX_TOTAL_XML_BYTES)
      throw new Error("Office ZIP expands beyond 128 MiB; use a streaming reader.");
  }
  return zip;
}
async function xmlPart(zip, name) {
  const entry = zip.file(name);
  if (!entry) throw new Error(`Missing OOXML part: ${name}`);
  // Check declared uncompressed size before allocation, then enforce the decoded limit.
  if (entry._data?.uncompressedSize > MAX_XML_BYTES) throw new Error(`XML part too large: ${name}`);
  const xml = await entry.async("string");
  if (Buffer.byteLength(xml) > MAX_XML_BYTES) throw new Error(`XML part too large: ${name}`);
  const validity = XMLValidator.validate(xml);
  if (validity !== true) throw new Error(`Invalid XML in ${name}: ${validity.err.msg}`);
  return xml;
}
function relationshipMap(root) {
  return new Map(
    descendants(root, "Relationship").map((node) => [
      String(attr(node, "Id")),
      String(attr(node, "Target")),
    ]),
  );
}
function partPath(base, target) {
  const parts = target.startsWith("/") ? [] : base.split("/").slice(0, -1);
  for (const part of target.split("/")) {
    if (part === "..") parts.pop();
    else if (part && part !== ".") parts.push(part);
  }
  return parts.join("/");
}
function windowRecords(records, options, metadata) {
  const offset = Math.max(1, Number(options.offset) || 1);
  const limit = Math.min(500, Math.max(1, Number(options.limit) || 100));
  return {
    ...metadata,
    total: records.length,
    offset,
    records: records.slice(offset - 1, offset - 1 + limit),
    nextOffset: offset - 1 + limit < records.length ? offset + limit : null,
  };
}

/** Read saved Office files as addressable records, not unstructured XML dumps. */
export async function readOffice(path, options = {}) {
  path = resolve(path);
  const extension = extname(path).toLowerCase();
  const zip = await loadZip(path);
  if ([".docx", ".dotx"].includes(extension)) {
    const body = descendants(tree(await xmlPart(zip, "word/document.xml")), "body")[0];
    const records = [];
    for (const node of body?.children ?? []) {
      if (local(node.name) === "p")
        records.push({
          kind: "paragraph",
          index: records.length + 1,
          text: text(node),
          style: attr(descendants(node, "pStyle")[0], "val"),
        });
      if (local(node.name) === "tbl")
        records.push({
          kind: "table",
          index: records.length + 1,
          rows: descendants(node, "tr").map((row) =>
            (row.children ?? [])
              .filter((cell) => local(cell.name) === "tc")
              .map((cell) => descendants(cell, "p").map(text).join("\n")),
          ),
        });
    }
    return windowRecords(records, options, {
      format: "docx",
      path,
      warnings: [
        "Saved file only; drawings, headers, footers, comments and revisions require inspecting additional OOXML parts.",
      ],
    });
  }
  if ([".xlsx", ".xlsm"].includes(extension)) {
    const strings = zip.file("xl/sharedStrings.xml")
      ? descendants(tree(await xmlPart(zip, "xl/sharedStrings.xml")), "si").map(text)
      : [];
    const workbook = tree(await xmlPart(zip, "xl/workbook.xml"));
    const relationships = relationshipMap(tree(await xmlPart(zip, "xl/_rels/workbook.xml.rels")));
    const sheets = descendants(workbook, "sheet").map((sheet) => ({
      name: String(attr(sheet, "name")),
      part: partPath("xl/workbook.xml", relationships.get(String(attr(sheet, "id"))) ?? ""),
    }));
    const records = [];
    for (const sheet of sheets.filter((sheet) => !options.sheet || sheet.name === options.sheet)) {
      for (const cell of descendants(tree(await xmlPart(zip, sheet.part)), "c")) {
        const type = attr(cell, "t");
        const raw = text(descendants(cell, "v")[0] ?? {});
        const formula = descendants(cell, "f")[0];
        const value =
          type === "s"
            ? strings[Number(raw)]
            : type === "inlineStr"
              ? text(descendants(cell, "is")[0] ?? {})
              : type === "b"
                ? raw === "1"
                : raw;
        records.push({
          sheet: sheet.name,
          address: attr(cell, "r"),
          type: type ?? "n",
          value,
          ...(formula ? { formula: text(formula), formulaAttributes: formula.attrs } : {}),
        });
      }
    }
    if (options.sheet && !sheets.some((sheet) => sheet.name === options.sheet))
      throw new Error(`Worksheet not found: ${options.sheet}`);
    return windowRecords(records, options, {
      format: "xlsx",
      path,
      sheets: sheets.map((sheet) => sheet.name),
      warnings: [
        "Formula values are cached; dates may be serial numbers. This reader does not recalculate.",
        ...(extension === ".xlsm"
          ? [
              "Macro-enabled workbook: do not rewrite with ExcelJS; preserve VBA and unsupported parts via targeted OOXML edits.",
            ]
          : []),
      ],
    });
  }
  if (extension === ".pptx") {
    const presentation = tree(await xmlPart(zip, "ppt/presentation.xml"));
    const relationships = relationshipMap(
      tree(await xmlPart(zip, "ppt/_rels/presentation.xml.rels")),
    );
    const slides = descendants(presentation, "sldId").map((slide) =>
      partPath(
        "ppt/presentation.xml",
        relationships.get(
          String(Object.entries(slide.attrs ?? {}).find(([key]) => key.endsWith(":id"))?.[1]),
        ) ?? "",
      ),
    );
    const records = [];
    for (let index = 0; index < slides.length; index++) {
      if (options.slide && Number(options.slide) !== index + 1) continue;
      const slide = tree(await xmlPart(zip, slides[index]));
      records.push({
        slide: index + 1,
        part: slides[index],
        paragraphs: descendants(slide, "p").map(text).filter(Boolean),
      });
    }
    return windowRecords(records, options, {
      format: "pptx",
      path,
      slideCount: slides.length,
      warnings: ["Text extraction does not prove visual layout; render slides before delivery."],
    });
  }
  throw new Error(
    `Unsupported Office extension: ${extension}; convert legacy .doc/.xls/.ppt first.`,
  );
}

/** ZIP/XML integrity validation, not a complete OOXML schema or layout validator. */
export async function validateOffice(path) {
  const zip = await loadZip(path);
  // Check CRC only after inspecting declared uncompressed sizes.
  await JSZip.loadAsync(await readFile(path), { checkCRC32: true });
  const extension = extname(path).toLowerCase();
  const required = [
    "[Content_Types].xml",
    "_rels/.rels",
    extension === ".pptx"
      ? "ppt/presentation.xml"
      : [".xlsx", ".xlsm"].includes(extension)
        ? "xl/workbook.xml"
        : "word/document.xml",
  ];
  for (const part of required) await xmlPart(zip, part);
  let count = 0;
  let total = 0;
  for (const entry of Object.values(zip.files).filter(
    (entry) => !entry.dir && /\.(xml|rels)$/.test(entry.name),
  )) {
    total += entry._data?.uncompressedSize ?? 0;
    if (total > MAX_TOTAL_XML_BYTES)
      throw new Error("Office XML exceeds 128 MiB; use a streaming validator.");
    await xmlPart(zip, entry.name);
    count++;
  }
  return {
    path: resolve(path),
    valid: true,
    xmlParts: count,
    scope:
      "ZIP CRC and XML syntax plus required package parts; no schema, formula calculation or visual validation",
  };
}

export async function saveDocx(document, path) {
  await mkdir(dirname(resolve(path)), { recursive: true });
  await writeFile(path, await docx.Packer.toBuffer(document));
  return validateOffice(path);
}
export async function saveWorkbook(workbook, path) {
  if (extname(path).toLowerCase() !== ".xlsx")
    throw new Error(
      "ExcelJS output must be .xlsx; macro preservation requires targeted OOXML editing.",
    );
  workbook.calcProperties.fullCalcOnLoad = true;
  await mkdir(dirname(resolve(path)), { recursive: true });
  await workbook.xlsx.writeFile(path);
  return validateOffice(path);
}
export async function savePresentation(presentation, path) {
  await mkdir(dirname(resolve(path)), { recursive: true });
  await presentation.writeFile({ fileName: resolve(path), compression: true });
  return validateOffice(path);
}

/** Exact installed paths; never search PATH or download a conversion engine. */
async function kitPaths() {
  const local = dirname(fileURLToPath(import.meta.url));
  let root = local;
  try {
    const config = JSON.parse(await readFile(join(local, "kit-path.json"), "utf8"));
    root = config.resourceDirectory;
    if (typeof root !== "string") throw new Error("Invalid Office runtime configuration");
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
  const node = join(root, process.platform === "win32" ? "node.exe" : "node");
  const worker = join(root, "kit/worker.mjs");
  try {
    await access(node);
    await access(worker);
  } catch {
    throw new Error(
      "Bundled LibreOffice Kit is missing. Rebuild the Office runtime or reinstall Anya.",
    );
  }
  return { node, worker };
}

async function kitOperation(operation, input, output, timeoutMs, images) {
  if (!Number.isSafeInteger(timeoutMs) || timeoutMs <= 0)
    throw new Error("Office timeout must be a positive integer");
  const { node, worker } = await kitPaths();
  return await new Promise((resolvePromise, reject) => {
    const child = spawn(node, [worker], {
      windowsHide: true,
      stdio: ["pipe", "pipe", "pipe"],
      detached: process.platform !== "win32",
    });
    let stdout = "";
    let stderr = "";
    let failure;
    function terminate() {
      if (process.platform === "win32" && child.pid) {
        const killer = spawn("taskkill.exe", ["/PID", String(child.pid), "/T", "/F"], {
          windowsHide: true,
          stdio: "ignore",
        });
        killer.on("error", () => child.kill());
      } else {
        try {
          process.kill(-child.pid, "SIGKILL");
        } catch {
          child.kill();
        }
      }
    }
    child.stdout.on("data", (data) => {
      stdout += data;
      if (stdout.length > 4 * 1024 * 1024) {
        failure = new Error("Office result exceeds 4 MiB");
        terminate();
      }
    });
    child.stderr.on("data", (data) => {
      stderr = (stderr + data).slice(-16000);
    });
    const timer = setTimeout(
      () => {
        failure = new Error(`Bundled Office operation timed out: ${stderr}`);
        terminate();
      },
      timeoutMs * (operation === "render" ? 2 : 1) + 15000,
    );
    child.once("error", (error) => {
      clearTimeout(timer);
      reject(error);
    });
    child.stdin.on("error", (error) => {
      failure ??= error;
    });
    child.once("close", (code) => {
      clearTimeout(timer);
      if (failure) {
        reject(failure);
        return;
      }
      if (code !== 0) {
        reject(new Error(`Bundled LibreOffice Kit failed: ${stderr}`));
        return;
      }
      try {
        resolvePromise(JSON.parse(stdout));
      } catch {
        reject(new Error("Bundled Office engine returned invalid JSON"));
      }
    });
    child.stdin.end(
      JSON.stringify({
        operation,
        input: resolve(input),
        output: resolve(output),
        images,
        timeoutMs,
      }),
    );
  });
}

/** Recalculate a fresh XLSX copy using the bundled engine. */
export async function recalculateWorkbook(input, outputDir, { timeoutMs = 120000 } = {}) {
  if (extname(input).toLowerCase() !== ".xlsx")
    throw new Error(
      "Recalculation supports .xlsx copies only; do not convert VBA workbooks silently.",
    );
  const runDir = join(resolve(outputDir), `calc-${crypto.randomUUID()}`);
  await mkdir(runDir, { recursive: true });
  const output = join(runDir, basename(input));
  const result = await kitOperation("recalculate", input, output, timeoutMs);
  await validateOffice(output);
  return {
    ...result,
    path: output,
    note: "Read back formula results and test affected inputs. Excel-specific formulas/features may differ; retain the original.",
  };
}

/** Render PDF and ordered PNG pages entirely with the bundled engine. */
export async function renderOffice(input, outputDir, { timeoutMs = 120000 } = {}) {
  const runDir = join(resolve(outputDir), `render-${crypto.randomUUID()}`);
  await mkdir(runDir, { recursive: true });
  const pdf = join(runDir, basename(input, extname(input)) + ".pdf");
  const result = await kitOperation("render", input, pdf, timeoutMs, join(runDir, "pages"));
  if (!(await stat(pdf)).size || !result.pages?.length)
    throw new Error("Bundled renderer produced no PDF or page images");
  for (const page of result.pages)
    if (!(await stat(page)).size) throw new Error("Bundled renderer produced an empty page image");
  return {
    ...result,
    pdf,
    note: "Inspect every returned page image; render success alone is not a visual quality check.",
  };
}

/** Convert a saved Office file to a distinct fresh output extension. */
export async function convertOffice(input, output, { timeoutMs = 120000 } = {}) {
  await mkdir(dirname(resolve(output)), { recursive: true });
  return { ...(await kitOperation("convert", input, output, timeoutMs)), path: resolve(output) };
}
