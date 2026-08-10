import type { AttachedFileDisplay } from "@/services/chat/attachFiles";
import { extractAttachedFiles } from "@/services/chat/attachFiles";

const SELECTION_OPEN = /\n\n<peek-selection lines="(\d+)">\n/;
const SELECTION_CLOSE = "\n</peek-selection>";
const ANALYSIS_TAG_RE =
  /<peek-image-analysis\s+model="([^"]*)">\s*([\s\S]*?)\s*<\/peek-image-analysis>/g;

export interface ImageAnalysis {
  model: string;
  text: string;
}

export interface SelectionAttachment {
  message: string;
  selection?: string;
  lineCount?: number;
  images?: string[];
  imageAnalyses?: ImageAnalysis[];
  attachedFiles?: AttachedFileDisplay[];
}

export function selectionLineCount(selection: string) {
  const normalized = selection.trim();
  return normalized ? normalized.split(/\r\n|\r|\n/).length : 0;
}

export function attachSelection(message: string, selection?: string) {
  const normalized = selection?.trim() ?? "";
  if (!normalized) return message.trim();
  const lines = selectionLineCount(normalized);
  return `${message.trim()}\n\n<peek-selection lines="${lines}">\n${normalized}${SELECTION_CLOSE}`;
}

export function parseSelectionAttachment(content: string | undefined | null): SelectionAttachment {
  const source = content ?? "";
  const match = SELECTION_OPEN.exec(source);
  let cleanContent = source;
  let selection: string | undefined = undefined;
  let lineCount: number | undefined = undefined;

  if (match) {
    const closeIndex = source.lastIndexOf(SELECTION_CLOSE);
    if (closeIndex >= match.index + match[0].length) {
      cleanContent = source.slice(0, match.index).trim();
      selection = source.slice(match.index + match[0].length, closeIndex);
      lineCount = Number(match[1]);
    }
  }

  const imageAnalyses: ImageAnalysis[] = [];
  let analysisMatch: RegExpExecArray | null;
  const analysisRe = new RegExp(ANALYSIS_TAG_RE.source, "g");
  while ((analysisMatch = analysisRe.exec(cleanContent)) !== null) {
    imageAnalyses.push({
      model: analysisMatch[1] || "",
      text: (analysisMatch[2] || "").trim(),
    });
  }

  const images: string[] = [];
  // Accept data URLs and plain paths/URLs inside ![image](...).
  const imageRegex = /!\[image\]\(([^)\s]+)\)/g;

  let m;
  while ((m = imageRegex.exec(cleanContent)) !== null) {
    const raw = (m[1] || "").trim();
    if (raw) images.push(raw);
  }

  let messageText = cleanContent
    .replace(new RegExp(ANALYSIS_TAG_RE.source, "g"), "")
    .replace(imageRegex, "")
    .trim();

  const extracted = extractAttachedFiles(messageText);
  messageText = extracted.text.replace(/\n\n+/g, "\n\n").trim();

  return {
    message: messageText,
    selection,
    lineCount,
    images: images.length > 0 ? images : undefined,
    imageAnalyses: imageAnalyses.length > 0 ? imageAnalyses : undefined,
    attachedFiles: extracted.attachedFiles.length > 0 ? extracted.attachedFiles : undefined,
  };
}
