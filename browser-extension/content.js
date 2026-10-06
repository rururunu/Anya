(() => {
  let timer;
  let previous = null;
  function selectionText() {
    let active = document.activeElement;
    while (active?.shadowRoot?.activeElement) active = active.shadowRoot.activeElement;
    if (active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement) {
      // Never capture password fields, including through clipboard fallback requests.
      if (active instanceof HTMLInputElement && active.type === "password") return "";
      if (typeof active.selectionStart === "number" && typeof active.selectionEnd === "number") {
        return active.value.slice(active.selectionStart, active.selectionEnd);
      }
    }
    return window.getSelection()?.toString() || "";
  }
  function report(force = false) {
    const text = selectionText().slice(0, 65536);
    if (!force && text === previous) return;
    previous = text;
    chrome.runtime
      .sendMessage({ type: "anya.selection", text, url: location.href, title: document.title })
      .catch(() => {});
  }
  function schedule() {
    clearTimeout(timer);
    timer = setTimeout(report, 60);
  }
  document.addEventListener("selectionchange", schedule);
  document.addEventListener("pointerup", schedule);
  document.addEventListener("keyup", schedule);
  chrome.runtime.onMessage.addListener((message) => {
    if (message.type === "anya.refreshSelection") report(true);
  });
  window.addEventListener("pageshow", () => report(true));
})();
