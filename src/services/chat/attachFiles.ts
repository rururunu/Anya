/** Limits for non-image file attachments embedded into the user message. */
export const ATTACH_FILE_MAX_BYTES = 256 * 1024;
export const ATTACH_FILES_TOTAL_MAX_BYTES = 512 * 1024;

/** Extensions we never try to inline as text. */
const BINARY_EXTENSIONS = new Set([
  "exe",
  "dll",
  "so",
  "dylib",
  "bin",
  "o",
  "obj",
  "a",
  "lib",
  "wasm",
  "pyc",
  "class",
  "jar",
  "apk",
  "dmg",
  "iso",
  "zip",
  "7z",
  "rar",
  "tar",
  "gz",
  "tgz",
  "bz2",
  "xz",
  "zst",
  "png",
  "jpg",
  "jpeg",
  "gif",
  "webp",
  "bmp",
  "ico",
  "heic",
  "mp3",
  "mp4",
  "wav",
  "flac",
  "avi",
  "mkv",
  "mov",
  "webm",
  "pdf",
  "doc",
  "docx",
  "xls",
  "xlsx",
  "ppt",
  "pptx",
  "ttf",
  "otf",
  "woff",
  "woff2",
  "eot",
  "sqlite",
  "db",
  "pdb",
  "node",
]);

export type AttachedFileChip = {
  path: string;
  name: string;
  size: number;
  content: string | null;
  skippedReason?: string;
};

/** Compact display metadata (content stays in raw message for the model). */
export type AttachedFileDisplay = {
  name: string;
  path: string;
  skipped?: string;
};

function extensionOf(name: string): string {
  const base = name.split(/[/\\]/).pop() ?? name;
  const dot = base.lastIndexOf(".");
  if (dot <= 0) return "";
  return base.slice(dot + 1).toLowerCase();
}

function isBinaryMime(type: string): boolean {
  if (!type) return false;
  if (type.startsWith("text/")) return false;
  if (
    type.includes("json") ||
    type.includes("xml") ||
    type.includes("javascript") ||
    type.includes("typescript") ||
    type.includes("svg")
  ) {
    return false;
  }
  if (type === "application/octet-stream") return false;
  return (
    type.startsWith("image/") ||
    type.startsWith("audio/") ||
    type.startsWith("video/") ||
    type.startsWith("font/") ||
    type === "application/pdf" ||
    type === "application/zip" ||
    type.includes("msword") ||
    type.includes("officedocument")
  );
}

function looksLikeUtf16Le(bytes: Uint8Array): boolean {
  if (bytes.length < 4 || bytes.length % 2 !== 0) return false;
  const sample = bytes.subarray(0, Math.min(bytes.length, 512));
  let asciiPairs = 0;
  let pairs = 0;
  for (let i = 0; i + 1 < sample.length; i += 2) {
    pairs += 1;
    const lo = sample[i]!;
    const hi = sample[i + 1]!;
    if (hi === 0 && lo !== 0 && lo < 0x80) asciiPairs += 1;
  }
  return pairs > 0 && asciiPairs / pairs > 0.6;
}

function decodeTextBytes(bytes: Uint8Array): string | null {
  if (bytes.length === 0) return "";

  if (bytes.length >= 2 && bytes[0] === 0xff && bytes[1] === 0xfe) {
    return new TextDecoder("utf-16le").decode(bytes);
  }
  if (bytes.length >= 2 && bytes[0] === 0xfe && bytes[1] === 0xff) {
    return new TextDecoder("utf-16be").decode(bytes);
  }
  if (bytes.length >= 3 && bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf) {
    return new TextDecoder("utf-8").decode(bytes);
  }

  if (looksLikeUtf16Le(bytes)) {
    return new TextDecoder("utf-16le").decode(bytes);
  }

  return new TextDecoder("utf-8", { fatal: false }).decode(bytes);
}

