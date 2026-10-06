let port = null;
let selectedFrame = null;
let selectedTab = null;
function send(packet) {
  try {
    if (!port) {
      port = chrome.runtime.connectNative("ai.anya.selection");
      port.onMessage.addListener(() => {});
      port.onDisconnect.addListener(() => {
        void chrome.runtime.lastError;
        port = null;
      });
    }
    port.postMessage(packet);
  } catch {
    port = null;
  }
}
function clearSelection() {
  selectedFrame = null;
  selectedTab = null;
  send({ text: "", url: "", title: "" });
}
chrome.runtime.onMessage.addListener((message, sender) => {
  if (
    sender.id !== chrome.runtime.id ||
    message.type !== "anya.selection" ||
    !sender.tab ||
    typeof message.text !== "string"
  )
    return;
  void (async () => {
    const tabs = await chrome.tabs.query({ active: true, lastFocusedWindow: true });
    const tab = tabs[0];
    if (!tab || tab.id !== sender.tab.id) return;
    const window = await chrome.windows.get(tab.windowId);
    if (!window.focused) return;
    if (!message.text && selectedFrame !== null && selectedFrame !== sender.frameId) return;
    selectedFrame = message.text ? sender.frameId : null;
    selectedTab = message.text ? tab.id : null;
    // Source comes from Chrome's sender/tab metadata rather than page-supplied title.
    send({
      text: message.text.slice(0, 65536),
      url: sender.url || tab.url || "",
      title: tab.title || "",
    });
  })().catch(() => {});
});
chrome.tabs.onActivated.addListener(({ tabId }) => {
  clearSelection();
  chrome.tabs.sendMessage(tabId, { type: "anya.refreshSelection" }).catch(() => {});
});
chrome.tabs.onUpdated.addListener((id, change) => {
  if (change.url && id === selectedTab) clearSelection();
});
chrome.windows.onFocusChanged.addListener((id) => {
  if (id === chrome.windows.WINDOW_ID_NONE) return;
  clearSelection();
  chrome.tabs
    .query({ active: true, windowId: id })
    .then((tabs) => {
      if (tabs[0]?.id)
        chrome.tabs.sendMessage(tabs[0].id, { type: "anya.refreshSelection" }).catch(() => {});
    })
    .catch(() => {});
});
