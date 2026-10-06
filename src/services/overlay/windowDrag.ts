import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

const NO_DRAG_SELECTOR =
  "input, textarea, button, a, select, [contenteditable='true'], [data-no-drag], [data-tauri-drag-region='false'], .message-list, .user-images, .user-image-btn, .lightbox, .command-list, .command-item, .model-menu-floating, .model-menu-item, .image-thumb-container, .image-thumb";

export function shouldStartWindowDrag(target: EventTarget | null) {
  if (!(target instanceof Element)) {
    return false;
  }

  return !target.closest(NO_DRAG_SELECTOR);
}

export async function startWindowDrag() {
  await getCurrentWebviewWindow().startDragging();
}

export function onWindowDragMouseDown(event: MouseEvent) {
  if (event.button !== 0 || event.defaultPrevented) {
    return;
  }

  if (!shouldStartWindowDrag(event.target)) {
    return;
  }

  event.preventDefault();
  void startWindowDrag();
}