export function looksBinary(bytes: Uint8Array, text: string): boolean {
  if (text.includes("\u0000")) return true;

  const sampleText = text.slice(0, 4096);
  if (sampleText.length > 0) {
    let bad = 0;
    for (let i = 0; i < sampleText.length; i += 1) {
      const code = sampleText.charCodeAt(i);
      if (code === 0xfffd) bad += 1;
      else if (code < 32 && code !== 9 && code !== 10 && code !== 13) bad += 1;
    }
    if (bad / sampleText.length > 0.3) return true;
  }

  const sample = bytes.subarray(0, Math.min(bytes.length, 4096));
  if (sample.length === 0 || looksLikeUtf16Le(bytes)) return false;

  let nul = 0;
  for (let i = 0; i < sample.length; i += 1) {
    if (sample[i] === 0) nul += 1;
  }
  return nul / sample.length > 0.02;
}

export function isImageFile(file: File): boolean {
  return file.type.startsWith("image/") || /\.(png|jpe?g|gif|webp|bmp|svg)$/i.test(file.name);
}

export function filePathHint(file: File): string {
  const withPath = file as File & { path?: string };
  if (typeof withPath.path === "string" && withPath.path.trim()) {
    return withPath.path.trim();
  }
  return file.name;
}

export async function readAttachedFile(file: File): Promise<AttachedFileChip> {
  const path = filePathHint(file);
  const name = file.name || path.split(/[/\\]/).pop() || "file";
  const size = file.size;

  if (size > ATTACH_FILE_MAX_BYTES) {
    return {
      path,
      name,
      size,
      content: null,
      skippedReason: `exceeds ${Math.round(ATTACH_FILE_MAX_BYTES / 1024)}KB limit`,
    };
  }

  const ext = extensionOf(name);
  if (BINARY_EXTENSIONS.has(ext) || isBinaryMime(file.type)) {
    return {
      path,
      name,
      size,
      content: null,
      skippedReason: "binary skipped",
    };
  }

  const bytes = new Uint8Array(await file.arrayBuffer());
  const text = decodeTextBytes(bytes);
  if (text == null || looksBinary(bytes, text)) {
    return {
      path,
      name,
      size,
      content: null,
      skippedReason: "binary skipped",
    };
  }

  return { path, name, size, content: text };
}

function escapeAttr(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

function unescapeAttr(value: string): string {
  return value
    .replace(/&quot;/g, '"')
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&amp;/g, "&");
}

function parseTagAttrs(raw: string): Record<string, string> {
  const attrs: Record<string, string> = {};
  const re = /([:\w-]+)\s*=\s*"([^"]*)"/g;
  let match: RegExpExecArray | null;
  while ((match = re.exec(raw)) !== null) {
    attrs[match[1]!] = unescapeAttr(match[2] ?? "");
  }
  return attrs;
}

function hasFilesystemPath(path: string): boolean {
  return /[/\\]/.test(path) || /^[a-zA-Z]:/.test(path);
}

function officeHint(name: string): string {
  const ext = extensionOf(name);
  if (ext === "pptx" || ext === "ppt") {
    return "Use python-pptx (or Expand-Archive + read ppt/slides/*.xml) via run_shell on this absolute path.";
  }
  if (ext === "docx" || ext === "doc") {
    return "Use python-docx via run_shell on this absolute path.";
  }
  if (ext === "xlsx" || ext === "xls") {
    return "Use openpyxl / pandas via run_shell on this absolute path.";
  }
  if (ext === "pdf") {
    return "Extract text with a PDF tool or Python (e.g. pypdf) via run_shell on this absolute path.";
  }
  return "Open this path with an appropriate tool via run_shell; read_file cannot decode this binary format.";
}

/**
 * Embed file bodies for the model, wrapped so the UI can collapse them to chips.
 * Full content remains in the stored message — only the chat bubble hides it.
 */
