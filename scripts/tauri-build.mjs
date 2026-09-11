import { spawn } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

// 1. Auto-detect and load signing private key if not set
if (!process.env.TAURI_SIGNING_PRIVATE_KEY) {
  const candidateKeys = [
    join(homedir(), ".tauri", "anya.key"),
    join(homedir(), ".tauri", "aaai.key"),
  ];

  for (const keyPath of candidateKeys) {
    if (existsSync(keyPath)) {
      try {
        process.env.TAURI_SIGNING_PRIVATE_KEY = readFileSync(keyPath, "utf8");
        console.log(`[tauri-build] Loaded signing private key from ${keyPath}`);
        break;
      } catch (err) {
        console.warn(`[tauri-build] Failed to read ${keyPath}:`, err);
      }
    }
  }
}

// 2. Ensure the ripgrep sidecar exists for this host (Tauri externalBin).
const fetchRg = spawn("node", [join("scripts", "fetch-rg.mjs")], {
  cwd: root,
  stdio: "inherit",
  shell: true,
  env: process.env,
});
await new Promise((resolve, reject) => {
  fetchRg.on("close", (code) => {
    if (code === 0) {
      resolve();
    } else {
      reject(new Error(`fetch-rg exited ${code}`));
    }
  });
});

const fetchDeno = spawn("node", [join("scripts", "fetch-deno.mjs")], {
  cwd: root,
  stdio: "inherit",
  shell: true,
  env: process.env,
});
await new Promise((resolve, reject) => {
  fetchDeno.on("close", (code) => {
    if (code === 0) {
      resolve();
    } else {
      reject(new Error(`fetch-deno exited ${code}`));
    }
  });
});

// 3. Run tauri build with passed arguments
const isWindows = process.platform === "win32";
const tauriBin = isWindows ? "tauri.cmd" : "tauri";
const args = ["build", ...process.argv.slice(2)];

const child = spawn(tauriBin, args, {
  cwd: root,
  stdio: "inherit",
  shell: true,
  env: process.env,
});

child.on("close", (code) => {
  if (code !== 0) {
    process.exit(code ?? 1);
  }
  // Run rename script
  const renameChild = spawn("node", [join("scripts", "rename-msi.mjs")], {
    cwd: root,
    stdio: "inherit",
    shell: true,
  });
  renameChild.on("close", (renameCode) => {
    process.exit(renameCode ?? 0);
  });
});
