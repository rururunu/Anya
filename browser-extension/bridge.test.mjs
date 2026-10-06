import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import net from "node:net";
import { spawn } from "node:child_process";
import vm from "node:vm";

const host = path.resolve("src-tauri/target/debug/anya-browser-bridge.exe");
test("native host preserves framing, injects authentication and rejects foreign origins", async () => {
  assert.ok(fs.existsSync(host), "Build anya-browser-bridge before running this test");
  const local = fs.mkdtempSync(path.join(os.tmpdir(), "anya-bridge-test-"));
  fs.mkdirSync(path.join(local, "Anya"));
  const origin = "chrome-extension://test/";
  fs.writeFileSync(path.join(local, "Anya/browser-extension-origin.txt"), origin);
  let received;
  const server = net.createServer((socket) => {
    let buffer = Buffer.alloc(0);
    socket.on("data", (chunk) => {
      buffer = Buffer.concat([buffer, chunk]);
      if (buffer.length < 4 || buffer.length < 4 + buffer.readUInt32LE(0)) return;
      received = JSON.parse(buffer.subarray(4).toString());
      const reply = Buffer.from('{"ok":true}');
      const header = Buffer.alloc(4);
      header.writeUInt32LE(reply.length);
      socket.end(Buffer.concat([header, reply]));
    });
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  fs.writeFileSync(
    path.join(local, "Anya/browser-bridge.json"),
    JSON.stringify({ port: server.address().port, token: "secret" }),
  );
  const run = async (extensionOrigin) => {
    const child = spawn(host, [extensionOrigin], {
      env: { ...process.env, LOCALAPPDATA: local },
      windowsHide: true,
    });
    const output = [];
    child.stdout.on("data", (data) => output.push(data));
    const payload = Buffer.from(
      JSON.stringify({ text: "中文 selection", title: "Page", url: "https://example.com" }),
    );
    const header = Buffer.alloc(4);
    header.writeUInt32LE(payload.length);
    child.stdin.on("error", () => {});
    child.stdin.end(Buffer.concat([header, payload]));
    await new Promise((resolve, reject) => {
      child.on("error", reject);
      child.on("exit", resolve);
    });
    return Buffer.concat(output);
  };
  try {
    const response = await run(origin);
    assert.equal(response.readUInt32LE(0), response.length - 4);
    assert.deepEqual(JSON.parse(response.subarray(4).toString()), { ok: true });
    assert.equal(received.token, "secret");
    assert.equal(received.text, "中文 selection");
    assert.equal((await run("chrome-extension://foreign/")).length, 0);
  } finally {
    server.close();
    const resolved = path.resolve(local);
    assert.equal(path.dirname(resolved), path.resolve(os.tmpdir()));
    assert.ok(path.basename(resolved).startsWith("anya-bridge-test-"));
    fs.rmSync(resolved, { recursive: true, force: true });
  }
});

test("content script captures editable selection but excludes passwords", async () => {
  const sent = [];
  let refresh;
  class Input {
    constructor(type) {
      this.type = type;
      this.value = "private text";
      this.selectionStart = 0;
      this.selectionEnd = 7;
    }
  }
  const document = { activeElement: new Input("password"), title: "Page", addEventListener() {} };
  const context = {
    document,
    HTMLInputElement: Input,
    HTMLTextAreaElement: class {},
    location: { href: "https://example.com" },
    window: { getSelection: () => ({ toString: () => "web selection" }), addEventListener() {} },
    chrome: {
      runtime: {
        sendMessage: async (message) => sent.push(message),
        onMessage: { addListener: (handler) => (refresh = handler) },
      },
    },
    setTimeout,
    clearTimeout,
  };
  vm.runInNewContext(fs.readFileSync("browser-extension/content.js", "utf8"), context);
  refresh({ type: "anya.refreshSelection" });
  assert.equal(sent[0].text, "");
  document.activeElement = new Input("text");
  refresh({ type: "anya.refreshSelection" });
  assert.equal(sent[1].text, "private");
  document.activeElement = null;
  refresh({ type: "anya.refreshSelection" });
  assert.equal(sent[2].text, "web selection");
});

test("background accepts only active-tab selections and keeps iframe ownership", async () => {
  let receive;
  const packets = [];
  const listener = { addListener() {} };
  const context = {
    chrome: {
      runtime: {
        id: "trusted",
        onMessage: { addListener: (handler) => (receive = handler) },
        connectNative: () => ({
          onMessage: listener,
          onDisconnect: listener,
          postMessage: (packet) => packets.push(packet),
        }),
      },
      tabs: {
        query: async () => [
          { id: 1, windowId: 2, title: "Actual page", url: "https://actual.example" },
        ],
        onActivated: listener,
        onUpdated: listener,
      },
      windows: { get: async () => ({ focused: true }), onFocusChanged: listener },
    },
  };
  vm.runInNewContext(fs.readFileSync("browser-extension/background.js", "utf8"), context);
  const message = {
    type: "anya.selection",
    text: "frame text",
    title: "fake title",
    url: "https://fake.example",
  };
  receive(message, { id: "foreign", tab: { id: 1 }, frameId: 4 });
  receive(message, { id: "trusted", tab: { id: 9 }, frameId: 4 });
  await new Promise(setImmediate);
  assert.equal(packets.length, 0);
  const sender = { id: "trusted", tab: { id: 1 }, frameId: 4, url: "https://actual.example/frame" };
  receive(message, sender);
  await new Promise(setImmediate);
  assert.equal(packets[0].title, "Actual page");
  assert.equal(packets[0].url, sender.url);
  receive({ ...message, text: "" }, { ...sender, frameId: 0 });
  await new Promise(setImmediate);
  assert.equal(packets.length, 1);
  receive({ ...message, text: "" }, sender);
  await new Promise(setImmediate);
  assert.equal(packets[1].text, "");
});