export function formatAttachedFilesForMessage(files: AttachedFileChip[]): string {
  if (files.length === 0) return "";

  const parts: string[] = [];
  let total = 0;
  for (const file of files) {
    const label = file.path || file.name;
    const name = file.name || label.split(/[/\\]/).pop() || "file";
    const nameAttr = escapeAttr(name);
    const pathAttr = escapeAttr(label);

    if (file.content == null) {
      const reason = file.skippedReason ?? "unavailable";
      const pathLine = hasFilesystemPath(label)
        ? `Absolute path: ${label}`
        : `Filename only (no absolute path from the OS): ${label}. Locate the real file (e.g. Desktop) before editing.`;
      const body = [
        `Binary/office attachment — content was NOT inlined (${reason}).`,
        pathLine,
        officeHint(name),
        "You MUST inspect THIS exact file before editing or rewriting it.",
        "Do NOT invent a substitute document on a different topic, title, or theme.",
        "Preserve the user's narrative; optimize or transform the given file.",
      ].join("\n");
      parts.push(
        `<peek-attached-file name="${nameAttr}" path="${pathAttr}" skipped="${escapeAttr(reason)}">\n${body}\n</peek-attached-file>`,
      );
      continue;
    }

    let body = file.content;
    const remaining = ATTACH_FILES_TOTAL_MAX_BYTES - total;
    if (remaining <= 0) {
      parts.push(
        `<peek-attached-file name="${nameAttr}" path="${pathAttr}" skipped="total attachment budget exceeded" />`,
      );
      continue;
    }
    if (body.length > remaining) {
      body = `${body.slice(0, remaining)}\n…(truncated)`;
    }
    total += body.length;
    parts.push(
      `<peek-attached-file name="${nameAttr}" path="${pathAttr}">\n${body}\n</peek-attached-file>`,
    );
  }
  return parts.join("\n\n");
}

const ATTACHED_FILE_TAG_RE =
  /<peek-attached-file\b([^>]*)\/>|<peek-attached-file\b([^>]*)>([\s\S]*?)<\/peek-attached-file>/gi;

/** Legacy format used before peek-attached-file tags. */
const LEGACY_ATTACHED_RE =
  /\[Attached file: ([^\]]+)\]\n(?:\(Skipped: ([^)]+)\)|```(?:\w*)\n([\s\S]*?)```)/g;

/** Reconstruct full attachment chips (with content) from a persisted message, for composer restore. */
export function extractAttachedFileChips(content: string): AttachedFileChip[] {
  const chips: AttachedFileChip[] = [];
  const re = new RegExp(ATTACHED_FILE_TAG_RE.source, "gi");
  let match: RegExpExecArray | null;
  while ((match = re.exec(content)) !== null) {
    const [, selfAttrs, openAttrs, body] = match;
    const attrs = parseTagAttrs(String(selfAttrs ?? openAttrs ?? ""));
    const path = attrs.path || attrs.name || "file";
    const name = attrs.name || path.split(/[/\\]/).pop() || "file";
    if (attrs.skipped) {
      chips.push({ path, name, size: 0, content: null, skippedReason: attrs.skipped });
      continue;
    }
    const text = body ?? "";
    chips.push({ path, name, size: text.length, content: text });
  }
  return chips;
}

/**
 * Strip attached-file payloads from visible user text and return chip metadata.
 * Model still receives the original content string unchanged.
 */
export function extractAttachedFiles(content: string): {
  text: string;
  attachedFiles: AttachedFileDisplay[];
} {
  const attachedFiles: AttachedFileDisplay[] = [];
  let text = content;

  text = text.replace(ATTACHED_FILE_TAG_RE, (_full, selfAttrs, openAttrs, _body) => {
    const attrs = parseTagAttrs(String(selfAttrs ?? openAttrs ?? ""));
    const path = attrs.path || attrs.name || "file";
    const name = attrs.name || path.split(/[/\\]/).pop() || "file";
    attachedFiles.push({
      name,
      path,
      skipped: attrs.skipped || undefined,
    });
    return "";
  });

  text = text.replace(LEGACY_ATTACHED_RE, (_full, label: string, skipped?: string) => {
    const path = label.trim();
    const name = path.split(/[/\\]/).pop() || path;
    attachedFiles.push({
      name,
      path,
      skipped: skipped?.trim() || undefined,
    });
    return "";
  });

  text = text.replace(/\n{3,}/g, "\n\n").trim();
  return { text, attachedFiles };
}
