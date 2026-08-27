/**
 * Image / file attachment chips for ChatInputBar.
 */

import { ref, type Ref } from "vue";
import { nextTick } from "vue";
import {
  formatAttachedFilesForMessage,
  isImageFile,
  readAttachedFile,
  type AttachedFileChip,
} from "@/services/chat/attachFiles";
import { compressImageDataUrl } from "@/services/chat/compressImage";
import {
  describeRegionPlacement,
  EDIT_REGION_IMAGE_ALT,
  type ImageEditReferencePayload,
} from "@/services/chat/imageEditReference";
import type { ComposerSnapshot } from "@/services/chat/composerUndo";
import type { I18nKey } from "@/services/i18n";
import { tr } from "@/services/i18n";
import type { AppLanguage } from "@/types/setting";

export function useComposerAttachments(options: {
  language: Ref<AppLanguage>;
  emitLayoutChange: () => void;
  emitPreviewImage: (url: string) => void;
  effectiveChatMode: () => string;
  selectChatMode: (mode: "ask" | "agent" | "plan" | "image") => void;
  pushComposerUndo: (snapshot: ComposerSnapshot) => void;
  captureComposerSnapshot: () => ComposerSnapshot;
  clearComposerSegments: () => void;
  setMentionSuggestSuppressed: (value: null) => void;
  setComposerDraftText: (draft: string) => void;
  resizeComposerInput: () => void;
  focusInput: () => Promise<void> | void;
}) {
  const attachedImages = ref<string[]>([]);
  /**
   * Parallel to `attachedImages`: full-brightness edits references when the thumb
   * is a region paint preview. Null/undefined = the display image is the reference.
   */
  const attachedEditSources = ref<Array<string | null>>([]);
  const attachedFiles = ref<AttachedFileChip[]>([]);
  const fileDragOver = ref(false);

  function previewImage(url: string) {
    options.emitPreviewImage(url);
  }

  function removeAttachedImage(index: number) {
    attachedImages.value.splice(index, 1);
    attachedEditSources.value.splice(index, 1);
    options.emitLayoutChange();
  }

  function removeAttachedFile(index: number) {
    attachedFiles.value.splice(index, 1);
    options.emitLayoutChange();
  }

  async function ingestDroppedOrPastedFiles(files: FileList | File[]) {
    const list = Array.from(files);
    if (list.length === 0) return;
    for (const file of list) {
      if (isImageFile(file)) {
        const dataUrl = await new Promise<string | null>((resolve) => {
          const reader = new FileReader();
          reader.onload = () => resolve(String(reader.result ?? "") || null);
          reader.onerror = () => resolve(null);
          reader.readAsDataURL(file);
        });
        if (!dataUrl) continue;
        const compressed = await compressImageDataUrl(dataUrl);
        attachedImages.value.push(compressed);
        attachedEditSources.value.push(null);
        continue;
      }
      const chip = await readAttachedFile(file);
      attachedFiles.value.push(chip);
    }
    options.emitLayoutChange();
  }

  function onFileDragOver(event: DragEvent) {
    if (!event.dataTransfer?.types?.includes("Files")) return;
    fileDragOver.value = true;
  }

  function onFileDragLeave() {
    fileDragOver.value = false;
  }

  async function onFileDrop(event: DragEvent) {
    fileDragOver.value = false;
    const files = event.dataTransfer?.files;
    if (!files?.length) return;
    await ingestDroppedOrPastedFiles(files);
  }

  async function applyCapturedImages(images?: string[]) {
    if (!images?.length) {
      return;
    }
    const compressed = await Promise.all(images.map((url) => compressImageDataUrl(url)));
    attachedImages.value = compressed;
    attachedEditSources.value = compressed.map(() => null);
    options.emitLayoutChange();
  }

  /**
   * Attach a generated image for follow-up edits.
   * Region edits show the paint preview in the composer; the bright original is
   * kept separately so the edits API does not inherit the dimmed composite.
   */
  async function attachImageEditReference(payload: ImageEditReferencePayload) {
    if (!payload.images.length) return;
    if (options.effectiveChatMode() !== "image") {
      options.selectChatMode("image");
    }
    attachedImages.value = [...payload.images];
    attachedEditSources.value = payload.images.map((_, index) => {
      const source = payload.editSources?.[index];
      return source && source !== payload.images[index] ? source : null;
    });
    let draft = payload.draftText?.trim() || "";
    if (!draft) {
      if (payload.region) {
        const placement = payload.regionBounds
          ? describeRegionPlacement(payload.regionBounds)
          : "center";
        const whereKey = `image.region.${placement}` as I18nKey;
        const where = tr(options.language.value, whereKey);
        draft = tr(options.language.value, "image.editRegionDraft", { where });
      } else {
        draft = tr(options.language.value, "image.editFullDraft");
      }
    }
    options.pushComposerUndo(options.captureComposerSnapshot());
    options.clearComposerSegments();
    options.setMentionSuggestSuppressed(null);
    options.setComposerDraftText(draft);
    options.emitLayoutChange();
    await nextTick();
    options.resizeComposerInput();
    await options.focusInput();
  }

  function formatAttachedImagesForMessage(): string {
    return attachedImages.value
      .map((img, index) => {
        const editSource = attachedEditSources.value[index];
        if (editSource) {
          return `![${EDIT_REGION_IMAGE_ALT}](${img})\n![image](${editSource})`;
        }
        return `![image](${img})`;
      })
      .join("\n");
  }

  function clearAttachedImages() {
    attachedImages.value = [];
    attachedEditSources.value = [];
  }

  function clearAttachedFiles() {
    attachedFiles.value = [];
  }

  function clearAllAttachments() {
    clearAttachedImages();
    clearAttachedFiles();
  }

  function attachedFilesMessagePrefix(): string {
    return formatAttachedFilesForMessage(attachedFiles.value);
  }

  return {
    attachedImages,
    attachedEditSources,
    attachedFiles,
    fileDragOver,
    previewImage,
    removeAttachedImage,
    removeAttachedFile,
    ingestDroppedOrPastedFiles,
    onFileDragOver,
    onFileDragLeave,
    onFileDrop,
    applyCapturedImages,
    attachImageEditReference,
    formatAttachedImagesForMessage,
    clearAttachedImages,
    clearAttachedFiles,
    clearAllAttachments,
    attachedFilesMessagePrefix,
  };
}
